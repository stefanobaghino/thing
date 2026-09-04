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
    // One alternation covers every valid escape: a four-hex-digit \u,
    // or one of n, t, r, backslash, quote.
    assert!(
        grammar.contains(r#"\\\\(u[0-9a-fA-F]{4}|[ntr\\\\\"])"#),
        "grammar escape class out of sync with the lexer"
    );
    for src in [
        r#""\n""#,
        r#""\t""#,
        r#""\r""#,
        r#""\\""#,
        r#""\"""#,
        r#""\u0041""#,
        r#""\ud83d\ude00""#,
    ] {
        assert!(
            ting::lexer::lex(src).is_ok(),
            "lexer rejects {src} that the grammar marks valid"
        );
    }
}

/// The lexer's number forms and the grammar's constant.numeric class
/// must stay in sync — a literal the grammar lacks renders as plain
/// text in editors.
#[test]
fn grammar_number_class_matches_lexer() {
    let grammar = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("editor/ting.tmLanguage.json"),
    )
    .expect("grammar missing");
    assert!(
        grammar.contains(
            r"\\b(0x[0-9a-fA-F][0-9a-fA-F_]*|0b[01][01_]*|[0-9][0-9_]*(\\.[0-9][0-9_]*)?([eE][+-]?[0-9][0-9_]*)?)\\b"
        ),
        "grammar number class out of sync with the lexer"
    );
    for src in [
        "0",
        "42",
        "2.5",
        "1_000_000",
        "0xff",
        "0xFF_FF",
        "0b1010",
        "0b1_0",
    ] {
        assert!(
            ting::lexer::lex(src).is_ok(),
            "lexer rejects {src} that the grammar marks numeric"
        );
    }
}
