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
