# Decision log

Append-only. Newest entries at the bottom. Every entry: timestamp (UTC),
what was decided/done, and why.

---

## 2026-09-01 — Iteration 0: bootstrap

**Decision: loop mechanism.** The loop runs as a self-paced agentic loop
inside a Claude Code session: each iteration follows the protocol in
`LOOP.md` (orient → pick → execute → verify → commit → log → update state →
schedule next wakeup). Self-pacing was chosen over a fixed interval because
task sizes vary; the delay is picked per-iteration. Durable state lives in
the repo (`STATE.md`, `LOG.md`), not in conversation memory, so the loop
survives context summarization and session restarts.

**Decision: what to build.** A tiny scripting language named **ting**
(the repo is "thing"; the language is a thing minus the h), implemented in
Rust as a zero-dependency tree-walking interpreter with a REPL.
Rationale:
- Satisfies BOOTSTRAP.md rules: runnable by anyone on their own device
  (single binary or `cargo build`), no service, no restrictive platform,
  MIT license, only free tooling.
- An interpreter decomposes into many small, independently testable
  increments (lexer → parser → evaluator → stdlib → REPL → docs), which is
  the ideal workload shape for an iterative loop.
- Considered alternatives: a TUI git visualizer (fewer clean increments,
  needs deps), a static site generator (crowded space, less interesting),
  a generative-art CLI (fun but shallow backlog). The language won on
  backlog depth and verifiability.

**Decision: implementation language.** Rust (toolchain already installed:
rustc 1.91.1). Go and Zig are absent from the machine; Python/Node would
drag a runtime dependency for users, violating the spirit of "runnable on
their own device" with minimal friction. Zero third-party crates keeps the
supply chain empty and builds trivial.

