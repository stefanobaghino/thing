//! Interactive REPL: std-only, persistent session, multi-line input.

use crate::diag;
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
        Err(e) => return Outcome::Error(diag::render("repl", src, &e.message, e.span)),
    };
    if let Ok(expr) = parser::parse_expr(&tokens) {
        return match interp.eval(&expr) {
            Ok(Value::Nil) => Outcome::Unit,
            Ok(v) => Outcome::Value(render(&v)),
            Err(e) => Outcome::Error(diag::render("repl", src, &e.message, e.span)),
        };
    }
    match parser::parse_program(&tokens) {
        Ok(prog) => run_program(interp, src, &prog),
        Err(e) if e.message.ends_with("found end of input") => {
            if let Ok(t2) = lexer::lex(&format!("{src};"))
                && let Ok(prog) = parser::parse_program(&t2)
            {
                return run_program(interp, src, &prog);
            }
            Outcome::Incomplete
        }
        Err(e) => Outcome::Error(diag::render("repl", src, &e.message, e.span)),
    }
}

fn run_program<W: Write>(
    interp: &mut Interpreter<W>,
    src: &str,
    prog: &[crate::ast::Stmt],
) -> Outcome {
    match interp.run(prog) {
        Ok(()) => Outcome::Unit,
        Err(e) => Outcome::Error(diag::render("repl", src, &e.message, e.span)),
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

/// All REPL output goes through here so a reader that went away
/// (`echo ':help' | ting | head`) ends the session quietly instead of
/// panicking inside the standard print macros, matching what print()
/// does in scripts.
fn emit(text: &str) {
    let mut out = std::io::stdout().lock();
    if let Err(e) = out.write_all(text.as_bytes()) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("ting: {e}");
        std::process::exit(1);
    }
}

fn say(text: &str) {
    emit(text);
    emit("\n");
}

/// `:doc NAME` — a builtin's signature and doc line, or a stdlib
/// function's module, signature and leading comment (every embedded
/// module is searched, imported or not).
fn print_doc(name: &str) {
    if let Some(b) = crate::value::Builtin::ALL.iter().find(|b| b.name() == name) {
        let (sig, text) = b.doc();
        say(&format!("{sig}\n  {text}"));
        return;
    }
    // A source that imports every module makes the LSP's scanner
    // return all stdlib functions.
    let everything: String = crate::eval::embedded_stdlib()
        .iter()
        .map(|(path, _)| format!("import(\"{path}\");\n"))
        .collect();
    let hits: Vec<_> = crate::lsp::imported_stdlib_functions(&everything)
        .into_iter()
        .filter(|(_, n, _, _)| n == name)
        .collect();
    if hits.is_empty() {
        say(&format!("(no builtin or stdlib function named {name})"));
    }
    for (path, _, sig, comment) in hits {
        say(&format!("{sig}  [{path}]"));
        if !comment.is_empty() {
            say(&format!("  {comment}"));
        }
    }
}

/// `:help` — every builtin's signature and one-liner, in name order.
fn print_help() {
    let mut docs: Vec<_> = crate::value::Builtin::ALL.iter().map(|b| b.doc()).collect();
    docs.sort();
    let width = docs.iter().map(|(sig, _)| sig.len()).max().unwrap_or(0);
    for (sig, text) in docs {
        say(&format!("{sig:width$}  {text}"));
    }
    say(
        "(:doc NAME explains a builtin or stdlib function; :vars bindings; :load <file> runs a file here; :fmt reprints the last chunk formatted; :clear resets; ctrl-d exits)",
    );
}

fn run_inner() -> ExitCode {
    let stdin = std::io::stdin();
    let tty = stdin.is_terminal();
    if tty {
        say(&format!(
            "ting {} — :help, :doc NAME, :vars, :load <file>, :fmt, :clear",
            env!("CARGO_PKG_VERSION")
        ));
    }
    let mut interp = Interpreter::new(std::io::stdout());
    let mut buffer = String::new();
    // The last complete chunk that was evaluated, for :fmt.
    let mut last = String::new();
    loop {
        if tty {
            emit(if buffer.is_empty() { "> " } else { ".. " });
            std::io::stdout().flush().ok();
        }
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                if tty {
                    say("");
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
        // Meta-commands: only at the start of a fresh chunk.
        if buffer.is_empty() && line.trim() == ":help" {
            print_help();
            continue;
        }
        if buffer.is_empty() && line.trim() == ":clear" {
            interp = Interpreter::new(std::io::stdout());
            say("(session cleared)");
            continue;
        }
        if buffer.is_empty()
            && let Some(name) = line.trim().strip_prefix(":doc ")
        {
            print_doc(name.trim());
            continue;
        }
        if buffer.is_empty() && line.trim() == ":fmt" {
            if last.is_empty() {
                say("(nothing to format yet)");
            } else {
                match crate::fmt::format(&last) {
                    Ok(formatted) => emit(&formatted),
                    Err(e) => eprintln!("{}", diag::render("repl", &last, &e.message, e.span)),
                }
            }
            continue;
        }
        if buffer.is_empty() && line.trim() == ":vars" {
            let bindings = interp.user_bindings();
            if bindings.is_empty() {
                say("(no bindings yet)");
            }
            for (name, ty) in bindings {
                say(&format!("{name}: {ty}"));
            }
            continue;
        }
        if buffer.is_empty()
            && let Some(path) = line.trim().strip_prefix(":load ")
        {
            match std::fs::read_to_string(path.trim()) {
                Ok(src) => match eval_chunk(&mut interp, &src) {
                    Outcome::Incomplete => eprintln!("ting: {path}: incomplete program"),
                    Outcome::Unit => {}
                    Outcome::Value(v) => say(&v.to_string()),
                    Outcome::Error(msg) => eprintln!("{msg}"),
                },
                Err(e) => eprintln!("ting: cannot read {path}: {e}"),
            }
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
            Outcome::Value(s) => say(&s),
            Outcome::Error(msg) => eprintln!("{msg}"),
        }
        last = std::mem::take(&mut buffer);
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
    fn errors_are_rendered_with_carets_and_session_survives() {
        let mut i = fresh();
        assert_eq!(
            eval_chunk(&mut i, "xyz"),
            Outcome::Error("repl:1:1: error: undefined variable 'xyz'\n 1 | xyz\n   | ^^^".into())
        );
        match eval_chunk(&mut i, "1 = 2;") {
            Outcome::Error(msg) => assert!(msg.contains("invalid assignment target")),
            other => panic!("expected error, got {other:?}"),
        }
        assert_eq!(eval_chunk(&mut i, "2 + 2"), Outcome::Value("4".into()));
    }

    #[test]
    fn multiline_chunk_errors_point_at_the_right_line() {
        let mut i = fresh();
        match eval_chunk(&mut i, "fn f() {\n  return zzz;\n}\nf()") {
            Outcome::Error(msg) => {
                assert!(
                    msg.starts_with("repl:2:10: error: undefined variable 'zzz'"),
                    "got:\n{msg}"
                );
                assert!(msg.contains("  return zzz;"), "got:\n{msg}");
            }
            other => panic!("expected error, got {other:?}"),
        }
    }

    #[test]
    fn prints_go_to_the_interpreter_writer() {
        let mut i = fresh();
        eval_chunk(&mut i, "print(\"hi\");");
        assert_eq!(String::from_utf8(i.into_out()).unwrap(), "hi\n");
    }
}
