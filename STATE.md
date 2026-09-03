# State

## Objective

Build **ting**: a tiny, zero-dependency scripting language, implemented in
Rust. One self-contained binary that runs scripts and offers a REPL,
formatter (`--fmt`), static checker (`--check`), and LSP server. MIT
licensed; source + release binaries + docs site on GitHub. Full history of
every iteration lives in LOG.md (append-only); this file is only the
current orientation.

## Standing shape (stable since v2.0)

- Two engines (bytecode VM default, tree-walking reference) held
  byte-identical by differential tests incl. a grammar fuzzer
  (env-tunable seed/cases) and a CI job rerunning everything on eval.
- 44 builtins; five embedded stdlib modules (list/map/string/math/test);
  25 selftest+example ting programs; 182 Rust tests in 14 suites.
- Distribution: 3-platform GitHub release archives per tag, Pages site
  (playground with run+fmt, tutorial/reference/stdlib/changelog/story),
  wasm via hand-rolled ABI.
- 2.x stability promise (docs/reference.md#stability): additive only.

## Working rhythm (per LOOP.md, incl. the no-idle rule)

1. Maintenance check every tick: issues, PRs, CI, tree.
2. One small verifiable stroke per tick (feature, docs, test, health
   check); fmt + clippy + full suite before every push — no exceptions
   (clippy skipped once, iteration 182, cost a red CI).
3. Release when ~3 strokes accumulate; verify every release by cold
   asset download and execution; verdicts always from the API, never
   from gh run watch's exit code.

## Now

- v2.12.0 RELEASED and verified (32nd release, all verified).
- Post-182: fuzz generator emits find + stepped range; clippy fix
  green on CI.
- v2.13.0 RELEASED and verified (33rd); darwin-arm64 cold-tested.
- v2.14.0 RELEASED and verified (34th); darwin-arm64 cold-tested.
- v2.15.0 RELEASED and verified (35th); darwin-arm64 cold-tested.
- Loop stopped by the human after 195b, restarted at 196 (2026-09-03).
- 196: group_by; 197: take/drop; 198: partition — three strokes
  banked; RELEASE v2.16.0 next tick if quiet.
- Backlog after the release: chars/reverse in lib/string.ting;
  health tick (bench vs BASELINE.md, big fuzz sweep).
- Toolchain note: rustc 1.98 locally; rustfmt+clippy reinstalled 196.
- Periodic health ticks (bench vs BASELINE.md, big fuzz sweeps)
  when quiet.
