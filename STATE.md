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
- 66 builtins; twelve embedded stdlib modules
  (list/map/string/math/json/fs/test/time/sh/args/err/csv, 174
  functions, guarded); 39 ting programs (21 selftest files, 18 examples with .out); 316 Rust tests
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
- Backlog (one per tick, in order):
  (1) compound assignment `+= -= *= /= %=` in lexer, parser, AST and
  both engines, evaluating an indexed target's subscript once;
  (2) formatter, checker and LSP follow, plus a selftest;
  (3) `try(f, ...args)`; (4) fuzzers learn the tokens and the corpus
  adopts both; (5) docs; (6) RELEASE v2.107.0; (7) verify;
  (8) health tick.
- Not chosen in 649, with reasons: string interpolation is the
  strongest pressure in the corpus (124 `+` concatenations against 21
  format() calls) and the one thing that cannot be added safely — a
  sigil inside an existing literal changes what it means; a new
  literal prefix buys safety with two spellings of a string forever.
  A --check warning suggesting `+=` was also declined: the nine
  warnings each claim "this is probably a bug", and a style
  preference would change what --strict's exit status means.
- Small strokes available any time: `try(f, ...args)` calling f with
  those arguments (79 corpus wrappers are `try(fn() { return ...; })`,
  29 of them a single call).
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
- Tags: 126 (v2.105.0), 126 verified; v2.29.0 is publicly marked broken
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