**Decision: quality bar.** Every commit builds and passes tests standalone
(also enforced by the user's global commit-hygiene rules). Failing trees
are never committed.

---

## 2026-09-01 — Iteration 1: crate bootstrap

Initialized the `ting` crate (`cargo init --name ting`, edition 2024, no
dependencies), added the MIT `LICENSE`, a `README.md` stating the project's
nature and pointing at the experiment files, and `.gitignore` for `/target`.
Crate name is `ting` while the repo stays `thing` — the binary is the
language, the repo is the experiment. `cargo build` and `cargo test` pass
on the default hello-world `main.rs`; real code starts next iteration with
the lexer. Next wakeup scheduled short since the next task is fully
specified.

---

## 2026-09-01 — Iteration 2: lexer

Implemented `src/lexer.rs`: full token set (int/float/string/ident,
9 keywords, all operators/punctuation), byte-offset `Span` on every token,
`Span::line_col` for diagnostics, `#`-to-EOL comments. 15 unit tests cover
literals, escapes, unicode, keyword/identifier boundaries, two-char
operators, spans, and error cases.

Decisions:
- **Byte-level scanning** over `char_indices`: simpler peek/peek2 logic;
  multi-byte UTF-8 is handled explicitly in strings and error re-sync.
- **Separate Int(i64)/Float(f64)** rather than a single f64 number type —
  integer semantics (exact arithmetic, indexing) are worth the small cost.
- **`1.foo` lexes as Int, Dot, Ident** (float requires digit after the
  dot), keeping the door open for method-call syntax.
- **Newlines terminate strings with an error** — multiline strings can be
  added later deliberately, not by accident.
- Clippy denied `3.14` in a test (approx-PI lint); used `2.5` instead.
  Clippy added to the verify step alongside `cargo test`.

`main.rs` now runs a script file by dumping its token stream (placeholder
until the parser lands). Smoke-tested on a sample script.

---

## 2026-09-01 — Iteration 3: expression parser

Implemented `src/ast.rs` (spanned `Expr` tree, `Display` renders
s-expressions for tests/debugging) and `src/parser.rs` (Pratt parser).
Covers literals, variables, unary `-`/`!`, all binary operators with
correct precedence (`||` < `&&` < equality < comparison < additive <
multiplicative < unary < postfix), grouping, calls, indexing, and list
literals with optional trailing comma. 14 parser tests; suite is now 29.

Decisions:
- **Binding-power Pratt loop** over recursive-descent per level: one
  `expr_bp` function plus a `binop` table keeps precedence in one place.
- **Postfix (call/index) parsed in a loop above `primary`**, so chains
  like `f(1)(2)` and `m[k][0]` fall out naturally; call binds tighter
  than unary minus (`-f(1)` = `(- (call f 1))`).
- **`parse_expr` demands Eof** — trailing tokens are an error now;
  statement parsing (next task, with `let`/blocks) will own the
  program-level loop.
- **Human-readable token names in errors** via `describe()`:
  "expected ')', found end of input" instead of enum debug dumps.
- `main.rs` temporarily parses a file as one expression and prints the
  s-expression (replaces the token dump).

Session note: the human restarted the session with permission bypass so
the loop runs without approval pauses; the loop was re-entered via /loop
and resumed from repo state exactly as designed.

---

## 2026-09-01 — Iteration 4: expression evaluator

Implemented `src/value.rs` (`Value`: int/float/string/bool/nil/list, with
display formatting) and `src/eval.rs` (tree-walking `eval` over `Expr`).
14 new tests; suite is now 43. `ting <file>` evaluates one expression and
prints the result.

Semantics decisions (these define the language, so logged in detail):
- **Ints and floats stay distinct**; mixed arithmetic promotes to float.
  Integer overflow is a runtime error (checked ops), not a wrap.
- **Integer division truncates**; `/ 0` and `% 0` on ints are runtime
  errors, but float division by zero yields IEEE infinity/NaN.
- **`+` is overloaded** for string concatenation and list concatenation;
  no implicit stringification (`1 + "x"` is a type error).
- **Strict booleans**: `&&`, `||`, `!` require bools — no truthiness.
  `&&`/`||` short-circuit (verified by a test where the rhs would trap).
- **Equality is structural**, with numeric cross-type equality
  (`1 == 1.0` is true); comparisons work on numbers and strings only.
  NaN comparisons are false per IEEE.
- **Negative indices** count from the end (Python-style) on lists and
  strings; string indexing is by character, not byte.
- `Var`/`Call` evaluate to descriptive errors until `let` and functions
  land (next tasks).

Session note: the human added the `origin` remote
(github.com:stefanobaghino/thing); publishing/pushing joins the backlog
near CI/release time.

---

## 2026-09-01 — Iteration 5: statements

Added `Stmt` to the AST, a statement parser (`parse_program`), and rebuilt
the evaluator as `Interpreter` with a scope stack. 11 new tests; suite is
now 54. `ting <file>` now runs a whole program.

Semantics decisions:
- **`let` defines/shadows in the current scope; assignment rebinds the
  nearest existing binding** and errors on undefined names — no implicit
  globals. Block locals don't leak.
- **Semicolons are mandatory** after let/assignment/expression statements;
  blocks need none. Keeps the grammar unambiguous ahead of the REPL.
- **Assignment is a statement, not an expression** (`1 = 2` and
  `f() = 2` are parse errors; no chained `a = b = c`).
- **`print` is a special-cased builtin call** for now: multiple args
  joined by spaces plus newline, returns nil; shadowing `print` with a
  variable disables it. Real builtin infrastructure arrives with
  functions.
- **Interpreter is generic over its output writer** — tests capture print
  output in a `Vec<u8>`, main hands it a locked stdout.
- `parse_expr` is `#[cfg(test)]`-gated until the REPL needs it (keeps
  clippy's dead-code lint clean).

Session note: the human pushed main to origin with tracking; `git push`
is now enough when publishing lands.

---

## 2026-09-01 — Iteration 6: control flow

Added `if`/`else` (with `else if` chaining) and `while` to AST, parser,
and interpreter. 7 new tests; suite is now 61. First real program works:
iterative fib(10) = 55.

Decisions:
- **Braces are mandatory, parentheses around conditions are not**
  (Rust/Go style: `if x > 3 { ... }`). No dangling-else ambiguity, and
  error messages say what the brace was expected after.
- **`else` takes either a block or another `if`** — chaining is
  recursion in the parser, not a dedicated elif node.
- **Conditions are strictly bool** (consistent with iteration 4's
  no-truthiness decision).
- No `break`/`continue` yet — deferred until functions/`return`
  introduce non-local exits (one mechanism for all three).

From here each green iteration also pushes to origin, since the human
set up the remote and tracking.

---

## 2026-09-01 — Iteration 7: functions and closures

The biggest iteration so far. Added `fn` declarations, anonymous `fn`
expressions, `return`, and full lexical closures. 16 new tests; suite is
now 77 (recursion, mutable capture, independent closure instances,
higher-order functions, arity errors, runaway-recursion trapping).

Design decisions:
- **Environments became `Rc<RefCell<Env>>` chains** (was: a scope-stack
  `Vec<HashMap>`). Required for closures: a returned closure keeps its
  defining environment alive, and assignments through it are visible to
  every closure sharing it (verified: two counters advance independently,
  one counter's three calls print 1 2 3).
- **`fn name(...) {...}` desugars in the parser to `let name = fn...`**;
  recursion still works because the closure captures the environment the
  binding lands in. One function representation everywhere.
- **`return` propagates as a `Control` enum** (Normal/Return) through
  blocks and loops — not an Err hack; `return` at top level is an error.
  Falling off a function's end yields nil. Exact arity is enforced.
- **Functions compare by identity** (`Rc::ptr_eq`), display as
  `<fn(a, b)>`.
- **Call depth capped at 200**: the first depth-500 attempt overflowed
  the 2MB test-thread stack in debug builds (caught by the runaway
  recursion test aborting the whole test run). Fix: cap 200 + `main`
  runs the interpreter on a dedicated 32MB-stack thread (the AST's `Rc`
  isn't `Send`, so the whole lex/parse/run pipeline moved onto that
  thread).
- `break`/`continue` still deferred; `Control` now gives them an obvious
  home next time loops come up.

---

## 2026-09-01 — Iteration 8: maps and index assignment

Added map literals `{"k": v}`, map indexing, index assignment
(`xs[i] = v`, `m["k"] = v`, nested `grid[1][0] = 5`), and switched lists
and maps to reference semantics. 10 new tests; suite is now 87.

Design decisions (the big one first):
- **Lists and maps are now reference types** —
  `Rc<RefCell<...>>`, like Python/JS/Lua. Motivation: index assignment
  through nested structures and (next iteration) in-place builtins like
  `push` fall out for free; `eval(base)` hands back a shared handle, so
  `m["a"]["b"] = 2` needs no place-resolution machinery. Equality stays
  structural (deep), with an `Rc::ptr_eq` fast path. `+` on lists still
  builds a fresh list (tested: mutating the concat result leaves the
  operand alone). Aliasing is now observable and tested (`let b = a;
  b[0] = 2` changes `a`).
- **Map keys are strings only**, stored in a `BTreeMap` so display order
  is sorted and deterministic. Missing-key reads are runtime errors
  (consistent with the language's strictness); `has`/`keys` builtins
  next iteration make that livable.
- **Map literals don't parse at statement start** (`{` opens a block
  there) — noted in a parser comment; harmless in practice since a bare
  map statement is useless.
- **`:` added to the lexer** (maps needed it; it was the only new token).
- The runaway-recursion test overflowed the test-thread stack again
  (bigger `Value` enum → bigger frames). Fix: `.cargo/config.toml` sets
  `RUST_MIN_STACK=32MB` so test threads match main's interpreter thread,
  keeping MAX_DEPTH at 200.

---

## 2026-09-01 — Iteration 9: builtins

Added 11 builtins: `print`, `len`, `push`, `pop`, `keys`, `has`, `str`,
`int`, `float`, `type`, `range`. 7 new tests; suite is now 94.

Decisions:
- **Builtins are first-class values** (`Value::Builtin` enum variant)
  pre-bound in the global scope — not parser keywords or call-site
  special cases. The `print` hack from iteration 5 is gone. Consequences,
  all tested: `let f = len;` works, shadowing works lexically and
  un-shadows when the block ends, `type(len)` is `"function"`.
- `push`/`pop` mutate in place (paying off iteration 8's reference
  semantics); `pop` on an empty list is an error, matching the
  language's strictness.
- `int("abc")` is an error, not nil; `int(3.9)` truncates toward zero.
- `range(n)`/`range(lo, hi)` materializes a list — no lazy iterators in
  a tree-walker this size; half-open, empty when hi <= lo.
- `keys` returns sorted keys for free via the BTreeMap decision.
- Skipped `input()`/file I/O for now: the Interpreter would need a
  reader handle; deferred until after the REPL exists.

---

## 2026-09-01 — Iteration 10: REPL

Added `src/repl.rs`; `ting` with no arguments now starts an interactive
session. 8 new tests; suite is now 102.

Decisions:
- **The core is a pure, testable function** `eval_chunk(interp, src)` →
  Incomplete | Unit | Value(string) | Error(string); the I/O loop around
  it is thin. All REPL behavior is unit-tested without a TTY.
- **Expression echo**: a chunk that parses as a single expression is
  evaluated and echoed (strings quoted); nil results stay silent.
  Otherwise the chunk runs as statements.
- **Multi-line input**: a parse error at end-of-input first retries with
  `;` appended (so `let x = 1` just works), else prints a `.. `
  continuation prompt. An empty line cancels the pending buffer.
- **Pipe-friendly**: prompts and the banner appear only when stdin is a
  TTY (`IsTerminal`), so `echo 'expr' | ting` emits clean output.
- **No line editing** — std-only rules out readline; the terminal's
  cooked mode provides basics. Documented trade-off, not an oversight
  (backlog originally said "line editing"; scoped down to honor the
  zero-dependency decision, which wins).
- Session state persists across chunks (one Interpreter for the whole
  session); errors leave the session usable.

---

## 2026-09-01 — Iteration 11: caret diagnostics

Added `src/diag.rs`: script errors now print the offending source line
with a caret underline sized to the span, rustc-style. 6 new tests;
suite is now 108. Every error path (lex, parse, runtime) flows through
it because iterations 2-7 put byte spans on tokens, AST nodes, and
runtime errors — that early decision paid off here with a ~40-line
renderer.

Details: multi-line spans clamp to their first line; tabs in the
underline prefix stay tabs so carets align in terminals; caret width
counts chars (not bytes) so unicode lines up; zero-width spans (EOF
errors) get one caret. REPL errors stay message-only for now — chunks
are short and the buffer is on screen.

---

## 2026-09-01 — Iteration 12: examples as integration tests

Added six programs under `examples/` (hello, fizzbuzz, fibonacci,
closures, collections, sort) with golden `examples/*.out` files, and
`tests/examples.rs`, which runs the real `ting` binary
(`CARGO_BIN_EXE_ting`) against every example and diffs stdout. Suite is
now 108 unit + 1 integration test covering 6 programs.

Decisions:
- **Golden files over inline expectations**: examples double as
  documentation, and the .out files show what running them looks like.
  Outputs were generated by the binary, then hand-verified (fib(20) =
  6765, compose = 41, sort output, word counts) before being frozen.
- **The integration test enforces a minimum count** (≥6) so a glob typo
  can't silently turn the test into a no-op, and every .ting must have
  its .out.
- Examples deliberately exercise deterministic output only (`keys()`
  sortedness makes the word-count example stable).
- Noticed while writing examples: no string split/join builtins and no
  `for`-style loop; both would improve example ergonomics. Added to the
  backlog as a stretch item rather than blocking docs/CI/release.

---

## 2026-09-01 — Iteration 13: language reference

Wrote `docs/reference.md` — the complete language in one document:
source form, values/types, reference semantics, operator precedence
table, statements, functions/closures, all 11 builtins, error model,
limits. Rewrote the README status section with a real code sample and a
pointer to docs and examples.

Verification notes:
- The README closure sample was piped through the binary (prints
  `1 2 3`) before committing — docs only claim what runs.
- Checked the cyclic-data claim: building `xs[0] = xs` succeeds quietly;
  only printing/comparing it diverges. The reference says exactly that.
- Full suite still green (108 + examples).

---

## 2026-09-01 — Iteration 14: rustfmt + CI

Applied `cargo fmt` to the whole tree (it had drifted — fmt is now part
of the bar) and added `.github/workflows/ci.yml`: on push/PR to main,
run `cargo fmt --check`, `clippy --all-targets -D warnings`, and
`cargo test` on a Linux/macOS/Windows matrix.

Decisions:
- **Three-OS matrix** because the interpreter touches OS-y things
  (thread stack sizes via `RUST_MIN_STACK` in `.cargo/config.toml`,
  TTY detection in the REPL) — Windows is the likeliest to differ.
- **No caching/pinning actions**: only `actions/checkout` plus the
  runner's preinstalled rustup; a zero-dependency crate builds in
  seconds, and fewer third-party actions is a smaller supply chain.
- All three CI steps were run locally before pushing. The first cloud
  run (id 33494921787) is being watched in the background; its outcome
  lands in the next loop iteration.

**CI fix (same day):** run 33494921787 failed on Windows only — git's
CRLF conversion rewrote the golden `.out` files on checkout, so the
byte-for-byte comparison saw `\r\n` vs the interpreter's `\n`. Fix:
`.gitattributes` with `* text=auto eol=lf` (run 33495058612 verifies).
The 3-OS matrix earned its keep on its very first run.

---

## 2026-09-01 — Iteration 15: v0.1.0 release

CI confirmed green on all three OSes, so the release shipped: added
`.github/workflows/release.yml` (on `v*` tag: create the GitHub release,
then a 3-target matrix builds `--release` and uploads
`ting-v0.1.0-{x86_64-linux-gnu,aarch64-apple-darwin,x86_64-windows-msvc}`
archives), tagged `v0.1.0`, and pushed the tag. Run 33495207887 is
watched in the background.

Decisions:
- **Native-runner targets only** (no cross-compilation): three real
  platforms with zero extra tooling. More targets can come later if
  anyone asks.
- **`gh release` CLI instead of third-party upload actions** — same
  small-supply-chain reasoning as CI; `actions/checkout` remains the
  only external action.
- **Release notes point at docs/reference.md and LOG.md** — the decision
  log is part of the product for this experiment.
- Version stays 0.1.0 (first release, matching Cargo.toml).

**Release verified:** run 33495207887 green; all three assets present at
https://github.com/stefanobaghino/thing/releases/tag/v0.1.0 (darwin
276KB, windows 186KB, linux 310KB). Downloaded the darwin archive and
ran two examples with the released binary — correct output. Every
original backlog item from iteration 0 is now shipped: a complete,
documented, tested, CI'd, released language. Remaining work is the
stretch list (split/join, for-in, break/continue, REPL carets).

---

## 2026-09-01 — Iteration 16: string builtins (stretch)

Added `split`, `join`, `trim` (builtin count 11 → 14). 2 new tests;
suite is now 110. Docs table and README count updated.

Decisions:
- `split(s, "")` splits into single-character strings (chars, not
  bytes) — the useful reading of an empty separator, instead of
  erroring like Python.
- `join` requires a list of strings — no implicit stringification,
  consistent with `+`.
- `trim` included beyond the original stretch note: trivial and pairs
  naturally with `split` for input handling.

---

## 2026-09-01 — Iteration 17: for-in, break, continue (stretch)

Added `for x in iterable { ... }`, `break`, and `continue` (new keywords
`for`/`in`/`break`/`continue`). 8 new tests; suite is now 118. The
collections example now reads idiomatically (split + two for loops) and
its golden file was regenerated; the reference documents the semantics.

Design decisions:
- **Iteration targets**: lists (elements), strings (chars), maps (keys,
  sorted). Anything else errors.
- **Snapshot semantics**: the iterable is copied shallowly at loop
  entry, so the body can safely mutate the underlying list/map (tested:
  pushing while iterating terminates).
- **Per-iteration binding**: each iteration gets a fresh scope for the
  loop variable, so closures capture that iteration's value (tested:
  three closures print 0 1 2, not 2 2 2 — avoids the classic JS `var`
  trap).
- **`break`/`continue` ride the existing `Control` enum** (the payoff
  predicted in iteration 7). They stop at the innermost loop, error at
  top level, and error when escaping a function boundary (tested).
- CI green on the split/join/trim commit confirmed before starting.

---

## 2026-09-01 — Iteration 18: REPL carets + v0.2.0

REPL errors now render with the same caret diagnostics as scripts
(`repl:LINE:COL` against the current input chunk, so multi-line
definitions point at the right line). This supersedes iteration 11's
"message-only" note. 1 net new test; suite is now 119.

That was the last stretch item, so v0.2.0 ships in the same iteration:
version bumped in Cargo.toml, tagged, pushed — the release workflow
builds the three platform binaries. v0.2.0 over v0.1.0: split/join/trim
builtins, for-in loops with break/continue, REPL caret diagnostics.

Backlog after this: idle maintenance. The loop will slow its cadence and
look for external signals (issues, PRs) rather than inventing features —
scope discipline is part of the experiment.

**v0.2.0 verified:** CI and release workflows green; all three platform
assets present on the release. The loop now enters idle maintenance at a
slow cadence.

---

## 2026-09-01 — Protocol change: no idle (human directive)

The experiment's owner said: "My intention is for you to keep building
more and not stop in a maintenance state." That is the external signal
idle maintenance was waiting for — and it retires idle maintenance as a
concept.

Enforcement, not just intention:

- `LOOP.md` gains a **No idle** section: an empty backlog makes
  *replenishment* the iteration's task (design the next milestone, log
  it, refill `STATE.md`). Maintenance may preempt building for one
  iteration but never replace it. Since every iteration starts by
  re-reading `LOOP.md`, the rule survives compaction and restarts.
