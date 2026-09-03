//! A minimal Language Server Protocol server (`ting --lsp`): stdio
//! JSON-RPC with Content-Length framing, full-text document sync, and
//! push diagnostics from the lexer/parser/compiler. Reuses the json
//! module — the LSP payloads are ordinary ting Values.
//!
//! Character offsets are Unicode-scalar based (the server does not
//! implement UTF-16 position encoding; for error underlines this is a
//! benign approximation outside the astral planes).

use crate::value::{Builtin, Value};
use crate::{compile, json, lexer, parser};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};

fn obj(entries: Vec<(&str, Value)>) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in entries {
        m.insert(k.to_string(), v);
    }
    Value::map(m)
}

fn s(text: &str) -> Value {
    Value::Str(text.to_string())
}

/// Read one framed JSON-RPC message; None on clean EOF.
/// One framed message: None at end of input (or an unreadable frame),
/// Some(None) for a frame whose body is not JSON — skipped by the
/// caller, so one malformed message cannot end the session.
fn read_message(input: &mut impl BufRead) -> Option<Option<Value>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            content_length = v.trim().parse().ok();
        }
    }
    let len = content_length?;
    let mut buf = vec![0u8; len];
    input.read_exact(&mut buf).ok()?;
    Some(
        std::str::from_utf8(&buf)
            .ok()
            .and_then(|text| json::decode(text).ok()),
    )
}

/// The filesystem path of a file: URI, tolerating both `file:///tmp/x`
/// and Windows' `file:///C:/x` (whose leading slash is not part of
/// the path).
fn uri_to_path(uri: &str) -> Option<std::path::PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let bytes = rest.as_bytes();
    let rest = if bytes.len() > 2 && bytes[0] == b'/' && bytes[2] == b':' {
        &rest[1..]
    } else {
        rest
    };
    Some(std::path::PathBuf::from(rest))
}

/// The file: URI of a path, with forward slashes and the leading slash
/// Windows drive letters need.
fn path_to_uri(path: &std::path::Path) -> String {
    let text = path.display().to_string().replace('\\', "/");
    if text.starts_with('/') {
        format!("file://{text}")
    } else {
        format!("file:///{text}")
    }
}

fn write_message(output: &mut impl Write, msg: &Value) {
    let body = json::encode(msg).expect("LSP message encodes");
    let _ = write!(output, "Content-Length: {}\r\n\r\n{body}", body.len());
    let _ = output.flush();
}

fn get(v: &Value, key: &str) -> Option<Value> {
    match v {
        Value::Map(m) => m.borrow().get(key).cloned(),
        _ => None,
    }
}

fn get_str(v: &Value, key: &str) -> Option<String> {
    match get(v, key)? {
        Value::Str(s) => Some(s),
        _ => None,
    }
}

/// 0-based LSP position for a byte offset.
fn position(src: &str, offset: usize) -> Value {
    let (line, col) = lexer::Span::new(offset, offset).line_col(src);
    obj(vec![
        ("line", Value::Int(line as i64 - 1)),
        ("character", Value::Int(col as i64 - 1)),
    ])
}

/// Top-level `let` bindings as a flat DocumentSymbol list: functions
/// get SymbolKind Function (12), everything else Variable (13).
fn document_symbols(src: &str) -> Value {
    let Ok(tokens) = lexer::lex(src) else {
        return Value::list(vec![]);
    };
    let Ok(program) = crate::parser::parse_program(&tokens) else {
        return Value::list(vec![]);
    };
    let mut symbols = Vec::new();
    for stmt in &program {
        if let crate::ast::StmtKind::Let(name, expr) = &stmt.kind {
            let kind = match expr.kind {
                crate::ast::ExprKind::Fn(..) => 12,
                _ => 13,
            };
            let range = obj(vec![
                ("start", position(src, stmt.span.start)),
                ("end", position(src, stmt.span.end)),
            ]);
            symbols.push(obj(vec![
                ("name", s(name)),
                ("kind", Value::Int(kind)),
                ("range", range.clone()),
                ("selectionRange", range),
            ]));
        }
    }
    Value::list(symbols)
}

/// workspace/symbol: every top-level binding of every open document
/// whose name contains the query (case-insensitive; empty matches
/// all), as SymbolInformation with a Location, documents in uri order.
fn workspace_symbols(docs: &BTreeMap<String, String>, query: &str) -> Value {
    let q = query.to_lowercase();
    let mut out = Vec::new();
    for (uri, src) in docs {
        let Ok(tokens) = lexer::lex(src) else {
            continue;
        };
        let Ok(program) = crate::parser::parse_program(&tokens) else {
            continue;
        };
        for stmt in &program {
            if let crate::ast::StmtKind::Let(name, expr) = &stmt.kind
                && name.to_lowercase().contains(&q)
            {
                let kind = match expr.kind {
                    crate::ast::ExprKind::Fn(..) => 12,
                    _ => 13,
                };
                out.push(obj(vec![
                    ("name", s(name)),
                    ("kind", Value::Int(kind)),
                    (
                        "location",
                        obj(vec![
                            ("uri", s(uri)),
                            (
                                "range",
                                obj(vec![
                                    ("start", position(src, stmt.span.start)),
                                    ("end", position(src, stmt.span.end)),
                                ]),
                            ),
                        ]),
                    ),
                ]));
            }
        }
    }
    Value::list(out)
}

/// Definition of the identifier at (line, character): the top-level
/// `let` (or fn sugar) binding that name, as a Location in `uri`.
fn definition_result(src: &str, uri: &str, line: usize, character: usize) -> Value {
    let Some(name) = ident_at(src, line, character) else {
        return Value::Nil;
    };
    let Ok(tokens) = lexer::lex(src) else {
        return Value::Nil;
    };
    let Ok(program) = crate::parser::parse_program(&tokens) else {
        return Value::Nil;
    };
    for stmt in &program {
        if let crate::ast::StmtKind::Let(n, _) = &stmt.kind
            && n == &name
        {
            return obj(vec![
                ("uri", s(uri)),
                (
                    "range",
                    obj(vec![
                        ("start", position(src, stmt.span.start)),
                        ("end", position(src, stmt.span.end)),
                    ]),
                ),
            ]);
        }
    }
    Value::Nil
}

/// Every occurrence of the identifier at (line, character), as
/// Locations in `uri` — token-level, so shadowing is not resolved.
fn references_result(src: &str, uri: &str, line: usize, character: usize) -> Value {
    let Some(name) = ident_at(src, line, character) else {
        return Value::Nil;
    };
    let Ok(tokens) = lexer::lex(src) else {
        return Value::Nil;
    };
    let mut locations = Vec::new();
    for tok in &tokens {
        if let lexer::TokenKind::Ident(n) = &tok.kind
            && n == &name
        {
            locations.push(obj(vec![
                ("uri", s(uri)),
                (
                    "range",
                    obj(vec![
                        ("start", position(src, tok.span.start)),
                        ("end", position(src, tok.span.end)),
                    ]),
                ),
            ]));
        }
    }
    Value::list(locations)
}

/// Document highlights: every occurrence of the identifier under the
/// cursor in this document, a binding site (`let name`, `fn name`) as
/// Write (3) and any other as Read (2) — what an editor lights up on
/// every cursor move. Same token-level scan as references.
fn highlight_result(src: &str, line: usize, character: usize) -> Value {
    let Some(name) = ident_at(src, line, character) else {
        return Value::Nil;
    };
    let Ok(tokens) = lexer::lex(src) else {
        return Value::Nil;
    };
    let mut out = Vec::new();
    for (i, tok) in tokens.iter().enumerate() {
        let lexer::TokenKind::Ident(n) = &tok.kind else {
            continue;
        };
        if n != &name {
            continue;
        }
        let binding = i > 0
            && matches!(
                tokens[i - 1].kind,
                lexer::TokenKind::Let | lexer::TokenKind::Fn
            );
        out.push(obj(vec![
            (
                "range",
                obj(vec![
                    ("start", position(src, tok.span.start)),
                    ("end", position(src, tok.span.end)),
                ]),
            ),
            ("kind", Value::Int(if binding { 3 } else { 2 })),
        ]));
    }
    if out.is_empty() {
        // The cursor was on a number or something else that is not a
        // name in this document.
        return Value::Nil;
    }
    Value::list(out)
}

