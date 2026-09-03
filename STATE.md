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
  (env-tunable seed/cases), a crash fuzzer (incl. cyclic values), a
  formatter fuzzer, and a CI job rerunning everything on eval.
- 44 builtins; six embedded stdlib modules
  (list/map/string/math/json/test, 121 functions, guarded); 28 ting programs
  (11 selftest files, 17 examples with .out); 249 Rust tests in 11
  suites.
- One binary is the toolchain: REPL (9 meta-commands), --fmt (dirs,
  stdin, --diff, keeps CRLF), --check (dirs, stdin, follows local
  imports, six warnings, --strict), --doc (names, module, file, or
  everything), --test (dirs, --filter, --tap, -j, --slow,
  --fail-fast), --lsp (thirteen
  capabilities). Module errors point at the module's line with a
  call-site note; cyclic data prints, compares and json-fails cleanly.
- Distribution: six release archives per tag since v2.30.0 (x86_64
  and aarch64 Linux gnu + musl, darwin arm64, windows), built on
  22.04 runners with a glibc floor guard; Pages site at
  www.baghino.me/thing (playground with run, fmt and check at the
  root; tutorial, reference, stdlib, cookbook, retrospective,
  changelog), wasm via a hand-rolled ABI.
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

History: every iteration is in LOG.md (append-only). This section
holds only the current milestone and the standing rules.

- 404: milestone "the loop's own house" (v2.60–v2.61), reasoning in
  LOG.md. 405: STATE.md refreshed and compacted. v2.60.0 VERIFIED
  (81st tag; strokes 401, 402, 405; both aarch64 archives executed
  here).
- v2.61.0 VERIFIED (82nd tag; strokes 407, 408, 409; both aarch64
  archives executed here). 411: health tick + audit green —
  milestone "the loop's own house" complete.
- 412: replenishment — milestone "the session" (v2.62–v2.63),
  reasoning in LOG.md.
- v2.62.0 VERIFIED (83rd tag; strokes 413, 414, 415; both aarch64
  archives executed here).
- 417: merge_with — one stroke banked toward v2.63.0. 418: health
  tick + audit green — milestone "the session" complete.
- 419: replenishment — milestone "the editor, again" (v2.63–v2.64),
  reasoning in LOG.md.
- v2.63.0 VERIFIED (84th tag; strokes 417, 420, 421; both aarch64
  archives executed here).
- 423: thirteen capabilities counted everywhere; 424: squeeze — two
  strokes banked toward v2.64.0. 425: health tick + audit green —
  milestone "the editor, again" complete. Found: `:load` resolves
  relative imports against the cwd, not the file, and names "repl"
  in the diagnostic.
- 426: replenishment — milestone "load and import" (v2.64–v2.65),
  reasoning in LOG.md.
- v2.64.0 VERIFIED (85th tag; strokes 423, 424, 427; both aarch64
  archives executed here).
- v2.65.0 VERIFIED (86th tag; strokes 429, 430, 431; both aarch64
  archives executed here). 433: health tick + audit green —
  milestone "load and import" complete. Found: CRLF files fail
  --fmt-check and get rewritten to LF; unused locals in function
  bodies are not warned about.
- 434: replenishment — milestone "the small print" (v2.66–v2.67),
  reasoning in LOG.md.
- v2.66.0 VERIFIED (87th tag; strokes 435, 436, 437; both aarch64
  archives executed here).
- 439: hypot — one stroke banked toward v2.67.0. 440: health tick +
  audit green — milestone "the small print" complete.
- 441: replenishment — milestone "tests and json" (v2.67–v2.68),
  reasoning in LOG.md.
- v2.67.0 VERIFIED (88th tag; strokes 439, 442, 443; both aarch64
  archives executed here).
- 445: formatting edit ends at the last position; 446: diff and
  flatten in the tutorial — two strokes banked toward v2.68.0. 447:
  health tick + audit green — milestone "tests and json" (the
  25th) complete.
- 448: replenishment — milestone "the ninth act" (v2.68–v2.69),
  reasoning in LOG.md.
- v2.68.0 VERIFIED (89th tag; strokes 445, 446, 449; both aarch64
  archives executed here).
- v2.69.0 VERIFIED (90th tag; strokes 451, 452, 453; both aarch64
  archives executed here). 455: health tick + audit green —
  milestone "the ninth act" complete.
- 456: replenishment — milestone "counted and guarded" (v2.70–v2.71),
  reasoning in LOG.md.
