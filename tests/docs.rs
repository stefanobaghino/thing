//! The reference must document every builtin: adding one without a
//! docs/reference.md entry fails here (companion to tests/grammar.rs).

#[test]
fn reference_documents_every_builtin() {
    let reference = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/reference.md"),
    )
    .expect("docs/reference.md missing");
    let mut missing = Vec::new();
    for b in ting::value::Builtin::ALL {
        let name = b.name();
        if !reference.contains(&format!("`{name}(")) && !reference.contains(&format!("`{name}`")) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "builtins missing from docs/reference.md: {missing:?}"
    );
}

/// GitHub-rendered markdown treats `<word>` as raw HTML: known tags
/// like `<pre>` open a block that swallows the rest of the page, and
/// unknown ones (`<RefCell>`) are silently stripped. Outside code
/// fences and inline backticks, tag-shaped tokens must be escaped.
/// (Found the hard way: LOG.md once shipped a bare `<pre>`.)
/// docs/stdlib.md must carry a row for every function in lib/*.ting
/// and state the real function count in its opening: a helper added
/// without a row, or a count left behind, fails here.
#[test]
fn stdlib_page_lists_every_function_and_the_right_count() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page =
        std::fs::read_to_string(root.join("docs/stdlib.md")).expect("docs/stdlib.md missing");
    let mut total = 0usize;
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(root.join("lib")).expect("lib/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ting") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        for line in src.lines() {
            // A module offers a name either by defining it or, for a
            // builtin it re-exports, by declaring `let f = f;`. Both
            // belong on the page, so both are counted here.
            let name = match line.strip_prefix("fn ") {
                Some(rest) => &rest[..rest.find('(').unwrap_or(rest.len())],
                None => {
                    let Some(rest) = line.strip_prefix("let ") else {
                        continue;
                    };
                    let Some((name, init)) = rest.split_once(" = ") else {
                        continue;
                    };
                    if init != format!("{name};") {
                        continue;
                    }
                    name
                }
            };
            total += 1;
            if !page.contains(&format!("`{name}(")) {
                missing.push(format!(
                    "{}: {name}",
                    path.file_name().unwrap().to_string_lossy()
                ));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "stdlib functions without a row on docs/stdlib.md: {missing:?}"
    );
    let stated = page
        .lines()
        .find_map(|l| {
            let i = l.find(" functions between them")?;
            l[..i].split_whitespace().last()?.parse::<usize>().ok()
        })
        .expect("docs/stdlib.md should state \"N functions between them\"");
    assert_eq!(
        stated, total,
        "docs/stdlib.md states {stated} functions; lib/ has {total}"
    );
}

#[test]
fn markdown_has_no_bare_html_tags() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![
        root.join("README.md"),
        root.join("LOG.md"),
        root.join("STATE.md"),
        root.join("LOOP.md"),
        root.join("CHANGELOG.md"),
    ];
    for entry in std::fs::read_dir(root.join("docs")).expect("docs/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            files.push(path);
        }
    }

    let tag_shaped = |line: &str| -> Option<String> {
        // Split out backtick spans; scan only the outside parts.
        for (i, part) in line.split('`').enumerate() {
            if i % 2 == 1 {
                continue;
            }
            let bytes = part.as_bytes();
            let mut k = 0;
            while k < bytes.len() {
                if bytes[k] == b'<' && k + 1 < bytes.len() && bytes[k + 1].is_ascii_alphabetic() {
                    let end = part[k + 1..].find('>').map(|e| k + 1 + e);
                    if let Some(end) = end
                        && part[k + 1..end].chars().all(|c| c.is_ascii_alphanumeric())
                    {
                        return Some(part[k..=end].to_string());
                    }
                }
                k += 1;
            }
        }
        None
    };

    let mut offenders = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let mut fenced = false;
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("```") {
                fenced = !fenced;
                continue;
            }
            if fenced {
                continue;
            }
            if let Some(tag) = tag_shaped(line) {
                offenders.push(format!("{}:{}: {tag}", path.display(), n + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "bare HTML-shaped tokens in markdown (wrap in backticks):\n{}",
        offenders.join("\n")
    );
}

/// docs/cookbook.md is generated by tools/cookbook.py from examples/;
/// this keeps the committed page honest: every example's source and
/// golden output must appear in it verbatim, and nothing else may
/// claim to be an example.
#[test]
fn cookbook_matches_examples() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(root.join("docs/cookbook.md")).expect("docs/cookbook.md");
    let mut count = 0;
    for entry in std::fs::read_dir(root.join("examples")).expect("examples/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ting") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let src = std::fs::read_to_string(&path).unwrap();
        let out = std::fs::read_to_string(path.with_extension("out")).unwrap();
        assert!(
            page.contains(&format!("\n## {stem}\n")),
            "cookbook lacks section for {stem}"
        );
        assert!(
            page.contains(src.trim_end()),
            "cookbook source for {stem} is stale; run python3 tools/cookbook.py"
        );
        assert!(
            page.contains(out.trim_end()),
            "cookbook output for {stem} is stale; run python3 tools/cookbook.py"
        );
        count += 1;
    }
    let sections = page.matches("\n## ").count();
    assert_eq!(
        sections, count,
        "cookbook has {sections} sections for {count} examples"
    );
}

