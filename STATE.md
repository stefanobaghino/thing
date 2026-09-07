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
- 73 builtins; twelve embedded stdlib modules
  (list/map/string/math/json/fs/test/time/sh/args/err/csv, 194
  functions, guarded); 44 ting programs (22 selftest files — 21 tests
  plus _lib.ting, the module modules.ting imports, which checks
  nothing on its own — and 22 examples with .out); 362 Rust tests
  in 15 suites.
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

1. Maintenance check every tick: issues, PRs, CI, tree.
2. One small verifiable stroke per tick (feature, docs, test, health
   check); before every push run what CI runs, in CI's own words —
   `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test` — plus `ting --fmt .` and the corpus check. NO
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
- Backlog (one per tick, in order; NEVER numbered — hand-numbering
  left a stale "(3)" twice, in 735 and 743, when the item above it
  was struck out):
  - replenishment: choose the next milestone (two releases' worth) and
  write the reasoning in LOG.md.
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
- Tags: 145 (v2.124.0), 145 verified; v2.29.0 is publicly marked broken
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
- A tick's shell chain is ONE `&&` list (heredoc bodies follow the
  line); `set -e` is NOT honoured by the harness (377b); never a bare
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
  seven warnings, guarded by a test since 499, all on purpose:
  edge.ting shadows `len` (451), repeats a map key and writes a
  statement after a return (507), errors.ting reads the unbound
  `totl` (495) and, since 680, `amonut` and `volme`, and
  functions.ting calls `add(1)` to prove arity (498). A file's
  warnings come in line order (507).
- Site audit paths: https://www.baghino.me/thing/ (github.io
  redirects there); playground at the root — /, /examples.js,
  /ting.wasm — plus reference, tutorial, cookbook, stdlib,
  retrospective, changelog .html (vm.md is not published).
- Distribution audit expectation: 3 assets up to v2.16.0, 4 from
  v2.17.0, 6 from v2.30.0.
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
