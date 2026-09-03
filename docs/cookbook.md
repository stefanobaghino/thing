# Cookbook

Every program under `examples/` in the repository, with the output it
prints — the same pairs CI runs on every commit, so these never drift.
Copy one, run it with `ting name.ting`, change it.

## calc

calc: a tiny arithmetic language interpreted BY ting — a tokenizer, a recursive-descent parser, and an evaluator, all in ting.

```ting
# calc: a tiny arithmetic language interpreted BY ting — a tokenizer,
# a recursive-descent parser, and an evaluator, all in ting.

fn tokenize(src) {
  let toks = [];
  let i = 0;
  let n = len(src);
  while i < n {
    let c = src[i];
    if c == " " {
      i = i + 1;
      continue;
    }
    if contains("0123456789", c) {
      let num = "";
      while i < n && contains("0123456789.", src[i]) {
        num = num + src[i];
        i = i + 1;
      }
      push(toks, {"kind": "num", "text": num});
      continue;
    }
    if contains("+-*/()", c) {
      push(toks, {"kind": c, "text": c});
      i = i + 1;
      continue;
    }
    let name = "";
    while i < n && !contains(" +-*/()", src[i]) {
      name = name + src[i];
      i = i + 1;
    }
    push(toks, {"kind": "ident", "text": name});
  }
  push(toks, {"kind": "end", "text": "<end>"});
  return toks;
}

# The parser state is a map — reference semantics make it shared.
fn peek(st) { return st["toks"][st["pos"]]; }
fn advance(st) { st["pos"] = st["pos"] + 1; }

fn parse_factor(st) {
  let t = peek(st);
  if t["kind"] == "num" {
    advance(st);
    if contains(t["text"], ".") { return {"type": "num", "v": float(t["text"])}; }
    return {"type": "num", "v": int(t["text"])};
  }
  if t["kind"] == "ident" {
    advance(st);
    return {"type": "var", "name": t["text"]};
  }
  if t["kind"] == "(" {
    advance(st);
    let e = parse_expr(st);
    advance(st);  # the ')'
    return e;
  }
  if t["kind"] == "-" {
    advance(st);
    return {"type": "neg", "e": parse_factor(st)};
  }
  fail(format("unexpected token {}", t["text"]));
}

fn parse_term(st) {
  let node = parse_factor(st);
  while peek(st)["kind"] == "*" || peek(st)["kind"] == "/" {
    let op = peek(st)["kind"];
    advance(st);
    node = {"type": "bin", "op": op, "l": node, "r": parse_factor(st)};
  }
  return node;
}

fn parse_expr(st) {
  let node = parse_term(st);
  while peek(st)["kind"] == "+" || peek(st)["kind"] == "-" {
    let op = peek(st)["kind"];
    advance(st);
    node = {"type": "bin", "op": op, "l": node, "r": parse_term(st)};
  }
  return node;
}

fn evaluate(node, vars) {
  let t = node["type"];
  if t == "num" { return node["v"]; }
  if t == "var" { return vars[node["name"]]; }
  if t == "neg" { return -evaluate(node["e"], vars); }
  let l = evaluate(node["l"], vars);
  let r = evaluate(node["r"], vars);
  let op = node["op"];
  if op == "+" { return l + r; }
  if op == "-" { return l - r; }
  if op == "*" { return l * r; }
  return l / r;
}

let vars = {"pi": 3.14159, "x": 10};
for src in ["1 + 2 * 3", "(1 + 2) * 3", "2 * pi", "x * (x - 1) / 2", "-(3 - 5) * 4"] {
  let ast = parse_expr({"toks": tokenize(src), "pos": 0});
  print(src, "=", evaluate(ast, vars));
}
```

```text
1 + 2 * 3 = 7
(1 + 2) * 3 = 9
2 * pi = 6.28318
x * (x - 1) / 2 = 45
-(3 - 5) * 4 = 8
```

## closures

Closures capture their environment by reference.

```ting
# Closures capture their environment by reference.
fn make_counter() {
  let n = 0;
  fn tick() {
    n = n + 1;
    return n;
  }
  return tick;
}

let c1 = make_counter();
let c2 = make_counter();
print(c1(), c1(), c1());  # each call advances the same n
print(c2());  # a fresh counter starts over

fn compose(f, g) {
  return fn(x) { return f(g(x)); };
}
let inc = fn(x) { return x + 1; };
let double = fn(x) { return x * 2; };
print(compose(inc, double)(20));  # double, then inc
```

