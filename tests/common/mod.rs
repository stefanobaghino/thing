//! Shared test helpers: the deterministic RNG and the grammar-directed
//! program generator used by the differential and formatter fuzz
//! suites. One grammar, two invariants.
#![allow(dead_code)]

/// xorshift64* — same generator as tests/fuzz.rs.
pub struct Rng(pub u64);

const COMPOUND: &[&str] = &["+", "-", "*", "/", "%"];

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

pub struct Gen {
    pub rng: Rng,
    pub fresh: usize,
}

impl Gen {
    pub fn new(seed: u64) -> Self {
        Gen {
            rng: Rng(seed),
            fresh: 0,
        }
    }
}

impl Gen {
    pub fn program(&mut self) -> String {
        let mut out = String::from(
            "let a = 3; let b = -2; let s = \"ab\"; let xs = [1, 2, 3];\n\
             fn h(v) { return v + 1; }\n\
             fn g(v) { return str(v) + \"!\"; }\n\
             fn d(v, w = 1, u = w + 1) { return v + w + u; }\n\
             fn r(v, ...rest) { return [v, len(rest), rest]; }\n",
        );
        let n = 2 + self.rng.below(5);
        for _ in 0..n {
            out.push_str(&self.stmt(2));
            out.push('\n');
        }
        out
    }

    fn stmt(&mut self, depth: usize) -> String {
        if depth == 0 {
            return format!("print({});", self.expr(1));
        }
        match self.rng.below(12) {
            // Compound assignment, to a variable and into a list slot:
            // the operator is folded in, and the subscript is only
            // written once.
            10 => format!(
                "a {}= {};",
                COMPOUND[self.rng.below(COMPOUND.len())],
                self.expr(1)
            ),
            11 => format!(
                "xs[{}] {}= {};",
                self.expr(1),
                COMPOUND[self.rng.below(COMPOUND.len())],
                self.expr(1)
            ),
            0 => format!("let v{} = {};", self.rng.below(3), self.expr(2)),
            1 => format!("a = {};", self.expr(2)),
            2 => format!("print({}, {});", self.expr(2), self.expr(1)),
            3 => format!(
                "if {} {{ {} }} else {{ {} }}",
                self.expr(1),
                self.stmt(depth - 1),
                self.stmt(depth - 1)
            ),
            4 => format!(
                "for i in [{}, {}] {{ {} }}",
                self.expr(1),
                self.expr(1),
                self.stmt(depth - 1)
            ),
            5 => format!("{{ let inner = {}; print(inner); }}", self.expr(2)),
            6 => format!("xs[{}] = {};", self.expr(1), self.expr(1)),
            7 => {
                // Bounded while: a fresh counter strictly increases.
                self.fresh += 1;
                let c = format!("w{}", self.fresh);
                format!(
                    "let {c} = 0; while {c} < {} {{ {c} = {c} + 1; {} }}",
                    2 + self.rng.below(4),
                    self.stmt(depth - 1)
                )
            }
            8 => format!(
                "print(format(\"{{}}|{{}}\", {}, upper(str({}))));",
                self.expr(1),
                self.expr(1)
            ),
            _ => format!("print(try(fn() {{ return {}; }}));", self.expr(2)),
        }
    }

