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


**Post-2.0 small strokes** (designed in LOG.md replenishment entry,
2026-09-01; release v2.1.0 when value accumulates):

1. Ongoing: maintenance (issues/PRs/CI); replenish per LOOP.md
   with small strokes; release when value accumulates.

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
  fns, `return` via Control enum, `Rc<RefCell<Env>>` environments, depth
  cap 200, 32MB interpreter thread; 16 tests.
- Maps (`{"k": v}`, string keys, BTreeMap) + index assignment incl.
  nested; lists/maps now reference types (`Rc<RefCell>`); 10 tests.
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
  scopes skipped, `Rc<str>` env keys); BASELINE.md updated.
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
- CHANGELOG.md (retroactive v0.1.0..v0.7.0, then per release).
- v0.8.0 RELEASED and verified: 3 assets; todo showcase ran on the
  shipped darwin binary.
  https://github.com/stefanobaghino/thing/releases/tag/v0.8.0
- VM design doc (docs/vm.md: stack machine, span-parallel chunks,
  Env kept, differential-testing plan).
- Expression VM (compile.rs+vm.rs) behind --vm; differential corpus
  byte-identical incl. diagnostics (2 span divergences caught+fixed);
  155 tests.
- VM control flow (if/while/for/break/continue/scoped blocks;
  IterNext snapshot loops); 19 more differential programs; 156 tests.
- VM functions via MakeFn hybrid (AST bodies, captured Env): full
  language parity; selftest suite byte-identical on both engines;
  158 tests.
- Engines benchmarked: vm +0-2% vs eval (no win; hybrid leaves hot
  paths tree-walked) — default stays eval, verdict in docs/vm.md.
- v0.9.0 RELEASED and verified: 3 assets; both engines identical in
  the shipped binary.
  https://github.com/stefanobaghino/thing/releases/tag/v0.9.0
- Grammar-directed differential fuzzing (600 valid generated
  programs per run, both engines byte-identical; token soup proved
  unparseable and was dropped).
- CI test-vm job: full suite with TING_ENGINE=vm (verified locally
  first).
- Doc-coverage guard (tests/docs.rs) + README refreshed (engines,
  43 builtins, links); 159 tests.
- v1.0.0 RELEASED and verified: 3 assets; full selftest suite passes
  on the shipped binary under both engines.
  https://github.com/stefanobaghino/thing/releases/tag/v1.0.0
- Compiled function bodies (FnBody::Chunk, Op::Return): parity holds;
  re-benchmarked: vm +2-7% (worse) — dispatch was never the cost.
- Local slot resolution (capture analysis, GetSlot/SetSlot, no-Env
  frames): vm now -35% fib, -29% lists, -11% strings, +1% maps.
- VM is now the DEFAULT engine (--eval escape hatch; CI test-eval
  row; wasm playground on VM too); docs/README/BASELINE updated.
- v1.1.0 RELEASED and verified: 3 assets; both engines correct in the
  shipped binary.
  https://github.com/stefanobaghino/thing/releases/tag/v1.1.0
- LSP server (src/lsp.rs, ting --lsp): framing, lifecycle,
  publishDiagnostics; tests/lsp.rs drives real pipes; editor wiring
  docs; 160 tests.
- v1.2.0 RELEASED and verified: 3 assets; shipped binary's LSP driven
  over pipes end-to-end.
  https://github.com/stefanobaghino/thing/releases/tag/v1.2.0
- Stdlib in ting (lib/list.ting, lib/string.ting; 13 selftest
  assertions; shipped in release archives).
- LSP hover (Builtin::doc for all 43; document tracking; tested).
- Retrospective (docs/retrospective.md) linked from README and the
  site nav; rendered by pages.yml.
- v1.3.0 RELEASED and verified: 3 assets; lib/ bundled and imported
  from the shipped binary.
  https://github.com/stefanobaghino/thing/releases/tag/v1.3.0
- LSP completions (builtins+docs, keywords, document identifiers);
  tested.
- lib/test.ting framework (+selftest/testlib.ting, examples/
  testing.ting golden).
- VM micro-pass: buffer pooling (fib/lists now -45% vs eval) + const
  dedup; BASELINE updated.
- v1.4.0 RELEASED and verified: 3 assets; test framework + stdlib ran
  from the shipped archive.
  https://github.com/stefanobaghino/thing/releases/tag/v1.4.0
- Embedded stdlib fallback in import() (include_str!, fs-first;
  tested); fuzz harness hardened against importing mutants.
- Playground "stdlib" example (embedded fallback browser-verified).
- docs/stdlib.md on the site (nav "stdlib"); render verified.
- v1.5.0 RELEASED and verified: 3 assets; embedded stdlib serves with
  lib/ deleted; stdlib page live.
  https://github.com/stefanobaghino/thing/releases/tag/v1.5.0
- Formatter core (src/fmt.rs, gap-scan comments, brace-kind stack);
  idempotence+AST guards over all 21 repo .ting files; 14 suites.
- ting --fmt/--fmt-check CLI; repo reformatted; formatted-ness now
  CI-enforced (3 style bugs found by dogfooding, fixed+tested).
- LSP documentFormatting (whole-doc edit; tested).
- v1.6.0 RELEASED and verified: 3 assets; shipped formatter
  round-trip confirmed.
  https://github.com/stefanobaghino/thing/releases/tag/v1.6.0
