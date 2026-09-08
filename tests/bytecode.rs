//! What the compiler emits, where the shape of the bytecode is the
//! point rather than the answer it computes. A superinstruction that
//! quietly stops being emitted costs nothing but speed, so nothing
//! else in the suite would notice.

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
