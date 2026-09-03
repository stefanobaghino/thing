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
- 44 builtins; six embedded stdlib modules
  (list/map/string/math/json/test);
  25 selftest+example ting programs; 182 Rust tests in 14 suites.
- Distribution: 4-platform GitHub release archives per tag (from
  v2.17.0; 3 before), Pages site
  (playground with run+fmt, tutorial/reference/stdlib/changelog/story),
  wasm via hand-rolled ABI.
- 2.x stability promise (docs/reference.md#stability): additive only.

## Working rhythm (per LOOP.md, incl. the no-idle rule)

1. Maintenance check every tick: issues, PRs, CI, tree.
2. One small verifiable stroke per tick (feature, docs, test, health
   check); fmt + clippy + full suite before every push — no exceptions
   (clippy skipped once, iteration 182, cost a red CI). After writing
   LOG/STATE, rerun the docs guard and gate the push on the literal
   `test result: ok` (a grep for "test result" passed a FAILED line
   in 238 and shipped a red commit). No angle-bracket placeholders
   anywhere in markdown, quoted or not. Linux release builds stay on
   the oldest runner (22.04); the glibc-floor step in release.yml is
   the guard — never move them to -latest.
3. Release when ~3 strokes accumulate; verify every release by cold
   asset download and execution; verdicts always from the API, never
   from gh run watch's exit code. A failed Pages deploy is retried
   ONLY with `gh workflow run pages.yml --ref main`: `--failed`
   reruns leave a duplicate artifact, and LOG/STATE-only pushes miss
   the workflow's path filter.

## Now

- v2.12.0 RELEASED and verified (32nd release, all verified).
- Post-182: fuzz generator emits find + stepped range; clippy fix
  green on CI.
- v2.13.0 RELEASED and verified (33rd); darwin-arm64 cold-tested.
- v2.14.0 RELEASED and verified (34th); darwin-arm64 cold-tested.
- v2.15.0 RELEASED and verified (35th); darwin-arm64 cold-tested.
- Loop stopped by the human after 195b, restarted at 196 (2026-09-03).
- v2.16.0 RELEASED and verified (36th); structural cold check only,
  since this host is aarch64 Linux (no matching asset yet).
- 200: aarch64 Linux in CI + release matrices (CI arm job green);
  201: string chars/reverse — two strokes toward v2.17.0.
- 202: health tick green (checksums match, ratios hold, 20k fuzz
  cases agree).
- v2.17.0 RELEASED and verified (37th); aarch64-linux asset
  executed cold on this host, both engines.
- v2.18.0 RELEASED and verified (38th); aarch64-linux asset
  executed cold on this host, both engines.
- 209: tutorial snippet for partition/group_by/take — first stroke
  toward v2.19.0. 210: distribution audit green (38 releases, arm64
  assets included, site all 200).
- v2.19.0 RELEASED and verified (39th); aarch64-linux asset
  executed cold on this host, both engines.
- 214: REPL :fmt; 215: count_by/invert — two strokes toward
  v2.20.0.
- 216: health tick — checksums match, 30k fuzz cases agree; bench
  times contended by other workloads (load 5.5), re-measure when
  quiet.
- v2.20.0 RELEASED and verified (40th); aarch64-linux asset
  executed cold on this host, both engines.
- v2.21.0 RELEASED and verified (41st); aarch64-linux asset
  executed cold on this host, both engines.
- 223: Pages deploy for v2.21.0 recovered via workflow_dispatch;
  site current.
- v2.22.0 RELEASED and verified (42nd); aarch64-linux asset
  executed cold on this host, both engines; Pages green first time.
- v2.23.0 RELEASED and verified (43rd); aarch64-linux asset
  executed cold on this host, both engines.
- v2.24.0 RELEASED and verified (44th); aarch64-linux asset
  executed cold on this host, both engines.
- v2.25.0 RELEASED and verified (45th); aarch64-linux asset
  executed cold on this host, both engines.
- v2.26.0 RELEASED and verified (46th); aarch64-linux asset
  executed cold on this host, both engines.
- v2.27.0 RELEASED and verified (47th); aarch64-linux asset
  executed cold on this host, both engines.
- v2.28.0 RELEASED and verified (48th); aarch64-linux asset
  executed cold on this host, both engines.
- 255: replenishment tick — milestone "programs, not one-liners"
  (v2.29–v2.31), reasoning in LOG.md.
- v2.29.0 (49th) shipped Linux binaries needing glibc 2.39 (cold
  test failed; release notes warn). v2.29.1 (50th) RELEASED and
  verified: Linux builds on ubuntu-22.04(-arm), GLIBC_2.34, guard
  step green, aarch64 asset executed cold here.
- v2.30.0 RELEASED and verified (52nd tag; six assets; both aarch64
  Linux archives executed cold here, musl fully static).
- 264: replenishment — milestone "the runner and the operator"
  (v2.31–v2.33), reasoning in LOG.md.
- v2.31.0 RELEASED and verified (53rd tag); aarch64 glibc archive
  executed cold here. 268b: --test lists files before descending
  (correction, unreleased).
- v2.32.0 RELEASED and verified (54th tag; musl archive executed
  cold here). The 264 milestone is complete.
- 272: replenishment — milestone "data in, data out" (v2.33–v2.35),
  reasoning in LOG.md.
- v2.33.0 RELEASED and verified (54th tag — counts in 263b–276b
  were one high; 278 audit corrected them); aarch64 glibc archive
  executed cold here with the bundled json module.
- 277: --test --filter (first toward v2.34.0); 278: health tick
  green — the 272 milestone is complete.
- 279: replenishment — milestone "polish the loop's tools"
  (v2.34–v2.36), reasoning in LOG.md.
- v2.34.0 RELEASED and verified (55th tag); aarch64 musl archive
  executed cold here.
- v2.35.0 RELEASED and verified (56th tag); aarch64 glibc archive
  executed cold here. The 279 milestone is complete.
- 287: replenishment — milestone "trust and teach" (v2.36–v2.38),
  reasoning in LOG.md.
- v2.36.0 RELEASED and verified (57th tag); aarch64 musl archive
  executed cold here.
- 292: bench/json.ting + baseline row — milestone stroke 4; first
  toward v2.37.0.
- Backlog (one per tick, in order): (5) LSP folding ranges, protocol
  test; then health tick, release v2.37.0 and replenish.
- Distribution audit expectation: 3 assets up to v2.16.0, 4 from
  v2.17.0, 6 from v2.30.0.
- Toolchain note: rustc 1.98 locally; rustfmt+clippy reinstalled 196.
- Periodic health ticks (bench vs BASELINE.md — recorded on this
  host, six rows — big fuzz sweeps) when quiet.