- Docs run-links (13 on tutorial; fragment decode verified) +
  --version/--help.
- v1.7.0 RELEASED and verified: 3 assets; run-links live.
  https://github.com/stefanobaghino/thing/releases/tag/v1.7.0
- Retrospective refreshed (full VM arc, tooling act, current
  numbers).
- lib/map.ting (embedded, selftested, documented) + rlwrap note.
- v1.8.0 RELEASED and verified: 3 assets; embedded map module served.
  https://github.com/stefanobaghino/thing/releases/tag/v1.8.0
- Fuzz generator v2 (bounded while, try-expr, string ops; 800
  cases) — byte-identical first run.
- calc.ting: tokenizer+parser+evaluator in ting, golden-tested.
- v1.9.0 RELEASED and verified: 3 assets; calc showcase ran on the
  shipped binary.
  https://github.com/stefanobaghino/thing/releases/tag/v1.9.0
- Grand audit: 7-point full-system sweep, all green (logged).
- Docs pass (README intro modernized) + 2.x stability promise in
  the reference.
- v2.0.0 RELEASED and verified: 3 assets; both engines correct on the
  shipped binary. Twentieth release.
  https://github.com/stefanobaghino/thing/releases/tag/v2.0.0
- Playground calc example (browser-verified).
- selftest/edge.ting (25 pinned edges) — found+fixed deep numeric
  equality bug (Value::PartialEq now promotes Int/Float).
- v2.1.0 RELEASED and verified (equality fix + edge suite + calc
  example). Twenty-first release.
  https://github.com/stefanobaghino/thing/releases/tag/v2.1.0
- json_str(v, indent) pretty printing (additive; tested; docs
  updated) — accumulating toward v2.2.0.
- Tutorial "Working with JSON" section (snippets CI-tested).
- ting --check: static verification CLI (lex/parse/compile only).
- v2.2.0 RELEASED and verified (json_str pretty + --check +
  tutorial JSON); darwin-arm64 asset smoke-tested cold.
- LSP documentSymbol (outline view) — accumulating toward v2.3.0.
- LSP go-to-definition for top-level bindings.
- Reference "Tooling" section (fmt/check/LSP capabilities).
- v2.3.0 RELEASED and verified (LSP symbols + definition +
  Tooling docs); darwin-arm64 asset smoke-tested cold.
- Perf health check: no regression vs BASELINE.md.
- lib/math.ting (clamp/sign/pow/gcd/round/sqrt) — toward v2.4.0.
- range(lo, hi, step) with negative steps (additive).
- examples/stats.ting golden pair (dogfoods both).
- v2.4.0 RELEASED and verified (math + stepped range + stats);
  darwin-arm64 asset smoke-tested cold incl. bundled math module.
- Playground "stats" example (wasm-verified via real ABI).
- LSP find-references (token-level) — toward v2.5.0.
- LSP rename (WorkspaceEdit) + Tooling list refreshed.
- v2.5.0 RELEASED and verified; darwin-arm64 smoke-tested cold.
- Retrospective third act (post-2.0 small-strokes era).
- REPL :help meta-command — toward v2.6.0.
- Human-reported markdown rendering bug fixed (bare tag-shaped
  tokens backticked) + docs guard test added (guard already caught
  one regression in CI).
- find() builtin (44th).
- v2.6.0 RELEASED and verified; darwin-arm64 smoke-tested cold.
- REPL :load — toward v2.7.0.
- Docs: REPL meta-commands + "Beyond scripts" tutorial section.
- lib/list any/all/min_by/max_by.
- v2.7.0 RELEASED and verified; darwin-arm64 smoke-tested cold.
- 50k-case differential sweep, 5 fresh seeds, zero divergences
  (fuzzer now env-parameterized).
- Playground fmt button (ting_fmt wasm export) — toward v2.8.0.
- fmt button verified live via Playwright; ting_fmt doc corrected
  (token-stream formatter fails only on lex errors).
- lib/map values/map_values.
- lib/string split_once.
- v2.8.0 RELEASED and verified; darwin-arm64 smoke-tested cold.
- Changelog page live on the site (verified); toward v2.9.0.
- LSP signatureHelp (9th capability).
- read_file("-") reads stdin.
- v2.9.0 RELEASED and verified; stdin pipe smoke-tested cold.
- lib/test check_err — toward v2.10.0.
- lib/math floor/ceil.
- lib/list chunk.
- v2.10.0 RELEASED and verified (30th); darwin-arm64 cold-tested.
- README refreshed (44 builtins, 5 modules, 9 LSP caps, 182 tests).
- \r escape (lexer) + lib/string trim_start/trim_end.
- Grammar escape class synced + guarded.
- v2.11.0 tagged; release run in flight — verify assets next wake.
- v0.3.0 RELEASED and verified: 3 assets, darwin binary smoke-tested
  (fizzbuzz + try/slice/upper).
  https://github.com/stefanobaghino/thing/releases/tag/v0.3.0
- v0.1.0 RELEASED: 3-platform binaries verified (downloaded darwin
  asset, ran examples). https://github.com/stefanobaghino/thing/releases/tag/v0.1.0

## Blockers

None.
