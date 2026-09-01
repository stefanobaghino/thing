# The ting standard library

Three modules written in ting itself, living in `lib/` — and also
embedded in the interpreter, so `import("lib/...")` works from any
directory, in the REPL, and in the browser playground. A real file at
the same path always wins over the embedded copy, so you can vendor
and modify them freely.

```ting
let l = import("lib/list.ting");
print(l["sum"]([1, 2, 3]));   # 6
```

Imports return a map, so functions are reached with `["name"]`.

## lib/list.ting

| Function | Does |
|----------|------|
| `sum(xs)` | adds the elements (0 for an empty list) |
| `reverse(xs)` | a fresh list in reverse order |
| `zip(a, b)` | list of `[a[i], b[i]]` pairs, trimmed to the shorter input |
| `enumerate(xs)` | list of `[index, value]` pairs |
| `unique(xs)` | first occurrence of each element, order preserved (structural equality) |
| `flatten(xs)` | one level of nesting removed; non-lists pass through |

## lib/string.ting

| Function | Does |
|----------|------|
| `repeat(s, n)` | `s` concatenated `n` times |
| `pad_left(s, width, fill)` | prepends `fill` until at least `width` chars |
| `pad_right(s, width, fill)` | appends `fill` until at least `width` chars |
| `lines(s)` | split on `"\n"` |
| `title(s)` | first character of each space-separated word uppercased |

## lib/test.ting

A tiny test framework:

```ting
let t = import("lib/test.ting");
t["check"]("math works", 1 + 1 == 2);
t["check_eq"]("name", upper("a"), "A");
t["summary"]();   # prints failures + totals; exits 1 if any failed
```

| Export | Does |
|--------|------|
| `check(name, cond)` | records a pass or a named failure |
| `check_eq(name, got, want)` | like `check`, recording got/want on failure |
| `summary()` | prints `FAIL:` lines and totals; `exit(1)` on any failure |
| `state` | the counters map (`passed`, `failed`, `failures`) for tooling |

All of this is ordinary ting — read the sources in
[lib/](https://github.com/stefanobaghino/thing/tree/main/lib); the
self-hosted suite (`selftest/stdlib.ting`, `selftest/testlib.ting`)
keeps every function honest on both engines.
