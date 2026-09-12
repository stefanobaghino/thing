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
ting --coverage paths...     # run each, then report which lines ran
ting --bundle script.ting    # print it and its local modules as one file
ting --bundle s.ting -o one  # write it there instead of to stdout
ting --doc [WORDS...]        # explain, search, list a module, or list all
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
cancels), and forgives a missing final `;`. An echoed value stops at
2000 characters and says how many there were — an expression is
echoed to be read, and `range(100000)` is 688890 characters that
would take the session's scrollback with them. `print(x)` writes the
whole value, in the REPL as in a script, and a program's own output
never passes through that cut. Nine meta-commands:
`:help` lists the nine with what each takes; `:doc NAME` explains
one builtin or stdlib function (module, signature, comment), `:doc
MODULE` lists a module's members and `:doc` alone the whole table of
contents, as `--doc` does; `:vars` lists the session's own bindings
with each one's value, a line each and cut to fit;
`:load <file>` evaluates a file in the current session so its
bindings stay available; `:time EXPR` evaluates a one-line chunk and
prints the elapsed milliseconds; `:fmt` reprints the last evaluated
chunk as the formatter would write it; `:history` lists every chunk
that ran without error, numbered; `:save <file>` writes those chunks
as a script that replays the session; and `:clear` resets the
session and the transcript. A colon that opens a chunk is the
REPL's to answer, not the parser's: a name it does not have names the
nearest of the nine, and one of the nine given the wrong thing says
what it takes. Everything the session says about itself is a
parenthetical on its own output — the refusals included: a file it
cannot read or write, a chunk that stops in the middle. What goes to
the error stream is a diagnostic, which carries its own
`file:line:col`; `ting: ...` is the voice the binary uses before
there is a session. It has no built-in line editing or
up-arrow recall (zero dependencies); wrap it with
[rlwrap](https://github.com/hanslub42/rlwrap) — `rlwrap ting` — for
both.

## Source form

- UTF-8 text. Whitespace is insignificant except as a token separator.
- Comments run from `#` to the end of the line. There is no `//` and
  no `/* */`; writing either says so in the error.
- Words other languages use for the same job are not keywords here,
  and writing one says what ting writes instead: `else if` rather than
  `elif`, `fn` rather than `def` or `function`, `let` rather than
  `var` or `const`, `fn(x) { return x; }` rather than an arrow, `&&`,
  `||` and `!` rather than `and`, `or` and `not`, and `nil`, `true`
  and `false` rather than `null`, `None`, `True` or `False`. None of
  them is reserved, so a program may still use one as a name.
- Strings are written with double quotes only. A `'` or a backtick
  says so, and there are no template literals; text is built with
  `format`. There is no `.`: a call is `f(x)` and a map key is
  `m["key"]`, and writing `s.len()` or `m.a` says which — with the
  map spelling `s["len"](...)` beside it, since a module is a map and
  that is how the stdlib is called.
- `in` belongs to a `for` header and nowhere else. Membership is a
  call: `has(m, k)` for a map key, `contains(xs, v)` for a list, and
  writing `k in m` says both, named after the two words either side.
- `if` is a statement, never a value. A name that depends on a
  condition is assigned in both branches, and the three borrowed
  spellings — `c ? a : b`, an `if` where a value belongs, and
  Python's `a if c else b` — each say so. The `{` after a condition
  is what tells a real `if` statement from a forgotten `;` in front
  of one.
- There are no type annotations. A parameter, a return and a `let`
  are names alone, and writing a type after any of them — with a `:`
  or with a `->` — says so; `type(v)` is how a program asks what a
  value is.
- There is no `try`/`catch`/`finally` statement and no `throw`.
  `try(fn() { ... })` is a builtin that hands back a map with `ok` or
  `err`, `fail(msg)` raises, and what follows the call runs either
  way; writing any of the borrowed forms says so.
- There are no comprehensions. `map` and `filter` are builtins, and
  `[f(x) for x in xs]` — in a list or a map literal, with or without
  a guard — says which of them to reach for, naming the iterable when
  it is a single name.
- There is no `++` or `--`, and no C `for` header. Counting up is
  `i += 1`, and a counted loop is `for i in range(n)`; writing either
  borrowed form says so. `--i` is the exception that parses: it
  negates twice, which is what it means here.
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

### Memory

A value is freed as soon as the last thing referring to it lets go.
There is no garbage collector and so no pause: the memory a list, map
or closure holds comes back at the statement that drops it, not at
some later moment chosen for you.

The exception is data that refers to itself, directly or around a
loop of containers. Freeing works by counting references, and a cycle
counts itself, so it is never reclaimed:

```ting
let xs = [];
push(xs, xs);   # xs refers to itself: not freed while the program runs
xs[0] = nil;    # breaking the loop frees it
```

A program that builds a cycle once pays nothing worth measuring; one
that builds a cycle per iteration of a long-running loop grows
without bound, and breaking the link — assigning over it, or `pop`ing
it — is the whole remedy.

A function keeps the scope it was defined in for as long as the
function itself lives, which is how it sees the names around it. That
scope goes when the function does — a helper defined inside a call
and left there goes when the call returns, even if it calls itself,
and one that escapes goes when the last reference to it does.

The exception is again a cycle, and it takes a function that is
reachable BY NAME from inside itself: a recursive `fn`, or a pair
that call each other, that also escapes the call where it was
defined. The scope has to keep the name so the call can be made, and
the function keeps the scope, so the two hold each other up for the
life of the process.

## Operators

Tightest first; binary operators associate left.

| Precedence | Operators                    | Operand rules                                 |
|-----------|-------------------------------|-----------------------------------------------|
| postfix   | `f(args)`, `x[i]`             | calls and indexing chain freely               |
| unary     | `-`, `!`, `~`                 | `-` on numbers, `!` on bools, `~` on ints     |
| factor    | `*`, `/`, `%`                 | numbers; int `/` truncates; `/ 0` and `% 0` on ints error |
| term      | `+`, `-`                      | `+` also concatenates strings and lists       |
| shift     | `<<`, `>>`                    | ints; count 0 to 63; `>>` keeps the sign      |
| bit and   | `&`                           | ints                                          |
| bit xor   | `^`                           | ints                                          |
| bit or    | `\|`                          | ints                                          |
| compare   | `<`, `<=`, `>`, `>=`          | numbers (mixed ok), strings, and lists element by element |
| equality  | `==`, `!=`                    | any values; `1 == 1.0` is true                |
| and       | `&&`                          | bools; short-circuits                         |
| or        | `\|\|`                        | bools; short-circuits                         |

Mixed int/float arithmetic promotes to float. There is no implicit
conversion anywhere else: `1 + "x"` is a type error, `if 1 { }` is a
type error.

Dividing by zero therefore answers in two different ways, and which
one you get is decided by the operands, not by the zero. Two ints
error — `1 / 0` and `1 % 0` both raise `division by zero`. If either
side is a float the result is a float, and floats follow IEEE 754:
`1.0 / 0.0` is `inf`, `-1.0 / 0.0` is `-inf`, and `0.0 / 0.0` and
`1.0 % 0.0` are `NaN`. Mixing the two promotes first, so `1 / 0.0` is
`inf` and not an error.

Those values then travel: `str` prints them (`inf`, `-inf`, `NaN`),
and they are refused where nothing sensible could be written —
`int(1.0 / 0.0)` and `json_str(1.0 / 0.0)` both error. `NaN` is not
equal to itself, as IEEE requires, so `x == x` is `false` for it and
a `NaN` in a list makes that list unequal to a copy of itself.

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
  `has(m, "key")` first, or read it with `get(m, "key", default)`.
  `m["key"] = v` adds or replaces in place, and `pop(m, "key")` takes
  the key out and hands back what it held. Assigning `nil` is not the
  same thing: it stores `nil` under a key that is still there, so `len`
  and `has` both still count it.

`get` covers all three: it takes the index or key and the value to use
when it is absent, so a read that may miss needs no branch around it. An
index or key of the wrong type for the base is still an error.

## Statements

```ting
# not a program: each line is a form on its own, not a sequence to run
let x = 1;          # define (or shadow) in the current scope
x = 2;              # rebind the nearest existing x; undefined name errors
xs[0] = 9;          # write a list slot / insert or update a map key
x += 1;             # also -=, *=, /=, %=: read, apply, write back
xs[i] += 1;         # base and subscript are evaluated once, not twice
let [a, b] = pair;  # take a list apart by position into names
let {code, out} = r;  # take a map apart by key into names
{ let y = 1; }      # block: introduces a scope; y does not leak
if c { } else if d { } else { }
while c { }
for x in xs { }     # iterate a list, a string (chars), or a map (keys)
for [k, v] in ps { }  # taking each element apart the same way
break;              # exit the innermost loop
continue;           # next iteration of the innermost loop
fn add(a, b) { return a + b; }
return expr;        # only inside a function; bare `return;` yields nil
expr;               # expression statement (e.g. a call)
```

`if`/`while`/`for` require braces and take no parentheses around the
condition. Assignment is a statement, not an expression (`a = b = c` and
`1 = 2` are parse errors).

The compound forms `+=`, `-=`, `*=`, `/=` and `%=` apply the binary
operator of the same name, so they do whatever it does — `s += "b"`
concatenates. The right-hand side is a whole expression, taken before
the operator is applied: `n *= 3 + 4` multiplies by 7. On an indexed
target the base and the subscript are evaluated once and used for both
the read and the write, which `m[k()] = m[k()] + 1` cannot promise. The
target must already exist, and reading comes first, so both a missing
name and a missing key are errors. `m[k] = get(m, k, 0) + 1` is the way
to write a tally, since the first sighting has nothing to add to.

Growing a string or a list with `+` costs what was added, not what was
already there. `s += piece` and `s = s + piece` are the same statement
and cost the same: when nothing else holds the value, it is extended
where it lies. When something else does hold it — a second name, a
list or map it sits in, a closure that captured it, a snapshot pushed
onto another list — it is copied, which is what keeps the two apart.
So a loop that appends is linear, and a loop that also keeps every
intermediate value is not, because it cannot be.

Nothing else costs the text. Binding a second name to a string,
passing it to a function, putting it in a list or a map, returning it:
all of these share the text rather than copy it, whatever its length.
Only the write pays, and only when someone else is still holding what
it would overwrite.

A call on the right-hand side does not change this. `s += str(n)`
costs what `s += piece` costs, even where some function names `s` and
could therefore assign it from inside that call. The old value is read
before the call runs, as it must be, and the name is asked to let go
of it afterwards — which it does only if the call left it alone. A
call that does reassign the name behaves as it always did: the value
read before it ran is the one added to, and the result overwrites.

`len` on a string counts characters. It walks the string the first
time and remembers the answer, so a loop that asks repeatedly — `while
len(s) < width` — pays for one walk, not one per turn. Appending keeps
the count rather than dropping it, so building a string in a loop
never re-counts what it built.

Reading a character out of a string, with `s[i]` or `slice`, is
constant time when every character in it is one byte, which the count
already knows. When it is not, the read walks to the character it
wants: correct either way, and the difference only shows on a long
string with something outside ASCII in it.

`for` iterates over a **snapshot** taken when the loop starts, so the
body may mutate the list or map it is iterating. Map iteration visits
keys in sorted order. The loop variable is a fresh binding each
iteration, so closures created in the body capture that iteration's
value. `break`/`continue` apply to the innermost `while`/`for` and may
not cross a function boundary.

### Taking a value apart

A `let` may name several things at once by writing where they sit in
a list. A `for` loop and a parameter take the same brackets, and mean
the same thing by them:

```ting
let counts = {"ant": 2, "bee": 5};
let [first, second] = items(counts);
print(first, second);
for [name, n] in items(counts) { print(name, n); }
fn line([name, n]) { return name + ": " + str(n); }
print(join(map(items(counts), line), ", "));
```

```text
["ant", 2] ["bee", 5]
ant 2
bee 5
ant: 2, bee: 5
```

The value must be a list and its length must match the pattern
exactly: `this pattern takes 2 values, and the list has 3` says so,
and so does `this pattern takes a list apart, and the value is int`.
Nothing is trimmed and nothing is padded, because a pair that arrived
with three things in it is a bug rather than a shape to guess at.
`_` stands where a value is matched and dropped, patterns nest
(`let [a, [b, c]] = ...`), and `let [] = xs;` asserts that `xs` is
empty and binds nothing.

The loop and the parameter forms are the `let` form: the parser
writes the `let` into the front of the body, so the names are the
body's own bindings and everything else about a loop or a call is
unchanged. A pattern parameter counts as one argument, takes no
default, and may not repeat a name another parameter already binds —
`fn f(k, [k, v])` is refused. What it prints in a signature or a
trace is the pattern as it was written: `f([k, v], n)`.

A map is taken apart by key rather than by position, and the pattern
is written the way the literal it matches is: a bare name is the key
of that name, and `"key": pattern` spells a key out and nests.

```ting
let r = {"code": 0, "out": "hi there", "err": ""};
let {code, out} = r;
print(code, out);
let {"out": text} = r;
print(len(split(text, " ")));
for {name, n} in [{"name": "ada", "n": 1}, {"name": "bo", "n": 2}] {
  print(name, n);
}
fn label({name, n}) { return name + ":" + str(n); }
print(label({"name": "cai", "n": 3, "extra": true}));
```

```text
0 hi there
2
ada 1
bo 2
cai:3
```

EXTRA KEYS ARE FINE — `label` above was handed a third one and said
nothing — and that asymmetry with a list pattern is the design. A
list's length is its shape, so a list of the wrong length is the
wrong value; a map's keys are its contents, and asking three fields
of a ten-field record is the ordinary thing to do. What is still a
mistake is asking for what is not there: `this pattern asks for the
key "nope", and the map has no such key`, and `this pattern takes a
map apart, and the value is list` for a value of the wrong kind. A
key may be asked for once per pattern; a repeat is a parse error.

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

A parameter may carry a default: `fn greet(name, greeting = "hello")`
can be called with one argument or two. Parameters with defaults come
last — a plain parameter after one that has a default is a parse
error, since there would be no way to reach it.

```ting
fn span(from, to = from + 10, size = to - from) { return [from, to, size]; }
print(span(1));      # [1, 11, 10]
print(span(1, 4));   # [1, 4, 3]
```

A default is an expression, not a stored value: it is evaluated at
every call that leaves the parameter out, in the callee's own scope
and left to right, so a later default can name an earlier parameter
(whether that one was passed in or defaulted too) and `fn f(xs = [])`
gets a fresh list each time rather than one shared list.

The last parameter may instead be written `...rest`, and then it
binds a list of every argument the fixed parameters did not take —
an empty list when there were none. A rest parameter takes no
default, and nothing may follow it.

```ting
fn log(prefix, ...rest) { print(prefix, ...rest); }
log("note:", 1, [2]);   # note: 1 [2]
```

`f(...xs)` goes the other way: the list `xs` is spread into the call
as arguments. A spread may sit anywhere in an argument list, before
or after plain arguments and alongside other spreads, and spreading
anything but a list is an error naming the type. It is an argument
and nothing else — `...` does not parse outside a call. Together the
two make forwarding possible: what a rest parameter collects, a
spread passes on, which is how a ting function can wrap a variadic
builtin like `format`.

Calls check arity: exactly, or against a range when there are
defaults (`f expects 1 to 3 arguments, got 4`), or against a floor
when there is a rest parameter (`f expects at least 1 argument, got
0`). A spread is counted after the list is unpacked, so the arity
error names what the list actually held. Falling off the end of
a function returns nil. Recursion is capped, and the cap is derived from the host stack
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
| `pop(xs)`, `pop(m, k)` | takes the last element out of a list, or key `k` out of a map, and returns it; an empty list or a missing key errors |
| `keys(m)`      | the map's keys as a sorted list                             |
| `values(m)`    | the map's values, in that same key order                    |
| `items(m)`     | the map's entries as `[key, value]` pairs, in that same order — what `for [k, v] in items(m)` walks |
| `has(m, k)`    | whether string key `k` is present                           |
| `get(x, k, default)` | `x[k]` where it is present, otherwise `default`; reads a map by key and a list or string by index (negatives count from the end), and never errors on absence. Indexing a type that cannot take that key still errors |
| `str(v)`       | the value rendered as a string                              |
| `int(v)`       | from int/float (truncates)/numeric string; a string is read the way a literal is (`0xff`, `0b1010`, `1_000`, a leading sign); else error |
| `float(v)`     | from int/float/numeric string; a string that would be infinite or is not a number errors |
| `hex(n)` / `bin(n)` | an int as a `0x` or `0b` literal, sign kept (`hex(-255)` is `-0xff`); `int` reads them back |
| `type(v)`      | the type name as a string, e.g. `"list"`                    |
| `range(hi)` / `range(lo, hi)` / `range(lo, hi, step)` | list of ints, half-open; `step` may be negative, never 0. `for x in range(...)` counts through the bounds rather than building that list, so a loop costs nothing per element — but only when `range` still means this builtin |
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
| `input()` / `input("lossy")` | one line from stdin without the newline; `nil` at end of input. A line that is not text fails; `"lossy"` reads it anyway, replacing each bad byte |
| `read_file(path)` / `read_file(path, "lossy")` | the file's entire contents as a string; `"-"` reads stdin to EOF. Bytes that are not text fail, naming where; `"lossy"` reads them anyway, replacing each bad byte |
| `write_file(path, s)` / `write_file(path, s, "append")` | writes (or overwrites) the file; `"append"` adds to the end |
| `each_line(path, f)` / `each_line(path, f, "lossy")` | reads the file one line at a time, calling `f(line)` for each — newline gone, and the carriage return before it, as `input()` gives them. Only the current line is held. `"-"` is stdin; returning `false` from `f` stops the read; answers how many lines `f` was given. A line that is not text fails, naming which; `"lossy"` hands it over with each bad byte replaced |
| `list_dir(path)` | the names in the directory, sorted; not the paths, and not recursive; a path that is not a readable directory errors |
| `exists(path)` | whether anything is at that path; `is_dir(path)` whether that thing is a directory. Questions, so an absent or unreadable path is `false`, never an error |
| `is_dir(path)` | see `exists` |
| `stat(path)` | what a file is besides its name: a map of `size` (bytes), `modified` (ms since the epoch, the clock `time_ms()` reads) and `kind` (`"file"`, `"dir"` or `"other"`). `nil` when nothing readable is there, so it is a question like `exists` |
| `make_dir(path)` | creates the directory and any missing parents; a directory that is already there is not an error |
| `remove_file(path)` | deletes the file; absent, or a directory, errors |
| `remove_dir(path)` | deletes an empty directory; one with anything in it errors. `lib/fs.ting`'s `remove_tree` composes the recursive version |
| `rename(from, to)` | gives a file or directory another name, which is what a move is: nothing is copied, so the size does not matter and the modification time comes through untouched. An existing target is replaced. Errors when the two paths are on different filesystems |
| `copy_file(from, to)` | copies a file's bytes, whatever they are, without holding them in memory, and gives the copy the original's permission bits and modification time. An existing target is overwritten; a directory, or a target that is the same file as the source, errors |
| `sort(xs)`     | a fresh sorted list; all numbers, all strings or all lists, else error |
| `sort_by(xs, f)` | a fresh list sorted by key `f(x)`, stable; keys obey `sort`'s rules, so a list is a compound key |
| `sort_with(xs, cmp)` | a fresh list sorted by a three-way comparator: `cmp(a, b)` negative when `a` comes first, positive when `b` does, `0` for ties, which keep their input order |
| `try(f, ...args)` | calls `f` with the arguments that follow it; `{"ok": result}` on success, and on a runtime error `{"err": message, "at": where it was raised, "trace": the calls it came out of}` |
| `fail(msg)`    | raises a runtime error with the given string message         |
| `map(xs, f)`   | a fresh list of `f(x)` for each element                      |
| `filter(xs, f)` | a fresh list of the elements where `f(x)` is `true` (bool required) |
| `reduce(xs, init, f)` | folds left: `f(f(init, x0), x1)…`                     |
| `min(xs)` / `max(xs)` | smallest / largest element; `sort`'s ordering rules; empty list errors |
| `abs(n)`       | absolute value of an int or float                            |
| `assert(cond)` / `assert(cond, msg)` | error unless `cond` is `true` (bool required); a refused comparison also shows both sides, as `assertion failed: three kilos (9 == 8)` |
| `import(path)` | runs the file once and returns its top-level bindings as a map; see below |
| `format(fmt, ...)` | fills `{}` placeholders left-to-right (`{{`/`}}` for literal braces); placeholder/value count mismatch errors. A placeholder may carry a spec — `{:[[fill]align][width][.places]}`, whose width and places may be `{}` and come from the arguments; see below |
| `json_parse(s)` | JSON text to ting values (object→map, array→list, null→nil); malformed input errors with an offset. A byte order mark at the head of the document is skipped — another program may have written one — but only there |
| `json_str(v)` / `json_str(v, indent)` | ting value to JSON — compact, or pretty with `indent` spaces per level (map keys sorted); functions and non-finite floats error |
| `env(name)`    | the environment variable's value, or `nil` if unset          |
| `exit()` / `exit(code)` | ends the program with that status (default 0); not catchable by `try` |
| `time_ms()`    | milliseconds since the Unix epoch, as an int                 |
| `mono_ms()`    | milliseconds since the process started, as a float, from a clock that only moves forward — subtract two readings to time something |
| `local_zone()` / `local_zone(ms)` | the local zone at that instant, now by default: a map of `offset` (milliseconds east of UTC), `abbr` (what the platform calls that period — `"CEST"` on Unix, `"W. Europe Daylight Time"` on Windows) and `dst`. `nil` where the platform keeps nothing to read, which a script can tell from a real zero |
| `sleep_ms(ms)` | pauses for that many milliseconds, flushing output first; a negative count, or anything but an int, errors |
| `random()`     | a float in `[0, 1)`, drawn from the 53 bits a double can hold |
| `random_int(lo, hi)` | an int in `[lo, hi)`, half-open like `range`; an empty span errors |
| `seed(n)`      | restarts the generator at `n`; unseeded, it starts from the clock |
| `run(cmd)` / `run(cmd, args)` / `run(cmd, args, stdin)` / `run(cmd, args, options)` | runs a program with that argv (no shell) and waits; a map of `code`, `out`, `err` and `signal`. The child reads `stdin` there and reads nothing without it; a map in that place is options instead — `"stdin"` is the same text, `"dir"` is the directory to run the child in, `"env"` is a map of variables the child gets on top of the ones it inherits, a name bound to `nil` being one it will not have, and `"show": true` lets the child write to ting's own stdout and stderr as it goes, for the build or test run whose output IS the feedback — a shown child has no `out` or `err` in the map at all, since an empty string would say it said nothing. Between them they are the `cd`, the `VAR=value` and the plain command a script would otherwise spawn a shell for. An option nothing knows is an error, and a `"dir"` that is not a directory is its own error rather than a spawn failure that reads like a missing program. `out` and `err` are decoded lossily, where every reader that takes text *into* ting refuses instead — *Bytes that are not text* below says what each door does and why. A program that cannot be started errors; `code` is `nil` when a signal ended it, and `signal` is that number where the platform has signals — `nil` everywhere else, including after a normal exit |
| `eprint(...)`  | like `print`, but to stderr, after flushing stdout so the two stay in order |
| `cwd()`        | the working directory, as a string                           |
| `re_test(s, pattern)` | whether the pattern matches anywhere in the string |
| `re_find(s, pattern)` | the leftmost match as a map of `start`, `end`, `text` and `groups` (a list, `nil` where a group took no part), or `nil`. Positions count characters, as `find` and `slice` do |
| `re_find_all(s, pattern)` | every non-overlapping match, left to right, as a list of those maps |
| `re_replace(s, pattern, repl)` | every match replaced; `$0` is the whole match, `$1` to `$9` its groups, `$$` a literal `$`. A reference to a group the pattern does not have errors |
| `re_split(s, pattern)` | the string cut at every match; leading and trailing empty pieces are kept, as `split` keeps them |
| `display_width(s)` | how many terminal COLUMNS the string takes, where `len` counts characters: an East Asian wide or fullwidth character (an ideograph, a fullwidth digit, most emoji) counts two, a combining mark or a control counts none, everything else one. What lines text up in a terminal — a padded column, a caret under a diagnostic — has to count these, and ting's own tools now do. Character by character, which is where a terminal's arithmetic stops too: an emoji sequence joined by zero-width joiners counts as its parts |
| `compare(a, b)` | which of two values comes first in ting's order: `-1` for `a`, `1` for `b`, `0` for a tie. The three-way answer `<` gives one bit of, so a `sort_with` comparator over several fields is one line each. `nil` when a NaN leaves the pair unordered, and the error `<` gives where the two kinds have no order between them |
| `fingerprint(v)` | a string two values share exactly when `==` says they are equal, so a map lookup can stand in for a scan; `nil` where equality cannot be a key — a function (compared by identity, not by what it says), a NaN (equal to nothing, itself included), a number past 2^53 (where `1 == 1.0` numeric equality stops being transitive between ints and floats), or a value that contains itself. The text itself is not promised: compare fingerprints, do not read them |

### Format specs

A `format` placeholder is `{}` on its own, or `{:spec}` where the spec
lays the value out in a column:

    {:[[fill]align][0][width][.places]}

`align` is `<` for left, `>` for right and `^` for centred; `fill` is
any single character placed before it, and defaults to a space;
`width` is the number of COLUMNS the result should occupy, counted
the way a terminal counts them: `display_width`, not `len`, so an
ideograph takes two and a combining mark none. A value already that
wide is written unchanged — a spec pads, it never truncates.

    format("{:>5}", 42)        # "   42"
    format("{:<8}|", "hi")     # "hi      |"
    format("{:^9}", "hi")      # "   hi    "
    format("{:0>2}", 7)        # "07"
    format("{:.^10}", "mid")   # "...mid...."

A zero written in front of the width fills with zeroes, as it does in
Rust, Python, C and Go, and it is sign-aware: the zeroes go after a
minus sign rather than in front of it. An alignment beside it says
where they go instead, and a fill character written out wins outright
— `{:.^05}` keeps its dots and reads the zero as part of the width.

    format("{:05}", 42)        # "00042"
    format("{:05}", -42)       # "-0042"
    format("{:07.2}", -3.5)    # "-003.50"
    format("{:>05}", -42)      # "00-42", as {:0>5} spells out
    format("{:.^05}", 7)       # "..7.."

A width with no alignment puts numbers to the right and everything
else to the left, which is what a column of figures wants:

    format("{:5}", 42)         # "   42"
    format("{:5}", "ab")       # "ab   "

When a centred value cannot be centred exactly, the extra column
goes on the right, as `lib/string.ting`'s `center` puts it. `{}` and
`{:}` mean the same thing. Widths are capped at 100000 columns, so
a mistyped spec reports an error rather than exhausting memory.

`.places` writes a number with exactly that many digits after the
point, and needs a number to write — a spec with decimal places on a
string or a list is an error:

    format("{:.2}", 100.0 / 3.0)   # "33.33"
    format("{:.2}", 3)             # "3.00"
    format("{:.0}", 2.5)           # "3"
    format("{:>8.2}", 1234.5)      # " 1234.50"

Halves go away from zero, which is what `lib/math.ting`'s `round`
promises — `{:.2}` of 0.125 is `0.13`. An int is written digit for
digit rather than through a float, so a value past a float's exact
range keeps every digit it had. Places are capped at 100.

Rounding a number and writing one are different jobs, and only the
second is a spec's: `{:.2}` always writes two digits after the point,
where `round(x * 100) / 100` gives back a number that prints as `17.3`
when the second digit is a zero. Reach for `round` when the VALUE
should change, and for a spec when only the writing should.

A width or a number of places written `{}` is taken from the argument
list instead of the template. The value comes first, then the spec's
holes left to right, so a column measured from the data lines up
without going through `pad_right`:

    let w = 7;
    format("[{:<{}}]", "ab", w)          # "[ab     ]"
    format("[{:>{}.{}}]", 3.14159, 9, 3) # "[    3.142]"

A hole reads a non-negative int, and is capped exactly as a written
number is. Because `{` in a spec opens a hole, a spec cannot pad a
column with braces.

### Patterns

`re_test`, `re_find`, `re_find_all`, `re_replace` and `re_split` take
a pattern as an ordinary string, so a backslash in a pattern is
written twice: `"\\d+"` is the pattern `\d+`.

The syntax:

| Piece | Means |
|-------|-------|
| `abc` | those characters, in that order |
| `.` | any character except a newline |
| `[abc]`, `[a-z]`, `[^a-z]` | a character in the set, a range, or one outside it |
| `\d` `\w` `\s` | a digit, a word character (letter, digit or `_`), a space; `\D` `\W` `\S` for the opposites, inside brackets or out |
| `\n` `\t` `\r` `\.` `\\` | a newline, tab, return, and any punctuation as itself |
| `^` `$` | the start and the end of the string |
| `(...)` | a capturing group; `(?:...)` groups without capturing |
| `a\|b` | either side, preferring the left |
| `*` `+` `?` | none or more, one or more, none or one |
| `{n}` `{n,}` `{n,m}` | exactly, at least, or between that many |
| `*?` `+?` `??` `{n,m}?` | the same, preferring the shorter match |

What it deliberately leaves out: backreferences, lookaround, named
groups, and flags. The engine runs every alternative in lockstep
rather than backtracking, which is what makes matching linear in the
length of the string — `(a+)+b` against a long line of `a`s answers
at once instead of hanging — and backreferences cannot be had that
way. A pattern that asks for one of them is refused by name —
`re_find: negative lookbehind is not supported at 4` — rather than
read as something else.

The rest of the semantics:

- The search is leftmost, and alternation prefers its earlier branch,
  so `re_find("foobar", "foo|foobar")` matches `foo`.
- Positions count characters, not bytes, as `len`, `slice` and `find`
  do.
- A group that took no part in the match is `nil`, not `""`.
- An empty match cannot advance a scan, so `re_find_all` and
  `re_split` step one character past one: `re_split(s, "")` cuts a
  string into its characters.
- A counted repetition may ask for at most 1000 copies, and a pattern
  may compile to at most 100000 instructions; `(a{1000}){1000}` is
  refused for the second reason.
- An invalid pattern errors, naming the builtin and where in the
  pattern the trouble is: `re_find: unclosed ( at 2`.
- Compiled patterns are cached, so a match inside a loop compiles
  once.

### Files and directories

`read_file`, `each_line` and `write_file` handle a file's contents;
`list_dir`,
`exists`, `is_dir`, `stat`, `make_dir`, `rename` and `copy_file`
handle the tree around it.
The split between them is deliberate:

- `exists`, `is_dir` and `stat` are **questions**. An absent,
  unreadable or otherwise awkward path answers `false` — `nil` for
  `stat` — and none of them ever raises, so they can be used in an
  `if` without a `try` around them.
- `list_dir` is a **demand**, and errors when the path is not a
  readable directory. Asking what is inside something that is not
  there is a mistake, and an empty list would hide it. A name that
  is not valid UTF-8 fails the whole listing rather than being
  dropped or lossily converted, since a lossy name would not reopen
  the file it came from.
- `make_dir` creates missing parents, and a directory that already
  exists is success rather than an error: the useful postcondition
  is that the directory is there, not that this call made it.

`stat` answers the questions a name cannot. `size` is **bytes**,
which `len(read_file(p))` is not — that counts characters, so a file
with any UTF-8 in it reports short, and a file that is not text
`local_zone` reads the zone the machine itself keeps, because there
is no other honest source for it. On Unix that is the TZif file at
`/etc/localtime`, or the one `TZ` names under `/usr/share/zoneinfo`
when it is set. Windows keeps its zones in the registry instead, in a
shape no file reader would recognise, so there it asks the system:
the year's rules from `GetTimeZoneInformationForYear`, applied by
`SystemTimeToTzSpecificLocalTime`. `TZ` is not consulted on Windows,
because the Windows clock does not consult it either.

It answers for an instant rather than for the present, since a report
over last winter's timestamps needs last winter's offset, and both
platforms keep the changes a zone has been through — including the
years before a country kept summer time, and, on Unix, the local mean
time that predates zones altogether, which is why an offset is a
whole number of seconds but not always of minutes.

How far back that reaches differs. A zone file records a century of
changes; Windows keeps per-year rules going back a couple of decades
and applies the earliest it has to anything older. So the two
platforms agree about recent years and can disagree about distant
ones: 1980 in Zurich is `+01:00` from a zone file, which knows
Switzerland kept no summer time until 1981, and `+02:00` in July from
Windows, which has no entry that says so.

`abbr` is what the platform calls that period, and the two platforms
call it different things. A zone file holds a real abbreviation
(`"CEST"`), or, for a zone that never had one, the offset itself
(`"+1245"` for Chatham). Windows has no abbreviations at all: it
holds full names, in the language the system is installed in, so
`abbr` there reads `"W. Europe Daylight Time"` — or
`"Mitteleuropäische Sommerzeit"` on a German machine. A program that
compares `abbr` against a fixed string is therefore wrong on some
machine somewhere; a program that prints it is right everywhere. The
alternative was to throw away the only naming Windows has and hand
back a number `offset` already holds, which would have made the two
platforms look alike by making one of them say less.

It answers `nil` where there is nothing to read: the browser
playground, and a `TZ` that carries a rule rather than a name. That
is a refusal rather than a guess, because a caller told UTC cannot
tell it apart from a caller told the truth. On Unix the instants
after the last transition a file records take the last offset in it,
since the POSIX rule in the file's footer is not read.

cannot be read at all. `modified` is on the same clock as `time_ms()`
and signed the same way, so `time_ms() - stat(p)["modified"]` is an
age in milliseconds and a file older than 1970 counts backwards
rather than wrapping; it is `nil` on a platform that cannot say. A
directory's `size` is whatever the filesystem records for the
directory itself, not the size of what it holds — the number `ls -l`
prints. Symbolic links are followed, as they are for `exists` and
`is_dir`, so a broken link is `nil`.

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

`rename` gives a file another name, and that is what a move is. The
file is not copied, so it costs the same for a line of text and a
video, and — the reason it exists — the modification time comes
through untouched. A script that files things by the day they were
written would otherwise rewrite every date it sorted by, since a
copy is stamped at the moment it is made. A directory moves whole
and an existing target is replaced, both silently, as the system
call and `mv` do.

The one thing `rename` will not do is cross a filesystem. The
operating system refuses there, saying "invalid cross-device link",
which names nothing a script can act on; `mv` quietly copies
instead. ting neither hides the refusal behind an unfamiliar phrase
nor turns a cheap move into an expensive copy without saying so — it
reports that the two paths are on different filesystems, and the
caller chooses what to do about it.

`copy_file` is what the caller chooses. It copies bytes rather than
text, so it moves a photograph that `read_file` cannot even open, and
it streams them instead of holding the file in memory, which
`write_file(t, read_file(s))` cannot avoid. The copy gets the
original's permission bits and its modification time. That last part
is a decision: `cp` needs `-p` for it, but `mv` keeps the date even
when it has to fall back to copying across a filesystem, and a
cross-filesystem move in ting is `copy_file` followed by
`remove_file` — a copy that dropped the date would put back the very
bug these two builtins exist to remove. Where the filesystem cannot
record a time, the bytes still arrive; that is the same platform
limit `stat` reports as a `nil` `modified`.

Two refusals. A directory is not a file to copy, and the recursive
version stays composable ting for the same reason `remove_tree` does.
And a source and target that are **the same file** error rather than
proceeding: the copy underneath opens the target for writing, which
truncates the source it is about to read, and would report a
successful copy of nothing. Sameness is asked of the filesystem, not
of the spelling, so `a` and `./a` are caught and so is a hard link.

What `copy_file` is not is atomic: a failure part way leaves a
partial target, and that is not hidden. A copy that cannot be seen
half-done is a copy to a temporary name followed by a `rename` onto
the target — two lines, in ting, where you can read them.

`read_file` hands back the whole file, which is the right answer
until the file is large. Counting the matching lines in a 147 MB log
that way costs 454 MB of memory: the file once, and then the list of
its 2000001 lines, which weighs more than the file it came from.
`each_line` reads the same file in 9 MB and slightly less time,
because nothing but the current line is ever held — the same nine
megabytes whether the file is a kilobyte or a terabyte.

The line arrives as `input()` gives it, without its newline and
without a carriage return before it, so a file written on Windows
reads like one written here; a last line with no newline after it
still counts. `"-"` is stdin, the name `read_file` already uses, and
it shares the buffer `input()` reads from, so the two compose. `f`
returning `false` stops the read and nothing further is read from
disk, which is what makes "the first ten lines" or "the first line
that matches" cost what they should; every other answer, `nil`
included, carries on.

`lib/fs.ting` asks the four questions people actually ask a file too
big to hold: `count_lines`, `head`, `tail` and `lines_matching`. They
are there so the streaming is not something you have to remember to
do, and because two of them are easy to write badly — `head` has to
stop the read, and `tail` has to hold a window rather than a list.

The move that works either way is `lib/fs.ting`'s `move`: a `rename`
where that succeeds, and where it cannot, the copy and the removal
`mv` falls back to. It lives there rather than in the binary so that
the expensive path is readable, and so the rare case cannot pretend
to be the cheap one.

### Bytes that are not text

Every string in ting is text: a sequence of Unicode characters, with
no bytes type beside it. So every door that takes bytes from outside
has to decide what a byte that is not UTF-8 means, and there are only
three answers in the whole language. Each door picks one on purpose.

**Refuse, and say where.** Anything offered to ting *as text* fails
rather than guessing: `read_file`, `each_line`, `input`, a script
given as a path or as `-`, `import`, the REPL's `:load`, and
`--check`, `--fmt` and `--bundle` over a file or a directory. The
message names the byte and where it is, in the terms that door has:

```ting
# not a program: the three shapes the refusal takes
not UTF-8 text: byte 0xe9 at offset 10 (line 2, byte 3)
not UTF-8 text: byte 0xe9 at byte 3 of line 2
not UTF-8 text: byte 0xe9 at byte 3 of the line
```

A whole file can count both, so it gives the offset and the line.
`each_line` is counting lines anyway, so it names the line and stops
there — the good lines before it were already handed over. `input()`
gives no line number at all, because nobody numbered the stream: the
program has read as many lines as it has read, and a number invented
here would name a different line than the reader's own count.

**Read it anyway, when asked.** The three readers take a mode string
in the shape `write_file` already uses for `"append"`:

```ting
# not a program: each one replaces every bad byte with U+FFFD
read_file(path, "lossy")
each_line(path, f, "lossy")
input("lossy")
```

Each bad byte becomes one replacement character, so a lossy read
substitutes into the line rather than shortening it. Any other mode
string is refused by name — `read_file mode must be the string
"lossy", got "skip"` — since a mode that is silently ignored is a
worse answer than an error.

**Replace, always.** `run()` decodes a child's `out` and `err`
lossily and has no strict form. The asymmetry is deliberate: a file
is offered to ting as text, so mojibake would be a wrong answer,
while a child's output is whatever the child printed, ting has no
bytes to hand back instead, and failing would throw away the exit
code, the stderr and the signal along with it.

Two doors nearby answer differently for reasons of their own.
`list_dir` fails the whole listing on a name that is not UTF-8,
because a lossily converted name would not reopen the file it came
from. `lib/base64.ting` decodes to numbers with `decode_bytes` and
only refuses in `from_bytes`, where the bytes become a string.

Going out, nothing can go wrong: `write_file` writes a ting string,
which is always text, so a file ting wrote always reads back. And
what a byte costs is not what a character costs — `stat(p)["size"]`
counts bytes where `len(read_file(p))` counts characters.

### Modules

`import(path)` loads another ting file, runs it in a fresh global
scope, and returns a map of everything its top level defined:

```ting
# not a program: two files, shown together
# mathutils.ting
fn double(x) { return x * 2; }

# main.ting
let m = import("mathutils.ting");
print(m["double"](21));   # 42
```

"Everything its top level defined" means every `let` and `fn` written
at the top level of the module, and nothing else. The builtins are in
scope inside a module the way they are everywhere, but they are not
exports; a module that wants one in its map has to declare it, and
`let sort = sort;` does exactly that.

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

### There is no set type

A map is the set: keys are strings, and `fingerprint(v)` turns any
value into one that two values share exactly when `==` says they are
equal. That is what `lib/list.ting` builds on — `membership`, `holds`
and `remember` to ask a list about a value without walking it, and
`union`, `intersection` and `difference` on top of those. Written by
hand with `contains`, each of those is a scan per element, which is
quadratic; the module's versions are one lookup per element.

`fingerprint` answers `nil` where equality cannot be a key at all: a
function, a NaN, a number past 2^53, or a value that contains itself.
The module's helpers fall back to the scan for exactly those, so their
answers always agree with `==`.

### What ting puts in order

`<` and its three siblings order numbers among themselves, strings by
code point, and lists element by element: the first difference
decides, and a list that is a prefix of another comes first. Nothing
else has an order — `nil < nil` is an error, and so is comparing a
string with a number — because there is no answer that would mean
anything, and a made-up one would sort silently wrong.

Order therefore goes as deep as `==` does, and agrees with it: `[1,
2]` and `[1, 2.0]` are equal, so neither comes first.

`sort`, `sort_by`, `min` and `max` read that same order, which makes
two everyday things work. A frequency table sorts itself, since
`items(m)` is a list of pairs:

```ting
let m = import("lib/map.ting");
print(sort(m["items"]({"pears": 2, "apples": 5})));
```

```text
[["apples", 5], ["pears", 2]]
```

And a compound key is a list — `sort_by(people, fn(p) { return
[p["last"], p["first"]]; })` sorts by surname, then given name.

For anything else there is `compare(a, b)`, the same order as `-1`,
`0` or `1`, which is what `sort_with` wants. Mixed directions are one
line per field:

```ting
# not a program: `staff` stands for a list the caller already has.
sort_with(staff, fn(x, y) {
  let by_name = compare(x["name"], y["name"]);
  if by_name != 0 { return by_name; }
  return compare(y["age"], x["age"]);
});
```

A refusal inside a list is still a refusal: `sort([[nil], [nil]])`
says it cannot order `nil`. A NaN is unordered wherever it sits —
every comparison against it is `false`, and `compare` answers `nil`
rather than calling that a tie.

Maps have no order. Their keys are a set, and a set has none to read
off; sort `items(m)` when you want one.

## Errors

An unhandled runtime error stops the program with a diagnostic pointing
at the offending source:

```text
script.ting:2:7: error: undefined variable 'totl' (did you mean 'total'?)
 2 | print(totl + 1);
   |       ^^^^
```

When the name is one a stdlib module exports, the error says which
module has it, since the module is inside this binary and an `import`
away:

```text
script.ting:1:7: error: undefined variable 'repeat' (lib/string.ting has it)
 1 | print(repeat("-", 20));
   |       ^^^^^^
```

Where several modules export the name, all of them are named.
Otherwise, when the name you typed is close to one that is in scope —
a binding, a parameter or a builtin — the error names it, as above. A suggestion
is offered only when at most a third of the name is wrong (swapping two
neighbours counts as one slip), or when one of the two names starts the
other (`lenght` finds `len`); names under three characters get none. A
guess is also broken at its underscores and each part asked in turn,
last part first, because a guess carried over from another language
holds the name inside it with a qualifier in front: `to_float` finds
`float`, `array_len` finds `len`, `string_upper` finds `upper`. The guess
may also be a part OF a name — the head of a compound name carries no
information, so the tail is the half that is remembered, and `approx`
finds `check_approx`. That last rule has no length floor, since being
a whole part of a name is identity rather than distance: `eq` finds
`check_eq`, while `er` finds nothing, being no part of anything. How an answer was found ranks before how far
away it is: the whole guess one slip away, then a part that is a name
outright, then the whole guess on a shared start, then a name the
guess is part of, then a part near a name. That is why
`list_sort` finds `sort` rather than `list_dir`. A key that a
map does not hold is treated the same way, so a misspelled member of
an imported module is named both by `--check` and at runtime.

A name the file bound over a builtin gets the same treatment from the
other side: calling it when it no longer holds a function reads `map
is not callable (`args` shadows the builtin of that name)`, the
sentence `--check` prints about the `let` itself.

An error is reported against the file and line that raised it — for
one raised inside a function an imported module defines, that
module's own file (for an embedded stdlib module, its `lib/...`
path) — followed by a
`note: in NAME(args), called from FILE:LINE:COL` line for every call
it unwound through, innermost first. A function is named after the
binding it was defined as; one that never had a name reads
`an anonymous function`.

