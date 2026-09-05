//! Tree-walking interpreter for ting.

use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::lexer::Span;
use crate::value::{Builtin, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeError {
    pub message: String,
    pub span: Span,
    /// The module the span belongs to, when the error escaped a
    /// function defined in an imported file; None means the span is
    /// in the source being run.
    pub origin: Option<Rc<Origin>>,
    /// The calls the error came out of, innermost first: one frame per
    /// function it unwound through, pushed as it went.
    pub frames: Vec<Frame>,
}

/// One call an error passed through on its way out: the function it
/// was raised inside (None when that function is anonymous), the span
/// of the call that entered it, and the origin the *caller* belongs
/// to (None for the source being run), which is the file the span is
/// to be read against.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub name: Option<Rc<str>>,
    pub span: Span,
    pub origin: Option<Rc<Origin>>,
}

/// A file a function was defined in: what an error raised inside it
/// renders against.
#[derive(Debug, PartialEq)]
pub struct Origin {
    pub path: String,
    pub src: Rc<str>,
}

/// Which offsets of which files ran. Offsets rather than lines: a
/// line number costs a scan of the source, and the run is the hot
/// part — the report can pay for it once, at the end.
#[derive(Debug, Default)]
pub struct Coverage {
    /// Keyed by the file's identity — the origin's address, or 0 for
    /// the script itself, which has no origin.
    files: HashMap<usize, FileCoverage>,
}

/// One file's share of a run.
#[derive(Debug)]
pub struct FileCoverage {
    pub path: String,
    pub src: Rc<str>,
    /// Where every statement in the file starts: what could run.
    pub coverable: std::collections::HashSet<usize>,
    /// Where the statements that did run start.
    pub hit: std::collections::HashSet<usize>,
}

impl Coverage {
    /// Every file the run touched, ordered by path so a report reads
    /// the same way twice.
    pub fn files(&self) -> Vec<&FileCoverage> {
        let mut out: Vec<&FileCoverage> = self.files.values().collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }
}

impl RuntimeError {
    /// The diagnostic, rendered against the origin's file when the
    /// error came out of an imported module and against `path`/`src`
    /// otherwise.
    pub fn render(&self, path: &str, src: &str) -> String {
        let mut text = match &self.origin {
            Some(o) => crate::diag::render(&o.path, &o.src, &self.message, self.span),
            None => crate::diag::render(path, src, &self.message, self.span),
        };
        let n = self.frames.len();
        for (i, frame) in self.frames.iter().enumerate() {
            // A deep trace (runaway recursion, most often) is elided in
            // the middle: the innermost calls say what failed and the
            // outermost say who started it, and the count in between
            // says how much was left out.
            if n > TRACE_LIMIT && i == TRACE_EDGE {
                let hidden = n - 2 * TRACE_EDGE;
                text.push_str(&format!("\nnote: ... {hidden} more frames"));
            }
            if n > TRACE_LIMIT && i >= TRACE_EDGE && i < n - TRACE_EDGE {
                continue;
            }
            let (p, s): (&str, &str) = match &frame.origin {
                Some(o) => (&o.path, &o.src),
                None => (path, src),
            };
            let (line, col) = frame.span.line_col(s);
            let what = match &frame.name {
                Some(name) => format!("in {name}"),
                None => "in an anonymous function".to_string(),
            };
            text.push_str(&format!("\nnote: {what}, called from {p}:{line}:{col}"));
        }
        text
    }
}

/// Traces longer than this are elided in the middle, keeping
/// `TRACE_EDGE` frames at each end.
const TRACE_LIMIT: usize = 10;
const TRACE_EDGE: usize = 4;

/// The values a `...xs` argument contributes. Both engines call this,
/// so the refusal reads the same either way.
pub(crate) fn spread_values(v: Value, span: Span) -> Result<Vec<Value>, RuntimeError> {
    match v {
        Value::List(items) => Ok(items.borrow().clone()),
        other => Err(error(
            format!("cannot spread {}, expected a list", other.type_name()),
            span,
        )),
    }
}

/// Where every statement starts, nested blocks and function bodies
/// included: exactly the statements `exec` runs and the compiler puts
/// a `Mark` in front of, so what could run and what did are counted
/// against the same set.
fn statement_offsets(stmts: &[Stmt], out: &mut std::collections::HashSet<usize>) {
    fn expr(e: &Expr, out: &mut std::collections::HashSet<usize>) {
        match &e.kind {
            ExprKind::Fn(_, body) => statement_offsets(body, out),
            ExprKind::List(xs) => xs.iter().for_each(|x| expr(x, out)),
            ExprKind::Map(kvs) => kvs.iter().for_each(|(k, v)| {
                expr(k, out);
                expr(v, out);
            }),
            ExprKind::Unary(_, a) | ExprKind::Spread(a) => expr(a, out),
            ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
                expr(a, out);
                expr(b, out);
            }
            ExprKind::Call(callee, args) => {
                expr(callee, out);
                args.iter().for_each(|a| expr(a, out));
            }
            _ => {}
        }
    }
    for s in stmts {
        out.insert(s.span.start);
        match &s.kind {
            StmtKind::Let(_, e) | StmtKind::Assign(_, e) | StmtKind::Expr(e) => expr(e, out),
            StmtKind::IndexAssign(a, b, c) => {
                expr(a, out);
                expr(b, out);
                expr(c, out);
            }
            StmtKind::Block(body) => statement_offsets(body, out),
            StmtKind::If(cond, then, els) => {
                expr(cond, out);
                statement_offsets(std::slice::from_ref(then), out);
                if let Some(e) = els {
                    statement_offsets(std::slice::from_ref(e), out);
                }
            }
            StmtKind::While(cond, body) => {
                expr(cond, out);
                statement_offsets(std::slice::from_ref(body), out);
            }
            StmtKind::For(_, iterable, body) => {
                expr(iterable, out);
                statement_offsets(std::slice::from_ref(body), out);
            }
            StmtKind::Return(Some(e)) => expr(e, out),
            StmtKind::Break | StmtKind::Continue | StmtKind::Return(None) => {}
        }
    }
}

/// The line each offset falls on, 1-based, computed once per file:
/// asking a span for its line scans the source from the start, which
/// is fine for one diagnostic and not for a few thousand offsets.
fn lines_of(src: &str, offsets: &std::collections::HashSet<usize>) -> Vec<usize> {
    let mut starts = vec![0usize];
    starts.extend(
        src.char_indices()
            .filter(|(_, c)| *c == '\n')
            .map(|(i, _)| i + 1),
    );
    let mut lines: Vec<usize> = offsets
        .iter()
        .map(|o| starts.partition_point(|start| start <= o))
        .collect();
    lines.sort_unstable();
    lines.dedup();
    lines
}

pub(crate) fn error(message: impl Into<String>, span: Span) -> RuntimeError {
    RuntimeError {
        message: message.into(),
        span,
        origin: None,
        frames: Vec::new(),
    }
}

/// A lexical environment: closures keep their defining Env alive via Rc,
/// and RefCell lets assignments through a closure be seen everywhere.
#[derive(Debug)]
pub struct Env {
    vars: HashMap<Rc<str>, Value>,
    parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    fn child(parent: &Rc<RefCell<Env>>) -> Rc<RefCell<Env>> {
        Rc::new(RefCell::new(Env {
            vars: HashMap::new(),
            parent: Some(Rc::clone(parent)),
        }))
    }

    /// Every name visible from here outwards, innermost first.
    fn names(env: &Rc<RefCell<Env>>, out: &mut Vec<String>) {
        let e = env.borrow();
        out.extend(e.vars.keys().map(|k| k.to_string()));
        if let Some(p) = e.parent.as_ref() {
            Env::names(p, out);
        }
    }

    fn get(env: &Rc<RefCell<Env>>, name: &str) -> Option<Value> {
        let e = env.borrow();
        match e.vars.get(name) {
            Some(v) => Some(v.clone()),
            None => e.parent.as_ref().and_then(|p| Env::get(p, name)),
        }
    }

    /// Rebind the nearest existing binding; false if none exists.
    fn assign(env: &Rc<RefCell<Env>>, name: &str, v: Value) -> bool {
        let mut e = env.borrow_mut();
        if let Some(slot) = e.vars.get_mut(name) {
            *slot = v;
            true
        } else {
            match &e.parent {
                Some(p) => Env::assign(&Rc::clone(p), name, v),
                None => false,
            }
        }
    }
}

/// A user-defined function: parameters, body, and the captured
/// environment. The body is AST when the tree-walker created the
/// closure, bytecode when the VM did — either engine can call either.
#[derive(Debug)]
pub struct Function {
    /// The name it was defined under, for traces; None when the
    /// function is an anonymous literal.
    pub name: Option<Rc<str>>,
    /// Where it was defined, for a profile to point at a line.
    pub def: Span,
    pub params: Vec<Rc<str>>,
    /// One entry per parameter: the expression standing in for it when
    /// the caller leaves it out, evaluated at each call.
    pub defaults: Rc<Vec<Option<crate::ast::Expr>>>,
    /// True when the last parameter was written `...name`: it takes a
    /// list of whatever the fixed parameters left over, so there is no
    /// upper bound on how many arguments the function accepts.
    pub rest: bool,
    pub body: FnBody,
    pub env: Rc<RefCell<Env>>,
    /// The imported file this function was defined in, if any.
    pub origin: Option<Rc<Origin>>,
}

#[derive(Debug)]
pub enum FnBody {
    Ast(Rc<Vec<Stmt>>),
    Chunk(Rc<crate::compile::Chunk>),
}

enum ControlOrValue {
    Control(Control),
    Value(Option<Value>),
}

/// How a statement finished: fell through, or hit a non-local exit.
enum Control {
    Normal,
    Return(Value, Span),
    Break(Span),
    Continue(Span),
}

/// What one ting call costs in host stack, measured on this project's
/// own engines by reading the address of a frame-local at successive
/// depths: about 1.6 KB on the VM and 2.6 KB on the tree-walker in
/// release, and roughly eleven times that unoptimized. The budget
/// below rounds the worse engine up, so both engines get the same cap
/// and stay byte-identical about where they refuse.
const FRAME_COST: usize = if cfg!(debug_assertions) {
    32 * 1024
} else {
    4 * 1024
};

/// The cap when nobody has said how much stack there is: an embedder
/// on an unknown thread, or the wasm build, which has no thread of its
/// own to size. Deliberately small — it is the number that has to be
/// safe everywhere.
const DEFAULT_MAX_DEPTH: usize = 200;

/// The stack the interpreter has been told it may use, in bytes; 0
/// means nobody said. A process that gives the interpreter a thread of
/// a known size (the runner and the REPL both spawn one) says so here,
/// and every Interpreter made afterwards derives its call-depth cap
/// from it.
static STACK_BUDGET: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Declare the host stack the interpreter may use. Call before making
/// an Interpreter; half the budget is left for everything that is not
/// call frames.
pub fn set_stack_budget(bytes: usize) {
    STACK_BUDGET.store(bytes, std::sync::atomic::Ordering::Relaxed);
}

/// The call-depth cap this process allows: derived from the declared
/// stack, never below the default.
pub fn max_depth() -> usize {
    match STACK_BUDGET.load(std::sync::atomic::Ordering::Relaxed) {
        0 => DEFAULT_MAX_DEPTH,
        bytes => (bytes / 2 / FRAME_COST).max(DEFAULT_MAX_DEPTH),
    }
}

pub struct Interpreter<W: Write> {
    env: Rc<RefCell<Env>>,
    out: W,
    depth: usize,
    /// This interpreter's cap, fixed when it was made, so a running
    /// program cannot have the limit moved under it.
    max_depth: usize,
    script_args: Vec<String>,
    /// Directory import paths resolve against; the top is the directory
    /// of the file currently executing (script, or module mid-import).
    dir_stack: Vec<std::path::PathBuf>,
    import_cache: HashMap<std::path::PathBuf, Value>,
    importing: Vec<std::path::PathBuf>,
    /// The module whose top level is currently running, if any.
    origin_stack: Vec<Rc<Origin>>,
    /// The origin of each function on the call stack (None for one
    /// defined in the source being run), innermost last.
    call_origins: Vec<Option<Rc<Origin>>>,
    /// The file being run: what a span with no origin is read
    /// against when a program asks where something failed.
    source: Option<(String, Rc<str>)>,
    /// Per-function counts, when `--profile` asked for them. Keyed by
    /// the function's defining file and the offset it starts at, which
    /// is one entry per `fn` in the program however many closures it
    /// makes.
    profile: Option<HashMap<(usize, usize), Counted>>,
    /// Which offsets ran, when `--coverage` asked for them.
    coverage: Option<Coverage>,
    /// Time charged to the callees of each active call, innermost
    /// last: what a frame subtracts from its own elapsed time.
    child_ns: Vec<u128>,
    /// Patterns already compiled, keyed by their source text: a match
    /// inside a loop compiles once. Cleared wholesale when it grows
    /// past its cap, since a program that generates patterns should
    /// not be able to grow the interpreter without bound.
    patterns: HashMap<String, Rc<crate::regex::Regex>>,
    /// The random generator's state, None until something asks for a
    /// number: a program that never rolls a die never reads the clock,
    /// and one that calls seed() first is reproducible.
    rng: Option<u64>,
}

/// Where an unseeded generator starts. The clock is the only entropy
/// a zero-dependency binary has; wasm has no clock, so a page reloads
/// into the same sequence unless the program calls seed().
fn default_seed() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        0x853C_49E6_748F_EA9B
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x853C_49E6_748F_EA9B)
    }
}

/// A match as ting sees it: where it sat, the text it covered, and
/// one entry per capturing group — the group's text, or nil where the
/// group took no part in the match.
fn match_value(chars: &[char], caps: &[Option<usize>], groups: usize) -> Value {
    let slice = |lo: usize, hi: usize| -> String { chars[lo..hi].iter().collect() };
    let mut m = std::collections::BTreeMap::new();
    let (start, end) = (caps[0].unwrap_or(0), caps[1].unwrap_or(0));
    m.insert("start".to_string(), Value::Int(start as i64));
    m.insert("end".to_string(), Value::Int(end as i64));
    m.insert("text".to_string(), Value::Str(slice(start, end)));
    let mut list = Vec::new();
    for g in 1..=groups {
        match (
            caps.get(g * 2).copied().flatten(),
            caps.get(g * 2 + 1).copied().flatten(),
        ) {
            (Some(lo), Some(hi)) => list.push(Value::Str(slice(lo, hi))),
            _ => list.push(Value::Nil),
        }
    }
    m.insert("groups".to_string(), Value::list(list));
    Value::map(m)
}

/// A replacement string with its `$` references filled in: `$0` is
/// the whole match, `$1` to `$9` its groups, `$$` a literal dollar. A
/// reference to a group the pattern does not have is an error rather
/// than an empty string, since it can only be a mistake.
fn expand(
    repl: &str,
    chars: &[char],
    caps: &[Option<usize>],
    groups: usize,
    span: Span,
) -> Result<String, RuntimeError> {
    let mut out = String::new();
    let mut rest = repl.chars().peekable();
    while let Some(c) = rest.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        match rest.peek() {
            Some('$') => {
                rest.next();
                out.push('$');
            }
            Some(d) if d.is_ascii_digit() => {
                let n = d.to_digit(10).unwrap() as usize;
                rest.next();
                if n > groups {
                    return Err(error(
                        format!("re_replace: the pattern has no group {n}"),
                        span,
                    ));
                }
                if let (Some(lo), Some(hi)) = (
                    caps.get(n * 2).copied().flatten(),
                    caps.get(n * 2 + 1).copied().flatten(),
                ) {
                    out.extend(&chars[lo..hi]);
                }
            }
            _ => out.push('$'),
        }
    }
    Ok(out)
}

/// Every non-overlapping match, left to right. An empty match cannot
/// advance the search on its own, so the scan steps past it by one
/// character: without that, `re_find_all(s, "x*")` would never end.
fn all_matches(re: &crate::regex::Regex, chars: &[char]) -> Vec<Vec<Option<usize>>> {
    let mut out = Vec::new();
    let mut from = 0;
    while from <= chars.len() {
        let Some(caps) = re.find_at(chars, from) else {
            break;
        };
        let (start, end) = (caps[0].unwrap_or(from), caps[1].unwrap_or(from));
        out.push(caps);
        from = if end > start { end } else { start + 1 };
    }
    out
}

/// One row of a profile: a function, and how much it did.
#[derive(Debug)]
struct Counted {
    name: Option<Rc<str>>,
    origin: Option<Rc<Origin>>,
    def: Span,
    /// A native function, which has no file to point at.
    builtin: bool,
    calls: u64,
    /// Nanoseconds spent inside this function and not inside anything
    /// it called. Self time rather than total, so a recursive
    /// function is not credited with the same span once per level.
    self_ns: u128,
}

/// How many rows a profile prints before it starts counting the rest.
const PROFILE_ROWS: usize = 20;

/// How many missed lines a file's row names before it just counts the
/// rest: enough to act on, not so many that the table stops being one.
const MISSED_LINES: usize = 12;

/// A whole-number percentage, and 100 only when nothing was missed.
fn percent(part: usize, whole: usize) -> usize {
    if whole == 0 {
        return 100;
    }
    part * 100 / whole
}

/// Nanoseconds as milliseconds, three decimals: one unit for the
/// whole table, so a column can be compared by eye.
fn millis(ns: u128) -> String {
    format!("{:.3}ms", ns as f64 / 1_000_000.0)
}

