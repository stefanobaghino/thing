//! Runtime values for ting.

use crate::eval::Function;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

/// Lists and maps have reference semantics (like Python/JS/Lua):
/// assigning or passing one shares the same underlying storage.
pub type ListRef = Rc<ListCell>;
pub type MapRef = Rc<MapCell>;

/// A list's storage. It exists as a type of its own only so that
/// dropping it can dismantle what it holds ITERATIVELY: values nest
/// as deep as a program builds them, and freeing `[[[[...]]]]` a
/// million levels deep by recursion killed the process on the way
/// out, after the program had finished (854).
pub struct ListCell(RefCell<Vec<Value>>);

/// A map's storage, for the same reason.
pub struct MapCell(RefCell<BTreeMap<String, Value>>);

impl ListCell {
    pub fn new(items: Vec<Value>) -> ListCell {
        ListCell(RefCell::new(items))
    }
}

impl MapCell {
    pub fn new(entries: BTreeMap<String, Value>) -> MapCell {
        MapCell(RefCell::new(entries))
    }
}

impl std::fmt::Debug for ListCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for MapCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for ListCell {
    type Target = RefCell<Vec<Value>>;
    fn deref(&self) -> &RefCell<Vec<Value>> {
        &self.0
    }
}

impl std::ops::Deref for MapCell {
    type Target = RefCell<BTreeMap<String, Value>>;
    fn deref(&self) -> &RefCell<BTreeMap<String, Value>> {
        &self.0
    }
}

/// How deep dropping may recurse before it starts using a worklist.
/// The ordinary path is the one the compiler writes — freeing a list
/// frees its elements, which frees theirs — and it is both the
/// fastest and the kindest to the allocator, which frees in the order
/// it allocated. It is also one host frame per level, so past this
/// many levels the two below take the contents apart iteratively
/// instead. A hundred frames of drop glue is a few kilobytes of
/// stack; a million is a dead process (858).
const DROP_RECURSION: usize = 100;

