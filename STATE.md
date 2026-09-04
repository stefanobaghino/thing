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
- 61 builtins; nine embedded stdlib modules
  (list/map/string/math/json/fs/test/time/sh, 154 functions, guarded);
  31 ting programs (15 selftest files, 17 examples with .out); 286 Rust tests
  in 11 suites.
- One binary is the toolchain: a script may be a path or `-`
  (stdin); REPL (9 meta-commands), --fmt (dirs,
  stdin, --diff, keeps CRLF), --check (dirs, stdin, follows local
  imports, nine warnings, --strict, --watch), --doc (names, module, file, or
  everything), --test (dirs, --filter, --tap, -j, --slow,
  --fail-fast, --watch, per-file check counts), --profile (calls and self
  time per function and builtin, top twenty), --lsp (thirteen
  capabilities). A runtime error points at the line that raised it
  and carries a note per call it unwound through (named, capped at
  ten with the middle elided), which try() also hands back as "at"
  and "trace"; module errors point into the module's own file; cyclic data prints, compares and json-fails cleanly.
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
- v2.77.0 VERIFIED (98th tag; strokes 495, 496; both aarch64 archives
  executed here).
- 498: a call that cannot match; 499: the corpus warning set guarded
  by a test (499b fixed its Windows path assumption).
- v2.78.0 VERIFIED (99th tag; strokes 498, 499; both aarch64 archives
  executed here).
- 501: health tick + audit green — milestone "before it runs"
  complete.
- 502: replenishment — milestone "the tenth act" (v2.79-v2.80),
  reasoning in LOG.md.
- 503: the key written twice; 504: what can never run.
- v2.79.0 VERIFIED (100th tag; strokes 503, 504; both aarch64
  archives executed here).
- 506: the tenth act written, "Where it stands" current; 507: both
  new checks tested from inside ting (507b: a tick's two commits,
  the rule restated).
- v2.80.0 VERIFIED (101st tag; strokes 506, 507; both aarch64
  archives executed here).
- 509: health tick + audit green — milestone "the tenth act"
  complete.
- 510: replenishment — milestone "how much it checked" (v2.81-v2.82),
  reasoning in LOG.md.
- 511: counting the checks (TING_TEST_REPORT); 512: what each file
  verified.
- v2.81.0 VERIFIED (102nd tag; strokes 511, 512; both aarch64
  archives executed here). 514: lib/test.ting's helpers count too;
  515: the docs say what the counts mean.
- v2.82.0 VERIFIED (103rd tag; strokes 514, 515; both aarch64
  archives executed here).
- 517: health tick + audit green — milestone "how much it checked"
  complete.
- 518: replenishment — milestone "the way back" (v2.83-v2.84),
  reasoning in LOG.md.
- 519: the whole way back — every call an error unwinds through
  leaves a named frame; deep traces elide the middle. 520: arity
  errors count in English and name the function called.
- v2.83.0 VERIFIED (104th tag; strokes 519, 520; both aarch64
  archives executed here).
- 522: try() hands back "at" and "trace"; lib/test.ting names the
  line that raised. 523: the docs read the trace (tutorial,
  reference, stdlib).
- v2.84.0 VERIFIED (105th tag; strokes 522, 523; both aarch64
  archives executed here).
- 525: health tick + audit green — milestone "the way back"
  complete.
- 526: replenishment — milestone "where the time went"
  (v2.85-v2.86), reasoning in LOG.md.
- 527: counting the calls (--profile), and closures now belong to
  the file that defined them. 528: self time per function, slowest
  first. 530: builtins in the table, twenty rows and a count of the
  rest. 531: the docs read the profile.
- v2.85.0 VERIFIED (106th tag; strokes 527, 528; both aarch64
  archives executed here).
- v2.86.0 VERIFIED (107th tag; strokes 530, 531; both aarch64
  archives executed here).
- 533: health tick + audit green — milestone "where the time went"
  complete (533b: the profile test no longer asserts an order two
  microseconds can swap).
- 534: replenishment — milestone "at the terminal" (v2.87-v2.88),
  reasoning in LOG.md.
- 535: --test --watch — an mtime-and-length poll re-runs the files
  whenever one changes, is added or goes away, a rule line per run
  naming the cause. 536: --check and --fmt-check watch too, over one
  shared loop; --fmt --watch is refused (it would answer its own
  rewrites).
- v2.87.0 VERIFIED (108th tag; strokes 535, 536; both aarch64
  archives executed here).
- 539: ting - runs a script from stdin (args, diagnostics named -,
  imports against the cwd; input() then sees EOF).
- 540: the docs read the terminal (reference, tutorial, README).
- v2.88.0 VERIFIED (109th tag; strokes 539, 540; both aarch64
  archives executed here).
