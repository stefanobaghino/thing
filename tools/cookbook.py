#!/usr/bin/env python3
"""Render examples/*.ting and their .out files into docs/cookbook.md.

Usage: python3 tools/cookbook.py
Commit the result; tests/docs.rs checks the page carries every
example's source and output verbatim, so a stale page fails CI.
"""

import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
EXAMPLES = ROOT / "examples"
OUT = ROOT / "docs" / "cookbook.md"

HEADER = """# Cookbook

Every program under `examples/` in the repository, with the output it
prints — the same pairs CI runs on every commit, so these never drift.
Copy one, run it with `ting name.ting`, change it.

"""


def main() -> None:
    parts = [HEADER]
    for src in sorted(EXAMPLES.glob("*.ting")):
        out = src.with_suffix(".out")
        body = src.read_text()
        lines = body.split("\n")
        intro = []
        while lines and lines[0].startswith("#"):
            intro.append(lines.pop(0).lstrip("#").strip())
        parts.append(f"## {src.stem}\n\n")
        if intro:
            parts.append(" ".join(intro) + "\n\n")
        parts.append("```ting\n" + body.rstrip("\n") + "\n```\n\n")
        parts.append("```text\n" + out.read_text().rstrip("\n") + "\n```\n\n")
    OUT.write_text("".join(parts).rstrip("\n") + "\n")


if __name__ == "__main__":
    main()
