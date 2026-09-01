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
    let src = match std::fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("ting: cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Until statements land, running a file evaluates it as a single
    // expression and prints the result.
    let tokens = match lexer::lex(&src) {
        Ok(tokens) => tokens,
        Err(e) => return report(path, &src, &e.message, e.span),
    };
    let expr = match parser::parse_expr(&tokens) {
        Ok(expr) => expr,
        Err(e) => return report(path, &src, &e.message, e.span),
    };
    match eval::eval(&expr) {
        Ok(v) => {
            println!("{v}");
            ExitCode::SUCCESS
        }
        Err(e) => report(path, &src, &e.message, e.span),
    }
}

fn report(path: &str, src: &str, message: &str, span: lexer::Span) -> ExitCode {
    let (line, col) = span.line_col(src);
    eprintln!("{path}:{line}:{col}: error: {message}");
    ExitCode::FAILURE
}
