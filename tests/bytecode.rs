//! What the compiler emits, where the shape of the bytecode is the
//! point rather than the answer it computes. A superinstruction that
//! quietly stops being emitted costs nothing but speed, so nothing
//! else in the suite would notice.

mod common;

use ting::compile::Op;

/// Every instruction in the program, function bodies included: each
/// `fn` compiles to a chunk of its own, and most code lives in one.
fn ops(src: &str) -> Vec<Op> {
    let tokens = ting::lexer::lex(src).expect("lexes");
    let program = ting::parser::parse_program(&tokens).expect("parses");
    let chunk = match ting::compile::compile_program(&program) {
        Ok(chunk) => chunk,
        Err(e) => panic!("compiles: {}", e.message),
    };
    fn gather(chunk: &ting::compile::Chunk, out: &mut Vec<Op>) {
        out.extend(chunk.code.iter().cloned());
        for proto in &chunk.protos {
            gather(&proto.chunk, out);
        }
    }
    let mut out = Vec::new();
    gather(&chunk, &mut out);
    out
}

fn count(src: &str, f: impl Fn(&Op) -> bool) -> usize {
    ops(src).iter().filter(|op| f(op)).count()
}

#[test]
fn a_local_compared_with_a_literal_is_one_instruction() {
    // The commonest three instructions in every program measured
    // (LOG 800): read a local, push a constant, apply the operator.
    // Neither half can fail, so nothing can be observed between them.
    for src in [
        "let c = \"x\"; let hit = c == \",\";",
        "let s = \"a\"; let t = s + \"!\";",
        "let v = 1; let w = v * 2;",
        "let z = nil; let y = z == nil;",
    ] {
        assert_eq!(
            count(src, |op| matches!(op, Op::BinarySlotConst(..))),
            1,
            "not fused: {src}"
        );
        assert_eq!(
            count(src, |op| matches!(op, Op::Binary(_))),
            0,
            "left over: {src}"
        );
    }
}

#[test]
fn two_locals_are_one_instruction_too() {
    // `i < n` is in every loop there is, and it is the whole
    // difference between seven instructions an iteration and five.
    for src in [
        "let a = 1; let b = 2; let c = a + b;",
        "fn f(x, y) { return x * y; } f(2, 3);",
    ] {
        assert_eq!(
            count(src, |op| matches!(op, Op::BinarySlots(..))),
            1,
            "not fused: {src}"
        );
        assert_eq!(
            count(src, |op| matches!(op, Op::Binary(_))),
            0,
            "left over: {src}"
        );
    }
}

#[test]
fn a_condition_is_the_test_and_the_branch_in_one() {
    // In an `if` or a `while` the answer is looked at where it is
    // made: it never reaches the stack, and no separate instruction
    // pops it to ask whether it is true.
    for (src, fused) in [
        (
            "let c = \"x\"; if c == \",\" { print(1); }",
            "JumpIfFalseSlotConst",
        ),
        (
            "let i = 0; let n = 5; while i < n { i += 1; }",
            "JumpIfFalseSlots",
        ),
    ] {
        let n = match fused {
            "JumpIfFalseSlotConst" => count(src, |op| matches!(op, Op::JumpIfFalseSlotConst(..))),
            _ => count(src, |op| matches!(op, Op::JumpIfFalseSlots(..))),
        };
        assert_eq!(n, 1, "not fused: {src}");
        assert_eq!(
            count(src, |op| matches!(op, Op::JumpIfFalse(_))),
            0,
            "left over: {src}"
        );
    }
    // A condition that is not a comparison against a local keeps the
    // separate test and branch.
    for src in [
        "let a = true; if a { print(1); }",
        "fn f() { return true; } if f() { print(1); }",
        "let a = 1; let b = 2; if a < b && a > 0 { print(1); }",
    ] {
        assert_eq!(
            count(src, |op| matches!(
                op,
                Op::JumpIfFalseSlotConst(..) | Op::JumpIfFalseSlots(..)
            )),
            0,
            "wrongly fused: {src}"
        );
    }
}

#[test]
fn nothing_else_is_folded_into_the_instruction() {
    // Only a literal may ride along. Anything that has to be run
    // could fail or have an effect, and the order it happens in is
    // part of the language.
    for src in [
        "fn f() { return 1; } let a = 1; let c = a < f();",
        "let a = 1; let c = a < [1][0];",
        "let a = 1; let c = 3 < a;",
        "let a = 1; let c = [a][0] < a;",
    ] {
        assert_eq!(
            count(src, |op| matches!(
                op,
                Op::BinarySlotConst(..) | Op::BinarySlots(..)
            )),
            0,
            "wrongly fused: {src}"
        );
    }
}

