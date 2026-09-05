//! Pratt (precedence-climbing) expression parser for ting.

use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::lexer::{Span, Token, TokenKind};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

/// Parse a whole program: a sequence of statements up to Eof.
pub fn parse_program(tokens: &[Token]) -> Result<Vec<Stmt>, ParseError> {
    let mut p = Parser { tokens, pos: 0 };
    let mut stmts = Vec::new();
    while p.peek() != &TokenKind::Eof {
        stmts.push(p.statement()?);
    }
    Ok(stmts)
}

/// Parse a complete expression; every token before Eof must be consumed.
/// Used by the REPL to echo expression results.
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

/// (params, body, byte offset just past the closing brace)
type FnParts = (Vec<crate::ast::Param>, Rc<Vec<Stmt>>, usize);

impl<'a> Parser<'a> {
    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek2(&self) -> &TokenKind {
        let i = (self.pos + 1).min(self.tokens.len() - 1);
        &self.tokens[i].kind
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

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.span().start;
        match self.peek() {
            TokenKind::Let => {
                self.advance();
                let name = match self.peek().clone() {
                    TokenKind::Ident(name) => {
                        self.advance();
                        name
                    }
                    k => {
                        return Err(
                            self.error(format!("expected variable name, found {}", describe(&k)))
                        );
                    }
                };
                self.expect(&TokenKind::Eq, "'='")?;
                let init = self.expr_bp(0)?;
                let end = self.span().end;
                self.expect(&TokenKind::Semi, "';'")?;
                Ok(Stmt {
                    kind: StmtKind::Let(name, init),
                    span: Span::new(start, end),
                })
            }
            TokenKind::LBrace => {
                self.advance();
                let mut stmts = Vec::new();
                while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                    stmts.push(self.statement()?);
                }
                let end = self.span().end;
                self.expect(&TokenKind::RBrace, "'}'")?;
                Ok(Stmt {
                    kind: StmtKind::Block(stmts),
                    span: Span::new(start, end),
                })
            }
            TokenKind::If => {
                self.advance();
                let cond = self.expr_bp(0)?;
                let then = self.block_stmt("after 'if' condition")?;
                let els = if self.peek() == &TokenKind::Else {
                    self.advance();
                    // `else if ...` chains; otherwise a block is required.
                    let branch = if self.peek() == &TokenKind::If {
                        self.statement()?
                    } else {
                        self.block_stmt("after 'else'")?
                    };
                    Some(Box::new(branch))
                } else {
                    None
                };
                let end = els.as_ref().map_or_else(|| then.span.end, |e| e.span.end);
                Ok(Stmt {
                    kind: StmtKind::If(cond, Box::new(then), els),
                    span: Span::new(start, end),
                })
            }
            // `fn name(...) { ... }` declaration; a lone `fn(...)` falls
            // through to the expression path (anonymous function).
            TokenKind::Fn if matches!(self.peek2(), TokenKind::Ident(_)) => {
                self.advance();
                let name = match self.peek().clone() {
                    TokenKind::Ident(name) => {
                        self.advance();
                        name
                    }
                    _ => unreachable!("guarded by peek2"),
                };
                let (params, body, end) = self.fn_params_and_body()?;
                // Desugars to `let name = fn(...) {...};` — recursion works
                // because the closure and the binding share the same
                // environment at call time.
                Ok(Stmt {
                    kind: StmtKind::Let(
                        name,
                        Expr {
                            kind: ExprKind::Fn(params, body),
                            span: Span::new(start, end),
                        },
                    ),
                    span: Span::new(start, end),
                })
            }
            TokenKind::Return => {
                self.advance();
                let value = if self.peek() == &TokenKind::Semi {
                    None
                } else {
                    Some(self.expr_bp(0)?)
                };
                let end = self.span().end;
                self.expect(&TokenKind::Semi, "';'")?;
                Ok(Stmt {
                    kind: StmtKind::Return(value),
                    span: Span::new(start, end),
                })
            }
            TokenKind::While => {
                self.advance();
                let cond = self.expr_bp(0)?;
                let body = self.block_stmt("after 'while' condition")?;
                let end = body.span.end;
                Ok(Stmt {
                    kind: StmtKind::While(cond, Box::new(body)),
                    span: Span::new(start, end),
                })
            }
            TokenKind::For => {
                self.advance();
                let var = match self.peek().clone() {
                    TokenKind::Ident(name) => {
                        self.advance();
                        name
                    }
                    k => {
                        return Err(
                            self.error(format!("expected loop variable, found {}", describe(&k)))
                        );
                    }
                };
                self.expect(&TokenKind::In, "'in'")?;
                let iterable = self.expr_bp(0)?;
                let body = self.block_stmt("after 'for' iterable")?;
                let end = body.span.end;
                Ok(Stmt {
                    kind: StmtKind::For(var, iterable, Box::new(body)),
                    span: Span::new(start, end),
                })
            }
            TokenKind::Break => {
                self.advance();
                let end = self.span().end;
                self.expect(&TokenKind::Semi, "';'")?;
                Ok(Stmt {
                    kind: StmtKind::Break,
                    span: Span::new(start, end),
                })
            }
            TokenKind::Continue => {
                self.advance();
                let end = self.span().end;
                self.expect(&TokenKind::Semi, "';'")?;
                Ok(Stmt {
                    kind: StmtKind::Continue,
                    span: Span::new(start, end),
                })
            }
            _ => {
                let expr = self.expr_bp(0)?;
                // Assignment targets: a bare variable or an index expression.
                if self.peek() == &TokenKind::Eq {
                    let kind = match expr.kind {
                        ExprKind::Var(name) => {
                            self.advance();
                            StmtKind::Assign(name, self.expr_bp(0)?)
                        }
                        ExprKind::Index(base, idx) => {
                            self.advance();
                            StmtKind::IndexAssign(*base, *idx, self.expr_bp(0)?)
                        }
                        _ => return Err(self.error("invalid assignment target")),
                    };
                    let end = self.span().end;
                    self.expect(&TokenKind::Semi, "';'")?;
                    return Ok(Stmt {
                        kind,
                        span: Span::new(start, end),
                    });
                }
                let end = self.span().end;
                self.expect(&TokenKind::Semi, "';'")?;
                Ok(Stmt {
                    kind: StmtKind::Expr(expr),
                    span: Span::new(start, end),
                })
            }
        }
    }

    /// Parse `(a, b, c) { stmts }` after `fn` [name]; returns params, body,
    /// and the byte offset just past the closing brace.
    fn fn_params_and_body(&mut self) -> Result<FnParts, ParseError> {
        self.expect(&TokenKind::LParen, "'('")?;
        let mut params = Vec::new();
        if self.peek() != &TokenKind::RParen {
            loop {
                match self.peek().clone() {
                    TokenKind::Ellipsis => {
                        self.advance();
                        let TokenKind::Ident(name) = self.peek().clone() else {
                            return Err(self.error(format!(
                                "expected a name after '...', found {}",
                                describe(self.peek())
                            )));
                        };
                        if params.iter().any(|p: &crate::ast::Param| p.name == name) {
                            return Err(self.error(format!("duplicate parameter '{name}'")));
                        }
                        self.advance();
                        params.push(crate::ast::Param {
                            name,
                            default: None,
                            rest: true,
                        });
                        // Everything left over goes here, so there is
                        // nothing a later parameter could receive.
                        if self.peek() == &TokenKind::Comma {
                            return Err(
                                self.error("a rest parameter must be the last one".to_string())
                            );
                        }
                        break;
                    }
                    TokenKind::Ident(name) => {
                        if params.iter().any(|p: &crate::ast::Param| p.name == name) {
                            return Err(self.error(format!("duplicate parameter '{name}'")));
                        }
                        self.advance();
                        // `name = expr` gives the parameter a value for
                        // the calls that leave it out.
                        let default = if self.peek() == &TokenKind::Eq {
                            self.advance();
                            Some(self.expr_bp(0)?)
                        } else {
                            None
                        };
                        if default.is_none()
                            && params
                                .iter()
                                .any(|p: &crate::ast::Param| p.default.is_some())
                        {
                            return Err(self.error(format!(
                                "parameter '{name}' has no default but follows one that does"
                            )));
                        }
                        params.push(crate::ast::Param {
                            name,
                            default,
                            rest: false,
                        });
                    }
                    k => {
                        return Err(
                            self.error(format!("expected parameter name, found {}", describe(&k)))
                        );
                    }
                }
                if self.peek() == &TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen, "')'")?;
        self.expect(&TokenKind::LBrace, "'{'")?;
        let mut body = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            body.push(self.statement()?);
        }
        let end = self.span().end;
        self.expect(&TokenKind::RBrace, "'}'")?;
        Ok((params, Rc::new(body), end))
    }

    /// Parse a `{ ... }` block, with a context note for the error message.
    fn block_stmt(&mut self, context: &str) -> Result<Stmt, ParseError> {
        if self.peek() != &TokenKind::LBrace {
            return Err(self.error(format!(
                "expected '{{' {context}, found {}",
                describe(self.peek())
            )));
        }
        self.statement()
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
            TokenKind::Tilde => Some(UnaryOp::BitNot),
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
                            // `...xs` spreads a list into the call. It is
                            // an argument, not an expression: nowhere else
                            // parses one.
                            if self.peek() == &TokenKind::Ellipsis {
                                let start = self.span().start;
                                self.advance();
                                let inner = self.expr_bp(0)?;
                                let span = Span::new(start, inner.span.end);
                                args.push(Expr {
                                    kind: ExprKind::Spread(Box::new(inner)),
                                    span,
                                });
                            } else {
                                args.push(self.expr_bp(0)?);
                            }
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
            TokenKind::Fn => {
                self.advance();
                let (params, body, end) = self.fn_params_and_body()?;
                return Ok(Expr {
                    kind: ExprKind::Fn(params, body),
                    span: Span::new(span.start, end),
                });
            }
            // Map literal. Note: at statement position `{` starts a block,
            // so a map literal statement needs to sit inside an expression.
            TokenKind::LBrace => {
                self.advance();
                let mut entries = Vec::new();
                if self.peek() != &TokenKind::RBrace {
                    loop {
                        let key = self.expr_bp(0)?;
                        self.expect(&TokenKind::Colon, "':'")?;
                        let value = self.expr_bp(0)?;
                        entries.push((key, value));
                        if self.peek() == &TokenKind::Comma {
                            self.advance();
                            if self.peek() == &TokenKind::RBrace {
                                break; // trailing comma
                            }
                        } else {
                            break;
                        }
                    }
                }
                let end = self.span().end;
                self.expect(&TokenKind::RBrace, "'}'")?;
                return Ok(Expr {
                    kind: ExprKind::Map(entries),
                    span: Span::new(span.start, end),
                });
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
        // Rust's ordering, not C's: every bitwise operator binds
        // tighter than a comparison, so `a & b == c` is `(a & b) == c`.
        TokenKind::Pipe => (BinaryOp::BitOr, 9, 10),
        TokenKind::Caret => (BinaryOp::BitXor, 11, 12),
        TokenKind::Amp => (BinaryOp::BitAnd, 13, 14),
        TokenKind::Shl => (BinaryOp::Shl, 15, 16),
        TokenKind::Shr => (BinaryOp::Shr, 15, 16),
        TokenKind::Plus => (BinaryOp::Add, 17, 18),
        TokenKind::Minus => (BinaryOp::Sub, 17, 18),
        TokenKind::Star => (BinaryOp::Mul, 19, 20),
        TokenKind::Slash => (BinaryOp::Div, 19, 20),
        TokenKind::Percent => (BinaryOp::Rem, 19, 20),
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
                TokenKind::For => "for",
                TokenKind::In => "in",
                TokenKind::Break => "break",
                TokenKind::Continue => "continue",
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
                TokenKind::Colon => ":",
                TokenKind::Dot => ".",
                TokenKind::Ellipsis => "...",
                // Data-carrying kinds and Eof are handled above; keep a
                // harmless fallback so a future token can't panic the
                // error path (found by tests/fuzz.rs).
                _ => return "token".to_string(),
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
    fn bit_precedence_follows_rust_not_c() {
        // C puts `&` below `==`, which makes this mean `a & (b == c)`.
        assert_eq!(sexpr("a & b == c"), "(== (& a b) c)");
        assert_eq!(sexpr("a | b ^ c & d"), "(| a (^ b (& c d)))");
        assert_eq!(sexpr("a << b + c"), "(<< a (+ b c))");
        assert_eq!(sexpr("a && b | c"), "(&& a (| b c))");
        assert_eq!(sexpr("a >> b >> c"), "(>> (>> a b) c)");
    }

    #[test]
    fn complement_is_a_unary_operator() {
        assert_eq!(sexpr("~a & b"), "(& (~ a) b)");
        assert_eq!(sexpr("~~a"), "(~ (~ a))");
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

    fn program(src: &str) -> String {
        parse_program(&lex(src).unwrap())
            .unwrap()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn program_err(src: &str) -> String {
        parse_program(&lex(src).unwrap()).unwrap_err().message
    }

    #[test]
    fn let_assign_expr_statements() {
        assert_eq!(
            program("let x = 1; x = x + 1; print(x);"),
            "(let x 1) (= x (+ x 1)) (call print x)"
        );
    }

    #[test]
    fn nested_blocks() {
        assert_eq!(
            program("{ let a = 1; { a = 2; } }"),
            "(block (let a 1) (block (= a 2)))"
        );
    }

    #[test]
    fn missing_semicolon_is_an_error() {
        assert_eq!(program_err("let x = 1"), "expected ';', found end of input");
        assert_eq!(program_err("1 + 2"), "expected ';', found end of input");
    }

    #[test]
    fn stray_colon_is_an_error_not_a_panic() {
        // Regression: describe() missed Colon and panicked on this
        // (found by tests/fuzz.rs).
        assert_eq!(program_err("1 : 2;"), "expected ';', found ':'");
    }

    #[test]
    fn invalid_assignment_target_is_an_error() {
        assert_eq!(program_err("1 = 2;"), "invalid assignment target");
        assert_eq!(program_err("f() = 2;"), "invalid assignment target");
    }

    #[test]
    fn if_else_and_else_if_chain() {
        assert_eq!(
            program("if a { 1; } else if b { 2; } else { 3; }"),
            "(if a (block 1) (if b (block 2) (block 3)))"
        );
        assert_eq!(program("if a { 1; }"), "(if a (block 1))");
    }

    #[test]
    fn while_loop() {
        assert_eq!(
            program("while i < 3 { i = i + 1; }"),
            "(while (< i 3) (block (= i (+ i 1))))"
        );
    }

    #[test]
    fn control_flow_requires_braces() {
        assert_eq!(
            program_err("if a 1;"),
            "expected '{' after 'if' condition, found integer '1'"
        );
        assert_eq!(
            program_err("if a { 1; } else 2;"),
            "expected '{' after 'else', found integer '2'"
        );
        assert_eq!(
            program_err("while a 1;"),
            "expected '{' after 'while' condition, found integer '1'"
        );
    }

    #[test]
    fn map_literals() {
        assert_eq!(sexpr("{}"), "(map)");
        assert_eq!(
            sexpr("{\"a\": 1, \"b\": 2 + 3,}"),
            "(map (\"a\" 1) (\"b\" (+ 2 3)))"
        );
        assert_eq!(err("{1: 2"), "expected '}', found end of input");
        assert_eq!(err("{1, 2}"), "expected ':', found ','");
    }

    #[test]
    fn index_assignment_statements() {
        assert_eq!(program("xs[0] = 1;"), "(=[] xs 0 1)");
        assert_eq!(
            program("m[\"a\"][\"b\"] = 2;"),
            "(=[] (index m \"a\") \"b\" 2)"
        );
    }

    #[test]
    fn for_break_continue_statements() {
        assert_eq!(
            program("for x in xs { if x == 0 { continue; } break; }"),
            "(for x xs (block (if (== x 0) (block (continue))) (break)))"
        );
        assert_eq!(
            program_err("for 1 in xs { }"),
            "expected loop variable, found integer '1'"
        );
        assert_eq!(
            program_err("for x xs { }"),
            "expected 'in', found identifier 'xs'"
        );
        assert_eq!(
            program_err("for x in xs 1;"),
            "expected '{' after 'for' iterable, found integer '1'"
        );
        assert_eq!(program_err("break"), "expected ';', found end of input");
    }

    #[test]
    fn fn_declaration_desugars_to_let() {
        assert_eq!(
            program("fn add(a, b) { return a + b; }"),
            "(let add (fn (a b) (return (+ a b))))"
        );
        assert_eq!(program("fn noop() { }"), "(let noop (fn ()))");
    }

    #[test]
    fn anonymous_fn_is_an_expression() {
        assert_eq!(
            program("let f = fn(x) { return x; };"),
            "(let f (fn (x) (return x)))"
        );
        assert_eq!(
            sexpr("fn(x) { return x; }(1)"),
            "(call (fn (x) (return x)) 1)"
        );
    }

    #[test]
    fn return_forms() {
        assert_eq!(
            program("fn f() { return; return 1; }"),
            "(let f (fn () (return) (return 1)))"
        );
    }

    #[test]
    fn duplicate_parameter_is_an_error() {
        assert_eq!(program_err("fn f(a, a) { }"), "duplicate parameter 'a'");
    }

    #[test]
    fn parameters_may_carry_defaults() {
        assert_eq!(
            sexpr("fn(a, b = 1, c = b + 1) { return a; }"),
            "(fn (a (b 1) (c (+ b 1))) (return a))"
        );
        assert_eq!(
            program_err("fn f(a = 1, b) { return a; }"),
            "parameter 'b' has no default but follows one that does"
        );
    }

    #[test]
    fn the_last_parameter_may_take_the_rest() {
        assert_eq!(
            sexpr("fn(a, ...rest) { return rest; }"),
            "(fn (a ...rest) (return rest))"
        );
        assert_eq!(
            program_err("fn f(...rest, a) { return a; }"),
            "a rest parameter must be the last one"
        );
        assert_eq!(
            program_err("fn f(...) { return 1; }"),
            "expected a name after '...', found ')'"
        );
        assert_eq!(
            program_err("fn f(...a = 1) { return a; }"),
            "expected ')', found '='"
        );
    }

    #[test]
    fn a_spread_is_an_argument_and_nothing_else() {
        assert_eq!(sexpr("f(a, ...xs)"), "(call f a (... xs))");
        // Only an argument list parses one.
        assert_eq!(
            program_err("let x = ...xs;"),
            "expected expression, found '...'"
        );
        assert_eq!(
            program_err("print([...xs]);"),
            "expected expression, found '...'"
        );
    }

    #[test]
    fn fn_param_errors() {
        assert_eq!(
            program_err("fn f(1) { }"),
            "expected parameter name, found integer '1'"
        );
    }

    #[test]
    fn let_requires_a_name() {
        assert_eq!(
            program_err("let 1 = 2;"),
            "expected variable name, found integer '1'"
        );
    }

    #[test]
    fn spans_cover_whole_expression() {
        let src = "1 + 2 * 3";
        let expr = parse_expr(&lex(src).unwrap()).unwrap();
        assert_eq!(&src[expr.span.start..expr.span.end], src);
    }
}
