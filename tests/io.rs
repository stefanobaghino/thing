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

/// Every syntax error in a file, not the first: a reader with three
/// typos used to need three runs to see three messages, and an editor
/// underlined one mistake at a time. The count and the line numbers
/// are both pinned, because "reports more" is easy to get by
/// reporting the same mistake twice.
#[test]
fn check_reports_every_syntax_error_in_one_pass() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("ting-check-many-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "let a = ;\nlet b = 1;\nlet c = ;\nprint(b);\nlet e = ;\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "a file with mistakes fails");
    let errors: Vec<&str> = stderr.lines().filter(|l| l.contains(": error: ")).collect();
    assert_eq!(errors.len(), 3, "three typos, three messages:\n{stderr}");
    for (error, line) in errors.iter().zip(["1:9", "3:9", "5:9"]) {
        assert!(
            error.contains(&format!(":{line}: error: expected expression, found ';'")),
            "expected {line} in {error}"
        );
    }
    let _ = std::fs::remove_file(&path);
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

/// The other end of 1043 and 1044: a file whose failed check went
/// unprinted fails the run, and `--check` says so before it is run.
#[test]
fn check_flag_names_a_file_that_prints_none_of_its_checks() {
    let dir = std::env::temp_dir().join(format!("ting-check-summary-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let quiet = dir.join("quiet.ting");
    let spoken = dir.join("spoken.ting");
    std::fs::write(
        &quiet,
        "let t = import(\"lib/test.ting\");\nt[\"check\"](\"one\", 1 == 1);\n",
    )
    .unwrap();
    std::fs::write(
        &spoken,
        "let t = import(\"lib/test.ting\");\nt[\"check\"](\"one\", 1 == 1);\nt[\"summary\"]();\n",
    )
    .unwrap();
    let check = |path: &std::path::Path, strict: bool| {
        let mut args = vec!["--check"];
        if strict {
            args.push("--strict");
        }
        args.push(path.to_str().unwrap());
        Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(args)
            .output()
            .expect("failed to run ting")
    };
    let out = check(&quiet, false);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "a warning is not an error");
    assert!(
        stderr.contains(
            "warning: nothing prints these checks — this file never calls `t[\"summary\"]()`"
        ),
        "{stderr}"
    );
    assert!(stderr.contains(":2:4:"), "points at the check: {stderr}");
    assert_eq!(check(&quiet, true).status.code(), Some(1), "strict fails");
    let out = check(&spoken, false);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(stderr.matches("warning:").count(), 0, "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A member no module has: the checker and the run say the same
/// sentence about it, down to the order the names come in.
#[test]
fn a_missing_member_reads_the_same_to_the_checker_and_the_run() {
    let path = std::env::temp_dir().join(format!("ting-member-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "let a = import(\"lib/args.ting\");\nprint(a[\"zzqqxx\"]);\n",
    )
    .unwrap();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(args)
            .arg(path.to_str().unwrap())
            .output()
            .expect("failed to run ting");
        String::from_utf8_lossy(&out.stderr).into_owned()
    };
    let said = "lib/args.ting has no `zzqqxx` (it has `flag_of`, `help`, `main`, `option_of`, `pad`, `parse`, `spec_trouble`)";
    let checked = run(&["--check"]);
    assert!(checked.contains(&format!("warning: {said}")), "{checked}");
    let ran = run(&[]);
    assert!(ran.contains(&format!("error: {said}")), "{ran}");
    let _ = std::fs::remove_file(&path);
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

/// A pattern binds several names at once, and each answers for
/// itself: the checker warns about the ones nothing reads, at the top
/// level and inside a block, and never about a hole.
#[test]
fn check_flag_warns_about_unused_pattern_bindings() {
    let path = std::env::temp_dir().join(format!("ting-check-pattern-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "let [kept, idle] = [1, 2];
         let [_, [_deep, seen]] = [0, [1, 2]];
         fn f() {
  let [near, far] = [3, 4];
  return near;
}
         print(kept, seen, f());
",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("warning: `idle` is never used"), "{stderr}");
    assert!(stderr.contains("warning: `far` is never used"), "{stderr}");
    // What is read, what is deliberately named `_deep`, and the holes
    // themselves are all left alone.
    for quiet in ["`kept`", "`seen`", "`near`", "_deep", "`_`"] {
        assert!(!stderr.contains(quiet), "{quiet} in {stderr}");
    }
    let _ = std::fs::remove_file(&path);
}

/// `--check` counts the arguments of a call into an imported module
/// against what that module declares, so a stdlib call with the wrong
/// count is caught before it runs rather than on the line it reaches.
#[test]
fn check_flag_counts_arguments_of_module_calls() {
    let path = std::env::temp_dir().join(format!("ting-check-member-{}.ting", std::process::id()));
    std::fs::write(
        &path,
        "let st = import(\"lib/string.ting\");
let cs = import(\"lib/csv.ting\");
print(st[\"repeat\"](\"x\"));
print(cs[\"parse\"](\"a,b\"));
print(st[\"truncate\"](\"abc\", 2));
",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", path.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: `repeat` takes 2 arguments, called with 1"),
        "{stderr}"
    );
    // The calls that are right say nothing, defaults included.
    assert!(
        !stderr.contains("`parse`") && !stderr.contains("`truncate`"),
        "{stderr}"
    );
    let _ = std::fs::remove_file(&path);
}

/// `--check` reads a module beside the file the same way it reads an
/// embedded one: an unknown member is named, and a call's arguments
/// are counted against what that module declares.
#[test]
fn check_flag_reads_a_module_next_door() {
    let dir = std::env::temp_dir().join(format!("ting-check-local-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("util.ting"),
        "fn helper(a, b) { return a + b; }\nfn only(a) { return a; }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.ting"),
        "let u = import(\"./util.ting\");\nprint(u[\"helper\"](1));\nprint(u[\"helpr\"](1, 2));\nprint(u[\"only\"](1));\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", dir.join("main.ting").to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: `helper` takes 2 arguments, called with 1"),
        "{stderr}"
    );
    assert!(
        stderr.contains("warning: ./util.ting has no `helpr` (did you mean `helper`?)"),
        "{stderr}"
    );
    assert!(!stderr.contains("`only`"), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A lib/ beside the script shadows the embedded stdlib at run time,
/// so the checker reads the file on disk rather than the copy in the
/// binary: its arities are the ones a call is answered against.
#[test]
fn check_flag_prefers_a_lib_on_disk() {
    let dir = std::env::temp_dir().join(format!("ting-check-shadow-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(
        dir.join("lib/string.ting"),
        "fn truncate(s) { return s; }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("m.ting"),
        "let st = import(\"lib/string.ting\");\nprint(st[\"truncate\"](\"x\", 3));\nprint(st[\"repeat\"](\"x\", 2));\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check", dir.join("m.ting").to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: `truncate` takes 1 argument, called with 2"),
        "{stderr}"
    );
    // `repeat` is in the embedded module and not in this one, which is
    // the whole point of shadowing.
    assert!(
        stderr.contains("warning: lib/string.ting has no `repeat`"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
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
            stderr.contains("note: in boom(), called from") && stderr.contains("main.ting:3:"),
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
        stderr.contains("note: in mean(xs = []), called from") && stderr.contains("emb.ting:2:"),
        "{stderr}"
    );
    assert!(!stderr.contains("panicked"), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}
/// An import resolves to an absolute path, but the reader named the
/// file relative to where they ran the command — and so does
/// `--check`. Every place the error path prints that file says the
/// same short name: the diagnostic header, each trace note, and the
/// `file` of `try`'s `at` and of every frame in its `trace`.
#[test]
fn an_error_in_an_imported_file_names_it_the_way_check_does() {
    let dir = std::env::temp_dir().join(format!("ting-shorten-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("boom.ting"),
        "fn inner() { fail(\"boom\"); }\nfn outer() { inner(); }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.ting"),
        "let boom = import(\"./boom.ting\");\nlet r = try(boom[\"outer\"]);\n\
         print(r[\"at\"][\"file\"]);\nprint(r[\"trace\"][0][\"file\"]);\n\
         print(r[\"trace\"][1][\"file\"]);\nboom[\"outer\"]();\n",
    )
    .unwrap();
    // The name `--check` gives the module, to compare the run against.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .current_dir(&dir)
        .args(["--check", "boom.ting"])
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .current_dir(&dir)
            .arg("main.ting")
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(1), "{engine}");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(stdout, "boom.ting\nboom.ting\nmain.ting\n", "{engine}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.starts_with("boom.ting:1:14: error: boom"),
            "{engine}: {stderr}"
        );
        assert!(
            stderr.contains("note: in inner(), called from boom.ting:2:14"),
            "{engine}: {stderr}"
        );
        assert!(
            stderr.contains("note: in outer(), called from main.ting:6:1"),
            "{engine}: {stderr}"
        );
        // Nowhere does the absolute path the import resolved to leak
        // through: not into the header, a note, or the try map.
        let abs = dir.display().to_string();
        assert!(!stdout.contains(&abs), "{engine}: {stdout}");
        assert!(!stderr.contains(&abs), "{engine}: {stderr}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A terminal lays a line out in columns, not in characters: an
/// ideograph takes two and a combining accent none. The caret row
/// under a diagnostic is padded to match, so it sits under the token
/// however the line before it is spelled.
#[test]
fn the_caret_row_lines_up_under_the_token() {
    let dir = std::env::temp_dir().join(format!("ting-caret-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // print(" is seven columns, the three ideographs six more, and
    // ", three: sixteen before the name.
    let wide = dir.join("wide.ting");
    std::fs::write(&wide, "print(\"\u{65e5}\u{672c}\u{8a9e}\", nosuch);\n").unwrap();
    // The same line with a combining accent, which takes none: the e
    // it sits on is the only column it adds.
    let mark = dir.join("mark.ting");
    std::fs::write(&mark, "print(\"e\u{301}\", nosuch);\n").unwrap();
    for engine in ["vm", "eval"] {
        for (file, pad) in [(&wide, 16), (&mark, 11)] {
            let out = Command::new(env!("CARGO_BIN_EXE_ting"))
                .env("TING_ENGINE", engine)
                .arg(file)
                .output()
                .expect("failed to run ting");
            let stderr = String::from_utf8_lossy(&out.stderr);
            let want = format!("   | {}^^^^^^", " ".repeat(pad));
            assert!(
                stderr.lines().any(|l| l == want),
                "{engine} {file:?}: wanted {want:?} in {stderr}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// An error carries every call it unwound through: one note per
/// frame, innermost first, named after the function it was raised in,
/// identical under both engines. A long trace is elided in the middle
/// so a runaway recursion cannot bury the message.
/// The note lines carry what each call was given, and the two caps
/// that keep a long list or a wide signature from burying the
/// message. Both engines must print the same text, from the binary
/// rather than from the library.
#[test]
fn a_trace_shows_the_arguments_within_its_caps() {
    let dir = std::env::temp_dir().join(format!("ting-args-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("args.ting");
    std::fs::write(
        &script,
        "fn wide(a, b, c, d, e) { return a + e; }\n\
         fn big(xs) { return wide(1, 2, 3, 4, xs); }\n\
         big(range(0, 40));\n",
    )
    .unwrap();
    let mut seen = Vec::new();
    for engine in ["vm", "eval"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .env("TING_ENGINE", engine)
            .arg(&script)
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(1), "{engine}");
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let notes: Vec<&str> = stderr.lines().filter(|l| l.starts_with("note:")).collect();
        assert_eq!(notes.len(), 2, "{engine}: {stderr}");
        // Five parameters: four named, the rest counted.
        assert!(
            notes[0].starts_with("note: in wide(a = 1, b = 2, c = 3, d = 4, and 1 more)"),
            "{engine}: {stderr}"
        );
        // A long value is cut where the cap falls, and says so.
        assert!(
            notes[1].contains("xs = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 1..."),
            "{engine}: {stderr}"
        );
        assert!(!notes[1].contains(", 39]"), "{engine}: {stderr}");
        seen.push(stderr);
    }
    assert_eq!(seen[0], seen[1], "engines disagree");
    std::fs::remove_dir_all(&dir).ok();
}

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
            notes[0].starts_with("note: in inner(x = 1), called from"),
            "{engine}: {stderr}"
        );
        assert!(notes[0].ends_with("nested.ting:2:22"), "{engine}: {stderr}");
        assert!(
            notes[1].starts_with("note: in outer(x = 1), called from"),
            "{engine}: {stderr}"
        );
        assert!(notes[1].ends_with("nested.ting:3:28"), "{engine}: {stderr}");
        // `let apply = fn(..)` is named by the binding it is given,
        // and a function passed as an argument renders as one.
        assert!(
            notes[2].starts_with("note: in apply(f = <fn(x)>), called from"),
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
        stderr.contains("note: in an anonymous function(), called from"),
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

/// A coverage report is about the code its reader wrote. The stdlib
/// module the binary carries is named, not counted — and the same
/// module as a real file beside the script is theirs, and is.
#[test]
fn coverage_leaves_out_the_stdlib_that_came_with_the_binary() {
    let dir = std::env::temp_dir().join(format!("ting-coverage-lib-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("suite.ting");
    std::fs::write(
        &script,
        "let t = import(\"lib/test.ting\");\nt[\"check\"](\"kept\", true);\n",
    )
    .unwrap();
    let run = |dir: &std::path::Path| -> String {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .current_dir(dir)
            .arg("--coverage")
            .arg("suite.ting")
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(0));
        String::from_utf8_lossy(&out.stderr).to_string()
    };
    let rows = |report: &str| -> Vec<String> {
        report
            .lines()
            .filter(|l| l.starts_with(' ') && l.contains('%'))
            .map(|l| l.to_string())
            .collect()
    };

    let embedded = run(&dir);
    assert_eq!(
        rows(&embedded).len(),
        1,
        "only the script counts: {embedded}"
    );
    assert!(rows(&embedded)[0].ends_with("suite.ting"), "{embedded}");
    assert!(
        embedded.contains("not counted: lib/test.ting (embedded in the binary)"),
        "{embedded}"
    );
    // Two lines of script, both reached.
    assert!(
        embedded.starts_with("coverage: 2 of 2 lines (100%)"),
        "{embedded}"
    );

    // The same import, answered by a file the reader wrote: it is
    // their code, so it is in the table and nothing is left out.
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(
        dir.join("lib").join("test.ting"),
        "fn check(name, ok) {\n  if !ok {\n    print(name);\n  }\n}\n",
    )
    .unwrap();
    let theirs = run(&dir);
    assert_eq!(
        rows(&theirs).len(),
        2,
        "their lib/test.ting counts: {theirs}"
    );
    assert!(!theirs.contains("not counted"), "{theirs}");

    let _ = std::fs::remove_dir_all(&dir);
}

/// The two stdlib functions that print and exit — lib/test.ting's
/// `summary` and lib/args.ting's `main` — can only be checked from
/// outside, in a process of their own. Coverage found them untested
/// (iteration 645), which is what it is for.
#[test]
fn the_stdlib_functions_that_exit_do_what_they_say() {
    let dir = std::env::temp_dir().join(format!("ting-exits-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    // A suite with one failure: the FAIL: line, the totals, exit 1.
    let failing = dir.join("failing.ting");
    std::fs::write(
        &failing,
        "let t = import(\"lib/test.ting\");\nt[\"check\"](\"kept\", true);\nt[\"check_eq\"](\"broken\", 1, 2);\nt[\"summary\"]();\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&failing)
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("FAIL: broken: got 1, want 2"), "{stdout}");
    assert!(stdout.contains("1 passed, 1 failed"), "{stdout}");

    // The same suite with nothing wrong leaves happily.
    let passing = dir.join("passing.ting");
    std::fs::write(
        &passing,
        "let t = import(\"lib/test.ting\");\nt[\"check\"](\"kept\", true);\nt[\"summary\"]();\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&passing)
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("1 passed, 0 failed"),
        "{:?}",
        out.stdout
    );

    // args.main: --help prints the help and leaves 0; a command line
    // the spec does not describe prints the trouble and leaves 2.
    let cli = dir.join("cli.ting");
    std::fs::write(
        &cli,
        "let cli = import(\"lib/args.ting\");\nlet spec = {\"name\": \"demo\", \"summary\": \"a demo\", \"options\": [], \"positionals\": []};\nlet got = cli[\"main\"](spec, args());\nprint(\"ran\");\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&cli)
        .arg("--help")
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("usage: demo"), "{stdout}");
    assert!(
        !stdout.contains("ran"),
        "--help leaves before the program: {stdout}"
    );

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&cli)
        .arg("--nosuch")
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("demo: "), "{stderr}");
    assert!(
        stderr.contains("usage: demo"),
        "the help goes with it: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A module reached through `..` resolves to an absolute path. That
/// is the right identity and the wrong thing to print beside rows
/// named relative to the directory the command ran in.
#[test]
fn reports_name_modules_relative_to_the_working_directory() {
    let cwd = std::env::current_dir().expect("cwd");
    // The row keeps the platform's own separator, so the needle has to
    // be built rather than written.
    let want = std::path::Path::new("lib")
        .join("test.ting")
        .display()
        .to_string();
    let absolute = cwd.display().to_string();
    for flag in ["--coverage", "--profile"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg(flag)
            .arg("selftest/testlib.ting")
            .output()
            .expect("failed to run ting");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(text.contains(&want), "{flag}: {text}");
        assert!(!text.contains(&absolute), "{flag}: {text}");
    }
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
    // Broken at column 12 and still broken now that `[` and `{` open
    // a parameter pattern there: an int is a parameter name nowhere.
    std::fs::write(dir.join("sub/b.ting"), "fn broken( 1\n").unwrap();
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

/// A failing file's own output is the reason it failed, so `--test`
/// repeats it under the FAIL line. Until 901 the child's stdout went
/// to /dev/null and `FAIL <path>` was the whole report — which is
/// what `lib/test.ting` writes its failures to. A file that PASSES
/// stays silent, and a file that prints a great deal is cut off with
/// a count.
#[test]
fn test_flag_shows_why_a_file_failed() {
    let root = std::env::temp_dir().join(format!("ting-test-why-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Prints its reason, then exits 1 the way summary() does.
    std::fs::write(
        root.join("a.ting"),
        "print(\"FAIL: three kilos: got 9, want 8\");\nexit(1);\n",
    )
    .unwrap();
    // Says nothing and passes: its output must not appear either way.
    std::fs::write(root.join("b.ting"), "print(\"quiet success\");\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.contains("     FAIL: three kilos: got 9, want 8"),
        "the reason is missing:\n{stdout}"
    );
    assert!(
        !stdout.contains("quiet success"),
        "a passing file stayed noisy:\n{stdout}"
    );

    // A flood is cut off, and says how much it cut.
    std::fs::write(
        root.join("a.ting"),
        "for i in range(0, 500) { print(i); }\nexit(1);\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("more lines"), "no trailer:\n{stdout}");
    assert!(
        stdout.lines().filter(|l| l.starts_with("     ")).count() <= 41,
        "the flood was not cut:\n{stdout}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A file that fails still says how many checks it ran. The count
/// travels from child to harness on a line printed as the process
/// ends, and `exit()` never came back to print it — so a file that
/// failed the way `lib/test.ting`'s `summary()` fails reported ZERO,
/// and the suite's totals lost count exactly when something had gone
/// wrong.
#[test]
fn a_failing_file_still_reports_its_checks() {
    let root = std::env::temp_dir().join(format!("ting-test-counts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("a.ting"),
        "assert(1 == 1, \"one\");\nassert(2 == 2, \"two\");\nprint(\"1 passed, 1 failed\");\nexit(1);\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.contains("0 passed, 1 failed, 2 checks"),
        "the checks were lost with the process:\n{stdout}"
    );
    // And the count is not printed where a reader would read it as
    // part of the file's own output.
    assert!(!stdout.contains("ting-checks"), "{stdout}");
    let _ = std::fs::remove_dir_all(&root);
}

/// `--test --fail-fast` stops at the first failing file: later files
/// are skipped (never run), the summary counts them, and in TAP mode
/// they are `# SKIP` lines so the plan still adds up.
#[test]
fn test_flag_fail_fast_skips_the_rest() {
    let root = std::env::temp_dir().join(format!("ting-fail-fast-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    // a.ting checks something, so that it is a pass rather than a
    // skip: what this test is about is the files after the failure.
    std::fs::write(root.join("a.ting"), "assert(true);\n").unwrap();
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

/// A colon that opens a fresh chunk is the REPL's to answer: a name it
/// does not have suggests the nearest command, a command given without
/// its argument (or with one it does not take) says what it takes, and
/// none of it reaches the parser, which would only report a `:`.
#[test]
fn repl_answers_a_command_it_does_not_have() {
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
        .write_all(b":vras\n:tim 1 + 1\n:xyzzy\n:load\n:help now\nlet x = 1;\n:vars\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    for expected in [
        "(no command :vras — did you mean :vars?)",
        "(no command :tim — did you mean :time?)",
        "(no command :xyzzy — :help lists them)",
        "(:load needs one: :load FILE)",
        "(:help takes nothing after it)",
        "x: 1",
    ] {
        assert!(stdout.contains(expected), "{expected}\n{stdout}");
    }
    // The parser never saw any of it.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("found ':'"), "{stderr}");
    assert_eq!(out.status.code(), Some(0));
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
    assert!(
        stdout.contains("\nlib/list.ting: List helpers, written in ting.\n"),
        "{stdout}"
    );
    // Asked for by name, a module leads with its header and then lists.
    assert!(
        stdout.contains("\nlib/math.ting:\n  Math helpers, written in ting."),
        "{stdout}"
    );
    assert!(stdout.contains("\n  clamp(x, lo, hi)"), "{stdout}");
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
        assert!(stderr.contains("nowhere.ting\" ("), "{engine}: {stderr}");
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
    // The widest header comment in lib/, which --doc prints since 909:
    // its worked examples are copied out line for line, not wrapped, so
    // nothing but the source keeps them inside eighty columns.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "lib/args.ting"])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let long: Vec<&str> = stdout.lines().filter(|l| l.chars().count() > 78).collect();
    assert!(long.is_empty(), "--doc lib/args.ting is too wide: {long:?}");

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
    assert!(stderr.contains("cannot read \"/missing.ting\""), "{stderr}");
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
        .write_all(
            b":vars\nlet total = 4;\nfn double(x) { return x * 2; }\nlet big = range(100000);\n:vars\n",
        )
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("(no bindings yet)"), "{stdout}");
    // What a binding IS, not what kind of thing it is: the type was
    // the one thing the reader could already guess from the name.
    assert!(stdout.contains("double: <fn(x)>"), "{stdout}");
    assert!(stdout.contains("total: 4"), "{stdout}");
    // Builtins stay out of the listing.
    assert!(!stdout.contains("print: "), "{stdout}");
    // A value too wide for a line is cut to one, and says how wide it
    // was: a listing is for running an eye down.
    let line = stdout
        .lines()
        .find(|l| l.starts_with("big: "))
        .unwrap_or_else(|| panic!("no line for big:\n{stdout}"));
    assert!(line.chars().count() < 100, "{line}");
    assert!(line.ends_with("… (688890 characters)"), "{line}");
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
    // The table of contents says what each module is for, in its own
    // words: the first sentence of the header comment (909).
    assert!(
        stdout.contains("\nlib/list.ting: List helpers, written in ting.\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("\nlib/test.ting: A tiny test framework, written in ting.\n"),
        "{stdout}"
    );
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
        stderr.contains("no builtin, stdlib function, module or file matches nosuch"),
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

/// 822 wrote the top-five-words program with a four-line hand-rolled
/// loop instead of `lib/map.ting`'s `top(m, n)`, because `--doc` was
/// an exact-name lookup and there was no way to ask for a function by
/// what it does. A word that names nothing is now searched for.
#[test]
fn doc_flag_searches_descriptions_when_a_word_names_nothing() {
    let doc = |arg: &str| {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(["--doc", arg])
            .output()
            .expect("failed to run ting");
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };

    // The search the milestone was chosen for.
    let (code, stdout) = doc("largest");
    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.starts_with("matching largest:\n"), "{stdout}");
    assert!(stdout.contains("\n  max_by(xs, key)"), "{stdout}");
    assert!(stdout.contains("\nlib/map.ting:\n  top(m, n)"), "{stdout}");

    // A name that IS a function is still answered in full, and first,
    // and then followed by what else the word finds.
    let (code, stdout) = doc("sort");
    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.starts_with("sort(xs)\n"), "{stdout}");
    // What else the word finds comes back as NAMES, grouped by where
    // they live: the entry answered the question, and forty-four
    // entries under it would bury the answer.
    assert!(stdout.contains("\nalso mentioned by:\n"), "{stdout}");
    assert!(stdout.contains("sort_with"), "{stdout}");
    assert!(!stdout.contains("sort_with(xs, cmp)"), "{stdout}");
    // The exact entry is not repeated underneath itself.
    assert_eq!(stdout.matches("A fresh sorted list").count(), 1, "{stdout}");

    // A comment matches only where a WORD of it starts with the
    // query: "arge" is inside "largest" and finds nothing, or every
    // search would drown in the middle of other words.
    let (code, stdout) = doc("arge");
    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.is_empty(), "{stdout}");

    // 825: `format`'s own doc line still described v2.129's format,
    // so the feature v2.130.0 shipped could not be found by the word
    // anyone would look for it under.
    let (code, stdout) = doc("width");
    assert_eq!(code, Some(0), "{stdout}");
    // An index line carries the first sentence only, so the search
    // shows that `format` is the answer and `--doc format` says how.
    assert!(stdout.contains("format(fmt, ...)"), "{stdout}");
    let (code, stdout) = doc("format");
    assert_eq!(code, Some(0), "{stdout}");
    assert!(
        stdout.contains("{:>5}") && stdout.contains("{:.2}"),
        "{stdout}"
    );

    // A module keeps its index, and `list` is the module that proves
    // it: half the comments in the library say "list", so a search
    // would bury it if the branches were the other way round.
    let (code, stdout) = doc("list");
    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.starts_with("lib/list.ting:\n"), "{stdout}");
    // Not "matching" alone: `find_index`'s own comment says it.
    assert!(!stdout.contains("matching list:"), "{stdout}");
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
            "no builtin, stdlib function, module or file matches medain (did you mean median?)"
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

    // And the runtime says the same sentence about the same lookup,
    // on both engines: a module is not a map with keys, it is a file
    // with members, and both surfaces name it that way.
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
        seen[0].contains("lib/list.ting has no `medain` (did you mean `median`?)"),
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

/// A member a module retired into a builtin. The nearest export is a
/// guess; a builtin of exactly that name is the answer, so it wins.
#[test]
fn unknown_members_name_the_builtin() {
    let dir = std::env::temp_dir().join("ting-member-builtin");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let script = dir.join("m.ting");
    std::fs::write(
        &script,
        "let m = import(\"lib/map.ting\");\nprint(m[\"get\"]({\"a\": 1}, \"z\", 0));\n",
    )
    .expect("write");

    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--check"])
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("lib/map.ting has no `get` (`get` is a builtin)"),
        "{stderr}"
    );
    assert!(!stderr.contains("did you mean"), "{stderr}");
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
        stderr.contains("no builtin, stdlib function, module or file matches nosuch"),
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
            .contains("no builtin, stdlib function, module or file matches nosuchthing")
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
        stdout.contains("(no builtin, stdlib function or module matches nosuchthing)"),
        "{stdout}"
    );
    assert_eq!(out.status.code(), Some(0));
}

/// The same question typed two ways has to give the same answer:
/// `:doc WORD` in the REPL is `ting --doc WORD` on the command line,
/// character for character, search included.
#[test]
fn repl_doc_searches_exactly_as_the_flag_does() {
    use std::io::Write as _;
    let both = |word: &str| {
        let flag = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(["--doc", word])
            .output()
            .expect("failed to run ting");
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
            .write_all(format!(":doc {word}\n").as_bytes())
            .unwrap();
        let repl = child.wait_with_output().unwrap();
        (
            String::from_utf8_lossy(&flag.stdout).into_owned(),
            String::from_utf8_lossy(&repl.stdout).into_owned(),
        )
    };

    // A word that names nothing: the search, grouped by module.
    let (flag, repl) = both("largest");
    assert!(flag.starts_with("matching largest:\n"), "{flag}");
    assert!(flag.contains("\nlib/map.ting:\n  top(m, n)"), "{flag}");
    assert_eq!(flag, repl, "the flag and the REPL disagree");

    // A name that is a function: the entry, then what else it finds.
    let (flag, repl) = both("sort");
    assert!(flag.contains("also mentioned by"), "{flag}");
    assert!(flag.contains("sort_with"), "{flag}");
    assert_eq!(flag, repl, "the flag and the REPL disagree");

    // A module keeps its index in both, and `list` is the module
    // that proves it: half the comments in the library say "list", so
    // a search would answer instead of the index if the two branches
    // were the other way round.
    let (flag, repl) = both("list");
    assert!(flag.starts_with("lib/list.ting:\n"), "{flag}");
    // Not "matching" alone: `find_index`'s own comment says it.
    assert!(!flag.contains("matching list:"), "{flag}");
    assert_eq!(flag, repl, "the flag and the REPL disagree");

    // A phrase, which is how a reader asks about something they
    // cannot name: the words must sit together and in order.
    let (flag, repl) = both("how many");
    assert!(flag.starts_with("matching how many:\n"), "{flag}");
    assert!(flag.contains("count_lines(p)"), "{flag}");
    assert_eq!(flag, repl, "the flag and the REPL disagree");
    let (flag, _) = both("many how");
    assert!(!flag.contains("count_lines(p)"), "{flag}");

    // A word that neither names nor describes anything: the REPL says
    // so on stdout and the flag on stderr, so only the suggestion is
    // comparable. 826 gave `top` the word "frequency", which used to
    // be this case and is now a hit — a typo of a name is what stays
    // one.
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
        .write_all(b":doc medain\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("matches medain; did you mean median?"),
        "{stdout}"
    );
}

/// A search can only find words that were written: 825 measured the
/// search and found 19 of 266 entries carrying a signature and
/// nothing else, `list.unique` among them — invisible by
/// construction, however good the matching. Nothing kept them in
/// step with the rest, so this does.
#[test]
fn every_documented_entry_says_what_it_does() {
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc"])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");

    // The index is one line per entry: two spaces, the signature,
    // then two spaces and the first sentence — or, when that will not
    // fit in eighty columns, the sentence wrapped underneath at six.
    let lines: Vec<&str> = stdout.lines().collect();
    let mut entries = 0;
    let mut bare = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("  ") else {
            continue;
        };
        if rest.starts_with(' ') || !rest.contains('(') {
            continue;
        }
        entries += 1;
        let described = rest.split_once(")  ").is_some()
            || lines.get(i + 1).is_some_and(|n| n.starts_with("      "));
        if !described {
            bare.push(*line);
        }
    }
    assert!(entries > 250, "the index shrank: {entries} entries");
    assert!(
        bare.is_empty(),
        "{} entries carry a signature and nothing else: {bare:#?}",
        bare.len()
    );
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
/// total in the summary, and a file that checked nothing reported as
/// a skip rather than a pass — it ran, but it stands behind none of
/// the suite, which is what `--fail-fast`'s skips mean too.
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
    let nothing = stdout
        .lines()
        .find(|l| l.contains("none.ting"))
        .expect("a line for the file that checked nothing");
    assert!(nothing.starts_with("skip "), "{stdout}");
    assert!(nothing.ends_with("none.ting (no checks)"), "{stdout}");
    assert!(stdout.contains("one.ting (1 check)"), "{stdout}");
    assert!(stdout.contains("two.ting (2 checks)"), "{stdout}");
    assert!(
        stdout.contains("2 passed, 0 failed, 1 skipped, 3 checks"),
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
    // A TAP skip is a pass carrying a directive, so the stream stays
    // valid for a consumer that knows nothing about ting.
    assert!(stdout.contains("none.ting # SKIP no checks\n"), "{stdout}");

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

/// A child runs where it is told to. The directory is the option a
/// script visiting several checkouts needs, and the alternative was
/// `run("sh", ["-c", "cd ... && ..."])` — a shell string, on the
/// platforms that have a shell.
#[test]
fn run_takes_the_directory_to_run_the_child_in() {
    let exe = env!("CARGO_BIN_EXE_ting").replace('\\', "/");
    let name = format!("ting-io-run-dir-{}", std::process::id());
    let dir = std::env::temp_dir().join(&name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let here = dir.to_str().unwrap().replace('\\', "/");
    // The child prints where it stands; the parent asks for the same
    // directory both with and without something on the child's stdin,
    // since those are two different spawns.
    let child = dir.join("where.ting");
    std::fs::write(&child, "print(cwd());\nprint(input());\n").unwrap();
    let child_path = child.to_str().unwrap().replace('\\', "/");
    let script = dir.join("parent.ting");
    std::fs::write(
        &script,
        format!(
            "let plain = run(\"{exe}\", [\"{child_path}\"], {{\"dir\": \"{here}\"}});\n\
             let fed = run(\"{exe}\", [\"{child_path}\"], {{\"dir\": \"{here}\", \"stdin\": \"fed\\n\"}});\n\
             print(ends_with(split(plain[\"out\"], \"\\n\")[0], \"{name}\"), plain[\"code\"]);\n\
             print(ends_with(split(trim(fed[\"out\"]), \"\\n\")[0], \"{name}\"), trim(split(fed[\"out\"], \"\\n\")[1]));\n\
             let elsewhere = run(\"{exe}\", [\"{child_path}\"]);\n\
             print(ends_with(split(elsewhere[\"out\"], \"\\n\")[0], \"{name}\"));\n"
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "true 0", "{text}");
    assert_eq!(lines.next().unwrap(), "true fed", "{text}");
    // Without the option the child inherits this process's directory,
    // which is the crate root rather than the temp directory.
    assert_eq!(lines.next().unwrap(), "false", "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A child is given variables, keeps the ones it inherits, and can
/// be told to do without one. Both spawn paths again: the plain one
/// and the one with something on the child's stdin.
#[test]
fn run_gives_the_child_the_environment_it_is_told_to() {
    let exe = env!("CARGO_BIN_EXE_ting").replace('\\', "/");
    let dir = std::env::temp_dir().join(format!("ting-io-run-env-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let child = dir.join("say.ting");
    std::fs::write(
        &child,
        "print(env(\"TING_PROBE\"), env(\"TING_INHERITED\"), env(\"TING_DROPPED\"));\n",
    )
    .unwrap();
    let child_path = child.to_str().unwrap().replace('\\', "/");
    let script = dir.join("parent.ting");
    std::fs::write(
        &script,
        format!(
            "let opts = {{\"env\": {{\"TING_PROBE\": \"here\", \"TING_DROPPED\": nil}}}};\n\
             print(trim(run(\"{exe}\", [\"{child_path}\"], opts)[\"out\"]));\n\
             opts[\"stdin\"] = \"\";\n\
             print(trim(run(\"{exe}\", [\"{child_path}\"], opts)[\"out\"]));\n\
             print(trim(run(\"{exe}\", [\"{child_path}\"])[\"out\"]));\n"
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .env("TING_INHERITED", "kept")
        .env("TING_DROPPED", "gone")
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    // Given, inherited, and removed — in that order, twice.
    assert_eq!(lines.next().unwrap(), "here kept nil", "{text}");
    assert_eq!(lines.next().unwrap(), "here kept nil", "{text}");
    // Without the option the child has this process's environment.
    assert_eq!(lines.next().unwrap(), "nil kept gone", "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A shown child writes where the parent writes: its output lands in
/// the parent's own streams, in order, rather than coming back as
/// text at the end. The child is the binary under test, so there is
/// no program to assume.
#[test]
fn run_lets_a_child_write_to_the_streams_ting_is_writing_to() {
    let exe = env!("CARGO_BIN_EXE_ting").replace('\\', "/");
    let dir = std::env::temp_dir().join(format!("ting-io-run-show-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let child = dir.join("noisy.ting");
    std::fs::write(
        &child,
        "print(\"from the child\");\neprint(\"and its stderr\");\nexit(3);\n",
    )
    .unwrap();
    let child_path = child.to_str().unwrap().replace('\\', "/");
    let script = dir.join("parent.ting");
    std::fs::write(
        &script,
        format!(
            "print(\"before\");\n\
             let shown = run(\"{exe}\", [\"{child_path}\"], {{\"show\": true}});\n\
             print(\"after\", shown[\"code\"], has(shown, \"out\"), has(shown, \"err\"));\n\
             let caught = run(\"{exe}\", [\"{child_path}\"]);\n\
             print(trim(caught[\"out\"]), has(caught, \"err\"));\n"
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    // The child's stdout is in the parent's, between what the parent
    // printed before and after it.
    assert_eq!(lines.next().unwrap(), "before", "{text}");
    assert_eq!(lines.next().unwrap(), "from the child", "{text}");
    assert_eq!(lines.next().unwrap(), "after 3 false false", "{text}");
    // The same child, captured: the output comes back instead.
    assert_eq!(lines.next().unwrap(), "from the child true", "{text}");
    // Its stderr went to the parent's, and only once — the captured
    // run kept the second one to itself.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        stderr.matches("and its stderr").count(),
        1,
        "stderr: {stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Bytes that are not text, both ways round: a file ting is asked to
/// read as text FAILS, in ting's words rather than std's; a child's
/// output is decoded lossily, because there is no bytes type to hand
/// back and failing would throw away the code, the stderr and the
/// signal with it. The rule is written down in docs/reference.md;
/// this is what holds it.
#[test]
fn bytes_that_are_not_text_fail_on_the_way_in_and_are_replaced_on_the_way_out() {
    let exe = env!("CARGO_BIN_EXE_ting").replace('\\', "/");
    let dir = std::env::temp_dir();
    let bad = dir.join("ting-io-bad-bytes.bin");
    std::fs::write(&bad, [0xff, 0xfe, b'h', b'i', b'\n']).unwrap();
    let bad_path = bad.to_str().unwrap().replace('\\', "/");
    let script = dir.join("ting-io-bad-bytes.ting");
    std::fs::write(
        &script,
        format!(
            "print(try(fn() {{ return read_file(\"{bad_path}\"); }})[\"err\"]);\n\
             print(try(fn() {{ return each_line(\"{bad_path}\", fn(l) {{ return nil; }}); }})[\"err\"]);\n\
             let d = run(\"{exe}\", [\"--fmt\", \"-\"], \"print( 1 );\\n\");\n\
             print(d[\"out\"] == \"print(1);\\n\", d[\"code\"]);\n"
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    for whose in ["read_file", "each_line"] {
        let line = lines.next().unwrap();
        assert!(
            line.contains("not UTF-8 text: byte 0xff at "),
            "{whose} must say it in ting's words, and where:\n{text}"
        );
        assert!(
            !line.contains("stream"),
            "std's phrasing calls a named file a stream:\n{text}"
        );
    }
    assert_eq!(lines.next().unwrap(), "true 0", "unexpected:\n{text}");

    let _ = std::fs::remove_file(&bad);
    let _ = std::fs::remove_file(&script);
}

/// The other half of the rule, where a child prints bytes that are
/// not text: they come back REPLACED and the exit code survives.
/// Unix only, because it needs a program that will print arbitrary
/// bytes and `cat` is the one POSIX guarantees — no ting program can
/// stand in, since a ting string is UTF-8 by construction.
#[cfg(unix)]
#[test]
fn a_childs_bytes_are_replaced_rather_than_refused() {
    let dir = std::env::temp_dir();
    let bad = dir.join("ting-io-bad-out.bin");
    std::fs::write(&bad, [0xff, 0xfe, b'h', b'i']).unwrap();
    let bad_path = bad.to_str().unwrap();
    let script = dir.join("ting-io-bad-out.ting");
    std::fs::write(
        &script,
        format!(
            "let d = run(\"cat\", [\"{bad_path}\"]);\n\
             print(d[\"out\"] == chr(65533) + chr(65533) + \"hi\", d[\"code\"], len(d[\"out\"]));\n"
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(text.trim(), "true 0 4", "unexpected:\n{text}");
    let _ = std::fs::remove_file(&bad);
    let _ = std::fs::remove_file(&script);
}

/// A child with something to read, and the deadlock that shape
/// invites. The child echoes what it is given, so its stdout fills
/// while its stdin is still being written; writing the input on the
/// calling thread hangs here forever, which is why it does not.
/// Bounded rather than trusted: a deadlock must fail this test, not
/// wedge the suite.
#[test]
fn a_child_reads_what_it_is_given_without_deadlocking() {
    let exe = env!("CARGO_BIN_EXE_ting");
    let echo = std::env::temp_dir().join("ting-io-echo.ting");
    let script = std::env::temp_dir().join("ting-io-feed.ting");
    std::fs::write(
        &echo,
        "each_line(\"-\", fn(l) { print(l); return nil; });\n",
    )
    .unwrap();
    let echo_path = echo.to_str().unwrap().replace('\\', "/");
    let exe_path = exe.replace('\\', "/");
    std::fs::write(
        &script,
        format!(
            "let s = import(\"lib/string.ting\");\n\
             let big = s[\"repeat\"](s[\"repeat\"](\"x\", 99) + \"\\n\", 20000);\n\
             let d = run(\"{exe_path}\", [\"{echo_path}\"], big);\n\
             print(len(d[\"out\"]), d[\"code\"]);\n\
             print(run(\"{exe_path}\", [\"{echo_path}\"], \"a\\nb\\n\")[\"out\"] == \"a\\nb\\n\");\n\
             print(run(\"{exe_path}\", [\"{echo_path}\"])[\"out\"] == \"\");\n\
             print(run(\"{exe_path}\", [\"{echo_path}\"], nil)[\"out\"] == \"\");\n\
             print(try(fn() {{ return run(\"{exe_path}\", [], 5); }})[\"err\"]);\n"
        ),
    )
    .unwrap();

    let mut child = Command::new(exe)
        .arg(&script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(_) => break,
            None => {
                if std::time::Instant::now() > deadline {
                    let _ = child.kill();
                    panic!("run with input deadlocked: the write must not be on this thread");
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
    let out = child.wait_with_output().expect("wait_with_output");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "2000000 0", "unexpected:\n{text}");
    assert_eq!(lines.next().unwrap(), "true", "short input:\n{text}");
    assert_eq!(lines.next().unwrap(), "true", "no input is EOF:\n{text}");
    assert_eq!(lines.next().unwrap(), "true", "nil input is EOF:\n{text}");
    assert_eq!(
        lines.next().unwrap(),
        "run expects stdin as a string or options as a map, got int",
        "unexpected:\n{text}"
    );
    let _ = std::fs::remove_file(&script);
    let _ = std::fs::remove_file(&echo);
}

/// A child killed by a signal: no exit code, and the number that
/// ended it. Unix only, because `signal` is nil where the platform
/// has no signals — which is what the selftest checks portably.
#[cfg(unix)]
#[test]
fn a_signalled_child_has_no_code_and_names_the_signal() {
    let script = std::env::temp_dir().join("ting-io-signal.ting");
    std::fs::write(
        &script,
        "let d = run(\"sh\", [\"-c\", \"kill -9 $$\"]);\n\
         print(d[\"code\"], d[\"signal\"]);\n\
         let sh = import(\"lib/sh.ting\");\n\
         print(try(fn() { return sh[\"check\"](\"sh\", [\"-c\", \"kill -9 $$\"]); })[\"err\"]);\n\
         let fine = run(\"sh\", [\"-c\", \"exit 5\"]);\n\
         print(fine[\"code\"], fine[\"signal\"]);\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "nil 9", "unexpected:\n{text}");
    let msg = lines.next().unwrap();
    assert!(
        msg.contains("was killed by signal 9"),
        "check must say what killed it, not \"exited nil\":\n{text}"
    );
    assert!(
        !msg.contains("nil"),
        "no nil status reaches a message:\n{text}"
    );
    assert_eq!(lines.next().unwrap(), "5 nil", "unexpected:\n{text}");
    let _ = std::fs::remove_file(&script);
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

/// A directory of ting files, named apart from any other test's.
fn tree(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ting-bundle-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, src) in files {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, src).unwrap();
    }
    dir
}

fn ting(args: &[&std::path::Path]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(args)
        .output()
        .expect("failed to run ting")
}

/// `--bundle` writes one file that prints exactly what the several
/// files printed: local modules inlined, in the order their imports
/// ask for, with the standard library left where it is.
#[test]
fn bundle_prints_what_the_separate_files_printed() {
    let dir = tree(
        "same",
        &[
            (
                "app.ting",
                "let greeter = import(\"greeter.ting\");\n\
                 let again = import(\"./greeter.ting\");\n\
                 print(greeter[\"greet\"](\"world\"));\n\
                 print(again[\"greet\"](\"ting\"));\n\
                 print(greeter == again);\n",
            ),
            (
                "greeter.ting",
                "let s = import(\"lib/string.ting\");\n\
                 let shout = import(\"sub/shout.ting\");\n\
                 fn greet(name) {\n\
                 \x20 return shout[\"loud\"](s[\"title\"](name));\n\
                 }\n",
            ),
            ("sub/shout.ting", "fn loud(t) {\n  return t + \"!\";\n}\n"),
        ],
    );
    let app = dir.join("app.ting");
    let before = ting(&[&app]);
    assert!(before.status.success(), "{:?}", before);

    let bundled = ting(&[std::path::Path::new("--bundle"), &app]);
    assert!(bundled.status.success(), "{:?}", bundled);
    let one = dir.join("one.ting");
    std::fs::write(&one, &bundled.stdout).unwrap();
    let after = ting(&[&one]);
    assert_eq!(
        String::from_utf8_lossy(&after.stdout),
        String::from_utf8_lossy(&before.stdout)
    );

    let text = String::from_utf8_lossy(&bundled.stdout);
    // The two local imports are gone and the stdlib one stayed: that
    // is the whole trade, one file instead of three because the
    // twelfth module is already in the binary.
    assert!(!text.contains("import(\"greeter.ting\")"), "{text}");
    assert!(!text.contains("import(\"sub/shout.ting\")"), "{text}");
    assert!(text.contains("import(\"lib/string.ting\")"), "{text}");
    // Inlined once, not per import site: a module holding state must
    // stay one module, which is what importing a file twice gives.
    assert_eq!(text.matches("fn loud(t)").count(), 1, "{text}");
    assert_eq!(text.matches("fn __ting_module_").count(), 2, "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two modules importing a third: the third is inlined once and both
/// see the same map, exactly as two imports of one file do.
#[test]
fn bundle_shares_a_module_two_modules_import() {
    let dir = tree(
        "diamond",
        &[
            (
                "app.ting",
                "let a = import(\"a.ting\");\n\
                 let b = import(\"b.ting\");\n\
                 print(a[\"c\"] == b[\"c\"], a[\"n\"]() + b[\"n\"]());\n",
            ),
            (
                "a.ting",
                "let c = import(\"c.ting\");\nfn n() { return c[\"v\"]; }\n",
            ),
            (
                "b.ting",
                "let c = import(\"c.ting\");\nfn n() { return c[\"v\"] * 2; }\n",
            ),
            ("c.ting", "let v = 5;\n"),
        ],
    );
    let app = dir.join("app.ting");
    let before = ting(&[&app]);
    let bundled = ting(&[std::path::Path::new("--bundle"), &app]);
    assert!(bundled.status.success(), "{:?}", bundled);
    let one = dir.join("one.ting");
    std::fs::write(&one, &bundled.stdout).unwrap();
    let after = ting(&[&one]);
    assert_eq!(String::from_utf8_lossy(&before.stdout), "true 15\n");
    assert_eq!(
        String::from_utf8_lossy(&after.stdout),
        String::from_utf8_lossy(&before.stdout)
    );
    let text = String::from_utf8_lossy(&bundled.stdout);
    assert_eq!(text.matches("let v = 5;").count(), 1, "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// What `--bundle` will not do: follow a path it cannot read until the
/// program runs, or unroll a cycle. Both say so where the import is.
#[test]
fn bundle_refuses_a_cycle_and_a_computed_path() {
    let dir = tree(
        "refuse",
        &[
            ("a.ting", "let b = import(\"b.ting\");\nprint(b);\n"),
            ("b.ting", "let a = import(\"a.ting\");\nlet x = 1;\n"),
            ("dyn.ting", "let p = \"b.ting\";\nlet m = import(p);\n"),
        ],
    );
    let cycle = ting(&[std::path::Path::new("--bundle"), &dir.join("a.ting")]);
    assert_eq!(cycle.status.code(), Some(1));
    let err = String::from_utf8_lossy(&cycle.stderr);
    assert!(
        err.starts_with("b.ting:1:9: error: cannot bundle: circular import of \"a.ting\""),
        "{err}"
    );
    let computed = ting(&[std::path::Path::new("--bundle"), &dir.join("dyn.ting")]);
    assert_eq!(computed.status.code(), Some(1));
    let err = String::from_utf8_lossy(&computed.stderr);
    assert!(
        err.starts_with("dyn.ting:2:9: error: cannot bundle: this import's path is not a literal"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// An import naming nothing at all was copied into the bundle and the
/// bundler exited 0, so the mistake arrived on somebody else's
/// machine as a run-time error. It is found where the other two are.
/// What must keep working beside it: a name the binary answers, spelt
/// plainly or with a `./` in front.
#[test]
fn bundle_refuses_an_import_that_names_nothing() {
    let dir = tree(
        "nothing",
        &[
            ("miss.ting", "let m = import(\"nope.ting\");\n"),
            ("under.ting", "let m = import(\"lib/nope.ting\");\n"),
            (
                "embedded.ting",
                "let a = import(\"lib/list.ting\");\nlet b = import(\"./lib/math.ting\");\nprint(a[\"sum\"]([1, 2]), b[\"gcd\"](8, 12));\n",
            ),
        ],
    );
    for (file, under) in [("miss.ting", false), ("under.ting", true)] {
        let out = ting(&[std::path::Path::new("--bundle"), &dir.join(file)]);
        assert_eq!(out.status.code(), Some(1), "{file}");
        assert!(out.stdout.is_empty(), "{file} wrote a bundle anyway");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.starts_with(&format!("{file}:1:9: error: cannot bundle: no file at ")),
            "{err}"
        );
        // The path in the message is the one the PLATFORM resolved,
        // printed with {:?} — so on Windows every separator in it is
        // a backslash and every backslash is written twice. The last
        // component carries none, and a directory is asked about with
        // the separators folded rather than matched.
        assert!(
            err.contains("nope.ting\", and no embedded module of that name"),
            "{err}"
        );
        assert_eq!(
            err.replace('\\', "/").contains("/lib/"),
            under,
            "the path under lib/ is the one that resolves there: {err}"
        );
    }
    let embedded = ting(&[std::path::Path::new("--bundle"), &dir.join("embedded.ting")]);
    assert_eq!(embedded.status.code(), Some(0));
    let one = dir.join("one.ting");
    std::fs::write(&one, &embedded.stdout).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&ting(&[&one]).stdout),
        "3 4\n",
        "an embedded import has to survive bundling"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A script somebody made executable stays executable once bundled:
/// the `#!` line means nothing anywhere but the first, and the
/// bundle's own header used to take that place. A module's `#!` is
/// left where it is — inside the function its body becomes, where it
/// is a comment and nothing else — and so is a `#!` that was never
/// first.
#[test]
fn bundle_keeps_the_shebang_first() {
    let dir = tree(
        "shebang",
        &[
            (
                "util/text.ting",
                "#!/usr/bin/env ting\nfn shout(s) { return upper(s) + \"!\"; }\n",
            ),
            (
                "cli.ting",
                "#!/usr/bin/env ting\nlet t = import(\"util/text.ting\");\nprint(t[\"shout\"](\"hi\"));\n",
            ),
            (
                "plain.ting",
                "let x = 1;\n#!not a shebang here\nprint(x);\n",
            ),
        ],
    );
    let out = ting(&[std::path::Path::new("--bundle"), &dir.join("cli.ting")]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("#!/usr/bin/env ting"));
    assert_eq!(
        lines.next(),
        Some("# cli.ting, bundled by `ting --bundle`.")
    );
    assert_eq!(
        text.matches("#!/usr/bin/env ting").count(),
        2,
        "the module keeps its own, where it is a comment: {text}"
    );
    let one = dir.join("one.ting");
    std::fs::write(&one, &out.stdout).unwrap();
    assert_eq!(String::from_utf8_lossy(&ting(&[&one]).stdout), "HI!\n");

    // Only the first line is a shebang. One further down is a
    // comment the bundler must not move.
    let plain = ting(&[std::path::Path::new("--bundle"), &dir.join("plain.ting")]);
    let text = String::from_utf8_lossy(&plain.stdout);
    assert!(
        text.starts_with("# plain.ting, bundled by"),
        "a bundle of a script without one starts with the header: {text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--bundle` takes one file, and only a file: a script's imports
/// resolve against its own directory, which stdin does not have.
#[test]
fn bundle_takes_exactly_one_file() {
    for args in [
        vec!["--bundle"],
        vec!["--bundle", "-"],
        vec!["--bundle", "x.ting", "-o"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(&args)
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(
            said.starts_with("ting: ") && said.contains("--help"),
            "{said}"
        );
    }
}

/// Every `import("...")` in `src` whose path names a file next to it:
/// the imports a bundle has to inline, told apart from the standard
/// library exactly as the interpreter tells them apart — filesystem
/// first, and what has no file is embedded in the binary.
fn has_a_local_import(src: &str, dir: &std::path::Path) -> bool {
    src.match_indices("import(\"").any(|(at, marker)| {
        let rest = &src[at + marker.len()..];
        match rest.find('"') {
            Some(end) => dir.join(&rest[..end]).is_file(),
            None => false,
        }
    })
}

/// The promise `--bundle` has to keep, over every program in the
/// corpus that imports a local module: the bundle prints exactly the
/// bytes the separate files printed and exits the same way, and it is
/// itself ting the toolchain accepts — `--check` clean and already in
/// the format `--fmt` would produce. A bundler that emitted code the
/// formatter would rewrite would not be emitting ting.
///
/// Here that means the standard library too: `selftest/` and
/// `examples/` reach it as `../lib/...`, which is a file in this
/// repository, so those bundles inline the real modules and run them.
/// The one thing a bundle cannot keep identical is a program that
/// prints where its own code sits — `try()` hands back the file and
/// line, and in a bundle that is the bundle. Nothing in the corpus
/// does, and the guard would say so if something started to.
#[test]
fn bundling_never_changes_what_a_program_prints() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = std::env::temp_dir().join(format!("ting-bundle-corpus-{}", std::process::id()));
    std::fs::create_dir_all(&out_dir).unwrap();
    let mut checked = 0;
    for corner in ["selftest", "examples"] {
        let dir = root.join(corner);
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .expect("corpus directory missing")
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("ting"))
            .collect();
        paths.sort();
        for path in paths {
            let src = std::fs::read_to_string(&path).unwrap();
            if !has_a_local_import(&src, &dir) {
                continue;
            }
            let before = ting(&[&path]);
            let bundled = ting(&[std::path::Path::new("--bundle"), &path]);
            assert!(
                bundled.status.success(),
                "--bundle failed on {}:\n{}",
                path.display(),
                String::from_utf8_lossy(&bundled.stderr)
            );
            let one = out_dir.join(path.file_name().unwrap());
            std::fs::write(&one, &bundled.stdout).unwrap();
            let after = ting(&[&one]);
            assert_eq!(
                String::from_utf8_lossy(&after.stdout),
                String::from_utf8_lossy(&before.stdout),
                "bundling changed what {} prints",
                path.display()
            );
            assert_eq!(
                after.status.code(),
                before.status.code(),
                "bundling changed how {} exits:\n{}",
                path.display(),
                String::from_utf8_lossy(&after.stderr)
            );
            let checked_bundle = ting(&[std::path::Path::new("--check"), &one]);
            assert!(
                checked_bundle.status.success(),
                "the bundle of {} does not check:\n{}",
                path.display(),
                String::from_utf8_lossy(&checked_bundle.stderr)
            );
            let formatted = ting(&[std::path::Path::new("--fmt-check"), &one]);
            assert!(
                formatted.status.success(),
                "the bundle of {} is not formatted as ting",
                path.display()
            );
            checked += 1;
        }
    }
    // Fourteen today. A scan that silently matched nothing would pass
    // every assertion above.
    assert!(checked >= 12, "only {checked} programs had a local import");
    let _ = std::fs::remove_dir_all(&out_dir);
}

/// An `import` does not have to sit at a module's top level, and a
/// module's top level can do more than define things. A bundle that
/// ran every module at the top of the file would print what a module
/// prints whether or not the program ever asked for it, so a bundled
/// module runs on the first ask and hands back the same map after —
/// which is what `import` itself does.
#[test]
fn a_bundled_module_runs_only_when_it_is_asked_for() {
    let dir = tree(
        "lazy",
        &[
            (
                "app.ting",
                "print(\"start\");\n\
                 if false {\n\
                 \x20 let m = import(\"noisy.ting\");\n\
                 \x20 print(m[\"answer\"]);\n\
                 }\n\
                 let first = import(\"noisy.ting\");\n\
                 let second = import(\"noisy.ting\");\n\
                 print(first[\"answer\"], first == second);\n",
            ),
            ("noisy.ting", "print(\"loaded\");\nlet answer = 42;\n"),
        ],
    );
    let app = dir.join("app.ting");
    let before = ting(&[&app]);
    let bundled = ting(&[std::path::Path::new("--bundle"), &app]);
    assert!(bundled.status.success(), "{:?}", bundled);
    let one = dir.join("one.ting");
    std::fs::write(&one, &bundled.stdout).unwrap();
    let after = ting(&[&one]);
    // The branch is never taken, so the module runs once, when the
    // first real import asks for it — after "start" and not before.
    assert_eq!(
        String::from_utf8_lossy(&before.stdout),
        "start\nloaded\n42 true\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&after.stdout),
        String::from_utf8_lossy(&before.stdout)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `-o` writes the bundle to a file and says nothing on stdout, and
/// it refuses to write over a file that went into the bundle —
/// however that file is spelled. This is not a hypothetical: a shell
/// redirection onto an input truncates it before ting is started, so
/// `ting --bundle main.ting > main.ting` leaves a bundle of an empty
/// program where the script was. `-o` is the way to write a bundle
/// without losing what it was made from.
#[test]
fn bundle_writes_where_o_says_and_never_over_its_own_source() {
    let dir = tree(
        "out",
        &[
            (
                "app.ting",
                "let g = import(\"greeter.ting\");\nprint(g[\"greet\"](\"ting\"));\n",
            ),
            ("greeter.ting", "fn greet(n) {\n  return \"hi, \" + n;\n}\n"),
        ],
    );
    let app = dir.join("app.ting");
    let one = dir.join("one.ting");
    let written = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("--bundle")
        .arg(&app)
        .arg("-o")
        .arg(&one)
        .output()
        .expect("failed to run ting");
    assert!(written.status.success(), "{written:?}");
    assert_eq!(String::from_utf8_lossy(&written.stdout), "");
    let ran = ting(&[&one]);
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "hi, ting\n");

    // Both the entry and a module, and a spelling that only resolves
    // once the path is followed.
    let greeter = dir.join("greeter.ting");
    let before = std::fs::read_to_string(&greeter).unwrap();
    for target in [
        app.clone(),
        greeter.clone(),
        dir.join("sub").join("..").join("greeter.ting"),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg("--bundle")
            .arg(&app)
            .arg("-o")
            .arg(&target)
            .output()
            .expect("failed to run ting");
        assert_eq!(out.status.code(), Some(2), "{}", target.display());
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("which went into the bundle"),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert_eq!(std::fs::read_to_string(&greeter).unwrap(), before);
    assert!(std::fs::read_to_string(&app).unwrap().contains("import("));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The claim rename is for: the file keeps its modification time,
/// because nothing was copied. Both files are backdated to a fixed
/// instant first, so "the stamp survived" cannot be a coincidence of
/// two operations landing in the same millisecond.
#[test]
fn rename_keeps_the_date_that_copying_loses() {
    let dir = tree(
        "rename-date",
        &[("old.txt", "hello"), ("copy-me.txt", "hello")],
    );
    let then = std::time::UNIX_EPOCH + std::time::Duration::from_millis(1_000_000_000_000);
    for name in ["old.txt", "copy-me.txt"] {
        std::fs::File::options()
            .write(true)
            .open(dir.join(name))
            .unwrap()
            .set_modified(then)
            .unwrap();
    }
    std::fs::write(
        dir.join("run.ting"),
        "rename(\"old.txt\", \"new.txt\");\n\
         write_file(\"copied.txt\", read_file(\"copy-me.txt\"));\n\
         print(stat(\"new.txt\")[\"modified\"]);\n\
         print(stat(\"copied.txt\")[\"modified\"]);\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("run.ting")
        .current_dir(&dir)
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    assert_eq!(
        lines.next(),
        Some("1000000000000"),
        "renaming kept the stamp: {text}"
    );
    let copied: i64 = lines.next().unwrap().parse().unwrap();
    assert!(
        copied > 1_000_000_000_000,
        "copying stamped the copy now, not then: {copied}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// What rename refuses, in words a script author can act on: the
/// system call's own "Invalid cross-device link" names nothing a
/// caller can do. Needs a second filesystem to be mounted, so it
/// reports and passes where there is none rather than pretending.
#[cfg(unix)]
#[test]
fn rename_says_when_the_two_paths_are_on_different_filesystems() {
    use std::os::unix::fs::MetadataExt;
    let dir = tree("rename-across", &[("a.txt", "hello")]);
    let here = std::fs::metadata(&dir).unwrap().dev();
    let elsewhere = ["/dev/shm", "/tmp", "/var/tmp", "/run/user/1000"]
        .into_iter()
        .map(std::path::PathBuf::from)
        .find(|p| {
            std::fs::metadata(p)
                .map(|m| m.dev() != here && m.is_dir())
                .unwrap_or(false)
        });
    let Some(elsewhere) = elsewhere else {
        eprintln!("only one filesystem here; nothing to cross");
        return;
    };
    let target = elsewhere.join(format!("ting-across-{}.txt", std::process::id()));
    std::fs::write(
        dir.join("run.ting"),
        format!("rename(\"a.txt\", {:?});\n", target.display().to_string()),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("run.ting")
        .current_dir(&dir)
        .output()
        .expect("failed to run ting");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("they are on different filesystems"),
        "got: {err}"
    );
    assert!(
        dir.join("a.txt").exists(),
        "and the file stayed where it was"
    );
    let _ = std::fs::remove_file(&target);
    let _ = std::fs::remove_dir_all(&dir);
}

/// copy_file is for the bytes rename cannot move and read_file cannot
/// read. Both claims at once: the source is not valid UTF-8, and its
/// modification time is backdated to a fixed instant so that "the date
/// followed" cannot be two operations in the same millisecond.
#[test]
fn copy_file_carries_any_bytes_and_the_date() {
    let dir = tree("copy-bytes", &[]);
    std::fs::create_dir_all(&dir).unwrap();
    let bytes: &[u8] = &[0xff, 0xfe, 0x00, 0x01, b'h', b'i'];
    std::fs::write(dir.join("photo.bin"), bytes).unwrap();
    let then = std::time::UNIX_EPOCH + std::time::Duration::from_millis(1_000_000_000_000);
    std::fs::File::options()
        .write(true)
        .open(dir.join("photo.bin"))
        .unwrap()
        .set_modified(then)
        .unwrap();
    std::fs::write(
        dir.join("run.ting"),
        "copy_file(\"photo.bin\", \"copy.bin\");\n\
         print(stat(\"copy.bin\")[\"modified\"], stat(\"copy.bin\")[\"size\"]);\n\
         print(try(read_file, \"copy.bin\")[\"err\"] != nil);\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("run.ting")
        .current_dir(&dir)
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        text,
        "1000000000000 6\ntrue\n",
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read(dir.join("copy.bin")).unwrap(),
        bytes,
        "the copy is the same bytes"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// std::fs::copy from a file to itself opens the target for writing,
/// truncating the source it is about to read, and calls that a
/// successful copy of nothing. A hard link is the spelling of "the
/// same file" that comparing paths cannot catch.
#[cfg(unix)]
#[test]
fn copy_file_refuses_the_file_it_would_empty() {
    let dir = tree("copy-self", &[("original.txt", "still here")]);
    std::fs::hard_link(dir.join("original.txt"), dir.join("alias.txt")).unwrap();
    std::fs::write(
        dir.join("run.ting"),
        "print(try(copy_file, \"original.txt\", \"alias.txt\")[\"err\"]);\n\
         print(read_file(\"original.txt\"));\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("run.ting")
        .current_dir(&dir)
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        text,
        "cannot copy \"original.txt\" to \"alias.txt\": they are the same file\nstill here\n",
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// each_line's "-" is read_file's "-": stdin, so a script can take a
/// path or a pipe without caring which. It shares the buffer input()
/// reads from, so the two compose rather than losing a line between
/// them — which a separate reader would.
#[test]
fn each_line_reads_stdin_where_input_left_off() {
    let dir = tree(
        "each-line-stdin",
        &[(
            "run.ting",
            "print(\"first \" + input());\n\
             print(each_line(\"-\", fn(l) { print(\"then \" + l); return nil; }));\n",
        )],
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("run.ting")
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"a\nb\nc\n")
            .unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "first a\nthen b\nthen c\n2\n",
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A program nested past what the parser will follow is TOLD so.
/// Before 848 it died of a host stack overflow: killed by a signal,
/// no line, no message, nothing a caller could catch — the same
/// corpse a driving script gets from a real crash. Every route into
/// the parser is checked, because they are what a user actually
/// runs.
#[test]
fn a_program_nested_too_deeply_is_told_so_rather_than_killed() {
    let script = std::env::temp_dir().join("ting-nested-too-deeply.ting");
    let deep = format!("{}let x = 1;{}\n", "{".repeat(20000), "}".repeat(20000));
    std::fs::write(&script, deep).expect("write deep script");
    let path = script.to_str().expect("path is text");
    for (engine, args) in [
        ("vm", vec![path]),
        ("eval", vec![path]),
        ("vm", vec!["--check", path]),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(&args)
            .env("TING_ENGINE", engine)
            .output()
            .expect("failed to run ting");
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(1),
            "{engine} {args:?} did not report a failure: {}",
            &err[..err.len().min(200)]
        );
        assert!(
            err.contains("nested too deeply (the limit is 200 levels)"),
            "{engine} {args:?} said: {}",
            &err[..err.len().min(200)]
        );
        assert!(
            err.contains(":1:"),
            "{engine} {args:?} named no line: {}",
            &err[..err.len().min(200)]
        );
    }
    let _ = std::fs::remove_file(&script);
}

/// The same program, on a main thread the size Windows promises: one
/// megabyte. 848's limit was not enough by itself, because `--check`
/// parsed on the main thread, and an unoptimized parse at the limit
/// wants three and a half megabytes — so CI died on windows-latest
/// with exit 0xC00000FD, "has overflowed its stack", while this host
/// printed a clean error from its eight. `ulimit -s` builds the same
/// small main thread here, which is the only way a Linux host gets to
/// check what Windows will do.
#[cfg(unix)]
#[test]
fn a_deep_program_is_told_so_on_a_main_stack_the_size_windows_gives() {
    let script = std::env::temp_dir().join("ting-nested-small-stack.ting");
    let deep = format!("{}let x = 1;{}\n", "{".repeat(20000), "}".repeat(20000));
    std::fs::write(&script, deep).expect("write deep script");
    let bin = env!("CARGO_BIN_EXE_ting");
    let path = script.to_str().expect("path is text");
    // Both routes into the parser: the checker and the runner. The
    // formatter is not here because it never parses.
    for args in ["--check", ""] {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("ulimit -s 1024; exec '{bin}' {args} '{path}'"))
            .output()
            .expect("failed to run ting under a small stack");
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            out.status.code(),
            Some(1),
            "`{args}` on a 1 MB main stack: {}",
            &err[..err.len().min(200)]
        );
        assert!(
            err.contains("nested too deeply"),
            "`{args}` on a 1 MB main stack said: {}",
            &err[..err.len().min(200)]
        );
    }
    let _ = std::fs::remove_file(&script);
}

/// A program that finished is finished: freeing what it built must
/// not walk it. Before 859 both of these printed their answer and
/// then died on the way out — exit 134, after the output, with
/// nothing to catch — because `drop` descended one frame per level.
/// Two shapes, because two types nest: the values a program builds,
/// and the tree the program IS. A million levels either way; the
/// limit on source nesting is 200, but a chain of `+` is not nested
/// source, and neither is a list a loop grows one level at a time.
#[test]
fn a_deep_program_is_freed_without_walking_what_it_built() {
    let script = std::env::temp_dir().join("ting-deep-value.ting");
    for (what, src) in [
        ("list", "let x = []; while i < 1000000 { x = [x]; i += 1; }"),
        (
            "map",
            "let x = {}; while i < 1000000 { x = {\"a\": x}; i += 1; }",
        ),
    ] {
        std::fs::write(&script, format!("let i = 0; {src} print(len(x));\n")).expect("write");
        for engine in ["vm", "eval"] {
            let out = Command::new(env!("CARGO_BIN_EXE_ting"))
                .arg(&script)
                .env("TING_ENGINE", engine)
                .output()
                .expect("failed to run ting");
            assert_eq!(
                out.status.code(),
                Some(0),
                "{engine} on a deep {what}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&out.stdout),
                "1\n",
                "{engine} {what}"
            );
        }
    }
    let _ = std::fs::remove_file(&script);
}

#[test]
fn a_long_chain_is_freed_without_walking_the_tree_it_parsed_to() {
    let script = std::env::temp_dir().join("ting-deep-tree.ting");
    let terms = ["1"; 1000000].join("+");
    std::fs::write(&script, format!("print({terms});\n")).expect("write");
    let path = script.to_str().expect("path is text");
    for (engine, args) in [
        ("vm", vec![path]),
        ("eval", vec![path]),
        ("vm", vec!["--check", path]),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(&args)
            .env("TING_ENGINE", engine)
            .output()
            .expect("failed to run ting");
        assert_eq!(
            out.status.code(),
            Some(0),
            "{engine} {args:?} on a million-term chain: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let _ = std::fs::remove_file(&script);
}

/// A log is a million good lines and one byte from an older
/// encoding. "not UTF-8 text" alone leaves nothing to search for, so
/// every door into a file says WHERE the bytes stop being text: the
/// byte itself, its offset, and the line it falls in.
#[test]
fn a_byte_that_is_not_text_is_reported_where_it_is() {
    let dir = std::env::temp_dir().join(format!("ting-dirty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let log = dir.join("dirty.log");
    // "ok line\nca<0xe9> bad\nlast\n": the bad byte is the third of
    // the second line, at offset 10.
    std::fs::write(&log, b"ok line\nca\xe9 bad\nlast\n").expect("write fixture");

    let script = dir.join("read.ting");
    std::fs::write(&script, "print(len(read_file(args()[0])));\n").expect("write script");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .arg(&log)
        .output()
        .expect("failed to run ting");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("not UTF-8 text: byte 0xe9 at offset 10 (line 2, byte 3)"),
        "read_file: {err}"
    );

    // The line reader counts lines, so it says which one.
    let each = dir.join("each.ting");
    std::fs::write(&each, "each_line(args()[0], fn(l) { print(len(l)); });\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&each)
        .arg(&log)
        .output()
        .expect("failed to run ting");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("not UTF-8 text: byte 0xe9 at byte 3 of line 2"),
        "each_line: {err}"
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "7\n",
        "the good line before it was handed over"
    );

    // A script that is not text, through the three tools that read
    // one, all say the same thing.
    let source = dir.join("bad.ting");
    std::fs::write(&source, b"print(\"h\xe9llo\");\n").expect("write source");
    for flags in [vec![], vec!["--check"], vec!["--fmt"]] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(&flags)
            .arg(&source)
            .output()
            .expect("failed to run ting");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("not UTF-8 text: byte 0xe9 at offset 8 (line 1, byte 9)"),
            "{flags:?}: {err}"
        );
        assert_eq!(out.status.code(), Some(1), "{flags:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// input() reads a stream nobody has numbered, so it names the byte
/// within the line rather than inventing a line number — and the
/// lines before it were already handed over, which is why saying
/// where matters here most.
#[test]
fn a_stream_that_goes_bad_says_where_in_the_line() {
    let script = std::env::temp_dir().join(format!("ting-stream-{}.ting", std::process::id()));
    std::fs::write(
        &script,
        "let line = input();\nwhile line != nil {\n  print(len(line));\n  line = input();\n}\n",
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"ok\nca\xe9 bad\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let _ = std::fs::remove_file(&script);
    assert_eq!(String::from_utf8_lossy(&out.stdout), "2\n");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("input failed: not UTF-8 text: byte 0xe9 at byte 3 of the line"),
        "{err}"
    );
}

/// A module that is THERE but unreadable is not a missing module: the
/// embedded stdlib must not answer for a lib/ file the script can
/// see, or a corrupt copy would silently run something else.
#[test]
fn an_unreadable_module_does_not_fall_back_to_the_embedded_one() {
    let dir = std::env::temp_dir().join(format!("ting-shadow-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("lib")).expect("temp dir");
    std::fs::write(dir.join("lib/list.ting"), b"# \xe9\nlet x = 1;\n").expect("write module");
    std::fs::write(
        dir.join("use.ting"),
        "let l = import(\"lib/list.ting\");\nprint(len(keys(l)));\n",
    )
    .expect("write script");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("use.ting")
        .current_dir(&dir)
        .output()
        .expect("failed to run ting");
    let err = String::from_utf8_lossy(&out.stderr);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains("not UTF-8 text"), "{err}");
    assert!(
        !err.contains("no embedded module"),
        "a file that is there is not a missing file: {err}"
    );
}

/// Saying where the bytes go bad is half of it; the other half is
/// being able to read the file anyway. `"lossy"` asks for exactly
/// what a child's output has always got — a replacement character
/// per bad byte — so one word means one thing in both directions.
#[test]
fn a_reader_can_ask_for_the_text_anyway() {
    let dir = std::env::temp_dir().join(format!("ting-lossy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let log = dir.join("dirty.log");
    std::fs::write(&log, b"ok line\nca\xe9 bad\nlast\n").expect("write fixture");
    let script = dir.join("lossy.ting");
    std::fs::write(
        &script,
        "let whole = read_file(args()[0], \"lossy\");\n\
         print(len(whole), len(split(whole, \"\\n\")));\n\
         let widths = [];\n\
         print(each_line(args()[0], fn(l) { push(widths, len(l)); }, \"lossy\"), widths);\n\
         print(try(read_file, args()[0], \"skip\")[\"err\"]);\n",
    )
    .expect("write script");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .arg(&log)
        .output()
        .expect("failed to run ting");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    // 21 bytes of text, one replacement character, and the trailing
    // newline leaves a fourth (empty) piece.
    assert_eq!(lines.next().unwrap(), "21 4", "whole file:\n{text}");
    assert_eq!(
        lines.next().unwrap(),
        "3 [7, 7, 4]",
        "line by line:\n{text}"
    );
    assert_eq!(
        lines.next().unwrap(),
        "read_file mode must be the string \"lossy\", got \"skip\"",
        "a mode nobody has:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The same for the stream: a filter that would have dropped the one
/// bad line can now reach it.
#[test]
fn a_stream_can_be_read_lossily_too() {
    let script = std::env::temp_dir().join(format!("ting-lossy-in-{}.ting", std::process::id()));
    std::fs::write(
        &script,
        "let line = input(\"lossy\");\nwhile line != nil {\n  print(len(line));\n  line = input(\"lossy\");\n}\n",
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run ting");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"ok\nca\xe9 bad\nlast\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let _ = std::fs::remove_file(&script);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "2\n7\n4\n",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.status.success());
}

/// Every module opens with a comment saying what it is for — how to
/// import it, and the shape of the values its functions take, which
/// no per-function line has room for. `--doc lib/args.ting` used to
/// answer with five functions, two of which take a "spec", and
/// nothing anywhere said what a spec was: it is in that comment, ten
/// lines up in the file, and no tool printed it (908).
#[test]
fn every_module_says_what_it_is() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    for entry in std::fs::read_dir(root.join("lib")).expect("lib/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ting") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        let header: Vec<String> = src
            .lines()
            .take_while(|l| l.starts_with('#'))
            .map(|l| l[1..].trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        assert!(
            !header.is_empty(),
            "{} has no header comment",
            path.display()
        );
        // By path: `--doc args` is the builtin args(), which is its
        // own problem and the next stroke's.
        let short = format!("lib/{}", path.file_name().unwrap().to_str().unwrap());
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(["--doc", &short])
            .output()
            .expect("failed to run ting");
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in &header {
            assert!(
                stdout.contains(line.as_str()),
                "--doc {short} does not carry its header line {line:?}:\n{stdout}"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 13, "expected thirteen modules");
}

/// A file's leading comment is the FILE's only when a blank line
/// follows it. A comment sitting straight on top of the first
/// declaration documents that declaration, and printing it twice —
/// once as the file's, once as the function's — would be worse than
/// not printing it at all.
#[test]
fn a_comment_on_the_first_function_is_not_the_file_header() {
    let dir = std::env::temp_dir().join(format!("ting-doc-header-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let attached = dir.join("attached.ting");
    std::fs::write(&attached, "# Adds one.\nfn inc(n) { return n + 1; }\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", attached.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.matches("Adds one.").count(), 1, "{stdout}");
    assert!(stdout.contains("inc(n)  Adds one."), "{stdout}");

    let headed = dir.join("headed.ting");
    std::fs::write(
        &headed,
        "# A tiny ledger.\n#\n# An entry is {\"who\", \"amount\"}.\n\n# Adds one.\nfn inc(n) { return n + 1; }\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", headed.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\n  A tiny ledger.\n"), "{stdout}");
    assert!(stdout.contains("An entry is"), "{stdout}");
    assert_eq!(stdout.matches("Adds one.").count(), 1, "{stdout}");

    std::fs::remove_dir_all(&dir).unwrap();
}

/// Two module names are also the name of something that answers
/// first: `map` is the builtin map(xs, f) and `args` is args(). The
/// answer has to say the module is there, or lib/map.ting and
/// lib/args.ting are unreachable by the name a reader would try
/// (908) — and lib/args.ting is where the shape of a spec is written.
#[test]
fn a_builtin_that_shares_a_module_name_points_at_the_module() {
    for (name, path) in [("args", "lib/args.ting"), ("map", "lib/map.ting")] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(["--doc", name])
            .output()
            .expect("failed to run ting");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(0), "{stdout}");
        assert!(stdout.starts_with(&format!("{name}(")), "{stdout}");
        assert!(
            stdout.contains(&format!(
                "({path} is a module of the same name: --doc {path})"
            )),
            "--doc {name} does not mention {path}:\n{stdout}"
        );
    }
    // A name with no module of its own says nothing extra.
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--doc", "len"])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("is a module of the same name"), "{stdout}");
}

/// A builtin that hands back a shaped map has to NAME its keys where
/// `--doc` will show them, not just say "a map": 915 found `re_find`
/// described as "a map ... groups included" and `try` as {"ok"} or
/// {"err"}, when the real failure map also carries "at" and "trace".
/// The value is produced here and its own keys are what the doc is
/// checked against, so a key added later fails this rather than going
/// unmentioned.
#[test]
fn the_doc_for_a_shaped_value_names_every_key_it_has() {
    fn names(text: &str, word: &str) -> bool {
        text.match_indices(word).any(|(i, _)| {
            let before = text[..i].chars().next_back();
            let after = text[i + word.len()..].chars().next();
            let edge = |c: Option<char>| !c.is_some_and(|c| c.is_alphanumeric() || c == '_');
            edge(before) && edge(after)
        })
    }

    let dir = std::env::temp_dir().join(format!("ting-shapes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("f.txt");
    std::fs::write(&target, "hello").unwrap();
    let quoted = target.to_str().unwrap().replace('\\', "\\\\");

    for (builtin, program) in [
        (
            "re_find",
            "print(join(keys(re_find(\"ab\", \"(a)\")), \" \"));".to_string(),
        ),
        (
            "try",
            "print(join(keys(try(fn() { fail(\"x\"); })), \" \"));".to_string(),
        ),
        (
            "stat",
            format!("print(join(keys(stat(\"{quoted}\")), \" \"));"),
        ),
    ] {
        let script = dir.join(format!("{builtin}.ting"));
        std::fs::write(&script, &program).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg(&script)
            .output()
            .expect("failed to run ting");
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        assert_eq!(
            out.status.code(),
            Some(0),
            "{builtin}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let keys: Vec<&str> = stdout.trim().split(' ').collect();
        assert!(keys.len() >= 3, "{builtin} gave too few keys: {keys:?}");

        let doc = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(["--doc", builtin])
            .output()
            .expect("failed to run ting");
        let doc = String::from_utf8_lossy(&doc.stdout).to_string();
        let entry = doc.split("\nalso mentioned by").next().unwrap_or(&doc);
        for key in keys {
            assert!(
                names(entry, key),
                "--doc {builtin} does not name the key {key:?} the value carries:\n{entry}"
            );
        }
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The checker and the run are one sentence about a module member
/// that is not there, not two. The checker sees the lookup before the
/// program runs and the run sees it when it gets there, and a reader
/// who fixes what one of them said should not be told something else
/// by the other.
#[test]
fn a_missing_module_member_reads_the_same_before_and_during_a_run() {
    let dir = std::env::temp_dir().join("ting-member-sentence");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let script = dir.join("m.ting");
    // A name the module retired into a builtin, which is the case
    // where the sentence says something a map's keys cannot.
    std::fs::write(
        &script,
        "let st = import(\"lib/string.ting\");\nprint(st[\"ends_with\"](\"a\", \"b\"));\n",
    )
    .expect("write");

    let said = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(args)
            .arg(&script)
            .output()
            .expect("failed to run ting");
        let text = String::from_utf8_lossy(&out.stderr).to_string();
        let line = text
            .lines()
            .next()
            .unwrap_or_else(|| panic!("nothing said for {args:?}:\n{text}"))
            .to_string();
        // Past "warning: " or "error: ": the level and the position
        // differ by surface, the sentence is what must not.
        let i = line
            .find(": ")
            .and_then(|i| line[i + 2..].find(": "))
            .unwrap();
        line[line.find(": ").unwrap() + i + 4..].to_string()
    };

    let checked = said(&["--check"]);
    assert_eq!(
        checked,
        "lib/string.ting has no `ends_with` (`ends_with` is a builtin)"
    );
    assert_eq!(said(&[]), checked, "the run says something else");

    // A map the program built for itself is still a map: it has keys,
    // not members, and nothing names a file at it. The module stays
    // imported, so what tells the two apart is identity and not
    // whether this run imported anything.
    std::fs::write(
        &script,
        "let st = import(\"lib/string.ting\");\nlet m = {\"alpha\": 1};\nprint(m[\"beta\"]);\n",
    )
    .expect("write");
    assert_eq!(said(&[]), "key \"beta\" not found");
    let _ = std::fs::remove_dir_all(&dir);
}

/// One way to name a path in a message. The tools used to write it
/// bare — `cannot read nosuch` — where every runtime error quotes it,
/// and a path with a space in it then had no ends: the reader could
/// not tell the name from the sentence around it. Every surface that
/// names a path it could not read is here, because the one that is
/// left out is the one that goes back to writing it bare.
#[test]
fn every_tool_quotes_the_path_it_could_not_read() {
    let dir = std::env::temp_dir().join(format!("ting-quoted-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    // The name is the test: a path with a space in it is unreadable
    // in a message that does not delimit it.
    let missing = dir.join("no such file.ting");
    let name = missing.to_str().unwrap();

    for args in [
        vec![name],
        vec!["--check", name],
        vec!["--fmt", name],
        vec!["--fmt-check", name],
        vec!["--test", name],
        vec!["--coverage", name],
        vec!["--profile", name],
        vec!["--bundle", name],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(&args)
            .output()
            .expect("failed to run ting");
        let said = String::from_utf8_lossy(&out.stderr).to_string()
            + &String::from_utf8_lossy(&out.stdout);
        assert!(
            said.contains(&format!("cannot read {name:?}")),
            "{args:?} said: {said}"
        );
        assert!(!out.status.success(), "{args:?} left happy: {said}");
    }

    // And the message about a directory holding nothing to run names
    // the directory the same way, on each of the three tools that
    // can say it.
    for tool in ["--test", "--check", "--fmt"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args([tool, dir.to_str().unwrap()])
            .output()
            .expect("failed to run ting");
        let said = String::from_utf8_lossy(&out.stderr).to_string();
        assert!(
            said.contains(&format!(
                "no .ting files found under {:?}",
                dir.to_str().unwrap()
            )),
            "{tool} said: {said}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// An expression is echoed to be read. The REPL used to echo whatever
/// the value printed as — `range(100000)` is 688890 characters — and
/// what the reader was looking at went up the scrollback with it. The
/// cut belongs to the prompt alone: `print` still writes everything,
/// in a session as in a script.
#[test]
fn the_repl_echo_stops_and_says_how_much_there_was() {
    let echoed = |src: &str| -> String {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to run ting");
        use std::io::Write as _;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(src.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        String::from_utf8_lossy(&out.stdout).to_string()
    };

    // Small enough to read: exactly what it was, nothing added.
    assert_eq!(echoed("[1, 2, 3]\n"), "[1, 2, 3]\n");

    let big = echoed("range(100000)\n");
    assert!(
        big.chars().count() < 2200,
        "echoed {} characters",
        big.chars().count()
    );
    assert!(
        big.contains("(2000 of 688890 characters; print() writes all of it)"),
        "{}",
        &big[big.len().saturating_sub(200)..]
    );

    // print() is the way to see all of it, and it is not cut.
    let printed = echoed("print(range(100000));\n");
    assert!(
        printed.chars().count() > 600000,
        "print wrote {} characters",
        printed.chars().count()
    );
    assert!(
        !printed.contains("print() writes all of it"),
        "print was cut"
    );
}

/// The checker's sentence about a format template and the run's must
/// be the same sentence: `--check` reads the template with
/// `format_trouble` (1037) and the Format arm reads it again while
/// building the string. Two walks over the same grammar drift unless
/// something compares them, so this runs both over every shape that
/// does not depend on a value.
#[test]
fn a_format_template_reads_the_same_to_the_checker_and_the_run() {
    let dir = std::env::temp_dir().join("ting-format-template");
    std::fs::create_dir_all(&dir).unwrap();
    let calls = [
        r#"format("{:.1f}", 1.0)"#,
        r#"format("{} {}", 1)"#,
        r#"format("{}", 1, 2)"#,
        r#"format("{", 1)"#,
        r#"format("}", 1)"#,
        r#"format("{:q}", 1)"#,
        r#"format("{:{}}", "a")"#,
        r#"format("{:.{}}", 1.5)"#,
        r#"format("{:>{}.{}}", 1.5, 8)"#,
        r#"format("hello")"#,
        r#"format("{{literal}}", 1)"#,
        r#"format("{} and {}", 1, 2, 3)"#,
        // A spec that read a number from the arguments makes
        // "placeholders against arguments" the wrong sum, so the
        // template says what it takes instead.
        r#"format("{:{}}", 1, 4, 9)"#,
    ];
    let mut disagreed = Vec::new();
    let mut checked = 0;
    for (i, call) in calls.iter().enumerate() {
        let script = dir.join(format!("t{i}.ting"));
        std::fs::write(&script, format!("print({call});\n")).unwrap();
        let run = Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg(&script)
            .output()
            .expect("failed to run ting");
        let check = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(["--check"])
            .arg(&script)
            .output()
            .expect("failed to run ting");
        let said = |out: &std::process::Output, mark: &str| -> Option<String> {
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .find_map(|l| l.split_once(mark).map(|(_, rest)| rest.trim().to_string()))
        };
        let ran = said(&run, "error: format:");
        let checked_it = said(&check, "warning: format:");
        if ran != checked_it {
            disagreed.push(format!("{call}\n  run:   {ran:?}\n  check: {checked_it:?}"));
        }
        if ran.is_some() {
            checked += 1;
        }
    }
    assert!(
        disagreed.is_empty(),
        "the checker and the run disagree:\n{}",
        disagreed.join("\n")
    );
    // A table that stopped producing errors would agree about nothing
    // and pass.
    assert!(checked >= 9, "only {checked} of the calls were refused");
}

/// Both engines say the same thing about a shadowed builtin that
/// cannot be called: the VM reads the name out of the span its Call
/// op carries, the tree-walker out of the callee expression, and the
/// two must not drift (1038).
#[test]
fn both_engines_name_a_shadowed_builtin_that_is_not_callable() {
    let dir = std::env::temp_dir().join("ting-shadow-note");
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("shadow.ting");
    std::fs::write(&script, "let args = {};\nprint(args());\n").unwrap();
    let said = |engine: Option<&str>| -> String {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_ting"));
        cmd.arg(&script);
        if let Some(e) = engine {
            cmd.env("TING_ENGINE", e);
        }
        let out = cmd.output().expect("failed to run ting");
        String::from_utf8_lossy(&out.stderr)
            .lines()
            .next()
            .unwrap_or_default()
            .to_string()
    };
    let vm = said(None);
    assert!(
        vm.ends_with("map is not callable (`args` shadows the builtin of that name)"),
        "the vm said: {vm}"
    );
    assert_eq!(vm, said(Some("eval")));
}

/// A module's own text answers for a span inside it: the note names
/// what the MODULE wrote, not whatever stands at those offsets in the
/// file that imported it (1038).
#[test]
fn a_shadowed_builtin_inside_a_module_reads_the_modules_own_source() {
    let dir = std::env::temp_dir().join("ting-shadow-module");
    std::fs::create_dir_all(&dir).unwrap();
    // The `let` is far enough down that the same offsets in main.ting
    // are a different line entirely.
    std::fs::write(
        dir.join("m.ting"),
        "let len = {\"a\": 1};\nfn go() { return len([1]); }\nlet go = go;\n",
    )
    .unwrap();
    let main = dir.join("main.ting");
    std::fs::write(&main, "let m = import(\"m.ting\");\nprint(m[\"go\"]());\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&main)
        .output()
        .expect("failed to run ting");
    let first = String::from_utf8_lossy(&out.stderr)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    assert!(
        first.ends_with("map is not callable (`len` shadows the builtin of that name)"),
        "ting said: {first}"
    );
    assert!(first.contains("m.ting"), "ting said: {first}");
}

/// A file that used lib/test.ting, failed a check and never called
/// `summary()` used to report `ok` and exit 0 — the failure sat in a
/// map nobody read (1043). It fails now, with or without the last
/// line, and `reset()` is how a file that arranged its failures says
/// it has read them.
#[test]
fn a_check_that_failed_unprinted_still_fails_the_file() {
    let root = std::env::temp_dir().join(format!("ting-unprinted-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let forgot = "let t = import(\"lib/test.ting\");\nt[\"check_eq\"](\"two is three\", 2, 3);\n";
    std::fs::write(root.join("forgot.ting"), forgot).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.contains("FAIL: two is three: got 2, want 3"),
        "the failure is missing:\n{stdout}"
    );
    assert!(
        stdout.contains("never called summary()"),
        "no word about the missing line:\n{stdout}"
    );

    // The same file, having read its failures and cleared them.
    std::fs::write(
        root.join("forgot.ting"),
        format!("{forgot}t[\"reset\"]();\n"),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");

    // And a file that passes is untouched: one check, no verdict of
    // its own, exit 0.
    std::fs::write(
        root.join("forgot.ting"),
        "let t = import(\"lib/test.ting\");\nt[\"check_eq\"](\"two is two\", 2, 2);\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .args(["--test", root.to_str().unwrap()])
        .output()
        .expect("failed to run ting");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("1 check"), "{stdout}");
}

/// `exit()` is the other way out of a run, and 1043's verdict was not
/// on it: a file that failed a check and then left happily reported
/// `ok` (1044). A code that already says "failed" is left alone.
#[test]
fn a_happy_exit_does_not_swallow_a_failed_check() {
    let root = std::env::temp_dir().join(format!("ting-happy-exit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let run = |src: &str| -> (Option<i32>, String) {
        std::fs::write(root.join("e.ting"), src).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .args(["--test", root.to_str().unwrap()])
            .output()
            .expect("failed to run ting");
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).to_string(),
        )
    };
    let failing = "let t = import(\"lib/test.ting\");\nt[\"check_eq\"](\"two is three\", 2, 3);\n";
    let (code, stdout) = run(&format!("{failing}exit(0);\n"));
    assert_eq!(code, Some(1), "{stdout}");
    assert!(
        stdout.contains("FAIL: two is three: got 2, want 3"),
        "the failure is missing:\n{stdout}"
    );
    // A file that checked and passed leaves by the door it chose.
    let (code, stdout) =
        run("let t = import(\"lib/test.ting\");\nt[\"check_eq\"](\"fine\", 2, 2);\nexit(0);\n");
    assert_eq!(code, Some(0), "{stdout}");
    // And summary(), which exits 1 itself, still reports its own way
    // rather than twice.
    let (code, stdout) = run(&format!("{failing}t[\"summary\"]();\n"));
    assert_eq!(code, Some(1), "{stdout}");
    assert_eq!(
        stdout.matches("two is three").count(),
        1,
        "reported twice:\n{stdout}"
    );
}
