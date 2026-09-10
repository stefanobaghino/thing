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
- 74 builtins; thirteen embedded stdlib modules
  (list/map/string/math/json/fs/test/time/sh/args/err/csv/base64, 203
  functions, guarded); 46 ting programs (24 selftest files — 23 tests
  plus _lib.ting, the module modules.ting imports, which checks
  nothing on its own — and 22 examples with .out; 2769 selftest checks on all four
  CI platforms, Windows included); 449 Rust tests
  in 17 suites (counted at 916; the 399 written here had been
  stale for a while). `ting --fmt .` reports 79 unchanged; BASELINE is ELEVEN
  rows since bench/scan.ting joined in 794, regenerated at 832 for
  v2.133.0.
- One binary is the toolchain: a script may be a path or `-`
  (stdin); REPL (9 meta-commands), --fmt (dirs,
  stdin, --diff, keeps CRLF), --check (dirs, stdin, follows local
  imports, nine warnings, --strict, --watch), --doc (names, module, file, or
  everything), --test (dirs, --filter, --tap, -j, --slow,
  --fail-fast, --watch, per-file check counts), --bundle (a script and
  the local modules it imports as one file, stdlib imports left alone),
  --profile (calls and self
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

1. Maintenance check every tick: issues, PRs, CI, tree. NOT DONE
   UNTIL A CI VERDICT FOR THE CURRENT HEAD HAS BEEN READ FROM THE
   API. At 838 and 839 `git status` + `git log` + `uptime` stood in
   for it and CI stayed red from 837 for three ticks while two more
   commits went on top. A green local gate is a REASON to look at
   CI, not a substitute: four platforms run it and this host is one.
2. One small verifiable stroke per tick (feature, docs, test, health
   check); before every push run what CI runs, in CI's own words —
   `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test` (16 suites) — plus `ting --fmt .` and the corpus check. NO
   EXCEPTIONS (clippy skipped once, iteration 182, cost a red CI;
   `cargo fmt --check` was never in this list at all until 763, and
   the first hand-written Rust in a while turned CI red on all four
   runners). After writing
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
- 599: tutorial subsection "Driving other programs". Its snippets are
  executed on every CI platform and no program exists on all three,
  so the happy path is written behind a which() guard — the idiom the
  section teaches anyway.
- v2.99.0 VERIFIED (120th tag; strokes 595, 596, 598, 599; both
  aarch64 archives executed here on the whole milestone). Milestone
  "driving other programs" is complete.
- 602: health tick green — six bench checksums match baseline, 50000
  differential + 20000 formatter cases at seed 601, corpus at five
  warnings, six assets, site serving lib/sh.ting.
- 623: the formatter already spaced defaults correctly (token-based),
  now pinned by a test; the shared fuzz generator emits optional
  arguments at every call length.
- 622: --check's arity warning and the LSP hover learned the range;
  a default is shown from its source span, not the AST's s-expression.
- 621: defaults land in the language. One Interpreter::call fills
  missing arguments, so both engines agree by construction. The
  compiler's capture analysis now walks parameter defaults too — a
  nested default naming an enclosing parameter was a VM-only
  "undefined variable" until it did.
- 620: replenishment tick. New milestone "arguments that can be left
  out" (v2.102.0, v2.103.0), chosen from evidence: csv, list and map
  all carry *_with twins that exist only because user functions
  cannot have optional arguments, while builtins have had them all
  along.
- 619: health tick green — bench checksums match, 50000 differential
  + 20000 formatter + 200000 pattern cases at seed 618, corpus at
  five warnings, site carrying the report example and lib/csv.ting.
- v2.101.0 VERIFIED (122nd tag; strokes 613-616; both aarch64
  archives executed here on all three modules). Milestone "the
  script's own front door" is complete.
- 616: tutorial section "The front door" and examples/report.ting
  (args + csv + err together); cookbook and playground regenerated.
- 615: lib/csv.ting (parse, text, parse_with, text_with, maps,
  quote), the twelfth module. Parsed with a for-loop state machine,
  since string indexing counts characters and would be quadratic.
- 614: lib/err.ting (message, failed, value, wrap, site, trace), the
  eleventh module; all six selftests that hand-rolled the helper now
  import it. value() treats a returned nil as a value, not a failure.
- 613: lib/args.ting (parse, main, help + three helpers), the tenth
  module. Unknown options error; short options are not bundled;
  --help answers even when the line is otherwise incomplete.
- 612: replenishment tick. New milestone "the script's own front
  door" (v2.101.0, v2.102.0), chosen from evidence in this repo: five
  selftests copy the same err helper, and examples/todo.ting takes
  args() apart by hand.
- 611: health tick green — bench checksums match, 50000 differential
  + 20000 formatter + 200000 pattern cases at seed 610, corpus at
  five warnings, site serving the patterns reference.
- v2.100.0 VERIFIED (121st tag; strokes 604-608; both aarch64
  archives executed here on the whole milestone). Milestone
  "patterns" is complete.
- 608: reference section (syntax table, semantics, the omissions and
  why), tutorial section, selftest/regex.ting (33 checks). README's
  builtin count was stale at 52; now 66.
- 607: pattern fuzzer (TING_RE_SEED / TING_RE_CASES, default 20000)
  plus a named test for (a+)+b; the five re_* builtins added to the
  crash-fuzzer alphabet and the differential corpus.
- 606: re_find_all, re_replace ($0-$9, $$; an unknown group errors)
  and re_split. One shared scan helper steps past an empty match by
  one character, which is what makes re_split(s, "") split into
  characters instead of hanging.
- 605b: two clippy warnings shipped in 8c0376c because the tick
  PRINTED the warning count instead of gating on it. Gate on a
  comparison (`test -z`, `grep -c ... = 0`), never on a printed
  number.
- 605: re_test and re_find. Compiled patterns cached on the
  interpreter, cleared wholesale past 256 entries.
- 604: src/regex.rs — parser, compiler and Pike VM, 10 unit tests,
  no ting-facing builtins yet. Limits: 1000 copies per count, 100000
  instructions per pattern. `.` stops at a newline; a{b} is literal
  but a{2}{3} is an error.
- 603: replenishment tick. New milestone "patterns" (v2.100.0,
  v2.101.0): a Pike-VM regex engine — linear time, leftmost-first, no
  backreferences, char offsets to agree with len/slice/find, compiled
  patterns cached in the interpreter.
- 624: the docs and selftest for optional arguments. The selftest
  found a checker bug: `unused_params` read every identifier between
  the parentheses as a parameter, so a default that called a function
  reported that function as an unused parameter. Name position only
  now, and a name a sibling default reads counts as used.
- v2.102.0 VERIFIED (123rd tag; strokes 621-624; both aarch64
  archives executed here on defaults, both engines hashing alike).
- 627: health tick green — six bench checksums match baseline, 50000
  differential + 20000 formatter + 2000000 pattern cases at seed 627,
  corpus at five warnings, six assets, site serving v2.102.0 and both
  new docs passages. Milestone "arguments that can be left out" is
  complete.
- 628: replenishment — milestone "as many as you like"
  (v2.103.0, v2.104.0), reasoning in LOG.md: builtins take as many
  arguments as you give them and no ting function can, so no ting
  program can wrap format().
- 629: `...rest` parameters land — one new token (a lone `.` belonged
  to no expression form), bound in the shared Interpreter::call with
  the leftovers split off before the defaults run, so a default
  cannot see them. The checker's arity upper bound is now
  `Option<usize>`: unbounded is not a large number.
- 630: `f(...xs)` spreads a list into a call. A spread is an argument,
  not an expression, so it parses nowhere else. Calls that contain one
  compile to `Op::Spread` + `MakeList(1)` + `Op::CallSpread`; calls
  without one keep `Op::Call` and pay nothing.
- v2.103.0 VERIFIED (124th tag; strokes 629, 630; both aarch64
  archives executed here on a variadic wrapper, a forwarded spread
  and both refusals).
- 633: fuzz generator reaches variadic calls five ways, `...` in the
  crash alphabet, selftest/varargs.ting (20 checks), reference and
  tutorial sections.
- 634: lib/test.ting's five checks go through pass()/fail_with()
  (variadic, so it could not exist before); lib/csv.ting's parse and
  text take an optional separator, the _with twins kept as the older
  spelling. 174 stdlib functions.
- v2.104.0 VERIFIED (125th tag; strokes 633, 634; both aarch64
  archives run from inside their own directory, so the shipped lib/
  was the one imported). Milestone "as many as you like" complete.
- 637: health tick green — six bench checksums match baseline, 50000
  differential + 20000 formatter + 2000000 pattern cases at seed 637,
  corpus at five warnings, six assets, site serving v2.104.0 and the
  varargs docs. json/maps read -1% against the VM today where 627
  read +5%/+3%: timings are weather, and 628's "no gap to chase" was
  right for a better reason than it gave.
- 638: replenishment — milestone "which lines ran" (v2.105.0,
  v2.106.0), reasoning in LOG.md: --profile counts calls, so nothing
  says which branch inside a called function never ran.
- 639: recording lands — Interpreter::cover() collects offsets per
  file; the compiler emits Op::Mark before each statement only when
  compiled for coverage, so both engines record statement starts and
  agree by construction (an op's span can sit on another line).
- 640: `--coverage` reports — share per file plus the missed lines
  (twelve named, the rest counted), on stderr like the profile. What
  could run comes from the AST (statement walk at parse time), so the
  denominator matches for both engines by construction.
- v2.105.0 VERIFIED (126th tag; strokes 639, 640; both aarch64
  archives, both engines, the same coverage table byte for byte).
- 643: `--coverage` takes paths (dirs recurse), one interpreter per
  script sharing one record. `ting --coverage selftest` reads 2191 of
  2210 lines. The differential test found a real bug: records keyed by
  address merged two files when an allocation was reused; the key is
  the path now.
- 643b: 643 went red — the new coverage differential test ran
  selftest/fs.ting in-process while the older test ran it as a child,
  racing on its fixed directory name. fs.ting and sh.ting are skipped
  there now.
- 644: reference and tutorial document `--coverage` (a statement is
  the unit; a `fn` definition is covered when the file runs, its body
  when it is called); the CLI test covers the multi-script form.
- 645: coverage's findings fixed — set_in's refusal, max_by's replace
  branch (every case had the largest element first), args main() and
  test summary(), the last two from Rust because they print and exit.
  lib/ reads 2203 of 2215; the eleven left are those two exiting paths
  and sh.ting's Windows-only branch.
- 645b: the tick's STATE and CHANGELOG edits were a separate command
  from the gate chain, so a failed assertion in them did not stop the
  commit. Every edit script belongs in the chain that gates the push.
- 646: v2.106.0 released (127th tag). Two repairs: the CHANGELOG had
  645b's bullet twice, and the release smoke test counted warnings
  with `^warning:` when a checker warning starts with
  `file:line:col:` — it read zero where five were printed, so the
  gate could only fail open. It matches `: warning:` now.
- v2.106.0 VERIFIED (127th tag; strokes 641-645; both aarch64
  archives executed here, defaults, rest, spread, imports and both
  engines byte-identical; --coverage, --check, --fmt and stdin all
  behaved; the site serves v2.106.0).
- 648: health tick green — six bench checksums, three fuzzers at seed
  648, gate, corpus at five, coverage at 2203/2215, nine site paths.
  The published site set is exactly what pages.yml lists: the six doc
  pages, the root, index.html and ting.wasm. docs/vm.md is repo-only
  and there is no playground.html — the playground is the root.
- 649: replenishment. Corpus counts: 44 of 110 plain assignments name
  their target twice (5 repeat an index, so it is evaluated twice);
  80 try() wrappers, 33 around a single call with arguments.
  Milestone "saying it once" (v2.107.0, v2.108.0): compound
  assignment and `try(f, ...args)`.
- 650: compound assignment landed. Assign/IndexAssign carry an
  `Option<BinaryOp>`; two new ops, `IndexKeep` (reads base[idx]
  leaving both operands for IndexSet) and `GetVarToUpdate` (a read
  that reports the assignment error, so `x += 1` fails the way
  `x = 1` does — the engines disagreed here on the first run).
- 650b: that note was wrong, and checking beat assuming. The checker
  warns on neither `let x = 0; x += 1;` nor `x = x + 1` — an
  assignment already counts as a mention — and the formatter needs
  nothing, since it spaces tokens by default and a compound file
  formats unchanged. Only the LSP and a selftest are left.
- 651: selftest/compound.ting (14 checks, both engines) and the LSP.
  The LSP was already right on `+=` — it works from identifier
  tokens — but a document highlight called only `let`/`fn` names
  writes, so `count = count + 1` read as two reads. It now asks
  whether the next token assigns, which covers `=` and the five
  compound spellings.
- 652: `try(f, ...args)` and all six lib/err.ting functions with it.
  The first spelling was `...args`, which shadows the `args` builtin —
  the five-warning corpus guard caught it; it is `...rest` now.
- 653: fuzzers learn both (crash alphabet, generator statements and
  `try(h, e)`); the corpus adopts them — 44 compound assignments, 28
  try calls with arguments, zero self-referential assignments left.
  Two reverts worth keeping in mind: a lambda is a trace frame, and a
  spread in try's own argument list is try's to evaluate, so the
  failure escapes instead of being caught.
- 654: docs. Reference (statement block, a paragraph on the semantics,
  try's row and example), tutorial (+= where the loop already added,
  try(parse_age, raw)), stdlib err.ting table, README. Both the
  reference and the tutorial spell out who evaluates what: inside the
  lambda try can catch, in try's argument list it cannot.
- 655: v2.107.0 released (128th tag; strokes 650-654).
- v2.107.0 VERIFIED (128th tag; strokes 650-654; both aarch64
  archives executed here on one script using the whole milestone —
  compound assignment on a map key, a counter and a string, try with
  arguments, lib/err.ting with them, defaults/rest/spread — four runs
  byte-identical; --coverage, --check, --fmt and stdin all behaved;
  the site serves v2.107.0). Milestone "saying it once" complete.
- 657: health tick green — six bench checksums, three fuzzers at seed
  657, gate, corpus at five, nine site paths. Coverage caught two
  dead `return`s inside compound.ting's try lambdas; removed, and
  selftest now reads 2266 of 2278 with every miss deliberate.
- Noted, not chased: the coverage report names lib/test.ting by
  absolute path where the rest are relative, because
  selftest/testlib.ting imports it as `"../lib/test.ting"` and the
  report prints the path as resolved.
- 658: replenishment. Measured gap: a failure's trace names the calls
  and never the values — three note lines and not one says which row.
  Milestone "what the values were" (v2.108.0, v2.109.0). The seam:
  frames are pushed in exactly one place, `Interpreter::call`'s
  `map_err`, which both engines go through and which runs only after
  something has failed, so arguments cost nothing until then.
- 659: frames carry their arguments; the diagnostic's note lines show
  them. Caps: four named then `and N more`, each value cut to 32
  chars, rendered as a list renders elements so strings keep quotes.
  Cost measured, not assumed: interleaved A/B on bench/fib.ting gives
  +2.9% median, +0.4% minimum. Not zero, as 658 claimed.
- 660: `try`'s trace frames carry `"args"` (name to value, the values
  themselves — the note line's 32-char cut is for people, not
  programs); lib/err.ting gained `given(f, ...rest)`, 175 stdlib
  functions. Selftests in errors.ting and errlib.ting, two
  differential lines.
- 661: the fuzzers already covered it — measured, not assumed: 1862
  of 2000 generated programs fail uncaught and 268 print a note with
  arguments, so ~6500 per 50000-case sweep are already compared
  across engines. Added what was missing instead: a CLI test pinning
  both caps on both engines, and a selftest stating that the caps are
  the diagnostic's and not the data's.
- 662: docs. Reference (the note-line shape, the three caps as one
  thought, `"args"` in the frame map, and the caps-are-the-
  diagnostic's line), tutorial (its stale illustrative trace, and a
  worked example that reads `["args"]`), README.
- 663: v2.108.0 released (129th tag; strokes 659-662).
- v2.108.0 VERIFIED (129th tag; strokes 659-662; both aarch64
  archives executed here on the milestone itself — a trace naming
  each frame's arguments, err["given"], the uncapped data a program
  reads back, and two uncaught diagnostics matching line for line
  with the caps applied; four runs byte-identical; --check and --fmt
  quiet; the site serves v2.108.0 and counts 175 stdlib functions).
  Milestone "what the values were" complete.
- 665: health tick + audit green — all six bench checksums matching
  BASELINE, three fuzzers at seed 665 clean (50000 differential,
  20000 formatter, 2000000 pattern), the gate green (fmt, clippy at
  zero, fourteen suites), the corpus at its five deliberate warnings,
  coverage 2286/2298 with only the known misses, six assets on each
  of the last two tags, CI green on HEAD, nine site paths at 200
  serving v2.108.0 and 175 stdlib functions.
- 666: replenishment — milestone "the key that isn't there"
  (v2.109-v2.110), reasoning in LOG.md. Measured: 40 `has(m, k)`
  guards over a read of `m[k]` (13 control-flow, 12 plain presence,
  15 value-fallback); `lib/map.ting`'s `get` called zero times in the
  corpus; `m[k] += 1` on a missing key is an error, so v2.107.0's
  compound assignment does not reach counting.
- 667: `get(x, k, default)` as a builtin, sharing one lookup with
  indexing via `index_opt`; `lib/map.ting`'s `get` retired into it
  (builtins 66->67, stdlib 175->174). Adopted where it reads better
  and left where it does not (reasoning in LOG.md). Tests at three
  levels, the crash-fuzzer alphabet, the editor grammar, reference
  and stdlib docs. Corrects 666's "zero calls": map.get had two, as
  `m["get"](...)`, which the grep for `get(` missed.
- 668: docs. Tutorial (the "test with has first" sentence rewritten,
  the counting example, lists and strings), reference (the builtin
  table, indexing, compound assignment), the v2.109.0 changelog
  heading. The cookbook renders from examples/ and already carried
  the one-liner.
- 669: v2.109.0 released (130th tag; strokes 667-668), the
  milestone's first release.
- 670: v2.109.0 verified. Six assets, three green runs, both aarch64
  archives run cold on the milestone (four byte-identical runs across
  two archives and two engines), corpus at five, nine site paths at
  200 serving v2.109.0 and 174 stdlib functions, and the playground
  wasm rebuilt from this tree (it embeds list.ting's new `get` call
  and no longer carries map.ting's retired one).
- 671: lib/err.ting reads `get` — message, value, site and trace are
  one line each, given names the trace once; failed (presence) and
  wrap (control flow) stay. examples/todo.ting's load likewise. No
  behaviour change, so no changelog bullet.
- 672: health tick green — six bench checksums matching BASELINE,
  three fuzzers at seed 672 clean (50000 differential, 20000
  formatter, 2000000 pattern), the gate green, the corpus at five,
  coverage 2268/2280 with only the known misses (the denominator fell
  18 lines because 671 collapsed that many covered lines out of
  lib/err.ting), six assets on each of the last two tags, CI green on
  HEAD, nine site paths at 200 serving v2.109.0 and 174 functions.
- 673: the member warning names the builtin. `lib/map.ting has no
  `get`` now adds "(`get` is a builtin)" — an exact builtin beats the
  nearest-export guess, which is what an upgrade past v2.109.0 needs.
  Measured and declined: a nil-coalescing form (only three of eight
  `if x == nil` sites fall back to a value; the rest are control flow).
- 674: diag::shorten relativises a path for display; --coverage and
  --profile both use it, closing 657. Declined: a runtime "is a
  builtin" hint (a module is a plain map with no provenance).
- 675: repaired 674's red CI. The new integration test wrote
  `lib/test.ting` literally; Windows shortens to `lib\test.ting`, so
  the needle is built from Path::join now. A misplaced doc comment
  from the same tick went back too.
- 676: 675 was not enough. The test's second assertion was right:
  `shorten` did nothing on Windows, where canonicalize returns a
  verbatim path that shares no prefix with current_dir. It
  canonicalises the working directory before comparing now.
- 677: v2.110.0 released (131st tag; strokes 671, 673, 674), closing
  milestone "the key that isn't there".
- 678: v2.110.0 verified. Six assets, green runs, both aarch64
  archives run cold on lib/err.ting and the member warning (four
  byte-identical runs), both reports naming lib/test.ting relative
  from each archive, corpus at five, nine site paths at 200 serving
  v2.110.0 and 174 functions, the wasm built from this tree.
  Milestone "the key that isn't there" complete.
- 679: replenishment — milestone "the top of the file"
  (v2.111-v2.112), reasoning in LOG.md. Measured: the VM is 14.6%
  slower than eval on bench/json.ting over nine interleaved runs each,
  and every bit of it is outside functions (--profile has the VM
  ahead, 150ms to 184ms, inside them). The same source moved into a
  function swings 2.9x: an empty 300k loop is vm +14.1% at top level
  and vm -64.6% in a function. src/compile.rs:89 names the cause —
  frame slots are for function chunks, "0 at top level", so every
  script-scope name resolves through the environment.
- 680: an engine divergence the slots change uncovered and that was
  older than it — the VM gave no nearest-name suggestion for a
  misspelled local, because frame slots carry no name at runtime.
  Chunk::in_scope records the names in scope per instruction, so both
  engines suggest the same one and neither offers an out-of-scope
  name. The slots change was reverted and waits for the next tick.
- 681: the top-level chunk has frame slots. bench/json.ting goes from
  vm +14.6% to vm -4.6%, an empty top-level loop from +14.1% to
  -65.9%; all six checksums unchanged. A top-level `let` that takes a
  slot still binds its name to nil in the environment, because the
  environment is what a diagnostic means by "in scope" — and nothing
  can read that binding, since any name a closure mentions is captured
  and captured names never get slots. Top-level blocks always need a
  runtime scope now, or the nil binding outlives its block (caught by
  edge.ting's shadowed `len`).
- 682: bench/toplevel.ting guards 681 — a function-free 200000-step
  loop, checksum `1199980 97 200 10 2062`, eval 469.4 ms against vm
  309.4 ms. BASELINE regenerated, seven rows, older checksums
  unchanged; its absolute times run high because the host was busy,
  the ratios match history.
- 683: v2.111.0 released (132nd tag; strokes 680, 681, 682).
- 684: v2.111.0 verified. Six assets, green runs, both aarch64
  archives run cold on the fixed diagnostic and its scope rule (four
  byte-identical runs across two programs), each printing
  bench/toplevel.ting's baseline checksum and the corpus at seven,
  nine site paths at 200 serving v2.111.0.
- 685: health tick on a quiet host — all seven checksums match, every
  timing under the recorded median (weather; the baseline says
  checksums decide), and the VM still beats eval on toplevel by 40%
  where before v2.111.0 it lost. Fuzzers at seed 685 (50000
  differential, 20000 formatter, 2000000 pattern, crash), gate green,
  coverage unchanged, six assets on the last two tags, nine site paths
  at 200. Milestone "the top of the file" complete, having shipped
  v2.111.0 alone: two of its four ticks went to bugs it did not
  create, which is what touching a resolver costs.
- 686: the host wedged twice, and a human asked whether the loop had a
  hand in it. It does. Measured rather than guessed, and the first
  guess was wrong: memory is not the pressure (peaks above), CPU is —
  four cores saturated for minutes by a background chore. Every step
  of a tick now runs nice'd; the gate was rerun that way to prove it
  still passes. No coverage was traded away for it.
- 687: replenishment — milestone "the code you imported"
  (v2.112-v2.113), reasoning in LOG.md. Measured: stdlib.ting is the
  one BASELINE row where the VM loses (+6%), and --profile puts ~976 ms
  of its 1212 ms in ting code inside lib/, which `import_module` runs
  on the tree-walker whatever engine started the script
  (src/eval.rs:2698). The same sort_with imported is vm -11%; pasted
  inline so it compiles, vm -42%, with eval unmoved at 661 vs 649 —
  about 37% on module-heavy work. The seam exists already
  (FnBody::Ast | FnBody::Chunk, src/eval.rs:329, both engines call
  either); what is missing is that Interpreter carries no engine to
  branch on. Hazard: docs/vm.md's accepted divergence (stray
  return/break rejected at compile time) would reach import.
- 688: the VM compiles what a script imports. Interpreter gained
  `compile_imports`, set only by `vm::run_chunk_compiling_imports`, so
  --eval and the REPL still interpret modules; `import_module` branches
  on it. `compile_module` keeps a module's TOP LEVEL bound by name —
  `import_module` reads exports out of that environment and a frame
  slot holds no name, so compiling it like a script would export nils.
  bench/stdlib.ting goes from vm +6% to vm -44%, all seven checksums
  unchanged, BASELINE regenerated. The 687 hazard landed exactly as
  predicted and is now an accepted divergence in docs/vm.md: both
  engines refuse a module with a top-level return/break with the same
  message, but the VM refuses before it runs, so earlier statements
  take effect under --eval only. tests/differential.rs had NO import
  in it; it has twelve now, plus fixtures under tests/fixtures/.
- 689: a guard for 688. No differential test can see whether a module
  was compiled — both engines agree either way — so the wiring could
  have been deleted with the suite still green.
  `an_imported_module_is_compiled_only_for_the_vm` asserts the
  exported function carries FnBody::Ast with compile_imports off and
  FnBody::Chunk with it on; verified by switching the feature off and
  watching it fail. Only repl.rs builds an Interpreter outside lib.rs
  (tree-walker by design), so --test/--profile/--coverage all get
  compiled imports from the two entry points 688 wired.
- 690: v2.112.0 released (133rd tag; strokes 688, 689).
- 691: v2.112.0 verified. Six assets, green runs, both aarch64
  archives run cold: module output byte-identical across two archives
  and two engines (including a module's own top-level binding read
  back as an export), the docs/vm.md divergence reproducing exactly,
  the corpus at seven, 22 selftests / 2423 checks, baseline checksums
  for stdlib and toplevel, and the shipped binary showing the flip
  itself (stdlib.ting 913 ms eval against 505 ms vm). Nine site paths
  at 200 serving v2.112.0, stdlib page at 174.
- 692: health tick green — all seven bench checksums match, stdlib.ting
  at -45% on a quiet host where the milestone began at +6%, and no
  benchmark row left that the VM loses. Fuzzers at seed 692 (50000
  differential, 20000 formatter, 2000000 pattern, crash), gate green,
  coverage 2272/2284, six assets on the last two tags, nine site paths
  at 200. The differential sweep ran 7.9 s against seed 685's 21.4 s;
  checked rather than assumed — the case count is honoured (5000 /
  50000 / 200000 take 0.8 / 7.7 / 32.4 s), and the old number was a
  loaded host running unniced. Milestone "the code you imported"
  complete.
- 693: replenishment — milestone "the matcher's inner loop"
  (v2.113-v2.114), reasoning in LOG.md. First measured that the VM
  vein is worked out: six unbenched shapes (try, closures, deep
  recursion, varargs, string building, patterns) are vm -8% to -64%,
  so no shape is left where the VM loses. The -8% is one builtin, not
  the VM: re_test on 11 chars costs 4.65 us warm. Varying one thing at
  a time — 32x the length costs +13%, three groups +12%, a match that
  fails on the first char 1.00 us — puts the cost per CHARACTER STEP
  at ~330 ns over a ~1 us floor, so the per-call `Vec<char>` the
  builtins build is not where the time goes.
  src/regex.rs `find_at` allocates nlist and nseen per position and
  clones caps per thread per step. Against it: re_* appears in one
  corpus file only (selftest/regex.ting), so pressure is weak — drop
  this first if a tick finds better. For it: the matcher is a true
  Thompson NFA (24 a's vs `^(a+)+$` under 1 ms), so no semantics are
  at risk, and the guard exists already (selftest/regex.ting plus the
  2000000-case pattern fuzzer).
- 694: `find_at` reuses two thread lists and a `Scratch` (the `seen`
  vector and `add`'s closure stack) instead of allocating a list, a
  seen vector and a stack per character position. Per match 4.65 us to
  2.75 us, the per-step cost ~330 ns to ~173 ns, the 693 probe 320 ms
  to 195 ms. Semantics unchanged: 36 regex checks, the pattern fuzzer
  clean at seed 694 over 2000000 cases, full suite green.
- 695: capture slots are an `Rc`, shared until a `Save` writes, and
  every leftmost restart shares one empty set instead of allocating
  per position. Per match 2.75 us to 2.30 us; over both strokes
  4.65 us to 2.30 us, and the per-step cost 330 ns to 127 ns.
  NEGATIVE RESULT worth not re-deriving: copying capture slots was NOT
  the remaining cost. The three-group probe moved least (61 to 59 ms)
  though it has eight slots to copy, because a `Save` usually finds
  its slots shared after a `Split` and `Rc::make_mut` copies anyway;
  the gain came from the shared empty set and from the thread copy in
  the main loop becoming a refcount. What is left is a ~0.90 us floor
  before any character is examined, and it is not the `Vec<char>` (341
  extra chars of subject cost 0.70 us, ~2 ns each).
- 696: bench/regex.ting (checksum `24000 5989512 37 109`, 204 of its
  213 ms inside the four re_ builtins; BASELINE is eight rows) and
  tests/alloc.rs, a test binary of its own that counts allocations and
  asserts ten times the subject does not cost ten times as many. THE
  GUARD FAILED FIRST: 54 allocations over 23 chars against 468 over
  230. 695's claim was untrue and its "negative result" was a bug —
  every restart shared one permanently held empty capture set, and a
  restart's first act is `Save(0)`, so `Rc::make_mut` always found it
  shared and always copied. Capture sets now return to a pool on the
  Scratch. The suite is FIFTEEN binaries now, not fourteen — the gate
  greps for 15. Honest trade: bench/regex 246 to 230 ms on the VM, but
  the anchored micro-probes 46 to 55 ms, since pooling walks the list
  where `clear()` was free.
- 697: a pattern that can only match at the start no longer begins a
  thread at every position. The flag is conservative — `Save(0)` is
  first, so it is set when prog[1] is `Start`; alternation puts a
  `Split` there, so `^a|b` is correctly NOT anchored. Probes 55 to
  29 ms (93 ms at the milestone's start), per match 4.65 us to ~1.45 us
  = 3.2x; bench/regex 230 to 198 ms on the VM, recovering 696's
  regression several times over. A wrong flag would be SILENT, so the
  deciding shapes are pinned in selftest/regex.ting (44 checks, 2431
  across the suite) and the fuzzer ran 4000000 cases at seed 697.
  BASELINE regenerated; CHANGELOG has the Unreleased entry.
- 698: v2.113.0 TAGGED (135th tag; strokes 694, 695, 696, 697). Cut
  from f4f6d50 with CI and Pages green; gate green at the tag (fmt,
  zero clippy warnings, fifteen suites, corpus at seven warnings, 22
  selftests / 2431 checks) and the release binary reports 2.113.0.
- v2.113.0 VERIFIED (135th tag; strokes 694, 695, 696, 697; both
  aarch64 archives downloaded cold and executed here, 22 selftests /
  2431 checks each, and the shipped binary asked the anchoring
  questions the release turns on). Six assets, site audit green on all
  nine paths, published changelog carries the tag.
- 700: health tick + audit green — milestone "the matcher's inner
  loop" complete. All eight bench checksums identical to BASELINE;
  50000 differential, crash, 20000 formatter and 2000000 pattern cases
  clean in release at seed 699. Found: I ran the pattern sweep against
  tests/grammar.rs, which does not read TING_RE_*, and it reported
  `test result: ok` in 0.00 seconds having fuzzed nothing.
- 701: replenishment — milestone "what the standard library costs"
  (v2.114-v2.115), reasoning in LOG.md. Picked by profile:
  bench/stdlib.ting spends 421 of 810 ms in `sort_with`
  (lib/list.ting:453) and 121 ms in one call to `words`
  (lib/string.ting:118). Measured against their native siblings on
  20000 elements / 108 KB: sort_with 346 ms vs sort_by 6 ms vs sort
  2 ms; words 55 ms vs split+filter 8 ms. `sort` and `sort_by` are
  builtins and `sort_with` is not, and a builtin calling back into
  user code is already established (sort_by, map, filter, reduce).
  CONSTRAINT: 2.x is additive-only and `sort_with` is reached as
  `import("lib/list.ting")["sort_with"]`, so lib/list.ting must keep
  exporting the name whatever runs underneath — how a module
  re-exports a native implementation without its own binding
  shadowing the builtin is the first stroke's question, to be settled
  before any Rust.
- 702: a module exports what its top level declares — every `let` and
  `fn` at depth zero, read from the AST, the environment asked only
  for values. Replaces a value-identity heuristic that dropped any
  builtin still bound to its own name, so `let sort = sort;` exported
  nothing. Verified by dumping all twelve stdlib modules from binaries
  built before and after: IDENTICAL at 175 names. Also split
  `import_module`'s doc comment, which had been merged into the top of
  `current_origin`'s (the 675 shape, unnoticed because it warns about
  nothing). CORRECTION to 701: a native `sort_with` must still call the
  ting comparator ~n log2 n times (287000 for n = 20000), measured at
  64 ms through `reduce` and 47 ms through `map`, so its floor is ~70 ms
  and the prize is ~5x, NOT the 58x that comparing against `sort_by`
  suggested — `sort_by` makes 20000 key calls, not 287000.
- 703: `sort_with` is a builtin (68 now), a bottom-up merge sort in
  Rust so the comparator's raise travels on `?` and stability is
  structural. 346->60 ms on 20000 elements, right at 702's predicted
  ~70 ms floor; bench/stdlib 866.8->345.6 eval, 476.5->196.0 vm,
  checksum unchanged. Equivalence measured, not argued: the old ting
  merge sort copied verbatim into a probe, 160 cases plus a stability
  case, zero mismatches on both engines. lib/list.ting keeps the name
  via `let sort_with = sort_with;`. Three guards fired and all three
  were right: --check's shadow warning (now skips `let f = f;`
  exactly, still fires on `let len = 5;`), tests/docs.rs's fn-line
  count (now counts the re-export form), tests/grammar.rs's editor
  grammar. BASELINE regenerated.
- 704: `s += x` appends instead of copying. It was QUADRATIC in the
  language, not just in lib/: 25000/50000/100000 appends took
  19/66/240 ms, now 4/6/14 on the VM. The old value is moved out of
  its binding, which needs BOTH conditions: the move happens after the
  right-hand side, so it is only sound when the RHS provably cannot
  reach the name (`cannot_reach` in eval.rs says no to any call, fn
  literal or mention of the name); and a moved-out binding cannot be
  restored, so only string-appended-to-string, the one pair that
  cannot fail, is moved. VM fuses read/op/write into UpdateSlot, or
  CheckVar + UpdateVar for the environment with the check kept ahead
  of the RHS. The tree-walker got the second condition wrong first —
  moved before checking the pair, so a failed `u += 1` left u nil
  while the VM left it alone; the engines disagreeing is what caught
  it. Deciding cases pinned in tests/differential.rs. bench/accum.ting
  is new (166.7->74.6 eval, 142.3->44.1 vm); BASELINE is NINE rows.
  FOUND: `words` barely moved (55->48 ms). Its words are ~5 chars, so
  the quadratic term was never its cost — --profile puts 52.8 of 86 ms
  in the per-character loop and 95599 `contains` calls. The
  accumulator was the right fix picked for the wrong reason.
- 705: v2.114.0 TAGGED (136th tag; strokes 702, 703, 704). Cut from
  82a4797 with CI and Pages green; gate green at the tag (fmt, zero
  clippy warnings, fifteen suites, corpus at seven warnings, 22
  selftests / 2431 checks) and the release binary reports 2.114.0.
- v2.114.0 VERIFIED (136th tag; strokes 702, 703, 704; both aarch64
  archives downloaded cold and executed here, 22 selftests / 2431
  checks each). The shipped binaries were asked what the release
  claims: `import("lib/list.ting")["sort_with"]` still there, still
  sorting, still stable; `sort_with` as a bare builtin; and the append
  linear at 50000/100000 (6/14 ms gnu, 11/17 musl) with `s += s` and a
  failed `u += 1` still behaving. Six assets, site audit green on all
  nine paths, published changelog carries the tag and the published
  reference carries `sort_with`.
- 707: `words` splits instead of scanning. The re-profile picked it, as
  704 predicted: with sort_with native and the append linear, the 810 ms
  in functions was 296.7 and `words` at 58.9 ms was the largest thing
  still in ting. Its cost was the per-character loop and 95599
  `contains` calls, so three `replace` passes turn every separator into
  a space and one `split` does the work — 48->9 ms plain, 69->20 ms
  tab/newline-heavy, out of the profile entirely. bench/stdlib
  340.4->256.2 eval, 191.2->155.8 vm, checksum unchanged. Fifteen
  shapes agreed before the change; two selftest assertions now pin the
  carriage return nothing had pinned (2433 checks). MILESTONE TOTAL:
  bench/stdlib 866.8->256.2 eval (3.4x), 476.5->155.8 vm (3.1x),
  checksum never moved.
- 708: health tick + audit green — milestone "what the standard
  library costs" complete. All NINE bench checksums identical to
  BASELINE (stdlib's `10006 10 500 w0 18974763` the one that mattered:
  every stroke changed how it was computed, none what it computed);
  50000 differential, crash, 20000 formatter and 2000000 pattern cases
  clean in release at seed 708, the pattern sweep taking 3.01 s
  against 0.22 for the default count. Site audit green on nine paths,
  six assets on each of the last six tags. Nothing found.
- 709: replenishment — milestone "the code the docs promise"
  (v2.115-v2.116), reasoning in LOG.md. The docs hold 52 ting code
  blocks (44 tutorial, 8 reference) and NOT ONE is executed by any
  test; the cookbook has that guarantee only because it is generated
  from examples/, which tests/examples.rs replays against recorded
  .out files. Measured before proposing: 50 of the 52 run correctly
  today, and the 2 that do not are deliberate illustrations (the
  reference's syntax cheat-sheet with a bare `break;`, and its
  two-files-in-one-block module example). A guard must give each block
  its OWN DIRECTORY — the tutorial's walk_ext example calls
  `make_dir("report/data")` and left a stray report/ in the tree when
  run from the root.
  NOT CHOSEN, with reasons: what a call costs (bench/fib is 1028457
  calls in 497 ms, ~0.48 us each and the largest number in the suite,
  but params already land in slots, a capture-free body allocates no
  Env, and the locals buffer is pooled — no defect, only a grind);
  destructuring (`let [a, b] = pair;` is a parse error but only nine
  corpus sites index a pair). CLOSED, verified this tick: unused
  bindings inside function bodies ARE warned about now, and a CRLF
  file passes `--fmt --diff` untouched — both old health-tick findings
  are gone.
- 710: `documented_snippets_run` in tests/docs.rs — every ting block in
  tutorial.md and reference.md is extracted, written to a fresh temp
  directory, run there with stdin closed, and required to exit zero.
  Own directory is NOT tidiness: the tutorial's walk_ext block calls
  `make_dir("report/data")`. The 2 illustrations say so on their first
  line, `# not a program: <why>`, which the reader gets too. Two counts
  stop it passing on nothing: >= 45 ran, exactly 2 skipped. Checked
  against real rot (a `no_such_builtin` inserted into the tutorial's
  first block turned it red, removing it green). 0.17 s, so it rides
  along in the docs guard every tick already reruns.
- 711: five of the eight try-wrapper sites now use `try(f, ...args)`;
  three keep the lambda ON PURPOSE. The reference's was never drift —
  it illustrates the paragraph saying a lambda guards more than one
  call and that try's own arguments are evaluated before try runs; the
  cookbook's todo example reads and parses in one lambda for the same
  reason. The tutorial WAS drift: it teaches the short form at line
  394 (`try(parse_age, raw)`) then used a lambda four times after.
  The fifth (line 318, closures section) comes before the reader meets
  try at all, so it gets the simpler form. examples/machine.ting
  changed and cookbook regenerated — machine.out UNCHANGED, the proof
  that mattered. The tutorial has no recording, so the five blocks were
  run and diffed BY HAND against the text block each sits above: all
  seven try blocks match.
  FOUND, and it is the next stroke: 710's guard checks a snippet RUNS,
  not that it prints what the page CLAIMS. 43 of the tutorial's 44
  blocks are followed by a claimed output and NOTHING verifies any of
  them (the reference has 0 such claims).
- 712: the snippet guard now checks the CLAIMED OUTPUT, not just the
  exit status. Pairing is structural — the page is read as an ordered
  list of fences and a `ting` block takes the `text` block immediately
  after it as its claim. All 43 tutorial claims were ALREADY TRUE on
  the first run (stdout only); the stroke found no rot, it closed the
  way rot gets in — five of the 43 had been hand-edited one tick
  earlier. Three counts pinned per page so a claim that stopped being
  paired FAILS rather than silently stops being checked: tutorial
  (43 compared, 1 run-only, 0 illustrations), reference (0, 6, 2). The
  run-only one is the `sh` block that prints nothing definite when git
  is absent. Made to fail on purpose first: "insufficient funds" ->
  "plenty of funds" named the block and printed both sides.
- 713: v2.115.0 TAGGED (137th tag; strokes 707, 710, 711, 712). Cut
  from ff172db with CI and Pages green; gate green at the tag (fmt,
  zero clippy warnings, fifteen suites, corpus at seven warnings, 22
  selftests / 2433 checks) and the release binary reports 2.115.0.
- v2.115.0 VERIFIED (137th tag; strokes 707, 710, 711, 712; both
  aarch64 archives downloaded cold and executed here, 22 selftests /
  2433 checks each). The shipped binaries were asked what the release
  claims: `words` returns the same lists (CRLF, empty, all-spaces) and
  `squeeze` still collapses, and it is LINEAR — ten times the text
  costs ~ten times the time (9->96 ms gnu, 11->114 musl), not a
  hundred; both spellings of `try` work. Six assets, site audit green
  on nine paths, and the PUBLISHED pages carry the changes: changelog
  has v2.115.0, tutorial has `try(json_parse`, reference has the "not
  a program" line — worth checking on the site because it is a
  sentence for readers, not only a marker for a test.
- 715: health tick + audit green — milestone "the code the docs
  promise" complete. All NINE bench checksums identical to BASELINE;
  50000 differential, crash, 20000 formatter and 2000000 pattern cases
  clean in release at seed 715, the pattern sweep 2.98 s against 0.22
  for the default count. Site audit green on nine paths, six assets on
  each of the last six tags. Nothing found — the milestone added
  guards rather than changing behaviour, and both pages were already
  correct (50 of 52 blocks ran first try, all 43 stated outputs
  already exact). What it leaves: every block runs in its own
  directory, every claim checked character for character with
  (compared, run-only, illustration) counts pinned per page, two
  illustrations saying so in a sentence readers get, and five `try`
  sites reading the way the tutorial teaches. Both guards were made to
  fail on purpose before being believed.
- 716: replenishment — milestone "a script you can hand over"
  (v2.116-v2.117), reasoning in LOG.md. The binary is self-contained
  and the stdlib is embedded, but a USER's script that splits into
  modules is two files forever: local modules are first-class in the
  tutorial, --check and the LSP, and nowhere in handing the result to
  somebody. `--bundle` inlines local imports into one runnable .ting
  file; `lib/...` imports stay, being already in the binary.
  THREE PROPERTIES CHECKED THIS TICK, not assumed: (a) a module is a
  map of what its top level declares — 702's rule is exactly what lets
  a module become an expression; (b) importing twice gives the SAME
  map (`a["extra"] = 1` visible through b, `a == b`), so a bundle
  inlines once and shares, never pastes per import site; (c) circular
  imports error and must keep erroring. Name capture: each module body
  needs its own scope.
  NOT CHOSEN: a shebang line — `#!/usr/bin/env ting` already works,
  tested this tick.
- 717: `--bundle` for the straight case — a script and the local
  modules it imports as one file on stdout, nothing written to disk.
  Each module body becomes `let __ting_module_N = fn() { ... return
  {names}; }();`, emitted after whatever it imports, and every import
  site reads that one binding (716's identity property: pasting per
  site would split a module that holds state). lib/ imports stay.
  REFUSED, each at the import's own file:line:col — a cycle, a
  non-literal path, and a module that returns from its top level (the
  bundle would hand back that value instead of the map). Only a file,
  never `-`: imports resolve against the script's own directory, the
  trap :load fell into at 425. Item 3's three open shapes were all
  decided by building item 1: diamonds share, a module's imports
  resolve against its own directory, a cycle is refused.
- 718: the guard — every corpus program with a local import (14
  today, the count asserted) is bundled and rerun: same stdout bytes,
  same exit, bundle passes --check and --fmt-check. Both halves made
  to fail on purpose (three-space indent breaks fmt on selftest/fs;
  pasting per import site breaks selftest/modules, which already
  tests map identity). FOUND, and the docs corrected: selftest/ and
  examples/ reach the stdlib as ../lib/..., a FILE here, so those
  bundles inline the real modules (838 lines for examples/text). The
  rule is the interpreter's own — filesystem first, no file means
  embedded — not "lib/ is always left alone". Also recorded: a bundle
  cannot keep try()'s file/line identical, those being where the code
  now sits; err() messages are identical.
- 719: the tutorial's module section now ends with --bundle — two
  files, the command, the bundle it writes — and the listing is
  CHECKED, not transcribed: a docs test pulls the two sources out of
  the page, bundles them and compares the third block byte for byte
  (renaming the binding in the page fails it). FOUND while writing
  it: --fmt-check on a bundle answers for the files that went in
  (the bundler copies source as written), so "a bundle passes
  --fmt-check" was true of the formatted corpus and too strong in
  general; both pages now say so. Also corrected: the tutorial said
  the stdlib page documents "all seven" modules — there are twelve.
  NOT done: a cookbook entry. The cookbook is generated from
  examples/ by tools/cookbook.py, and an example that shells out to
  ting to demonstrate ting does not belong in the corpus.
- 720: v2.116.0 cut and pushed (137th tag; strokes 717, 718, 719).
  Gate green at the tag: fmt, zero clippy, fifteen suites, corpus at
  seven warnings, 22 selftests / 2433 checks on a binary reporting
  2.116.0. ORDINAL CORRECTED: `git tag --sort=creatordate` makes
  v2.113/114/115 the 134th/135th/136th, so recent entries were each
  one high.
- v2.116.0 VERIFIED (137th tag; strokes 717, 718, 719; both aarch64
  archives downloaded cold and executed here, 22 selftests / 2433
  checks each). The shipped binaries bundled a purpose-built program
  (direct import + the same module through a subdirectory reaching
  back out with ../, lib/string.ting in the middle) and the bundle
  ran from a directory holding nothing else: same bytes, diamond
  still shared, lib/ import answered by the embedded stdlib, --check
  and --fmt-check clean, all three refusals as documented. Site audit
  green on nine paths; changelog/tutorial/reference carry the
  release.
- 722: an import that might not run was a BUG, not a footnote.
  v2.116.0's bundle hoisted every module and ran it; `if false { let
  m = import("noisy.ting"); }` printed the module's output where the
  two files printed nothing. A bundled module is now a function that
  returns early if it already ran, else runs its body and keeps the
  map — what import does — so dependency order stops mattering and a
  module nothing asks for never runs. Import sites became calls.
  RECORDED: map `==` in ting is structural, not identity, so the `==`
  in selftest/modules.ting does not by itself prove two imports give
  one map (716 proved it by writing through one name and reading
  through the other); the new test uses a side effect.
- 723: `--bundle -o FILE`. MEASURED FIRST: `ting --bundle main.ting >
  main.ting` exits 0 and leaves a bundle of an EMPTY program where the
  script was — the shell truncates before ting starts. -o writes the
  file and refuses when it is one of the bundle's own sources, by
  resolved path (`./x`, `sub/../x` too). The first version missed
  `sub/../x` when `sub` does not exist, because canonicalize fails
  outright; it now absolutises and resolves `.`/`..` by hand first —
  but only after trying canonicalize, since lexical-first would
  misname `../o/x`. Docs teach -o and say what `>` does.
  NOT CHOSEN: module file names in a bundle's diagnostics — ting has
  no way for a file to say a line belongs elsewhere, and inventing one
  for the bundler alone is bigger than this milestone.
- 724: v2.117.0 cut and pushed (138th tag, read from `git tag
  --sort=creatordate`; strokes 722, 723). Two strokes rather than
  three on purpose: both fix something v2.116.0 shipped, and one of
  them loses a file. Gate green at the tag: fmt, zero clippy, fifteen
  suites, corpus at seven warnings, 22 selftests / 2433 checks on a
  binary reporting 2.117.0.
- v2.117.0 VERIFIED (138th tag; strokes 722, 723; both aarch64
  archives downloaded cold and executed here, 22 selftests / 2433
  checks each). The shipped binaries proved both corrections: a
  module imported inside `if false` does not load, the same module
  loads where the program does ask for it, bundled output is
  byte-identical and runs from a foreign directory, and `-o` pointed
  at `work/./greeter.ting` is refused with the file intact. Site
  audit green on nine paths; changelog, tutorial (`-o one.ting`,
  "runs the first time") and reference (`-o FILE`) carry the release.
- 726: health tick + audits green — milestone "a script you can hand
  over" COMPLETE. Nine bench checksums identical; timings 8-25% above
  baseline uniformly across both engines, which is this shared host's
  weather (ratios unchanged, and --bundle adds nothing to a path a
  benchmark runs). Fuzz in release: 50000 differential 11.03s,
  patterns at 2000000 3.29s, 20000 formatter 4.14s — runtimes quoted
  because they are the evidence the sweep ran (700's trap). All 138
  releases carry the assets their era calls for (3 / 4 / 6), checked
  release by release; nine site paths 200.
- 727: replenishment — milestone "what a file is, besides its name"
  (v2.118-v2.119), reasoning in LOG.md. Nothing in 68 builtins or
  twelve modules gives a file's SIZE or its MODIFICATION TIME;
  list_dir + is_dir is the whole of what a program can learn without
  opening the file. MEASURED, not assumed: len(read_file) counts
  CHARACTERS (src/eval.rs is 192530 bytes, ting says 192474 — wrong
  by its own UTF-8, silently); read_file REFUSES non-UTF-8, so 4978
  of .git's 5108 files cannot be sized at all; sizing that 105 MB
  tree by reading it takes 3920 ms and peaks at the largest file;
  `run("stat", ...)` costs 0.75 ms per file (a process each) and is
  GNU-only, so it breaks on macOS and says nothing on Windows. mtime
  cannot be computed from anything that exists — "what changed since
  yesterday" is impossible, not merely awkward.
- 728: `stat(path)` is the 69th builtin — {size (bytes), modified (ms
  on time_ms()'s clock), kind ("file"/"dir"/"other")}, nil when
  nothing readable is there. DECIDED: nil not an error (exists() sets
  the precedent — a question, not a demand, and no try() to ask how
  big something is); symlinks FOLLOWED (what fs::metadata, exists and
  is_dir already do, so a broken link is nil). Signed like time_ms(),
  so a pre-1970 file counts backwards; a directory's size is the
  filesystem's number for the directory, not its contents. Seven
  checks in selftest/fs.ting (2433 -> 2440 checks), tutorial block
  run by the docs guard, reference row + prose, editor grammar.
- 729: lib/fs gained size(p), facts(d) and total_size(d), chosen by
  drafting the example first and keeping what it wanted twice. facts
  exists because sorting walk's paths with stat inside the comparator
  asks once per COMPARISON: 249 ms vs 139 ms over .git's 5132 files
  (1.8x, not the 12x n log n suggests — the comment carries the
  measurement, not a complexity claim). NOT added: modified(p), asked
  for once where size was asked for on every line.
  FOUND, and it constrains the next tick: files written one after
  another in the same run share a `modified` (writes are faster than
  the clock), so "which is newest" is a tie that sort_with resolves
  by input order. No way to set an mtime from ting, and sleep_ms
  would still tie on a one-second filesystem. The example may ask HOW
  RECENT something is; it must not claim which file is newest.
- 730: examples/tree.ting — a directory by size: total bytes, three
  largest, bytes by extension, how many changed in the last day. None
  of those five numbers existed for a ting program a release ago.
  Takes a path or builds and removes its own tree (so the .out is
  stable), which also keeps it out of the playground — the browser
  has no filesystem. `modified` asks HOW RECENT, never which is
  newest (729's tie). A missing path exits 2 rather than reporting an
  empty directory. Human sizes done in ting (scale to tenths, round,
  put the point back), CHECKED against the filesystem: eval.rs 190.3
  KB for 194911 bytes, src total matches du -sb less the directory
  entries.
- 731: v2.118.0 cut and pushed (139th tag, read from `git tag
  --sort=creatordate`; strokes 728, 729, 730). Gate green at the tag:
  fmt, zero clippy, fifteen suites, corpus at seven warnings, 22
  selftests / 2447 checks on a binary reporting 2.118.0.
- v2.118.0 VERIFIED (139th tag; strokes 728, 729, 730; both aarch64
  archives downloaded cold and executed here, 22 selftests / 2447
  checks each). Shipped binaries checked against the OS: stat says 7
  where len(read_file) says 6 on the same file; a 300000-byte random
  file sizes although read_file still errors on it; an age from
  time_ms() is sane; fs["total_size"] is du -sb less the 4096 the
  directory entry takes; tree.ting prints its recorded report and
  says 570.0 KB for src, matching 583635 bytes summed in Python. Site
  audit green on nine paths; changelog, reference (stat row), stdlib
  (177 functions) and cookbook (tree) carry the release.
- 733: health tick + audits green — milestone "what a file is,
  besides its name" COMPLETE. Nine bench checksums identical; timings
  5-14% above baseline (closer than the last tick's 8-25%, same host
  less busy), ratios unchanged, and no benchmark calls stat. Fuzz in
  release: 50000 differential 9.12s, patterns at 2000000 3.20s, 20000
  formatter 3.82s. All 139 releases carry their era's assets (3/4/6),
  checked one by one; nine site paths 200; corpus at seven warnings;
  all 62 ting files formatted.
- 734: replenishment — milestone "moving a file, not retyping it"
  (v2.119-v2.120), reasoning in LOG.md. No rename and no copy among
  the 69 builtins, so a move is read_file + write_file + remove_file.
  PROBED with the most ordinary tidying script there is (files into
  folders named for their modification day): it moved the text files
  and FAILED on the photograph ("stream did not contain valid
  UTF-8"), and the files it did move came out dated NOW — 16 ms after
  the original — so a script that organises by date rewrites every
  date it used. Checked, not assumed: `mv` keeps the date (true),
  `cp` without -p does not (false). Speed is NOT the argument: 9 MB
  copied through ting is 11 ms against cp -p's 6. What is true is the
  whole file goes through memory and write-then-remove is not atomic.
- 735: `rename(from, to)` — the 70th builtin. MEASURED first, on this
  host, ext4 against tmpfs: rename keeps the modification time
  (true), takes a directory whole, replaces an existing target
  silently, and refuses across filesystems with EXDEV ("Invalid
  cross-device link", os error 18) — the one case `mv` handles by
  quietly copying. DECIDED not to fall back: a copy has a different
  cost and a different date, and a rename that is sometimes a copy is
  the surprise this milestone exists to remove. So the error says
  "they are on different filesystems" in ting's words, not the
  kernel's. std::io::ErrorKind::CrossesDevices is stably matchable at
  rustc 1.98 (checked by compiling it). Guards: two in tests/io.rs,
  each made to FAIL on purpose first — the date test backdates both
  files to a fixed instant (1000000000000) so "the stamp survived"
  cannot be two operations landing in the same millisecond, and the
  cross-device test really crossed to /dev/shm (proved by reading the
  message out of the deliberate failure) and reports-and-passes where
  only one filesystem is mounted. 7 new selftest checks (2447 ->
  2454). The grammar guard caught the alternation inserted in the
  wrong place: editor/ting.tmLanguage.json must hold Builtin::ALL's
  order exactly, so rename goes after remove_dir, not after make_dir.
- 736: `copy_file(from, to)` — the 71st builtin. MEASURED first: `mv`
  across filesystems KEEPS the date (981169506 on both sides), `cp`
  without -p does not, `cp -p` does; `cp` refuses a self-copy AND a
  hard-link alias ("are the same file", exit 1), which it must,
  because std::fs::copy there truncates the source it is about to
  read and reports a successful copy of nothing (measured 735).
  DECIDED: copy_file keeps the permission bits (std does) and the
  modification time (std does NOT — set explicitly, after the bytes),
  because a cross-filesystem move in ting is copy_file + remove_file
  and mv keeps the date there; a copy that dropped it would restore
  the bug that opened this milestone. Same file refused by inode on
  unix, canonicalized paths elsewhere. A directory refused in ting's
  words, not std's "neither a regular file nor a symlink to a regular
  file". NOT atomic, deliberately and documented: the
  half-done-invisible version is copy to a temporary name + rename,
  two readable lines now that both builtins exist; baking it in would
  need write permission in the target's directory, would break
  copying into a file something holds open or hard-links, and would
  leave debris with a name nobody chose. rename's EXDEV message
  gained its second half ("which copy_file crosses and rename
  cannot"). Guards: 2 in tests/io.rs (backdated stamp + six non-UTF-8
  bytes; hard-link refusal with the source still reading "still
  here"), both made to FAIL on purpose; 7 selftest checks (2454 ->
  2461).
- 737: `lib/fs.ting`'s `move` (rename, else copy_file + remove_file,
  the mv fallback written where it is readable; it does NOT read the
  error message — whatever stops the rename stops the copy, and the
  copy's error says it better) plus examples/organize.ting, the
  tidying script finished. Writing it turned up three things the 734
  probe never reached: a name clash is a DELETION (rename replaces
  silently, so the example numbers the second file and prints how
  often it had to); filing twice must move nothing (a file already in
  its day folder has target == path, and without that skip the clash
  check sees the file itself and makes copies forever); and a
  tidy-up that leaves empty directories is not tidy (prune, depth
  first). Output line `every date survived: true` = sorted mtimes
  before == after, which reads false if the same script copies
  instead. Demo lands in ONE folder because ting cannot write a file
  dated earlier than now, and holds no binary because write_file
  takes a string — both said in the header, not hidden. FOUND BY
  RUNNING: try() returns {"ok": ...} with NO err key on success, so
  try(...)["err"] raises "key not found"; use has(r, "ok"). The
  cookbook guard caught the new example — every example needs a
  section in docs/cookbook.md, regenerated with
  `python3 tools/cookbook.py`. 4 selftest checks (2461 -> 2465).
- 738: v2.119.0 released (140th tag; strokes 735, 736, 737). CAUGHT
  AT RELEASE TIME: all three strokes had missed their CHANGELOG entry
  (`## Unreleased` was not in the file), and README still said 69
  builtins. Written in a commit ahead of the release, which keeps the
  release commit what it has always been: CHANGELOG heading +
  Cargo.toml + Cargo.lock. The rule that caught it, worth keeping: a
  release tick's FIRST act is to diff the previous release commit and
  check the tree is in the state that commit assumed.
- 739: v2.119.0 VERIFIED. Six assets, all four runs on the tag green
  by API verdict, both aarch64 archives downloaded cold and executed
  here (22 selftests / 2465 checks on gnu and musl, `ting 2.119.0`),
  and the release's own example run by the released musl binary
  diffed clean against examples/organize.out. Site audit: nine paths
  200, ting.wasm 819704 bytes, changelog carries v2.119.0, cookbook
  has organize, reference has copy_file, github.io still redirects.
- 740: health tick GREEN — milestone "moving a file, not retyping it"
  complete. All nine bench checksums match BASELINE exactly; timings
  ran ~25-30% above it across the board, which is the host being busy
  and not a regression (checksums decide, timings are weather). VM
  ahead of eval on all nine rows (-22% to -47%). Fuzzers at seed 740:
  50000 differential (16.96 s vs ~1 s default), 20000 formatter
  (5.35 s), 2000000 patterns (3.78 s vs 0.22 s) — runtimes are the
  evidence they ran. Suite 352 tests in 15 suites, corpus 7 warnings.
  AUDIT of the counts the docs claim, and a NEW GUARD: never count
  stdlib functions with `grep '^fn '` — it says 177 where the truth
  is 178, because lib/list.ting re-exports the builtin sort_with with
  a `let` (line 455) and lib/test.ting exports a non-function `state`
  map. Asking the modules (keys + type == "function") is the only
  right answer, and tests/docs.rs now does exactly that against the
  "N functions between them" sentence, made to fail on purpose. Also
  corrected here: 22 selftest files, not 21 (the 22nd is _lib.ting,
  which checks nothing on its own).
- 741: replenishment — milestone "a file read a line at a time"
  (v2.120-v2.121), reasoning in LOG.md. MEASURED on a 147 MB,
  2000000-line log, same program, same answer (666306): given the
  PATH 2.11 s / 454 MB peak; given the same bytes on STDIN 1.38 s /
  9 MB; grep -c 0.37 s / 9 MB. ting can already stream — input()
  gives a line and nil at the end — but only from stdin, never from a
  file. Cost split to locate it: read_file alone 149 MB, + split on
  newlines 376 MB (the 2000001 strings weigh 227 MB MORE than the
  file). That KILLS a read_lines(path) list builtin: it would pay the
  expensive half. Writing is NOT the pain (checked): appending 100000
  lines one at a time is 381 ms vs 122 ms building and writing once.
  Closures can accumulate into an outer variable (checked), so a
  callback shape works.
- 742: `each_line(path, f)` — the 72nd builtin. Back to back on this
  host: read_file + split 1.35 s / 454 MB, each_line 1.24 s / 9 MB,
  same 147 MB log and same answer (666306). All five questions
  settled by looking: input() strips the CR as well as the NL
  (measured), so each_line does; a last line without a newline
  counts; a file that is not UTF-8 errors as read_file does; `false`
  stops the read and every other answer including nil carries on (a
  bool requirement like filter's would make every callback end in
  `return true;`, and without a stop there is no cheap `head`); "-"
  is stdin AND shares input()'s buffer, so the two compose (that is
  the Rust guard, broken on purpose). One String reused for the whole
  read. CAUGHT BY THE MACHINERY: the reference guard fired before any
  prose was written (every builtin must be in docs/reference.md), and
  the corpus check went 7 -> 9 because `fn(l) { return nil; }` warns
  about an unused parameter — `_l` is the checker's own escape. 7
  selftest checks (2465 -> 2472).
- 743: lib/fs gained count_lines, head, tail, lines_matching on top
  of each_line (178 -> 182 stdlib functions). Two are easy to write
  badly, so they are written once: head must STOP the read (return
  len(out) < n), and tail must hold a WINDOW, not a list. MEASURED
  over 1000000 lines, naive (push then drop the front) vs ring
  (ring[seen % n], unrolled once): n=10 2007 ms vs 925 ms; n=1000
  75000 ms vs 926 ms — the naive one is O(n) per line, the ring is
  flat. Both versions checked against each other before one was kept.
  The selftest asks for the last three of five, where the ring's
  start is not zero and a forgotten rotation answers out of order.
  11 selftest checks (2472 -> 2483). The 740 count guard did its job
  on a change for the first time.
- 744: examples/logreport.ting — a log read in one pass, printing
  what the file weighs beside what reading it cost (216388 bytes
  against 86 held: a counter per level, one per source, the first
  line, the last, the longest — nothing that grows with the file).
  It does NOT use fs head/tail for the ends: each is another pass,
  and inside a pass already running the ends are free; the library
  ones are for when the ends are all you want. FOUND BY RUNNING IT
  ON /dev/null: an empty log crashed on len(nil) — examples must be
  run against what they were not written for. Also confirmed the pipe
  form (`cat log | ting logreport.ting -`). ORDER LESSON: --fmt
  reformatted the example AFTER tools/cookbook.py had generated the
  page from it, so the guard failed on a stale page — format first,
  generate second.
- 745: v2.120.0 released (141st tag; strokes 742, 743, 744). The 738
  rule worked: diffing the previous release commit FIRST showed the
  tree already in the state it assumed — `## Unreleased` present with
  all three entries, written by the ticks that earned them, and
  README already at 72 builtins. Nothing to repair.
- 746: v2.120.0 VERIFIED. Six assets, all four runs on the tag green
  by API verdict, both aarch64 archives downloaded cold and executed
  here (2483 checks each, `ting 2.120.0`), logreport run by the
  released musl binary diffed clean against its .out. Because this
  release is about MEMORY, the claim was checked on the artifact: the
  downloaded musl binary counted a fresh 2000000-line log in 1.80 s /
  9 MB peak. Site audit: nine paths 200, ting.wasm 825122 bytes,
  changelog has v2.120.0, cookbook has logreport, reference has
  each_line, github.io still redirects.
- 747: health tick GREEN — milestone "a file read a line at a time"
  complete. All nine bench checksums match BASELINE, and the timings
  ANSWERED 740's open question: they are back ON the baseline on a
  quiet host (accum 72.9 vs 75.0, fib 538.8 vs 535.3, toplevel 434.2
  vs 451.1), so 740's uniform 25-30% excess really was the host and
  not a regression — a second sample is what makes "timings are
  weather" a measurement rather than a saying. VM ahead on all nine
  (-19% to -45%). Fuzzers at seed 747: 50000 differential 8.15 s,
  20000 formatter 3.70 s, 2000000 patterns 3.00 s, all back to their
  historical runtimes. Audit: 72 builtins, 182 stdlib functions (by
  the 740 guard), 22 selftest files, 21 examples with .out, 43 ting
  programs, 353 Rust tests in 15 suites, 7 corpus warnings. NOT
  added: a bench row for each_line — the bench decides by checksum
  over deterministic work, and file reading is the filesystem's mood;
  the memory claim belongs on the release artifact, where 746 checked
  it (9 MB for 2000000 lines).
- 748: replenishment — milestone "reading what other programs wrote"
  (v2.121-v2.122), reasoning in LOG.md. BOTH halves are traps, not
  difficulties. CSV: a 15.7 MB file of 300000 rows (a third with a
  newline inside a quoted field) costs 9.36 s / 964 MB peak through
  csv["parse"](read_file(p)) — 61x the file — and lib/csv cannot be
  asked for one row. The obvious workaround is not just slower, it is
  WRONG: the file is 400001 LINES and 300001 ROWS, so a per-line
  split invents 100000 rows silently. TIME: lib/time writes iso() and
  cannot read it. The five lines a script writes today round-trip
  correctly (including pre-epoch) but on real input give
  `cannot convert "not " to int` for "not a date" — a message about
  the wrong thing — and for "2026-13-45T99:99:99Z" a confident
  1802925639000.
- 749: lib/csv gained each_row(path, f, sep) — 964 MB -> 9 MB on the
  15.7 MB / 300000-row export, 9.55 s -> 10.45 s, same rows, amount
  column summing to 67499775000 both ways. THE DESIGN IS THE POINT:
  no second parser. Cutting on newlines invents 100000 rows; counting
  quotes per line is also wrong (a stray quote in an unquoted field
  is a literal to parse). So parse was taken apart into fresh() /
  scan(st, text, sep) / finish(st), parse is now
  finish(scan(fresh(), text, sep))["rows"], and each_row feeds the
  SAME scanner a line at a time — they cannot disagree. Hot loop
  still on locals (loaded at entry, stored at exit): a map lookup per
  character would cost more than 14 loads and stores. The two
  selftests that carry the argument: streamed == parse(doc) for a
  field with a line break, AND the same for a document ending inside
  an open quote — malformed input must be read the same way by both,
  so each_row flushes as parse does rather than refusing. 7 selftest
  checks (2483 -> 2490), stdlib 186. REMEMBER (729, cost time again):
  the stdlib is embedded at compile time — a new lib function does
  not exist until cargo build --release.
- 750: lib/time gained from_iso(s) (+ digits(s)); stdlib 188.
  ACCEPTS: what iso writes, a bare date (midnight), a space for the
  T, missing seconds, fractions (truncated to ms, not rounded),
  offsets +02:00 and -0500, pre-epoch. REFUSES (all nil):
  2026-13-45T99:99:99Z, 2026-02-30, 2023-02-29, hour 24, minute 60,
  second 60 (a leap second is refused, not moved), 2026/09/06,
  "+2:00", an empty fraction, "", a trailing letter, a non-string.
  THREE CHOICES: nil not an error (reading a file's timestamp is a
  QUESTION, like stat on a path); no offset means UTC (a stated
  convention, not a guess — the module has no local zone anywhere);
  a leap second refused rather than answered. 22 selftest checks
  (2490 -> 2512). The acceptance and refusal tables ARE the tests,
  line for line, which is the right shape for a parser.
- 751: examples/monthly.ting — a CSV totalled month by month without
  holding it. Three things in its output ARE the milestone: "5000
  rows" from a file that is 5001 rows in 6001 LINES (every fifth note
  has a line break in a quoted field), so a newline-cutting reader
  would invent a thousand rows; "dates nothing could read: 3" from
  planted "not a date" / "2026-02-30" / "" fields, which the
  hand-rolled parser would have turned into numbers (one of them a
  date in March) and added to a month's total; and columns found BY
  NAME in the header. Counts add up: 1080+1119+1239+1199+360 = 4997 =
  5000 - 3. Bounded state: two ints per month, two column numbers,
  the row in hand. Format BEFORE generating the cookbook (744's
  lesson) — the guard passed first time.
