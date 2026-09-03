# The ting language reference

ting is a small, strict, dynamically typed scripting language. This
document describes the whole language as implemented; it fits in one
sitting.

## Running

```sh
ting script.ting [args...]   # run a file; extra args go to args()
ting                         # interactive REPL (ctrl-d exits)
ting --check files...        # report syntax errors without running
ting --fmt files...          # reformat in place (--fmt-check to verify)
```

Scripts run on the bytecode VM by default; `--eval` (or
`TING_ENGINE=eval`) selects the reference tree-walking interpreter —
the two are held byte-identical by differential tests. The REPL uses
the reference engine.

The REPL echoes the value of bare expressions, keeps state across lines,
continues multi-line constructs with a `.. ` prompt (an empty line
cancels), and forgives a missing final `;`. Seven meta-commands:
`:help` lists every builtin with its doc line, `:doc NAME` explains
one builtin or stdlib function (module, signature, comment), `:vars`
lists the
session's own bindings, `:load <file>` evaluates a file in the
current session so its bindings stay available, `:time EXPR`
evaluates a one-line chunk and prints the elapsed milliseconds,
`:fmt` reprints the
last evaluated chunk as the formatter would write it, and `:clear`
resets the session. It has no built-in line editing or history (zero
dependencies); wrap it with
[rlwrap](https://github.com/hanslub42/rlwrap) — `rlwrap ting` — for
both.

## Source form

- UTF-8 text. Whitespace is insignificant except as a token separator.
- Comments run from `#` to the end of the line.
- Statements end with `;` — mandatory, except after a closing `}`.

## Values and types

| Type       | Literals / examples          | Notes                                  |
|------------|------------------------------|----------------------------------------|
| `int`      | `42`, `-7`                   | 64-bit signed; overflow is an error    |
| `float`    | `2.5`, `0.125`               | IEEE 754 double                        |
| `string`   | `"hi\n"`                     | immutable; escapes: `\n` `\t` `\r` `\\` `\"` |
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
| `range(hi)` / `range(lo, hi)` / `range(lo, hi, step)` | list of ints, half-open; `step` may be negative, never 0 |
| `split(s, sep)` | list of pieces; `split(s, "")` splits into characters |
| `join(xs, sep)` | joins a list of strings; non-string elements error     |
| `trim(s)`      | the string without leading/trailing whitespace              |
| `find(s, sub)` / `find(xs, v)` | index of the first match (character index for strings), or `nil` |
| `contains(s, sub)` / `contains(xs, v)` | substring test / list membership (structural `==`) |
| `replace(s, from, to)` | all occurrences replaced; empty `from` errors    |
| `starts_with(s, p)` / `ends_with(s, p)` | prefix / suffix test           |
| `upper(s)` / `lower(s)` | Unicode-aware case conversion                  |
| `slice(x, lo, hi)` | sub-string (by chars) or fresh sub-list, half-open; negatives count from the end, out-of-range clamps |
| `args()`       | the command-line arguments after the script path, as a list of strings |
| `input()`      | one line from stdin without the newline; `nil` at end of input |
| `read_file(path)` | the file's entire contents as a string; `"-"` reads stdin to EOF |
| `write_file(path, s)` / `write_file(path, s, "append")` | writes (or overwrites) the file; `"append"` adds to the end |
| `sort(xs)`     | a fresh sorted list; all numbers or all strings, else error |
| `sort_by(xs, f)` | a fresh list sorted by key `f(x)`, stable; keys obey `sort`'s rules |
| `try(f)`       | calls `f()`; `{"ok": result}` on success, `{"err": message}` on a runtime error |
| `fail(msg)`    | raises a runtime error with the given string message         |
| `map(xs, f)`   | a fresh list of `f(x)` for each element                      |
| `filter(xs, f)` | a fresh list of the elements where `f(x)` is `true` (bool required) |
| `reduce(xs, init, f)` | folds left: `f(f(init, x0), x1)…`                     |
| `min(xs)` / `max(xs)` | smallest / largest element; `sort`'s ordering rules; empty list errors |
| `abs(n)`       | absolute value of an int or float                            |
| `assert(cond)` / `assert(cond, msg)` | error unless `cond` is `true` (bool required) |
| `import(path)` | runs the file once and returns its top-level bindings as a map; see below |
| `format(fmt, ...)` | fills `{}` placeholders left-to-right (`{{`/`}}` for literal braces); placeholder/value count mismatch errors |
| `json_parse(s)` | JSON text to ting values (object→map, array→list, null→nil); malformed input errors with an offset |
| `json_str(v)` / `json_str(v, indent)` | ting value to JSON — compact, or pretty with `indent` spaces per level (map keys sorted); functions and non-finite floats error |
| `env(name)`    | the environment variable's value, or `nil` if unset          |
| `exit()` / `exit(code)` | ends the program with that status (default 0); not catchable by `try` |
| `time_ms()`    | milliseconds since the Unix epoch, as an int                 |

### Modules

`import(path)` loads another ting file, runs it in a fresh global
scope, and returns a map of everything its top level defined:

```ting
# mathutils.ting
fn double(x) { return x * 2; }

# main.ting
let m = import("mathutils.ting");
print(m["double"](21));   # 42
```

Relative paths resolve against the importing file's directory. A module
runs once per program: later imports return the very same map (mutating
it is visible everywhere). Circular imports, missing files, and errors
inside the module are ordinary runtime errors (the message carries the
module's own line and column).

The standard library (`lib/list.ting`, `lib/string.ting`,
`lib/test.ting`) is also embedded in the binary: when an imported
`lib/...` path has no matching file, the built-in copy is used — so
`import("lib/list.ting")` works from any directory, in the REPL, and
in the browser playground. A real file with that path always wins.

## Errors

An unhandled runtime error stops the program with a diagnostic pointing
at the offending source:

```text
script.ting:2:7: error: undefined variable 'totl'
 2 | print(totl + 1);
   |       ^^^^
```

The interpreter is strict on purpose: no truthiness, no implicit
conversions, exact arity, integer overflow checks, missing map keys and
out-of-bounds indices error immediately.

To recover from an expected failure, wrap the risky code in a function
and hand it to `try`; raise your own errors with `fail`:

```ting
let r = try(fn() { return int(input()); });
if has(r, "err") { print("not a number:", r["err"]); }
```

## Tooling

The `ting` binary is the whole toolchain — no separate installs:

- `ting --fmt <paths...>` reformats in place; `--fmt-check` exits 1
  if anything would change (use it in CI). Directories recurse. The formatter is
  idempotent and never alters program meaning.
- `ting --check <paths...>` reports lexer, parser, and compiler
  diagnostics without running anything — built for pre-commit hooks.
  Directories recurse.
  Clean files may still get warnings (an imported stdlib module
  indexed with a name it does not export); warnings never change
  the exit status.
- `ting --test <paths...>` runs each file (directories recurse,
  sorted; `--filter SUBSTR` keeps only matching paths; `--tap`
  emits Test Anything Protocol output for CI consumers) in its own
  process and
  prints `ok` or `FAIL` per file (with the diagnostic under a
  failure) and a summary; exit 1 if anything failed. Pair it with
  `lib/test.ting` or plain `assert` calls.
- All three accept `-` for stdin; `ting --fmt -` is a filter that
  writes the formatted source to stdout, for editor integrations.
- `ting --doc NAME` prints what the REPL's `:doc` would: a builtin's
  signature and doc line, or a stdlib function's signature, module
  and comment. Exit 1 for an unknown name.
- `ting --lsp` speaks the Language Server Protocol on stdio:
  diagnostics as you type (syntax errors, and a warning when an
  imported stdlib module is indexed with a name it does not export),
  hover docs for every builtin (and for imported stdlib functions),
  completion
  (builtins, keywords, the document's own names, and the functions of
  any stdlib module it imports),
  whole-document formatting, an outline of top-level bindings
  (document symbols), go-to-definition for them, token-level
  find-references and rename, signature help inside builtin
  calls, folding ranges for multi-line braces, workspace symbols
  across open files, document links on `import(...)` paths that
  exist on disk, and a quickfix that corrects a misspelt stdlib
  member to the nearest export.

Point your editor's generic LSP client at `ting --lsp`; a TextMate
grammar for syntax highlighting ships in the repo under `editor/`.

Scripts behave as shell citizens: `ting x.ting | head` ends quietly
with exit 0 when the reader goes away, and a runtime error prints a
diagnostic and exits 1.

## Stability

As of 2.0, the language described on this page is stable: programs
relying on documented behavior keep working across 2.x releases.
Builtins may be added in minor releases, never removed or changed
incompatibly; a breaking change to syntax or semantics would require
a 3.0. The two engines are held to this same document by differential
tests.

## Limits

- Call depth: 200.
- Integers: i64 range; overflow raises an error rather than wrapping.
- Map keys: strings only.
- Cyclic data (`xs[0] = xs;`) prints and compares infinitely — don't.
