#!/usr/bin/env python3
"""Time the release binary on every bench/*.ting.

Usage: python3 bench/run.py [--write]
  --write  also rewrite bench/BASELINE.md with these numbers

Each script prints a checksum line; a changed checksum means the
interpreter broke, not just slowed down. Medians of 5 runs.
"""

import pathlib
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
BINARY = ROOT / "target" / "release" / "ting"
RUNS = 5


def main() -> int:
    subprocess.run(
        ["cargo", "build", "--release", "--quiet"], cwd=ROOT, check=True
    )
    rows = []
    for script in sorted((ROOT / "bench").glob("*.ting")):
        times = []
        checksum = None
        for _ in range(RUNS):
            start = time.perf_counter()
            out = subprocess.run(
                [BINARY, script], capture_output=True, text=True, check=True
            )
            times.append((time.perf_counter() - start) * 1000)
            checksum = out.stdout.strip()
        median = sorted(times)[RUNS // 2]
        rows.append((script.name, median, checksum))
        print(f"{script.name:15} {median:8.1f} ms   [{checksum}]")

    if "--write" in sys.argv:
        lines = [
            "# Benchmark baseline",
            "",
            "Median of 5 runs of the release binary (`python3 bench/run.py`).",
            "Not a CI gate; regenerate with `--write` on the machine named",
            "below and compare like with like.",
            "",
            "| script | median | checksum |",
            "|--------|-------:|----------|",
        ]
        for name, median, checksum in rows:
            lines.append(f"| {name} | {median:.1f} ms | `{checksum}` |")
        import platform

        lines += ["", f"Recorded on: {platform.platform()} / {platform.machine()}", ""]
        (ROOT / "bench" / "BASELINE.md").write_text("\n".join(lines))
        print("wrote bench/BASELINE.md")
    return 0


if __name__ == "__main__":
    sys.exit(main())