- 752: v2.121.0 released (142nd tag, read from
  `git tag --sort=creatordate | grep -n`; strokes 749, 750, 751),
  the milestone's first release. The 738 rule ran first and again
  found nothing to repair: `git show --stat 358c21a` touched only
  CHANGELOG/Cargo.toml/Cargo.lock, so `## Unreleased` had to arrive
  carrying all three entries, and it did. README's builtin count
  needed no move — all three strokes are library and example work on
  the 72nd builtin, not a 73rd. Gate green at the tag: fifteen
  `test result: ok`, zero clippy, formatter 0 of 69, corpus at seven.
  Binary reports ting 2.121.0.
- 753: v2.121.0 VERIFIED. Four workflows green by the API; six assets.
  The cold-downloaded aarch64 musl binary (statically linked) reports
  2.121.0, runs the selftests from the tarball's own lib/ (22 passed,
  2512 checks), and diffs clean on both monthly.ting and
  logreport.ting. The release's claim checked ON THE ARTIFACT: a
  9.4 MB export, 300000 rows in 400001 LINES (every third note wraps
  in a quoted field) — parse(read_file(p)) 494 MB / 9.67 s,
  each_row 10 MB / 11.74 s, both 300000 rows totalling
  14920889.85 to the last digit. 51x the memory for a tenth less
  time. Site: every published path 200, ting.wasm 836714 bytes (was
  825122), changelog carries v2.121.0, stdlib has each_row and
  from_iso, github.io still 301s to www.baghino.me/thing.
  MY OWN PROBING WAS WRONG, NOT THE SITE: /vm.html and /playground/
  404 because neither ever existed — pages.yml copies playground/. to
  the site ROOT (so the playground is /) and renders six docs
  (tutorial, reference, stdlib, cookbook, retrospective, changelog),
  not docs/vm.md. THE WORKFLOW IS THE AUTHORITY on what the site
  contains; a guessed URL is not evidence of a missing page.