/// Prepare a rename: the range of the identifier under the cursor and
/// its text as the placeholder, or null when there is no identifier
/// there or it is a keyword or a builtin — so the editor declines
/// before opening the prompt instead of after.
fn prepare_rename_result(src: &str, line: usize, character: usize) -> Value {
    let Some(text) = src.lines().nth(line) else {
        return Value::Nil;
    };
    let chars: Vec<char> = text.chars().collect();
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut start = character.min(chars.len());
    while start > 0 && is_ident(chars[start - 1]) {
        start -= 1;
    }
    let mut end = start;
    while end < chars.len() && is_ident(chars[end]) {
        end += 1;
    }
    if start == end || chars[start].is_ascii_digit() {
        return Value::Nil;
    }
    let name: String = chars[start..end].iter().collect();
    if KEYWORDS.contains(&name.as_str()) || Builtin::ALL.iter().any(|b| b.name() == name) {
        return Value::Nil;
    }
    obj(vec![
        (
            "range",
            obj(vec![
                (
                    "start",
                    obj(vec![
                        ("line", Value::Int(line as i64)),
                        ("character", Value::Int(start as i64)),
                    ]),
                ),
                (
                    "end",
                    obj(vec![
                        ("line", Value::Int(line as i64)),
                        ("character", Value::Int(end as i64)),
                    ]),
                ),
            ]),
        ),
        ("placeholder", s(&name)),
    ])
}

/// Rename every occurrence of the identifier at (line, character):
/// a WorkspaceEdit with one TextEdit per token, same scan as
/// references (token-level, shadowing not resolved).
/// Rename the identifier at (line, character) in `uri` across every
/// open document: the WorkspaceEdit carries one change list per
/// document in which the name occurs (documents in uri order).
fn rename_result(
    docs: &BTreeMap<String, String>,
    uri: &str,
    line: usize,
    character: usize,
    new_name: &str,
) -> Value {
    let valid = !new_name.is_empty()
        && !new_name.chars().next().unwrap().is_ascii_digit()
        && new_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid {
        return Value::Nil;
    }
    let Some(src) = docs.get(uri) else {
        return Value::Nil;
    };
    let Some(name) = ident_at(src, line, character) else {
        return Value::Nil;
    };
    let mut changes = Vec::new();
    for (doc_uri, doc_src) in docs {
        let Ok(tokens) = lexer::lex(doc_src) else {
            continue;
        };
        let edits: Vec<Value> = tokens
            .iter()
            .filter(|tok| matches!(&tok.kind, lexer::TokenKind::Ident(n) if n == &name))
            .map(|tok| {
                obj(vec![
                    (
                        "range",
                        obj(vec![
                            ("start", position(doc_src, tok.span.start)),
                            ("end", position(doc_src, tok.span.end)),
                        ]),
                    ),
                    ("newText", s(new_name)),
                ])
            })
            .collect();
        if !edits.is_empty() {
            changes.push((doc_uri.as_str(), Value::list(edits)));
        }
    }
    if changes.is_empty() {
        return Value::Nil;
    }
    obj(vec![("changes", obj(changes))])
}

/// Document links: every `import("path")` whose path resolves, relative
/// to the document's directory, to a file that exists becomes a link
/// to that file. Embedded stdlib modules with no file on disk get no
/// link (there is nothing to open); `..` and `.` segments normalise
/// lexically.
fn document_links(src: &str, uri: &str) -> Value {
    let Ok(tokens) = lexer::lex(src) else {
        return Value::list(vec![]);
    };
    let Some(dir) = uri_to_path(uri).and_then(|p| p.parent().map(|d| d.to_path_buf())) else {
        return Value::list(vec![]);
    };
    let mut links = Vec::new();
    for (span, target) in import_targets(&tokens, &dir) {
        links.push(obj(vec![
            (
                "range",
                obj(vec![
                    ("start", position(src, span.start)),
                    ("end", position(src, span.end)),
                ]),
            ),
            ("target", s(&path_to_uri(&target))),
        ]));
    }
    Value::list(links)
}

/// Every `import("path")` in a token stream whose path, resolved
/// against `dir` with `.` and `..` normalised lexically, is a file on
/// disk: the string's span and the file. Embedded stdlib modules with
/// no file are skipped.
pub fn import_targets(
    tokens: &[lexer::Token],
    dir: &std::path::Path,
) -> Vec<(lexer::Span, std::path::PathBuf)> {
    let mut out = Vec::new();
    for w in tokens.windows(3) {
        let (lexer::TokenKind::Ident(name), lexer::TokenKind::LParen, lexer::TokenKind::Str(path)) =
            (&w[0].kind, &w[1].kind, &w[2].kind)
        else {
            continue;
        };
        if name != "import" {
            continue;
        }
        let mut target = std::path::PathBuf::new();
        for part in dir.join(path).components() {
            match part {
                std::path::Component::ParentDir => {
                    target.pop();
                }
                std::path::Component::CurDir => {}
                other => target.push(other),
            }
        }
        if target.is_file() {
            out.push((w[2].span, target));
        }
    }
    out
}

/// Folding ranges: every `{ ... }` pair that spans more than one line,
/// from the token stream (blocks and map literals alike), outermost
/// first in source order.
fn folding_ranges(src: &str) -> Value {
    let Ok(tokens) = lexer::lex(src) else {
        return Value::list(vec![]);
    };
    let line_of = |offset: usize| lexer::Span::new(offset, offset).line_col(src).0 as i64 - 1;
    let mut open: Vec<usize> = Vec::new();
    let mut ranges: Vec<(i64, i64)> = Vec::new();
    for tok in &tokens {
        match tok.kind {
            lexer::TokenKind::LBrace => open.push(tok.span.start),
            lexer::TokenKind::RBrace => {
                if let Some(start) = open.pop() {
                    let (a, b) = (line_of(start), line_of(tok.span.start));
                    if b > a {
                        ranges.push((a, b));
                    }
                }
            }
            _ => {}
        }
    }
    ranges.sort();
    Value::list(
        ranges
            .into_iter()
            .map(|(a, b)| {
                obj(vec![
                    ("startLine", Value::Int(a)),
                    ("endLine", Value::Int(b)),
                    ("kind", s("region")),
                ])
            })
            .collect(),
    )
}

/// Lex + parse + compile; the first error becomes the diagnostic list.
/// Errors in local files this document imports: one error diagnostic
/// on each `import("...")` string whose file fails to lex, parse or
/// compile, carrying the module's file name, position and message —
/// so a broken import shows in the importer without opening it.
fn import_diagnostics(src: &str, uri: &str) -> Vec<Value> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    let Some(dir) = uri_to_path(uri).and_then(|p| p.parent().map(|d| d.to_path_buf())) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (span, target) in import_targets(&tokens, &dir) {
        let Ok(text) = std::fs::read_to_string(&target) else {
            continue;
        };
        let err = match lexer::lex(&text) {
            Err(e) => Some((e.message, e.span)),
            Ok(t) => match parser::parse_program(&t) {
                Err(e) => Some((e.message, e.span)),
                Ok(program) => match compile::compile_program(&program) {
                    Err(e) => Some((e.message, e.span)),
                    Ok(_) => None,
                },
            },
        };
        let Some((message, espan)) = err else {
            continue;
        };
        let (line, col) = espan.line_col(&text);
        let name = target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push(obj(vec![
            (
                "range",
                obj(vec![
                    ("start", position(src, span.start)),
                    ("end", position(src, span.end)),
                ]),
            ),
            ("severity", Value::Int(1)),
            ("source", s("ting")),
            ("message", s(&format!("{name}:{line}:{col}: {message}"))),
        ]));
    }
    out
}

fn diagnostics(src: &str, uri: &str) -> Value {
    let err = match lexer::lex(src) {
        Err(e) => Some((e.message, e.span)),
        Ok(tokens) => match parser::parse_program(&tokens) {
            Err(e) => Some((e.message, e.span)),
            Ok(program) => match compile::compile_program(&program) {
                Err(e) => Some((e.message, e.span)),
                Ok(_) => None,
            },
        },
    };
    let mut list = match err {
        None => Vec::new(),
        Some((message, span)) => vec![obj(vec![
            (
                "range",
                obj(vec![
                    ("start", position(src, span.start)),
                    ("end", position(src, span.end.max(span.start))),
                ]),
            ),
            ("severity", Value::Int(1)),
            ("source", s("ting")),
            ("message", s(&message)),
        ])],
    };
    list.extend(import_diagnostics(src, uri));
    for (start, end, message) in warnings(src) {
        list.push(obj(vec![
            (
                "range",
                obj(vec![
                    ("start", position(src, start)),
                    ("end", position(src, end)),
                ]),
            ),
            ("severity", Value::Int(2)), // Warning
            ("source", s("ting")),
            ("message", s(&message)),
        ]));
    }
    Value::list(list)
}

