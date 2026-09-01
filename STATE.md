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

1. Lexer: tokens for numbers, strings, identifiers, keywords, operators,
   with source positions. Unit tests.
2. Parser: Pratt/precedence-climbing expression parser producing an AST.
3. Evaluator: arithmetic, comparison, boolean logic on literals.
4. Statements: `let`, assignment, blocks, `print`.
5. Control flow: `if`/`else`, `while`.
6. Functions and closures.
7. Data: strings ops, lists, maps, indexing.
8. Builtins: `len`, `push`, type conversion, basic I/O.
9. REPL with line editing (std-only; no deps).
10. Diagnostics: spans in error messages, source excerpts.
11. Example programs under `examples/`, run as integration tests.
12. Language reference in `docs/`.
13. GitHub Actions CI (build + test on push).
14. Tagged release with prebuilt binaries.

## Done

- Loop protocol designed (`LOOP.md`), state/log files created.
- Crate bootstrapped: `ting` binary crate, MIT license, README, .gitignore.

## Blockers

None.
