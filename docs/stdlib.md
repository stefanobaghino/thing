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
| `sum_by(xs, f)` | adds `f(x)` over the elements (0 for an empty list) |
| `scan(xs, init, f)` | running reduce: `[init, f(init, x0), f(that, x1), ...]` |
| `product(xs)` | multiplies the elements (1 for an empty list) |
| `reverse(xs)` | a fresh list in reverse order |
| `rotate(xs, n)` | a fresh list rotated left by `n` (negative rotates right) |
| `zip(a, b)` | list of `[a[i], b[i]]` pairs, trimmed to the shorter input |
| `interleave(a, b)` | elements alternated starting with `a`; the longer tail follows |
| `enumerate(xs)` | list of `[index, value]` pairs |
| `unique(xs)` | first occurrence of each element, order preserved (structural equality) |
| `unique_by(xs, key)` | first element for each distinct `key(x)`, order preserved |
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
| `mean_by(xs, f)` | mean of `f(x)` as a float; empty list fails |
| `flatten(xs)` | one level of nesting removed; non-lists pass through |
| `group_by(xs, key)` | map from `key(x)` (a string) to the elements with that key, in input order |
| `count_by(xs, key)` | map from `key(x)` (a string) to how many elements share it |
| `frequencies(xs)` | map from each string element to its number of occurrences |
| `take(xs, n)` | the first `n` elements (fewer if the list is shorter) |
| `drop(xs, n)` | everything after the first `n` elements |
| `partition(xs, pred)` | `[matching, rest]` split by `pred`, both in input order |
| `window(xs, n)` | sliding windows of `n` consecutive elements (empty if shorter) |
| `first(xs)` | the first element, or `nil` on empty |
| `last(xs)` | the last element, or `nil` on empty |

## lib/string.ting

| Function | Does |
|----------|------|
| `repeat(s, n)` | `s` concatenated `n` times |
| `pad_left(s, width, fill)` | prepends `fill` until at least `width` chars |
| `pad_right(s, width, fill)` | appends `fill` until at least `width` chars |
| `center(s, width, fill)` | `s` centred in `width` with `fill` on both sides (odd gap: extra on the right) |
| `truncate(s, width, suffix)` | at most `width` chars, ending in `suffix` when cut |
| `indent(s, prefix)` | `prefix` before every non-empty line |
| `lines(s)` | split on `"\n"` |
| `words(s)` | whitespace-separated words, no empties |
| `title(s)` | first character of each space-separated word uppercased |
| `split_once(s, sep)` | `[before, after]` around the first `sep`, or `nil` |
| `trim_start(s)` | leading whitespace removed |
| `trim_end(s)` | trailing whitespace removed |
| `strip_prefix(s, prefix)` | `s` without a leading `prefix` (unchanged if absent) |
| `strip_suffix(s, suffix)` | `s` without a trailing `suffix` (unchanged if absent) |
| `count(s, sub)` | non-overlapping occurrences of `sub` |
| `chars(s)` | the characters as a list of one-character strings |
| `reverse(s)` | the characters in reverse order |
| `is_digit(s)` | non-empty and all ASCII digits |
| `is_alpha(s)` | non-empty and all cased letters (upper and lower forms differ) |

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
| `filter_map(m, pred)` | a fresh map with only the entries where `pred(key, value)` holds |
| `has_all(m, ks)` | true if every listed key is present |
| `invert(m)` | keys and values swapped (values must be strings; last key wins on duplicates) |
| `with(m, k, v)` | a fresh map with `k` set to `v` |
| `update(m, k, f)` | a fresh map with `f` applied to the value at `k` (must exist) |
| `top(m, n)` | the `n` entries with the largest numeric values as `[key, value]` pairs |

## lib/math.ting

| Function | Does |
|----------|------|
| `clamp(x, lo, hi)` | `x` limited to the range `[lo, hi]` |
| `sign(x)` | `-1`, `0`, or `1` |
| `pow(base, n)` | integer exponentiation by squaring; `n >= 0` |
| `gcd(a, b)` | greatest common divisor (absolute values) |
| `lcm(a, b)` | least common multiple (non-negative; 0 if either is 0) |
| `abs_diff(a, b)` | absolute difference |
| `round(x)` | nearest integer, halves away from zero |
| `floor(x)` | largest integer `<= x` |
| `ceil(x)` | smallest integer `>= x` |
| `sqrt(x)` | Newton's method square root, returns a float |
| `is_prime(n)` | true for prime `n` (trial division) |

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
