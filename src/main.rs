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
                 \x20 ting --fmt <paths...>       reformat files in place (- filters stdin to stdout)\n\
                 \x20   [--diff]                  print what would change instead; exit 1 if anything\n\
                 \x20 ting --fmt-check <paths...> exit 1 if any file needs reformatting\n\
                 \x20 ting --check <paths...>     report syntax errors without running\n\
                 \x20                             (tool flags accept - for stdin; dirs recurse)\n\
                 \x20 ting --test <paths...>      run each file (dirs recurse); ok/FAIL per file, exit 1 if any fail\n\
                 \x20   [--filter SUBSTR]         only files whose path contains SUBSTR\n\
                 \x20   [--tap]                   Test Anything Protocol output\n\
                 \x20   [-j N]                    run up to N files at once (output stays ordered)\n\
                 \x20   [--slow N]                list the N slowest files after the summary\n\
                 \x20 ting --doc NAME             explain a builtin or stdlib function\n\
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
        let mut rest: Vec<String> = args.collect();
        // `--diff`: show what --fmt would change, touch nothing.
        let diff = rest.iter().any(|a| a == "--diff");
        rest.retain(|a| a != "--diff");
        return run_fmt(check, diff, rest);
    }
    if args.peek().map(String::as_str) == Some("--check") {
        args.next();
        return run_check(args.collect());
    }
    if args.peek().map(String::as_str) == Some("--doc") {
        args.next();
        return match args.next() {
            Some(name) => match repl::doc_text(&name) {
                Some(text) => {
                    println!("{text}");
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!("ting: no builtin or stdlib function named {name}");
                    ExitCode::FAILURE
                }
            },
            None => {
                eprintln!("usage: ting --doc NAME");
                ExitCode::FAILURE
            }
        };
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

/// Directory arguments expand to every .ting file beneath them (files
/// first, then subdirectories, sorted); other arguments pass through.
fn expand_paths(args: &[String]) -> Vec<String> {
    let mut files = Vec::new();
    for a in args {
        if std::path::Path::new(a).is_dir() {
            collect_ting_files(std::path::Path::new(a), &mut files);
        } else {
            files.push(a.clone());
        }
    }
    files
}

fn run_check(args: Vec<String>) -> ExitCode {
    if args.is_empty() {
        eprintln!("usage: ting --check <files or directories...>");
        return ExitCode::FAILURE;
    }
    let files = expand_paths(&args);
    if files.is_empty() {
        eprintln!("ting: no .ting files found under {}", args.join(" "));
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for f in &files {
        let src = match read_tool_source(f) {
            Ok(src) => src,
            Err(code) => return code,
        };
        match ting::check_source(f, &src) {
            Err(diagnostic) => {
                eprintln!("{diagnostic}");
                failed = true;
            }
            // Warnings never change the exit status; they are advice.
            Ok(()) => {
                for w in ting::check_warnings(f, &src) {
                    eprintln!("{w}");
                }
            }
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
fn run_tests(args: Vec<String>) -> ExitCode {
    // `--filter SUBSTR` (anywhere among the arguments) keeps only the
    // files whose path contains the substring.
    let mut filter: Option<String> = None;
    let mut tap = false;
    let mut jobs = 1usize;
    let mut slow = 0usize;
    let mut paths = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--tap" {
            tap = true;
        } else if a == "--slow" {
            match it.next().and_then(|n| n.parse::<usize>().ok()) {
                Some(n) => slow = n,
                None => {
                    eprintln!(
                        "usage: ting --test [-j N] [--filter SUBSTR] [--tap] [--slow N] <files or directories...>"
                    );
                    return ExitCode::FAILURE;
                }
            }
        } else if a == "-j" {
            match it.next().and_then(|n| n.parse::<usize>().ok()) {
                Some(n) if n >= 1 => jobs = n,
                _ => {
                    eprintln!(
                        "usage: ting --test [-j N] [--filter SUBSTR] [--tap] [--slow N] <files or directories...>"
                    );
                    return ExitCode::FAILURE;
                }
            }
        } else if a == "--filter" {
            match it.next() {
                Some(f) => filter = Some(f),
                None => {
                    eprintln!(
                        "usage: ting --test [-j N] [--filter SUBSTR] [--tap] [--slow N] <files or directories...>"
                    );
                    return ExitCode::FAILURE;
                }
            }
        } else {
            paths.push(a);
        }
    }
    if paths.is_empty() {
        eprintln!(
            "usage: ting --test [-j N] [--filter SUBSTR] [--tap] [--slow N] <files or directories...>"
        );
        return ExitCode::FAILURE;
    }
    // Directories expand to every .ting file beneath them, sorted, so
    // `ting --test selftest` is the whole suite in a stable order.
    let mut files = expand_paths(&paths);
    if let Some(f) = &filter {
        files.retain(|p| p.contains(f.as_str()));
    }
    if files.is_empty() {
        match &filter {
            Some(f) => eprintln!(
                "ting: no .ting files matching \"{f}\" under {}",
                paths.join(" ")
            ),
            None => eprintln!("ting: no .ting files found under {}", paths.join(" ")),
        }
        return ExitCode::FAILURE;
    }
    let me = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ting: cannot locate own binary: {e}");
            return ExitCode::FAILURE;
        }
    };
    // TAP (Test Anything Protocol) mode: a plan, `ok N - path` /
    // `not ok N - path` lines, diagnostics as `# ` comments, and the
    // elapsed time per file as a comment — for CI systems and TAP
    // consumers. The human-readable default is unchanged.
    if tap {
        println!("1..{}", files.len());
    }
    // `-j N` runs up to N files at once; results are collected per
    // file and printed in the original order, so TAP numbering and
    // the human output are identical whatever the parallelism.
    let results: Vec<TestOutcome> = if jobs <= 1 {
        files.iter().map(|f| run_one(&me, f)).collect()
    } else {
        let next = std::sync::Mutex::new(0usize);
        let slots: Vec<std::sync::Mutex<Option<TestOutcome>>> =
            files.iter().map(|_| std::sync::Mutex::new(None)).collect();
        std::thread::scope(|scope| {
            for _ in 0..jobs.min(files.len()) {
                scope.spawn(|| {
                    loop {
                        let i = {
                            let mut n = next.lock().unwrap();
                            let i = *n;
                            *n += 1;
                            i
                        };
                        if i >= files.len() {
                            break;
                        }
                        let r = run_one(&me, &files[i]);
                        *slots[i].lock().unwrap() = Some(r);
                    }
                });
            }
        });
        slots
            .into_iter()
            .map(|m| m.into_inner().unwrap().expect("every file ran"))
            .collect()
    };
    let mut failed = 0usize;
    let mut timings: Vec<(u128, &str)> = Vec::new();
    for (i, (f, (ok, diag, ms))) in files.iter().zip(results).enumerate() {
        timings.push((ms, f));
        if !ok {
            failed += 1;
        }
        if tap {
            println!("{} {} - {f}", if ok { "ok" } else { "not ok" }, i + 1);
            for line in &diag {
                println!("# {line}");
            }
            println!("# time: {ms}ms");
        } else if ok {
            println!("ok   {f}");
        } else {
            println!("FAIL {f}");
            for line in &diag {
                println!("     {line}");
            }
        }
    }
    if tap {
        println!("# {} passed, {} failed", files.len() - failed, failed);
    } else {
        println!("{} passed, {} failed", files.len() - failed, failed);
    }
    // `--slow N`: the N slowest files after the summary, opt-in so the
    // default output is unchanged (as a TAP comment in --tap mode).
    if slow > 0 {
        timings.sort_by_key(|t| std::cmp::Reverse(t.0));
        let prefix = if tap { "# " } else { "" };
        println!("{prefix}slowest:");
        for (ms, f) in timings.iter().take(slow) {
            println!("{prefix}  {ms}ms {f}");
        }
    }
    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// (passed, stderr lines, elapsed ms) for one test file.
type TestOutcome = (bool, Vec<String>, u128);

/// One test file in a child process.
fn run_one(me: &std::path::Path, f: &str) -> TestOutcome {
    let started = std::time::Instant::now();
    let out = std::process::Command::new(me)
        .arg(f)
        .env("TING_ENGINE", engine_name())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .output();
    let ms = started.elapsed().as_millis();
    match out {
        Ok(out) if out.status.success() => (true, Vec::new(), ms),
        Ok(out) => (
            false,
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .map(str::to_string)
                .collect(),
            ms,
        ),
        Err(e) => (false, vec![format!("cannot run: {e}")], ms),
    }
}

fn collect_ting_files(dir: &std::path::Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    // Files of a directory first, then its subdirectories: a suite's
    // own tests report before anything nested under it.
    for p in paths.iter().filter(|p| !p.is_dir()) {
        if p.extension().and_then(|e| e.to_str()) == Some("ting") {
            out.push(p.to_string_lossy().into_owned());
        }
    }
    for p in paths.iter().filter(|p| p.is_dir()) {
        collect_ting_files(p, out);
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

fn run_fmt(check: bool, diff: bool, args: Vec<String>) -> ExitCode {
    if args.is_empty() {
        eprintln!(
            "usage: ting --fmt [--diff] <files or directories...> | ting --fmt-check <files or directories...>"
        );
        return ExitCode::FAILURE;
    }
    let files = expand_paths(&args);
    if files.is_empty() {
        eprintln!("ting: no .ting files found under {}", args.join(" "));
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
                if diff {
                    print_line_diff(f, &src, &formatted);
                    dirty = true;
                } else if check {
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
    if (check || diff) && dirty {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// A line diff of `old` against `new` for `--fmt --diff`: a header
/// naming the file, then every changed line prefixed with `-` or `+`
/// and its line number in the respective version. Computed from a
/// longest-common-subsequence table; the inputs are source files, so
/// quadratic space is fine.
fn print_line_diff(path: &str, old: &str, new: &str) {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let mut lcs = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    println!("--- {path}");
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            i += 1;
            j += 1;
        } else if j < b.len() && (i == a.len() || lcs[i][j + 1] > lcs[i + 1][j]) {
            println!("+{}: {}", j + 1, b[j]);
            j += 1;
        } else {
            println!("-{}: {}", i + 1, a[i]);
            i += 1;
        }
    }
}
