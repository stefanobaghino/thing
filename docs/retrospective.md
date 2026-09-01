# How a loop built a language

ting exists because of a one-page file, [BOOTSTRAP.md](https://github.com/stefanobaghino/thing/blob/main/BOOTSTRAP.md),
in which a human told an AI agent (Claude Code): *build "something",
keep going, keep a log*. This page is the story of what happened next,
written by the agent that did the work. Every claim below has a
timestamped paper trail in
[LOG.md](https://github.com/stefanobaghino/thing/blob/main/LOG.md).

## The loop

The first thing built was not the language — it was the process.
Three files run the project:

- **LOOP.md** — the protocol: orient, pick one task, execute, verify,
  commit, log, update state, schedule the next iteration. A later
  amendment (a human directive) made idling illegal: when the backlog
  empties, the iteration's task is to design the next milestone.
- **STATE.md** — rewritten every iteration: current milestone, ordered
  backlog, done list. This is what survives restarts and context loss.
- **LOG.md** — append-only. Decisions, reasons, measurements, and
  mistakes, including the embarrassing ones.

Each iteration is small enough to verify completely: nothing lands
without `cargo fmt`, clippy at `-D warnings`, and the full test suite;
nothing is called released until the built artifacts are downloaded
and exercised.

## What got built

A hundred logged iterations later, ting is:

- a **strict, dynamically typed scripting language** — closures,
  reference-semantics collections, modules, JSON, error recovery with
  `try`/`fail`, 43 builtins — implemented in Rust with **zero
  dependencies**;
- **two execution engines** — a bytecode VM (the default, ~45% faster
  on call-heavy work) and the tree-walking reference interpreter —
  held byte-identical by differential tests, generated random
  programs, and a CI job that reruns everything on the second engine;
- a **browser playground** (the interpreter compiled to WebAssembly
  with a hand-rolled ABI), live with syntax highlighting and
  share-by-URL;
- an **LSP server** (`ting --lsp`) with live diagnostics, hover docs,
  completions, and format-on-save, plus a TextMate grammar and a
  canonical **formatter** (`ting --fmt`) that is provably idempotent
  and AST-preserving;
- a **standard library written in ting** (lists, strings, a test
  framework), embedded in the binary so `import("lib/...")` works
  anywhere — including the browser;
- a **self-hosted test suite** (ting programs asserting ting's
  semantics), golden-file examples, an executable tutorial whose
  snippets CI runs, fuzz tests, and benchmarks with a recorded
  baseline;
- **seventeen releases** with binaries for Linux, macOS, and Windows.

## What the log preserves that a changelog wouldn't

**Measurements over intentions.** The VM story took four attempts,
and the log kept score the whole way. The first version reached full
behavioral parity and was honestly recorded as *not faster* (+0-2%) —
the default stayed the tree-walker, and the log said why. Compiling
function bodies made it *slower* (+2-7%), which sharpened the
diagnosis: dispatch was never the cost. Slot-resolved locals (killing
the per-call HashMap frame) delivered -35% on call-heavy code — and
only then did the default flip. A later pooling pass (recycling the
per-call stack and locals buffers) pushed the margin to -45%. Four
verdicts, three of them "not yet", all preserved.

**Bugs found by the harnesses, not by luck.** The fuzzer's first run
caught a parser panic (a stray `:` hit an `unreachable!()`) that had
shipped in three releases. The differential corpus caught two span
divergences the day the VM was born, and later a botched patch that
would have shipped broken function calls. A new example crashed the
mutation fuzzer by calling `exit()` mid-test — the test infrastructure
itself got fixed.

**Scope discipline.** Twice the backlog ran dry and the loop chose
idle maintenance over invented features; a human then said *keep
building*, and the protocol was amended so that replenishment — not
idling — is the rule.

## The constraints that shaped it

BOOTSTRAP's rules pushed the design somewhere specific: no operated
service meant everything must run on the user's machine (hence the
static playground and the stdio LSP); free distribution meant GitHub
releases and Pages; zero dependencies became a house style that forced
a hand-rolled JSON codec, wasm ABI, and JSON-RPC framing — each of
which later powered a feature.

## The honest ledger

Not everything worked first try, and the log says so: a token-soup
differential-fuzzing plan tested nothing (0 of 3000 programs parsed)
and was replaced by a grammar-directed generator; a "pre-size the call
frame" optimization measured at zero and was logged so it wouldn't be
retried; position encoding in the LSP is a documented approximation.
The project's rule of thumb, applied to itself: *strict on purpose,
and loud about what it doesn't do.*

## Where it stands

Seventeen releases in, the loop still runs: pick one verifiable task,
land it green, log the reasons, repeat. The most recent act was all
tooling — the formatter that keeps the repo's own ting sources
canonical, LSP completions and formatting, run-in-playground links on
every docs snippet — because at some point the most valuable thing to
build for a language stops being the language.
