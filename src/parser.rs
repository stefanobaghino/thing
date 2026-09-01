//! Pratt (precedence-climbing) expression parser for ting.

use crate::ast::{BinaryOp, Expr, ExprKind, UnaryOp};
use crate::lexer::{Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

/// Parse a complete expression; every token before Eof must be consumed.
pub fn parse_expr(tokens: &[Token]) -> Result<Expr, ParseError> {
    let mut p = Parser { tokens, pos: 0 };
    let expr = p.expr_bp(0)?;
    match p.peek() {
        TokenKind::Eof => Ok(expr),
        k => Err(p.error(format!("unexpected {}", describe(k)))),
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            span: self.span(),
        }
    }

    fn expect(&mut self, kind: &TokenKind, what: &str) -> Result<(), ParseError> {
        if self.peek() == kind {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected {what}, found {}", describe(self.peek()))))
        }
    }

    fn expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.unary()?;
        while let Some((op, lbp, rbp)) = binop(self.peek()) {
            if lbp < min_bp {
                break;
            }
            self.advance();
            let rhs = self.expr_bp(rbp)?;
            let span = Span::new(lhs.span.start, rhs.span.end);
            lhs = Expr {
                kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                span,
            };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        let op = match self.peek() {
            TokenKind::Minus => Some(UnaryOp::Neg),
            TokenKind::Bang => Some(UnaryOp::Not),
            _ => None,
        };
        if let Some(op) = op {
            let start = self.span().start;
            self.advance();
            let operand = self.unary()?;
            let span = Span::new(start, operand.span.end);
            return Ok(Expr {
                kind: ExprKind::Unary(op, Box::new(operand)),
                span,
            });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        loop {
            match self.peek() {
                TokenKind::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != &TokenKind::RParen {
                        loop {
                            args.push(self.expr_bp(0)?);
                            if self.peek() == &TokenKind::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    let end = self.span().end;
                    self.expect(&TokenKind::RParen, "')'")?;
                    let span = Span::new(expr.span.start, end);
                    expr = Expr {
                        kind: ExprKind::Call(Box::new(expr), args),
                        span,
                    };
                }
                TokenKind::LBracket => {
                    self.advance();
                    let idx = self.expr_bp(0)?;
                    let end = self.span().end;
                    self.expect(&TokenKind::RBracket, "']'")?;
                    let span = Span::new(expr.span.start, end);
                    expr = Expr {
                        kind: ExprKind::Index(Box::new(expr), Box::new(idx)),
                        span,
                    };
                }
                _ => return Ok(expr),
            }
        }
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();
        let kind = match self.peek().clone() {
            TokenKind::Int(n) => ExprKind::Int(n),
            TokenKind::Float(x) => ExprKind::Float(x),
            TokenKind::Str(s) => ExprKind::Str(s),
            TokenKind::True => ExprKind::Bool(true),
            TokenKind::False => ExprKind::Bool(false),
            TokenKind::Nil => ExprKind::Nil,
            TokenKind::Ident(name) => ExprKind::Var(name),
            TokenKind::LParen => {
                self.advance();
                let inner = self.expr_bp(0)?;
                self.expect(&TokenKind::RParen, "')'")?;
                return Ok(inner);
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if self.peek() != &TokenKind::RBracket {
                    loop {
                        items.push(self.expr_bp(0)?);
                        if self.peek() == &TokenKind::Comma {
                            self.advance();
                            // allow trailing comma
                            if self.peek() == &TokenKind::RBracket {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                let end = self.span().end;
                self.expect(&TokenKind::RBracket, "']'")?;
                return Ok(Expr {
                    kind: ExprKind::List(items),
                    span: Span::new(span.start, end),
                });
            }
            k => return Err(self.error(format!("expected expression, found {}", describe(&k)))),
        };
        self.advance();
        Ok(Expr { kind, span })
    }
}

/// (operator, left binding power, right binding power); left-assoc: rbp = lbp + 1.
fn binop(kind: &TokenKind) -> Option<(BinaryOp, u8, u8)> {
    Some(match kind {
        TokenKind::PipePipe => (BinaryOp::Or, 1, 2),
        TokenKind::AmpAmp => (BinaryOp::And, 3, 4),
        TokenKind::EqEq => (BinaryOp::Eq, 5, 6),
        TokenKind::BangEq => (BinaryOp::Ne, 5, 6),
        TokenKind::Lt => (BinaryOp::Lt, 7, 8),
        TokenKind::LtEq => (BinaryOp::Le, 7, 8),
        TokenKind::Gt => (BinaryOp::Gt, 7, 8),
        TokenKind::GtEq => (BinaryOp::Ge, 7, 8),
        TokenKind::Plus => (BinaryOp::Add, 9, 10),
        TokenKind::Minus => (BinaryOp::Sub, 9, 10),
        TokenKind::Star => (BinaryOp::Mul, 11, 12),
        TokenKind::Slash => (BinaryOp::Div, 11, 12),
        TokenKind::Percent => (BinaryOp::Rem, 11, 12),
        _ => return None,
    })
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Int(n) => format!("integer '{n}'"),
        TokenKind::Float(x) => format!("float '{x}'"),
        TokenKind::Str(_) => "string literal".to_string(),
        TokenKind::Ident(name) => format!("identifier '{name}'"),
        TokenKind::Eof => "end of input".to_string(),
        k => {
            let text = match k {
                TokenKind::Let => "let",
                TokenKind::Fn => "fn",
                TokenKind::If => "if",
                TokenKind::Else => "else",
                TokenKind::While => "while",
                TokenKind::Return => "return",
                TokenKind::True => "true",
                TokenKind::False => "false",
                TokenKind::Nil => "nil",
                TokenKind::Plus => "+",
                TokenKind::Minus => "-",
                TokenKind::Star => "*",
                TokenKind::Slash => "/",
                TokenKind::Percent => "%",
                TokenKind::Eq => "=",
                TokenKind::EqEq => "==",
                TokenKind::BangEq => "!=",
                TokenKind::Lt => "<",
                TokenKind::LtEq => "<=",
                TokenKind::Gt => ">",
                TokenKind::GtEq => ">=",
                TokenKind::Bang => "!",
                TokenKind::AmpAmp => "&&",
                TokenKind::PipePipe => "||",
                TokenKind::LParen => "(",
                TokenKind::RParen => ")",
                TokenKind::LBrace => "{",
                TokenKind::RBrace => "}",
                TokenKind::LBracket => "[",
                TokenKind::RBracket => "]",
                TokenKind::Comma => ",",
                TokenKind::Semi => ";",
                TokenKind::Dot => ".",
                _ => unreachable!(),
            };
            format!("'{text}'")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn sexpr(src: &str) -> String {
        parse_expr(&lex(src).unwrap()).unwrap().to_string()
    }

    fn err(src: &str) -> String {
        parse_expr(&lex(src).unwrap()).unwrap_err().message
    }

    #[test]
    fn precedence_mul_over_add() {
        assert_eq!(sexpr("1 + 2 * 3"), "(+ 1 (* 2 3))");
    }

    #[test]
    fn left_associativity() {
        assert_eq!(sexpr("10 - 3 - 2"), "(- (- 10 3) 2)");
        assert_eq!(sexpr("20 / 2 / 5"), "(/ (/ 20 2) 5)");
    }

    #[test]
    fn parens_override_precedence() {
        assert_eq!(sexpr("(1 + 2) * 3"), "(* (+ 1 2) 3)");
    }

    #[test]
    fn logical_precedence() {
        assert_eq!(
            sexpr("a || b && c == d < e + f"),
            "(|| a (&& b (== c (< d (+ e f)))))"
        );
    }

    #[test]
    fn unary_binds_tighter_than_binary() {
        assert_eq!(sexpr("-1 + 2"), "(+ (- 1) 2)");
        assert_eq!(sexpr("!a && b"), "(&& (! a) b)");
        assert_eq!(sexpr("--x"), "(- (- x))");
    }

    #[test]
    fn calls_and_args() {
        assert_eq!(sexpr("f()"), "(call f)");
        assert_eq!(sexpr("f(1, 2 + 3)"), "(call f 1 (+ 2 3))");
        assert_eq!(sexpr("f(1)(2)"), "(call (call f 1) 2)");
    }

    #[test]
    fn indexing_chains_and_mixes_with_calls() {
        assert_eq!(sexpr("xs[0]"), "(index xs 0)");
        assert_eq!(sexpr("m[k][0]"), "(index (index m k) 0)");
        assert_eq!(sexpr("f(x)[1]"), "(index (call f x) 1)");
    }

    #[test]
    fn list_literals() {
        assert_eq!(sexpr("[]"), "(list)");
        assert_eq!(sexpr("[1, 2 * 3, \"x\",]"), "(list 1 (* 2 3) \"x\")");
    }

    #[test]
    fn literals() {
        assert_eq!(sexpr("nil"), "nil");
        assert_eq!(sexpr("true"), "true");
        assert_eq!(sexpr("2.5"), "2.5");
        assert_eq!(sexpr("\"hi\""), "\"hi\"");
    }

    #[test]
    fn call_binds_tighter_than_unary_minus() {
        assert_eq!(sexpr("-f(1)"), "(- (call f 1))");
    }

    #[test]
    fn trailing_tokens_are_an_error() {
        assert_eq!(err("1 2"), "unexpected integer '2'");
    }

    #[test]
    fn missing_operand_is_an_error() {
        assert_eq!(err("1 +"), "expected expression, found end of input");
    }

    #[test]
    fn unclosed_paren_is_an_error() {
        assert_eq!(err("(1 + 2"), "expected ')', found end of input");
    }

    #[test]
    fn spans_cover_whole_expression() {
        let src = "1 + 2 * 3";
        let expr = parse_expr(&lex(src).unwrap()).unwrap();
        assert_eq!(&src[expr.span.start..expr.span.end], src);
    }
}
