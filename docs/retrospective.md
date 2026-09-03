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

## The third act: small strokes

After 2.0 froze the language, the loop settled into a different
gait: one small, finished thing per iteration — a feature, a doc,
a test, a health check — released whenever a few of them add up to
something a user would want. Five minor releases came out of that
rhythm in two days. The pattern that emerged:

- **Additive only.** `json_str` grew an indent argument; `range`
  grew a step; the stability promise stayed intact because nothing
  existing changed shape.
- **Each feature pulls its documentation with it.** The tutorial
  gained a JSON chapter whose snippets are CI-executed — and that
  harness caught three wrong claims (about `push`, compact output,
  and an error message) before they could mislead anyone.
- **The toolchain became the product.** `--check` for pre-commit
  hooks; the LSP went from three capabilities to eight (outline,
  definition, references, rename) in four strokes, each landed with
  a pipe-driven protocol test.
- **Trust nothing that wasn't re-verified.** A flaky network call
  inside the CI watcher produced five false "FAILED" verdicts; the
  loop's rule is now to re-read the run's conclusion from the API
  before believing any failure — and every release is still
  downloaded cold and executed before it's called done.

## The fourth act: a new machine

The loop was stopped by its owner after the thirty-fifth release
and restarted the same day on a different computer — a four-core
arm64 Linux board instead of a Mac. Three things followed from that
one change, and each says something about what the loop had
actually been relying on.

- **Verification is only as honest as the hardware under it.** None
  of the three release archives ran on the new host, so the first
  release there was verified structurally — archive contents, binary
  formats, bundled stdlib — and the log says so in as many words.
  The next stroke added an `aarch64-unknown-linux-gnu` target to the
  release matrix (and the same runner to CI, so the label was proven
  on a push before it was needed on a tag). Every release since has
  been downloaded cold and *executed* again.
- **Running the binary finds what tests don't.** Poking the released
  `ting` from a shell turned up two rough edges no suite had asked
  about: piping output into `head` ended in a broken-pipe panic, and
  the formatter and checker could not read stdin. Both became
  strokes with regression tests proven to fail on the old code; the
  broken-pipe test then earned its keep on Windows in CI. Later,
  writing an example with a multi-line list literal showed the
  formatter had never been given one — its bracket handling was
  simply absent, and the corpus had never exercised it.
- **Every incident becomes a rule.** A transient GitHub outage
  failed a Pages deploy; rerunning only the failed job made it worse
  (the single-job workflow ended up with two artifacts), and a fresh
  push did nothing because the workflow filters on paths that the
  log files miss. The recovery was a manual dispatch, and the
  operating rules in `STATE.md` now say exactly that, so the next
  incident costs one tick instead of three.

The gait did not change: one small verifiable stroke per tick,
a release every third or so, the log carrying the reasons. The
stdlib roughly doubled across the act — grouping, windows, keyed
uniqueness, string predicates — each function landing with
selftests that run on both engines and a row in the reference.

## The fifth act: what "zero dependencies" meant

The forty-ninth release was the first whose cold test failed. The
new test runner spawned child processes; on Linux that pulled a
`pidfd` symbol out of the C library that Ubuntu 24.04 versions at
glibc 2.39, and the binaries stopped starting on anything older —
including the board the loop itself runs on. Nothing in the crate
had changed its dependencies; the *standard library's* choice of
libc symbol had, because the code path was new. Three things came
out of an afternoon:

- Linux release builds moved to the oldest supported runner, and a
  workflow step now reads each binary's highest glibc symbol
  version and fails the build above the floor — the regression
  class is closed, not just the instance.
- Fully static musl archives joined the release, so there is a
  Linux binary with no C library dependency at all. The crate's
  "zero dependencies" finally means what it says.
- The broken release stayed published, with its notes rewritten to
  say exactly what is wrong and where the fix is. Deleting it would
  have erased the evidence; the log keeps the rest.

The cold test — download the artifact, run it on a real machine —
had felt like ceremony for forty-eight releases. It was the only
check in the whole pipeline that could have caught this, because CI
runs where it builds.

## The sixth act: the rhythm

After the machine move the loop found a cadence and kept it for
six milestones and twenty-six releases. Each milestone begins with
a replenishment tick that does no building: it reads what the last
milestone left uneven, weighs candidates, rejects some with a
reason, and writes five ordered strokes into the backlog. Then one
stroke per tick — a stdlib function with selftests on both engines,
a tool flag with an integration test, a chapter with an executed
snippet — and a release whenever three have landed, each release
downloaded cold and executed on the loop's own machine. A health
tick closes the milestone: benchmark checksums, fuzz sweeps on the
engines and the formatter, an audit of every release's assets and
every page on the site.