/// Warnings for `m["name"]` where `m` is bound by `let m = import(...)`
/// to an embedded stdlib module that exports no `name` (functions and
/// top-level lets both count). Text-based like the rest of this file:
/// (byte start, byte end, message) per offending key.
/// Warnings for top-level `let` (or fn) bindings whose name is never
/// referenced anywhere else in the file — the binding's own identifier
/// token is the only occurrence. Names starting with `_` are exempt by
/// convention, and a file consisting only of bindings is a module
/// whose names are exports. (byte start, byte end, message) per
/// binding.
/// Names read but bound nowhere the checker can see: not a parameter,
/// not a `let` in an enclosing block, not a builtin. The walk mirrors
/// the interpreter's scoping, with one deliberate slackening — every
/// `let` of a block is in scope for the whole block, since a function
/// defined late may be called from one defined early — so a name it
/// reports is one no run could resolve.
pub fn unbound_names(src: &str) -> Vec<(usize, usize, String)> {
    unbound_findings(src)
        .into_iter()
        .map(|f| {
            let message = match f.near {
                Some(near) => format!("`{}` is bound nowhere (did you mean `{near}`?)", f.name),
                None => format!("`{}` is bound nowhere", f.name),
            };
            (f.start, f.end, message)
        })
        .collect()
}

/// Statements that can never run: whatever follows a `return`, a
/// `break` or a `continue` in the same block. Only the first orphan
/// is reported — the rest of the block is the same mistake.
pub fn unreachable_code(src: &str) -> Vec<(usize, usize, String)> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    let Ok(program) = crate::parser::parse_program(&tokens) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    visit_blocks(&program, &mut |stmts| {
        use crate::ast::StmtKind as S;
        for (i, stmt) in stmts.iter().enumerate() {
            let word = match &stmt.kind {
                S::Return(_) => "return",
                S::Break => "break",
                S::Continue => "continue",
                _ => continue,
            };
            let Some(next) = stmts.get(i + 1) else {
                break;
            };
            out.push((
                next.span.start,
                next.span.end,
                format!("this can never run: the {word} above always leaves"),
            ));
            break;
        }
    });
    out.sort_by_key(|(start, _, _)| *start);
    out
}

/// Every block of statements in the program: the top level, function
/// bodies, and the bodies of `if`, `while`, `for` and bare blocks.
fn visit_blocks(stmts: &[crate::ast::Stmt], f: &mut impl FnMut(&[crate::ast::Stmt])) {
    use crate::ast::{ExprKind as E, StmtKind as S};
    f(stmts);
    for stmt in stmts {
        match &stmt.kind {
            S::Block(inner) => visit_blocks(inner, f),
            S::If(_, then, els) => {
                visit_blocks(std::slice::from_ref(then), f);
                if let Some(e) = els {
                    visit_blocks(std::slice::from_ref(e), f);
                }
            }
            S::While(_, body) | S::For(_, _, body) => visit_blocks(std::slice::from_ref(body), f),
            _ => {}
        }
    }
    // Function bodies are blocks too, wherever the literal sits.
    visit_exprs(stmts, &mut |e| {
        if let E::Fn(_, body) = &e.kind {
            visit_blocks(body, f);
        }
    });
}

/// Map literals that give the same string key twice: the last wins
/// silently, so `{"a": 1, "a": 2}` is `{"a": 2}` and the first entry
/// was written for nothing. Only literal string keys are judged; a
/// computed key is decided at run time.
pub fn duplicate_map_keys(src: &str) -> Vec<(usize, usize, String)> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    let Ok(program) = crate::parser::parse_program(&tokens) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    visit_exprs(&program, &mut |e| {
        let crate::ast::ExprKind::Map(entries) = &e.kind else {
            return;
        };
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for (k, _) in entries {
            if let crate::ast::ExprKind::Str(key) = &k.kind
                && !seen.insert(key.as_str())
            {
                out.push((
                    k.span.start,
                    k.span.end,
                    format!("duplicate key `{key}`: the last one wins"),
                ));
            }
        }
    });
    out.sort_by_key(|(start, _, _)| *start);
    out
}

/// Every expression in the program, outermost first, for the passes
/// that judge one node at a time.
fn visit_exprs(stmts: &[crate::ast::Stmt], f: &mut impl FnMut(&crate::ast::Expr)) {
    use crate::ast::{ExprKind as E, StmtKind as S};
    fn expr(e: &crate::ast::Expr, f: &mut impl FnMut(&crate::ast::Expr)) {
        f(e);
        match &e.kind {
            E::List(items) => items.iter().for_each(|i| expr(i, f)),
            E::Map(entries) => entries.iter().for_each(|(k, v)| {
                expr(k, f);
                expr(v, f);
            }),
            E::Unary(_, a) => expr(a, f),
            E::Binary(_, a, b) => {
                expr(a, f);
                expr(b, f);
            }
            E::Call(callee, args) => {
                expr(callee, f);
                args.iter().for_each(|a| expr(a, f));
            }
            E::Index(base, idx) => {
                expr(base, f);
                expr(idx, f);
            }
            E::Fn(_, body) => visit_exprs(body, f),
            _ => {}
        }
    }
    for stmt in stmts {
        match &stmt.kind {
            S::Let(_, e) | S::Assign(_, e) | S::Expr(e) | S::Return(Some(e)) => expr(e, f),
            S::IndexAssign(base, idx, value) => {
                expr(base, f);
                expr(idx, f);
                expr(value, f);
            }
            S::Block(inner) => visit_exprs(inner, f),
            S::If(cond, then, els) => {
                expr(cond, f);
                visit_exprs(std::slice::from_ref(then), f);
                if let Some(e) = els {
                    visit_exprs(std::slice::from_ref(e), f);
                }
            }
            S::While(cond, body) => {
                expr(cond, f);
                visit_exprs(std::slice::from_ref(body), f);
            }
            S::For(_, iterable, body) => {
                expr(iterable, f);
                visit_exprs(std::slice::from_ref(body), f);
            }
            S::Break | S::Continue | S::Return(None) => {}
        }
    }
}

/// Calls whose argument count cannot match the function called, for
/// the plainest case there is: a function bound once at the top level,
/// never reassigned, never shadowed by a parameter or an inner `let`
/// anywhere in the file. Anything less certain is left to the run.
pub fn arity_mismatches(src: &str) -> Vec<(usize, usize, String)> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    let Ok(program) = crate::parser::parse_program(&tokens) else {
        return Vec::new();
    };
    use crate::ast::{ExprKind as E, StmtKind as S};
    // Top-level `let name = fn(...)`, with the arity it was given.
    let mut arities: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for stmt in &program {
        if let S::Let(name, value) = &stmt.kind {
            *seen.entry(name.clone()).or_insert(0) += 1;
            if let E::Fn(params, _) = &value.kind {
                arities.insert(name.clone(), params.len());
            }
        }
    }
    // Any name bound twice, rebound, shadowed or used as a parameter
    // is beyond this pass: drop it rather than guess.
    let mut unsure: std::collections::HashSet<String> = seen
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|(name, _)| name.clone())
        .collect();
    collect_rebindings(&program, true, &mut unsure);
    arities.retain(|name, _| !unsure.contains(name));

    let mut out = Vec::new();
    check_calls(&program, &arities, &mut out);
    out.sort_by_key(|(start, _, _)| *start);
    out
}

