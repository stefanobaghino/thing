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
pub mod regex;
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
    run_source_profiled(engine, path, src, out, script_args, false).0
}

/// `run_source_engine`, optionally counting what every function did:
/// the second half of the answer is the profile table, present only
/// when `profile` asked for it. The run's own result is unchanged by
/// it, and a failed run still reports what it managed to do.
/// What a run should report on besides its own output.
#[derive(Default, Clone, Copy)]
pub struct Reports {
    pub profile: bool,
    pub coverage: bool,
}

pub fn run_source_profiled<W: Write>(
    engine: Engine,
    path: &str,
    src: &str,
    out: W,
    script_args: Vec<String>,
    profile: bool,
) -> (Result<(), String>, Option<String>) {
    run_source_reported(
        engine,
        path,
        src,
        out,
        script_args,
        Reports {
            profile,
            coverage: false,
        },
    )
}

/// Run several scripts, each in its own interpreter — separate
/// globals, as running them one after another means — but sharing one
/// coverage record, so the report is about all of them together. The
/// first failure stops the run and is returned with what was recorded
/// up to then.
pub fn run_covered<W: Write>(
    engine: Engine,
    files: &[(String, String)],
    mut out: W,
) -> (Result<(), String>, Option<String>) {
    let mut coverage = eval::Coverage::default();
    let mut failure = None;
    for (path, src) in files {
        let tokens = match lexer::lex(src) {
            Ok(t) => t,
            Err(e) => {
                failure = Some(diag::render(path, src, &e.message, e.span));
                break;
            }
        };
        let program = match parser::parse_program(&tokens) {
            Ok(p) => p,
            Err(e) => {
                failure = Some(diag::render(path, src, &e.message, e.span));
                break;
            }
        };
        let mut interp = eval::Interpreter::new(&mut out);
        interp.set_source(path, src);
        interp.cover_into(coverage);
        interp.note_coverable(&program);
        if let Some(dir) = std::path::Path::new(path).parent() {
            interp.set_base_dir(dir.to_path_buf());
        }
        let result = match engine {
            Engine::Eval => interp.run(&program).map_err(|e| e.render(path, src)),
            Engine::Vm => match compile::compile_program_covered(&program) {
                Ok(chunk) => vm::run_chunk_compiling_imports(&mut interp, &chunk)
                    .map(|_| ())
                    .map_err(|e| e.render(path, src)),
                Err(e) => Err(diag::render(path, src, &e.message, e.span)),
            },
        };
        let report = interp.coverage_report();
        coverage = interp.take_coverage().unwrap_or_default();
        if let Err(e) = result {
            failure = Some(e);
            let _ = report;
            break;
        }
    }
    // The table is built from the record the last run handed back, so
    // a run that failed still reports what it reached.
    let mut interp = eval::Interpreter::new(Vec::new());
    interp.cover_into(coverage);
    let report = interp.coverage_report();
    match failure {
        Some(e) => (Err(e), report),
        None => (Ok(()), report),
    }
}

/// `run_source_engine`, with whichever reports were asked for
/// appended into the second half of the answer. The run's own result
/// is unchanged by them, and a failed run still reports what it
/// managed to do.
pub fn run_source_reported<W: Write>(
    engine: Engine,
    path: &str,
    src: &str,
    out: W,
    script_args: Vec<String>,
    reports: Reports,
) -> (Result<(), String>, Option<String>) {
    let render = |m: &str, s: lexer::Span| diag::render(path, src, m, s);
    let tokens = match lexer::lex(src) {
        Ok(t) => t,
        Err(e) => return (Err(render(&e.message, e.span)), None),
    };
    let program = match parser::parse_program(&tokens) {
        Ok(p) => p,
        Err(e) => return (Err(render(&e.message, e.span)), None),
    };
    let mut interp = eval::Interpreter::new(out);
    interp.set_args(script_args);
    interp.set_source(path, src);
    if reports.profile {
        interp.profile();
    }
    if reports.coverage {
        interp.cover();
        interp.note_coverable(&program);
    }
    if let Some(dir) = std::path::Path::new(path).parent() {
        interp.set_base_dir(dir.to_path_buf());
    }
    // Coverage needs a `Mark` in front of every statement, which is
    // a different chunk: a plain run must not pay for one.
    let compile_for = |program: &[ast::Stmt]| {
        if reports.coverage {
            compile::compile_program_covered(program)
        } else {
            compile::compile_program(program)
        }
    };
    let result = match engine {
        Engine::Eval => interp.run(&program).map_err(|e| e.render(path, src)),
        Engine::Vm => match compile_for(&program) {
            Ok(chunk) => vm::run_chunk_compiling_imports(&mut interp, &chunk)
                .map(|_| ())
                .map_err(|e| e.render(path, src)),
            Err(e) => Err(render(&e.message, e.span)),
        },
    };
    let mut report = String::new();
    if let Some(table) = interp.profile_report() {
        report.push_str(&table);
    }
    if let Some(table) = interp.coverage_report() {
        report.push_str(&table);
    }
    let report = if report.is_empty() {
        None
    } else {
        Some(report)
    };
    (result, report)
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
