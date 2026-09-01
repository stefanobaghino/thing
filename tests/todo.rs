//! Drives the examples/todo.ting showcase end to end with real argv,
//! a TODO_FILE in a temp dir, and observed exit codes.

use std::path::{Path, PathBuf};
use std::process::Command;

fn todo(data: &Path, args: &[&str]) -> (String, Option<i32>) {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/todo.ting");
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(script)
        .args(args)
        .env("TODO_FILE", data)
        .output()
        .expect("failed to run ting");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

#[test]
fn todo_cli_scenario() {
    let data = std::env::temp_dir().join(format!("ting-todo-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&data);

    assert_eq!(todo(&data, &[]), ("nothing to do!\n".into(), Some(0)));
    assert_eq!(
        todo(&data, &["add", "buy", "milk"]),
        ("added #1\n".into(), Some(0))
    );
    assert_eq!(
        todo(&data, &["add", "write tests"]),
        ("added #2\n".into(), Some(0))
    );
    assert_eq!(
        todo(&data, &["list"]),
        ("1. [ ] buy milk\n2. [ ] write tests\n".into(), Some(0))
    );
    assert_eq!(
        todo(&data, &["done", "2"]),
        ("done: write tests\n".into(), Some(0))
    );
    assert_eq!(todo(&data, &["rm", "1"]), ("removed #1\n".into(), Some(0)));
    assert_eq!(
        todo(&data, &["list"]),
        ("1. [x] write tests\n".into(), Some(0))
    );

    // Errors: bad number, bad command — message + exit 2.
    let (out, code) = todo(&data, &["done", "9"]);
    assert!(out.contains("no item #9"), "{out}");
    assert_eq!(code, Some(2));
    let (out, code) = todo(&data, &["frobnicate"]);
    assert!(out.starts_with("usage:"), "{out}");
    assert_eq!(code, Some(2));

    // The data file is real JSON on disk.
    let raw = std::fs::read_to_string(&data).unwrap();
    assert_eq!(raw, "[{\"done\":true,\"text\":\"write tests\"}]");
    let _ = std::fs::remove_file(&data);
}