/// Names that a second binding, an assignment, a parameter list or an
/// inner `let` puts beyond the top-level view.
fn collect_rebindings(
    stmts: &[crate::ast::Stmt],
    top: bool,
    out: &mut std::collections::HashSet<String>,
) {
    use crate::ast::{ExprKind as E, StmtKind as S};
    fn expr(e: &crate::ast::Expr, out: &mut std::collections::HashSet<String>) {
        match &e.kind {
            E::Fn(params, body) => {
                out.extend(params.iter().cloned());
                collect_rebindings(body, false, out);
            }
            E::List(items) => items.iter().for_each(|i| expr(i, out)),
            E::Map(entries) => entries.iter().for_each(|(k, v)| {
                expr(k, out);
                expr(v, out);
            }),
            E::Unary(_, a) => expr(a, out),
            E::Binary(_, a, b) => {
                expr(a, out);
                expr(b, out);
            }
            E::Call(callee, args) => {
                expr(callee, out);
                args.iter().for_each(|a| expr(a, out));
            }
            E::Index(base, idx) => {
                expr(base, out);
                expr(idx, out);
            }
            _ => {}
        }
    }
    for stmt in stmts {
        match &stmt.kind {
            S::Let(name, value) => {
                if !top {
                    out.insert(name.clone());
                }
                expr(value, out);
            }
            S::Assign(name, value) => {
                out.insert(name.clone());
                expr(value, out);
            }
            S::IndexAssign(base, idx, value) => {
                expr(base, out);
                expr(idx, out);
                expr(value, out);
            }
            S::Expr(e) => expr(e, out),
            S::Block(inner) => collect_rebindings(inner, false, out),
            S::If(cond, then, els) => {
                expr(cond, out);
                collect_rebindings(std::slice::from_ref(then), false, out);
                if let Some(e) = els {
                    collect_rebindings(std::slice::from_ref(e), false, out);
                }
            }
            S::While(cond, body) => {
                expr(cond, out);
                collect_rebindings(std::slice::from_ref(body), false, out);
            }
            S::For(var, iterable, body) => {
                out.insert(var.clone());
                expr(iterable, out);
                collect_rebindings(std::slice::from_ref(body), false, out);
            }
            S::Return(Some(e)) => expr(e, out),
            S::Break | S::Continue | S::Return(None) => {}
        }
    }
}

fn check_calls(
    stmts: &[crate::ast::Stmt],
    arities: &std::collections::HashMap<String, usize>,
    out: &mut Vec<(usize, usize, String)>,
) {
    use crate::ast::{ExprKind as E, StmtKind as S};
    fn expr(
        e: &crate::ast::Expr,
        arities: &std::collections::HashMap<String, usize>,
        out: &mut Vec<(usize, usize, String)>,
    ) {
        match &e.kind {
            E::Call(callee, args) => {
                if let E::Var(name) = &callee.kind
                    && let Some(want) = arities.get(name)
                    && *want != args.len()
                {
                    let s = if *want == 1 { "" } else { "s" };
                    out.push((
                        callee.span.start,
                        callee.span.end,
                        format!(
                            "`{name}` takes {want} argument{s}, called with {}",
                            args.len()
                        ),
                    ));
                }
                expr(callee, arities, out);
                args.iter().for_each(|a| expr(a, arities, out));
            }
            E::Fn(_, body) => check_calls(body, arities, out),
            E::List(items) => items.iter().for_each(|i| expr(i, arities, out)),
            E::Map(entries) => entries.iter().for_each(|(k, v)| {
                expr(k, arities, out);
                expr(v, arities, out);
            }),
            E::Unary(_, a) => expr(a, arities, out),
            E::Binary(_, a, b) => {
                expr(a, arities, out);
                expr(b, arities, out);
            }
            E::Index(base, idx) => {
                expr(base, arities, out);
                expr(idx, arities, out);
            }
            _ => {}
        }
    }
    for stmt in stmts {
        match &stmt.kind {
            S::Let(_, e) | S::Assign(_, e) | S::Expr(e) | S::Return(Some(e)) => {
                expr(e, arities, out)
            }
            S::IndexAssign(base, idx, value) => {
                expr(base, arities, out);
                expr(idx, arities, out);
                expr(value, arities, out);
            }
            S::Block(inner) => check_calls(inner, arities, out),
            S::If(cond, then, els) => {
                expr(cond, arities, out);
                check_calls(std::slice::from_ref(then), arities, out);
                if let Some(e) = els {
                    check_calls(std::slice::from_ref(e), arities, out);
                }
            }
            S::While(cond, body) => {
                expr(cond, arities, out);
                check_calls(std::slice::from_ref(body), arities, out);
            }
            S::For(_, iterable, body) => {
                expr(iterable, arities, out);
                check_calls(std::slice::from_ref(body), arities, out);
            }
            S::Break | S::Continue | S::Return(None) => {}
        }
    }
}

/// One name read but never bound, with the nearest name in scope.
pub(crate) struct UnboundFinding {
    pub start: usize,
    pub end: usize,
    pub name: String,
    pub near: Option<String>,
}

/// The findings behind `unbound_names`, before they become sentences:
/// the editor turns them into quickfixes as well as diagnostics.
pub(crate) fn unbound_findings(src: &str) -> Vec<UnboundFinding> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    let Ok(program) = crate::parser::parse_program(&tokens) else {
        return Vec::new();
    };
    let mut scopes: Vec<std::collections::HashSet<String>> = vec![
        Builtin::ALL
            .iter()
            .map(|b| b.name().to_string())
            .collect::<std::collections::HashSet<String>>(),
    ];
    let mut out = Vec::new();
    walk_block(&program, &tokens, &mut scopes, &mut out);
    out.sort_by_key(|f| f.start);
    out
}

type Scopes = Vec<std::collections::HashSet<String>>;
type Findings = Vec<UnboundFinding>;

fn walk_block(
    stmts: &[crate::ast::Stmt],
    tokens: &[lexer::Token],
    scopes: &mut Scopes,
    out: &mut Findings,
) {
    let mut names = std::collections::HashSet::new();
    for s in stmts {
        if let crate::ast::StmtKind::Let(name, _) = &s.kind {
            names.insert(name.clone());
        }
    }
    scopes.push(names);
    for s in stmts {
        walk_stmt(s, tokens, scopes, out);
    }
    scopes.pop();
}

fn walk_stmt(
    stmt: &crate::ast::Stmt,
    tokens: &[lexer::Token],
    scopes: &mut Scopes,
    out: &mut Findings,
) {
    use crate::ast::StmtKind as S;
    match &stmt.kind {
        S::Let(_, e) => walk_expr(e, tokens, scopes, out),
        S::Assign(name, e) => {
            if !bound(scopes, name) {
                report(name, stmt.span.start, tokens, scopes, out);
            }
            walk_expr(e, tokens, scopes, out);
        }
        S::IndexAssign(base, idx, value) => {
            walk_expr(base, tokens, scopes, out);
            walk_expr(idx, tokens, scopes, out);
            walk_expr(value, tokens, scopes, out);
        }
        S::Expr(e) => walk_expr(e, tokens, scopes, out),
        S::Block(stmts) => walk_block(stmts, tokens, scopes, out),
        S::If(cond, then, els) => {
            walk_expr(cond, tokens, scopes, out);
            walk_stmt(then, tokens, scopes, out);
            if let Some(e) = els {
                walk_stmt(e, tokens, scopes, out);
            }
        }
        S::While(cond, body) => {
            walk_expr(cond, tokens, scopes, out);
            walk_stmt(body, tokens, scopes, out);
        }
        S::For(var, iterable, body) => {
            walk_expr(iterable, tokens, scopes, out);
            scopes.push(std::iter::once(var.clone()).collect());
            walk_stmt(body, tokens, scopes, out);
            scopes.pop();
        }
        S::Return(Some(e)) => walk_expr(e, tokens, scopes, out),
        S::Break | S::Continue | S::Return(None) => {}
    }
}

fn walk_expr(
    expr: &crate::ast::Expr,
    tokens: &[lexer::Token],
    scopes: &mut Scopes,
    out: &mut Findings,
) {
    use crate::ast::ExprKind as E;
    match &expr.kind {
        E::Var(name) => {
            if !bound(scopes, name) {
                report(name, expr.span.start, tokens, scopes, out);
            }
        }
        E::List(items) => {
            for e in items {
                walk_expr(e, tokens, scopes, out);
            }
        }
        E::Map(entries) => {
            for (k, v) in entries {
                walk_expr(k, tokens, scopes, out);
                walk_expr(v, tokens, scopes, out);
            }
        }
        E::Unary(_, e) => walk_expr(e, tokens, scopes, out),
        E::Binary(_, a, b) => {
            walk_expr(a, tokens, scopes, out);
            walk_expr(b, tokens, scopes, out);
        }
        E::Call(callee, args) => {
            walk_expr(callee, tokens, scopes, out);
            for a in args {
                walk_expr(a, tokens, scopes, out);
            }
        }
        E::Index(base, idx) => {
            walk_expr(base, tokens, scopes, out);
            walk_expr(idx, tokens, scopes, out);
        }
        E::Fn(params, body) => {
            scopes.push(params.iter().cloned().collect());
            walk_block(body, tokens, scopes, out);
            scopes.pop();
        }
        E::Int(_) | E::Float(_) | E::Str(_) | E::Bool(_) | E::Nil => {}
    }
}