/// The standard library, baked into the binary at build time (always
/// in sync with lib/ by construction). import() falls back to these
/// when no matching file exists.
const EMBEDDED_STDLIB: &[(&str, &str)] = &[
    ("lib/args.ting", include_str!("../lib/args.ting")),
    ("lib/csv.ting", include_str!("../lib/csv.ting")),
    ("lib/err.ting", include_str!("../lib/err.ting")),
    ("lib/fs.ting", include_str!("../lib/fs.ting")),
    ("lib/json.ting", include_str!("../lib/json.ting")),
    ("lib/list.ting", include_str!("../lib/list.ting")),
    ("lib/map.ting", include_str!("../lib/map.ting")),
    ("lib/math.ting", include_str!("../lib/math.ting")),
    ("lib/sh.ting", include_str!("../lib/sh.ting")),
    ("lib/string.ting", include_str!("../lib/string.ting")),
    ("lib/test.ting", include_str!("../lib/test.ting")),
    ("lib/time.ting", include_str!("../lib/time.ting")),
];

/// The embedded stdlib as (path, source) pairs, for tooling that wants
/// to know what `import("lib/...")` will resolve to.
pub fn embedded_stdlib() -> &'static [(&'static str, &'static str)] {
    EMBEDDED_STDLIB
}