thread_local! {
    /// How many nested drops are on this thread's stack right now.
    static DROP_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl Drop for ListCell {
    fn drop(&mut self) {
        // Nothing nested: the ordinary drop that follows frees the
        // numbers and strings, and this costs one scan.
        if !self.0.get_mut().iter().any(nests) {
            return;
        }
        let mut items = std::mem::take(self.0.get_mut());
        DROP_DEPTH.with(|depth| {
            let at = depth.get();
            if at < DROP_RECURSION {
                depth.set(at + 1);
                drop(items);
                depth.set(at);
            } else {
                let mut todo = Vec::new();
                uproot_list(&mut items, &mut todo);
                while let Some(v) = todo.pop() {
                    uproot(v, &mut todo);
                }
            }
        });
    }
}

impl Drop for MapCell {
    fn drop(&mut self) {
        if !self.0.get_mut().values().any(nests) {
            return;
        }
        let mut entries = std::mem::take(self.0.get_mut());
        DROP_DEPTH.with(|depth| {
            let at = depth.get();
            if at < DROP_RECURSION {
                depth.set(at + 1);
                drop(entries);
                depth.set(at);
            } else {
                let mut todo = Vec::new();
                uproot_map(&mut entries, &mut todo);
                while let Some(v) = todo.pop() {
                    uproot(v, &mut todo);
                }
            }
        });
    }
}

/// Free one value without recursing into it. A container this holds
/// the LAST reference to has its own nested containers lifted out
/// before it goes out of scope, so the drop that follows finds
/// nothing to descend into; one that is still shared is simply
/// released, as it always was.
fn uproot(v: Value, todo: &mut Vec<Value>) {
    match v {
        Value::List(items) => {
            if let Some(mut cell) = Rc::into_inner(items) {
                uproot_list(cell.0.get_mut(), todo);
            }
        }
        Value::Map(entries) => {
            if let Some(mut cell) = Rc::into_inner(entries) {
                uproot_map(cell.0.get_mut(), todo);
            }
        }
        _ => {}
    }
}

/// Whether a value is a container, and so the reason any of this
/// exists: only these two nest.
fn nests(v: &Value) -> bool {
    matches!(v, Value::List(_) | Value::Map(_))
}

/// Take the nested containers out and leave everything else where it
/// is: numbers and strings are freed by the ordinary drop that
/// follows, and nothing is moved that does not have to be.
fn uproot_list(items: &mut [Value], todo: &mut Vec<Value>) {
    for slot in items {
        if nests(slot) {
            todo.push(std::mem::replace(slot, Value::Nil));
        }
    }
}

fn uproot_map(entries: &mut BTreeMap<String, Value>, todo: &mut Vec<Value>) {
    for slot in entries.values_mut() {
        if nests(slot) {
            todo.push(std::mem::replace(slot, Value::Nil));
        }
    }
}

/// A ting string.
///
/// Strings are immutable to a ting program, so two values holding the
/// same text can hold the same buffer: copying one is a pointer copy,
/// not a copy of the text. That is what makes reading a name cheap.
/// A string is still a VALUE — `a = b` then `b += "x"` must not
/// change `a` — so the one operation that writes, appending, copies
/// the text first UNLESS this is the only reference to it, the same
/// bargain `+` already strikes for lists.
#[derive(Clone, Default)]
pub struct Str(Rc<Repr>);

#[derive(Default)]
struct Repr {
    text: String,
    /// Characters, counted on the first ask and then remembered.
    /// `NOT_COUNTED` until someone asks. It lives beside the text and
    /// inside the `Rc`, so every name for this string gets the answer
    /// the first one paid for.
    chars: std::cell::Cell<usize>,
}

/// No character count has been asked for yet. A real count can never
/// be this, since it is at most the byte length.
const NOT_COUNTED: usize = usize::MAX;

impl Repr {
    fn new(text: String) -> Repr {
        Repr {
            text,
            chars: std::cell::Cell::new(NOT_COUNTED),
        }
    }

    /// A count already known, plus the characters in `added`. An
    /// append keeps a count it had rather than throwing it away, so
    /// building a string in a loop never re-walks what it built.
    fn grew_by(was: usize, added: &str) -> usize {
        if was == NOT_COUNTED {
            NOT_COUNTED
        } else {
            was + added.chars().count()
        }
    }
}

/// A string prints as its text does; the wrapper is not part of what
/// a ting value looks like (`["a"]`, never `[Str("a")]`).
impl fmt::Debug for Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl Str {
    pub fn as_str(&self) -> &str {
        &self.0.text
    }

    /// How many characters this string has. Counted once, however
    /// many times it is asked and by however many names.
    pub fn char_len(&self) -> usize {
        let known = self.0.chars.get();
        if known != NOT_COUNTED {
            return known;
        }
        let n = self.0.text.chars().count();
        self.0.chars.set(n);
        n
    }

    /// Whether every character is one byte, so the nth character
    /// starts at byte n and indexing need not walk. Asking counts the
    /// characters if nobody has yet, which is why it is worth asking
    /// once and reusing the answer.
    pub fn is_byte_indexed(&self) -> bool {
        self.char_len() == self.0.text.len()
    }

    /// Whether this string shares its buffer with no one, so writing
    /// to it costs the write rather than the whole text.
    pub fn is_unshared(&self) -> bool {
        Rc::strong_count(&self.0) == 1
    }

    /// Whether these two are the same string, not merely equal ones.
    pub fn is_same_buffer(&self, other: &Str) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::ops::Deref for Str {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0.text
    }
}

impl From<String> for Str {
    fn from(s: String) -> Str {
        Str(Rc::new(Repr::new(s)))
    }
}

impl From<&str> for Str {
    fn from(s: &str) -> Str {
        Str(Rc::new(Repr::new(s.to_string())))
    }
}

impl From<Str> for String {
    fn from(s: Str) -> String {
        match Rc::try_unwrap(s.0) {
            Ok(repr) => repr.text,
            Err(rc) => rc.text.clone(),
        }
    }
}

impl PartialEq for Str {
    fn eq(&self, other: &Str) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || self.as_str() == other.as_str()
    }
}

impl Eq for Str {}