/// playground/examples.js is generated by tools/playground_examples.py:
/// every example that the browser can run (no stdin, no arguments)
/// must appear under its stem with its first code line verbatim (with
/// "../lib/" imports rewritten to "lib/"), and nothing else may.
#[test]
fn playground_examples_match_examples_dir() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let js = std::fs::read_to_string(root.join("playground/examples.js")).expect("examples.js");
    let mut expected = 0;
    for entry in std::fs::read_dir(root.join("examples")).expect("examples/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ting") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        if src.contains("read_file(\"-\")") || src.contains("args()") {
            continue;
        }
        expected += 1;
        let stem = path.file_stem().unwrap().to_str().unwrap();
        assert!(
            js.contains(&format!("\"{stem}\": ")),
            "examples.js lacks {stem}; run python3 tools/playground_examples.py"
        );
        let first_code = src
            .lines()
            .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .unwrap()
            .replace("\"../lib/", "\"lib/");
        let escaped = first_code.replace('\\', "\\\\").replace('"', "\\\"");
        assert!(
            js.contains(&escaped),
            "examples.js is stale for {stem} ({first_code}); regenerate"
        );
    }
    let keys = js.matches("\": \"").count();
    assert_eq!(
        keys, expected,
        "examples.js has {keys} entries for {expected} runnable examples"
    );
}

