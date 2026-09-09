//! Pratt (precedence-climbing) expression parser for ting.

use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::lexer::{Span, Token, TokenKind};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

/// How deep one program may nest — braces inside braces, parentheses
/// inside parentheses, a list inside a list inside a call. The parser
/// descends one host frame per level, so past a certain depth the
/// process dies of a stack overflow with no line, no message and
/// nothing a caller can catch (843 found the cliff at about 15000
/// nested blocks and 30000 nested parentheses on a 32 MB stack: the
/// frames measure 2176 and 1088 bytes). This limit is a plain number
/// rather than one derived from whatever stack the process happens to
/// have, so `--check`, `--lsp`, the REPL, the runner and both engines
/// all refuse exactly the same programs. At 200 the parser spends
/// under half a megabyte on the worst shape, which is inside the
/// budget even a wasm build has, and it is twenty-five times the
/// deepest nesting anything in this repository reaches.
pub const MAX_NESTING: usize = 200;

/// Parse a whole program: a sequence of statements up to Eof.
pub fn parse_program(tokens: &[Token]) -> Result<Vec<Stmt>, ParseError> {
    let mut p = Parser {
        tokens,
        pos: 0,
        depth: 0,
    };
    let mut stmts = Vec::new();
    while p.peek() != &TokenKind::Eof {
        stmts.push(p.statement()?);
    }
    Ok(stmts)
}