The arguments are what the body saw: defaults filled in, the rest
list included, and each value written the way it reads inside a list,
so a string keeps its quotes. Three caps keep a diagnostic readable
whatever it is handed: at most four arguments are named and the rest
counted (`and 2 more`), each value is cut to 32 characters, and a
trace longer than ten frames keeps four at each end and replaces the
rest with `note: ... N more frames`. So a failure deep in a fold
says which value it choked on rather than only which functions were
on the way there.

The interpreter is strict on purpose: no truthiness, no implicit
conversions, exact arity, integer overflow checks, missing map keys and
out-of-bounds indices error immediately.

To recover from an expected failure, hand the function to `try` along
with the arguments to call it with; raise your own errors with `fail`:

```ting
let r = try(int, input());
if has(r, "err") { print("not a number:", r["err"]); }
```

A lambda is still the way to guard more than one call, or a piece of
code that is not a call at all — and it is not merely a longer
spelling. What goes inside it is evaluated by `try`; what goes in
`try`'s own argument list is evaluated before `try` runs, so
`try(f, ...xs)` catches nothing if `xs` turns out not to be a list.
Each lambda also adds the frame it is to `"trace"`.

```ting
let r = try(fn() { return json_parse(read_file(path)); });
```

A caught failure carries what the diagnostic would have printed:
`"err"` is the message, `"at"` is a map of the `"file"`, `"line"`
and `"col"` it was raised at, and `"trace"` is the list of calls it
came out of, innermost first — each frame those same three fields
plus `"fn"`, the function's name or `nil` for one that has none, and
`"args"`, a map from parameter name to the value it was given. The
trace always holds at least the call `try` itself made. Every
`"file"` is named the way `--check` names it: an imported module
resolved to an absolute path, and is written back relative to the
directory the command ran in, so a failure a program reports and a
diagnostic a reader sees point at the same name.