What the rules gained along the way, each from a specific slip:
verdicts come from the API and never from a watcher's exit code;
the docs guard runs *after* the log entry is written, and the push
gates on the literal `test result: ok` because a looser grep once
let a failed run through; Linux release builds stay on the oldest
runner with a glibc floor enforced in the workflow; a failed Pages
deploy is retried only by dispatching the workflow; the cold test
is never skipped, because it is the only check that runs where
users do rather than where CI builds.

The stdlib roughly tripled to six modules; the runner grew
directories, filters and a TAP mode; the editor server reached ten
capabilities, several of them stdlib-aware. None of it changed the
language. That was the point of freezing it.

## The seventh act: second opinions

Eleven more tags, and the loop stopped once in the middle of them.
A human said "stop", and it stopped; a human said "go", and it
resumed at the next backlog line as if the pause had been an
ordinary wakeup. That is the whole design of the state file: the
loop keeps nothing in its head that it has not written down.

The act's theme arrived sideways. The front door had drifted three
milestones behind the house, so one milestone rewrote the README
as prose that links rather than a feature list that rots. The
playground turned out to carry a hand-written copy of the examples
that had stopped tracking them long ago, so another milestone made
it generated, with a guard that fails CI when the copy goes stale —
the same treatment the cookbook already had. Duplication the loop
would refuse in code had survived in the site for months because
nobody ran a check on it. Now something does.

Then the checker and the editor learned to give the same second
opinion. One function produces every semantic warning, and both
tools call it: a stdlib module indexed with a name it does not
export, a top-level binding nothing uses, a parameter a body never
names. The first of the new warnings flagged seventy-nine bindings
across the corpus and broke a test — every stdlib module's
functions are exports, and a file that is only `let x = 1;` is the
smallest possible module. The rule that fixed both was a shape,
not a list: a file whose top-level statements are all bindings is a
module, and modules are exempt. The parameter warning found twelve
hits, every one a constant callback in the test suite that really
did ignore its argument, and the fix was twelve underscores. A
warning that produces no false positives on the corpus and a
handful of honest true ones is calibrated about right.

Smaller things landed in the same rhythm: rename across open
documents, document links, hover and signature help for the user's
own functions, the runner going parallel with its output kept in
order, `--fmt --diff` to show a change instead of making it, and
`--doc` with no name printing the table of contents that until
then lived only on the website. One slip is worth keeping: the
site's audit probed paths that had moved, read the 404s as an
outage, and only the next probe noticed the redirect. The audit
now records where things live, which is the only reason it can
tell an outage from its own mistake.

## The eighth act: where it happened

Seven more tags. The act opened with a bug the loop found by
tripping over it: a runtime error inside an imported module's
function was reported at some unrelated line of the importing file,
because the error carried the module's byte offset and nothing else.
Reproducing it with an embedded stdlib module found a worse one
underneath — the foreign offset landed past the end of the
importer's source and the diagnostic renderer panicked. The fix was
to make a function remember the file it was defined in and let an
error pick that origin up as it leaves, on both engines through the
one call path they share; the note that followed names the place in
the importer that called in. The renderer now clamps a stray offset
to the line instead of dying on it. The checker learned to follow
local imports, the editor learned to flag a broken one on the import
string, and the tutorial shows the diagnostic that the loop's own
binary printed — after one push quoted the wrong column, which is
its own small lesson about reading the smoke output before writing
the prose that quotes it.

Two milestones were spent keeping the story straight. The README
said nine editor capabilities when there were twelve; the VM design
document still called itself a design for v0.9.0; the stdlib had
grown a dozen helpers that no example used. Each got a stroke rather
than a rewrite: a paragraph, a status section, two example programs
whose output is diffed across both engines and regenerated into the
cookbook and the playground by the tools that guard them. A binary
that lists what it knows — `--doc` with no name, a module name, or
the path of your own file — closed the gap between the site and the
shell.

Then cycles. The reference had documented for a long time that a
list containing itself "prints and compares infinitely — don't", and
a probe showed what that meant: four one-line programs, each
taking the whole process down with a stack overflow and no
diagnostic. Print now marks the point of recursion, equality keeps
the pairs it is inside and takes a revisit as agreement, and the
JSON encoder keeps its path and refuses with an ordinary error. The
crash fuzzer, whose generator never builds a cycle, was handed five
on purpose. A limit that says "don't" is a bug report the project
filed against itself and left open; this act closed it.

The process slipped twice in the same way and the rule tightened
twice. A tick's shell script ran on past a failed step and pushed a
log entry describing a green gate that had been red; the fix was to
chain every step with `&&`. It happened again because the second
version leaned on `set -e`, which the harness running the commands
quietly ignores — checked directly, and written down. The log
carries both corrections next to the entries they correct, which is
the only form of honesty a permanent record can offer.

## Where it stands

Seventy-nine tags in, the loop still runs: pick one verifiable
task, land it green, log the reasons, repeat. The lasting lesson of
the tooling acts is that at some point the most valuable thing to
build for a language stops being the language — and the lasting
lesson of the loop is that a project can be driven indefinitely on
one honest iteration at a time.
