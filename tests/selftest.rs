//! The self-hosted suite: every selftest/*.ting is a ting program
//! full of assert() calls. Success is exit 0 with no output — a stray
//! print or a failed assertion fails the build.

use std::path::Path;
use std::process::Command;

#[test]
fn selftests_pass_silently() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("selftest");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("selftest/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ting") {
            continue;
        }
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg(&path)
            .output()
            .expect("failed to run ting");
        assert!(
            out.status.success(),
            "{} failed:\n{}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            out.stdout.is_empty(),
            "{} printed unexpectedly:\n{}",
            path.display(),
            String::from_utf8_lossy(&out.stdout)
        );
        checked += 1;
    }
    assert!(
        checked >= 5,
        "expected at least 5 selftests, found {checked}"
    );
}

/// The whole corpus under `--check`: the warnings it may print are
/// enumerated here, so a new false positive fails the build. Every one
/// is deliberate — a shadowed builtin, a duplicate key, a statement
/// after a return, eight unbound names and a wrong-arity call — and
/// each was written to test the runtime that catches it.
#[test]
fn corpus_check_warnings_are_the_expected_seven() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("--check")
        .args(["lib", "selftest", "examples", "bench"])
        .current_dir(root)
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0), "the corpus must check clean");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let warnings: Vec<&str> = stderr.lines().filter(|l| l.contains("warning:")).collect();
    assert_eq!(warnings.len(), 12, "{stderr}");
    // File names only: Windows prints the paths with backslashes. A
    // file's warnings come in the order its lines do.
    let expected = [
        ("edge.ting", "shadows a builtin"),
        ("edge.ting", "duplicate key `a`"),
        ("edge.ting", "can never run"),
        ("errors.ting", "`totl` is bound nowhere"),
        ("errors.ting", "`amonut` is bound nowhere"),
        ("errors.ting", "`volme` is bound nowhere"),
        // 817: four words other languages use, each answered with
        // ting's spelling rather than with a guess.
        (
            "errors.ting",
            "`null` is bound nowhere (ting writes this as `nil`)",
        ),
        (
            "errors.ting",
            "`None` is bound nowhere (ting writes this as `nil`)",
        ),
        (
            "errors.ting",
            "`True` is bound nowhere (ting writes this as `true`)",
        ),
        (
            "errors.ting",
            "`FALSE` is bound nowhere (ting writes this as `false`)",
        ),
        (
            "errors.ting",
            "`totl` is bound nowhere (did you mean `total`?)",
        ),
        ("functions.ting", "called with 1"),
    ];
    for (i, (file, phrase)) in expected.iter().enumerate() {
        assert!(
            warnings[i].contains(file) && warnings[i].contains(phrase),
            "{stderr}"
        );
    }
}

/// The twelve modules the binary carries and the twelve files the
/// release archive ships have to be the same twelve. `include_str!`
/// keeps each entry's TEXT in step with its file for free, but
/// nothing keeps the TABLE in step with the directory: a thirteenth
/// module added to `lib/` and forgotten in `EMBEDDED_STDLIB` ships in
/// the archive and is missing from the binary, and `("lib/map.ting",
/// include_str!("../lib/list.ting"))` compiles and is silently wrong.
/// The archive's copy is `cp -r lib dist/lib`, so guarding the table
/// against the directory guards it against the archive too.
#[test]
fn the_embedded_stdlib_is_exactly_the_lib_directory() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("lib");
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .expect("lib/ missing")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("ting"))
        .map(|p| format!("lib/{}", p.file_name().unwrap().to_string_lossy()))
        .collect();
    on_disk.sort();

    let mut embedded: Vec<String> = ting::eval::embedded_stdlib()
        .iter()
        .map(|(path, _)| (*path).to_string())
        .collect();
    embedded.sort();

    assert_eq!(
        embedded, on_disk,
        "the embedded stdlib and lib/ are not the same modules"
    );
    assert_eq!(embedded.len(), 12, "twelve modules, counted everywhere");

    for (path, source) in ting::eval::embedded_stdlib() {
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        let text = std::fs::read_to_string(&file).expect("module file");
        assert_eq!(
            *source,
            text.as_str(),
            "{path} carries the text of another file"
        );
    }
}

/// A `lib/` beside a script SHADOWS the embedded stdlib, so the
/// archive's copy is what an unpacked release actually runs — and
/// nothing has ever checked that the two agree once running. This
/// asks every module for its function names both ways: once from a
/// directory holding a copy of `lib/`, once from a directory holding
/// none, so the answer can only come from inside the binary.
#[test]
fn a_shadowing_lib_and_the_embedded_stdlib_answer_alike() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let base = std::env::temp_dir().join(format!("ting-stdlib-{}", std::process::id()));
    let shadowed = base.join("shadowed");
    let bare = base.join("bare");
    std::fs::create_dir_all(shadowed.join("lib")).expect("temp dir");
    std::fs::create_dir_all(&bare).expect("temp dir");
    for (path, _) in ting::eval::embedded_stdlib() {
        std::fs::copy(root.join(path), shadowed.join(path)).expect("copy module");
    }

    let mut program = String::new();
    for (path, _) in ting::eval::embedded_stdlib() {
        program.push_str(&format!(
            "let m = import(\"{path}\"); print(\"{path}\", sort(keys(m)));\n"
        ));
    }
    std::fs::write(shadowed.join("probe.ting"), &program).expect("write probe");
    std::fs::write(bare.join("probe.ting"), &program).expect("write probe");

    let run = |dir: &std::path::Path| {
        let out = Command::new(env!("CARGO_BIN_EXE_ting"))
            .arg(dir.join("probe.ting"))
            .current_dir(dir)
            .output()
            .expect("failed to run ting");
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let with_lib = run(&shadowed);
    let without = run(&bare);
    let _ = std::fs::remove_dir_all(&base);

    assert!(
        with_lib.lines().count() == 12 && with_lib.contains("lib/csv.ting"),
        "the probe did not reach every module: {with_lib}"
    );
    assert_eq!(
        with_lib, without,
        "a lib/ beside the script and the copy inside the binary do not agree"
    );
}