The caps above are the diagnostic's, not the data's: `"args"` holds
every parameter and the whole of each value, because a program
reading a failure back wants to look inside what it was given rather
than at 32 characters of it. `lib/err.ting`'s `given` is the short
way to ask for the innermost call's.

## Tooling

The `ting` binary is the whole toolchain — no separate installs:

- `ting --fmt <paths...>` reformats in place; `--fmt-check` exits 1
  if anything would change (use it in CI); `--fmt --diff` prints the
  changed lines instead of writing. Directories recurse. The formatter is
  idempotent, never alters program meaning, and keeps the file's line
  endings (a CRLF file stays CRLF). It works on TOKENS rather than on
  a parsed program, which is why it can tidy a file you are in the
  middle of writing: a file that does not parse is still reformatted,
  as long as it lexes. Nothing is checked on the way through — run
  `--check` for that. Over several files every one is
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
  EVERY syntax error in a file is reported, not the first: after one,
  the parser skips to where a statement can start again — past the
  next `;`, out of the braces the mistake was inside, or up to a
  keyword that opens a statement — and carries on. The errors come in
  line order, no position is reported twice, and a file stops at
  twenty of them, since the later ones in a file that confused the
  parser are usually gone once the first are fixed. A file with a
  syntax error gets ONLY its syntax errors: what parsed is the
  statements around the mistakes, so the compiler and the warnings
  below would be judging a program nobody wrote — a name bound in a
  statement that failed looks bound nowhere. A file that does not
  lex is a different matter: there are no tokens past the bad
  character, so that is one error on its own.
  Clean files may still get warnings (a statement that can never run,
  after a `return`, `break` or `continue` in the same block; a map
  literal that gives the
  same string key twice, where the last one silently wins; a call
  whose argument count
  cannot match the function called, whether that function is a
  builtin, bound once at the top level of this file, or offered by a
  module this file imported once — `st["truncate"]("x")` is counted
  against what lib/string.ting declares and `len()` against what the
  builtin takes, defaults making a range and `...rest` a
  floor; a file that binds the name itself takes it back, since
  `len` is whatever that file made it; a `format` whose template is
  written at the call site, judged the way the run will judge it —
  the braces, the spec, and the arguments the template asks for,
  counting a `{}` inside a spec as one of them; a name that is bound nowhere
  the checker can see — not a parameter, not a `let` in an enclosing
  block, not a builtin — with the stdlib module that exports that
  name, when one does, and the nearest name in scope otherwise;
  an imported module
  indexed with a name it does not export, naming the builtin of that
  name where there is one, then whichever is nearer of the closest
  export and the closest builtin — a tie goes to the module, since
  that is what was indexed — and otherwise what the module DOES have — every name when
  there are eight or fewer, and how many there are plus the `--doc`
  command that prints them when there are more; a top-level binding that
  is never used — prefix the name with `_` to opt out; a file made
  only of bindings is a module and exempt; a `let` inside a block
  that nothing in the block uses, same opt-out; a function parameter
  its body never names, same opt-out; a binding or parameter named
  after a builtin, which hides it; a file that records checks with
  `lib/test.ting` and calls neither `summary()`, which prints them and
  decides the verdict, nor `reset()`, which is what a file that
  arranged its failures on purpose calls instead); warnings never change the exit
  status unless `--strict` is given, which makes any warning exit 1
  for hooks and CI that want them enforced. `--watch` (below) checks
  again on every change.
  The module in both of those is whichever one `import` would run: a
  file beside the script wins over an embedded module of the same
  path, exactly as it does at run time. Either way the checker reads
  the module's own top level, so what it says is what the module
  declares rather than a table kept by hand. A call it cannot be sure
  of is left to the run — a module binding that is reassigned,
  imported twice, shadowed by a parameter or written into answers for
  nothing — and so is a member reached any other way than
  `name["key"](...)`, a spread call, or a member the module
  re-exports from a builtin rather than declaring itself. Writing a
  key into a module map (`m["extra"] = v;`) puts it there rather than
  asking for it, so neither that line nor a later read of it is an
  unknown member.
