//! The self-hosted suite: every selftest/*.ting is a ting program
//! full of assert() calls. Success is exit 0 with no output — a stray
//! print or a failed assertion fails the build.

use std::path::Path;
use std::process::Command;

#[test]
fn selftests_pass_silently() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("selftest");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("selftest/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ting") {
            continue;
        }
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg(&path)
            .output()
            .expect("failed to run ting");
        assert!(
            out.status.success(),
            "{} failed:\n{}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            out.stdout.is_empty(),
            "{} printed unexpectedly:\n{}",
            path.display(),
            String::from_utf8_lossy(&out.stdout)
        );
        checked += 1;
    }
    assert!(
        checked >= 5,
        "expected at least 5 selftests, found {checked}"
    );
}

/// The whole corpus under `--check`: the warnings it may print are
/// enumerated here, so a new false positive fails the build. Every one
/// is deliberate — a shadowed builtin, a duplicate key, a statement
/// after a return, three unbound names and a wrong-arity call — and
/// each was written to test the runtime that catches it.
#[test]
fn corpus_check_warnings_are_the_expected_seven() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("--check")
        .args(["lib", "selftest", "examples", "bench"])
        .current_dir(root)
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0), "the corpus must check clean");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let warnings: Vec<&str> = stderr.lines().filter(|l| l.contains("warning:")).collect();
    assert_eq!(warnings.len(), 7, "{stderr}");
    // File names only: Windows prints the paths with backslashes. A
    // file's warnings come in the order its lines do.
    let expected = [
        ("edge.ting", "shadows a builtin"),
        ("edge.ting", "duplicate key `a`"),
        ("edge.ting", "can never run"),
        ("errors.ting", "`totl` is bound nowhere"),
        ("errors.ting", "`amonut` is bound nowhere"),
        ("errors.ting", "`volme` is bound nowhere"),
        ("functions.ting", "called with 1"),
    ];
    for (i, (file, phrase)) in expected.iter().enumerate() {
        assert!(
            warnings[i].contains(file) && warnings[i].contains(phrase),
            "{stderr}"
        );
    }
}
