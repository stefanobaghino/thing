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
