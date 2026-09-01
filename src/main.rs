use std::process::ExitCode;
use ting::{Engine, repl, run_source_engine};

fn main() -> ExitCode {
    let mut engine = match std::env::var("TING_ENGINE").as_deref() {
        Ok("vm") => Engine::Vm,
        _ => Engine::Eval,
    };
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().map(String::as_str) == Some("--vm") {
        engine = Engine::Vm;
        args.next();
    }
    match args.next() {
        None => repl::run(),
        // Everything after the script path is the script's own argv,
        // exposed via the args() builtin.
        Some(path) => run_file(engine, path, args.collect()),
    }
}

fn run_file(engine: Engine, path: String, script_args: Vec<String>) -> ExitCode {
    // The AST holds Rc (not Send), so the whole pipeline runs on one
    // dedicated thread, sized generously because deep ting recursion
    // consumes host stack.
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || run_file_inner(engine, &path, script_args))
        .expect("failed to spawn interpreter thread")
        .join()
        .expect("interpreter thread panicked")
}

fn run_file_inner(engine: Engine, path: &str, script_args: Vec<String>) -> ExitCode {
    let src = match std::fs::read_to_string(path) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("ting: cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    match run_source_engine(engine, path, &src, std::io::stdout().lock(), script_args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(diagnostic) => {
            eprintln!("{diagnostic}");
            ExitCode::FAILURE
        }
    }
}
