use std::process::ExitCode;
use ting::{repl, run_source};

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
    match run_source(path, &src, std::io::stdout().lock(), script_args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(diagnostic) => {
            eprintln!("{diagnostic}");
            ExitCode::FAILURE
        }
    }
}