    fn expr(&mut self, depth: usize) -> String {
        if depth == 0 {
            return match self.rng.below(8) {
                0 => "1".into(),
                1 => "42".into(),
                2 => "1.5".into(),
                3 => "\"x\"".into(),
                4 => "true".into(),
                5 => "nil".into(),
                6 => "a".into(),
                _ => "b".into(),
            };
        }
        match self.rng.below(38) {
            0 => format!("({} + {})", self.expr(depth - 1), self.expr(depth - 1)),
            1 => format!("({} * {})", self.expr(depth - 1), self.expr(depth - 1)),
            2 => format!("({} / {})", self.expr(depth - 1), self.expr(depth - 1)),
            3 => format!("({} == {})", self.expr(depth - 1), self.expr(depth - 1)),
            4 => format!("({} < {})", self.expr(depth - 1), self.expr(depth - 1)),
            5 => format!("({} && {})", self.expr(depth - 1), self.expr(depth - 1)),
            6 => format!("[{}, {}]", self.expr(depth - 1), self.expr(depth - 1)),
            7 => format!("{{\"k\": {}}}", self.expr(depth - 1)),
            8 => format!("h({})", self.expr(depth - 1)),
            9 => format!("len(str({}))", self.expr(depth - 1)),
            10 => format!("xs[{}]", self.expr(depth - 1)),
            11 => format!("try(fn() {{ return {}; }})", self.expr(depth - 1)),
            12 => format!("g({})", self.expr(depth - 1)),
            13 => format!("slice(str({}), 0, 2)", self.expr(depth - 1)),
            14 => format!("find(str({}), \"1\")", self.expr(depth - 1)),
            15 => format!(
                "find([1, {}], {})",
                self.expr(depth - 1),
                self.expr(depth - 1)
            ),
            16 => format!(
                "range(0, len(str({})), {})",
                self.expr(depth - 1),
                // Step in [-2, 2] \ {0}: negative steps and empty spans
                // both get exercised.
                ["-2", "-1", "1", "2"][self.rng.below(4)]
            ),
            // String and list builtins (iteration 237 audit): str() and
            // literal wrappers keep every call well-typed for *some*
            // inputs while still letting type errors through, which
            // both engines must report identically.
            17 => format!("starts_with(str({}), \"1\")", self.expr(depth - 1)),
            18 => format!("ends_with(str({}), \"x\")", self.expr(depth - 1)),
            19 => format!("replace(str({}), \"1\", \"one\")", self.expr(depth - 1)),
            20 => format!("split(str({}), \"1\")", self.expr(depth - 1)),
            21 => format!("trim(format(\"  {{}} \", {}))", self.expr(depth - 1)),
            22 => format!("lower(upper(str({})))", self.expr(depth - 1)),
            23 => format!("max([1, {}])", self.expr(depth - 1)),
            24 => format!("type({})", self.expr(depth - 1)),
            25 => format!(
                "filter([1, {}], fn(e) {{ return e == 1; }})",
                self.expr(depth - 1)
            ),
            26 => format!(
                "reduce([1, {}], 0, fn(p, q) {{ return p + 1; }})",
                self.expr(depth - 1)
            ),
            // Optional arguments: the same call with each count, so a
            // default that misbehaves on one engine shows up as a
            // difference on the next line.
            27 => format!("d({})", self.expr(depth - 1)),
            28 => format!("d({}, 2)", self.expr(depth - 1)),
            29 => format!("d({}, 2, 3)", self.expr(depth - 1)),
            30 => format!(
                "(fn(p, q = {}) {{ return [p, q]; }})({})",
                self.expr(depth - 1),
                self.expr(depth - 1)
            ),
            // Rest parameters and spreads: the same call reached
            // directly, through a spread, and through a mix of the two,
            // so a leftover the engines split differently shows up.
            31 => format!("r({}, {}, 3)", self.expr(depth - 1), self.expr(depth - 1)),
            32 => format!("r(...[{}, {}])", self.expr(depth - 1), self.expr(depth - 1)),
            33 => format!(
                "r({}, ...xs, {})",
                self.expr(depth - 1),
                self.expr(depth - 1)
            ),
            34 => format!(
                "(fn(...p) {{ return len(p); }})({}, ...[{}])",
                self.expr(depth - 1),
                self.expr(depth - 1)
            ),
            35 => format!("try(fn() {{ return r(...{}); }})", self.expr(depth - 1)),
            // try with the arguments handed straight over, which is
            // the wrapper above without the wrapper.
            36 => format!("try(h, {})", self.expr(depth - 1)),
            _ => format!("-({})", self.expr(depth - 1)),
        }
    }
}

/// How much a doubling of the input multiplies the work, measured as
/// robustly as a shared runner allows.
///
/// The four guards that use this exist to catch a quadratic coming
/// back: broken scores near four, healthy near two, and the line is
/// drawn at three. That gap is wide enough for the work and too
/// narrow for the machine — 848 failed on ubuntu at 3.5 and 851 on
/// macOS at 3.8 with nothing regressed, both times because a
/// co-tenant landed on the larger size and not the smaller.
///
/// So the two sizes are timed ALTERNATELY, which puts a slow patch
/// of the machine on both; the best of five stands for each size;
/// and the whole measurement is repeated three times, keeping the
/// SMALLEST ratio. A quadratic scores four in every attempt, so the
/// minimum still catches it, while noise now has to strike all three.
pub fn doubling_ratio(mut small: impl FnMut(), mut large: impl FnMut()) -> f64 {
    let mut best = f64::INFINITY;
    for _ in 0..3 {
        let mut small_best = std::time::Duration::MAX;
        let mut large_best = std::time::Duration::MAX;
        for _ in 0..5 {
            let at = std::time::Instant::now();
            small();
            small_best = small_best.min(at.elapsed());
            let at = std::time::Instant::now();
            large();
            large_best = large_best.min(at.elapsed());
        }
        best = best.min(large_best.as_secs_f64() / small_best.as_secs_f64());
    }
    best
}