fn bound(scopes: &Scopes, name: &str) -> bool {
    scopes.iter().any(|s| s.contains(name))
}

/// One finding, spanning the identifier itself (found in the token
/// stream at or after `from`), with the nearest name in scope.
fn report(name: &str, from: usize, tokens: &[lexer::Token], scopes: &Scopes, out: &mut Findings) {
    let Some(tok) = tokens.iter().find(|t| {
        t.span.start >= from && matches!(&t.kind, lexer::TokenKind::Ident(n) if n == name)
    }) else {
        return;
    };
    let visible: Vec<&str> = scopes.iter().flatten().map(String::as_str).collect();
    out.push(UnboundFinding {
        start: tok.span.start,
        end: tok.span.end,
        name: name.to_string(),
        near: crate::diag::nearest(name, visible),
    });
}

pub fn unused_top_level_lets(src: &str) -> Vec<(usize, usize, String)> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    let Ok(program) = crate::parser::parse_program(&tokens) else {
        return Vec::new();
    };
    // A file made only of bindings is a module: its top-level names
    // are exports for importers, not unused.
    if program
        .iter()
        .all(|stmt| matches!(stmt.kind, crate::ast::StmtKind::Let(..)))
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    for stmt in &program {
        let crate::ast::StmtKind::Let(name, _) = &stmt.kind else {
            continue;
        };
        if name.starts_with('_') {
            continue;
        }
        let uses = tokens
            .iter()
            .filter(|t| matches!(&t.kind, lexer::TokenKind::Ident(n) if n == name))
            .count();
        if uses > 1 {
            continue;
        }
        let Some(tok) = tokens.iter().find(|t| {
            t.span.start >= stmt.span.start
                && matches!(&t.kind, lexer::TokenKind::Ident(n) if n == name)
        }) else {
            continue;
        };
        out.push((
            tok.span.start,
            tok.span.end,
            format!("`{name}` is never used"),
        ));
    }
    out
}

/// Parameters no identifier in the function's body ever names, from
/// the token stream alone: `fn` `(` params `)` then a brace-balanced
/// body. Underscore-prefixed names are exempt. A nested function that
/// reuses the name counts as a use (a rare false negative, never a
/// false positive).
pub fn unused_params(src: &str) -> Vec<(usize, usize, String)> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if !matches!(tokens[i].kind, lexer::TokenKind::Fn) {
            i += 1;
            continue;
        }
        // Skip an optional name, expect `(`.
        let mut j = i + 1;
        if matches!(
            tokens.get(j).map(|t| &t.kind),
            Some(lexer::TokenKind::Ident(_))
        ) {
            j += 1;
        }
        if !matches!(
            tokens.get(j).map(|t| &t.kind),
            Some(lexer::TokenKind::LParen)
        ) {
            i += 1;
            continue;
        }
        let mut params: Vec<(usize, usize, &str)> = Vec::new();
        j += 1;
        while let Some(t) = tokens.get(j) {
            match &t.kind {
                lexer::TokenKind::Ident(n) => params.push((t.span.start, t.span.end, n)),
                lexer::TokenKind::RParen => break,
                _ => {}
            }
            j += 1;
        }
        // Body: the next `{` through its matching `}`.
        let Some(open) = tokens[j..]
            .iter()
            .position(|t| matches!(t.kind, lexer::TokenKind::LBrace))
            .map(|k| j + k)
        else {
            break;
        };
        let mut depth = 0usize;
        let mut close = open;
        for (k, t) in tokens.iter().enumerate().skip(open) {
            match t.kind {
                lexer::TokenKind::LBrace => depth += 1,
                lexer::TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        close = k;
                        break;
                    }
                }
                _ => {}
            }
        }
        let body = &tokens[open..=close];
        for (start, end, name) in params {
            if name.starts_with('_') {
                continue;
            }
            let used = body
                .iter()
                .any(|t| matches!(&t.kind, lexer::TokenKind::Ident(n) if n == name));
            if !used {
                out.push((start, end, format!("parameter `{name}` is never used")));
            }
        }
        i += 1;
    }
    out
}

/// `let` bindings inside a block (a function body, a loop, an `if`
/// arm) whose name appears nowhere else in that block, from the token
/// stream: the enclosing block is the innermost `{`..`}` around the
/// `let`. Underscore-prefixed names are exempt. A nested block that
/// reuses the name counts as a use (a false negative, never a false
/// positive); the top level is the other warning's job.
pub fn unused_local_lets(src: &str) -> Vec<(usize, usize, String)> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    // For every token index, the index of the innermost enclosing `{`
    // (None at the top level), and for every `{` its matching `}`.
    let mut enclosing: Vec<Option<usize>> = Vec::with_capacity(tokens.len());
    let mut closing: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut stack: Vec<usize> = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        match t.kind {
            lexer::TokenKind::LBrace => {
                enclosing.push(stack.last().copied());
                stack.push(i);
            }
            lexer::TokenKind::RBrace => {
                if let Some(open) = stack.pop() {
                    closing.insert(open, i);
                }
                enclosing.push(stack.last().copied());
            }
            _ => enclosing.push(stack.last().copied()),
        }
    }
    let mut out = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        if !matches!(t.kind, lexer::TokenKind::Let) {
            continue;
        }
        let Some(open) = enclosing[i] else {
            continue;
        };
        let Some(name_tok) = tokens.get(i + 1) else {
            continue;
        };
        let lexer::TokenKind::Ident(name) = &name_tok.kind else {
            continue;
        };
        if name.starts_with('_') {
            continue;
        }
        let close = closing.get(&open).copied().unwrap_or(tokens.len() - 1);
        let used = tokens[open..=close].iter().enumerate().any(|(k, tok)| {
            open + k != i + 1 && matches!(&tok.kind, lexer::TokenKind::Ident(n) if n == name)
        });
        if !used {
            out.push((
                name_tok.span.start,
                name_tok.span.end,
                format!("`{name}` is never used"),
            ));
        }
    }
    out
}

/// Bindings named after a builtin — a `let`, a `fn`, or a parameter —
/// which hide it for the rest of their scope; the language allows it,
/// but a later `len(xs)` failing with "not callable" is the usual
/// outcome. Token-based: the identifier after `let`/`fn`, and every
/// identifier inside a `fn`'s parameter list.
pub fn shadowed_builtins(src: &str) -> Vec<(usize, usize, String)> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    let is_builtin = |n: &str| Builtin::ALL.iter().any(|b| b.name() == n);
    let mut out = Vec::new();
    let mut in_params = false;
    for (i, t) in tokens.iter().enumerate() {
        match &t.kind {
            lexer::TokenKind::Fn => {
                // `fn name(` or `fn(`: the parameter list follows the `(`.
                in_params = true;
            }
            lexer::TokenKind::RParen if in_params => in_params = false,
            lexer::TokenKind::Ident(n) => {
                let after_let = i > 0 && matches!(tokens[i - 1].kind, lexer::TokenKind::Let);
                let after_fn = i > 0 && matches!(tokens[i - 1].kind, lexer::TokenKind::Fn);
                let param = in_params && !after_fn;
                if (after_let || after_fn || param) && is_builtin(n) {
                    out.push((t.span.start, t.span.end, format!("`{n}` shadows a builtin")));
                }
            }
            _ => {}
        }
    }
    out
}

/// Every semantic warning for a source: unknown stdlib members, then
/// unused top-level bindings, then unused parameters. Shared by
/// --check and the LSP.
pub fn warnings(src: &str) -> Vec<(usize, usize, String)> {
    let mut all = unknown_stdlib_members(src);
    all.extend(unbound_names(src));
    all.extend(arity_mismatches(src));
    all.extend(duplicate_map_keys(src));
    all.extend(unreachable_code(src));
    all.extend(unused_top_level_lets(src));
    all.extend(unused_params(src));
    all.extend(unused_local_lets(src));
    all.extend(shadowed_builtins(src));
    // One file's warnings read in the order its lines do, whatever
    // pass found them.
    all.sort_by_key(|(start, _, _)| *start);
    all
}

pub fn unknown_stdlib_members(src: &str) -> Vec<(usize, usize, String)> {
    stdlib_member_findings(src)
        .into_iter()
        .map(|f| {
            let near = crate::diag::nearest(&f.key, f.exports.iter().map(String::as_str));
            let message = match near {
                Some(n) => format!("{} has no `{}` (did you mean `{}`?)", f.module, f.key, n),
                None => format!("{} has no `{}`", f.module, f.key),
            };
            (f.start, f.end, message)
        })
        .collect()
}

