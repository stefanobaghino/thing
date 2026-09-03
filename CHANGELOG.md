# Changelog

All notable changes to ting. Versions are git tags; binaries for
Linux (x86-64 and arm64, glibc and fully static musl), macOS and
Windows are attached to each
[GitHub release](https://github.com/stefanobaghino/thing/releases).

## v2.63.0 (2026-09-03)

- LSP: documentHighlight (occurrences of the symbol under the
  cursor, binding sites as writes) and prepareRename (the editor
  declines a rename on a keyword or builtin before prompting).
- lib/map.ting: `merge_with(a, b, f)`.

## v2.62.0 (2026-09-03)

- REPL: a session transcript — `:history` lists every chunk that
  ran without error, `:save FILE` writes them as a runnable script,
  `:clear` empties them — and `:doc` alone lists everything like
  `--doc`.

## v2.61.0 (2026-09-03)

- LSP completion offers the file's own functions with their
  signature and the comment above them.
- lib/list.ting: `flatten_deep(xs)`; `flatten` documented.
- Tutorial: the closing chapter matches the toolchain (warnings,
  import walk, `--doc`, `--fmt --diff`, the playground's check).

## v2.60.0 (2026-09-03)

- LSP hover on a function defined in the file shows the `#` comment
  above it.
- lib/map.ting: `map_keys(m, f)`.

## v2.59.0 (2026-09-03)

- Docs: the tutorial's Testing chapter covers the runner's flags,
  the stdlib page opens with all six modules and the `--doc` route
  to the same text, and the retrospective gains its eighth act.
- The crash fuzzer exercises cyclic values.

## v2.58.0 (2026-09-03)

- Cyclic data no longer crashes the process: printing shows `[...]`
  / `{...}` at the point of recursion, `==` terminates (two cycles
  that agree everywhere they can be inspected are equal), and
  `json_str` reports a cyclic value as an error.

## v2.57.0 (2026-09-03)

- lib/list.ting: `find_index(xs, pred)`.
- examples/series.ting: extent, mean, median, mode, percentile,
  window and chunk_by on a numeric series; cookbook and playground
  regenerated.
- editor/README.md describes the language server's twelve
  capabilities and its warnings.

## v2.56.0 (2026-09-03)

- Playground: a check button (the checker and its warnings, via a
  new `ting_check` wasm export).
- lib/list.ting: `chunk_by(xs, key)` groups consecutive runs.
- examples/text.ting: words, frequencies, slug, wrap and
  levenshtein at work; cookbook and playground regenerated.

## v2.55.0 (2026-09-03)

- `ting --test --fail-fast` stops after the first failing file; the
  rest count as skipped (TAP `# SKIP` lines).
- `ting --doc path/to/file.ting` lists a file's top-level functions
  with the comments above them.
- docs/vm.md opens with the VM's current status.

## v2.54.0 (2026-09-03)

- LSP: an `import` of a local file that fails to lex, parse or
  compile is an error diagnostic on the import string, with the
  module's position and message.
- README tooling paragraphs and the tutorial's modules chapter
  brought up to date (module error locations, the call-site note,
  `--check` following imports).

## v2.53.0 (2026-09-03)

- A module-origin error is followed by `note: called from
  FILE:LINE:COL`, the call site in the importer.
- `--check` follows `import("...")` of local files, checking each
  reached file once under its own path.
- lib/string.ting: `slug(s)`.

## v2.52.0 (2026-09-03)

- Runtime errors raised inside an imported module's function are
  reported against the module's file and line (both engines); an
  error from an embedded stdlib module no longer panics the
  diagnostic renderer.
- lib/list.ting: `mode(xs)`, the most frequent element.
- Retrospective: seventh act, "second opinions".

## v2.51.0 (2026-09-03)

- `ting --doc` with no name lists every builtin and stdlib function;
  `--doc MODULE` lists one module. The REPL's `:doc` does the same.
- `--check` and the LSP warn about function parameters the body
  never names (underscore-prefixed names are exempt).
- lib/list.ting: `extent(xs)` returns `[smallest, largest]`.

## v2.50.0 (2026-09-03)

- `--check` and the LSP warn about unused top-level bindings
  (underscore-prefixed names and binding-only module files are
  exempt).
- LSP: signature help for the file's own functions.
- `ting --fmt --diff` prints the changed lines instead of writing.

## v2.49.0 (2026-09-03)

- `ting --test --slow N` lists the N slowest files after the summary.

## v2.48.0 (2026-09-03)

- Playground: the example dropdown is generated from `examples/`
  (twelve runnable examples) and guarded against drift.
- LSP: hover shows the signature of the file's own functions.
- `lib/math.ting`: `percentile`.

## v2.47.0 (2026-09-03)

- LSP: rename applies across every open document.

## v2.46.0 (2026-09-03)

- `ting --test -j N` runs up to N files at once, output kept in
  order.
- `lib/string.ting`: `dedent`.
- Tutorial: a "Shell scripting" chapter.

## v2.45.0 (2026-09-03)

- Tutorial: the modules chapter points at `--doc`, `:doc` and editor
  hover for reading a stdlib function.

## v2.44.0 (2026-09-03)

- LSP: document links on `import(...)` paths; a malformed message
  no longer ends the session; Windows drive-letter file URIs are
  handled.
- `lib/math.ting`: `variance`, `stddev` (the stats example uses
  them).
- README: status and tooling paragraphs brought up to date.

## v2.43.0 (2026-09-03)

- LSP: workspace symbols across open documents.

## v2.42.0 (2026-09-03)

- REPL: `:time EXPR` reports elapsed milliseconds.
- `examples/config.ting`: layered configuration with `lib/json.ting`.
- Tutorial: the JSON chapter shows `get_in`, `set_in` and `merge_in`.

## v2.41.0 (2026-09-03)

- `lib/string.ting`: `levenshtein`.
- Retrospective: a sixth act on the loop's rhythm.

## v2.40.0 (2026-09-03)

- `ting --test --tap` emits Test Anything Protocol output with
  per-file timings.
- `lib/list.ting`: `binary_search`.
- Tutorial: a "Closures as objects" chapter.

## v2.39.0 (2026-09-03)

- `lib/json.ting`: `diff`.
- `examples/machine.ting`: a state machine from closures and a
  transition table.

## v2.38.0 (2026-09-03)

- `ting --doc NAME` explains a builtin or stdlib function from the
  shell.
- `--check`, `--fmt` and `--fmt-check` accept directories.
- `lib/string.ting`: `wrap`.

## v2.37.0 (2026-09-03)

- LSP: folding ranges for multi-line braces.
- `bench/json.ting`: a JSON benchmark; the baseline gains its row.

## v2.36.0 (2026-09-03)

- `lib/list.ting`: `zip_with`, `cartesian`.
- Tutorial: a Testing chapter (`lib/test.ting` and `ting --test`).
- The formatter is now fuzzed against generated programs for
  idempotence and AST preservation.

## v2.35.0 (2026-09-03)

- REPL: `:doc NAME` explains a builtin or any stdlib function
  (module, signature, comment).
- `lib/test.ting`: `check_approx` for floats.

## v2.34.0 (2026-09-03)

- LSP: a quickfix code action replaces a misspelt stdlib member with
  the nearest export.
- `ting --test --filter SUBSTR` runs only matching paths.
- `lib/json.ting`: `merge_in`, a deep merge.

## v2.33.0 (2026-09-03)

- `lib/json.ting`, a sixth embedded module: `get_in`, `set_in`,
  `paths` for nested values.
- `ting --check` prints the unknown-stdlib-member warning the LSP
  shows (exit status unchanged).
- `examples/pipeline.ting`: records from stdin, grouped and tabled.

## v2.32.0 (2026-09-03)

- LSP: a warning when an imported stdlib module is indexed with a
  name it does not export.
- `ting --test` lists a directory's own files before descending
  into subdirectories.
- Retrospective: a fifth act on the glibc episode.

## v2.31.0 (2026-09-03)

- `ting --test` accepts directories (recursive, sorted), so
  `ting --test tests/` is the whole suite; CI runs the binary's own
  runner over `selftest/` on every platform.
- `lib/string.ting`: `table`, aligned columns for CLI output; the
  logs example prints one.

## v2.30.0 (2026-09-03)

- Releases now also ship fully static Linux archives
  (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`): no C
  library dependency at all.
- The crash fuzzer covers every pure builtin and the bytecode VM.
- `bench/stdlib.ting`: an import-heavy benchmark; the baseline is
  regenerated on one machine for all rows.

## v2.29.1 (2026-09-03)

- Linux binaries are built against glibc 2.35 again (v2.29.0's
  required 2.39 and would not start on Debian 12 / Ubuntu 22.04); the
  release workflow now fails if a Linux binary needs anything newer.

## v2.29.0 (2026-09-03)

- `ting --test <files...>`: a test runner — each file in its own
  process, `ok`/`FAIL` per file, a summary, exit 1 on any failure.
- `lib/list.ting`: `sort_with`, a stable sort by a three-way
  comparator.
- Site: a cookbook page rendering every example with its output.

## v2.28.0 (2026-09-03)

- `lib/list.ting`: `product`, `mean_by`, `compact`.
- `lib/string.ting`: `is_blank`.
- Tutorial: the word-frequency script tallies with `frequencies` and
  `top`.

## v2.27.0 (2026-09-03)

- `lib/string.ting`: `indent`.
- `lib/map.ting`: `top`.
- Tutorial: the closing chapter covers every REPL meta-command, the
  stdin filter, and the editor's stdlib awareness.

## v2.26.0 (2026-09-03)

- LSP: hover and signature help work for stdlib functions called
  through an imported module map (`l["median"](...)`), showing the
  signature and the function's comment; completion items carry the
  same comment.
- `lib/list.ting`: `frequencies`.

## v2.25.0 (2026-09-03)

- LSP: completion offers the functions of every stdlib module the
  document imports, with module and signature as detail.
- `lib/list.ting`: `interleave`.
- The differential fuzz generator now emits ten more builtins
  (string predicates, `replace`, `split`, `trim`, `lower`, `max`,
  `type`, `filter`, `reduce`).

## v2.24.0 (2026-09-03)

- `lib/list.ting`: `scan`.
- `lib/string.ting`: `strip_prefix`, `strip_suffix`.
- Retrospective: a fourth act on the move to a new machine.

## v2.23.0 (2026-09-03)

- `lib/list.ting`: `rotate`, `unique_by`.
- `lib/string.ting`: `truncate`.
- `lib/math.ting`: `is_prime`.

## v2.22.0 (2026-09-03)

- `lib/list.ting`: `sum_by`.
- `lib/string.ting`: `words`.
- `lib/map.ting`: `with`, `update`.
- Tutorial: the word-frequency script tallies with `words` and
  `count_by`.

## v2.21.0 (2026-09-03)

- Formatter: a `[` or `(` that ends its line indents its
  continuation lines by one level until the closer (inline openers
  are unchanged).
- `lib/string.ting`: `is_digit`, `is_alpha`.
- `examples/logs.ting`: a log summary using `count_by`, `window` and
  `is_digit`.

## v2.20.0 (2026-09-03)

- REPL: `:fmt` reprints the last evaluated chunk as the formatter
  would write it.
- `lib/list.ting`: `count_by`, `first`, `last`.
- `lib/map.ting`: `invert`.

## v2.19.0 (2026-09-03)

- `lib/list.ting`: `window`.
- `lib/string.ting`: `center`.
- Tutorial: the modules chapter shows `partition`, `group_by` and
  `take`/`drop`.

## v2.18.0 (2026-09-03)

- `ting x.ting | head` ends quietly with exit 0 when the reader goes
  away; the REPL does the same instead of panicking.
- `--fmt`, `--fmt-check` and `--check` accept `-` for stdin; `--fmt -`
  filters to stdout.
- `lib/math.ting`: `lcm`, `abs_diff`.

## v2.17.0 (2026-09-03)

- Releases now also ship an `aarch64-unknown-linux-gnu` archive (four
  platforms).
- `lib/string.ting`: `chars`, `reverse`.
- `lib/map.ting`: `filter_map`, `has_all`.

## v2.16.0 (2026-09-03)

- `lib/list.ting`: `group_by`, `take`, `drop`, `partition`.

## v2.15.0 (2026-09-03)

- `lib/list.ting`: `median`.
- REPL: `:clear` resets the session.
- `examples/stats.ting` now uses `mean`/`median` from the stdlib.

## v2.14.0 (2026-09-03)

- REPL: `:vars` lists the session's own bindings (name and type).
- `lib/list.ting`: `mean`.
- Tutorial: the modules chapter now shows the embedded stdlib and
  the disk-first fallback rule.

## v2.13.0 (2026-09-03)

- `lib/map.ting`: `pick`, `omit`.
- `lib/string.ting` and `lib/list.ting`: `count`.
- The differential fuzz generator now emits `find` and stepped
  `range` expressions, extending engine-equivalence coverage to the
  newer builtins.

## v2.12.0 (2026-09-02)

- `write_file(path, s, "append")`: optional append mode; any other
  mode errors.
- `lib/list.ting`: `insert_at`, `remove_at` (fresh lists, loud
  range checks).
- Selftests pin JSON control-character escaping and round trips.

## v2.11.0 (2026-09-02)

- Strings accept the `\r` escape (previously a carriage return was
  inexpressible in source); the TextMate grammar and a new sync
  guard follow.
- `lib/string.ting`: `trim_start`, `trim_end`.
- README brought up to date with the current feature set.

## v2.10.0 (2026-09-02)

- `lib/test.ting`: `check_err(name, f, want)` — error-path testing
  with distinct failure messages for wrong-error vs no-error.
- `lib/math.ting`: `floor`, `ceil` (correct on negatives, where
  `int()` truncation differs).
- `lib/list.ting`: `chunk(xs, n)`.

## v2.9.0 (2026-09-02)

- `read_file("-")` reads stdin to EOF, so ting scripts compose in
  Unix pipelines.
- LSP: `textDocument/signatureHelp` — builtin signatures and docs
  inside call parentheses (ninth capability).
- The changelog is now published on the site, linked from every
  page's nav.

## v2.8.0 (2026-09-02)

- Playground: a "fmt" button reformats the editor in place, backed
  by a new `ting_fmt` wasm export (verified against the live site).
- `lib/map.ting`: `values`, `map_values`.
- `lib/string.ting`: `split_once` (built on `find`, so indices are
  character-based).

## v2.7.0 (2026-09-02)

- REPL: `:load <file>` evaluates a file in the live session, keeping
  its bindings around to poke at.
- `lib/list.ting`: `any`, `all`, `min_by`, `max_by`.
- Docs: the reference documents both REPL meta-commands; the
  tutorial closes with a "Beyond scripts" tour of the toolchain.

## v2.6.0 (2026-09-02)

- `find(s, sub)` / `find(xs, v)`: 44th builtin — index of the first
  match or `nil`; strings use character indexing (matching `slice`),
  lists use structural equality (matching `contains`).
- REPL: `:help` lists every builtin's signature and doc line.
- A guard test now keeps repo markdown free of bare HTML-shaped
  tokens (a bare angle-bracketed token had broken LOG.md's rendering
  on GitHub; found by a reader).

## v2.5.0 (2026-09-02)

- LSP: `textDocument/references` — every occurrence of the
  identifier under the cursor (token-level).
- LSP: `textDocument/rename` — a WorkspaceEdit over those same
  occurrences; invalid new names are rejected.
- Reference: Tooling section updated with the full LSP capability
  list.

## v2.4.0 (2026-09-02)

- `lib/math.ting`: fifth stdlib module — `clamp`, `sign`, `pow`,
  `gcd`, `round`, `sqrt` (embedded in the binary like the rest).
- `range(lo, hi, step)`: optional third argument; negative steps
  count down, zero is an error. Existing forms unchanged.
- `examples/stats.ting`: descriptive statistics golden pair using
  both of the above.

## v2.3.0 (2026-09-01)

- LSP: `textDocument/documentSymbol` — an outline of top-level
  bindings, functions and variables distinguished.
- LSP: `textDocument/definition` — jump from an identifier to its
  top-level binding.
- Reference: new "Tooling" section documenting `--fmt`, `--check`,
  the LSP's capabilities, and the TextMate grammar.

## v2.2.0 (2026-09-01)

- `json_str(v, indent)`: optional pretty printing — `indent` spaces
  per level (0–16), empty containers stay inline, output round-trips
  through `json_parse`. Single-argument compact form is unchanged.
- `ting --check <files...>`: static verification — lex, parse, and
  compile without running; one diagnostic per bad file, exit 1 if any
  fail. Built for pre-commit hooks and CI.
- Tutorial: new "Working with JSON" section (parse, mutate, compact
  vs pretty output, error recovery); every snippet is CI-tested.

## v2.1.0 (2026-09-01)

- Fix: `==` now compares ints and floats numerically at every depth —
  `[1] == [1.0]` is true, matching the documented top-level rule
  (this also corrects `contains` and `lib/list.ting`'s `unique` for
  mixed int/float data).
- `selftest/edge.ting`: 25 sharp-edge assertions pinned on both
  engines (this suite is what caught the bug above).
- Playground: a "calc" example — a tiny calculator language
  interpreted by ting, in the browser.

## v2.0.0 — maturity (2026-09-01)

No new features — a milestone of confidence. A seven-point
full-system audit (both engines' suites, cross-engine benchmark
checksums, wasm in Node, formatter round trip, the live site, the
release assets) came back all green, and on that evidence the
reference now carries a stability promise: the documented language is
stable across 2.x; builtins are only ever added; breaking syntax or
semantics would mean a 3.0.

## v1.9.0 — depth (2026-09-01)

- Differential fuzzing generates a wider grammar (bounded loops,
  try-expressions, string operations; 800 cases per run) — both
  engines stay byte-identical.
- `examples/calc.ting`: a small arithmetic language interpreted by
  ting itself — tokenizer, parser, and evaluator in ting.

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
