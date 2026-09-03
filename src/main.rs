use std::process::ExitCode;
use ting::{Engine, repl, run_source_engine};

fn main() -> ExitCode {
    // The bytecode VM is the default (see docs/vm.md for the numbers);
    // the tree-walker remains available as the reference engine.
    let mut engine = match std::env::var("TING_ENGINE").as_deref() {
        Ok("eval") => Engine::Eval,
        _ => Engine::Vm,
    };
    let mut args = std::env::args().skip(1).peekable();
    match args.peek().map(String::as_str) {
        Some("--version") => {
            println!("ting {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("--help") => {
            println!(
                "ting {} — a tiny, zero-dependency scripting language\n\n\
                 usage:\n\
                 \x20 ting                        start the REPL\n\
                 \x20 ting <script> [args...]     run a script (argv reaches args())\n\
                 \x20 ting --eval <script>        run on the reference tree-walker\n\
                 \x20 ting --vm <script>          run on the bytecode VM (the default)\n\
                 \x20 ting --fmt <files...>       reformat files in place (- filters stdin to stdout)\n\
                 \x20 ting --fmt-check <files...> exit 1 if any file needs reformatting\n\
                 \x20 ting --check <files...>     report syntax errors without running\n\
                 \x20                             (tool flags accept - for stdin)\n\
                 \x20 ting --test <files...>      run each file; ok/FAIL per file, exit 1 if any fail\n\
                 \x20 ting --lsp                  language server on stdio\n\
                 \x20 ting --version | --help\n\n\
                 env: TING_ENGINE=eval|vm selects the engine\n\
                 docs: http://www.baghino.me/thing/",
                env!("CARGO_PKG_VERSION")
            );
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    if matches!(
        args.peek().map(String::as_str),
        Some("--fmt") | Some("--fmt-check")
    ) {
        let check = args.next().as_deref() == Some("--fmt-check");
        return run_fmt(check, args.collect());
    }
    if args.peek().map(String::as_str) == Some("--check") {
        args.next();
        return run_check(args.collect());
    }
    if args.peek().map(String::as_str) == Some("--test") {
        args.next();
        return run_tests(args.collect());
    }
    if args.peek().map(String::as_str) == Some("--lsp") {
        return match ting::lsp::run() {
            0 => ExitCode::SUCCESS,
            _ => ExitCode::FAILURE,
        };
    }
    match args.peek().map(String::as_str) {
        Some("--vm") => {
            engine = Engine::Vm;
            args.next();
        }
        Some("--eval") => {
            engine = Engine::Eval;
            args.next();
        }
        _ => {}
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

/// Source for a tool flag: a file path, or `-` for stdin (read to
/// EOF), the same convention read_file() follows inside scripts.
fn read_tool_source(f: &str) -> Result<String, ExitCode> {
    let read = if f == "-" {
        std::io::read_to_string(std::io::stdin().lock())
    } else {
        std::fs::read_to_string(f)
    };
    read.map_err(|e| {
        eprintln!("ting: cannot read {f}: {e}");
        ExitCode::FAILURE
    })
}

fn run_check(files: Vec<String>) -> ExitCode {
    if files.is_empty() {
        eprintln!("usage: ting --check <files...>");
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for f in &files {
        let src = match read_tool_source(f) {
            Ok(src) => src,
            Err(code) => return code,
        };
        if let Err(diagnostic) = ting::check_source(f, &src) {
            eprintln!("{diagnostic}");
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Test runner: every file runs in its own process (so a script's
/// exit() or a failed assert cannot take the runner down), and the
/// verdict is its exit status. stdout is discarded; stderr is shown
/// under a FAIL line so the diagnostic stays next to its file.
fn run_tests(files: Vec<String>) -> ExitCode {
    if files.is_empty() {
        eprintln!("usage: ting --test <files...>");
        return ExitCode::FAILURE;
    }
    let me = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ting: cannot locate own binary: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut failed = 0usize;
    for f in &files {
        let out = std::process::Command::new(&me)
            .arg(f)
            .env("TING_ENGINE", engine_name())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .output();
        match out {
            Ok(out) if out.status.success() => println!("ok   {f}"),
            Ok(out) => {
                failed += 1;
                println!("FAIL {f}");
                for line in String::from_utf8_lossy(&out.stderr).lines() {
                    println!("     {line}");
                }
            }
            Err(e) => {
                failed += 1;
                println!("FAIL {f}");
                println!("     cannot run: {e}");
            }
        }
    }
    println!("{} passed, {} failed", files.len() - failed, failed);
    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// The engine the runner's children should use: the same one this
/// process was told to use, so `TING_ENGINE=eval ting --test` tests
/// the reference engine.
fn engine_name() -> &'static str {
    match std::env::var("TING_ENGINE").as_deref() {
        Ok("eval") => "eval",
        _ => "vm",
    }
}

fn run_fmt(check: bool, files: Vec<String>) -> ExitCode {
    if files.is_empty() {
        eprintln!("usage: ting --fmt <files...> | ting --fmt-check <files...>");
        return ExitCode::FAILURE;
    }
    let mut dirty = false;
    for f in &files {
        let src = match read_tool_source(f) {
            Ok(src) => src,
            Err(code) => return code,
        };
        match ting::fmt::format(&src) {
            // Stdin cannot be rewritten in place: `--fmt -` is a filter
            // and always writes the formatted source to stdout.
            Ok(formatted) if f == "-" && !check => {
                use std::io::Write;
                if std::io::stdout()
                    .lock()
                    .write_all(formatted.as_bytes())
                    .is_err()
                {
                    return ExitCode::FAILURE;
                }
            }
            Ok(formatted) if formatted != src => {
                if check {
                    println!("would reformat {f}");
                    dirty = true;
                } else if let Err(e) = std::fs::write(f, formatted) {
                    eprintln!("ting: cannot write {f}: {e}");
                    return ExitCode::FAILURE;
                } else {
                    println!("reformatted {f}");
                }
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", ting::diag::render(f, &src, &e.message, e.span));
                return ExitCode::FAILURE;
            }
        }
    }
    if check && dirty {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