- `ting --test <paths...>` runs each file (directories recurse,
  sorted; `--filter SUBSTR` keeps only matching paths; `--tap`
  emits Test Anything Protocol output for CI consumers; `-j N` runs
  up to N files at once with the output kept in order; `--slow N`
  lists the N slowest files after the summary; `--fail-fast` stops
  after the first failing file and counts the rest as skipped) in its own
  process and
  prints `ok`, `skip` or `FAIL` per file (with the diagnostic under
  a failure) and a summary; exit 1 if anything failed. Pair it with
  `lib/test.ting` or plain `assert` calls. A file that used
  `lib/test.ting` and left a failed check unprinted fails anyway —
  its helpers record and return, and `summary()` is what prints them,
  so forgetting that line used to report a pass — and so did failing
  a check and then calling `exit(0)`. A file that records
  failures ON PURPOSE, to test the checks themselves, calls
  `reset()` when it has read them. Each line says how much
  the file verified — `ok   tests/list.ting (12 checks)`, one check
  per `assert` — and the summary totals them. A file that ran and
  checked nothing is a `skip`, not a pass: it stands behind none of
  the suite, which is what `--fail-fast`'s skips mean too, so the
  summary counts the two together. In TAP a skip is an `ok` line
  carrying a `# SKIP` directive.
  A failing file's own output is repeated under the `FAIL` line,
  indented, before the error that killed it: what the file printed
  is usually the reason, and it is what `lib/test.ting`'s
  `summary()` prints. A file that passes stays silent, and a flood
  is cut after forty lines, keeping the head and counting the rest.
  The check count survives a failure too — a file that dies partway,
  or one that ends in `exit()`, still reports what it verified
  before it stopped. `--watch` (below) re-runs the suite on every
  change.
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
- `ting --coverage PATHS...` runs each script — directories recurse,
  as they do for the other tools here — and then prints, on stderr,
  which lines ran. One row per file: the share of its statements
  reached, and the line numbers of those that were not, twelve of
  them named before the rest are counted. A statement is the unit,
  so a line holding one is coverable and a blank line, a comment or
  a closing brace is not; a `fn` definition counts as the statement
  that binds it, which runs whether or not the function is ever
  called. Imported modules are counted against their own files, and a
  file reached by several of the scripts is one row. A stdlib module
  that came out of the binary is left out of the table and the total
  and named on a last line instead: the report is about the code its
  reader wrote, and those lines arrived with the interpreter. A
  `lib/` module that is a real file next to the script is that
  reader's own code, and is counted like any other. Each script runs
  in its own interpreter, so their globals stay apart exactly as
  running them one after another would; the record does not. Both
  engines report the same lines — a differential test over the
  self-hosted suite says so — and a run that fails still reports what
  it reached. Without the flag nothing is recorded and nothing is
  printed.