/// Parse a complete expression; every token before Eof must be consumed.
/// Used by the REPL to echo expression results.
pub fn parse_expr(tokens: &[Token]) -> Result<Expr, ParseError> {
    let mut p = Parser {
        tokens,
        pos: 0,
        depth: 0,
    };
    let expr = p.expr_bp(0)?;
    match p.peek() {
        TokenKind::Eof => Ok(expr),
        k => Err(p.error(format!("unexpected {}", describe(k)))),
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    /// Levels of nesting open at this point, against MAX_NESTING.
    depth: usize,
}

/// (params, body, byte offset just past the closing brace)
type FnParts = (Vec<crate::ast::Param>, Rc<Vec<Stmt>>, usize);

/// A word another language uses where ting uses something else.
/// `elif`, `def` and `var` all parse as a bare name here, so the
/// message is about the missing `;` after it — true, and about the
/// wrong thing. The hint is only ever added to a message the parser
/// was already going to produce.
fn instead_of(name: &str) -> Option<&'static str> {
    Some(match name {
        "elif" | "elseif" | "elsif" => "ting writes this as `else if`",
        "def" | "function" | "func" | "fun" => "a function is `fn name(...) { ... }`",
        "var" | "const" | "local" => "a binding is `let name = ...;`",
        _ => return None,
    })
}

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

    /// The `;` at the end of a statement, with a word about the
    /// statement's FIRST token when that token is a keyword from
    /// another language — which is where the mistake actually is.
    fn expect_semi(&mut self, first: usize) -> Result<(), ParseError> {
        if self.peek() == &TokenKind::Semi {
            self.advance();
            return Ok(());
        }
        let mut message = format!("expected ';', found {}", describe(self.peek()));
        if let TokenKind::Ident(name) = &self.tokens[first].kind
            && let Some(hint) = instead_of(name)
        {
            message.push_str(&format!(" ({hint})"));
        } else if let Some(hint) = self.operator_word() {
            message.push_str(&format!(" ({hint})"));
        } else if self.peek() == &TokenKind::Eq
            && self.peek2() == &TokenKind::Gt
            && self.tokens[self.pos + 1].span.start == self.span().end
        {
            // `(x) => x + 1` — an arrow where the value should end.
            message.push_str(" (a function is `fn(x) { return x; }`)");
        }
        Err(self.error(message))
    }

    fn expect(&mut self, kind: &TokenKind, what: &str) -> Result<(), ParseError> {
        if self.peek() == kind {
            self.advance();
            return Ok(());
        }
        let mut message = format!("expected {what}, found {}", describe(self.peek()));
        if let Some(hint) = self.operator_word() {
            message.push_str(&format!(" ({hint})"));
        }
        Err(self.error(message))
    }

    /// `and`, `or` and `not` are ordinary names here, not operators,
    /// so a condition written with them stops at a word the parser
    /// cannot place. `not x` stops one token PAST the word, because
    /// `not` was read as the whole condition and `x` is what follows
    /// it — so the token behind is worth a look too. Nothing here can
    /// reach a program that parses; someone whose own variable is
    /// called `not` gets a suggestion that is merely unhelpful, on a
    /// program that was already wrong.
    fn operator_word(&self) -> Option<&'static str> {
        if let TokenKind::Ident(name) = self.peek() {
            match name.as_str() {
                "and" => return Some("ting writes this as `&&`"),
                "or" => return Some("ting writes this as `||`"),
                "not" => return Some("ting writes this as `!`"),
                _ => {}
            }
        }
        // A `.` is never part of anything the parser accepts, so
        // saying what it was probably reaching for costs nothing.
        // `s.len()` wants a call, `m.key` wants a key.
        if self.peek() == &TokenKind::Dot {
            return Some(
                match (
                    self.peek2(),
                    &self.tokens[(self.pos + 2).min(self.tokens.len() - 1)].kind,
                ) {
                    (TokenKind::Ident(_), TokenKind::LParen) => {
                        "ting has no methods — a call is `f(x)`"
                    }
                    (TokenKind::Ident(_), _) => "ting has no fields — a map key is `m[\"key\"]`",
                    _ => "ting has no `.`",
                },
            );
        }
        // `f"..."` is one token immediately after another, which
        // nothing valid ever is.
        if matches!(self.peek(), TokenKind::Str(_))
            && self.pos > 0
            && matches!(&self.tokens[self.pos - 1].kind, TokenKind::Ident(_))
            && self.tokens[self.pos - 1].span.end == self.span().start
        {
            return Some("ting has no f-strings — build text with `format(\"{} ...\", x)`");
        }
        if self.pos > 0
            && let TokenKind::Ident(name) = &self.tokens[self.pos - 1].kind
            && name == "not"
        {
            return Some("ting writes `not` as `!`");
        }
        None
    }

    /// One level deeper, or the error that says the program is nested
    /// past what the parser will follow. Both recursive descents —
    /// statements into blocks, expressions into anything bracketed —
    /// go through a wrapper like this one, so the count covers every
    /// shape that costs a host frame.
    fn nested<T>(
        &mut self,
        inner: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        self.depth += 1;
        let out = if self.depth > MAX_NESTING {
            Err(self.error(format!(
                "nested too deeply (the limit is {MAX_NESTING} levels)"
            )))
        } else {
            inner(self)
        };
        self.depth -= 1;
        out
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        self.nested(Self::statement_inner)
    }

    fn statement_inner(&mut self) -> Result<Stmt, ParseError> {
        let start = self.span().start;
        let first = self.pos;
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
                self.expect_semi(first)?;
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
                self.expect_semi(first)?;
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
                self.expect_semi(first)?;
                Ok(Stmt {
                    kind: StmtKind::Break,
                    span: Span::new(start, end),
                })
            }
            TokenKind::Continue => {
                self.advance();
                let end = self.span().end;
                self.expect_semi(first)?;
                Ok(Stmt {
                    kind: StmtKind::Continue,
                    span: Span::new(start, end),
                })
            }
            _ => {
                let expr = self.expr_bp(0)?;
                // Assignment targets: a bare variable or an index expression.
                // `=` writes; `+=` and its four siblings read, apply the
                // operator and write back.
                if let Some(op) = assign_op(self.peek()) {
                    let kind = match expr.kind {
                        ExprKind::Var(name) => {
                            self.advance();
                            StmtKind::Assign(name, op, self.expr_bp(0)?)
                        }
                        ExprKind::Index(base, idx) => {
                            self.advance();
                            StmtKind::IndexAssign(*base, *idx, op, self.expr_bp(0)?)
                        }
                        _ => return Err(self.error("invalid assignment target")),
                    };
                    let end = self.span().end;
                    self.expect_semi(first)?;
                    return Ok(Stmt {
                        kind,
                        span: Span::new(start, end),
                    });
                }
                let end = self.span().end;
                self.expect_semi(first)?;
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
            let mut message = format!("expected '{{' {context}, found {}", describe(self.peek()));
            if let Some(hint) = self.operator_word() {
                message.push_str(&format!(" ({hint})"));
            }
            return Err(self.error(message));
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
        self.nested(Self::unary_inner)
    }

    /// Every descent into a deeper expression passes here: `expr_bp`
    /// starts with it, and so does everything bracketed, since a
    /// parenthesis, a list element, a map value and an argument all
    /// re-enter through `expr_bp`. A unary chain (`!!!!x`) recurses
    /// here directly, which is why the count sits on this function
    /// rather than on `expr_bp`.
    fn unary_inner(&mut self) -> Result<Expr, ParseError> {
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
            k => {
                // `//` and `/*` are the commonest way to write a
                // comment in a language that looks like this one, and
                // `expected expression, found '/'` says nothing about
                // where the real one is. Two adjacent tokens, not
                // merely two nearby ones: `a / /b` is a different
                // mistake and gets the plain message.
                let hint = matches!(k, TokenKind::Slash)
                    && matches!(self.peek2(), TokenKind::Slash | TokenKind::Star)
                    && self.tokens[self.pos + 1].span.start == self.span().end;
                let mut message = format!("expected expression, found {}", describe(&k));
                if hint {
                    message.push_str(" (a comment starts with `#`)");
                } else if let Some(hint) = self.operator_word() {
                    message.push_str(&format!(" ({hint})"));
                }
                return Err(self.error(message));
            }
        };
        self.advance();
        Ok(Expr { kind, span })
    }
}

