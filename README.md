# ting

A tiny, zero-dependency scripting language. A thing, minus the h.

`ting` is implemented in Rust with no third-party dependencies. One
binary contains everything: two execution engines (a bytecode VM and a
reference tree-walking interpreter), a REPL, a canonical formatter
(`--fmt`), a static checker (`--check`), and a nine-capability
language server (`--lsp`).

> This project is being built autonomously by Claude Code as an experiment;
> see [BOOTSTRAP.md](BOOTSTRAP.md) for the charter, [LOOP.md](LOOP.md) for
> the process, [LOG.md](LOG.md) for every decision taken along the way, and
> [the retrospective](docs/retrospective.md) for the story so far.

## Status

The language is complete: ints/floats/strings/bools/nil, lists and
maps, functions and closures, control flow, modules via `import()`
plus five embedded stdlib modules (list/map/string/math/test),
44 builtins (file and stdin I/O, JSON with pretty printing, sorting,
map/filter/reduce, try/fail error recovery, string formatting), an
interactive REPL with `:help` and `:load`, and rustc-style caret
diagnostics. Binaries for Linux (x86-64 and arm64), macOS and Windows
are attached to every
[release](https://github.com/stefanobaghino/thing/releases). **Try it
in your browser at the
[playground](http://www.baghino.me/thing/)** — the interpreter
compiled to WebAssembly, running entirely on your machine. Start with
the [tutorial](docs/tutorial.md) — every snippet in it is run by CI —
then the [language reference](docs/reference.md),
[examples/](examples/), and the [changelog](CHANGELOG.md).

Two execution engines share one semantics: the bytecode VM (default —
11-35% faster on function-heavy work) and the reference tree-walking
interpreter (`--eval`), held byte-identical by differential tests —
including generated random programs — and a CI job that reruns the
whole suite on the reference engine. Editor highlighting lives in
[editor/](editor/); the performance story in [docs/vm.md](docs/vm.md)
and [bench/](bench/).

```ting
fn make_counter() {
  let n = 0;
  fn tick() { n = n + 1; return n; }
  return tick;
}
let c = make_counter();
print(c(), c(), c());   # 1 2 3
```

ting is strict on purpose: no truthiness, no implicit conversions,
integer overflow checks, and missing map keys or out-of-bounds indices
fail loudly with a caret pointing at the source.

## Building

```sh
cargo build --release
./target/release/ting script.ting   # run a script
./target/release/ting               # start the REPL
./target/release/ting --help        # everything else
```

Requires only a Rust toolchain — zero dependencies. `cargo test` runs
the full suite: unit tests, golden-file examples, the self-hosted
selftest/ programs (ting testing ting), differential engine tests,
and fuzz tests — 180+ in all.

## License

[MIT](LICENSE)