/// Every ting code block in the tutorial and the reference runs, and
/// prints what the page says it prints.
///
/// The cookbook has had this guarantee all along, because it is
/// generated from examples/, which tests/examples.rs replays against
/// recorded output. The tutorial and the reference are written by
/// hand, and nothing ran them: a snippet that the language had moved
/// out from under would sit there until a reader copied it.
///
/// A block followed by a ```text``` block is claiming that output, and
/// the claim is checked exactly. A block with no such claim is only
/// run — the tutorial has one, whose output depends on whether git is
/// installed, and the reference makes no claims at all.
///
/// A block that is an illustration rather than a program says so on
/// its first line, `# not a program:` and the reason, which is a
/// sentence the reader gets as well. That way "this one is not meant
/// to run" is a decision written in the file, not a gap in the test.
///
/// Each block runs in a directory of its own, because they write into
/// the working one — the tutorial's walk_ext example makes
/// `report/data` and fills it.
#[test]
fn documented_snippets_run() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // Per page: blocks whose output was compared, blocks only run, and
    // blocks marked as illustrations. Pinning all three means a claim
    // that quietly disappeared would fail here rather than stop being
    // checked.
    for (page, want_checked, want_run_only, want_skipped) in
        [("tutorial", 46, 1, 0), ("reference", 0, 6, 2)]
    {
        let src = std::fs::read_to_string(root.join(format!("docs/{page}.md")))
            .unwrap_or_else(|_| panic!("docs/{page}.md missing"));
        let (mut checked, mut run_only, mut skipped) = (0, 0, 0);
        for (i, (block, claim)) in ting_blocks(&src).into_iter().enumerate() {
            if block.starts_with("# not a program:") {
                skipped += 1;
                continue;
            }
            let dir = std::env::temp_dir()
                .join(format!("ting-snippet-{}-{page}-{i}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("a directory for the snippet");
            let file = dir.join("snippet.ting");
            std::fs::write(&file, &block).expect("the snippet is written");
            let out = std::process::Command::new(env!("CARGO_BIN_EXE_ting"))
                .arg(&file)
                .current_dir(&dir)
                .stdin(std::process::Stdio::null())
                .output()
                .expect("failed to run ting");
            let status = out.status;
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            std::fs::remove_dir_all(&dir).expect("the directory goes away again");
            assert!(
                status.success(),
                "docs/{page}.md block {i} does not run:\n{block}\n{stderr}"
            );
            match claim {
                Some(want) => {
                    assert_eq!(
                        stdout, want,
                        "docs/{page}.md block {i} does not print what it claims:\n{block}"
                    );
                    checked += 1;
                }
                None => run_only += 1,
            }
        }
        assert_eq!(
            (checked, run_only, skipped),
            (want_checked, want_run_only, want_skipped),
            "docs/{page}.md: (checked, run only, illustrations) changed"
        );
    }
}

/// The bodies of the ```ting``` fenced blocks, in order, each with the
/// ```text``` block that immediately follows it where there is one —
/// which is how the pages state what a snippet prints.
fn ting_blocks(src: &str) -> Vec<(String, Option<String>)> {
    let mut fences: Vec<(&str, String)> = Vec::new();
    let mut open: Option<(&str, String)> = None;
    for line in src.lines() {
        match &mut open {
            None => {
                let trimmed = line.trim_end();
                if let Some(kind) = trimmed.strip_prefix("```")
                    && matches!(kind, "ting" | "text")
                {
                    open = Some((kind, String::new()));
                }
            }
            Some((_, body)) if line.trim_end() == "```" => {
                let (kind, body) = open.take().expect("a fence is open");
                fences.push((kind, body));
            }
            Some((_, body)) => {
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    let mut out = Vec::new();
    for (i, (kind, body)) in fences.iter().enumerate() {
        if *kind != "ting" {
            continue;
        }
        let claim = match fences.get(i + 1) {
            Some((next, text)) if *next == "text" => Some(text.clone()),
            _ => None,
        };
        out.push((body.clone(), claim));
    }
    out
}

/// The tutorial shows a bundle: two files, the command, and what it
/// writes. That listing is generated output, so it is checked the way
/// the snippets are — the two source files come out of the page, run
/// through `--bundle`, and the result must be the third block
/// character for character. A changed header comment or a changed
/// binding name would go stale here rather than in front of a reader.
#[test]
fn the_tutorials_bundle_is_what_bundle_prints() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(root.join("docs/tutorial.md")).expect("the tutorial");
    let section = src
        .split_once("### Handing the program over as one file")
        .expect("the bundling section")
        .1;
    let section = section.split_once("\n## ").expect("a section after it").0;
    // The blocks with no language: the two sources and the bundle.
    // The `sh` line between them carries one and is skipped.
    let mut blocks = Vec::new();
    let mut open: Option<(String, String)> = None;
    for line in section.lines() {
        match &mut open {
            None => {
                if let Some(kind) = line.trim_end().strip_prefix("```") {
                    open = Some((kind.to_string(), String::new()));
                }
            }
            Some(_) if line.trim_end() == "```" => {
                let (kind, body) = open.take().expect("a fence is open");
                if kind.is_empty() {
                    blocks.push(body);
                }
            }
            Some((_, body)) => {
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    assert_eq!(blocks.len(), 3, "expected greeter, main and the bundle");

    let dir = std::env::temp_dir().join(format!("ting-tutorial-bundle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory for the two files");
    std::fs::write(dir.join("greeter.ting"), &blocks[0]).unwrap();
    let main = dir.join("main.ting");
    std::fs::write(&main, &blocks[1]).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("--bundle")
        .arg(&main)
        .output()
        .expect("failed to run ting");
    assert!(
        out.status.success(),
        "--bundle failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        blocks[2],
        "the tutorial's bundle is not what --bundle prints"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