- All three accept `-` for stdin; `ting --fmt -` is a filter that
  writes the formatted source to stdout, for editor integrations.
- `ting --bundle SCRIPT` prints the script and the local modules it
  imports as one file, on stdout, changing nothing on disk. Each
  module becomes a function holding what its top level declared, which
  runs the first time something asks for it and hands back the same
  map ever after — which is what `import` itself does, so a module
  holding state stays one module, a module two others import is
  inlined once and shared, and a module nothing asks for never runs
  (an `import` inside a branch not taken included). An import is inlined when its path names a
  file and left alone when it names a module embedded in the binary —
  the order the interpreter resolves in, filesystem first — so
  `import("lib/list.ting")` stays, the binary answering it, which is
  what makes one file enough, while a copy of that module sitting
  beside your script is inlined like any other. A script whose first line is `#!/usr/bin/env ting` keeps
  it as the bundle's first line, the header following it, so the one
  file worth `chmod +x` stays executable; a `#!` anywhere else is a
  comment and is left where it is. What a bundle cannot keep identical
  is a program that prints where its own code sits: `try()` hands back a file and a line, and
  in a bundle those are the bundle's. Four things are refused rather
  than guessed at, each named at the file, line and column of the
  import that could not be followed: a cycle, an `import` whose path
  is not a literal string, an `import` whose path names neither a file
  nor an embedded module — copying that one in would hand somebody
  else a bundle that fails where it lands — and a module that returns
  from its own top level, since the bundle would hand back that value
  instead of the module's map, and quietly. It takes a file and only a file: a
  script's imports resolve against its own directory, and a script
  read from stdin has none. `-o FILE` writes the bundle there instead
  of to stdout, and refuses when FILE is one of the files that went
  into it, however it is spelled. That refusal is the reason `-o`
  exists: a shell redirection onto an input truncates it before ting
  is started, so `ting --bundle main.ting > main.ting` reads an empty
  file, reports success, and leaves a bundle of nothing where the
  script was. Every program in this repository that
  imports a local module is bundled and rerun by a test, which is
  where the promise is kept: the same bytes on stdout, the same exit,
  and a bundle that passes `--check` and `--fmt-check`. The bundler
  adds nothing the formatter would rewrite, so a bundle of files that
  pass `--fmt-check` passes it too — and a bundle of files that do not
  does not, the source being copied as it was written.
