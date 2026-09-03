# A ting tutorial

This is a guided walk through ting, from nothing to a small real
program. Every ` ```ting ` snippet on this page is standalone: it runs
as-is, and `tests/tutorial.rs` executes each one against the current
interpreter on every CI run — when a snippet has a ` ```text ` block
right after it, that is its exact output, verified. The
[reference](reference.md) covers the full language; this page covers
the road.

## Hello

Save this as `hello.ting` and run `ting hello.ting`:

```ting
print("hello, world");
```

```text
hello, world
```

Statements end with `;`. Comments run from `#` to the end of the line.

## Values

ting has ints, floats, strings, bools, `nil`, lists, and maps. It is
dynamically typed but strict: nothing converts implicitly, and there is
no truthiness.

```ting
let n = 6 * 7;              # int
let x = 1 / 2;              # int division truncates: 0
let y = 1.0 / 2;            # mixing with a float promotes: 0.5
let s = "tin" + "g";        # + concatenates strings (and lists)
print(n, x, y, s, type(s));
```

```text
42 0 0.5 ting string
```

Conditions must be actual bools — `if 1 { }` is a type error, not a
surprise:

```ting
let hour = 13;
if hour >= 12 && hour < 18 {
  print("good afternoon");
} else {
  print("hello");
}
```

```text
good afternoon
```

## Loops

`while` and `for` need braces and take no parentheses. `for` iterates
lists, strings (by character), and maps (by sorted key):

```ting
let total = 0;
for n in range(1, 11) {
  if n % 2 == 1 {
    continue;             # skip odd numbers
  }
  total = total + n;
}
print("sum of evens up to 10:", total);
```

```text
sum of evens up to 10: 30
```

## Functions are values

`fn name(...) { ... }` defines a function; anonymous `fn(...) { ... }`
is an expression. Functions close over their environment — captured
variables are shared, not copied:

```ting
fn make_counter() {
  let n = 0;
  fn tick() { n = n + 1; return n; }
  return tick;
}
let c = make_counter();
print(c(), c(), c());
```

```text
1 2 3
```

Builtins are ordinary values too, so you can pass them around:

```ting
let words = ["kiwi", "fig", "banana"];
print(sort_by(words, len));
```

```text
["fig", "kiwi", "banana"]
```

## Lists and maps share, copies are explicit

Lists and maps have reference semantics, like Python or JavaScript.
`slice` (or `+ []`) makes a real copy:

```ting
let a = [1, 2, 3];
let b = a;                  # same list
let c = slice(a, 0, len(a)); # fresh copy
b[0] = 99;
print(a, c);
```

```text
[99, 2, 3] [1, 2, 3]
```

Maps have string keys, kept sorted. Reading a missing key is an error,
so test with `has` first:

```ting
let ages = {"ada": 36, "linus": 55};
ages["grace"] = 85;
for name in ages {
  print(name, ages[name]);
}
print(has(ages, "ada"), has(ages, "bob"));
```

```text
ada 36
grace 85
linus 55
true false
```

## When things go wrong

Runtime errors stop the program with a caret diagnostic — unless you
catch them. `try(f)` calls `f()` and gives you a map instead of a
crash; `fail(msg)` raises your own:

```ting
fn parse_age(s) {
  let n = int(s);           # errors on non-numeric input
  if n < 0 { fail("age cannot be negative"); }
  return n;
}
for raw in ["42", "-1", "unknown"] {
  let r = try(fn() { return parse_age(raw); });
  if has(r, "ok") {
    print(raw, "->", r["ok"]);
  } else {
    print(raw, "-> error:", r["err"]);
  }
}
```

```text
42 -> 42
-1 -> error: age cannot be negative
unknown -> error: cannot convert "unknown" to int
```

## Splitting code into modules

`import(path)` runs another file once and hands you its top-level
definitions as a map. (This snippet writes its module first so it's
self-contained — normally the module just sits next to your script.)

```ting
write_file("greeter.ting",
  "fn greet(name) { return \"hi, \" + name; }\nlet version = 1;\n");

let g = import("greeter.ting");
print(g["greet"]("ting"), "- module v" + str(g["version"]));
```

```text
hi, ting - module v1
```

Relative paths resolve against the importing file's directory. A module
runs once per program: every later `import` of the same file returns
the very same map, and errors inside a module point at the module's own
line and column.

Six stdlib modules ship embedded in the interpreter itself — any
path starting with `lib/` falls back to the built-in copy when no
such file exists on disk:

```ting
let li = import("lib/list.ting");
let ma = import("lib/math.ting");

let sizes = map(["a", "bbb", "cc"], fn(s) { return len(s); });
print(li["sum"](sizes), ma["pow"](2, 8));
```

```text
6 256
```

The list module is where most of the everyday shape-shifting lives —
splitting by a predicate, grouping by a key, taking a prefix:

```ting
let li = import("lib/list.ting");
let words = ["ant", "bee", "cow", "eel", "fox"];

let split = li["partition"](words, fn(w) { return contains("aeiou", w[0]); });
print(split[0], split[1]);

let by_len = li["group_by"](["a", "bb", "cc", "d"], fn(w) { return str(len(w)); });
print(by_len);
print(li["take"](words, 2), li["drop"](words, 4));
```

