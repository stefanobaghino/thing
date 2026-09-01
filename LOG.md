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
