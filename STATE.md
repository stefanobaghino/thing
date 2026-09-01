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

1. `cargo init` the crate, add MIT license, README skeleton, `.gitignore`.
2. Lexer: tokens for numbers, strings, identifiers, keywords, operators,
   with source positions. Unit tests.
3. Parser: Pratt/precedence-climbing expression parser producing an AST.
4. Evaluator: arithmetic, comparison, boolean logic on literals.
5. Statements: `let`, assignment, blocks, `print`.
6. Control flow: `if`/`else`, `while`.
7. Functions and closures.
8. Data: strings ops, lists, maps, indexing.
9. Builtins: `len`, `push`, type conversion, basic I/O.
10. REPL with line editing (std-only; no deps).
11. Diagnostics: spans in error messages, source excerpts.
12. Example programs under `examples/`, run as integration tests.
13. Language reference in `docs/`.
14. GitHub Actions CI (build + test on push).
15. Tagged release with prebuilt binaries.

## Done

- Loop protocol designed (`LOOP.md`), state/log files created.

## Blockers

None.
