//! AST for ting. Every node carries the span of the source it came from.

use crate::lexer::Span;
use std::fmt;

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
    /// Bare expression followed by `;`.
    Expr(Expr),
    /// `{ ... }` — introduces a scope.
    Block(Vec<Stmt>),
    /// `if cond { ... } else { ... }` — else branch is a Block or another If.
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    /// `while cond { ... }`
    While(Expr, Box<Stmt>),
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            StmtKind::Let(name, e) => write!(f, "(let {name} {e})"),
            StmtKind::Assign(name, e) => write!(f, "(= {name} {e})"),
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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
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
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
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
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
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
        }
    }
}
