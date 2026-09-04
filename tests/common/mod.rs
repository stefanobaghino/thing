//! Shared test helpers: the deterministic RNG and the grammar-directed
//! program generator used by the differential and formatter fuzz
//! suites. One grammar, two invariants.
#![allow(dead_code)]

/// xorshift64* — same generator as tests/fuzz.rs.
pub struct Rng(pub u64);

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
             fn d(v, w = 1, u = w + 1) { return v + w + u; }\n",
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
        match self.rng.below(10) {
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
        match self.rng.below(32) {
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
            _ => format!("-({})", self.expr(depth - 1)),
        }
    }
}
