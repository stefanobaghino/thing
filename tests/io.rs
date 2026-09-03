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
