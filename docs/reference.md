# The ting language reference

ting is a small, strict, dynamically typed scripting language. This
document describes the whole language as implemented; it fits in one
sitting.

## Running

```sh
ting script.ting [args...]   # run a file; extra args go to args()
ting - [args...]             # run a script read from stdin
ting                         # interactive REPL (ctrl-d exits)
ting --check files...        # report errors and warnings without running
ting --fmt files...          # reformat in place (--fmt-check to verify)
ting --test dirs...          # run every .ting file as a test
ting --test --watch dirs...  # and again on every change (--check too)
ting --doc [NAMES...]        # explain functions, list a module, or list all
ting --lsp                   # language server on stdio
ting --version | -V          # the version
ting --help | -h             # every option
```

A script may be a path or `-`, which reads it from standard input, so
a generated or piped program runs without a file to put it in:
`echo 'print(41 + 1);' | ting -`. Arguments after the dash reach
`args()` as usual, diagnostics name the script `-`, and a relative
`import` resolves against the working directory, a piped script
having no directory of its own. The script is the stream, so by the
time it runs stdin is at end of file and `input()` returns `nil`
immediately — pipe a script or pipe it data, not both.

Scripts run on the bytecode VM by default; `--eval` (or
`TING_ENGINE=eval`) selects the reference tree-walking interpreter —
the two are held byte-identical by differential tests. The REPL uses
the reference engine.

Exit status is 0 on success; 1 when the tool reports a failure — a
script that raises, a red test file, a file `--fmt-check` would
change, a warning under `--check --strict`; and 2 on a usage error —
an unknown option, a mode with no operand, a bad option value —
which also prints a pointer to `--help`, and names the option you
probably meant when one is close (`--fmr` finds `--fmt`).

