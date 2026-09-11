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
    assert!(
        checked >= 6,
        "expected at least 6 examples, found {checked}"
    );
}

/// What the `.out` beside an example cannot hold: the run above gives
/// every example the empty command line, so an example that is a
/// command-line program is only half checked by it. `report.ting` is
/// that example — it reaches the front door in lib/args.ting with a
/// real `args()` — and these are the two answers a person gets from
/// it without ever seeing stdout.
#[test]
fn the_command_line_example_answers_help_and_refuses_the_rest() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("report.ting");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg(&path)
            .args(args)
            .output()
            .expect("failed to run ting")
    };

    let help = run(&["--help"]);
    let text = String::from_utf8_lossy(&help.stdout).to_string();
    assert_eq!(
        help.status.code(),
        Some(0),
        "--help left with {:?}",
        help.status.code()
    );
    assert!(text.contains("usage: report [options] [file...]"), "{text}");
    assert!(
        !text.contains("region,total"),
        "--help ran the program too:\n{text}"
    );

    let bad = run(&["--nope"]);
    let said = String::from_utf8_lossy(&bad.stderr).to_string();
    assert_eq!(
        bad.status.code(),
        Some(2),
        "a bad command line left with {:?}",
        bad.status.code()
    );
    assert!(
        said.starts_with("report: unknown option --nope\n"),
        "{said}"
    );
    assert!(said.contains("usage: report [options] [file...]"), "{said}");
    assert!(
        bad.stdout.is_empty(),
        "the trouble belongs on stderr: {:?}",
        String::from_utf8_lossy(&bad.stdout)
    );

    // And the positional it now takes is a file it really reads.
    let dir = std::env::temp_dir().join(format!("ting-report-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("sales.csv");
    std::fs::write(&csv, "region,rep,amount\nwest,Ada,7\nwest,Bo,3\n").unwrap();
    let read = run(&["--quiet", csv.to_str().unwrap()]);
    let printed = String::from_utf8_lossy(&read.stdout).to_string();
    assert!(
        read.status.success(),
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    assert!(printed.starts_with("west,10\n"), "{printed}");
    let _ = std::fs::remove_dir_all(&dir);
}
