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

## Closures as objects

Several closures over the same variables behave like an object: the
variables are its private state, the closures its methods. Return
them in a map and you have an interface without any new syntax:

```ting
fn make_account(balance) {
  let history = [];
  fn deposit(n) {
    balance = balance + n;
    push(history, "+" + str(n));
    return balance;
  }
  fn withdraw(n) {
    if n > balance { fail("insufficient funds"); }
    balance = balance - n;
    push(history, "-" + str(n));
    return balance;
  }
  fn statement() { return join(history, " "); }
  return {"deposit": deposit, "withdraw": withdraw, "statement": statement};
}

let acct = make_account(10);
acct["deposit"](5);
acct["withdraw"](12);
print(acct["statement"]());
let r = try(fn() { return acct["withdraw"](100); });
print(r["err"]);
```

```text
+5 -12
insufficient funds
```

Each call to `make_account` makes a fresh `balance` and `history`, so
two accounts never share state — the same rule as the counter above.
The cookbook's `machine` example takes this to a full state machine:
a transition table in a map and a `send` closure that walks it.

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
the very same map.

When something fails inside a module's function, the diagnostic points
at the module's own file and line, and a note names the place in your
script that called into it. Say `greeter.ting` had a typo in `greet`
and the script above called it on line 4:

```text
greeter.ting:1:34: error: undefined variable 'nam' (did you mean 'name'?)
 1 | fn greet(name) { return "hi, " + nam; }
   |                                  ^^^
note: in greet, called from main.ting:4:7
```

That note is not only for modules. Every call an error unwound
through leaves one, innermost first, so a failure deep in a chain of
calls says how the program got there:

```text
report.ting:1:22: error: cannot apply '+' to int and string
 1 | fn total(x) { return x + "!"; }
   |                      ^^^^^^^
note: in total, called from report.ting:2:21
note: in line, called from report.ting:3:7
```

Runaway recursion would otherwise print one note per frame, so a
trace longer than ten keeps four at each end and says how many it
left out (`note: ... 192 more frames`).

A program can read the same three things. `try` returns the message
under `"err"`, where it was raised under `"at"`, and the calls it
came out of under `"trace"` — each frame a map of `"fn"` (nil for a
function with no name), `"file"`, `"line"` and `"col"`:

```ting
fn parse(s) { return int(s); }
let r = try(fn() { return parse("x"); });
print(r["err"]);
print("raised on line", r["at"]["line"], "in", r["trace"][0]["fn"]);
```

```text
cannot convert "x" to int
raised on line 1 in parse
```

The parenthesis after the name is ting guessing at a typo: when the
name you wrote is close to one in scope — a binding, a parameter, a
builtin — the error names it. Keys work the same way (`key "medain"
not found (did you mean "median"?)`), and so do `ting --doc` and the
command line itself, where `--fmr` is told about `--fmt`.

Errors the checker can find without running — a syntax error in a
module, say — surface earlier: `ting --check main.ting` follows every
`import` of a local file and reports each one under its own path.

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

The [stdlib page](stdlib.html) documents all seven
(list/map/string/math/json/fs/test), and you never have to open a
module's source to read about one function: `ting --doc median` in
a shell, or `:doc median` in the REPL, prints its signature, module
and comment — the same text an LSP-capable editor shows on hover.

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

For nested documents, `lib/json.ting` navigates by path — a list of
keys and indices — instead of chained indexing that errors on the
first missing step:

```ting
let j = import("lib/json.ting");
let doc = json_parse("{\"users\": [{\"name\": \"ann\"}, {\"name\": \"bob\", \"admin\": true}]}");

print(j["get_in"](doc, ["users", 1, "name"]), j["get_in"](doc, ["users", 0, "admin"]));
let doc2 = j["set_in"](doc, ["users", 0, "admin"], false);
print(j["get_in"](doc2, ["users", 0, "admin"]));
let merged = j["merge_in"]({"a": {"x": 1}}, {"a": {"y": 2}});
print(merged);
```

```text
bob nil
false
{"a": {"x": 1, "y": 2}}
```

`get_in` answers `nil` for anything missing, `set_in` returns a fresh
document (the original is untouched), and `merge_in` folds maps
together recursively. The cookbook's `config` example layers
defaults, a file and environment overrides that way.

Two more views help when documents change hands: `diff` lists every
leaf path where two documents disagree, and `flatten` turns a nested
document into one map from dotted paths to leaves — the shape that
diffing tools, environment variables and log lines like:

```ting
let j = import("lib/json.ting");
let before = {"port": 8080, "log": {"level": "info"}};
let after = {"port": 9090, "log": {"level": "info", "file": "app.log"}};
for d in j["diff"](before, after) { print(d); }
print(j["flatten"](after));
```

```text
[["port"], 8080, 9090]
[["log", "file"], nil, "app.log"]
{"log.file": "app.log", "log.level": "info", "port": 9090}
```

Each difference is a `[path, left, right]` triple, with `nil` standing
in for a side that has no such leaf.

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

## Shell scripting

A ting script is a shell citizen. `args()` is the argument list after
the script path, `env(name)` reads an environment variable (`nil` when
unset), `read_file("-")` reads all of stdin, and `exit(code)` sets the
process's exit status:

```ting
let verbose = env("TING_TUTORIAL_VERBOSE") != nil;
print(len(args()), "arguments; verbose:", verbose);
if len(args()) > 3 {
  print("too many arguments");
  exit(2);
}
```

```text
0 arguments; verbose: false
```

Combined with the stdlib that makes small filters short: split stdin
into `lines`, keep the ones matching a `contains`, count them with
`frequencies`, print an aligned `table`. Errors that escape the script
print a caret diagnostic to stderr and exit 1, so `set -e` and CI
steps behave; `ting script.ting | head` exits quietly when the reader
goes away, like any well-behaved filter. The cookbook's `pipeline`
example is a complete stdin-to-report script.

## Files and directories

Four builtins let a script see the tree it is running in, and
`lib/fs.ting` turns them into something comfortable:

```ting
let fs = import("lib/fs.ting");

make_dir("report/data");
write_file("report/data/one.ting", "print(1);\n");
write_file("report/data/notes.txt", "hello");

print(list_dir("report/data"));
print(fs["walk_ext"]("report", "ting"));
print(exists("report/data"), is_dir("report/nope"));
print(fs["stem"]("report/data/one.ting"), fs["ext"]("notes.txt"));
```

```text
["notes.txt", "one.ting"]
["report/data/one.ting"]
true false
one txt
```

`list_dir` gives names, sorted; `fs["entries"]` gives the same thing
as paths, and `fs["walk"]` recurses to every file below a directory
with the directories left out — `walk_ext` filters that by
extension, which is most of what a tool that runs over a tree needs.
`make_dir` creates missing parents and does not mind a directory
that is already there, so it pairs with `write_file` into a tree
that does not exist yet.

`exists` and `is_dir` are questions, so an absent or unreadable path
answers `false` rather than raising — they can be used in an `if`
without wrapping. `list_dir` is a demand, and errors when the path
is not a readable directory, because asking what is inside
something that is not there is a mistake worth hearing about. There
is no way to delete anything: ting can create a tree and read it,
not remove it.

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
message containing `want`, and `check_type(name, v, "list")` that a
value has the type you expect. A failed check keeps its name, and
`check_eq` and `check_approx` say what they got and what they
wanted, so the summary reads like a report rather than a stack
trace. Plain `assert(cond, msg)` works too when a framework is more
than a script needs.

Put test files under a directory and run them all with
`ting --test tests/` — one process per file, `ok` or `FAIL` per file,
exit 1 if any failed. Each line says how much that file verified —
`ok tests/list.ting (12 checks)`, counting every `assert` and every
`lib/test.ting` helper call — and the summary totals them, so a
suite that quietly stops checking anything is visible instead of
green. A file that passes without a single check is named as such,
in its line and in the summary. While iterating on one file,
`--filter NAME`
runs only the files whose path contains `NAME`, and `--fail-fast`
stops at the first red one. On a multi-core machine `-j 4` runs four
files at once with the output kept in order; `--slow 3` names the
three slowest files after the summary; and `--tap` switches to Test
Anything Protocol output for CI systems that consume it. The
project's own suite under `selftest/` runs exactly that way in CI.

## Leaving it running

Editing and re-running by hand is the loop the tooling exists to
remove, so `--test`, `--check` and `--fmt-check` take `--watch`:

```sh
ting --test --watch tests/
```

The suite runs, and then runs again every time a watched file
changes, appears or disappears. A rule line separates one run from
the next and says what set it off:

```
-- run 1 ------------------------------------------------------------
ok   tests/list.ting (2 checks)
1 passed, 0 failed, 2 checks
-- run 2: tests/map.ting added --------------------------------------
ok   tests/list.ting (2 checks)
ok   tests/map.ting (1 check)
2 passed, 0 failed, 3 checks
```

(Eighty columns of rule, trimmed above to fit the page.) The
directory is looked at afresh each time, so the file you have just
written joins the next run without restarting anything. Ctrl-C ends
it. `ting --check --watch src/` does the same for the checker, and
`ting --fmt-check --watch src/` for the formatter's verdict —
though not `ting --fmt --watch`, which would rewrite a file, notice
its own write, and run forever; that one is a usage error pointing
at the two modes that write nothing.

## Scripts from a pipe

A script does not need a file. `-` in place of the path reads the
program from standard input, which is how a generated script, a
heredoc, or the output of another tool gets run:

```sh
echo 'print("hello, " + args()[0]);' | ting - world
```

Arguments after the dash reach `args()` as usual and errors name the
script `-`. One caveat, worth knowing before you meet it: the script
*is* what was piped in, so stdin is at end of file by the time it
runs and `input()` returns `nil` straight away. Pipe a script or
pipe it data — not both.

## Beyond scripts

Everything else ships in the same binary:

- The REPL (`ting` with no arguments) keeps state across lines;
  `:help` lists the builtins, `:doc median` explains one function
  (builtin or stdlib), `:load somefile.ting` pulls a script's
  definitions into your session to poke at them, `:vars` shows what
  you have bound, `:time EXPR` says how long a line took, `:fmt`
  reprints your last line the way the formatter would write it,
  `:history` lists what you have run so far, `:save notes.ting`
  writes it out as a script you can run again, and `:clear` starts
  over.
- `ting --check *.ting` reports syntax errors without running
  anything, follows `import` to your local modules, and warns about
  a name bound nowhere, a call whose argument count cannot match the
  function it names, a duplicate key in a map literal, a statement
  that can never run, a misspelt stdlib member, an unused binding (at
  the top level or inside a function), an unused parameter or a name
  that shadows a builtin — wire it into a pre-commit hook, with
  `--strict` if the warnings should block the commit too. The
  playground's check button does the same in the browser.
- `ting --test tests/` runs every `.ting` file under a directory
  and prints `ok` or `FAIL` per file, how many checks each ran, and a
  summary — a test runner with no setup (the [Testing](#testing)
  chapter has its flags).
- `ting --fmt *.ting` reformats in place and ends with a summary of
  what it did; CI can enforce it with `--fmt-check`, `--fmt --diff`
  shows what would change, and `ting --fmt -` filters stdin to stdout
  for editor integrations. A file that does not lex is reported and
  the others are still handled.
- `ting --profile myscript.ting` runs it and then says where the
  time went — every function and builtin, how often it ran and how
  long it spent in its own body, slowest first, on stderr:

```text
profile: 8 functions, 120003 calls, 43.209ms in them
     calls        self  function                  where
     20000    16.223ms  slug                      report.ting:1:1
         1    13.089ms  slugs                     report.ting:2:1
     40000     5.218ms  push                      a builtin
     20000     3.383ms  replace                   a builtin
     20000     2.434ms  trim                      a builtin
     20000     2.353ms  lower                     a builtin
         1     0.469ms  len                       a builtin
         1     0.040ms  print                     a builtin
```

  The time is self time, so `slugs` — which does nothing but loop
  and call `slug` — is charged only its own loop, and the work it
  asked for shows up under the function that did it. Twenty rows at
  most; the rest are counted at the end.
- `ting --doc` prints the table of contents — every builtin and
  stdlib function — and `ting --doc list`, `ting --doc median` or
  `ting --doc myfile.ting` narrow it to a module, a function, or the
  functions in your own file with the comments above them.
- `ting --lsp` gives any LSP-capable editor diagnostics (the same
  warnings, plus an error on a broken import), hover docs,
  completion, signature help, formatting, outline, definition,
  references, rename across open files, folding and links on
  imports. Import a stdlib module and the editor knows its
  functions too: completion lists them, and hovering `l["median"]`
  shows the signature and its comment — as does hovering a
  function of your own that has a `#` comment above it.
- Scripts are shell citizens: `ting x.ting | head` exits quietly
  when the reader goes away, and `read_file("-")` reads stdin.

The [reference](reference.html) has the full language; the
[stdlib page](stdlib.html) documents the importable modules.
