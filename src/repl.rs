//! Interactive REPL: std-only, persistent session, multi-line input.

use crate::eval::Interpreter;
use crate::lexer;
use crate::parser;
use crate::value::Value;
use std::io::{BufRead, IsTerminal, Write};
use std::process::ExitCode;

/// What became of one accumulated input chunk.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Syntactically unfinished — keep reading lines.
    Incomplete,
    /// Ran fine, nothing to echo.
    Unit,
    /// A bare expression: its rendered value.
    Value(String),
    Error(String),
}

/// Evaluate one chunk of REPL input against a live interpreter.
///
/// A chunk that parses as a single expression is echoed; otherwise it runs
/// as statements. A parse that dies at end-of-input first retries with a
/// `;` appended (so `let x = 1` works), then reports Incomplete to request
/// another line.
pub fn eval_chunk<W: Write>(interp: &mut Interpreter<W>, src: &str) -> Outcome {
    let tokens = match lexer::lex(src) {
        Ok(t) => t,
        Err(e) => return Outcome::Error(e.message),
    };
    if let Ok(expr) = parser::parse_expr(&tokens) {
        return match interp.eval(&expr) {
            Ok(Value::Nil) => Outcome::Unit,
            Ok(v) => Outcome::Value(render(&v)),
            Err(e) => Outcome::Error(e.message),
        };
    }
    match parser::parse_program(&tokens) {
        Ok(prog) => run_program(interp, &prog),
        Err(e) if e.message.ends_with("found end of input") => {
            if let Ok(t2) = lexer::lex(&format!("{src};"))
                && let Ok(prog) = parser::parse_program(&t2)
            {
                return run_program(interp, &prog);
            }
            Outcome::Incomplete
        }
        Err(e) => Outcome::Error(e.message),
    }
}

fn run_program<W: Write>(interp: &mut Interpreter<W>, prog: &[crate::ast::Stmt]) -> Outcome {
    match interp.run(prog) {
        Ok(()) => Outcome::Unit,
        Err(e) => Outcome::Error(e.message),
    }
}

/// Echoed values quote strings, so `"a" + "b"` shows as `"ab"`.
fn render(v: &Value) -> String {
    match v {
        Value::Str(s) => format!("{s:?}"),
        v => v.to_string(),
    }
}

pub fn run() -> ExitCode {
    // Same big-stack thread as script execution (deep ting recursion).
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(run_inner)
        .expect("failed to spawn REPL thread")
        .join()
        .expect("REPL thread panicked")
}

fn run_inner() -> ExitCode {
    let stdin = std::io::stdin();
    let tty = stdin.is_terminal();
    if tty {
        println!(
            "ting {} — empty line cancels multi-line input, ctrl-d exits",
            env!("CARGO_PKG_VERSION")
        );
    }
    let mut interp = Interpreter::new(std::io::stdout());
    let mut buffer = String::new();
    loop {
        if tty {
            print!("{}", if buffer.is_empty() { "> " } else { ".. " });
            std::io::stdout().flush().ok();
        }
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                if tty {
                    println!();
                }
                return ExitCode::SUCCESS;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("ting: {e}");
                return ExitCode::FAILURE;
            }
        }
        if !buffer.is_empty() && line.trim().is_empty() {
            buffer.clear();
            continue;
        }
        buffer.push_str(&line);
        if buffer.trim().is_empty() {
            buffer.clear();
            continue;
        }
        match eval_chunk(&mut interp, &buffer) {
            Outcome::Incomplete => continue,
            Outcome::Unit => {}
            Outcome::Value(s) => println!("{s}"),
            Outcome::Error(msg) => eprintln!("error: {msg}"),
        }
        buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Interpreter<Vec<u8>> {
        Interpreter::new(Vec::new())
    }

    #[test]
    fn expressions_echo_their_value() {
        let mut i = fresh();
        assert_eq!(eval_chunk(&mut i, "1 + 2"), Outcome::Value("3".into()));
        assert_eq!(
            eval_chunk(&mut i, "\"a\" + \"b\""),
            Outcome::Value("\"ab\"".into())
        );
        assert_eq!(eval_chunk(&mut i, "{}"), Outcome::Value("{}".into()));
    }

    #[test]
    fn nil_expressions_do_not_echo() {
        let mut i = fresh();
        assert_eq!(eval_chunk(&mut i, "nil"), Outcome::Unit);
        assert_eq!(eval_chunk(&mut i, "print(7)"), Outcome::Unit);
    }

    #[test]
    fn state_persists_across_chunks() {
        let mut i = fresh();
        assert_eq!(eval_chunk(&mut i, "let x = 40;"), Outcome::Unit);
        assert_eq!(eval_chunk(&mut i, "x = x + 2;"), Outcome::Unit);
        assert_eq!(eval_chunk(&mut i, "x"), Outcome::Value("42".into()));
    }

    #[test]
    fn missing_final_semicolon_is_forgiven() {
        let mut i = fresh();
        assert_eq!(eval_chunk(&mut i, "let y = 1"), Outcome::Unit);
        assert_eq!(eval_chunk(&mut i, "y"), Outcome::Value("1".into()));
    }

    #[test]
    fn open_constructs_request_more_input() {
        let mut i = fresh();
        assert_eq!(eval_chunk(&mut i, "fn f() {"), Outcome::Incomplete);
        assert_eq!(eval_chunk(&mut i, "if true {"), Outcome::Incomplete);
        assert_eq!(eval_chunk(&mut i, "1 +"), Outcome::Incomplete);
    }

    #[test]
    fn completed_multiline_function_runs() {
        let mut i = fresh();
        let chunk = "fn double(x) {\n  return x * 2;\n}";
        assert_eq!(eval_chunk(&mut i, chunk), Outcome::Unit);
        assert_eq!(
            eval_chunk(&mut i, "double(21)"),
            Outcome::Value("42".into())
        );
    }

    #[test]
    fn errors_are_reported_and_session_survives() {
        let mut i = fresh();
        assert_eq!(
            eval_chunk(&mut i, "xyz"),
            Outcome::Error("undefined variable 'xyz'".into())
        );
        assert_eq!(
            eval_chunk(&mut i, "1 = 2;"),
            Outcome::Error("invalid assignment target".into())
        );
        assert_eq!(eval_chunk(&mut i, "2 + 2"), Outcome::Value("4".into()));
    }

    #[test]
    fn prints_go_to_the_interpreter_writer() {
        let mut i = fresh();
        eval_chunk(&mut i, "print(\"hi\");");
        assert_eq!(String::from_utf8(i.into_out()).unwrap(), "hi\n");
    }
}