- 543: health tick + audit green — milestone "at the terminal"
  complete (all six bench checksums match; five corpus warnings;
  six assets per tag; the site serves v2.88.0).
- 544: replenishment — milestone "the working directory"
  (v2.89-v2.90), reasoning in LOG.md.
- 545: list_dir — the names in a directory, sorted; not a readable
  directory, or a name that is not UTF-8, errors.
  546: exists, is_dir (questions, so false rather than an error) and
  make_dir (parents included, already there is fine).
  549: lib/fs.ting — eleven functions, paths split on both separators
  and joined with "/", plus entries/walk/walk_ext.
  550: the docs read the filesystem (reference, tutorial, README).
- v2.89.0 VERIFIED (110th tag; strokes 545, 546; both aarch64
  archives executed here).
- v2.90.0 VERIFIED (111th tag; strokes 549, 550; both aarch64
  archives executed here).
- 553: health tick + audit green — milestone "the working
  directory" complete (all six bench checksums match; five corpus
  warnings; six assets per tag; the site serves v2.90.0 and seven
  modules).
- 554: replenishment — milestone "where it says no" (v2.91-v2.92),
  reasoning in LOG.md.
- 555: the call-depth cap is derived from the stack the process
  declares (measured: 1.6/2.6 KB a frame optimized, 12.6/28 KB not),
  so the runner and REPL allow 4096 frames in release and 512 in
  debug; an embedder or the wasm build that declares nothing keeps
  the old 200.
  556: remove_file and remove_dir (demands; remove_dir wants an empty
  directory), with the recursive remove_tree written in ting.
- v2.91.0 VERIFIED (112th tag; strokes 555, 556; both aarch64
  archives executed here; the shipped cap reads 4096).
- 559: \uXXXX escapes in string literals, spelled as JSON spells
  them (surrogate pairs included), plus ord and chr.
  560: the docs read the limits (two stale tutorial passages
  corrected; sections on recursion depth and spelling a character).
- v2.92.0 VERIFIED (113th tag; strokes 559, 560; both aarch64
  archives executed here).
- 563: health tick + audit green — milestone "where it says no"
  complete (all six bench checksums match; five corpus warnings;
  six assets per tag; the site serves v2.92.0).
- 564: replenishment — milestone "bits and numbers" (v2.93-v2.94),
  reasoning in LOG.md.
- 565: hex and binary literals with a lowercase prefix, and `_`
  between digits in any radix; a literal that runs into a letter or
  a foreign digit is an error naming the offender.
- 566: exponent floats (1e3, 1.5e-3, 2E+2); an exponent always makes
  a float, a half-written one is reported against the letter, and a
  literal that parses to infinity is an error.
- v2.93.0 VERIFIED (114th tag; strokes 565, 566; both aarch64
  archives executed here, the shipped refusals checked too).
- 569: bitwise operators & | ^ ~ << >>, int-only, Rust's precedence
  (every bit operator binds tighter than a comparison); floats and
  a shift of 64 or more are errors; the VM needed no change because
  Op::Binary delegates to the shared evaluator.
- 570: the docs read the bits (reference operator table and prose,
  a tutorial section); the stale "Call depth: 200" bullet in Limits
  corrected — the prose had been fixed in 560, the bullet had not.
- v2.94.0 VERIFIED (115th tag; strokes 569, 570; both aarch64
  archives executed here, the shipped refusals checked too).
- 573: health tick + audit green — milestone "bits and numbers"
  complete (all six bench checksums match; five corpus warnings;
  six assets per tag; the site serves v2.94.0).
- 574: replenishment — milestone "numbers that read back"
  (v2.95-v2.96), reasoning in LOG.md.
- 575: float printing goes through one value::float_repr, shared by
  Display and json_str: exponent form outside 1e-4..1e17, shortest
  round-tripping form inside, .0 kept on integral values; a test
  lexes every printed float back and compares bits.
- 576: the conversions agree with the literals — float("1e400"),
  float("inf"), float("nan") and json_parse("1e999") are errors, and
  int() of a non-finite or out-of-range float names the value instead
  of saturating.
- v2.95.0 VERIFIED (116th tag; strokes 575, 576; both aarch64
  archives executed here, the shipped refusals checked too).
- 579: hex(n) and bin(n) write the literal forms (sign kept, not
  wrapped), and int(s) reads a string the way the lexer reads a
  literal, so int(hex(n)) == n for every int. 54 builtins.
- 580: the docs read the numbers (reference prose on printing and on
  the conversions, a Limits bullet saying infinity is reachable by
  arithmetic and by nothing else; a tutorial snippet for hex/bin/int
  and a paragraph on 0.1 + 0.2).
