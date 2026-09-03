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

/// A directory argument expands to every .ting file beneath it, in
/// sorted order, recursively; other files are ignored.
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

    let _ = std::fs::remove_dir_all(&root);
}