- 754: lib/csv gained each_map(path, f, sep) + entry_of(header, row);
  stdlib 188 -> 190 (asked of the modules, per 740). NO SECOND
  IMPLEMENTATION, the 749 argument again: entry_of is the one place a
  row is given its column names and BOTH maps and each_map call it,
  so a file read whole and a row at a time is named identically.
  10 selftest checks (2512 -> 2522), each run past a deliberately
  wrong each_map built in a scratch script from the module's own
  exports: header not skipped, missing column dropped instead of nil,
  false ignored — all four assertions false. COST measured on the
  9.4 MB / 300000-row export, twice each: each_row 8.07/8.07 s,
  each_map 8.94/8.97 s, both 10 MB and both totalling 14920889.85.
  ~11% for the naming, the bound unweakened. KNOWN GAP: each_map does
  not hand over the header — the map's keys are it, so
  has(row, "date") on the first record answers "has this file the
  column", but a header-only file never calls f and never asks.
  LEARNED: a real lib/ next to a script SHADOWS the embedded stdlib
  (a leftover release tarball in the bench dir gave
  `key "each_map" not found` from a binary that has it); the embedded
  stdlib is a fallback, not an override.
- 755: examples/monthly.ting reads by name; 10 code lines gone (two
  nils outside the callback, the rows == 1 header scan, the guard on
  every later row, the rows += 1 that only marked the first).
  EVERY NUMBER UNCHANGED — 5000 rows, the five month totals to the
  cent, "dates nothing could read: 3"; only the "held while reading"
  line moved. THE 754 GAP FACED, all three cases checked: wrong
  header with rows still exits 2; right header with no rows still
  reports 0; WRONG header with NO rows changed from exit 2 to a 0-row
  report — accepted because both answers describe the same nothing
  and no total could have been wrong, and said in a comment in place.
  NOT DONE: handing the header to f or returning it — the map's keys
  ARE the header, and a second way to ask one question is the thing
  749 and 754 both avoided.
- 756: a byte order mark is no longer content. CAME TO RELEASE, DID
  NOT: asked what other programs actually write before closing a
  milestone named for it, and the first answer broke the milestone's
  own example — a spreadsheet-exported CSV made monthly.ting print
  "the header has no date and amount columns", because behind a mark
  the column is named \ufeffdate and 755 had just made asking BY NAME
  how it works. json_parse was worse ("unexpected character at offset
  0" on a fine document; RFC 8259 forbids the mark but lets a parser
  ignore it). Fixed in lib/csv's SHARED scanner (new `begun` flag, so
  whole-file and row-at-a-time drop the same mark — the 749 rule) and
  in json::decode. ONE mark, ONLY at the head: six of the eleven new
  checks are about what is NOT stripped (a second mark, a mark in a
  later field, inside a JSON string, at the end). 2522 -> 2533.
  NOT TOUCHED: read_file, each_line, trim, int — a file's bytes are
  its bytes; only a DOCUMENT reader may decide the first character is
  not part of the document.