- `ting --doc NAME` prints what the REPL's `:doc` would: a builtin's
  signature and doc line, or a stdlib function's signature, module
  and comment. A module name (`list` or `lib/list.ting`) lists that
  module's members, one line each, and so does the path of one of
  your own `.ting` files (its top-level functions with the `#`
  comments above them); no name at all lists every builtin and every
  stdlib function. Several names are allowed (`ting --doc len median
  slug`): the entries are printed in the order asked, separated by a
  blank line. A word that names nothing is SEARCHED for instead:
  every entry whose name contains it, or whose comment uses a word
  starting with it, listed the way a module's members are — so
  `ting --doc largest` finds `max_by` and `lib/map.ting`'s
  `top`. Quote several words and they are a PHRASE: they must sit
  next to each other and in that order, each starting a word, so
  `ting --doc "how many"` finds `count_lines`, whose comment begins
  with it. Against a name the words are joined the way ting spells a
  phrase, so `ting --doc "sort by"` finds `sort_by`. ONE word that IS a function is answered in full and then
  told what else mentions it — the names alone, grouped by where
  they live, since `--doc sort` should not leave `sort_with`
  unmentioned, and spelling out all forty-four entries that say the
  word "map" buries the two lines that answered the question;
  several names are a lookup of names you already know and are
  answered one entry each, and a module or a file is answered with
  its index alone. Exit 1 when a word neither names
  nor describes anything — the other names are still printed, and
  one close to a documented name is suggested.
