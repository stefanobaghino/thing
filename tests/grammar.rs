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
