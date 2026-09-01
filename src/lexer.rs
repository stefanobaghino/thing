//! Lexer for ting: turns source text into a Vec<Token> with byte-offset spans.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// 1-based (line, column) of the span's start, for diagnostics.
    pub fn line_col(&self, src: &str) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in src.char_indices() {
            if i >= self.start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Int(i64),
    Float(f64),
    Str(String),
    Ident(String),
    // keywords
    Let,
    Fn,
    If,
    Else,
    While,
    Return,
    True,
    False,
    Nil,
    // operators and punctuation
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Bang,
    AmpAmp,
    PipePipe,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Dot,
    Colon,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(src).run()
}

struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn error(&self, message: impl Into<String>, start: usize) -> LexError {
        LexError {
            message: message.into(),
            span: Span::new(start, self.pos),
        }
    }

    fn run(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_trivia();
            let start = self.pos;
            let Some(b) = self.bump() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: Span::new(start, start),
                });
                return Ok(tokens);
            };
            let kind = match b {
                b'+' => TokenKind::Plus,
                b'-' => TokenKind::Minus,
                b'*' => TokenKind::Star,
                b'/' => TokenKind::Slash,
                b'%' => TokenKind::Percent,
                b'(' => TokenKind::LParen,
                b')' => TokenKind::RParen,
                b'{' => TokenKind::LBrace,
                b'}' => TokenKind::RBrace,
                b'[' => TokenKind::LBracket,
                b']' => TokenKind::RBracket,
                b',' => TokenKind::Comma,
                b';' => TokenKind::Semi,
                b'.' => TokenKind::Dot,
                b':' => TokenKind::Colon,
                b'=' => self.two(b'=', TokenKind::EqEq, TokenKind::Eq),
                b'!' => self.two(b'=', TokenKind::BangEq, TokenKind::Bang),
                b'<' => self.two(b'=', TokenKind::LtEq, TokenKind::Lt),
                b'>' => self.two(b'=', TokenKind::GtEq, TokenKind::Gt),
                b'&' => {
                    if self.peek() == Some(b'&') {
                        self.pos += 1;
                        TokenKind::AmpAmp
                    } else {
                        return Err(self.error("expected '&&'", start));
                    }
                }
                b'|' => {
                    if self.peek() == Some(b'|') {
                        self.pos += 1;
                        TokenKind::PipePipe
                    } else {
                        return Err(self.error("expected '||'", start));
                    }
                }
                b'"' => self.string(start)?,
                b'0'..=b'9' => self.number(start)?,
                b if b.is_ascii_alphabetic() || b == b'_' => self.ident(start),
                _ => {
                    // Re-sync to the char boundary for a sane span on non-ASCII.
                    while self.peek().is_some_and(|b| b & 0xC0 == 0x80) {
                        self.pos += 1;
                    }
                    let ch = &self.src[start..self.pos];
                    return Err(self.error(format!("unexpected character '{ch}'"), start));
                }
            };
            tokens.push(Token {
                kind,
                span: Span::new(start, self.pos),
            });
        }
    }

    fn two(&mut self, second: u8, matched: TokenKind, single: TokenKind) -> TokenKind {
        if self.peek() == Some(second) {
            self.pos += 1;
            matched
        } else {
            single
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n') => {
                    self.pos += 1;
                }
                Some(b'#') => {
                    while self.peek().is_some_and(|b| b != b'\n') {
                        self.pos += 1;
                    }
                }
                _ => return,
            }
        }
    }

    fn string(&mut self, start: usize) -> Result<TokenKind, LexError> {
        let mut out = String::new();
        loop {
            match self.bump() {
                None | Some(b'\n') => {
                    return Err(self.error("unterminated string", start));
                }
                Some(b'"') => return Ok(TokenKind::Str(out)),
                Some(b'\\') => match self.bump() {
                    Some(b'n') => out.push('\n'),
                    Some(b't') => out.push('\t'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'"') => out.push('"'),
                    _ => return Err(self.error("invalid escape sequence", self.pos - 1)),
                },
                Some(b) if b < 0x80 => out.push(b as char),
                Some(_) => {
                    // Multi-byte UTF-8: copy the whole char.
                    let ch_start = self.pos - 1;
                    while self.peek().is_some_and(|b| b & 0xC0 == 0x80) {
                        self.pos += 1;
                    }
                    out.push_str(&self.src[ch_start..self.pos]);
                }
            }
        }
    }

    fn number(&mut self, start: usize) -> Result<TokenKind, LexError> {
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
        }
        let is_float = self.peek() == Some(b'.') && self.peek2().is_some_and(|b| b.is_ascii_digit());
        if is_float {
            self.pos += 1; // '.'
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = &self.src[start..self.pos];
        if is_float {
            text.parse::<f64>()
                .map(TokenKind::Float)
                .map_err(|_| self.error("invalid float literal", start))
        } else {
            text.parse::<i64>()
                .map(TokenKind::Int)
                .map_err(|_| self.error("integer literal too large", start))
        }
    }

    fn ident(&mut self, start: usize) -> TokenKind {
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.pos += 1;
        }
        match &self.src[start..self.pos] {
            "let" => TokenKind::Let,
            "fn" => TokenKind::Fn,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            name => TokenKind::Ident(name.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn empty_source_yields_eof() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn integers_and_floats() {
        assert_eq!(
            kinds("42 2.5 0"),
            vec![
                TokenKind::Int(42),
                TokenKind::Float(2.5),
                TokenKind::Int(0),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn int_followed_by_dot_is_not_a_float() {
        assert_eq!(
            kinds("1.foo"),
            vec![
                TokenKind::Int(1),
                TokenKind::Dot,
                TokenKind::Ident("foo".into()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn strings_with_escapes() {
        assert_eq!(
            kinds(r#""hi\n\t\"\\""#),
            vec![TokenKind::Str("hi\n\t\"\\".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn unicode_in_strings() {
        assert_eq!(
            kinds(r#""héllo → wörld""#),
            vec![TokenKind::Str("héllo → wörld".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn unterminated_string_errors() {
        let err = lex("\"abc").unwrap_err();
        assert_eq!(err.message, "unterminated string");
    }

    #[test]
    fn newline_terminates_string_with_error() {
        assert!(lex("\"abc\ndef\"").is_err());
    }

    #[test]
    fn keywords_vs_identifiers() {
        assert_eq!(
            kinds("let letter fn fnord if nil nils"),
            vec![
                TokenKind::Let,
                TokenKind::Ident("letter".into()),
                TokenKind::Fn,
                TokenKind::Ident("fnord".into()),
                TokenKind::If,
                TokenKind::Nil,
                TokenKind::Ident("nils".into()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn operators_single_and_double() {
        assert_eq!(
            kinds("= == ! != < <= > >= && ||"),
            vec![
                TokenKind::Eq,
                TokenKind::EqEq,
                TokenKind::Bang,
                TokenKind::BangEq,
                TokenKind::Lt,
                TokenKind::LtEq,
                TokenKind::Gt,
                TokenKind::GtEq,
                TokenKind::AmpAmp,
                TokenKind::PipePipe,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn punctuation() {
        assert_eq!(
            kinds("( ) { } [ ] , ; . : + - * / %"),
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Comma,
                TokenKind::Semi,
                TokenKind::Dot,
                TokenKind::Colon,
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn comments_are_skipped() {
        assert_eq!(
            kinds("1 # a comment\n2"),
            vec![TokenKind::Int(1), TokenKind::Int(2), TokenKind::Eof]
        );
    }

    #[test]
    fn lone_ampersand_errors() {
        assert_eq!(lex("&").unwrap_err().message, "expected '&&'");
    }

    #[test]
    fn unexpected_character_errors_with_char() {
        let err = lex("let x = €").unwrap_err();
        assert_eq!(err.message, "unexpected character '€'");
    }

    #[test]
    fn spans_and_line_col() {
        let src = "let x =\n  42";
        let tokens = lex(src).unwrap();
        let forty_two = &tokens[3];
        assert_eq!(forty_two.kind, TokenKind::Int(42));
        assert_eq!(&src[forty_two.span.start..forty_two.span.end], "42");
        assert_eq!(forty_two.span.line_col(src), (2, 3));
    }

    #[test]
    fn integer_overflow_errors() {
        assert_eq!(
            lex("99999999999999999999").unwrap_err().message,
            "integer literal too large"
        );
    }
}
