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

1. Builtins: `len`, `push`, type conversion, basic I/O.
2. REPL with line editing (std-only; no deps).
3. Diagnostics: spans in error messages, source excerpts.
4. Example programs under `examples/`, run as integration tests.
5. Language reference in `docs/`.
6. GitHub Actions CI (build + test on push).
7. Tagged release with prebuilt binaries.

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

## Blockers

None.
