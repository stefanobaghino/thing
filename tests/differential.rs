//! Differential testing: the same program through both engines must
//! produce byte-identical stdout, or the same rendered error. Corpus
//! limited to what the VM supports so far (see docs/vm.md rollout).

use ting::{Engine, run_source_engine};

fn run(engine: Engine, src: &str) -> Result<String, String> {
    let mut out = Vec::new();
    let r = run_source_engine(engine, "diff", src, &mut out, Vec::new());
    let stdout = String::from_utf8(out).unwrap();
    match r {
        Ok(()) => Ok(stdout),
        Err(diag) => Err(format!("{stdout}--\n{diag}")),
    }
}

#[track_caller]
fn same(src: &str) {
    let a = run(Engine::Eval, src);
    let b = run(Engine::Vm, src);
    assert_eq!(a, b, "engines diverge on:\n{src}");
}

#[test]
fn expressions_match_across_engines() {
    let corpus: &[&str] = &[
        // arithmetic, precedence, promotion, overflow, div by zero
        "print(1 + 2 * 3 - 4, 7 / 2, 7 % 2, 7.0 / 2, 1 + 0.5, -5);",
        "print(9223372036854775807 + 1);",
        "print(1 / 0);",
        "print(1.0 / 0.0);",
        // strings/lists/maps, concat, structural equality, indexing
        "print(\"foo\" + \"bar\", [1] + [2, 3], [1, [2]] == [1, [2]]);",
        "let xs = [10, 20, 30]; print(xs[0], xs[-1], len(xs));",
        "let m = {\"b\": 2, \"a\": 1}; m[\"c\"] = 3; print(m, keys(m), has(m, \"a\"));",
        "let s = \"héllo\"; print(s[1], s[-1], len(s));",
        "print([1,2,3][5]);",
        "print({\"a\": 1}[\"z\"]);",
        "print({1: 2});",
        // comparisons and equality
        "print(1 < 1.5, \"a\" < \"b\", 1 == 1.0, 1 == \"1\", nil == nil);",
        // strict short-circuit logic
        "print(true && false, false || true, !true);",
        "print(false && 1 / 0 == 0, true || 1 / 0 == 0);",
        "print(1 && true);",
        "print(true && 1);",
        "print(false || \"x\");",
        "print(!0);",
        // variables
        "let x = 1; x = x + 41; print(x);",
        "y = 1;",
        "print(nope);",
        // index assignment
        "let xs = [1, 2]; xs[1] = 9; print(xs);",
        "let xs = [1]; xs[5] = 0;",
        "let n = 1; n[0] = 2;",
        // builtins through Call, including errors
        "print(sort([3, 1, 2]), sort_by([\"bbb\", \"a\"], len));",
        "print(format(\"{} and {}\", 1, [2]));",
        "print(json_parse(\"[1, 2.5, null]\"), json_str({\"a\": [true]}));",
        "print(len());",
        "print(pop([]));",
        "print(min([1, \"a\"]));",
        "print(upper(\"héllo\"), slice(\"hello\", 1, -1), abs(-4));",
        // callee is not callable
        "print(3(1));",
        // nesting
        "print([{\"k\": [1, 2]}][0][\"k\"][-1]);",
    ];
    for src in corpus {
        same(src);
    }
}

#[test]
fn vm_reports_unsupported_constructs() {
    let err = run(Engine::Vm, "if true { print(1); }").unwrap_err();
    assert!(err.contains("not yet supported by --vm"), "{err}");
    // The reference engine still runs them.
    assert_eq!(run(Engine::Eval, "if true { print(1); }").unwrap(), "1\n");
}