- v2.96.0 VERIFIED (117th tag; strokes 579, 580; both aarch64
  archives executed here, the round trip checked to i64::MIN).
- 583: health tick + audit green — milestone "numbers that read
  back" complete (all six bench checksums match, twice; five corpus
  warnings; six assets per tag; the site serves v2.96.0).
- 584: replenishment — milestone "the clock and the dice"
  (v2.97-v2.98), reasoning in LOG.md.
- 585: sleep_ms(ms), int milliseconds to match time_ms, flushing
  before the pause; wasm refuses it. Correction: 584's survey said
  ting had no clock — time_ms() has existed for many versions, and
  the backlog below is the corrected one.
- 586: lib/time.ting — fourteen functions, UTC only and saying so,
  Hinnant's conversions with floor division for pre-epoch instants.
- v2.97.0 VERIFIED (118th tag; strokes 585, 586; both aarch64
  archives executed here, module import and pause included).
- 589: random(), random_int(lo, hi) and seed(n) landed — SplitMix64
  in Interpreter, so both engines draw the same stream; unseeded it
  starts from the clock (wasm has none, and says so in the docs).
  None of the three is in any fuzzer alphabet.
- 590: tutorial section "The clock and the dice". It pins one seeded
  sequence in a doc test, so changing the generator means editing
  docs/tutorial.md; that is the only pinned draw anywhere.
- v2.98.0 VERIFIED (119th tag; strokes 589, 590; both aarch64
  archives executed here, seeded replay and a measured pause).
  Milestone "the clock and the dice" is complete.
- 593: health tick green — six bench checksums match baseline, 50000
  differential + 20000 formatter cases at seed 592, corpus at exactly
  five warnings, six assets, site serving v2.98.0.
- 598: lib/sh.ting (ok, check, lines, which, path_dirs, dir_sep,
  windows) — the ninth module. Its selftest asks which("sh") and
  stands down where there is none, so Windows runs the refusals.
- 597: fixed a Windows-only red — a test compared cwd() with a
  canonicalized path. Paths in tests: match names, never separators
  or prefixes (third time; see 499b).
- 596: eprint(...) and cwd(). eprint flushes stdout first so notes
  cannot overtake data; on wasm it writes alongside print, since a
  page has one stream.
- 595: run(cmd, args) landed — argv list, spawn failure is an error,
  code nil on a signal. lib/list.ting's chunk_by renamed its local
  `run` to `group` so the corpus stays at five warnings.
- 594: replenishment tick. New milestone "driving other programs"
  (v2.99.0, v2.100.0): ting is a good shell citizen but cannot call
  anything, has no stderr of its own and no cwd.
- Backlog (one per tick, in order):
  (1) the docs learn to drive (reference rows, second half of the
  tutorial's shell section);
  (2) RELEASE v2.99.0; (3) verify; (4) health tick.
- Still on the list, not chosen in 594: a regex engine (a milestone
  of its own), match expressions, a set type, threads.
- Found in the 574 survey, all at the text boundary: 1e23 prints as
  99999999999999991611392.0 and 1e300 * 10.0 as three hundred digits;
  float("1e400") is inf while the literal is an error; json_str
  refuses non-finite floats but json_parse("1e999") makes one;
  int(1.0 / 0.0) saturates to i64::MAX with no error.
- Surveyed and not chosen (564): no destructuring, no default
  parameter values, no variadic parameters — real absences, but each
  adds syntax to a language whose smallness is a feature, and none
  blocks work the way a missing & does. Indexed iteration is already
  covered by lib/list.ting's enumerate.
- Surveyed and found sound (554): deeply nested data is not
  fragile — fifty thousand levels of nested list parse from JSON,
  build in a loop and print without trouble. Only call frames are
  capped.
- Tags: 119 (v2.98.0), 119 verified; v2.29.0 is publicly marked broken
  (its Linux binaries needed glibc 2.39).

Standing rules (each from a slip; the LOG entry named has the story):

- Verdicts from the API (`gh run view --json conclusion`), never from
  a watcher's exit code. Tests that read paths out of tool output
  match file names, not separators: Windows prints backslashes
  (499b). A test over timings asserts what timings cannot swap:
  never the order of two rows that a loaded runner can reverse
  (533b). Every release cold-verified by downloading
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
  five warnings, guarded by a test since 499, all on purpose:
  edge.ting shadows `len` (451), repeats a map key and writes a
  statement after a return (507), errors.ting reads the unbound
  `totl` (495) and functions.ting calls `add(1)` to prove arity
  (498). A file's warnings come in line order (507).
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
