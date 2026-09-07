#!/usr/bin/env bash
# Run an unpacked ting archive the way somebody who downloaded it
# would: start the binary out of the directory it was unpacked into,
# and put it to work on real programs.
#
#   tools/smoke.sh UNPACKED-ROOT [version it must report]
#
# The version defaults to the one in Cargo.toml, so a caller that is
# checking a build of this tree does not have to dig it out in shell;
# the release passes the tag instead, which is the whole point there.
#
# The suites are copied in beside the binary on purpose. A script
# imports "lib/..." relative to its own directory, so selftest/ finds
# no lib/ and falls through to the stdlib compiled into the binary,
# while examples/ imports "../lib/..." and gets the lib/ the archive
# ships. One run therefore exercises both copies — which matters,
# because a lib/ next to a script silently shadows the embedded one
# and nothing else in this project notices.
set -eu

root=$1
# ${2-...}, not ${2:-...}: only an ABSENT argument takes the default.
# An argument that arrived empty is a caller whose shell went wrong,
# and falling back to Cargo.toml there would quietly check a release
# archive against this tree's version instead of against its tag.
want=${2-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}
[ -n "$want" ] || { echo "::error::the version to expect came through empty"; exit 1; }

ting=$root/ting
[ -f "$ting" ] || ting=$root/ting.exe
[ -f "$ting" ] || { echo "::error::no ting binary in $root"; exit 1; }
# Absolute, because the example loop runs from another directory.
ting=$(cd "$(dirname "$ting")" && pwd)/$(basename "$ting")

got=$("$ting" --version </dev/null)
if [ "$got" != "ting $want" ]; then
  echo "::error::the archive reports '$got', expected 'ting $want'"
  exit 1
fi
echo "started: $got"

rm -rf "$root/selftest" "$root/examples"
# The lib/ in the archive and the lib/ this binary embedded at compile
# time are two copies of the same twelve modules, and a packaging step
# that copied a stale or partial one would be invisible to everything
# else here: the binary would keep working on its embedded copy while
# a script beside the archive quietly got the other. So compare them.
diff -r lib "$root/lib"

cp -r selftest examples "$root"/
"$ting" --test "$root/selftest" </dev/null

clean=0
failed=0
for script in "$root"/examples/*.ting; do
  expected=${script%.ting}.out
  # Stdin is closed, not inherited: pipeline.ting reads it, and a
  # harness that leaves it open waits for a keystroke that is never
  # coming. One of those was still blocked on a terminal thirty hours
  # after the tick that started it.
  actual=$("$ting" "$script" </dev/null 2>/dev/null) || {
    echo "::error::$(basename "$script") exited nonzero"
    failed=$((failed + 1))
    continue
  }
  if [ "$actual" = "$(cat "$expected")" ]; then
    clean=$((clean + 1))
  else
    echo "::error::$(basename "$script") printed something else"
    failed=$((failed + 1))
  fi
done
echo "examples: $clean clean, $failed differing"
[ "$failed" -eq 0 ]