/// One unknown-member occurrence: the key's byte span, the module it
/// was looked up in, the key itself, and what the module does export
/// (for suggestions).
struct MemberFinding {
    start: usize,
    end: usize,
    module: &'static str,
    key: String,
    exports: Vec<String>,
}

fn stdlib_member_findings(src: &str) -> Vec<MemberFinding> {
    let mut out = Vec::new();
    // Bindings: `let <ident> = import("<...lib/x.ting>")`.
    let mut bindings: Vec<(String, &'static str, Vec<String>)> = Vec::new();
    for line in src.lines() {
        let Some(rest) = line.trim_start().strip_prefix("let ") else {
            continue;
        };
        let Some((name, value)) = rest.split_once('=') else {
            continue;
        };
        let value = value.trim();
        let Some(arg) = value.strip_prefix("import(\"") else {
            continue;
        };
        let Some(path_end) = arg.find('"') else {
            continue;
        };
        let path = &arg[..path_end];
        for (module, source) in crate::eval::embedded_stdlib() {
            if path.ends_with(module) {
                let exports = source
                    .lines()
                    .filter_map(|l| {
                        l.strip_prefix("fn ")
                            .and_then(|r| r.split('(').next())
                            .or_else(|| l.strip_prefix("let ").and_then(|r| r.split('=').next()))
                    })
                    .map(|n| n.trim().to_string())
                    .collect();
                bindings.push((name.trim().to_string(), module, exports));
            }
        }
    }
    for (name, module, exports) in &bindings {
        let needle = format!("{name}[\"");
        let mut from = 0;
        while let Some(i) = src[from..].find(&needle) {
            let key_start = from + i + needle.len();
            // Must be a whole identifier: not preceded by an ident char.
            let bounded = src[..from + i]
                .chars()
                .next_back()
                .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
            let Some(key_len) = src[key_start..].find('"') else {
                break;
            };
            let key = &src[key_start..key_start + key_len];
            if bounded && !exports.iter().any(|e| e == key) {
                out.push(MemberFinding {
                    start: key_start,
                    end: key_start + key_len,
                    module,
                    key: key.to_string(),
                    exports: exports.clone(),
                });
            }
            from = key_start + key_len;
        }
    }
    out
}

/// Edit distance with adjacent transpositions counting one (optimal
/// string alignment), so "medain" is one step from "median" rather
/// than tying with "mean".
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

/// Quickfix code actions for unknown-member warnings whose key lies on
/// one of the requested lines: replace the key with the nearest export
/// when it is close enough to be a plausible typo.
fn code_action_result(src: &str, uri: &str, first_line: usize, last_line: usize) -> Value {
    let mut actions = Vec::new();
    let fix = |start: usize, end: usize, best: &str, actions: &mut Vec<Value>| {
        let line = src[..start].matches('\n').count();
        if line < first_line || line > last_line {
            return;
        }
        let edit = obj(vec![
            (
                "range",
                obj(vec![
                    ("start", position(src, start)),
                    ("end", position(src, end)),
                ]),
            ),
            ("newText", s(best)),
        ]);
        actions.push(obj(vec![
            ("title", s(&format!("Replace with `{best}`"))),
            ("kind", s("quickfix")),
            (
                "edit",
                obj(vec![("changes", obj(vec![(uri, Value::list(vec![edit]))]))]),
            ),
        ]));
    };
    // A name bound nowhere, when something in scope is close to it.
    for f in unbound_findings(src) {
        if let Some(near) = &f.near {
            fix(f.start, f.end, near, &mut actions);
        }
    }
    for f in stdlib_member_findings(src) {
        let line = src[..f.start].matches('\n').count();
        if line < first_line || line > last_line {
            continue;
        }
        let Some((best, dist)) = f
            .exports
            .iter()
            .map(|e| (e, levenshtein(&f.key, e)))
            .min_by_key(|(e, d)| (*d, (*e).clone()))
        else {
            continue;
        };
        // Allow one edit per four characters, at least two.
        if dist > (f.key.chars().count() / 4).max(2) {
            continue;
        }
        let edit = obj(vec![
            (
                "range",
                obj(vec![
                    ("start", position(src, f.start)),
                    ("end", position(src, f.end)),
                ]),
            ),
            ("newText", s(best)),
        ]);
        actions.push(obj(vec![
            ("title", s(&format!("Replace with `{best}`"))),
            ("kind", s("quickfix")),
            (
                "edit",
                obj(vec![("changes", obj(vec![(uri, Value::list(vec![edit]))]))]),
            ),
        ]));
    }
    Value::list(actions)
}

fn publish(output: &mut impl Write, uri: &str, src: &str) {
    write_message(
        output,
        &obj(vec![
            ("jsonrpc", s("2.0")),
            ("method", s("textDocument/publishDiagnostics")),
            (
                "params",
                obj(vec![
                    ("uri", s(uri)),
                    ("diagnostics", diagnostics(src, uri)),
                ]),
            ),
        ]),
    );
}

/// The identifier under a 0-based (line, character) position, if any.
fn ident_at(src: &str, line: usize, character: usize) -> Option<String> {
    let text = src.lines().nth(line)?;
    let chars: Vec<char> = text.chars().collect();
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut start = character.min(chars.len());
    while start > 0 && is_ident(chars[start - 1]) {
        start -= 1;
    }
    let mut end = start;
    while end < chars.len() && is_ident(chars[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    Some(chars[start..end].iter().collect())
}

/// Exported functions of every embedded stdlib module the document
/// imports (matched on the module-path suffix, so relative paths
/// count): (module path, name, signature, leading comment).
pub fn imported_stdlib_functions(src: &str) -> Vec<(&'static str, String, String, String)> {
    let mut out = Vec::new();
    for (path, source) in crate::eval::embedded_stdlib() {
        if !src.contains(path) {
            continue;
        }
        for (name, sig, comment) in source_functions(source) {
            out.push((*path, name, sig, comment));
        }
    }
    out
}

/// The top-level `fn name(params)` declarations of a source with the
/// `#` comment lines directly above each: (name, signature, comment
/// joined by spaces). Line-based, so it works on any ting file —
/// stdlib modules and the user's own.
pub fn source_functions(source: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut comment: Vec<&str> = Vec::new();
    for line in source.lines() {
        if let Some(text) = line.strip_prefix('#') {
            comment.push(text.trim());
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn ")
            && let Some(close) = rest.find(')')
        {
            let sig = &rest[..=close];
            let name = &sig[..sig.find('(').unwrap_or(sig.len())];
            out.push((name.to_string(), sig.to_string(), comment.join(" ")));
        }
        comment.clear();
    }
    out
}

fn hover_result(src: &str, line: usize, character: usize) -> Value {
    let Some(word) = ident_at(src, line, character) else {
        return Value::Nil;
    };
    let text = if let Some(b) = Builtin::ALL.iter().find(|b| b.name() == word) {
        let (sig, summary) = b.doc();
        format!("```ting\n{sig}\n```\n\n{summary}")
    } else if let Some((path, _, sig, comment)) = imported_stdlib_functions(src)
        .into_iter()
        .find(|(_, name, _, _)| *name == word)
    {
        let about = if comment.is_empty() {
            format!("from `{path}`")
        } else {
            format!("{comment}\n\n(from `{path}`)")
        };
        format!("```ting\n{sig}\n```\n\n{about}")
    } else if let Some(params) = user_fn_params(src, &word) {
        // A top-level fn (or let bound to a fn literal) in this document,
        // with the `#` comment above it when there is one — the user's
        // own code documents itself the way the stdlib does.
        let comment = source_functions(src)
            .into_iter()
            .find(|(name, _, _)| *name == word)
            .map(|(_, _, comment)| comment)
            .unwrap_or_default();
        let about = if comment.is_empty() {
            "defined in this file".to_string()
        } else {
            format!("{comment}\n\n(defined in this file)")
        };
        format!("```ting\nfn {word}({})\n```\n\n{about}", params.join(", "))
    } else {
        return Value::Nil;
    };
    obj(vec![(
        "contents",
        obj(vec![("kind", s("markdown")), ("value", s(&text))]),
    )])
}

/// The parameter names of a top-level function bound to `name` in
/// `src`, if the document parses and such a binding exists.
fn user_fn_params(src: &str, name: &str) -> Option<Vec<String>> {
    let tokens = lexer::lex(src).ok()?;
    let program = crate::parser::parse_program(&tokens).ok()?;
    program.iter().find_map(|stmt| match &stmt.kind {
        crate::ast::StmtKind::Let(n, expr) if n == name => match &expr.kind {
            crate::ast::ExprKind::Fn(params, _) => Some(params.clone()),
            _ => None,
        },
        _ => None,
    })
}

/// Signature help inside a call: scan left from the cursor for the
/// innermost unclosed '(' , take the identifier before it, and if
/// it's a builtin return its signature and doc line.
fn signature_help_result(src: &str, line: usize, character: usize) -> Value {
    let Some(text) = src.lines().nth(line) else {
        return Value::Nil;
    };
    let chars: Vec<char> = text.chars().collect();
    let upto = character.min(chars.len());
    let mut depth = 0i32;
    let mut open = None;
    for i in (0..upto).rev() {
        match chars[i] {
            ')' => depth += 1,
            '(' if depth > 0 => depth -= 1,
            '(' => {
                open = Some(i);
                break;
            }
            _ => {}
        }
    }
    let Some(open) = open else {
        return Value::Nil;
    };
    // Identifier ending right before the '('.
    let mut start = open;
    while start > 0 && (chars[start - 1].is_ascii_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    let word: String = chars[start..open].iter().collect();
    let (sig, doc) = if let Some(b) = Builtin::ALL.iter().find(|b| b.name() == word) {
        let (sig, summary) = b.doc();
        (sig.to_string(), summary.to_string())
    } else if let Some(key) = quoted_key_before(&chars, open)
        && let Some((path, _, sig, comment)) = imported_stdlib_functions(src)
            .into_iter()
            .find(|(_, name, _, _)| *name == key)
    {
        // A stdlib call through its module map: m["name"](...).
        (sig, format!("{comment} (from {path})"))
    } else if let Some(params) = user_fn_params(src, &word) {
        // One of the file's own top-level functions.
        (
            format!("{word}({})", params.join(", ")),
            "defined in this file".to_string(),
        )
    } else {
        return Value::Nil;
    };
    obj(vec![
        (
            "signatures",
            Value::list(vec![obj(vec![
                ("label", s(&sig)),
                ("documentation", s(doc.trim())),
            ])]),
        ),
        ("activeSignature", Value::Int(0)),
    ])
}

/// The string inside `["..."]` ending right before position `end`,
/// for calls made through a module map.
fn quoted_key_before(chars: &[char], end: usize) -> Option<String> {
    if end < 4 || chars[end - 1] != ']' || chars[end - 2] != '"' {
        return None;
    }
    let close = end - 2;
    let open = chars[..close].iter().rposition(|&c| c == '"')?;
    if open == 0 || chars[open - 1] != '[' {
        return None;
    }
    Some(chars[open + 1..close].iter().collect())
}

const KEYWORDS: &[&str] = &[
    "let", "fn", "if", "else", "while", "for", "in", "break", "continue", "return", "true",
    "false", "nil",
];

/// Completion items: every builtin (with docs), keywords, and the
/// identifiers already present in the document.
fn completion_result(src: &str) -> Value {
    let mut items = Vec::new();
    for b in Builtin::ALL {
        let (sig, summary) = b.doc();
        items.push(obj(vec![
            ("label", s(b.name())),
            ("kind", Value::Int(3)), // Function
            ("detail", s(sig)),
            ("documentation", s(summary)),
        ]));
    }
    for kw in KEYWORDS {
        items.push(obj(vec![("label", s(kw)), ("kind", Value::Int(14))]));
    }
    // The file's own top-level functions complete as functions, with
    // their signature and the `#` comment above them, like the stdlib's.
    let own: Vec<(String, String, String)> = source_functions(src);
    for (name, sig, comment) in &own {
        items.push(obj(vec![
            ("label", s(name)),
            ("kind", Value::Int(3)), // Function
            ("detail", s(&format!("fn {sig}"))),
            ("documentation", s(comment)),
        ]));
    }
    let mut seen: Vec<String> = Vec::new();
    let mut word = String::new();
    for c in src.chars().chain([' ']) {
        if c.is_ascii_alphanumeric() || c == '_' {
            word.push(c);
        } else if !word.is_empty() {
            let w = std::mem::take(&mut word);
            if !w.chars().next().is_some_and(|c| c.is_ascii_digit())
                && !KEYWORDS.contains(&w.as_str())
                && Builtin::ALL.iter().all(|b| b.name() != w)
                && own.iter().all(|(name, _, _)| *name != w)
                && !seen.contains(&w)
            {
                seen.push(w);
            }
        }
    }
    for w in seen {
        items.push(obj(vec![("label", s(&w)), ("kind", Value::Int(6))])); // Variable
    }
    // Exported functions of every stdlib module the document imports
    // (matched by the "lib/<name>.ting" suffix, so "../lib/list.ting"
    // counts too): the names users reach for through m["name"].
    for (path, name, sig, comment) in imported_stdlib_functions(src) {
        items.push(obj(vec![
            ("label", s(&name)),
            ("kind", Value::Int(3)), // Function
            ("detail", s(&format!("{path}: {sig}"))),
            ("documentation", s(&comment)),
        ]));
    }
    Value::list(items)
}

pub fn run() -> i32 {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    let mut shutdown_seen = false;
    let mut docs: BTreeMap<String, String> = BTreeMap::new();

    while let Some(read) = read_message(&mut input) {
        let Some(msg) = read else {
            continue;
        };
        let method = get_str(&msg, "method").unwrap_or_default();
        let id = get(&msg, "id");
        match method.as_str() {
            "initialize" => {
                let result = obj(vec![
                    (
                        "capabilities",
                        // 1 = full-text document sync on every change.
                        obj(vec![
                            ("textDocumentSync", Value::Int(1)),
                            ("hoverProvider", Value::Bool(true)),
                            ("completionProvider", obj(vec![])),
                            ("documentFormattingProvider", Value::Bool(true)),
                            ("documentSymbolProvider", Value::Bool(true)),
                            ("definitionProvider", Value::Bool(true)),
                            ("referencesProvider", Value::Bool(true)),
                            ("documentHighlightProvider", Value::Bool(true)),
                            (
                                "renameProvider",
                                obj(vec![("prepareProvider", Value::Bool(true))]),
                            ),
                            ("codeActionProvider", Value::Bool(true)),
                            ("foldingRangeProvider", Value::Bool(true)),
                            ("workspaceSymbolProvider", Value::Bool(true)),
                            (
                                "documentLinkProvider",
                                obj(vec![("resolveProvider", Value::Bool(false))]),
                            ),
                            (
                                "signatureHelpProvider",
                                obj(vec![(
                                    "triggerCharacters",
                                    Value::list(vec![s("("), s(",")]),
                                )]),
                            ),
                        ]),
                    ),
                    (
                        "serverInfo",
                        obj(vec![
                            ("name", s("ting")),
                            ("version", s(env!("CARGO_PKG_VERSION"))),
                        ]),
                    ),
                ]);
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "shutdown" => {
                shutdown_seen = true;
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", Value::Nil),
                    ]),
                );
            }
            "exit" => return if shutdown_seen { 0 } else { 1 },
            "textDocument/didOpen" => {
                if let Some(doc) = get(&msg, "params").and_then(|p| get(&p, "textDocument"))
                    && let (Some(uri), Some(text)) = (get_str(&doc, "uri"), get_str(&doc, "text"))
                {
                    publish(&mut output, &uri, &text);
                    docs.insert(uri, text);
                }
            }
            "textDocument/didChange" => {
                let params = get(&msg, "params");
                let uri = params
                    .as_ref()
                    .and_then(|p| get(p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let text = params
                    .as_ref()
                    .and_then(|p| get(p, "contentChanges"))
                    .and_then(|c| match c {
                        Value::List(items) => items.borrow().first().cloned(),
                        _ => None,
                    })
                    .and_then(|change| get_str(&change, "text"));
                if let (Some(uri), Some(text)) = (uri, text) {
                    publish(&mut output, &uri, &text);
                    docs.insert(uri, text);
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = get(&msg, "params")
                    .and_then(|p| get(&p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"))
                {
                    docs.remove(&uri);
                }
            }
            "textDocument/completion" => {
                let uri = get(&msg, "params")
                    .and_then(|p| get(&p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let result = uri
                    .and_then(|u| docs.get(&u).map(|src| completion_result(src)))
                    .unwrap_or(Value::Nil);
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/definition" => {
                let params = get(&msg, "params");
                let uri = params
                    .as_ref()
                    .and_then(|p| get(p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let pos = params.as_ref().and_then(|p| get(p, "position"));
                let line = pos.as_ref().and_then(|p| get(p, "line"));
                let character = pos.as_ref().and_then(|p| get(p, "character"));
                let result = match (uri, line, character) {
                    (Some(uri), Some(Value::Int(l)), Some(Value::Int(c))) => docs
                        .get(&uri)
                        .map(|src| {
                            definition_result(src, &uri, l.max(0) as usize, c.max(0) as usize)
                        })
                        .unwrap_or(Value::Nil),
                    _ => Value::Nil,
                };
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/references" => {
                let params = get(&msg, "params");
                let uri = params
                    .as_ref()
                    .and_then(|p| get(p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let pos = params.as_ref().and_then(|p| get(p, "position"));
                let line = pos.as_ref().and_then(|p| get(p, "line"));
                let character = pos.as_ref().and_then(|p| get(p, "character"));
                let result = match (uri, line, character) {
                    (Some(uri), Some(Value::Int(l)), Some(Value::Int(c))) => docs
                        .get(&uri)
                        .map(|src| {
                            references_result(src, &uri, l.max(0) as usize, c.max(0) as usize)
                        })
                        .unwrap_or(Value::Nil),
                    _ => Value::Nil,
                };
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/documentHighlight" => {
                let params = get(&msg, "params");
                let uri = params
                    .as_ref()
                    .and_then(|p| get(p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let pos = params.as_ref().and_then(|p| get(p, "position"));
                let line = pos.as_ref().and_then(|p| get(p, "line"));
                let character = pos.as_ref().and_then(|p| get(p, "character"));
                let result = match (uri, line, character) {
                    (Some(uri), Some(Value::Int(l)), Some(Value::Int(c))) => docs
                        .get(&uri)
                        .map(|src| highlight_result(src, l.max(0) as usize, c.max(0) as usize))
                        .unwrap_or(Value::Nil),
                    _ => Value::Nil,
                };
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/prepareRename" => {
                let params = get(&msg, "params");
                let uri = params
                    .as_ref()
                    .and_then(|p| get(p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let pos = params.as_ref().and_then(|p| get(p, "position"));
                let line = pos.as_ref().and_then(|p| get(p, "line"));
                let character = pos.as_ref().and_then(|p| get(p, "character"));
                let result = match (uri, line, character) {
                    (Some(uri), Some(Value::Int(l)), Some(Value::Int(c))) => docs
                        .get(&uri)
                        .map(|src| prepare_rename_result(src, l.max(0) as usize, c.max(0) as usize))
                        .unwrap_or(Value::Nil),
                    _ => Value::Nil,
                };
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/rename" => {
                let params = get(&msg, "params");
                let uri = params
                    .as_ref()
                    .and_then(|p| get(p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let pos = params.as_ref().and_then(|p| get(p, "position"));
                let line = pos.as_ref().and_then(|p| get(p, "line"));
                let character = pos.as_ref().and_then(|p| get(p, "character"));
                let new_name = params.as_ref().and_then(|p| get_str(p, "newName"));
                let result = match (uri, line, character, new_name) {
                    (Some(uri), Some(Value::Int(l)), Some(Value::Int(c)), Some(new_name)) => {
                        rename_result(&docs, &uri, l.max(0) as usize, c.max(0) as usize, &new_name)
                    }
                    _ => Value::Nil,
                };
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/codeAction" => {
                let params = get(&msg, "params");
                let uri = params
                    .as_ref()
                    .and_then(|p| get(p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let range = params.as_ref().and_then(|p| get(p, "range"));
                let first = range
                    .as_ref()
                    .and_then(|r| get(r, "start"))
                    .and_then(|p| get(&p, "line"));
                let last = range
                    .as_ref()
                    .and_then(|r| get(r, "end"))
                    .and_then(|p| get(&p, "line"));
                let result = match (uri, first, last) {
                    (Some(uri), Some(Value::Int(a)), Some(Value::Int(b))) => docs
                        .get(&uri)
                        .map(|src| {
                            code_action_result(src, &uri, a.max(0) as usize, b.max(0) as usize)
                        })
                        .unwrap_or_else(|| Value::list(vec![])),
                    _ => Value::list(vec![]),
                };
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/signatureHelp" => {
                let params = get(&msg, "params");
                let uri = params
                    .as_ref()
                    .and_then(|p| get(p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let pos = params.as_ref().and_then(|p| get(p, "position"));
                let line = pos.as_ref().and_then(|p| get(p, "line"));
                let character = pos.as_ref().and_then(|p| get(p, "character"));
                let result = match (uri, line, character) {
                    (Some(uri), Some(Value::Int(l)), Some(Value::Int(c))) => docs
                        .get(&uri)
                        .map(|src| signature_help_result(src, l.max(0) as usize, c.max(0) as usize))
                        .unwrap_or(Value::Nil),
                    _ => Value::Nil,
                };
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "workspace/symbol" => {
                let query = get(&msg, "params")
                    .and_then(|p| get_str(&p, "query"))
                    .unwrap_or_default();
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", workspace_symbols(&docs, &query)),
                    ]),
                );
            }
            "textDocument/documentLink" => {
                let uri = get(&msg, "params")
                    .and_then(|p| get(&p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let result = uri
                    .and_then(|u| docs.get(&u).map(|src| document_links(src, &u)))
                    .unwrap_or_else(|| Value::list(vec![]));
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/foldingRange" => {
                let uri = get(&msg, "params")
                    .and_then(|p| get(&p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let result = uri
                    .and_then(|u| docs.get(&u).map(|src| folding_ranges(src)))
                    .unwrap_or_else(|| Value::list(vec![]));
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/documentSymbol" => {
                let uri = get(&msg, "params")
                    .and_then(|p| get(&p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let result = uri
                    .and_then(|u| docs.get(&u).map(|src| document_symbols(src)))
                    .unwrap_or(Value::Nil);
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/formatting" => {
                let uri = get(&msg, "params")
                    .and_then(|p| get(&p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let result = match uri.and_then(|u| docs.get(&u).cloned()) {
                    Some(src) => match crate::fmt::format(&src) {
                        Ok(formatted) if formatted != src => {
                            // One edit replacing the whole document.
                            // The whole document: from its first position
                            // to its real last one (the split count used
                            // to land one line past the end).
                            Value::list(vec![obj(vec![
                                (
                                    "range",
                                    obj(vec![
                                        (
                                            "start",
                                            obj(vec![
                                                ("line", Value::Int(0)),
                                                ("character", Value::Int(0)),
                                            ]),
                                        ),
                                        ("end", position(&src, src.len())),
                                    ]),
                                ),
                                ("newText", s(&formatted)),
                            ])])
                        }
                        Ok(_) => Value::list(vec![]),
                        Err(_) => Value::Nil,
                    },
                    None => Value::Nil,
                };
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            "textDocument/hover" => {
                let params = get(&msg, "params");
                let uri = params
                    .as_ref()
                    .and_then(|p| get(p, "textDocument"))
                    .and_then(|d| get_str(&d, "uri"));
                let pos = params.as_ref().and_then(|p| get(p, "position"));
                let line = pos.as_ref().and_then(|p| get(p, "line"));
                let character = pos.as_ref().and_then(|p| get(p, "character"));
                let result = match (uri, line, character) {
                    (Some(uri), Some(Value::Int(l)), Some(Value::Int(c))) => docs
                        .get(&uri)
                        .map(|src| hover_result(src, l.max(0) as usize, c.max(0) as usize))
                        .unwrap_or(Value::Nil),
                    _ => Value::Nil,
                };
                write_message(
                    &mut output,
                    &obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.unwrap_or(Value::Nil)),
                        ("result", result),
                    ]),
                );
            }
            // Requests we don't implement get a MethodNotFound error;
            // unknown notifications are ignored, per the spec.
            _ => {
                if let Some(id) = id {
                    write_message(
                        &mut output,
                        &obj(vec![
                            ("jsonrpc", s("2.0")),
                            ("id", id),
                            (
                                "error",
                                obj(vec![
                                    ("code", Value::Int(-32601)),
                                    ("message", s("method not found")),
                                ]),
                            ),
                        ]),
                    );
                }
            }
        }
    }
    0
}
