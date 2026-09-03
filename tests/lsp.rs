//! Drives `ting --lsp` over real pipes with LSP traffic: lifecycle,
//! diagnostics on open, cleared diagnostics after a fixing change.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

fn send(stdin: &mut ChildStdin, body: &str) {
    write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    stdin.flush().unwrap();
}

fn recv(reader: &mut BufReader<ChildStdout>) -> String {
    let mut len = 0usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            len = v.trim().parse().unwrap();
        }
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

fn spawn_server() -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ting"))
        .arg("--lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn ting --lsp");
    let stdin = child.stdin.take().unwrap();
    let reader = BufReader::new(child.stdout.take().unwrap());
    (child, stdin, reader)
}

#[test]
fn lsp_session_lifecycle_and_diagnostics() {
    let (mut child, mut stdin, mut reader) = spawn_server();

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(init.contains("\"textDocumentSync\":1"), "{init}");
    assert!(init.contains("\"name\":\"ting\""), "{init}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );

    // Open a broken document: expect one diagnostic with a range.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///t.ting","languageId":"ting","version":1,"text":"let x = ;"}}}"#,
    );
    let diag = recv(&mut reader);
    assert!(diag.contains("publishDiagnostics"), "{diag}");
    assert!(diag.contains("file:///t.ting"), "{diag}");
    assert!(diag.contains("expected"), "{diag}");
    assert!(diag.contains("\"line\":0"), "{diag}");

    // A fixing change clears the diagnostics.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///t.ting","version":2},"contentChanges":[{"text":"let x = 1;"}]}}"#,
    );
    let cleared = recv(&mut reader);
    assert!(cleared.contains("\"diagnostics\":[]"), "{cleared}");

    // Hover over "print" (line 0, inside the word) shows builtin docs.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":5,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///t.ting"},"position":{"line":0,"character":2}}}"#,
    );
    let hov = recv(&mut reader);
    assert!(!hov.contains("let x = 1;"), "{hov}");
    // The document is "let x = 1;": position 2 is inside "let" (a
    // keyword, not a builtin) -> null result.
    assert!(hov.contains("\"result\":null"), "{hov}");

    // Replace with a builtin call and hover over it.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///t.ting","version":3},"contentChanges":[{"text":"print(1);"}]}}"#,
    );
    let _diag = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":6,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///t.ting"},"position":{"line":0,"character":2}}}"#,
    );
    let hov = recv(&mut reader);
    assert!(hov.contains("print(...)"), "{hov}");
    assert!(hov.contains("markdown"), "{hov}");

    // Completions include builtins with docs, keywords, and the
    // document's own identifiers (the doc is now "print(1);" — add one
    // with a variable first).
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///t.ting","version":4},"contentChanges":[{"text":"let counter_total = 1;"}]}}"#,
    );
    let _diag = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":7,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///t.ting"},"position":{"line":0,"character":0}}}"#,
    );
    let comp = recv(&mut reader);
    assert!(comp.contains("\"label\":\"print\""), "{comp}");
    assert!(comp.contains("print(...)"), "{comp}");
    assert!(comp.contains("\"label\":\"while\""), "{comp}");
    assert!(comp.contains("\"label\":\"counter_total\""), "{comp}");

    // Formatting: unformatted source yields one whole-document edit;
    // canonical source yields no edits.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///t.ting","version":5},"contentChanges":[{"text":"let x=1+2;"}]}}"#,
    );
    let _diag = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":8,"method":"textDocument/formatting","params":{"textDocument":{"uri":"file:///t.ting"},"options":{}}}"#,
    );
    let fmt = recv(&mut reader);
    assert!(fmt.contains("let x = 1 + 2;"), "{fmt}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///t.ting","version":6},"contentChanges":[{"text":"let x = 1 + 2;\n"}]}}"#,
    );
    let _diag = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":9,"method":"textDocument/formatting","params":{"textDocument":{"uri":"file:///t.ting"},"options":{}}}"#,
    );
    let fmt = recv(&mut reader);
    assert!(fmt.contains("\"result\":[]"), "{fmt}");

    // Unknown request gets a MethodNotFound error.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":11,"method":"textDocument/typeDefinition","params":{}}"#,
    );
    let err = recv(&mut reader);
    assert!(err.contains("-32601"), "{err}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#,
    );
    let shut = recv(&mut reader);
    assert!(shut.contains("\"result\":null"), "{shut}");
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);

    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(0), "clean exit after shutdown");
}

