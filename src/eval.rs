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

fn error(message: impl Into<String>, span: Span) -> RuntimeError {
    RuntimeError {
        message: message.into(),
        span,
    }
}

/// A lexical environment: closures keep their defining Env alive via Rc,
/// and RefCell lets assignments through a closure be seen everywhere.
#[derive(Debug)]
pub struct Env {
    vars: HashMap<String, Value>,
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

/// A user-defined function: parameters, body, and the captured environment.
#[derive(Debug)]
pub struct Function {
    pub params: Vec<String>,
    pub body: Rc<Vec<Stmt>>,
    pub env: Rc<RefCell<Env>>,
}

/// How a statement finished: fell through, or hit `return`.
enum Control {
    Normal,
    Return(Value, Span),
}

/// Call-depth cap; ting recursion consumes the host stack, so trap it
/// before Rust's stack overflows. 200 fits comfortably in a 2MB thread
/// stack even in debug builds (tests run on such threads).
const MAX_DEPTH: usize = 200;

pub struct Interpreter<W: Write> {
    env: Rc<RefCell<Env>>,
    out: W,
    depth: usize,
}

impl<W: Write> Interpreter<W> {
    pub fn new(out: W) -> Self {
        let mut globals = HashMap::new();
        for b in Builtin::ALL {
            globals.insert(b.name().to_string(), Value::Builtin(b));
        }
        Interpreter {
            env: Rc::new(RefCell::new(Env {
                vars: globals,
                parent: None,
            })),
            out,
            depth: 0,
        }
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
                self.env.borrow_mut().vars.insert(name.clone(), v);
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
                let saved = Rc::clone(&self.env);
                self.env = Env::child(&saved);
                let result = self.run_block(stmts);
                self.env = saved;
                result
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
                        Control::Normal => {}
                        ret => return Ok(ret),
                    }
                }
                Ok(Control::Normal)
            }
            StmtKind::Return(value) => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Nil,
                };
                Ok(Control::Return(v, stmt.span))
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<Value> {
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
                arity(1, 2)?;
                let (lo, hi) = match args.as_slice() {
                    [Value::Int(hi)] => (0, *hi),
                    [Value::Int(lo), Value::Int(hi)] => (*lo, *hi),
                    _ => {
                        return Err(error("range expects int argument(s)", span));
                    }
                };
                Ok(Value::list((lo..hi).map(Value::Int).collect()))
            }
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
        let frame = Env::child(&func.env);
        for (p, a) in func.params.iter().zip(args) {
            frame.borrow_mut().vars.insert(p.clone(), a);
        }
        let saved = std::mem::replace(&mut self.env, frame);
        self.depth += 1;
        let result = self.run_block(&func.body);
        self.depth -= 1;
        self.env = saved;
        match result? {
            Control::Return(v, _) => Ok(v),
            Control::Normal => Ok(Value::Nil),
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
                params: params.clone(),
                body: Rc::clone(body),
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

fn as_bool(v: Value, span: Span) -> Result<bool, RuntimeError> {
    match v {
        Value::Bool(b) => Ok(b),
        other => Err(error(
            format!("expected bool, got {}", other.type_name()),
            span,
        )),
    }
}

fn unary(op: UnaryOp, v: Value, span: Span) -> Result<Value, RuntimeError> {
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

fn binary(op: BinaryOp, l: Value, r: Value, span: Span) -> Result<Value, RuntimeError> {
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
    match (l, r) {
        (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => *a as f64 == *b,
        (l, r) => l == r,
    }
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
fn effective_index(i: i64, len: usize, span: Span) -> Result<usize, RuntimeError> {
    let len = len as i64;
    let eff = if i < 0 { i + len } else { i };
    if eff < 0 || eff >= len {
        Err(error(format!("index {i} out of bounds (len {len})"), span))
    } else {
        Ok(eff as usize)
    }
}

fn index(base: Value, idx: Value, span: Span) -> Result<Value, RuntimeError> {
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
    fn builtin_arity_errors() {
        assert_eq!(program_err("len();"), "len expects 1 argument(s), got 0");
        assert_eq!(
            program_err("range(1, 2, 3);"),
            "range expects 1 to 2 argument(s), got 3"
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