impl PartialEq<str> for Str {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Str {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialOrd for Str {
    fn partial_cmp(&self, other: &Str) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Str {
    fn cmp(&self, other: &Str) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl std::hash::Hash for Str {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl fmt::Display for Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::borrow::Borrow<str> for Str {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for Str {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<std::ffi::OsStr> for Str {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.as_str().as_ref()
    }
}

impl AsRef<std::path::Path> for Str {
    fn as_ref(&self) -> &std::path::Path {
        self.as_str().as_ref()
    }
}

impl AsRef<[u8]> for Str {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl FromIterator<char> for Str {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Str {
        Str::from(iter.into_iter().collect::<String>())
    }
}

impl<'a> FromIterator<&'a char> for Str {
    fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> Str {
        Str::from(iter.into_iter().copied().collect::<String>())
    }
}

impl std::ops::Add<&str> for Str {
    type Output = Str;
    /// Appending copies the text unless this is the only reference to
    /// it, in which case it extends the buffer already there.
    fn add(mut self, rhs: &str) -> Str {
        let was = self.0.chars.get();
        match Rc::get_mut(&mut self.0) {
            Some(repr) => {
                repr.text.push_str(rhs);
                repr.chars.set(Repr::grew_by(was, rhs));
                self
            }
            None => {
                let mut text = String::with_capacity(self.0.text.len() + rhs.len());
                text.push_str(&self.0.text);
                text.push_str(rhs);
                let repr = Repr::new(text);
                repr.chars.set(Repr::grew_by(was, rhs));
                Str(Rc::new(repr))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(Str),
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
    Values,
    Items,
    Has,
    Get,
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
    EachLine,
    ListDir,
    Exists,
    IsDir,
    Stat,
    MakeDir,
    RemoveFile,
    RemoveDir,
    Rename,
    CopyFile,
    Ord,
    Chr,
    Sort,
    SortBy,
    SortWith,
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
    MonoMs,
    LocalZone,
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
    Fingerprint,
    Width,
    Compare,
}

impl Builtin {
    pub const ALL: [Builtin; 79] = [
        Builtin::Print,
        Builtin::Len,
        Builtin::Push,
        Builtin::Pop,
        Builtin::Keys,
        Builtin::Values,
        Builtin::Items,
        Builtin::Has,
        Builtin::Get,
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
        Builtin::EachLine,
        Builtin::ListDir,
        Builtin::Exists,
        Builtin::IsDir,
        Builtin::Stat,
        Builtin::MakeDir,
        Builtin::RemoveFile,
        Builtin::RemoveDir,
        Builtin::Rename,
        Builtin::CopyFile,
        Builtin::Ord,
        Builtin::Chr,
        Builtin::Sort,
        Builtin::SortBy,
        Builtin::SortWith,
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
        Builtin::MonoMs,
        Builtin::LocalZone,
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
        Builtin::Fingerprint,
        Builtin::Width,
        Builtin::Compare,
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
                "pop(xs) / pop(m, k)",
                "Removes and returns the last element of a list, or the value at key k of a map, deleting it from the map in place; an empty list, or a key the map does not have, errors.",
            ),
            Builtin::Keys => ("keys(m)", "The map's keys as a sorted list."),
            Builtin::Values => (
                "values(m)",
                "The map's values in sorted key order, matching keys() and items().",
            ),
            Builtin::Items => (
                "items(m)",
                "The map's entries as [key, value] pairs in sorted key order — what a `for [k, v] in items(m)` loop walks.",
            ),
            Builtin::Has => ("has(m, k)", "Whether string key k is present in the map."),
            Builtin::Get => (
                "get(x, k, default)",
                "x[k] where it is present, otherwise default; never errors on absence.",
            ),
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
                "input() / input(\"lossy\")",
                "One line from stdin without the newline; nil at end of input. A line that is not UTF-8 fails, naming the byte; \"lossy\" reads it anyway, with a replacement character where each bad byte was.",
            ),
            Builtin::ReadFile => (
                "read_file(path) / read_file(path, \"lossy\")",
                "The file's entire contents as a string (\"-\" reads stdin); unreadable file errors, and so does one that is not UTF-8, naming the byte and the line it falls in. \"lossy\" reads it anyway, with a replacement character where each bad byte was — what run() has always done with a child's output.",
            ),
            Builtin::WriteFile => (
                "write_file(path, s) / write_file(path, s, \"append\")",
                "Writes (or overwrites) the file; \"append\" adds to the end. Returns nil.",
            ),
            Builtin::EachLine => (
                "each_line(path, f) / each_line(path, f, \"lossy\")",
                "Reads the file one line at a time, calling f(line) for each — newline removed, CRLF too, as input() does. Only the current line is held, so a file larger than memory still reads. \"-\" is stdin. Returning false from f stops the read. Answers how many lines f was given. A line that is not UTF-8 fails, naming the byte and which line it was; \"lossy\" hands the line over with a replacement character where each bad byte was.",
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
            Builtin::Stat => (
                "stat(path)",
                "What a file is besides its name: a map of size (bytes), modified (ms since the epoch, the clock time_ms() reads) and kind (\"file\", \"dir\" or \"other\"). nil when nothing readable is there.",
            ),
            Builtin::Rename => (
                "rename(from, to)",
                "Moves a file or directory by giving it another name. Nothing is copied, so the size does not matter and the modification time comes through untouched; an existing target is replaced. Errors when the two paths are on different filesystems.",
            ),
            Builtin::CopyFile => (
                "copy_file(from, to)",
                "Copies a file's bytes, whatever they are, without holding them in memory, and gives the copy the original's permission bits and modification time. An existing target is overwritten; a directory, or a target that is the same file as the source, errors. Returns nil.",
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
            Builtin::SortWith => (
                "sort_with(xs, cmp)",
                "A fresh list sorted by a three-way comparator: cmp(a, b) is negative when a comes first, positive when b does, 0 for ties, which keep their input order.",
            ),
            Builtin::Try => (
                "try(f) / try(f, ...args)",
                "Calls f with the arguments that follow it; {\"ok\": result} on success, and on a runtime error {\"err\": message, \"at\": a map of file, line and col, \"trace\": the calls it came out of, innermost first}.",
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
                "Errors unless cond is true (bool required); a refused comparison shows both sides.",
            ),
            Builtin::Import => (
                "import(path)",
                "Runs the file once and returns its top-level bindings as a map.",
            ),
            Builtin::Format => (
                "format(fmt, ...)",
                "Fills {} placeholders left-to-right; {{ and }} escape braces. A \
                 placeholder may carry a spec after a colon: width and \
                 alignment ({:>5} right, {:<5} left, {:^5} centred, {:0>2} \
                 filled with any character) and decimal places ({:.2}, \
                 {:>8.2}). A zero in front of the width fills with zeroes past \
                 the sign ({:05} of -42 is \"-0042\"). A width or a number of \
                 places written {} is read from the arguments, after the \
                 value: format(\"{:<{}}\", name, width).",
            ),
            Builtin::JsonParse => (
                "json_parse(s)",
                "JSON text to ting values; malformed input errors with an offset, and so does a document nested deeper than 1000. A byte order mark at the head of the document is skipped, since another program may have written one.",
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
            Builtin::MonoMs => (
                "mono_ms()",
                "Milliseconds since this process started, as a float, from a clock that only moves forward — what to subtract to learn how long something took. time_ms() reads the wall clock, which can step backwards.",
            ),
            Builtin::LocalZone => (
                "local_zone() / local_zone(ms)",
                "The local zone at that instant (now by default): a map of offset (milliseconds east of UTC), abbr and dst. Read from the TZif file on Unix and from the system zone data on Windows, where abbr is the full name Windows uses. Nil where the platform keeps nothing to read, so a script can tell that from a real zero.",
            ),
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
                "run(cmd) / run(cmd, args) / run(cmd, args, stdin) / run(cmd, args, options)",
                "Runs a program and waits: a map of code, out, err and signal. \
                 A signal killed it when code is nil, and signal is that \
                 number where the platform has them. The child reads stdin \
                 there, and reads nothing without it. A map in that place \
                 is options instead — {\"stdin\": text, \"dir\": path}, the \
                 directory being where the child runs; an option nothing \
                 knows is an error. Bytes that are not UTF-8 come back as \
                 replacement characters, where reading a file would be an \
                 error.",
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
                "The leftmost match as a map of start, end, text and groups (a list, nil where a group took no part), or nil; positions count characters, as find and slice do.",
            ),
            Builtin::ReFindAll => (
                "re_find_all(s, pattern)",
                "Every non-overlapping match, left to right, as a list of the maps re_find gives.",
            ),
            Builtin::ReReplace => (
                "re_replace(s, pattern, repl)",
                "Every match replaced; $1 to $9 name groups, $$ is a $.",
            ),
            Builtin::ReSplit => (
                "re_split(s, pattern)",
                "The string cut at every match, as a list of pieces.",
            ),
            Builtin::Width => (
                "display_width(s)",
                "How many terminal columns the string takes: an East Asian wide or fullwidth character counts two, a combining mark or a control none, everything else one — what len counts in characters and a terminal counts in columns.",
            ),
            Builtin::Compare => (
                "compare(a, b)",
                "Which of two values comes first in ting's order: -1 for a, 1 for b, 0 for a tie — the three-way answer < gives one bit of, so a sort_with comparator over several fields is one line each. nil when a NaN leaves the pair unordered; the error < gives where the two have no order between them.",
            ),
            Builtin::Fingerprint => (
                "fingerprint(v)",
                "A string two values share exactly when == says they are equal, so a map can stand in for a scan; nil where equality cannot be a key: a function (compared by identity), a NaN (equal to nothing), a number past 2^53 (where int and float equality stops being transitive), or a value that contains itself.",
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
            Builtin::Values => "values",
            Builtin::Items => "items",
            Builtin::Has => "has",
            Builtin::Get => "get",
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
            Builtin::EachLine => "each_line",
            Builtin::ListDir => "list_dir",
            Builtin::Exists => "exists",
            Builtin::IsDir => "is_dir",
            Builtin::Stat => "stat",
            Builtin::MakeDir => "make_dir",
            Builtin::RemoveFile => "remove_file",
            Builtin::RemoveDir => "remove_dir",
            Builtin::Rename => "rename",
            Builtin::CopyFile => "copy_file",
            Builtin::Ord => "ord",
            Builtin::Chr => "chr",
            Builtin::Sort => "sort",
            Builtin::SortBy => "sort_by",
            Builtin::SortWith => "sort_with",
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
            Builtin::MonoMs => "mono_ms",
            Builtin::LocalZone => "local_zone",
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
            Builtin::Fingerprint => "fingerprint",
            Builtin::Width => "display_width",
            Builtin::Compare => "compare",
        }
    }
}

impl Value {
    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(ListCell::new(items)))
    }

    pub fn map(entries: BTreeMap<String, Value>) -> Value {
        Value::Map(Rc::new(MapCell::new(entries)))
    }

    pub fn str(text: impl Into<Str>) -> Value {
        Value::Str(text.into())
    }

    /// Whether these two values are backed by the same storage, so
    /// that letting go of one leaves the other holding it alone.
    /// Equality is not enough: two strings that read the same may be
    /// two strings.
    pub fn shares_storage(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Str(a), Value::Str(b)) => a.is_same_buffer(b),
            (Value::List(a), Value::List(b)) => Rc::ptr_eq(a, b),
            (Value::Map(a), Value::Map(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
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

/// How many levels of containers printing follows. Past this the
/// marker that already means "there is more here" stands in for the
/// rest, exactly as it does for a cycle — because the alternatives
/// are worse: the walker recurses per level, and a list nested deep
/// enough KILLED the process (854 measured the cliff between 100000
/// and 200000). It also made printing quadratic, since the check for
/// a cycle scanned the whole path per container: 51 ms at 10000 deep
/// and 4492 at 100000, for output nobody reads.
///
/// A thousand is what json_parse follows too. Data this deep is a
/// data structure rather than a document, and its shape is not what
/// printing it is for.
pub const MAX_PRINT_DEPTH: usize = 1000;

/// Run `body` with `ptr` marked as being printed; None (and no call)
/// when it already is, or when the path is as deep as printing goes —
/// the caller prints the marker instead.
fn with_printing<T>(ptr: *const (), body: impl FnOnce() -> T) -> Option<T> {
    let entered = PRINTING.with(|p| {
        let mut p = p.borrow_mut();
        if p.len() >= MAX_PRINT_DEPTH || p.contains(&ptr) {
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

    /// A value deeper than printing follows ends in the marker that
    /// already means "there is more here", rather than in a stack
    /// overflow. 854 measured the old walker dying between 100000 and
    /// 200000 levels, and taking 4.5 seconds at 100000 on the way.
    #[test]
    fn printing_stops_at_a_stated_depth_rather_than_at_the_cliff() {
        fn nest(levels: usize) -> Value {
            let mut v = Value::list(Vec::new());
            for _ in 0..levels {
                v = Value::list(vec![v]);
            }
            v
        }
        // One container per level plus the empty one at the middle:
        // a value with exactly MAX_PRINT_DEPTH containers is shown
        // whole.
        let whole = nest(MAX_PRINT_DEPTH - 1).to_string();
        assert!(!whole.contains("[...]"), "nothing to elide yet");
        assert_eq!(whole.len(), 2 * MAX_PRINT_DEPTH);

        let deeper = nest(MAX_PRINT_DEPTH).to_string();
        assert!(deeper.contains("[...]"), "the marker says it stopped");
        assert_eq!(deeper.len(), 2 * MAX_PRINT_DEPTH + 5);

        // Far past the cliff, and the answer is the same size.
        let far = nest(50_000).to_string();
        assert_eq!(far.len(), 2 * MAX_PRINT_DEPTH + 5);
    }

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
