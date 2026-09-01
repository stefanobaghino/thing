mod ast;
mod eval;
mod lexer;
mod parser;
mod value;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_] => {
            eprintln!("ting: REPL not implemented yet; usage: ting <script>");
            ExitCode::FAILURE
        }
        [_, path] => run_file(path),
        _ => {
            eprintln!("usage: ting [script]");
            ExitCode::FAILURE
        }
    }
}

fn run_file(path: &str) -> ExitCode {
    // The AST holds Rc (not Send), so the whole pipeline runs on one
    // dedicated thread, sized generously because deep ting recursion
    // consumes host stack.
    let path = path.to_string();
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || run_file_inner(&path))
        .expect("failed to spawn interpreter thread")
        .join()
        .expect("interpreter thread panicked")
}

fn run_file_inner(path: &str) -> ExitCode {
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
    match interp.run(&program) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => report(path, &src, &e.message, e.span),
    }
}

fn report(path: &str, src: &str, message: &str, span: lexer::Span) -> ExitCode {
    let (line, col) = span.line_col(src);
    eprintln!("{path}:{line}:{col}: error: {message}");
    ExitCode::FAILURE
}
