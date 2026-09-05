//! Differential testing: the same program through both engines must
//! produce byte-identical stdout, or the same rendered error. Corpus
//! limited to what the VM supports so far (see docs/vm.md rollout).

mod common;

use common::Gen;
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
        // bit operations: precedence, sign, and the two refusals
        "print(0b1100 & 0b1010, 0b1100 | 0b1010, 0b1100 ^ 0b1010, ~0, ~5);",
        "print(1 << 10, -16 >> 2, 1 << 2 + 1, 7 & 3 | 8, 1 | 2 ^ 3 & 4, 0xff & 0x0f == 0x0f);",
        "print(1 << 64);",
        "print(1.5 & 2);",
        "print(~1.5);",
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
        // defaults: filled at the call, seeing earlier parameters
        "fn f(a, b = a * 2) { return [a, b]; } print(f(3), f(3, 9));",
        "fn f(x, xs = []) { push(xs, x); return xs; } print(f(1), f(2));",
        "fn f(a, b = 1) { return a; } print(f());",
        "fn outer(n = 2) { let g = fn(m = n + 1) { return m; }; return g(); } print(outer(), outer(10));",
        "fn f(a = fail(\"no\")) { return a; } print(f());",
        // rest parameters and spreads: what is left over, what is
        // forwarded, and the two refusals
        "fn r(a, ...rest) { return [a, rest]; } print(r(1), r(1, 2, 3));",
        "fn r(...xs) { push(xs, 0); return xs; } print(r(1), r(2));",
        "fn add(a, b) { return a + b; } fn f(...xs) { return add(...xs); } print(f(1, 2));",
        "let xs = [1, 2]; print(0, ...xs, 3);",
        "print(...[]);",
        "print(...5);",
        "fn add(a, b) { return a + b; } print(add(...[1]));",
        "fn r(a, ...rest) { return a; } print(r());",
        "fn r(a, b = 2, ...rest) { return [a, b, rest]; } print(r(1), r(1, 9, 8, 7));",
        // patterns: the map a match returns, a scan, and a refusal
        "print(re_test(\"héllo\", \"l+o\"), re_find(\"a1\", \"([a-z])(\\\\d)\"));",
        "print(re_find_all(\"a1 b2\", \"\\\\w\\\\d\"), re_split(\"a1b\", \"\\\\d\"));",
        "print(re_replace(\"a1\", \"(a)(1)\", \"$2$1$$\"));",
        "print(re_find(\"x\", \"(a\"));",
        "print(re_replace(\"a\", \"a\", \"$3\"));",
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
fn control_flow_matches_across_engines() {
    let corpus: &[&str] = &[
        // if/else chains, strict conditions
        "if 1 < 2 { print(\"yes\"); } else { print(\"no\"); }",
        "let n = 7; if n % 15 == 0 { print(\"fb\"); } else if n % 3 == 0 { print(\"f\"); } else if n % 5 == 0 { print(\"b\"); } else { print(n); }",
        "if 1 { print(1); }",
        "if false { print(1); } else if \"x\" { print(2); }",
        // while, mutation, nested conditions
        "let i = 0; let total = 0; while i < 10 { i = i + 1; if i % 2 == 1 { continue; } total = total + i; } print(i, total);",
        "let i = 0; while true { i = i + 1; if i == 5 { break; } } print(i);",
        "while nil { }",
        // for over list/string/map; snapshot semantics; loop var scope
        "for x in [10, 20, 30] { print(x); }",
        "let out = []; for ch in \"héllo\" { push(out, ch); } print(join(out, \"-\"));",
        "for k in {\"b\": 2, \"a\": 1} { print(k); }",
        "let xs = [1, 2]; for x in xs { push(xs, x + 10); } print(xs);",
        "for x in 42 { print(x); }",
        "let x = \"outer\"; for x in [1] { } print(x);",
        // break/continue in nested loops
        "for i in range(3) { for j in range(3) { if j == 1 { break; } print(i, j); } }",
        "for i in range(5) { if i % 2 == 0 { continue; } print(i); }",
        // scoped blocks and shadowing
        "let y = 1; { let y = 2; print(y); } print(y);",
        "{ let z = 9; print(z); } print(z);",
        // break inside a scoped block inside a loop
        "let i = 0; while i < 5 { i = i + 1; { let t = i * 10; if t > 20 { break; } print(t); } } print(\"end\", i);",
        // runtime errors inside loops keep their spans
        "for i in range(3) { print(1 / (1 - i)); }",
    ];
    for src in corpus {
        same(src);
    }
}

