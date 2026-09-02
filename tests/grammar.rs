//! The editor grammar must know every builtin: adding a builtin
//! without updating editor/ting.tmLanguage.json fails here.

#[test]
fn grammar_lists_every_builtin() {
    let grammar = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("editor/ting.tmLanguage.json"),
    )
    .expect("editor/ting.tmLanguage.json missing");
    let expected = ting::value::Builtin::ALL
        .iter()
        .map(|b| b.name())
        .collect::<Vec<_>>()
        .join("|");
    assert!(
        grammar.contains(&expected),
        "grammar builtins out of sync; expected the alternation:\n{expected}"
    );
}

/// The lexer's escape set (src/lexer.rs read_string) and the grammar's
/// constant.character.escape class must stay in sync — a lexer escape
/// the grammar lacks renders as invalid.illegal in editors.
#[test]
fn grammar_escape_class_matches_lexer() {
    let grammar = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("editor/ting.tmLanguage.json"),
    )
    .expect("grammar missing");
    // One char class covers all valid escapes: n, t, r, backslash, quote.
    assert!(
        grammar.contains(r#"\\\\[ntr\\\\\"]"#),
        "grammar escape class out of sync with the lexer"
    );
    for src in ["\"\\n\"", "\"\\t\"", "\"\\r\"", "\"\\\\\"", "\"\\\"\""] {
        assert!(
            ting::lexer::lex(src).is_ok(),
            "lexer rejects {src} that the grammar marks valid"
        );
    }
}
