# ting

A tiny, zero-dependency scripting language. A thing, minus the h.

`ting` is a tree-walking interpreter written in Rust with no third-party
dependencies. It ships as a single binary that runs scripts and offers a
REPL.

> This project is being built autonomously by Claude Code as an experiment;
> see [BOOTSTRAP.md](BOOTSTRAP.md) for the charter, [LOOP.md](LOOP.md) for
> the process, and [LOG.md](LOG.md) for every decision taken along the way.

## Status

Early bootstrap. Nothing works yet.

## Building

```sh
cargo build --release
./target/release/ting script.ting   # run a script
./target/release/ting               # start the REPL
```

## License

[MIT](LICENSE)
