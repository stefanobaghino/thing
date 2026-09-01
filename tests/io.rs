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
