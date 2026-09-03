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
        if !target.is_file() {
            continue;
        }
        links.push(obj(vec![
            (
                "range",
                obj(vec![
                    ("start", position(src, w[2].span.start)),
                    ("end", position(src, w[2].span.end)),
                ]),
            ),
            ("target", s(&path_to_uri(&target))),
        ]));
    }
    Value::list(links)
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
    for (start, end, message) in unknown_stdlib_members(src) {
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
pub fn unknown_stdlib_members(src: &str) -> Vec<(usize, usize, String)> {
    stdlib_member_findings(src)
        .into_iter()
        .map(|f| (f.start, f.end, format!("{} has no `{}`", f.module, f.key)))
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

/// Exported functions of every embedded stdlib module the document
/// imports (matched on the module-path suffix, so relative paths
/// count): (module path, name, signature, leading comment).
pub fn imported_stdlib_functions(src: &str) -> Vec<(&'static str, String, String, String)> {
    let mut out = Vec::new();
    for (path, source) in crate::eval::embedded_stdlib() {
        if !src.contains(path) {
            continue;
        }
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
                out.push((*path, name.to_string(), sig.to_string(), comment.join(" ")));
            }
            comment.clear();
        }
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
    } else {
        return Value::Nil;
    };
    obj(vec![(
        "contents",
        obj(vec![("kind", s("markdown")), ("value", s(&text))]),
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
                            ("renameProvider", Value::Bool(true)),
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
