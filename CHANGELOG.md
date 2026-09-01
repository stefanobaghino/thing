# Changelog

All notable changes to ting. Versions are git tags; binaries for
Linux/macOS/Windows are attached to each
[GitHub release](https://github.com/stefanobaghino/thing/releases).

## Unreleased

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
