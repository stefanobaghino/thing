# Changelog

All notable changes to ting. Versions are git tags; binaries for
Linux/macOS/Windows are attached to each
[GitHub release](https://github.com/stefanobaghino/thing/releases).

## v1.8.0 — the second act (2026-09-01)

- `lib/map.ting`: get-with-default, merge, items, from_items — the
  stdlib's map gap, closed (embedded like the rest).
- The website's story page now tells the full arc, VM redemption
  included.
- The reference recommends `rlwrap ting` for REPL line editing.

## v1.7.0 — finishing touches (2026-09-01)

- Every ting snippet on the docs site now carries a "run it in the
  playground" link that opens it preloaded and running.
- `ting --version` and `ting --help`.

## v1.6.0 — a formatter (2026-09-01)

- `ting --fmt` / `ting --fmt-check`: a canonical formatter that
  preserves comments and the author's line breaks, guaranteed
  idempotent and AST-preserving by tests. Also available as
  format-on-save through `ting --lsp` (documentFormatting).
- The repo's own ting sources are formatted with it, enforced by CI.

## v1.5.0 — the stdlib everywhere (2026-09-01)

- The standard library is embedded in the interpreter: when an
  imported `lib/...` path has no matching file, the built-in copy is
  used — so it works from any directory, in the REPL, and in the
  browser playground (which gained a stdlib example). A real file
  always wins.
- `docs/stdlib.md` documents all three modules, on the website as
  "stdlib".

## v1.4.0 — sharper tools (2026-09-01)

- LSP completions: builtins with docs, keywords, and the document's
  own identifiers.
- `lib/test.ting`: a tiny test framework written in ting
  (`check`/`check_eq`/`summary`), with a golden example.
- VM: pooled per-call buffers roughly doubled its lead — fib and
  list-heavy work now run ~45% faster than the reference engine.

## v1.3.0 — batteries + story (2026-09-01)

- A standard library written in ting itself: `lib/list.ting` and
  `lib/string.ting`, shipped inside the release archives and covered
  by the self-hosted suite.
- LSP hover: signature and summary for every builtin.
- The experiment's [retrospective](docs/retrospective.md), on the
  website as "story".

## v1.2.0 — a language server (2026-09-01)

- `ting --lsp`: the binary doubles as an LSP server — JSON-RPC over
  stdio, full-text sync, live lex/parse/compile diagnostics with real
  ranges — implemented with zero new dependencies on top of ting's own
  JSON codec. Wiring instructions for Neovim/VS Code/Zed in `editor/`.

## v1.1.0 — the VM earns its keep (2026-09-01)

- The bytecode VM is now the default engine: with compiled function
  bodies and local slot resolution it is 11-35% faster on the
  function-heavy benchmarks with no regressions. `--eval` or
  `TING_ENGINE=eval` selects the reference tree-walker; CI runs the
  full suite on both engines.

## v1.0.0 — confidence (2026-09-01)

The language and tooling are complete and held together by guards:
grammar-directed differential fuzzing (600 random valid programs per
test run, both engines byte-identical), a CI job that reruns the whole
suite on the VM engine, and coverage guards that fail the build if a
builtin ever ships without documentation or editor support. No
language changes — 1.0 marks stability, not novelty.

## v0.9.0 — bytecode VM (2026-09-01)

- A bytecode compiler and VM covering the whole language, selectable
  with `--vm` or `TING_ENGINE=vm`; differential tests hold both
  engines byte-identical (including the entire self-hosted suite).
  Measured honestly: no speedup over the tree-walker yet, so the
  tree-walker stays the default (see `docs/vm.md`).
- Benchmarks now compare both engines (`bench/run.py`).

## v0.8.0 — a real scripting citizen (2026-09-01)

- `json_parse` / `json_str` builtins: full JSON both ways (objects↔maps,
  surrogate pairs, strict errors with byte offsets).
- `env`, `exit`, `time_ms` builtins (wasm builds return clean errors
  where the platform can't support them).
- Showcase: `examples/todo.ting`, a JSON-file-backed todo CLI, driven
  end-to-end by its own integration test.
- This changelog.

## v0.7.0 — developer experience (2026-09-01)

- `format(fmt, ...)` builtin with strict `{}` placeholder rules.
- TextMate grammar under `editor/` (VS Code/Sublime/Zed install
  guide); a guard test keeps its builtin list in sync.
- Playground: live syntax highlighting (overlay, no library).
- Tutorial and reference rendered onto the website next to the
  playground (`tools/md2html.py`, stdlib-only).

## v0.6.0 — performance + polish (2026-09-01)

- ~10% faster interpreter (measured with the new `bench/` harness and
  recorded baseline): blocks without declarations no longer allocate
  scopes; env keys are `Rc<str>`.
- Playground share-by-URL (source encoded in the fragment).
- Tutorial: modules section.

## v0.5.0 — expressiveness (2026-09-01)

- `map`, `filter`, `reduce`, `min`, `max`, `abs` builtins.
- `assert` builtin + self-hosted `selftest/` suite (ting programs that
  test ting, run by CI).
- Modules: `import(path)` — fresh scope, exports as a map, per-path
  caching, cycle detection, module-located diagnostics.

## v0.4.0 — ting in the browser + robustness (2026-09-01)

- The interpreter compiled to WebAssembly with a hand-rolled ABI (no
  wasm-bindgen; still zero dependencies) and a browser playground,
  live on GitHub Pages.
- Fuzz tests (token soup, example mutants, deep nesting) — which
  found and fixed a real parser panic on a stray `:`.

## v0.3.0 — a practical scripting tool (2026-09-01)

- String builtins: `contains`, `replace`, `starts_with`, `ends_with`,
  `upper`, `lower`, `slice`.
- Script I/O: `args`, `input`, `read_file`, `write_file`; argv after
  the script path reaches `args()`.
- Stable `sort` / `sort_by`; error recovery with `try` / `fail`.
- Executable tutorial: every snippet is run and diffed by CI.

## v0.2.0 (2026-09-01)

- `split`, `join`, `trim` builtins.
- `for`-in over lists/strings/maps with `break`/`continue`.
- REPL errors render caret diagnostics.

## v0.1.0 (2026-09-01)

- The language core: ints/floats/strings/bools/nil, lists and maps
  with reference semantics, closures, control flow, strict semantics
  (no truthiness, checked overflow, exact arity), caret diagnostics,
  a REPL, 11 builtins, examples, CI on three platforms.
