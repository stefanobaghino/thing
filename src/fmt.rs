//! Canonical source formatter. Works on the token stream (not the
//! AST) so comments survive; literal text is copied verbatim from the
//! source via spans. Guarantees checked by tests: idempotent, and the
//! formatted program parses to the identical AST.
//!
//! Rules: two-space indentation from brace depth, plus one level for
//! every `[` or `(` that a line break happens inside, counted for the
//! innermost open delimiter only, so a `{` that already took a level
//! does not take a second one; the author's line breaks are kept
//! (runs of blank lines
//! collapse to one); canonical single spaces between tokens; trailing
//! comments get two spaces before the `#`.

use crate::lexer::{self, LexError, Span, TokenKind};

/// An open delimiter. A brace takes its level at the opener; a `[` or
/// `(` takes one the first time a line break happens while it is the
/// innermost delimiter open, and gives it back at its closer.
enum Open {
    /// true = map literal, false = block.
    Brace(bool),
    /// true = a line break happened inside it.
    Delim(bool),
}

enum Piece {
    Token(TokenKind, Span),
    Comment(Span),
}

/// Tokens plus comments plus blank-line info, reconstructed from the
/// ordinary lexer output: everything between two token spans is
/// whitespace and comments by construction, so scanning those gaps
/// for `#` is safe (never inside a string).
fn pieces(src: &str) -> Result<Vec<(Piece, usize)>, LexError> {
    let tokens = lexer::lex(src)?;
    let mut out: Vec<(Piece, usize)> = Vec::new();
    let mut pos = 0usize;
    for t in tokens {
        let gap_end = t.span.start;
        let mut newlines = 0usize;
        let mut i = pos;
        let bytes = src.as_bytes();
        while i < gap_end {
            match bytes[i] {
                b'\n' => {
                    newlines += 1;
                    i += 1;
                }
                b'#' => {
                    let start = i;
                    while i < gap_end && bytes[i] != b'\n' {
                        i += 1;
                    }
                    out.push((Piece::Comment(Span::new(start, i)), newlines));
                    newlines = 0;
                }
                _ => i += 1,
            }
        }
        let is_eof = matches!(t.kind, TokenKind::Eof);
        if !is_eof {
            out.push((Piece::Token(t.kind, t.span), newlines));
        }
        pos = gap_end.max(t.span.end);
        if is_eof {
            break;
        }
    }
    Ok(out)
}

fn value_like(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Ident(_)
            | TokenKind::Int(_)
            | TokenKind::Float(_)
            | TokenKind::Str(_)
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
    )
}

/// A `{` opens a map literal when it sits in expression position.
fn brace_is_map(prev: Option<&TokenKind>) -> bool {
    use TokenKind::*;
    matches!(
        prev,
        Some(
            // `let {a, b} = m;` and `for {a, b} in ms` open a
            // pattern, not a block, and a pattern is written the way
            // the literal it matches is.
            Let | For
                | In
                | Eq
                | LParen
                | LBracket
                | Comma
                | Colon
                | Return
                | Plus
                | Minus
                | Star
                | Slash
                | Percent
                | EqEq
                | BangEq
                | Lt
                | LtEq
                | Gt
                | GtEq
                | AmpAmp
                | PipePipe
                | Amp
                | Pipe
                | Caret
                | Shl
                | Shr
                | Tilde
                | Bang
        )
    )
}

fn needs_space(prev: &TokenKind, prev2: Option<&TokenKind>, cur: &TokenKind) -> bool {
    use TokenKind::*;
    match cur {
        Comma | Semi | Colon | RParen | RBracket | Dot => false,
        // `fn(`, `!(`, and unary `-(` stay tight.
        LParen | LBracket => {
            !(matches!(prev, LParen | LBracket | Fn | Bang | Tilde | Ellipsis)
                || (matches!(prev, Minus) && !prev2.map(value_like).unwrap_or(false))
                || value_like(prev))
        }
        _ => match prev {
            LParen | LBracket | Dot => false,
            // `...name` is one thing, the way `!x` is.
            Ellipsis => false,
            Bang | Tilde => false,
            // Unary minus: previous minus not preceded by a value.
            Minus if !prev2.map(value_like).unwrap_or(false) => false,
            _ => true,
        },
    }
}