#[test]
fn completion_offers_imported_stdlib_functions() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let _ = recv(&mut reader);

    // No import: stdlib names stay out of the list.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///c.ting","text":"print(1);\n"}}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///c.ting"},"position":{"line":0,"character":0}}}"#,
    );
    let comp = recv(&mut reader);
    assert!(!comp.contains("\"label\":\"median\""), "{comp}");

    // Importing a module (by any relative path) exposes its functions
    // with the module and signature as detail; other modules stay out.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///c.ting","version":2},"contentChanges":[{"text":"let l = import(\"../lib/list.ting\");\n"}]}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///c.ting"},"position":{"line":1,"character":0}}}"#,
    );
    let comp = recv(&mut reader);
    assert!(comp.contains("\"label\":\"median\""), "{comp}");
    assert!(comp.contains("lib/list.ting: median(xs)"), "{comp}");
    assert!(!comp.contains("\"label\":\"pad_left\""), "{comp}");

    // Hover on the name inside l["median"] shows the signature and
    // the function's leading comment; an unimported module's name
    // gets nothing.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///c.ting","version":3},"contentChanges":[{"text":"let l = import(\"lib/list.ting\");\nprint(l[\"median\"]([1]), pad_left);\n"}]}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":10,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///c.ting"},"position":{"line":1,"character":11}}}"#,
    );
    let hov = recv(&mut reader);
    assert!(hov.contains("median(xs)"), "{hov}");
    assert!(hov.contains("sorted values"), "{hov}");
    assert!(hov.contains("lib/list.ting"), "{hov}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":11,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///c.ting"},"position":{"line":1,"character":28}}}"#,
    );
    let hov = recv(&mut reader);
    assert!(hov.contains("\"result\":null"), "{hov}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":4,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn document_symbols_list_top_level_lets() {
    let (mut child, mut stdin, mut reader) = spawn_server();

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(init.contains("\"documentSymbolProvider\":true"), "{init}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///s.ting","text":"fn twice(x) { return x * 2; }\nlet limit = 10;\nprint(twice(limit));\n"}}}"#,
    );
    let _diags = recv(&mut reader);

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":"file:///s.ting"}}}"#,
    );
    let syms = recv(&mut reader);
    // fn sugar -> SymbolKind Function (12); plain let -> Variable (13).
    assert!(syms.contains("\"name\":\"twice\""), "{syms}");
    assert!(syms.contains("\"kind\":12"), "{syms}");
    assert!(syms.contains("\"name\":\"limit\""), "{syms}");
    assert!(syms.contains("\"kind\":13"), "{syms}");
    // The bare print(...) statement is not a symbol.
    assert!(!syms.contains("print"), "{syms}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn definition_jumps_to_top_level_binding() {
    let (mut child, mut stdin, mut reader) = spawn_server();

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(init.contains("\"definitionProvider\":true"), "{init}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///d.ting","text":"let limit = 10;\nprint(limit);\n"}}}"#,
    );
    let _diags = recv(&mut reader);

    // Cursor on the `limit` usage inside print(...) on line 1.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///d.ting"},"position":{"line":1,"character":8}}}"#,
    );
    let def = recv(&mut reader);
    assert!(def.contains("\"uri\":\"file:///d.ting\""), "{def}");
    assert!(def.contains("\"line\":0"), "{def}");

    // An identifier with no top-level binding resolves to null.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/definition","params":{"textDocument":{"uri":"file:///d.ting"},"position":{"line":1,"character":1}}}"#,
    );
    let none = recv(&mut reader);
    assert!(none.contains("\"result\":null"), "{none}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":4,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn references_list_every_occurrence() {
    let (mut child, mut stdin, mut reader) = spawn_server();

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(init.contains("\"referencesProvider\":true"), "{init}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///r.ting","text":"let count = 1;\ncount = count + 1;\nprint(count);\n"}}}"#,
    );
    let _diags = recv(&mut reader);

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{"textDocument":{"uri":"file:///r.ting"},"position":{"line":0,"character":5}}}"#,
    );
    let refs = recv(&mut reader);
    // Four occurrences of `count`: let, assign target, rhs use, print arg.
    assert_eq!(
        refs.matches("\"uri\":\"file:///r.ting\"").count(),
        4,
        "{refs}"
    );
    assert!(refs.contains("\"line\":2"), "{refs}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn rename_produces_a_workspace_edit() {
    let (mut child, mut stdin, mut reader) = spawn_server();

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(init.contains("\"renameProvider\":true"), "{init}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///n.ting","text":"let n = 1;\nprint(n + n);\n"}}}"#,
    );
    let _diags = recv(&mut reader);

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/rename","params":{"textDocument":{"uri":"file:///n.ting"},"position":{"line":0,"character":4},"newName":"total"}}"#,
    );
    let edit = recv(&mut reader);
    assert_eq!(edit.matches("\"newText\":\"total\"").count(), 3, "{edit}");
    assert!(edit.contains("\"changes\""), "{edit}");

    // An invalid identifier is rejected with a null result.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/rename","params":{"textDocument":{"uri":"file:///n.ting"},"position":{"line":0,"character":4},"newName":"9bad"}}"#,
    );
    let bad = recv(&mut reader);
    assert!(bad.contains("\"result\":null"), "{bad}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":4,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn signature_help_inside_a_builtin_call() {
    let (mut child, mut stdin, mut reader) = spawn_server();

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(init.contains("\"signatureHelpProvider\""), "{init}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///g.ting","text":"print(slice(\"abc\", 1, 2));\n"}}}"#,
    );
    let _diags = recv(&mut reader);

    // Cursor just after "slice(" — nested call resolves to slice, not print.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/signatureHelp","params":{"textDocument":{"uri":"file:///g.ting"},"position":{"line":0,"character":12}}}"#,
    );
    let sig = recv(&mut reader);
    assert!(sig.contains("slice("), "{sig}");
    assert!(sig.contains("\"activeSignature\":0"), "{sig}");

    // Outside any call: null.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/signatureHelp","params":{"textDocument":{"uri":"file:///g.ting"},"position":{"line":0,"character":0}}}"#,
    );
    let none = recv(&mut reader);
    assert!(none.contains("\"result\":null"), "{none}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":4,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}
