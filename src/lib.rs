//! The ting interpreter as a library: the `ting` binary and the wasm
//! playground both link this same engine.

pub mod ast;
pub mod diag;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod repl;
pub mod value;
pub mod wasm;

use std::io::Write;

/// Lex, parse, and run a whole program, writing its output to `out`.
/// On any error, returns the fully rendered caret diagnostic (with
/// `path` as the file name in the header).
pub fn run_source<W: Write>(
    path: &str,
    src: &str,
    out: W,
    script_args: Vec<String>,
) -> Result<(), String> {
    let render = |m: &str, s: lexer::Span| diag::render(path, src, m, s);
    let tokens = lexer::lex(src).map_err(|e| render(&e.message, e.span))?;
    let program = parser::parse_program(&tokens).map_err(|e| render(&e.message, e.span))?;
    let mut interp = eval::Interpreter::new(out);
    interp.set_args(script_args);
    interp.run(&program).map_err(|e| render(&e.message, e.span))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_source_captures_output() {
        let mut out = Vec::new();
        run_source("t", "print(6 * 7);", &mut out, Vec::new()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "42\n");
    }

    #[test]
    fn run_source_renders_diagnostics() {
        let err = run_source("t", "print(x);", Vec::new(), Vec::new()).unwrap_err();
        assert!(err.starts_with("t:1:7: error: undefined variable 'x'"));
    }
}