- `STATE.md` refilled with the v0.3.0 milestone: string builtins batch 2,
  script I/O (`args`/`input`/`read_file`/`write_file`), `sort`/`sort_by`,
  runtime error recovery (likely a `try(f)` builtin), a tutorial, then
  the release. Post-v0.3 candidates parked: WASM playground, fuzzing,
  bytecode VM.
- Cadence returns to active (short wakeups while mid-feature).

Rationale for the milestone: everything shipped so far makes ting a
language demo; I/O + sorting + error recovery make it a *tool* someone
could actually script with, which is the strongest next increment of
real-world value per BOOTSTRAP's spirit.

---

## 2026-09-01 — Iteration 19: string builtins, batch 2

First v0.3.0 task (kicked off by the owner's "push the loop forward").
Seven new builtins in `src/eval.rs::call_builtin`: `contains` (substring,
or list membership via structural `==`), `replace` (all occurrences;
empty search string is an error rather than Rust's surprising
intersperse behavior), `starts_with`/`ends_with`, `upper`/`lower`
(Unicode-aware), and `slice` on strings (by chars) and lists (fresh
copy). `slice` follows Python bounds: negatives from the end, clamping,
backwards range is empty — one `slice_bounds` helper, documented there.
5 new tests (123 unit + examples green); reference table updated.

---

## 2026-09-01 — Iteration 20: script I/O builtins

`args()`, `input()`, `read_file()`, `write_file()` in
`src/eval.rs::call_builtin`; `ting script.ting [args...]` now forwards
everything after the script path to `args()` (`src/main.rs`). Decisions:
`input()` strips the trailing newline (and `\r`) and returns `nil` at
EOF — the idiomatic read loop is `while line != nil`; `read_file`
returns the whole file as one string (pair with `split` for lines);
errors carry the OS message. Deviation from the backlog: no golden
example, because the example runner provides no stdin/args — instead
`tests/io.rs` runs the real binary with piped stdin and argv. 25
builtins, 127 tests, reference updated.

---

## 2026-09-01 — Iteration 21: sort and sort_by

`sort(xs)` and `sort_by(xs, f)` in `src/eval.rs`. Decisions: both return
a fresh list (consistent with `slice`; in-place mutation stays the
domain of `push`/`pop`); `sort_by` takes a key function, not a
comparator — cheaper to use, and keys are computed once
(decorate-sort-undecorate) so user code runs len(xs) times, not
O(n log n) times. Ordering rules: all numbers (int/float mixed) or all
strings; anything else — including mixing the two classes — errors
before sorting via `ensure_sortable`. Rust's stable `sort_by` gives
stability for free (tested with equal keys). New `call_value` helper
lets builtins invoke user functions — the piece `try(f)` will need
next. 2 tests; suite at 129. Reference updated.

---

## 2026-09-01 — Iteration 22: try/fail error recovery

Error recovery landed as two builtins, no new syntax: `try(f)` calls
`f()` and returns `{"ok": result}` or `{"err": message}`; `fail(msg)`
raises. Rationale for builtins over try/catch syntax: zero
lexer/parser/AST changes, first-class like every other builtin, and the
map-result shape composes with `has`. Verified the risky invariant:
`call` restores depth and env before an error propagates, so `try`
catches even a stack overflow and the interpreter stays usable after
(tested). Reference's "no exception handling (yet)" note replaced with
a recovery example. 1 test (8 assertions); suite at 130.

---

## 2026-09-01 — Iteration 23: tutorial, executable

`docs/tutorial.md`: hello → values → loops → closures → reference
semantics → try/fail → a word-frequency script that uses args, file I/O
with recovery, maps, sort_by, and slice. The honesty mechanism is
`tests/tutorial.rs`: it extracts every ```ting block, runs it through
the real binary, and when a ```text block follows, diffs stdout against
it exactly — the tutorial cannot rot without CI going red. All 9
snippets are standalone (the wordfreq one falls back to a built-in
sample when no file argument is given, so it runs deterministically
everywhere). README now leads with the tutorial and reflects 29
builtins. Suite at 131.

---

## 2026-09-01 — Iteration 24: v0.3.0 released

CI green on the tutorial commit closed out the milestone; version
bumped and tag v0.3.0 pushed — the release workflow builds the three
platform binaries. v0.3.0 over v0.2.0, "ting as a practical scripting
tool": string builtins batch 2 (contains/replace/starts_with/ends_with/
upper/lower/slice), script I/O (args/input/read_file/write_file with
argv forwarding), stable sort/sort_by, try/fail error recovery, and an
executable tutorial. 29 builtins, 131 tests. Verification of the
release assets follows when the workflow finishes.

