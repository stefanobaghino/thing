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
    Hex,
    Bin,
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
    ListDir,
    Exists,
    IsDir,
    MakeDir,
    RemoveFile,
    RemoveDir,
    Ord,
    Chr,
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
    SleepMs,
    Random,
    RandomInt,
    Seed,
    Run,
    EPrint,
    Cwd,
    ReTest,
    ReFind,
    ReFindAll,
    ReReplace,
    ReSplit,
}

impl Builtin {
    pub const ALL: [Builtin; 66] = [
        Builtin::Print,
        Builtin::Len,
        Builtin::Push,
        Builtin::Pop,
        Builtin::Keys,
        Builtin::Has,
        Builtin::Str,
        Builtin::Int,
        Builtin::Float,
        Builtin::Hex,
        Builtin::Bin,
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
        Builtin::ListDir,
        Builtin::Exists,
        Builtin::IsDir,
        Builtin::MakeDir,
        Builtin::RemoveFile,
        Builtin::RemoveDir,
        Builtin::Ord,
        Builtin::Chr,
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
        Builtin::SleepMs,
        Builtin::Random,
        Builtin::RandomInt,
        Builtin::Seed,
        Builtin::Run,
        Builtin::EPrint,
        Builtin::Cwd,
        Builtin::ReTest,
        Builtin::ReFind,
        Builtin::ReFindAll,
        Builtin::ReReplace,
        Builtin::ReSplit,
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
            Builtin::Hex => (
                "hex(n)",
                "An int as a hex literal, e.g. 0xff; negatives keep the sign.",
            ),
            Builtin::Bin => (
                "bin(n)",
                "An int as a binary literal, e.g. 0b1010; negatives keep the sign.",
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
                "The file's entire contents as a string (\"-\" reads stdin); unreadable file errors.",
            ),
            Builtin::WriteFile => (
                "write_file(path, s) / write_file(path, s, \"append\")",
                "Writes (or overwrites) the file; \"append\" adds to the end. Returns nil.",
            ),
            Builtin::ListDir => (
                "list_dir(path)",
                "The names in a directory, sorted; not a directory, or unreadable, errors.",
            ),
            Builtin::Exists => (
                "exists(path)",
                "Whether anything is at that path — a file, a directory, or something else.",
            ),
            Builtin::IsDir => (
                "is_dir(path)",
                "Whether the path is a directory; false if it is anything else or absent.",
            ),
            Builtin::MakeDir => (
                "make_dir(path)",
                "Creates the directory and any missing parents; already a directory is fine. Returns nil.",
            ),
            Builtin::RemoveFile => (
                "remove_file(path)",
                "Deletes the file; a path that is absent or is a directory errors. Returns nil.",
            ),
            Builtin::RemoveDir => (
                "remove_dir(path)",
                "Deletes an empty directory; one with anything in it errors. Returns nil.",
            ),
            Builtin::Ord => (
                "ord(s)",
                "The code point of a one-character string; any other length errors.",
            ),
            Builtin::Chr => (
                "chr(n)",
                "The one-character string at that code point; not a code point errors.",
            ),
            Builtin::Sort => (
                "sort(xs)",
                "A fresh sorted list; all numbers or all strings, else error.",
            ),
            Builtin::SortBy => ("sort_by(xs, f)", "A fresh list sorted by key f(x), stable."),
            Builtin::Try => (
                "try(f) / try(f, ...args)",
                "Calls f with the arguments that follow it; {\"ok\": result} on success, {\"err\": message} on a runtime error.",
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
            Builtin::SleepMs => (
                "sleep_ms(ms)",
                "Pauses for that many milliseconds; a negative count errors.",
            ),
            Builtin::Random => ("random()", "A float in [0, 1)."),
            Builtin::RandomInt => (
                "random_int(lo, hi)",
                "An int in [lo, hi), like range; an empty span errors.",
            ),
            Builtin::Seed => (
                "seed(n)",
                "Restarts the generator at n, so a run repeats exactly.",
            ),
            Builtin::Run => (
                "run(cmd) / run(cmd, args)",
                "Runs a program and waits: a map of code, out and err.",
            ),
            Builtin::EPrint => (
                "eprint(...)",
                "Prints to stderr, so data and diagnostics can part ways.",
            ),
            Builtin::Cwd => ("cwd()", "The working directory, as a string."),
            Builtin::ReTest => (
                "re_test(s, pattern)",
                "Whether the pattern matches anywhere in the string.",
            ),
            Builtin::ReFind => (
                "re_find(s, pattern)",
                "The leftmost match as a map, or nil; groups included.",
            ),
            Builtin::ReFindAll => (
                "re_find_all(s, pattern)",
                "Every non-overlapping match, left to right, as a list.",
            ),
            Builtin::ReReplace => (
                "re_replace(s, pattern, repl)",
                "Every match replaced; $1 to $9 name groups, $$ is a $.",
            ),
            Builtin::ReSplit => (
                "re_split(s, pattern)",
                "The string cut at every match, as a list of pieces.",
            ),
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
            Builtin::Hex => "hex",
            Builtin::Bin => "bin",
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
            Builtin::ListDir => "list_dir",
            Builtin::Exists => "exists",
            Builtin::IsDir => "is_dir",
            Builtin::MakeDir => "make_dir",
            Builtin::RemoveFile => "remove_file",
            Builtin::RemoveDir => "remove_dir",
            Builtin::Ord => "ord",
            Builtin::Chr => "chr",
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
            Builtin::SleepMs => "sleep_ms",
            Builtin::Random => "random",
            Builtin::RandomInt => "random_int",
            Builtin::Seed => "seed",
            Builtin::Run => "run",
            Builtin::EPrint => "eprint",
            Builtin::Cwd => "cwd",
            Builtin::ReTest => "re_test",
            Builtin::ReFind => "re_find",
            Builtin::ReFindAll => "re_find_all",
            Builtin::ReReplace => "re_replace",
            Builtin::ReSplit => "re_split",
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
            (Value::List(a), Value::List(b)) => {
                Rc::ptr_eq(a, b)
                    || with_comparing(
                        Rc::as_ptr(a) as *const (),
                        Rc::as_ptr(b) as *const (),
                        || *a.borrow() == *b.borrow(),
                    )
            }
            (Value::Map(a), Value::Map(b)) => {
                Rc::ptr_eq(a, b)
                    || with_comparing(
                        Rc::as_ptr(a) as *const (),
                        Rc::as_ptr(b) as *const (),
                        || *a.borrow() == *b.borrow(),
                    )
            }
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

thread_local! {
    /// The (left, right) container pairs currently being compared. A
    /// pair met again while it is still being compared is taken as
    /// equal: two structures that agree everywhere they are finite are
    /// equal, and the comparison terminates on cycles instead of
    /// overflowing the stack. Pointers only; never dereferenced.
    static COMPARING: RefCell<Vec<(*const (), *const ())>> = const { RefCell::new(Vec::new()) };
}

/// Compare with the pair marked as in progress; true at once when the
/// same pair is already being compared further up the stack.
fn with_comparing(a: *const (), b: *const (), body: impl FnOnce() -> bool) -> bool {
    let entered = COMPARING.with(|c| {
        let mut c = c.borrow_mut();
        if c.contains(&(a, b)) {
            false
        } else {
            c.push((a, b));
            true
        }
    });
    if !entered {
        return true;
    }
    let out = body();
    COMPARING.with(|c| {
        c.borrow_mut().pop();
    });
    out
}

thread_local! {
    /// The containers currently being printed, innermost last, so a
    /// container that contains itself (directly or through others)
    /// prints as `[...]` / `{...}` at the point of recursion instead of
    /// overflowing the stack. Pointers only; never dereferenced.
    static PRINTING: RefCell<Vec<*const ()>> = const { RefCell::new(Vec::new()) };
}

/// Run `body` with `ptr` marked as being printed; None (and no call)
/// when it already is — the caller prints the cycle marker instead.
fn with_printing<T>(ptr: *const (), body: impl FnOnce() -> T) -> Option<T> {
    let entered = PRINTING.with(|p| {
        let mut p = p.borrow_mut();
        if p.contains(&ptr) {
            false
        } else {
            p.push(ptr);
            true
        }
    });
    if !entered {
        return None;
    }
    let out = body();
    PRINTING.with(|p| {
        p.borrow_mut().pop();
    });
    Some(out)
}

/// Elements inside containers print with strings quoted, so nested
/// output stays unambiguous.
/// A value as it reads inside a container: a string keeps its quotes,
/// so `x = "x"` cannot be mistaken for a name. Diagnostics show
/// arguments this way for the same reason.
pub(crate) fn element_repr(v: &Value) -> String {
    match v {
        Value::Str(s) => format!("{s:?}"),
        other => other.to_string(),
    }
}

fn write_element(f: &mut fmt::Formatter<'_>, v: &Value) -> fmt::Result {
    match v {
        Value::Str(s) => write!(f, "{s:?}"),
        other => write!(f, "{other}"),
    }
}

/// A float as text that can be read back: the shortest form that
/// round-trips, an exponent where the plain form would be a wall of
/// digits (1e23 rather than 99999999999999991611392.0), and a `.0` on
/// an integral value so it stays visibly a float. json_str spells
/// floats this way too — every form here is also a ting literal and
/// valid JSON.
pub fn float_repr(x: f64) -> String {
    if !x.is_finite() {
        return x.to_string();
    }
    let magnitude = x.abs();
    if x != 0.0 && !(1e-4..1e17).contains(&magnitude) {
        return format!("{x:e}");
    }
    if x.fract() == 0.0 {
        format!("{x:.1}")
    } else {
        x.to_string()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(x) => f.write_str(&float_repr(*x)),
            Value::Str(s) => f.write_str(s),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Nil => f.write_str("nil"),
            Value::List(items) => {
                let ptr = Rc::as_ptr(items) as *const ();
                match with_printing(ptr, || {
                    f.write_str("[")?;
                    for (i, it) in items.borrow().iter().enumerate() {
                        if i > 0 {
                            f.write_str(", ")?;
                        }
                        write_element(f, it)?;
                    }
                    f.write_str("]")
                }) {
                    Some(r) => r,
                    None => f.write_str("[...]"),
                }
            }
            Value::Map(entries) => {
                let ptr = Rc::as_ptr(entries) as *const ();
                match with_printing(ptr, || {
                    f.write_str("{")?;
                    for (i, (k, v)) in entries.borrow().iter().enumerate() {
                        if i > 0 {
                            f.write_str(", ")?;
                        }
                        write!(f, "{k:?}: ")?;
                        write_element(f, v)?;
                    }
                    f.write_str("}")
                }) {
                    Some(r) => r,
                    None => f.write_str("{...}"),
                }
            }
            Value::Fn(func) => write!(f, "<fn({})>", func.params.join(", ")),
            Value::Builtin(b) => write!(f, "<builtin {}>", b.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_print_in_a_form_that_reads_back() {
        for (x, text) in [
            (1e23, "1e23"),
            (1e-7, "1e-7"),
            (0.1, "0.1"),
            (1.0, "1.0"),
            (-0.0, "-0.0"),
            (0.0, "0.0"),
            (2.5, "2.5"),
            (1e16, "10000000000000000.0"),
            (1e17, "1e17"),
            (1e-4, "0.0001"),
            (1e-5, "1e-5"),
            (0.1 + 0.2, "0.30000000000000004"),
        ] {
            assert_eq!(float_repr(x), text, "{x} printed wrong");
        }
        assert_eq!(float_repr(f64::INFINITY), "inf");
        assert_eq!(float_repr(f64::NAN), "NaN");
    }

    #[test]
    fn every_printed_float_lexes_back_to_itself() {
        for x in [
            1e23,
            1e-7,
            0.1,
            1.0,
            2.5,
            0.1 + 0.2,
            f64::MAX,
            f64::MIN_POSITIVE,
            1e300 * 10.0,
        ] {
            let text = float_repr(x);
            let tokens = crate::lexer::lex(&text).unwrap_or_else(|e| panic!("{text}: {e:?}"));
            match tokens[0].kind {
                crate::lexer::TokenKind::Float(back) => {
                    assert_eq!(back.to_bits(), x.to_bits(), "{text} read back as {back}")
                }
                ref other => panic!("{text} lexed as {other:?}, not a float"),
            }
        }
    }
}
