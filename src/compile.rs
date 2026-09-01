//! AST → bytecode compiler (see docs/vm.md). Rollout step 1: full
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
}

pub struct Chunk {
    pub code: Vec<Op>,
    pub consts: Vec<Value>,
    pub names: Vec<String>,
    /// spans[i] belongs to code[i]; used for diagnostics.
    pub spans: Vec<Span>,
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
    let mut c = Compiler {
        chunk: Chunk {
            code: Vec::new(),
            consts: Vec::new(),
            names: Vec::new(),
            spans: Vec::new(),
        },
    };
    for s in stmts {
        c.stmt(s)?;
    }
    Ok(c.chunk)
}

struct Compiler {
    chunk: Chunk,
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
            // A top-level block without declarations is transparent for
            // the tree-walker too; scoped blocks come with control flow.
            StmtKind::Block(stmts) => {
                if stmts.iter().any(|st| matches!(st.kind, StmtKind::Let(..))) {
                    return Err(unsupported("a block with declarations", s.span));
                }
                for st in stmts {
                    self.stmt(st)?;
                }
            }
            StmtKind::If(..) => return Err(unsupported("if", s.span)),
            StmtKind::While(..) => return Err(unsupported("while", s.span)),
            StmtKind::For(..) => return Err(unsupported("for", s.span)),
            StmtKind::Break => return Err(unsupported("break", s.span)),
            StmtKind::Continue => return Err(unsupported("continue", s.span)),
            StmtKind::Return(..) => return Err(unsupported("return", s.span)),
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
            ExprKind::Fn(..) => return Err(unsupported("fn literals", e.span)),
        }
        Ok(())
    }

    fn patch(&mut self, at: usize, target: i32) {
        let rel = target - at as i32 - 1;
        match &mut self.chunk.code[at] {
            Op::Jump(o) | Op::JumpIfFalse(o) | Op::OrJump(o) | Op::AndJump(o) => *o = rel,
            _ => unreachable!("patched op is not a jump"),
        }
    }
}
