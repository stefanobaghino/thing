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
/// enumerated here, so a new false positive fails the build. Each of
/// these three is deliberate — a shadowed builtin, an unbound name and
/// a wrong-arity call, every one of them written to test the runtime
/// that catches it.
#[test]
fn corpus_check_warnings_are_the_expected_three() {
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
    assert_eq!(warnings.len(), 3, "{stderr}");
    // File names only: Windows prints the paths with backslashes.
    assert!(
        warnings[0].contains("edge.ting") && warnings[0].contains("shadows a builtin"),
        "{stderr}"
    );
    assert!(
        warnings[1].contains("errors.ting") && warnings[1].contains("bound nowhere"),
        "{stderr}"
    );
    assert!(
        warnings[2].contains("functions.ting") && warnings[2].contains("called with 1"),
        "{stderr}"
    );
}
