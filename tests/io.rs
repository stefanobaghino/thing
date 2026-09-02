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