- 757: v2.122.0 released (143rd tag; strokes 754, 755, 756), closing
  the milestone "reading what other programs wrote". 738 rule found
  the tree as 909c4b8 left it. THE GATE CAUGHT A FLAKE BEFORE THE
  TAG: selftest/csv.ting failed ONLY inside
  both_engines_cover_the_same_lines (Eval, "a header alone is no
  records"), passing standalone on both engines. Cause was in that
  test's OWN COMMENT, written about fs.ting and sh.ting: it reruns
  every selftest in-process while another test runs them in parallel,
  so a FIXED FIXTURE NAME races. csv.ting had written
  selftest-csv-rows.csv since 749 (two writes); 754 and 756 took it
  to nine, widening the window. MEASURED: fixed name 1 failure in 6
  runs, unique name 0 in 10. Fixed with
  format("selftest-csv-rows-{}.csv", random_int(0, 1000000000)) —
  the RNG seed is the clock in nanoseconds, so two interpreters in
  two threads do not share one. Chose this over the precedent of
  adding csv.ting to the skip list, which would drop a
  twice-changed module from the coverage comparison. Committed AHEAD
  of the release, no CHANGELOG entry (not news to a downloader).
- 758: v2.122.0 VERIFIED, milestone closed and shipped. Four
  workflows green by the API, six assets; cold aarch64 musl binary
  reports 2.122.0 and runs the selftests on its own embedded stdlib
  (22 passed, 2533 checks); both examples diff clean against the
  tarball's OWN lib/. THE RELEASE'S OWN CHECK: a file written the way
  a spreadsheet writes one (BOM, CRLF, quoted fields with a line
  break inside) read by the downloaded binary — 12 rows from a file
  each_line counts as 17 LINES, zero unreadable dates, months named
  at all (the same binary before 756 would have exited 2 with "the
  header has no date and amount columns"). Totals check by hand:
  10.50..21.51 sums to 192.46 = 60.18 + 64.12 + 68.16. json_parse of
  a BOM'd document answers on the artifact too.
  SITE: TEN paths 200, not nine — earlier entries counted / and
  /index.html as one; ting.wasm 839346 bytes (was 836714); changelog
  carries v2.122.0, stdlib has each_map and says 190 functions,
  reference mentions the byte order mark, github.io still 301s.
- 759: both_engines_cover_the_same_lines runs EVERY selftest now.
  MEASURED FIRST (the skip might have been necessary): skip removed
  with fs.ting's root still fixed = 6 ok / 4 FAILED in 10; with the
  root uniquely named = 10 ok / 0 failed. The failure was
  `fs.ting:86 cannot index nil with string` — two runs building and
  removing each other's tree, remove_tree landing between another
  run's write_file and its stat — and it also took down
  selftest_programs_match_across_engines, so the race could break a
  test that passes today. sh.ting needed NOTHING: it spawns sh -c and
  touches no file; its exclusion was guilt by association. WHAT IT
  BUYS (checked, not assumed): only fs.ting and sh.ting import
  lib/fs.ting and lib/sh.ting, so two of the twelve stdlib modules
  had never been in the engine coverage comparison at all. The skip
  block is now a comment saying to fix the fixture rather than add a
  skip back. .gitignore (which had only /target, playground/ting.wasm
  and .playwright-mcp/) now covers selftest-fs-tree-* and
  selftest-csv-rows-*: a failed run leaves its fixture behind, and
  under the old FIXED name that leftover could have been committed —
  one such empty directory was sitting in the tree and was removed.
- 760: health tick, green. Bench: all nine checksums match, timings
  at or under baseline (accum 73.5 vs 75.0, fib 515.1 vs 535.3,
  toplevel 434.3 vs 451.1); VM ahead on all nine, -17% to -44%.
  Fuzzers at seed 760: 50000 differential 7.66 s, 20000 formatter
  3.62 s, 2000000 patterns 2.97 s — historical runtimes, which is
  what says they fuzzed anything. Audit: 72 builtins, 190 stdlib
  functions (asked of the modules), 22 selftest files, 22 examples
  with .out, 44 ting programs, 353 Rust tests in 15 suites, 7 corpus
  warnings, 2533 checks. Site ten paths 200.
  FOUND: STATE's own standing shape said 188 functions, wrong since
  754, through a release, in the file I orient from every tick.
  docs/stdlib.md was right because 740's test guards it; STATE was
  guarded by nothing. Extended that same test to STATE (it already
  has the number in hand), whitespace-insensitive so a rewrap cannot
  read as a wrong count, and made it fail on purpose with 188 put
  back. THE LESSON: "it only misleads me" says who gets hurt, not
  whether to guard it.
  Housekeeping still offered, still not urgent: target/ 41 GB (a
  cargo clean costs one full rebuild), .git 117 MB.
- 761: replenishment — milestone "the time it is here"
  (v2.123-v2.124), reasoning in LOG.md. THE EVIDENCE: it is 05:50
  here and every ting program says 03:50; env("TZ") is nil, so a
  script has nothing to read. examples/organize.ting (SHIPPED, with a
  recorded .out) files by tm["date"](f["modified"]) = the UTC day —
  demonstrated: mtime 2026-09-07 00:30 +0200 is filed under
  2026-09-06 while every other tool says the 7th. Two hours a day
  wrong here, half a day at +12. Same error anywhere a timestamp is
  printed (stat modified is epoch ms, iso() renders UTC).
  WHY IT IS MISSING: Rust's std has NO local-time API. The
  zero-dependency path is to read the platform's own data —
  /etc/localtime is TZif v2 (1909 bytes here -> Europe/Zurich), a
  self-contained binary format, and TZ names a file under
  /usr/share/zoneinfo. Same kind of work as this project's regex
  engine and JSON parser.
  THE QUESTION IT MUST ANSWER, NOT DODGE: Windows has no TZif and we
  ship a Windows binary. Falling back to UTC is defensible ONLY if a
  script can tell it happened.
  MEASURED AND NOT CHOSEN: (1) "writing what other programs read" is
  ALREADY SOLVED — write_file(p, s, "append") exists; 300000 CSV rows
  appended one at a time cost 16 MB / 5.36 s against 157 MB / 4.91 s
  for the whole document, a tenth the memory for 9% more time.
  (2) money is not a trap here — int(float(t) * 100.0 + 0.5) is exact
  for all 100000 cent values; float sums do drift (6999.9999999921 vs
  7000.00) but the corpus already uses cents. (3) arithmetic edges
  are SOUND — overflow, 1/0 and int(1e30) all error rather than going
  quietly wrong. (4) a conditional expression is real but ergonomic —
  28 sites of `let x = A; if c { x = B; }` across six stdlib modules
  and three examples, plus two if/else assignment pairs; kept as a
  candidate, not chosen over a bug. (5) destructuring: ZERO sites.
- 762: local_zone() is the 73rd builtin — the local zone AT AN
  INSTANT (offset in ms east of UTC, abbr, dst) or nil. src/tz.rs
  reads the TZif file (RFC 8536) the machine keeps: /etc/localtime,
  or what TZ names under /usr/share/zoneinfo. Rust's std has NO
  local-time API, which is why this is a parser and not a call.
  VERIFIED AGAINST `date`: a minute either side of both 2026
  transitions; 1980-06-01 = +01:00 CET (June, and Switzerland kept no
  summer time until 1981 — a month-based guess fails this);
  1874 = +00:29:46 BMT local mean time (so an offset is a whole
  number of SECONDS, not minutes — the selftest must not assume
  minutes); TZ=America/New_York -04:00 EDT and TZ=Asia/Kolkata
  +05:30 IST. NIL IS A REFUSAL: Windows has no TZif and a TZ holding
  a POSIX rule names no file; reporting UTC there would be worse than
  the bug, since the caller could not tell. Selftests hold either way
  (everything specific behind `if here != nil`), so the Windows
  runner passes. THREW AWAY A FAKE TEST: the first TZ test asserted a
  COPY of the rule inline and agreed with itself; the path decision
  is now zone_path(tz) and the test calls it. 10 selftest checks
  (2533 -> 2543), 4 Rust tests (353 -> 357).
- 763: CI WENT RED on 762 — `cargo fmt --check`, all four runners.
  My gate ran the TING formatter, clippy and the suite and NEVER
  cargo fmt; it had not mattered while ticks touched ting and
  markdown, and 762 was the first hand-written Rust in a while.
  Fixed in four minutes, but the real repair is to the RULE: the gate
  in this file now says run what CI runs IN CI'S OWN WORDS
  (cargo fmt --check; cargo clippy --all-targets -- -D warnings;
  cargo test), not a paraphrase — a gate that summarises another gate
  drifts from it silently.
  Then the stroke: examples/organize.ting files by the LOCAL day.
  Two files an hour apart across local midnight (23:30 and 00:30)
  now land in 2026-09-06/ and 2026-09-07/; before, both went into
  2026-09-06/. THREE DECISIONS: the day is asked PER FILE (a year of
  history spans summer time changes); where local_zone is nil the
  example falls back to the UTC day and SAYS SO — checked by forcing
  it with a POSIX-rule TZ; and it says so on STDERR, because
  tests/examples.rs compares stdout to the .out and a
  platform-dependent line would fail the Windows runner. Report
  identical either way, verified under both.
- 764: lib/time gained local_date, local_clock, local_iso and
  offset_iso; stdlib 190 -> 194. local_iso + from_iso ROUND TRIP,
  which is why the offset goes in the string:
  2026-03-29T00:59Z -> ...T01:59+01:00 and 01:00Z -> ...T03:00+02:00,
  the hour that never existed visible in the pair.
  THE DECISION: the three that need a zone answer nil where there is
  none, NOT a silent UTC fallback — a library that returned the UTC
  day under the name local_date would have made 763's warning
  impossible to write. organize.ting now does the fallback in its own
  four lines, where a reader sees it. offset_iso TRUNCATES a
  sub-minute offset (+00:29:46 -> +00:29, as date prints it); the
  selftest asserts the truncation and guards the round-trip check
  with off % 60000 == 0, because for those instants there is no round
  trip to have. 12 checks (2543 -> 2555), run four ways: both
  engines, TZ=Asia/Kolkata, and a POSIX-rule TZ for the nil branch.
  THE 760 GUARD PAID FOR ITSELF: it caught STATE still saying 190 and
  the cookbook still carrying the old example, unprompted.
- 765: v2.123.0 released (144th tag, ordinal read from
  `git tag --sort=creatordate | grep -n`), carrying 762, 763 and 764.
  The 738 rule ran first and found nothing to repair for the third
  release running: `git show --stat d80a797` touched only CHANGELOG,
  Cargo.toml and Cargo.lock, and `## Unreleased` already held one
  entry per stroke; README's builtin count had moved to 73 in 762,
  where the builtin was added. Gate green IN CI'S OWN WORDS (763's
  repair): cargo fmt --check, cargo clippy --all-targets -D warnings,
  fifteen `test result: ok`, ting --fmt 0 of 69, corpus at seven.
  NOT VERIFIED YET.
- v2.123.0 VERIFIED (144th tag; strokes 762, 763, 764; both aarch64
  archives executed here). 766: six zones checked against `date`
  character for character on the DOWNLOADED binary, quarter-hour
  zones included (Chatham +12:45 is already on the next day while UTC
  is not); the nil branch shown on the artifact for a POSIX-rule TZ
  and for a TZ climbing out of the zone directory; and the milestone
  in three lines — two files written at one instant file into
  2026-09-06 under America/Los_Angeles and 2026-09-07 under UTC.
  Tarball lib/ checked byte-identical to the repo's with `diff -r`,
  not assumed. Site: ten paths 200, wasm 843223 (was 839346).
  READ FROM THE SOURCE, not the browser: in the playground
  local_zone(ms) is nil (no read_tzif on wasm) and local_zone()
  errors as time_ms() does — the same answer Windows gives.
- 767: LOCAL_ZONE ANSWERS ON WINDOWS. Not by reading the registry —
  its TZI blobs and per-year Dynamic DST entries are not TZif and
  re-implementing the rule evaluation would rewrite the one part
  Windows does correctly — but by asking the system:
  GetTimeZoneInformationForYear for that year, applied by
  SystemTimeToTzSpecificLocalTime, and THE OFFSET IS THE DIFFERENCE
  THE SYSTEM COMPUTED, not one derived here from a rule.
  THE DECISION on abbr: Windows has no abbreviations, only full
  localized names ("W. Europe Daylight Time"), and it returns those
  rather than the numeric form used for unnamed zones ("+1245").
  Uniformity was never real — a program comparing abbr to a fixed
  string was already wrong on Chatham — and a number is what offset
  already holds. Said so in the reference where a caller reads it.
  Five Rust tests (357 -> 362), anchors read from `date -u`, one of
  which caught a wrong constant on its first run.
  THE LAYOUT GUARD: three const size_of assertions for the repr(C)
  structs, each broken on purpose and watched to fail — a field in
  the wrong place does not fail to compile, it answers the wrong hour.
  The selftest needed NO new checks: 762 wrote it property-wise, so
  its assertions simply go live on the Windows runner.
- 767b: THE GUARD THAT WAS NOT RUNNING. Writing 767's entry made
  markdown_has_no_bare_html_tags fail from inside a fenced block,
  which it skips. It decided a fence by starts_with("```"), and
  iteration 711 wrote a paragraph beginning with an INLINE code span
  (three backticks, "text", three backticks). That toggled the fence
  open and nothing closed it, so the guard skipped everything after
  711 — FIFTY-SIX ITERATIONS of LOG.md — while reporting success. A
  fence line is now one whose backtick run is not closed on the same
  line. The repaired guard was proved by appending a bare tag to the
  END of LOG.md and watching it be reported, which the old one could
  not have done. The tail is otherwise clean. A GUARD THAT PASSES IS
  NOT A GUARD THAT RAN: this one counted files, not lines examined,
  and surfaced only because my own text flipped the parity back.
- 768: GREEN IS NOT EVIDENCE. 767's CI was green on all five jobs and
  said only that the code compiles on Windows and the const size_of
  layout assertions hold. selftest/time.ting is property-wise, so
  every property is satisfied by nil: a build where the whole Windows
  branch returned None would have been just as green, and the runner
  log confirmed no test called zone_at.
  THE TRAP: a runner that sits in UTC proves nothing, because a
  reader answering zero for everything passes.
  THE WAY OUT: GetTimeZoneInformationForYear takes a
  DYNAMIC_TIME_ZONE_INFORMATION carrying only a TimeZoneKeyName and
  uses THAT zone's rules — the freedom TZ gives on Unix. zone_at now
  calls an inner at_named(at, None); only tests pass a key. The six
  2026 rows of the Unix ZURICH table (read from `date`) are asserted
  on Windows through "W. Europe Standard Time": two platforms, two
  sources of zone data, the same six answers. Plus a machine's-own-
  zone test and one comparing against PowerShell.
  DELIBERATELY NOT ASSERTED: the 1980 and epoch rows (Windows keeps
  per-year rules for a couple of decades, not a century, so the
  platforms genuinely disagree and the reference says so), and an
  empty zone key (Windows falls back rather than failing, and I have
  not read how). FOUR Windows-only tests in tz.rs do not run on this
  host: the gate's count stays 362 here and is four higher there.
- 769: THE RUNNER PROVED IT. All four Windows-only tests passed by
  name in the log; a_named_zone_answers_what_the_zone_file_answers is
  the milestone — six Zurich instants, both 2026 transitions bracketed
  to the minute, out of the REGISTRY with the offsets `date` gives
  HERE. Two platforms, two unrelated sources, the same six answers.
  a_key_that_names_no_zone_answers_nothing settled the question 768
  refused to answer from memory: it does return nothing.
- v2.124.0 released (145th tag; strokes 767, 767b, 768). 738 rule ran
  first for the fourth release running and found nothing to repair.
  Two CHANGELOG lines added before the bump: the Windows anchors
  (they make the headline claim checkable) and the guard repair (it
  is about docs a reader trusts). Gate green in CI's own words across
  THREE targets now. NOT VERIFIED YET.
- v2.124.0 VERIFIED (145th tag; strokes 767, 767b, 768; both aarch64
  archives executed here). 770: three zones agree with `date` on the
  cold binary; selftest 2555, examples 22 of 22.
  THE HONEST LIMIT, WRITTEN DOWN: the Windows archive is the one this
  release changed and the one nobody here can run. "Both aarch64
  archives executed here" is the usual whole artifact check and this
  time it OMITS the platform the release is about. What can be said:
  the asset is there at 958584 bytes (up 3230), unzips to ting.exe
  plus the twelve modules, and the Windows RUNNER built and tested
  this exact source green — a different build of the same source than
  the zip, which is not the same sentence as "this file was run".
  ting.wasm SHRANK, 843223 -> 842947: the TZif reader is #[cfg(unix)]
  since 767, so the playground no longer carries a parser for files
  it cannot open.
- 771: health tick + audit green — MILESTONE "THE TIME IT IS HERE"
  COMPLETE (v2.123-v2.124; strokes 762, 763, 764, 767, 767b, 768).
  All nine bench checksums identical to BASELINE; timings a few per
  cent either side, which is this shared host. Fuzzers green in
  release: 50000 differential, 20000 formatter, crash, 2000000
  pattern.
  AND THE SWEEPS WERE CHECKED TO BE SWEEPS (700's finding is that
  naming the wrong target passes in no time having fuzzed nothing,
  which looks exactly like a fast green run): differential 0.4s at
  500 cases vs 10.5s at 50000, formatter 0.1s at 200 vs 4.1s at
  20000, pattern 0.6s default vs 3.9s at 2000000. A run that had
  fuzzed nothing would have been flat.
  Distribution: six assets on each of the last five tags; v2.29.0
  still carries its glibc warning.
  THE MILESTONE'S THREAD: never return UTC when the answer is
  unknown, because the caller cannot tell those apart afterwards.
- 772: replenishment — milestone "THE ARCHIVE THAT WAS RUN"
  (v2.125-v2.126), reasoning in LOG.md. THE EVIDENCE: release.yml
  builds, checks the glibc floor, packages and uploads, and HAS NO
  STEP THAT RUNS WHAT IT PACKAGED. Six archives per release since
  v2.30.0 (51st tag); v2.124.0 is the 145th, so 95 releases x 6 = 570
  archives. CI executes none (it tests a debug build from the source
  tree); I run two per release by hand, both aarch64, on this one
  machine = 190. So 380 ARCHIVES HAVE BEEN OFFERED TO STRANGERS
  WITHOUT ANYONE EVER STARTING THEM — x86_64 gnu, x86_64 musl, macOS
  and Windows, every release, 95 times.
  NOT THEORETICAL: v2.29.0 shipped Linux binaries that would not
  start, and the guard that came out of it reads objdump symbols
  rather than starting anything. 754 also lost time to a lib/ beside
  a binary shadowing the embedded stdlib — visible in an unpacked
  archive, invisible in a source tree.
  NOT CHOSEN, with reasons: streaming JSON and a bytes type (no
  measured pressure; 734's rule is to reverse a refusal on evidence,
  not appetite); string interpolation (still forbidden by the 2.x
  promise); --deps (deepest import chain is two); performance (bench
  stable, both engines agree to the checksum, nobody waiting);
  adopting try(f, ...args) — RECOUNTED TODAY at 64 sites, up from 53,
  but overwhelmingly in selftest/ where testing a closure is the
  point: a tidy-up, not a milestone.
- 773: THE RELEASE RUNS WHAT IT PACKAGES. tools/smoke.sh takes an
  unpacked archive and a version and does what a downloader does:
  starts the binary from where it was unpacked, checks --version
  against the tag, runs --test selftest, diffs every example.
  release.yml unpacks each archive on its own runner and runs it
  BEFORE upload (a target that will not start leaves its archive
  MISSING); ci.yml runs the same script on every push against an
  archive-shaped directory.
  THE LAYOUT IS DELIBERATE: the suites are copied in beside the
  binary, so selftest/ (no lib/ next to it) falls through to the
  EMBEDDED stdlib and examples/ ("../lib/...") gets the SHIPPED one.
  One run exercises both copies — the 754 trap.
  ALL THREE FAILURE MODES WERE MADE TO FAIL FIRST: wrong version, no
  binary, an example whose .out was edited to lie — each prints
  ::error:: AND EXITS 1, which is the part CI reads.
  A THIRTY-HOUR FINDING: the first run hung on examples/pipeline.ting,
  which reads stdin; tests/examples.rs never hit it because
  Command::output() closes stdin. Found a second ting still blocked
  the same way on reference/06.ting (input()), started by an ad-hoc
  script of mine THIRTY HOURS earlier and still sitting on this shared
  host. Killed. NEW RULE, in the script's own comment: a harness that
  runs corpus programs closes stdin, never inherits it.
- 774: I TESTED A TRANSCRIPTION OF THE STEP, NOT THE STEP. 773 went
  red on all four runners over three characters: the YAML carried
  `cut -d'\"'` (a backslash that belonged to the Python string that
  WROTE the yaml), so cut got a two-character delimiter and the
  version came through empty. The guard behaved perfectly — error
  line, exit 1, everywhere. The VERIFICATION failed: I "ran the step
  verbatim" by RETYPING it, and naturally typed what I meant.
  THE REPAIR IS THE METHOD, NOT THE QUOTING: tools/workflow_step.py
  prints a named step's run: block out of a workflow, and the
  rehearsal pipes those bytes into bash. Proved both ways — a copy of
  ci.yml with the old line exits 1, the real one exits 0.
  AND THE FIX NEARLY HID THE BUG: moving the version lookup into
  smoke.sh with ${2:-default} made the broken workflow line PASS,
  checking the archive against the tree's version instead of the
  tag's — the exact failure the check exists to prevent, restored by
  a convenience. Now ${2-default}: only an ABSENT argument defaults;
  an empty one is an error. Three checks: broken workflow 1, explicit
  "" 1, real workflow 0.
- 775: SMOKE RAN ON MACOS AND WINDOWS FOR THE FIRST TIME (CI green,
  and the step's own output read from the log rather than trusted):
  binary started from where it was unpacked, selftest against the
  embedded stdlib, 22 of 22 examples against the shipped lib/, on all
  four runners.
  AND FOUR MACHINES GAVE FOUR TOTALS: 2555 here, 2559 arm, 2564
  macOS, 2618 Windows. Per-file diff found ONE culprit everywhere —
  selftest/sh.ting asserted ONCE PER PATH ENTRY (82 entries on the
  Windows runner against 11 here = 71 phantom checks). Folded into
  one check that counts empties. PREDICTION WRITTEN BEFORE RUNNING —
  sh.ting 15, total 2545 — and both came out exactly.
  THE REMAINING VARIANCE IS DELIBERATE AND SINGULAR: where there is
  no `sh`, eight checks stand down, so Windows should read 2537 and
  everywhere else 2545. No doc quoted the total; STATE did.
- 776: THE PREDICTION WAS HALF WRONG. All four runners report
  sh.ting 15 / 2545 total — Windows included. I predicted 2537 there,
  reasoning the eight shell checks would stand down; they did not,
  because GitHub's Windows runner has Git Bash on PATH and
  which("sh") finds one. So my decomposition of the old 88 was wrong
  too: not 6 + 82 PATH entries with the block skipped, but 6 + 8 + 74
  with it running. Both give 88; I picked one and stated it as fact.
  The conclusion (the count moved with PATH length) was right; the
  story about Windows was not. 15 rather than 7 is the distinguishing
  evidence, and it was there to be asked for.
- 776: THE ARCHIVE THE WORLD DOWNLOADS IS NOW THE ONE THAT GETS RUN.
  The release ran the archive THIS RUNNER BUILT, which is a different
  claim: upload, storage and asset naming were untested until
  something fetched it back. A step after upload now downloads the
  asset by name, unpacks and smokes it. Rehearsed against the real
  published v2.124.0: 2545 checks, 22 of 22 examples, exit 0. Zip is
  unpacked with `tar -xf`, not unzip — tar on Windows is bsdtar and
  reads zip, Git Bash ships no unzip.
- 776: THE SECOND COPY OF THE STDLIB IS CHECKED. smoke.sh diffs the
  archive's lib/ against the tree's; a stale or partial copy would be
  invisible otherwise, since the binary keeps working on its embedded
  copy while a script beside the archive gets the other. Both
  failures made to happen (a line appended to lib/time.ting, and
  lib/csv.ting deleted): each exits 1 and names the file.
- 777: v2.125.0 released (146th tag; strokes 773, 774, 775, 776).
  738 rule ran first for the fifth release running, nothing to
  repair. Three CHANGELOG lines added before the bump (the downloaded
  archive, the two copies of the stdlib, the PATH-dependent count).
  Gate green across three targets. THE FIRST RELEASE THAT RUNS
  ITSELF: package six, unpack and run each on its own runner, upload,
  download back, run again — twelve executions of a ting binary where
  this morning there were zero. NOT VERIFIED YET.
- v2.125.0 VERIFIED, with one honest caveat (146th tag; strokes 773,
  774, 775, 776). ELEVEN OF TWELVE EXECUTIONS: all six archives were
  unpacked and run on their own runner before upload (2545 checks and
  22 of 22 examples each, read from the log per target, not from the
  job colour) — WHICH IS THE MILESTONE'S CLAIM AND IT HELD ON ALL
  SIX, WINDOWS INCLUDED. Five were also fetched back from the release
  page and run again; the Windows one was not.
  WHY: `tar` under Git Bash is GNU tar (C:\Program Files\Git\usr\bin),
  NOT the bsdtar in System32, and GNU tar does not read zip. I wrote
  the opposite in a comment one tick earlier, confidently, and it
  reached a tag.
  FIX uses a mechanism proven IN THAT SAME RUN: the download is
  unpacked per platform and the zip gets Expand-Archive, which
  succeeded on that very runner for the packaged archive. Unix path
  rehearsed verbatim from the file against the published v2.125.0.
  THE UPLOADED WINDOWS ZIP, as far as this host can go: 13 entries,
  ting.exe 2402816 bytes, lib/ byte-identical to `git archive
  v2.125.0 lib`.
  Six assets; both aarch64 archives cold here at 2.125.0; ten site
  paths 200, changelog.html carries v2.125.0.
- 779: THE RELEASE PAGE SAYS WHAT WAS CHECKED. A `finish` job with
  `needs: build` uploads SHA256SUMS and rewrites the notes — and
  because it is reachable ONLY when all six targets built, ran their
  packaged archive, uploaded, fetched back and ran that too, THE
  CLAIM IS TRUE BY CONSTRUCTION. One target failing leaves the plain
  notes `create` gave them; nobody is told a check happened that did
  not. The notes say a checksum served from the same page proves
  INTEGRITY, NOT PROVENANCE.
  Rehearsed against published v2.125.0 with the two mutating commands
  stubbed and everything else the bytes from the file; the gnu
  checksum matched a copy downloaded an hour earlier.
  CHECKED RATHER THAN ASSUMED: `gh release edit` DOES have
  --notes-file (my grep anchored on the long form and missed the
  `-F,` line — I nearly recorded that it did not).
  v2.125.0's notes deliberately NOT back-dated: the sentence is not
  true of that release, and back-dating it is the dishonesty the job
  exists to prevent.
- 780: v2.126.0 released (147th tag; strokes 778, 779). 738 rule ran
  first for the sixth release running, nothing to repair. Gate green
  across three targets. BOTH STROKES ARE UNPROVEN UNTIL THIS TAG
  RUNS, and they fail differently: 778's Expand-Archive on the
  downloaded zip (the previous release stopped at eleven executions),
  and 779's finish job, which HAS NEVER RUN — it needs all six build
  jobs green to be reachable, which is exactly what v2.125.0 failed.
  NOT VERIFIED YET.
- v2.126.0 VERIFIED (147th tag; strokes 778, 779). TWELVE OF TWELVE
  EXECUTIONS, counted per target from the log: every target started a
  ting binary twice, once as the archive it packaged and once as the
  archive it fetched back. 778's Expand-Archive held on the
  downloaded zip, as it had on the packaged one — the argument for
  choosing an observed mechanism over a claim about `tar`.
  779's finish job ran FOR THE FIRST TIME (it could not before:
  v2.125.0 had five green builds, not six). SHA256SUMS uploaded and
  ALL SIX LINES VERIFY HERE via `sha256sum -c` on a fresh download —
  including the two archives this host cannot execute, which is the
  one thing a machine of the wrong architecture can say about them.
  The release page now states every archive was run before it was
  offered, and that the checksums are integrity not provenance.
  NINETY-SIX RELEASES WENT OUT WITH FOUR OF SIX ARCHIVES NEVER
  STARTED BY ANYBODY. That is now false, and the page says so in a
  sentence `needs: build` makes unwriteable otherwise.
- 782: health tick green, milestone "the archive that was run"
  COMPLETE. All nine bench checksums identical to BASELINE (compared
  by parsing both tables, not by eye); timings 8-20% high across the
  board, uniform, weather. Three sweeps at seed 781 clean, each
  proven to be a sweep TWO ways: a case count in the output and a
  tenfold count taking tenfold the time (diff 1.3->14.4s, fmt
  0.4->4.2s, re 0.4->4.5s). Crash fuzzer 6 passed. Gate green,
  coverage 2635/2652.
  771's rule paid before the timings could: my first attempt gave
  `--exact` three test names typed from memory, every one a PREFIX of
  the real name, and --exact makes a prefix match NOTHING — three
  sweeps of zero cases, exit 0, one second. Read the names out of the
  files next time before running, not after.
  THE SEVENTEEN COVERAGE MISSES ARE NOW ENUMERATED, not gestured at
  (LOG 782): args 191-194,197-198 (exit paths), fs 226-227
  (cross-device fallback), sh 42-43 (Windows PATHEXT), test 95-97
  (the runner's own summary and exit), edge 90 (unreachable by
  design), time 128-130 (no-zone-data branch). Compare against this
  list, not against a total.
  SITE AUDIT RETARGETED AND CORRECTED — see below.
  Defect found and fixed: README and the `docs:` line in `ting --help`
  linked the site as plain `http`. HTTPS is not enforced on that
  domain, so http was served as http, not redirected. Both now https.
- 783: replenishment — milestone "THE LOOP THAT REBUILDS WHAT IT JUST
  BUILT" (v2.127-v2.128), reasoning in LOG.md. MEASURED at 80000
  appends on the release binary: `s = s + x` 7.863s vs `s += x`
  0.015s (520x); `xs = xs + [i]` 47.652s and `xs += [i]` 46.269s vs
  `push(xs, i)` 0.025s (1850x). Strings have one fast spelling out of
  two; LISTS HAVE NONE — push is the only linear way to grow one.
  Both quadratic because `+` always builds a fresh container.
  The fix needs no language change: extend in place when the target
  holds the only reference. The semantics to preserve are pinned by
  tests written today — `let t = r; r += [2];` leaves t as [1], and a
  string `+=` captured in a list leaves the copy alone; both are
  unobservable at count 1, which is exactly when it applies.
  docs/tutorial.md:196 TEACHES the quadratic form in a while loop;
  lib/time.ting:222 uses it in the shipped stdlib.
  Measured and NOT chosen: error messages (a runtime error already
  carries caret, source line, and every frame with its argument
  values); the shebang path (works, arguments and all).
- 783 also: THE PLAYGROUND WAS SERVING FOUR-DAY-OLD CODE. Six of the
  fifteen examples in playground/examples.js were the pre-v2.107
  versions. The guard read each example's FIRST NON-COMMENT LINE
  only, so everything below line one drifted while CI stayed green —
  the second inert guard this month after 767b's markdown fence.
  Regenerated, and the guard now compares each example's WHOLE body;
  proven by running the strengthened guard against the stale file CI
  had been passing, which failed. AUDITED: cookbook_matches_examples
  already compares whole source and whole output, which is why
  docs/cookbook.md never drifted. One sampled guard, not a habit.
  RULE: a guard over a generated file compares the whole file. A
  guard that samples is a guard that reports success.
- 784: A LIST THAT STOPS COPYING ITSELF. `xs += [x]` round a loop was
  quadratic; 200000 appends went 47.652s -> 0.021s. binary's
  (List, List) arm extends in place when Rc::strong_count is 1, and
  the three hard-coded string move-out sites became one predicate,
  eval::appends_in_place, naming the two pairs binary cannot fail on.
  The compiler's existing fuse condition (op is + AND the RHS
  cannot_reach the name) is what makes the moved-out value unshared.
  SEMANTICS PROVEN UNCHANGED: nine aliasing shapes run against a
  binary built from HEAD BEFORE the change and one built after, both
  engines, byte-identical. The `snaps` shape is the one to remember —
  pushing the growing list onto another list shares it every
  iteration, so that loop stays quadratic and must.
  THE GUARD WEIGHS BYTES, NOT COUNTS: a copy-per-iteration and an
  append-per-iteration both allocate about once round the loop; the
  SIZE is what goes quadratic. tests/alloc.rs gained bytes() beside
  allocations(). Old code 144188911 -> 576356719 B (x4.00); new
  310975 -> 601087 (x1.93). Proven by running the new guard against
  the old implementation: fails on the list, passes on the string, so
  it measures what changed rather than that something did.
- 785: THE LONG WAY ROUND COSTS THE SAME NOW. `x = x + y` fuses into
  the `x += y` path in both engines: 80000 appends went 7.863s ->
  0.032s for a string and 47.652s -> 0.036s for a list. binary was
  already right after 784; the cost was upstream, in the clone that
  reading a name performs. eval::folds_into_append recognises an Add
  whose left operand is Var(name) and whose right cannot_reach it.
  THE SPANS WERE THE DIFFICULTY and are preserved exactly: `q = q + 1`
  reports the READ ("undefined variable 'q'", at the q), `q += 1` the
  WRITE ("cannot assign to undefined variable"); `t = t + 1` puts the
  caret under `t + 1`, `t += 1` under the whole statement. So the
  fused op carries value.span, and Op::CheckVarRead joins Op::CheckVar
  to report boundness as a read. Read off BOTH binaries, not reasoned.
  Fifteen shapes (784's nine in the long form, plus prepending, an
  int, and two type errors) byte-identical old vs new, both engines.
  Alloc guards now hold BOTH spellings; proven against the old
  implementation where both new cases fail at exactly x4.00 bytes.
  Selftest 2551 -> 2555, ten bench checksums unchanged, 50000
  differential and 20000 formatter at seed 785 clean, Windows checked.
- 786: WHAT A CALL CANNOT REACH. `s += str(i)` inside a function is
  linear: 1.680s -> 0.030s at 80000 (56x), and `xs = xs + [str(i)]`
  with it. cannot_reach was aimed at the wrong thing -- fusing moves
  the READ after the right-hand side, so a right-hand side that reads
  the name is harmless and only one that ASSIGNS it can tell. For an
  Env binding those cannot be separated (any function assigns a
  global); in a frame they can, and the compiler ALREADY KNOWS:
  captured_names is an over-approximation of what closures capture,
  and a slot is given only to names it does not contain. So
  resolve(name) == Some(slot) means no closure even mentions the
  name, hence no call can reach it. The change is `slot.is_some() ||
  cannot_reach(value, name)`.
  Ten shapes byte-identical against the binary from before, both
  engines (closure-forces-Env, call reading the var through an
  argument, recursion, rebound `str`, failing call, snapshots,
  parameter, type error, loop variable).
  lib/time.ting:222 needed no rewrite after all -- frac is a slot.
  THE CORPUS CHECK CAUGHT THE TEST: `let str = fn(x)` in
  selftest/compound.ting added two warnings to a corpus pinned at
  seven (shadows a builtin; unused parameter). Correct behaviour, so
  the case moved to differential.rs's
  a_compound_append_does_not_disturb_what_it_appends_to, where the
  rest of this optimisation's edge cases live.
  Selftest 2555 -> 2558, Rust 364 -> 365, ten bench checksums
  unchanged, seeds 786 clean, Windows checked.
- 787: docs for the milestone -- and writing the rule down is what
  found that I DID NOT HAVE THE RULE. 786's STATE entry claimed a
  top-level binding still copies with a call on the right; measured,
  it is LINEAR (0.008 -> 0.028s at 20000 -> 80000). The top level has
  a frame and a capture set like any other body. Still sound, for the
  same reason as 786, and the proving case was already pinned in
  differential.rs and passing.
  THE REAL RULE IS ABOUT WHETHER ANY FUNCTION MENTIONS THE NAME, not
  about top level: no mention -> linear even with a call (x3.8); a
  function that merely READS it -> x27.2. reference.md now says that
  in those terms.
  SECOND FINDING: docs/tutorial.md:196's loop is quadratic, but not
  from the append -- `len` on a string counts characters. 80000
  appends cost 0.016s with a counter in the condition, 0.618s with
  len(s). Tried the ASCII fast path, measured 15%, reverted (see
  backlog). The example stays: it is the idiom and at width 12 it is
  instant.
- 788: v2.127.0 cut (148th tag; commit db3fc45). Release run
  34151404664 in flight. FOUR changelog entries, and ONE WAS WRONG
  UNTIL THIS TICK: 786's bullet claimed a top-level binding still
  copies, which 787 measured false. Caught before publication;
  corrected to the rule that was measured (no function mentions the
  name -> the saving holds). SECOND TIME THIS MILESTONE that writing
  for a reader found the error. A log entry is cheap to fix; a
  release note is permanent.
  Gate green at the new version, --version reports ting 2.127.0.
- 789: v2.127.0 VERIFIED (148th tag). TWELVE OF TWELVE executions
  counted per target; seven jobs green; seven assets; sha256sum -c OK
  on a fresh download of all six. Both aarch64 archives cold report
  ting 2.127.0 AND RUN THE MILESTONE'S OWN CHANGE: 80000 appends of
  str(i) in 0.030s (gnu) / 0.034s (musl), against 1.680s before it.
  A number from a local cargo build is about this working copy; that
  one is about the file on the page.
  Site: ten paths 200, ting.wasm 843780 (was 842967 -- Pages
  rebuilt), changelog at v2.127.0, reference serving the new cost
  note, and EXAMPLES.JS AS SERVED IS BYTE-IDENTICAL TO THE REPO's, so
  783's fix reached the front door and not just git.
- 790: health tick green, milestone "the loop that rebuilds what it
  just built" COMPLETE. All TEN bench checksums identical to BASELINE
  (growth.ting included). Sweeps at seed 790 clean, each proven a
  sweep two ways; crash fuzzer 6 passed. Gate green on all three
  targets. Coverage 2694/2711 AND THE SEVENTEEN MISSES ARE THE SAME
  SEVENTEEN 782 enumerated -- the denominator grew 59 lines and not
  one new line went uncovered, which is what comparing against a list
  rather than a total is for. CI green on HEAD, seven assets on each
  of the last two tags, ten site paths 200.
  A REGRESSION THAT WASN'T: the first bench run showed the VM LOSING
  regex (+27%) and strings (+9%), which has not happened since the VM
  took the lead. Second run: -29% and -32%. Host load average 4 from
  FOUR UNRELATED PROCESSES (checked they were not mine -- 773 was).
  The rule against asserting an ordering a loaded runner can reverse
  is exactly this; the answer was a second measurement.
- 791: replenishment — milestone "A CHARACTER AT A TIME"
  (v2.128-v2.129), reasoning in LOG.md. THE EVIDENCE: `s[i]` and
  `slice` build a `Vec<char>` OF THE WHOLE STRING on every call
  (src/eval.rs, the (Str, Int) index arm and the slice builtin), so
  walking a string is quadratic in time AND in bytes asked of the
  allocator: 0.665 / 2.430 / 9.707 / 39.404 s at 20000 / 40000 /
  80000 / 160000 chars, exactly x4 per doubling; slice(s,j,j+1) the
  same. FOUND BY ASKING WHAT TING IS LIKE AT THE SIZE ANOTHER PROGRAM
  WRITES: 7.6 MB CSV and 10.9 MB JSON. read_file 0.009s; json_parse
  0.390s (28 MB/s, a Rust builtin); csv["parse"] 6.018s (1.2 MB/s, a
  ting module) — 23x slower per byte for the PLAINER format; ting's
  own split over the same bytes 0.226s, i.e. 27x faster than the
  scanner built on it.
  NOT a csv_parse builtin: that papers over the primitive and leaves
  every other text-reading ting program where it is.
  Nine sites collect a whole string into a `Vec<char>`; five are the
  regex builtins, which need random access and pay once per call —
  those stay. THIS DOES NOT OVERTURN 693/695: they measured the SAME
  construction in the regex builtins and rightly found it cheap (341
  extra chars of subject, 0.70 us, ~2 ns each) because it happens
  once per CALL. Indexing does it once per CHARACTER.
  Measured and NOT chosen: streaming (the file fits — 7.6 MB read in
  9 ms; the cost is scanning); a bytes type; startup (1.1 ms for
  `ting hello.ting` against 11.7 ms for `python3 -c` and 0.6 ms for
  /bin/echo).
- 792: first stroke of "a character at a time" — indexing and `slice`
  walk with `chars().nth` / `char_indices` instead of collecting a
  `Vec<char>`; new `byte_of` helper in src/eval.rs. A bound counted
  from the START needs no length, so only a negative bound counts the
  characters. Index scan 10x faster: 0.665 -> 0.060 / 2.430 -> 0.243
  / 9.707 -> 1.061 / 39.404 -> 3.897 s at 20k/40k/80k/160k. STILL
  QUADRATIC (x4 per doubling) — the remaining cost is the CLONE, not
  the decode: `Value::Str` owns its text, so evaluating `s` copies the
  whole string and `len(s)` in a loop condition copies it again. That
  is the next two strokes.
  TWO CORRECTIONS TO 791'S FRAMING, both measured: (a) CSV IS NOT
  EXPLAINED BY INDEXING — 6.018 -> 6.453 s, unchanged, because
  lib/csv.ting's scanner uses `for c in text`, not indexing; the
  milestone stands on the index numbers above, not on that file.
  (b) THE PER-CHARACTER FLOOR IS 204 ns — bare `for c in text { n +=
  1; }` over 7.6M chars is 1.555 s; with a branch and an append
  2.625 s (345 ns). CSV's 6 s is ~790 ns/char, ~4x the floor, so most
  of it is the ting-level scanner, not the primitive.
  THE GUARD COULD NOT BE A RATIO: both spellings pay the same clone,
  so the old code passed a ratio test at x3.97. tests/alloc.rs's
  `reading_a_string_by_index_does_not_cost_the_string_each_time`
  measures BYTES PER CHARACTER PER READ absolutely (< 3); it reports
  9.0 and FAILS on the old implementation, passes on the new.
  Semantics checked byte-identical against a binary built from the
  previous commit across fourteen shapes on both engines; six of them
  now live in selftest/strings.ting.
