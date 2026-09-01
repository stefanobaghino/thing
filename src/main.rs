mod ast;
mod lexer;
mod parser;

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
    // Until the evaluator lands, running a file parses it as a single
    // expression and dumps the AST as an s-expression.
    let tokens = match lexer::lex(&src) {
        Ok(tokens) => tokens,
        Err(e) => return report(path, &src, &e.message, e.span),
    };
    match parser::parse_expr(&tokens) {
        Ok(expr) => {
            println!("{expr}");
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
