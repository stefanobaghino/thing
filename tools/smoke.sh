#!/usr/bin/env bash
# Run an unpacked ting archive the way somebody who downloaded it
# would: start the binary out of the directory it was unpacked into,
# and put it to work on real programs.
#
#   tools/smoke.sh <unpacked-root> <version it must report>
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
want=$2

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
