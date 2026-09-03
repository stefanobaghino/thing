//! Formatter guards over every .ting file in the repo: formatting is
//! idempotent and preserves the parsed AST exactly.

use std::path::PathBuf;

fn ast_fingerprint(src: &str) -> String {
    let tokens = ting::lexer::lex(src).unwrap();
    let program = ting::parser::parse_program(&tokens).unwrap();
    program
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn formatting_is_idempotent_and_ast_preserving() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    for dir in ["examples", "selftest", "lib", "bench"] {
        for entry in std::fs::read_dir(root.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("ting") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            let once = ting::fmt::format(&src)
                .unwrap_or_else(|e| panic!("{} does not lex: {}", path.display(), e.message));
            let twice = ting::fmt::format(&once).unwrap();
            assert_eq!(once, twice, "not idempotent: {}", path.display());
            assert_eq!(
                once,
                src,
                "not formatted (run: ting --fmt {}):",
                path.display()
            );
            assert_eq!(
                ast_fingerprint(&src),
                ast_fingerprint(&once),
                "AST changed by formatting: {}",
                path.display()
            );
            checked += 1;
        }
    }
    assert!(checked >= 15, "only {checked} ting files checked");
}

mod common;

/// The formatter's two invariants — idempotence and an unchanged AST —
/// over thousands of grammar-generated programs, not just the
/// hand-written corpus. TING_FMT_CASES / TING_FMT_SEED tune a sweep.
#[test]
fn generated_programs_format_idempotently_and_preserve_ast() {
    let seed = std::env::var("TING_FMT_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0xF0F0);
    let cases = std::env::var("TING_FMT_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2000);
    let mut g = common::Gen::new(seed);
    for case in 0..cases {
        let src = g.program();
        let once = ting::fmt::format(&src)
            .unwrap_or_else(|e| panic!("case {case} does not lex: {}\n{src}", e.message));
        let twice = ting::fmt::format(&once).unwrap();
        assert_eq!(once, twice, "not idempotent on case {case}:\n{src}");
        assert_eq!(
            ast_fingerprint(&src),
            ast_fingerprint(&once),
            "AST changed by formatting on case {case}:\n{src}\n--- formatted:\n{once}"
        );
        // The same program with CRLF endings formats to the same text
        // with CRLF endings, and stays put on a second pass.
        let crlf = src.replace('\n', "\r\n");
        let once_crlf = ting::fmt::format(&crlf).unwrap();
        assert_eq!(
            once_crlf,
            once.replace('\n', "\r\n"),
            "CRLF case {case}:\n{src}"
        );
        assert_eq!(
            ting::fmt::format(&once_crlf).unwrap(),
            once_crlf,
            "CRLF idempotence {case}"
        );
    }
}
