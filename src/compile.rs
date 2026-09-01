//! expressions plus let/assign/index-assign/expression statements and
//! bare blocks. Everything else reports "not yet supported by --vm".

use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::lexer::Span;
use crate::value::Value;

#[derive(Debug, Clone)]
pub enum Op {
    /// Push consts[i].
    Const(u32),
    Nil,
    True,
    False,
    /// Push the variable's value (names[i]).
    GetVar(u32),
    /// Define names[i] in the current scope from the top of stack.
    Define(u32),
    /// Rebind names[i] from the top of stack.
    SetVar(u32),
    Unary(UnaryOp),
    Binary(BinaryOp),
    /// Pop n items into a fresh list.
    MakeList(u32),
    /// Pop 2n items (key/value pairs, in order) into a fresh map.
    MakeMap(u32),
    /// stack: [base, idx] -> [base[idx]]
    Index,
    /// stack: [base, idx, value] -> []
    IndexSet,
    /// stack: [callee, arg0..argn-1] -> [result]; the span names the
    /// callee for not-callable errors (matching the tree-walker).
    Call(u8, Span),
    /// Relative jump.
    Jump(i32),
    /// Pop a bool (strict); jump when false.
    JumpIfFalse(i32),
    /// Peek a bool (strict); jump when true, else pop. (for ||)
    OrJump(i32),
    /// Peek a bool (strict); jump when false, else pop. (for &&)
    AndJump(i32),
    /// Error unless the top of stack is a bool; leaves it in place.
    CheckBool,
    /// Error unless the top of stack is a string (a map key).
    CheckMapKey,
    Pop,
    /// Enter a fresh lexical scope.
    PushScope,
    /// Leave the current scope.
    PopScope,
    /// Pop the iterable, push its snapshot list (for-loop semantics).
    IterNew,
    /// stack: [snap, idx]. If idx == len(snap): jump. Else: bump idx
    /// and push snap[idx].
    IterNext(i32),
    /// Create a closure from protos[i], capturing the current env.
    MakeFn(u32),
    /// Pop the return value and leave the current function frame.
    Return,
}

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<Op>,
    pub consts: Vec<Value>,
    pub names: Vec<String>,
    /// Function bodies stay AST: the VM builds ordinary closures that
    /// the reference engine executes (docs/vm.md hybrid step).
    pub protos: Vec<FnProto>,
    /// spans[i] belongs to code[i]; used for diagnostics.
    pub spans: Vec<Span>,
}

#[derive(Debug)]
pub struct FnProto {
    pub params: Vec<String>,
    pub chunk: std::rc::Rc<Chunk>,
}

pub struct CompileError {
    pub message: String,
    pub span: Span,
}

fn unsupported(what: &str, span: Span) -> CompileError {
    CompileError {
        message: format!("{what} is not yet supported by --vm"),
        span,
    }
}

pub fn compile_program(stmts: &[Stmt]) -> Result<Chunk, CompileError> {
    compile_stmts(stmts, false)
}

fn compile_stmts(stmts: &[Stmt], in_function: bool) -> Result<Chunk, CompileError> {
    let mut c = Compiler {
        chunk: Chunk {
            code: Vec::new(),
            consts: Vec::new(),
            names: Vec::new(),
            protos: Vec::new(),
            spans: Vec::new(),
        },
        loops: Vec::new(),
        scope_depth: 0,
        in_function,
    };
    for s in stmts {
        c.stmt(s)?;
    }
    Ok(c.chunk)
}

/// Per-loop bookkeeping for break/continue lowering.
struct LoopCtx {
    /// Where continue jumps (cond for while, IterNext for for).
    continue_target: usize,
    /// Jump ops to patch to the loop's end.
    break_patches: Vec<usize>,
    /// Compiler scope depth just outside the loop body; break/continue
    /// emit PopScope down to it before jumping. Break needs no extra
    /// stack cleanup: a for-loop's [snapshot, index] slots are popped
    /// at the shared end label both break and exhaustion jump to.
    scope_depth: usize,
}

struct Compiler {
    chunk: Chunk,
    loops: Vec<LoopCtx>,
    scope_depth: usize,
    in_function: bool,
}

impl Compiler {
    fn emit(&mut self, op: Op, span: Span) {
        self.chunk.code.push(op);
        self.chunk.spans.push(span);
    }

    fn konst(&mut self, v: Value) -> u32 {
        self.chunk.consts.push(v);
        (self.chunk.consts.len() - 1) as u32
    }