/// 829 measured `for i in range(10000000)` at 307 MB against 3 MB for
/// the same loop written with `while`, and `range(100000000000)` was
/// OOM-killed — exit 137, no message, no line. The counting loop is
/// emitted for the shape that causes it, and only for that shape.
#[test]
fn a_for_over_range_counts_instead_of_building_a_list() {
    let started = |src: &str| count(src, |o| matches!(o, Op::IterStart(_)));
    let snapshot = |src: &str| count(src, |o| matches!(o, Op::IterNew));

    for src in [
        "for i in range(3) { print(i); }",
        "for i in range(2, 5) { print(i); }",
        "for i in range(9, 0, -1) { print(i); }",
        "fn f(n) { for i in range(n) { print(i); } }",
    ] {
        assert_eq!(started(src), 1, "{src}");
        assert_eq!(snapshot(src), 0, "{src}");
    }

    // Everything else keeps the snapshot: another iterable, another
    // call, a wrong argument count, and a spread — which makes the
    // count a runtime fact.
    for src in [
        "for x in [1, 2] { print(x); }",
        "for c in \"ab\" { print(c); }",
        "for x in sort([2, 1]) { print(x); }",
        "for i in range() { print(i); }",
        "for i in range(1, 2, 3, 4) { print(i); }",
        "let a = [1, 3]; for i in range(...a) { print(i); }",
    ] {
        assert_eq!(started(src), 0, "{src}");
        assert_eq!(snapshot(src), 1, "{src}");
    }

    // The loop owns three stack slots either way, so the two shapes
    // clean up identically — counted against each other rather than
    // against a number, since the body pops too.
    let pops = |src: &str| count(src, |o| matches!(o, Op::Pop));
    assert_eq!(
        pops("for i in range(3) { print(i); }"),
        pops("for x in [1] { print(x); }")
    );
}

/// The constant and name pools are found by lookup, not by scanning
/// them. They used to be searched linearly under a comment saying
/// they stay tiny: true of every program in the corpus, false of a
/// generated one, and 843 measured 8000 functions costing 650 ms to
/// compile against 80 ms to run on the tree-walker.
///
/// A ratio rather than a number, best of three at each size, as the
/// checker's guard is: doubling the input can only double linear
/// work, and a scan per entry lands near 4.
#[test]
fn the_pools_are_not_searched_by_scanning_them() {
    // Distinct literals, few names: the pools grow with n while the
    // resolver's scopes do not, so this measures the pools alone.
    // One statement per literal rather than one long sum, because a
    // chain of `+` is a left-leaning tree and the compiler walks it
    // by recursion: at 12000 terms the tree alone overflowed the test
    // thread, which measures the wrong thing loudly.
    fn source(n: usize) -> String {
        let mut src = String::new();
        for i in 0..n {
            src.push_str(&format!("print(\"lit{i}\", {i});\n"));
        }
        src
    }
    // Keep the work, and pin what dedup means: one constant per
    // literal, and the pools are what grows with n.
    fn compile(program: &[ting::ast::Stmt], n: usize) {
        let Ok(chunk) = ting::compile::compile_program(program) else {
            panic!("compile failed");
        };
        assert!(
            chunk.consts.len() >= 2 * n,
            "constants went missing: {}",
            chunk.consts.len()
        );
    }
    fn parsed(src: &str) -> Vec<ting::ast::Stmt> {
        let tokens = ting::lexer::lex(src).expect("lex");
        ting::parser::parse_program(&tokens).expect("parse")
    }
    // Three times 845's sizes: at 1500 the small measurement was five
    // milliseconds, and on a shared runner a co-tenant is worth more
    // than that. Not larger than this — the mutation these numbers
    // exist to catch is quadratic, so ten times the size is a hundred
    // times the failing run, and a guard nobody waits for is not a
    // guard.
    let (small, large) = (parsed(&source(5000)), parsed(&source(10000)));
    let ratio =
        common::doubling_ratio_under(3.0, || compile(&small, 5000), || compile(&large, 10000));
    assert!(
        ratio < 3.0,
        "doubling the program multiplied compilation by {ratio:.1}: \
         the pool scan is back"
    );
}

/// The resolver finds a name by lookup and notes the scope by
/// reference, not by walking or copying it. 845 measured this shape —
/// many functions, many calls, so the top level's own resolver holds
/// every binding — at a ratio of 3.9 with the pools already indexed.
#[test]
fn the_resolver_does_not_walk_the_scope_per_name() {
    fn source(n: usize) -> String {
        let mut src = String::new();
        for i in 0..n {
            src.push_str(&format!("fn f{i}(a) {{ return a + {i}; }}\n"));
        }
        for i in 0..n {
            src.push_str(&format!("print(f{i}(1));\n"));
        }
        src
    }
    fn compile(program: &[ting::ast::Stmt], n: usize) {
        let Ok(chunk) = ting::compile::compile_program(program) else {
            panic!("compile failed");
        };
        assert!(chunk.names.len() >= n, "names went missing");
    }
    fn parsed(src: &str) -> Vec<ting::ast::Stmt> {
        let tokens = ting::lexer::lex(src).expect("lex");
        ting::parser::parse_program(&tokens).expect("parse")
    }
    let (small, large) = (parsed(&source(1500)), parsed(&source(3000)));
    let ratio =
        common::doubling_ratio_under(3.0, || compile(&small, 1500), || compile(&large, 3000));
    assert!(
        ratio < 3.0,
        "doubling the program multiplied compilation by {ratio:.1}: \
         the scope walk is back"
    );
}
