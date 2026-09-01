//! Every ```ting snippet in docs/tutorial.md must run cleanly, and
//! when a ```text block follows it, must print exactly that.

use std::path::Path;
use std::process::Command;

struct Snippet {
    line: usize,
    code: String,
    expected: Option<String>,
}

fn parse_snippets(doc: &str) -> Vec<Snippet> {
    let mut snippets: Vec<Snippet> = Vec::new();
    let mut lines = doc.lines().enumerate().peekable();
    while let Some((i, line)) = lines.next() {
        if line.trim() != "```ting" {
            continue;
        }
        let mut code = String::new();
        for (_, l) in lines.by_ref() {
            if l.trim() == "```" {
                break;
            }
            code.push_str(l);
            code.push('\n');
        }
        // An immediately following ```text block is the exact output.
        let mut expected = None;
        while lines.peek().is_some_and(|(_, l)| l.trim().is_empty()) {
            lines.next();
        }
        if lines.peek().is_some_and(|(_, l)| l.trim() == "```text") {
            lines.next();
            let mut out = String::new();
            for (_, l) in lines.by_ref() {
                if l.trim() == "```" {
                    break;
                }
                out.push_str(l);
                out.push('\n');
            }
            expected = Some(out);
        }
        snippets.push(Snippet {
            line: i + 1,
            code,
            expected,
        });
    }
    snippets
}

#[test]
fn tutorial_snippets_run_and_match() {
    let doc_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/tutorial.md");
    let doc = std::fs::read_to_string(&doc_path).expect("docs/tutorial.md missing");
    let snippets = parse_snippets(&doc);
    assert!(
        snippets.len() >= 8,
        "expected at least 8 ting snippets, found {}",
        snippets.len()
    );

    for (n, s) in snippets.iter().enumerate() {
        let script = std::env::temp_dir().join(format!("ting-tutorial-{n}.ting"));
        std::fs::write(&script, &s.code).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg(&script)
            .output()
            .expect("failed to run ting");
        assert!(
            out.status.success(),
            "tutorial snippet at line {} exited nonzero:\n{}",
            s.line,
            String::from_utf8_lossy(&out.stderr)
        );
        if let Some(expected) = &s.expected {
            assert_eq!(
                String::from_utf8_lossy(&out.stdout),
                *expected,
                "wrong output for tutorial snippet at line {}",
                s.line
            );
        }
        let _ = std::fs::remove_file(&script);
    }
}
