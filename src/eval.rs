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
}

pub(crate) fn error(message: impl Into<String>, span: Span) -> RuntimeError {
    RuntimeError {
        message: message.into(),
        span,
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
    pub params: Vec<Rc<str>>,
    pub body: FnBody,
    pub env: Rc<RefCell<Env>>,
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

/// Call-depth cap; ting recursion consumes the host stack, so trap it
/// before Rust's stack overflows. 200 fits comfortably in a 2MB thread
/// stack even in debug builds (tests run on such threads).
const MAX_DEPTH: usize = 200;

pub struct Interpreter<W: Write> {
    env: Rc<RefCell<Env>>,
    out: W,
    depth: usize,
    script_args: Vec<String>,
    /// Directory import paths resolve against; the top is the directory
    /// of the file currently executing (script, or module mid-import).
    dir_stack: Vec<std::path::PathBuf>,
    import_cache: HashMap<std::path::PathBuf, Value>,
    importing: Vec<std::path::PathBuf>,
}

/// The standard library, baked into the binary at build time (always
/// in sync with lib/ by construction). import() falls back to these
/// when no matching file exists.
const EMBEDDED_STDLIB: &[(&str, &str)] = &[
    ("lib/list.ting", include_str!("../lib/list.ting")),
    ("lib/map.ting", include_str!("../lib/map.ting")),
    ("lib/math.ting", include_str!("../lib/math.ting")),
    ("lib/string.ting", include_str!("../lib/string.ting")),
    ("lib/test.ting", include_str!("../lib/test.ting")),
];

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
            script_args: Vec::new(),
            dir_stack: vec![std::path::PathBuf::new()],
            import_cache: HashMap::new(),
            importing: Vec::new(),
        }
    }

    /// Command-line arguments exposed to the script via `args()`.
    pub fn set_args(&mut self, args: Vec<String>) {
        self.script_args = args;
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
        match &stmt.kind {
            StmtKind::Let(name, init) => {
                let v = self.eval(init)?;
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
                    Err(error(
                        format!("cannot assign to undefined variable '{name}'"),
                        stmt.span,
                    ))
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

    fn call_builtin(
        &mut self,
        b: Builtin,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let arity = |lo: usize, hi: usize| -> Result<(), RuntimeError> {
            if args.len() < lo || args.len() > hi {
                let want = if lo == hi {
                    format!("{lo}")
                } else {
                    format!("{lo} to {hi}")
                };
                Err(error(
                    format!(
                        "{} expects {want} argument(s), got {}",
                        b.name(),
                        args.len()
                    ),
                    span,
                ))
            } else {
                Ok(())
            }
        };
        match b {
            Builtin::Print => {
                let parts: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                writeln!(self.out, "{}", parts.join(" "))
                    .map_err(|e| error(format!("print failed: {e}"), span))?;
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
                    Value::Float(x) => Ok(Value::Int(*x as i64)),
                    Value::Str(s) => s
                        .trim()
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| error(format!("cannot convert {s:?} to int"), span)),
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
                    Value::Str(s) => s
                        .trim()
                        .parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| error(format!("cannot convert {s:?} to float"), span)),
                    v => Err(error(
                        format!("cannot convert {} to float", v.type_name()),
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
                        return Err(error("range expects int argument(s)", span));
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
                arity(2, 2)?;
                match (&args[0], &args[1]) {
                    (Value::Str(path), Value::Str(s)) => std::fs::write(path, s)
                        .map(|_| Value::Nil)
                        .map_err(|e| error(format!("cannot write {path:?}: {e}"), span)),
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
                            Ok(v) => m.insert("ok".to_string(), v),
                            Err(e) => m.insert("err".to_string(), Value::Str(e.message)),
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
                            "format: {} placeholder(s) but {} value argument(s)",
                            next - 1,
                            args.len() - 1
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
                    return Err(error(format!("cannot import {path:?}: {e}"), span));
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

        let saved_env = std::mem::replace(&mut self.env, global_env());
        self.importing.push(resolved.clone());
        self.dir_stack.push(
            resolved
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default(),
        );
        let result = self.run(&program);
        self.dir_stack.pop();
        self.importing.pop();
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
        if args.len() != func.params.len() {
            return Err(error(
                format!(
                    "expected {} argument(s), got {}",
                    func.params.len(),
                    args.len()
                ),
                span,
            ));
        }
        if self.depth >= MAX_DEPTH {
            return Err(error(
                format!("stack overflow (max call depth {MAX_DEPTH})"),
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
        let saved = std::mem::replace(&mut self.env, frame);
        self.depth += 1;
        let result = match &func.body {
            FnBody::Ast(stmts) => self.run_block(stmts).map(ControlOrValue::Control),
            // Compiled bodies cannot leak break/continue (the compiler
            // rejects them), so the VM returns a plain value.
            FnBody::Chunk(chunk) => {
                crate::vm::run_chunk_with(self, chunk, &mut locals).map(ControlOrValue::Value)
            }
        };
        self.depth -= 1;
        self.env = saved;
        crate::vm::give_buf(locals);
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
                None => Err(error(format!("undefined variable '{name}'"), expr.span)),
            },
            ExprKind::Call(callee, args) => {
                let callee_v = self.eval(callee)?;
                let mut arg_vals = Vec::with_capacity(args.len());
                for a in args {
                    arg_vals.push(self.eval(a)?);
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
            ExprKind::Fn(params, body) => Ok(Value::Fn(Rc::new(Function {
                params: params.iter().map(|p| Rc::from(p.as_str())).collect(),
                body: FnBody::Ast(Rc::clone(body)),
                env: Rc::clone(&self.env),
            }))),
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

pub(crate) fn unary(op: UnaryOp, v: Value, span: Span) -> Result<Value, RuntimeError> {
    match (op, v) {
        (UnaryOp::Neg, Value::Int(n)) => n
            .checked_neg()
            .map(Value::Int)
            .ok_or_else(|| error("integer overflow", span)),
        (UnaryOp::Neg, Value::Float(x)) => Ok(Value::Float(-x)),
        (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
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
        _ => Err(error(
            format!(
                "cannot apply '{op}' to {} and {}",
                l.type_name(),
                r.type_name()
            ),
            span,
        )),
    }
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
        (Value::Map(entries), Value::Str(k)) => match entries.borrow().get(&k) {
            Some(v) => Ok(v.clone()),
            None => Err(error(format!("key {k:?} not found"), span)),
        },
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
        assert_eq!(
            output("print(try(fn() { return 1 / 0; }));"),
            "{\"err\": \"division by zero\"}\n"
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

    #[test]
    fn builtin_arity_errors() {
        assert_eq!(program_err("len();"), "len expects 1 argument(s), got 0");
        assert_eq!(
            program_err("range(1, 2, 3, 4);"),
            "range expects 1 to 3 argument(s), got 4"
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
            "expected 2 argument(s), got 1"
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
