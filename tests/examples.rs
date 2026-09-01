//! Every examples/*.ting must run cleanly and print exactly its
//! examples/*.out counterpart.

use std::path::Path;
use std::process::Command;

#[test]
fn examples_produce_expected_output() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("examples/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ting") {
            continue;
        }
        let expected_path = path.with_extension("out");
        let expected = std::fs::read_to_string(&expected_path)
            .unwrap_or_else(|_| panic!("missing {}", expected_path.display()));
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg(&path)
            .output()
            .expect("failed to run ting");
        assert!(
            out.status.success(),
            "{} exited nonzero:\n{}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            expected,
            "wrong output for {}",
            path.display()
        );
        checked += 1;
    }
    assert!(checked >= 6, "expected at least 6 examples, found {checked}");
}