/// Checks run in this process: every `assert` call, and every
/// `lib/test.ting` helper (which calls `assert(true)` to say so).
/// `--test` asks the child for this count; nothing else reads it.
static CHECKS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub(crate) fn count_check() {
    CHECKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// How many checks this process has run.
pub fn checks_run() -> usize {
    CHECKS.load(std::sync::atomic::Ordering::Relaxed)
}

fn global_env() -> Rc<RefCell<Env>> {
    let mut globals = HashMap::new();
    for b in Builtin::ALL {
        globals.insert(Rc::from(b.name()), Value::Builtin(b));
    }
    Rc::new(RefCell::new(Env {
        vars: globals,
        parent: None,
    }))
}

impl<W: Write> Interpreter<W> {
    pub fn new(out: W) -> Self {
        Interpreter {
            env: global_env(),
            out,
            depth: 0,
            max_depth: max_depth(),
            script_args: Vec::new(),
            dir_stack: vec![std::path::PathBuf::new()],
            import_cache: HashMap::new(),
            importing: Vec::new(),
            origin_stack: Vec::new(),
            call_origins: Vec::new(),
            source: None,
            profile: None,
            coverage: None,
            child_ns: Vec::new(),
            patterns: HashMap::new(),
            rng: None,
        }
    }

    /// The compiled form of a pattern, compiling it if this is the
    /// first time it has been seen.
    fn pattern(
        &mut self,
        b: Builtin,
        text: &str,
        span: Span,
    ) -> Result<Rc<crate::regex::Regex>, RuntimeError> {
        if let Some(re) = self.patterns.get(text) {
            return Ok(re.clone());
        }
        let re = crate::regex::Regex::new(text)
            .map_err(|e| error(format!("{}: {e}", b.name()), span))?;
        let re = Rc::new(re);
        const MAX_PATTERNS: usize = 256;
        if self.patterns.len() >= MAX_PATTERNS {
            self.patterns.clear();
        }
        self.patterns.insert(text.to_string(), re.clone());
        Ok(re)
    }

    /// The two string arguments every pattern builtin starts with.
    fn subject_and_pattern(
        b: Builtin,
        args: &[Value],
        span: Span,
    ) -> Result<(String, String), RuntimeError> {
        match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(p)) => Ok((s.clone(), p.clone())),
            (l, r) => Err(error(
                format!(
                    "{} expects two strings, got {} and {}",
                    b.name(),
                    l.type_name(),
                    r.type_name()
                ),
                span,
            )),
        }
    }

    /// SplitMix64: one multiply-xor-shift finalizer over a counter.
    /// Small enough to read, good enough that a program cannot see
    /// the pattern, and identical on both engines because both call
    /// this.
    fn next_u64(&mut self) -> u64 {
        let state = self.rng.get_or_insert_with(default_seed);
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Command-line arguments exposed to the script via `args()`.
    /// User-defined names in the top-level environment (builtins
    /// excluded), sorted — the REPL's :vars view.
    pub fn user_bindings(&self) -> Vec<(String, String)> {
        let env = self.env.borrow();
        let mut out: Vec<(String, String)> = env
            .vars
            .iter()
            .filter(|(_, v)| !matches!(v, Value::Builtin(_)))
            .map(|(k, v)| (k.to_string(), v.type_name().to_string()))
            .collect();
        out.sort();
        out
    }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.script_args = args;
    }

    /// The path and text of the source being run, so `try` can say
    /// where a failure happened in it. Unset, locations in it are
    /// reported as line 1 of an unnamed file.
    pub fn set_source(&mut self, path: &str, src: &str) {
        self.source = Some((path.to_string(), Rc::from(src)));
    }

    /// The origin a closure created right now belongs to: the file of
    /// the function being executed, or — at a top level — the module
    /// being imported, if any. A closure made inside a module's
    /// function belongs to that module, not to whoever called it.
    pub(crate) fn defining_origin(&self) -> Option<Rc<Origin>> {
        match self.call_origins.last() {
            Some(o) => o.clone(),
            None => self.current_origin(),
        }
    }

    /// A function's row in the profile: its defining file and where
    /// in it the definition starts, which is one row per `fn` however
    /// many closures it makes.
    fn profile_key(func: &Rc<Function>) -> (usize, usize) {
        (
            func.origin.as_ref().map_or(0, |o| Rc::as_ptr(o) as usize),
            func.def.start,
        )
    }

    /// A builtin's row: keyed apart from every ting function, since
    /// it has no defining file.
    fn builtin_key(b: Builtin) -> (usize, usize) {
        (usize::MAX, b as usize)
    }

    /// Charge `elapsed` minus what its callees took to the row `key`
    /// names, and the whole of it to whoever called it.
    fn charge(&mut self, key: (usize, usize), elapsed: u128) {
        let children = self.child_ns.pop().unwrap_or(0);
        if let Some(parent) = self.child_ns.last_mut() {
            *parent += elapsed;
        }
        if let Some(row) = self.profile.as_mut().and_then(|p| p.get_mut(&key)) {
            row.self_ns += elapsed.saturating_sub(children);
        }
    }

    /// One more call of the builtin `b`, in the profile.
    fn count_builtin(&mut self, b: Builtin) {
        let counts = self.profile.as_mut().expect("profiling is on");
        counts
            .entry(Self::builtin_key(b))
            .or_insert_with(|| Counted {
                name: Some(Rc::from(b.name())),
                origin: None,
                def: Span::new(0, 0),
                builtin: true,
                calls: 0,
                self_ns: 0,
            })
            .calls += 1;
    }

    /// One more call of `func`, in the profile being collected.
    fn count_call(&mut self, func: &Rc<Function>) {
        let key = Self::profile_key(func);
        let counts = self.profile.as_mut().expect("profiling is on");
        counts
            .entry(key)
            .or_insert_with(|| Counted {
                name: func.name.clone(),
                origin: func.origin.clone(),
                def: func.def,
                builtin: false,
                calls: 0,
                self_ns: 0,
            })
            .calls += 1;
    }

    /// Count what every function does from here on; the counts come
    /// back from `profile_report`.
    pub fn profile(&mut self) {
        self.profile = Some(HashMap::new());
    }

    /// Start recording which lines run. Off by default: the recording
    /// costs a hash insert per statement, which a plain run should not
    /// pay for.
    pub fn cover(&mut self) {
        self.coverage = Some(Coverage::default());
    }

    /// What ran, when recording was asked for.
    pub fn coverage(&self) -> Option<&Coverage> {
        self.coverage.as_ref()
    }

    /// Note that the statement starting at `offset` ran, against
    /// whichever file the code currently running was defined in.
    pub(crate) fn record(&mut self, offset: usize) {
        if self.coverage.is_none() {
            return;
        }
        self.file_coverage().hit.insert(offset);
    }

    /// Note where every statement of `stmts` starts, so the report can
    /// say what did not run as well as what did. Called once per file,
    /// with that file's origin in place.
    pub(crate) fn note_coverable(&mut self, stmts: &[Stmt]) {
        if self.coverage.is_none() {
            return;
        }
        let mut offsets = std::collections::HashSet::new();
        statement_offsets(stmts, &mut offsets);
        self.file_coverage().coverable.extend(offsets);
    }

    /// The current file's row, created on first use. The file is the
    /// one the running code was defined in, the same answer a trace
    /// gives.
    fn file_coverage(&mut self) -> &mut FileCoverage {
        let origin = self.defining_origin();
        let key = origin.as_ref().map_or(0, |o| Rc::as_ptr(o) as usize);
        let source = self.source.clone();
        let coverage = self.coverage.as_mut().expect("recording is on");
        coverage.files.entry(key).or_insert_with(|| {
            let (path, src) = match &origin {
                Some(o) => (o.path.clone(), Rc::clone(&o.src)),
                None => match &source {
                    Some((p, s)) => (p.clone(), Rc::clone(s)),
                    None => (String::new(), Rc::from("")),
                },
            };
            FileCoverage {
                path,
                src,
                coverable: Default::default(),
                hit: Default::default(),
            }
        })
    }

    /// The profile as a table, busiest function first: how often each
    /// ran and where it was defined. None when counting was never
    /// asked for.
    /// What ran, as a table: one row per file, the share of its
    /// statements reached, and the lines of those that were not. None
    /// when recording was never asked for.
    pub fn coverage_report(&self) -> Option<String> {
        let coverage = self.coverage.as_ref()?;
        let files = coverage.files();
        let mut rows = Vec::new();
        let (mut total, mut total_hit) = (0usize, 0usize);
        for f in &files {
            let coverable = lines_of(&f.src, &f.coverable);
            let hit = lines_of(&f.src, &f.hit);
            let missed: Vec<usize> = coverable
                .iter()
                .copied()
                .filter(|l| !hit.contains(l))
                .collect();
            total += coverable.len();
            total_hit += coverable.len() - missed.len();
            rows.push((f.path.clone(), coverable.len(), missed));
        }
        let mut out = format!(
            "coverage: {} of {} ({}%)\n",
            total_hit,
            crate::diag::plural(total, "line"),
            percent(total_hit, total)
        );
        for (path, lines, missed) in rows {
            let reached = lines - missed.len();
            out.push_str(&format!(
                "{:>4}%  {:>5}/{:<5}  {}",
                percent(reached, lines),
                reached,
                lines,
                path
            ));
            if !missed.is_empty() {
                let shown: Vec<String> = missed
                    .iter()
                    .take(MISSED_LINES)
                    .map(|l| l.to_string())
                    .collect();
                out.push_str(&format!("  missed {}", shown.join(", ")));
                let hidden = missed.len().saturating_sub(MISSED_LINES);
                if hidden > 0 {
                    out.push_str(&format!(" and {hidden} more"));
                }
            }
            out.push('\n');
        }
        Some(out)
    }

    pub fn profile_report(&self) -> Option<String> {
        let counts = self.profile.as_ref()?;
        let mut rows: Vec<&Counted> = counts.values().collect();
        // Slowest first: the reason to read a profile at all. Ties go
        // to the busier function, then to where it sits, so running a
        // program twice reports it the same way twice.
        rows.sort_by(|a, b| {
            b.self_ns
                .cmp(&a.self_ns)
                .then_with(|| b.calls.cmp(&a.calls))
                .then_with(|| self.where_defined(a).cmp(&self.where_defined(b)))
        });
        let calls: u64 = rows.iter().map(|r| r.calls).sum();
        let total: u128 = rows.iter().map(|r| r.self_ns).sum();
        let mut out = format!(
            "profile: {}, {}, {} in them\n",
            crate::diag::plural(rows.len(), "function"),
            crate::diag::plural(calls as usize, "call"),
            millis(total)
        );
        out.push_str(&format!(
            "{:>10}  {:>10}  {:<24}  {}\n",
            "calls", "self", "function", "where"
        ));
        // A long table buries its own headline, so only the busiest
        // rows are printed and the rest are counted.
        let hidden = rows.len().saturating_sub(PROFILE_ROWS);
        for r in rows.into_iter().take(PROFILE_ROWS) {
            let name = match &r.name {
                Some(n) => n.to_string(),
                None => "an anonymous function".to_string(),
            };
            out.push_str(&format!(
                "{:>10}  {:>10}  {:<24}  {}\n",
                r.calls,
                millis(r.self_ns),
                name,
                self.where_defined(r)
            ));
        }
        if hidden > 0 {
            out.push_str(&format!(
                "... {}\n",
                crate::diag::plural(hidden, "more function")
            ));
        }
        Some(out)
    }

    /// "file.ting:12:1" for a profiled function, and a plain word for
    /// a builtin, which is defined in Rust rather than in ting.
    fn where_defined(&self, r: &Counted) -> String {
        if r.builtin {
            return "a builtin".to_string();
        }
        let (path, src): (&str, &str) = match &r.origin {
            Some(o) => (&o.path, &o.src),
            None => match &self.source {
                Some((p, s)) => (p, s),
                None => ("", ""),
            },
        };
        let (line, col) = r.def.line_col(src);
        format!("{path}:{line}:{col}")
    }

    /// A span as the entries of a ting map: the file it belongs to,
    /// and the line and column in that file.
    fn site(
        &self,
        span: Span,
        origin: &Option<Rc<Origin>>,
    ) -> std::collections::BTreeMap<String, Value> {
        let (path, src): (&str, &str) = match origin {
            Some(o) => (&o.path, &o.src),
            None => match &self.source {
                Some((p, s)) => (p, s),
                None => ("", ""),
            },
        };
        let (line, col) = span.line_col(src);
        let mut m = std::collections::BTreeMap::new();
        m.insert("file".to_string(), Value::Str(path.to_string()));
        m.insert("line".to_string(), Value::Int(line as i64));
        m.insert("col".to_string(), Value::Int(col as i64));
        m
    }

    /// Define a name in the current (global, for the VM) scope.
    pub(crate) fn define(&mut self, name: &str, v: Value) {
        self.env.borrow_mut().vars.insert(Rc::from(name), v);
    }

    /// The current environment handle (VM closure capture).
    pub(crate) fn env_handle(&self) -> Rc<RefCell<Env>> {
        Rc::clone(&self.env)
    }

    /// Enter a fresh lexical scope (VM block/loop-iteration support).
    pub(crate) fn push_scope(&mut self) {
        self.env = Env::child(&self.env);
    }

    /// Leave the current scope, restoring its parent.
    pub(crate) fn pop_scope(&mut self) {
        let parent = self
            .env
            .borrow()
            .parent
            .clone()
            .expect("pop_scope at global scope");
        self.env = parent;
    }

    /// Rebind an existing name; false if it doesn't exist.
    pub(crate) fn assign(&mut self, name: &str, v: Value) -> bool {
        Env::assign(&self.env, name, v)
    }

    /// Directory that relative import() paths resolve against.
    pub fn set_base_dir(&mut self, dir: std::path::PathBuf) {
        self.dir_stack[0] = dir;
    }

    /// The current base directory (see set_base_dir), so a caller can
    /// change it for a while and put it back.
    pub fn base_dir(&self) -> std::path::PathBuf {
        self.dir_stack[0].clone()
    }

    /// Consume the interpreter, handing back its output writer (used by
    /// tests to inspect what a session printed).
    #[cfg(test)]
    pub fn into_out(self) -> W {
        self.out
    }

    pub fn run(&mut self, stmts: &[Stmt]) -> Result<(), RuntimeError> {
        match self.run_block(stmts)? {
            Control::Normal => Ok(()),
            Control::Return(_, span) => Err(error("return outside function", span)),
            Control::Break(span) => Err(error("break outside loop", span)),
            Control::Continue(span) => Err(error("continue outside loop", span)),
        }
    }

    fn run_block(&mut self, stmts: &[Stmt]) -> Result<Control, RuntimeError> {
        for s in stmts {
            match self.exec(s)? {
                Control::Normal => {}
                ret => return Ok(ret),
            }
        }
        Ok(Control::Normal)
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<Control, RuntimeError> {
        self.record(stmt.span.start);
        match &stmt.kind {
            StmtKind::Let(name, init) => {
                // `fn f(..) {..}` parses as a let of a fn literal, so
                // this is where a function learns its name.
                let v = match &init.kind {
                    ExprKind::Fn(params, body) => {
                        self.make_fn(params, body, Some(name.as_str()), init.span)
                    }
                    _ => self.eval(init)?,
                };
                self.env
                    .borrow_mut()
                    .vars
                    .insert(Rc::from(name.as_str()), v);
                Ok(Control::Normal)
            }
            StmtKind::Assign(name, value) => {
                let v = self.eval(value)?;
                if Env::assign(&self.env, name, v) {
                    Ok(Control::Normal)
                } else {
                    Err(self.undefined_assign(name, stmt.span))
                }
            }
            StmtKind::IndexAssign(base, idx, value) => {
                let b = self.eval(base)?;
                let i = self.eval(idx)?;
                let v = self.eval(value)?;
                match (b, i) {
                    (Value::List(items), Value::Int(n)) => {
                        let mut items = items.borrow_mut();
                        let eff = effective_index(n, items.len(), stmt.span)?;
                        items[eff] = v;
                        Ok(Control::Normal)
                    }
                    (Value::Map(entries), Value::Str(k)) => {
                        entries.borrow_mut().insert(k, v);
                        Ok(Control::Normal)
                    }
                    (b, i) => Err(error(
                        format!(
                            "cannot index-assign {} with {}",
                            b.type_name(),
                            i.type_name()
                        ),
                        stmt.span,
                    )),
                }
            }
            StmtKind::Expr(e) => {
                self.eval(e)?;
                Ok(Control::Normal)
            }
            StmtKind::Block(stmts) => {
                // Only a direct `let` (fn decls desugar to let) can bind
                // into this block's scope; without one, a child env is
                // pure allocation overhead — if/while bodies hit this on
                // every entry (see bench/).
                if stmts.iter().any(|s| matches!(s.kind, StmtKind::Let(..))) {
                    let saved = Rc::clone(&self.env);
                    self.env = Env::child(&saved);
                    let result = self.run_block(stmts);
                    self.env = saved;
                    result
                } else {
                    self.run_block(stmts)
                }
            }
            StmtKind::If(cond, then, els) => {
                if as_bool(self.eval(cond)?, cond.span)? {
                    self.exec(then)
                } else if let Some(els) = els {
                    self.exec(els)
                } else {
                    Ok(Control::Normal)
                }
            }
            StmtKind::While(cond, body) => {
                while as_bool(self.eval(cond)?, cond.span)? {
                    match self.exec(body)? {
                        Control::Normal | Control::Continue(_) => {}
                        Control::Break(_) => break,
                        ret @ Control::Return(..) => return Ok(ret),
                    }
                }
                Ok(Control::Normal)
            }
            StmtKind::For(var, iterable, body) => {
                let items: Vec<Value> = match self.eval(iterable)? {
                    // Iterate a snapshot, so the body may mutate the
                    // original list/map safely.
                    Value::List(l) => l.borrow().clone(),
                    Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    Value::Map(m) => m.borrow().keys().cloned().map(Value::Str).collect(),
                    v => {
                        return Err(error(
                            format!("cannot iterate over {}", v.type_name()),
                            iterable.span,
                        ));
                    }
                };
                for item in items {
                    // A fresh scope per iteration: closures made in the
                    // body capture that iteration's binding.
                    let saved = Rc::clone(&self.env);
                    self.env = Env::child(&saved);
                    self.env
                        .borrow_mut()
                        .vars
                        .insert(Rc::from(var.as_str()), item);
                    let result = self.exec(body);
                    self.env = saved;
                    match result? {
                        Control::Normal | Control::Continue(_) => {}
                        Control::Break(_) => break,
                        ret @ Control::Return(..) => return Ok(ret),
                    }
                }
                Ok(Control::Normal)
            }
            StmtKind::Break => Ok(Control::Break(stmt.span)),
            StmtKind::Continue => Ok(Control::Continue(stmt.span)),
            StmtKind::Return(value) => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Nil,
                };
                Ok(Control::Return(v, stmt.span))
            }
        }
    }

    pub(crate) fn lookup(&self, name: &str) -> Option<Value> {
        Env::get(&self.env, name)
    }

    /// A closure over the current environment, named when it is being
    /// bound to a name and anonymous otherwise.
    pub(crate) fn make_fn(
        &self,
        params: &[crate::ast::Param],
        body: &Rc<Vec<Stmt>>,
        name: Option<&str>,
        def: Span,
    ) -> Value {
        Value::Fn(Rc::new(Function {
            name: name.map(Rc::from),
            def,
            params: params.iter().map(|p| Rc::from(p.name.as_str())).collect(),
            defaults: Rc::new(params.iter().map(|p| p.default.clone()).collect()),
            rest: params.last().is_some_and(|p| p.rest),
            body: FnBody::Ast(Rc::clone(body)),
            env: Rc::clone(&self.env),
            origin: self.defining_origin(),
        }))
    }

    /// "undefined variable 'cont'", plus the nearest name in scope
    /// (a binding or a builtin) when there is one worth naming.
    pub(crate) fn undefined(&self, name: &str, span: Span) -> RuntimeError {
        self.undefined_msg("undefined variable", name, span)
    }

    /// The same for an assignment to a name that was never bound.
    pub(crate) fn undefined_assign(&self, name: &str, span: Span) -> RuntimeError {
        self.undefined_msg("cannot assign to undefined variable", name, span)
    }

    fn undefined_msg(&self, what: &str, name: &str, span: Span) -> RuntimeError {
        let mut names = Vec::new();
        Env::names(&self.env, &mut names);
        match crate::diag::nearest(name, names.iter().map(String::as_str)) {
            Some(near) => error(format!("{what} '{name}' (did you mean '{near}'?)"), span),
            None => error(format!("{what} '{name}'"), span),
        }
    }

    /// Builtins are in the profile too: a program can spend its time
    /// inside `json_parse` as easily as inside its own loops. The
    /// clock only runs when profiling is on.
    fn call_builtin(
        &mut self,
        b: Builtin,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if self.profile.is_none() {
            return self.call_builtin_inner(b, args, span);
        }
        self.count_builtin(b);
        self.child_ns.push(0);
        let started = std::time::Instant::now();
        let out = self.call_builtin_inner(b, args, span);
        self.charge(Self::builtin_key(b), started.elapsed().as_nanos());
        out
    }

    fn call_builtin_inner(
        &mut self,
        b: Builtin,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let arity = |lo: usize, hi: usize| -> Result<(), RuntimeError> {
            if args.len() < lo || args.len() > hi {
                let want = if lo == hi {
                    crate::diag::plural(lo, "argument")
                } else {
                    format!("{lo} to {hi} arguments")
                };
                Err(error(
                    format!("{} expects {want}, got {}", b.name(), args.len()),
                    span,
                ))
            } else {
                Ok(())
            }
        };
        match b {
            Builtin::Print => {
                let parts: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                if let Err(e) = writeln!(self.out, "{}", parts.join(" ")) {
                    // A reader that went away (`ting x.ting | head`) is
                    // not the script's fault: leave quietly, as CLI
                    // filters conventionally do, instead of reporting
                    // "print failed: Broken pipe" on every such run.
                    if e.kind() == std::io::ErrorKind::BrokenPipe && !cfg!(target_arch = "wasm32") {
                        std::process::exit(0);
                    }
                    return Err(error(format!("print failed: {e}"), span));
                }
                Ok(Value::Nil)
            }
            Builtin::Len => {
                arity(1, 1)?;
                match &args[0] {
                    Value::List(items) => Ok(Value::Int(items.borrow().len() as i64)),
                    Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                    Value::Map(entries) => Ok(Value::Int(entries.borrow().len() as i64)),
                    v => Err(error(
                        format!("len does not apply to {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Push => {
                arity(2, 2)?;
                match &args[0] {
                    Value::List(items) => {
                        items.borrow_mut().push(args[1].clone());
                        Ok(Value::Nil)
                    }
                    v => Err(error(
                        format!("push expects a list, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Pop => {
                arity(1, 1)?;
                match &args[0] {
                    Value::List(items) => items
                        .borrow_mut()
                        .pop()
                        .ok_or_else(|| error("pop from empty list", span)),
                    v => Err(error(
                        format!("pop expects a list, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Keys => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Map(entries) => Ok(Value::list(
                        entries.borrow().keys().cloned().map(Value::Str).collect(),
                    )),
                    v => Err(error(
                        format!("keys expects a map, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Has => {
                arity(2, 2)?;
                match (&args[0], &args[1]) {
                    (Value::Map(entries), Value::Str(k)) => {
                        Ok(Value::Bool(entries.borrow().contains_key(k)))
                    }
                    (v, k) => Err(error(
                        format!(
                            "has expects a map and a string key, got {} and {}",
                            v.type_name(),
                            k.type_name()
                        ),
                        span,
                    )),
                }
            }
            Builtin::Str => {
                arity(1, 1)?;
                Ok(Value::Str(args[0].to_string()))
            }
            Builtin::Int => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(*n)),
                    // Anything a float can hold that an int cannot is an
                    // error, not a saturated i64: this language raises on
                    // integer overflow everywhere else.
                    Value::Float(x) => {
                        const LIMIT: f64 = 9_223_372_036_854_775_808.0; // 2^63
                        if x.is_finite() && (-LIMIT..LIMIT).contains(x) {
                            Ok(Value::Int(*x as i64))
                        } else {
                            Err(error(
                                format!("cannot convert {} to int", crate::value::float_repr(*x)),
                                span,
                            ))
                        }
                    }
                    // A string is read the way a literal is: the same
                    // radix prefixes, the same separator rule. int(hex(n))
                    // is n for every int.
                    Value::Str(s) => int_from_text(s)
                        .map(Value::Int)
                        .ok_or_else(|| error(format!("cannot convert {s:?} to int"), span)),
                    v => Err(error(
                        format!("cannot convert {} to int", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Float => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Int(n) => Ok(Value::Float(*n as f64)),
                    Value::Float(x) => Ok(Value::Float(*x)),
                    // The lexer refuses a literal that is not finite, so
                    // the conversion refuses the same text: "1e400",
                    // "inf" and "nan" are all errors here.
                    Value::Str(s) => s
                        .trim()
                        .parse::<f64>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .map(Value::Float)
                        .ok_or_else(|| error(format!("cannot convert {s:?} to float"), span)),
                    v => Err(error(
                        format!("cannot convert {} to float", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Hex | Builtin::Bin => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Int(n) => {
                        let sign = if *n < 0 { "-" } else { "" };
                        let magnitude = n.unsigned_abs();
                        Ok(Value::Str(match b {
                            Builtin::Hex => format!("{sign}0x{magnitude:x}"),
                            _ => format!("{sign}0b{magnitude:b}"),
                        }))
                    }
                    v => Err(error(
                        format!("{} needs an int, got {}", b.name(), v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Type => {
                arity(1, 1)?;
                Ok(Value::Str(args[0].type_name().to_string()))
            }
            Builtin::Range => {
                arity(1, 3)?;
                let (lo, hi, step) = match args.as_slice() {
                    [Value::Int(hi)] => (0, *hi, 1),
                    [Value::Int(lo), Value::Int(hi)] => (*lo, *hi, 1),
                    [Value::Int(lo), Value::Int(hi), Value::Int(step)] => (*lo, *hi, *step),
                    _ => {
                        return Err(error("range expects int arguments", span));
                    }
                };
                if step == 0 {
                    return Err(error("range step must not be 0", span));
                }
                let mut out = Vec::new();
                let mut i = lo;
                while if step > 0 { i < hi } else { i > hi } {
                    out.push(Value::Int(i));
                    i += step;
                }
                Ok(Value::list(out))
            }
            Builtin::Split => {
                arity(2, 2)?;
                match (&args[0], &args[1]) {
                    // Empty separator splits into single-character strings.
                    (Value::Str(s), Value::Str(sep)) if sep.is_empty() => Ok(Value::list(
                        s.chars().map(|c| Value::Str(c.to_string())).collect(),
                    )),
                    (Value::Str(s), Value::Str(sep)) => Ok(Value::list(
                        s.split(sep.as_str())
                            .map(|p| Value::Str(p.to_string()))
                            .collect(),
                    )),
                    (a, b) => Err(error(
                        format!(
                            "split expects two strings, got {} and {}",
                            a.type_name(),
                            b.type_name()
                        ),
                        span,
                    )),
                }
            }
            Builtin::Join => {
                arity(2, 2)?;
                match (&args[0], &args[1]) {
                    (Value::List(items), Value::Str(sep)) => {
                        let items = items.borrow();
                        let mut parts = Vec::with_capacity(items.len());
                        for it in items.iter() {
                            match it {
                                Value::Str(s) => parts.push(s.clone()),
                                v => {
                                    return Err(error(
                                        format!(
                                            "join expects a list of strings, found {}",
                                            v.type_name()
                                        ),
                                        span,
                                    ));
                                }
                            }
                        }
                        Ok(Value::Str(parts.join(sep)))
                    }
                    (a, b) => Err(error(
                        format!(
                            "join expects a list and a string, got {} and {}",
                            a.type_name(),
                            b.type_name()
                        ),
                        span,
                    )),
                }
            }
            Builtin::Trim => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
                    v => Err(error(
                        format!("trim expects a string, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Contains => {
                arity(2, 2)?;
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(sub)) => Ok(Value::Bool(s.contains(sub.as_str()))),
                    (Value::Str(_), v) => Err(error(
                        format!(
                            "contains on a string expects a string, got {}",
                            v.type_name()
                        ),
                        span,
                    )),
                    (Value::List(items), v) => {
                        Ok(Value::Bool(items.borrow().iter().any(|it| it == v)))
                    }
                    (a, _) => Err(error(
                        format!("contains expects a string or list, got {}", a.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Find => {
                arity(2, 2)?;
                match (&args[0], &args[1]) {
                    // Char index, consistent with slice()'s char addressing.
                    (Value::Str(hay), Value::Str(needle)) => Ok(hay
                        .find(needle.as_str())
                        .map(|byte| Value::Int(hay[..byte].chars().count() as i64))
                        .unwrap_or(Value::Nil)),
                    (Value::Str(_), v) => Err(error(
                        format!("find on a string expects a string, got {}", v.type_name()),
                        span,
                    )),
                    (Value::List(items), v) => Ok(items
                        .borrow()
                        .iter()
                        .position(|it| it == v)
                        .map(|i| Value::Int(i as i64))
                        .unwrap_or(Value::Nil)),
                    (a, _) => Err(error(
                        format!("find expects a string or list, got {}", a.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Replace => {
                arity(3, 3)?;
                match (&args[0], &args[1], &args[2]) {
                    (Value::Str(s), Value::Str(from), Value::Str(to)) => {
                        if from.is_empty() {
                            Err(error(
                                "replace does not accept an empty search string",
                                span,
                            ))
                        } else {
                            Ok(Value::Str(s.replace(from.as_str(), to)))
                        }
                    }
                    (a, b, c) => Err(error(
                        format!(
                            "replace expects three strings, got {}, {} and {}",
                            a.type_name(),
                            b.type_name(),
                            c.type_name()
                        ),
                        span,
                    )),
                }
            }
            Builtin::StartsWith | Builtin::EndsWith => {
                arity(2, 2)?;
                match (&args[0], &args[1]) {
                    (Value::Str(s), Value::Str(x)) => {
                        Ok(Value::Bool(if b == Builtin::StartsWith {
                            s.starts_with(x.as_str())
                        } else {
                            s.ends_with(x.as_str())
                        }))
                    }
                    (a, c) => Err(error(
                        format!(
                            "{} expects two strings, got {} and {}",
                            b.name(),
                            a.type_name(),
                            c.type_name()
                        ),
                        span,
                    )),
                }
            }
            Builtin::Upper | Builtin::Lower => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Str(s) => Ok(Value::Str(if b == Builtin::Upper {
                        s.to_uppercase()
                    } else {
                        s.to_lowercase()
                    })),
                    v => Err(error(
                        format!("{} expects a string, got {}", b.name(), v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Slice => {
                arity(3, 3)?;
                let (lo, hi) = match (&args[1], &args[2]) {
                    (Value::Int(lo), Value::Int(hi)) => (*lo, *hi),
                    (a, c) => {
                        return Err(error(
                            format!(
                                "slice expects int bounds, got {} and {}",
                                a.type_name(),
                                c.type_name()
                            ),
                            span,
                        ));
                    }
                };
                match &args[0] {
                    Value::Str(s) => {
                        let chars: Vec<char> = s.chars().collect();
                        let (lo, hi) = slice_bounds(lo, hi, chars.len());
                        Ok(Value::Str(chars[lo..hi].iter().collect()))
                    }
                    Value::List(items) => {
                        let items = items.borrow();
                        let (lo, hi) = slice_bounds(lo, hi, items.len());
                        Ok(Value::list(items[lo..hi].to_vec()))
                    }
                    v => Err(error(
                        format!("slice expects a string or list, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Args => {
                arity(0, 0)?;
                Ok(Value::list(
                    self.script_args.iter().cloned().map(Value::Str).collect(),
                ))
            }
            Builtin::Input => {
                arity(0, 0)?;
                use std::io::BufRead;
                let mut line = String::new();
                match std::io::stdin().lock().read_line(&mut line) {
                    Ok(0) => Ok(Value::Nil),
                    Ok(_) => {
                        if line.ends_with('\n') {
                            line.pop();
                            if line.ends_with('\r') {
                                line.pop();
                            }
                        }
                        Ok(Value::Str(line))
                    }
                    Err(e) => Err(error(format!("input failed: {e}"), span)),
                }
            }
            Builtin::ReadFile => {
                arity(1, 1)?;
                match &args[0] {
                    // "-" is the conventional name for stdin, read to EOF.
                    Value::Str(path) if path == "-" => {
                        let mut buf = String::new();
                        std::io::Read::read_to_string(&mut std::io::stdin().lock(), &mut buf)
                            .map(|_| Value::Str(buf))
                            .map_err(|e| error(format!("cannot read stdin: {e}"), span))
                    }
                    Value::Str(path) => std::fs::read_to_string(path)
                        .map(Value::Str)
                        .map_err(|e| error(format!("cannot read {path:?}: {e}"), span)),
                    v => Err(error(
                        format!("read_file expects a string path, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::WriteFile => {
                arity(2, 3)?;
                let append = match args.get(2) {
                    None => false,
                    Some(Value::Str(mode)) if mode == "append" => true,
                    Some(v) => {
                        return Err(error(
                            format!(
                                "write_file mode must be the string \"append\", got {}",
                                match v {
                                    Value::Str(m) => format!("{m:?}"),
                                    other => other.type_name().to_string(),
                                }
                            ),
                            span,
                        ));
                    }
                };
                match (&args[0], &args[1]) {
                    (Value::Str(path), Value::Str(s)) => {
                        let result = if append {
                            std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(path)
                                .and_then(|mut f| std::io::Write::write_all(&mut f, s.as_bytes()))
                        } else {
                            std::fs::write(path, s)
                        };
                        result
                            .map(|_| Value::Nil)
                            .map_err(|e| error(format!("cannot write {path:?}: {e}"), span))
                    }
                    (a, b) => Err(error(
                        format!(
                            "write_file expects two strings, got {} and {}",
                            a.type_name(),
                            b.type_name()
                        ),
                        span,
                    )),
                }
            }
            Builtin::ListDir => {
                arity(1, 1)?;
                let Value::Str(path) = &args[0] else {
                    return Err(error(
                        format!(
                            "list_dir expects a string path, got {}",
                            args[0].type_name()
                        ),
                        span,
                    ));
                };
                let entries = std::fs::read_dir(std::path::Path::new(&**path))
                    .map_err(|e| error(format!("cannot list {path:?}: {e}"), span))?;
                let mut names = Vec::new();
                for entry in entries {
                    let entry =
                        entry.map_err(|e| error(format!("cannot list {path:?}: {e}"), span))?;
                    // A name that is not UTF-8 cannot become a ting
                    // string that reopens the same file, and quietly
                    // dropping it would make a walk lie about what it
                    // saw, so the whole listing says so instead.
                    match entry.file_name().into_string() {
                        Ok(name) => names.push(Value::Str(name)),
                        Err(bad) => {
                            return Err(error(
                                format!(
                                    "cannot list {path:?}: the name {:?} is not valid UTF-8",
                                    bad.to_string_lossy()
                                ),
                                span,
                            ));
                        }
                    }
                }
                // read_dir's order is whatever the filesystem says;
                // sorted, so a script sees the same list twice.
                names.sort_by(|a, b| match (a, b) {
                    (Value::Str(a), Value::Str(b)) => a.cmp(b),
                    _ => std::cmp::Ordering::Equal,
                });
                Ok(Value::List(Rc::new(RefCell::new(names))))
            }
            // A question about the filesystem, not a demand of it:
            // exists and is_dir answer false rather than raising when
            // the path is absent or unreadable, since "is it there?"
            // already covers "no".
            Builtin::Exists | Builtin::IsDir => {
                arity(1, 1)?;
                let Value::Str(path) = &args[0] else {
                    return Err(error(
                        format!(
                            "{} expects a string path, got {}",
                            b.name(),
                            args[0].type_name()
                        ),
                        span,
                    ));
                };
                let path = std::path::Path::new(&**path);
                Ok(Value::Bool(match b {
                    Builtin::IsDir => path.is_dir(),
                    _ => path.exists(),
                }))
            }
            Builtin::MakeDir => {
                arity(1, 1)?;
                let Value::Str(path) = &args[0] else {
                    return Err(error(
                        format!(
                            "make_dir expects a string path, got {}",
                            args[0].type_name()
                        ),
                        span,
                    ));
                };
                // Parents included, and an existing directory is not
                // an error: the point is that the directory is there
                // afterwards, which is what write_file needs.
                std::fs::create_dir_all(std::path::Path::new(&**path))
                    .map_err(|e| error(format!("cannot create {path:?}: {e}"), span))?;
                Ok(Value::Nil)
            }
            // Demands, like list_dir: removing something that is not
            // there is a mistake worth hearing about, and remove_dir
            // takes only an empty directory — a recursive delete is
            // composed in ting (lib/fs.ting's remove_tree), where it
            // is readable, rather than hidden behind one builtin.
            Builtin::RemoveFile | Builtin::RemoveDir => {
                arity(1, 1)?;
                let Value::Str(path) = &args[0] else {
                    return Err(error(
                        format!(
                            "{} expects a string path, got {}",
                            b.name(),
                            args[0].type_name()
                        ),
                        span,
                    ));
                };
                let p = std::path::Path::new(&**path);
                let done = match b {
                    Builtin::RemoveDir => std::fs::remove_dir(p),
                    _ => std::fs::remove_file(p),
                };
                done.map_err(|e| error(format!("cannot remove {path:?}: {e}"), span))?;
                Ok(Value::Nil)
            }
            // The two directions between a character and its number.
            // ord takes exactly one character, not a prefix: a string
            // of several characters has several code points, and
            // silently picking the first would be a guess.
            Builtin::Ord => {
                arity(1, 1)?;
                let Value::Str(s) = &args[0] else {
                    return Err(error(
                        format!("ord expects a string, got {}", args[0].type_name()),
                        span,
                    ));
                };
                let mut chars = s.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => Ok(Value::Int(c as u32 as i64)),
                    _ => Err(error(
                        format!(
                            "ord expects a one-character string, got {}",
                            crate::diag::plural(s.chars().count(), "character")
                        ),
                        span,
                    )),
                }
            }
            Builtin::Chr => {
                arity(1, 1)?;
                let Value::Int(n) = &args[0] else {
                    return Err(error(
                        format!("chr expects an int, got {}", args[0].type_name()),
                        span,
                    ));
                };
                // Surrogates and anything past the last code point are
                // not characters; say so rather than substituting one.
                match u32::try_from(*n).ok().and_then(char::from_u32) {
                    Some(c) => Ok(Value::Str(c.to_string())),
                    None => Err(error(format!("chr: {n} is not a code point"), span)),
                }
            }
            Builtin::Sort => {
                arity(1, 1)?;
                match &args[0] {
                    Value::List(items) => {
                        let mut items = items.borrow().clone();
                        ensure_sortable(items.iter(), "sort", span)?;
                        items.sort_by(cmp_ordered);
                        Ok(Value::list(items))
                    }
                    v => Err(error(
                        format!("sort expects a list, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::SortBy => {
                arity(2, 2)?;
                let f = args[1].clone();
                match (&args[0], &f) {
                    (Value::List(items), Value::Fn(_) | Value::Builtin(_)) => {
                        let snapshot = items.borrow().clone();
                        let mut keyed = Vec::with_capacity(snapshot.len());
                        for v in snapshot {
                            let k = self.call_value(&f, vec![v.clone()], span)?;
                            keyed.push((k, v));
                        }
                        ensure_sortable(keyed.iter().map(|(k, _)| k), "sort_by keys", span)?;
                        keyed.sort_by(|a, b| cmp_ordered(&a.0, &b.0));
                        Ok(Value::list(keyed.into_iter().map(|(_, v)| v).collect()))
                    }
                    (a, f) => Err(error(
                        format!(
                            "sort_by expects a list and a function, got {} and {}",
                            a.type_name(),
                            f.type_name()
                        ),
                        span,
                    )),
                }
            }
            Builtin::Try => {
                arity(1, 1)?;
                match &args[0] {
                    f @ (Value::Fn(_) | Value::Builtin(_)) => {
                        let f = f.clone();
                        let mut m = std::collections::BTreeMap::new();
                        match self.call_value(&f, Vec::new(), span) {
                            Ok(v) => {
                                m.insert("ok".to_string(), v);
                            }
                            Err(e) => {
                                // The same three things the diagnostic
                                // prints: what failed, where, and the
                                // calls it came out of.
                                m.insert(
                                    "at".to_string(),
                                    Value::map(self.site(e.span, &e.origin)),
                                );
                                let trace = e
                                    .frames
                                    .iter()
                                    .map(|f| {
                                        let mut site = self.site(f.span, &f.origin);
                                        site.insert(
                                            "fn".to_string(),
                                            match &f.name {
                                                Some(n) => Value::Str(n.to_string()),
                                                None => Value::Nil,
                                            },
                                        );
                                        Value::map(site)
                                    })
                                    .collect();
                                m.insert("trace".to_string(), Value::list(trace));
                                m.insert("err".to_string(), Value::Str(e.message));
                            }
                        };
                        Ok(Value::map(m))
                    }
                    v => Err(error(
                        format!("try expects a function, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Fail => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Str(msg) => Err(error(msg.clone(), span)),
                    v => Err(error(
                        format!("fail expects a string message, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Map | Builtin::Filter => {
                arity(2, 2)?;
                match (&args[0], &args[1]) {
                    (Value::List(items), f @ (Value::Fn(_) | Value::Builtin(_))) => {
                        let f = f.clone();
                        let snapshot = items.borrow().clone();
                        let mut out = Vec::with_capacity(snapshot.len());
                        for v in snapshot {
                            let r = self.call_value(&f, vec![v.clone()], span)?;
                            if b == Builtin::Map {
                                out.push(r);
                            } else {
                                match r {
                                    Value::Bool(true) => out.push(v),
                                    Value::Bool(false) => {}
                                    other => {
                                        return Err(error(
                                            format!(
                                                "filter predicate must return bool, got {}",
                                                other.type_name()
                                            ),
                                            span,
                                        ));
                                    }
                                }
                            }
                        }
                        Ok(Value::list(out))
                    }
                    (a, f) => Err(error(
                        format!(
                            "{} expects a list and a function, got {} and {}",
                            b.name(),
                            a.type_name(),
                            f.type_name()
                        ),
                        span,
                    )),
                }
            }
            Builtin::Reduce => {
                arity(3, 3)?;
                match (&args[0], &args[2]) {
                    (Value::List(items), f @ (Value::Fn(_) | Value::Builtin(_))) => {
                        let f = f.clone();
                        let snapshot = items.borrow().clone();
                        let mut acc = args[1].clone();
                        for v in snapshot {
                            acc = self.call_value(&f, vec![acc, v], span)?;
                        }
                        Ok(acc)
                    }
                    (a, f) => Err(error(
                        format!(
                            "reduce expects a list, an initial value, and a function, got {}, {} and {}",
                            a.type_name(),
                            args[1].type_name(),
                            f.type_name()
                        ),
                        span,
                    )),
                }
            }
            Builtin::Min | Builtin::Max => {
                arity(1, 1)?;
                match &args[0] {
                    Value::List(items) => {
                        let items = items.borrow();
                        if items.is_empty() {
                            return Err(error(format!("{} of an empty list", b.name()), span));
                        }
                        ensure_sortable(items.iter(), b.name(), span)?;
                        let mut best = items[0].clone();
                        for v in items.iter().skip(1) {
                            let ord = cmp_ordered(v, &best);
                            if (b == Builtin::Min) == ord.is_lt() && !ord.is_eq() {
                                best = v.clone();
                            }
                        }
                        Ok(best)
                    }
                    v => Err(error(
                        format!("{} expects a list, got {}", b.name(), v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Abs => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Int(n) => n
                        .checked_abs()
                        .map(Value::Int)
                        .ok_or_else(|| error("integer overflow", span)),
                    Value::Float(x) => Ok(Value::Float(x.abs())),
                    v => Err(error(
                        format!("abs expects a number, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Assert => {
                arity(1, 2)?;
                // Every assert is a check, whether it holds or not:
                // `--test` reports the count per file.
                count_check();
                let msg = match args.get(1) {
                    None => None,
                    Some(Value::Str(s)) => Some(s.clone()),
                    Some(v) => {
                        return Err(error(
                            format!("assert message must be a string, got {}", v.type_name()),
                            span,
                        ));
                    }
                };
                match &args[0] {
                    Value::Bool(true) => Ok(Value::Nil),
                    Value::Bool(false) => Err(error(
                        match msg {
                            Some(m) => format!("assertion failed: {m}"),
                            None => "assertion failed".to_string(),
                        },
                        span,
                    )),
                    v => Err(error(
                        format!("assert expects a bool, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Format => {
                if args.is_empty() {
                    return Err(error("format expects at least 1 argument, got 0", span));
                }
                let Value::Str(fmt) = &args[0] else {
                    return Err(error(
                        format!("format expects a string, got {}", args[0].type_name()),
                        span,
                    ));
                };
                let mut out = String::with_capacity(fmt.len());
                let mut next = 1;
                let mut chars = fmt.chars().peekable();
                while let Some(c) = chars.next() {
                    match c {
                        '{' if chars.peek() == Some(&'{') => {
                            chars.next();
                            out.push('{');
                        }
                        '}' if chars.peek() == Some(&'}') => {
                            chars.next();
                            out.push('}');
                        }
                        '{' if chars.peek() == Some(&'}') => {
                            chars.next();
                            if next >= args.len() {
                                return Err(error(
                                    "format: more {} placeholders than value arguments",
                                    span,
                                ));
                            }
                            out.push_str(&args[next].to_string());
                            next += 1;
                        }
                        '{' => {
                            return Err(error(
                                "format: '{' must be followed by '}' (write '{{' for a literal brace)",
                                span,
                            ));
                        }
                        '}' => {
                            return Err(error(
                                "format: stray '}' (write '}}' for a literal brace)",
                                span,
                            ));
                        }
                        c => out.push(c),
                    }
                }
                if next != args.len() {
                    return Err(error(
                        format!(
                            "format: {} but {}",
                            crate::diag::plural(next - 1, "placeholder"),
                            crate::diag::plural(args.len() - 1, "value argument")
                        ),
                        span,
                    ));
                }
                Ok(Value::Str(out))
            }
            Builtin::JsonParse => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Str(s) => crate::json::decode(s).map_err(|m| error(m, span)),
                    v => Err(error(
                        format!("json_parse expects a string, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::JsonStr => {
                arity(1, 2)?;
                let result = match args.get(1) {
                    None => crate::json::encode(&args[0]),
                    Some(Value::Int(n)) if *n >= 0 && *n <= 16 => {
                        crate::json::encode_pretty(&args[0], *n as usize)
                    }
                    Some(v) => {
                        return Err(error(
                            format!(
                                "json_str indent must be an int from 0 to 16, got {}",
                                v.type_name()
                            ),
                            span,
                        ));
                    }
                };
                result.map(Value::Str).map_err(|m| error(m, span))
            }
            Builtin::Env => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Str(name) => Ok(match std::env::var(name) {
                        Ok(v) => Value::Str(v),
                        Err(_) => Value::Nil,
                    }),
                    v => Err(error(
                        format!("env expects a string name, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::Exit => {
                arity(0, 1)?;
                let code = match args.first() {
                    None => 0,
                    Some(Value::Int(n)) => *n,
                    Some(v) => {
                        return Err(error(
                            format!("exit expects an int code, got {}", v.type_name()),
                            span,
                        ));
                    }
                };
                if cfg!(target_arch = "wasm32") {
                    // process::exit would trap the wasm instance.
                    return Err(error("exit is not available in this environment", span));
                }
                self.out
                    .flush()
                    .map_err(|e| error(format!("exit: flush failed: {e}"), span))?;
                std::process::exit(code.clamp(0, 255) as i32)
            }
            Builtin::TimeMs => {
                arity(0, 0)?;
                if cfg!(target_arch = "wasm32") {
                    // SystemTime::now() panics on wasm32-unknown-unknown.
                    return Err(error("time_ms is not available in this environment", span));
                }
                let ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| error(format!("time_ms: {e}"), span))?
                    .as_millis();
                Ok(Value::Int(ms as i64))
            }
            Builtin::SleepMs => {
                arity(1, 1)?;
                let ms = match &args[0] {
                    Value::Int(n) => *n,
                    v => {
                        return Err(error(
                            format!("sleep_ms expects an int, got {}", v.type_name()),
                            span,
                        ));
                    }
                };
                if ms < 0 {
                    return Err(error(format!("sleep_ms: {ms} is negative"), span));
                }
                if cfg!(target_arch = "wasm32") {
                    // Blocking the wasm instance would freeze the page.
                    return Err(error("sleep_ms is not available in this environment", span));
                }
                // Output written before a pause should be visible during it.
                self.out
                    .flush()
                    .map_err(|e| error(format!("sleep_ms: flush failed: {e}"), span))?;
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                Ok(Value::Nil)
            }
            Builtin::Random => {
                arity(0, 0)?;
                // The top 53 bits are exactly a double's mantissa, so
                // every value is representable and none is favoured.
                let bits = self.next_u64() >> 11;
                Ok(Value::Float(bits as f64 * (1.0 / (1u64 << 53) as f64)))
            }
            Builtin::RandomInt => {
                arity(2, 2)?;
                let (lo, hi) = match (&args[0], &args[1]) {
                    (Value::Int(lo), Value::Int(hi)) => (*lo, *hi),
                    (l, r) => {
                        return Err(error(
                            format!(
                                "random_int expects two ints, got {} and {}",
                                l.type_name(),
                                r.type_name()
                            ),
                            span,
                        ));
                    }
                };
                if lo >= hi {
                    return Err(error(format!("random_int: {lo} to {hi} is empty"), span));
                }
                // Width as unsigned, so the whole int range is one span.
                let width = (hi as u64).wrapping_sub(lo as u64);
                // Reject the short tail so every value is equally likely.
                let tail = width.wrapping_neg() % width;
                let mut x = self.next_u64();
                while x < tail {
                    x = self.next_u64();
                }
                Ok(Value::Int(lo.wrapping_add((x % width) as i64)))
            }
            Builtin::Seed => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Int(n) => {
                        self.rng = Some(*n as u64);
                        Ok(Value::Nil)
                    }
                    v => Err(error(
                        format!("seed expects an int, got {}", v.type_name()),
                        span,
                    )),
                }
            }
            Builtin::EPrint => {
                let parts: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                // Ordered against print: what was said about the data
                // must not overtake the data itself.
                let _ = self.out.flush();
                if cfg!(target_arch = "wasm32") {
                    // A page has one stream, so the playground shows
                    // both together rather than dropping half of them.
                    if let Err(e) = writeln!(self.out, "{}", parts.join(" ")) {
                        return Err(error(format!("eprint failed: {e}"), span));
                    }
                    return Ok(Value::Nil);
                }
                let mut err = std::io::stderr();
                if let Err(e) = writeln!(err, "{}", parts.join(" ")) {
                    // Same courtesy print extends to a reader that left.
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        std::process::exit(0);
                    }
                    return Err(error(format!("eprint failed: {e}"), span));
                }
                Ok(Value::Nil)
            }
            Builtin::Cwd => {
                arity(0, 0)?;
                if cfg!(target_arch = "wasm32") {
                    // A page stands nowhere.
                    return Err(error("cwd is not available in this environment", span));
                }
                let dir = std::env::current_dir().map_err(|e| error(format!("cwd: {e}"), span))?;
                Ok(Value::Str(dir.to_string_lossy().into_owned()))
            }
            Builtin::ReTest => {
                arity(2, 2)?;
                let (text, pattern) = Self::subject_and_pattern(b, &args, span)?;
                let re = self.pattern(b, &pattern, span)?;
                let chars: Vec<char> = text.chars().collect();
                Ok(Value::Bool(re.find_at(&chars, 0).is_some()))
            }
            Builtin::ReFind => {
                arity(2, 2)?;
                let (text, pattern) = Self::subject_and_pattern(b, &args, span)?;
                let re = self.pattern(b, &pattern, span)?;
                let chars: Vec<char> = text.chars().collect();
                match re.find_at(&chars, 0) {
                    None => Ok(Value::Nil),
                    Some(caps) => Ok(match_value(&chars, &caps, re.groups())),
                }
            }
            Builtin::ReFindAll => {
                arity(2, 2)?;
                let (text, pattern) = Self::subject_and_pattern(b, &args, span)?;
                let re = self.pattern(b, &pattern, span)?;
                let chars: Vec<char> = text.chars().collect();
                let found = all_matches(&re, &chars)
                    .iter()
                    .map(|caps| match_value(&chars, caps, re.groups()))
                    .collect();
                Ok(Value::list(found))
            }
            Builtin::ReReplace => {
                arity(3, 3)?;
                let (text, pattern) = Self::subject_and_pattern(b, &args, span)?;
                let repl = match &args[2] {
                    Value::Str(s) => s.clone(),
                    v => {
                        return Err(error(
                            format!(
                                "re_replace expects a string to put in, got {}",
                                v.type_name()
                            ),
                            span,
                        ));
                    }
                };
                let re = self.pattern(b, &pattern, span)?;
                let chars: Vec<char> = text.chars().collect();
                let mut out = String::new();
                let mut at = 0;
                for caps in all_matches(&re, &chars) {
                    let (start, end) = (caps[0].unwrap_or(0), caps[1].unwrap_or(0));
                    out.extend(&chars[at..start]);
                    out.push_str(&expand(&repl, &chars, &caps, re.groups(), span)?);
                    at = end;
                }
                out.extend(&chars[at..]);
                Ok(Value::Str(out))
            }
            Builtin::ReSplit => {
                arity(2, 2)?;
                let (text, pattern) = Self::subject_and_pattern(b, &args, span)?;
                let re = self.pattern(b, &pattern, span)?;
                let chars: Vec<char> = text.chars().collect();
                let mut pieces = Vec::new();
                let mut at = 0;
                for caps in all_matches(&re, &chars) {
                    let (start, end) = (caps[0].unwrap_or(0), caps[1].unwrap_or(0));
                    pieces.push(Value::Str(chars[at..start].iter().collect()));
                    at = end;
                }
                pieces.push(Value::Str(chars[at..].iter().collect()));
                Ok(Value::list(pieces))
            }
            Builtin::Run => {
                arity(1, 2)?;
                let cmd = match &args[0] {
                    Value::Str(s) => s.clone(),
                    v => {
                        return Err(error(
                            format!("run expects a program name, got {}", v.type_name()),
                            span,
                        ));
                    }
                };
                // An argv list, never a shell string: there are no
                // quoting rules to get wrong and nothing to inject.
                let mut argv: Vec<String> = Vec::new();
                if let Some(v) = args.get(1) {
                    match v {
                        Value::List(items) => {
                            for item in items.borrow().iter() {
                                match item {
                                    Value::Str(s) => argv.push(s.to_string()),
                                    other => {
                                        return Err(error(
                                            format!(
                                                "run expects a list of strings, got {} in it",
                                                other.type_name()
                                            ),
                                            span,
                                        ));
                                    }
                                }
                            }
                        }
                        other => {
                            return Err(error(
                                format!(
                                    "run expects a list of arguments, got {}",
                                    other.type_name()
                                ),
                                span,
                            ));
                        }
                    }
                }
                if cfg!(target_arch = "wasm32") {
                    // There is nothing to spawn inside a page.
                    return Err(error("run is not available in this environment", span));
                }
                // Anything already printed belongs before whatever the
                // child says about itself.
                self.out
                    .flush()
                    .map_err(|e| error(format!("run: flush failed: {e}"), span))?;
                let done = std::process::Command::new(&*cmd).args(&argv).output();
                // A program that is not there is an error, not an exit
                // code: "not installed" must never read as "ran and
                // failed".
                let done =
                    done.map_err(|e| error(format!("run: cannot start {cmd}: {e}"), span))?;
                let mut m = std::collections::BTreeMap::new();
                m.insert(
                    "code".to_string(),
                    // No code at all means a signal killed it.
                    match done.status.code() {
                        Some(c) => Value::Int(c as i64),
                        None => Value::Nil,
                    },
                );
                m.insert(
                    "out".to_string(),
                    Value::Str(String::from_utf8_lossy(&done.stdout).into_owned()),
                );
                m.insert(
                    "err".to_string(),
                    Value::Str(String::from_utf8_lossy(&done.stderr).into_owned()),
                );
                Ok(Value::map(m))
            }
            Builtin::Import => {
                arity(1, 1)?;
                match &args[0] {
                    Value::Str(path) => {
                        let path = path.clone();
                        self.import_module(&path, span)
                    }
                    v => Err(error(
                        format!("import expects a string path, got {}", v.type_name()),
                        span,
                    )),
                }
            }
        }
    }

    /// Load, run, and cache a module. The module executes in a fresh
    /// global environment; its top-level bindings (minus untouched
    /// builtins) come back as a map, the same map on every import.
    /// The module currently being imported, for functions defined
    /// during its top-level run.
    pub(crate) fn current_origin(&self) -> Option<Rc<Origin>> {
        self.origin_stack.last().cloned()
    }

    fn import_module(&mut self, path: &str, span: Span) -> Result<Value, RuntimeError> {
        let base = self.dir_stack.last().cloned().unwrap_or_default();
        let raw = if std::path::Path::new(path).is_absolute() {
            std::path::PathBuf::from(path)
        } else {
            base.join(path)
        };
        let mut resolved = raw.canonicalize().unwrap_or(raw);
        if let Some(cached) = self.import_cache.get(&resolved) {
            return Ok(cached.clone());
        }
        if self.importing.contains(&resolved) {
            return Err(error(format!("circular import of {path:?}"), span));
        }
        // Filesystem first; a "lib/<name>.ting" path that has no file
        // falls back to the stdlib embedded in the binary, so the
        // standard library works from any directory, in the REPL, and
        // in the wasm playground.
        let src = match std::fs::read_to_string(&resolved) {
            Ok(src) => src,
            Err(e) => {
                let key = path.trim_start_matches("./");
                let hit = EMBEDDED_STDLIB
                    .iter()
                    .find(|(name, _)| key == *name || key.ends_with(&format!("/{name}")));
                let Some((name, embedded)) = hit else {
                    // Say where the lookup went: the path it resolved to
                    // relative to the importing file, and that no embedded
                    // module has that name either.
                    return Err(error(
                        format!(
                            "cannot import {path:?}: no file at {} ({e}), and no embedded module of that name",
                            resolved.display()
                        ),
                        span,
                    ));
                };
                resolved = std::path::PathBuf::from(format!("<embedded>/{name}"));
                if let Some(cached) = self.import_cache.get(&resolved) {
                    return Ok(cached.clone());
                }
                embedded.to_string()
            }
        };
        let in_module = |m: &str, s: Span, src: &str| {
            let (line, col) = s.line_col(src);
            error(
                format!("error in module {path:?} at {line}:{col}: {m}"),
                span,
            )
        };
        let tokens = crate::lexer::lex(&src).map_err(|e| in_module(&e.message, e.span, &src))?;
        let program = crate::parser::parse_program(&tokens)
            .map_err(|e| in_module(&e.message, e.span, &src))?;

        let origin_path = match resolved.to_str() {
            Some(p) if p.starts_with("<embedded>/") => p["<embedded>/".len()..].to_string(),
            _ => resolved.display().to_string(),
        };
        self.origin_stack.push(Rc::new(Origin {
            path: origin_path,
            src: Rc::from(src.as_str()),
        }));
        let saved_env = std::mem::replace(&mut self.env, global_env());
        self.importing.push(resolved.clone());
        self.dir_stack.push(
            resolved
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default(),
        );
        self.note_coverable(&program);
        let result = self.run(&program);
        self.dir_stack.pop();
        self.importing.pop();
        self.origin_stack.pop();
        let module_env = std::mem::replace(&mut self.env, saved_env);
        result.map_err(|e| in_module(&e.message, e.span, &src))?;

        let mut exports = std::collections::BTreeMap::new();
        for (name, v) in module_env.borrow().vars.iter() {
            // Builtins still bound to their own name are ambient, not
            // something the module defined.
            if let Value::Builtin(b) = v
                && b.name() == name.as_ref()
            {
                continue;
            }
            exports.insert(name.to_string(), v.clone());
        }
        let map = Value::map(exports);
        self.import_cache.insert(resolved, map.clone());
        Ok(map)
    }

    /// Call any callable value (used by builtins that take functions).
    pub(crate) fn call_value(
        &mut self,
        f: &Value,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match f {
            Value::Fn(func) => self.call(&Rc::clone(func), args, span),
            Value::Builtin(b) => self.call_builtin(*b, args, span),
            other => Err(error(
                format!("{} is not callable", other.type_name()),
                span,
            )),
        }
    }

    fn call(
        &mut self,
        func: &Rc<Function>,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        // Parameters with a default may be left out; the rest may not.
        // A `...rest` parameter is never required and has no upper
        // bound: it takes whatever the fixed ones leave behind.
        let fixed = func.params.len() - usize::from(func.rest);
        let required = func
            .defaults
            .iter()
            .position(|d| d.is_some())
            .unwrap_or(fixed)
            .min(fixed);
        let mut args = args;
        if args.len() < required || (!func.rest && args.len() > func.params.len()) {
            // Named after the function it was defined as, the same way
            // a trace frame is; a literal has no name to give.
            let who = match &func.name {
                Some(name) => name.to_string(),
                None => "an anonymous function".to_string(),
            };
            let wanted = if func.rest {
                format!("at least {}", crate::diag::plural(required, "argument"))
            } else if required == func.params.len() {
                crate::diag::plural(func.params.len(), "argument")
            } else {
                format!("{required} to {} arguments", func.params.len())
            };
            return Err(error(
                format!("{who} expects {wanted}, got {}", args.len()),
                span,
            ));
        }
        // The leftovers become one list before the defaults run, so a
        // default cannot see them and the scratch scope holds exactly
        // the fixed parameters bound so far.
        let leftovers = if func.rest {
            let tail = if args.len() > fixed {
                args.split_off(fixed)
            } else {
                Vec::new()
            };
            Some(Value::list(tail))
        } else {
            None
        };
        if args.len() < fixed {
            // Defaults are evaluated here, at the call, in a scope that
            // already holds the arguments bound so far — so a later
            // default may name an earlier parameter, and `fn f(xs = [])`
            // hands back a fresh list every time rather than sharing one.
            let mut vars: HashMap<Rc<str>, Value> = HashMap::new();
            for (p, a) in func.params.iter().zip(&args) {
                vars.insert(Rc::clone(p), a.clone());
            }
            let scratch = Rc::new(RefCell::new(Env {
                vars,
                parent: Some(Rc::clone(&func.env)),
            }));
            let saved = std::mem::replace(&mut self.env, scratch);
            let mut filled = Ok(());
            for i in args.len()..fixed {
                let default = func.defaults[i]
                    .as_ref()
                    .expect("a parameter past the required ones has a default");
                match self.eval(default) {
                    Ok(v) => {
                        self.env
                            .borrow_mut()
                            .vars
                            .insert(Rc::clone(&func.params[i]), v.clone());
                        args.push(v);
                    }
                    Err(e) => {
                        filled = Err(e);
                        break;
                    }
                }
            }
            self.env = saved;
            filled?;
        }
        if let Some(rest) = leftovers {
            args.push(rest);
        }
        if self.depth >= self.max_depth {
            return Err(error(
                format!("stack overflow (max call depth {})", self.max_depth),
                span,
            ));
        }
        let (frame, mut locals) = match &func.body {
            FnBody::Ast(_) => {
                let mut vars = HashMap::with_capacity(func.params.len());
                for (p, a) in func.params.iter().zip(args) {
                    vars.insert(Rc::clone(p), a);
                }
                (
                    Rc::new(RefCell::new(Env {
                        vars,
                        parent: Some(Rc::clone(&func.env)),
                    })),
                    Vec::new(),
                )
            }
            FnBody::Chunk(chunk) => {
                let mut locals = crate::vm::take_buf();
                locals.resize(chunk.slots as usize, Value::Nil);
                let mut env_params = HashMap::new();
                for ((p, a), loc) in func.params.iter().zip(args).zip(&chunk.param_locs) {
                    match loc {
                        Some(i) => locals[*i as usize] = a,
                        None => {
                            env_params.insert(Rc::clone(p), a);
                        }
                    }
                }
                // Closure-free bodies keep all state in slots and run
                // directly against the captured env: no Env allocation.
                let frame = if chunk.needs_env_frame {
                    Rc::new(RefCell::new(Env {
                        vars: env_params,
                        parent: Some(Rc::clone(&func.env)),
                    }))
                } else {
                    Rc::clone(&func.env)
                };
                (frame, locals)
            }
        };
        let clock = if self.profile.is_some() {
            self.count_call(func);
            self.child_ns.push(0);
            Some(std::time::Instant::now())
        } else {
            None
        };
        let saved = std::mem::replace(&mut self.env, frame);
        self.depth += 1;
        let caller = self.defining_origin();
        self.call_origins.push(func.origin.clone());
        let result = match &func.body {
            FnBody::Ast(stmts) => self.run_block(stmts).map(ControlOrValue::Control),
            // Compiled bodies cannot leak break/continue (the compiler
            // rejects them), so the VM returns a plain value.
            FnBody::Chunk(chunk) => {
                crate::vm::run_chunk_with(self, chunk, &mut locals).map(ControlOrValue::Value)
            }
        };
        self.call_origins.pop();
        self.depth -= 1;
        self.env = saved;
        crate::vm::give_buf(locals);
        if let Some(started) = clock {
            let key = Self::profile_key(func);
            self.charge(key, started.elapsed().as_nanos());
        }
        // An error leaving a module's function is located in that
        // module's file; every call it unwinds through leaves a frame,
        // so the diagnostic can show the whole way back.
        let result = result.map_err(|mut e| {
            if e.origin.is_none() {
                e.origin = func.origin.clone();
            }
            e.frames.push(Frame {
                name: func.name.clone(),
                span,
                origin: caller.clone(),
            });
            e
        });
        match result? {
            ControlOrValue::Value(v) => Ok(v.unwrap_or(Value::Nil)),
            ControlOrValue::Control(Control::Return(v, _)) => Ok(v),
            ControlOrValue::Control(Control::Normal) => Ok(Value::Nil),
            ControlOrValue::Control(Control::Break(span)) => Err(error("break outside loop", span)),
            ControlOrValue::Control(Control::Continue(span)) => {
                Err(error("continue outside loop", span))
            }
        }
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match &expr.kind {
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Float(x) => Ok(Value::Float(*x)),
            ExprKind::Str(s) => Ok(Value::Str(s.clone())),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Nil => Ok(Value::Nil),
            ExprKind::List(items) => {
                let mut vals = Vec::with_capacity(items.len());
                for it in items {
                    vals.push(self.eval(it)?);
                }
                Ok(Value::list(vals))
            }
            ExprKind::Map(entries) => {
                let mut map = std::collections::BTreeMap::new();
                for (k, v) in entries {
                    let key = match self.eval(k)? {
                        Value::Str(s) => s,
                        other => {
                            return Err(error(
                                format!("map keys must be strings, got {}", other.type_name()),
                                k.span,
                            ));
                        }
                    };
                    map.insert(key, self.eval(v)?);
                }
                Ok(Value::map(map))
            }
            ExprKind::Var(name) => match self.lookup(name) {
                Some(v) => Ok(v),
                None => Err(self.undefined(name, expr.span)),
            },
            ExprKind::Call(callee, args) => {
                let callee_v = self.eval(callee)?;
                let mut arg_vals = Vec::with_capacity(args.len());
                for a in args {
                    match &a.kind {
                        ExprKind::Spread(inner) => {
                            let v = self.eval(inner)?;
                            arg_vals.extend(spread_values(v, inner.span)?);
                        }
                        _ => arg_vals.push(self.eval(a)?),
                    }
                }
                match callee_v {
                    Value::Fn(func) => self.call(&func, arg_vals, expr.span),
                    Value::Builtin(b) => self.call_builtin(b, arg_vals, expr.span),
                    other => Err(error(
                        format!("{} is not callable", other.type_name()),
                        callee.span,
                    )),
                }
            }
            ExprKind::Fn(params, body) => Ok(self.make_fn(params, body, None, expr.span)),
            // The parser only builds one in an argument list, where the
            // call above consumes it before this ever runs.
            ExprKind::Spread(_) => Err(error("'...' outside a call", expr.span)),
            ExprKind::Unary(op, operand) => {
                let v = self.eval(operand)?;
                unary(*op, v, expr.span)
            }
            ExprKind::Binary(op, lhs, rhs) => {
                // && and || short-circuit; everything else evaluates both sides.
                match op {
                    BinaryOp::And | BinaryOp::Or => {
                        let l = as_bool(self.eval(lhs)?, lhs.span)?;
                        match (op, l) {
                            (BinaryOp::And, false) => Ok(Value::Bool(false)),
                            (BinaryOp::Or, true) => Ok(Value::Bool(true)),
                            _ => Ok(Value::Bool(as_bool(self.eval(rhs)?, rhs.span)?)),
                        }
                    }
                    _ => {
                        let l = self.eval(lhs)?;
                        let r = self.eval(rhs)?;
                        binary(*op, l, r, expr.span)
                    }
                }
            }
            ExprKind::Index(base, idx) => {
                let b = self.eval(base)?;
                let i = self.eval(idx)?;
                index(b, i, expr.span)
            }
        }
    }
}

/// The for-loop snapshot conversion (mirrors StmtKind::For exactly).
pub(crate) fn iter_snapshot(v: Value, span: Span) -> Result<Vec<Value>, RuntimeError> {
    match v {
        Value::List(l) => Ok(l.borrow().clone()),
        Value::Str(s) => Ok(s.chars().map(|c| Value::Str(c.to_string())).collect()),
        Value::Map(m) => Ok(m.borrow().keys().cloned().map(Value::Str).collect()),
        v => Err(error(
            format!("cannot iterate over {}", v.type_name()),
            span,
        )),
    }
}

pub(crate) fn as_bool(v: Value, span: Span) -> Result<bool, RuntimeError> {
    match v {
        Value::Bool(b) => Ok(b),
        other => Err(error(
            format!("expected bool, got {}", other.type_name()),
            span,
        )),
    }
}

/// Read an int the way the lexer reads a literal: an optional sign,
/// an optional `0x`/`0b` prefix, and `_` only between two digits.
fn int_from_text(text: &str) -> Option<i64> {
    let text = text.trim();
    let (sign, rest) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text.strip_prefix('+').unwrap_or(text)),
    };
    let (radix, digits) = match rest.strip_prefix("0x") {
        Some(digits) => (16, digits),
        None => match rest.strip_prefix("0b") {
            Some(digits) => (2, digits),
            None => (10, rest),
        },
    };
    let mut clean = String::with_capacity(digits.len());
    let bytes = digits.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'_' {
            let between = i > 0
                && bytes
                    .get(i + 1)
                    .is_some_and(|next| (*next as char).is_digit(radix));
            if !between {
                return None;
            }
        } else if (b as char).is_digit(radix) {
            clean.push(b as char);
        } else {
            return None;
        }
    }
    if clean.is_empty() {
        return None;
    }
    i64::from_str_radix(&format!("{sign}{clean}"), radix).ok()
}

pub(crate) fn unary(op: UnaryOp, v: Value, span: Span) -> Result<Value, RuntimeError> {
    match (op, v) {
        (UnaryOp::Neg, Value::Int(n)) => n
            .checked_neg()
            .map(Value::Int)
            .ok_or_else(|| error("integer overflow", span)),
        (UnaryOp::Neg, Value::Float(x)) => Ok(Value::Float(-x)),
        (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
        (UnaryOp::BitNot, Value::Int(n)) => Ok(Value::Int(!n)),
        (op, v) => Err(error(
            format!("cannot apply '{op}' to {}", v.type_name()),
            span,
        )),
    }
}

pub(crate) fn binary(op: BinaryOp, l: Value, r: Value, span: Span) -> Result<Value, RuntimeError> {
    use BinaryOp::*;
    use Value::*;
    match op {
        Add => match (l, r) {
            (Int(a), Int(b)) => a
                .checked_add(b)
                .map(Int)
                .ok_or_else(|| error("integer overflow", span)),
            (Str(a), Str(b)) => Ok(Str(a + &b)),
            // Concatenation builds a fresh list; neither operand is mutated.
            (List(a), List(b)) => {
                let mut out = a.borrow().clone();
                out.extend(b.borrow().iter().cloned());
                Ok(Value::list(out))
            }
            (l, r) => numeric_or_type_error(op, l, r, span, |a, b| a + b),
        },
        Sub => match (l, r) {
            (Int(a), Int(b)) => a
                .checked_sub(b)
                .map(Int)
                .ok_or_else(|| error("integer overflow", span)),
            (l, r) => numeric_or_type_error(op, l, r, span, |a, b| a - b),
        },
        Mul => match (l, r) {
            (Int(a), Int(b)) => a
                .checked_mul(b)
                .map(Int)
                .ok_or_else(|| error("integer overflow", span)),
            (l, r) => numeric_or_type_error(op, l, r, span, |a, b| a * b),
        },
        Div => match (l, r) {
            (Int(_), Int(0)) => Err(error("division by zero", span)),
            (Int(a), Int(b)) => Ok(Int(a.wrapping_div(b))),
            (l, r) => numeric_or_type_error(op, l, r, span, |a, b| a / b),
        },
        Rem => match (l, r) {
            (Int(_), Int(0)) => Err(error("division by zero", span)),
            (Int(a), Int(b)) => Ok(Int(a.wrapping_rem(b))),
            (l, r) => numeric_or_type_error(op, l, r, span, |a, b| a % b),
        },
        BitAnd | BitOr | BitXor => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(match op {
                BitAnd => a & b,
                BitOr => a | b,
                _ => a ^ b,
            })),
            (l, r) => Err(type_error(op, l, r, span)),
        },
        // Shifting by 64 or more is not a number the hardware agrees
        // about, so it is an error rather than a silent zero. `>>` keeps
        // the sign, the way the type does.
        Shl | Shr => match (l, r) {
            (Int(a), Int(b)) => {
                if !(0..64).contains(&b) {
                    return Err(error(
                        format!("shift amount {b} out of range (0 to 63)"),
                        span,
                    ));
                }
                Ok(Int(match op {
                    Shl => a.wrapping_shl(b as u32),
                    _ => a.wrapping_shr(b as u32),
                }))
            }
            (l, r) => Err(type_error(op, l, r, span)),
        },
        Eq => Ok(Bool(values_equal(&l, &r))),
        Ne => Ok(Bool(!values_equal(&l, &r))),
        Lt | Le | Gt | Ge => compare(op, l, r, span),
        And | Or => unreachable!("short-circuit ops handled in eval"),
    }
}

/// Mixed int/float arithmetic promotes to float; anything else is a type error.
fn numeric_or_type_error(
    op: BinaryOp,
    l: Value,
    r: Value,
    span: Span,
    f: impl Fn(f64, f64) -> f64,
) -> Result<Value, RuntimeError> {
    match (&l, &r) {
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(f(*a as f64, *b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(f(*a, *b as f64))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(f(*a, *b))),
        _ => Err(type_error(op, l, r, span)),
    }
}

/// The bit operations are int-only: a float has no bits worth talking
/// about at this level, and promoting one would need a rounding rule
/// nobody asked for.
fn type_error(op: BinaryOp, l: Value, r: Value, span: Span) -> RuntimeError {
    error(
        format!(
            "cannot apply '{op}' to {} and {}",
            l.type_name(),
            r.type_name()
        ),
        span,
    )
}

/// == is structural; ints and floats compare numerically (1 == 1.0).
fn values_equal(l: &Value, r: &Value) -> bool {
    // Value's PartialEq handles numeric promotion at every depth.
    l == r
}

fn compare(op: BinaryOp, l: Value, r: Value, span: Span) -> Result<Value, RuntimeError> {
    let ord = match (&l, &r) {
        (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
        (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
        (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
        (Value::Str(a), Value::Str(b)) => a.partial_cmp(b),
        _ => {
            return Err(error(
                format!("cannot compare {} and {}", l.type_name(), r.type_name()),
                span,
            ));
        }
    };
    // NaN comparisons are false, matching IEEE semantics.
    let b = match ord {
        None => false,
        Some(ord) => match op {
            BinaryOp::Lt => ord.is_lt(),
            BinaryOp::Le => ord.is_le(),
            BinaryOp::Gt => ord.is_gt(),
            BinaryOp::Ge => ord.is_ge(),
            _ => unreachable!(),
        },
    };
    Ok(Value::Bool(b))
}

/// Resolve a possibly negative index against a length; negative indices
/// count from the end, Python-style.
pub(crate) fn effective_index(i: i64, len: usize, span: Span) -> Result<usize, RuntimeError> {
    let len = len as i64;
    let eff = if i < 0 { i + len } else { i };
    if eff < 0 || eff >= len {
        Err(error(format!("index {i} out of bounds (len {len})"), span))
    } else {
        Ok(eff as usize)
    }
}

pub(crate) fn index(base: Value, idx: Value, span: Span) -> Result<Value, RuntimeError> {
    match (base, idx) {
        (Value::List(items), Value::Int(i)) => {
            let items = items.borrow();
            let eff = effective_index(i, items.len(), span)?;
            Ok(items[eff].clone())
        }
        (Value::Map(entries), Value::Str(k)) => {
            let entries = entries.borrow();
            match entries.get(&k) {
                Some(v) => Ok(v.clone()),
                None => {
                    // The map's own keys are the candidates: a misspelled
                    // member of an imported module lands here too.
                    let near = crate::diag::nearest(&k, entries.keys().map(|k| k.as_ref()));
                    Err(match near {
                        Some(n) => {
                            error(format!("key {k:?} not found (did you mean {n:?}?)"), span)
                        }
                        None => error(format!("key {k:?} not found"), span),
                    })
                }
            }
        }
        (Value::Str(s), Value::Int(i)) => {
            let chars: Vec<char> = s.chars().collect();
            let len = chars.len() as i64;
            let eff = if i < 0 { i + len } else { i };
            if eff < 0 || eff >= len {
                Err(error(format!("index {i} out of bounds (len {len})"), span))
            } else {
                Ok(Value::Str(chars[eff as usize].to_string()))
            }
        }
        (base, idx) => Err(error(
            format!("cannot index {} with {}", base.type_name(), idx.type_name()),
            span,
        )),
    }
}

/// All values orderable, and not strings mixed with numbers.
fn ensure_sortable<'a>(
    vals: impl Iterator<Item = &'a Value>,
    who: &str,
    span: Span,
) -> Result<(), RuntimeError> {
    let (mut nums, mut strs) = (false, false);
    for v in vals {
        match v {
            Value::Int(_) | Value::Float(_) => nums = true,
            Value::Str(_) => strs = true,
            v => {
                return Err(error(format!("{who} cannot order {}", v.type_name()), span));
            }
        }
    }
    if nums && strs {
        return Err(error(
            format!("{who} cannot order numbers and strings together"),
            span,
        ));
    }
    Ok(())
}

/// Total order over values ensure_sortable accepted. Int/Float compare
/// numerically; NaN sorts as equal to everything (partial_cmp fallback).
fn cmp_ordered(a: &Value, b: &Value) -> std::cmp::Ordering {
    let as_f = |v: &Value| match v {
        Value::Int(n) => *n as f64,
        Value::Float(x) => *x,
        _ => unreachable!("ensure_sortable admits only numbers and strings"),
    };
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Str(x), Value::Str(y)) => x.cmp(y),
        _ => as_f(a)
            .partial_cmp(&as_f(b))
            .unwrap_or(std::cmp::Ordering::Equal),
    }
}

/// Python-style slice bounds: negatives count from the end, everything
/// clamps to the valid range, and a backwards range is empty.
fn slice_bounds(lo: i64, hi: i64, len: usize) -> (usize, usize) {
    let len = len as i64;
    let norm = |i: i64| if i < 0 { i + len } else { i }.clamp(0, len);
    let lo = norm(lo);
    let hi = norm(hi).max(lo);
    (lo as usize, hi as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse_expr;

    fn run(src: &str) -> Value {
        let mut interp = Interpreter::new(Vec::new());
        interp
            .eval(&parse_expr(&lex(src).unwrap()).unwrap())
            .unwrap()
    }

    fn run_err(src: &str) -> String {
        let mut interp = Interpreter::new(Vec::new());
        interp
            .eval(&parse_expr(&lex(src).unwrap()).unwrap())
            .unwrap_err()
            .message
    }

    /// Run a whole program and return what it printed.
    fn output(src: &str) -> String {
        use crate::parser::parse_program;
        let mut interp = Interpreter::new(Vec::new());
        interp
            .run(&parse_program(&lex(src).unwrap()).unwrap())
            .unwrap();
        String::from_utf8(interp.out).unwrap()
    }

    fn program_err(src: &str) -> String {
        use crate::parser::parse_program;
        let mut interp = Interpreter::new(Vec::new());
        interp
            .run(&parse_program(&lex(src).unwrap()).unwrap())
            .unwrap_err()
            .message
    }

    #[test]
    fn integer_arithmetic() {
        assert_eq!(run("1 + 2 * 3 - 4"), Value::Int(3));
        assert_eq!(run("7 / 2"), Value::Int(3));
        assert_eq!(run("7 % 2"), Value::Int(1));
        assert_eq!(run("-5 + 3"), Value::Int(-2));
    }

    #[test]
    fn float_and_mixed_arithmetic() {
        assert_eq!(run("1.5 + 1.5"), Value::Float(3.0));
        assert_eq!(run("1 + 0.5"), Value::Float(1.5));
        assert_eq!(run("7.0 / 2"), Value::Float(3.5));
    }

    #[test]
    fn string_concat_and_list_concat() {
        assert_eq!(run("\"foo\" + \"bar\""), Value::Str("foobar".into()));
        assert_eq!(
            run("[1] + [2, 3]"),
            Value::list(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn division_by_zero_errors() {
        assert_eq!(run_err("1 / 0"), "division by zero");
        assert_eq!(run_err("1 % 0"), "division by zero");
    }

    #[test]
    fn float_division_by_zero_is_infinity() {
        assert_eq!(run("1.0 / 0.0"), Value::Float(f64::INFINITY));
    }

    #[test]
    fn integer_overflow_errors() {
        assert_eq!(run_err("9223372036854775807 + 1"), "integer overflow");
        assert_eq!(run_err("-(-9223372036854775807 - 1)"), "integer overflow");
    }

    #[test]
    fn comparisons() {
        assert_eq!(run("1 < 2"), Value::Bool(true));
        assert_eq!(run("2 <= 1"), Value::Bool(false));
        assert_eq!(run("1 < 1.5"), Value::Bool(true));
        assert_eq!(run("\"a\" < \"b\""), Value::Bool(true));
    }

    #[test]
    fn equality_is_structural_and_numeric() {
        assert_eq!(run("1 == 1.0"), Value::Bool(true));
        assert_eq!(run("[1, 2] == [1, 2]"), Value::Bool(true));
        assert_eq!(run("\"x\" == \"x\""), Value::Bool(true));
        assert_eq!(run("nil == nil"), Value::Bool(true));
        assert_eq!(run("1 == \"1\""), Value::Bool(false));
        assert_eq!(run("1 != 2"), Value::Bool(true));
    }

    #[test]
    fn boolean_logic_short_circuits() {
        assert_eq!(run("true && false"), Value::Bool(false));
        assert_eq!(run("false || true"), Value::Bool(true));
        assert_eq!(run("!true"), Value::Bool(false));
        // rhs would error (1/0), but short-circuit skips it
        assert_eq!(run("false && 1 / 0 == 0"), Value::Bool(false));
        assert_eq!(run("true || 1 / 0 == 0"), Value::Bool(true));
    }

    #[test]
    fn booleans_are_strict() {
        assert_eq!(run_err("1 && true"), "expected bool, got int");
        assert_eq!(run_err("!0"), "cannot apply '!' to int");
    }

    #[test]
    fn type_errors_name_both_sides() {
        assert_eq!(run_err("1 + \"x\""), "cannot apply '+' to int and string");
        assert_eq!(run_err("nil < 1"), "cannot compare nil and int");
    }

    #[test]
    fn indexing_lists_and_strings() {
        assert_eq!(run("[10, 20, 30][1]"), Value::Int(20));
        assert_eq!(run("[10, 20, 30][-1]"), Value::Int(30));
        assert_eq!(run("\"héllo\"[1]"), Value::Str("é".into()));
        assert_eq!(run_err("[1][5]"), "index 5 out of bounds (len 1)");
        assert_eq!(run_err("[1][-2]"), "index -2 out of bounds (len 1)");
    }

    #[test]
    fn builtin_len() {
        assert_eq!(
            output("print(len([1, 2, 3]), len(\"héllo\"), len({\"a\": 1}));"),
            "3 5 1\n"
        );
        assert_eq!(program_err("len(1);"), "len does not apply to int");
    }

    #[test]
    fn builtin_push_and_pop() {
        assert_eq!(
            output("let xs = []; push(xs, 1); push(xs, 2); print(pop(xs), xs);"),
            "2 [1]\n"
        );
        assert_eq!(program_err("pop([]);"), "pop from empty list");
    }

    #[test]
    fn builtin_keys_and_has() {
        assert_eq!(
            output("let m = {\"b\": 1, \"a\": 2}; print(keys(m), has(m, \"a\"), has(m, \"z\"));"),
            "[\"a\", \"b\"] true false\n"
        );
    }

    #[test]
    fn builtin_conversions() {
        assert_eq!(
            output("print(int(\"42\"), int(3.9), float(\"2.5\"), float(1), str(42) + \"!\");"),
            "42 3 2.5 1.0 42!\n"
        );
        assert_eq!(
            program_err("int(\"abc\");"),
            "cannot convert \"abc\" to int"
        );
        assert_eq!(program_err("int([]);"), "cannot convert list to int");
    }

    #[test]
    fn builtin_type_and_range() {
        assert_eq!(
            output("print(type(1), type(1.0), type(\"s\"), type(nil), type(len));"),
            "int float string nil function\n"
        );
        assert_eq!(
            output("print(range(3), range(2, 5), range(5, 2));"),
            "[0, 1, 2] [2, 3, 4] []\n"
        );
    }

    #[test]
    fn builtins_are_values_and_shadowable() {
        assert_eq!(output("let f = len; print(f(\"abc\"));"), "3\n");
        assert_eq!(output("print(len);"), "<builtin len>\n");
        assert_eq!(
            output("{ let len = 5; print(len); } print(len(\"ab\"));"),
            "5\n2\n"
        );
    }

    #[test]
    fn for_iterates_lists_strings_maps() {
        assert_eq!(output("for x in [1, 2, 3] { print(x); }"), "1\n2\n3\n");
        assert_eq!(output("for c in \"héj\" { print(c); }"), "h\né\nj\n");
        assert_eq!(
            output("for k in {\"b\": 2, \"a\": 1} { print(k); }"),
            "a\nb\n"
        );
        assert_eq!(output("for x in [] { print(x); }"), "");
        assert_eq!(program_err("for x in 5 { }"), "cannot iterate over int");
    }

    #[test]
    fn break_and_continue() {
        assert_eq!(
            output("for x in range(10) { if x == 3 { break; } print(x); }"),
            "0\n1\n2\n"
        );
        assert_eq!(
            output("for x in range(5) { if x % 2 == 0 { continue; } print(x); }"),
            "1\n3\n"
        );
        assert_eq!(
            output("let i = 0; while true { i = i + 1; if i == 2 { break; } } print(i);"),
            "2\n"
        );
    }

    #[test]
    fn break_only_exits_innermost_loop() {
        assert_eq!(
            output("for a in range(2) { for b in range(9) { if b == 1 { break; } } print(a); }"),
            "0\n1\n"
        );
    }

    #[test]
    fn return_escapes_a_for_loop() {
        assert_eq!(
            output(
                "fn find(xs, want) { for x in xs { if x == want { return true; } } return false; } print(find([1, 2], 2), find([1], 9));"
            ),
            "true false\n"
        );
    }

    #[test]
    fn break_continue_outside_loop_error() {
        assert_eq!(program_err("break;"), "break outside loop");
        assert_eq!(program_err("continue;"), "continue outside loop");
        assert_eq!(
            program_err("for x in [1] { let f = fn() { break; }; f(); }"),
            "break outside loop"
        );
    }

    #[test]
    fn for_iterates_a_snapshot() {
        assert_eq!(
            output("let xs = [1, 2]; for x in xs { push(xs, x); } print(xs);"),
            "[1, 2, 1, 2]\n"
        );
    }

    #[test]
    fn loop_variable_is_per_iteration() {
        assert_eq!(
            output(
                "let fs = []; for x in range(3) { push(fs, fn() { return x; }); } \
                 print(fs[0](), fs[1](), fs[2]());"
            ),
            "0 1 2\n"
        );
    }

    #[test]
    fn builtin_split_join_trim() {
        assert_eq!(
            output("print(split(\"a,b,,c\", \",\"));"),
            "[\"a\", \"b\", \"\", \"c\"]\n"
        );
        assert_eq!(
            output("print(split(\"héllo\", \"\"));"),
            "[\"h\", \"é\", \"l\", \"l\", \"o\"]\n"
        );
        assert_eq!(
            output("print(join([\"a\", \"b\", \"c\"], \"-\"));"),
            "a-b-c\n"
        );
        assert_eq!(output("print(join([], \"-\") + \"empty\");"), "empty\n");
        assert_eq!(output("print(trim(\"  hi \"));"), "hi\n");
        assert_eq!(
            output("print(join(split(\"one two three\", \" \"), \"+\"));"),
            "one+two+three\n"
        );
    }

    #[test]
    fn builtin_split_join_type_errors() {
        assert_eq!(
            program_err("split(\"a\", 1);"),
            "split expects two strings, got string and int"
        );
        assert_eq!(
            program_err("join([1], \",\");"),
            "join expects a list of strings, found int"
        );
        assert_eq!(program_err("trim(1);"), "trim expects a string, got int");
    }

    #[test]
    fn builtin_string_predicates_and_case() {
        assert_eq!(run("contains(\"haystack\", \"stack\")"), Value::Bool(true));
        assert_eq!(run("contains(\"haystack\", \"z\")"), Value::Bool(false));
        assert_eq!(run("contains([1, \"a\", nil], \"a\")"), Value::Bool(true));
        assert_eq!(run("contains([1, 2], 3)"), Value::Bool(false));
        assert_eq!(run("starts_with(\"ting\", \"ti\")"), Value::Bool(true));
        assert_eq!(run("ends_with(\"ting\", \"ti\")"), Value::Bool(false));
        assert_eq!(run("upper(\"héllo\")"), Value::Str("HÉLLO".into()));
        assert_eq!(run("lower(\"HÉLLO\")"), Value::Str("héllo".into()));
    }

    #[test]
    fn builtin_replace() {
        assert_eq!(
            run("replace(\"a-b-c\", \"-\", \"+\")"),
            Value::Str("a+b+c".into())
        );
        assert_eq!(
            run("replace(\"abc\", \"x\", \"y\")"),
            Value::Str("abc".into())
        );
        assert_eq!(
            run_err("replace(\"abc\", \"\", \"y\")"),
            "replace does not accept an empty search string"
        );
    }

    #[test]
    fn builtin_slice() {
        assert_eq!(run("slice(\"hello\", 1, 3)"), Value::Str("el".into()));
        assert_eq!(run("slice(\"héllo\", 0, 2)"), Value::Str("hé".into()));
        // Negative bounds count from the end; out-of-range clamps.
        assert_eq!(run("slice(\"hello\", -3, 99)"), Value::Str("llo".into()));
        assert_eq!(run("slice(\"hello\", 3, 1)"), Value::Str("".into()));
        assert_eq!(
            run("slice([1, 2, 3, 4], 1, -1)"),
            Value::list(vec![Value::Int(2), Value::Int(3)])
        );
        // Slicing copies: mutating the slice leaves the source alone.
        assert_eq!(
            output("let a = [1, 2, 3]; let b = slice(a, 0, 2); b[0] = 9; print(a, b);"),
            "[1, 2, 3] [9, 2]\n"
        );
        assert_eq!(
            run_err("slice(1, 0, 1)"),
            "slice expects a string or list, got int"
        );
        assert_eq!(
            run_err("slice(\"x\", 0.5, 1)"),
            "slice expects int bounds, got float and int"
        );
    }

    #[test]
    fn builtin_string_batch2_type_errors() {
        assert_eq!(
            program_err("contains(1, 2);"),
            "contains expects a string or list, got int"
        );
        assert_eq!(
            program_err("contains(\"a\", 1);"),
            "contains on a string expects a string, got int"
        );
        assert_eq!(
            program_err("starts_with(\"a\", 1);"),
            "starts_with expects two strings, got string and int"
        );
        assert_eq!(program_err("upper(1);"), "upper expects a string, got int");
        assert_eq!(
            program_err("replace(\"a\", \"b\", 1);"),
            "replace expects three strings, got string, string and int"
        );
    }

    #[test]
    fn builtin_import_embedded_stdlib() {
        use crate::parser::parse_program;
        let empty = std::env::temp_dir().join(format!("ting-embed-{}", std::process::id()));
        std::fs::create_dir_all(&empty).unwrap();

        // No lib/ anywhere near the base dir: the embedded copy serves.
        let mut interp = Interpreter::new(Vec::new());
        interp.set_base_dir(empty.clone());
        interp
            .run(
                &parse_program(
                    &lex("let l = import(\"lib/list.ting\"); print(l[\"sum\"]([1, 2, 3]));")
                        .unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(String::from_utf8(interp.into_out()).unwrap(), "6\n");

        // A real file with the same path wins over the embedded copy.
        std::fs::create_dir_all(empty.join("lib")).unwrap();
        std::fs::write(empty.join("lib/list.ting"), "let marker = \"fs\";\n").unwrap();
        let mut interp = Interpreter::new(Vec::new());
        interp.set_base_dir(empty.clone());
        interp
            .run(
                &parse_program(&lex("print(import(\"lib/list.ting\")[\"marker\"]);").unwrap())
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(String::from_utf8(interp.into_out()).unwrap(), "fs\n");
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn builtin_import() {
        let dir = std::env::temp_dir().join(format!("ting-import-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("lib.ting"),
            "let x = 1;\nfn inc(n) { return n + 1; }\n",
        )
        .unwrap();
        // nested.ting imports lib.ting relative to its own directory.
        std::fs::write(
            dir.join("nested.ting"),
            "let lib = import(\"lib.ting\");\nlet y = lib[\"inc\"](lib[\"x\"]);\n",
        )
        .unwrap();
        std::fs::write(dir.join("selfloop.ting"), "import(\"selfloop.ting\");\n").unwrap();
        std::fs::write(dir.join("broken.ting"), "let = ;\n").unwrap();

        use crate::parser::parse_program;
        let run_in = |src: &str| -> Result<String, RuntimeError> {
            let mut interp = Interpreter::new(Vec::new());
            interp.set_base_dir(dir.clone());
            interp.run(&parse_program(&lex(src).unwrap()).unwrap())?;
            Ok(String::from_utf8(interp.into_out()).unwrap())
        };

        assert_eq!(
            run_in("let m = import(\"nested.ting\"); print(m[\"y\"]);").unwrap(),
            "2\n"
        );
        let cycle = run_in("import(\"selfloop.ting\");").unwrap_err();
        assert!(
            cycle.message.contains("circular import"),
            "{}",
            cycle.message
        );
        let broken = run_in("import(\"broken.ting\");").unwrap_err();
        assert!(
            broken
                .message
                .starts_with("error in module \"broken.ting\" at 1:5:"),
            "{}",
            broken.message
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn builtin_map_filter_reduce() {
        assert_eq!(
            output("print(map([1, 2, 3], fn(x) { return x * x; }));"),
            "[1, 4, 9]\n"
        );
        assert_eq!(output("print(map([\"a\", \"bb\"], len));"), "[1, 2]\n");
        assert_eq!(
            output("print(filter(range(1, 10), fn(x) { return x % 3 == 0; }));"),
            "[3, 6, 9]\n"
        );
        assert_eq!(
            output("print(reduce([1, 2, 3, 4], 0, fn(a, x) { return a + x; }));"),
            "10\n"
        );
        assert_eq!(
            output("print(reduce([], \"seed\", fn(a, x) { return x; }));"),
            "seed\n"
        );
        // map copies: source list untouched.
        assert_eq!(
            output("let a = [1]; let m = map(a, fn(x) { return x + 1; }); print(a, m);"),
            "[1] [2]\n"
        );
        assert_eq!(
            run_err("filter([1], fn(x) { return 1; })"),
            "filter predicate must return bool, got int"
        );
        assert_eq!(
            run_err("map(1, len)"),
            "map expects a list and a function, got int and function"
        );
    }

    #[test]
    fn builtin_min_max_abs() {
        assert_eq!(run("min([3, 1.5, 2])"), Value::Float(1.5));
        assert_eq!(run("max([3, 1.5, 2])"), Value::Int(3));
        assert_eq!(run("min([\"pear\", \"fig\"])"), Value::Str("fig".into()));
        assert_eq!(run_err("min([])"), "min of an empty list");
        assert_eq!(
            run_err("max([1, \"a\"])"),
            "max cannot order numbers and strings together"
        );
        assert_eq!(run("abs(-42)"), Value::Int(42));
        assert_eq!(run("abs(1.5)"), Value::Float(1.5));
        assert_eq!(run("abs(-1.5)"), Value::Float(1.5));
        assert_eq!(run_err("abs(-9223372036854775807 - 1)"), "integer overflow");
        assert_eq!(run_err("abs(\"x\")"), "abs expects a number, got string");
    }

    #[test]
    fn builtin_try_and_fail() {
        assert_eq!(
            output("print(try(fn() { return 41 + 1; }));"),
            "{\"ok\": 42}\n"
        );
        // The error branch carries the message, where it was raised
        // and the calls it came out of — here just the literal `try`
        // itself called, and no source set to name.
        assert_eq!(
            output("print(try(fn() { return 1 / 0; })[\"err\"]);"),
            "division by zero\n"
        );
        assert_eq!(
            output("print(len(try(fn() { return 1 / 0; })[\"trace\"]));"),
            "1\n"
        );
        assert_eq!(
            output("print(has(try(fn() { return 1 / 0; }), \"at\"));"),
            "true\n"
        );
        // fail raises; try catches; the message travels.
        assert_eq!(
            output(
                "let r = try(fn() { fail(\"boom\"); });\n\
                 if has(r, \"err\") { print(\"caught:\", r[\"err\"]); }"
            ),
            "caught: boom\n"
        );
        // A function that returns nil still signals success.
        assert_eq!(output("print(try(fn() { }));"), "{\"ok\": nil}\n");
        // Stack overflow is caught, and the interpreter stays usable.
        assert_eq!(
            output(
                "fn f() { return f(); }\n\
                 let r = try(f);\n\
                 print(has(r, \"err\"), try(fn() { return 7; }));"
            ),
            "true {\"ok\": 7}\n"
        );
        assert_eq!(program_err("fail(\"boom\");"), "boom");
        assert_eq!(run_err("try(1)"), "try expects a function, got int");
        assert_eq!(run_err("fail(1)"), "fail expects a string message, got int");
    }

    #[test]
    fn builtin_sort() {
        assert_eq!(
            run("sort([3, 1.5, 2])"),
            Value::list(vec![Value::Float(1.5), Value::Int(2), Value::Int(3)])
        );
        assert_eq!(
            run("sort([\"pear\", \"fig\", \"kiwi\"])"),
            Value::list(vec![
                Value::Str("fig".into()),
                Value::Str("kiwi".into()),
                Value::Str("pear".into())
            ])
        );
        assert_eq!(run("sort([])"), Value::list(vec![]));
        // sort copies: the input list is untouched.
        assert_eq!(
            output("let a = [2, 1]; let b = sort(a); print(a, b);"),
            "[2, 1] [1, 2]\n"
        );
        assert_eq!(
            run_err("sort([1, \"a\"])"),
            "sort cannot order numbers and strings together"
        );
        assert_eq!(run_err("sort([nil])"), "sort cannot order nil");
        assert_eq!(run_err("sort(1)"), "sort expects a list, got int");
    }

    #[test]
    fn builtin_sort_by() {
        assert_eq!(
            output("print(sort_by([\"kiwi\", \"fig\", \"pear\"], len));"),
            "[\"fig\", \"kiwi\", \"pear\"]\n"
        );
        assert_eq!(
            output(
                "let xs = sort_by([[2, \"b\"], [1, \"a\"]], fn(p) { return p[0]; }); print(xs);"
            ),
            "[[1, \"a\"], [2, \"b\"]]\n"
        );
        // Stable: equal keys keep their original order.
        assert_eq!(
            output("print(sort_by([\"bb\", \"aa\", \"cc\"], len));"),
            "[\"bb\", \"aa\", \"cc\"]\n"
        );
        assert_eq!(
            program_err("sort_by([1], 2);"),
            "sort_by expects a list and a function, got list and int"
        );
        assert_eq!(
            program_err("sort_by([nil], fn(x) { return x; });"),
            "sort_by keys cannot order nil"
        );
    }

    #[test]
    fn builtin_args() {
        use crate::parser::parse_program;
        let mut interp = Interpreter::new(Vec::new());
        interp.set_args(vec!["in.txt".into(), "-v".into()]);
        interp
            .run(&parse_program(&lex("print(args(), len(args()));").unwrap()).unwrap())
            .unwrap();
        assert_eq!(
            String::from_utf8(interp.into_out()).unwrap(),
            "[\"in.txt\", \"-v\"] 2\n"
        );
        // No args set: empty list.
        assert_eq!(output("print(args());"), "[]\n");
    }

    #[test]
    fn builtin_read_write_file() {
        let path = std::env::temp_dir().join("ting-eval-io-test.txt");
        // Forward slashes keep the path valid inside a ting string
        // literal on Windows too.
        let p = path.to_str().unwrap().replace('\\', "/");
        let src = format!("write_file(\"{p}\", \"hi\\nthere\"); print(read_file(\"{p}\"));");
        assert_eq!(output(&src), "hi\nthere\n");
        let _ = std::fs::remove_file(&path);

        assert!(program_err("read_file(\"ting-no-such-file-xyz\");").starts_with("cannot read"));
        assert_eq!(
            program_err("read_file(1);"),
            "read_file expects a string path, got int"
        );
        assert_eq!(
            program_err("write_file(\"x\", 1);"),
            "write_file expects two strings, got string and int"
        );
    }

    /// The dice are a property, not a table of numbers: a pinned
    /// constant would freeze the generator, and the promise is only
    /// that a seed repeats and that values stay in their span.
    #[test]
    fn the_same_seed_replays_the_same_sequence() {
        let roll = "seed(7); let a = []; let i = 0; \
                    while (i < 200) { push(a, random_int(-5, 5)); i = i + 1; } \
                    seed(7); let b = []; i = 0; \
                    while (i < 200) { push(b, random_int(-5, 5)); i = i + 1; }";
        assert_eq!(output(&(roll.to_string() + " print(a == b);")), "true\n");
        let again = "seed(8); i = 0; let c = []; \
                     while (i < 200) { push(c, random_int(-5, 5)); i = i + 1; } \
                     print(a == c);";
        assert_eq!(output(&(roll.to_string() + " " + again)), "false\n");
    }

    #[test]
    fn every_roll_lands_inside_its_span() {
        assert_eq!(
            output(
                "seed(3); let i = 0; let ok = true; \
                 while (i < 1000) { let x = random(); \
                   if (x < 0.0 || x >= 1.0) { ok = false; } \
                   let n = random_int(-3, 4); \
                   if (n < -3 || n >= 4) { ok = false; } i = i + 1; } print(ok);"
            ),
            "true\n"
        );
        // A span of one has exactly one answer, and no rejection loop
        // may spin forever reaching it.
        assert_eq!(output("seed(1); print(random_int(9, 10));"), "9\n");
    }

    #[test]
    fn an_empty_span_and_a_wrong_type_are_refused() {
        assert_eq!(
            program_err("random_int(5, 5);"),
            "random_int: 5 to 5 is empty"
        );
        assert_eq!(
            program_err("random_int(2, 1);"),
            "random_int: 2 to 1 is empty"
        );
        assert_eq!(
            program_err("random_int(0, 1.5);"),
            "random_int expects two ints, got int and float"
        );
        assert_eq!(
            program_err("seed(\"x\");"),
            "seed expects an int, got string"
        );
        assert_eq!(
            program_err("random(1);"),
            "random expects 0 arguments, got 1"
        );
    }

    #[test]
    fn patterns_find_and_test() {
        assert_eq!(output("print(re_test(\"hello\", \"l+o\"));"), "true\n");
        assert_eq!(output("print(re_test(\"hello\", \"^ello\"));"), "false\n");
        assert_eq!(
            output("print(json_str(re_find(\"a x9 b\", \"([a-z])(\\\\d)\")));"),
            "{\"end\":4,\"groups\":[\"x\",\"9\"],\"start\":2,\"text\":\"x9\"}\n"
        );
        assert_eq!(output("print(re_find(\"abc\", \"z\"));"), "nil\n");
        // Positions count characters, as len and slice do.
        assert_eq!(
            output("print(re_find(\"héllo\", \"llo\")[\"start\"]);"),
            "2\n"
        );
        // A group that took no part in the match is nil, not empty.
        assert_eq!(
            output("print(json_str(re_find(\"b\", \"(a)|(b)\")[\"groups\"]));"),
            "[null,\"b\"]\n"
        );
    }

    #[test]
    fn a_bad_pattern_names_the_builtin_and_the_position() {
        assert_eq!(
            program_err("re_find(\"x\", \"(a\");"),
            "re_find: unclosed ( at 2"
        );
        assert_eq!(
            program_err("re_test(\"x\", \"[z-a]\");"),
            "re_test: a range runs the wrong way at 4"
        );
        assert_eq!(
            program_err("re_test(1, \"a\");"),
            "re_test expects two strings, got int and string"
        );
    }

    #[test]
    fn patterns_scan_replace_and_split() {
        assert_eq!(
            output("print(len(re_find_all(\"a1 b2 c3\", \"\\\\w\\\\d\")));"),
            "3\n"
        );
        assert_eq!(
            output("print(json_str(re_split(\"a1b22c\", \"\\\\d+\")));"),
            "[\"a\",\"b\",\"c\"]\n"
        );
        assert_eq!(
            output("print(re_replace(\"a1 b2\", \"([a-z])(\\\\d)\", \"$2$1\"));"),
            "1a 2b\n"
        );
        assert_eq!(output("print(re_replace(\"x\", \"x\", \"$$\"));"), "$\n");
        // An empty match cannot advance the scan on its own.
        assert_eq!(output("print(len(re_find_all(\"abc\", \"x*\")));"), "4\n");
        assert_eq!(
            output("print(json_str(re_split(\"abc\", \"\")));"),
            "[\"\",\"a\",\"b\",\"c\",\"\"]\n"
        );
        assert_eq!(
            program_err("re_replace(\"a\", \"a\", \"$3\");"),
            "re_replace: the pattern has no group 3"
        );
    }

    #[test]
    fn a_parameter_with_a_default_may_be_left_out() {
        assert_eq!(
            output("fn f(a, b = 2) { return a + b; } print(f(1), f(1, 10));"),
            "3 11\n"
        );
        // A default is evaluated at the call, in the callee's scope, so
        // it may name an earlier parameter...
        assert_eq!(
            output("fn f(a, b = a * 2) { return b; } print(f(3), f(3, 1));"),
            "6 1\n"
        );
        // ...and a fresh list every time, which is the trap this
        // language will not have.
        assert_eq!(
            output("fn f(x, xs = []) { push(xs, x); return xs; } print(f(1), f(2));"),
            "[1] [2]\n"
        );
        // A failing default fails the call.
        assert_eq!(
            program_err("fn f(a = fail(\"no\")) { return a; } f();"),
            "no"
        );
    }

    #[test]
    fn a_rest_parameter_takes_what_is_left() {
        assert_eq!(
            output("fn f(...xs) { return xs; } print(f(), f(1, 2));"),
            "[] [1, 2]\n"
        );
        // The fixed parameters are filled first, defaults included.
        assert_eq!(
            output("fn f(a, b = 2, ...rest) { return [a, b, rest]; } print(f(1), f(1, 9, 8, 7));"),
            "[1, 2, []] [1, 9, [8, 7]]\n"
        );
        // Each call gets its own list.
        assert_eq!(
            output("fn f(...xs) { push(xs, 0); return xs; } print(f(1), f(2));"),
            "[1, 0] [2, 0]\n"
        );
        // Only the fixed parameters can be missing.
        assert_eq!(
            program_err("fn f(a, ...rest) { return a; } f();"),
            "f expects at least 1 argument, got 0"
        );
        assert_eq!(
            program_err("fn f(a, b, ...rest) { return a; } f(1);"),
            "f expects at least 2 arguments, got 1"
        );
    }

    /// Both engines record the same statements: the tree-walker from
    /// its own statement spans, the VM from a Mark the compiler put
    /// before each one. Agreement is the point — a coverage report
    /// that depends on which engine ran would be worse than none.
    #[test]
    fn both_engines_record_the_same_statements() {
        use crate::parser::parse_program;
        let src = "let n = 0;\n                   fn bump(by) { n = n + by; return n; }\n                   if bump(1) == 1 { print(\"one\"); } else { print(\"other\"); }\n                   fn never() { print(\"unreached\"); }\n";
        let program = parse_program(&lex(src).unwrap()).unwrap();

        let mut walker = Interpreter::new(Vec::new());
        walker.cover();
        walker.run(&program).unwrap();
        let walked = offsets(&walker);

        let mut vm = Interpreter::new(Vec::new());
        vm.cover();
        let Ok(chunk) = crate::compile::compile_program_covered(&program) else {
            panic!("the program compiles");
        };
        crate::vm::run_chunk(&mut vm, &chunk).unwrap();
        let ran = offsets(&vm);

        assert_eq!(walked, ran);
        // The body of `never` is the one statement that did not run.
        let unreached = src.find("print(\"unreached\")").unwrap();
        assert!(!walked.contains(&unreached), "offsets: {walked:?}");
        assert!(walked.contains(&src.find("print(\"one\")").unwrap()));
        assert!(!walked.contains(&src.find("print(\"other\")").unwrap()));
    }

    fn offsets(interp: &Interpreter<Vec<u8>>) -> Vec<usize> {
        let files = interp.coverage().expect("recording is on").files();
        assert_eq!(files.len(), 1, "one file ran");
        let mut hits: Vec<usize> = files[0].hit.iter().copied().collect();
        hits.sort_unstable();
        hits
    }

    /// Recording is off unless it is asked for, so a plain run pays
    /// nothing and reports nothing.
    #[test]
    fn coverage_is_off_by_default() {
        use crate::parser::parse_program;
        let mut interp = Interpreter::new(Vec::new());
        interp
            .run(&parse_program(&lex("print(1);").unwrap()).unwrap())
            .unwrap();
        assert!(interp.coverage().is_none());
    }

    #[test]
    fn a_spread_hands_a_list_over_as_arguments() {
        assert_eq!(output("let xs = [1, 2, 3]; print(...xs);"), "1 2 3\n");
        assert_eq!(output("let xs = [1, 2]; print(0, ...xs, 3);"), "0 1 2 3\n");
        assert_eq!(output("print(...[]);"), "\n");
        // Forwarding: what a rest parameter collects, a spread passes on.
        assert_eq!(
            output(
                "fn add(a, b) { return a + b; } fn f(...xs) { return add(...xs); } print(f(1, 2));"
            ),
            "3\n"
        );
        // The arity check still happens, against what the list held.
        assert_eq!(
            program_err("fn add(a, b) { return a + b; } add(...[1]);"),
            "add expects 2 arguments, got 1"
        );
        assert_eq!(
            program_err("print(...5);"),
            "cannot spread int, expected a list"
        );
    }

    #[test]
    fn arity_errors_name_the_range_when_there_is_one() {
        assert_eq!(
            program_err("fn f(a, b = 1) { return a; } f();"),
            "f expects 1 to 2 arguments, got 0"
        );
        assert_eq!(
            program_err("fn f(a, b = 1) { return a; } f(1, 2, 3);"),
            "f expects 1 to 2 arguments, got 3"
        );
        // A let-bound literal is named after the binding, as before.
        assert_eq!(
            program_err("fn f(a, b) { return a; } f(1);"),
            "f expects 2 arguments, got 1"
        );
        assert_eq!(
            program_err("let g = fn(a = 1) { return a; }; g(1, 2);"),
            "g expects 0 to 1 arguments, got 2"
        );
    }

    #[test]
    fn int_reads_back_every_hex_and_bin_it_writes() {
        for n in [
            "0",
            "1",
            "-1",
            "255",
            "-255",
            "1000000",
            "9223372036854775807",
        ] {
            for call in ["hex", "bin"] {
                let src = format!("int({call}({n}))");
                assert_eq!(run(&src), Value::Int(n.parse().unwrap()), "{src}");
            }
        }
        // The far end, which cannot be written as a positive literal.
        assert_eq!(
            run("int(hex(-9223372036854775807 - 1))"),
            Value::Int(i64::MIN)
        );
    }

    #[test]
    fn a_separator_or_prefix_without_digits_is_not_a_number() {
        for text in [
            "0x", "0b", "1_", "_1", "1__0", "0b12", "", " ", "0xg", "--1",
        ] {
            assert_eq!(int_from_text(text), None, "{text:?} should not convert");
        }
    }

    #[test]
    fn builtin_arity_errors() {
        assert_eq!(program_err("len();"), "len expects 1 argument, got 0");
        assert_eq!(
            program_err("range(1, 2, 3, 4);"),
            "range expects 1 to 3 arguments, got 4"
        );
    }

    #[test]
    fn map_literals_get_and_set() {
        assert_eq!(
            output("let m = {\"a\": 1, \"b\": 2}; print(m[\"a\"] + m[\"b\"]);"),
            "3\n"
        );
        assert_eq!(
            output("let m = {}; m[\"x\"] = 10; m[\"x\"] = m[\"x\"] + 1; print(m);"),
            "{\"x\": 11}\n"
        );
        assert_eq!(
            output("print({\"b\": 2, \"a\": [1, \"s\"]});"),
            "{\"a\": [1, \"s\"], \"b\": 2}\n"
        );
    }

    #[test]
    fn missing_map_key_errors() {
        assert_eq!(
            program_err("let m = {}; m[\"nope\"];"),
            "key \"nope\" not found"
        );
    }

    #[test]
    fn map_keys_must_be_strings() {
        assert_eq!(
            program_err("let m = {1: 2};"),
            "map keys must be strings, got int"
        );
        assert_eq!(
            program_err("let m = {\"a\": 1}; m[0];"),
            "cannot index map with int"
        );
    }

    #[test]
    fn list_index_assignment() {
        assert_eq!(
            output("let xs = [1, 2, 3]; xs[0] = 10; xs[-1] = 30; print(xs);"),
            "[10, 2, 30]\n"
        );
        assert_eq!(
            program_err("let xs = [1]; xs[5] = 0;"),
            "index 5 out of bounds (len 1)"
        );
    }

    #[test]
    fn nested_index_assignment() {
        assert_eq!(
            output("let m = {\"a\": {\"b\": 1}}; m[\"a\"][\"b\"] = 2; print(m[\"a\"][\"b\"]);"),
            "2\n"
        );
        assert_eq!(
            output("let grid = [[0, 0], [0, 0]]; grid[1][0] = 5; print(grid);"),
            "[[0, 0], [5, 0]]\n"
        );
    }

    #[test]
    fn lists_and_maps_are_references() {
        assert_eq!(
            output("let a = [1]; let b = a; b[0] = 2; print(a[0]);"),
            "2\n"
        );
        assert_eq!(
            output("fn poke(m) { m[\"k\"] = 1; } let m = {}; poke(m); print(m[\"k\"]);"),
            "1\n"
        );
        // concat still copies
        assert_eq!(
            output("let a = [1]; let c = a + [2]; c[0] = 9; print(a, c);"),
            "[1] [9, 2]\n"
        );
    }

    #[test]
    fn map_equality_is_structural() {
        assert_eq!(output("print({\"a\": 1} == {\"a\": 1});"), "true\n");
        assert_eq!(output("print({\"a\": 1} == {\"a\": 2});"), "false\n");
    }

    #[test]
    fn index_assign_type_errors() {
        assert_eq!(
            program_err("let s = \"abc\"; s[0] = \"x\";"),
            "cannot index-assign string with int"
        );
    }

    #[test]
    fn undefined_variable_errors() {
        assert!(run_err("x + 1").contains("undefined variable 'x'"));
    }

    #[test]
    fn let_assign_and_print() {
        assert_eq!(
            output("let x = 1; x = x + 41; print(x, \"is the answer\");"),
            "42 is the answer\n"
        );
    }

    #[test]
    fn print_with_no_args_prints_empty_line() {
        assert_eq!(output("print();"), "\n");
    }

    #[test]
    fn blocks_scope_lets_but_share_assignments() {
        // let inside a block shadows; assignment reaches the outer variable
        assert_eq!(
            output("let x = 1; { let x = 2; print(x); } print(x);"),
            "2\n1\n"
        );
        assert_eq!(output("let x = 1; { x = 2; } print(x);"), "2\n");
    }

    #[test]
    fn block_locals_do_not_leak() {
        assert!(program_err("{ let y = 1; } print(y);").contains("undefined variable 'y'"));
    }

    #[test]
    fn if_else_branches() {
        assert_eq!(
            output("let x = 5; if x > 3 { print(\"big\"); } else { print(\"small\"); }"),
            "big\n"
        );
        assert_eq!(
            output(
                "let x = 1; if x > 3 { print(\"big\"); } else if x > 0 { print(\"mid\"); } else { print(\"small\"); }"
            ),
            "mid\n"
        );
        assert_eq!(output("if false { print(\"no\"); }"), "");
    }

    #[test]
    fn while_countdown() {
        assert_eq!(
            output("let i = 3; while i > 0 { print(i); i = i - 1; }"),
            "3\n2\n1\n"
        );
    }

    #[test]
    fn while_false_never_runs() {
        assert_eq!(output("while false { print(\"no\"); }"), "");
    }

    #[test]
    fn conditions_must_be_bool() {
        assert_eq!(program_err("if 1 { print(1); }"), "expected bool, got int");
        assert_eq!(
            program_err("while \"x\" { print(1); }"),
            "expected bool, got string"
        );
    }

    #[test]
    fn assignment_to_undefined_variable_errors() {
        assert_eq!(
            program_err("x = 1;"),
            "cannot assign to undefined variable 'x'"
        );
    }

    #[test]
    fn shadowed_print_is_not_callable() {
        assert_eq!(
            program_err("let print = 1; print(2);"),
            "int is not callable"
        );
    }

    #[test]
    fn function_declaration_and_call() {
        assert_eq!(
            output("fn add(a, b) { return a + b; } print(add(2, 40));"),
            "42\n"
        );
    }

    #[test]
    fn recursion() {
        assert_eq!(
            output(
                "fn fib(n) { if n < 2 { return n; } return fib(n - 1) + fib(n - 2); } print(fib(15));"
            ),
            "610\n"
        );
    }

    #[test]
    fn closures_capture_and_mutate() {
        assert_eq!(
            output(
                "fn counter() { let n = 0; fn inc() { n = n + 1; return n; } return inc; } \
                 let c = counter(); print(c(), c(), c());"
            ),
            "1 2 3\n"
        );
    }

    #[test]
    fn closures_are_independent() {
        assert_eq!(
            output(
                "fn counter() { let n = 0; fn inc() { n = n + 1; return n; } return inc; } \
                 let a = counter(); let b = counter(); print(a(), a(), b());"
            ),
            "1 2 1\n"
        );
    }

    #[test]
    fn anonymous_functions_and_higher_order() {
        assert_eq!(
            output(
                "let twice = fn(f, x) { return f(f(x)); }; print(twice(fn(n) { return n * 3; }, 2));"
            ),
            "18\n"
        );
    }

    #[test]
    fn falling_off_the_end_returns_nil() {
        assert_eq!(output("fn f() { 1; } print(f());"), "nil\n");
        assert_eq!(output("fn f() { return; } print(f());"), "nil\n");
    }

    #[test]
    fn return_stops_a_loop_inside_a_function() {
        assert_eq!(
            output(
                "fn first() { let i = 0; while true { if i == 3 { return i; } i = i + 1; } } print(first());"
            ),
            "3\n"
        );
    }

    #[test]
    fn arity_mismatch_errors() {
        assert_eq!(
            program_err("fn f(a, b) { return a; } f(1);"),
            "f expects 2 arguments, got 1"
        );
    }

    #[test]
    fn return_outside_function_errors() {
        assert_eq!(program_err("return 1;"), "return outside function");
    }

    #[test]
    fn runaway_recursion_is_trapped() {
        assert_eq!(
            program_err("fn f() { return f(); } f();"),
            "stack overflow (max call depth 200)"
        );
    }

    #[test]
    fn functions_display_and_compare_by_identity() {
        assert_eq!(output("fn f(a, b) { } print(f);"), "<fn(a, b)>\n");
        assert_eq!(output("fn f() { } let g = f; print(f == g);"), "true\n");
        assert_eq!(
            output("let a = fn() { }; let b = fn() { }; print(a == b);"),
            "false\n"
        );
    }

    #[test]
    fn nan_comparisons_are_false() {
        assert_eq!(run("(0.0 / 0.0) < 1.0"), Value::Bool(false));
        assert_eq!(run("(0.0 / 0.0) >= 1.0"), Value::Bool(false));
    }
}