- v2.70.0 VERIFIED (91st tag; strokes 457, 458, 459; both aarch64
  archives executed here).
- 461: key_of — one stroke banked toward v2.71.0. 462: health tick
  + audit green — milestone "counted and guarded" complete. Found:
  --fmt over a directory stops at the first file that fails to lex.
- 463: replenishment — milestone "every file, every time"
  (v2.71–v2.72), reasoning in LOG.md.
- v2.71.0 VERIFIED (92nd tag; strokes 461, 464, 465; both aarch64
  archives executed here).
- 467: the formatter's contract in the docs; 468: plural — two
  strokes banked toward v2.72.0. 469: health tick + audit green —
  milestone "every file, every time" complete. Found: unknown
  options (-h, -V, --nosuch) are taken for script paths.
- 470: replenishment — milestone "the front door's handle"
  (v2.72–v2.73), reasoning in LOG.md.
- v2.72.0 VERIFIED (93rd tag; strokes 467, 468, 471; both aarch64
  archives executed here).
- v2.73.0 VERIFIED (94th tag; strokes 473, 474, 475; both aarch64
  archives executed here). 477: health tick + audit green —
  milestone "the front door's handle" complete.
- 478: replenishment — milestone "reading width" (v2.74–v2.75),
  reasoning in LOG.md.
- v2.74.0 VERIFIED (95th tag; strokes 479, 480, 481; both aarch64
  archives executed here).
- 483: ordinal — one stroke banked toward v2.75.0. 484: health tick
  + audit green — milestone "reading width" complete.
- 485: replenishment — milestone "the nearest name" (v2.75-v2.76),
  reasoning in LOG.md.
- v2.75.0 VERIFIED (96th tag; strokes 486, 487, 488; both aarch64
  archives executed here).
- v2.76.0 VERIFIED (97th tag; strokes 490, 491; both aarch64 archives
  executed here).
- 493: health tick + audit green — milestone "the nearest name"
  complete.
- 494: replenishment — milestone "before it runs" (v2.77-v2.78),
  reasoning in LOG.md.
- 495: bound nowhere (--check and the LSP) — one stroke banked
  toward v2.77.0.
- Backlog (one per tick, in order): (1) LSP tests for the new
  diagnostic — then RELEASE v2.77.0; (2) --check warns
  about a call whose argument count cannot match a function in the
  same file; (4) docs + selftests, then RELEASE v2.78.0; (5) health
  tick + audit.
- Tags: 97 (v2.76.0), 96 verified; v2.29.0 is publicly marked broken
  (its Linux binaries needed glibc 2.39).

Standing rules (each from a slip; the LOG entry named has the story):

- Verdicts from the API (`gh run view --json conclusion`), never from
  a watcher's exit code. Every release cold-verified by downloading
  and executing an aarch64 archive on this host (musl and gnu).
- A tick's shell chain is ONE `&&` list (heredoc bodies follow the
  line); `set -e` is NOT honoured by the harness (377b); never a bare
  line after the gate (358, 377 pushed green records for red gates).
  Read the smoke output before writing prose that quotes it (370).
  Check a grep's result before promising a stroke on it (404).
- After writing LOG/STATE, rerun the docs guard and gate the push on
  the literal `test result: ok` (238). No angle-bracket placeholders
  in markdown (238, 262).
- Linux release builds stay on 22.04 runners; the glibc-floor step is
  the guard (v2.29.1). A failed Pages deploy is retried only with
  `gh workflow run pages.yml --ref main`.
- Bench on this shared host: checksums decide, timings are weather.
- Corpus scan (`--check lib selftest examples bench`) expects exactly
  two warnings, both on purpose: selftest/edge.ting shadows `len`
  (451) and selftest/errors.ting reads the unbound `totl` (495).
- Site audit paths: https://www.baghino.me/thing/ (github.io
  redirects there); playground at the root — /, /examples.js,
  /ting.wasm — plus reference, tutorial, cookbook, stdlib,
  retrospective, changelog .html (vm.md is not published).
- Distribution audit expectation: 3 assets up to v2.16.0, 4 from
  v2.17.0, 6 from v2.30.0.
- Toolchain: rustc 1.98 locally; rustfmt and clippy reinstalled at 196.
- Periodic health ticks (bench vs bench/BASELINE.md — recorded on this
  host, six rows — plus 50000 differential, crash and 20000 formatter
  fuzz cases in release) close every milestone.