The REPL echoes the value of bare expressions, keeps state across lines,
continues multi-line constructs with a `.. ` prompt (an empty line
cancels), and forgives a missing final `;`. Nine meta-commands:
`:help` lists every builtin with its doc line; `:doc NAME` explains
one builtin or stdlib function (module, signature, comment), `:doc
MODULE` lists a module's members and `:doc` alone the whole table of
contents, as `--doc` does; `:vars` lists the session's own bindings;
`:load <file>` evaluates a file in the current session so its
bindings stay available; `:time EXPR` evaluates a one-line chunk and
prints the elapsed milliseconds; `:fmt` reprints the last evaluated
chunk as the formatter would write it; `:history` lists every chunk
that ran without error, numbered; `:save <file>` writes those chunks
as a script that replays the session; and `:clear` resets the
session and the transcript. It has no built-in line editing or
up-arrow recall (zero dependencies); wrap it with
[rlwrap](https://github.com/hanslub42/rlwrap) — `rlwrap ting` — for
both.

## Source form

- UTF-8 text. Whitespace is insignificant except as a token separator.
- Comments run from `#` to the end of the line.
- Statements end with `;` — mandatory, except after a closing `}`.

## Values and types

| Type       | Literals / examples          | Notes                                  |
|------------|------------------------------|----------------------------------------|
| `int`      | `42`, `-7`, `0xff`, `0b1010` | 64-bit signed; overflow is an error    |
| `float`    | `2.5`, `0.125`, `1.5e-3`     | IEEE 754 double                        |
| `string`   | `"hi\n"`                     | immutable; escapes: `\n` `\t` `\r` `\\` `\"` `\uXXXX` |
| `bool`     | `true`, `false`              | no truthiness — conditions demand bool |
| `nil`      | `nil`                        | the absence of a value                 |
| `list`     | `[1, "two", [3]]`            | mutable, reference semantics           |
| `map`      | `{"a": 1}`                   | string keys only, sorted; reference semantics |
| `function` | `fn(x) { return x; }`, `len` | first-class; compared by identity      |

`1.` is not a float literal (it lexes as `1` then `.`); `1.0` is.

An integer may be written in hex (`0xff`) or binary (`0b1010`) — the
prefix is lowercase, the hex digits are not. Any run of digits may be
broken up with `_` for legibility: `1_000_000`, `0xFF_FF`, `0b1010_1010`.
A separator has to sit between two digits, so `1_`, `_1` and `1__0` are
not numbers, and a literal cannot run into a name or a digit outside its
radix: `0b12` and `12abc` are errors rather than two tokens.

A decimal number carrying an exponent is a float whether or not it has
a point: `1e3` is `1000.0`, and `1.5e-3`, `2E+2` and `6e23` all read.
The exponent has to be complete — the letter, an optional sign, then at
least one digit — so `1e` is an error rather than a number followed by
a name. A literal too large for a double (`1e400`) is an error; one too
small (`1e-400`) is zero, as it is in JSON.

A number printed by `print`, `str` or `json_str` is written so it can
be read back. A float takes the shortest spelling that round-trips to
the same double, with an exponent outside the range 1e-4 to 1e17 —
`1e23` rather than twenty-three digits of expansion — and keeps a `.0`
when it is integral, so `1.0` stays visibly a float. Every form is
both a ting literal and valid JSON.

The conversions refuse exactly what a literal refuses. `float(s)` fails
on a string that would be infinite or is not a number at all, including
the words `inf` and `nan`; `json_parse` fails on a number out of range
for a double (`1e999`) rather than decoding it as infinity, which is
what `json_str` has always refused to encode. `int(x)` on a float
truncates toward zero, and fails rather than saturating when the value
is non-finite or outside i64. `int(s)` reads a string the way the lexer
reads a literal — sign, `0x`/`0b` prefix, `_` between digits — so
`int(hex(n))` and `int(bin(n))` give back `n`.

A string literal spells a character outside the escape set either
directly (source is UTF-8, so `"café"` is fine) or as `\uXXXX` —
four hex digits, a high surrogate followed by a low one for anything
past U+FFFF, exactly as JSON spells it. That is deliberate: a string
copied out of a JSON document means the same thing in a literal as it
does through `json_parse`. `ord` and `chr` convert between a
one-character string and its code point.

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
| unary     | `-`, `!`, `~`                 | `-` on numbers, `!` on bools, `~` on ints     |
| factor    | `*`, `/`, `%`                 | numbers; int `/` truncates; `/ 0` on ints errors |
| term      | `+`, `-`                      | `+` also concatenates strings and lists       |
| shift     | `<<`, `>>`                    | ints; count 0 to 63; `>>` keeps the sign      |
| bit and   | `&`                           | ints                                          |
| bit xor   | `^`                           | ints                                          |
| bit or    | `\|`                          | ints                                          |
| compare   | `<`, `<=`, `>`, `>=`          | numbers (mixed ok) and strings                |
| equality  | `==`, `!=`                    | any values; `1 == 1.0` is true                |
| and       | `&&`                          | bools; short-circuits                         |
| or        | `\|\|`                        | bools; short-circuits                         |

Mixed int/float arithmetic promotes to float. There is no implicit
conversion anywhere else: `1 + "x"` is a type error, `if 1 { }` is a
type error.

The bit operators are int-only — `1.5 & 2` is a type error, not a
rounded promotion — and they bind tighter than every comparison, so
`flags & MASK == MASK` groups the mask first and means what it looks
like. (C orders them the other way and has been explaining the
consequence ever since.) `~` complements: `~0` is `-1`. `>>` is an
arithmetic shift, so the sign survives: `-16 >> 2` is `-4`. A shift
count outside 0 to 63 is an error rather than a value the hardware
would have to invent.

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
nil. Recursion is capped, and the cap is derived from the host stack
the interpreter was given rather than fixed: the `ting` binary hands
its interpreter 32 MB and allows a few thousand frames from it
(fewer in an unoptimized build, where a frame costs several times as
much); an embedder that says nothing gets a conservative default.
Past the cap the interpreter raises `stack overflow (max call depth
N)`, naming the N it enforced, rather than crashing.

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
| `int(v)`       | from int/float (truncates)/numeric string; a string is read the way a literal is (`0xff`, `0b1010`, `1_000`, a leading sign); else error |
| `float(v)`     | from int/float/numeric string; a string that would be infinite or is not a number errors |
| `hex(n)` / `bin(n)` | an int as a `0x` or `0b` literal, sign kept (`hex(-255)` is `-0xff`); `int` reads them back |
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
| `ord(s)` / `chr(n)` | a one-character string to its code point, and back; `ord` of anything but one character, and `chr` of a number that is not a code point, error |
| `slice(x, lo, hi)` | sub-string (by chars) or fresh sub-list, half-open; negatives count from the end, out-of-range clamps |
| `args()`       | the command-line arguments after the script path, as a list of strings |
| `input()`      | one line from stdin without the newline; `nil` at end of input |
| `read_file(path)` | the file's entire contents as a string; `"-"` reads stdin to EOF |
| `write_file(path, s)` / `write_file(path, s, "append")` | writes (or overwrites) the file; `"append"` adds to the end |
| `list_dir(path)` | the names in the directory, sorted; not the paths, and not recursive; a path that is not a readable directory errors |
| `exists(path)` | whether anything is at that path; `is_dir(path)` whether that thing is a directory. Questions, so an absent or unreadable path is `false`, never an error |
| `is_dir(path)` | see `exists` |
| `make_dir(path)` | creates the directory and any missing parents; a directory that is already there is not an error |
| `remove_file(path)` | deletes the file; absent, or a directory, errors |
| `remove_dir(path)` | deletes an empty directory; one with anything in it errors. `lib/fs.ting`'s `remove_tree` composes the recursive version |
| `sort(xs)`     | a fresh sorted list; all numbers or all strings, else error |
| `sort_by(xs, f)` | a fresh list sorted by key `f(x)`, stable; keys obey `sort`'s rules |
| `try(f)`       | calls `f()`; `{"ok": result}` on success, and on a runtime error `{"err": message, "at": where it was raised, "trace": the calls it came out of}` |
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

### Files and directories

`read_file` and `write_file` handle a file's contents; `list_dir`,
`exists`, `is_dir` and `make_dir` handle the tree around it. The
split between them is deliberate:

- `exists` and `is_dir` are **questions**. An absent, unreadable or
  otherwise awkward path answers `false`; neither ever raises, so
  they can be used in an `if` without a `try` around them.
- `list_dir` is a **demand**, and errors when the path is not a
  readable directory. Asking what is inside something that is not
  there is a mistake, and an empty list would hide it. A name that
  is not valid UTF-8 fails the whole listing rather than being
  dropped or lossily converted, since a lossy name would not reopen
  the file it came from.
- `make_dir` creates missing parents, and a directory that already
  exists is success rather than an error: the useful postcondition
  is that the directory is there, not that this call made it.

`list_dir` answers with names, not paths — joining them onto the
directory is `lib/fs.ting`'s `entries`, which is also where `walk`
(every file at or below a directory), path splitting and extension
handling live.

`remove_file` and `remove_dir` are demands too, and `remove_dir`
takes only an empty directory. The recursive version is not a
builtin: `lib/fs.ting`'s `remove_tree` walks a tree, removing files
and then each directory once it is empty, so the one operation that
can destroy a lot of work at once is readable ting rather than a
word that hides it.

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
script.ting:2:7: error: undefined variable 'totl' (did you mean 'total'?)
 2 | print(totl + 1);
   |       ^^^^
```

When the name you typed is close to one that is in scope — a binding,
a parameter or a builtin — the error names it, as above. A suggestion
is offered only when at most a third of the name is wrong (swapping two
neighbours counts as one slip), or when one of the two names starts the
other (`lenght` finds `len`); names under three characters get none. A key that a
map does not hold is treated the same way, so a misspelled member of
an imported module is named both by `--check` and at runtime.

An error is reported against the file and line that raised it — for
one raised inside a function an imported module defines, that
module's own file (for an embedded stdlib module, its `lib/...`
path) — followed by a `note: in NAME, called from FILE:LINE:COL`
line for every call it unwound through, innermost first. A function
is named after the binding it was defined as; one that never had a
name reads `an anonymous function`. A trace longer than ten frames
keeps four at each end and replaces the rest with `note: ... N more
frames`, so runaway recursion cannot bury the message.

The interpreter is strict on purpose: no truthiness, no implicit
conversions, exact arity, integer overflow checks, missing map keys and
out-of-bounds indices error immediately.

To recover from an expected failure, wrap the risky code in a function
and hand it to `try`; raise your own errors with `fail`:

```ting
let r = try(fn() { return int(input()); });
if has(r, "err") { print("not a number:", r["err"]); }
```

A caught failure carries what the diagnostic would have printed:
`"err"` is the message, `"at"` is a map of the `"file"`, `"line"`
and `"col"` it was raised at, and `"trace"` is the list of calls it
came out of, innermost first — each frame those same three fields
plus `"fn"`, the function's name or `nil` for one that has none. The
trace always holds at least the call `try` itself made.

## Tooling

The `ting` binary is the whole toolchain — no separate installs:

- `ting --fmt <paths...>` reformats in place; `--fmt-check` exits 1
  if anything would change (use it in CI); `--fmt --diff` prints the
  changed lines instead of writing. Directories recurse. The formatter is
  idempotent, never alters program meaning, and keeps the file's line
  endings (a CRLF file stays CRLF). Over several files every one is
  processed — a file that cannot be read, does not lex or cannot be
  written is reported and the run goes on — and the run ends with a
  summary line (reformatted / unchanged / failed, or "would change"
  under `--fmt-check`); exit 1 if anything failed or would change.
  `--fmt-check` and `--fmt --diff` take `--watch` (below); `--fmt`
  itself does not, since rewriting a file would set the watch off.
- `ting --check <paths...>` reports lexer, parser, and compiler
  diagnostics without running anything — built for pre-commit hooks.
  Directories recurse, and files reached through `import("...")` of a
  local path are checked too, each once under its own path.
  Clean files may still get warnings (a statement that can never run,
  after a `return`, `break` or `continue` in the same block; a map
  literal that gives the
  same string key twice, where the last one silently wins; a call
  whose argument count
  cannot match a function bound once at the top level and never
  rebound or shadowed; a name that is bound nowhere
  the checker can see — not a parameter, not a `let` in an enclosing
  block, not a builtin — with the nearest name in scope suggested;
  an imported stdlib module
  indexed with a name it does not export; a top-level binding that
  is never used — prefix the name with `_` to opt out; a file made
  only of bindings is a module and exempt; a `let` inside a block
  that nothing in the block uses, same opt-out; a function parameter
  its body never names, same opt-out; a binding or parameter named
  after a builtin, which hides it); warnings never change the exit
  status unless `--strict` is given, which makes any warning exit 1
  for hooks and CI that want them enforced. `--watch` (below) checks
  again on every change.
- `ting --test <paths...>` runs each file (directories recurse,
  sorted; `--filter SUBSTR` keeps only matching paths; `--tap`
  emits Test Anything Protocol output for CI consumers; `-j N` runs
  up to N files at once with the output kept in order; `--slow N`
  lists the N slowest files after the summary; `--fail-fast` stops
  after the first failing file and counts the rest as skipped) in its own
  process and
  prints `ok` or `FAIL` per file (with the diagnostic under a
  failure) and a summary; exit 1 if anything failed. Pair it with
  `lib/test.ting` or plain `assert` calls. Each line says how much
  the file verified — `ok tests/list.ting (12 checks)`, one check
  per `assert` — the summary totals them, and a file that passed
  while checking nothing is named there, since it proves nothing.
  `--watch` (below) re-runs the suite on every change.
- `--watch` turns `--test`, `--check` and `--fmt-check` into
  something that stays open: the pass runs, and then runs again
  every time one of the watched files changes, is added or is
  deleted. A rule line separates the runs and names the cause:

  ```
  -- run 1 ------------------------------------------------------------
  ok   tests/list.ting (2 checks)
  1 passed, 0 failed, 2 checks
  -- run 2: tests/map.ting added --------------------------------------
  ok   tests/list.ting (2 checks)
  ok   tests/map.ting (1 check)
  2 passed, 0 failed, 3 checks
  ```

  (The rules are eighty columns wide, trimmed here to fit the page;
  a cause longer than that keeps its name and runs past.)

  The paths named on the command line are expanded again before
  every poll, so a file added to a watched directory joins the next
  run and one deleted leaves it. Watching is a poll of modification
  times and lengths, a fifth of a second apart — no dependency, no
  platform-specific API. Only Ctrl-C ends it, so the exit status of
  a watched run is nobody's answer; under `--tap` the rule is a
  comment, so the plan still parses.
- `ting --profile SCRIPT` runs the script and then prints, on
  stderr so the program's own output is untouched, how much each
  function did: the number of calls, the time spent in its own body,
  and where it came from. The measure is self time, not total — the
  callees' time is subtracted — so a recursive function is credited
  once rather than once per level, and a function that only
  delegates ranks by what it kept for itself. Builtins are in the
  table too, marked `a builtin` instead of a file and line, since a
  program can spend its time inside `sort` as easily as inside its
  own loops. Rows are slowest first, ties broken by call count and
  then by position, so the same program reports the same way twice;
  only the busiest twenty are printed, and a longer table ends with
  `... N more functions`. Both engines count the same, and a run
  that fails still reports what it managed. Without the flag nothing
  is measured and nothing is printed.
- All three accept `-` for stdin; `ting --fmt -` is a filter that
  writes the formatted source to stdout, for editor integrations.
- `ting --doc NAME` prints what the REPL's `:doc` would: a builtin's
  signature and doc line, or a stdlib function's signature, module
  and comment. A module name (`list` or `lib/list.ting`) lists that
  module's members, one line each, and so does the path of one of
  your own `.ting` files (its top-level functions with the `#`
  comments above them); no name at all lists every builtin and every
  stdlib function. Several names are allowed (`ting --doc len median
  slug`): the entries are printed in the order asked, separated by a
  blank line. Exit 1 for an unknown name — the others are still
  printed, and a name close to a documented one is suggested.