**v0.3.0 verified:** release workflow green; all three assets present;
downloaded the darwin binary, ran fizzbuzz and a smoke test exercising
the new builtins (`try`/`slice`/`upper`) — correct output.

---

## 2026-09-01 — Replenishment: v0.4.0 milestone

Backlog was empty, so per LOOP.md "No idle" this iteration designs the
next milestone. Candidates weighed: WASM playground (high reach, keeps
the no-service rule — a static page anyone can open or self-host),
fuzz-style robustness testing (cheap, catches panics), bytecode VM
(large internal rewrite, little visible value now — parked). Chosen:
**v0.4.0 — ting in the browser + robustness**. The interesting
constraint is keeping the zero-dependency claim: no wasm-bindgen —
instead a cdylib with a small hand-rolled extern "C" ABI (alloc/run
returning UTF-8 in wasm memory), which a dozen lines of JS can call.
Step 1 is extracting a lib.rs so the interpreter is linkable from both
the binary and the wasm cdylib.

---

## 2026-09-01 — Iteration 25: interpreter extracted into lib.rs

First v0.4.0 task. `src/lib.rs` now owns the modules and a
`run_source(path, src, out, args)` entry point that does the whole
lex/parse/run pipeline and returns rendered caret diagnostics as
strings; `src/main.rs` shrank to argv handling, the big-stack thread,
and printing. The wasm cdylib (next task) will call `run_source` with a
Vec<u8> writer. 2 lib tests; suite at 133. Unit tests moved with their
modules untouched.

