//! One way to name a file. A path the reader typed on the command
//! line is printed back the way they typed it; a path the run
//! resolved for itself — an import — is written relative to the
//! directory the command ran in. `--check` has always done both, and
//! this holds every other surface that prints a file name to it:
//! `--fmt`, `--doc`, `--test`, `--profile` and `--coverage`. The
//! `--lsp` guard lives in tests/lsp.rs, beside the server harness.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A directory holding a module in `sub/`, three scripts that reach
/// it, and one file that wants reformatting. `sub/` is the point: a
/// surface naming the module by its last component alone would pass
/// every assertion below without ever saying which `m.ting` it meant.
fn fixture(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ting-paths-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(
        dir.join("sub").join("m.ting"),
        "# What the module is for.\nfn slow(n) {\n  let s = 0;\n  for i in range(n) { s = s + i; }\n  return s;\n}\nfn boom() { fail(\"boom\"); }\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.ting"),
        "let m = import(\"./sub/m.ting\");\nlet unused = 1;\nfn local(n) { return m[\"slow\"](n); }\nassert(local(10) == 45, \"slow\");\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("failing.ting"),
        "let m = import(\"./sub/m.ting\");\nm[\"boom\"]();\n",
    )
    .unwrap();
    std::fs::write(dir.join("bad.ting"), "let  x=1;\nprint(x);\n").unwrap();
    dir
}

/// `ting` run with its working directory inside the fixture, with the
/// two streams joined: which one a table goes to is not what is being
/// checked here.
fn ting(dir: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to run ting");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// The absolute form of a name inside the fixture, as a reader would
/// type it on the command line.
fn abs(dir: &Path, name: &str) -> String {
    dir.join(name).display().to_string()
}

/// The rule itself, at the surface every other one is held to.
#[test]
fn check_prints_back_the_path_it_was_given() {
    let dir = fixture("check");
    let out = ting(&dir, &["--check", "main.ting"]);
    assert!(out.contains("main.ting:2:5: warning:"), "{out}");
    let long = abs(&dir, "main.ting");
    let out = ting(&dir, &["--check", &long]);
    assert!(out.contains(&format!("{long}:2:5: warning:")), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fmt_prints_back_the_path_it_was_given() {
    let dir = fixture("fmt");
    let out = ting(&dir, &["--fmt-check", "bad.ting"]);
    assert!(out.contains("would reformat bad.ting\n"), "{out}");
    let long = abs(&dir, "bad.ting");
    let out = ting(&dir, &["--fmt-check", &long]);
    assert!(out.contains(&format!("would reformat {long}\n")), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn doc_prints_back_the_path_it_was_given() {
    let dir = fixture("doc");
    let out = ting(&dir, &["--doc", "sub/m.ting"]);
    assert!(out.starts_with("sub/m.ting:\n"), "{out}");
    let long = abs(&dir, "sub/m.ting");
    let out = ting(&dir, &["--doc", &long]);
    assert!(out.starts_with(&format!("{long}:\n")), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--test` names two files at once: the file it was given, and the
/// module the failure came out of.
#[test]
fn test_names_the_file_it_was_given_and_shortens_the_module() {
    let dir = fixture("test");
    let out = ting(&dir, &["--test", "failing.ting"]);
    assert!(out.contains("FAIL failing.ting\n"), "{out}");
    assert!(out.contains("sub/m.ting:7:13: error: boom"), "{out}");
    let long = abs(&dir, "failing.ting");
    let out = ting(&dir, &["--test", &long]);
    assert!(out.contains(&format!("FAIL {long}\n")), "{out}");
    assert!(out.contains("sub/m.ting:7:13: error: boom"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The profile table names where each function was defined, and so
/// holds both kinds of name at once: `local` is in the file the
/// reader named, `slow` in the module the run resolved.
#[test]
fn profile_keeps_the_typed_name_and_shortens_the_module() {
    let dir = fixture("profile");
    for arg in ["main.ting".to_string(), abs(&dir, "main.ting")] {
        let out = ting(&dir, &["--profile", &arg]);
        assert!(out.contains("sub/m.ting:2:1"), "{out}");
        assert!(out.contains(&format!("{arg}:3:1")), "{out}");
        assert!(!out.contains(&format!("{}/sub", dir.display())), "{out}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The coverage table holds both kinds of name in one column, so it
/// is where the two halves of the rule have to agree.
#[test]
fn coverage_keeps_the_typed_name_and_shortens_the_module() {
    let dir = fixture("coverage");
    let out = ting(&dir, &["--coverage", "main.ting"]);
    assert!(out.contains(" main.ting\n"), "{out}");
    assert!(out.contains(" sub/m.ting\n"), "{out}");
    let long = abs(&dir, "main.ting");
    let out = ting(&dir, &["--coverage", &long]);
    assert!(out.contains(&format!(" {long}\n")), "{out}");
    assert!(out.contains(" sub/m.ting\n"), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}