#[test]
fn functions_match_across_engines() {
    let corpus: &[&str] = &[
        "fn add(a, b) { return a + b; } print(add(2, 40));",
        // closures share captured state; independent instances
        "fn mk() { let n = 0; fn t() { n = n + 1; return n; } return t; } \
         let a = mk(); let b = mk(); print(a(), a(), b());",
        // recursion + depth cap through try
        "fn fib(n) { if n < 2 { return n; } return fib(n - 1) + fib(n - 2); } print(fib(15));",
        "fn f() { return f(); } let r = try(f); print(has(r, \"err\"));",
        // higher-order builtins with fn literals
        "print(map([1, 2, 3], fn(x) { return x * x; }));",
        "print(sort_by([[2, \"b\"], [1, \"a\"]], fn(p) { return p[0]; }));",
        // try/fail round trip, arity errors, implicit nil
        "let r = try(fn() { fail(\"boom\"); }); print(r[\"err\"]);",
        "fn one(a) { return a; } print(one(1, 2));",
        "fn nothing() { } print(nothing());",
        // loop-variable capture per iteration
        "let fs = []; for x in range(3) { push(fs, fn() { return x; }); } \
         print(fs[0](), fs[1](), fs[2]());",
        // fn value display + identity equality
        "fn g(x) { return x; } print(g == g, g);",
    ];
    for src in corpus {
        same(src);
    }
}

#[test]
fn selftest_programs_match_across_engines() {
    // The whole self-hosted suite through both engines: silent success
    // on each, byte-identical otherwise.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("selftest");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("selftest/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ting") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap();
        let p = path.to_str().unwrap();
        let run_file = |engine: Engine| {
            let mut out = Vec::new();
            let r = run_source_engine(engine, p, &src, &mut out, Vec::new());
            (String::from_utf8(out).unwrap(), r)
        };
        let (out_a, res_a) = run_file(Engine::Eval);
        let (out_b, res_b) = run_file(Engine::Vm);
        assert_eq!(res_a.is_ok(), res_b.is_ok(), "{p} verdicts differ");
        assert_eq!(out_a, out_b, "{p} outputs differ");
        assert!(res_b.is_ok(), "{p} failed under vm: {:?}", res_b.err());
        checked += 1;
    }
    assert!(
        checked >= 8,
        "expected at least 8 selftests, found {checked}"
    );
}

#[test]
fn generated_programs_match_across_engines() {
    // Grammar-directed differential fuzzing: random *valid* programs
    // built structurally (token soup almost never parses). Everything
    // terminates by construction: while loops use a fresh strictly
    // increasing counter, for only iterates small literals. Runtime
    // errors are fine as long as both engines agree byte-for-byte.
    // TING_DIFF_SEED / TING_DIFF_CASES let a sweep run bigger or on a
    // fresh seed without editing this file; CI uses the defaults.
    let seed = std::env::var("TING_DIFF_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0xF00D);
    let cases = std::env::var("TING_DIFF_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(800);
    let mut g = Gen::new(seed);
    for case in 0..cases {
        let src = g.program();
        let a = run(Engine::Eval, &src);
        let b = run(Engine::Vm, &src);
        assert_eq!(a, b, "engines diverge on case {case}:\n{src}");
    }
}

#[test]
fn vm_rejects_stray_return_at_compile_time() {
    // Accepted divergence (docs/vm.md): same message, surfaces earlier.
    let err = run(Engine::Vm, "return 1;").unwrap_err();
    assert!(err.contains("return outside function"), "{err}");
}

#[test]
fn vm_rejects_stray_break_at_compile_time() {
    // Accepted divergence (docs/vm.md): same message, surfaces earlier.
    let err = run(Engine::Vm, "if false { break; }").unwrap_err();
    assert!(err.contains("break outside loop"), "{err}");
}

/// Coverage is part of the shared semantics: the two engines must not
/// only compute the same values but agree on which lines they took to
/// get there. Run the self-hosted suite both ways and compare the
/// tables.
#[test]
fn both_engines_cover_the_same_lines() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("selftest");
    let mut files: Vec<(String, String)> = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("selftest/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ting") {
            continue;
        }
        // fs.ting builds a tree under a fixed name and sh.ting spawns
        // programs. The test above already runs both, in processes of
        // their own; running them again here, in this process and
        // twice over, races that on the one directory name.
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if matches!(name, "fs.ting" | "sh.ting") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("unreadable selftest");
        files.push((path.display().to_string(), src));
    }
    files.sort();
    let mut reports = Vec::new();
    for engine in [ting::Engine::Vm, ting::Engine::Eval] {
        let (result, report) = ting::run_covered(engine, &files, Vec::new());
        assert!(result.is_ok(), "{engine:?}: {result:?}");
        reports.push(report.expect("a covered run reports"));
    }
    assert_eq!(reports[0], reports[1], "engines cover different lines");
}
