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

/// A failed assertion says what the two sides came out as, and both
/// engines say it in the same words — the tree-walker keeps the pair
/// where it evaluates the argument, the VM from an opcode of its own,
/// and nothing but this test stands between those two paths and a
/// quiet divergence.
#[test]
fn a_failed_assertion_shows_the_same_values_on_both_engines() {
    let cases: &[&str] = &[
        "assert(1 == 2);",
        "assert(1 == 2, \"named\");",
        "assert([1, 2, 3] == [1, 2, 4], \"a list\");",
        "assert({\"a\": 1} == {\"a\": 2}, \"a map\");",
        "assert(len(\"abc\") < 2, \"too short\");",
        "assert(nil == false, \"nil\");",
        "assert(\"x\" != \"x\", \"the same\");",
        "assert(1.5 >= 2.0, \"floats\");",
        // Not a comparison: nothing to show, and nothing stale from
        // the assert before it either.
        "assert(1 == 2, \"first\");",
        "assert(has({\"a\": 1}, \"b\"), \"no comparison\");",
        // Shadowed: the pair is recorded and never read.
        "fn assert(c, m) { print(m); } assert(1 == 2, \"mine\");",
        // A value too long to print in full is cut the same way twice.
        "assert(\"the quick brown fox jumps over the lazy dog and then some\" == \"x\");",
        // Nested: the inner one fails first, and inside a try() the
        // outer still reports its own sides.
        "print(try(fn() { assert(1 == 2, \"inner\"); })[\"err\"]); assert(3 == 4, \"outer\");",
    ];
    for src in cases {
        same(src);
    }
    // And the message really does carry the values, on both.
    for engine in [Engine::Eval, Engine::Vm] {
        let err = run(engine, "assert(1 == 2, \"two\");").unwrap_err();
        assert!(
            err.contains("assertion failed: two (1 == 2)"),
            "{engine:?}: {err}"
        );
    }
}

/// Taking a key out of a map is the first thing `pop` does to
/// something that is not a list, so every way of getting it wrong has
/// to read the same on both engines — including the "did you mean"
/// that reading a missing key already gave.
#[test]
fn a_fingerprint_reads_the_same_on_both_engines() {
    let cases: &[&str] = &[
        // Equal values, one key: == says 1 == 1.0 at every depth.
        "print(fingerprint(1) == fingerprint(1.0), fingerprint([1]) == fingerprint([1.0]));",
        "print(fingerprint({\"a\": 1}) == fingerprint({\"a\": 1.0}));",
        "print(fingerprint(0.0) == fingerprint(-0.0));",
        // Different values, different keys.
        "print(fingerprint(1) == fingerprint(\"1\"), fingerprint(true) == fingerprint(\"true\"));",
        "print(fingerprint([1, \"1\"]) == fingerprint([\"1\", 1]));",
        "print(fingerprint([\"ab\"]) == fingerprint([\"a\", \"b\"]));",
        // Everything it refuses.
        "print(fingerprint(print), fingerprint(fn(x) { return x; }), fingerprint([1, print]));",
        "print(fingerprint(9007199254740992) == nil, fingerprint(9007199254740993));",
        "print(fingerprint(0.0 / 0.0));",
        "let c = [1]; push(c, c); print(fingerprint(c));",
        "let m = {\"a\": 1}; m[\"m\"] = m; print(fingerprint(m));",
        "print(try(fingerprint)[\"err\"], try(fingerprint, 1, 2)[\"err\"]);",
    ];
    for src in cases {
        same(src);
    }
}

