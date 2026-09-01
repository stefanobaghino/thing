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
