# The ting language reference

ting is a small, strict, dynamically typed scripting language. This
document describes the whole language as implemented; it fits in one
sitting.

## Running

```sh
ting script.ting   # run a file
ting               # interactive REPL (ctrl-d exits)
```

The REPL echoes the value of bare expressions, keeps state across lines,
continues multi-line constructs with a `.. ` prompt (an empty line
cancels), and forgives a missing final `;`.

## Source form

- UTF-8 text. Whitespace is insignificant except as a token separator.
- Comments run from `#` to the end of the line.
- Statements end with `;` — mandatory, except after a closing `}`.

## Values and types

| Type       | Literals / examples          | Notes                                  |
|------------|------------------------------|----------------------------------------|
| `int`      | `42`, `-7`                   | 64-bit signed; overflow is an error    |
| `float`    | `2.5`, `0.125`               | IEEE 754 double                        |
| `string`   | `"hi\n"`                     | immutable; escapes: `\n` `\t` `\\` `\"` |
| `bool`     | `true`, `false`              | no truthiness — conditions demand bool |
| `nil`      | `nil`                        | the absence of a value                 |
| `list`     | `[1, "two", [3]]`            | mutable, reference semantics           |
| `map`      | `{"a": 1}`                   | string keys only, sorted; reference semantics |
| `function` | `fn(x) { return x; }`, `len` | first-class; compared by identity      |

`1.` is not a float literal (it lexes as `1` then `.`); `1.0` is.

### Reference semantics

Lists and maps behave like Python/JS objects: assignment, argument
passing, and nesting share the same underlying storage.

```ting
let a = [1];
let b = a;
b[0] = 2;
print(a);        # [2]
let c = a + [];  # + always builds a fresh list — use it to copy
```

Equality (`==`) on lists and maps is structural (deep); on functions it
is identity.

## Operators

Tightest first; binary operators associate left.

| Precedence | Operators                    | Operand rules                                 |
|-----------|-------------------------------|-----------------------------------------------|
| postfix   | `f(args)`, `x[i]`             | calls and indexing chain freely               |
| unary     | `-`, `!`                      | `-` on numbers, `!` on bools                  |
| factor    | `*`, `/`, `%`                 | numbers; int `/` truncates; `/ 0` on ints errors |
| term      | `+`, `-`                      | `+` also concatenates strings and lists       |
| compare   | `<`, `<=`, `>`, `>=`          | numbers (mixed ok) and strings                |
| equality  | `==`, `!=`                    | any values; `1 == 1.0` is true                |
| and       | `&&`                          | bools; short-circuits                         |
| or        | `\|\|`                        | bools; short-circuits                         |

Mixed int/float arithmetic promotes to float. There is no implicit
conversion anywhere else: `1 + "x"` is a type error, `if 1 { }` is a
type error.

### Indexing

- Lists: `xs[i]` with an int; negative indices count from the end
  (`xs[-1]` is the last element); out of bounds is an error.
- Strings: `s[i]` yields a one-character string, by character (not byte).
- Maps: `m["key"]` with a string; a missing key is an error — test with
  `has(m, "key")` first.

## Statements

```ting
let x = 1;          # define (or shadow) in the current scope
x = 2;              # rebind the nearest existing x; undefined name errors
xs[0] = 9;          # write a list slot / insert or update a map key
{ let y = 1; }      # block: introduces a scope; y does not leak
if c { } else if d { } else { }
while c { }
for x in xs { }     # iterate a list, a string (chars), or a map (keys)
break;              # exit the innermost loop
continue;           # next iteration of the innermost loop
fn add(a, b) { return a + b; }
return expr;        # only inside a function; bare `return;` yields nil
expr;               # expression statement (e.g. a call)
```

`if`/`while`/`for` require braces and take no parentheses around the
condition. Assignment is a statement, not an expression (`a = b = c` and
`1 = 2` are parse errors).

`for` iterates over a **snapshot** taken when the loop starts, so the
body may mutate the list or map it is iterating. Map iteration visits
keys in sorted order. The loop variable is a fresh binding each
iteration, so closures created in the body capture that iteration's
value. `break`/`continue` apply to the innermost `while`/`for` and may
not cross a function boundary.

## Functions

`fn name(a, b) { ... }` is sugar for `let name = fn(a, b) { ... };`.
Functions are closures: they capture their defining environment by
reference, so captured variables can be mutated and the mutation is
shared.

```ting
fn make_counter() {
  let n = 0;
  fn tick() { n = n + 1; return n; }
  return tick;
}
let c = make_counter();
print(c(), c());   # 1 2
```

Calls check arity exactly. Falling off the end of a function returns
nil. Recursion is fine up to a call depth of 200, after which the
interpreter raises `stack overflow` rather than crashing.

## Builtins

All builtins are ordinary global bindings — they can be passed around
(`let f = len;`) and shadowed (`let len = 5;` hides the builtin for that
scope).

| Builtin        | Does                                                        |
|----------------|-------------------------------------------------------------|
| `print(...)`   | prints args separated by spaces, then a newline; returns nil |
| `len(x)`       | length of a list, string (in chars), or map                 |
| `push(xs, v)`  | appends to a list in place; returns nil                     |
| `pop(xs)`      | removes and returns the last element; empty list errors     |
| `keys(m)`      | the map's keys as a sorted list                             |
| `has(m, k)`    | whether string key `k` is present                           |
| `str(v)`       | the value rendered as a string                              |
| `int(v)`       | from int/float (truncates)/numeric string; else error       |
| `float(v)`     | from int/float/numeric string; else error                   |
| `type(v)`      | the type name as a string, e.g. `"list"`                    |
| `range(hi)` / `range(lo, hi)` | list of ints, half-open; empty if `hi <= lo` |
| `split(s, sep)` | list of pieces; `split(s, "")` splits into characters |
| `join(xs, sep)` | joins a list of strings; non-string elements error     |
| `trim(s)`      | the string without leading/trailing whitespace              |
| `contains(s, sub)` / `contains(xs, v)` | substring test / list membership (structural `==`) |
| `replace(s, from, to)` | all occurrences replaced; empty `from` errors    |
| `starts_with(s, p)` / `ends_with(s, p)` | prefix / suffix test           |
| `upper(s)` / `lower(s)` | Unicode-aware case conversion                  |
| `slice(x, lo, hi)` | sub-string (by chars) or fresh sub-list, half-open; negatives count from the end, out-of-range clamps |

## Errors

There is no exception handling (yet): any runtime error stops the
program with a diagnostic pointing at the offending source:

```text
script.ting:2:7: error: undefined variable 'totl'
 2 | print(totl + 1);
   |       ^^^^
```

The interpreter is strict on purpose: no truthiness, no implicit
conversions, exact arity, integer overflow checks, missing map keys and
out-of-bounds indices error immediately.

## Limits

- Call depth: 200.
- Integers: i64 range; overflow raises an error rather than wrapping.
- Map keys: strings only.
- Cyclic data (`xs[0] = xs;`) prints and compares infinitely — don't.
