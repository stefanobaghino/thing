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

/// The tutorial counts the modules that ship inside the binary, and
/// the count is prose rather than a table, so nothing else would
/// catch it going stale. It said six for a long time after there
/// were thirteen.
#[test]
fn the_tutorial_counts_the_embedded_modules() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page =
        std::fs::read_to_string(root.join("docs/tutorial.md")).expect("docs/tutorial.md missing");
    let count = ting::eval::embedded_stdlib().len();
    // One string rather than an array of them: rustfmt versions
    // disagree about how to wrap an array of twenty short literals,
    // and a string literal is wrapped by nobody.
    let words = "Zero One Two Three Four Five Six Seven Eight Nine Ten Eleven Twelve Thirteen Fourteen Fifteen Sixteen Seventeen Eighteen Nineteen Twenty";
    let word = words
        .split(' ')
        .nth(count)
        .expect("more modules than words here");
    assert!(
        page.contains(&format!("{word} stdlib modules ship embedded")),
        "docs/tutorial.md does not say \"{word} stdlib modules ship embedded\"; lib/ has {count}"
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
            // A fence opens or closes a block; a line that merely
            // STARTS with an inline code span (```text``` at a line
            // break) is prose. Reading the second as the first left
            // this guard toggled open over the whole tail of LOG.md
            // for fifty-six iterations, checking nothing (767).
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("```")
                && !rest.contains('`')
            {
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

/// docs/cookbook.md is generated by tools/cookbook.ting from
/// examples/;
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
            "cookbook source for {stem} is stale; run ting tools/cookbook.ting"
        );
        assert!(
            page.contains(out.trim_end()),
            "cookbook output for {stem} is stale; run ting tools/cookbook.ting"
        );
        count += 1;
    }
    let sections = page.matches("\n## ").count();
    assert_eq!(
        sections, count,
        "cookbook has {sections} sections for {count} examples"
    );
}

/// playground/examples.js is generated by
/// tools/playground_examples.ting:
/// every example that the browser can run (no stdin, no arguments)
/// must appear under its stem with its WHOLE body verbatim (with
/// "../lib/" imports rewritten to "lib/"), and nothing else may.
/// Reading one line was enough for six examples to go stale below it
/// (LOG 783).
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
            "examples.js lacks {stem}; run ting tools/playground_examples.ting"
        );
        let escaped = src
            .replace("\"../lib/", "\"lib/")
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        assert!(
            js.contains(&escaped),
            "examples.js is stale for {stem}; run ting tools/playground_examples.ting"
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
        [("tutorial", 52, 1, 0), ("reference", 3, 7, 5)]
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

/// docs/stdlib.md opens by claiming how many functions the twelve
/// modules hold. That number drifts silently — a module gains a
/// function and the sentence does not — so it is asked of the
/// modules themselves rather than counted by hand. `grep '^fn '` is
/// not the answer either: lib/list.ting re-exports the builtin
/// sort_with with a `let`, and lib/test.ting exports a map of state
/// that is not a function at all.
#[test]
fn the_stdlib_page_counts_what_the_modules_export() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = r#"let names = ["list", "map", "string", "math", "json", "fs",
                                 "test", "time", "sh", "args", "err", "csv",
                                 "base64"];
let total = 0;
for n in names {
  let m = import("lib/" + n + ".ting");
  for k in keys(m) { if type(m[k]) == "function" { total += 1; } }
}
print(total);
"#;
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("-")
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(script.as_bytes())?;
            child.wait_with_output()
        })
        .expect("failed to run ting");
    let counted = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let page = std::fs::read_to_string(root.join("docs/stdlib.md")).expect("docs/stdlib.md");
    assert!(
        page.contains(&format!("{counted} functions between them")),
        "docs/stdlib.md does not say \"{counted} functions between them\", \
         which is what the modules export"
    );
    // STATE.md carries the same number in its standing shape, and
    // nothing was watching it: it said 188 for two ticks after the
    // modules moved to 190. It is orientation rather than published
    // documentation, so a wrong count there misleads only me — which
    // is reason enough to check it here, where the number is already
    // in hand.
    let state = std::fs::read_to_string(root.join("STATE.md")).expect("STATE.md");
    // Whitespace-insensitive: the count sits mid-sentence and a rewrap
    // must not read as a wrong number.
    let flat = state.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains(&format!("{counted} functions, guarded")),
        "STATE.md's standing shape does not say \"{counted} functions\", \
         which is what the modules export"
    );
}

/// The nesting limit is a number in three places: the parser
/// enforces it, the reference states it, and the message names it.
/// A change to the constant that left the page saying 200 would be
/// a documented promise the parser no longer keeps.
#[test]
fn the_reference_states_the_nesting_limit_the_parser_enforces() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(root.join("docs/reference.md")).expect("docs/reference.md");
    let limit = ting::parser::MAX_NESTING;
    assert!(
        page.contains(&format!("Nesting: {limit} levels")),
        "docs/reference.md does not say \"Nesting: {limit} levels\""
    );
    assert!(
        page.contains(&format!("nested too deeply (the limit is {limit} levels)")),
        "docs/reference.md does not quote the message the parser raises"
    );
}

