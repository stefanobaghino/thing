mod lexer;

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
    // Until the parser lands, running a file dumps its token stream.
    match lexer::lex(&src) {
        Ok(tokens) => {
            for t in &tokens {
                println!("{:?}", t.kind);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            let (line, col) = e.span.line_col(&src);
            eprintln!("{path}:{line}:{col}: error: {}", e.message);
            ExitCode::FAILURE
        }
    }
}
