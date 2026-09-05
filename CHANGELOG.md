# Changelog

All notable changes to ting. Versions are git tags; binaries for
Linux (x86-64 and arm64, glibc and fully static musl), macOS and
Windows are attached to each
[GitHub release](https://github.com/stefanobaghino/thing/releases).

## Unreleased

- The bytecode VM was slower than the tree-walking reference on
  top-level code — the shape most scripts are made of — because a
  function's locals resolve to frame slots at compile time and a
  script's did not. They do now. An empty 300k loop at the top of a
  file goes from 14% slower than the reference to 66% faster, and
  `bench/json.ting` from 15% slower to 5% faster. Every benchmark
  checksum is unchanged.

- The two engines said different things about a misspelled local. A
  function's locals live in frame slots under the bytecode VM, where
  they carry no name at runtime, so `amonut` next to a local `amount`
  got the bare "undefined variable" from the VM and the suggestion
  from the tree-walker. The compiler now records what was in scope at
  each site that can raise it, and both engines name the nearest one —
  and neither offers a name that is out of scope where the failure
  happened.

## v2.110.0 (2026-09-05)

- `--check` on a module member that does not exist now names the
  builtin of that name where there is one, instead of guessing at the
  nearest export. A module that retires a function into a builtin —
  as `lib/map.ting`'s `get` did in v2.109.0 — leaves its callers here.
- `--coverage` and `--profile` name an imported module the way the
  rest of their rows are named, relative to the directory the command
  ran in, instead of printing the absolute path it resolved to.

## v2.109.0 (2026-09-05)

- New builtin `get(x, k, default)`: `x[k]` where it is present,
  otherwise `default`. It reads a map by key and a list or string by
  index, negatives counting from the end, and never errors on an
  absence — so a tally is `m[k] = get(m, k, 0) + 1` rather than a
  branch around the first sighting. A key of the wrong type for the
  base is still an error, because a default answers an absence, not a
  bug.
- `lib/map.ting`'s `get` is gone: the builtin subsumes it, and a
  module function that shadows a builtin is one of the checker's
  warnings. Call `get(m, k, default)` directly, with no import.

## v2.108.0 (2026-09-05)

- A runtime error's `note:` lines now show what each call was given:
  `note: in scale(row = [3, "x"], factor = 2), called from ...`.
  Defaults and the rest list are included, because those are what the
  body saw. At most four arguments are named and each value is cut to
  32 characters, so a big list cannot bury the message. Both engines
  render the same text.
- Every frame in the `"trace"` `try` hands back carries an `"args"`
  map from parameter name to value — the values themselves, not the
  diagnostic's shortened rendering. `lib/err.ting` gains
  `given(f, ...rest)`, the arguments of the innermost failing call.
- The reference and the tutorial cover both, and say which caps belong
  to the diagnostic and which do not.

## v2.107.0 (2026-09-05)

- Compound assignment: `+=`, `-=`, `*=`, `/=` and `%=` on a variable
  or an indexed element. The target is named once, and for
  `m[k] op= v` the base and the subscript are evaluated once and used
  for both the read and the write.
- `try(f, ...args)` calls f with the arguments that follow it, so a
  call with arguments needs no lambda to carry them. Every function in
  `lib/err.ting` takes them the same way.
- A document highlight now calls the target of an assignment a write,
  compound or plain, rather than only the names a `let` or `fn`
  introduces.
- The reference and the tutorial cover both, including what a lambda
  around a `try` still buys you; the corpus uses them throughout, and
  the fuzzers generate them.

## v2.106.0 (2026-09-05)

- `--coverage` takes several paths (directories recurse, as every
  other tool flag here does): each script runs in its own interpreter,
  and they add up to one report.
- The reference and the tutorial document `--coverage`, including
  what counts as a coverable line.
- Tests for what coverage found untested: `lib/json.ting`'s `set_in`
  refusal, `lib/list.ting`'s `max_by` replacing its running best,
  `lib/args.ting`'s `main`, and — from processes of their own —
  `lib/test.ting`'s `summary` and `main`'s two exits.

## v2.105.0 (2026-09-05)

- New `ting --coverage SCRIPT`: runs the script, then reports on
  stderr — per file — the share of statements reached and the lines
  of those that were not. Imported modules are counted against their
  own files, and both engines report the same lines.

## v2.104.0 (2026-09-05)

- The tutorial and reference cover rest parameters and spreads, and
  selftest/varargs.ting exercises them on both engines. The
  differential generator emits variadic calls; the crash fuzzer's
  alphabet has `...`.
- `lib/test.ting` gains `pass()` and `fail_with(pattern, ...parts)`,
  which its five checks now go through and a check of your own can
  too.
- `lib/csv.ting`'s `parse` and `text` take an optional separator;
  `parse_with` and `text_with` remain as the older spelling.

## v2.103.0 (2026-09-05)

- A function's last parameter may be written `...rest`, and then it
  binds a list of every argument the fixed parameters did not take.
  Arity errors and `--check` warnings say "at least N arguments";
  the formatter keeps `...name` tight and hover shows it as written.
- `f(...xs)` spreads a list into a call, so what a rest parameter
  collects a spread can pass on. Spreading anything but a list is an
  error naming the type; `...` outside an argument list does not
  parse.

## v2.102.0 (2026-09-05)

- Function parameters may carry defaults (`fn f(a, b = 1)`), so a
  call can leave the tail off. Defaults are expressions evaluated at
  each call in the callee's scope, left to right, and arity errors
  name a range. Both engines, the checker, the formatter and the
  language server understand them.

## v2.101.0 (2026-09-05)

- New stdlib module `lib/args.ting`: command-line parsing from a
  spec, with the `--help` text built from the same spec.
- New stdlib module `lib/err.ting`: `message`, `failed`, `value`,
  `wrap`, `site` and `trace` over `try`.
- New stdlib module `lib/csv.ting`: delimited text both directions,
  quotes and embedded line breaks included.

## v2.100.0 (2026-09-05)

- New builtins `re_test(s, pattern)` and `re_find(s, pattern)`, over a
  new regular expression engine (`src/regex.rs`): a Pike VM, so
  matching is linear in the input and no pattern can be made to hang.
  Positions count characters, as `find` and `slice` do.
- New builtins `re_find_all(s, pattern)`, `re_replace(s, pattern,
  repl)` with `$1` group references, and `re_split(s, pattern)`.

## v2.99.0 (2026-09-05)

- New builtin `run(cmd)` / `run(cmd, args)` runs a program and waits,
  handing back a map of `code`, `out` and `err`. An argv list, never
  a shell string; a program that cannot be started is an error rather
  than an exit code. Refused on wasm, as `exit` and `sleep_ms` are.
- New builtins `eprint(...)`, which prints to stderr after flushing
  stdout so the two stay in order, and `cwd()`.
- New stdlib module `lib/sh.ting`: `ok`, `check` and `lines` over
  `run`, plus `which` and the PATH handling under it.

## v2.98.0 (2026-09-04)

- New builtins `random()`, `random_int(lo, hi)` and `seed(n)`:
  a float in `[0, 1)`, an int in a half-open span like `range`, and a
  restart point that makes a run repeat. Unseeded, the generator
  starts from the clock. See docs/reference.md.

## v2.97.0 (2026-09-04)

- New builtin `sleep_ms(ms)` pauses for that many milliseconds,
  flushing output first; a negative count or a non-int errors, and
  wasm refuses it as it does `exit` and `time_ms`.
- New stdlib module `lib/time.ting`: `iso`, `date`, `clock`, `parts`,
  `from_parts`, `span` and the civil-date arithmetic under them. UTC
  throughout, exact either side of the epoch.

## v2.96.0 (2026-09-04)

- New builtins `hex(n)` and `bin(n)` write the literal forms
  (`0xff`, `0b1010`), keeping the sign rather than wrapping:
  `hex(-255)` is `-0xff`.
- `int(s)` reads a string the way the lexer reads a literal — sign,
  `0x`/`0b` prefix, `_` between digits — so `int(hex(n))` is `n`.
- Docs: the reference and tutorial cover how numbers print and
  convert, and the Limits section says where infinity can come from.

## v2.95.0 (2026-09-04)

- Floats print in a form that reads back: an exponent outside the
  range 1e-4 to 1e17, the shortest round-tripping form inside it, and
  a `.0` on integral values. `1e23` printed as a 23-digit expansion
  before. `json_str` spells them the same way.
- Conversions refuse what a literal refuses: `float("1e400")`,
  `float("inf")`, `float("nan")` and `json_parse("1e999")` are errors
  rather than infinities.
- `int(x)` on a non-finite or out-of-range float is an error naming
  the value instead of saturating to `i64::MAX`.

## v2.94.0 (2026-09-04)

- Bitwise operators: `&`, `|`, `^`, `~`, `<<` and `>>`, on ints only.
  They bind tighter than every comparison, so `flags & MASK == MASK`
  applies the mask first — Rust's ordering, not C's.
- `>>` keeps the sign; a shift count outside 0 to 63 is an error, and
  a float operand is a type error rather than a promotion.
- Docs: the reference operator table and a tutorial section cover the
  bits and the literal forms; the Limits section no longer claims a
  fixed call depth of 200.

## v2.93.0 (2026-09-04)

- Integers can be written in hex (`0xff`) or binary (`0b1010`), and
  any run of digits can be broken up with `_` (`1_000_000`,
  `0xFF_FF`). A separator must sit between two digits.
- Floats take an exponent: `1e3`, `1.5e-3`, `2E+2`. An exponent
  always makes a float; a literal out of range for a double is an
  error rather than infinity.
- A literal that runs into a letter or a digit outside its radix is
  an error naming the offender (`0b12`, `12abc`), where it used to
  split into two tokens and fail elsewhere.

## v2.92.0 (2026-09-04)

- String literals take `\uXXXX`, four hex digits with a surrogate
  pair past U+FFFF — the spelling JSON uses, so a string copied out
  of a JSON document means the same thing either way.
- New builtins `ord(s)` and `chr(n)` convert between a
  one-character string and its code point.
- Docs: the reference and tutorial cover the recursion limit, the
  removal builtins and how to spell a character.

## v2.91.0 (2026-09-04)

- The call-depth cap is derived from the host stack the process
  declares rather than fixed at 200: the runner and the REPL hand
  their interpreter 32 MB and allow a few thousand frames from it
  (fewer unoptimized, where a frame costs several times as much).
  An embedder that declares nothing keeps the old conservative cap.
  The diagnostic names the cap it enforced.
- New builtins `remove_file(path)` and `remove_dir(path)`; the
  latter takes only an empty directory. `lib/fs.ting` gains
  `remove_tree`, the recursive version, written in ting.

## v2.90.0 (2026-09-04)

- New stdlib module `lib/fs.ting`, the seventh: `base`, `dir`,
  `ext`, `stem`, `parts`, `normal`, `join_path` and `with_ext` split
  and reassemble paths (on both `/` and `\`, joining with `/`);
  `entries`, `walk` and `walk_ext` list and recurse through a tree.
- Docs: the reference and tutorial cover the filesystem builtins and
  the module.

## v2.89.0 (2026-09-04)

- Four builtins let a script see the filesystem the toolchain
  already walks: `list_dir(path)` (the names in a directory,
  sorted), `exists(path)` and `is_dir(path)` (questions, so an
  absent path is `false`, never an error), and `make_dir(path)`
  (missing parents included; a directory already there is fine).

## v2.88.0 (2026-09-04)

- `ting -` runs a script read from standard input: arguments after
  the dash reach `args()`, diagnostics name the script `-`, and a
  relative `import` resolves against the working directory. The
  script is the stream, so `input()` sees end of file.
- Docs: the reference and tutorial cover watch mode and piped
  scripts.

## v2.87.0 (2026-09-04)

- New `--watch` for `--test`, `--check` and `--fmt-check`: the pass
  runs again whenever a watched file changes, is added or is
  deleted, with a rule line naming the run and its cause. The
  paths named on the command line are expanded before every poll,
  so new files join the next run. `--fmt --watch` is a usage error
  (it would answer its own rewrites); `--fmt --diff --watch` works.

## v2.86.0 (2026-09-04)

- `--profile` counts builtins too, marked `a builtin` where a ting
  function names its file and line, and prints at most twenty rows
  before counting the rest.
- Docs: the reference and tutorial explain the profiler, self time
  and the table.

## v2.85.0 (2026-09-04)

- New `ting --profile` runs a script and then reports, on stderr,
  how often each function ran, the time it spent in its own body
  (self time, so recursion is counted once), and where it was
  defined — slowest first.
- Fix: a closure created inside an imported module's function now
  belongs to that module's file, so it is reported under the file
  it was written in.

## v2.84.0 (2026-09-04)

- `try` hands a caught failure back whole: `"err"` is the message,
  `"at"` is the file, line and column it was raised at, and
  `"trace"` is the calls it came out of, each with the function's
  name (`nil` when it has none).
- `lib/test.ting`: a `check_err` whose error carries the wrong
  message now names the line that raised it.
- Docs: the tutorial and reference explain traces, how frames are
  named, and the ten-frame cap.

## v2.83.0 (2026-09-04)

- A runtime error shows the whole way back: one `note: in NAME,
  called from FILE:LINE:COL` per call it unwound through, innermost
  first. A trace longer than ten frames keeps four at each end and
  says how many it left out.
- Arity errors count in words and name the function called: "len
  expects 1 argument, got 0", "two expects 2 arguments, got 1".

## v2.82.0 (2026-09-03)

- Every `lib/test.ting` helper counts as a check, so files built on
  the framework report their totals under `ting --test` too.
- Docs: the tutorial and the stdlib page explain the counts.

## v2.81.0 (2026-09-03)

- `ting --test` says how much each file verified: `ok FILE (12
  checks)`, a total in the summary, `# 12 checks` in the TAP
  stream.
- A file that passes while checking nothing is named as such, in its
  own line and in the summary.

## v2.80.0 (2026-09-03)

- A file's `--check` warnings are printed in line order, whatever
  pass found them.
- Docs: the retrospective's tenth act; the tutorial and reference
  list every warning the checker gives.

## v2.79.0 (2026-09-03)

- `ting --check` and the LSP warn about a map literal that gives the
  same string key twice — the last one silently wins.
- They also warn about a statement that can never run, after a
  `return`, `break` or `continue` in the same block.

## v2.78.0 (2026-09-03)

- `ting --check` and the LSP warn when a call's argument count
  cannot match the function it names — for a function bound once at
  the top level and never rebound or shadowed.
- The corpus scan's warning set is guarded by a test, so a false
  positive from either static check fails the build.

## v2.77.0 (2026-09-03)

- `ting --check` and the LSP warn about a name that is bound nowhere
  — not a parameter, not a `let` in an enclosing block, not a
  builtin — and name the nearest one in scope.
- The LSP offers a quickfix that replaces such a name with the
  nearest one, beside the one for stdlib members.

## v2.76.0 (2026-09-03)

- An unknown option names the one you probably meant (`--fmr` finds
  `--fmt`).
- Suggestions count a swap of neighbours as one slip (`--lps` finds
  `--lsp`) and stay silent for names under three characters.
- The tutorial explains the suggestions; selftest/errors.ting checks
  them from inside ting.

## v2.75.0 (2026-09-03)

- "Did you mean?": an undefined variable (or an assignment to one)
  names the nearest binding, parameter or builtin in scope.
- A key a map does not hold names the nearest key it does, and the
  `--check`/LSP warning for an unknown stdlib member names the
  nearest member.
- `ting --doc` and the REPL's `:doc` suggest the nearest documented
  name for a name they do not know.

## v2.74.0 (2026-09-03)

- `--doc` and `:doc` wrap at 78 columns: a comment under its
  signature, an index line's first sentence beside the name when it
  fits and underneath when it does not.
- `ting --doc len median slug`: several names at once, printed in
  order and separated by a blank line; an unknown name exits 1 and
  the rest are still printed.
- A new example, `examples/inventory.ting` (stock list: `key_of`,
  `take_while`/`drop_while`, `flatten`, `plural`).

## v2.73.0 (2026-09-03)

- Exit codes: 0 success, 1 a reported failure, 2 a usage error
  (missing operand, bad option value, unknown option); `--help`
  and the reference's Running section say so.
- lib/list.ting: `take_while(xs, pred)` and `drop_while(xs, pred)`.

## v2.72.0 (2026-09-03)

- An unknown option is a usage error (exit 2) that names it and
  points at `--help`; `-h` and `-V` work as short forms.
- lib/string.ting: `plural(n, one, many)`.
- Docs: the formatter's every-file run and summary line.

## v2.71.0 (2026-09-03)

- `--fmt`, `--fmt-check`, `--fmt --diff` and `--check` process every
  file before failing: a file that cannot be read, does not lex or
  cannot be written is reported and the run continues, with exit 1
  at the end.
- A multi-file `--fmt` run ends with a summary line.
- lib/map.ting: `key_of(m, v)`.

## v2.70.0 (2026-09-03)

- `ting --check --strict`: warnings fail the check.
- The stdlib page is guarded against missing rows and a stale
  function count (116 functions).

## v2.69.0 (2026-09-03)

- `--check` and the LSP warn when a `let`, `fn` or parameter shadows
  a builtin.
- lib/string.ting: `is_number(s)`; lib/list.ting: `argmax(xs)` and
  `argmin(xs)`.

## v2.68.0 (2026-09-03)

- LSP: the whole-document formatting edit ends at the document's
  real last position.
- Tutorial: diff and flatten in the JSON chapter; retrospective:
  ninth act.

## v2.67.0 (2026-09-03)

- lib/test.ting: `check_type(name, v, type_name)`; check, check_eq
  and summary documented.
- lib/json.ting: `flatten(v)`, a map from dotted leaf paths to
  values.
- lib/math.ting: `hypot(a, b)`.

## v2.66.0 (2026-09-03)

- The formatter keeps the file's line endings: a CRLF file stays
  CRLF, and `--fmt-check` no longer flags a Windows checkout.
- `--check` and the LSP warn about a `let` inside a block that
  nothing in the block uses (underscore-prefixed names exempt).

## v2.65.0 (2026-09-03)

- REPL: `:load` reports how many bindings it added.
- A failed `import` names the path it resolved to and says no
  embedded module matched.
- lib/list.ting: `transpose(xss)`.

## v2.64.0 (2026-09-03)

- REPL: `:load FILE` resolves the file's relative imports against
  its own directory and names the file in diagnostics.
- lib/string.ting: `squeeze(s)`.
- Docs count the thirteen editor capabilities.

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