```text
["ant", "eel"] ["bee", "cow", "fox"]
{"1": ["a", "d"], "2": ["bb", "cc"]}
["ant", "bee"] ["fox"]
```

Keys of a map are always strings, so `group_by`'s key function must
return one — `str(...)` is the idiom.

The [stdlib page](stdlib.html) documents all six
(list/map/string/math/json/test).

## Working with JSON

`json_parse` turns a JSON string into ting values (objects become maps,
arrays become lists) and `json_str` goes the other way. Map keys stay
sorted, so output is deterministic. Pass an indent to pretty-print.

```ting
let cfg = json_parse("{\"name\": \"ting\", \"tags\": [\"small\", \"strict\"]}");
print(cfg["name"], "has", len(cfg["tags"]), "tags");

push(cfg["tags"], "scripting");
print(json_str(cfg));
print(json_str(cfg, 2));
```

```text
ting has 2 tags
{"name":"ting","tags":["small","strict","scripting"]}
{
  "name": "ting",
  "tags": [
    "small",
    "strict",
    "scripting"
  ]
}
```

Malformed input fails like any other error, so `try` gives you a
recovery path:

```ting
let bad = try(fn() { return json_parse("{oops"); });
if has(bad, "err") { print("rejected:", bad["err"]); } else { print("parsed"); }
```

```text
rejected: json_parse: expected a string key at offset 1
```

## A real script: word frequency

Everything together: arguments, file I/O with recovery, and the
stdlib's `words`, `frequencies` and `top`. Run it as
`ting wordfreq.ting somefile.txt` — with no argument it demonstrates
itself on a built-in sample:

```ting
let text = "the cat sat on the mat and the cat slept";
if len(args()) > 0 {
  let r = try(fn() { return read_file(args()[0]); });
  if has(r, "err") {
    print("cannot read", args()[0], "-", r["err"]);
  } else {
    text = r["ok"];
  }
}

let li = import("lib/list.ting");
let st = import("lib/string.ting");
let ma = import("lib/map.ting");
let counts = li["frequencies"](st["words"](lower(text)));

# The three most frequent words as [word, count] pairs, ties in
# alphabetical order.
for pair in ma["top"](counts, 3) {
  print(pair[1], pair[0]);
}
```

```text
3 the
2 cat
1 and
```

That's the whole tour. From here: the [reference](reference.md) for
every operator and builtin, or `ting` with no arguments for a REPL to
poke at.

## Testing

`lib/test.ting` is a test framework in forty lines of ting: record
checks, print a summary, exit 1 if anything failed. A test file is
just a script that imports it:

```ting
let t = import("lib/test.ting");
let li = import("lib/list.ting");

t["check"]("sum adds up", li["sum"]([1, 2, 3]) == 6);
t["check_eq"]("median of four", li["median"]([4, 1, 3, 2]), 2.5);
t["check_approx"]("floats are close enough", 0.1 + 0.2, 0.3, 0.000001);
t["summary"]();
```

```text
3 passed, 0 failed
```

`check_err(name, f, want)` asserts that calling `f` fails with a
message containing `want`. A failed check keeps its name, and
`check_eq` and `check_approx` say what they got and what they
wanted, so the summary reads like a report rather than a stack
trace. Plain `assert(cond, msg)` works too when a framework is more
than a script needs.

Put test files under a directory and run them all with
`ting --test tests/` — one process per file, `ok` or `FAIL` per file,
exit 1 if any failed — and `--filter NAME` to run only the files
whose path contains `NAME` while iterating on one of them. The
project's own suite under `selftest/` runs exactly that way in CI.

## Beyond scripts

Everything else ships in the same binary:

- The REPL (`ting` with no arguments) keeps state across lines;
  `:help` lists the builtins, `:doc median` explains one function
  (builtin or stdlib), `:load somefile.ting` pulls a script's
  definitions into your session to poke at them, `:vars` shows what
  you have bound, `:fmt` reprints your last line the way the
  formatter would write it, and `:clear` starts over.
- `ting --check *.ting` reports syntax errors without running
  anything — wire it into a pre-commit hook.
- `ting --test tests/` runs every `.ting` file under a directory
  and prints `ok` or `FAIL` per file plus a summary — a test runner
  with no setup.
- `ting --fmt *.ting` reformats in place; CI can enforce it with
  `--fmt-check`, and `ting --fmt -` filters stdin to stdout for
  editor integrations.
- `ting --lsp` gives any LSP-capable editor diagnostics, hover docs,
  completion, signature help, formatting, outline, definition,
  references, and rename. Import a stdlib module and the editor
  knows its functions too: completion lists them, and hovering
  `l["median"]` shows the signature and its comment.
- Scripts are shell citizens: `ting x.ting | head` exits quietly
  when the reader goes away, and `read_file("-")` reads stdin.

The [reference](reference.html) has the full language; the
[stdlib page](stdlib.html) documents the importable modules.
