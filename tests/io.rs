//! The script I/O builtins against the real binary: args() sees the
//! command line after the script path, input() reads piped stdin line
//! by line and returns nil at EOF.

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn args_and_stdin_reach_the_script() {
    let script = std::env::temp_dir().join("ting-io-integration.ting");
    std::fs::write(
        &script,
        "print(args());\n\
         let line = input();\n\
         while line != nil {\n\
           print(upper(line));\n\
           line = input();\n\
         }\n",
    )
    .unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .arg("one")
        .arg("two")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"hello\nworld\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();

    assert!(
        out.status.success(),
        "ting exited nonzero:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "[\"one\", \"two\"]\nHELLO\nWORLD\n"
    );
    let _ = std::fs::remove_file(&script);
}

#[test]
fn sleep_ms_waits_at_least_that_long() {
    let script = std::env::temp_dir().join("ting-sleep.ting");
    // A lower bound only: a loaded runner can make any pause longer,
    // and none can make it shorter.
    std::fs::write(
        &script,
        "let t = time_ms();\n\
         sleep_ms(50);\n\
         let waited = time_ms() - t;\n\
         assert(waited >= 40, \"waited \" + str(waited));\n\
         print(sleep_ms(0));\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "nil\n");
    let _ = std::fs::remove_file(&script);
}

#[test]
fn env_exit_and_time_reach_the_process() {
    let script = std::env::temp_dir().join("ting-proc-integration.ting");
    std::fs::write(
        &script,
        "print(env(\"TING_TEST_VAR\"), env(\"TING_UNSET_VAR\"));\n\
         let t = time_ms();\n\
         assert(t > 1500000000000, \"epoch millis\");\n\
         assert(time_ms() >= t, \"monotonic-ish\");\n\
         exit(3);\n\
         print(\"unreachable\");\n",
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .env("TING_TEST_VAR", "hello-env")
        .env_remove("TING_UNSET_VAR")
        .output()
        .expect("failed to run ting");

    assert_eq!(String::from_utf8_lossy(&out.stdout), "hello-env nil\n");
    assert_eq!(out.status.code(), Some(3), "exit code must be 3");
    let _ = std::fs::remove_file(&script);
}

#[test]
fn check_flag_reports_without_running() {
    let dir = std::env::temp_dir();
    let good = dir.join(format!("ting-check-good-{}.ting", std::process::id()));
    let bad = dir.join(format!("ting-check-bad-{}.ting", std::process::id()));
    // exit(7) proves --check never executes the program.
    std::fs::write(&good, "exit(7);\n").unwrap();
    std::fs::write(&bad, "let = 3;\n").unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", good.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0), "clean file, not executed");

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", good.to_str().unwrap(), bad.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1), "bad file fails the batch");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("expected variable name"), "got: {stderr}");

    let _ = std::fs::remove_file(&good);
    let _ = std::fs::remove_file(&bad);
}

/// `--check` also prints the semantic warning the LSP knows — an
/// imported stdlib module indexed with a name it lacks — without
/// changing the exit status.
/// `--fmt --diff` prints changed lines with `-`/`+` and line numbers,
/// leaves the file alone, and exits 1 only when something would change.
#[test]
fn fmt_diff_shows_changes_without_writing() {
    let path = std::env::temp_dir().join(format!("ting-fmt-diff-{}.ting", std::process::id()));
    std::fs::write(&path, "let a = 1;\nlet   b=2;\nprint(a + b);\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--fmt", "--diff", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(stdout.starts_with("--- "), "{stdout}");
    assert!(
        stdout.contains("\n-2: let   b=2;\n") && stdout.contains("\n+2: let b = 2;\n"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("print(a + b)"),
        "unchanged lines are not printed: {stdout}"
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "let a = 1;\nlet   b=2;\nprint(a + b);\n",
        "untouched"
    );
    std::fs::write(&path, "let b = 2;\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--fmt", "--diff", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn check_and_fmt_flags_expand_directories() {
    let root = std::env::temp_dir().join(format!("ting-check-dir-{}", std::process::id()));
    let nested = root.join("sub");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(root.join("ok.ting"), "let x = 1;\n").unwrap();
    std::fs::write(nested.join("bad.ting"), "let = 3;\n").unwrap();
    std::fs::write(nested.join("ugly.ting"), "let   y=2;\n").unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("bad.ting") && stderr.contains("expected variable name"),
        "{stderr}"
    );

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--fmt-check", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    // bad.ting lexes, so the batch reaches ugly.ting and reports it.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.contains("would reformat") && stdout.contains("ugly.ting"),
        "{stdout}"
    );

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", root.join("empty-nowhere").to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn check_flag_prints_stdlib_member_warnings() {
    let path = std::env::temp_dir().join(format!("ting-check-warn-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "let l = import(\"lib/list.ting\");\nprint(l[\"medain\"]([1]), l[\"median\"]([1]));\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0), "warnings do not fail the check");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: lib/list.ting has no `medain`"),
        "{stderr}"
    );
    assert!(stderr.contains(":2:10:"), "points at the key: {stderr}");
    // The correct call on the same line is not warned about: one warning,
    // and `median` appears only as the suggestion for the misspelling.
    assert_eq!(stderr.matches("warning:").count(), 1, "{stderr}");
    let _ = std::fs::remove_file(&path);
}

/// `--check` warns about a top-level binding that is never used;
/// underscore-prefixed names are exempt and used ones are silent.
#[test]
fn check_flag_warns_about_unused_top_level_lets() {
    let path = std::env::temp_dir().join(format!("ting-check-unused-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "let used = 1;\nlet unused = 2;\nlet _scratch = 3;\nfn helper() { return 0; }\nprint(used);\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: `unused` is never used"),
        "{stderr}"
    );
    assert!(
        stderr.contains("warning: `helper` is never used"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("`used`") && !stderr.contains("_scratch"),
        "{stderr}"
    );
    let _ = std::fs::remove_file(&path);
}

/// `--check` warns about a parameter the function body never names;
/// `_`-prefixed parameters and used ones are silent.
#[test]
fn check_flag_warns_about_unused_params() {
    let path = std::env::temp_dir().join(format!("ting-check-params-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "fn add(a, b) { return a; }\nlet f = fn(x, _ignored) { return x + 1; };\nprint(add(1, 2), f(3));\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: parameter `b` is never used"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("`a`") && !stderr.contains("`x`") && !stderr.contains("_ignored"),
        "{stderr}"
    );
    let _ = std::fs::remove_file(&path);
}

/// A runtime error raised inside an imported module's function is
/// reported against the module's file and line, not the importer's.
#[test]
fn module_runtime_errors_point_into_the_module() {
    let dir = std::env::temp_dir().join(format!("ting-origin-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("m.ting"),
        "# a module\nfn boom() {\n  return nosuch + 1;\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.ting"),
        "let m = import(\"./m.ting\");\nprint(\"before\");\nm[\"boom\"]();\n",
    )
    .unwrap();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg(dir.join("main.ting"))
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(1), "{engine}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("m.ting:3:10: error: undefined variable 'nosuch'"),
            "{engine}: {stderr}"
        );
        assert!(stderr.contains("return nosuch + 1;"), "{engine}: {stderr}");
        // The importer's call site follows as a named frame, and
        // nowhere else.
        assert!(
            stderr.contains("note: in boom, called from") && stderr.contains("main.ting:3:"),
            "{engine}: {stderr}"
        );
        assert_eq!(
            stderr.matches("main.ting:").count(),
            1,
            "{engine}: {stderr}"
        );
    }
    // The same for a function from an embedded stdlib module: the
    // path is the module's, and the foreign offset never panics the
    // renderer.
    std::fs::write(
        dir.join("emb.ting"),
        "let l = import(\"lib/list.ting\");\nprint(l[\"mean\"]([]));\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(dir.join("emb.ting"))
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("lib/list.ting:") && stderr.contains("error: mean:"),
        "{stderr}"
    );
    assert!(
        stderr.contains("note: in mean, called from") && stderr.contains("emb.ting:2:"),
        "{stderr}"
    );
    assert!(!stderr.contains("panicked"), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// An error carries every call it unwound through: one note per
/// frame, innermost first, named after the function it was raised in,
/// identical under both engines. A long trace is elided in the middle
/// so a runaway recursion cannot bury the message.
#[test]
fn errors_show_the_whole_way_back() {
    let dir = std::env::temp_dir().join(format!("ting-trace-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let nested = dir.join("nested.ting");
    std::fs::write(
        &nested,
        "fn inner(x) { return x + \"a\"; }\nfn outer(x) { return inner(x); }\nlet apply = fn(f) { return f(1); };\napply(outer);\n",
    )
    .unwrap();
    let mut seen = Vec::new();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg(&nested)
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(1), "{engine}");
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let notes: Vec<&str> = stderr.lines().filter(|l| l.starts_with("note:")).collect();
        assert_eq!(notes.len(), 3, "{engine}: {stderr}");
        assert!(
            notes[0].starts_with("note: in inner, called from"),
            "{engine}: {stderr}"
        );
        assert!(notes[0].ends_with("nested.ting:2:22"), "{engine}: {stderr}");
        assert!(
            notes[1].starts_with("note: in outer, called from"),
            "{engine}: {stderr}"
        );
        assert!(notes[1].ends_with("nested.ting:3:28"), "{engine}: {stderr}");
        // `let apply = fn(..)` is named by the binding it is given.
        assert!(
            notes[2].starts_with("note: in apply, called from"),
            "{engine}: {stderr}"
        );
        assert!(notes[2].ends_with("nested.ting:4:1"), "{engine}: {stderr}");
        seen.push(stderr);
    }
    assert_eq!(seen[0], seen[1], "engines disagree");

    // A function with no name of its own says so.
    let anon = dir.join("anon.ting");
    std::fs::write(
        &anon,
        "fn run(f) { return f(); }\nrun(fn() { return nosuch; });\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&anon)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("note: in an anonymous function, called from"),
        "{stderr}"
    );

    // However deep the cap allows, the trace keeps four frames at
    // each end and counts the rest — so the elided count is the cap
    // less the eight that are shown, whatever the cap is (it comes
    // from the stack the runner declares, so it differs between an
    // optimized build and an unoptimized one).
    let deep = dir.join("deep.ting");
    std::fs::write(&deep, "fn r(n) { return r(n + 1); }\nr(0);\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&deep)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error: stack overflow"), "{stderr}");
    let notes = stderr.lines().filter(|l| l.starts_with("note:")).count();
    assert_eq!(notes, 9, "{stderr}");
    let cap: usize = stderr
        .split("max call depth ")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no cap in the diagnostic:\n{stderr}"));
    assert!(
        stderr.contains(&format!("note: ... {} more frames", cap - 8)),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `try` hands a failure's location and call trace back to the
/// program, identically under both engines.
#[test]
fn try_reports_where_a_failure_happened() {
    let path = std::env::temp_dir().join(format!("ting-try-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "fn inner(x) { return x + \"a\"; }\nfn outer(x) { return inner(x); }\nlet r = try(fn() { return outer(1); });\nprint(r[\"at\"][\"line\"], r[\"at\"][\"col\"]);\nfor f in r[\"trace\"] { print(f[\"fn\"], f[\"line\"]); }\n",
    )
    .unwrap();
    let mut seen = Vec::new();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg(&path)
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(0), "{engine}");
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        assert_eq!(stdout.lines().next(), Some("1 22"), "{engine}: {stdout}");
        assert!(stdout.contains("inner 2"), "{engine}: {stdout}");
        assert!(stdout.contains("outer 3"), "{engine}: {stdout}");
        assert!(stdout.contains("nil 3"), "{engine}: {stdout}");
        seen.push(stdout);
    }
    assert_eq!(seen[0], seen[1], "engines disagree");
    let _ = std::fs::remove_file(&path);
}

/// `--coverage` says which lines ran, on stderr, and says the same
/// thing whichever engine ran them.
#[test]
fn coverage_flag_reports_the_lines_that_ran() {
    let path = std::env::temp_dir().join(format!("ting-coverage-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "fn taken(n) {\n  if n > 0 {\n    return \"yes\";\n  }\n  return \"no\";\n}\nfn never() {\n  print(\"unreached\");\n}\nprint(taken(1));\n",
    )
    .unwrap();
    let mut seen = Vec::new();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg("--coverage")
            .arg(&path)
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(0), "{engine}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "yes\n", "{engine}");
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        // The branch not taken and the body of the function never
        // called are the two lines that did not run.
        assert!(stderr.contains("missed 5, 8"), "{engine}: {stderr}");
        assert!(stderr.starts_with("coverage: "), "{engine}: {stderr}");
        seen.push(stderr);
    }
    assert_eq!(seen[0], seen[1], "engines disagree");

    // Several scripts add up to one report, and a file both of them
    // import is one row rather than two.
    let other = std::env::temp_dir().join(format!("ting-coverage2-{}.ting", std::process::id()));
    std::fs::write(&other, "print(1);\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("--coverage")
        .arg(&path)
        .arg(&other)
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "yes\n1\n");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let rows = stderr
        .lines()
        .filter(|l| l.contains("ting-coverage"))
        .count();
    assert_eq!(rows, 2, "one row per script: {stderr}");
    assert!(stderr.contains("missed 5, 8"), "{stderr}");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&other);
}

/// `--profile` counts what every function did and prints the table on
/// stderr, leaving the program's own output alone.
#[test]
fn profile_flag_counts_calls_per_function() {
    let path = std::env::temp_dir().join(format!("ting-profile-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "fn fib(n) { if n < 2 { return n; } return fib(n - 1) + fib(n - 2); }\nfn once(x) { return x; }\nprint(once(fib(10)));\n",
    )
    .unwrap();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg("--profile")
            .arg(&path)
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(0), "{engine}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), "55\n", "{engine}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        // Two ting functions and the one builtin the script calls.
        assert!(
            stderr.contains("profile: 3 functions, 179 calls"),
            "{engine}: {stderr}"
        );
        let rows: Vec<&str> = stderr.lines().skip(2).collect();
        // Each row names how often it ran, how long it spent there
        // itself and where it came from. Which row comes first is a
        // matter of microseconds on a loaded machine, so nothing here
        // asserts an order between them.
        assert!(
            rows.iter()
                .any(|r| r.contains("177") && r.contains("fib") && r.ends_with(".ting:1:1")),
            "{engine}: {stderr}"
        );
        assert!(
            rows.iter()
                .any(|r| r.contains("once") && r.ends_with(".ting:2:1")),
            "{engine}: {stderr}"
        );
        // A builtin is in the table too, and says so instead of
        // naming a ting file.
        assert!(
            rows.iter()
                .any(|r| r.contains("print") && r.ends_with("a builtin")),
            "{engine}: {stderr}"
        );
        for row in &rows {
            assert!(row.contains("ms  "), "{engine}: {stderr}");
        }
    }

    // Only the busiest rows are printed; the rest are counted.
    let many = std::env::temp_dir().join(format!("ting-rows-{}.ting", std::process::id()));
    let mut src = String::new();
    for i in 0..30 {
        src.push_str(&format!("fn f{i}() {{ return {i}; }}\n"));
    }
    for i in 0..30 {
        src.push_str(&format!("f{i}();\n"));
    }
    std::fs::write(&many, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("--profile")
        .arg(&many)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("profile: 30 functions"), "{stderr}");
    assert_eq!(stderr.lines().count(), 23, "{stderr}");
    assert!(
        stderr.trim_end().ends_with("... 10 more functions"),
        "{stderr}"
    );
    let _ = std::fs::remove_file(&many);

    // Self time, not total: a function that only delegates keeps
    // almost none of the time its callee spends.
    let delegating = std::env::temp_dir().join(format!("ting-self-{}.ting", std::process::id()));
    std::fs::write(
        &delegating,
        "fn spin(n) { let s = 0; let i = 0; while i < n { s = s + i; i = i + 1; } return s; }\nfn only_calls(n) { return spin(n); }\nprint(only_calls(200000));\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("--profile")
        .arg(&delegating)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let rows: Vec<&str> = stderr.lines().skip(2).collect();
    // A 200000-iteration loop against a single delegating call: this
    // ordering is not a matter of microseconds.
    let rank = |name: &str| rows.iter().position(|r| r.contains(name));
    assert!(rank("spin") < rank("only_calls"), "{stderr}");
    let _ = std::fs::remove_file(&delegating);
    // Without the flag, nothing is counted and nothing is said.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&path)
        .output()
        .expect("failed to run ting");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
    let _ = std::fs::remove_file(&path);
}

/// `--check` follows local imports: a broken module reached through
/// `import("./...")` is reported under its own path, once, and fails
/// the check; embedded stdlib imports are not files and are skipped.
#[test]
fn check_flag_follows_local_imports() {
    let dir = std::env::temp_dir().join(format!("ting-check-imports-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(
        dir.join("main.ting"),
        "let l = import(\"lib/list.ting\");\nlet a = import(\"./sub/a.ting\");\nlet b = import(\"./sub/b.ting\");\nprint(l, a, b);\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("sub/a.ting"),
        "let b = import(\"../sub/b.ting\");\n",
    )
    .unwrap();
    std::fs::write(dir.join("sub/b.ting"), "fn broken( {\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", dir.join("main.ting").to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("b.ting:1:12: error:"), "{stderr}");
    assert_eq!(stderr.matches("b.ting:1:12").count(), 1, "{stderr}");
    assert!(
        !stderr.contains("main.ting:") && !stderr.contains("a.ting:"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--test --fail-fast` stops at the first failing file: later files
/// are skipped (never run), the summary counts them, and in TAP mode
/// they are `# SKIP` lines so the plan still adds up.
#[test]
fn test_flag_fail_fast_skips_the_rest() {
    let root = std::env::temp_dir().join(format!("ting-fail-fast-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a.ting"), "print(1);\n").unwrap();
    std::fs::write(root.join("b.ting"), "fail(\"red\");\n").unwrap();
    let marker = root.join("c-ran");
    std::fs::write(
        root.join("c.ting"),
        format!("write_file({:?}, \"yes\");\n", marker.to_str().unwrap()),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", "--fail-fast", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("FAIL ") && stdout.contains("b.ting"),
        "{stdout}"
    );
    assert!(stdout.contains("1 passed, 1 failed, 1 skipped"), "{stdout}");
    assert!(!stdout.contains("c.ting"), "{stdout}");
    assert!(!marker.exists(), "c.ting ran despite --fail-fast");

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", "--tap", "--fail-fast", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("1..3\n"), "{stdout}");
    assert!(stdout.contains("# SKIP fail-fast"), "{stdout}");
    assert!(
        stdout.contains("# 1 passed, 1 failed, 1 skipped"),
        "{stdout}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `--doc FILE.ting` lists the user's own top-level functions with the
/// comment above each, the way a stdlib module is listed.
#[test]
fn doc_flag_lists_a_user_file() {
    let path = std::env::temp_dir().join(format!("ting-doc-file-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "# Adds one. Never fails.\nfn inc(n) { return n + 1; }\n\nfn helper() { return 0; }\nlet x = 1;\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(
        stdout.starts_with(&format!("{}:\n", path.display())),
        "{stdout}"
    );
    assert!(stdout.contains("\n  inc(n)  Adds one.\n"), "{stdout}");
    assert!(stdout.contains("\n  helper()"), "{stdout}");
    assert!(!stdout.contains("let x"), "{stdout}");
    let _ = std::fs::remove_file(&path);
}

/// A container that contains itself prints with a cycle marker where
/// the recursion would start, on both engines, instead of overflowing
/// the stack; str() goes through the same path.
#[test]
fn cyclic_values_print_with_a_marker() {
    let path = std::env::temp_dir().join(format!("ting-cyclic-print-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "let xs = [1];\npush(xs, xs);\nprint(xs);\nlet m = {\"k\": 1};\nm[\"me\"] = m;\nprint(m);\nlet ys = [[xs]];\nprint(str(ys));\n",
    )
    .unwrap();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg(&path)
            .output()
            .expect("failed to run ting");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(0), "{engine}: {stdout}");
        assert_eq!(
            stdout, "[1, [...]]\n{\"k\": 1, \"me\": {...}}\n[[[1, [...]]]]\n",
            "{engine}"
        );
    }
    let _ = std::fs::remove_file(&path);
}

/// Comparing cyclic containers terminates on both engines: two cycles
/// of the same shape are equal, a cycle with a different element is
/// not, and a container equals itself.
#[test]
fn cyclic_values_compare_without_overflowing() {
    let path = std::env::temp_dir().join(format!("ting-cyclic-eq-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "let a = [1];\npush(a, a);\nlet b = [1];\npush(b, b);\nlet c = [2];\npush(c, c);\nprint(a == b, a == c, a == a, a != b);\nlet m = {\"k\": 1};\nm[\"me\"] = m;\nlet n = {\"k\": 1};\nn[\"me\"] = n;\nprint(m == n);\n",
    )
    .unwrap();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg(&path)
            .output()
            .expect("failed to run ting");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(0), "{engine}: {stdout}");
        assert_eq!(stdout, "true false true false\ntrue\n", "{engine}");
    }
    let _ = std::fs::remove_file(&path);
}

/// json_str on a cyclic value is a catchable error on both engines,
/// in the compact and the pretty form; the same container appearing
/// twice without a cycle still encodes.
#[test]
fn json_str_reports_cycles_as_errors() {
    let path = std::env::temp_dir().join(format!("ting-cyclic-json-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "let a = [1];\npush(a, a);\nlet r = try(fn() { return json_str(a); });\nprint(r[\"err\"]);\nlet p = try(fn() { return json_str({\"a\": a}, 2); });\nprint(p[\"err\"]);\nlet shared = [1];\nprint(json_str([shared, shared]));\n",
    )
    .unwrap();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg(&path)
            .output()
            .expect("failed to run ting");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(0), "{engine}: {stdout}");
        assert_eq!(
            stdout,
            "json_str cannot encode a cyclic value\njson_str cannot encode a cyclic value\n[[1],[1]]\n",
            "{engine}"
        );
    }
    let _ = std::fs::remove_file(&path);
}

/// `:history` lists every chunk that evaluated without error, numbered,
/// multi-line chunks indented under their number; a chunk that failed
/// is left out, and `:clear` empties the transcript.
#[test]
fn repl_history_lists_successful_chunks() {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"let x = 1;\nfn inc(n) {\n  return n + 1;\n}\nnosuch\ninc(x)\n:history\n:clear\n:history\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(
            "  1  let x = 1;\n  2  fn inc(n) {\n       return n + 1;\n     }\n  3  inc(x)\n"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("nosuch"), "{stdout}");
    assert!(stdout.contains("(nothing evaluated yet)"), "{stdout}");
    assert_eq!(out.status.code(), Some(0));
}

/// `:save FILE` writes the transcript as a script that replays the
/// session; with nothing evaluated it says so and writes no file.
#[test]
fn repl_save_writes_a_runnable_script() {
    use std::io::Write as _;
    let path = std::env::temp_dir().join(format!("ting-repl-save-{}.ting", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    let script = format!(
        ":save {p}\nlet x = 2;\nfn sq(n) {{\n  return n * n;\n}}\nnosuch\nprint(sq(x));\n:save {p}\n",
        p = path.display()
    );
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("(nothing to save yet)"), "{stdout}");
    assert!(stdout.contains("(saved 3 chunk(s) to "), "{stdout}");
    let saved = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        saved,
        "let x = 2;\n\nfn sq(n) {\n  return n * n;\n}\n\nprint(sq(x));\n"
    );
    let rerun = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&path)
        .output()
        .expect("failed to run ting");
    assert_eq!(String::from_utf8_lossy(&rerun.stdout), "4\n");
    let _ = std::fs::remove_file(&path);
}

/// `:doc` alone prints the table of contents and `:doc MODULE` one
/// module, as the CLI's --doc does.
#[test]
fn repl_doc_alone_lists_everything() {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b":doc\n:doc math\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("builtins:\n"), "{stdout}");
    assert!(stdout.contains("\nlib/list.ting:\n"), "{stdout}");
    assert!(
        stdout.contains("\nlib/math.ting:\n  clamp(x, lo, hi)"),
        "{stdout}"
    );
    assert_eq!(out.status.code(), Some(0));
}

/// `:load FILE` resolves the file's relative imports against the
/// file's directory (like `ting FILE`) and names the file in its
/// diagnostics, not "repl".
#[test]
fn repl_load_uses_the_files_directory_and_name() {
    use std::io::Write as _;
    let dir = std::env::temp_dir().join(format!("ting-repl-load-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("m.ting"), "fn hi() { return \"hi\"; }\n").unwrap();
    std::fs::write(
        dir.join("main.ting"),
        "let m = import(\"./m.ting\");\nlet greeting = m[\"hi\"]();\n",
    )
    .unwrap();
    std::fs::write(dir.join("bad.ting"), "let y = nosuch;\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    let script = format!(
        ":load {main}\ngreeting\n:load {bad}\n",
        main = dir.join("main.ting").display(),
        bad = dir.join("bad.ting").display()
    );
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains("\"hi\"\n"), "{stdout}\n{stderr}");
    // A successful load says what it added; a failed one does not.
    assert!(stdout.contains("main.ting: 2 new binding(s))"), "{stdout}");
    assert_eq!(stdout.matches("new binding(s)").count(), 1, "{stdout}");
    assert!(
        stderr.contains("bad.ting:1:9: error: undefined variable 'nosuch'"),
        "{stderr}"
    );
    assert!(!stderr.contains("repl:"), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A failed import names the path it resolved to (relative to the
/// importing file) and says no embedded module matched, on both
/// engines.
#[test]
fn failed_import_says_where_it_looked() {
    let dir = std::env::temp_dir().join(format!("ting-import-miss-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(
        dir.join("sub/main.ting"),
        "let m = import(\"../nowhere.ting\");\n",
    )
    .unwrap();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg(dir.join("sub/main.ting"))
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(1), "{engine}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("cannot import \"../nowhere.ting\": no file at "),
            "{engine}: {stderr}"
        );
        assert!(stderr.contains("nowhere.ting ("), "{engine}: {stderr}");
        assert!(
            stderr.contains("and no embedded module of that name"),
            "{engine}: {stderr}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A CRLF file that is otherwise formatted passes --fmt-check, and
/// --fmt on a misformatted CRLF file keeps every line ending.
#[test]
fn fmt_keeps_crlf_line_endings() {
    let dir = std::env::temp_dir().join(format!("ting-fmt-crlf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let clean = dir.join("clean.ting");
    std::fs::write(&clean, "let x = 1;\r\nprint(x);\r\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--fmt-check", clean.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let messy = dir.join("messy.ting");
    std::fs::write(&messy, "let   x=1;\r\nprint( x );\r\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--fmt", messy.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        std::fs::read_to_string(&messy).unwrap(),
        "let x = 1;\r\nprint(x);\r\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--check` warns about a `let` inside a function body (or any block)
/// that nothing in that block uses; `_`-prefixed and used ones are
/// silent, and a use in a nested block counts.
#[test]
fn check_flag_warns_about_unused_local_lets() {
    let path = std::env::temp_dir().join(format!("ting-check-locals-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "fn f(a) {\n  let stale = 2;\n  let _scratch = 3;\n  let kept = 4;\n  if a > 0 { return kept; }\n  return a;\n}\nprint(f(1));\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: `stale` is never used"),
        "{stderr}"
    );
    assert!(stderr.contains(":2:7:"), "{stderr}");
    assert!(
        !stderr.contains("`kept`") && !stderr.contains("_scratch") && !stderr.contains("`a`"),
        "{stderr}"
    );
    let _ = std::fs::remove_file(&path);
}

/// `--check` warns about a let, fn or parameter named after a builtin;
/// ordinary names are silent.
#[test]
fn check_flag_warns_about_shadowed_builtins() {
    let path = std::env::temp_dir().join(format!("ting-check-shadow-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "let len = 3;\nfn print(x) { return x; }\nfn f(map, total) { return total; }\nprint(len, f(1, 2));\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    for name in ["len", "print", "map"] {
        assert!(
            stderr.contains(&format!("warning: `{name}` shadows a builtin")),
            "{name}: {stderr}"
        );
    }
    assert!(
        !stderr.contains("`total`") && !stderr.contains("`f`") && !stderr.contains("`x`"),
        "{stderr}"
    );
    let _ = std::fs::remove_file(&path);
}

/// `--check --strict` turns warnings into a failing exit status; a
/// clean file still passes, and without the flag warnings stay advice.
#[test]
fn check_flag_strict_fails_on_warnings() {
    let dir = std::env::temp_dir().join(format!("ting-check-strict-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let warned = dir.join("warned.ting");
    std::fs::write(&warned, "let unused = 1;\nprint(2);\n").unwrap();
    let clean = dir.join("clean.ting");
    std::fs::write(&clean, "print(2);\n").unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(args)
            .output()
            .expect("failed to run ting")
    };
    let out = run(&["--check", warned.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let out = run(&["--check", "--strict", warned.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("warning: `unused` is never used"));
    let out = run(&["--check", "--strict", clean.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--fmt` over a directory with a file that does not lex reports it,
/// still reformats the files after it, and exits 1; `--fmt-check`
/// likewise lists every file that would change. `--check` continues
/// past an unreadable file the same way.
#[test]
fn fmt_and_check_process_every_file_before_failing() {
    let dir = std::env::temp_dir().join(format!("ting-every-file-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.ting"), "let   a=1;\n").unwrap();
    std::fs::write(dir.join("b.ting"), "let b = \"unterminated;\n").unwrap();
    std::fs::write(dir.join("c.ting"), "let   c=3;\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--fmt-check", dir.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("would reformat") && stdout.contains("c.ting"),
        "{stdout}"
    );
    assert!(
        stdout.ends_with("2 would change, 0 unchanged, 1 failed\n"),
        "{stdout}"
    );
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--fmt", dir.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unterminated string"));
    assert_eq!(
        std::fs::read_to_string(dir.join("c.ting")).unwrap(),
        "let c = 3;\n"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.ends_with("2 reformatted, 0 unchanged, 1 failed\n"),
        "{stdout}"
    );
    // A second pass finds nothing to do and says so; a single file
    // gets no summary line.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args([
            "--fmt-check",
            dir.join("a.ting").to_str().unwrap(),
            dir.join("c.ting").to_str().unwrap(),
        ])
        .output()
        .expect("failed to run ting");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "0 would change, 2 unchanged, 0 failed\n"
    );
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--fmt-check", dir.join("a.ting").to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");
    // --check: a missing file is reported and the next one still runs.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args([
            "--check",
            dir.join("missing.ting").to_str().unwrap(),
            dir.join("b.ting").to_str().unwrap(),
        ])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot read") && stderr.contains("unterminated string"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `-h` and `-V` work like their long forms; an option no mode knows
/// is a usage error (exit 2) that names it and points at --help, at
/// the top level and under --test, --check and --fmt.
#[test]
fn unknown_options_are_usage_errors() {
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(args)
            .output()
            .expect("failed to run ting")
    };
    let out = run(&["-h"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("usage:"));
    let out = run(&["-V"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("ting "));
    for args in [
        &["--nosuch"][..],
        &["--test", "--nosuch", "selftest"][..],
        &["--check", "--nosuch", "-"][..],
        &["--fmt-check", "--nosuch"][..],
    ] {
        let out = run(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("unknown option --nosuch (see --help)"),
            "{args:?}: {stderr}"
        );
    }
}

/// Exit codes mean one thing each: 0 for success, 1 for a failure the
/// tool reports (a script that raises, a red test), 2 for a usage error
/// (a mode with no operand, a bad option value).
#[test]
fn exit_codes_are_zero_one_two() {
    let dir = std::env::temp_dir().join(format!("ting-exit-codes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ok = dir.join("ok.ting");
    std::fs::write(&ok, "print(1);\n").unwrap();
    let bad = dir.join("bad.ting");
    std::fs::write(&bad, "fail(\"red\");\n").unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(args)
            .output()
            .expect("failed to run ting")
            .status
            .code()
    };
    assert_eq!(run(&[ok.to_str().unwrap()]), Some(0));
    assert_eq!(run(&[bad.to_str().unwrap()]), Some(1));
    assert_eq!(run(&["--test", bad.to_str().unwrap()]), Some(1));
    assert_eq!(run(&["--test"]), Some(2));
    assert_eq!(run(&["--check"]), Some(2));
    assert_eq!(run(&["--fmt"]), Some(2));
    assert_eq!(
        run(&["--test", "--slow", "x", ok.to_str().unwrap()]),
        Some(2)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every line --doc prints fits an 80-column terminal: a long comment
/// wraps under its signature, in the index and for a single entry.
#[test]
fn doc_output_fits_eighty_columns() {
    for args in [
        &["--doc"][..],
        &["--doc", "json"][..],
        &["--doc", "get_in"][..],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(args)
            .output()
            .expect("failed to run ting");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let long: Vec<&str> = stdout.lines().filter(|l| l.chars().count() > 78).collect();
        assert!(
            long.is_empty(),
            "{args:?} has lines over 78 columns: {long:?}"
        );
        assert!(stdout.contains("get_in(v, path)"), "{args:?}: {stdout}");
    }
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "get_in"])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.starts_with("get_in(v, path)  [lib/json.ting]\n  The value at path"),
        "{stdout}"
    );
    assert!(stdout.lines().count() >= 3, "{stdout}");
}

#[test]
fn repl_help_lists_builtins() {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b":help\nprint(1 + 1);\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("abs(n)"), "{stdout}");
    assert!(stdout.contains("json_str(v)"), "{stdout}");
    // The session keeps working after :help.
    assert!(stdout.contains("\n2\n"), "{stdout}");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn repl_load_runs_a_file_into_the_session() {
    use std::io::Write as _;
    let dir = std::env::temp_dir();
    let script = dir.join(format!("ting-load-{}.ting", std::process::id()));
    std::fs::write(&script, "let base = 40;\n").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            format!(
                ":load {}\nprint(base + 2);\n:load /missing.ting\n",
                script.display()
            )
            .as_bytes(),
        )
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // The loaded binding is visible to later lines.
    assert!(stdout.contains("42"), "{stdout}");
    // A bad path reports and the session survives (exit 0 on ctrl-d).
    assert!(stderr.contains("cannot read /missing.ting"), "{stderr}");
    assert_eq!(out.status.code(), Some(0));
    let _ = std::fs::remove_file(&script);
}

#[test]
fn read_file_dash_reads_stdin() {
    use std::io::Write as _;
    let dir = std::env::temp_dir();
    let script = dir.join(format!("ting-stdin-{}.ting", std::process::id()));
    std::fs::write(
        &script,
        "let text = read_file(\"-\");\nprint(len(text), trim(text));\n",
    )
    .unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"hello pipe\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "11 hello pipe\n");
    assert_eq!(out.status.code(), Some(0));
    let _ = std::fs::remove_file(&script);
}

#[test]
fn write_file_append_mode() {
    let dir = std::env::temp_dir();
    let script = dir.join(format!("ting-append-{}.ting", std::process::id()));
    let data = dir.join(format!("ting-append-{}.txt", std::process::id()));
    std::fs::write(
        &script,
        format!(
            "write_file({p:?}, \"one\\n\");\n\
             write_file({p:?}, \"two\\n\", \"append\");\n\
             print(read_file({p:?}));\n\
             write_file({p:?}, \"three\\n\");\n\
             print(read_file({p:?}));\n\
             let bad = try(fn() {{ return write_file({p:?}, \"x\", \"nope\"); }});\n\
             print(has(bad, \"err\"));\n",
            p = data.to_str().unwrap()
        ),
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Append grew the file; the later plain write truncated it.
    assert_eq!(stdout, "one\ntwo\n\nthree\n\ntrue\n", "{stdout}");
    assert_eq!(out.status.code(), Some(0));
    let _ = std::fs::remove_file(&script);
    let _ = std::fs::remove_file(&data);
}

#[test]
fn repl_vars_lists_user_bindings() {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b":vars\nlet total = 4;\nfn double(x) { return x * 2; }\n:vars\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("(no bindings yet)"), "{stdout}");
    assert!(stdout.contains("double: function"), "{stdout}");
    assert!(stdout.contains("total: int"), "{stdout}");
    // Builtins stay out of the listing.
    assert!(!stdout.contains("print: "), "{stdout}");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn repl_clear_resets_the_session() {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"let gone = 1;\n:clear\n:vars\nprint(gone);\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains("(session cleared)"), "{stdout}");
    assert!(stdout.contains("(no bindings yet)"), "{stdout}");
    // The old binding is really gone, and the session survives the error.
    assert!(stderr.contains("undefined variable 'gone'"), "{stderr}");
    assert_eq!(out.status.code(), Some(0));
}

/// A reader that goes away (`ting x.ting | head -1`) must end the run
/// quietly: exit 0, nothing on stderr — both for print() in scripts
/// and for the REPL's own output.
#[test]
fn broken_pipe_exits_quietly() {
    use std::io::Read;
    let script = std::env::temp_dir().join("ting-io-broken-pipe.ting");
    std::fs::write(&script, "for i in range(200000) { print(i); }\n").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    let mut first = [0u8; 2];
    child.stdout.take().unwrap().read_exact(&mut first).unwrap();
    // Dropping stdout above closed the read end; the script's next
    // print hits EPIPE.
    let out = child.wait_with_output().unwrap();
    assert_eq!(&first, b"0\n");
    assert!(out.status.success(), "status: {:?}", out.status);
    assert!(
        out.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(":help\n".repeat(200).as_bytes())
        .unwrap();
    let mut first = [0u8; 3];
    child.stdout.take().unwrap().read_exact(&mut first).unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(&first, b"abs");
    assert!(out.status.success(), "status: {:?}", out.status);
    assert!(
        out.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `-` means stdin for the tool flags: `--fmt -` filters to stdout,
/// `--fmt-check -` and `--check -` judge the piped source.
#[test]
fn tool_flags_accept_dash_for_stdin() {
    let run = |args: &[&str], stdin: &str| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to run ting");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    };

    let out = run(&["--fmt", "-"], "let   x=1;\n");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "let x = 1;\n");

    // Already formatted input still echoes through: a filter never
    // swallows its input.
    let out = run(&["--fmt", "-"], "let x = 1;\n");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "let x = 1;\n");

    let out = run(&["--fmt-check", "-"], "let   x=1;\n");
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("would reformat -"));

    let out = run(&["--check", "-"], "exit(7);\n");
    assert_eq!(out.status.code(), Some(0), "clean stdin, not executed");

    let out = run(&["--check", "-"], "let = 3;\n");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("expected variable name"), "got: {stderr}");
}

#[test]
fn doc_flag_lists_everything_or_a_module() {
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc"])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.starts_with("builtins:\n"), "{stdout}");
    assert!(stdout.contains("\n  len(x)"), "{stdout}");
    assert!(stdout.contains("\nlib/list.ting:\n"), "{stdout}");
    assert!(stdout.contains("\nlib/test.ting:\n"), "{stdout}");
    assert!(stdout.contains("\n  median(xs)"), "{stdout}");

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "math"])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.starts_with("lib/math.ting:\n"), "{stdout}");
    assert!(stdout.contains("  clamp(x, lo, hi)"), "{stdout}");
    assert!(
        !stdout.contains("builtins:") && !stdout.contains("lib/list.ting"),
        "{stdout}"
    );

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "nosuch"])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("no builtin, stdlib function, module or file named nosuch"),
        "{stderr}"
    );
}

#[test]
fn checks_are_counted_and_reported_on_request() {
    let dir = std::env::temp_dir().join("ting-checks");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let script = dir.join("c.ting");
    std::fs::write(&script, "assert(1 == 1, \"a\");\nassert(2 == 2, \"b\");\n").expect("write");

    // Both engines count the same, and only when asked.
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .env("TING_TEST_REPORT", "1")
            .arg(&script)
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(0));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("ting-checks: 2"), "{engine}: {stderr}");
    }
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("ting-checks"), "{stderr}");

    // A failing assert is still a check that ran.
    std::fs::write(
        &script,
        "assert(1 == 1, \"a\");\nassert(false, \"boom\");\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .env("TING_TEST_REPORT", "1")
        .arg(&script)
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("assertion failed: boom"), "{stderr}");
    assert!(stderr.contains("ting-checks: 2"), "{stderr}");

    // A file that checks nothing says so.
    std::fs::write(&script, "# nothing here\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .env("TING_TEST_REPORT", "1")
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("ting-checks: 0"), "{stderr}");
}

#[test]
fn check_flag_warns_about_code_that_can_never_run() {
    let dir = std::env::temp_dir().join("ting-unreachable");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let script = dir.join("r.ting");
    std::fs::write(
        &script,
        "fn f() { return 1; print(\"never\"); }\nprint(f());\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0), "warnings do not fail the check");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("this can never run: the return above always leaves"),
        "{stderr}"
    );
    // Only the first orphan is reported, at its own column.
    assert_eq!(stderr.matches("warning:").count(), 1, "{stderr}");
    assert!(stderr.contains(":1:20:"), "{stderr}");

    // `break` and `continue` end a block the same way.
    std::fs::write(
        &script,
        "for x in [1, 2] {\n  if x > 1 { break; print(x); }\n  print(x);\n}\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("the break above always leaves"), "{stderr}");

    // A return at the end of its block, and one inside a branch that
    // the block continues past, are both fine.
    std::fs::write(
        &script,
        "fn h(n) {\n  if n > 0 { return n; }\n  return 0;\n}\nprint(h(1), h(-1));\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("can never run"), "{stderr}");
}

#[test]
fn check_flag_warns_about_a_duplicate_map_key() {
    let dir = std::env::temp_dir().join("ting-dupkey");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let script = dir.join("d.ting");
    std::fs::write(&script, "let m = {\"a\": 1, \"a\": 2};\nprint(m);\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0), "warnings do not fail the check");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("duplicate key `a`: the last one wins"),
        "{stderr}"
    );
    // The second key is the one underlined.
    assert!(stderr.contains(":1:18:"), "{stderr}");

    // Nested literals are judged too.
    std::fs::write(&script, "print({\"x\": [{\"y\": 1, \"y\": 2}]});\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("duplicate key `y`"), "{stderr}");

    // A computed key is decided at run time: nothing is claimed.
    std::fs::write(
        &script,
        "let k = \"a\";\nlet m = {k: 1, \"a\": 2};\nprint(m);\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("duplicate key"), "{stderr}");
}

#[test]
fn check_flag_warns_about_a_call_that_cannot_match() {
    let dir = std::env::temp_dir().join("ting-arity");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let script = dir.join("a.ting");
    std::fs::write(
        &script,
        "fn f(a, b) { return a + b; }\nprint(f(1));\nprint(f(1, 2));\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0), "warnings do not fail the check");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`f` takes 2 arguments, called with 1"),
        "{stderr}"
    );
    // Only the wrong call is reported.
    assert_eq!(stderr.matches("warning:").count(), 1, "{stderr}");

    // One parameter reads as a singular.
    std::fs::write(&script, "fn g(a) { return a; }\nprint(g(1, 2));\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`g` takes 1 argument, called with 2"),
        "{stderr}"
    );

    // A name that is rebound, shadowed or taken as a parameter is
    // beyond the pass: nothing is claimed about it.
    std::fs::write(
        &script,
        "fn h(a) { return a; }\nh = fn(a, b) { return a + b; };\nprint(h(1, 2));\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("called with"), "{stderr}");

    std::fs::write(
        &script,
        "fn apply(k, x) { return k(x); }\nfn twice(v) { return v * 2; }\nprint(apply(twice, 3));\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("called with"), "{stderr}");
}

#[test]
fn check_flag_warns_about_a_name_bound_nowhere() {
    let dir = std::env::temp_dir().join("ting-unbound");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let script = dir.join("u.ting");
    std::fs::write(&script, "fn g(a) { return a + b; }\nprint(g(1));\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0), "warnings do not fail the check");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("`b` is bound nowhere"), "{stderr}");
    assert!(stderr.contains(":1:22:"), "points at the name: {stderr}");

    // The nearest name in scope is named.
    std::fs::write(&script, "let total = 1;\nprint(totl);\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("`totl` is bound nowhere (did you mean `total`?)"),
        "{stderr}"
    );

    // Forward references, loop variables, parameters, closures and
    // builtins are all bound: nothing to report.
    std::fs::write(
        &script,
        "let xs = [1, 2];\nfn later(n) { return helper(n); }\nfn helper(n) { return n * 2; }\nfor x in xs {\n  let y = x + 1;\n  print(later(y));\n}\nprint(len(xs));\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("bound nowhere"), "{stderr}");

    // An assignment to a name that was never bound is reported too.
    std::fs::write(&script, "nope = 1;\nprint(nope);\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("`nope` is bound nowhere"), "{stderr}");
}

#[test]
fn unknown_options_suggest_the_nearest_option() {
    for (typo, meant) in [("--fmr", "--fmt"), ("--tst", "--test"), ("--lps", "--lsp")] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args([typo, "x"])
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(&format!(
                "unknown option {typo} (did you mean {meant}?) (see --help)"
            )),
            "{stderr}"
        );
    }

    // Nothing near it (and never for a one-letter option).
    for typo in ["--nosuch", "-x"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args([typo, "x"])
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(&format!("unknown option {typo} (see --help)")),
            "{stderr}"
        );
    }
}

#[test]
fn doc_flag_suggests_the_nearest_documented_name() {
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "medain"])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(
            "no builtin, stdlib function, module or file named medain (did you mean median?)"
        ),
        "{stderr}"
    );

    // Module names count as documented names too.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "strng"])
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("(did you mean string?)"), "{stderr}");

    // Nothing near it, nothing added.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "zqxjw"])
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("did you mean"), "{stderr}");
}

#[test]
fn unknown_members_suggest_the_nearest_one() {
    let dir = std::env::temp_dir().join("ting-suggest-member");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let script = dir.join("m.ting");
    std::fs::write(
        &script,
        "let l = import(\"lib/list.ting\");\nprint(l[\"medain\"]([1, 2, 3]));\n",
    )
    .expect("write");

    // The checker's warning names the member it meant.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("lib/list.ting has no `medain` (did you mean `median`?)"),
        "{stderr}"
    );

    // So does the runtime error, on both engines.
    let mut seen = Vec::new();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg(&script)
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(1));
        seen.push(String::from_utf8_lossy(&out.stderr).to_string());
    }
    assert_eq!(seen[0], seen[1], "{seen:?}");
    assert!(
        seen[0].contains("key \"medain\" not found (did you mean \"median\"?)"),
        "{}",
        seen[0]
    );

    // A plain map with nothing close keeps the bare message.
    std::fs::write(
        &script,
        "let m = {\"alpha\": 1, \"beta\": 2};\nprint(m[\"gamma\"]);\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("key \"gamma\" not found") && !stderr.contains("did you mean"),
        "{stderr}"
    );
}

#[test]
fn undefined_names_suggest_the_nearest_one() {
    let dir = std::env::temp_dir().join("ting-suggest");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let script = dir.join("s.ting");
    std::fs::write(&script, "let count = 1;\nprint(cont);\n").expect("write");

    // Both engines say the same thing, down to the byte.
    let mut seen = Vec::new();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg(&script)
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(1));
        seen.push(String::from_utf8_lossy(&out.stderr).to_string());
    }
    assert_eq!(seen[0], seen[1], "{seen:?}");
    assert!(
        seen[0].contains("undefined variable 'cont' (did you mean 'count'?)"),
        "{}",
        seen[0]
    );

    // A builtin counts as a name in scope; an assignment is told too.
    std::fs::write(&script, "print(lenght(\"abc\"));\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("(did you mean 'len'?)"), "{stderr}");

    std::fs::write(&script, "let count = 1;\ncont = 2;\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot assign to undefined variable 'cont' (did you mean 'count'?)"),
        "{stderr}"
    );

    // Nothing near it, nothing added.
    std::fs::write(&script, "print(zqxjw);\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("undefined variable 'zqxjw'") && !stderr.contains("did you mean"),
        "{stderr}"
    );
}

#[test]
fn doc_flag_explains_several_names_at_once() {
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "len", "median", "slug"])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    let len_at = stdout.find("len(x)").expect("len missing");
    let median_at = stdout.find("median(xs)").expect("median missing");
    let slug_at = stdout.find("slug(s)").expect("slug missing");
    assert!(len_at < median_at && median_at < slug_at, "{stdout}");
    // One blank line between entries, none at the end.
    assert_eq!(stdout.matches("\n\n").count(), 2, "{stdout}");
    assert!(!stdout.ends_with("\n\n"), "{stdout}");

    // An unknown name fails the run, but the known ones are still printed.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "len", "nosuch", "slug"])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    assert!(
        stdout.contains("len(x)") && stdout.contains("slug(s)"),
        "{stdout}"
    );
    assert!(
        stderr.contains("no builtin, stdlib function, module or file named nosuch"),
        "{stderr}"
    );
}

#[test]
fn doc_flag_explains_a_name_from_the_shell() {
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "median"])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(
        stdout.contains("median(xs)  [lib/list.ting]") && stdout.contains("sorted values"),
        "{stdout}"
    );

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "len"])
        .output()
        .expect("failed to run ting");
    assert!(
        String::from_utf8_lossy(&out.stdout).starts_with("len("),
        "builtin"
    );

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "nosuchthing"])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("no builtin, stdlib function, module or file named nosuchthing")
    );
}

#[test]
fn repl_doc_explains_builtins_and_stdlib_functions() {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b":doc len\n:doc median\n:doc count\n:doc nosuchthing\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("len("), "{stdout}");
    assert!(stdout.contains("median(xs)  [lib/list.ting]"), "{stdout}");
    assert!(stdout.contains("sorted values"), "{stdout}");
    // `count` exists in two modules: both are listed.
    assert!(
        stdout.contains("count(xs, v)  [lib/list.ting]")
            && stdout.contains("count(s, sub)  [lib/string.ting]"),
        "{stdout}"
    );
    assert!(
        stdout.contains("(no builtin, stdlib function or module named nosuchthing)"),
        "{stdout}"
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn repl_time_reports_milliseconds() {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b":time len(range(1000))\n:time let z = 1;\n:time 1 +\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains("1000\n("), "value then timing: {stdout}");
    assert_eq!(stdout.matches(" ms)").count(), 3, "{stdout}");
    assert!(stderr.contains("needs a complete expression"), "{stderr}");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn repl_fmt_reprints_the_last_chunk_formatted() {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn repl");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b":fmt\nlet   x=[1,2 ,3];\n:fmt\n:fmt\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("(nothing to format yet)"), "{stdout}");
    // Printed twice: :fmt does not consume the chunk.
    assert_eq!(
        stdout.matches("let x = [1, 2, 3];\n").count(),
        2,
        "{stdout}"
    );
    assert_eq!(out.status.code(), Some(0));
}

/// `--test` runs every file in its own process: ok/FAIL per file with
/// the diagnostic under a failure, a summary, exit 1 if any failed.
#[test]
fn test_flag_runs_files_and_summarises() {
    let dir = std::env::temp_dir();
    let good = dir.join(format!("ting-test-good-{}.ting", std::process::id()));
    let bad = dir.join(format!("ting-test-bad-{}.ting", std::process::id()));
    // A passing test may print; the runner discards stdout. A failing
    // one may exit(1) itself (lib/test.ting's summary does) without
    // taking the runner down.
    std::fs::write(&good, "assert(1 + 1 == 2); print(\"noise\");\n").unwrap();
    std::fs::write(&bad, "assert(1 == 2, \"arithmetic is broken\"); exit(1);\n").unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", good.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.starts_with("ok   "), "{stdout}");
    assert!(stdout.contains("1 passed, 0 failed"), "{stdout}");
    assert!(!stdout.contains("noise"), "{stdout}");

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", good.to_str().unwrap(), bad.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("FAIL "), "{stdout}");
    assert!(stdout.contains("arithmetic is broken"), "{stdout}");
    assert!(stdout.contains("1 passed, 1 failed"), "{stdout}");

    let _ = std::fs::remove_file(&good);
    let _ = std::fs::remove_file(&bad);
}

/// `--test` says how much each file verified: a count per file, a
/// total in the summary, and a passing file that checked nothing
/// named as such.
#[test]
fn test_flag_counts_checks() {
    let dir = std::env::temp_dir().join(format!("ting-test-counts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("two.ting"), "assert(true);\nassert(1 == 1);\n").unwrap();
    std::fs::write(dir.join("one.ting"), "assert(true);\n").unwrap();
    std::fs::write(dir.join("none.ting"), "# nothing to see\n").unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", dir.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("none.ting (no checks)"), "{stdout}");
    assert!(stdout.contains("one.ting (1 check)"), "{stdout}");
    assert!(stdout.contains("two.ting (2 checks)"), "{stdout}");
    assert!(
        stdout.contains("3 passed, 0 failed, 3 checks (1 file checked nothing)"),
        "{stdout}"
    );

    // lib/test.ting's helpers count as checks too.
    std::fs::write(
        dir.join("lib_test.ting"),
        "let t = import(\"lib/test.ting\");\nt[\"check\"](\"one\", true);\nt[\"check_eq\"](\"two\", 1, 1);\nt[\"summary\"]();\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", dir.join("lib_test.ting").to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("lib_test.ting (2 checks)"), "{stdout}");

    // The count is a TAP comment in --tap mode.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", "--tap", dir.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\n# no checks\n"), "{stdout}");
    assert!(stdout.contains("\n# 2 checks\n"), "{stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

/// A directory argument expands to every .ting file beneath it, in
/// sorted order, recursively; other files are ignored.
/// `--tap` emits a TAP stream: plan, ok/not ok lines numbered from 1,
/// diagnostics and timings as comments, exit status as before.
#[test]
fn test_flag_tap_output() {
    let dir = std::env::temp_dir();
    let good = dir.join(format!("ting-tap-good-{}.ting", std::process::id()));
    let bad = dir.join(format!("ting-tap-bad-{}.ting", std::process::id()));
    std::fs::write(&good, "assert(true);\n").unwrap();
    std::fs::write(&bad, "fail(\"tap boom\");\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args([
            "--test",
            "--tap",
            good.to_str().unwrap(),
            bad.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[0], "1..2", "{stdout}");
    assert!(lines[1].starts_with("ok 1 - "), "{stdout}");
    assert!(stdout.contains("\nnot ok 2 - "), "{stdout}");
    assert!(
        stdout.contains("\n# ") && stdout.contains("tap boom"),
        "{stdout}"
    );
    assert!(stdout.contains("# time: "), "{stdout}");
    assert!(
        stdout.trim_end().ends_with("# 1 passed, 1 failed, 1 check"),
        "{stdout}"
    );
    // Every non-comment line is a plan or a test line: TAP-clean.
    for l in &lines {
        assert!(
            l.starts_with('#')
                || l.starts_with("ok ")
                || l.starts_with("not ok ")
                || l.starts_with("1.."),
            "stray line: {l}"
        );
    }
    let _ = std::fs::remove_file(&good);
    let _ = std::fs::remove_file(&bad);
}

#[test]
fn test_flag_expands_directories() {
    let root = std::env::temp_dir().join(format!("ting-test-dir-{}", std::process::id()));
    // Named to sort BEFORE the files: the runner must still list the
    // directory's own files first, then descend.
    let nested = root.join("aa-nested");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(root.join("b.ting"), "assert(true);\n").unwrap();
    std::fs::write(root.join("a.ting"), "assert(true);\n").unwrap();
    std::fs::write(root.join("notes.txt"), "not a test\n").unwrap();
    std::fs::write(nested.join("c.ting"), "assert(false, \"deep failure\");\n").unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    let a = stdout.find("a.ting").unwrap();
    let b = stdout.find("b.ting").unwrap();
    let c = stdout.find("c.ting").unwrap();
    assert!(
        a < b && b < c,
        "sorted, files before the nested dir: {stdout}"
    );
    assert!(!stdout.contains("notes.txt"), "{stdout}");
    assert!(stdout.contains("deep failure"), "{stdout}");
    assert!(stdout.contains("2 passed, 1 failed"), "{stdout}");

    // -j runs files concurrently but reports them in the same order
    // with the same summary as the sequential run.
    let seq = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", "--tap", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let par = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", "--tap", "-j", "3", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let strip_times = |b: &[u8]| -> String {
        String::from_utf8_lossy(b)
            .lines()
            .filter(|l| !l.starts_with("# time:"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(strip_times(&seq.stdout), strip_times(&par.stdout));
    assert_eq!(par.status.code(), Some(1));
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", "-j", "0", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(2), "-j 0 is a usage error");

    // --slow N appends the slowest files after the summary; as a TAP
    // comment in --tap mode so the stream stays clean.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", "--slow", "2", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let after = stdout
        .split("2 passed, 1 failed, 3 checks\n")
        .nth(1)
        .expect("summary first");
    assert!(after.starts_with("slowest:\n"), "{stdout}");
    assert_eq!(after.matches("ms ").count(), 2, "{stdout}");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", "--tap", "--slow", "1", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\n# slowest:\n# "), "{stdout}");

    // --filter keeps only paths containing the substring; a filter
    // that matches nothing is an error, not "0 passed".
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", "--filter", "c.ti", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("FAIL ") && !stdout.contains("a.ting"),
        "{stdout}"
    );
    assert!(stdout.contains("0 passed, 1 failed"), "{stdout}");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", root.to_str().unwrap(), "--filter", "zzz"])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no .ting files matching"));

    let _ = std::fs::remove_dir_all(&root);
}

/// A watching child and the output it has produced so far.
/// `--watch` never exits on its own, so its stdout is drained by a
/// thread into a buffer the test polls, and the child is killed when
/// the test is done with it. Nothing here waits on a stopwatch: every
/// step polls until what it expects arrives, or a generous deadline
/// runs out.
struct Watcher {
    child: std::process::Child,
    seen: std::sync::Arc<std::sync::Mutex<String>>,
}

impl Watcher {
    fn spawn(args: &[&str]) -> Watcher {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to run ting");
        let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        // Warnings go to stderr and results to stdout; a watcher's
        // reader wants both, in one buffer.
        for stream in [
            Box::new(child.stdout.take().unwrap()) as Box<dyn std::io::Read + Send>,
            Box::new(child.stderr.take().unwrap()),
        ] {
            let sink = std::sync::Arc::clone(&seen);
            let mut stream = stream;
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                while let Ok(n) = stream.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    sink.lock()
                        .unwrap()
                        .push_str(&String::from_utf8_lossy(&buf[..n]));
                }
            });
        }
        Watcher { child, seen }
    }

    fn wait_for(&self, needle: &str) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while std::time::Instant::now() < deadline {
            if self.seen().contains(needle) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        false
    }

    fn seen(&self) -> String {
        self.seen.lock().unwrap().clone()
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `--test --watch` runs the files, then runs them again whenever one
/// of them changes on disk — a rule line naming the run and its cause
/// separating one run from the next.
#[test]
fn test_flag_watch_runs_again_when_a_file_changes() {
    let root = std::env::temp_dir().join(format!("ting-watch-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let a = root.join("a.ting");
    std::fs::write(&a, "assert(1 == 1, \"one\");\n").unwrap();

    let w = Watcher::spawn(&["--test", "--watch", root.to_str().unwrap()]);
    let first = w.wait_for("-- run 1 ") && w.wait_for("1 passed, 0 failed");
    // A file added to a watched directory joins the next run.
    std::fs::write(root.join("b.ting"), "assert(2 == 2, \"two\");\n").unwrap();
    let added = first && w.wait_for("-- run 2: ") && w.wait_for("2 passed, 0 failed");
    // An edit to a file already watched sets off another run.
    std::fs::write(
        &a,
        "assert(3 == 3, \"three\");\nassert(4 == 4, \"four\");\n",
    )
    .unwrap();
    let changed = added && w.wait_for("-- run 3: ");

    let seen = w.seen();
    drop(w);
    let _ = std::fs::remove_dir_all(&root);

    assert!(first, "no first run:\n{seen}");
    assert!(added, "a new file did not set off a run:\n{seen}");
    assert!(changed, "an edit did not set off a run:\n{seen}");
    assert!(seen.contains("b.ting added"), "{seen}");
    assert!(seen.contains("a.ting changed"), "{seen}");
    // The rule reaches eighty columns, so runs are told apart at a
    // glance in a scrollback.
    let rule = seen
        .lines()
        .find(|l| l.starts_with("-- run 1 "))
        .expect("no rule line");
    assert_eq!(rule.chars().count(), 80, "{rule:?}");
}

/// `--check --watch` and `--fmt-check --watch` re-run the same way,
/// and `--fmt --watch` — which would answer its own rewrites — is a
/// usage error naming the two modes that write nothing.
#[test]
fn check_and_fmt_check_watch_too() {
    let root = std::env::temp_dir().join(format!("ting-watch-check-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let a = root.join("a.ting");
    std::fs::write(&a, "let x = 1;\nprint(x);\n").unwrap();

    let w = Watcher::spawn(&["--check", "--watch", root.to_str().unwrap()]);
    let first = w.wait_for("-- run 1 ");
    std::fs::write(&a, "let x = 1;\nprint(nope);\n").unwrap();
    let checked = first && w.wait_for("-- run 2: ") && w.wait_for("`nope` is bound nowhere");
    let seen = w.seen();
    drop(w);
    assert!(first, "no first check:\n{seen}");
    assert!(checked, "the edited file was not checked again:\n{seen}");

    std::fs::write(&a, "let x = 1;\nprint(x);\n").unwrap();
    let w = Watcher::spawn(&["--fmt-check", "--watch", root.to_str().unwrap()]);
    let first = w.wait_for("-- run 1 ");
    std::fs::write(&a, "let x   =  1;\nprint( x );\n").unwrap();
    let noticed = first && w.wait_for("would reformat") && w.wait_for("-- run 2: ");
    let seen = w.seen();
    drop(w);
    assert!(first, "no first fmt check:\n{seen}");
    assert!(noticed, "the edited file was not re-checked:\n{seen}");

    // Rewriting in place under a watch would trigger the watch.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--fmt", "--watch", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--fmt-check or --fmt --diff"), "{stderr}");

    let _ = std::fs::remove_dir_all(&root);
}

/// A script can arrive on stdin: `ting -` runs it, the arguments
/// after the dash reach args(), diagnostics name `-` the way every
/// tool flag does, and a relative import resolves against the
/// working directory because a piped script has no directory of its
/// own. input() sees EOF, since the script was the stream.
#[test]
fn a_script_can_arrive_on_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["-", "one", "two"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"print(args());\nprint(input());\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "[\"one\", \"two\"]\nnil\n"
    );

    // A failure names `-` as the file, at the right line and column.
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"let x = 1;\nfail(\"boom\");\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("-:2:1: error: boom"), "{stderr}");

    // A relative import resolves against the working directory.
    let root = std::env::temp_dir().join(format!("ting-stdin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("m.ting"), "let two = 2;\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("-")
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"let m = import(\"m.ting\");\nprint(m[\"two\"]);\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "2\n");
    let _ = std::fs::remove_dir_all(&root);
}

/// `list_dir` answers with the names in a directory, sorted, and says
/// so when the path is not a readable directory. Names, not paths:
/// joining is the caller's business.
#[test]
fn list_dir_names_a_directory() {
    let root = std::env::temp_dir().join(format!("ting-listdir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let data = root.join("data");
    std::fs::create_dir_all(data.join("sub")).unwrap();
    std::fs::write(data.join("b.ting"), "").unwrap();
    std::fs::write(data.join("a.txt"), "").unwrap();

    let script = root.join("show.ting");
    std::fs::write(
        &script,
        format!(
            "print(list_dir({:?}));\nprint(try(fn() {{ return list_dir({:?}); }})[\"err\"]);\n",
            data.to_str().unwrap(),
            data.join("a.txt").to_str().unwrap()
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut lines = stdout.lines();
    assert_eq!(lines.next(), Some("[\"a.txt\", \"b.ting\", \"sub\"]"));
    // A file is not a directory, and the message says which path.
    let err = lines.next().unwrap_or_default();
    assert!(err.starts_with("cannot list "), "{err}");
    assert!(err.contains("a.txt"), "{err}");

    let _ = std::fs::remove_dir_all(&root);
}

/// `exists` and `is_dir` are questions — an absent or unreadable path
/// is `false`, not an error — and `make_dir` creates parents and
/// forgives a directory that is already there, so `write_file` into a
/// fresh tree works from inside the language.
#[test]
fn exists_is_dir_and_make_dir() {
    let root = std::env::temp_dir().join(format!("ting-makedir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let deep = root.join("a").join("b");
    let file = deep.join("x.txt");

    let script = root.join("run.ting");
    std::fs::write(
        &script,
        format!(
            "print(exists({deep:?}), is_dir({deep:?}));\n\
             make_dir({deep:?});\n\
             make_dir({deep:?});\n\
             write_file({file:?}, \"ok\");\n\
             print(exists({file:?}), is_dir({file:?}), read_file({file:?}));\n",
            deep = deep.to_str().unwrap(),
            file = file.to_str().unwrap()
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "false false\ntrue false ok\n"
    );
    assert!(file.exists(), "make_dir did not create the tree");
    let _ = std::fs::remove_dir_all(&root);
}

/// lib/fs.ting's `entries`, `walk` and `walk_ext` over a real tree —
/// the part of that module the selftest cannot cover, since a ting
/// script can make a directory but not remove one.
#[test]
fn fs_module_walks_a_tree() {
    let root = std::env::temp_dir().join(format!("ting-fswalk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let tree = root.join("tree");
    std::fs::create_dir_all(tree.join("deep").join("deeper")).unwrap();
    for (path, _) in [
        (tree.join("a.ting"), ()),
        (tree.join("b.txt"), ()),
        (tree.join("deep").join("c.ting"), ()),
        (tree.join("deep").join("deeper").join("d.ting"), ()),
    ] {
        std::fs::write(path, "").unwrap();
    }

    let script = root.join("walk.ting");
    std::fs::write(
        &script,
        format!(
            "let fs = import(\"lib/fs.ting\");\n\
             let root = {tree:?};\n\
             print(list_dir(root));\n\
             print(len(fs[\"entries\"](root)));\n\
             let found = fs[\"walk\"](root);\n\
             print(len(found), found[0] == fs[\"join_path\"]([root, \"a.ting\"]));\n\
             print(len(fs[\"walk_ext\"](root, \"ting\")));\n\
             print(fs[\"walk\"](fs[\"join_path\"]([root, \"b.txt\"])) == [fs[\"join_path\"]([root, \"b.txt\"])]);\n",
            tree = tree.to_str().unwrap()
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        // names sorted; three direct children; four files with no
        // directories among them; three of them .ting; and a file
        // walks to itself.
        "[\"a.ting\", \"b.txt\", \"deep\"]\n3\n4 true\n3\ntrue\n"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The call-depth cap is derived from the stack the runner gives its
/// interpreter thread, not from a number someone guessed: a plain
/// recursive fold over three hundred elements — impossible under the
/// old cap of 200 — now runs, and the refusal, when it comes, names
/// the cap it enforced. The number itself depends on the build
/// profile (an unoptimized frame costs several times an optimized
/// one), so the test asserts the floor, not the value.
#[test]
fn recursion_goes_as_deep_as_the_stack_allows() {
    let deep = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            c.stdin
                .take()
                .unwrap()
                .write_all(b"fn sum(xs, i) { if i >= len(xs) { return 0; } return xs[i] + sum(xs, i + 1); }\nprint(sum(range(0, 300), 0));\n")?;
            c.wait_with_output()
        })
        .expect("failed to run ting");
    assert!(
        deep.status.success(),
        "300-deep recursion failed:\n{}",
        String::from_utf8_lossy(&deep.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&deep.stdout), "44850\n");

    let over = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            c.stdin.take().unwrap().write_all(
                b"fn f(n) { if n == 0 { return 0; } return f(n - 1) + 1; }\nprint(f(1000000));\n",
            )?;
            c.wait_with_output()
        })
        .expect("failed to run ting");
    assert_eq!(over.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&over.stderr);
    let cap: usize = stderr
        .split("max call depth ")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no cap in the diagnostic:\n{stderr}"));
    assert!(cap >= 512, "cap did not move above the old 200: {cap}");
}

/// `run` against the one program every machine building this is
/// guaranteed to have: the binary under test. Exit code, both
/// streams, and the argv going through untouched.
#[test]
fn run_spawns_a_program_and_reports_what_it_did() {
    let exe = env!("CARGO_BIN_EXE_ting").replace('\\', "/");
    let script = std::env::temp_dir().join("ting-io-run.ting");
    let child = std::env::temp_dir().join("ting-io-run-child.ting");
    let child_path = child.to_str().unwrap().replace('\\', "/");
    std::fs::write(
        &child,
        "print(join(args(), \"|\"));\nfail(\"from the child\");\n",
    )
    .unwrap();
    std::fs::write(
        &script,
        format!(
            "let ok = run(\"{exe}\", [\"--version\"]);\n\
             print(ok[\"code\"], starts_with(ok[\"out\"], \"ting \"), ok[\"err\"] == \"\");\n\
             let bad = run(\"{exe}\", [\"{child_path}\", \"a b\", \"c\"]);\n\
             print(bad[\"code\"], trim(bad[\"out\"]), contains(bad[\"err\"], \"from the child\"));\n\
             print(try(fn() {{ return run(\"ting-no-such-program-xyz\"); }})[\"err\"]);\n"
        ),
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "0 true true");
    // The child's own failure is its exit code, not an error here.
    assert_eq!(lines.next().unwrap(), "1 a b|c true");
    assert!(
        lines.next().unwrap().starts_with("run: cannot start "),
        "unexpected spawn error:\n{text}"
    );
    let _ = std::fs::remove_file(&script);
    let _ = std::fs::remove_file(&child);
}

/// eprint goes to the other stream, and stays behind the stdout it
/// was written after.
#[test]
fn eprint_writes_to_stderr_and_cwd_reports_the_directory() {
    let script = std::env::temp_dir().join("ting-io-eprint.ting");
    std::fs::write(
        &script,
        // The leaf, not the whole path: Windows hands back a
        // canonical form with its own prefix and separators, and this
        // test is about cwd() naming where the process stands, not
        // about how an OS spells it.
        "print(\"data\");\n\
         eprint(\"note\", 1, [2]);\n\
         print(ends_with(cwd(), args()[0]), is_dir(cwd()));\n",
    )
    .unwrap();
    let here = std::env::temp_dir().join("ting-io-cwd-dir");
    std::fs::create_dir_all(&here).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .arg("ting-io-cwd-dir")
        .current_dir(&here)
        .output()
        .expect("failed to run ting");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "data\ntrue true\n");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "note 1 [2]\n");
    let _ = std::fs::remove_file(&script);
    let _ = std::fs::remove_dir(&here);
}
