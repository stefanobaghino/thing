# Bytecode VM design

Status: design for the v0.9.0 milestone. The tree-walking interpreter
in `eval.rs` stays as the reference implementation; the VM must be
behaviorally identical and is only worth keeping if the benchmarks say
so.

## Why

Profiling (iteration 36) showed the tree-walker's cost is diffuse
dispatch overhead: every expression re-matches AST enum variants
through nested `eval`/`exec` calls. A bytecode VM replaces that with a
flat instruction loop over a compact array. The ~10% we got from
targeted fixes is the ceiling of tweaking; the VM is the structural
answer.

## Shape

Two new modules, no changes to lexer/parser/AST:

- `compile.rs` — walks the AST once, emits a `Chunk`.
- `vm.rs` — executes a `Chunk` against the same `Value` type,
  builtins, and I/O writer the tree-walker uses.

```text
source → lex → parse → AST ─┬→ eval.rs   (reference)
                            └→ compile.rs → Chunk → vm.rs
```

Selected by a `--vm` flag on the CLI (`ting --vm script.ting`) until
parity + performance justify flipping the default.

## Chunk format

```rust
struct Chunk {
    code: Vec<Op>,          // fixed-size ops, no serialization needed
    consts: Vec<Value>,     // literal pool (strings, floats, fn protos)
    spans: Vec<Span>,       // spans[i] belongs to code[i] — diagnostics
}
```

`Op` is a plain Rust enum with small payloads (indices, offsets); the
chunk is an in-memory artifact, never written to disk, so there is no
binary format to stabilize.

## Instruction set (initial)

Stack machine. Working set, subject to growth:

- Constants/values: `Const(u32)`, `Nil`, `True`, `False`, `Int(i64)`
- Variables: `GetVar(u32)`, `SetVar(u32)`, `Define(u32)`
  (operand indexes a name table; storage is the Env chain, see below)
- Arithmetic/logic: `Add Sub Mul Div Rem Neg Not Eq Ne Lt Le Gt Ge`
  (same checked/promoting semantics as `eval::binary`)
- Data: `MakeList(u32)`, `MakeMap(u32)`, `Index`, `IndexSet`
- Control: `Jump(i32)`, `JumpIfFalse(i32)`, `JumpIfTrue(i32)`
  (relative; `&&`/`||` compile to jumps to keep short-circuit + strict
  bool checks)
- Calls: `Call(u8)` (argc; callee below args on the stack),
  `MakeFn(u32)` (const index of a FnProto: params + its own Chunk),
  `Return`
- Misc: `Pop`, `Print` is NOT special — builtins stay `Value::Builtin`
  called through `Call`.

## What deliberately stays the same

- **Values**: `Value` unchanged — lists/maps keep Rc reference
  semantics, equality/display untouched.
- **Variable storage**: the `Rc<RefCell<Env>>` chain, at first. The
  measured cost is dispatch, not storage; keeping Env preserves
  closure semantics (shared mutable capture) for free and keeps the
  first VM small. Resolving locals to stack slots is a later,
  separately-measured step.
- **Builtins**: the `call_builtin` implementation is reused verbatim —
  it needs an interpreter-like context (out, dir stack, import cache),
  which the VM carries too.
- **Errors**: `RuntimeError { message, span }`; the VM looks up
  `spans[ip]` when an op fails, so caret diagnostics are identical.
- **Depth cap**: same MAX_DEPTH on call frames; `try` still catches
  everything including it.

## Control flow lowering

- `if`/`while`/`for` compile to conditional jumps; jump targets are
  back-patched (emit placeholder, patch after the block ends).
- `for` keeps snapshot semantics: compile to
  `[eval iterable, snapshot, index=0]` + a loop reading `snapshot[i]`,
  with the loop variable `Define`d fresh each iteration.
- `break`/`continue` are jumps recorded per enclosing loop during
  compilation (a loop-context stack in the compiler); using them
  outside a loop is a compile error carrying the same message the
  tree-walker produces at runtime. (Known acceptable divergence:
  errors surface earlier; message text must match.)
- `return` unwinds the current frame; at top level it is a compile
  error, same message rule.

## Differential testing plan

Parity is the whole game:

1. `tests/differential.rs`: every selftest/, example, and tutorial
   program runs through both engines; stdout and error text must be
   byte-identical.
2. The fuzz token-soup corpus runs through both; results (ok/error
   message) must agree.
3. CI runs the full `cargo test` suite with the VM forced on via env
   var (`TING_ENGINE=vm`) in one extra matrix row, once parity lands.

## Rollout

1. Expressions only (arith, vars, calls into builtins) + differential
   tests.
2. Statements, control flow.
3. Functions/closures/`try`/`import`.
4. Full-suite parity, benchmark, and only then consider the default.

Non-goals for v0.9.0: serialized bytecode, register allocation, local
slot resolution, inline caches, GC changes.

## Measured outcome (v0.9.0)

Full parity achieved (differential corpus + the entire selftest suite
byte-identical). Performance: **no speedup** — +0-2% vs the
tree-walker on all four benchmarks (see bench/BASELINE.md). Reasons:
function bodies and builtins still tree-walk under the hybrid, which
is where fib/lists/maps spend their time, and the remaining per-op
costs (Env HashMap lookups, Value clone traffic) dominate over AST
dispatch. Per the rule above, the default engine remains the
tree-walker; `--vm` / `TING_ENGINE=vm` stay available and at parity.
The next levers, if performance becomes a goal again: compile function
bodies (remove the hybrid) and resolve locals to stack slots — both
larger than dispatch-only and to be measured on their own.
