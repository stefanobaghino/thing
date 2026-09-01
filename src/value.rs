//! Runtime values for ting.

use crate::eval::Function;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
    List(Vec<Value>),
    Fn(Rc<Function>),
}

/// Structural equality, except functions: two closures are equal only if
/// they are the same closure.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Fn(a), Value::Fn(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Value {
    /// Type name used in error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::List(_) => "list",
            Value::Fn(_) => "function",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(x) => {
                if x.fract() == 0.0 && x.is_finite() {
                    write!(f, "{x:.1}")
                } else {
                    write!(f, "{x}")
                }
            }
            Value::Str(s) => f.write_str(s),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Nil => f.write_str("nil"),
            Value::List(items) => {
                f.write_str("[")?;
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    // Strings inside lists are quoted to keep output unambiguous.
                    match it {
                        Value::Str(s) => write!(f, "{s:?}")?,
                        other => write!(f, "{other}")?,
                    }
                }
                f.write_str("]")
            }
            Value::Fn(func) => write!(f, "<fn({})>", func.params.join(", ")),
        }
    }
}