    fn name(&mut self, n: &str) -> u32 {
        if let Some(i) = self.chunk.names.iter().position(|x| x == n) {
            return i as u32;
        }
        self.chunk.names.push(n.to_string());
        (self.chunk.names.len() - 1) as u32
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), CompileError> {
        match &s.kind {
            StmtKind::Let(name, init) => {
                self.expr(init)?;
                let i = self.name(name);
                self.emit(Op::Define(i), s.span);
            }
            StmtKind::Assign(name, value) => {
                self.expr(value)?;
                let i = self.name(name);
                self.emit(Op::SetVar(i), s.span);
            }
            StmtKind::IndexAssign(base, idx, value) => {
                self.expr(base)?;
                self.expr(idx)?;
                self.expr(value)?;
                self.emit(Op::IndexSet, s.span);
            }
            StmtKind::Expr(e) => {
                self.expr(e)?;
                self.emit(Op::Pop, s.span);
            }
            // Same rule as the tree-walker: only a block with direct
            // declarations needs its own scope.
            StmtKind::Block(stmts) => {
                let scoped = stmts.iter().any(|st| matches!(st.kind, StmtKind::Let(..)));
                if scoped {
                    self.emit(Op::PushScope, s.span);
                    self.scope_depth += 1;
                }
                for st in stmts {
                    self.stmt(st)?;
                }
                if scoped {
                    self.scope_depth -= 1;
                    self.emit(Op::PopScope, s.span);
                }
            }
            StmtKind::If(cond, then, els) => {
                self.expr(cond)?;
                let to_else = self.chunk.code.len();
                self.emit(Op::JumpIfFalse(0), cond.span);
                self.stmt(then)?;
                match els {
                    Some(els) => {
                        let to_end = self.chunk.code.len();
                        self.emit(Op::Jump(0), s.span);
                        let here = self.chunk.code.len() as i32;
                        self.patch(to_else, here);
                        self.stmt(els)?;
                        let here = self.chunk.code.len() as i32;
                        self.patch(to_end, here);
                    }
                    None => {
                        let here = self.chunk.code.len() as i32;
                        self.patch(to_else, here);
                    }
                }
            }
            StmtKind::While(cond, body) => {
                let loop_start = self.chunk.code.len();
                self.expr(cond)?;
                let to_end = self.chunk.code.len();
                self.emit(Op::JumpIfFalse(0), cond.span);
                self.loops.push(LoopCtx {
                    continue_target: loop_start,
                    break_patches: vec![to_end],
                    scope_depth: self.scope_depth,
                });
                self.stmt(body)?;
                let back = self.chunk.code.len();
                self.emit(Op::Jump(0), s.span);
                self.patch(back, loop_start as i32);
                let ctx = self.loops.pop().expect("loop ctx");
                let end = self.chunk.code.len() as i32;
                for at in ctx.break_patches {
                    self.patch(at, end);
                }
            }
            StmtKind::For(var, iterable, body) => {
                self.expr(iterable)?;
                self.emit(Op::IterNew, iterable.span);
                let zero = self.konst(Value::Int(0));
                self.emit(Op::Const(zero), s.span);
                let next_ip = self.chunk.code.len();
                self.emit(Op::IterNext(0), s.span);
                self.loops.push(LoopCtx {
                    continue_target: next_ip,
                    break_patches: vec![next_ip],
                    scope_depth: self.scope_depth,
                });
                // Fresh scope per iteration, like the tree-walker.
                self.emit(Op::PushScope, s.span);
                self.scope_depth += 1;
                let vi = self.name(var);
                self.emit(Op::Define(vi), s.span);
                self.stmt(body)?;
                self.scope_depth -= 1;
                self.emit(Op::PopScope, s.span);
                let back = self.chunk.code.len();
                self.emit(Op::Jump(0), s.span);
                self.patch(back, next_ip as i32);
                let ctx = self.loops.pop().expect("loop ctx");
                let end = self.chunk.code.len() as i32;
                for at in ctx.break_patches {
                    self.patch(at, end);
                }
                // The loop owned [snapshot, index] on the stack; both
                // jump-to-end paths (done and break) land here.
                self.emit(Op::Pop, s.span);
                self.emit(Op::Pop, s.span);
            }
            StmtKind::Break => {
                let Some(ctx_depth) = self.loops.last().map(|c| c.scope_depth) else {
                    return Err(CompileError {
                        message: "break outside loop".to_string(),
                        span: s.span,
                    });
                };
                for _ in ctx_depth..self.scope_depth {
                    self.emit(Op::PopScope, s.span);
                }
                let at = self.chunk.code.len();
                self.emit(Op::Jump(0), s.span);
                self.loops
                    .last_mut()
                    .expect("loop ctx")
                    .break_patches
                    .push(at);
            }
            StmtKind::Continue => {
                let Some(ctx_depth) = self.loops.last().map(|c| c.scope_depth) else {
                    return Err(CompileError {
                        message: "continue outside loop".to_string(),
                        span: s.span,
                    });
                };
                for _ in ctx_depth..self.scope_depth {
                    self.emit(Op::PopScope, s.span);
                }
                let target = self.loops.last().expect("loop ctx").continue_target;
                let at = self.chunk.code.len();
                self.emit(Op::Jump(0), s.span);
                self.patch(at, target as i32);
            }
            StmtKind::Return(value) => {
                if !self.in_function {
                    // Same message as the tree-walker, surfaced at
                    // compile time (accepted divergence).
                    return Err(CompileError {
                        message: "return outside function".to_string(),
                        span: s.span,
                    });
                }
                match value {
                    Some(e) => self.expr(e)?,
                    None => self.emit(Op::Nil, s.span),
                }
                self.emit(Op::Return, s.span);
            }
        }
        Ok(())
    }

