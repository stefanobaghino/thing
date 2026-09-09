//! The repo's own tools, written in the language they belong to. A
//! tool that builds or rehearses this project has to keep working
//! without anything installed beside the binary the build already
//! made, so each one is exercised here against the real files it
//! reads.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn workflows() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(root().join(".github/workflows"))
        .expect(".github/workflows missing")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yml"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no workflows to read");
    paths
}

fn step(workflow: &Path, name: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("tools/workflow_step.ting")
        .arg(workflow)
        .arg(name)
        .current_dir(root())
        .output()
        .expect("failed to run ting")
}

/// Whether a multi-line block sits in the workflow at that
/// indentation: introduced by a block scalar, whole, and ended by a
/// line that leaves it.
fn sits_in_the_file(text: &str, body: &str, indent: usize) -> bool {
    let pad = " ".repeat(indent);
    let candidate: Vec<String> = body
        .lines()
        .map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("{pad}{l}")
            }
        })
        .collect();
    let candidate = candidate.join("\n");
    text.match_indices(&candidate).any(|(at, _)| {
        let before = &text[..at];
        if !before.ends_with("run: |\n") && !before.ends_with("run: >\n") {
            return false;
        }
        match text[at + candidate.len()..].strip_prefix('\n') {
            None => true,
            Some(rest) => {
                let next = rest.lines().next().unwrap_or("");
                next.trim().is_empty() || next.len() - next.trim_start().len() < indent
            }
        }
    })
}

/// Rehearsing a CI step means running the bytes in the file: this
/// prints a named step's `run:` block so it can be piped into a
/// shell. Retyping a step rehearses a different step, which is how a
/// broken `cut -d` once reached four runners (774).
///
/// Every named step in every workflow goes through it. The block that
/// comes back cannot be checked against a copy of itself — a copy is
/// the mistake the tool exists to prevent — so it is checked against
/// the file: each line it printed has to be a line of the workflow,
/// which fails for a tool that drops, reorders or invents one.
#[test]
fn every_named_workflow_step_comes_back_from_the_file() {
    let mut steps = 0;
    for workflow in workflows() {
        let text = std::fs::read_to_string(&workflow).expect("workflow unreadable");
        let names: Vec<String> = text
            .lines()
            .filter_map(|l| l.trim().strip_prefix("- name: ").map(str::to_string))
            .collect();
        for name in names {
            let out = step(&workflow, &name);
            assert_eq!(
                out.status.code(),
                Some(0),
                "{} :: {name}\n{}",
                workflow.display(),
                String::from_utf8_lossy(&out.stderr)
            );
            let block = String::from_utf8_lossy(&out.stdout);
            assert!(
                !block.trim().is_empty(),
                "{} :: {name} came back empty",
                workflow.display()
            );
            // The block cannot be checked against a copy of itself
            // — a copy is the mistake the tool exists to prevent —
            // so it is checked against its place in the file: the
            // lines come back adjacent and in order, they start
            // right after the `run:` that introduces them, and the
            // line after them leaves the block, which is what a
            // truncated answer fails.
            let body = block.trim_end_matches('\n');
            let placed = if body.contains('\n') {
                (1..=20).any(|indent| sits_in_the_file(&text, body, indent))
            } else {
                text.contains(&format!("run: {body}\n"))
            };
            assert!(
                placed,
                "{} :: {name} printed something the workflow does not hold there:\n{body}",
                workflow.display()
            );
            steps += 1;
        }
    }
    assert!(steps >= 20, "only {steps} named steps found");
}

/// A step name nobody wrote is an error, not an empty block: a
/// rehearsal that silently runs nothing is a rehearsal that passes.
#[test]
fn an_unknown_step_name_is_refused() {
    let workflow = root().join(".github/workflows/ci.yml");
    let out = step(&workflow, "No Such Step");
    assert_ne!(out.status.code(), Some(0), "an absent step must fail");
    assert!(out.stdout.is_empty(), "an absent step must print nothing");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no step named"), "{err}");
}