---

## 2026-09-01 — Iteration 26: wasm cdylib, no wasm-bindgen

`src/wasm.rs`: a five-function extern "C" ABI (ting_alloc/ting_dealloc/
ting_run/ting_result_ptr/ting_result_len); the result buffer lives in a
thread-local and is valid until the next run — wasm is single-threaded,
so that is effectively a global. On error the rendered caret diagnostic
is appended after whatever the program printed. The module is plain
Rust compiled on every target, so the host suite tests the ABI exactly
as the JS glue will drive it (3 tests). Cargo grows `crate-type =
["rlib", "cdylib"]`; the wasm build is `cargo build --release --lib
--target wasm32-unknown-unknown` (--lib avoids a bin/lib ting.wasm
filename collision). Verified beyond compilation: instantiated the
294KB ting.wasm in Node and ran three programs through the raw ABI —
loops+builtins, a caret diagnostic, fib(20) — all correct. Suite at
136.

---

## 2026-09-01 — Iteration 27: browser playground

`playground/index.html`: a single static page — editor, output pane,
six example programs, ctrl+enter — that drives the wasm ABI. The
interpreter runs in a Web Worker built from a Blob (still one file), so
a runaway script can't freeze the tab: after 5s the worker is killed
and rebuilt. `playground/build.sh` compiles and copies ting.wasm (the
artifact is gitignored; the Pages workflow will build it). Verified in
a real browser via Playwright against a local static server: examples
produce correct output, caret diagnostics render, `while true {}`
times out and the page recovers and runs again. Next: the Pages deploy.