```text
1 2 3
1
41
```

## collections

Lists, maps, for-in, and the container builtins.

```ting
# Lists, maps, for-in, and the container builtins.
let words = split("the cat sat on the mat the end", " ");

let counts = {};
for w in words {
  if has(counts, w) {
    counts[w] = counts[w] + 1;
  } else {
    counts[w] = 1;
  }
}

# Map iteration visits keys in sorted order, so this is deterministic.
for k in counts {
  print(k, counts[k]);
}
```

```text
cat 1
end 1
mat 1
on 1
sat 1
the 3
```

## fibonacci

Fibonacci two ways: naive recursion and an iterative list build.

```ting
# Fibonacci two ways: naive recursion and an iterative list build.
fn fib(n) {
  if n < 2 {
    return n;
  }
  return fib(n - 1) + fib(n - 2);
}
print("fib(20) =", fib(20));

let seq = [];
let a = 0;
let b = 1;
while len(seq) < 10 {
  push(seq, a);
  let t = a + b;
  a = b;
  b = t;
}
print("first ten:", seq);
```

```text
fib(20) = 6765
first ten: [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
```

## fizzbuzz

FizzBuzz 1..=15: %, if/else chains, string building.

```ting
# FizzBuzz 1..=15: %, if/else chains, string building.
let i = 1;
while i <= 15 {
  if i % 15 == 0 {
    print("FizzBuzz");
  } else if i % 3 == 0 {
    print("Fizz");
  } else if i % 5 == 0 {
    print("Buzz");
  } else {
    print(i);
  }
  i = i + 1;
}
```

```text
1
2
Fizz
4
Buzz
Fizz
7
8
Fizz
Buzz
11
Fizz
13
14
FizzBuzz
```

## hello

The traditional starting point.

```ting
# The traditional starting point.
print("hello, world");
```

```text
hello, world
```

## logs

Summarising a log: tally lines by level with count_by, smooth the latencies with a sliding window, validate fields with is_digit, print the slow ones as an aligned table.

```ting
# Summarising a log: tally lines by level with count_by, smooth the
# latencies with a sliding window, validate fields with is_digit,
# print the slow ones as an aligned table.

let li = import("../lib/list.ting");
let st = import("../lib/string.ting");

let lines = [
  "INFO  12 GET /",
  "INFO  18 GET /docs",
  "WARN  95 GET /search",
  "INFO  15 GET /",
  "ERROR 210 POST /login",
  "INFO  x GET /broken",
  "INFO  22 GET /about",
];

let parsed = [];
for line in lines {
  let fields = filter(split(line, " "), fn(f) { return f != ""; });
  if !st["is_digit"](fields[1]) {
    print("skipping malformed line:", line);
    continue;
  }
  push(parsed, {"level": fields[0], "ms": int(fields[1]), "path": fields[3]});
}

print("by level:", li["count_by"](parsed, fn(e) { return e["level"]; }));

let latencies = map(parsed, fn(e) { return e["ms"]; });
let smoothed = map(li["window"](latencies, 3), fn(w) { return li["mean"](w); });
print("3-point moving average:", map(smoothed, fn(x) { return int(x); }));

let slow = filter(parsed, fn(e) { return e["ms"] >= 90; });
print("slow requests:");
let rows = [["level", "ms", "path"]];
for e in slow { push(rows, [e["level"], str(e["ms"]), e["path"]]); }
print(st["indent"](st["table"](rows), "  "));
```

```text
skipping malformed line: INFO  x GET /broken
by level: {"ERROR": 1, "INFO": 4, "WARN": 1}
3-point moving average: [41, 42, 106, 82]
slow requests:
  level  ms   path
  WARN   95   /search
  ERROR  210  /login
```

## sort

Insertion sort: index assignment and nested while loops.

```ting
# Insertion sort: index assignment and nested while loops.
fn sorted(xs) {
  let out = xs + [];  # copy, so the input list is untouched
  let i = 1;
  while i < len(out) {
    let key = out[i];
    let j = i - 1;
    while j >= 0 && out[j] > key {
      out[j + 1] = out[j];
      j = j - 1;
    }
    out[j + 1] = key;
    i = i + 1;
  }
  return out;
}

let nums = [5, 3, 8, 1, 9, 2, 7];
print("input: ", nums);
print("sorted:", sorted(nums));
print("input again:", nums);
```

