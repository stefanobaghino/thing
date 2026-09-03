//! Deterministic fuzz-ish robustness tests: random inputs may error,
//! but must never panic the lexer, parser, or interpreter. Every case
//! is reproducible from its printed seed.

use std::panic::{AssertUnwindSafe, catch_unwind};

/// xorshift64* — tiny, deterministic, no dependencies.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const TOKENS: &[&str] = &[
    "let",
    "fn",
    "if",
    "else",
    "while",
    "for",
    "in",
    "break",
    "continue",
    "return",
    "true",
    "false",
    "nil",
    "x",
    "y",
    "f",
    "g",
    "(",
    ")",
    "{",
    "}",
    "[",
    "]",
    ",",
    ";",
    ":",
    "+",
    "-",
    "*",
    "/",
    "%",
    "==",
    "!=",
    "<",
    "<=",
    ">",
    ">=",
    "&&",
    "||",
    "!",
    "=",
    "0",
    "1",
    "2",
    "42",
    "1.5",
    "\"s\"",
    "\"\"",
    "print",
    "len",
    "range",
    "push",
    "sort",
    "try",
    "fail",
    // Every pure builtin (iteration 260 audit); I/O, blocking and
    // clock builtins stay out on purpose. The closure token feeds the
    // higher-order ones.
    "abs",
    "assert",
    "ends_with",
    "filter",
    "find",
    "float",
    "format",
    "has",
    "int",
    "json_parse",
    "json_str",
    "keys",
    "lower",
    "map",
    "max",
    "min",
    "pop",
    "reduce",
    "replace",
    "slice",
    "sort_by",
    "split",
    "starts_with",
    "trim",
    "type",
    "upper",
    "fn(a) { return a; }",
];

fn token_soup(rng: &mut Rng) -> String {
    let len = 1 + rng.below(80);
    let mut src = String::new();
    for _ in 0..len {
        src.push_str(TOKENS[rng.below(TOKENS.len())]);
        src.push(' ');
    }
    src
}

/// Lex, parse, and (when it cannot loop forever) run; errors are fine,
/// panics are the failure being hunted.
fn exercise(src: &str) {
    let Ok(tokens) = ting::lexer::lex(src) else {
        return;
    };
    let Ok(program) = ting::parser::parse_program(&tokens) else {
        return;
    };
    // `while` is the only unbounded construct (for iterates snapshots,
    // recursion hits the depth cap); `exit` would terminate the test
    // process itself, and `import` can reach exit (or the filesystem)
    // through the imported module — programs mentioning any of these
    // only get parsed.
    if !src.contains("while") && !src.contains("exit") && !src.contains("import") {
        let mut interp = ting::eval::Interpreter::new(std::io::sink());
        let _ = interp.run(&program);
        // The bytecode path must be just as panic-free: compile errors
        // and runtime errors are fine, unwinding is not.
        if let Ok(chunk) = ting::compile::compile_program(&program) {
            let mut interp = ting::eval::Interpreter::new(std::io::sink());
            let _ = ting::vm::run_chunk(&mut interp, &chunk);
        }
    }
}

#[test]
fn token_soup_never_panics() {
    for seed in 1..=3000u64 {
        let src = token_soup(&mut Rng(seed));
        let outcome = catch_unwind(AssertUnwindSafe(|| exercise(&src)));
        assert!(outcome.is_ok(), "panicked on seed {seed}:\n{src}");
    }
}

#[test]
fn mutated_examples_never_panic() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let printable: Vec<char> = (' '..='~').chain(['\n', '"', '\\']).collect();
    let mut rng = Rng(0xDECAFBAD);
    for entry in std::fs::read_dir(&dir).expect("examples/ missing") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ting") {
            continue;
        }
        let original = std::fs::read_to_string(&path).unwrap();
        for i in 0..300 {
            let mut chars: Vec<char> = original.chars().collect();
            let at = rng.below(chars.len());
            chars[at] = printable[rng.below(printable.len())];
            let mutated: String = chars.into_iter().collect();
            let outcome = catch_unwind(AssertUnwindSafe(|| exercise(&mutated)));
            assert!(
                outcome.is_ok(),
                "panicked on {} mutation {i}:\n{mutated}",
                path.display()
            );
        }
    }
}

#[test]
fn deep_nesting_parses_or_errors_cleanly() {
    // Nested expressions recurse in the parser and evaluator; under the
    // test stack (RUST_MIN_STACK) a depth of 1000 must hold.
    for (open, close) in [("(", ")"), ("[", "]")] {
        let src = format!("let v = {}1{};", open.repeat(1000), close.repeat(1000));
        let outcome = catch_unwind(AssertUnwindSafe(|| exercise(&src)));
        assert!(outcome.is_ok(), "panicked on deep {open}{close} nesting");
    }
}
