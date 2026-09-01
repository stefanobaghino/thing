//! Tree-walking interpreter for ting.

use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::lexer::Span;
use crate::value::Value;
use std::collections::HashMap;
use std::io::Write;

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

pub struct Interpreter<W: Write> {
    /// Innermost scope last; index 0 is the global scope.
    scopes: Vec<HashMap<String, Value>>,
    out: W,
}

impl<W: Write> Interpreter<W> {
    pub fn new(out: W) -> Self {
        Interpreter {
            scopes: vec![HashMap::new()],
            out,
        }
    }

    pub fn run(&mut self, stmts: &[Stmt]) -> Result<(), RuntimeError> {
        for s in stmts {
            self.exec(s)?;
        }
        Ok(())
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<(), RuntimeError> {
        match &stmt.kind {
            StmtKind::Let(name, init) => {
                let v = self.eval(init)?;
                self.scopes
                    .last_mut()
                    .expect("scope stack is never empty")
                    .insert(name.clone(), v);
                Ok(())
            }
            StmtKind::Assign(name, value) => {
                let v = self.eval(value)?;
                for scope in self.scopes.iter_mut().rev() {
                    if let Some(slot) = scope.get_mut(name) {
                        *slot = v;
                        return Ok(());
                    }
                }
                Err(error(
                    format!("cannot assign to undefined variable '{name}'"),
                    stmt.span,
                ))
            }
            StmtKind::Expr(e) => {
                self.eval(e)?;
                Ok(())
            }
            StmtKind::Block(stmts) => {
                self.scopes.push(HashMap::new());
                let result = self.run(stmts);
                self.scopes.pop();
                result
            }
            StmtKind::If(cond, then, els) => {
                if as_bool(self.eval(cond)?, cond.span)? {
                    self.exec(then)
                } else if let Some(els) = els {
                    self.exec(els)
                } else {
                    Ok(())
                }
            }
            StmtKind::While(cond, body) => {
                while as_bool(self.eval(cond)?, cond.span)? {
                    self.exec(body)?;
                }
                Ok(())
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
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
                Ok(Value::List(vals))
            }
            ExprKind::Var(name) => match self.lookup(name) {
                Some(v) => Ok(v.clone()),
                None => Err(error(format!("undefined variable '{name}'"), expr.span)),
            },
            ExprKind::Call(callee, args) => {
                // Until user functions land, the only callable is the
                // built-in `print` (unless shadowed by a variable).
                if let ExprKind::Var(name) = &callee.kind
                    && name == "print"
                    && self.lookup("print").is_none()
                {
                    let mut parts = Vec::with_capacity(args.len());
                    for a in args {
                        parts.push(self.eval(a)?.to_string());
                    }
                    writeln!(self.out, "{}", parts.join(" "))
                        .map_err(|e| error(format!("print failed: {e}"), expr.span))?;
                    return Ok(Value::Nil);
                }
                Err(error("functions are not implemented yet", expr.span))
            }
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
            (List(mut a), List(b)) => {
                a.extend(b);
                Ok(List(a))
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
                format!(
                    "cannot compare {} and {}",
                    l.type_name(),
                    r.type_name()
                ),
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

fn index(base: Value, idx: Value, span: Span) -> Result<Value, RuntimeError> {
    match (base, idx) {
        (Value::List(items), Value::Int(i)) => {
            let len = items.len() as i64;
            // Negative indices count from the end, Python-style.
            let eff = if i < 0 { i + len } else { i };
            if eff < 0 || eff >= len {
                Err(error(format!("index {i} out of bounds (len {len})"), span))
            } else {
                Ok(items[eff as usize].clone())
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
            format!(
                "cannot index {} with {}",
                base.type_name(),
                idx.type_name()
            ),
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
        interp.eval(&parse_expr(&lex(src).unwrap()).unwrap()).unwrap()
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
            Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
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
        assert_eq!(
            output("let x = 1; { x = 2; } print(x);"),
            "2\n"
        );
    }

    #[test]
    fn block_locals_do_not_leak() {
        assert!(
            program_err("{ let y = 1; } print(y);").contains("undefined variable 'y'")
        );
    }

    #[test]
    fn if_else_branches() {
        assert_eq!(
            output("let x = 5; if x > 3 { print(\"big\"); } else { print(\"small\"); }"),
            "big\n"
        );
        assert_eq!(
            output("let x = 1; if x > 3 { print(\"big\"); } else if x > 0 { print(\"mid\"); } else { print(\"small\"); }"),
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
    fn shadowed_print_is_not_callable_yet() {
        assert_eq!(
            program_err("let print = 1; print(2);"),
            "functions are not implemented yet"
        );
    }

    #[test]
    fn nan_comparisons_are_false() {
        assert_eq!(run("(0.0 / 0.0) < 1.0"), Value::Bool(false));
        assert_eq!(run("(0.0 / 0.0) >= 1.0"), Value::Bool(false));
    }
}
