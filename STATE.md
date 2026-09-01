# State

## Objective

Build **ting**: a tiny, zero-dependency scripting language, implemented in
Rust as a tree-walking interpreter. One self-contained binary (`ting`) that
runs scripts and offers a REPL. MIT licensed, publishable as plain source +
release binaries on GitHub.

## Why this artifact

- Decomposes into an effectively unbounded stream of small, independently
  verifiable tasks — ideal fuel for an agentic loop.
- Zero runtime dependencies; anyone can `cargo build` it or run the binary
  on their own machine. No service, no restrictive platform.
- Rust toolchain is installed, free, and produces static-ish binaries.

## Now (next iteration picks from the top)

Milestone **v0.5.0 — expressiveness** (designed in LOG.md
replenishment entry, 2026-09-01):

1. assert(cond, msg) builtin + self-hosted test suite: ting programs
   under selftest/ that test the language, run by CI via the real
   binary (new integration test).
2. Modules: import(path) — one design iteration first (return value
   vs. namespace map, caching, cycles), then implement.
3. Release v0.5.0.

Maintenance runs alongside (never instead): watch issues/PRs, keep CI
green.

## Done

- Loop protocol designed (`LOOP.md`), state/log files created.
- Crate bootstrapped: `ting` binary crate, MIT license, README, .gitignore.
- Lexer (`src/lexer.rs`): full token set, spans, comments; 15 unit tests.
- Parser (`src/parser.rs`) + AST (`src/ast.rs`): Pratt expression parser —
  precedence, unary, calls, indexing, lists; 14 tests. `ting <file>`
  currently parses one expression and prints the s-expression.
- Evaluator (`src/eval.rs`) + values (`src/value.rs`): arithmetic with
  int/float promotion and overflow checks, string/list concat, strict
  short-circuit booleans, structural equality, negative indexing; 14
  tests. `ting <file>` now evaluates one expression.
- Statements: `let`/assignment/blocks/`print` with a scope stack in
  `Interpreter`; semicolons mandatory; 11 tests. `ting <file>` runs a
  whole program.
- Control flow: `if`/`else if`/`else` and `while`, brace-required,
  strict-bool conditions; 7 tests. fib(10) works.
- Functions and closures: `fn` decls (desugared to `let`), anonymous
  fns, `return` via Control enum, Rc<RefCell<Env>> environments, depth
  cap 200, 32MB interpreter thread; 16 tests.
- Maps (`{"k": v}`, string keys, BTreeMap) + index assignment incl.
  nested; lists/maps now reference types (Rc<RefCell>); 10 tests.
- Builtins as first-class values: print, len, push, pop, keys, has,
  str, int, float, type, range; 7 tests.
- REPL (`src/repl.rs`): expression echo, multi-line continuation,
  auto-semicolon, pipe-friendly, persistent session; 8 tests.
- Diagnostics (`src/diag.rs`): caret underlines with source excerpt on
  all script error paths; 6 tests.
- Six examples/ programs with golden .out files, run by
  tests/examples.rs against the real binary.
- Language reference (docs/reference.md); README rewritten with a
  verified sample.
- rustfmt applied tree-wide; CI (fmt+clippy+test, 3-OS matrix) on
  push/PR. Green on all 3 OSes after a Windows CRLF golden-file fix
  (.gitattributes eol=lf).
- String builtins split/join/trim (14 builtins total); 2 tests.
- for-in (lists/strings/maps, snapshot, per-iteration binding) +
  break/continue; 8 tests. collections example modernized.
- REPL caret diagnostics; v0.2.0 tagged and released.
- String builtins batch 2: contains, replace, starts_with/ends_with,
  upper/lower, slice (Python-style bounds; 21 builtins total); 5 tests.
- Script I/O: args()/input()/read_file()/write_file(), argv forwarding
  in main.rs, tests/io.rs against the real binary (25 builtins, 127
  tests).
- sort/sort_by (fresh list, stable, key function, class-checked
  ordering) + call_value helper for function-taking builtins; 129
  tests.
- try(f)/fail(msg) error recovery as builtins (catches everything incl.
  stack overflow; interpreter stays usable); 29 builtins, 130 tests.
- Executable tutorial (docs/tutorial.md, 9 standalone snippets) run and
  output-diffed by tests/tutorial.rs; README updated; 131 tests.
- Interpreter extracted to lib.rs with run_source() entry point;
  main.rs is a thin consumer; 133 tests.
- wasm ABI (src/wasm.rs, 5 extern fns, no wasm-bindgen); 294KB
  ting.wasm verified end-to-end in Node; build with
  `cargo build --release --lib --target wasm32-unknown-unknown`;
  136 tests.
- Browser playground (playground/index.html + build.sh): worker
  isolation with 5s timeout; browser-verified via Playwright.
- Playground LIVE on GitHub Pages (pages.yml, path-filtered);
  verified in-browser at http://www.baghino.me/thing/; README links
  it.
- Fuzz tests (tests/fuzz.rs: token soup, example mutants, deep
  nesting); found+fixed a real describe() panic on stray ':'; 141
  tests.
- v0.4.0 RELEASED and verified: 3 assets; darwin binary renders the
  fixed stray-colon diagnostic cleanly.
  https://github.com/stefanobaghino/thing/releases/tag/v0.4.0
- map/filter/reduce/min/max/abs (35 builtins); playground "map &
  filter" example; 143 tests.
- v0.3.0 RELEASED and verified: 3 assets, darwin binary smoke-tested
  (fizzbuzz + try/slice/upper).
  https://github.com/stefanobaghino/thing/releases/tag/v0.3.0
- v0.1.0 RELEASED: 3-platform binaries verified (downloaded darwin
  asset, ran examples). https://github.com/stefanobaghino/thing/releases/tag/v0.1.0

## Blockers

None.