---

## 2026-09-01 — Iteration 28: playground live on GitHub Pages

Enabled Pages (build_type=workflow) via the API and added
`.github/workflows/pages.yml`: builds ting.wasm, uploads playground/ as
the site artifact, deploys with actions/deploy-pages; path-filtered so
LOG/STATE-only commits don't redeploy. First deploy green. Verified the
live site in a real browser at http://www.baghino.me/thing/ (the
account's custom domain fronts the project page): wasm loads, hello and
word-frequency examples produce correct output. README now leads with
the playground link. BOOTSTRAP note: this is still not an operated
service — the site is a static artifact anyone can rebuild and host
(`playground/build.sh` + any file server); Pages is just distribution.

---

## 2026-09-01 — Iteration 29: fuzz tests find and fix a real panic

`tests/fuzz.rs`: a zero-dependency xorshift64* PRNG drives (a) 3000
random token-soup programs, (b) 300 single-character mutations of every
example, (c) 1000-deep nested expressions — all through
lex/parse/execute (execution skipped when `while` appears, the one
unbounded construct) under catch_unwind; any panic fails with the
reproducing seed.

It paid for itself on the first run: `parser.rs::describe()` missed
`TokenKind::Colon` and hit `unreachable!()` — meaning any script with a
misplaced `:` (e.g. `1 : 2;`) crashed the parser instead of printing a
diagnostic, reachable from the released binary and the playground.
Fixed (Colon arm + harmless fallback instead of unreachable!), plus a
direct regression test. Suite at 141.

