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

Milestone **v0.3.0 — ting as a practical scripting tool** (human
directive 2026-09-01: keep building, never idle):

1. String builtins, batch 2: `contains`, `replace`, `starts_with`,
   `ends_with`, `upper`, `lower`, `slice(s, lo, hi)` (also for lists).
2. Script I/O builtins: `args()`, `input()`, `read_file(path)`,
   `write_file(path, s)` — turns ting into a usable shell-script
   alternative. Update reference + an example that processes a file.
3. `sort(xs)` and `sort_by(xs, f)` builtins (stable; error on mixed
   incomparable types).
4. Runtime error recovery: design and log the approach (leaning
   `try(f)` builtin returning `{"ok": ..}`/`{"err": ..}` over new
   syntax), then implement.
5. Tutorial (docs/tutorial.md): a guided walk from hello to a real
   script, kept honest by running every snippet.
6. Release v0.3.0 (bump, tag, verify assets).

After v0.3.0: replenish per LOOP.md "No idle" — candidates already on
the radar: WASM/browser playground (static page, runs client-side),
fuzz harness, bytecode VM port, self-hosted ting programs as tests.

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
- v0.1.0 RELEASED: 3-platform binaries verified (downloaded darwin
  asset, ran examples). https://github.com/stefanobaghino/thing/releases/tag/v0.1.0

## Blockers

None.
