//! Rendering of error diagnostics with a source excerpt.

use crate::lexer::Span;

/// Render a `path:line:col` header plus the offending line with a caret
/// underline covering the span (clamped to that line).
///
/// ```text
/// script.ting:3:9: error: undefined variable 'x'
///  3 | print(x + 1);
///    |       ^
/// ```
pub fn render(path: &str, src: &str, message: &str, span: Span) -> String {
    render_level(path, src, "error", message, span)
}

/// `render` with an explicit level word ("error", "warning").
pub fn render_level(path: &str, src: &str, level: &str, message: &str, span: Span) -> String {
    let (line, col) = span.line_col(src);
    let line_start = src[..span.start.min(src.len())]
        .rfind('\n')
        .map_or(0, |i| i + 1);
    let line_end = src[line_start..]
        .find('\n')
        .map_or(src.len(), |i| line_start + i);
    let text = &src[line_start..line_end];

    // A span past the line (a foreign offset) degrades to a one-wide
    // caret at the line's end rather than a panic.
    let span_start = span.start.min(line_end);
    let span_end = span.end.clamp(span_start, line_end);
    let width = src[span_start..span_end].chars().count().max(1);
    // Tabs in the prefix stay tabs so the caret lines up in terminals.
    let prefix: String = src[line_start..span.start.min(line_end)]
        .chars()
        .map(|c| if c == '\t' { '\t' } else { ' ' })
        .collect();

    let gutter = line.to_string();
    let pad = " ".repeat(gutter.len());
    let carets = "^".repeat(width);
    format!(
        "{path}:{line}:{col}: {level}: {message}\n {gutter} | {text}\n {pad} | {prefix}{carets}"
    )
}

/// The candidate nearest to `name` by edit distance, if one is close
/// enough to be worth suggesting: at most a third of the name wrong
/// (and always at least one edit, so short names still get help), or
/// a name of three characters or more that one of the two starts with
/// — `lenght` is three edits from `len`, but nobody doubts the intent.
/// Equal distances are settled by the longer shared start (`medain`
/// means `median`, not `mean`) and then alphabetically, so the answer
/// never depends on the order the candidates arrive in.
pub fn nearest<'a>(name: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let limit = (name.chars().count() / 3).max(1);
    let mut best: Option<(usize, usize, &str)> = None;
    for c in candidates {
        if c == name {
            continue;
        }
        let d = distance(name, c);
        let shares_start = c.chars().count() >= 3
            && name.chars().count() >= 3
            && (name.starts_with(c) || c.starts_with(name));
        if d > limit && !shares_start {
            continue;
        }
        let shared = name
            .chars()
            .zip(c.chars())
            .take_while(|(a, b)| a == b)
            .count();
        match best {
            // Nearer wins; then the longer shared start; then the name.
            Some((bd, bs, bc))
                if (bd, std::cmp::Reverse(bs), bc) <= (d, std::cmp::Reverse(shared), c) => {}
            _ => best = Some((d, shared, c)),
        }
    }
    best.map(|(_, _, c)| c.to_string())
}

/// Levenshtein distance in characters (insert, delete and substitute
/// each cost one), over one row of state.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caret_under_mid_line_span() {
        let src = "let x = 1;\nprint(y + 1);\n";
        let start = src.find('y').unwrap();
        let out = render(
            "t.ting",
            src,
            "undefined variable 'y'",
            Span::new(start, start + 1),
        );
        assert_eq!(
            out,
            "t.ting:2:7: error: undefined variable 'y'\n \
             2 | print(y + 1);\n   |       ^"
        );
    }

    #[test]
    fn caret_width_matches_span() {
        let src = "1 + true;";
        let out = render("t.ting", src, "boom", Span::new(4, 8));
        assert!(out.ends_with(" | 1 + true;\n   |     ^^^^"), "got:\n{out}");
    }

    #[test]
    fn span_at_end_of_input_gets_one_caret() {
        let src = "let x =";
        let out = render("t.ting", src, "expected expression", Span::new(7, 7));
        assert!(out.ends_with(" | let x =\n   |        ^"), "got:\n{out}");
    }

    #[test]
    fn multi_line_span_clamps_to_first_line() {
        let src = "if true {\n  1;\n}";
        let out = render("t.ting", src, "msg", Span::new(0, src.len()));
        assert!(out.contains(" | if true {\n"), "got:\n{out}");
        assert!(out.ends_with(" | ^^^^^^^^^"), "got:\n{out}");
    }

    #[test]
    fn tabs_keep_caret_aligned() {
        let src = "\tprint(z);";
        let start = src.find('z').unwrap();
        let out = render(
            "t.ting",
            src,
            "undefined variable 'z'",
            Span::new(start, start + 1),
        );
        assert!(out.ends_with(" | \t      ^"), "got:\n{out}");
    }

    #[test]
    fn unicode_before_span_counts_chars_not_bytes() {
        let src = "let héllo = wörld;";
        let start = src.find('w').unwrap();
        let out = render(
            "t.ting",
            src,
            "undefined variable 'wörld'",
            Span::new(start, start + "wörld".len()),
        );
        assert!(out.ends_with(" |             ^^^^^"), "got:\n{out}");
    }

    #[test]
    fn nearest_suggests_only_close_names() {
        assert_eq!(
            nearest("cont", ["count", "print", "len"]),
            Some("count".to_string())
        );
        assert_eq!(
            nearest("lenght", ["len", "length", "left"]),
            Some("length".to_string())
        );
        // Three edits, but "len" starts the name: still the answer.
        assert_eq!(nearest("lenght", ["len", "map"]), Some("len".to_string()));
        assert_eq!(
            nearest("prnt", ["print", "push"]),
            Some("print".to_string())
        );
        // Nothing within a third of the name is no suggestion at all.
        assert_eq!(nearest("elephant", ["print", "len"]), None);
        // The name itself is never its own suggestion.
        assert_eq!(nearest("len", ["len"]), None);
        // Ties go to the longer shared start, then to the first name.
        assert_eq!(
            nearest("medain", ["mean", "median"]),
            Some("median".to_string())
        );
        assert_eq!(
            nearest("medain", ["median", "mean"]),
            Some("median".to_string())
        );
        assert_eq!(nearest("ab", ["ac", "bb"]), Some("ac".to_string()));
        assert_eq!(nearest("ab", ["bb", "ac"]), Some("ac".to_string()));
    }
}