---

## 2026-09-01 — Iteration 30: v0.4.0 released

Milestone complete; version bumped, tag v0.4.0 pushed. v0.4.0 over
v0.3.0, "ting in the browser + robustness": interpreter as a library
(lib.rs/run_source), wasm cdylib with a hand-rolled ABI (no
wasm-bindgen, still zero dependencies), the browser playground live on
GitHub Pages, fuzz tests (which found and fixed a real parser panic on
stray ':'), and the describe() hardening. 141 tests. Asset verification
follows when the release workflow finishes.

**v0.4.0 verified:** release workflow green; three assets present;
darwin binary smoke-tested — the stray-colon program that panicked
v0.3.0 now prints a clean caret diagnostic (exit 1), sort_by works.

---

## 2026-09-01 — Replenishment: v0.5.0 milestone

Per LOOP.md "No idle". Parked candidates reviewed: bytecode VM still
poor value-per-iteration; self-hosted tests promoted. Chosen: **v0.5.0
— expressiveness**: (1) functional builtins map/filter/reduce plus
min/max/abs — the biggest everyday gap now that sort_by exists; (2) an
assert(cond, msg) builtin and a self-hosted test suite — ting programs
that test ting, run by CI through the real binary, which both proves
the language is usable and grows coverage in the language itself; (3)
modules: import(path) — needs a design iteration first (return value
vs. namespace map, caching, cycle detection); (4) release. Playground
examples get map/filter once they land.

---

## 2026-09-01 — Iteration 31: map/filter/reduce/min/max/abs

Six functional builtins. Decisions: all three higher-order forms
iterate a snapshot and build fresh lists (consistent with sort/slice);
`filter` demands an actual bool from the predicate — returning anything
else errors, in keeping with no-truthiness; `reduce(xs, init, f)` puts
init in the middle so the call reads left-to-right; `min`/`max` reuse
`ensure_sortable`/`cmp_ordered` (empty list errors); `abs` checks i64
overflow (abs(i64::MIN)). Playground gains a "map & filter" example
(deploys via the path filter). 2 tests (18 assertions); suite at 143.

---

## 2026-09-01 — Iteration 32: assert + self-hosted test suite

`assert(cond)` / `assert(cond, msg)` builtin (strict: non-bool
condition or non-string message errors; failure is an ordinary
catchable runtime error, so ting can test assert with try — and does).
The self-hosted suite: five ting programs under selftest/ —
arithmetic, strings, collections, functions/control-flow, errors —
100+ assertions about language semantics, written in the language
itself. `tests/selftest.rs` runs each through the real binary and
requires exit 0 with empty stdout, so a stray print fails CI too.
All passed first run. 36 builtins; suite at 144 (+5 ting programs).

---

## 2026-09-01 — Iteration 33: import() modules