    fn expr(&mut self, e: &Expr) -> Result<(), CompileError> {
        match &e.kind {
            ExprKind::Int(n) => {
                let i = self.konst(Value::Int(*n));
                self.emit(Op::Const(i), e.span);
            }
            ExprKind::Float(x) => {
                let i = self.konst(Value::Float(*x));
                self.emit(Op::Const(i), e.span);
            }
            ExprKind::Str(s) => {
                let i = self.konst(Value::Str(s.clone()));
                self.emit(Op::Const(i), e.span);
            }
            ExprKind::Bool(true) => self.emit(Op::True, e.span),
            ExprKind::Bool(false) => self.emit(Op::False, e.span),
            ExprKind::Nil => self.emit(Op::Nil, e.span),
            ExprKind::Var(name) => {
                let i = self.name(name);
                self.emit(Op::GetVar(i), e.span);
            }
            ExprKind::List(items) => {
                for it in items {
                    self.expr(it)?;
                }
                self.emit(Op::MakeList(items.len() as u32), e.span);
            }
            ExprKind::Map(entries) => {
                for (k, v) in entries {
                    self.expr(k)?;
                    self.emit(Op::CheckMapKey, k.span);
                    self.expr(v)?;
                }
                self.emit(Op::MakeMap(entries.len() as u32), e.span);
            }
            ExprKind::Unary(op, operand) => {
                self.expr(operand)?;
                self.emit(Op::Unary(*op), e.span);
            }
            ExprKind::Binary(BinaryOp::And, lhs, rhs) => {
                self.expr(lhs)?;
                let patch = self.chunk.code.len();
                self.emit(Op::AndJump(0), lhs.span);
                self.expr(rhs)?;
                self.emit(Op::CheckBool, rhs.span);
                let target = self.chunk.code.len() as i32;
                self.patch(patch, target);
            }
            ExprKind::Binary(BinaryOp::Or, lhs, rhs) => {
                self.expr(lhs)?;
                let patch = self.chunk.code.len();
                self.emit(Op::OrJump(0), lhs.span);
                self.expr(rhs)?;
                self.emit(Op::CheckBool, rhs.span);
                let target = self.chunk.code.len() as i32;
                self.patch(patch, target);
            }
            ExprKind::Binary(op, lhs, rhs) => {
                self.expr(lhs)?;
                self.expr(rhs)?;
                self.emit(Op::Binary(*op), e.span);
            }
            ExprKind::Index(base, idx) => {
                self.expr(base)?;
                self.expr(idx)?;
                self.emit(Op::Index, e.span);
            }
            ExprKind::Call(callee, args) => {
                self.expr(callee)?;
                for a in args {
                    self.expr(a)?;
                }
                if args.len() > u8::MAX as usize {
                    return Err(unsupported("more than 255 arguments", e.span));
                }
                self.emit(Op::Call(args.len() as u8, callee.span), e.span);
            }
            ExprKind::Fn(params, body) => {
                let chunk = compile_stmts(body, true)?;
                self.chunk.protos.push(FnProto {
                    params: params.clone(),
                    chunk: std::rc::Rc::new(chunk),
                });
                let i = (self.chunk.protos.len() - 1) as u32;
                self.emit(Op::MakeFn(i), e.span);
            }
        }
        Ok(())
    }

    fn patch(&mut self, at: usize, target: i32) {
        let rel = target - at as i32 - 1;
        match &mut self.chunk.code[at] {
            Op::Jump(o) | Op::JumpIfFalse(o) | Op::OrJump(o) | Op::AndJump(o) | Op::IterNext(o) => {
                *o = rel
            }
            _ => unreachable!("patched op is not a jump"),
        }
    }
}
