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
/// is deliberate — three shadowed builtins, a duplicate key, a
/// statement after a return, eleven unbound names and six
/// wrong-arity calls — and
/// each was written to test the runtime that catches it.
#[test]
fn corpus_check_warnings_are_the_expected_twenty_two() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("--check")
        .args(["lib", "selftest", "examples", "bench", "tools"])
        .current_dir(root)
        .output()
        .expect("failed to run ting");
    assert_eq!(out.status.code(), Some(0), "the corpus must check clean");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let warnings: Vec<&str> = stderr.lines().filter(|l| l.contains("warning:")).collect();
    assert_eq!(warnings.len(), 22, "{stderr}");
    // File names only: Windows prints the paths with backslashes. A
    // file's warnings come in the order its lines do.
    let expected = [
        // 981: lib/json.ting offers json_str as `str`, which is the
        // name the area already uses; inside that file the shadow is
        // the point, and its own code spells a step with format().
        ("json.ting", "`str` shadows a builtin"),
        // 830: the selftest that proves a shadowed `range` beats the
        // fused counting loop has to shadow one to do it.
        ("collections.ting", "`range` shadows a builtin"),
        // 1036: the checker counts a call to a builtin now, and the
        // selftests that prove the runtime's arity errors call three
        // of them wrongly on purpose.
        (
            "collections.ting",
            "`fingerprint` takes 1 argument, called with 0",
        ),
        (
            "collections.ting",
            "`compare` takes 2 arguments, called with 1",
        ),
        ("edge.ting", "shadows a builtin"),
        ("edge.ting", "duplicate key `a`"),
        ("edge.ting", "can never run"),
        ("edge.ting", "`cwd` takes 0 arguments, called with 1"),
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
        (
            "errors.ting",
            "`and` is bound nowhere (ting writes this as `&&`)",
        ),
        // 1022: a name the stdlib exports is answered with the module
        // that has it, ahead of any guess at a nearby name.
        (
            "errors.ting",
            "`repeat` is bound nowhere (lib/string.ting has it)",
        ),
        (
            "errors.ting",
            "`parse` is bound nowhere (lib/args.ting, lib/csv.ting and lib/json.ting have it)",
        ),
        ("functions.ting", "called with 1"),
        (
            "strings.ting",
            "`display_width` takes 1 argument, called with 0",
        ),
        (
            "time.ting",
            "`local_zone` takes 0 to 1 arguments, called with 2",
        ),
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
    assert_eq!(embedded.len(), 13, "thirteen modules, counted everywhere");

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
        with_lib.lines().count() == 13 && with_lib.contains("lib/csv.ting"),
        "the probe did not reach every module: {with_lib}"
    );
    assert_eq!(
        with_lib, without,
        "a lib/ beside the script and the copy inside the binary do not agree"
    );
}

mod common;

/// "Have I seen this value?" was a scan of everything kept so far, so
/// `unique` cost a comparison per pair: 4/16/68/261/1022 ms at 2000
/// through 32000 when 923 measured it, against 3/7/15/37 once 924
/// keyed it on `fingerprint`. `intersection` was the same nested loop
/// written by hand, since no module carried one until 925.
///
/// A TIMING ratio, not the allocation weight 918 used, and the reason
/// is worth stating: a scan allocates nothing per comparison. Putting
/// the quadratic version back and weighing bytes measured no
/// difference at all — the guard passed on the code it was written to
/// catch. Time is the only thing that changes here.
///
/// `doubling_ratio` interleaves the two sizes, takes the best of five
/// and the best of three rounds, so a busy host lengthens both sides
/// together: doubling the input can only double linear work, while
/// either scan quadruples it.
#[test]
fn the_sameness_helpers_cost_the_elements_not_the_squares() {
    fn unique_of(n: usize) -> String {
        format!(
            "let l = import(\"lib/list.ting\"); \
             let xs = []; let i = 0; while i < {n} {{ push(xs, str(i % ({n} / 2))); i += 1; }} \
             if len(l[\"unique\"](xs)) != {n} / 2 {{ fail(\"wrong answer\"); }}"
        )
    }
    fn intersection_of(n: usize) -> String {
        format!(
            "let l = import(\"lib/list.ting\"); \
             let a = []; let b = []; let i = 0; \
             while i < {n} {{ push(a, str(i)); push(b, str(i + {n} / 2)); i += 1; }} \
             if len(l[\"intersection\"](a, b)) != {n} / 2 {{ fail(\"wrong answer\"); }}"
        )
    }
    let run = |src: &str| {
        ting::run_source("bench", src, std::io::sink(), Vec::new()).expect("runs");
    };
    for (what, small, large) in [
        ("unique", unique_of(4000), unique_of(8000)),
        ("intersection", intersection_of(4000), intersection_of(8000)),
    ] {
        let ratio = common::doubling_ratio(|| run(&small), || run(&large));
        assert!(
            ratio < 3.0,
            "doubling the elements multiplied {what}'s work by {ratio:.1}: the scan is back"
        );
    }
}