/// Format ting source. Errors only if the source doesn't lex. A source
/// with CRLF line endings formats to CRLF, so a Windows checkout is
/// neither "unformatted" nor rewritten to LF by --fmt.
pub fn format(src: &str) -> Result<String, LexError> {
    if src.contains("\r\n") {
        let lf = src.replace("\r\n", "\n");
        return format_lf(&lf).map(|out| out.replace('\n', "\r\n"));
    }
    format_lf(src)
}

fn format_lf(src: &str) -> Result<String, LexError> {
    let pieces = pieces(src)?;
    let mut out = String::new();
    let mut depth: usize = 0;
    let mut at_line_start = true;
    let mut prev: Option<TokenKind> = None;
    let mut prev2: Option<TokenKind> = None;
    // Open delimiters, innermost last. A brace's map-or-block is
    // decided from the token before `{`: expression positions mean a map.
    let mut opens: Vec<Open> = Vec::new();

    for (piece, newlines) in pieces.iter() {
        if *newlines > 0 && !out.is_empty() {
            // The line continues whatever delimiter is open around it.
            if let Some(Open::Delim(crossed)) = opens.last_mut()
                && !*crossed
            {
                *crossed = true;
                depth += 1;
            }
            out.push('\n');
            if *newlines >= 2 {
                out.push('\n');
            }
            at_line_start = true;
            prev = None;
            prev2 = None;
        }
        match piece {
            Piece::Comment(span) => {
                if at_line_start {
                    out.push_str(&"  ".repeat(depth));
                    at_line_start = false;
                } else {
                    out.push_str("  ");
                }
                out.push_str(src[span.start..span.end].trim_end());
            }
            Piece::Token(kind, span) => {
                let closes_continuation = matches!(kind, TokenKind::RBracket | TokenKind::RParen)
                    && matches!(opens.last(), Some(Open::Delim(true)));
                let line_depth = if matches!(kind, TokenKind::RBrace) || closes_continuation {
                    depth.saturating_sub(1)
                } else {
                    depth
                };
                if at_line_start {
                    out.push_str(&"  ".repeat(line_depth));
                    at_line_start = false;
                } else if let Some(p) = &prev {
                    let space =
                        if matches!(kind, TokenKind::RBrace) || matches!(p, TokenKind::LBrace) {
                            // A block's braces stand apart ("{ }"), a map's tight.
                            !matches!(opens.last(), Some(Open::Brace(true)))
                        } else {
                            needs_space(p, prev2.as_ref(), kind)
                        };
                    if space {
                        out.push(' ');
                    }
                }
                out.push_str(&src[span.start..span.end]);
                match kind {
                    TokenKind::LBrace => {
                        depth += 1;
                        opens.push(Open::Brace(brace_is_map(prev.as_ref())));
                    }
                    TokenKind::RBrace => {
                        depth = depth.saturating_sub(1);
                        if matches!(opens.last(), Some(Open::Brace(_))) {
                            opens.pop();
                        }
                    }
                    TokenKind::LBracket | TokenKind::LParen => opens.push(Open::Delim(false)),
                    TokenKind::RBracket | TokenKind::RParen => {
                        if let Some(Open::Delim(crossed)) = opens.last() {
                            let crossed = *crossed;
                            opens.pop();
                            depth = depth.saturating_sub(usize::from(crossed));
                        }
                    }
                    _ => {}
                }
                prev2 = prev.take();
                prev = Some(kind.clone());
            }
        }
    }
    while out.ends_with('\n') {
        out.pop();
    }
    if !out.is_empty() {
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_spacing() {
        assert_eq!(format("let x=1+2*3;").unwrap(), "let x = 1 + 2 * 3;\n");
        assert_eq!(
            format("print ( x [ 0 ] , y ) ;").unwrap(),
            "print(x[0], y);\n"
        );
        assert_eq!(
            format("let m={\"a\":1,\"b\":[1,2]};").unwrap(),
            "let m = {\"a\": 1, \"b\": [1, 2]};\n"
        );
        assert_eq!(format("let y=-x+f(-1);").unwrap(), "let y = -x + f(-1);\n");
        assert_eq!(
            format("print(! ( a )&&- ( b ));").unwrap(),
            "print(!(a) && -(b));\n"
        );
        assert_eq!(format("if !ok&&a<b { }").unwrap(), "if !ok && a < b { }\n");
    }

    #[test]
    fn indentation_follows_braces() {
        let src = "fn f(n) {\nif n<2 {\nreturn n;\n}\nreturn f(n-1);\n}";
        assert_eq!(
            format(src).unwrap(),
            "fn f(n) {\n  if n < 2 {\n    return n;\n  }\n  return f(n - 1);\n}\n"
        );
    }

    #[test]
    fn delimiters_indent_their_continuation() {
        let src = "let xs = [\n\"a\",\n[1,\n2],\n];\nprint(\nxs,\nlen(xs)\n);";
        assert_eq!(
            format(src).unwrap(),
            "let xs = [\n  \"a\",\n  [1,\n    2],\n];\nprint(\n  xs,\n  len(xs)\n);\n"
        );
        // The break need not follow the opener: what continues a call is
        // indented whether or not the opener ended its line.
        assert_eq!(format("print(a,\nb);").unwrap(), "print(a,\n  b);\n");
        // One level per delimiter, innermost first.
        assert_eq!(
            format("foo(bar(a,\nb),\nc);").unwrap(),
            "foo(bar(a,\n  b),\n  c);\n"
        );
        // A `{` has taken the level already, so the `(` around it does
        // not take a second one (closure-as-argument, map-as-argument).
        let src = "sort_by(xs, fn(a) {\nreturn a;\n});";
        assert_eq!(
            format(src).unwrap(),
            "sort_by(xs, fn(a) {\n  return a;\n});\n"
        );
        assert_eq!(
            format("foo({\n\"a\": 1,\n});").unwrap(),
            "foo({\n  \"a\": 1,\n});\n"
        );
    }

    #[test]
    fn comments_and_blank_lines_survive() {
        let src = "# header\n\n\n\nlet x = 1;   # trailing\n# footer";
        assert_eq!(
            format(src).unwrap(),
            "# header\n\nlet x = 1;  # trailing\n# footer\n"
        );
    }

    #[test]
    fn anonymous_fn_and_map_after_in() {
        assert_eq!(
            format("let f=fn (x) {return x;};").unwrap(),
            "let f = fn(x) { return x; };\n"
        );
        assert_eq!(
            format("for k in { \"a\" : 1 } { print(k); }").unwrap(),
            "for k in {\"a\": 1} { print(k); }\n"
        );
    }

    #[test]
    fn one_line_blocks_keep_inner_spaces() {
        assert_eq!(
            format("fn t() {n=n+1;return n;}").unwrap(),
            "fn t() { n = n + 1; return n; }\n"
        );
    }

    #[test]
    fn a_default_is_spaced_like_an_assignment() {
        assert_eq!(
            format("fn f(a,b=1,c = 2+3,d=[1,2]) { return a; }").unwrap(),
            "fn f(a, b = 1, c = 2 + 3, d = [1, 2]) { return a; }\n"
        );
        assert_eq!(
            format("let g = fn(x=nil){return x;};").unwrap(),
            "let g = fn(x = nil) { return x; };\n"
        );
    }

    #[test]
    fn a_spread_argument_keeps_its_dots() {
        assert_eq!(
            format("print(... xs,... [1]);").unwrap(),
            "print(...xs, ...[1]);\n"
        );
    }

    #[test]
    fn a_rest_parameter_keeps_its_dots() {
        assert_eq!(
            format("fn f(a,... rest){return rest;}").unwrap(),
            "fn f(a, ...rest) { return rest; }\n"
        );
    }

    #[test]
    fn idempotent_on_samples() {
        for src in [
            "let x=1;# c\nif x==1 {\nprint(x);\n}\n",
            "fn f(a,b) {return a*-b;}\nprint(f(1,2),[1,-2],{\"k\":-3});",
        ] {
            let once = format(src).unwrap();
            assert_eq!(format(&once).unwrap(), once, "not idempotent for:\n{src}");
        }
    }
}
