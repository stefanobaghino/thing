mod ast;
mod diag;
mod eval;
mod lexer;
mod parser;
mod repl;
mod value;

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next() {
        None => repl::run(),
        // Everything after the script path is the script's own argv,
        // exposed via the args() builtin.
        Some(path) => run_file(path, args.collect()),
    }
}

fn run_file(path: String, script_args: Vec<String>) -> ExitCode {
    // The AST holds Rc (not Send), so the whole pipeline runs on one
    // dedicated thread, sized generously because deep ting recursion
    // consumes host stack.
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || run_file_inner(&path, script_args))
        .expect("failed to spawn interpreter thread")
        .join()
        .expect("interpreter thread panicked")
}

fn run_file_inner(path: &str, script_args: Vec<String>) -> ExitCode {
    let src = match std::fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("ting: cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let tokens = match lexer::lex(&src) {
        Ok(tokens) => tokens,
        Err(e) => return report(path, &src, &e.message, e.span),
    };
    let program = match parser::parse_program(&tokens) {
        Ok(program) => program,
        Err(e) => return report(path, &src, &e.message, e.span),
    };
    let mut interp = eval::Interpreter::new(std::io::stdout().lock());
    interp.set_args(script_args);
    match interp.run(&program) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => report(path, &src, &e.message, e.span),
    }
}

fn report(path: &str, src: &str, message: &str, span: lexer::Span) -> ExitCode {
    eprintln!("{}", diag::render(path, src, message, span));
    ExitCode::FAILURE
}
