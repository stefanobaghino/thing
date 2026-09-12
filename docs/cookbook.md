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
      i += 1;
      continue;
    }
    if contains("0123456789", c) {
      let num = "";
      while i < n && contains("0123456789.", src[i]) {
        num += src[i];
        i += 1;
      }
      push(toks, {"kind": "num", "text": num});
      continue;
    }
    if contains("+-*/()", c) {
      push(toks, {"kind": c, "text": c});
      i += 1;
      continue;
    }
    let name = "";
    while i < n && !contains(" +-*/()", src[i]) {
      name += src[i];
      i += 1;
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
    n += 1;
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
  counts[w] = get(counts, w, 0) + 1;
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

## columns

Laying out a table when the DATA decides how wide the columns are. table() does the whole thing in one call, with one character per column saying which way it leans; a format spec can take a width it is handed rather than one written into the template, for the lines a table has no row for.

```ting
# Laying out a table when the DATA decides how wide the columns are.
# table() does the whole thing in one call, with one character per
# column saying which way it leans; a format spec can take a width it
# is handed rather than one written into the template, for the lines
# a table has no row for.

let li = import("../lib/list.ting");
let st = import("../lib/string.ting");

let folders = [
  { "name": "src", "files": 14, "bytes": 482301 },
  { "name": "documentation", "files": 6, "bytes": 91244 },
  { "name": "lib", "files": 13, "bytes": 120558 },
  { "name": "t", "files": 209, "bytes": 3311902 },
];

fn mb(bytes) { return float(bytes) / 1000000.0; }

# The cells as strings, then one call. The alignment marks the two
# columns that hold numbers, so they line up under their headings
# instead of hanging off the left.
let rows = [["folder", "files", "MB"]];
for {name, files, bytes} in folders {
  push(rows, [name, str(files), format("{:.2}", mb(bytes))]);
}
print(st["table"](rows, "<>>"));
print("");

# A rule under the heading and a total below the last row are not
# rows of the table, so they are laid out by hand — against the same
# widths, which come from the same data.
let width = 0;
for {name} in folders {
  if len(name) > width { width = len(name); }
}
let total = li["sum_by"](folders, fn({bytes}) { return mb(bytes); });

print(format("{:<{}}   {:>8}", "folder", width, "MB"));
print(st["repeat"]("-", width + 11));
for {name, bytes} in folders {
  print(format("{:<{}}   {:>8.2}", name, width, mb(bytes)));
}
print(st["repeat"]("-", width + 11));
print(format("{:<{}}   {:>8.2}", "total", width, total));
print("");

# pad_right fills with a space unless told otherwise, so a label and
# its value need no third argument.
for {name, files} in folders {
  print(st["pad_right"](name, width) + "  " + st["plural"](files, "file", "files"));
}
```

```text
folder         files    MB
src               14  0.48
documentation      6  0.09
lib               13  0.12
t                209  3.31

folder                MB
------------------------
src                 0.48
documentation       0.09
lib                 0.12
t                   3.31
------------------------
total               4.01

src            14 files
documentation  6 files
lib            13 files
t              209 files
```

## config

Layered configuration with lib/json.ting: built-in defaults, a config-file overlay and environment-style overrides folded together with merge_in, the effective settings printed as a table, and diff reporting exactly what the overrides changed.

```ting
# Layered configuration with lib/json.ting: built-in defaults, a
# config-file overlay and environment-style overrides folded together
# with merge_in, the effective settings printed as a table, and diff
# reporting exactly what the overrides changed.

let j = import("../lib/json.ting");
let st = import("../lib/string.ting");

let defaults = {
  "server": {"host": "127.0.0.1", "port": 8080, "tls": false},
  "log": {"level": "info", "format": "text"},
  "features": ["health"],
};

# What a config file might contribute (here parsed from a string).
let file = json_parse("{\"server\": {\"port\": 9000}, \"features\": [\"health\", \"metrics\"]}");

# Environment-style overrides as dotted paths.
let overrides = {"server.tls": "true", "log.level": "debug"};

let effective = j["merge_in"](defaults, file);
for key in keys(overrides) {
  let value = overrides[key];
  if value == "true" { value = true; } else if value == "false" { value = false; }
  effective = j["set_in"](effective, split(key, "."), value);
}

let rows = [["setting", "value"]];
for path in j["paths"](effective) {
  push(rows, [join(map(path, str), "."), str(j["get_in"](effective, path))]);
}
print(st["table"](rows));

print("changed from defaults:");
for [path, was, now] in j["diff"](defaults, effective) {
  print(" ", join(map(path, str), "."), str(was), "->", str(now));
}
```

```text
setting      value
features.0   health
features.1   metrics
log.format   text
log.level    debug
server.host  127.0.0.1
server.port  9000
server.tls   true
changed from defaults:
  log.level info -> debug
  server.port 8080 -> 9000
  server.tls false -> true
  features.1 nil -> metrics
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
  i += 1;
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

## inventory

A stock list: nested config flattened to dotted paths, the item behind a stock count found with key_of, the first items still in stock taken with take_while, and a summary that pluralises itself.

```ting
# A stock list: nested config flattened to dotted paths, the item
# behind a stock count found with key_of, the first items still in
# stock taken with take_while, and a summary that pluralises itself.

let li = import("../lib/list.ting");
let ma = import("../lib/map.ting");
let st = import("../lib/string.ting");
let js = import("../lib/json.ting");

let stock = {"bolt": 120, "nut": 0, "washer": 45, "screw": 0, "rivet": 7};

# Which item has exactly 45 in stock? key_of is the inverse lookup.
print("45 in stock:", ma["key_of"](stock, 45));
print("9 in stock:", ma["key_of"](stock, 9));

# Items sorted by count, then the leading run that is out of stock.
let names = sort_by(keys(stock), fn(k) { return stock[k]; });
let empty = li["take_while"](names, fn(k) { return stock[k] == 0; });
let stocked = li["drop_while"](names, fn(k) { return stock[k] == 0; });
print("out of stock:", empty);
print("in stock:", stocked);

# A nested warehouse record, flattened for a one-line-per-setting view.
let warehouse = {"site": "north", "bays": {"a": {"rows": 4}, "b": {"rows": 6}}};
for path in keys(js["flatten"](warehouse)) {
  print(path, "=", js["flatten"](warehouse)[path]);
}

# The summary reads correctly whatever the counts are.
print(st["plural"](len(empty), "item", "items"), "to reorder,",
st["plural"](len(stocked), "item", "items"), "on the shelf");
```

```text
45 in stock: washer
9 in stock: nil
out of stock: ["nut", "screw"]
in stock: ["rivet", "washer", "bolt"]
bays.a.rows = 4
bays.b.rows = 6
site = north
2 items to reorder, 3 items on the shelf
```

## logreport

What a log says, without holding the log — a report that costs the same nine megabytes whether the file is a kilobyte or a terabyte, because each_line hands over one line at a time and this keeps only what it is counting.  ting logreport.ting              # report on a log this makes ting logreport.ting /var/log/x   # report on a log of your own cat big.log | ting logreport.ting -  With no argument it builds a log, reports on it and removes it, so the example prints the same thing every time. The demo is small enough to run anywhere; the point is that nothing in the report grows with the file.

```ting
# What a log says, without holding the log — a report that costs the
# same nine megabytes whether the file is a kilobyte or a terabyte,
# because each_line hands over one line at a time and this keeps only
# what it is counting.
#
#   ting logreport.ting              # report on a log this makes
#   ting logreport.ting /var/log/x   # report on a log of your own
#   cat big.log | ting logreport.ting -
#
# With no argument it builds a log, reports on it and removes it, so
# the example prints the same thing every time. The demo is small
# enough to run anywhere; the point is that nothing in the report
# grows with the file.

let fs = import("../lib/fs.ting");
let tm = import("../lib/time.ting");

fn build(path) {
  let levels = ["INFO", "INFO", "INFO", "WARN", "WARN", "ERROR"];
  let sources = ["api", "db", "cache"];
  let lines = [];
  let start = 1788652800000;
  for i in range(5000) {
    push(lines, format("{} {} {} request {}",
    tm["iso"](start + i * 1000), levels[i % 6], sources[i % 3], i));
  }
  write_file(path, join(lines, "\n") + "\n");
  return path;
}

let given = args();
let path = "logreport-demo.log";
let mine = len(given) == 0;
if !mine { path = given[0]; }
if mine { build(path); }

if path != "-" && !exists(path) {
  eprint(format("logreport: no such path: {}", path));
  exit(2);
}

# One pass. Everything below is bounded: three counters per level,
# one per source, and two lines held at a time. fs["head"] and
# fs["tail"] would give the first and last just as well, but each of
# them is another pass over the file, and the ends are free here.
let by_level = {};
let by_source = {};
let first = nil;
let last = nil;
let longest = "";
let lines = each_line(path, fn(line) {
  let fields = split(line, " ");
  if len(fields) >= 3 {
    by_level[fields[1]] = get(by_level, fields[1], 0) + 1;
    by_source[fields[2]] = get(by_source, fields[2], 0) + 1;
  }
  if first == nil { first = line; }
  last = line;
  if len(line) > len(longest) { longest = line; }
  return nil;
});

fn ranked(counts) {
  let rows = [];
  for k in keys(counts) { push(rows, {"name": k, "n": counts[k]}); }
  return sort_with(rows, fn(a, b) { return b["n"] - a["n"]; });
}

# The comparison the report is for: what the file weighs against what
# reading it cost. A pipe has no size to ask for, so it says so.
let size = fs["size"](path);

# An empty log is an ordinary thing to be handed, and everything below
# this point assumes there was a line to look at.
if lines == 0 {
  print(format("{}: no lines", path));
  if mine { remove_file(path); }
  exit(0);
}

let held = len(longest) + len(last);
if size == nil {
  print(format("{}: {} lines", path, lines));
} else {
  print(format("{}: {} lines, {} bytes", path, lines, size));
}
print(format("held while reading: {} bytes (the longest line and the last one)", held));
print("");

print("by level");
for row in ranked(by_level) {
  print(format("  {:>5} {}", row["n"], row["name"]));
}
print("");

print("by source");
for row in ranked(by_source) {
  print(format("  {:>5} {}", row["n"], row["name"]));
}
print("");

print(format("first  {}", first));
print(format("last   {}", last));

if mine { remove_file(path); }
```

```text
logreport-demo.log: 5000 lines, 216388 bytes
held while reading: 86 bytes (the longest line and the last one)

by level
   2501 INFO
   1666 WARN
    833 ERROR

by source
   1667 api
   1667 db
   1666 cache

first  2026-09-06T00:00:00Z INFO api request 0
last   2026-09-06T01:23:19Z INFO db request 4999
```

## logs

Summarising a log: tally lines by level with count_by, smooth the latencies with a sliding window, validate fields with is_digit, print the slow ones as an aligned table. Every record comes apart by field name — `fn({ms})` — rather than by subscript.

```ting
# Summarising a log: tally lines by level with count_by, smooth the
# latencies with a sliding window, validate fields with is_digit,
# print the slow ones as an aligned table. Every record comes apart
# by field name — `fn({ms})` — rather than by subscript.

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

print("by level:", li["count_by"](parsed, fn({level}) { return level; }));

let latencies = map(parsed, fn({ms}) { return ms; });
let smoothed = map(li["window"](latencies, 3), fn(w) { return li["mean"](w); });
print("3-point moving average:", map(smoothed, fn(x) { return int(x); }));

let slow = filter(parsed, fn({ms}) { return ms >= 90; });
print("slow requests:");
let rows = [["level", "ms", "path"]];
for {level, ms, path} in slow { push(rows, [level, str(ms), path]); }
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

## machine

A state machine from closures and a map: the machine is a function that captures its current state and a trace, and a transition table maps state -> event -> [next state, action]. A turnstile: it locks after each push, and a coin unlocks it.

```ting
# A state machine from closures and a map: the machine is a function
# that captures its current state and a trace, and a transition table
# maps state -> event -> [next state, action]. A turnstile: it locks
# after each push, and a coin unlocks it.

let li = import("../lib/list.ting");
let st = import("../lib/string.ting");

let table = {
  "locked": {"coin": ["unlocked", "unlock"], "push": ["locked", "alarm"]},
  "unlocked": {"coin": ["unlocked", "refund"], "push": ["locked", "pass"]},
};

fn make_machine(table, start) {
  let state = start;
  let trace = [];
  fn send(event) {
    let moves = table[state];
    if !has(moves, event) { fail("no transition for " + event + " in " + state); }
    let next = moves[event][0];
    push(trace, [state, event, moves[event][1], next]);
    state = next;
    return moves[event][1];
  }
  fn current() { return state; }
  fn history() { return trace; }
  return {"send": send, "current": current, "history": history};
}

let m = make_machine(table, "locked");
for event in ["push", "coin", "coin", "push", "push", "coin"] {
  m["send"](event);
}

let rows = [["from", "event", "action", "to"]];
for step in m["history"]() { push(rows, step); }
print(st["table"](rows));
print("final state:", m["current"]());
print("actions:", li["frequencies"](map(m["history"](), fn(s) { return s[2]; })));
let bad = try(m["send"], "kick");
print("unknown event:", bad["err"]);
```

```text
from      event  action  to
locked    push   alarm   locked
locked    coin   unlock  unlocked
unlocked  coin   refund  unlocked
unlocked  push   pass    locked
locked    push   alarm   locked
locked    coin   unlock  unlocked
final state: unlocked
actions: {"alarm": 2, "pass": 1, "refund": 1, "unlock": 2}
unknown event: no transition for kick in unlocked
```

## monthly

What a CSV says, month by month, without holding the CSV — the two things a report over exported data needs and could not have before: rows read one at a time (a row is not a line, since a quoted field may hold line breaks) and a date column that is read rather than guessed at.  ting monthly.ting                 # report on a file this makes ting monthly.ting sales.csv       # report on a file of your own cat sales.csv | ting monthly.ting -  The columns are asked for by name — a date column and an amount column, whatever else the file carries and in whatever order. With no argument it builds a file, reports on it and removes it, so the example prints the same thing every time — and that file is 5001 rows in 6001 lines, so a reader that cut on newlines would invent a thousand rows that are not there.

```ting
# What a CSV says, month by month, without holding the CSV — the two
# things a report over exported data needs and could not have before:
# rows read one at a time (a row is not a line, since a quoted field
# may hold line breaks) and a date column that is read rather than
# guessed at.
#
#   ting monthly.ting                 # report on a file this makes
#   ting monthly.ting sales.csv       # report on a file of your own
#   cat sales.csv | ting monthly.ting -
#
# The columns are asked for by name — a date column and an amount
# column, whatever else the file carries and in whatever order. With no argument
# it builds a file, reports on it and removes it, so the example
# prints the same thing every time — and that file is 5001 rows in
# 6001 lines, so a reader that cut on newlines would invent a
# thousand rows that are not there.

let csv = import("../lib/csv.ting");
let tm = import("../lib/time.ting");

fn build(path) {
  let rows = [["date", "customer", "note", "amount"]];
  let day = 86400000;
  let start = tm["from_iso"]("2026-01-05");
  for i in range(5000) {
    let when = tm["iso"](start + i * day / 40);
    # Every fifth note carries a comma and a line break, which is
    # what makes the file more rows than lines.
    let note = "plain";
    if i % 5 == 0 { note = "urgent, see\nthe attached sheet"; }
    let date = when;
    # Three rows carry a date nothing can read. Exports do this.
    if i == 1200 { date = "not a date"; }
    if i == 2400 { date = "2026-02-30"; }
    if i == 3600 { date = ""; }
    push(rows, [date, format("customer {}", i % 97), note, format("{}.{}", 10 + i % 90, i % 100)]);
  }
  write_file(path, csv["text"](rows));
  return path;
}

let given = args();
let path = "monthly-demo.csv";
let mine = len(given) == 0;
if !mine { path = given[0]; }
if mine { build(path); }

if path != "-" && !exists(path) {
  eprint(format("monthly: no such path: {}", path));
  exit(2);
}

# One pass. What is held: one counter and one total per month, and the
# row in hand — a map, because each_map reads the header row and names
# every later row with it, so a column is asked for by the name it has
# in the file rather than by a number this program has to find first.
let months = {};
let counts = {};
let unreadable = 0;
let usable = nil;

let rows = csv["each_map"](path, fn(row) {
  # A row's keys are the header, so the first row answers whether the
  # file has the columns this report needs. A file with a header and
  # no rows never gets asked, and does not need to be: the report on
  # it is empty whatever its columns are called.
  if usable == nil { usable = has(row, "date") && has(row, "amount"); }
  if !usable { return false; }
  let when = tm["from_iso"](row["date"]);
  # A date that cannot be read is counted, not guessed at, and not
  # allowed to stop the report.
  if when == nil {
    unreadable += 1;
    return nil;
  }
  let month = slice(tm["date"](when), 0, 7);
  let cents = int(float(row["amount"]) * 100.0 + 0.5);
  months[month] = get(months, month, 0) + cents;
  counts[month] = get(counts, month, 0) + 1;
  return nil;
});

if usable == false {
  eprint("monthly: the header has no date and amount columns");
  exit(2);
}

let size = "";
if path != "-" { size = format(", {} bytes", stat(path)["size"]); }
print(format("{}: {} rows{}", path, rows, size));
print("held while reading: a total per month, and one row under its column names");
print("");

print("by month");
for month in keys(months) {
  print(format("  {}  {:>5} rows  {:>12.2}", month, counts[month],
  float(months[month]) / 100.0));
}
print("");
print(format("dates nothing could read: {}", unreadable));

if mine { remove_file(path); }
```

```text
monthly-demo.csv: 5000 rows, 250966 bytes
held while reading: a total per month, and one row under its column names

by month
  2026-01   1080 rows      59431.15
  2026-02   1119 rows      60606.95
  2026-03   1239 rows      68464.45
  2026-04   1199 rows      65732.60
  2026-05    360 rows      19822.35

dates nothing could read: 3
```

## organize

Filing a directory by the local day each file was last written — the tidy-up a script could not do before, because moving a file meant reading it and writing it somewhere else, which stamps the copy with the moment it was made and destroys the very dates it is sorting by.  ting organize.ting             # file a small tree this makes ting organize.ting downloads   # file a directory of your own  With no argument it builds a directory, files it, reports and removes it, so the example prints the same thing every time. Its files are all made moments apart, so they all belong to one day; a directory with some history in it spreads over as many folders as it has days. A real one also holds files that are not text, which read_file cannot open and this moves without noticing.

```ting
# Filing a directory by the local day each file was last written — the
# tidy-up a script could not do before, because moving a file meant
# reading it and writing it somewhere else, which stamps the copy with
# the moment it was made and destroys the very dates it is sorting by.
#
#   ting organize.ting             # file a small tree this makes
#   ting organize.ting downloads   # file a directory of your own
#
# With no argument it builds a directory, files it, reports and
# removes it, so the example prints the same thing every time. Its
# files are all made moments apart, so they all belong to one day; a
# directory with some history in it spreads over as many folders as it
# has days. A real one also holds files that are not text, which
# read_file cannot open and this moves without noticing.

let fs = import("../lib/fs.ting");
let tm = import("../lib/time.ting");

fn build(root) {
  fs["remove_tree"](root);
  make_dir(root + "/old/deep");
  write_file(root + "/notes.txt", "some notes\n");
  write_file(root + "/report.md", "# a report\n");
  write_file(root + "/script.ting", "print(1);\n");
  write_file(root + "/old/notes.txt", "older notes, same name\n");
  write_file(root + "/old/deep/data.csv", "a,b\n1,2\n");
  return root;
}

# Two files can carry the same name in different directories, and
# rename replaces its target without asking. Numbering the second one
# is the difference between a tidy-up and a deletion.
fn free_name(target) {
  if !exists(target) { return target; }
  let tail = "";
  if fs["ext"](target) != "" { tail = "." + fs["ext"](target); }
  let n = 2;
  while true {
    let candidate = fs["dir"](target) + "/" + fs["stem"](target) + "-" + str(n) + tail;
    if !exists(candidate) { return candidate; }
    n += 1;
  }
}

# The day a file was written, where it was written. A file's date is
# an instant, and an instant is not a day until you say whose: at half
# past midnight here it is already tomorrow, and UTC still calls it
# yesterday. The platform is the only source for that, so where it
# keeps none local_zone answers nil and this falls back to the UTC
# day, having said so. It is asked per file rather
# than once, because a directory with a year of history in it spans
# summer time changes.
fn local_day(ms) {
  let day = tm["local_date"](ms);
  if day == nil { return tm["date"](ms); }
  return day;
}

# Every file at or below the root into a folder named for its day.
# The facts are read once, before anything moves, because moving is
# what changes them.
fn file_all(root) {
  let moved = 0;
  let renamed = 0;
  let days = {};
  for f in fs["facts"](root) {
    let day = local_day(f["modified"]);
    days[day] = get(days, day, 0) + 1;
    let target = root + "/" + day + "/" + fs["base"](f["path"]);
    # Already where it belongs: filing again must move nothing.
    if f["path"] == target { continue; }
    make_dir(root + "/" + day);
    let free = free_name(target);
    if free != target { renamed += 1; }
    fs["move"](f["path"], free);
    moved += 1;
  }
  return {"moved": moved, "renamed": renamed, "days": len(keys(days))};
}

# A tidy-up that leaves the emptied directories behind is not tidy.
fn prune(d) {
  for child in fs["entries"](d) {
    if is_dir(child) { prune(child); }
  }
  if len(list_dir(d)) == 0 { remove_dir(d); }
}

let given = args();
let root = "organize-demo";
let mine = len(given) == 0;
if !mine { root = given[0]; }
if mine { build(root); }

if !exists(root) {
  eprint(format("organize: no such path: {}", root));
  exit(2);
}

# On stderr, not in the report: a folder named for the wrong day is
# the mistake this example exists to avoid, so a machine that cannot
# name the right one has to say it out loud rather than file quietly.
if local_zone() == nil {
  eprint("organize: this machine keeps no time zone data; filing by the UTC day");
}

# The dates as they were. If any of them changes, the filing was done
# by copying and the folders are now a record of when it ran.
let before = sort(map(fs["facts"](root), fn(f) { return f["modified"]; }));
print(format("{}: {} files", root, len(before)));
print("");

let done = file_all(root);
prune(root);
let after = sort(map(fs["facts"](root), fn(f) { return f["modified"]; }));

let loose = 0;
for child in fs["entries"](root) {
  if !is_dir(child) { loose += 1; }
}

let folders = "folders";
if done["days"] == 1 { folders = "folder"; }
print(format("  filed into {} {}", done["days"], folders));
print(format("  numbered to keep a name clash from deleting one: {}", done["renamed"]));
print(format("  every date survived: {}", after == before));
print(format("  left at the top: {}", loose));
print(format("  filing it again moves: {}", file_all(root)["moved"]));

if mine { fs["remove_tree"](root); }
```

```text
organize-demo: 5 files

  filed into 1 folder
  numbered to keep a name clash from deleting one: 1
  every date survived: true
  left at the top: 0
  filing it again moves: 0
```

## pipeline

A data pipeline over stdin: one "name,region,amount" record per line (try `cat sales.csv | ting pipeline.ting`). With nothing on stdin it runs on a built-in sample so the output is reproducible.

```ting
# A data pipeline over stdin: one "name,region,amount" record per line
# (try `cat sales.csv | ting pipeline.ting`). With nothing on stdin it
# runs on a built-in sample so the output is reproducible.

let li = import("../lib/list.ting");
let st = import("../lib/string.ting");
let ma = import("../lib/map.ting");

let text = read_file("-");
if st["is_blank"](text) {
  text = "ann,north,120\nbob,south,80\nann,north,40\ncid,east,x\nbob,north,60\n";
  print("(no stdin; using the built-in sample)");
}

let records = [];
for line in st["lines"](trim(text)) {
  let fields = split(line, ",");
  if len(fields) != 3 || !st["is_digit"](fields[2]) {
    print("skipping:", line);
    continue;
  }
  push(records, {"name": fields[0], "region": fields[1], "amount": int(fields[2])});
}

let by_region = li["group_by"](records, fn(r) { return r["region"]; });
let rows = [["region", "records", "total", "mean"]];
for region in keys(by_region) {
  let amounts = map(by_region[region], fn(r) { return r["amount"]; });
  push(rows, [region, str(len(amounts)), str(li["sum"](amounts)), str(li["mean"](amounts))]);
}
print(st["table"](rows));

let per_name = li["count_by"](records, fn(r) { return r["name"]; });
print("most records:", ma["top"](per_name, 1)[0][0]);
```

```text
(no stdin; using the built-in sample)
skipping: cid,east,x
region  records  total  mean
north   3        220    73.33333333333333
south   1        80     80.0
most records: ann
```

## ranking

Putting records in order by more than one thing: a list is a compound key, because lists compare element by element, and compare() is that same order as a number when the directions differ.

```ting
# Putting records in order by more than one thing: a list is a
# compound key, because lists compare element by element, and
# compare() is that same order as a number when the directions differ.

let li = import("../lib/list.ting");

let runners = [
  { "name": "Ada", "points": 12, "time": 91.4 },
  { "name": "Bo", "points": 15, "time": 88.2 },
  { "name": "Cai", "points": 12, "time": 87.9 },
  { "name": "Dev", "points": 15, "time": 90.1 },
];

# One direction: the key is a list, read left to right. Points first,
# name second, so a tie on points is settled alphabetically.
for r in sort_by(runners, fn(r) { return [r["points"], r["name"]]; }) {
  print(r["points"], r["name"]);
}

print("");

# Mixed directions need a comparator, and compare() makes it one line
# per field: most points first, and the faster time breaks a tie.
let ranked = li["sort_with"](runners, fn(x, y) {
  let by_points = compare(y["points"], x["points"]);
  if by_points != 0 { return by_points; }
  return compare(x["time"], y["time"]);
});
for i in range(len(ranked)) {
  print(i + 1, ranked[i]["name"], ranked[i]["points"], ranked[i]["time"]);
}

print("");

# A frequency table sorts itself: items are [key, count] pairs, and a
# pair is a list like any other.
let tally = li["frequencies"](["red", "blue", "red", "green", "red", "blue"]);
print(sort(items(tally)));

# The busiest colour, without sorting at all: max reads the same order,
# so a [count, name] pair puts count first.
let pairs = [];
for c in keys(tally) { push(pairs, [tally[c], c]); }
print("most seen:", max(pairs)[1]);
```

```text
12 Ada
12 Cai
15 Bo
15 Dev

1 Bo 15 88.2
2 Dev 15 90.1
3 Cai 12 87.9
4 Ada 12 91.4

[["blue", 2], ["green", 1], ["red", 3]]
most seen: red
```

## records

A CSV read into records, changed, and written back out as the file it came from — the round trip a program actually does, and the one place hand-joining the fields quietly ruins the file.  ting records.ting              # a file this makes ting records.ting prices.csv   # a file of your own  Records are what a program wants to hold: a column is asked for by name, not by a number someone has to count. A map has no order, though, so the header the file arrived with is kept and handed back to rows() — otherwise the columns come out alphabetical, which is a different file. With no argument it builds a file, rewrites it, reports and removes it, so the example prints the same thing every time.

```ting
# A CSV read into records, changed, and written back out as the file
# it came from — the round trip a program actually does, and the one
# place hand-joining the fields quietly ruins the file.
#
#   ting records.ting              # a file this makes
#   ting records.ting prices.csv   # a file of your own
#
# Records are what a program wants to hold: a column is asked for by
# name, not by a number someone has to count. A map has no order,
# though, so the header the file arrived with is kept and handed back
# to rows() — otherwise the columns come out alphabetical, which is a
# different file. With no argument it builds a file, rewrites it,
# reports and removes it, so the example prints the same thing every
# time.

let csv = import("../lib/csv.ting");
let st = import("../lib/string.ting");

fn build(path) {
  write_file(path, csv["text"]([
    ["sku", "name", "note", "price"],
    ["A-1", "hex bolt", "plain", "0.40"],
    ["A-2", "wing nut", "sold in tens, see\nthe sheet", "1.25"],
    ["B-7", "\"long\" bracket", "last of them", "3.10"],
  ]));
}

let path = "records-demo.csv";
let mine = len(args()) > 0;
if mine { path = args()[0]; } else { build(path); }

let parsed = csv["parse"](read_file(path));
let header = parsed[0];
let records = csv["maps"](parsed);
print(path + ":", len(records), "records,", len(header), "columns:", join(header, ", "));

# The change: ten per cent on, and a column added to say so. A new
# column is why the header is rebuilt rather than reused as it is.
for r in records {
  r["price"] = format("{:.2}", float(r["price"]) * 1.1);
  r["raised"] = "yes";
}
let columns = header + ["raised"];

# On screen, a table of the columns worth looking at: stating them is
# choosing them, which leaves out the note — it holds a line break,
# and a cell with a line break in it is not a row of a table.
print(st["table"](csv["rows"](records, ["sku", "name", "price", "raised"])));

# Out the way it came in. The note with a comma and a line break in
# it, and the name with quotes around part of it, survive because
# text() quotes them — which is what a hand-rolled join(fields, ",")
# loses, silently, on exactly these rows.
write_file(path, csv["text"](csv["rows"](records, columns)));
let back = csv["maps"](csv["parse"](read_file(path)));
print("written back:", len(read_file(path)), "bytes;", "same records:", back == records);

# The shape is checked now, so the way NOT to do it says so.
print("records straight to text:", try(csv["text"], records)["err"]);

if !mine { remove_file(path); }
```

```text
records-demo.csv: 3 records, 4 columns: sku, name, note, price
sku  name            price  raised
A-1  hex bolt        0.44   yes
A-2  wing nut        1.38   yes
B-7  "long" bracket  3.41   yes
written back: 152 bytes; same records: true
records straight to text: text: a row is a list of fields, not a record — pass rows(records)
```

## report

A small report, and a whole command-line program: the three modules a script reaches for first — a command line, delimited input, and something to say when it goes wrong.  The front door is main(), which is parse() with the two things every program does around it: --help prints the help built from this same spec and leaves with 0, and a command line the spec does not describe prints its trouble and that help to stderr and leaves with 2. Run it for yourself:  ting examples/report.ting --help ting examples/report.ting --by rep sales.csv ting examples/report.ting --nope  Named no file — which is how it runs here, and how the .out beside it was made — it reports on the small table written below, so the example prints the same thing every time.

```ting
# A small report, and a whole command-line program: the three modules
# a script reaches for first — a command line, delimited input, and
# something to say when it goes wrong.
#
# The front door is main(), which is parse() with the two things every
# program does around it: --help prints the help built from this same
# spec and leaves with 0, and a command line the spec does not
# describe prints its trouble and that help to stderr and leaves with
# 2. Run it for yourself:
#
#   ting examples/report.ting --help
#   ting examples/report.ting --by rep sales.csv
#   ting examples/report.ting --nope
#
# Named no file — which is how it runs here, and how the .out beside
# it was made — it reports on the small table written below, so the
# example prints the same thing every time.

let cli = import("../lib/args.ting");
let csv = import("../lib/csv.ting");
let err = import("../lib/err.ting");

let spec = {
  "name": "report",
  "summary": "totals by column from a CSV",
  "flags": [{"long": "quiet", "short": "q", "help": "no header line"}],
  "options": [{"long": "by", "short": "b", "value": "COLUMN", "help": "column to group by", "default": "region"}],
  "positionals": [{"name": "file", "help": "the CSVs to read", "many": true}],
};

let opts = cli["main"](spec, args());

let sample = "region,rep,amount\n" +
"north,\"Smith, J\",120\n" +
"south,Okafor,340\n" +
"north,\"O\"\"Neill\",95\n" +
"south,Tanaka,210\n";

let rows = [];
let files = opts["positionals"]["file"];
if len(files) == 0 {
  for row in csv["maps"](csv["parse"](sample)) { push(rows, row); }
} else {
  for path in files {
    for row in csv["maps"](csv["parse"](read_file(path))) { push(rows, row); }
  }
}

let column = opts["options"]["by"];
let totals = {};
for row in rows {
  let key = row[column];
  totals[key] = get(totals, key, 0) + int(row["amount"]);
}

# The output is CSV too, written by the same module that read it.
let out = [];
if !opts["flags"]["quiet"] { push(out, [column, "total"]); }
for key in sort(keys(totals)) { push(out, [key, totals[key]]); }
print(trim(csv["text"](out)));
print("");

# A field with a comma and a field with a quote both came back whole.
for row in rows { print(row["rep"]); }
print("");

# The command lines this run was not given. What --help prints, and
# leaves with 0:
print(cli["help"](spec));
print("");

# and what goes to stderr ahead of it, before leaving with 2: the
# parser fails with a message the program can print, rather than
# guessing.
print(err["message"](fn() { return cli["parse"](spec, ["--nope", "x"]); }));
print(err["message"](fn() { return cli["parse"](spec, ["--by"]); }));
```

```text
region,total
north,215
south,550

Smith, J
Okafor
O"Neill
Tanaka

report — totals by column from a CSV

usage: report [options] [file...]

options:
  -b, --by COLUMN  column to group by (default region)
  -q, --quiet      no header line
  -h, --help       show this and leave

unknown option --nope
--by needs a value
```

## series

A numeric series: two weeks of daily temperatures summarised with extent, mean, median, mode and a percentile, smoothed with a three-day moving average, and split into warm and cool runs.

```ting
# A numeric series: two weeks of daily temperatures summarised with
# extent, mean, median, mode and a percentile, smoothed with a
# three-day moving average, and split into warm and cool runs.

let li = import("../lib/list.ting");
let ma = import("../lib/math.ting");

let temps = [18, 19, 21, 24, 24, 23, 20, 17, 16, 16, 19, 22, 25, 24];

let span = li["extent"](temps);
print("days:", len(temps), "range:", span[0], "to", span[1]);
print("mean:", ma["round"](li["mean"](temps)), "median:", li["median"](temps),
"mode:", li["mode"](temps));
print("90th percentile:", ma["percentile"](temps, 90));

# Three-day moving average, one decimal.
let smooth = map(li["window"](temps, 3), fn(w) {
  return ma["round"](li["mean"](w) * 10) / 10.0;
});
print("moving average:", smooth);

# Runs of days at or above the mean ("warm") and below it ("cool").
let avg = li["mean"](temps);
let runs = li["chunk_by"](temps, fn(t) { return t >= avg; });
for run in runs {
  let label = "cool";
  if run[0] >= avg { label = "warm"; }
  print(label, "run of", str(len(run)) + ":", run);
}
```

```text
days: 14 range: 16 to 25
mean: 21 median: 20.5 mode: 24
90th percentile: 24
moving average: [19.3, 21.3, 23.0, 23.7, 22.3, 20.0, 17.7, 16.3, 17.0, 19.0, 22.0, 23.7]
cool run of 2: [18, 19]
warm run of 4: [21, 24, 24, 23]
cool run of 5: [20, 17, 16, 16, 19]
warm run of 3: [22, 25, 24]
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
      j -= 1;
    }
    out[j + 1] = key;
    i += 1;
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

## stamps

Timestamps as other programs wrote them. An access log and a syslog line carry the same instant in two different shapes, and neither is ISO 8601: time.parse reads a shape stated in the strftime codes the world already describes these with, and time.text writes one back.  A stamp that does not match answers nil, because a line out of a file may be anything. A syslog line carries no year at all, so the year is supplied rather than guessed.

```ting
# Timestamps as other programs wrote them. An access log and a syslog
# line carry the same instant in two different shapes, and neither is
# ISO 8601: time.parse reads a shape stated in the strftime codes the
# world already describes these with, and time.text writes one back.
#
# A stamp that does not match answers nil, because a line out of a
# file may be anything. A syslog line carries no year at all, so the
# year is supplied rather than guessed.

let time = import("../lib/time.ting");
let li = import("../lib/list.ting");

let access = [
  "10.0.0.7 - - [12/Sep/2026:06:00:01 +0000] \"GET / HTTP/1.1\" 200 1043",
  "10.0.0.9 - - [12/Sep/2026:06:14:52 +0000] \"GET /docs HTTP/1.1\" 200 8122",
  "10.0.0.7 - - [12/Sep/2026:07:02:10 +0000] \"POST /login HTTP/1.1\" 302 0",
  "10.0.0.3 - - [whenever] \"GET /favicon.ico HTTP/1.1\" 404 209",
  "10.0.0.9 - - [12/Sep/2026:07:41:33 +0000] \"GET /search HTTP/1.1\" 200 3310",
];

let hits = [];
for line in access {
  let field = split(split(line, "[")[1], "]")[0];
  let at = time["parse"](field, "%d/%b/%Y:%H:%M:%S %z");
  if at == nil {
    print("not a stamp:", field);
    continue;
  }
  push(hits, at);
}

print("first:", time["iso"](hits[0]));
print("last: ", time["iso"](hits[len(hits) - 1]));
print("span: ", time["span"](hits[len(hits) - 1] - hits[0]));
print("hits by hour:", li["count_by"](hits, fn(ms) { return time["text"](ms, "%H:00"); }));

# Syslog writes the month by name, pads the day with a space, and
# says nothing about the year: whoever reads the file is expected to
# know. Saying so is one argument.
let syslog = [
  "Sep  3 06:00:03 web-1 sshd[4123]: Accepted publickey for deploy",
  "Sep 12 07:02:11 web-1 sudo: deploy : TTY=pts/0 ; COMMAND=/bin/systemctl",
];

print("syslog:");
for line in syslog {
  let at = time["parse"](slice(line, 0, 15), "%b %e %H:%M:%S", {"year": 2026});
  print("  " + time["iso"](at), slice(line, 16, len(line)));
}

# And the other direction: one instant, in the shapes each of those
# tools expects to be handed.
print("the first hit, written for:");
for pattern in ["%F %T", "%d/%b/%Y:%H:%M:%S %z", "%b %e %H:%M:%S", "%A %B %e, %Y at %I:%M %p"] {
  print("  " + pattern + "  ->  " + time["text"](hits[0], pattern));
}
```

```text
not a stamp: whenever
first: 2026-09-12T06:00:01Z
last:  2026-09-12T07:41:33Z
span:  1h 41m 32s
hits by hour: {"06:00": 2, "07:00": 2}
syslog:
  2026-09-03T06:00:03Z web-1 sshd[4123]: Accepted publickey for deploy
  2026-09-12T07:02:11Z web-1 sudo: deploy : TTY=pts/0 ; COMMAND=/bin/systemctl
the first hit, written for:
  %F %T  ->  2026-09-12 06:00:01
  %d/%b/%Y:%H:%M:%S %z  ->  12/Sep/2026:06:00:01 +0000
  %b %e %H:%M:%S  ->  Sep 12 06:00:01
  %A %B %e, %Y at %I:%M %p  ->  Saturday September 12, 2026 at 06:00 AM
```

## stats

Descriptive statistics over a fixed sample, using lib/math.ting, lib/list.ting and range with a step.

```ting
# Descriptive statistics over a fixed sample, using lib/math.ting,
# lib/list.ting and range with a step.

let ma = import("../lib/math.ting");
let li = import("../lib/list.ting");

# Every third value in [2, 60): 2, 5, 8, ...
let sample = range(2, 60, 3);

let n = len(sample);
let mean = li["mean"](sample);

let stddev = ma["stddev"](sample);

print("n      =", n);
print("min    =", min(sample), " max =", max(sample));
print("mean   =", mean, " median =", li["median"](sample));
print(format("stddev = {:.2}", stddev));
print("gcd of extremes =", ma["gcd"](min(sample), max(sample)));
```

```text
n      = 20
min    = 2  max = 59
mean   = 30.5  median = 30.5
stddev = 17.30
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

## text

Working with text: the most common words across some titles, a URL slug for each, a paragraph wrapped to a narrow column, and a "did you mean" built on edit distance.

```ting
# Working with text: the most common words across some titles, a
# URL slug for each, a paragraph wrapped to a narrow column, and a
# "did you mean" built on edit distance.

let li = import("../lib/list.ting");
let st = import("../lib/string.ting");

let titles = [
  "Zero-Dependency Scripting in Rust",
  "Why Two Engines Beat One",
  "Testing the Formatter with Random Programs",
  "The Formatter, Byte for Byte",
  "Scripting the Shell Without Surprises",
];

# Word frequencies over every title, lowercased.
let all_words = [];
for t in titles {
  for w in st["words"](lower(t)) { push(all_words, w); }
}
let freq = li["frequencies"](all_words);
let ranked = sort_by(keys(freq), fn(w) { return -freq[w]; });
print("top words:", li["take"](ranked, 3));

# Slugs: what each title looks like in a URL.
for t in titles { print(st["slug"](t)); }

# A paragraph wrapped at 36 columns.
let blurb = "ting is a tiny scripting language whose whole toolchain "
+ "lives in one binary: a runner, a formatter, a checker, a test "
+ "runner and a language server.";
print(st["wrap"](blurb, 36));

# Did you mean: the known word closest to a typo by edit distance.
let known = ["formatter", "scripting", "engines", "programs"];
for typo in ["fromatter", "scriptng", "enginee"] {
  let best = li["min_by"](known, fn(w) { return st["levenshtein"](w, typo); });
  print(typo, "->", best, "(distance " + str(st["levenshtein"](best, typo)) + ")");
}
```

```text
top words: ["the", "byte", "scripting"]
zero-dependency-scripting-in-rust
why-two-engines-beat-one
testing-the-formatter-with-random-programs
the-formatter-byte-for-byte
scripting-the-shell-without-surprises
ting is a tiny scripting language
whose whole toolchain lives in one
binary: a runner, a formatter, a
checker, a test runner and a
language server.
fromatter -> formatter (distance 2)
scriptng -> scripting (distance 1)
enginee -> engines (distance 1)
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
  # A missing or corrupt file is not an error here: start fresh.
  return get(try(fn() { return json_parse(read_file(path)); }), "ok", []);
}

fn save(tasks) { write_file(path, json_str(tasks)); }

fn item_number(argv, tasks) {
  if len(argv) < 2 { fail("expected an item number"); }
  let n = int(argv[1]);
  if n < 1 || n > len(tasks) { fail(format("no item #{}", n)); }
  return n;
}

fn show(tasks) {
  if len(tasks) == 0 {
    print("nothing to do!");
    return nil;
  }
  let i = 0;
  for item in tasks {
    i += 1;
    let mark = " ";
    if item["done"] { mark = "x"; }
    print(format("{}. [{}] {}", i, mark, item["text"]));
  }
}

let argv = args();
let cmd = "list";
if len(argv) > 0 { cmd = argv[0]; }
let tasks = load();

if cmd == "list" {
  show(tasks);
} else if cmd == "add" {
  if len(argv) < 2 {
    print("add what?");
    exit(2);
  }
  push(tasks, {"text": join(slice(argv, 1, len(argv)), " "), "done": false});
  save(tasks);
  print(format("added #{}", len(tasks)));
} else if cmd == "done" || cmd == "rm" {
  let r = try(item_number, argv, tasks);
  if has(r, "err") {
    print("error:", r["err"]);
    exit(2);
  }
  let n = r["ok"];
  if cmd == "done" {
    tasks[n - 1]["done"] = true;
    print(format("done: {}", tasks[n - 1]["text"]));
  } else {
    let kept = [];
    let i = 0;
    for item in tasks {
      i += 1;
      if i != n { push(kept, item); }
    }
    tasks = kept;
    print(format("removed #{}", n));
  }
  save(tasks);
} else {
  print("usage: ting todo.ting [list | add <text> | done <n> | rm <n>]");
  exit(2);
}
```

```text
nothing to do!
```

## tree

What a directory holds, by size — the report a script cannot write from names alone, since a name says nothing about bytes or when a file was last written.  ting tree.ting            # report on a small tree this makes ting tree.ting src        # report on a directory of your own  With no argument it builds a tree, reports on it and removes it, so the example prints the same thing every time.

```ting
# What a directory holds, by size — the report a script cannot write
# from names alone, since a name says nothing about bytes or when a
# file was last written.
#
#   ting tree.ting            # report on a small tree this makes
#   ting tree.ting src        # report on a directory of your own
#
# With no argument it builds a tree, reports on it and removes it, so
# the example prints the same thing every time.

let fs = import("../lib/fs.ting");
let st = import("../lib/string.ting");

# Bytes as a person would say them: exact under a kilobyte, one
# decimal above it. 1024, not 1000 — the unit `ls -lh` uses.
fn human(n) {
  let units = ["KB", "MB", "GB"];
  if n < 1024 { return str(n) + " B"; }
  let size = float(n);
  let unit = "";
  for u in units {
    if size >= 1024.0 {
      size = size / 1024.0;
      unit = u;
    }
  }
  # One decimal place, without a formatting language: scale, round,
  # and put the point back.
  let tenths = int(size * 10.0 + 0.5);
  return str(tenths / 10) + "." + str(tenths % 10) + " " + unit;
}

fn build(root) {
  fs["remove_tree"](root);
  make_dir(root + "/src");
  make_dir(root + "/docs");
  write_file(root + "/README.md", "# a small tree\n");
  write_file(root + "/src/main.ting", "print(\"hello\");\n");
  write_file(root + "/src/util.ting", st["repeat"]("# a comment line\n", 40));
  write_file(root + "/docs/guide.md", st["repeat"]("Some prose.\n", 200));
  write_file(root + "/docs/notes.md", "Shorter.\n");
  return root;
}

let given = args();
let root = "tree-demo";
let mine = len(given) == 0;
if !mine { root = given[0]; }
if mine { build(root); }

# A path that is not there would otherwise report nothing at all,
# which reads like an empty directory rather than a mistake.
if !exists(root) {
  eprint(format("tree: no such path: {}", root));
  exit(2);
}

let rows = fs["facts"](root);
print(format("{}: {} files, {}", root, len(rows), human(fs["total_size"](root))));
print("");

print("largest");
let by_size = sort_with(rows, fn(a, b) { return b["size"] - a["size"]; });
for row in slice(by_size, 0, 3) {
  print(format("  {:>9} {}", human(row["size"]), row["path"]));
}
print("");

# Where the bytes are, rather than where the files are.
print("by extension");
let bytes = {};
for row in rows {
  let e = fs["ext"](row["path"]);
  if e == "" { e = "(none)"; }
  bytes[e] = get(bytes, e, 0) + row["size"];
}
for e in keys(bytes) {
  print(format("  {:>9} {}", human(bytes[e]), e));
}
print("");

# `modified` is milliseconds on the same clock `time_ms()` reads, so
# an age is a subtraction. Files written moments apart can share a
# stamp — the write is quicker than the clock — so this asks how
# recent they are rather than which one is newest.
let day = 24 * 60 * 60 * 1000;
let recent = 0;
for row in rows {
  if time_ms() - row["modified"] < day { recent += 1; }
}
print(format("changed in the last day: {} of {}", recent, len(rows)));

if mine { fs["remove_tree"](root); }
```

```text
tree-demo: 5 files, 3.0 KB

largest
     2.3 KB tree-demo/docs/guide.md
      680 B tree-demo/src/util.ting
       16 B tree-demo/src/main.ting

by extension
     2.4 KB md
      696 B ting

changed in the last day: 5 of 5
```