```text
input:  [5, 3, 8, 1, 9, 2, 7]
sorted: [1, 2, 3, 5, 7, 8, 9]
input again: [5, 3, 8, 1, 9, 2, 7]
```

## stats

Descriptive statistics over a fixed sample, using lib/math.ting and range with a step.

```ting
# Descriptive statistics over a fixed sample, using lib/math.ting
# and range with a step.

let ma = import("../lib/math.ting");
let li = import("../lib/list.ting");

# Every third value in [2, 60): 2, 5, 8, ...
let sample = range(2, 60, 3);

let n = len(sample);
let mean = li["mean"](sample);

let variance = 0.0;
for x in sample {
  let d = x - mean;
  variance = variance + d * d;
}
variance = variance / n;
let stddev = ma["sqrt"](variance);

print("n      =", n);
print("min    =", min(sample), " max =", max(sample));
print("mean   =", mean, " median =", li["median"](sample));
print("stddev =", ma["round"](stddev * 100) / 100.0);
print("gcd of extremes =", ma["gcd"](min(sample), max(sample)));
```

```text
n      = 20
min    = 2  max = 59
mean   = 30.5  median = 30.5
stddev = 17.3
gcd of extremes = 1
```

## testing

The bundled test framework (lib/test.ting) in action.

```ting
# The bundled test framework (lib/test.ting) in action.
let t = import("../lib/test.ting");

t["check"]("math still works", 6 * 7 == 42);
t["check_eq"]("upper", upper("ting"), "TING");
t["check_eq"]("json round trip", json_parse(json_str([1, 2.5])), [1, 2.5]);

t["summary"]();
```

```text
3 passed, 0 failed
```

## todo

A todo CLI backed by a JSON file — args, env, json, file I/O, and error recovery working together.  ting todo.ting                 # list (the default) ting todo.ting add buy milk    # add an item ting todo.ting done 2          # mark #2 done ting todo.ting rm 1            # delete #1  The list lives in todo.json (override with the TODO_FILE env var).

```ting
# A todo CLI backed by a JSON file — args, env, json, file I/O, and
# error recovery working together.
#
#   ting todo.ting                 # list (the default)
#   ting todo.ting add buy milk    # add an item
#   ting todo.ting done 2          # mark #2 done
#   ting todo.ting rm 1            # delete #1
#
# The list lives in todo.json (override with the TODO_FILE env var).

let path = env("TODO_FILE");
if path == nil { path = "todo.json"; }

fn load() {
  let r = try(fn() { return json_parse(read_file(path)); });
  if has(r, "ok") { return r["ok"]; }
  return [];  # missing or corrupt file: start fresh
}

fn save(items) { write_file(path, json_str(items)); }

fn item_number(argv, items) {
  if len(argv) < 2 { fail("expected an item number"); }
  let n = int(argv[1]);
  if n < 1 || n > len(items) { fail(format("no item #{}", n)); }
  return n;
}

fn show(items) {
  if len(items) == 0 {
    print("nothing to do!");
    return nil;
  }
  let i = 0;
  for item in items {
    i = i + 1;
    let mark = " ";
    if item["done"] { mark = "x"; }
    print(format("{}. [{}] {}", i, mark, item["text"]));
  }
}

let argv = args();
let cmd = "list";
if len(argv) > 0 { cmd = argv[0]; }
let items = load();

if cmd == "list" {
  show(items);
} else if cmd == "add" {
  if len(argv) < 2 {
    print("add what?");
    exit(2);
  }
  push(items, {"text": join(slice(argv, 1, len(argv)), " "), "done": false});
  save(items);
  print(format("added #{}", len(items)));
} else if cmd == "done" || cmd == "rm" {
  let r = try(fn() { return item_number(argv, items); });
  if has(r, "err") {
    print("error:", r["err"]);
    exit(2);
  }
  let n = r["ok"];
  if cmd == "done" {
    items[n - 1]["done"] = true;
    print(format("done: {}", items[n - 1]["text"]));
  } else {
    let kept = [];
    let i = 0;
    for item in items {
      i = i + 1;
      if i != n { push(kept, item); }
    }
    items = kept;
    print(format("removed #{}", n));
  }
  save(items);
} else {
  print("usage: ting todo.ting [list | add <text> | done <n> | rm <n>]");
  exit(2);
}
```

```text
nothing to do!
```
