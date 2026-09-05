//! AST for ting. Every node carries the span of the source it came from.

use crate::lexer::Span;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// `let name = expr;` — defines (or shadows) in the current scope.
    Let(String, Expr),
    /// `name = expr;` — rebinds an existing variable.
    Assign(String, Expr),
    /// `base[index] = expr;` — writes into a list slot or map key.
    IndexAssign(Expr, Expr, Expr),
    /// Bare expression followed by `;`.
    Expr(Expr),
    /// `{ ... }` — introduces a scope.
    Block(Vec<Stmt>),
    /// `if cond { ... } else { ... }` — else branch is a Block or another If.
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    /// `while cond { ... }`
    While(Expr, Box<Stmt>),
    /// `for x in iterable { ... }` — iterates a snapshot of the iterable.
    For(String, Expr, Box<Stmt>),
    Break,
    Continue,
    /// `return;` or `return expr;`
    Return(Option<Expr>),
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            StmtKind::Let(name, e) => write!(f, "(let {name} {e})"),
            StmtKind::Assign(name, e) => write!(f, "(= {name} {e})"),
            StmtKind::IndexAssign(base, idx, e) => write!(f, "(=[] {base} {idx} {e})"),
            StmtKind::Expr(e) => write!(f, "{e}"),
            StmtKind::Block(stmts) => {
                f.write_str("(block")?;
                for s in stmts {
                    write!(f, " {s}")?;
                }
                f.write_str(")")
            }
            StmtKind::If(cond, then, None) => write!(f, "(if {cond} {then})"),
            StmtKind::If(cond, then, Some(els)) => write!(f, "(if {cond} {then} {els})"),
            StmtKind::While(cond, body) => write!(f, "(while {cond} {body})"),
            StmtKind::For(var, iterable, body) => write!(f, "(for {var} {iterable} {body})"),
            StmtKind::Break => f.write_str("(break)"),
            StmtKind::Continue => f.write_str("(continue)"),
            StmtKind::Return(None) => f.write_str("(return)"),
            StmtKind::Return(Some(e)) => write!(f, "(return {e})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// One parameter: a name, and the expression standing in for it when
/// the caller leaves it out. A default is evaluated at each call, in
/// the callee's own scope, so a later one may name an earlier
/// parameter and `fn f(xs = [])` gets a fresh list every time.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub default: Option<Expr>,
    /// The last parameter may be written `...name`, and then it binds
    /// a list of every argument the fixed parameters did not take.
    pub rest: bool,
}

impl std::fmt::Display for Param {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match (&self.default, self.rest) {
            (Some(e), _) => write!(f, "({} {e})", self.name),
            (None, true) => write!(f, "...{}", self.name),
            (None, false) => f.write_str(&self.name),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
    Var(String),
    List(Vec<Expr>),
    /// `{k: v, ...}` — keys must evaluate to strings at runtime.
    Map(Vec<(Expr, Expr)>),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    /// `fn(a, b) { ... }` — body is Rc-shared with the closures created
    /// from it, so evaluating the same literal twice doesn't clone it.
    Fn(Vec<Param>, Rc<Vec<Stmt>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
            UnaryOp::BitNot => "~",
        })
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
        })
    }
}

/// S-expression rendering, used by tests and the temporary AST dump in main.
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ExprKind::Int(n) => write!(f, "{n}"),
            ExprKind::Float(x) => write!(f, "{x:?}"),
            ExprKind::Str(s) => write!(f, "{s:?}"),
            ExprKind::Bool(b) => write!(f, "{b}"),
            ExprKind::Nil => f.write_str("nil"),
            ExprKind::Var(name) => f.write_str(name),
            ExprKind::List(items) => {
                f.write_str("(list")?;
                for it in items {
                    write!(f, " {it}")?;
                }
                f.write_str(")")
            }
            ExprKind::Map(entries) => {
                f.write_str("(map")?;
                for (k, v) in entries {
                    write!(f, " ({k} {v})")?;
                }
                f.write_str(")")
            }
            ExprKind::Unary(op, e) => write!(f, "({op} {e})"),
            ExprKind::Binary(op, l, r) => write!(f, "({op} {l} {r})"),
            ExprKind::Call(callee, args) => {
                write!(f, "(call {callee}")?;
                for a in args {
                    write!(f, " {a}")?;
                }
                f.write_str(")")
            }
            ExprKind::Index(base, idx) => write!(f, "(index {base} {idx})"),
            ExprKind::Fn(params, body) => {
                let params: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "(fn ({})", params.join(" "))?;
                for s in body.iter() {
                    write!(f, " {s}")?;
                }
                f.write_str(")")
            }
        }
    }
}