#[test]
fn taking_a_key_out_of_a_map_reads_the_same_on_both_engines() {
    let cases: &[&str] = &[
        "let m = {\"a\": 1, \"b\": 2}; print(pop(m, \"a\"), m, len(m));",
        // The value comes back whatever it is, the map keeps the rest.
        "let m = {\"a\": [1, 2], \"b\": {\"c\": 3}}; print(pop(m, \"b\"), m);",
        "let m = {\"a\": nil}; print(pop(m, \"a\"), has(m, \"a\"), len(m));",
        // Emptied and refilled: the map is the same map throughout.
        "let m = {\"a\": 1}; pop(m, \"a\"); m[\"a\"] = 2; print(m);",
        // Through a binding: it is in place, so the alias sees it.
        "let m = {\"a\": 1}; let n = m; pop(n, \"a\"); print(m, n);",
        // Every refusal.
        "let m = {\"alpha\": 1}; print(try(pop, m, \"alhpa\")[\"err\"]);",
        "let m = {\"a\": 1}; print(try(pop, m)[\"err\"]);",
        "print(try(pop, [1, 2], \"x\")[\"err\"]);",
        "let m = {\"a\": 1}; print(try(pop, m, 1)[\"err\"]);",
        "print(try(pop, \"abc\")[\"err\"]);",
        "print(try(pop, [])[\"err\"]);",
        // The list side is untouched.
        "let xs = [1, 2, 3]; print(pop(xs), xs);",
        // Draining in a loop, which is what the milestone is for.
        "let m = {}; let i = 0; while i < 50 { m[str(i)] = i * i; i += 1; } \
         let s = 0; i = 0; while i < 50 { s += pop(m, str(i)); i += 1; } print(s, len(m));",
    ];
    for src in cases {
        same(src);
    }
    // A map key really is gone, not merely nil: `m[k] = nil` was the
    // only spelling before, and it stores nil and keeps the key (915).
    for engine in [Engine::Eval, Engine::Vm] {
        let out = run(
            engine,
            "let m = {\"a\": 1}; m[\"a\"] = nil; print(len(m), has(m, \"a\")); \
             pop(m, \"a\"); print(len(m), has(m, \"a\"));",
        )
        .unwrap();
        assert_eq!(out, "1 true\n0 false\n", "{engine:?}");
    }
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
        "let m = {\"a\": 1}; print(get(m, \"a\", 0), get(m, \"z\", 0));",
        "print(get([1, 2, 3], 0, -1), get([1, 2, 3], -1, -1), get([1, 2, 3], 9, -1));",
        "print(get(\"abc\", 1, \"?\"), get(\"abc\", 9, \"?\"));",
        "let c = {}; for w in [\"a\", \"b\", \"a\"] { c[w] = get(c, w, 0) + 1; } print(c);",
        "print(try(get, [1], \"k\", 0)[\"err\"]);",
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
        // compound assignment: the arithmetic, the write into a
        // container, and the failures on either half.
        "let i = 7; i += 3; i -= 2; i *= 4; i /= 3; i %= 4; print(i);",
        "let s = \"a\"; s += \"b\"; print(s);",
        "let xs = [1, 2]; xs[0] += 10; xs[-1] *= 3; print(xs);",
        "fn k() { print(\"k\"); return \"n\"; } let m = {\"n\": 1}; m[k()] += 1; print(m);",
        "fn f() { let n = 0; n += 1; return n; } print(f(), f());",
        "nope += 1;",
        "let m = {}; m[\"k\"] += 1;",
        "let s = \"a\"; s -= 1;",
        "let xs = [1]; xs[9] += 1;",
        // try with arguments: the callee's own failures, and try's.
        "print(try(int, \"7\"), try(int, \"x\")[\"err\"]);",
        "fn add(a, b) { return a + b; } print(try(add, 1, 2), try(add, 1)[\"err\"]);",
        "fn add(a, b) { return a + b; } print(try(add, ...[3, 4]));",
        "try();",
        "try(1, 2);",
        // A frame's arguments reach the diagnostic, and both engines
        // must render them the same way.
        "fn f(a, b) { return a + b; } f(1, \"x\");",
        "fn f() { fail(\"x\"); } f();",
        "fn f(a, b = 2, ...r) { fail(\"x\"); } f(1, 2, 3, 4);",
        "fn f(a, b, c, d, e) { fail(\"x\"); } f(1, 2, 3, 4, 5);",
        "fn f(g) { return g(); } f(fn() { return nosuch; });",
        "fn f(xs) { return xs[9]; } f(range(0, 40));",
        // try's trace carries the same arguments, as values.
        "fn f(a, b) { return a / b; } let r = try(f, 1, 0); print(r[\"trace\"][0][\"args\"]);",
        "fn f() { fail(\"x\"); } print(try(f)[\"trace\"][0][\"args\"]);",
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
        // 830/831: both engines count through `for x in range(...)`
        // instead of building the list, and both decide that at RUN
        // TIME — a bound `range` has to win in each.
        "for i in range(2, 9, 3) { print(i); }",
        "for i in range(3, 0, -1) { print(i); }",
        "for i in range(5, 5) { print(i); }",
        "fn f() { let range = fn(_n) { return [\"a\", \"b\"]; }; \
          for i in range(3) { print(i); } } f();",
        "fn range(n) { return [n]; } for i in range(7) { print(i); }",
        "for i in range(1, 2, 0) { print(i); }",
        "for i in range(\"a\") { print(i); }",
        "for i in range() { print(i); }",
        "let a = [0, 3]; for i in range(...a) { print(i); }",
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
        // A near-miss on a local: the VM keeps those in frame slots,
        // where they have no name at runtime, and must still offer the
        // one the tree-walker offers from its environment.
        "fn f() { let amount = 1; return amount + amonut; } print(f());",
        "fn f() { let amount = 1; amonut = 2; return amount; } print(f());",
        // ...and must not offer one that is out of scope at the point
        // it failed, which the environment would never have held.
        "fn f() { if false { let volume = 1; print(volume); } return volme; } print(f());",
        "fn f(amount) { return amonut; } print(f(1));",
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

/// Since v2.112.0 the VM compiles what a script imports, so a module
/// runs on the engine that imported it rather than always on the
/// tree-walker. That makes every module a second place the two can
/// disagree, and the standard library is the one every program uses.
#[test]
fn imported_modules_match_across_engines() {
    let corpus: &[&str] = &[
        // Exports come out of the environment a module ran in, and a
        // frame slot holds no name: a module whose top level took
        // slots would export nils.
        "let l = import(\"lib/list.ting\"); print(l[\"sum\"]([1, 2, 3]));",
        "let l = import(\"lib/list.ting\"); print(len(keys(l)) > 0);",
        // A module's own top-level state, read back through a function.
        "let m = import(\"lib/math.ting\"); print(m[\"clamp\"](9, 0, 5));",
        // Recursion and a caller-supplied closure crossing into it.
        "let l = import(\"lib/list.ting\"); \
         print(l[\"sort_with\"]([3, 1, 2], fn(p, q) { return q - p; }));",
        // A builtin the module shadows must not leak back out.
        "let s = import(\"lib/string.ting\"); print(s[\"words\"](\"a b\"), len(\"xy\"));",
        // Two modules, one importing the other, and the cache between.
        "let a = import(\"lib/list.ting\"); let b = import(\"lib/list.ting\"); \
         print(a[\"sum\"]([1]) + b[\"sum\"]([2]));",
        // Errors raised inside a module point into the module's file.
        "let l = import(\"lib/list.ting\"); print(l[\"mean\"]([]));",
        "let l = import(\"lib/list.ting\"); print(try(fn() { return l[\"mean\"]([]); })[\"err\"]);",
        // A module off the filesystem rather than the embedded stdlib,
        // whose functions close over its own top-level binding.
        "let g = import(\"tests/fixtures/good.ting\"); print(g[\"scaled\"](4), g[\"base\"]);",
        "let g = import(\"tests/fixtures/good.ting\"); \
         print(g[\"twice\"](g[\"scaled\"], 1));",
        // A module that does not exist, and a member that does not.
        "print(try(fn() { return import(\"lib/nope.ting\"); })[\"err\"]);",
        "let l = import(\"lib/list.ting\"); print(get(l, \"nope\", \"absent\"));",
    ];
    for src in corpus {
        same(src);
    }
}

/// The accepted divergence of docs/vm.md, now reaching import: the
/// message and span match, but a module the VM refuses to compile
/// never runs, so the statements before the bad one have no effect.
/// Both engines must still reject it, and say the same thing.
#[test]
fn a_broken_module_is_rejected_by_both_engines() {
    for (src, want) in [
        (
            "let m = import(\"tests/fixtures/broken_return.ting\"); print(m);",
            "return outside function",
        ),
        (
            "let m = import(\"tests/fixtures/broken_break.ting\"); print(m);",
            "break outside loop",
        ),
    ] {
        let a = run(Engine::Eval, src).unwrap_err();
        let b = run(Engine::Vm, src).unwrap_err();
        // Naming the error keeps this from passing on some other one:
        // a missing fixture would fail to import and agree about that.
        assert!(a.contains(want), "eval said something else:\n{a}");
        assert!(b.contains(want), "vm said something else:\n{b}");
        // Same diagnostic; the stdout before it is what may differ.
        let tail = |s: &str| s.split("--\n").nth(1).unwrap_or(s).to_string();
        assert_eq!(tail(&a), tail(&b), "diagnostics differ on:\n{src}");
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
        // Every selftest, with none held out. fs.ting and sh.ting used
        // to be, because this test reruns them in-process while the
        // test above runs them in parallel and fs.ting's tree had one
        // fixed name; the fixtures name themselves uniquely now, so
        // the two runs cannot reach each other's files. Do not put a
        // skip back here without fixing the fixture instead.
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

/// A module exports what its top level declares, and `let sort = sort;`
/// is a declaration. The rule this replaced asked the environment
/// which of its names looked module-defined, and answered by value:
/// a builtin still bound to its own name was ambient. That is right
/// for the ones a module never touches and wrong for one it rebinds
/// on purpose, so the name simply vanished from the module map.
#[test]
fn a_module_can_re_export_a_builtin() {
    let src = "let m = import(\"tests/fixtures/reexport.ting\");\n\
               print(m[\"sort\"]([3, 1, 2]));\n\
               print(m[\"named\"]([1, 2]));\n\
               print(m[\"own\"]([2, 1]));\n\
               print(has(m, \"push\"));";
    let want = "[1, 2, 3]\n2\n[1, 2]\nfalse\n";
    for engine in [Engine::Eval, Engine::Vm] {
        let got = run(engine, src).expect("the module imports");
        assert_eq!(got, want, "{engine:?} exported something else");
    }
}

/// `s += x` moves the string out of its binding and appends in place,
/// which is only sound when the right-hand side cannot reach the name
/// and only safe when the operator cannot fail. Both conditions are
/// invisible when they hold, so the cases that decide them are pinned
/// here: without them the optimisation could silently start reading a
/// binding it had already emptied.
#[test]
fn a_compound_append_does_not_disturb_what_it_appends_to() {
    let cases = [
        // The right-hand side is the name itself.
        ("let s = \"a\"; s += s; print(s);", "aa\n"),
        // The right-hand side writes the name, so the old value is the
        // one the statement began with, not the one the call left.
        (
            "let t = \"a\"; fn f() { t = \"z\"; return \"b\"; } t += f(); print(t);",
            "ab\n",
        ),
        // A failed operator leaves the binding as it found it.
        (
            "let u = \"a\"; print(try(fn() { u += 1; })[\"err\"] != nil); print(u);",
            "true\na\n",
        ),
        (
            "let n = 9223372036854775807; print(try(fn() { n += 1; })[\"err\"] != nil); print(n);",
            "true\n9223372036854775807\n",
        ),
        // Everything else the operator does is untouched.
        ("let n = 1; n += 2; print(n);", "3\n"),
        ("let xs = [1]; xs += [2]; print(xs);", "[1, 2]\n"),
        (
            "let m = {\"k\": \"a\"}; m[\"k\"] += \"b\"; print(m);",
            "{\"k\": \"ab\"}\n",
        ),
        // A name that is not bound fails before the right-hand side runs.
        (
            "fn f() { print(\"ran\"); return 1; } print(try(fn() { nope += f(); })[\"err\"] != nil);",
            "true\n",
        ),
        // The long spelling is the same statement and answers the same.
        ("let a = \"a\"; a = a + a; print(a);", "aa\n"),
        (
            "let b = \"a\"; fn g() { b = \"z\"; return \"b\"; } b = b + g(); print(b);",
            "ab\n",
        ),
        ("let c = [1]; c = [0] + c; print(c);", "[0, 1]\n"),
        (
            "fn h() { print(\"ran\"); return 1; } print(try(fn() { gone = gone + h(); })[\"err\"] != nil);",
            "true\n",
        ),
        // A call on the right is allowed for a frame slot, where no
        // closure can name the binding. What the call answers is still
        // whatever the script bound the name to.
        (
            "fn f() { let str = fn(x) { return \"!\" + upper(x); }; let s = \"\"; s += str(\"a\"); return s; } print(f());",
            "!A\n",
        ),
        (
            "fn f() { let s = \"a\"; let read = fn() { return s; }; s += \"b\"; return s + \"/\" + read(); } print(f());",
            "ab/ab\n",
        ),
        (
            "fn f() { let s = \"a\"; let e = try(fn() { s += str(fail(\"no\")); })[\"err\"]; return e + \"/\" + s; } print(f());",
            "no/a\n",
        ),
    ];
    for (src, want) in cases {
        for engine in [Engine::Eval, Engine::Vm] {
            let got = run(engine, src).expect("the program runs");
            assert_eq!(got, want, "{engine:?} on:\n{src}");
        }
    }
}

/// A long flat expression is deep for everything downstream of the
/// parser: an operator chain leans left, so the compiler and the
/// tree-walker each spent one host frame per term. 854 measured the
/// tree-walker dying between 50000 and 100000 terms in release —
/// while the VM ran the same program and printed an answer, which is
/// two engines disagreeing about what a program does. Both walk the
/// spine iteratively now, and this holds them to the same answer at a
/// length neither could have survived unoptimized.
#[test]
fn a_long_operator_chain_says_the_same_thing_on_both_engines() {
    for (n, op, want) in [(20_000usize, " + ", 20_000), (20_000, " - ", -20_000)] {
        let src = format!("let x = 0{};\nprint(x);\n", format!("{op}1").repeat(n));
        assert_eq!(run(Engine::Vm, &src), Ok(format!("{want}\n")));
        same(&src);
    }
    // Mixed precedence and a short-circuit in the middle: the spine
    // stops where the operators stop being plain, and the answer is
    // still the same on both sides.
    let mixed = format!(
        "let a = 2;\nlet x = 1{}{};\nprint(x);\n",
        " + a * 2".repeat(5_000),
        " + 0"
    );
    same(&mixed);
    let short = format!("let x = true{};\nprint(x);\n", " && true".repeat(2_000));
    same(&short);
}
