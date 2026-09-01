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
