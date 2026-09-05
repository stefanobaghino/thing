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
    "&",
    "|",
    "^",
    "~",
    "<<",
    ">>",
    "!",
    "=",
    "+=",
    "-=",
    "*=",
    "/=",
    "%=",
    "...",
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
    "get",
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
    "re_test",
    "re_find",
    "re_find_all",
    "re_replace",
    "re_split",
    "\"[a-z]+(x|y)?\"",
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
fn cyclic_values_never_panic() {
    // The generator never builds a cycle, so print, str, ==, json_str
    // and the pretty encoder get one on purpose: each must return a
    // value or an error, never unwind or overflow.
    let srcs = [
        "let a = [1]; push(a, a); print(a); print(str(a));",
        "let a = [1]; push(a, a); let b = [1]; push(b, b); print(a == b, a != b, a == [1]);",
        "let a = [1]; push(a, a); print(try(fn() { return json_str(a); }));",
        "let m = {\"k\": 1}; m[\"me\"] = m; print(m); print(try(fn() { return json_str(m, 2); }));",
        "let a = [1]; push(a, a); print(contains([a], a), find([0, a], a));",
    ];
    for src in srcs {
        let outcome = catch_unwind(AssertUnwindSafe(|| exercise(src)));
        assert!(outcome.is_ok(), "panicked on {src}");
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

/// The pattern alphabet: the syntax the engine claims to accept, plus
/// enough punctuation to build patterns it must refuse. Nothing here
/// is filtered — a pattern that cannot compile is a fine outcome, a
/// panic or a hang is not.
const PATTERN_PIECES: &[&str] = &[
    "a", "b", "z", ".", "|", "*", "+", "?", "(", ")", "(?:", "[", "]", "[^", "-", "^", "$", "\\d",
    "\\w", "\\s", "\\D", "\\W", "\\S", "\\.", "\\\\", "{2}", "{1,3}", "{0,}", "{", "}", "*?", "+?",
    "??", "é", "\n",
];

fn pattern_soup(rng: &mut Rng) -> String {
    let len = 1 + rng.below(12);
    let mut out = String::new();
    for _ in 0..len {
        out.push_str(PATTERN_PIECES[rng.below(PATTERN_PIECES.len())]);
    }
    out
}

fn subject_soup(rng: &mut Rng) -> String {
    let alphabet = ["a", "b", "z", "1", " ", "\n", "é", "-", "."];
    let len = rng.below(40);
    let mut out = String::new();
    for _ in 0..len {
        out.push_str(alphabet[rng.below(alphabet.len())]);
    }
    out
}

/// Random patterns on random subjects: compiling may fail, matching
/// may find nothing, but nothing may panic — and nothing may take
/// long, which is the property the Pike VM exists to provide. A
/// backtracking engine would sit in this test until the runner gave
/// up.
#[test]
fn random_patterns_never_panic_and_never_hang() {
    let seed: u64 = std::env::var("TING_RE_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let cases: u64 = std::env::var("TING_RE_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20_000);
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let started = std::time::Instant::now();
    for case in 0..cases {
        let pattern = pattern_soup(&mut rng);
        let subject = subject_soup(&mut rng);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(re) = ting::regex::Regex::new(&pattern) {
                let chars: Vec<char> = subject.chars().collect();
                let _ = re.find_at(&chars, 0);
            }
        }));
        assert!(
            outcome.is_ok(),
            "panicked on case {case} of seed {seed}:\n  pattern {pattern:?}\n  subject {subject:?}"
        );
    }
    // Generous by a wide margin: the whole point is that no pattern in
    // here can take exponential time, so a run that goes long has lost
    // the property rather than the race.
    let elapsed = started.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(60),
        "{cases} cases took {elapsed:?}, which no linear-time engine should"
    );
}

/// The pattern a backtracker dies on, at a size that makes the
/// difference unmistakable.
#[test]
fn the_classic_blowup_is_linear_here() {
    let re = ting::regex::Regex::new("(a+)+b").unwrap();
    for n in [100, 1000, 5000] {
        let text: Vec<char> = "a".repeat(n).chars().collect();
        let started = std::time::Instant::now();
        assert_eq!(re.find_at(&text, 0), None);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(30),
            "{n} characters took too long"
        );
    }
}