/// The same for the JSON reader's own limit, which is a different
/// number for a different reason and stated on the same page.
#[test]
fn the_reference_states_the_json_depth_the_reader_enforces() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(root.join("docs/reference.md")).expect("docs/reference.md");
    let limit = ting::json::MAX_DEPTH;
    assert!(
        page.contains(&format!("JSON nesting: {limit} levels")),
        "docs/reference.md does not say \"JSON nesting: {limit} levels\""
    );
    assert!(
        page.contains(&format!("nested\n  deeper than {limit} at offset N")),
        "docs/reference.md does not quote the message json_parse raises"
    );
    let printed = ting::value::MAX_PRINT_DEPTH;
    assert!(
        page.contains(&format!("Printing: {printed} levels")),
        "docs/reference.md does not say \"Printing: {printed} levels\""
    );
}

/// What a program pays for the memory it holds is documented, not
/// discovered: 863 measured a recursive helper leaking its frame on
/// every call and 864 closed that, but a cycle a program builds
/// itself is still counted by references that count each other, and
/// the page has to say so — with the remedy, which is one assignment.
#[test]
fn the_reference_says_what_a_cycle_costs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(root.join("docs/reference.md")).expect("docs/reference.md");
    for claim in [
        "### Memory",
        "a cycle\ncounts itself, so it is never reclaimed",
        "xs[0] = nil;    # breaking the loop frees it",
    ] {
        assert!(
            page.contains(claim),
            "docs/reference.md does not carry: {claim}"
        );
    }
}

/// The message quoted for `assert` in the reference is the message
/// the binary prints. A doc example of a diagnostic drifts silently
/// otherwise: the wording lives in eval.rs, the quote lives here, and
/// nothing but this test connects them.
#[test]
fn the_reference_quotes_what_a_failed_assertion_really_prints() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(root.join("docs/reference.md")).expect("docs/reference.md");
    let dir = std::env::temp_dir().join(format!("ting-assert-doc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a directory for the program");
    let file = dir.join("kilos.ting");
    std::fs::write(&file, "let x = 9;\nassert(x == 8, \"three kilos\");\n").expect("written");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg(&file)
        .output()
        .expect("failed to run ting");
    std::fs::remove_dir_all(&dir).expect("the directory goes away again");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let message = stderr
        .lines()
        .find_map(|l| l.split_once("error: "))
        .map(|(_, m)| m.to_string())
        .unwrap_or_else(|| panic!("no diagnostic in:\n{stderr}"));
    assert!(
        page.contains(&format!("`{message}`")),
        "docs/reference.md does not quote what ting prints: {message}"
    );
}

/// What a spec IS lives in lib/args.ting's header comment, and 908
/// found that no tool and no page said it: `parse(spec, argv)` and
/// `help(spec)` named a shape nothing defined. The page carries it
/// now, and this pins the copy to the source so the two cannot drift.
#[test]
fn the_stdlib_page_carries_the_args_spec_from_the_source() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let module = std::fs::read_to_string(root.join("lib/args.ting")).expect("lib/args.ting");
    let page = std::fs::read_to_string(root.join("docs/stdlib.md")).expect("docs/stdlib.md");
    let header: Vec<String> = module
        .lines()
        .take_while(|l| l.starts_with('#'))
        .map(|l| l[1..].trim().to_string())
        .collect();
    let open = header
        .iter()
        .position(|l| l == "{")
        .expect("lib/args.ting's header should show a spec, opening with {");
    let close = open
        + header[open..]
            .iter()
            .position(|l| l == "}")
            .expect("and closing with }");
    for line in &header[open..=close] {
        assert!(
            page.contains(line.as_str()),
            "docs/stdlib.md does not carry the spec line {line:?} from lib/args.ting"
        );
    }
}

/// lib/time.ting opened with "there is no time zone here", and so did
/// its section on the page — while the module exports local_date,
/// local_clock and local_iso, which the same page lists three rows
/// down. 909 made that header something `--doc` prints, so a stale
/// comment became a wrong answer. Both have to name what the local_
/// functions actually do: ask the platform, through local_zone().
#[test]
fn what_says_time_has_no_zone_says_where_the_local_answers_come_from() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let module = std::fs::read_to_string(root.join("lib/time.ting")).expect("lib/time.ting");
    let header: String = module
        .lines()
        .take_while(|l| l.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        header.contains("local_zone()"),
        "lib/time.ting's header does not say where a local answer comes from:\n{header}"
    );
    let page = std::fs::read_to_string(root.join("docs/stdlib.md")).expect("docs/stdlib.md");
    let section = page
        .split("## lib/time.ting")
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .expect("docs/stdlib.md has no lib/time.ting section");
    assert!(
        section.contains("local_zone()"),
        "the page's lib/time.ting section does not either:\n{section}"
    );
}
