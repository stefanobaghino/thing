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
fn read_message(input: &mut impl BufRead) -> Option<Value> {
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
    json::decode(std::str::from_utf8(&buf).ok()?).ok()
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

/// Rename every occurrence of the identifier at (line, character):
/// a WorkspaceEdit with one TextEdit per token, same scan as
/// references (token-level, shadowing not resolved).
fn rename_result(src: &str, uri: &str, line: usize, character: usize, new_name: &str) -> Value {
    let valid = !new_name.is_empty()
        && !new_name.chars().next().unwrap().is_ascii_digit()
        && new_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid {
        return Value::Nil;
    }
    let Some(name) = ident_at(src, line, character) else {
        return Value::Nil;
    };
    let Ok(tokens) = lexer::lex(src) else {
        return Value::Nil;
    };
    let mut edits = Vec::new();
    for tok in &tokens {
        if let lexer::TokenKind::Ident(n) = &tok.kind
            && n == &name
        {
            edits.push(obj(vec![
                (
                    "range",
                    obj(vec![
                        ("start", position(src, tok.span.start)),
                        ("end", position(src, tok.span.end)),
                    ]),
                ),
                ("newText", s(new_name)),
            ]));
        }
    }
    if edits.is_empty() {
        return Value::Nil;
    }
    obj(vec![("changes", obj(vec![(uri, Value::list(edits))]))])
}

/// Lex + parse + compile; the first error becomes the diagnostic list.
fn diagnostics(src: &str) -> Value {
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
    let list = match err {
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
    Value::list(list)
}

fn publish(output: &mut impl Write, uri: &str, src: &str) {
    write_message(
        output,
        &obj(vec![
            ("jsonrpc", s("2.0")),
            ("method", s("textDocument/publishDiagnostics")),
            (
                "params",
                obj(vec![("uri", s(uri)), ("diagnostics", diagnostics(src))]),
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

fn hover_result(src: &str, line: usize, character: usize) -> Value {
    let Some(word) = ident_at(src, line, character) else {
        return Value::Nil;
    };
    let Some(b) = Builtin::ALL.iter().find(|b| b.name() == word) else {
        return Value::Nil;
    };
    let (sig, summary) = b.doc();
    obj(vec![(
        "contents",
        obj(vec![
            ("kind", s("markdown")),
            ("value", s(&format!("```ting\n{sig}\n```\n\n{summary}"))),
        ]),
    )])
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
    let Some(b) = Builtin::ALL.iter().find(|b| b.name() == word) else {
        return Value::Nil;
    };
    let (sig, summary) = b.doc();
    obj(vec![
        (
            "signatures",
            Value::list(vec![obj(vec![
                ("label", s(sig)),
                ("documentation", s(summary)),
            ])]),
        ),
        ("activeSignature", Value::Int(0)),
    ])
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
                && !seen.contains(&w)
            {
                seen.push(w);
            }
        }
    }
    for w in seen {
        items.push(obj(vec![("label", s(&w)), ("kind", Value::Int(6))])); // Variable
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

    while let Some(msg) = read_message(&mut input) {
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
                            ("renameProvider", Value::Bool(true)),
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
                    (Some(uri), Some(Value::Int(l)), Some(Value::Int(c)), Some(new_name)) => docs
                        .get(&uri)
                        .map(|src| {
                            rename_result(
                                src,
                                &uri,
                                l.max(0) as usize,
                                c.max(0) as usize,
                                &new_name,
                            )
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
                            let end_line = src.split('\n').count() as i64;
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
                                        (
                                            "end",
                                            obj(vec![
                                                ("line", Value::Int(end_line)),
                                                ("character", Value::Int(0)),
                                            ]),
                                        ),
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
