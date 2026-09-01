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

## A real script: word frequency

Everything together: arguments, file I/O with recovery, maps, sorting,
and string builtins. Run it as `ting wordfreq.ting somefile.txt` — with
no argument it demonstrates itself on a built-in sample:

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

let counts = {};
for w in split(trim(lower(text)), " ") {
  if w == "" { continue; }
  if has(counts, w) {
    counts[w] = counts[w] + 1;
  } else {
    counts[w] = 1;
  }
}

# Sort words by falling count (negate the count for the key).
let words = sort_by(keys(counts), fn(w) { return 0 - counts[w]; });
for w in slice(words, 0, 3) {
  print(counts[w], w);
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
