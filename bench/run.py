#!/usr/bin/env python3
"""Time the release binary on every bench/*.ting.

Usage: python3 bench/run.py [--write]
  --write  also rewrite bench/BASELINE.md with these numbers

Each script prints a checksum line; a changed checksum means the
interpreter broke, not just slowed down. Medians of 5 runs.
"""

import os
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
    def measure(script, engine):
        times = []
        checksum = None
        env = dict(os.environ, TING_ENGINE=engine)
        for _ in range(RUNS):
            start = time.perf_counter()
            out = subprocess.run(
                [BINARY, script], capture_output=True, text=True, check=True, env=env
            )
            times.append((time.perf_counter() - start) * 1000)
            checksum = out.stdout.strip()
        return sorted(times)[RUNS // 2], checksum

    rows = []
    for script in sorted((ROOT / "bench").glob("*.ting")):
        eval_ms, checksum = measure(script, "eval")
        vm_ms, vm_checksum = measure(script, "vm")
        assert checksum == vm_checksum, f"{script.name}: engines disagree!"
        rows.append((script.name, eval_ms, vm_ms, checksum))
        delta = (vm_ms - eval_ms) / eval_ms * 100
        print(
            f"{script.name:15} eval {eval_ms:7.1f} ms   vm {vm_ms:7.1f} ms "
            f"({delta:+.0f}%)   [{checksum}]"
        )

    if "--write" in sys.argv:
        lines = [
            "# Benchmark baseline",
            "",
            "Median of 5 runs of the release binary (`python3 bench/run.py`),",
            "for both engines. Not a CI gate; regenerate with `--write` on",
            "the machine named below and compare like with like.",
            "",
            "| script | eval | vm | checksum |",
            "|--------|-----:|---:|----------|",
        ]
        for name, eval_ms, vm_ms, checksum in rows:
            lines.append(f"| {name} | {eval_ms:.1f} ms | {vm_ms:.1f} ms | `{checksum}` |")
        import platform

        lines += ["", f"Recorded on: {platform.platform()} / {platform.machine()}", ""]
        (ROOT / "bench" / "BASELINE.md").write_text("\n".join(lines))
        print("wrote bench/BASELINE.md")
    return 0


if __name__ == "__main__":
    sys.exit(main())