Design decided and implemented in one iteration (it stayed small
because the builtin machinery already existed). `import(path)`: runs
the file in a fresh global environment on the same interpreter (its
prints still flow to the program's output), returns the module's
top-level bindings as a map — builtins still bound to their own names
are treated as ambient and excluded. Relative paths resolve against
the importing file's directory (a dir_stack handles nesting); modules
are cached by canonicalized path so every import returns the same map
(reference semantics make that observable and useful); circular
imports error; lex/parse/runtime errors inside a module surface as
`error in module "x.ting" at LINE:COL: ...`. Why a map and not new
syntax: no lexer/parser changes, and a namespace you can pass around
is more ting-like than magic bindings. selftest gains modules.ting +
_lib.ting (self-hosted import tests); a Rust test covers nested
imports, cycles, and module diagnostics. 37 builtins; suite at 145
(+7 ting programs).

---

## 2026-09-01 — Iteration 34: v0.5.0 released

Milestone complete; version bumped, tag v0.5.0 pushed. v0.5.0 over
v0.4.0, "expressiveness": map/filter/reduce/min/max/abs, assert + the
self-hosted selftest/ suite (7 ting programs now), and import()
modules. 37 builtins, 145 host tests. Asset verification follows when
the release workflow finishes.

**v0.5.0 verified:** release workflow green; three assets; darwin
binary smoke-tested with a two-file program exercising import, map,
reduce, assert — correct output.

---

## 2026-09-01 — Replenishment: v0.6.0 milestone

Per LOOP.md "No idle". Chosen: **v0.6.0 — performance + polish**.
Rationale: the language surface is now rich (37 builtins, modules,
error recovery); the next real value is making it measurably fast and
smoothing the edges people actually touch. Backlog: (1) a benchmark
harness — ting benchmark scripts timed against the release binary, a
baseline recorded in the repo so regressions are visible; (2) 1-2
interpreter optimizations guided by those numbers (candidates:
env-lookup cost, per-call Vec allocs, Value clone traffic — measure
first, then pick); (3) playground share-by-URL (source encoded in the
fragment — still fully static); (4) tutorial section on modules; (5)
release. Bytecode VM stays parked: optimizations must be measured
against the tree-walker before a rewrite earns its cost.

---

## 2026-09-01 — Iteration 35: benchmark harness + baseline

bench/: four ting workloads (fib(28) for call overhead; list
map/filter/reduce/sort churn at 100k; 100k-key map insert+iterate;
60k-part string build/join/split) and bench/run.py, which builds the
release binary, takes the median of 5 runs, checks each script's
printed checksum, and rewrites bench/BASELINE.md with --write.
Workloads were tuned so interpreter time dominates process startup
(first cut ran in 5-45ms; scaled to ~50-300ms). Baseline on this
machine: fib 295ms, lists 101ms, maps 112ms, strings 54ms. Not a CI
gate — numbers are machine-relative; the checksums are the correctness
tripwire. Next iteration profiles these and optimizes the top cost.

---

## 2026-09-01 — Iteration 36: measured optimization pass (~10%)

Three changes, each kept only after measuring against bench/:

1. Blocks with no direct `let` no longer allocate a child scope —
   if/while bodies hit that on every entry (fib 295→279ms, maps
   112→100ms).
2. Call frames build their HashMap up front with capacity (no
   measurable change alone — logged so it isn't retried).
3. Env keys and Function params are `Rc<str>` instead of `String`:
   binding a parameter is now an Rc clone, not a heap-allocating
   String clone per call (fib →265ms, lists →92ms).

Cumulative vs the recorded baseline: fib -10%, lists -9%, maps -10%,
strings -8%; all checksums unchanged, 145 tests green. A `sample`
profile guided change 3 (allocator frames visible; cost otherwise
diffuse across eval/exec/call — noted as the honest argument for the
parked bytecode VM, which stays parked). BASELINE.md regenerated.

---

## 2026-09-01 — Iteration 37: playground share-by-URL

A "share" button encodes the editor source as base64url in the
location fragment (`#code=...`) and copies the link; opening such a
link decodes, loads, and auto-runs the code. Fragments never reach the
server, so sharing stays fully static and private. Unicode survives
the round trip (TextEncoder → btoa; verified with "héllo wörld").
Browser-verified via Playwright: link-load auto-run, share hash,
clipboard status. Deploys via pages.yml's playground/ path filter.

---

## 2026-09-01 — Iteration 38: tutorial covers modules

New "Splitting code into modules" section: a verified snippet that
writes its own module with write_file and then imports it — fully
self-contained, so the executable-tutorial contract holds (output
diffed by tests/tutorial.rs). The runner now executes snippets from
the script's own directory, aligning write_file (cwd-relative) with
import (script-dir-relative), which is also how a user actually runs
ting in place. 10 verified snippets.

---

## 2026-09-01 — Iteration 39: v0.6.0 released

Milestone complete; version bumped, tag v0.6.0 pushed. v0.6.0 over
v0.5.0, "performance + polish": ~10% interpreter speedup (measured
against the new bench/ harness with a recorded baseline), playground
share-by-URL (live), and the tutorial's modules section. Asset
verification follows when the release workflow finishes.

**v0.6.0 verified:** release workflow green; three assets; darwin
binary smoke-tested (fib(20) + assert). Sixth release.

---

## 2026-09-01 — Replenishment: v0.7.0 milestone

Per LOOP.md "No idle". Chosen: **v0.7.0 — developer experience**.
The language is stable and fast enough; the friction now is around it:
no string formatting, no editor highlighting, docs only readable as
raw markdown. Backlog: (1) format(fmt, ...) builtin with {}
placeholders and {{}} escapes — the everyday gap print+str leave; (2)
a TextMate grammar under editor/ (installable by hand in VS Code/
Sublime/Zed — no marketplace, per BOOTSTRAP's distribution rule); (3)
playground syntax highlighting driven by a small JS tokenizer (verify
via Playwright); (4) docs on the Pages site: a stdlib-only Python
md→html converter in tools/, wired into pages.yml, so the tutorial and
reference get real URLs next to the playground; (5) release v0.7.0.
Bytecode VM stays parked.
