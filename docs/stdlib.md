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
| `any(xs, pred)` | true if `pred` holds for some element (false on empty) |
| `all(xs, pred)` | true if `pred` holds for every element (true on empty) |
| `min_by(xs, key)` | element with the smallest `key(x)`; `nil` on empty |
| `max_by(xs, key)` | element with the largest `key(x)`; `nil` on empty |
| `chunk(xs, n)` | sublists of `n` elements, last may be shorter |
| `insert_at(xs, i, v)` | fresh list with `v` inserted before index `i` |
| `remove_at(xs, i)` | fresh list without the element at index `i` |
| `count(xs, v)` | elements structurally equal to `v` |
| `mean(xs)` | arithmetic mean as a float; empty list fails |
| `median(xs)` | middle of the sorted values (mean of two middles when even) |
| `flatten(xs)` | one level of nesting removed; non-lists pass through |
| `group_by(xs, key)` | map from `key(x)` (a string) to the elements with that key, in input order |
| `take(xs, n)` | the first `n` elements (fewer if the list is shorter) |
| `drop(xs, n)` | everything after the first `n` elements |
| `partition(xs, pred)` | `[matching, rest]` split by `pred`, both in input order |

## lib/string.ting

| Function | Does |
|----------|------|
| `repeat(s, n)` | `s` concatenated `n` times |
| `pad_left(s, width, fill)` | prepends `fill` until at least `width` chars |
| `pad_right(s, width, fill)` | appends `fill` until at least `width` chars |
| `lines(s)` | split on `"\n"` |
| `title(s)` | first character of each space-separated word uppercased |
| `split_once(s, sep)` | `[before, after]` around the first `sep`, or `nil` |
| `trim_start(s)` | leading whitespace removed |
| `trim_end(s)` | trailing whitespace removed |
| `count(s, sub)` | non-overlapping occurrences of `sub` |
| `chars(s)` | the characters as a list of one-character strings |
| `reverse(s)` | the characters in reverse order |

## lib/map.ting

| Function | Does |
|----------|------|
| `get(m, k, default)` | `m[k]` if present, else `default` |
| `merge(a, b)` | a fresh map with `a`'s entries then `b`'s (`b` wins ties) |
| `items(m)` | list of `[key, value]` pairs in sorted key order |
| `from_items(pairs)` | a fresh map built from `[key, value]` pairs |
| `values(m)` | values in sorted key order |
| `map_values(m, f)` | a fresh map with `f` applied to every value |
| `pick(m, ks)` | a fresh map with only the listed keys (missing skipped) |
| `omit(m, ks)` | a fresh map without the listed keys |

## lib/math.ting

| Function | Does |
|----------|------|
| `clamp(x, lo, hi)` | `x` limited to the range `[lo, hi]` |
| `sign(x)` | `-1`, `0`, or `1` |
| `pow(base, n)` | integer exponentiation by squaring; `n >= 0` |
| `gcd(a, b)` | greatest common divisor (absolute values) |
| `round(x)` | nearest integer, halves away from zero |
| `floor(x)` | largest integer `<= x` |
| `ceil(x)` | smallest integer `>= x` |
| `sqrt(x)` | Newton's method square root, returns a float |

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
| `check_err(name, f, want)` | passes if `f()` fails with a message containing `want` |
| `summary()` | prints `FAIL:` lines and totals; `exit(1)` on any failure |
| `state` | the counters map (`passed`, `failed`, `failures`) for tooling |

All of this is ordinary ting — read the sources in
[lib/](https://github.com/stefanobaghino/thing/tree/main/lib); the
self-hosted suite (`selftest/stdlib.ting`, `selftest/testlib.ting`)
keeps every function honest on both engines.
