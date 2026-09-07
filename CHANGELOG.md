# Changelog

All notable changes to ting. Versions are git tags; binaries for
Linux (x86-64 and arm64, glibc and fully static musl), macOS and
Windows are attached to each
[GitHub release](https://github.com/stefanobaghino/thing/releases).

## v2.127.0 (2026-09-07)

- A call on the right no longer costs a copy, where it can be shown
  not to matter. `s += str(i)` and `s += format(...)` are the loops
  people actually write, and they stayed quadratic because a call
  might reassign the name. That is decidable whenever no function in
  the file mentions the name: nothing a call does can reach it, so
  the append happens in place. 80000 pieces went from 1.7 seconds to
  0.03. Where some function does mention the name it could be
  assigned from inside the call, and there the old value is still
  read first.
- `x = x + y` costs what `x += y` costs. The long spelling read the
  name by copying what it held, so a loop that built a string or a
  list the obvious way paid the length of it every time round: 80000
  appends took 7.9 seconds for a string and 47.7 for a list, and now
  take 0.03 either way. The fusing applies only where the right-hand
  side cannot reach the name, and an unbound name is still reported
  as the read it is written as.
- Growing a list with `xs += [x]` is linear. `+` copied the whole
  list every time round, so building one in a loop cost the square of
  its length: 200000 appends took 47 seconds and now take 0.02. When
  nothing else holds the list it is extended in place, and when
  anything does — a second name, a list or map it sits in, a closure
  that captured it, a snapshot pushed onto another list — it is
  copied exactly as before. `bench/growth.ting` measures it and
  `tests/alloc.rs` weighs the bytes so it cannot quietly come back.
- The docs link in `--help` and in the README is `https`. The site
  answers on both, and nothing about a language reference should
  travel in the clear when it need not.
- The browser playground shows the current examples again. Six of the
  fifteen had been left at their pre-2.107 versions because the guard
  compared only each example's first line; it compares whole bodies
  now.

## v2.126.0 (2026-09-07)

- The release page now carries a `SHA256SUMS` file and says what was
  checked. Every archive is unpacked and run on a machine of its own
  platform before upload and again after it, and the notes say so —
  written by a job that only runs when all six targets have done it,
  so the claim cannot outlive the check that backs it. The checksums
  are for arriving intact, not for provenance, and the notes say that
  too: they tell you the bytes you have are the bytes the page
  serves, and nothing about who made them.
- The downloaded archive is unpacked by each platform's own tool.
  v2.125.0's release went red on Windows because `tar` under Git Bash
  is GNU tar rather than the bsdtar in `System32`, and GNU tar does
  not read zip; the zip now gets PowerShell, which had already
  unpacked the packaged copy on that same runner minutes earlier.

## v2.125.0 (2026-09-07)

- **Every released archive is now started before anyone is offered
  it.** The release workflow built each archive, checked the glibc
  floor on Linux, packaged and uploaded — and never once ran the
  thing it packaged. Six archives a release since v2.30.0, ninety-five
  releases, and the only ones ever executed were the two aarch64 ones
  someone downloaded by hand afterwards: four in six went out having
  never been started at all. Each archive is now unpacked on its own
  runner and put to work — the binary run from where it was unpacked,
  the self-hosted suite run against the stdlib compiled into it, and
  every example diffed against its recorded output — before the
  upload step is reached. A target whose archive will not start now
  leaves that archive missing from the release rather than present
  and broken.
- The same check runs in CI on every push, against a directory laid
  out the way an archive is, so a release is never the first place it
  is tried on macOS or Windows.
- **The archive that is run is the one you download.** Running what
  the build produced and running what the release page serves are two
  different claims; everything in between — the upload, the storage,
  the asset name — was untested until something fetched it back. Each
  archive is now downloaded from the finished release and put through
  the same checks a second time.
- **The `lib/` in the archive is checked against the one compiled
  into the binary.** They are two copies of the same twelve modules,
  and a packaging step that shipped a stale or partial copy would
  have been invisible: the binary keeps working on its embedded copy
  while a script unpacked beside the archive quietly gets the other.
- `selftest/sh.ting` counted one check per entry in `PATH`, so the
  suite reported a different total on every machine that ran it — 82
  entries on a Windows runner against 11 on the machine this is
  written on. It is one check now, and the four platforms agree on
  2545.

## v2.124.0 (2026-09-07)

- `local_zone` answers on **Windows**. Until now it was `nil` there,
  which meant a quarter of the release archives could not tell what
  day it was locally: `examples/organize.ting` filed every file by
  the UTC day for every Windows user, which is the bug the example
  had just been fixed to avoid. Windows keeps no TZif file — its zone
  data is in the registry, in a shape of its own — so this asks the
  system for it instead: the year's rules from
  `GetTimeZoneInformationForYear`, applied by
  `SystemTimeToTzSpecificLocalTime`. The offset is the difference the
  system itself computed rather than one derived here from a rule, so
  the transitions are Windows' own. `TZ` is not consulted there,
  because the Windows clock does not consult it either.
- `abbr` says what each platform calls a period, and the platforms
  differ. A zone file holds a real abbreviation (`"CEST"`) or, for a
  zone that never had one, the offset itself (`"+1245"`). Windows has
  no abbreviations: it holds full names, in the language the system
  is installed in, so `abbr` reads `"W. Europe Daylight Time"` there.
  A program comparing `abbr` to a fixed string was already wrong on
  Chatham; a program printing it is right everywhere. Handing back a
  number `offset` already holds would have made the platforms look
  alike by making one of them say less.
- The browser playground still answers `nil`, and now says so for the
  right reason rather than by falling through the Unix path.
- The Windows answers are held to the answers a zone file gives. Six
  instants in Zurich — including the minute either side of both 2026
  transitions — are checked against the same values `date` reports,
  through the registry rather than through a file. Windows lets a
  caller ask for a named zone's rules rather than the machine's,
  which is what makes that checkable on a machine sitting in UTC.
- The guard that keeps HTML out of the markdown had been skipping
  everything after one paragraph written in June: it took a line
  beginning with an inline code span for a code fence, and nothing
  closed the fence it opened. Fifty-six iterations of the log went
  unchecked while it reported success. A fence is now a backtick run
  that is not closed again on the same line.
- How far back the answer reaches differs by platform, and the
  reference now says so. A zone file records a century of changes;
  Windows keeps per-year rules for a couple of decades and applies
  the earliest it has to anything older, so July 1980 in Zurich is
  `+01:00` from a zone file and `+02:00` from Windows.

## v2.123.0 (2026-09-07)

- `local_zone()` is the 73rd builtin: the local time zone at an
  instant — a map of `offset` (milliseconds east of UTC), `abbr` and
  `dst`, or `nil` where the platform keeps nothing to read. Until now
  ting had no idea what time it was anywhere but UTC, and that was
  not cosmetic: `examples/organize.ting` files each file into a
  folder named for the day it was written, and a file written at half
  past midnight went into yesterday's. Rust's standard library has no
  local-time API, so this reads what the system already keeps — the
  TZif file (RFC 8536) at `/etc/localtime`, or the one `TZ` names
  under `/usr/share/zoneinfo`. It answers for an instant rather than
  for now, because a report over last winter's timestamps needs last
  winter's offset: the file records every change a zone has been
  through, so June 1980 in Zurich is `+01:00` (the country kept no
  summer time until 1981) and 1874 is `+00:29:46` local mean time —
  which is why an offset is a whole number of seconds but not always
  of minutes. A `TZ` that carries a POSIX rule rather than a name,
  and a platform with no such file, both answer `nil` rather than a
  guess, so a caller can tell "unknown" from a real zero.
- `examples/organize.ting` files by the **local** day. It sorts files
  into folders named for the day each was last written, and until now
  that was the UTC day: two files an hour apart across local midnight
  both went into the earlier folder. They now go into the two folders
  they belong in. Where the platform keeps no zone data the example
  falls back to the UTC day and says so on stderr rather than filing
  quietly — a folder named for the wrong day is the mistake it exists
  to avoid. The day is asked per file, not once, because a directory
  with a year of history in it spans summer time changes.
- `lib/time.ting` gained `local_date(ms)`, `local_clock(ms)`,
  `local_iso(ms)` and `offset_iso(off)`, so a caller stops writing
  `+ local_zone(ms)["offset"]` by hand. `local_iso` writes the
  instant so that it carries where "here" was —
  `2026-09-07T00:30:00+02:00` — and `from_iso` reads it back to the
  same instant, which makes the pair a round trip rather than two
  half-measures. The three that need a zone answer `nil` where the
  platform keeps none, rather than handing back the UTC answer under
  a local name: a caller that wants UTC as a fallback says so in its
  own code, where a reader can see it, which is exactly what
  `examples/organize.ting` does in order to warn. `offset_iso`
  truncates a sub-minute offset towards zero, since ISO 8601 has no
  room for seconds in one and only the local mean times before about
  1900 have them — Zurich's `+00:29:46` writes as `+00:29`, as `date`
  prints it.

## v2.122.0 (2026-09-07)

- `lib/csv.ting` gained `each_map(path, f, sep = ",")`: what `maps`
  does to a parsed document, done to a file a row at a time. The
  first row is the header and every later row arrives as a map, so a
  column is asked for by name instead of by a number the caller has
  to find in the header and then carry through the read — the
  bookkeeping that `examples/monthly.ting` spends fourteen lines on.
  The naming is nearly free: on a 9.4 MB export of 300000 rows it
  costs about a tenth more time than `each_row` and the same 10 MB,
  because only the row in hand becomes a map. As with `each_row`
  there is no second implementation — `entry_of(header, row)` is now
  the one place a row is given its column names, and `maps` and
  `each_map` both call it, so a file read either way is named
  identically.
- `examples/monthly.ting` reads its rows by name. Ten lines of header
  bookkeeping — a first-row branch scanning for two column names, the
  two numbers it found carried out of the callback, and a guard on
  every row after — are gone, replaced by `row["date"]` and
  `row["amount"]`. Every number the report prints is unchanged, which
  is the point: the same file, the same five month totals, the same
  three dates nothing could read. The one output line that moved says
  what is held while reading, which is now a row under its column
  names rather than a pair of column numbers.
- A byte order mark at the head of a document is no longer content.
  A spreadsheet exporting a CSV writes one, and behind it the first
  column is called `\ufeffdate` rather than `date` — so the column
  asked for by name is not found, and `monthly.ting` refused a
  perfectly good export with "the header has no date and amount
  columns". `json_parse` was worse: it reported "unexpected character
  at offset 0" for a document that is fine apart from a mark RFC 8259
  says a parser may ignore. Both now skip one, and only one, and only
  at the head — inside a string, in a later field, or a second mark
  is a character like any other. In `lib/csv.ting` the skip lives in
  the shared scanner, so a file read whole and a file read a row at a
  time drop the same mark.

## v2.121.0 (2026-09-07)

- `lib/csv.ting` gained `each_row(path, f, sep = ",")`: the rows of a
  file one at a time, without holding the file. A 15.7 MB export of
  300000 rows costs 964 MB through `parse(read_file(p))` — sixty-one
  times the file, because a row is a list and a field is a string —
  and 9 MB through `each_row`, for about a tenth more time. Reading
  it with `each_line` and splitting on the separator is not merely
  slower, it is **wrong**: a quoted field may contain line breaks, so
  that same file is 400001 lines and 300001 rows. To be right rather
  than nearly right, `each_row` and `parse` are now the same scanner:
  `fresh()`, `scan(st, text, sep)` and `finish(st)` are the parser's
  state, one chunk through it, and the end of the text, so feeding a
  whole file and feeding it a line at a time give exactly the same
  rows — including for malformed text, where both stop in the same
  place.
- `lib/time.ting` gained `from_iso(s)`, the inverse `iso(ms)` never
  had, plus the `digits(s)` it is built on. It reads what `iso`
  writes, and also a bare date (midnight), a space where the `T`
  should be, seconds and fractional seconds when they are present,
  and an offset like `+02:00` or `-0500`, which it applies; a string
  with no offset is UTC, since the module has no other zone. It is a
  question rather than a demand — an unreadable string answers `nil`,
  the way `stat` does for a path. The refusals are the reason it
  exists: the handful of lines a script writes without it turns
  `"2026-13-45T99:99:99Z"` into a confident number and reports
  `cannot convert "not " to int` for `"not a date"`. Every field is
  checked against what exists, so February the 30th, the 29th of a
  non-leap year, an hour of 24 and a leap second are all `nil`.
- `examples/monthly.ting` reports a CSV month by month without
  holding it: the columns found by name in the header, one pass with
  `each_row`, the date column read with `from_iso`, and a count of
  the dates nothing could read rather than a guess at what they
  meant. The file it builds for the demo is 5001 rows in 6001 lines,
  so a reader that cut on newlines would invent a thousand rows.

## v2.120.0 (2026-09-06)

- `each_line(path, f)` is the 72nd builtin: a file read one line at a
  time, `f(line)` called for each, with only the current line held.
  Until now a script given a path had to call `read_file`, which is
  the whole file at once — counting the matching lines in a 147 MB
  log that way costs 454 MB, the file once and then the list of its
  2000001 lines, which weighs more than the file. The same count
  through `each_line` costs 9 MB and slightly less time. ting could
  already stream, but only from stdin, through `input()`; the
  workaround was `cat big.log | ting count.ting`, which stops the
  script from taking the path as an argument or reading two files.
  The line arrives as `input()` gives it — no newline, no carriage
  return before it — a last line without a newline still counts,
  `"-"` is stdin and shares the buffer `input()` reads from, and
  returning `false` from `f` stops the read, which is what makes
  "the first ten lines" cost what it should.
- `lib/fs.ting` gained `count_lines(p)`, `head(p, n)`, `tail(p, n)`
  and `lines_matching(p, needle)`, all built on `each_line` so the
  streaming is not something to remember. Two of them are easy to
  write badly: `head` has to stop the read rather than filter
  afterwards, and `tail` has to hold a window rather than a list —
  pushing every line and dropping the front copies the window once
  per line, which over 1000000 lines measured 2007 ms at n = 10 and
  75000 ms at n = 1000, against 925 ms and 926 ms for a ring that
  costs the same whatever n is.
- `examples/logreport.ting` reports on a log without holding it: one
  pass with `each_line`, counting by level and by source, keeping the
  first line, the last, and the longest. It prints what the file
  weighs beside what reading it cost — 216388 bytes against 86 —
  because nothing in the report grows with the file. It takes a path
  or `-` for a pipe, and an empty log is answered rather than
  crashed on.

## v2.119.0 (2026-09-06)

- `rename(from, to)` is the 70th builtin: a file or directory given
  another name, which is what a move is. Nothing is copied, so it
  costs the same for a byte and a gigabyte, and the modification time
  comes through untouched — the reason it exists. Until now the only
  way to move a file was `write_file(to, read_file(from))` followed by
  `remove_file`, which stamps the copy with the moment it ran: a
  script filing files by the day they were written destroyed every
  date it had just sorted by. It also could not move a file that is
  not text at all. `rename` replaces an existing target and moves a
  directory whole, both as the system call and `mv` do. It refuses to
  cross a filesystem, where `mv` quietly copies instead; a rename that
  is sometimes a copy has a different cost, a different failure mode
  and a different date, so it says so rather than hiding it.
- `copy_file(from, to)` is the 71st builtin: a file's bytes copied,
  whatever they are, without holding the file in memory. It carries
  the original's permission bits and its modification time. That last
  is a decision rather than an inheritance — `std::fs::copy` drops
  the date and `cp` needs `-p` to keep it, but `mv` keeps it even
  when it has to fall back to copying across a filesystem, and a
  cross-filesystem move in ting is `copy_file` followed by
  `remove_file`. A directory errors, and so does a target that is the
  same file as the source: the copy underneath opens the target for
  writing, which truncates the source it is about to read, and
  reports a successful copy of nothing. Sameness is asked of the
  filesystem rather than of the spelling, so `a` and `./a` are caught
  and so is a hard link. It is not atomic, deliberately: a copy that
  cannot be seen half-done is a copy to a temporary name and a
  `rename` onto the target, which is two readable lines.
- `lib/fs.ting` gained `move(from, to)`: a `rename` where that works,
  and where it does not, the copy and the removal `mv` falls back to.
  It is written in ting so the expensive path is readable, and so the
  rare case cannot pretend to be the cheap one.
- `examples/organize.ting` files a directory into folders named for
  the day each file was last written — a script that could not be
  written before, and that would have destroyed the dates it sorted
  by if it had. It handles what a real tidy-up must: a name clash is
  numbered rather than allowed to overwrite, filing twice moves
  nothing, and emptied directories are removed.

## v2.118.0 (2026-09-06)

- `stat(path)` is the 69th builtin: what a file is besides its name.
  A map of `size` in bytes, `modified` in milliseconds on the same
  clock `time_ms()` reads, and `kind` — `"file"`, `"dir"` or
  `"other"` — and `nil` when nothing readable is at the path, so it
  is a question like `exists` rather than a demand. Until now a ting
  program could learn a file's name and whether it was a directory,
  and nothing else. Sizing one meant `len(read_file(p))`, which
  counts characters rather than bytes (`src/eval.rs` is 192530 bytes
  and that expression says 192474) and refuses a file that is not
  valid UTF-8 at all — 4978 of this repository's 5108 `.git` files.
  A modification time could not be had by any arrangement of what
  existed, so "what changed since yesterday" was not a hard script
  but an impossible one.
- `lib/fs.ting` gained `size(p)` (a file's bytes, or `nil`, without
  the surrounding map), `facts(d)` (every file at or below a
  directory as a map of path, size, modified and kind, one `stat`
  each) and `total_size(d)` (the bytes below a directory, which is
  `du`). `facts` exists because the obvious alternative — sorting
  `walk`'s paths with `stat` inside the comparator — calls it once
  per comparison rather than once per file: 249 ms against 139 ms
  over 5132 files.
- A new example, `examples/tree.ting`, which could not have been
  written a release ago: what a directory holds by size, the largest
  files in it, where the bytes are by extension, and how many of them
  changed in the last day. It reports on a path you give it, or on a
  small tree it builds and removes, so it prints the same thing every
  time. It is in the cookbook like every other example, and CI
  replays it against its recorded output.

## v2.117.0 (2026-09-06)

- `ting --bundle SCRIPT -o FILE` writes the bundle to a file, and
  refuses when that file is one of the files that went into it,
  however it is spelled. That refusal is the reason it exists: a shell
  redirection opens its target before ting is started, so
  `ting --bundle main.ting > main.ting` truncates the script, reads
  the empty file it has become, reports success, and leaves a bundle
  of nothing where the program was. Measured, not imagined.

- A bundled module runs the first time something asks for it, not at
  the top of the file. An `import` does not have to sit at a module's
  top level — `if x { let m = import("noisy.ting"); }` runs the module
  only when the branch is taken — and v2.116.0's bundle hoisted every
  module and ran it whether or not the program asked. A module that
  only defines things could not tell the difference; one that prints,
  writes a file or takes time could. Each module is now a function
  holding what its top level declared, which runs its body once and
  hands back the same map ever after, which is what `import` itself
  does. The bundle's shape changed with it, and the tutorial's listing
  along with it.

## v2.116.0 (2026-09-06)

- `ting --bundle SCRIPT` prints a script and the local modules it
  imports as one file, on stdout, changing nothing on disk. The binary
  was always one self-contained thing, but a program of your own that
  split into modules stayed several files, with nothing to turn it
  back into one you could hand to somebody. Each module becomes a
  function returning what its top level declared, bound once, and
  every `import` of it reads that one binding — which is what
  importing the same file twice already gives, so a module that keeps
  state stays one module, and a module two others import is inlined
  once. An import is inlined when its path names a file and left alone
  when it does not, the order the interpreter itself resolves in, so
  `import("lib/list.ting")` normally stays: the binary answers it, and
  that is what makes one file enough. Three things are refused rather
  than guessed at, each named at the import that could not be
  followed: a circular import, a path that is not a literal string,
  and a module that returns from its own top level. It takes a file
  and only a file, a script's imports resolving against its own
  directory.
- The promise is a test, not a sentence: every program in the
  repository that imports a local module — fourteen of them, the
  standard library included — is bundled and rerun, and the bundle
  must print the same bytes on stdout, exit the same way, and pass
  `--check` and `--fmt-check`. The bundler copies a module's source as
  it was written, so a bundle of files that pass `--fmt-check` passes
  it too. What a bundle cannot keep identical is a program that prints
  where its own code sits: `try()` hands back a file and a line, and
  in a bundle those are the bundle's.
- The tutorial's module section, which used to end with a program in
  several files, now ends with `--bundle` — the two files, the
  command, and the bundle it writes, that listing checked against the
  real output by a test rather than transcribed. It also said the
  stdlib page documents "all seven" modules; there are twelve.

## v2.115.0 (2026-09-06)

- The tutorial uses `try(f, ...args)` where that is what it means. It
  taught the short form and then went on using a lambda in five later
  places; those five now read the way the page says to write them. The
  two that guard more than one call keep the lambda, which is what the
  reference already explains it is for.
- Every ting code block in the tutorial and the reference is run by
  the test suite, in a directory of its own, and each one is held to
  the output the page claims for it. The cookbook already had that
  guarantee through `examples/`; these two pages were written by hand
  and nothing ran them. All 43 of the tutorial's claimed outputs were
  already correct. The two blocks that are illustrations rather than
  programs now say so on their first line.
- `words` is about five times faster. It turned every separator into a
  space and split once, instead of looking at each character in ting:
  48 ms to 9 ms on a 108 KB text, and 69 ms to 20 ms on one full of
  tabs and newlines. What it returns is unchanged, carriage returns
  included, and `squeeze` gets the same gain for free.

## v2.114.0 (2026-09-06)

- `s += x` appends to the string instead of copying it. Building a
  string a piece at a time was quadratic: 25000, 50000 and 100000
  single-character appends took 19, 66 and 240 ms, and now take 4, 6
  and 14 ms on the VM. The old value is moved out of its binding
  rather than cloned, which is done only when appending a string to a
  string — the one pair that cannot fail — and only when the
  right-hand side cannot reach the name being assigned, so `s += s`,
  `s += f()` where f writes s, and a failed `s += 1` all behave
  exactly as before. The new `bench/accum.ting` covers the shape.

- `sort_with(xs, cmp)` is a builtin. The standard library's comparator
  sort was the one sort written in ting, and the slowest thing in the
  standard library: it is about six times faster now, 346 ms to 60 ms
  on 20000 elements. `import("lib/list.ting")["sort_with"]` is
  unchanged and still finds it, the ordering it produces is unchanged,
  and it is still a stable merge sort. A comparator that returns
  something other than a number now says so by name.
- `--check` no longer warns that `let f = f;` shadows a builtin. That
  line re-exports the builtin from a module rather than hiding it, and
  nobody writes it by accident.

- A module exports what its top level declares. `let` and `fn` at a
  module's top level are its exports, read from the module itself
  rather than guessed from the environment it ran in. The visible
  difference is that a module can now re-export a builtin under its
  own name — `let sort = sort;` puts `sort` in the module map, where
  before it vanished, because the old rule treated any builtin still
  bound to its own name as ambient. Every existing module exports
  exactly the same 175 names as before.

## v2.113.0 (2026-09-05)

- Pattern matching is about three times faster. The search reuses its
  thread lists, its `seen` vector and its epsilon stack across
  positions instead of allocating them per character; capture slots are
  shared between threads and recycled rather than copied; and a pattern
  that can only match at the start of the text no longer begins a
  thread at every position, since every one of those threads died on
  the same instruction. A match against a short subject goes from
  4.65 us to about 1.45 us, and the new `bench/regex.ting` is in
  BASELINE alongside the rest.
- Nothing about what a pattern matches changed. The matcher was already
  a Pike VM with no backtracking, and this is a constant-factor change
  to the same algorithm.

## v2.112.0 (2026-09-05)

- The bytecode VM now compiles what a script imports. A module used to
  run on the tree-walking reference whichever engine the script was
  started with, so the standard library — where a program that uses it
  spends most of its time — never saw the compiler. `bench/stdlib.ting`
  was the one benchmark the VM lost, at 6% behind the reference; it is
  now 44% ahead, and every benchmark checksum is unchanged. A module's
  own top level still binds by name, because that is where `import`
  reads a module's exports from.
- Accepted divergence, documented in docs/vm.md: a module with a
  `return` or `break` at its top level is refused by both engines with
  the same message, but the VM refuses it before the module runs, so
  statements ahead of the bad one take effect under `--eval` only.

## v2.111.0 (2026-09-05)

- The bytecode VM was slower than the tree-walking reference on
  top-level code — the shape most scripts are made of — because a
  function's locals resolve to frame slots at compile time and a
  script's did not. They do now. An empty 300k loop at the top of a
  file goes from 14% slower than the reference to 66% faster, and
  `bench/json.ting` from 15% slower to 5% faster. Every benchmark
  checksum is unchanged.
- The two engines said different things about a misspelled local. A
  function's locals live in frame slots under the bytecode VM, where
  they carry no name at runtime, so `amonut` next to a local `amount`
  got the bare "undefined variable" from the VM and the suggestion
  from the tree-walker. The compiler now records what was in scope at
  each site that can raise it, and both engines name the nearest one —
  and neither offers a name that is out of scope where the failure
  happened.

## v2.110.0 (2026-09-05)

- `--check` on a module member that does not exist now names the
  builtin of that name where there is one, instead of guessing at the
  nearest export. A module that retires a function into a builtin —
  as `lib/map.ting`'s `get` did in v2.109.0 — leaves its callers here.
- `--coverage` and `--profile` name an imported module the way the
  rest of their rows are named, relative to the directory the command
  ran in, instead of printing the absolute path it resolved to.

## v2.109.0 (2026-09-05)

- New builtin `get(x, k, default)`: `x[k]` where it is present,
  otherwise `default`. It reads a map by key and a list or string by
  index, negatives counting from the end, and never errors on an
  absence — so a tally is `m[k] = get(m, k, 0) + 1` rather than a
  branch around the first sighting. A key of the wrong type for the
  base is still an error, because a default answers an absence, not a
  bug.
- `lib/map.ting`'s `get` is gone: the builtin subsumes it, and a
  module function that shadows a builtin is one of the checker's
  warnings. Call `get(m, k, default)` directly, with no import.

## v2.108.0 (2026-09-05)

- A runtime error's `note:` lines now show what each call was given:
  `note: in scale(row = [3, "x"], factor = 2), called from ...`.
  Defaults and the rest list are included, because those are what the
  body saw. At most four arguments are named and each value is cut to
  32 characters, so a big list cannot bury the message. Both engines
  render the same text.
- Every frame in the `"trace"` `try` hands back carries an `"args"`
  map from parameter name to value — the values themselves, not the
  diagnostic's shortened rendering. `lib/err.ting` gains
  `given(f, ...rest)`, the arguments of the innermost failing call.
- The reference and the tutorial cover both, and say which caps belong
  to the diagnostic and which do not.

## v2.107.0 (2026-09-05)

- Compound assignment: `+=`, `-=`, `*=`, `/=` and `%=` on a variable
  or an indexed element. The target is named once, and for
  `m[k] op= v` the base and the subscript are evaluated once and used
  for both the read and the write.
- `try(f, ...args)` calls f with the arguments that follow it, so a
  call with arguments needs no lambda to carry them. Every function in
  `lib/err.ting` takes them the same way.
- A document highlight now calls the target of an assignment a write,
  compound or plain, rather than only the names a `let` or `fn`
  introduces.
- The reference and the tutorial cover both, including what a lambda
  around a `try` still buys you; the corpus uses them throughout, and
  the fuzzers generate them.

## v2.106.0 (2026-09-05)

- `--coverage` takes several paths (directories recurse, as every
  other tool flag here does): each script runs in its own interpreter,
  and they add up to one report.
- The reference and the tutorial document `--coverage`, including
  what counts as a coverable line.
- Tests for what coverage found untested: `lib/json.ting`'s `set_in`
  refusal, `lib/list.ting`'s `max_by` replacing its running best,
  `lib/args.ting`'s `main`, and — from processes of their own —
  `lib/test.ting`'s `summary` and `main`'s two exits.

## v2.105.0 (2026-09-05)

- New `ting --coverage SCRIPT`: runs the script, then reports on
  stderr — per file — the share of statements reached and the lines
  of those that were not. Imported modules are counted against their
  own files, and both engines report the same lines.

## v2.104.0 (2026-09-05)

- The tutorial and reference cover rest parameters and spreads, and
  selftest/varargs.ting exercises them on both engines. The
  differential generator emits variadic calls; the crash fuzzer's
  alphabet has `...`.
- `lib/test.ting` gains `pass()` and `fail_with(pattern, ...parts)`,
  which its five checks now go through and a check of your own can
  too.
- `lib/csv.ting`'s `parse` and `text` take an optional separator;
  `parse_with` and `text_with` remain as the older spelling.

## v2.103.0 (2026-09-05)

- A function's last parameter may be written `...rest`, and then it
  binds a list of every argument the fixed parameters did not take.
  Arity errors and `--check` warnings say "at least N arguments";
  the formatter keeps `...name` tight and hover shows it as written.
- `f(...xs)` spreads a list into a call, so what a rest parameter
  collects a spread can pass on. Spreading anything but a list is an
  error naming the type; `...` outside an argument list does not
  parse.

## v2.102.0 (2026-09-05)

- Function parameters may carry defaults (`fn f(a, b = 1)`), so a
  call can leave the tail off. Defaults are expressions evaluated at
  each call in the callee's scope, left to right, and arity errors
  name a range. Both engines, the checker, the formatter and the
  language server understand them.

## v2.101.0 (2026-09-05)

- New stdlib module `lib/args.ting`: command-line parsing from a
  spec, with the `--help` text built from the same spec.
- New stdlib module `lib/err.ting`: `message`, `failed`, `value`,
  `wrap`, `site` and `trace` over `try`.
- New stdlib module `lib/csv.ting`: delimited text both directions,
  quotes and embedded line breaks included.

## v2.100.0 (2026-09-05)

- New builtins `re_test(s, pattern)` and `re_find(s, pattern)`, over a
  new regular expression engine (`src/regex.rs`): a Pike VM, so
  matching is linear in the input and no pattern can be made to hang.
  Positions count characters, as `find` and `slice` do.
- New builtins `re_find_all(s, pattern)`, `re_replace(s, pattern,
  repl)` with `$1` group references, and `re_split(s, pattern)`.

## v2.99.0 (2026-09-05)

- New builtin `run(cmd)` / `run(cmd, args)` runs a program and waits,
  handing back a map of `code`, `out` and `err`. An argv list, never
  a shell string; a program that cannot be started is an error rather
  than an exit code. Refused on wasm, as `exit` and `sleep_ms` are.
- New builtins `eprint(...)`, which prints to stderr after flushing
  stdout so the two stay in order, and `cwd()`.
- New stdlib module `lib/sh.ting`: `ok`, `check` and `lines` over
  `run`, plus `which` and the PATH handling under it.

## v2.98.0 (2026-09-04)

- New builtins `random()`, `random_int(lo, hi)` and `seed(n)`:
  a float in `[0, 1)`, an int in a half-open span like `range`, and a
  restart point that makes a run repeat. Unseeded, the generator
  starts from the clock. See docs/reference.md.

## v2.97.0 (2026-09-04)

- New builtin `sleep_ms(ms)` pauses for that many milliseconds,
  flushing output first; a negative count or a non-int errors, and
  wasm refuses it as it does `exit` and `time_ms`.
- New stdlib module `lib/time.ting`: `iso`, `date`, `clock`, `parts`,
  `from_parts`, `span` and the civil-date arithmetic under them. UTC
  throughout, exact either side of the epoch.

## v2.96.0 (2026-09-04)

- New builtins `hex(n)` and `bin(n)` write the literal forms
  (`0xff`, `0b1010`), keeping the sign rather than wrapping:
  `hex(-255)` is `-0xff`.
- `int(s)` reads a string the way the lexer reads a literal — sign,
  `0x`/`0b` prefix, `_` between digits — so `int(hex(n))` is `n`.
- Docs: the reference and tutorial cover how numbers print and
  convert, and the Limits section says where infinity can come from.

## v2.95.0 (2026-09-04)

- Floats print in a form that reads back: an exponent outside the
  range 1e-4 to 1e17, the shortest round-tripping form inside it, and
  a `.0` on integral values. `1e23` printed as a 23-digit expansion
  before. `json_str` spells them the same way.
- Conversions refuse what a literal refuses: `float("1e400")`,
  `float("inf")`, `float("nan")` and `json_parse("1e999")` are errors
  rather than infinities.
- `int(x)` on a non-finite or out-of-range float is an error naming
  the value instead of saturating to `i64::MAX`.

## v2.94.0 (2026-09-04)

- Bitwise operators: `&`, `|`, `^`, `~`, `<<` and `>>`, on ints only.
  They bind tighter than every comparison, so `flags & MASK == MASK`
  applies the mask first — Rust's ordering, not C's.
- `>>` keeps the sign; a shift count outside 0 to 63 is an error, and
  a float operand is a type error rather than a promotion.
- Docs: the reference operator table and a tutorial section cover the
  bits and the literal forms; the Limits section no longer claims a
  fixed call depth of 200.

## v2.93.0 (2026-09-04)

- Integers can be written in hex (`0xff`) or binary (`0b1010`), and
  any run of digits can be broken up with `_` (`1_000_000`,
  `0xFF_FF`). A separator must sit between two digits.
- Floats take an exponent: `1e3`, `1.5e-3`, `2E+2`. An exponent
  always makes a float; a literal out of range for a double is an
  error rather than infinity.
- A literal that runs into a letter or a digit outside its radix is
  an error naming the offender (`0b12`, `12abc`), where it used to
  split into two tokens and fail elsewhere.

## v2.92.0 (2026-09-04)

- String literals take `\uXXXX`, four hex digits with a surrogate
  pair past U+FFFF — the spelling JSON uses, so a string copied out
  of a JSON document means the same thing either way.
- New builtins `ord(s)` and `chr(n)` convert between a
  one-character string and its code point.
- Docs: the reference and tutorial cover the recursion limit, the
  removal builtins and how to spell a character.

## v2.91.0 (2026-09-04)

- The call-depth cap is derived from the host stack the process
  declares rather than fixed at 200: the runner and the REPL hand
  their interpreter 32 MB and allow a few thousand frames from it
  (fewer unoptimized, where a frame costs several times as much).
  An embedder that declares nothing keeps the old conservative cap.
  The diagnostic names the cap it enforced.
- New builtins `remove_file(path)` and `remove_dir(path)`; the
  latter takes only an empty directory. `lib/fs.ting` gains
  `remove_tree`, the recursive version, written in ting.

## v2.90.0 (2026-09-04)

- New stdlib module `lib/fs.ting`, the seventh: `base`, `dir`,
  `ext`, `stem`, `parts`, `normal`, `join_path` and `with_ext` split
  and reassemble paths (on both `/` and `\`, joining with `/`);
  `entries`, `walk` and `walk_ext` list and recurse through a tree.
- Docs: the reference and tutorial cover the filesystem builtins and
  the module.

## v2.89.0 (2026-09-04)

- Four builtins let a script see the filesystem the toolchain
  already walks: `list_dir(path)` (the names in a directory,
  sorted), `exists(path)` and `is_dir(path)` (questions, so an
  absent path is `false`, never an error), and `make_dir(path)`
  (missing parents included; a directory already there is fine).

## v2.88.0 (2026-09-04)

- `ting -` runs a script read from standard input: arguments after
  the dash reach `args()`, diagnostics name the script `-`, and a
  relative `import` resolves against the working directory. The
  script is the stream, so `input()` sees end of file.
- Docs: the reference and tutorial cover watch mode and piped
  scripts.

## v2.87.0 (2026-09-04)

- New `--watch` for `--test`, `--check` and `--fmt-check`: the pass
  runs again whenever a watched file changes, is added or is
  deleted, with a rule line naming the run and its cause. The
  paths named on the command line are expanded before every poll,
  so new files join the next run. `--fmt --watch` is a usage error
  (it would answer its own rewrites); `--fmt --diff --watch` works.

## v2.86.0 (2026-09-04)

- `--profile` counts builtins too, marked `a builtin` where a ting
  function names its file and line, and prints at most twenty rows
  before counting the rest.
- Docs: the reference and tutorial explain the profiler, self time
  and the table.

## v2.85.0 (2026-09-04)

- New `ting --profile` runs a script and then reports, on stderr,
  how often each function ran, the time it spent in its own body
  (self time, so recursion is counted once), and where it was
  defined — slowest first.
- Fix: a closure created inside an imported module's function now
  belongs to that module's file, so it is reported under the file
  it was written in.

## v2.84.0 (2026-09-04)

- `try` hands a caught failure back whole: `"err"` is the message,
  `"at"` is the file, line and column it was raised at, and
  `"trace"` is the calls it came out of, each with the function's
  name (`nil` when it has none).
- `lib/test.ting`: a `check_err` whose error carries the wrong
  message now names the line that raised it.
- Docs: the tutorial and reference explain traces, how frames are
  named, and the ten-frame cap.

## v2.83.0 (2026-09-04)

- A runtime error shows the whole way back: one `note: in NAME,
  called from FILE:LINE:COL` per call it unwound through, innermost
  first. A trace longer than ten frames keeps four at each end and
  says how many it left out.
- Arity errors count in words and name the function called: "len
  expects 1 argument, got 0", "two expects 2 arguments, got 1".

## v2.82.0 (2026-09-03)

- Every `lib/test.ting` helper counts as a check, so files built on
  the framework report their totals under `ting --test` too.
- Docs: the tutorial and the stdlib page explain the counts.

## v2.81.0 (2026-09-03)

- `ting --test` says how much each file verified: `ok FILE (12
  checks)`, a total in the summary, `# 12 checks` in the TAP
  stream.
- A file that passes while checking nothing is named as such, in its
  own line and in the summary.

## v2.80.0 (2026-09-03)

- A file's `--check` warnings are printed in line order, whatever
  pass found them.
- Docs: the retrospective's tenth act; the tutorial and reference
  list every warning the checker gives.

## v2.79.0 (2026-09-03)

- `ting --check` and the LSP warn about a map literal that gives the
  same string key twice — the last one silently wins.
- They also warn about a statement that can never run, after a
  `return`, `break` or `continue` in the same block.

## v2.78.0 (2026-09-03)

- `ting --check` and the LSP warn when a call's argument count
  cannot match the function it names — for a function bound once at
  the top level and never rebound or shadowed.
- The corpus scan's warning set is guarded by a test, so a false
  positive from either static check fails the build.

## v2.77.0 (2026-09-03)

- `ting --check` and the LSP warn about a name that is bound nowhere
  — not a parameter, not a `let` in an enclosing block, not a
  builtin — and name the nearest one in scope.
- The LSP offers a quickfix that replaces such a name with the
  nearest one, beside the one for stdlib members.

## v2.76.0 (2026-09-03)

- An unknown option names the one you probably meant (`--fmr` finds
  `--fmt`).
- Suggestions count a swap of neighbours as one slip (`--lps` finds
  `--lsp`) and stay silent for names under three characters.
- The tutorial explains the suggestions; selftest/errors.ting checks
  them from inside ting.

## v2.75.0 (2026-09-03)

- "Did you mean?": an undefined variable (or an assignment to one)
  names the nearest binding, parameter or builtin in scope.
- A key a map does not hold names the nearest key it does, and the
  `--check`/LSP warning for an unknown stdlib member names the
  nearest member.
- `ting --doc` and the REPL's `:doc` suggest the nearest documented
  name for a name they do not know.

## v2.74.0 (2026-09-03)

- `--doc` and `:doc` wrap at 78 columns: a comment under its
  signature, an index line's first sentence beside the name when it
  fits and underneath when it does not.
- `ting --doc len median slug`: several names at once, printed in
  order and separated by a blank line; an unknown name exits 1 and
  the rest are still printed.
- A new example, `examples/inventory.ting` (stock list: `key_of`,
  `take_while`/`drop_while`, `flatten`, `plural`).

## v2.73.0 (2026-09-03)

- Exit codes: 0 success, 1 a reported failure, 2 a usage error
  (missing operand, bad option value, unknown option); `--help`
  and the reference's Running section say so.
- lib/list.ting: `take_while(xs, pred)` and `drop_while(xs, pred)`.

## v2.72.0 (2026-09-03)

- An unknown option is a usage error (exit 2) that names it and
  points at `--help`; `-h` and `-V` work as short forms.
- lib/string.ting: `plural(n, one, many)`.
- Docs: the formatter's every-file run and summary line.

## v2.71.0 (2026-09-03)

- `--fmt`, `--fmt-check`, `--fmt --diff` and `--check` process every
  file before failing: a file that cannot be read, does not lex or
  cannot be written is reported and the run continues, with exit 1
  at the end.
- A multi-file `--fmt` run ends with a summary line.
- lib/map.ting: `key_of(m, v)`.

## v2.70.0 (2026-09-03)

- `ting --check --strict`: warnings fail the check.
- The stdlib page is guarded against missing rows and a stale
  function count (116 functions).

## v2.69.0 (2026-09-03)

- `--check` and the LSP warn when a `let`, `fn` or parameter shadows
  a builtin.
- lib/string.ting: `is_number(s)`; lib/list.ting: `argmax(xs)` and
  `argmin(xs)`.

## v2.68.0 (2026-09-03)

- LSP: the whole-document formatting edit ends at the document's
  real last position.
- Tutorial: diff and flatten in the JSON chapter; retrospective:
  ninth act.

## v2.67.0 (2026-09-03)

- lib/test.ting: `check_type(name, v, type_name)`; check, check_eq
  and summary documented.
- lib/json.ting: `flatten(v)`, a map from dotted leaf paths to
  values.
- lib/math.ting: `hypot(a, b)`.

## v2.66.0 (2026-09-03)

- The formatter keeps the file's line endings: a CRLF file stays
  CRLF, and `--fmt-check` no longer flags a Windows checkout.
- `--check` and the LSP warn about a `let` inside a block that
  nothing in the block uses (underscore-prefixed names exempt).

## v2.65.0 (2026-09-03)

- REPL: `:load` reports how many bindings it added.
- A failed `import` names the path it resolved to and says no
  embedded module matched.
- lib/list.ting: `transpose(xss)`.

## v2.64.0 (2026-09-03)

- REPL: `:load FILE` resolves the file's relative imports against
  its own directory and names the file in diagnostics.
- lib/string.ting: `squeeze(s)`.
- Docs count the thirteen editor capabilities.

## v2.63.0 (2026-09-03)

- LSP: documentHighlight (occurrences of the symbol under the
  cursor, binding sites as writes) and prepareRename (the editor
  declines a rename on a keyword or builtin before prompting).
- lib/map.ting: `merge_with(a, b, f)`.

## v2.62.0 (2026-09-03)

- REPL: a session transcript — `:history` lists every chunk that
  ran without error, `:save FILE` writes them as a runnable script,
  `:clear` empties them — and `:doc` alone lists everything like
  `--doc`.

## v2.61.0 (2026-09-03)

- LSP completion offers the file's own functions with their
  signature and the comment above them.
- lib/list.ting: `flatten_deep(xs)`; `flatten` documented.
- Tutorial: the closing chapter matches the toolchain (warnings,
  import walk, `--doc`, `--fmt --diff`, the playground's check).

## v2.60.0 (2026-09-03)

- LSP hover on a function defined in the file shows the `#` comment
  above it.
- lib/map.ting: `map_keys(m, f)`.

## v2.59.0 (2026-09-03)

- Docs: the tutorial's Testing chapter covers the runner's flags,
  the stdlib page opens with all six modules and the `--doc` route
  to the same text, and the retrospective gains its eighth act.
- The crash fuzzer exercises cyclic values.

## v2.58.0 (2026-09-03)

- Cyclic data no longer crashes the process: printing shows `[...]`
  / `{...}` at the point of recursion, `==` terminates (two cycles
  that agree everywhere they can be inspected are equal), and
  `json_str` reports a cyclic value as an error.

## v2.57.0 (2026-09-03)

- lib/list.ting: `find_index(xs, pred)`.
- examples/series.ting: extent, mean, median, mode, percentile,
  window and chunk_by on a numeric series; cookbook and playground
  regenerated.
- editor/README.md describes the language server's twelve
  capabilities and its warnings.

## v2.56.0 (2026-09-03)

- Playground: a check button (the checker and its warnings, via a
  new `ting_check` wasm export).
- lib/list.ting: `chunk_by(xs, key)` groups consecutive runs.
- examples/text.ting: words, frequencies, slug, wrap and
  levenshtein at work; cookbook and playground regenerated.

## v2.55.0 (2026-09-03)

- `ting --test --fail-fast` stops after the first failing file; the
  rest count as skipped (TAP `# SKIP` lines).
- `ting --doc path/to/file.ting` lists a file's top-level functions
  with the comments above them.
- docs/vm.md opens with the VM's current status.

## v2.54.0 (2026-09-03)

- LSP: an `import` of a local file that fails to lex, parse or
  compile is an error diagnostic on the import string, with the
  module's position and message.
- README tooling paragraphs and the tutorial's modules chapter
  brought up to date (module error locations, the call-site note,
  `--check` following imports).

## v2.53.0 (2026-09-03)

- A module-origin error is followed by `note: called from
  FILE:LINE:COL`, the call site in the importer.
- `--check` follows `import("...")` of local files, checking each
  reached file once under its own path.
- lib/string.ting: `slug(s)`.

## v2.52.0 (2026-09-03)

- Runtime errors raised inside an imported module's function are
  reported against the module's file and line (both engines); an
  error from an embedded stdlib module no longer panics the
  diagnostic renderer.
- lib/list.ting: `mode(xs)`, the most frequent element.
- Retrospective: seventh act, "second opinions".

## v2.51.0 (2026-09-03)

- `ting --doc` with no name lists every builtin and stdlib function;
  `--doc MODULE` lists one module. The REPL's `:doc` does the same.
- `--check` and the LSP warn about function parameters the body
  never names (underscore-prefixed names are exempt).
- lib/list.ting: `extent(xs)` returns `[smallest, largest]`.

## v2.50.0 (2026-09-03)

- `--check` and the LSP warn about unused top-level bindings
  (underscore-prefixed names and binding-only module files are
  exempt).
- LSP: signature help for the file's own functions.
- `ting --fmt --diff` prints the changed lines instead of writing.

## v2.49.0 (2026-09-03)

- `ting --test --slow N` lists the N slowest files after the summary.

## v2.48.0 (2026-09-03)

- Playground: the example dropdown is generated from `examples/`
  (twelve runnable examples) and guarded against drift.
- LSP: hover shows the signature of the file's own functions.
- `lib/math.ting`: `percentile`.

## v2.47.0 (2026-09-03)

- LSP: rename applies across every open document.

## v2.46.0 (2026-09-03)

- `ting --test -j N` runs up to N files at once, output kept in
  order.
- `lib/string.ting`: `dedent`.
- Tutorial: a "Shell scripting" chapter.

## v2.45.0 (2026-09-03)

- Tutorial: the modules chapter points at `--doc`, `:doc` and editor
  hover for reading a stdlib function.

## v2.44.0 (2026-09-03)

- LSP: document links on `import(...)` paths; a malformed message
  no longer ends the session; Windows drive-letter file URIs are
  handled.
- `lib/math.ting`: `variance`, `stddev` (the stats example uses
  them).
- README: status and tooling paragraphs brought up to date.

## v2.43.0 (2026-09-03)

- LSP: workspace symbols across open documents.

## v2.42.0 (2026-09-03)

- REPL: `:time EXPR` reports elapsed milliseconds.
- `examples/config.ting`: layered configuration with `lib/json.ting`.
- Tutorial: the JSON chapter shows `get_in`, `set_in` and `merge_in`.

## v2.41.0 (2026-09-03)

- `lib/string.ting`: `levenshtein`.
- Retrospective: a sixth act on the loop's rhythm.

## v2.40.0 (2026-09-03)

- `ting --test --tap` emits Test Anything Protocol output with
  per-file timings.
- `lib/list.ting`: `binary_search`.
- Tutorial: a "Closures as objects" chapter.

## v2.39.0 (2026-09-03)

- `lib/json.ting`: `diff`.
- `examples/machine.ting`: a state machine from closures and a
  transition table.

## v2.38.0 (2026-09-03)

- `ting --doc NAME` explains a builtin or stdlib function from the
  shell.
- `--check`, `--fmt` and `--fmt-check` accept directories.
- `lib/string.ting`: `wrap`.

## v2.37.0 (2026-09-03)

- LSP: folding ranges for multi-line braces.
- `bench/json.ting`: a JSON benchmark; the baseline gains its row.

## v2.36.0 (2026-09-03)

- `lib/list.ting`: `zip_with`, `cartesian`.
- Tutorial: a Testing chapter (`lib/test.ting` and `ting --test`).
- The formatter is now fuzzed against generated programs for
  idempotence and AST preservation.

## v2.35.0 (2026-09-03)

- REPL: `:doc NAME` explains a builtin or any stdlib function
  (module, signature, comment).
- `lib/test.ting`: `check_approx` for floats.

## v2.34.0 (2026-09-03)

- LSP: a quickfix code action replaces a misspelt stdlib member with
  the nearest export.
- `ting --test --filter SUBSTR` runs only matching paths.
- `lib/json.ting`: `merge_in`, a deep merge.

## v2.33.0 (2026-09-03)

- `lib/json.ting`, a sixth embedded module: `get_in`, `set_in`,
  `paths` for nested values.
- `ting --check` prints the unknown-stdlib-member warning the LSP
  shows (exit status unchanged).
- `examples/pipeline.ting`: records from stdin, grouped and tabled.

## v2.32.0 (2026-09-03)

- LSP: a warning when an imported stdlib module is indexed with a
  name it does not export.
- `ting --test` lists a directory's own files before descending
  into subdirectories.
- Retrospective: a fifth act on the glibc episode.

## v2.31.0 (2026-09-03)

- `ting --test` accepts directories (recursive, sorted), so
  `ting --test tests/` is the whole suite; CI runs the binary's own
  runner over `selftest/` on every platform.
- `lib/string.ting`: `table`, aligned columns for CLI output; the
  logs example prints one.

## v2.30.0 (2026-09-03)

- Releases now also ship fully static Linux archives
  (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`): no C
  library dependency at all.
- The crash fuzzer covers every pure builtin and the bytecode VM.
- `bench/stdlib.ting`: an import-heavy benchmark; the baseline is
  regenerated on one machine for all rows.

## v2.29.1 (2026-09-03)

- Linux binaries are built against glibc 2.35 again (v2.29.0's
  required 2.39 and would not start on Debian 12 / Ubuntu 22.04); the
  release workflow now fails if a Linux binary needs anything newer.

## v2.29.0 (2026-09-03)

- `ting --test <files...>`: a test runner — each file in its own
  process, `ok`/`FAIL` per file, a summary, exit 1 on any failure.
- `lib/list.ting`: `sort_with`, a stable sort by a three-way
  comparator.
- Site: a cookbook page rendering every example with its output.

## v2.28.0 (2026-09-03)

- `lib/list.ting`: `product`, `mean_by`, `compact`.
- `lib/string.ting`: `is_blank`.
- Tutorial: the word-frequency script tallies with `frequencies` and
  `top`.

## v2.27.0 (2026-09-03)

- `lib/string.ting`: `indent`.
- `lib/map.ting`: `top`.
- Tutorial: the closing chapter covers every REPL meta-command, the
  stdin filter, and the editor's stdlib awareness.

## v2.26.0 (2026-09-03)

- LSP: hover and signature help work for stdlib functions called
  through an imported module map (`l["median"](...)`), showing the
  signature and the function's comment; completion items carry the
  same comment.
- `lib/list.ting`: `frequencies`.

## v2.25.0 (2026-09-03)

- LSP: completion offers the functions of every stdlib module the
  document imports, with module and signature as detail.
- `lib/list.ting`: `interleave`.
- The differential fuzz generator now emits ten more builtins
  (string predicates, `replace`, `split`, `trim`, `lower`, `max`,
  `type`, `filter`, `reduce`).

## v2.24.0 (2026-09-03)

- `lib/list.ting`: `scan`.
- `lib/string.ting`: `strip_prefix`, `strip_suffix`.
- Retrospective: a fourth act on the move to a new machine.

## v2.23.0 (2026-09-03)

- `lib/list.ting`: `rotate`, `unique_by`.
- `lib/string.ting`: `truncate`.
- `lib/math.ting`: `is_prime`.

## v2.22.0 (2026-09-03)

- `lib/list.ting`: `sum_by`.
- `lib/string.ting`: `words`.
- `lib/map.ting`: `with`, `update`.
- Tutorial: the word-frequency script tallies with `words` and
  `count_by`.

## v2.21.0 (2026-09-03)

- Formatter: a `[` or `(` that ends its line indents its
  continuation lines by one level until the closer (inline openers
  are unchanged).
- `lib/string.ting`: `is_digit`, `is_alpha`.
- `examples/logs.ting`: a log summary using `count_by`, `window` and
  `is_digit`.

## v2.20.0 (2026-09-03)

- REPL: `:fmt` reprints the last evaluated chunk as the formatter
  would write it.
- `lib/list.ting`: `count_by`, `first`, `last`.
- `lib/map.ting`: `invert`.

## v2.19.0 (2026-09-03)

- `lib/list.ting`: `window`.
- `lib/string.ting`: `center`.
- Tutorial: the modules chapter shows `partition`, `group_by` and
  `take`/`drop`.

## v2.18.0 (2026-09-03)

- `ting x.ting | head` ends quietly with exit 0 when the reader goes
  away; the REPL does the same instead of panicking.
- `--fmt`, `--fmt-check` and `--check` accept `-` for stdin; `--fmt -`
  filters to stdout.
- `lib/math.ting`: `lcm`, `abs_diff`.

## v2.17.0 (2026-09-03)

- Releases now also ship an `aarch64-unknown-linux-gnu` archive (four
  platforms).
- `lib/string.ting`: `chars`, `reverse`.
- `lib/map.ting`: `filter_map`, `has_all`.

## v2.16.0 (2026-09-03)

- `lib/list.ting`: `group_by`, `take`, `drop`, `partition`.

## v2.15.0 (2026-09-03)

- `lib/list.ting`: `median`.
- REPL: `:clear` resets the session.
- `examples/stats.ting` now uses `mean`/`median` from the stdlib.

## v2.14.0 (2026-09-03)

- REPL: `:vars` lists the session's own bindings (name and type).
- `lib/list.ting`: `mean`.
- Tutorial: the modules chapter now shows the embedded stdlib and
  the disk-first fallback rule.

## v2.13.0 (2026-09-03)

- `lib/map.ting`: `pick`, `omit`.
- `lib/string.ting` and `lib/list.ting`: `count`.
- The differential fuzz generator now emits `find` and stepped
  `range` expressions, extending engine-equivalence coverage to the
  newer builtins.

## v2.12.0 (2026-09-02)

- `write_file(path, s, "append")`: optional append mode; any other
  mode errors.
- `lib/list.ting`: `insert_at`, `remove_at` (fresh lists, loud
  range checks).
- Selftests pin JSON control-character escaping and round trips.

## v2.11.0 (2026-09-02)

- Strings accept the `\r` escape (previously a carriage return was
  inexpressible in source); the TextMate grammar and a new sync
  guard follow.
- `lib/string.ting`: `trim_start`, `trim_end`.
- README brought up to date with the current feature set.

## v2.10.0 (2026-09-02)

- `lib/test.ting`: `check_err(name, f, want)` — error-path testing
  with distinct failure messages for wrong-error vs no-error.
- `lib/math.ting`: `floor`, `ceil` (correct on negatives, where
  `int()` truncation differs).
- `lib/list.ting`: `chunk(xs, n)`.

## v2.9.0 (2026-09-02)

- `read_file("-")` reads stdin to EOF, so ting scripts compose in
  Unix pipelines.
- LSP: `textDocument/signatureHelp` — builtin signatures and docs
  inside call parentheses (ninth capability).
- The changelog is now published on the site, linked from every
  page's nav.

## v2.8.0 (2026-09-02)

- Playground: a "fmt" button reformats the editor in place, backed
  by a new `ting_fmt` wasm export (verified against the live site).
- `lib/map.ting`: `values`, `map_values`.
- `lib/string.ting`: `split_once` (built on `find`, so indices are
  character-based).

## v2.7.0 (2026-09-02)

- REPL: `:load <file>` evaluates a file in the live session, keeping
  its bindings around to poke at.
- `lib/list.ting`: `any`, `all`, `min_by`, `max_by`.
- Docs: the reference documents both REPL meta-commands; the
  tutorial closes with a "Beyond scripts" tour of the toolchain.

## v2.6.0 (2026-09-02)

- `find(s, sub)` / `find(xs, v)`: 44th builtin — index of the first
  match or `nil`; strings use character indexing (matching `slice`),
  lists use structural equality (matching `contains`).
- REPL: `:help` lists every builtin's signature and doc line.
- A guard test now keeps repo markdown free of bare HTML-shaped
  tokens (a bare angle-bracketed token had broken LOG.md's rendering
  on GitHub; found by a reader).

## v2.5.0 (2026-09-02)

- LSP: `textDocument/references` — every occurrence of the
  identifier under the cursor (token-level).
- LSP: `textDocument/rename` — a WorkspaceEdit over those same
  occurrences; invalid new names are rejected.
- Reference: Tooling section updated with the full LSP capability
  list.

## v2.4.0 (2026-09-02)

- `lib/math.ting`: fifth stdlib module — `clamp`, `sign`, `pow`,
  `gcd`, `round`, `sqrt` (embedded in the binary like the rest).
- `range(lo, hi, step)`: optional third argument; negative steps
  count down, zero is an error. Existing forms unchanged.
- `examples/stats.ting`: descriptive statistics golden pair using
  both of the above.

## v2.3.0 (2026-09-01)

- LSP: `textDocument/documentSymbol` — an outline of top-level
  bindings, functions and variables distinguished.
- LSP: `textDocument/definition` — jump from an identifier to its
  top-level binding.
- Reference: new "Tooling" section documenting `--fmt`, `--check`,
  the LSP's capabilities, and the TextMate grammar.

## v2.2.0 (2026-09-01)

- `json_str(v, indent)`: optional pretty printing — `indent` spaces
  per level (0–16), empty containers stay inline, output round-trips
  through `json_parse`. Single-argument compact form is unchanged.
- `ting --check <files...>`: static verification — lex, parse, and
  compile without running; one diagnostic per bad file, exit 1 if any
  fail. Built for pre-commit hooks and CI.
- Tutorial: new "Working with JSON" section (parse, mutate, compact
  vs pretty output, error recovery); every snippet is CI-tested.

## v2.1.0 (2026-09-01)

- Fix: `==` now compares ints and floats numerically at every depth —
  `[1] == [1.0]` is true, matching the documented top-level rule
  (this also corrects `contains` and `lib/list.ting`'s `unique` for
  mixed int/float data).
- `selftest/edge.ting`: 25 sharp-edge assertions pinned on both
  engines (this suite is what caught the bug above).
- Playground: a "calc" example — a tiny calculator language
  interpreted by ting, in the browser.

## v2.0.0 — maturity (2026-09-01)

No new features — a milestone of confidence. A seven-point
full-system audit (both engines' suites, cross-engine benchmark
checksums, wasm in Node, formatter round trip, the live site, the
release assets) came back all green, and on that evidence the
reference now carries a stability promise: the documented language is
stable across 2.x; builtins are only ever added; breaking syntax or
semantics would mean a 3.0.

## v1.9.0 — depth (2026-09-01)

- Differential fuzzing generates a wider grammar (bounded loops,
  try-expressions, string operations; 800 cases per run) — both
  engines stay byte-identical.
- `examples/calc.ting`: a small arithmetic language interpreted by
  ting itself — tokenizer, parser, and evaluator in ting.

## v1.8.0 — the second act (2026-09-01)

- `lib/map.ting`: get-with-default, merge, items, from_items — the
  stdlib's map gap, closed (embedded like the rest).
- The website's story page now tells the full arc, VM redemption
  included.
- The reference recommends `rlwrap ting` for REPL line editing.

## v1.7.0 — finishing touches (2026-09-01)

- Every ting snippet on the docs site now carries a "run it in the
  playground" link that opens it preloaded and running.
- `ting --version` and `ting --help`.

## v1.6.0 — a formatter (2026-09-01)

- `ting --fmt` / `ting --fmt-check`: a canonical formatter that
  preserves comments and the author's line breaks, guaranteed
  idempotent and AST-preserving by tests. Also available as
  format-on-save through `ting --lsp` (documentFormatting).
- The repo's own ting sources are formatted with it, enforced by CI.

## v1.5.0 — the stdlib everywhere (2026-09-01)

- The standard library is embedded in the interpreter: when an
  imported `lib/...` path has no matching file, the built-in copy is
  used — so it works from any directory, in the REPL, and in the
  browser playground (which gained a stdlib example). A real file
  always wins.
- `docs/stdlib.md` documents all three modules, on the website as
  "stdlib".

## v1.4.0 — sharper tools (2026-09-01)

- LSP completions: builtins with docs, keywords, and the document's
  own identifiers.
- `lib/test.ting`: a tiny test framework written in ting
  (`check`/`check_eq`/`summary`), with a golden example.
- VM: pooled per-call buffers roughly doubled its lead — fib and
  list-heavy work now run ~45% faster than the reference engine.

## v1.3.0 — batteries + story (2026-09-01)

- A standard library written in ting itself: `lib/list.ting` and
  `lib/string.ting`, shipped inside the release archives and covered
  by the self-hosted suite.
- LSP hover: signature and summary for every builtin.
- The experiment's [retrospective](docs/retrospective.md), on the
  website as "story".

## v1.2.0 — a language server (2026-09-01)

- `ting --lsp`: the binary doubles as an LSP server — JSON-RPC over
  stdio, full-text sync, live lex/parse/compile diagnostics with real
  ranges — implemented with zero new dependencies on top of ting's own
  JSON codec. Wiring instructions for Neovim/VS Code/Zed in `editor/`.

## v1.1.0 — the VM earns its keep (2026-09-01)

- The bytecode VM is now the default engine: with compiled function
  bodies and local slot resolution it is 11-35% faster on the
  function-heavy benchmarks with no regressions. `--eval` or
  `TING_ENGINE=eval` selects the reference tree-walker; CI runs the
  full suite on both engines.

## v1.0.0 — confidence (2026-09-01)

The language and tooling are complete and held together by guards:
grammar-directed differential fuzzing (600 random valid programs per
test run, both engines byte-identical), a CI job that reruns the whole
suite on the VM engine, and coverage guards that fail the build if a
builtin ever ships without documentation or editor support. No
language changes — 1.0 marks stability, not novelty.

## v0.9.0 — bytecode VM (2026-09-01)

- A bytecode compiler and VM covering the whole language, selectable
  with `--vm` or `TING_ENGINE=vm`; differential tests hold both
  engines byte-identical (including the entire self-hosted suite).
  Measured honestly: no speedup over the tree-walker yet, so the
  tree-walker stays the default (see `docs/vm.md`).
- Benchmarks now compare both engines (`bench/run.py`).

## v0.8.0 — a real scripting citizen (2026-09-01)

- `json_parse` / `json_str` builtins: full JSON both ways (objects↔maps,
  surrogate pairs, strict errors with byte offsets).
- `env`, `exit`, `time_ms` builtins (wasm builds return clean errors
  where the platform can't support them).
- Showcase: `examples/todo.ting`, a JSON-file-backed todo CLI, driven
  end-to-end by its own integration test.
- This changelog.

## v0.7.0 — developer experience (2026-09-01)

- `format(fmt, ...)` builtin with strict `{}` placeholder rules.
- TextMate grammar under `editor/` (VS Code/Sublime/Zed install
  guide); a guard test keeps its builtin list in sync.
- Playground: live syntax highlighting (overlay, no library).
- Tutorial and reference rendered onto the website next to the
  playground (`tools/md2html.py`, stdlib-only).

## v0.6.0 — performance + polish (2026-09-01)

- ~10% faster interpreter (measured with the new `bench/` harness and
  recorded baseline): blocks without declarations no longer allocate
  scopes; env keys are `Rc<str>`.
- Playground share-by-URL (source encoded in the fragment).
- Tutorial: modules section.

## v0.5.0 — expressiveness (2026-09-01)

- `map`, `filter`, `reduce`, `min`, `max`, `abs` builtins.
- `assert` builtin + self-hosted `selftest/` suite (ting programs that
  test ting, run by CI).
- Modules: `import(path)` — fresh scope, exports as a map, per-path
  caching, cycle detection, module-located diagnostics.

## v0.4.0 — ting in the browser + robustness (2026-09-01)

- The interpreter compiled to WebAssembly with a hand-rolled ABI (no
  wasm-bindgen; still zero dependencies) and a browser playground,
  live on GitHub Pages.
- Fuzz tests (token soup, example mutants, deep nesting) — which
  found and fixed a real parser panic on a stray `:`.

## v0.3.0 — a practical scripting tool (2026-09-01)

- String builtins: `contains`, `replace`, `starts_with`, `ends_with`,
  `upper`, `lower`, `slice`.
- Script I/O: `args`, `input`, `read_file`, `write_file`; argv after
  the script path reaches `args()`.
- Stable `sort` / `sort_by`; error recovery with `try` / `fail`.
- Executable tutorial: every snippet is run and diffed by CI.

## v0.2.0 (2026-09-01)

- `split`, `join`, `trim` builtins.
- `for`-in over lists/strings/maps with `break`/`continue`.
- REPL errors render caret diagnostics.

## v0.1.0 (2026-09-01)

- The language core: ints/floats/strings/bools/nil, lists and maps
  with reference semantics, closures, control flow, strict semantics
  (no truthiness, checked overflow, exact arity), caret diagnostics,
  a REPL, 11 builtins, examples, CI on three platforms.