- `ting --lsp` speaks the Language Server Protocol on stdio:
  diagnostics as you type (every syntax error at once, on the same
  rules as `--check`; an error on an `import` of
  a local file that has one, with the module's position; and warnings
  for a name bound nowhere, for a call that cannot match the function
  it names — the file's own or a module's, since the document's URI
  says which directory a module beside it lives in — for a duplicate
  key in a map literal, for code that can
  never run, for an imported module indexed
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
  or a name bound nowhere to the nearest one, or to the stdlib module
  that has it.

While a file has a syntax error in it — which, in an editor, is most
of the time — the answers about WHERE THINGS ARE keep working from
what did parse: the outline, go-to-definition, workspace symbols and
the hover for the file's own functions all still list the lines above
and below the one being typed. The judgements wait: no warning is
published about a file that did not parse, for the reason `--check`
gives above.

Point your editor's generic LSP client at `ting --lsp`; a TextMate
grammar for syntax highlighting ships in the repo under `editor/`.

Scripts behave as shell citizens: `ting x.ting | head` ends quietly
with exit 0 when the reader goes away, and a runtime error prints a
diagnostic and exits 1.

Every tool names a file the same way. A path you typed on the command
line comes back exactly as you typed it; a path the run resolved for
itself — a module an `import` found — is written relative to the
directory the command ran in. One rule covers the header of a
diagnostic and the notes under it, the names `--fmt`, `--doc` and
`--test` print, the `--profile` and `--coverage` tables, the `"file"`
of `try`'s `"at"` and `"trace"`, and the message `--lsp` puts on a
broken `import`.

A path a program computed can be something a path from the command
line never is: the text it meant to read. A line break settles it —
no path a program means to open has one — so `read_file` and
`each_line` say `that is text, not a path` rather than passing on
`No such file or directory` about forty characters of a spreadsheet,
and what they quote is cut to the width a trace uses. A path without
one is named in full, however long: that is the file to go and look
at.

A path written into the middle of a message is quoted, wherever the
message comes from: `ting: cannot read "notes.ting": ...` from a
tool, `cannot read "notes.ting": ...` from a builtin that could not
read it. The quotes are what give a name with a space in it ends,
and a diagnostic's own `file:line:col:` header is the one place a
path is written bare, because there the colon after it does that
job.

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
- JSON nesting: 1000 levels of arrays and objects together, in both
  directions. `json_parse` refuses a deeper document with `nested
  deeper than 1000 at offset N` rather than following it, and
  `json_str` refuses a deeper value the way it refuses a cyclic one,
  since a truncated document would not be JSON. A document is input,
  not program text, so this is a limit a script is protected by
  rather than one it chose.
- Printing: 1000 levels of containers. A deeper one prints as `[...]`
  / `{...}` — the marker a cycle gets — so `str()` and `print()` show
  the shape without following it forever.
- Nesting: 200 levels, counting every block, bracket and unary
  operator a construct sits inside. A program past it is refused
  with `nested too deeply (the limit is 200 levels)` at the token
  that went too far. Unlike the call-depth cap this number is fixed,
  so every command and both engines refuse exactly the same
  programs. Length is not depth: a sum of fifty thousand terms, or a
  chain of calls and indexes, is flat.
- Integers: i64 range; overflow raises an error rather than wrapping.
- Shift counts: 0 to 63.
- Floats: IEEE 754 doubles. `1.0 / 0.0` is infinity and `0.0 / 0.0` is
  NaN, as the arithmetic says, but no literal or conversion produces
  either, and `json_str` refuses to encode one.
- Map keys: strings only.
- Cyclic data (`xs[0] = xs;`) prints with `[...]` / `{...}` at the point
  of recursion, `==` compares it by the parts that are finite (two
  cycles that agree everywhere they can be inspected are equal), and
  `json_str` refuses it with an error. Its memory is not reclaimed
  until the process ends (see Memory).