/// (operator, left binding power, right binding power); left-assoc: rbp = lbp + 1.
/// The token that opens an assignment, and the operator it folds in.
/// `Some(None)` is a plain `=`; anything else is not an assignment.
fn assign_op(kind: &TokenKind) -> Option<Option<BinaryOp>> {
    match kind {
        TokenKind::Eq => Some(None),
        TokenKind::PlusEq => Some(Some(BinaryOp::Add)),
        TokenKind::MinusEq => Some(Some(BinaryOp::Sub)),
        TokenKind::StarEq => Some(Some(BinaryOp::Mul)),
        TokenKind::SlashEq => Some(Some(BinaryOp::Div)),
        TokenKind::PercentEq => Some(Some(BinaryOp::Rem)),
        _ => None,
    }
}

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
                TokenKind::PlusEq => "+=",
                TokenKind::MinusEq => "-=",
                TokenKind::StarEq => "*=",
                TokenKind::SlashEq => "/=",
                TokenKind::PercentEq => "%=",
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

    /// `#` is the comment character, and a language that looks like
    /// this one draws `//` out of the fingers. The hint costs nothing
    /// on a program that parses and saves a search of the reference
    /// on one that does not.
    #[test]
    fn a_c_style_comment_says_where_the_real_one_is() {
        let want = "expected expression, found '/' (a comment starts with `#`)";
        for src in ["// note", "/* note */"] {
            assert_eq!(err(src), want, "{src}");
        }
        // And in the places a comment is actually written: on its own
        // line, after a statement, and inside a block.
        for src in [
            "// note\nprint(1);",
            "print(1); // note",
            "fn f() {\n  // note\n  return 1;\n}",
            "print(1); /* note */",
        ] {
            assert_eq!(prog_err(src), want, "{src}");
        }
    }

    /// Only two ADJACENT slashes are a comment someone meant to
    /// write. `/ / x` has the same two tokens with a space between
    /// them and is a different mistake, so it keeps the plain
    /// message — as does a division that simply lost its operand.
    #[test]
    fn a_divide_that_lost_its_operand_gets_no_comment_hint() {
        for src in ["/ / x", "/ * x", "a / / b", "/ x", "1 + / 2"] {
            assert_eq!(err(src), "expected expression, found '/'", "{src}");
        }
    }

    /// `elif`, `def` and `var` parse as a bare name, so the parser
    /// complains about the `;` that should follow it. True, and about
    /// the wrong thing: the mistake is the word.
    #[test]
    fn a_keyword_from_another_language_says_what_ting_writes() {
        for (src, want) in [
            ("if a { } elif b { }", "ting writes this as `else if`"),
            ("if a { } elseif b { }", "ting writes this as `else if`"),
            ("if a { } elsif b { }", "ting writes this as `else if`"),
            (
                "def f(x): return x;",
                "a function is `fn name(...) { ... }`",
            ),
            ("function f(x) { }", "a function is `fn name(...) { ... }`"),
            ("var x = 1;", "a binding is `let name = ...;`"),
            ("const x = 1;", "a binding is `let name = ...;`"),
            ("local x = 1;", "a binding is `let name = ...;`"),
            ("let f = (x) => x;", "a function is `fn(x) { return x; }`"),
        ] {
            let got = prog_err(src);
            assert!(got.starts_with("expected ';', found "), "{src}: {got}");
            assert!(got.ends_with(&format!("({want})")), "{src}: {got}");
        }
    }

    /// None of those words is reserved, so a program that uses one as
    /// a name still parses — and a statement that simply lost its
    /// semicolon gets the plain message, since the hint is about the
    /// FIRST token of the statement, not the one the parser stopped
    /// at. `= >` with a space is not an arrow.
    #[test]
    fn the_keyword_hint_stays_out_of_the_way() {
        for src in ["let x = 1\nprint(x);", "let f = (x) = > x;"] {
            let got = prog_err(src);
            assert!(
                got.starts_with("expected ';', found ") && !got.contains('('),
                "{src}: {got}"
            );
        }
        for src in [
            "let var = 1; print(var);",
            "let function = fn(x) { return x; }; print(function(2));",
            "let def = 3; print(def + 1);",
            "if a { } else if b { }",
            // A bare name is a whole statement, so these parse and
            // fail later, at run time, for the right reason.
            "elif;",
            "var;",
        ] {
            assert!(
                parse_program(&lex(src).unwrap()).is_ok(),
                "{src} should still parse"
            );
        }
    }

    /// `and`, `or` and `not` are names here, so a condition written
    /// with them stops at a word the parser cannot place. `not` is
    /// the awkward one: it is read as the whole condition, so the
    /// error lands one token PAST it.
    #[test]
    fn an_operator_word_says_what_ting_writes() {
        for (src, want) in [
            ("if a and b { }", "ting writes this as `&&`"),
            ("if a or b { }", "ting writes this as `||`"),
            ("while a and b { }", "ting writes this as `&&`"),
            ("print(a and b);", "ting writes this as `&&`"),
            ("a and b;", "ting writes this as `&&`"),
            ("if not a { }", "ting writes `not` as `!`"),
            ("if not true { }", "ting writes `not` as `!`"),
        ] {
            let got = prog_err(src);
            assert!(got.ends_with(&format!("({want})")), "{src}: {got}");
        }
    }

    /// None of the three is reserved, so a program that binds one and
    /// uses it still parses — including `if not { }`, where `not` is
    /// the whole condition and the block follows it properly.
    #[test]
    fn the_operator_hint_stays_out_of_the_way() {
        for src in [
            "let and = 1; print(and);",
            "let not = 5; print(not);",
            "let not = true; if not { print(1); }",
            "print(true && false);",
            "print(!true);",
            "if a { } else { }",
        ] {
            assert!(
                parse_program(&lex(src).unwrap()).is_ok(),
                "{src} should still parse"
            );
        }
    }

    /// A `.` is never part of anything the parser accepts, so the
    /// hint costs nothing and can say which of the two shapes was
    /// probably meant. An `f"..."` is two tokens with nothing
    /// between them, which nothing valid ever is.
    #[test]
    fn borrowed_access_syntax_says_what_ting_writes() {
        for (src, want) in [
            ("print(s.len());", "ting has no methods — a call is `f(x)`"),
            ("let n = s.len();", "ting has no methods — a call is `f(x)`"),
            (
                "print(m.a);",
                "ting has no fields — a map key is `m[\"key\"]`",
            ),
            ("print(.);", "ting has no `.`"),
            (
                "print(f\"n is\");",
                "ting has no f-strings — build text with `format(\"{} ...\", x)`",
            ),
        ] {
            let got = prog_err(src);
            assert!(got.ends_with(&format!("({want})")), "{src}: {got}");
        }
    }

    /// A `.` inside a number is part of the number, and an
    /// identifier with a SPACE before a string is an ordinary call
    /// that happens to fail elsewhere.
    #[test]
    fn the_access_hint_stays_out_of_the_way() {
        // `f "x"` is the same two tokens with a space between them:
        // a missing operator or comma, not an f-string.
        let spaced = prog_err("print(f \"n is\");");
        assert!(
            spaced.ends_with("found string literal"),
            "a space is not an f-string: {spaced}"
        );
        for src in [
            "print(1.5 + 2.25);",
            "print(0.5);",
            "let m = {\"a\": 1}; print(m[\"a\"]);",
            "let f = fn(x) { return x; }; print(f(\"hi\"));",
        ] {
            assert!(
                parse_program(&lex(src).unwrap()).is_ok(),
                "{src} should still parse"
            );
        }
    }

    fn prog_err(src: &str) -> String {
        parse_program(&lex(src).unwrap()).unwrap_err().message
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

    /// A deep parse costs host stack — 2.2 KB per nested block in
    /// release and about eight times that unoptimized — so these run
    /// on a thread whose stack is declared, rather than on whatever
    /// the test harness happens to hand out.
    fn parsing<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(f)
            .expect("spawn")
            .join()
            .expect("the parse panicked")
    }

    /// The same nesting written four ways: blocks, parentheses, list
    /// literals and a unary chain. Each recurses by its own route.
    fn deep_shapes(n: usize) -> Vec<String> {
        vec![
            format!("{}let x = 1;{}", "{".repeat(n), "}".repeat(n)),
            format!("print({}1{});", "(".repeat(n), ")".repeat(n)),
            format!("let x = {}1{};", "[".repeat(n), "]".repeat(n)),
            format!("let x = {}1;", "!".repeat(n)),
        ]
    }

    #[test]
    fn nesting_within_the_limit_parses() {
        parsing(|| {
            for src in deep_shapes(150) {
                assert!(
                    parse_program(&lex(&src).unwrap()).is_ok(),
                    "150 levels should parse: {}",
                    &src[..20]
                );
            }
        });
    }

    #[test]
    fn nesting_past_the_limit_is_an_error_and_not_a_dead_process() {
        parsing(|| {
            // Blocks, brackets and a unary chain each recurse by a
            // different route; every one of them is counted, and 843
            // measured the cliff each of them runs into.
            let mut deep = deep_shapes(400);
            // Far past the cliff, where the process used to die with
            // no line and nothing to catch.
            deep.push(format!(
                "{}let x = 1;{}",
                "{".repeat(60000),
                "}".repeat(60000)
            ));
            for src in deep {
                let e = parse_program(&lex(&src).unwrap()).unwrap_err();
                assert_eq!(
                    e.message,
                    format!("nested too deeply (the limit is {MAX_NESTING} levels)")
                );
                assert!(e.span.start > 0, "the error points at a token");
            }
        });
    }

    #[test]
    fn a_long_flat_expression_is_not_deep() {
        // The limit is on nesting, not on length: a sum of 50000
        // terms is one level, and a chain of calls and indexes is
        // read by a loop rather than by recursion.
        parsing(|| {
            let src = format!("let x = {};", vec!["1"; 50000].join(" + "));
            assert!(parse_program(&lex(&src).unwrap()).is_ok());
            let src = format!("let x = a{};", "(0)[1]".repeat(5000));
            assert!(parse_program(&lex(&src).unwrap()).is_ok());
        });
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
    fn a_compound_assignment_carries_its_operator() {
        assert_eq!(program("x += 1;"), "(+= x 1)");
        assert_eq!(program("x %= 2;"), "(%= x 2)");
        assert_eq!(program("m[k] -= 1;"), "(-=[] m k 1)");
        // The right-hand side is a whole expression, not one operand.
        assert_eq!(program("x *= a + b;"), "(*= x (+ a b))");
        // The target rules are the ones plain assignment has.
        assert_eq!(program_err("f() += 1;"), "invalid assignment target");
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
