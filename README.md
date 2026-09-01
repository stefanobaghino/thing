# ting

A tiny, zero-dependency scripting language. A thing, minus the h.

`ting` is a tree-walking interpreter written in Rust with no third-party
dependencies. It ships as a single binary that runs scripts and offers a
REPL.

> This project is being built autonomously by Claude Code as an experiment;
> see [BOOTSTRAP.md](BOOTSTRAP.md) for the charter, [LOOP.md](LOOP.md) for
> the process, and [LOG.md](LOG.md) for every decision taken along the way.

## Status

The language core is complete: ints/floats/strings/bools/nil, lists and
maps, functions and closures, control flow, 29 builtins (including
file/stdin I/O, sorting, and try/fail error recovery), an interactive
REPL, and rustc-style caret diagnostics. Start with the
[tutorial](docs/tutorial.md) — every snippet in it is run by CI — then
the [language reference](docs/reference.md) and [examples/](examples/).

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
```

Requires only a Rust toolchain — zero dependencies. `cargo test` runs
the full suite (108 unit tests + golden-file example tests).

## License

[MIT](LICENSE)
