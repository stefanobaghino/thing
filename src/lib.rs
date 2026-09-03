//! The ting interpreter as a library: the `ting` binary and the wasm
//! playground both link this same engine.

pub mod ast;
pub mod compile;
pub mod diag;
pub mod eval;
pub mod fmt;
pub mod json;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod repl;
pub mod value;
pub mod vm;
pub mod wasm;

use std::io::Write;

/// Which execution engine runs the program. `Eval` (the tree-walker)
/// is the reference implementation; `Vm` is the bytecode engine being
/// brought to parity (see docs/vm.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Eval,
    Vm,
}

/// Lex, parse, and run a whole program, writing its output to `out`.
/// On any error, returns the fully rendered caret diagnostic (with
/// `path` as the file name in the header). Runs on the default engine
/// (the VM; the wasm playground goes through here too).
pub fn run_source<W: Write>(
    path: &str,
    src: &str,
    out: W,
    script_args: Vec<String>,
) -> Result<(), String> {
    run_source_engine(Engine::Vm, path, src, out, script_args)
}

/// Lex, parse, and compile without running: everything that can be
/// diagnosed statically. Returns the rendered diagnostic on failure.
/// The files a source imports, resolved relative to its own directory
/// (`.` and `..` normalised lexically); only paths that exist on disk,
/// so embedded stdlib modules are not among them.
pub fn local_imports(path: &str, src: &str) -> Vec<std::path::PathBuf> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    let dir = std::path::Path::new(path)
        .parent()
        .map(|d| d.to_path_buf())
        .unwrap_or_default();
    lsp::import_targets(&tokens, &dir)
        .into_iter()
        .map(|(_, target)| target)
        .collect()
}

pub fn check_source(path: &str, src: &str) -> Result<(), String> {
    let render = |m: &str, s: lexer::Span| diag::render(path, src, m, s);
    let tokens = lexer::lex(src).map_err(|e| render(&e.message, e.span))?;
    let program = parser::parse_program(&tokens).map_err(|e| render(&e.message, e.span))?;
    compile::compile_program(&program).map_err(|e| render(&e.message, e.span))?;
    Ok(())
}

/// Semantic warnings for a source that already checks clean: rendered
/// diagnostics (level "warning") for stdlib member names that the
/// imported module does not export and for top-level bindings that
/// are never used. Shared with the LSP, which publishes the same
/// findings as warnings.
pub fn check_warnings(path: &str, src: &str) -> Vec<String> {
    lsp::warnings(src)
        .into_iter()
        .map(|(start, end, message)| {
            diag::render_level(path, src, "warning", &message, lexer::Span { start, end })
        })
        .collect()
}

/// `run_source` with an explicit engine choice.
pub fn run_source_engine<W: Write>(
    engine: Engine,
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
    if let Some(dir) = std::path::Path::new(path).parent() {
        interp.set_base_dir(dir.to_path_buf());
    }
    match engine {
        Engine::Eval => interp.run(&program).map_err(|e| e.render(path, src)),
        Engine::Vm => {
            let chunk =
                compile::compile_program(&program).map_err(|e| render(&e.message, e.span))?;
            vm::run_chunk(&mut interp, &chunk)
                .map(|_| ())
                .map_err(|e| e.render(path, src))
        }
    }
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
    fn check_source_reports_static_errors_only() {
        check_source("t", "print(x);").unwrap();
        let err = check_source("t", "let = 3;").unwrap_err();
        assert!(err.contains("error"), "got: {err}");
    }

    #[test]
    fn run_source_renders_diagnostics() {
        let err = run_source("t", "print(x);", Vec::new(), Vec::new()).unwrap_err();
        assert!(err.starts_with("t:1:7: error: undefined variable 'x'"));
    }
}
