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

    let span_end = span.end.clamp(span.start, line_end);
    let width = src[span.start.min(line_end)..span_end]
        .chars()
        .count()
        .max(1);
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
}
