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

Milestone **v0.8.0 — a real scripting citizen** (designed in LOG.md
replenishment entry, 2026-09-01):

1. CHANGELOG.md: retroactive for v0.1.0..v0.7.0, then per release.
2. Release v0.8.0.

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
- assert builtin + self-hosted selftest/ suite (5 ting programs, 100+
  assertions, exit-0-and-silent enforced by tests/selftest.rs); 36
  builtins, 144 tests.
- import() modules: fresh env, exports map, per-path cache, cycle
  detection, module-located diagnostics; selftest/modules.ting; 37
  builtins, 145 tests.
- v0.5.0 RELEASED and verified: 3 assets; darwin binary ran a
  two-file import/map/reduce program correctly.
  https://github.com/stefanobaghino/thing/releases/tag/v0.5.0
- Benchmark harness (bench/ + run.py + BASELINE.md: fib 295ms, lists
  101ms, maps 112ms, strings 54ms on the dev machine).
- Measured optimization pass: ~10% across benches (inert block
  scopes skipped, Rc<str> env keys); BASELINE.md updated.
- Playground share-by-URL (fragment-encoded source, auto-run on
  open); browser-verified.
- Tutorial modules section (self-contained write_file+import snippet;
  10 verified snippets).
- v0.6.0 RELEASED and verified: 3 assets; darwin binary smoke-tested.
  https://github.com/stefanobaghino/thing/releases/tag/v0.6.0
- format() builtin (strict placeholders; 38 builtins; selftest
  coverage).
- TextMate grammar (editor/) + install README + builtin-sync guard
  test; 146 tests.
- Playground syntax highlighting (overlay + regex tokenizer);
  Playwright-verified alignment and live update.
- Docs rendered to Pages (tools/md2html.py, nav-linked from the
  playground); local render verified.
- v0.7.0 RELEASED and verified: 3 assets; format() smoke-tested in
  the shipped binary.
  https://github.com/stefanobaghino/thing/releases/tag/v0.7.0
- JSON builtins (src/json.rs: full decoder incl. surrogate pairs;
  sorted-key encoder); 40 builtins, 151 tests.
- env/exit/time_ms builtins (43 total; wasm-safe errors); process
  integration tests; 152 tests.
- todo.ting showcase + scenario test (tests/todo.rs); fuzz exercise
  now parse-only for exit-mentioning programs; 153 tests.
- v0.3.0 RELEASED and verified: 3 assets, darwin binary smoke-tested
  (fizzbuzz + try/slice/upper).
  https://github.com/stefanobaghino/thing/releases/tag/v0.3.0
- v0.1.0 RELEASED: 3-platform binaries verified (downloaded darwin
  asset, ran examples). https://github.com/stefanobaghino/thing/releases/tag/v0.1.0

## Blockers

None.