- `ting --lsp` speaks the Language Server Protocol on stdio:
  diagnostics as you type (syntax errors; an error on an `import` of
  a local file that has one, with the module's position; and warnings
  for a name bound nowhere, for a call that cannot match the function
  it names, for a duplicate key in a map literal, for code that can
  never run, for an imported
  stdlib module indexed
  with a name it does not export, for unused bindings, top-level or
  local, and unused parameters, and for a name that shadows a
  builtin),
  hover docs for every builtin (and for imported stdlib functions,
  and the file's own functions with the `#` comment above them),
  completion
  (builtins, keywords, the document's own names, and the functions of
  any stdlib module it imports),
  whole-document formatting, an outline of top-level bindings
  (document symbols), go-to-definition for them, token-level
  find-references, document highlights of the symbol under the
  cursor (binding sites as writes), and rename across every open
  file with a prepare step that declines keywords and builtins,
  signature
  help inside calls of builtins, stdlib functions and the file's
  own functions, folding ranges for multi-line braces, workspace symbols
  across open files, document links on `import(...)` paths that
  exist on disk, and quickfixes that correct a misspelt stdlib member
  or a name bound nowhere to the nearest one.

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

- Call depth: derived from the interpreter's stack budget, not fixed
  (see Functions); the `ting` binary allows a few thousand frames.
- Integers: i64 range; overflow raises an error rather than wrapping.
- Shift counts: 0 to 63.
- Floats: IEEE 754 doubles. `1.0 / 0.0` is infinity and `0.0 / 0.0` is
  NaN, as the arithmetic says, but no literal or conversion produces
  either, and `json_str` refuses to encode one.
- Map keys: strings only.
- Cyclic data (`xs[0] = xs;`) prints with `[...]` / `{...}` at the point
  of recursion, `==` compares it by the parts that are finite (two
  cycles that agree everywhere they can be inspected are equal), and
  `json_str` refuses it with an error.
