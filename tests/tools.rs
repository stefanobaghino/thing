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

/// playground/examples.js is generated, and tests/docs.rs checks its
/// CONTENT against examples/. This checks the GENERATOR: run in a
/// directory holding nothing but a copy of examples/, it has to
/// write the file that is committed here, byte for byte — which also
/// proves the tool's `import("lib/fs.ting")` reaches the embedded
/// stdlib, there being no lib/ beside it to answer.
#[test]
fn the_playground_generator_writes_the_file_that_is_committed() {
    let root = root();
    let base = std::env::temp_dir().join(format!("ting-playground-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("examples")).expect("temp dir");
    std::fs::create_dir_all(base.join("playground")).expect("temp dir");
    for entry in std::fs::read_dir(root.join("examples")).expect("examples/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("ting") {
            std::fs::copy(&path, base.join("examples").join(path.file_name().unwrap()))
                .expect("copy example");
        }
    }
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(root.join("tools/playground_examples.ting"))
        .current_dir(&base)
        .output()
        .expect("failed to run ting");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read(base.join("playground/examples.js")).expect("nothing written");
    let committed = std::fs::read(root.join("playground/examples.js")).expect("examples.js");
    let _ = std::fs::remove_dir_all(&base);
    assert!(
        written == committed,
        "the generator and playground/examples.js have parted ways"
    );
}

/// The site's pages are rendered by a ting program, so the renderer
/// has no second implementation to be compared against any more.
/// What it is held to instead is the document: every fenced block
/// becomes a `<pre>`, every ting block also a run link, every header
/// its own tag, and the title comes from the first `# ` line — a
/// renderer that loses a section or stops escaping fails here.
#[test]
fn the_site_renderer_answers_for_every_page() {
    let root = root();
    let base = std::env::temp_dir().join(format!("ting-site-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("temp dir");
    for page in [
        "docs/tutorial.md",
        "docs/reference.md",
        "docs/stdlib.md",
        "docs/cookbook.md",
        "docs/retrospective.md",
        "CHANGELOG.md",
    ] {
        let md = std::fs::read_to_string(root.join(page)).expect("page missing");
        let out = base.join(format!("{}.html", page.replace('/', "-")));
        let run = Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg("tools/md2html.ting")
            .arg(page)
            .arg(&out)
            .current_dir(&root)
            .output()
            .expect("failed to run ting");
        assert_eq!(
            run.status.code(),
            Some(0),
            "{page}\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let html = std::fs::read_to_string(&out).expect("nothing written");

        // Counted the way the renderer reads the document: a fence
        // opens, the lines inside it are code, the next fence closes.
        let mut fences = 0;
        let mut runlinks = 0;
        let mut headers = [0usize; 3];
        let mut inside = false;
        for line in md.lines() {
            if let Some(lang) = line.strip_prefix("```") {
                if !inside {
                    fences += 1;
                    if lang.trim() == "ting" {
                        runlinks += 1;
                    }
                }
                inside = !inside;
                continue;
            }
            if inside {
                continue;
            }
            let hashes = line.len() - line.trim_start_matches('#').len();
            if (1..=3).contains(&hashes) && line[hashes..].starts_with(' ') {
                headers[hashes - 1] += 1;
            }
        }
        assert!(!inside, "{page} has an unclosed fence");
        assert_eq!(
            html.matches("<pre><code>").count(),
            fences,
            "{page}: one code block in, one out"
        );
        assert_eq!(
            html.matches("class=\"runlink\"").count(),
            runlinks,
            "{page}: every ting block gets a playground link"
        );
        for level in 1..=3 {
            assert_eq!(
                html.matches(&format!("<h{level}>")).count(),
                headers[level - 1],
                "{page}: h{level} count"
            );
        }
        let title = md
            .lines()
            .find(|l| l.starts_with("# "))
            .map(|l| &l[2..])
            .expect("no title");
        assert!(
            html.contains(&format!("<title>{title}</title>")),
            "{page}: the title comes from the first header"
        );
        assert!(
            html.starts_with("<!doctype html>") && html.ends_with("</html>\n"),
            "{page}: a whole document"
        );
    }
    let _ = std::fs::remove_dir_all(&base);
}

/// Nothing the workflows run needs a Python interpreter any more:
/// the site, the playground list and the step reader are ting
/// programs. A workflow that reaches for python3 again is a
/// dependency this project decided not to have.
#[test]
fn no_workflow_reaches_for_python() {
    for workflow in workflows() {
        let text = std::fs::read_to_string(&workflow).expect("workflow unreadable");
        assert!(
            !text.contains("python"),
            "{} runs python",
            workflow.display()
        );
    }
}

/// docs/cookbook.md is generated too, and tests/docs.rs checks its
/// content. This checks the generator the same way the playground's
/// is checked: given nothing but a copy of examples/, it has to
/// write the page that is committed here, byte for byte.
#[test]
fn the_cookbook_generator_writes_the_page_that_is_committed() {
    let root = root();
    let base = std::env::temp_dir().join(format!("ting-cookbook-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("examples")).expect("temp dir");
    std::fs::create_dir_all(base.join("docs")).expect("temp dir");
    for entry in std::fs::read_dir(root.join("examples")).expect("examples/ missing") {
        let path = entry.unwrap().path();
        std::fs::copy(&path, base.join("examples").join(path.file_name().unwrap()))
            .expect("copy example");
    }
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(root.join("tools/cookbook.ting"))
        .current_dir(&base)
        .output()
        .expect("failed to run ting");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read(base.join("docs/cookbook.md")).expect("nothing written");
    let committed = std::fs::read(root.join("docs/cookbook.md")).expect("cookbook.md");
    let _ = std::fs::remove_dir_all(&base);
    assert!(
        written == committed,
        "the generator and docs/cookbook.md have parted ways"
    );
}

/// The tools that build this project are ting programs, and the only
/// other language left in tools/ is the shell script that smoke-tests
/// a release archive. A .py file here again means something this
/// repository builds needs an interpreter it does not ship.
#[test]
fn nothing_in_tools_is_written_in_python() {
    let mut strays = Vec::new();
    for entry in std::fs::read_dir(root().join("tools")).expect("tools/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("py") {
            strays.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert!(strays.is_empty(), "python left in tools/: {strays:?}");
}
