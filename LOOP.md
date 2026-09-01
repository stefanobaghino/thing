# The Agentic Loop

This file defines the protocol Claude Code follows to autonomously carry out
the experiment described in [BOOTSTRAP.md](BOOTSTRAP.md). It is the loop's
operating manual: every iteration starts by re-reading this file.

## Files

| File | Role | Discipline |
|------|------|------------|
| `BOOTSTRAP.md` | The experiment's charter | Never modified |
| `LOOP.md` | This protocol | Modified only when the protocol itself changes; the change is logged |
| `STATE.md` | Current objective, backlog, and status | Rewritten every iteration |
| `LOG.md` | Append-only decision log | Every decision appended, never edited or deleted |

## Iteration protocol

Each iteration is one unit of work, executed as follows:

1. **Orient** — read `STATE.md`. If context was summarized or lost, also
   re-read `LOOP.md` and skim the tail of `LOG.md`.
2. **Pick** — select the single highest-value task from the backlog in
   `STATE.md`. Prefer finishing started work over starting new work.
3. **Execute** — do the task: write code, tests, or docs.
4. **Verify** — build and run the full test suite. A failing tree is never
   committed; fix or revert before proceeding.
5. **Commit** — one commit per coherent change. Every commit must build and
   pass tests on its own. Follow the commit-hygiene rules in the user's
   global instructions.
6. **Log** — append to `LOG.md`: what was decided, what was done, why, and
   anything surprising. Timestamped.
7. **Update state** — rewrite `STATE.md`: mark done work, reorder or refine
   the backlog, note blockers.
8. **Schedule** — schedule the next wakeup (self-paced). Short delay when
   mid-feature with clear next steps; longer when the backlog is thin and
   reflection is needed. Never stop the loop unless a human explicitly says
   to stop.

## No idle (human directive, 2026-09-01)

The experiment's owner directed the loop to keep building rather than
settle into maintenance. Therefore:

- **Idle maintenance is not a legal state.** Watching for issues/PRs and
  keeping CI green happens *alongside* building, never instead of it.
- **An empty backlog makes replenishment the task.** When step 2 finds no
  actionable work, the iteration is spent designing the next milestone:
  propose features, tooling, ports, docs, or a companion artifact; log the
  reasoning; write the new backlog into `STATE.md`; then continue building
  on the next tick.
- Maintenance signals (broken CI, an issue, a PR) preempt the backlog for
  one iteration, then building resumes.

## Failure handling

- A blocked task is logged with the blocker, moved down the backlog, and the
  next task is picked. The loop never idles on a blocker it can route around.
- If the same task fails twice, the third attempt must take a different
  approach (or the task is redesigned). Log the pivot.
- If the tree is broken at orient time, fixing it is automatically the
  highest-value task.

## Guardrails (from BOOTSTRAP.md, restated operationally)

- Never modify `BOOTSTRAP.md`.
- Nothing illicit or dubious; comply with all guardrails.
- The artifact must be runnable by anyone on their own device — no hosted
  service, no restrictive distribution platform. Permissive license (MIT).
- Only free-of-charge software may be used or installed.
- Stay within the Claude subscription: keep iterations focused, avoid
  wasteful re-derivation, don't spawn agents the task doesn't need.
- Every decision lands in `LOG.md`.
