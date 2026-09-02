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
    Find,
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
    Import,
    Format,
    JsonParse,
    JsonStr,
    Env,
    Exit,
    TimeMs,
}

impl Builtin {
    pub const ALL: [Builtin; 44] = [
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
        Builtin::Find,
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
        Builtin::Import,
        Builtin::Format,
        Builtin::JsonParse,
        Builtin::JsonStr,
        Builtin::Env,
        Builtin::Exit,
        Builtin::TimeMs,
    ];

    /// Signature and one-line summary, shown by the LSP on hover.
    pub fn doc(self) -> (&'static str, &'static str) {
        match self {
            Builtin::Print => (
                "print(...)",
                "Prints the arguments separated by spaces, then a newline; returns nil.",
            ),
            Builtin::Len => ("len(x)", "Length of a list, string (in chars), or map."),
            Builtin::Push => ("push(xs, v)", "Appends to a list in place; returns nil."),
            Builtin::Pop => (
                "pop(xs)",
                "Removes and returns the last element; empty list errors.",
            ),
            Builtin::Keys => ("keys(m)", "The map's keys as a sorted list."),
            Builtin::Has => ("has(m, k)", "Whether string key k is present in the map."),
            Builtin::Str => ("str(v)", "The value rendered as a string."),
            Builtin::Int => (
                "int(v)",
                "Converts int/float (truncates)/numeric string to int; else errors.",
            ),
            Builtin::Float => (
                "float(v)",
                "Converts int/float/numeric string to float; else errors.",
            ),
            Builtin::Type => ("type(v)", "The type name as a string, e.g. \"list\"."),
            Builtin::Range => (
                "range(hi) / range(lo, hi) / range(lo, hi, step)",
                "List of ints, half-open; step may be negative, never 0.",
            ),
            Builtin::Split => (
                "split(s, sep)",
                "List of pieces; empty separator splits into characters.",
            ),
            Builtin::Join => (
                "join(xs, sep)",
                "Joins a list of strings; non-string elements error.",
            ),
            Builtin::Trim => ("trim(s)", "The string without leading/trailing whitespace."),
            Builtin::Find => (
                "find(s, sub) / find(xs, v)",
                "Index of the first match (chars for strings), or nil.",
            ),
            Builtin::Contains => (
                "contains(s, sub) / contains(xs, v)",
                "Substring test, or list membership by structural equality.",
            ),
            Builtin::Replace => (
                "replace(s, from, to)",
                "All occurrences replaced; empty search string errors.",
            ),
            Builtin::StartsWith => ("starts_with(s, p)", "Whether s starts with prefix p."),
            Builtin::EndsWith => ("ends_with(s, p)", "Whether s ends with suffix p."),
            Builtin::Upper => ("upper(s)", "Unicode-aware uppercase."),
            Builtin::Lower => ("lower(s)", "Unicode-aware lowercase."),
            Builtin::Slice => (
                "slice(x, lo, hi)",
                "Sub-string (by chars) or fresh sub-list, half-open; negatives count from the end.",
            ),
            Builtin::Args => (
                "args()",
                "The command-line arguments after the script path.",
            ),
            Builtin::Input => (
                "input()",
                "One line from stdin without the newline; nil at end of input.",
            ),
            Builtin::ReadFile => (
                "read_file(path)",
                "The file's entire contents as a string; unreadable file errors.",
            ),
            Builtin::WriteFile => (
                "write_file(path, s)",
                "Writes (or overwrites) the file; returns nil.",
            ),
            Builtin::Sort => (
                "sort(xs)",
                "A fresh sorted list; all numbers or all strings, else error.",
            ),
            Builtin::SortBy => ("sort_by(xs, f)", "A fresh list sorted by key f(x), stable."),
            Builtin::Try => (
                "try(f)",
                "Calls f(); {\"ok\": result} on success, {\"err\": message} on a runtime error.",
            ),
            Builtin::Fail => (
                "fail(msg)",
                "Raises a runtime error with the given string message.",
            ),
            Builtin::Map => ("map(xs, f)", "A fresh list of f(x) for each element."),
            Builtin::Filter => (
                "filter(xs, f)",
                "A fresh list of the elements where f(x) is true (bool required).",
            ),
            Builtin::Reduce => ("reduce(xs, init, f)", "Folds left: f(f(init, x0), x1)..."),
            Builtin::Min => (
                "min(xs)",
                "Smallest element; sort's ordering rules; empty list errors.",
            ),
            Builtin::Max => (
                "max(xs)",
                "Largest element; sort's ordering rules; empty list errors.",
            ),
            Builtin::Abs => ("abs(n)", "Absolute value of an int or float."),
            Builtin::Assert => (
                "assert(cond) / assert(cond, msg)",
                "Errors unless cond is true (bool required).",
            ),
            Builtin::Import => (
                "import(path)",
                "Runs the file once and returns its top-level bindings as a map.",
            ),
            Builtin::Format => (
                "format(fmt, ...)",
                "Fills {} placeholders left-to-right; {{ and }} escape braces.",
            ),
            Builtin::JsonParse => (
                "json_parse(s)",
                "JSON text to ting values; malformed input errors with an offset.",
            ),
            Builtin::JsonStr => (
                "json_str(v)",
                "Ting value to compact JSON (map keys sorted).",
            ),
            Builtin::Env => (
                "env(name)",
                "The environment variable's value, or nil if unset.",
            ),
            Builtin::Exit => (
                "exit() / exit(code)",
                "Ends the program with that status (default 0); not catchable.",
            ),
            Builtin::TimeMs => ("time_ms()", "Milliseconds since the Unix epoch, as an int."),
        }
    }

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
            Builtin::Find => "find",
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
            Builtin::Import => "import",
            Builtin::Format => "format",
            Builtin::JsonParse => "json_parse",
            Builtin::JsonStr => "json_str",
            Builtin::Env => "env",
            Builtin::Exit => "exit",
            Builtin::TimeMs => "time_ms",
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
/// Int/Float compare numerically at every depth (1 == 1.0, and so
/// [1] == [1.0]), matching the documented `==` semantics.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => *a as f64 == *b,
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
