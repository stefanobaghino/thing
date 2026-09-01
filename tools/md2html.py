#!/usr/bin/env python3
"""Render one of this repo's markdown docs to a styled HTML page.

Stdlib only, and deliberately limited to the markdown this repo
actually writes: #/##/### headers, paragraphs, fenced code blocks,
tables, unordered lists, inline code/bold/links.

Usage: python3 tools/md2html.py input.md output.html
"""

import html
import re
import sys

STYLE = """
  * { box-sizing: border-box; }
  body { margin: 0; background: #14161a; color: #e6e6e6;
         font: 16px/1.6 system-ui, sans-serif; }
  nav { display: flex; gap: 1.2rem; align-items: baseline;
        padding: .6rem 1.2rem; border-bottom: 1px solid #2a2f38; }
  nav .name { font-weight: 600; }
  nav a { color: #8a919e; text-decoration: none; }
  nav a:hover { color: #e6e6e6; }
  main { max-width: 46rem; margin: 0 auto; padding: 1.5rem 1.2rem 4rem; }
  h1, h2, h3 { line-height: 1.25; }
  h1 { font-size: 1.7rem; } h2 { margin-top: 2.2rem; }
  a { color: #7fb4e0; }
  code { background: #1d2026; padding: .1em .35em; border-radius: 4px;
         font: .92em ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
  pre { background: #101216; border: 1px solid #2a2f38; border-radius: 8px;
        padding: .9rem 1rem; overflow-x: auto; }
  pre code { background: none; padding: 0; }
  table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
  th, td { border: 1px solid #2a2f38; padding: .4rem .6rem;
           text-align: left; vertical-align: top; }
  th { background: #1d2026; }
"""

NAV = (
    '<nav><span class="name">ting</span>'
    '<a href="./">playground</a>'
    '<a href="tutorial.html">tutorial</a>'
    '<a href="reference.html">reference</a>'
    '<a href="https://github.com/stefanobaghino/thing">github</a></nav>'
)


def inline(text: str) -> str:
    text = html.escape(text, quote=False)
    text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
    text = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", text)

    def link(m):
        href = m.group(2)
        href = re.sub(r"^docs/", "", href)
        href = re.sub(r"\.md$", ".html", href)
        return f'<a href="{href}">{m.group(1)}</a>'

    return re.sub(r"\[([^\]]+)\]\(([^)]+)\)", link, text)


def convert(md: str) -> str:
    out = []
    lines = md.splitlines()
    i = 0
    para: list[str] = []

    def flush_para():
        if para:
            out.append(f"<p>{inline(' '.join(para))}</p>")
            para.clear()

    while i < len(lines):
        line = lines[i]
        if line.startswith("```"):
            flush_para()
            i += 1
            block = []
            while i < len(lines) and not lines[i].startswith("```"):
                block.append(lines[i])
                i += 1
            code = html.escape("\n".join(block), quote=False)
            out.append(f"<pre><code>{code}</code></pre>")
        elif m := re.match(r"^(#{1,3}) (.*)$", line):
            flush_para()
            level = len(m.group(1))
            out.append(f"<h{level}>{inline(m.group(2))}</h{level}>")
        elif line.startswith("|"):
            flush_para()
            rows = []
            while i < len(lines) and lines[i].startswith("|"):
                cells = [c.strip() for c in lines[i].strip("|").split("|")]
                rows.append(cells)
                i += 1
            i -= 1
            head, body = rows[0], [r for r in rows[2:]]
            tr = lambda cells, tag: (
                "<tr>" + "".join(f"<{tag}>{inline(c)}</{tag}>" for c in cells) + "</tr>"
            )
            out.append(
                "<table><thead>"
                + tr(head, "th")
                + "</thead><tbody>"
                + "".join(tr(r, "td") for r in body)
                + "</tbody></table>"
            )
        elif line.startswith("- "):
            flush_para()
            items = []
            while i < len(lines) and (
                lines[i].startswith("- ") or lines[i].startswith("  ")
            ):
                if lines[i].startswith("- "):
                    items.append(lines[i][2:])
                else:
                    items[-1] += " " + lines[i].strip()
                i += 1
            i -= 1
            out.append(
                "<ul>" + "".join(f"<li>{inline(it)}</li>" for it in items) + "</ul>"
            )
        elif line.strip() == "":
            flush_para()
        else:
            para.append(line.strip())
        i += 1
    flush_para()
    return "\n".join(out)


def main() -> int:
    src_path, dst_path = sys.argv[1], sys.argv[2]
    md = open(src_path, encoding="utf-8").read()
    title_m = re.search(r"^# (.+)$", md, re.M)
    title = title_m.group(1) if title_m else "ting"
    body = convert(md)
    page = (
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n"
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        f"<title>{html.escape(title)}</title>\n<style>{STYLE}</style>\n</head>\n"
        f"<body>\n{NAV}\n<main>\n{body}\n</main>\n</body>\n</html>\n"
    )
    open(dst_path, "w", encoding="utf-8").write(page)
    print(f"wrote {dst_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