- 793: second stroke — `Value::Str` holds a `value::Str`, an
  `Rc<String>`, so copying a ting string is a POINTER COPY. Writing
  still copies first unless nobody else holds the buffer (784's
  bargain for lists). TAKEN OUT OF BACKLOG ORDER ON PURPOSE: the
  caches and the sharing want the SAME migration of 120 `Value::Str`
  sites, and 792 measured the leftover cost of a read to be the copy,
  so the representation goes under the caches rather than beside
  them. Passing a 2 MB string to a function 400 times: 0.766 ->
  0.126 s. The 792 index scan 4.644 -> 2.542 s at 160000 chars (1.8x,
  STILL QUADRATIC — `chars().nth` walks, and that is the cache's job).
  The per-read guard now reports 0.037 bytes/char/read (2.02 before
  this, 9.0 before 792) and its threshold is 0.5, watched failing at
  2.02 against HEAD's src first.
  THE TESTS CAUGHT A REAL BUG: the derived `Debug` on the newtype
  printed `Str("a")`, and `Debug` is what `print` uses for a string
  inside a list or map. Ten tests failed on it; `Str` forwards `Debug`
  to its text. A newtype changes how a value LOOKS.
  Semantics byte-identical against c8a734e on both engines over every
  way to take a second reference (name, list element, map value,
  parameter, snapshot) plus both append spellings, `a = a + a`,
  non-ASCII through eleven builtins, JSON round-trip and both `for`s;
  six now assert in selftest/compound.ting. All ten bench checksums
  match.
- 794: third stroke — `value::Str` carries its CHARACTER COUNT in a
  Cell beside the text and inside the Rc, counted on the first ask.
  ONE NUMBER DOES BOTH JOBS: count == byte length means every
  character is one byte, so the nth starts at byte n and indexing
  does not walk. An append adds the piece's characters to the count
  rather than dropping it. THE SCAN IS LINEAR AT LAST: 0.010 / 0.033
  / 0.063 / 0.127 s at 20k/80k/160k/320k, x2 per doubling where every
  version before was x4. bench/scan.ting (NEW, the eleventh BASELINE
  row, and the row that would show a return to quadratic): 148.89 s
  on 792's binary, 117.49 s on 793's, 0.40 s now — 372x, same
  checksum on all three. `len` in a loop paid once: 200 calls over
  800000 chars 0.035 -> 0.012 s.
  WHAT IT COSTS, AND IT IS REAL: the first index into a string counts
  it where before it walked only to the character asked for (0.006 ->
  0.013 s for one index into each of 200 fresh 200k strings; the
  count runs at ~0.17 ns/char, at most once per string). And the
  7.6 MB CSV parse is 4% SLOWER — 4.15 -> 4.33 s over three
  interleaved runs, reproducible, because that workload makes
  millions of short strings and indexes almost none of them. 4%
  against 372x is the trade; it is in the log, not in the rounding.
  (CSV was 6.34 s on 792's binary, so the milestone is 32% ahead on
  the file that started it.)
  GUARD MADE TO FAIL FIRST: selftest/strings.ting asserts the count
  survives appends of MIXED CHARACTER WIDTHS; making `grew_by` add
  bytes instead of characters — the bug this kind of cache actually
  gets — fails it. Plus nine strings x thirteen indices x forty-five
  slice bound pairs x both engines, byte-identical to the 793 AND 792
  binaries.
  docs/reference.md said `len` walks the string and `while len(s) <
  width` is quadratic: TRUE WHEN WRITTEN, FALSE NOW, and rewritten.
- 795: fourth stroke, and 787'S CASE IS CLOSED. `s += str(n)` and
  `s = s + str(n)` cost what they add even where a function names the
  target. New `Op::AppendVar` folds Binary(Add) and SetVar into one
  step and, between the call and the add, asks the binding to LET GO
  of what was read — `Env::release_if_same`, by POINTER not equality,
  so a call that reassigned the name leaves the binding holding
  something else and it is left alone. The store that always follows
  fills the hole. GATED ON `appends_in_place`: only (str,str) and
  (list,list) cannot fail, so only they may leave a binding empty for
  one instruction. The tree-walker does the same in its own two slow
  paths, so the engines share the BEHAVIOUR and not only the answer.
  0.013 / 0.022 / 0.040 / 0.078 s at 20k/40k/80k/160k against 794's
  0.019 / 0.048 / 0.158 / 0.518 — x2 per doubling against x3.3, 6.6x
  at 160000 and growing; the long spelling matches to the millisecond,
  which is v2.127.0's promise kept.
  GUARD MADE TO FAIL FIRST: tests/alloc.rs's
  `a_call_on_the_right_appends_in_place_even_when_a_function_names_it`
  reports 6.5 MB / 28.6 MB (x4.4) against 794's src and fails.
  CORRECTNESS WAS THE WHOLE RISK: a call that reassigns the name, one
  that stashes the old value, one that only reads it, one that appends
  to the name itself, one that assigns it to itself, lists in both
  spellings, two type errors — byte-identical to the 794 binary on
  both engines, spans and traces included; six now assert in
  selftest/compound.ting. CSV unchanged at 4.26 s; eleven checksums
  match.
  docs/reference.md said the saving "holds only while the right-hand
  side names nothing and calls nothing": TRUE WHEN WRITTEN, FALSE NOW,
  rewritten. Third false claim this milestone retired from the docs.
- 796: v2.128.0 tagged (149th tag; strokes 792, 793, 794, 795).
  Release run 34170087973 in flight, verdict next tick. FIVE
  CHANGELOG ENTRIES, AND THE FIFTH IS THE COST: a program that makes
  very many short strings and never reads a character out of one pays
  a little for a remembered count nothing asks about. A release note
  that lists only what got better is one that gets corrected later
  (788 nearly shipped a false one). Every number in the section was
  measured this milestone and is in LOG 792-795 with its shape.
- 797: v2.128.0 VERIFIED (149th tag; strokes 792, 793, 794, 795).
  All six targets green per job from the API; six assets plus
  SHA256SUMS; both aarch64 archives cold-downloaded, checksummed,
  extracted and run here (2583 checks from each, plus a smoke script
  covering this milestone's own claims, identical on both engines).
  THE SHIPPED BINARY WAS TIMED, NOT JUST RUN: bench/scan.ting is
  0.390 s from the gnu archive and 0.512 s from the musl one, against
  148.89 s on the binary that shipped as v2.127.0. An archive built
  WITHOUT the milestone would have passed every other check.
  glibc floor GLIBC_2.34 (under the enforced 2.35); musl static.
  Site: ten paths 200, changelog.html carries v2.128.0, deployed from
  the release commit 09e335e.
- 798: health tick — MILESTONE "A CHARACTER AT A TIME" COMPLETE.
  All green: CI on 42b5659, no PRs, clean tree, --version 2.128.0,
  --fmt 0/71, corpus at seven, 2583 checks, both cross targets, fuzz
  sweeps at seed 798 (differential 5000/50000, fmt 2000/20000, regex
  200000/2000000, each reporting `1 passed`) and the crash fuzzer.
  THE 45 GB target/ IS DEALT WITH, and it had stopped being cosmetic:
  44 GB of it was target/debug (27 GB incremental + 17 GB deps),
  accumulated by `cargo clippy --all-targets` over hundreds of
  iterations because CARGO NEVER COLLECTS OLD INCREMENTAL SESSIONS.
  The release profile — what everything here actually runs — was
  788 MB. Removed target/debug outright; the rebuild cost was
  MEASURED, NOT ASSUMED: 5 seconds, because clippy checks rather than
  links. target/ is 1.1 GB, disk 58% -> 18% full, gate rerun cold and
  green. STANDING RULE FROM HERE: the health tick checks `du -sh
  target` and removes target/debug when it passes a few GB. A loop
  that runs indefinitely accumulates indefinitely.
- 799: replenishment — milestone "THE COST OF A STEP"
  (v2.129-v2.130), reasoning in LOG.md. THE EVIDENCE, BY COUNTING
  (a temporary counter in the VM dispatch loop, reverted): an empty
  `while i < n { i += 1; }` iteration is SEVEN OPCODES and 88 ns —
  12.6 ns per opcode, about thirty cycles here — and every ting
  program pays it before doing anything. bench/scan.ting is 19.4 M
  opcodes at 20.1 ns each; the CSV parse is 29 OPCODES PER CHARACTER.
  The tree-walker is 324 ns for the same iteration, so this is the
  FAST engine. lib/csv.ting's scan is 96% of the parse (--profile)
  and is already written the right way, so what is left is the cost
  of running a line of ting, not the ting.
  ALREADY SIZED: the dispatch loop loads chunk.spans[ip] — 16 bytes
  from a second array, bounds-checked — before EVERY instruction, for
  an error message almost no arm needs. Replacing it with a constant
  (experiment, reverted) is 9% off the empty loop, 3% off scan.ting,
  6% off the CSV parse, over three interleaved runs.
  MY FIRST GUESS WAS WRONG AND THE COUNTER SETTLED IT: `for c in
  text` and a while loop over text[i] now cost the same to within 1%
  (1.893 s vs 1.916 s over 7.6 MB), so the for-snapshot materialising
  the whole string is a MEMORY cost, not a speed one.
  NOT CHOSEN: a faster CSV in ting (the scanner is already right);
  perf/valgrind (not on this machine, sudo is not mine to use —
  ablation is the instrument and it answered); threading (one binary
  anyone can run, and nothing here is parallel).
- 800: first stroke — `Op::BinarySlotConst` does `GetSlot Const
  Binary` in ONE instruction. Chosen from a histogram, not a guess: a
  throwaway counter (reverted) over every contiguous pair and triple
  — NOT across jumps, since only straight-line neighbours can be
  fused — made it the top triple in FOUR OF FIVE programs (csv 9.5%,
  scan 9.9%, fib 18.2%, toplevel 8.6%; stdlib's top is `GetSlot
  GetSlot Binary` at 10.4%). In the CSV parse GetSlot alone is 27% of
  all instructions and JumpIfFalse 20%.
  EMITTED FROM THE AST, never by a peephole over the code, so no jump
  can land in the middle of what was fused; and ONLY A LITERAL may
  ride along, because anything that has to be run could fail or have
  an effect and its order is part of the language.
  Measured, three interleaved runs each: `if c == ","` in a loop -9%,
  bench/scan.ting -6%, the CSV parse -9.5%, fib's VM row 355 -> 307
  ms. The empty loop is UNCHANGED and that is right — its condition
  is two slots, which is the next fusion.
  NEW SUITE tests/bytecode.rs (now 16 suites): nothing else here
  would notice a superinstruction quietly ceasing to be emitted,
  since every answer would still be right. Five shapes that must fuse,
  four that must not. Made to fail first by disabling the fusion while
  keeping the opcode.
  799'S SPAN ESTIMATE WAS TOO HIGH AND IS WITHDRAWN: the real change
  (look the span up only where an arm needs it) is 1-3%, and hoisting
  a length assertion is nothing. 799's 9% came from replacing the
  span with a CONSTANT, which measures what deleting the whole thing
  is worth — AN UPPER BOUND, NOT AN ESTIMATE OF ANY AVAILABLE CHANGE.
  Both versions written, measured, REVERTED: 2-3% does not buy
  twenty-six sites reading `chunk.spans[ip]` where they read `span`.
- 801: second stroke — `Op::BinarySlots` does `GetSlot GetSlot
  Binary` in one. Empty loop `while i < n { i += 1; }` -13% (0.170 ->
  0.148 s for 2M iterations), CSV -2%, scan and stdlib unchanged
  within noise.
  THE LESSON, AND IT IS ABOUT THE HISTOGRAM ITSELF: stdlib's top
  triple was this at 10.4% of instructions and the clock says
  NOTHING, because bench/stdlib.ting waits on allocation, not
  dispatch. A SHARE OF THE INSTRUCTION COUNT IS NOT A SHARE OF THE
  TIME. Weigh that before choosing the next fusion from the same
  table.
  Both fusions together against 794's BASELINE (VM column): accum -9,
  fib -13, growth -15, json -14, lists -7, maps -15, regex -3, scan
  -5, stdlib -9, strings -16, toplevel -5 percent; every row
  improved, all eleven checksums match. BASELINE is regenerated at
  the release, in one go, not row by row.
  THE GUARD HAD A HOLE: tests/bytecode.rs only read the TOP-LEVEL
  chunk, so a fusion inside `fn f(x, y)` looked absent — every fn
  compiles to a chunk of its own and most code lives in one. ops()
  now gathers protos recursively, which also widens what 800's tests
  were checking.
- 802: third stroke — `Op::JumpIfFalseSlotConst` and
  `Op::JumpIfFalseSlots` do the comparison AND the branch, from one
  new `jump_if_false` helper that `if` and `while` now share. The
  condition is still asked whether it is a bool, so `if x + 1 {}` is
  the same error at the same span.
  HISTOGRAM RE-RUN FIRST (two fusions had changed it): the CSV parse
  now runs 19.5 M instructions where it ran 26.3 M — A QUARTER FEWER
  — and JumpIfFalse is now 26.6% of all instructions, ahead of GetSlot
  at 14.7%.
  Measured against 801: empty loop -6%, `if c == ","` in a loop -8%,
  scan -1%, CSV -2%. SMALLER THAN THE INSTRUCTION COUNT PROMISED,
  AGAIN: fusing 12.8% of the parse's instructions in pairs removes ~6%
  of them and buys 2% of the clock. Instructions are not what the
  machine waits on; the loop shapes, where the saved instruction sits
  on a dependency chain, are where it shows.
  KEPT ON TWO GROUNDS, not one: the empty loop is 0.140 s against
  0.176 at 799 (-20% across the milestone), AND `if` and `while` now
  share a method where they duplicated four lines each. For
  complexity alone it would be a poor trade.
  THE GUARD CAUGHT THE CHANGE BEFORE THE CLOCK DID: two
  tests/bytecode.rs cases failed the moment the fusion landed, because
  `if c == ","` no longer emits BinarySlotConst at all. Working as
  intended; the tests now say which shape belongs where, with a fourth
  case for conditions that must NOT fuse.
- 803: fourth stroke — A MEASURED NEGATIVE RESULT, and the end of the
  fusion work. The backlog named the last two pairs and set the bar in
  advance at 3% end to end. Both were written, both work, neither
  reaches it. `Op::JumpIfFalseSlot` (branch on a local without pushing
  it): -0.1% CSV, -0.5% scan, -1.8% tight loop — nothing anywhere.
  `Op::UpdateSlotConst` (`slot op= literal` without the constant on
  the stack) on top of it: +0.6% CSV, +1.3% scan, +7.1% tight loop.
  Four interleaved runs of each of three programs, best of four, host
  at load 0.4. Both kept all 2583 selftest checks passing on both
  engines, so this is what an opcode that WORKS and does not PAY looks
  like.
  A BENCHMARK BUILT OUT OF THE INSTRUCTION IS NOT AN END-TO-END
  MEASUREMENT — 801 and 802's lesson turned around. A loop whose body
  is `if i > 0 { n += 1; }` weights the fused pair far above what any
  program does with it; the two programs that do real work said 1.3%
  and 0.6%. Both reverted (`git checkout src/compile.rs src/vm.rs`),
  the rebuilt binary byte-identical to 802's by `cmp`. The opcode
  table stays at the six fusions that paid.
- v2.129.0 VERIFIED (150th tag; strokes 800, 801, 802, 803; seven
  assets, `sha256sum -c` OK on all six archives, both aarch64
  archives executed here, 2583 checks from each on both engines, and
  a probe outside the unpacked directory proving the EMBEDDED stdlib
  answers).
- 916: first stroke — THE SHAPE IT HANDS BACK. lib/err.ting and
  docs/stdlib.md documented `site` as `{"file", "line", "column"}`
  where the key is `col` — silently nil for anyone who followed it —
  and `--doc` for re_find/re_find_all/try named shapes without their
  contents (re_find's four keys, try's "at" and "trace"). All fixed;
  docs/reference.md had them right. The guard RUNS each builtin,
  takes the keys off the value, and asserts `--doc` names each at a
  word boundary, so a key added later cannot go unmentioned.
  Mutation-tested.
- 915: REPLENISHMENT — MILESTONE "THE KEY YOU TAKE OUT" (v2.143.0),
  reasoning in LOG.md. A ting map can be FILLED but not EMPTIED:
  `m[k] = v` writes in place, push/pop mutate a list in place, and
  there is no in-place removal of a map key anywhere — only
  lib/map.ting's `omit`, which rebuilds the whole map, while
  `m[k] = nil` STORES nil (len and has still count it). Draining a
  map one key at a time measured 448/1848/8431 ms at 1000/2000/4000
  keys — 4.1x then 4.6x per doubling. Insertion is exactly linear
  (7/15/31/64/136 ms at 10k-160k), so it is removal alone.
- 914: HEALTH TICK, milestone "the module says what it is" complete.
  Bench: eleven checksums identical, timings 5-10% over BASELINE at
  load 2.31 — weather, and NO head-to-head this time on purpose:
  907 ran one because that milestone put a predicate in the
  tree-walker's per-argument path, whereas this one touched `--doc`,
  hover and comments, none of which run while a program does. Sweeps
  green (50000 differential twice, 2000000 patterns, crash, 20000
  formatter). Coverage 3169/3186 (99%), unchanged, same six
  deliberate gaps. Site audit strong form: six pages byte-identical
  to the local renderer, examples.js identical, four playground paths
  200. Next tick: replenishment.
- v2.142.0 VERIFIED (163rd tag; strokes 909, 910, 911; seven assets,
  `sha256sum -c` OK on all six archives, both aarch64 archives
  executed here, 2769 checks from each on both engines, a probe
  outside the unpacked directory proving the EMBEDDED stdlib answers,
  and the milestone's own `--doc` output read from the DOWNLOADED
  binary).
- 912: RELEASE v2.142.0 (163rd tag; strokes 909, 910, 911). Gate
  re-run after the bump: 17 suites (448 tests), `--fmt .` 79
  unchanged, corpus at fourteen, 2769 checks on both engines, Windows
  and wasm. Next: verify it.
- 911: third stroke — WHAT THE HEADER SAYS, CHECKED AGAINST THE
  MODULE. docs/stdlib.md carries what an args spec IS, pinned to
  lib/args.ting's header line by line by a test. Comparing all
  thirteen headers with their page sections found they had drifted
  BOTH ways, invisible while a header was an internal comment and
  visible since 909: lib/time.ting said "there is no time zone here"
  while exporting local_date/local_clock/local_iso (the page repeated
  it three rows above its own list of them) — both now say the module
  carries no zone DATABASE and the local_ functions ask the platform
  through local_zone(), guarded; base64's header said decoding
  refuses non-base64 while the page said it skips a wrapped
  document's line breaks, and MEASUREMENT says both, so both say
  both; csv's header did not mention the BOM it drops.
- 910: second stroke — THE BINDING AND THE NAME IT SHARES.
  `lsp::imported_modules` pairs each `let NAME = import("PATH");`
  with the embedded module its path ends in, and hovering the BINDING
  answers with the module's header and its function count (the branch
  sits AFTER the stdlib-function one, so `sum` is still the function).
  And `--doc map`/`--doc args` now end with a pointer at
  lib/map.ting/lib/args.ting: those two are the ONLY short names a
  builtin also owns (counted, not guessed), and they were hiding the
  module behind the one word a reader would try. The pointer lives in
  `doc_text`, the single place `--doc` and `:doc` share.
- 909: first stroke — THE MODULE SAYS WHAT IT IS. `lsp::source_header`
  reads a file's leading `#` block and `--doc` prints it: the whole
  header above the members when a module is asked for by name, the
  first sentence on the module's own line in the table of contents.
  The block is the FILE's only when a BLANK LINE follows it —
  otherwise it is the first declaration's, and source_functions
  already hands it out. Header lines keep their own indentation and
  are NOT wrapped, because a header is prose AND worked examples
  (args' spec, test's seven calls); only the source keeps them inside
  eighty columns, so the width test covers that module now. Guard
  walks lib/ and asserts all thirteen headers reach `--doc`, asking
  BY PATH since `args` is the builtin.
- 908: REPLENISHMENT — MILESTONE "THE MODULE SAYS WHAT IT IS"
  (v2.142.0), reasoning in LOG.md. Probed by WRITING programs: a CLI
  built on lib/args.ting needs a "spec", and what a spec is appears
  in no tool — only in the first 24 lines of lib/args.ting, a header
  comment `--doc` never prints. All thirteen modules have one (112
  lines between them) and so do users' own files; hover answers for
  builtins, imported stdlib functions and this file's functions but
  not for the module BINDING; and `--doc args` means the builtin
  without mentioning that the module exists.
- 907: HEALTH TICK, milestone "the failure tells you why" complete.
  Bench: eleven checksums identical, but every timing was up on
  BASELINE (eval ~10%, vm ~8%) and 903 had put a predicate in the
  tree-walker's per-argument path, so weather was NOT allowed to be
  the answer: v2.140.0 built in a worktree and run against HEAD
  interleaved, three rounds, best of three — median eval -0.3%,
  median vm -0.4%, no regression. Sweeps green (50000 differential
  twice, 2000000 patterns, crash, 20000 formatter). Coverage
  3169/3186 (99%), unchanged; `summary()` in lib/test.ting is
  uncovered BY DESIGN (a selftest calling it with a failure would
  exit 1) and its exit path is covered from tests/io.rs instead.
  Site audit strong form: six pages byte-identical to the local
  renderer, examples.js identical, four playground paths 200. Next
  tick: replenishment.
- v2.141.0 VERIFIED (162nd tag; strokes 901, 902, 903, 904; seven
  assets, `sha256sum -c` OK on all six archives, both aarch64
  archives executed here, 2769 checks from each on both engines, a
  probe outside the unpacked directory proving the EMBEDDED stdlib
  answers, and the milestone's own diagnostics read from the
  DOWNLOADED binary).
- 905: RELEASE v2.141.0 (162nd tag; strokes 901, 902, 903, 904).
  Gate re-run after the bump: 17 suites (440 tests), `--fmt .` 79
  unchanged, corpus at fourteen, 2769 checks on both engines,
  Windows and wasm. Next: verify it.
- 904: fourth stroke — THE PAGE SAYS WHAT A FAILURE LOOKS LIKE.
  docs/reference.md's `--test` bullet now states that a failing
  file's own output is repeated under the FAIL line before the error
  (forty-line cap, head kept), that a passing file stays silent, and
  that the check count survives a failure including `exit()`; the
  `assert` row and the builtin's summary in value.rs say a refused
  comparison shows both sides. The row QUOTES the diagnostic and a
  new docs test runs that program and asserts the page carries what
  the binary printed — a quoted diagnostic rots quietly otherwise.
  Twelve docs tests. Next: release v2.141.0.
- 903: third stroke — THE ASSERTION SHOWS ITS WORK. `assertion
  failed: three kilos (9 == 8)`. The two sides reach the builtin
  through the `Interpreter` (`set_compared`), filled by the
  tree-walker inline and by the VM's new `CompareShowing` opcode,
  with ONE shared predicate (`is_assert_comparison`) deciding the
  shape for both — syntactic, since the compiler cannot know what
  `assert` is bound to. Kept only when the comparison is FALSE;
  `assert` takes the pair only if its span lies inside its own call
  (so a shadowed `assert` cannot leave one behind); the predicate
  tests the argument shape before the callee name, because it runs
  for every argument of every call. Thirteen differential cases hold
  the engines to the same text.
- 902: second stroke — THE COUNT SURVIVES THE EXIT.
  `eval::report_checks_if_asked` is the one place that prints the
  `ting-checks:` line, called at the end of a run AND inside
  `Builtin::Exit`, so a file that fails through `summary()`'s
  `exit(1)` no longer reports zero checks. The test pins the number
  in the summary AND that the private line never reaches the reader,
  now that 901 shows a failing file's output.
- 901: first stroke — THE HARNESS REPEATS THE REASON. `--test` shows
  a failing file's own stdout under the FAIL line (the fix was
  deleting `.stdout(Stdio::null())` from `run_one`); what the file
  printed comes before what killed it, a passing file stays silent,
  and a flood is cut at forty lines keeping the HEAD, with a count.
- 900: REPLENISHMENT — MILESTONE "THE FAILURE TELLS YOU WHY"
  (v2.141.0), reasoning in LOG.md. Probed: RUNTIME ERRORS ARE ALREADY
  GOOD (types named, caret, a note per call with argument values), so
  the seam is what happens when a TEST fails. `--test` spawns each
  child with `.stdout(Stdio::null())`, so a file that prints why it
  failed — which is what `lib/test.ting`'s `summary()` does — reports
  only `FAIL <path>`; `summary()` then ends with `exit(1)`, which
  skips the `ting-checks:` line, so a failing file also reports ZERO
  checks. And `assert` shows neither side of the comparison it
  refused.
- 899: HEALTH TICK, milestone "every mistake at once" complete.
  Bench: eleven checksums identical to BASELINE, compared
  mechanically; timings a few percent slower than 890's across the
  board, which is weather on a shared host — the recovering parser
  only runs where the strict one FAILED. Sweeps green (50000
  differential twice, 2000000 patterns, crash — now exercising
  recovery too — 20000 formatter). Coverage 3169/3186 (99%),
  unchanged. Site audit strong form: six pages byte-identical to the
  local renderer, three playground paths 200. Next tick:
  replenishment.
- 898: v2.140.0 VERIFIED (161st tag; strokes 892, 893, 894, 895, 896;
  seven assets, `sha256sum -c` OK on all six, both aarch64 archives
  unpacked and run here, 2769 checks and 22 clean examples from each,
  three errors from `--check` on a three-typo file out of both
  builds, and the embedded stdlib answering outside the unpacked
  directory). Site: reference and changelog pages byte-identical to
  the local renderer.
- 897: RELEASE v2.140.0.
- 896: fifth stroke — WHAT RECOVERY PROMISES. The reference's
  `--check` bullet states the contract (every error, where recovery
  resumes, line order, no position twice, twenty per file, and why a
  broken file gets syntax errors ONLY); the `--lsp` bullet carries
  895's navigation/judgement line where a reader can see it; the
  `--fmt` bullet admits the formatter works on TOKENS, so it
  reformats a file that does not parse.
- 895: fourth stroke — THE EDITOR KEEPS ANSWERING. NAVIGATION YES,
  JUDGEMENT NO: `document_symbols`, `workspace_symbols`,
  `definition_result` and `user_fn_params` read the recovering
  parser's partial tree, so an outline keeps the functions either side
  of the line being typed; the five warning walks
  (`unreachable_code`, `duplicate_map_keys`, `arity_mismatches`,
  `unbound_findings`, `unused_top_level_lets`) KEEP the strict parser,
  because half a file supports no judgement — a name bound in a
  failed statement looks bound nowhere. The test holds both halves and
  was mutation-checked.
- 894: third stroke — THE EDITOR MARKS THEM ALL. `lsp::diagnostics`
  publishes every syntax error on the checker's rules; three typos
  underline three places and clear together. The test pins the COUNT
  and the lines, and was mutation-checked by putting `.take(1)` back.
- 893: second stroke — THE CHECKER SAYS ALL OF IT. `check_source_all`
  (lib.rs) gives `--check` every syntax error in a file; the
  playground's check button gets the same list through `ting_check`.
  After syntax errors THE COMPILER DOES NOT RUN: what parsed is the
  statements around the mistakes, so it would invent complaints about
  a program nobody wrote. `check_source` is unchanged for callers
  that want one answer. The test pins line numbers as well as the
  count, since "reports more" is satisfied by reporting one mistake
  twice.
- 892: first stroke — THE PARSER CARRIES ON.
  `parser::parse_program_recovering` gives the statements that parsed
  and every error, in line order; `parse_program` is untouched, so
  running a program still stops at the first. Recovery skips past a
  `;` at depth zero, out of the braces the error was inside, or up to
  a statement keyword — and ALWAYS CONSUMES AT LEAST ONE TOKEN, which
  is the property that keeps an editor from spinning. No span reports
  twice; MAX_PARSE_ERRORS caps a file at twenty. The crash fuzzer now
  runs recovery over everything that fails to parse, so termination
  is proved by seeds rather than by a list.
- 891: REPLENISHMENT — MILESTONE "EVERY MISTAKE AT ONCE" (v2.140.0),
  reasoning in LOG.md. Probed first: arithmetic edges are ALREADY
  TENDED (overflow, division by zero, `int(1e20)`, NaN, truncating
  `%`, `1 == 1.0`, hex in `int`) and so is runaway recursion (`stack
  overflow (max call depth 4096)` with argument values, no crash).
  The seam is that the parser reports THE FIRST syntax error and
  stops, so `--check` and the LSP each show one mistake at a time
  while every other diagnostic here comes in batches.
- 890: HEALTH TICK, milestone "the file you were given" complete.
  Bench: eleven checksums identical to BASELINE, compared
  mechanically; timings recorded, not read. Sweeps green (50000
  differential twice, 2000000 patterns, crash, 20000 formatter).
  Coverage 3169/3186 (99%), selftest/bytes.ting at 100%, the
  remaining gaps the same platform-shaped lines as before. Site audit
  in its strong form: all six pages byte-identical to what
  tools/md2html.ting renders here, three playground paths 200. Next
  tick: replenishment.
- 889: v2.139.0 VERIFIED (160th tag; strokes 883, 884, 885, 886, 887;
  seven assets, `sha256sum -c` OK on all six, both aarch64 archives
  unpacked and run here, 2769 checks and 22 clean examples from each,
  and a probe outside the unpacked directory proving the EMBEDDED
  stdlib answers and the release's own strict and lossy reads behave).
  Site: changelog and reference pages live with the new material.
- 888: RELEASE v2.139.0. CI green for da7b246 on all four platforms
  first — the Windows job is what proves `-text` holds `crlf.txt`
  through a checkout with `core.autocrlf` set, which this host cannot
  test.
- 887: CI RED, FIXED — A NAME THAT CANNOT BE CLONED. 886's fixture
  `nul.txt` killed the whole Windows job inside `actions/checkout`
  (`error: invalid path`): `NUL` is a reserved DOS device name and
  git for Windows will not write such a path. Renamed `has_nul.txt`.
  THE LOCAL GATE COULD NOT HAVE SEEN IT: this host builds the Windows
  TARGET but has never run a Windows CHECKOUT. Guard that needs no
  Windows: `every_path_here_can_exist_on_windows` (tests/tools.rs)
  walks the tree for reserved device stems, names ending in a space
  or dot, and the characters Windows paths cannot hold. Mutation
  tested.
- 886: fourth stroke — DIRTY FIXTURES, COMMITTED. A ting string is
  always text, so the self-hosted suite CANNOT PRODUCE the bytes it
  needs to be handed: `selftest/fixtures/` holds five committed files
  (latin-1, a character cut in half at EOF, a NUL, CRLF, a BOM) and
  `.gitattributes` marks the directory `-text`, since the `eol=lf`
  above it would have normalised `crlf.txt` into something else —
  proved with `git cat-file -p :path` and `cmp`, not assumed. A
  fixture may never be `.ting`: `--fmt .` and `--check` walk the tree
  for those, so an unreadable one would fail the gate instead of
  being tested by it. `selftest/bytes.ting` asserts what 885
  documented, in the language, on both engines.
- 885: third stroke — ONE PLACE THAT SAYS WHAT A BAD BYTE MEANS. The
  reference gained *Bytes that are not text*, organised by the ANSWER
  rather than the door, since there are only three in the language:
  refuse and say where; read it anyway when asked (`"lossy"`);
  replace always (`run()`, and why it is the odd one). The `run()`
  row, which had grown into a design essay inside a table cell, is
  one clause pointing at it. All three refusal shapes were re-read
  off the binary before being written down. The docs guard's
  reference counts moved to (0, 7, 4) for the two illustration
  blocks.
- 884: second stroke — A DELIBERATE WAY TO READ ANYWAY.
  `read_file(path, "lossy")`, `each_line(path, f, "lossy")` and
  `input("lossy")` follow the mode string `write_file` already took,
  and produce exactly what `run()` has always produced: a
  replacement character per bad byte. One helper each side —
  `read_mode` in the evaluator, the lossy twins in `diag` — so
  strict and lossy read the same bytes through the same code. A mode
  nobody has is refused by name. BOTH NEW TESTS FAILED FIRST TIME ON
  MY OWN ARITHMETIC: I predicted six characters where a seven-byte
  line with one bad byte gives seven. A lossy read SUBSTITUTES into
  the line, it does not shrink it.
- 883: first stroke — EVERY "not UTF-8" SAYS WHERE. The wording was
  never the problem: `diag::read_why` is handed an `std::io::Error`,
  which knows no position because `read_to_string` threw the bytes
  away before failing. The READS changed — read bytes, convert here,
  and `from_utf8` hands back `valid_up_to`. A whole file says `byte
  0xe9 at offset 10 (line 2, byte 3)`; `each_line` says `byte 3 of
  line 2`; `input()` says `byte 3 of the line`, because a stream
  nobody numbered has no line number — THE FIRST DRAFT INVENTED ONE,
  saying "line 1" about the third line of a stream. A SILENT WRONG
  ANSWER fell out of it: `import` treated unreadable as absent, so a
  corrupt lib/list.ting made the EMBEDDED module answer instead; a
  file that is there is not a missing file.
- 882: REPLENISHMENT — MILESTONE "THE FILE YOU WERE GIVEN"
  (v2.139.0), reasoning in LOG.md. A twelfth kind of looking: not the
  program, but what the program is HANDED. Probed first: one byte
  that is not UTF-8 makes a file unreadable through every door
  (`read_file`, `each_line`, running a script, `--check`, `--fmt`)
  with a message that never says WHERE, and there is no way to
  proceed anyway — `input()` even dies mid-stream after printing the
  good lines. Meanwhile `run()` has always read a program's output
  LOSSILY and nothing documents it. Size is not the problem: a
  ten-megabyte line reads in 6 ms.
  AND THE DEBT STATE SAID WAS OWED WAS ALREADY PAID: 787's two
  string cliffs are gone (`Rc<Repr>` with a kept character count), all
  four shapes linear at 20000/40000/80000 appends. The notes are
  corrected below rather than left to send a future tick after
  ghosts.
- 881: HEALTH TICK, milestone "the project builds itself" complete.
  Bench: eleven checksums match BASELINE on both engines, compared
  mechanically. No head-to-head against the last release — nothing
  here touched the hot paths, and weather against weather only
  invites reading noise as a result. Sweeps green (50000 differential
  twice, 2000000 patterns, crash, 20000 formatter). THE SITE AUDIT IS
  A DIFFERENT QUESTION NOW: the six live pages were fetched and
  compared BYTE FOR BYTE against what tools/md2html.ting renders
  here — all six identical, live examples.js identical too, nine
  paths 200, and all three generators left the tree clean. Coverage
  found lib/base64.ting at 106/107: the missed line was the `fail`
  for a character that starts and is not continued, which four error
  checks had not reached. Closed; 100%, suite at 2749. Next tick:
  replenishment.
- 880: v2.138.0 VERIFIED (159th tag; strokes 871, 872, 873, 874, 875,
  876, 877, 878; seven assets, `sha256sum -c` OK on all six, both
  aarch64 archives unpacked and run here, 2748 checks from each on
  both engines, and a probe in a directory holding only itself
  proving the EMBEDDED stdlib answers — plus base64, mono_ms, the
  named matcher error and the short JSON escapes asked of the release
  binary directly). The site is live and ting-rendered: changelog
  carries v2.138.0, reference carries mono_ms. THE PROBE'S 207 IS
  BINDINGS, NOT FUNCTIONS — 203 functions plus test's `state`, csv's
  `BOM`, base64's `STANDARD` and `URL`; checked instead of written
  down.
- 879: v2.138.0 TAGGED, milestone "the project builds itself" — five
  ports, one new builtin, one stdlib module, one matcher repair, one
  JSON fix, and no Python left in the tree. Release tick started with
  `git diff v2.137.0..HEAD` as it should. The CHANGELOG section is
  dated the day the release was CUT, not the day the work was done —
  the tick crossed midnight and the first draft had yesterday.
- 878: eighth stroke — bench/run.ting, NO PYTHON LEFT IN THE TREE.
  `--eval`/`--vm` replace the TING_ENGINE variable, ting's `run`
  taking no environment. THE COMPARISON CANNOT BE BYTE FOR BYTE — two
  harnesses timing the same programs disagree by definition — so it
  was checksums, shape, and numbers within noise: four passes
  alternating which ran first, all eleven checksums identical every
  time, every median within 3.9% with the differences going BOTH WAYS
  (accum -1.8%, maps +2.2%), which noise looks like and bias does not.
  The printed table matched column for column without aiming for it.
  BASELINE regenerated: eleven checksums unchanged, timings tonight's
  at load 1.1. The python guard is now the whole tree, not tools/.
- 877: seventh stroke — `mono_ms()`, THE CLOCK A BENCHMARK NEEDS.
  Started porting bench/run.py and stopped at its first line: ting's
  only clock was `time_ms()`, the WALL CLOCK, an integer that can step
  backwards, so a duration measured with it can come out negative. The
  74th builtin is milliseconds since the process started, as a float,
  zeroed at the first call, refusing on wasm32 as time_ms does. Four
  selftest checks on both engines. A BUILTIN IS COUNTED IN A THIRD
  PLACE: editor/ting.tmLanguage.json carries the alternation
  tests/grammar.rs holds src/value.rs to — the test caught it. README
  had also been claiming thirteen modules while listing twelve since
  871. NOTE FOR THE PORT: BASELINE cannot be compared byte for byte,
  timings differ every run; checksums and shape can, timings only
  within noise.
- 876: sixth stroke — tools/cookbook.ting, and NOTHING IN tools/ IS
  WRITTEN IN PYTHON. docs/cookbook.md byte for byte, plus eight
  generated examples (no blurb, several, a lone hash, trailing spaces
  in source and output, empty, no final newline, unicode, comments
  only). THE CARE POINT: Python's `rstrip("\n")` takes newlines,
  ting's `trim_end` takes all whitespace, so the port carries its own
  `without_trailing_newlines`. tests/tools.rs guards the generator
  against the committed page and forbids a .py in tools/. bench/run.py
  survives — it writes BASELINE.md, is in no build, and decides what a
  release publishes, so it is its own stroke.
- 875: fifth stroke — tools/md2html.ting, THE SITE IS RENDERED BY THE
  LANGUAGE IT DOCUMENTS. The six published pages byte for byte on the
  first run, then five hostile documents (escaped pipes, a four-hash
  line, list continuations, unclosed bold, links with parentheses, an
  empty file), then the workflow step itself pulled out with 873's
  tool and piped into bash. pages.yml builds the host binary and runs
  the ting renderer; NO WORKFLOW RUNS PYTHON ANY MORE and
  tests/tools.rs holds them to it. `re_find_all` carries start/end/
  groups, which is how links are rewritten without a replacement
  callback; the lookbehind the table splitter used is a character
  walk now, and reads better. The renderer's guard holds it to the
  DOCUMENT (a pre per fence, a run link per ting block, a tag per
  header), counted the way the renderer reads the file — the first
  version forgot fences and tripped on `# ` comments inside code.
- 874: fourth stroke — tools/playground_examples.ting, and THE TWO
  ESCAPES TING NEVER WROTE. The port is shorter than the Python
  because `json_str(entries, 2)` already is `json.dumps(indent=2,
  ensure_ascii=False)`: first run rewrote playground/examples.js byte
  for byte. Fourteen generated examples of hostile text then found
  the difference the real data could not: `json_str` spelled
  backspace and form feed as long escapes THE DECODER HAS ALWAYS READ
  BACK and no other writer produces — fixed in src/json.rs, two
  selftest checks. One difference stands on purpose: Python's
  text-mode read translates CR, `read_file` hands over the bytes, and
  the bytes are what the playground should run. tests/tools.rs runs
  the generator in a directory holding only examples/ and demands the
  committed file back, which also proves its import reaches the
  embedded stdlib.
- 873: third stroke — tools/workflow_step.ting, THE FIRST TOOL THAT
  BUILDS ITSELF. The rehearsal tool ported and the Python deleted,
  after proving the two byte for byte on every named step in every
  workflow (24 across ci.yml and release.yml, exit codes included;
  pages.yml's steps are unnamed, so neither sees them). tests/tools.rs
  is the seventeenth suite and the home for the next two ports. THE
  GUARD CANNOT COMPARE THE BLOCK AGAINST A COPY OF THE BLOCK — that
  copy is the mistake the tool exists to prevent — so it checks the
  block against its place in the file: adjacent, in order, opened by
  the `run:` above it, closed by a line that leaves it. The first
  version only asked whether each line existed somewhere and passed a
  tool that dropped the last one; the shipped version fails all three
  mutations. tools/ now joins the `--check` corpus.
- 872: second stroke — THE MATCHER NAMES THE CONSTRUCT IT CANNOT
  COMPILE. `(?` now looks at what follows: `(?:` is the group, every
  other spelling is refused by name (lookahead, lookbehind and their
  negatives, named groups in all three spellings, atomic groups,
  group comments, inline flags, and a fallback), instead of falling
  through to the atom parser and coming back "nothing to repeat at
  2" — a message about a character the pattern never meant to write.
  `\1` went with it, and it was worse than a bad message: a
  backreference was READ AS THE DIGIT, so `(a)\1` quietly matched
  `a1`. Fifteen unit tests, three selftest checks (2739 -> 2742) so
  the message is visible from ting, one sentence in the reference.
  Nothing in the corpus wrote either construct, which is why neither
  had been noticed.
- 871: first stroke — lib/base64.ting, THE THIRTEENTH MODULE. Both
  alphabets (standard padded, URL-safe unpadded), decoding either,
  and `bytes`/`from_bytes`/`decode_bytes` exported because ting has
  no bytes type and every byte-oriented format needs them. Checked
  against Python's `base64` on ten strings: byte for byte, emoji
  included. Selftest carries the RFC 4648 vectors and the four ways
  to be wrong; 2690 -> 2739 checks over 23 files. A new module costs
  a count in six places plus two module-counting assertions in
  tests/selftest.rs. NOTE: `"\xNN"` IS NOT A TING ESCAPE — strings
  are text — so a test needing byte values 62 and 63 uses `😀` and
  `ÿ?`, and two alphabets are compared as BYTES, not as text.
- 870: REPLENISHMENT — MILESTONE "THE PROJECT BUILDS ITSELF"
  (v2.138.0), reasoning in LOG.md. An eleventh kind of looking: the
  work THIS REPO does. tools/ is 354 lines of Python and bash that
  build the site, generate playground examples and cut a step out of
  a workflow; a scripting language whose own repo scripts in
  something else has never been asked whether it can do the job.
  MEASURED FIRST: base64url of UTF-8 written in ting matches Python
  byte for byte on seven cases including emoji, every regex md2html
  needs works (re_replace with $1 included), and the two absences —
  negative lookbehind and callback replacement — are a few lines of
  scanning. FOUND: `(?<!...)` and `(?i)` both report "nothing to
  repeat at 2", the message for a bare `*`.
- 869: HEALTH TICK GREEN — MILESTONE "WHAT A LONG-RUNNING PROGRAM
  KEEPS" (v2.137.0, strokes 862-868) COMPLETE. Eleven checksums
  match on both engines, and the interleaved A/B against a v2.136.0
  build (now standing practice) shows the VM FASTER across the board
  — maps 145/170, regex 192/207, lists 126/131, json 112/116, fib
  312/326 — because v2.136.0 shipped 862's slow drop; the
  tree-walker is mixed within a few per cent. Toplevel looked 8%
  slower on three samples and was 1% on seven: GO BACK FOR MORE
  SAMPLES BEFORE BELIEVING A SINGLE PASS. 863's duration probes
  re-run: churn flat at 19.5 MB on both engines, LSP flat at 3.1 MB
  over a thousand edits. Sweeps green (50000 differential twice,
  2000000 patterns, crash, 20000 formatter). Next tick:
  replenishment.
- 868: v2.137.0 VERIFIED (158th tag; strokes 862, 864, 865, 866;
  seven assets, `sha256sum -c` OK on all six, both aarch64 archives
  unpacked and run here, 2690 checks from each on both engines, from
  a directory holding only the probe so the EMBEDDED stdlib
  answered). ASKED WHAT THE RELEASE ANSWERS: 300000 calls with a
  recursive helper plus 300000 returned closures peak at 1.6-3.1 MB
  across gnu/musl and both engines, where v2.136.0 held some 300;
  cycles still print `[[...]]` and still free on `xs[0] = nil`.
  Site names v2.137.0, the reference page carries Memory, nine paths
  200.
- 867: v2.137.0 TAGGED (158th tag, 85c9b58; strokes 862, 864, 865,
  866). Nothing owed at release time; the CHANGELOG's four entries
  are the leak closed, the escaping closure freed, the Memory
  section, and 862's drop repair (v2.136.0 shipped the slow drop).
  Gate green on the release build, `ting --version` 2.137.0.
  NOT YET VERIFIED.
- 866: third stroke — THE TREE-WALKER KEEPS WHAT THE VM LETS GO.
  `Env::release` decides per binding now: the functions bound in the
  frame whose env IS the frame are counted, the frame's count must be
  exactly those plus our own (anything else — a child scope, a
  closure made in one — bails), and if some escaped, each binding
  whose NAME no body among them mentions is removed. Nobody can call
  it by that name again and the binding was the other half of the
  cycle. `mentions` is blunt on purpose (any occurrence counts, a
  compiled body always answers yes): a wrong yes keeps memory, a
  wrong no breaks a program. On --eval, 300000 rounds: returned `fn
  add` 180 -> 2.6 MB, returned counter 154 -> 2.6; the RECURSIVE one
  stays at 180 and the reference now names recursion, not escaping,
  as the condition. Guard in tests/alloc.rs on both engines; no cost
  measured against the previous commit.
- 865: second stroke — WHAT IS STILL NOT RECLAIMED, IN THE
  REFERENCE. New Memory section under Values and types: freed when
  the last reference lets go, no collector and no pause; data that
  refers to itself is never reclaimed; breaking the link frees it; a
  `fn` that ESCAPES carries its scope and, because the scope holds
  its name, is that same cycle. Measured over 300000 rounds: self-
  referencing list 42 MB, self-holding map 111 MB, broken link 2.6
  MB, returned recursive closure 141 MB, ANONYMOUS escaping closure
  nothing on either engine. Guarded by tests/docs.rs (the section and
  the remedy line) and tests/alloc.rs (the cycle keeps memory, the
  broken one keeps almost none). No selftest: a ting program cannot
  see what it holds. Reference snippet count 6 -> 7 run-only.
- 864: first stroke — THE FRAME LETS GO. tests/alloc.rs subtracts
  frees now, so `live_bytes(f)` is what a run STILL HOLDS when it
  ends; the guard runs 1000 and 10000 calls on each engine and
  requires the second not to keep four times the first (before: 435
  KB against 4.29 MB, 428 bytes a call). `Env::release` is trial
  deletion by the counts: at the end of a call, block or loop
  iteration, functions bound in the frame that nothing else holds
  (`strong_count == 1`) and whose env IS the frame are not a reason
  to keep it, so if the counts add up exactly the bindings go.
  IT COST 45% OF fib.ting UNTIL THE SECOND MEASUREMENT: a compiled
  body capturing nothing runs in the DEFINING env — the global one —
  so releasing walked every binding in the program per call; a
  `fresh` flag releases only frames the call allocated (fib 363/359,
  lists 147/146, stdlib 159/160, interleaved against the previous
  commit). The 300000-call shapes now hold 2.6 MB instead of 145 and
  180. Seven selftest checks for what must survive (2683 -> 2690).
  Both new names had to change: `fn get` added a fifteenth corpus
  warning and `fn add` DELETED one by masking the arity check.
- 863: REPLENISHMENT — MILESTONE "WHAT A LONG-RUNNING PROGRAM KEEPS"
  (v2.137.0), reasoning in LOG.md. A tenth kind of looking: DURATION
  — every lens before it measured a run that ENDS. Numbers and time
  were checked first and are already closed (574's findings are in
  the reference; lib/time.ting is Hinnant). HEALTHY, measured: 4000
  file reads at 4 descriptors and 2.6 MB, 3000 `run()` children at
  3.2 MB with no zombies, the LSP flat at 3.2 MB over 2000 edits,
  `--check --watch` flat over 300 rewrites, an eight-million-map
  churn flat at its working set. NOT HEALTHY: A FUNCTION THAT
  DEFINES A RECURSIVE HELPER LEAKS ITS FRAME ON EVERY CALL, both
  engines — 145 MB (vm) and 180 MB (eval) for 300000 calls, linear
  in the calls, and DEFINING the helper is enough (it never has to
  be called). A helper that does not name itself does not leak, nor
  does top-level recursion. Cause: the name is Env-allocated because
  a nested function mentions it, so the Env holds the closure and
  the closure holds the Env — the same Rc cycle that leaks 35 MB per
  300000 self-referencing lists.
- 862: HEALTH TICK — MILESTONE "HOW DEEP THE MACHINERY GOES"
  (v2.136.0, strokes 854-861) COMPLETE, AND THE DROP REPAIRED.
  Eleven checksums match on both engines; the timings were weather
  but AN INTERLEAVED A/B AGAINST A v2.135.0 BUILD WAS NOT: 859's
  drop cost 20% on bench/json.ting. Three causes, all fixed here:
  `MapCell::drop` allocated a Vec per map; moving every element into
  a worklist costs more than looking at it in place (only nested
  containers are lifted out now); and the last 13% was TEARDOWN, not
  the program — recursion is the FAST path (it frees in allocation
  order), so dropping recurses for the first 100 levels by a
  thread-local counter and only then uses the worklist. Now at
  parity on all four container benches, deep cases still exit 0.
  A three-way build also priced 855's depth check at 4% of a 1.25 MB
  parse — kept. Sweeps green (50000 differential twice, 2000000
  patterns, crash, 20000 formatter). NEW RULE: BASELINE ANSWERS
  "DID THE CHECKSUM CHANGE", NOT "DID THIS COST ANYTHING" — build
  the last release in a worktree and interleave.
- 861: v2.136.0 VERIFIED (157th tag; strokes 855-859; seven assets,
  `sha256sum -c` OK on all six, both aarch64 archives unpacked and
  run here, 2683 checks from each on both engines, from a directory
  holding only the probe so the EMBEDDED stdlib answered). THE
  SHIPPED BINARIES WERE ASKED WHAT THE RELEASE ANSWERS: million-deep
  list and map print and EXIT 0, million-term chain runs/evaluates/
  checks, 2000 levels of `[` is `json_parse: nested deeper than 1000
  at offset 1000`, a 3000-deep list prints as 2005 characters and
  `json_str` refuses it. Site names v2.136.0, nine paths 200.
- 860: v2.136.0 TAGGED (157th tag, 087c490; strokes 855-859).
  CHANGELOG section written in the release commit as always; the
  release tick's first act (738) found nothing else owed — no count
  in README or the docs moved this milestone, and Limits already
  carried the two new numbers. Four entries for five strokes: 857
  and 858 are one thing from outside. Gate green on the release
  build, `ting --version` 2.136.0. NOT YET VERIFIED.
- 859: fifth stroke — THE DROP, THE LAST THING THAT RECURSED. Both
  types that nest have an iterative `Drop` now: `Expr` takes each
  child's kind out by `mem::replace` into a worklist, and
  `ListRef`/`MapRef` are `Rc<ListCell>`/`Rc<MapCell>` — newtypes
  `Deref`ing to the same `RefCell` — whose drop steals the children
  of every nested container it holds the LAST reference to
  (`Rc::into_inner`) before the cell goes out of scope. Sharing still
  decides, the refcount alone. `impl Drop for Value` is NOT possible:
  eval moves out of a `Value` everywhere (E0509 — the parser's
  assignment target hit it and takes the kind out instead). Measured:
  a million-deep list and a million-deep map print their answer and
  EXIT 0 on both engines (was 134, after the output), a million-term
  chain runs, evaluates and checks, three million too. Guarded end to
  end in tests/io.rs, on the binary, because the exit code AFTER the
  output is the point.
- 858: fourth stroke — FIVE MORE WALKERS, AND WHAT IS LEFT IS THE
  DROP. A throwaway test called each of the checker's nine passes on
  a 200000-term chain: `unbound_names`, `arity_mismatches` and
  `duplicate_map_keys` overflowed, six did not. Worklists now in
  `compile::walk_expr`, `eval::statement_offsets`, and the checker's
  `visit_exprs`, `walk_expr`, `collect_rebindings` and `check_calls`
  — children pushed in REVERSE where finding order is visible. Fn
  literals still recurse (statements are bounded by the parser;
  length is not). NOW: 500000 terms runs, checks and formats on both
  engines; a million prints its answer AND THEN aborts, so what is
  left is the AST's own drop — same shape as the value drop already
  on the backlog. Guarded in tests/lsp.rs, which asserts a name at
  the FAR END of the chain is still reported.
- 857: third stroke — THE LEFT SPINE, WALKED RATHER THAN RECURSED.
  Both engines collect `(op, rhs, span)` down the left side with an
  explicit stack; the compiler stops descending wherever a node could
  fuse, so the superinstructions still match. Tree-walker: died
  50000-100000 release and under 5000 unoptimized, now 500000 and
  100000. VM unoptimized: died 5000-10000, now 20000. NO BOUND HERE
  ON PURPOSE — a 300-term sum runs today, so any limit safe in an
  unoptimized build would break working programs, and 2.x does not.
  The point was an INVARIANT: at 100000 terms the VM printed and the
  tree-walker aborted. Costs nothing (interleaved A/B: 0.996x,
  1.001x, 1.000x). Still standing: 500000 terms aborts in the small
  name-collecting walkers, `compile::walk_expr` and `eval::expr`.
- 856: second stroke — PRINTING STOPS WHERE IT SAYS IT DOES.
  `value::MAX_PRINT_DEPTH` is 1000; past it `str()`/`print()` write
  the `[...]` / `{...}` marker a CYCLE already gets. That kills two
  faults with one bound: the walker died past ~150000 levels, and it
  was QUADRATIC before it died (51/408/1596/4492 ms at 10k/30k/60k/
  100k) because the cycle check scanned the whole path per container.
  Now 200000 deep is 57 ms and the answer is 2005 characters however
  much deeper it goes. `json_str` REFUSES instead of eliding, at the
  same 1000 as the reader: a reader is served by a marked cut, a
  program is not served by half a document. Selftest 2679 -> 2683.
- 855: first stroke — A JSON DOCUMENT TOO DEEP IS REFUSED.
  `json::MAX_DEPTH` is 1000 (arrays and objects share the count) and
  a deeper document is `json_parse: nested deeper than 1000 at
  offset N` instead of exit 134 from inside the builtin. A THOUSAND
  here against the parser's TWO HUNDRED on purpose: one is a promise
  about a language, the other a bound on data someone else wrote —
  100x what real documents carry, 100x under the cliff (220 bytes a
  level). Boundary checked at 1000/1001 in arrays, objects and the
  two interleaved. Selftest carries it too, so 2676 checks becomes
  2679; reference Limits states it; tests/docs.rs guards the number.
- 854: REPLENISHMENT — MILESTONE "HOW DEEP THE MACHINERY GOES"
  (v2.136.0), reasoning in LOG.md. A ninth kind of looking: the DATA
  the program handles, and its SHAPE rather than its size. HEALTHY,
  measured: each_line streams (2 MB peak over a 200 MB file),
  read_file is one copy, 5M-element list 78 MB, 1M-key map 209 MB,
  json_parse+json_str linear (196 ms on 4.2 MB, 563 on 13.8), and no
  backtracking bomb in the matcher (2 ms). NOT HEALTHY: Rust
  recursion over a user-controlled shape, unbounded everywhere —
  json_parse on a nested document, str()/printing, dropping,
  `a + b + c + ...` in compile and eval, and `==`. Fine at 100000,
  ABORTS at 200000 (exit 134, no line, nothing catchable); drop
  survives to 500000 and dies at a million. THE FIRST ROW IS INPUT,
  not program text: a script cannot defend itself, the abort is
  inside the builtin. MAX_NESTING bounds none of it.
- 853: HEALTH TICK GREEN — MILESTONE "THE PROGRAM THAT GOT BIG"
  (v2.135.0, strokes 843-853) COMPLETE. Bench: all eleven checksums
  match on both engines (timings 5-15% high at load 2.2, weather).
  Sweeps in release: 50000 differential, crash fuzzer at 50000,
  20000 formatter — all green. Site audit: the ten paths that exist
  answer 200 at www.baghino.me/thing/ and the published changelog
  names v2.135.0. I probed /thing/playground/ first, which 404s
  because THE PLAYGROUND IS THE ROOT PAGE — 782 recorded that and I
  read the note after rather than before. Next tick: replenishment.
- 852: THE PERFORMANCE GUARDS STOP MEASURING THE RUNNER. macOS
  failed the resolver guard at 3.8 on a LOG-only commit — the second
  flake in five ticks, same cause both times: five runs of the small
  size THEN five of the large, so a co-tenant arriving in the second
  block inflates only the numerator. `common::doubling_ratio` times
  the two sizes ALTERNATELY, best of five each, whole thing three
  times, keeping the SMALLEST ratio. Measured: all four guards 1.9-
  2.1 healthy (twice), the resolver mutation 3.8 and failing, both
  suites green with three CPU hogs running. The line at 3.0 now has
  room on both sides.
- 851: v2.135.0 VERIFIED (156th tag; strokes 844-848 plus 849's
  fix; seven assets, `sha256sum -c` OK on all six, both aarch64
  archives unpacked and run here, 2676 checks from each on both
  engines, and from a directory holding only selftest/ so the
  EMBEDDED stdlib answered). THE PROBE ASKED THE SHIPPED BINARIES
  WHAT THIS RELEASE EXISTS TO ANSWER: the 20000-deep program is
  `deep.ting:1:201: error: nested too deeply (the limit is 200
  levels)`, exit 1, INCLUDING under `ulimit -s 1024`; `--check` on
  8000 functions is 575 ms (gnu) / 498 (musl) against v2.134.0's 29
  seconds. MILESTONE "THE PROGRAM THAT GOT BIG" COMPLETE bar its
  health tick.
- 850: v2.135.0 TAGGED (156th tag, adaf446; strokes 844-848 plus
  849's fix). HEAD-TO-HEAD against a v2.134.0 worktree, best of
  three, interleaved: `--check` over 500-8000 functions 58/228/997/
  5798/29179 ms -> 36/56/139/270/599, **48.7x** on the largest and
  linear at last. VERIFICATION IS THE NEXT TICK'S: six archives,
  `sha256sum -c`, both aarch64 archives run here from a directory
  with no lib/. BASELINE not regenerated (812): the engines' speed
  is untouched, all eleven checksums matched before the tag.
- 849: CI RED AFTER 848, ON TWO PLATFORMS, BOTH WORTH HAVING.
  (a) windows-latest: `--check` on the deep program still died (exit
  0xC00000FD) — the limit was right, the STACK was wrong. Only the
  runner and the REPL spawned the 32 MB thread; every tool flag
  parsed on a main thread, which Windows promises one megabyte and
  an unoptimized parse at MAX_NESTING wants 3.5. `main` now declares
  the budget and spawns the thread ONCE for every command;
  `run_file`'s spawn is gone. Reproduced here first with `sh -c
  'ulimit -s 1024; exec ting --check ...'` — 134 before, 1 after —
  and that is now a #[cfg(unix)] test: A LINUX HOST CAN CHECK WHAT A
  SMALL MAIN STACK DOES. (b) ubuntu-latest: 845's pool guard scored
  3.5 with nothing regressed, comparing 4.9 ms against 17.2 — five
  milliseconds on a shared runner is a co-tenant. Sizes 3x, best of
  five (all four ratio guards), and the source is one statement per
  literal, because a `+` chain is a left spine the COMPILER walks by
  recursion and 12000 terms overflowed the test thread. Mutation
  rerun: 4.0, fails in 166 s — which is why 3x and not 10x, since a
  quadratic mutation costs the square.
- 848: fifth stroke — A PROGRAM TOO DEEP TO PARSE IS TOLD SO.
  843's abort (exit 134, no line, nothing catchable) is now
  `nested too deeply (the limit is 200 levels)` with a caret and exit
  1, from both engines and `--check`. `parser::MAX_NESTING` is a
  FIXED 200, not derived from the stack like the call-depth cap:
  every command must refuse the same programs. Probed per-level
  stack: parser 2176 B a block and 1088 B a bracket (17664/9392
  unoptimized), compiler 224, tree-walker 640 — 32 MB over those is
  15400 and 30800, exactly where 843 saw the cliff. `Parser::nested`
  wraps `statement` and `unary` (not `expr_bp`: a unary chain
  recurses through `unary`). Length is not depth — 50000 terms still
  parse. Guarded in docs.rs against the constant.
- 847: fourth stroke — THE RESOLVER STOPS SCANNING THE SCOPE.
  `resolve` walked the scope vectors per name and `note_scope` cloned
  every name in scope per fallible instruction. `FnCtx::at` maps a
  name to its binding stack (a hash lookup, popped on leave_scope) and
  `Chunk::in_scope` is now `(ip, Option<u32>)` into `scope_nodes`, a
  parent-linked chain — recording a scope is O(1) and `in_scope_at`
  walks the chain and reverses it, so callers still see outermost
  first. `--check` over 500-8000 functions with calls: 34/73/170/301/
  602 ms, against 807 before the stroke and 13120 at 843 — doubling
  doubles. Guard in tests/bytecode.rs (1500 vs 3000, best of three,
  ratio < 3); both mutations score 4.0 and fail it.
- 846: third stroke — A DIAGNOSTIC FINDS ITS LINE. `lexer::Lines`
  holds each line's start offset and answers by binary search;
  `diag::render_level_at` takes one and `check_warnings` builds a
  single index per file. `--check` over 1000-8000 unused functions:
  38/116/424/1566 -> 16/34/77/181 ms (8.6x on the largest, doubling
  now 2.2x); the 8000-name file 1563 -> 163 ms. THE SAME BUG WAS IN
  THE EDITOR IN ELEVEN PLACES — every `lsp::position` call is inside
  a loop (symbols, definitions, references, highlights, renames,
  links, diagnostics, code actions, formatting) and `folding_ranges`
  called `line_col` twice per brace pair; all counted from the top on
  every keystroke. Guard in tests/lsp.rs is a ratio (1500 vs 3000
  warnings, best of three, under 3.0) that ALSO asserts the last
  warning's line number, so a fast wrong answer fails; the old shape
  scores 3.4.
- 845: second stroke — THE COMPILER'S POOLS, AND TWO MORE QUADRATICS
  FOUND. `konst` and `name` carry HashMap indexes (a `ConstKey` of
  Int/Str/Float, the three kinds the pool ever deduped). 8000
  functions with one call: 207 -> 57 ms; with 8000 calls: 650 -> 553
  ms and `--check` 931 -> 717 ms.
  844'S ATTRIBUTION WAS WRONG AND THIS TICK CORRECTS IT: the rest of
  the curve is NOT the pools. `--check` on the 8000-name file is
  still 1563 ms against 1697. The guard written for this stroke
  FAILED at 3.9 and turned up two more:
  (a) RENDERING A DIAGNOSTIC SCANS THE SOURCE — `Span::line_col`
  counts from byte zero, so N warnings cost N times the file: 38,
  116, 424, 1566 ms over 1000-8000 unused functions;
  (b) THE RESOLVER AND `note_scope` SCAN THE SCOPE — `resolve` walks
  the scope vectors per name and `note_scope` CLONES every name in
  scope per fallible instruction, and the top level has a resolver of
  its own.
  The guard therefore measures what was fixed: distinct literals with
  few names, ratio under 3 at 1500 vs 3000, best of three; the scan
  scores 3.5.
- 844: first stroke — THE CHECKER STOPS RESCANNING. One
  `ident_index` groups every identifier token by name in a single
  pass; `unused_top_level_lets` counts from it and `unused_local_lets`
  uses `partition_point` into that name's positions. `--check` over
  500-8000 functions was 45/152/572/2737/13120 ms and is now
  29/66/148/338/931 ms — 14x on the largest, and doubling now costs
  2.4x rather than 4.8x. MEASURED, NOT GUESSED, WHERE THE REST IS:
  timing each warning pass accounts for 412 ms of that 931, and only
  107 of 1697 ms on the 8000-name file — the remainder is
  `check_source` calling `compile_program`, which the second stroke
  addresses. The guard in tests/lsp.rs is a RATIO (1500 vs 3000
  bindings, best of three, fails above 3.0) because a wall-clock
  number on a shared runner is weather; the old shape scores 3.8.
- 843: REPLENISHMENT — MILESTONE "THE PROGRAM THAT GOT BIG"
  (v2.135.0), reasoning in LOG.md. An eighth kind of looking: the
  SIZE of the program. The largest ting program in the repo is 438
  lines; generated ones of 500-8000 functions (up to 1.3 MB) say
  `--check` IS QUADRATIC — 45, 152, 572, 2737, 13120 ms as the input
  doubles — while the formatter (4-40 ms) and the tree-walker
  (6-80 ms) stay linear. Isolated: it is the NUMBER OF NAMES, not the
  file size (8000 names 1 call = 4175 ms; 1 name 8000 calls = 70 ms;
  8000 locals in one function = 964 ms), and the cause is in
  src/lsp.rs, where `unused_top_level_lets` counts matching
  identifier tokens in the WHOLE FILE per binding and
  `unused_local_lets` rescans the block per local. The compiler has
  the same shape under a comment that says "the pool stays tiny so a
  scan is fine" — `konst` and `name` in src/compile.rs — worth 650 ms
  against the tree-walker's 80 ms on the same file. And past 30000
  nested parens (or between 5000 and 20000 nested blocks) the process
  ABORTS with `has overflowed its stack`, exit 134, no line and
  nothing catchable — the same corpse class as 829's OOM.
  Not wrong, and checked: 20000 nested list literals, a 50000-term
  expression, and an error on line 8001 pointing at the right line.
- 842: health tick green at load 0.1 — MILESTONE "THE OTHER PROGRAM"
  (v2.134.0, strokes 836-841) COMPLETE. Sweeps: 50000 differential at
  seed 842, 20000 formatter, crash + 2000000 regex, all ok. Bench: 22
  comparisons, no mismatches. Site audit: nine paths 200, the
  changelog HEADINGS reading v2.134.0/v2.133.0/v2.132.0/v2.131.0,
  reference.html carrying `run(cmd, args, stdin)` and stdlib.html
  `was killed by signal 9` — both pages are this release's. Disk:
  `target` 3.0 G against the 2.1 G 834 left, `target/debug` 1.3 G of
  it, about 150 MB a tick; nothing done, because 834 priced the cure
  at 107 s and the tree is half the size that made it worth paying.
  Unchanged caveat: last-modified proves the wasm was BUILT from this
  release, not that it ANSWERS anything — and `run` could not answer
  in a page regardless, there being nothing to spawn there.
- 841: v2.134.0 TAGGED AND VERIFIED (155th tag; strokes 837, 838,
  839 plus 840's fix; seven assets, `sha256sum -c` OK on all six,
  both aarch64 Linux archives unpacked and run here, 2676 checks from
  each on both engines). MILESTONE "THE OTHER PROGRAM" COMPLETE bar
  its health tick.
  THE PROBE ASKED THE SHIPPED BINARIES THE THREE QUESTIONS THIS
  RELEASE EXISTS TO ANSWER, from a directory with no lib/ so the
  EMBEDDED stdlib answered, and gnu and musl agree: `kill -9` gives
  code nil / signal 9 and `sh was killed by signal 9`; `sort` sorts
  what it is given, `head -c 2` answers `ab` without an error, and
  2 MB through `cat` returns 2 MB with no deadlock; `read_file` on
  bad bytes says `not UTF-8 text` while the same bytes through `cat`
  come back replaced with code 0.
  BASELINE NOT REGENERATED, per 812: nothing here touches speed, and
  all eleven checksums matched on both engines before the tag.
- 840: RED CI SINCE 837, FOUND AND FIXED; the release waits for a
  green verdict. 837's `assert(contains(killmsg, "killed"))` is not
  portable — Windows has no signals, so `kill -9 $$` there is an
  ordinary nonzero exit and `ended` rightly says "exited". It now
  reads `killed["code"] != nil || contains(killmsg, "killed")`. The
  other four checks in that block were disjunctions and passed; this
  one asserted the Unix outcome flat. VERDICT READ THE SAME TICK: CI
  on 4ec3a80 green on all four platforms, and that run is the first
  to reach 838's stdin checks on Windows (the failing assert had
  stopped the file before them), so `cat`, `sort`, `head` and `read`
  under git-bash do answer. v2.134.0 can be tagged.
- 839: third stroke — WHAT THE BYTES ARE. Measured, the picture is a
  rule followed everywhere but one place and written down nowhere:
  `read_file`, `each_line`, `input()`, a script file, `--check` and
  `-` FAIL on bytes that are not text; `run`'s out/err replace them.
  THE RULE STANDS AND NOW SAYS WHY — a file is offered to ting as
  text so mojibake would be a wrong answer, while a child's output is
  whatever the child printed, there is no bytes type to hand back,
  and refusing would throw away the code, the stderr and the signal
  with it. 829's other loose end closed on the way past: `read_why`
  in diag.rs turns std's "stream did not contain valid UTF-8" into
  `not UTF-8 text` and EVERY "cannot read" now goes through it
  (read_file, each_line, input, the script loader, :load, --bundle),
  while non-encoding errors are untouched. Pinned both ways — a
  portable test for the reading side, a `#[cfg(unix)]` one using
  `cat` for the child side, since no ting program can print bytes
  that are not UTF-8. Two mutations, two caught.
- 838: second stroke — SOMETHING ON THE CHILD'S STDIN.
  `run(cmd, args, stdin)`, and `sh.ok`/`check`/`lines` too; without
  it the child gets EOF at once, as it always did and as the docs now
  say. THE DEADLOCK IS REAL AND WAS RUN BEFORE THE GUARD WAS WRITTEN:
  writing the input on the calling thread hangs at 2 MB each way
  (exit 124 under `timeout`, proved with a `sh -c` child and with a
  ting one), so `spawn_with_input` puts the write on its own thread
  and carries the reason in its doc comment. A child that stops
  reading early is not an error — the broken pipe is how `head` says
  it has enough. The test is BOUNDED (`try_wait` against a 120 s
  deadline, kill and fail) because a deadlock must fail a test rather
  than wedge the suite. Two things caught me: `--check` said `input`
  SHADOWS A BUILTIN three times, so the parameter is `stdin`
  everywhere; and the wasm lib build failed on
  `#[cfg(not(target_arch = "wasm32"))]` over a function whose call
  site is guarded by a runtime `cfg!`.
- 837: first stroke — A KILLED CHILD SAYS SO. `run` hands back a
  fourth key, `signal`: the number that killed the child where the
  platform has signals, nil elsewhere and after every normal exit,
  from a `signal_of` helper whose two cfg branches are one line each
  so the split does not straddle the map. `sh.check` reports through
  a new `ended(done)` — `exited 4`, `was killed by signal 9`, or
  `was killed` — instead of the old `exited nil`, and splitting it
  out makes the three shapes checkable without spawning anything.
  Portable selftest checks (the key is always there, a killed child
  never looks happy, code-or-signal, no message says "nil") plus a
  `#[cfg(unix)]` test in tests/io.rs for the specifics. Two
  mutations, two caught. Stdlib count 194 -> 195.
- 836: REPLENISHMENT — MILESTONE "THE OTHER PROGRAM" (v2.134.0),
  reasoning in LOG.md. A seventh kind of looking: RUN TING THE WAY A
  SHELL RUNS IT — piped, redirected, killed, driving other programs.
  It is a good citizen everywhere I pushed (a broken pipe ends the
  loop in 2 ms and exits 0 where the script alone takes 656 ms;
  output through a pipe is incremental; data on stdout and
  diagnostics on stderr with no ANSI escapes; argv, `-` for stdin,
  the piped REPL, the documented exit codes, and a full disk that
  says `print failed: No space left on device` with a line number).
  The boundary that is thin is where ting IS the shell: `run()`.
  Three strokes — a killed child reports `code: nil` and throws the
  signal away (`sh.check` says "exited nil", wrong twice); nothing
  can be sent to a child's stdin, so `echo data | sort` needs a temp
  file or the `sh -c` quoting that argv exists to avoid; and a
  child's bytes are decoded lossily where `read_file` on the same
  bytes is an error, neither documented. Measured and NOT wrong: 8 MB
  on stdout and stderr at once do not deadlock, and a missing program
  is an error rather than a nonzero code.
- 835: THE RAGGED ROW, DECIDED — first stroke banked. 829 left the
  choice open; the answer is DROPPED BY DEFAULT, DOCUMENTED, AND
  OPTIONAL TO KEEP. The invariant decides it: every entry from `maps`
  and `each_map` carries the same keys, which is why a column can be
  read without checking, and an unnamed extra field under an invented
  name would break that. Erroring is worse than it looks — `a,b,c,`
  under a three-name header is FOUR fields, the trailing separator a
  spreadsheet writes, and refusing that file refuses many real ones.
  So `entry_of`/`maps`/`each_map` take an `extras` key: name one and
  the fields past the header arrive there as a list, empty on the rows
  that have none so the keys still match; the key may not be a column
  name and must be a string. Three mutations, three caught. The
  reasoning lives in the module comment and in docs/stdlib.md's csv
  section, because 829's finding was as much that `maps` SAID NOTHING
  as that it drops.
- 834: health tick green at load 1.4 — MILESTONE "THE LOOP THAT DOES
  NOT BUILD A LIST" (v2.133.0, strokes 829-833) COMPLETE. Sweeps:
  50000 differential at seed 834, 20000 formatter, crash + 2000000
  regex, all ok. Bench: 22 comparisons, no mismatches, against the
  BASELINE regenerated at 832. Site audit: nine paths 200, the
  changelog page's HEADINGS reading v2.133.0/v2.132.0/v2.131.0 (as
  headings, per 828), reference.html carrying the range caveat.
  THE DISK NUMBER BECAME A DECISION: `target` had reached 3.9 G and
  all the growth was `target/debug` (2.4 G, one `cargo test` per
  tick), so I deleted it and priced the deletion — a cold `cargo
  test` rebuilds and runs all 16 suites in 107 s, and `target` came
  back at 2.1 G. The debug tree is a cache, not a resource to
  protect. THREE STALE COUNTS in the standing shape corrected against
  the tools that know them (corpus warnings THIRTEEN -> FOURTEEN,
  selftest checks 2635 -> 2649, Rust tests 382 -> 386); a fourth
  "correction" to 193 functions was wrong and the docs guard caught
  it, because lib/list.ting re-exports the builtin `sort_with` and
  counting `^fn ` is not counting exports.
  Unchanged caveat from 821 and 828: last-modified proves
  the wasm was BUILT from this release, not that it ANSWERS
  anything.
- 833: v2.133.0 TAGGED AND VERIFIED (154th tag; strokes 830, 831,
  832; six archives, `sha256sum -c` OK on all six, both aarch64 Linux
  archives unpacked and run here). MILESTONE "THE LOOP THAT DOES NOT
  BUILD A LIST" COMPLETE.
  THE PROBE ASKED THE SHIPPED BINARIES THE FOUR QUESTIONS THIS
  RELEASE EXISTS TO ANSWER, and gnu and musl both answered: the
  range(100000000000) loop prints 4; ten million iterations peak at
  2.1 MB (vm) and 2.5 MB (eval) from the gnu archive and 1.0 MB from
  musl — a different allocator, not a different answer; a `range`
  shadowed by `fn` still wins in the loop and as a value; and
  `range(2, 9, 3)` is still [2, 5, 8].
  Four changelog entries: the memory, the speed that came free, the
  compatibility argument written out, and BASELINE's timings
  regenerated with every checksum unchanged.
- 832: third stroke — WHAT IT IS WORTH, measured against c0bb1fe
  built in a worktree at load 0.33 (the 804 rule, not a BASELINE
  delta). PEAK MEMORY IS FLAT: before it grows 1.1 / 33.1 / 307.6 MB
  with n; after it is 2.1 MB at every size and STILL 2.1 MB at a
  hundred million on both engines. The range(100000000000) loop 829
  watched the kernel kill answers on both; the old binary is still
  killed at 137 under the same timeout.
  TIME, best of five interleaved at ten million: vm -25.8%, eval
  -3.2% — the VM's loop was spending a quarter of itself building a
  list it read once; the tree-walker's cost is interpretation.
  THE CORPUS BENCHES SAY NOTHING CHANGED and that is right: eleven
  scripts, best of three interleaved, all within +-6%, and the
  largest single number (fib +5.7%) is a script with NO range loop —
  a noise band you can name.
  BASELINE REGENERATED, 812's rule in the other direction: this
  milestone IS about speed and memory. Every checksum unchanged, only
  timings moved.
  Documented in both places: docs/vm.md's control-flow section (three
  slots, and why the decision is at run time) and the `range` row in
  docs/reference.md ("but only when `range` still means this
  builtin").
- 831: second stroke — THE TREE-WALKER COUNTS TOO. Head to head on
  ONE binary, fused against not fused (the list bound to a name so the
  loop cannot see the call): vm 0.74 s / 3 MB against 1.26 s / 308 MB;
  eval 6.48 s / 2 MB against 6.56 s / 307 MB. A hundredfold on both,
  no time lost on either — the tree-walker's cost is interpretation,
  not allocation, which is why MEMORY was the point.
  One enum (`ForItems`) carries both shapes so the loop body is
  written once; its snapshot arm now calls the same `iter_snapshot`
  the VM uses, deleting an inlined copy of "cannot iterate over X".
  Evaluation order preserved deliberately: callee then arguments, as
  ExprKind::Call does them.
  NINE DIFFERENTIAL CASES pin it across engines (three-argument,
  negative step, empty, `let range` inside a function, `fn range` at
  top level, all three error shapes, a spread). Three mutations,
  three caught: no runtime check, `>` where `>=` belongs in the
  counter's bound, and ignoring the spread guard.
- 830: first stroke — THE COUNTING LOOP, IN THE VM. 307 MB -> 3 MB
  and 1.19 s -> 0.85 s on `for i in range(10000000)`; the
  100000000000 case that 829 watched the kernel kill now ANSWERS.
  THE DECISION IS AT RUN TIME AND THAT IS THE DESIGN: `range` is a
  name a program may bind, and the REPL binds it in an earlier chunk
  than the loop, so no compile-time scan is sound. The compiler emits
  callee + args + IterStart; IterStart looks at what the callee turned
  out to be — the builtin gives counter/limit/step, anything else is
  CALLED and gives a snapshot. One loop body, one IterNext, two
  shapes.
  THE LOOP NOW OWNS THREE STACK SLOTS EITHER WAY, so both clean up
  with three pops; the test compares the two shapes against each
  other, not against a number, because the body pops too.
  ONE COPY OF THE RULES: `range_bounds` in eval.rs does arity, type
  and step-zero; the builtin builds its list from it and IterStart
  counts from it, so every error is identical at the same span.
  Three mutations, three caught: no runtime check (a shadowed range
  loses), the compiler's shape test dropped, and fusing through a
  spread.
  THE CORPUS WARNING GUARD SPOKE AGAIN: the selftest proving a
  shadowed `range` wins has to shadow one — 13 -> 14, red until the
  table was told. The unused parameter it also raised was answered
  with `_n`, this corpus's convention.
  The tree-walker still builds the list. Outputs agree, which is what
  the differential compares, so this is a MEMORY divergence, not a
  behavioural one — the next stroke.
- 829: replenishment — MILESTONE "THE LOOP THAT DOES NOT BUILD A
  LIST" (v2.133.0), the fifth kind of looking: THE INPUT, NOT THE
  PROGRAM. Fed the readers invalid UTF-8, a BOM, CRLF, NUL bytes, an
  empty file, a directory, JSON with a trailing comma / duplicate
  keys / 100000-deep nesting, CSV with an unterminated quote and
  ragged rows, and the arithmetic edges.
  MOST OF IT HELD, which is the first finding: BOM skipped in JSON
  and CSV, CRLF parsed, empty file gives empty, a directory says so,
  a trailing comma names its offset, duplicate keys take the last,
  100000 levels parse without touching the stack, INTEGER OVERFLOW IS
  A REAL ERROR AND NOT A WRAP, negative indices and clamping slices
  behave as documented.
  THE EVIDENCE: `for i in range(10000000)` peaks at 307 MB where the
  same loop with `while` peaks at 3 MB — a hundredfold. Time is a
  wash (1.19 s vs 1.35 s; the list actually WINS, because while
  interprets more per step). AND THE FAILURE MODE IS THE KERNEL:
  `for i in range(100000000000)` is OOM-killed, exit 137, no message,
  no line — the only bad input in the whole sweep that produces a
  corpse rather than a ting error.
  STROKES: (a) the VM recognises `for NAME in range(...)` and emits a
  counting loop — THE TRAP IS SHADOWING, `fn range(n)` and `let range
  = fn(n)` are both legal and both were checked here, and the fusion
  must not fire for them; (b) the tree-walker likewise, or the
  engines diverge; (c) measure as 803 did — peak memory flat in n,
  the OOM case answering, eleven checksums unchanged on both engines,
  no speed regression (the list is FASTER today, so the fusion must
  justify itself on memory).
  COMPATIBLE BY CONSTRUCTION: `range(...)` as a value still returns a
  list; nothing observable changes. Same shape as the v2.129.0 opcode
  fusions.
  HELD, NOT CHOSEN, WITH THE EVIDENCE: `csv.maps` on a ragged row
  SILENTLY DROPS extra fields (`3,4,5,6` under a three-name header
  gives three entries) and fills a short row with nil; `parse` keeps
  every field, and `maps` documents neither case. Python's DictReader
  keeps the extras. Its own tick, not a rider.
  ALSO NOTICED: read_file on invalid UTF-8 says "stream did not
  contain valid UTF-8" — Rust's phrasing, not ting's.
- 828: health tick green at load 2.4 — MILESTONE "FINDING THE
  FUNCTION YOU NEED" (v2.132.0, strokes 822-827) COMPLETE. 11 bench
  rows x 2 engines, 22 comparisons, none differ. 50000 differential
  at seed 828 (10.8 s), 20000 formatter (4.1 s), crash + 2000000
  regex (3.6 s). target 3.1 G (was 2.4 G at 821) and THE GROWTH IS IN
  debug: 1.7 G against 991 M, one `cargo test` per tick. Windows 612
  M, release 855 M, 88 G free — a number to watch, not to act on.
  SITE AUDIT: nine paths 200; the changelog HEADINGS run v2.132.0,
  v2.131.0, v2.130.0 — checked as headings because the v2.132.0 entry
  MENTIONS v2.130.0 twice and a text grep would have read the mention
  as the next release; reference.html carries the `ting --doc
  largest` example; ting.wasm last-modified is this release's push,
  which still says BUILT FROM and not ANSWERS (no wasm runtime on
  this host; 827 checked the native archives).
- 827: v2.132.0 TAGGED AND VERIFIED (153rd tag; strokes 823, 824,
  825, 826; six archives, `sha256sum -c` OK on all six, both aarch64
  Linux archives unpacked and run here). MILESTONE "FINDING THE
  FUNCTION YOU NEED" COMPLETE.
  THE PROBE ASKED THE SHIPPED BINARIES ABOUT THIS RELEASE and gnu and
  musl answered alike: `--doc largest` reaches map.top; `--doc sort`
  has an also-matching half with sort_with; `--doc width` answers
  format and `--doc format` carries `{:>5}`; `--doc len median slug`
  has NO also-matching, so the single-word rule survived the build;
  "duplicate" reaches unique and "frequent" reaches top; `:doc
  largest` is byte-identical to `--doc largest`; the shipped index is
  266 entries, 0 undescribed.
  Six changelog entries, the last saying what did NOT change:
  nothing about the language.
  NO BASELINE REGENERATION, per 812: a milestone about finding
  functions is not one about running them. 11 rows x 2 engines, 22
  comparisons, no mismatches.
- 826: fourth stroke — WORDS THE SEARCH CAN FIND. The 19 entries now
  say what they do; 0 of 266 undescribed. Both 825 misses are hits:
  "duplicate" -> `list.unique`, "frequent" -> `map.top`.
  A CORRECTION TO 825: those 19 were undescribed IN THE BINARY, not
  in the project — docs/stdlib.md has always carried a line for every
  one. `--doc` reads the `#` comment above the function and there was
  none. Two places describe the same function; only one was checked.
  THE WORDS ARE CHOSEN FOR A SEARCHER: `unique` says "DUPLICATES
  removed"; `top` says "the most FREQUENT entries". "most-common-
  first" was not enough — the rule matches a word STARTING WITH the
  query and "frequency" does not start with "frequent".
  GUARD: every_documented_entry_says_what_it_does parses --doc's own
  index and fails on a signature with nothing after it. Mutated both
  ways (comment deleted -> fails; `described` forced true -> passes
  with the defect present, which is what proves the check is
  load-bearing).
  824's `:doc frequency` case went stale FOR THE RIGHT REASON and
  moved to `medain`: a typo of a name is what stays one.
  MY OWN MISTAKE, RECORDED: `git checkout tests/io.rs` to undo a
  mutation threw away the tick's uncommitted test work too. Copy the
  file aside before mutating, as I do for src/; never reach for
  checkout on a file with work in it.
- 825: third stroke — WHAT THE SEARCH IS WORTH, COUNTED. Ten small
  tasks written naively BEFORE any --doc was run, each query a word
  from the TASK's vocabulary rather than the function's name.
  3 OF 10 ARE HITS THE OLD EXACT LOOKUP COULD NOT MAKE: count ->
  count_by/frequencies, split -> split_once (the builtin used to match
  first and stop), column -> string.table/args.pad/list.transpose
  (was "no such name"). 5 OF 10 WERE ALREADY EXACT NAMES (median,
  wrap, size, slug, percentile) and the search changes nothing there
  — said plainly, not counted as a win.
  2 OF 10 STILL MISS AND IT IS NOT THE SEARCH'S FAULT: "duplicate"
  does not reach `unique`, "frequent" does not reach `top`. `unique`
  HAS NO COMMENT AT ALL; `top` says "the largest values".
  SO "SEE ALSO" IS ANSWERED: NO. Cross-references would not have
  helped any of the ten. THE AUDIT IT PROMPTED FOUND THE NUMBER: of
  266 documented entries, 19 HAVE NO DESCRIPTION and are unreachable
  by search by construction.
  AND A DEFECT FROM TWO RELEASES AGO: `format`'s doc line still
  described v2.129's format, so v2.130.0's specs could not be found
  under "width", "align" or "decimal". Fixed here with a test on both
  halves; `--doc width` now answers with `format`.
- 824: second stroke — `:doc` IS `--doc`, CHARACTER FOR CHARACTER.
  The test asserts EQUALITY of the two outputs, so the spellings
  cannot drift. NO CAP ON THE SEARCH IN THE REPL, on purpose: `:doc
  list` has always printed a forty-line index, so 28 lines is nothing
  new, and a REPL answering differently from the flag is the worse
  trap.
  A MUTATION SURVIVED AND THE TEST WAS AT FAULT: the module chosen to
  prove the branch order was `math`, and nothing in the library says
  "math", so the search finds nothing and the order cannot matter.
  `list` discriminates — half the comments say "list". Swapped; both
  orderings now fail when mutated. Then the corrected assertion was
  wrong too: `!contains("matching")` fails on the real index because
  `find_index`'s comment says it. Assert the HEADING, "matching
  list:", not the word.
- 823: first stroke — A WORD THAT NAMES NOTHING IS SEARCHED FOR.
  `--doc largest` now answers max, list.max_by, list.extent,
  list.argmax and map.top — the function 822 caught me hand-rolling.
  TWO RULES: a NAME matches on any substring (`sort` finds
  `sort_with`); a COMMENT matches only where one of its WORDS STARTS
  WITH the query (`len` finds "length" and not "silently", `sort`
  still finds "sorted"). A function name is answered IN FULL and then
  followed by what else that word finds, but ONLY when one word was
  asked — several names is a lookup of names already known. A module
  or file keeps its index alone; searching "list" would bury
  lib/list.ting.
  Four mutations, four caught: substring comment matching, the skip
  dropped so the exact hit repeats under itself, the module branch
  removed, the single-word gate removed.
  The miss message is now "... MATCHES x", since a name is no longer
  the only thing tried; the did-you-mean is unchanged.
- 822: replenishment — MILESTONE "FINDING THE FUNCTION YOU NEED"
  (v2.132.0), the fourth kind of looking: WRITE THE SAME PROGRAM
  TWICE, once in ting and once in another language, and compare.
  THE EVIDENCE IS AGAINST ME: top five words in a file is 4 lines of
  python and was 8 of mine, and 4 of those 8 hand-rolled what
  `lib/map.ting`'s `top(m, n)` already does — a function I wrote,
  whose docstring says "the largest values, largest first".
  THE CAUSE IS ONE SITE: `--doc` is an EXACT NAME lookup. `--doc top`
  works, `--doc frequency` is a did-you-mean, `--doc sort` prints the
  builtin and not `sort_with`, `max_by` or `top`. 266 documented
  names, reachable only by knowing them; good docstrings nothing can
  search.
  STROKES: (a) `--doc` searches names AND descriptions when the
  argument names nothing, printing name + summary per hit, keeping
  the did-you-mean for an empty search; (b) the REPL's `:doc` gets
  the same; (c) MEASURE IT as 803 measured its fusions — fresh tasks
  written naively, counting how often the search finds what the naive
  version hand-rolled. "See also" cross-references HELD, not chosen:
  stroke (c) answers whether they are needed.
  THREE LENSES CAME UP THIN AND THAT IS RECORDED TOO: the corpus does
  not repeat itself (`let out = [];` 35 times is the only weight, and
  41 of those are not a loop); an `unused function` warning would be
  wrong 125 times over, because a top-level `fn` IS a module's
  export; and the surface audit is clean (all 193 stdlib functions in
  docs/stdlib.md, 10 unused outside lib/ and all of them internal
  helpers).
  THE DEAD-CODE SCAN WAS WRONG FIRST: a regex that blanked strings
  AFTER stripping comments swallowed whole files at the first `#`
  inside a string literal, and reported 128 dead functions including
  two that are called five lines later. Rewritten as a one-pass
  character scanner: 0. A measurement that flatters the milestone you
  are hoping for is the one to re-run.
- 821: health tick green at load 2.4 — MILESTONE "THE MISTAKE YOU
  ACTUALLY MADE" (v2.131.0, strokes 815-820) COMPLETE. 11 bench rows
  x 2 engines, 22 comparisons, none differ. 50000 differential at
  seed 820 (10.0 s), 20000 formatter (4.1 s), crash + 2000000 regex
  (3.2 s). target 2.4 G, up from 1.7 G at 814 AND THE GROWTH IS
  EXPLAINED: target/x86_64-pc-windows-msvc is 523 M of it, from this
  milestone's cross-checks. 88 G free, nothing to clean.
  SITE AUDIT: nine paths 200 (root, five doc pages, changelog,
  examples.js, ting.wasm), changelog top entry v2.131.0, examples.js
  carries `{:.2}`, ting.wasm last-modified is this release's push.
  WHAT IT DOES NOT PROVE, and 814's did not either: last-modified
  says the wasm was BUILT from this release, not that it ANSWERS the
  new hints. This host has no node, deno or wasmtime and cannot
  instantiate it; the diagnostics were checked on the native aarch64
  archives in 820. Say which of the two was checked.
  The repository has ISSUES DISABLED, so the maintenance check's
  issue list is permanently empty — not a finding.
- 820: v2.131.0 TAGGED AND VERIFIED (152nd tag; strokes 816, 817,
  818, 819; six archives, `sha256sum -c` OK on all six, both aarch64
  Linux archives unpacked and run here). MILESTONE "THE MISTAKE YOU
  ACTUALLY MADE" COMPLETE: the twenty foreign shapes 815 probed all
  name ting's spelling now. Five changelog entries, the fifth stating
  the compatibility argument rather than assuming it — every hint
  fires on a program that had ALREADY failed, and none of the words
  is reserved, so nothing that ran can behave differently.
  NO BASELINE REGENERATION, per 812's rule: this milestone changes
  what ting says when a program is WRONG, not how fast a correct one
  runs. The eleven rows were CHECKED instead of assumed — 11 rows x 2
  engines, 22 comparisons, no mismatches.
  The shipped gnu binary was asked about what THIS pair of releases
  shipped: `if x and 2 {` answers `&&`, `null` answers `nil`,
  `{:>8.2}` of a third writes `    0.33`. Release, CI and Pages all
  green, verdicts from the API.
- 819: fourth stroke — WHAT DIVIDING BY ZERO ANSWERS, written as one
  rule: which answer you get is decided by the OPERANDS, not by the
  zero. Ints error (`/` AND `%`); either side a float means IEEE
  (inf, -inf, NaN); mixing promotes first, so `1 / 0.0` is inf. Plus
  where those values go: str writes them, int() and json_str() refuse
  them, NaN != NaN so a NaN in a list unequals a copy of itself.
  CORRECTION TO 815: it said reference.md documented NEITHER half.
  The table already said `/ 0` on ints errors; what was missing was
  the floats and `%`.
  Ten new checks pin every sentence (2635 now). A docs paragraph that
  nothing tests is a claim.
- 818: third stroke — `'hi'` and a backtick say ting's strings use
  double quotes (lexer); `s.len()` says there are no methods, `m.a`
  says there are no fields, `f"..."` says to use format (parser).
  A `.` IS NEVER PART OF ANYTHING THE PARSER ACCEPTS — lexed, then
  only ever printed in an error — so the hint is riskless and can
  afford to say WHICH shape was meant, from whether an ident and a
  `(` follow. A `.` in a number never reaches that path.
  `1 + and` NEVER REACHES THE PARSER: it parses as the sum of a
  number and an unbound name, so and/or/not joined the same word
  table null/True live in. ONE TABLE now serves parser, runtime and
  --check.
  A MUTATION CAUGHT WHAT THE TESTS HAD NOT, again: dropping the
  f-string adjacency left all 45 parser tests passing, because
  `f "n is"` WITH A SPACE was missing from the cases and is the only
  thing that tells an f-string from a missing comma. Corpus warnings
  now THIRTEEN. 382 Rust tests, 2625 checks.
- 817: second stroke — and/or/not say `&&`/`||`/`!`;
  null/None/undefined/True/FALSE say `nil`/`true`/`false`. TWO SITES,
  because the two mistakes fail in different places: the operator
  words are unexpected TOKENS (parser `expect` and `block_stmt`,
  where `if not a {` stops ONE TOKEN PAST the word because `not` was
  read as the whole condition — the look-behind is what makes that
  case work), the literals are unbound NAMES (diag::spelt_here_as,
  read by BOTH the runtime and --check). The word table sits IN FRONT
  of `nearest` deliberately: `null` is two edits from `nil` and
  `None` is three, so no edit distance would find them, and these are
  known words rather than typos.
  None of the seven is reserved; `let not = true; if not { }` parses.
  THE CORPUS WARNING GUARD DID ITS JOB ON ME: the new selftest cases
  add four deliberate unbound names, so THE COUNT IS NOW TWELVE, NOT
  SEVEN, and the build was red until the guard's table said so.
  Three mutations, all caught. 379 Rust tests, 2624 checks.
- 816: first stroke — NINE MISTAKES THAT ALL SAID THE SAME WRONG
  THING now say nine different right ones. elif/elseif/elsif ->
  `else if`, def/function -> `fn name(...) { ... }`,
  var/const/local -> `let name = ...;`, `(x) => x` ->
  `fn(x) { return x; }`. THE HINT IS KEYED ON THE STATEMENT'S FIRST
  TOKEN, not the one the parser stopped at — the mistake is at the
  start of the line and the failure is at the end of it. One helper
  (expect_semi) replaces the six places a statement expected a `;`.
  None of the words is reserved: `let var = 1; print(var);` still
  runs, `elif;` is a valid statement, `else if` untouched. The arrow
  wants `=` and `>` ADJACENT.
  Both guards run backwards: dropping the adjacency rule fails the
  test, and so does keying the hint on self.peek(). 377 Rust tests.
- 815: replenishment — MILESTONE "THE MISTAKE YOU ACTUALLY MADE"
  (v2.131.0), chosen by a THIRD kind of looking: 799 counted
  instructions, 808 wrote a program and counted corrections, 815
  wrote fifty programs that are WRONG and read what ting says back.
  MOST ANSWERS ARE GOOD and that is the first finding — 25 of 30
  ordinary mistakes name the cause plainly, including a `did you
  mean` for a misspelled builtin and an import error listing both
  places it looked.
  THE SEAM IS FOREIGN SYNTAX: 20 of 20 such mistakes produce a
  message about ting's grammar rather than about the mistake, and
  NINE produce the SAME one, `expected ';', found identifier 'x'`
  (elif/elseif/elsif/def/function/var/const, an arrow function, and a
  missing semicolon). 811 already proved the fix and its cost: twelve
  lines at one site, firing only on an already-failed parse.
- 814: health tick + site audit green — MILESTONE "WHAT A NUMBER
  LOOKS LIKE" (v2.130.0, strokes 808-813) COMPLETE. Load 3.2, so read
  on checksums: 11 bench rows x 2 engines, 22 comparisons, none
  differ; BASELINE not regenerated at this release and did not need
  to be. 50000 differential at seed 814 (10.1 s), 20000 formatter
  (4.4 s), crash + 2000000 regex (3.4 s). target 1.7 G, debug 442 M.
  SITE AUDIT WENT FURTHER THAN USUAL because a release that changes
  what format prints has to reach the PLAYGROUND: ten paths 200,
  changelog top entry v2.130.0, reference carries the new "Format
  specs" section, examples.js contains the rewritten stats.ting with
  `{:.2}` in it, and ting.wasm's last-modified is the v2.130.0 push.
  The wasm the playground runs is this release.
- v2.130.0 VERIFIED (151st tag; strokes 809, 810, 811; seven assets,
  `sha256sum -c` OK on all six, both aarch64 archives executed here,
  2618 checks from each on both engines). The probe asked about what
  THIS release shipped — every spec form, 0.125 rounding to 0.13 with
  the binary's own round() agreeing, a big int keeping its last digit,
  the `//` hint — from outside the unpacked directory so the stdlib
  could only come from inside the binary. 806's guard held across a
  release: the archives' lib/ is still the same twelve modules.
- 812: v2.130.0 tagged. Five
  changelog entries; the fifth states the compatibility argument
  rather than assuming it (every spec was an ERROR before, so no
  program that ran can change meaning).
  NO BASELINE REGENERATION, ON PURPOSE: 804 regenerated it because
  that milestone was about speed; this one changes what text comes
  out of format and nothing about how fast anything runs. The eleven
  rows stand — and were CHECKED, not assumed: all eleven checksums on
  both engines, 22 comparisons, no mismatches. A release that does
  not touch performance should not rewrite the record of it,
  especially at load 3.4.
- 811: third stroke — `// a note` now says `expected expression,
  found '/' (a comment starts with `#`)`, in every position a comment
  is written and for `/* */` too. One site (the parser's only
  "expected expression"), and the hint fires only when the two tokens
  are ADJACENT, so a division that lost its operand keeps the plain
  message and no program that parses can be affected.
  THE GUARD WAS WRONG FIRST AND THE MUTATION SAID SO: removing the
  adjacency rule left all 39 parser tests passing, because in `a / /
  b` the parser is already past the second slash when it fails, so
  peek2 is `b` and the branch is never reached. `/ / x` — the same two
  tokens with a space — is what discriminates. A TEST THAT PASSES FOR
  THE WRONG REASON IS WORSE THAN NO TEST, and only running the change
  backwards finds one. 375 Rust tests.
- 810: second stroke — DECIMAL PLACES, `{:.2}` and `{:>8.2}`.
  examples/monthly.ting's hand-written `money(cents)` IS GONE and the
  example no longer imports lib/string.ting; output byte-identical.
  HALVES GO AWAY FROM ZERO ON PURPOSE, against Rust's own float
  formatting (which takes them to even, 0.125 -> 0.12): lib/math.ting's
  round() promises away from zero and a language should not disagree
  with itself about what a half is. An INT is written digit for digit,
  not through a float, so 9007199254740993 keeps its last digit.
  FINDING: rounding a number and writing one are different jobs, and
  examples/stats.ting had been doing the wrong one — `round(stddev *
  100) / 100.0` asked for two places and printed `17.3`, because a
  rounded float still prints as short as it can. Now 17.30; .out
  updated; docs say when to reach for which.
  2618 selftest checks (+16); two made to fail on purpose first.
- 809: first stroke — FORMAT TAKES A SPEC, `{:[[fill]align][width]}`.
  Three decisions, each made to agree with something that already
  existed: a width with no alignment puts NUMBERS RIGHT and
  everything else left (what a column of figures wants); a value
  already wider than the width is written unchanged (a spec pads,
  never truncates, as pad_left does); a centred value puts the odd
  character on the RIGHT, as lib/string.ting's center does. Width
  counts CHARACTERS, capped at 100000 so a typo errors rather than
  eating the machine.
  ADDITIVE WITH RECEIPTS: all 71 corpus programs run under 808's
  binary and this one on BOTH engines, 142 runs, and the only three
  that differ are the three this stroke edited — those re-run
  old-source-on-old-binary vs new-on-new, identical.
  Four format calls stopped padding their own arguments and
  examples/logreport.ting no longer imports lib/string.ting.
  2602 selftest checks (+19); two made to fail on purpose first.
- 808: replenishment — MILESTONE "WHAT A NUMBER LOOKS LIKE"
  (v2.130.0), CHOSEN BY WRITING A PROGRAM RATHER THAN READING A
  HISTOGRAM (799's seam is closed; counting instructions again would
  only find smaller ones). A 60-line log report over a 4000-line
  access log took four corrections: `//` is not a comment and the
  error says nothing about `#`; `pad_left(s, n)` needs a third
  argument (no default parameters); `sort_by` takes a key, not a
  comparator; and FORMAT HAS NO SPECS AT ALL — `format("{:.1f}%",
  pct)` is an error and a percentage prints as 33.333333333333336.
  COUNTED: 47 format calls in the corpus, four of which pad their own
  arguments before calling it (`format("  {} {}",
  st["pad_left"](str(row["n"]), 5, " "), row["name"])`); 14 hand-pad
  calls across five files; three hand-rounds; and monthly.ting, the
  money example, carries `fn money(cents)` that exists for no other
  reason. ADDITIVE FOR THE BEST REASON: every spec is an error today.
  pad_left and friends STAY — the point is that format should not
  need them.
- 807: health tick + site audit green — MILESTONE "THE COST OF A
  STEP" (v2.129.0, strokes 799-806) COMPLETE. Host was NOT quiet
  (load 2.74, three unrelated workloads), so every bench timing came
  in above BASELINE on both engines — the weather, and the reason the
  gate compares CHECKSUMS: all eleven rows on both engines, 22
  comparisons by script, none differ. 50000 differential at seed 807
  (10.0 s), 20000 formatter (4.3 s), crash + 2000000 regex (3.9 s);
  each runtime is the evidence the knob was read. target 1.7 G,
  target/debug 405 M — under 798's threshold, nothing removed. All
  ten site paths 200 and the changelog page's top entry is v2.129.0.
- 806: 754 CLOSED. A lib/ beside a script shadows the binary's
  embedded stdlib, and a release archive ships exactly such a lib/
  next to ting — so an unpacked release runs the ARCHIVE's copy.
  include_str! keeps each entry's TEXT in step for free and the
  archive is `cp -r lib dist/lib` from one checkout, but THE TABLE IS
  NOT GUARDED: a thirteenth module added to lib/ and forgotten in
  EMBEDDED_STDLIB ships in the archive and is missing from the
  binary, and `("lib/map.ting", include_str!("../lib/list.ting"))`
  compiles. Two guards in tests/selftest.rs — the table against the
  directory as sets plus each entry's text against its file, and a
  probe run from a directory WITH a copy of lib/ and one WITHOUT,
  whose outputs must be equal. Both failed on purpose first; the
  mispairing failed them independently, on text and on behaviour.
  373 Rust tests now.
- 804: v2.129.0 tagged.
  BASELINE regenerated in one go at load 0.15, all eleven checksums
  identical to v2.128.0's.
  AND THE BASELINE DELTA IS NOT THE MILESTONE'S GAIN — I nearly wrote
  it down as one. The eval column moved too (json 190.8 -> 154.3 ms)
  and eval saw none of these changes: the two BASELINEs were recorded
  on different days on a machine whose weather differs. TIMINGS ARE
  WEATHER, CHECKSUMS DECIDE, and that applies to BASELINE against
  BASELINE just as much as to a single run. The CHANGELOG's number
  instead comes from a binary built from 1bff6df (v2.128.0's code) in
  a worktree, run head to head against HEAD, interleaved, best of
  five, same quiet host: CSV -9.7%, scan -7.5%, fib -6.5%, growth
  -7.4%, toplevel -4.2%, tight loop -19.5%.
- Backlog (one per tick, in order; NEVER numbered — hand-numbering
  left a stale "(3)" twice, in 735 and 743, when the item above it
  was struck out):
  - a builtin that takes a key out of a map IN PLACE, byte-identical
  on both engines.
  - lib/map.ting built on it, and a selftest that draining a map is
  linear.
  - the docs say a map can be emptied as well as filled, and what
  `m[k] = nil` does instead.
  - release v2.143.0.
  DONE SINCE, MEASURED AGAIN AT 882: 787's two string cliffs are
  closed. `Value::Str` is `Rc<Repr>`, the text is shared rather than
  copied on a read, an append writes in place when it holds the only
  reference, and `Repr` carries a character count that an append
  KEEPS instead of throwing away. At 20000/40000/80000 appends every
  shape is linear: 3/6/10 ms at top level, 3/5/11 inside a function,
  6/11/20 with a closure mentioning the name, 5/10/18 with `len(s)`
  in the loop condition. Neither the assignment scan nor the ASCII
  fast path was needed in the end.
  NOT CHOSEN: streaming JSON (json_parse also takes the whole
  document, but a JSON document is a tree, not a sequence, so it
  means an event reader and a different programming model; the
  evidence is not in hand and CSV is where the big files are).
  read_bytes/write_bytes stays where 734 left it.
  NOT CHOSEN: a file handle value (open/read_line/close) is a new
  type and a resource that leaks when a script forgets it, and ting
  has no destructor or defer; lazy iterators (`for line in
  lines(path)`) read better than a callback but are a generator
  protocol in the language, not a builtin, and the streaming should
  be understood before its looks are chosen.
  NOT CHOSEN: read_bytes/write_bytes. It would solve copying too, but
  a list of ints for a 9 MB file is nine million values, and a real
  bytes type is a language addition, not a builtin. Also absent and
  not chosen: making a file executable (run + chmod covers it, and
  nothing has made me want it).
  SUPERSEDED: an earlier tick declined rename/copy for want of
  measured pain. 734 measured it — the photograph that cannot be
  moved, the date destroyed by read+write, the non-atomic
  write-then-remove — and the decision reversed on the evidence.
- Housekeeping, offered and unanswered: `target/` is 41 GB, disk at
  53%. A `cargo clean` was attempted between ticks and DID NOT take
  effect (target still 41 GB, nothing rebuilt). Costs one full
  rebuild; no urgency.
- 657's coverage path closed in 674.
- Not chosen in 666, with reasons: a --check warning suggesting `get`
  (ruled out by 649's principle — the nine warnings each claim "this
  is probably a bug"); an index-and-element loop form (zero pressure:
  all ten `for i in range(len(X))` loops use `i` for itself);
  nearest-key suggestions on a missing key (already implemented);
  auto-seeding `m[k] += 1` (a missing key is an error on purpose);
  `has` for lists and strings (the corpus tests bounds with `len`).
- Not chosen in 658, with reasons: an import-graph tool (`--deps`) is
  the obvious next toolchain noun and has no measured pressure behind
  it — the corpus's deepest import chain is two, and nothing in 657
  iterations was hard to find for want of it.
- Not chosen in 649, with reasons: string interpolation is the
  strongest pressure in the corpus (124 `+` concatenations against 21
  format() calls) and the one thing that cannot be added safely — a
  sigil inside an existing literal changes what it means; a new
  literal prefix buys safety with two spellings of a string forever.
  A --check warning suggesting `+=` was also declined: the nine
  warnings each claim "this is probably a bug", and a style
  preference would change what --strict's exit status means.
- Small strokes available any time: adopting `try(f, ...args)` at the
  53 corpus sites still written `try(fn() { return f(x); })` (the
  builtin takes the arguments already; 649's count of 79 predates it).
- Defaults are evaluated at each call in the callee's scope, left to
  right, so a later default may name an earlier parameter and
  fn f(xs = []) gets a fresh list every call.
- Not chosen in 612, with reasons: match expressions and catch syntax
  need a new keyword, and a new keyword breaks a program using that
  word as a name (the 2.x promise forbids it); a set is a map with
  true in it; threads are the wrong shape for an Rc interpreter.
- Still on the list, not chosen: match expressions, a set type,
  threads.
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
- Tags: 148 (v2.127.0), 148 verified; v2.29.0 is publicly marked broken
  (its Linux binaries needed glibc 2.39).

Standing rules (each from a slip; the LOG entry named has the story):

- Verdicts from the API (`gh run view --json conclusion`), never from
  a watcher's exit code, and every CI monitor pinned to the SHA or
  the run id: a filter on the workflow name alone matches the
  previous tag's completed run (626b). Tests that read paths out of tool output
  match file names, not separators: Windows prints backslashes
  (499b). A test over timings asserts what timings cannot swap:
  never the order of two rows that a loaded runner can reverse
  (533b). Every release cold-verified by downloading
  and executing an aarch64 archive on this host (musl and gnu).
- A test that runs the corpus in-process skips the files that touch
  the filesystem or spawn programs: another test already runs those as
  child processes, and selftest/fs.ting's tree has a fixed name (643b).
- Edit scripts belong inside the gate chain: a heredoc python that
  failed its assertion left STATE.md unwritten and the commit went out
  anyway (645b).
- Rehearsing a CI step means running the BYTES IN THE FILE:
  `ting tools/workflow_step.ting` takes a workflow path and a step
  name and prints that step's run block; pipe it into
  `bash -e -o pipefail`.
  Retyping a step tests a different step — 774's `cut -d` quoting
  reached four runners because the retyped version was the one I
  meant rather than the one written down.
- A tick's shell chain is ONE `&&` list (heredoc bodies follow the
  line); `set -e` is NOT honoured by the harness (377b); A COMMAND
  PIPED INTO `tail` REPORTS TAIL'S STATUS, so a gate step whose
  output is trimmed must run under `set -o pipefail` or write to a
  file — 875 sailed past a real clippy error that way; never a bare
  line after the gate (358, 377 pushed green records for red gates).
  Read the smoke output before writing prose that quotes it (370).
  Check a grep's result before promising a stroke on it (404).
- After writing LOG/STATE, rerun the docs guard and gate the push on
  the literal `test result: ok` (238). No angle-bracket placeholders
  in markdown (238, 262) — and none in a fenced block either, since
  767b: a guard that skips fenced blocks is one fence-detection bug
  away from skipping everything. When a guard is repaired, prove the
  repair by making it fail where the old one was blind.
- A test that looks for a path must build the needle with Path::join:
  a written `a/b` passes everywhere but Windows, which is the one
  runner that catches it, an hour later (675).
- Linux release builds stay on 22.04 runners; the glibc-floor step is
  the guard (v2.29.1). A failed Pages deploy is retried only with
  `gh workflow run pages.yml --ref main`.
- Bench on this shared host: checksums decide, timings are weather.
- DO NOT ASSERT FACTS ABOUT PLATFORMS THIS HOST CANNOT RUN. 778's red
  release came from a comment claiming `tar` on Windows is bsdtar; it
  is GNU tar under Git Bash. Where a claim about another platform
  cannot be checked here, use the mechanism that platform has already
  been OBSERVED to accept in a log, and say in the comment where the
  observation came from.
- Any harness that runs corpus programs redirects stdin from
  /dev/null. examples/pipeline.ting and the reference's input()
  snippet read it, and an inherited terminal makes them wait forever
  (773 found one thirty hours old). Rust's Command::output() does this
  for you; a shell loop does not.
- This host is shared and has four cores, and a tick saturates all of
  them for minutes. Every step runs under `nice -n 19` (plus
  `ionice -c 3` where it touches the disk), bench included: nice costs
  nothing on an idle host and yields the box on a busy one, where the
  loop is a background chore competing with someone's foreground work.
  It does not lower the load average — it lowers priority, which is
  the part that keeps the host answering. Measured in 686, memory is
  NOT the pressure and a volume cut would buy nothing: the tick peaks
  under 1 GB (differential fuzz at 50000 cases is the high-water mark
  at 916 MB; the whole debug suite is 115 MB, a -j4 build 385 MB,
  pattern fuzz at 2000000 cases 172 MB, formatter fuzz 30 MB). Both
  engines run at the same nice level in one bench invocation, so the
  eval-to-vm ratio still compares even when the absolute times drift.
- Corpus scan (`--check lib selftest examples bench`) expects exactly
  FOURTEEN warnings since 830 (was thirteen since 818, seven before
  817), guarded by a test since 499, all on purpose:
  collections.ting shadows `range` (830), because the selftest proving
  a shadowed `range` beats the fused loop has to shadow one, and
  edge.ting shadows `len` (451), repeats a map key and writes a
  statement after a return (507), errors.ting reads the unbound
  `totl` (495) and, since 680, `amonut` and `volme`, and
  functions.ting calls `add(1)` to prove arity (498), and errors.ting
  names `null`, `None`, `True`, `FALSE` and `and` plus a second
  `totl` since 817-818, to prove each is answered with ting's spelling. A file's
  warnings come in line order (507).
- Site audit paths: https://www.baghino.me/thing/ (github.io
  redirects there); playground at the root — /, /examples.js,
  /ting.wasm — plus reference, tutorial, cookbook, stdlib,
  retrospective, changelog .html (vm.md is not published).
- SITE AUDIT (corrected in 782, was wrong twice): the canonical host
  is https://www.baghino.me/thing/ — the ACCOUNT's user site took a
  custom domain, so stefanobaghino.github.io/thing/ now answers 301
  on every path. And the ten paths are not guessable: they are the
  three files in playground/ (index.html, examples.js, ting.wasm,
  plus / itself) and the six pages tools/md2html.ting renders in
  .github/workflows/pages.yml. `playground.html`, `ting.js` and
  `style.css` DO NOT EXIST and never did.
- Distribution audit expectation: 3 assets up to v2.16.0, 4 from
  v2.17.0, 6 from v2.30.0, SEVEN from v2.126.0 (SHA256SUMS joins
  them, and `sha256sum -c` on a fresh download is part of verifying
  a release from now on).
- Toolchain: rustc 1.98 locally; rustfmt and clippy reinstalled at 196.
- THE GATE COVERS THREE TARGETS SINCE 767, because two of them were
  only ever compiled by CI, an hour away: `cargo check`/`clippy
  --target x86_64-pc-windows-msvc --all-targets` (no linker needed,
  so the #[cfg(windows)] path typechecks and lints HERE) and
  `cargo build --release --lib --target wasm32-unknown-unknown` (what
  Pages builds). Both targets installed with `rustup target add`.
  Platform-specific code is written so that everything but the
  platform calls themselves sits under `#[cfg(any(windows, test))]`
  (or the equivalent for another platform) and is therefore testable
  on this host.
- The fuzzers live where their env vars are read: TING_DIFF_* in
  tests/differential.rs, TING_FMT_* in tests/fmt.rs, TING_RE_* in
  tests/fuzz.rs beside the crash fuzzer. Naming any other target
  passes in no time having fuzzed nothing (700). A sweep's runtime is
  the comparison a sweep offers — 2000000 pattern cases take about
  3.0 s against 0.22 s for the default count.
- Periodic health ticks (bench vs bench/BASELINE.md — recorded on this
  host, eight rows since 696 — plus 50000 differential, crash and 20000 formatter
  fuzz cases in release) close every milestone.
