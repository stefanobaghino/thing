# ting

A tiny, zero-dependency scripting language. A thing, minus the h.

`ting` is implemented in Rust with no third-party dependencies. One
binary contains everything: two execution engines (a bytecode VM and a
reference tree-walking interpreter), a REPL, a canonical formatter
(`--fmt`), a static checker (`--check`), a profiler (`--profile`),
and a thirteen-capability language server (`--lsp`).

> This project is being built autonomously by Claude Code as an experiment;
> see [BOOTSTRAP.md](BOOTSTRAP.md) for the charter, [LOOP.md](LOOP.md) for
> the process, [LOG.md](LOG.md) for every decision taken along the way, and
> [the retrospective](docs/retrospective.md) for the story so far.

## Status

The language is complete: ints/floats/strings/bools/nil, lists and
maps, functions and closures (parameters may carry defaults, the
last may take the rest, and a list spreads into a call), control
flow, modules via `import()`
plus twelve embedded stdlib modules
(list/map/string/math/json/fs/test/time/sh/args/err/csv),
66 builtins (file and stdin I/O, listing, making and removing
directories, JSON with pretty printing, sorting, map/filter/reduce,
try/fail error recovery, string formatting, regular expressions,
running other programs, the clock and a seeded generator), and
rustc-style caret diagnostics. Binaries for Linux (x86-64 and arm64,
glibc and fully static musl), macOS and Windows are attached to every
[release](https://github.com/stefanobaghino/thing/releases). **Try it
in your browser at the
[playground](http://www.baghino.me/thing/)** — the interpreter
compiled to WebAssembly, running entirely on your machine. Start with
the [tutorial](docs/tutorial.md) — every snippet in it is run by CI —
then the [language reference](docs/reference.md), the
[stdlib page](docs/stdlib.md), the [cookbook](docs/cookbook.md) of
runnable examples, and the [changelog](CHANGELOG.md).

The whole toolchain is the one binary. A REPL with meta-commands
(`:help`, `:doc`, `:vars`, `:load`, `:time`, `:fmt`, `:history`, `:save`,
`:clear`);
`ting --test` running every file under a directory in its own process,
in parallel with `-j`, with `--filter`, `--slow` and Test Anything
Protocol output (`--tap`); `--watch` on the tests, the checker and
`--fmt-check`, re-running the pass whenever a watched file changes;
`--check` and `--fmt` over files or
directories (stdin with `-`), the formatter showing its changes with
`--diff` and keeping a file's line endings, the checker following
local imports and warning about misspelt stdlib members, unused
bindings (top-level or local), unused parameters and names that
shadow a builtin (`--strict` makes them fail the check); `--profile SCRIPT`
reporting calls and self time per function and builtin; `--doc NAME`
for any builtin or stdlib function, `--doc
MODULE` for a module's members and `--doc` alone for the whole table
of contents; and `--lsp`, a language server with diagnostics (the
same warnings), hover, completion, signature help, formatting,
symbols and workspace symbols, definition, references, highlights of
the symbol under the cursor, rename across open files (declined
early on keywords and builtins), folding, document links, and a
quickfix for those misspellings. A runtime error inside an imported module points at the
module's own line, with a note naming the call site. The
[reference](docs/reference.md#tooling) has the details.

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
echo 'print(1);' | ./target/release/ting -   # or one from stdin
./target/release/ting               # start the REPL
./target/release/ting -h            # everything else
```

Requires only a Rust toolchain — zero dependencies. `cargo test` runs
the full suite: unit tests, golden-file examples, the self-hosted
selftest/ programs (ting testing ting), differential engine tests,
and fuzz tests (engines and formatter) — 200+ in all.

## License

[MIT](LICENSE)
