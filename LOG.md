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
