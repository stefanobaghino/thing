//! Bytecode executor (see docs/vm.md). Runs a Chunk against the same
//! Interpreter context the tree-walker uses — same Env, builtins,
//! output writer, and error type — so behavior stays identical.

use crate::compile::{Chunk, Op};
use crate::eval::{self, Interpreter, RuntimeError};
use crate::value::Value;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;

// Function calls run a chunk each; pooling the operand-stack and
// locals buffers avoids two heap allocations per call.
thread_local! {
    static BUF_POOL: RefCell<Vec<Vec<Value>>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn take_buf() -> Vec<Value> {
    BUF_POOL.with(|p| p.borrow_mut().pop()).unwrap_or_default()
}

pub(crate) fn give_buf(mut buf: Vec<Value>) {
    buf.clear();
    BUF_POOL.with(|p| {
        let mut p = p.borrow_mut();
        if p.len() < 64 {
            p.push(buf);
        }
    });
}

/// Execute a chunk. `Ok(Some(v))` means an Op::Return fired (function
/// frames); `Ok(None)` means the code ran off the end (top level, or a
/// function falling through to an implicit nil).
pub fn run_chunk<W: Write>(
    interp: &mut Interpreter<W>,
    chunk: &Chunk,
) -> Result<Option<Value>, RuntimeError> {
    run_chunk_with(interp, chunk, &mut [])
}

/// `run_chunk` with a caller-prepared locals frame (function calls;
/// eval::call fills parameter slots before entry).
pub fn run_chunk_with<W: Write>(
    interp: &mut Interpreter<W>,
    chunk: &Chunk,
    locals: &mut [Value],
) -> Result<Option<Value>, RuntimeError> {
    let mut stack = take_buf();
    let r = exec(interp, chunk, locals, &mut stack);
    give_buf(stack);
    r
}

fn exec<W: Write>(
    interp: &mut Interpreter<W>,
    chunk: &Chunk,
    locals: &mut [Value],
    stack: &mut Vec<Value>,
) -> Result<Option<Value>, RuntimeError> {
    let mut ip = 0usize;
    while ip < chunk.code.len() {
        let span = chunk.spans[ip];
        match &chunk.code[ip] {
            Op::Const(i) => stack.push(chunk.consts[*i as usize].clone()),
            Op::Nil => stack.push(Value::Nil),
            Op::True => stack.push(Value::Bool(true)),
            Op::False => stack.push(Value::Bool(false)),
            Op::GetVar(i) => {
                let name = &chunk.names[*i as usize];
                match interp.lookup(name) {
                    Some(v) => stack.push(v),
                    None => {
                        return Err(eval::error(format!("undefined variable '{name}'"), span));
                    }
                }
            }
            Op::Define(i) => {
                let v = stack.pop().expect("stack underflow");
                interp.define(&chunk.names[*i as usize], v);
            }
            Op::SetVar(i) => {
                let v = stack.pop().expect("stack underflow");
                let name = &chunk.names[*i as usize];
                if !interp.assign(name, v) {
                    return Err(eval::error(
                        format!("cannot assign to undefined variable '{name}'"),
                        span,
                    ));
                }
            }
            Op::Unary(op) => {
                let v = stack.pop().expect("stack underflow");
                stack.push(eval::unary(*op, v, span)?);
            }
            Op::Binary(op) => {
                let r = stack.pop().expect("stack underflow");
                let l = stack.pop().expect("stack underflow");
                stack.push(eval::binary(*op, l, r, span)?);
            }
            Op::MakeList(n) => {
                let items = stack.split_off(stack.len() - *n as usize);
                stack.push(Value::list(items));
            }
            Op::MakeMap(n) => {
                let kvs = stack.split_off(stack.len() - 2 * *n as usize);
                let mut m = BTreeMap::new();
                for pair in kvs.chunks(2) {
                    match &pair[0] {
                        Value::Str(k) => {
                            m.insert(k.clone(), pair[1].clone());
                        }
                        other => {
                            return Err(eval::error(
                                format!("map keys must be strings, got {}", other.type_name()),
                                span,
                            ));
                        }
                    }
                }
                stack.push(Value::map(m));
            }
            Op::Index => {
                let idx = stack.pop().expect("stack underflow");
                let base = stack.pop().expect("stack underflow");
                stack.push(eval::index(base, idx, span)?);
            }
            Op::IndexSet => {
                let value = stack.pop().expect("stack underflow");
                let idx = stack.pop().expect("stack underflow");
                let base = stack.pop().expect("stack underflow");
                match (base, idx) {
                    (Value::List(items), Value::Int(n)) => {
                        let mut items = items.borrow_mut();
                        let eff = eval::effective_index(n, items.len(), span)?;
                        items[eff] = value;
                    }
                    (Value::Map(entries), Value::Str(k)) => {
                        entries.borrow_mut().insert(k, value);
                    }
                    (b, i) => {
                        return Err(eval::error(
                            format!(
                                "cannot index-assign {} with {}",
                                b.type_name(),
                                i.type_name()
                            ),
                            span,
                        ));
                    }
                }
            }
            Op::Call(argc, callee_span) => {
                let args = stack.split_off(stack.len() - *argc as usize);
                let callee = stack.pop().expect("stack underflow");
                match &callee {
                    Value::Fn(_) | Value::Builtin(_) => {
                        stack.push(interp.call_value(&callee, args, span)?)
                    }
                    other => {
                        return Err(eval::error(
                            format!("{} is not callable", other.type_name()),
                            *callee_span,
                        ));
                    }
                }
            }
            Op::Jump(o) => {
                ip = offset(ip, *o);
                continue;
            }
            Op::JumpIfFalse(o) => {
                let v = stack.pop().expect("stack underflow");
                if !eval::as_bool(v, span)? {
                    ip = offset(ip, *o);
                    continue;
                }
            }
            Op::OrJump(o) => match stack.last() {
                Some(Value::Bool(true)) => {
                    ip = offset(ip, *o);
                    continue;
                }
                Some(Value::Bool(false)) => {
                    stack.pop();
                }
                Some(v) => {
                    return Err(eval::error(
                        format!("expected bool, got {}", v.type_name()),
                        span,
                    ));
                }
                None => unreachable!("stack underflow"),
            },
            Op::AndJump(o) => match stack.last() {
                Some(Value::Bool(false)) => {
                    ip = offset(ip, *o);
                    continue;
                }
                Some(Value::Bool(true)) => {
                    stack.pop();
                }
                Some(v) => {
                    return Err(eval::error(
                        format!("expected bool, got {}", v.type_name()),
                        span,
                    ));
                }
                None => unreachable!("stack underflow"),
            },
            Op::CheckMapKey => match stack.last() {
                Some(Value::Str(_)) => {}
                Some(v) => {
                    return Err(eval::error(
                        format!("map keys must be strings, got {}", v.type_name()),
                        span,
                    ));
                }
                None => unreachable!("stack underflow"),
            },
            Op::CheckBool => match stack.last() {
                Some(Value::Bool(_)) => {}
                Some(v) => {
                    return Err(eval::error(
                        format!("expected bool, got {}", v.type_name()),
                        span,
                    ));
                }
                None => unreachable!("stack underflow"),
            },
            Op::Pop => {
                stack.pop();
            }
            Op::MakeFn(i) => {
                let proto = &chunk.protos[*i as usize];
                stack.push(Value::Fn(std::rc::Rc::new(eval::Function {
                    params: proto.params.iter().map(|p| p.as_str().into()).collect(),
                    body: eval::FnBody::Chunk(std::rc::Rc::clone(&proto.chunk)),
                    env: interp.env_handle(),
                    origin: interp.current_origin(),
                })));
            }
            Op::Return => {
                let v = stack.pop().expect("stack underflow");
                return Ok(Some(v));
            }
            Op::GetSlot(i) => stack.push(locals[*i as usize].clone()),
            Op::SetSlot(i) => {
                locals[*i as usize] = stack.pop().expect("stack underflow");
            }
            Op::PushScope => interp.push_scope(),
            Op::PopScope => interp.pop_scope(),
            Op::IterNew => {
                let v = stack.pop().expect("stack underflow");
                stack.push(Value::list(eval::iter_snapshot(v, span)?));
            }
            Op::IterNext(o) => {
                let len = stack.len();
                let idx = match &stack[len - 1] {
                    Value::Int(i) => *i as usize,
                    _ => unreachable!("iter index is always an int"),
                };
                let item = {
                    let Value::List(snap) = &stack[len - 2] else {
                        unreachable!("iter snapshot is always a list");
                    };
                    let snap = snap.borrow();
                    if idx >= snap.len() {
                        None
                    } else {
                        Some(snap[idx].clone())
                    }
                };
                match item {
                    Some(item) => {
                        stack[len - 1] = Value::Int(idx as i64 + 1);
                        stack.push(item);
                    }
                    None => {
                        ip = offset(ip, *o);
                        continue;
                    }
                }
            }
        }
        ip += 1;
    }
    Ok(None)
}

fn offset(ip: usize, rel: i32) -> usize {
    (ip as i64 + 1 + rel as i64) as usize
}
