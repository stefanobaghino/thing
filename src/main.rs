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
        Some("--version") | Some("-V") => {
            println!("ting {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("--help") | Some("-h") => {
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
                 \x20   [--strict]                warnings fail the check too\n\
                 \x20                             (tool flags accept - for stdin; dirs recurse)\n\
                 \x20 ting --test <paths...>      run each file (dirs recurse); ok/FAIL per file, exit 1 if any fail\n\
                 \x20   [--filter SUBSTR]         only files whose path contains SUBSTR\n\
                 \x20   [--tap]                   Test Anything Protocol output\n\
                 \x20   [-j N]                    run up to N files at once (output stays ordered)\n\
                 \x20   [--slow N]                list the N slowest files after the summary\n\
                 \x20   [--fail-fast]             stop after the first failing file (the rest are skipped)\n\
                 \x20 ting --doc [NAMES...]       explain builtins or stdlib functions;\n\
                 \x20                             a module or a .ting file lists its members,\n\
                 \x20                             no name lists all\n\
                 \x20 ting --lsp                  language server on stdio\n\
                 \x20 ting --version | --help    (also ting -V | -h)\n\n\
                 exit status: 0 ok; 1 a reported failure; 2 a usage error\n\n\
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
        if let Some(a) = rest.iter().find(|a| is_option(a)) {
            return unknown_option(a);
        }
        return run_fmt(check, diff, rest);
    }
    if args.peek().map(String::as_str) == Some("--check") {
        args.next();
        return run_check(args.collect());
    }
    if args.peek().map(String::as_str) == Some("--doc") {
        args.next();
        let names: Vec<String> = args.collect();
        if names.is_empty() {
            repl::say(&repl::doc_index(None).unwrap_or_default());
            return ExitCode::SUCCESS;
        }
        let mut printed = 0;
        let mut missing = false;
        for name in &names {
            match doc_lookup(name) {
                Some(text) => {
                    // Entries are separated by a blank line.
                    if printed > 0 {
                        repl::say("");
                    }
                    repl::say(&text);
                    printed += 1;
                }
                None => {
                    let names = repl::doc_names();
                    let near = ting::diag::nearest(name, names.iter().map(String::as_str));
                    match near {
                        Some(n) => eprintln!(
                            "ting: no builtin, stdlib function, module or file named {name} (did you mean {n}?)"
                        ),
                        None => eprintln!(
                            "ting: no builtin, stdlib function, module or file named {name}"
                        ),
                    }
                    missing = true;
                }
            }
        }
        return if missing {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
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
    if let Some(a) = args.peek()
        && is_option(a)
    {
        return unknown_option(a);
    }
    match args.next() {
        None => repl::run(),
        // Everything after the script path is the script's own argv,
        // exposed via the args() builtin.
        Some(path) => run_file(engine, path, args.collect()),
    }
}

/// An argument shaped like an option: a leading dash, but not the
/// lone `-` that names stdin.
fn is_option(a: &str) -> bool {
    a.starts_with('-') && a != "-"
}

/// Every option any mode accepts, for suggesting the one that was
/// meant. Keep in step with the dispatch above and the usage text.
const OPTIONS: [&str; 19] = [
    "--check",
    "--diff",
    "--doc",
    "--eval",
    "--fail-fast",
    "--filter",
    "--fmt",
    "--fmt-check",
    "--help",
    "--lsp",
    "--slow",
    "--strict",
    "--tap",
    "--test",
    "--version",
    "--vm",
    "-V",
    "-h",
    "-j",
];

/// An option no mode recognises: say so, name the nearest real option
/// when there is one, point at --help, exit 2 (a usage error, distinct
/// from the 1 a failed run or check exits with).
fn unknown_option(a: &str) -> ExitCode {
    match ting::diag::nearest(a, OPTIONS) {
        Some(near) => eprintln!("ting: unknown option {a} (did you mean {near}?) (see --help)"),
        None => eprintln!("ting: unknown option {a} (see --help)"),
    }
    ExitCode::from(2)
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
    let outcome = match run_source_engine(engine, path, &src, std::io::stdout().lock(), script_args)
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(diagnostic) => {
            eprintln!("{diagnostic}");
            ExitCode::FAILURE
        }
    };
    // `--test` runs each file as a child of itself and asks it, this
    // way, how many checks it ran.
    if std::env::var_os("TING_TEST_REPORT").is_some() {
        eprintln!("ting-checks: {}", ting::eval::checks_run());
    }
    outcome
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

/// What `--doc NAME` explains: a builtin, a stdlib function, a module,
/// or one of the user's own `.ting` files.
fn doc_lookup(name: &str) -> Option<String> {
    repl::doc_text(name)
        .or_else(|| repl::doc_index(Some(name)))
        .or_else(|| {
            // A .ting file of the user's own: its functions.
            (name.ends_with(".ting") && std::path::Path::new(name).is_file())
                .then(|| repl::doc_file(name))
                .flatten()
        })
}

fn run_check(mut args: Vec<String>) -> ExitCode {
    // `--strict` (anywhere among the arguments): warnings fail the
    // check too, for hooks and CI that want them enforced.
    let strict = args.iter().any(|a| a == "--strict");
    args.retain(|a| a != "--strict");
    if let Some(a) = args.iter().find(|a| is_option(a)) {
        return unknown_option(a);
    }
    if args.is_empty() {
        eprintln!("usage: ting --check [--strict] <files or directories...>");
        return ExitCode::from(2);
    }
    let files = expand_paths(&args);
    if files.is_empty() {
        eprintln!("ting: no .ting files found under {}", args.join(" "));
        return ExitCode::from(2);
    }
    let mut failed = false;
    // Files a checked file imports are checked too, each once, under
    // their own path; stdin (`-`) has no directory to resolve against.
    let mut queue: std::collections::VecDeque<String> = files.into_iter().collect();
    let mut seen = std::collections::HashSet::new();
    while let Some(f) = queue.pop_front() {
        let key = std::fs::canonicalize(&f).unwrap_or_else(|_| std::path::PathBuf::from(&f));
        if !seen.insert(key) {
            continue;
        }
        let src = match read_tool_source(&f) {
            Ok(src) => src,
            Err(_) => {
                failed = true;
                continue;
            }
        };
        match ting::check_source(&f, &src) {
            Err(diagnostic) => {
                eprintln!("{diagnostic}");
                failed = true;
            }
            // Warnings are advice and leave the exit status alone —
            // unless --strict asked for them to count.
            Ok(()) => {
                for w in ting::check_warnings(&f, &src) {
                    eprintln!("{w}");
                    if strict {
                        failed = true;
                    }
                }
            }
        }
        if f != "-" {
            for target in ting::local_imports(&f, &src) {
                queue.push_back(target.display().to_string());
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
    let mut fail_fast = false;
    let mut jobs = 1usize;
    let mut slow = 0usize;
    let mut paths = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--tap" {
            tap = true;
        } else if a == "--fail-fast" {
            fail_fast = true;
        } else if a == "--slow" {
            match it.next().and_then(|n| n.parse::<usize>().ok()) {
                Some(n) => slow = n,
                None => {
                    eprintln!(
                        "usage: ting --test [-j N] [--filter SUBSTR] [--tap] [--slow N] [--fail-fast] <files or directories...>"
                    );
                    return ExitCode::from(2);
                }
            }
        } else if a == "-j" {
            match it.next().and_then(|n| n.parse::<usize>().ok()) {
                Some(n) if n >= 1 => jobs = n,
                _ => {
                    eprintln!(
                        "usage: ting --test [-j N] [--filter SUBSTR] [--tap] [--slow N] [--fail-fast] <files or directories...>"
                    );
                    return ExitCode::from(2);
                }
            }
        } else if a == "--filter" {
            match it.next() {
                Some(f) => filter = Some(f),
                None => {
                    eprintln!(
                        "usage: ting --test [-j N] [--filter SUBSTR] [--tap] [--slow N] [--fail-fast] <files or directories...>"
                    );
                    return ExitCode::from(2);
                }
            }
        } else if is_option(&a) {
            return unknown_option(&a);
        } else {
            paths.push(a);
        }
    }
    if paths.is_empty() {
        eprintln!(
            "usage: ting --test [-j N] [--filter SUBSTR] [--tap] [--slow N] [--fail-fast] <files or directories...>"
        );
        return ExitCode::from(2);
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
        return ExitCode::from(2);
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
    // `--fail-fast` stops after the first failing file: no further
    // file starts (running ones finish), and the rest are skipped —
    // None below, reported as skipped rather than passed or failed.
    let results: Vec<Option<TestOutcome>> = if jobs <= 1 {
        let mut out = Vec::with_capacity(files.len());
        for f in &files {
            if fail_fast
                && out
                    .iter()
                    .any(|r: &Option<TestOutcome>| matches!(r, Some((false, ..))))
            {
                out.push(None);
                continue;
            }
            out.push(Some(run_one(&me, f)));
        }
        out
    } else {
        let next = std::sync::Mutex::new(0usize);
        let stop = std::sync::atomic::AtomicBool::new(false);
        let slots: Vec<std::sync::Mutex<Option<TestOutcome>>> =
            files.iter().map(|_| std::sync::Mutex::new(None)).collect();
        std::thread::scope(|scope| {
            for _ in 0..jobs.min(files.len()) {
                scope.spawn(|| {
                    loop {
                        if stop.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
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
                        if fail_fast && !r.0 {
                            stop.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                        *slots[i].lock().unwrap() = Some(r);
                    }
                });
            }
        });
        slots.into_iter().map(|m| m.into_inner().unwrap()).collect()
    };
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut total_checks = 0usize;
    let mut unchecked = 0usize;
    let mut timings: Vec<(u128, &str)> = Vec::new();
    for (i, (f, result)) in files.iter().zip(results).enumerate() {
        let Some((ok, diag, ms, checks)) = result else {
            skipped += 1;
            if tap {
                println!("ok {} - {f} # SKIP fail-fast", i + 1);
            }
            continue;
        };
        timings.push((ms, f));
        if !ok {
            failed += 1;
        }
        total_checks += checks;
        // Only a file that passed while checking nothing is worth
        // naming: a failure has already said what went wrong.
        if ok && checks == 0 {
            unchecked += 1;
        }
        // "(12 checks)", or "(no checks)" for a file that verified
        // nothing — which passes, but proves nothing.
        let count = match checks {
            0 => "no checks".to_string(),
            1 => "1 check".to_string(),
            n => format!("{n} checks"),
        };
        if tap {
            println!("{} {} - {f}", if ok { "ok" } else { "not ok" }, i + 1);
            for line in &diag {
                println!("# {line}");
            }
            println!("# {count}");
            println!("# time: {ms}ms");
        } else if ok {
            println!("ok   {f} ({count})");
        } else {
            println!("FAIL {f}");
            for line in &diag {
                println!("     {line}");
            }
        }
    }
    let passed = files.len() - failed - skipped;
    let prefix = if tap { "# " } else { "" };
    let checks = match total_checks {
        1 => ", 1 check".to_string(),
        n => format!(", {n} checks"),
    };
    let none = match unchecked {
        0 => String::new(),
        1 => " (1 file checked nothing)".to_string(),
        n => format!(" ({n} files checked nothing)"),
    };
    if skipped > 0 {
        println!("{prefix}{passed} passed, {failed} failed, {skipped} skipped{checks}{none}");
    } else {
        println!("{prefix}{passed} passed, {failed} failed{checks}{none}");
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

/// (passed, stderr lines, elapsed ms, checks run) for one test file.
type TestOutcome = (bool, Vec<String>, u128, usize);

/// One test file in a child process. The child is asked to report how
/// many checks it ran; that line is taken out of its diagnostics.
fn run_one(me: &std::path::Path, f: &str) -> TestOutcome {
    let started = std::time::Instant::now();
    let out = std::process::Command::new(me)
        .arg(f)
        .env("TING_ENGINE", engine_name())
        .env("TING_TEST_REPORT", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .output();
    let ms = started.elapsed().as_millis();
    match out {
        Ok(out) => {
            let mut checks = 0usize;
            let diag: Vec<String> = String::from_utf8_lossy(&out.stderr)
                .lines()
                .filter(|line| match line.strip_prefix("ting-checks: ") {
                    Some(n) => {
                        checks = n.trim().parse().unwrap_or(0);
                        false
                    }
                    None => true,
                })
                .map(str::to_string)
                .collect();
            if out.status.success() {
                (true, Vec::new(), ms, checks)
            } else {
                (false, diag, ms, checks)
            }
        }
        Err(e) => (false, vec![format!("cannot run: {e}")], ms, 0),
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
        return ExitCode::from(2);
    }
    let files = expand_paths(&args);
    if files.is_empty() {
        eprintln!("ting: no .ting files found under {}", args.join(" "));
        return ExitCode::from(2);
    }
    let mut dirty = false;
    // A file that cannot be read, does not lex, or cannot be written
    // is reported and the run goes on to the next one; the exit
    // status says at the end whether anything failed.
    let mut failed = false;
    // Counts for the summary a multi-file run ends with.
    let (mut changed, mut unchanged, mut failures) = (0usize, 0usize, 0usize);
    for f in &files {
        let src = match read_tool_source(f) {
            Ok(src) => src,
            Err(_) => {
                failed = true;
                failures += 1;
                continue;
            }
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
                    changed += 1;
                } else if check {
                    println!("would reformat {f}");
                    dirty = true;
                    changed += 1;
                } else if let Err(e) = std::fs::write(f, formatted) {
                    eprintln!("ting: cannot write {f}: {e}");
                    failed = true;
                    failures += 1;
                } else {
                    println!("reformatted {f}");
                    changed += 1;
                }
            }
            Ok(_) => unchanged += 1,
            Err(e) => {
                eprintln!("{}", ting::diag::render(f, &src, &e.message, e.span));
                failed = true;
                failures += 1;
            }
        }
    }
    // A run over more than one file ends with a summary, the way a
    // test run does; a single file's line already says everything.
    if files.len() > 1 {
        let verb = if check || diff {
            "would change"
        } else {
            "reformatted"
        };
        println!("{changed} {verb}, {unchanged} unchanged, {failures} failed");
    }
    if failed || ((check || diff) && dirty) {
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
