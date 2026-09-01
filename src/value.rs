//! Runtime values for ting.

use crate::eval::Function;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

/// Lists and maps have reference semantics (like Python/JS/Lua):
/// assigning or passing one shares the same underlying storage.
pub type ListRef = Rc<RefCell<Vec<Value>>>;
pub type MapRef = Rc<RefCell<BTreeMap<String, Value>>>;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
    List(ListRef),
    Map(MapRef),
    Fn(Rc<Function>),
    Builtin(Builtin),
}

/// Native functions, pre-bound in the global scope under their names
/// (shadowable like any variable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Print,
    Len,
    Push,
    Pop,
    Keys,
    Has,
    Str,
    Int,
    Float,
    Type,
    Range,
    Split,
    Join,
    Trim,
    Contains,
    Replace,
    StartsWith,
    EndsWith,
    Upper,
    Lower,
    Slice,
    Args,
    Input,
    ReadFile,
    WriteFile,
    Sort,
    SortBy,
    Try,
    Fail,
    Map,
    Filter,
    Reduce,
    Min,
    Max,
    Abs,
    Assert,
}

impl Builtin {
    pub const ALL: [Builtin; 36] = [
        Builtin::Print,
        Builtin::Len,
        Builtin::Push,
        Builtin::Pop,
        Builtin::Keys,
        Builtin::Has,
        Builtin::Str,
        Builtin::Int,
        Builtin::Float,
        Builtin::Type,
        Builtin::Range,
        Builtin::Split,
        Builtin::Join,
        Builtin::Trim,
        Builtin::Contains,
        Builtin::Replace,
        Builtin::StartsWith,
        Builtin::EndsWith,
        Builtin::Upper,
        Builtin::Lower,
        Builtin::Slice,
        Builtin::Args,
        Builtin::Input,
        Builtin::ReadFile,
        Builtin::WriteFile,
        Builtin::Sort,
        Builtin::SortBy,
        Builtin::Try,
        Builtin::Fail,
        Builtin::Map,
        Builtin::Filter,
        Builtin::Reduce,
        Builtin::Min,
        Builtin::Max,
        Builtin::Abs,
        Builtin::Assert,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Builtin::Print => "print",
            Builtin::Len => "len",
            Builtin::Push => "push",
            Builtin::Pop => "pop",
            Builtin::Keys => "keys",
            Builtin::Has => "has",
            Builtin::Str => "str",
            Builtin::Int => "int",
            Builtin::Float => "float",
            Builtin::Type => "type",
            Builtin::Range => "range",
            Builtin::Split => "split",
            Builtin::Join => "join",
            Builtin::Trim => "trim",
            Builtin::Contains => "contains",
            Builtin::Replace => "replace",
            Builtin::StartsWith => "starts_with",
            Builtin::EndsWith => "ends_with",
            Builtin::Upper => "upper",
            Builtin::Lower => "lower",
            Builtin::Slice => "slice",
            Builtin::Args => "args",
            Builtin::Input => "input",
            Builtin::ReadFile => "read_file",
            Builtin::WriteFile => "write_file",
            Builtin::Sort => "sort",
            Builtin::SortBy => "sort_by",
            Builtin::Try => "try",
            Builtin::Fail => "fail",
            Builtin::Map => "map",
            Builtin::Filter => "filter",
            Builtin::Reduce => "reduce",
            Builtin::Min => "min",
            Builtin::Max => "max",
            Builtin::Abs => "abs",
            Builtin::Assert => "assert",
        }
    }
}

impl Value {
    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(RefCell::new(items)))
    }

    pub fn map(entries: BTreeMap<String, Value>) -> Value {
        Value::Map(Rc::new(RefCell::new(entries)))
    }
}

/// Structural (deep) equality for data; identity for functions.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::List(a), Value::List(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Value::Map(a), Value::Map(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Value::Fn(a), Value::Fn(b)) => Rc::ptr_eq(a, b),
            (Value::Builtin(a), Value::Builtin(b)) => a == b,
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
            Value::Map(_) => "map",
            Value::Fn(_) | Value::Builtin(_) => "function",
        }
    }
}

/// Elements inside containers print with strings quoted, so nested
/// output stays unambiguous.
fn write_element(f: &mut fmt::Formatter<'_>, v: &Value) -> fmt::Result {
    match v {
        Value::Str(s) => write!(f, "{s:?}"),
        other => write!(f, "{other}"),
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
                for (i, it) in items.borrow().iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write_element(f, it)?;
                }
                f.write_str("]")
            }
            Value::Map(entries) => {
                f.write_str("{")?;
                for (i, (k, v)) in entries.borrow().iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{k:?}: ")?;
                    write_element(f, v)?;
                }
                f.write_str("}")
            }
            Value::Fn(func) => write!(f, "<fn({})>", func.params.join(", ")),
            Value::Builtin(b) => write!(f, "<builtin {}>", b.name()),
        }
    }
}
