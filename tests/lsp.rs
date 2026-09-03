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

    // Hover on a user-defined function shows its signature from the
    // document; a plain variable still gets nothing.
    send(
        &mut stdin,
        r##"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///c.ting","version":9},"contentChanges":[{"text":"# Area of a rectangle.\nfn area(w, h) { return w * h; }\nlet side = 3;\nprint(area(side, 2));\n"}]}}"##,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":30,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///c.ting"},"position":{"line":3,"character":7}}}"#,
    );
    let hov = recv(&mut reader);
    assert!(
        hov.contains("fn area(w, h)")
            && hov.contains("Area of a rectangle.")
            && hov.contains("defined in this file"),
        "{hov}"
    );
    // Signature help inside area(...) comes from the same binding.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":32,"method":"textDocument/signatureHelp","params":{"textDocument":{"uri":"file:///c.ting"},"position":{"line":3,"character":11}}}"#,
    );
    let sig = recv(&mut reader);
    assert!(sig.contains("\"label\":\"area(w, h)\""), "{sig}");
    assert!(sig.contains("defined in this file"), "{sig}");
    // Completion offers the file's own function as a function (not a
    // plain identifier), with its signature and comment.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":33,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///c.ting"},"position":{"line":3,"character":0}}}"#,
    );
    let comp = recv(&mut reader);
    assert!(
        comp.contains(r#""detail":"fn area(w, h)","documentation":"Area of a rectangle.","kind":3,"label":"area""#),
        "{comp}"
    );
    assert!(!comp.contains(r#""kind":6,"label":"area""#), "{comp}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":31,"method":"textDocument/hover","params":{"textDocument":{"uri":"file:///c.ting"},"position":{"line":2,"character":12}}}"#,
    );
    let hov = recv(&mut reader);
    assert!(hov.contains("\"result\":null"), "{hov}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///c.ting","version":10},"contentChanges":[{"text":"let l = import(\"lib/list.ting\");\nprint(l[\"median\"]([1]), pad_left);\n"}]}}"#,
    );
    let _ = recv(&mut reader);

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

    // Signature help inside l["median"](...) resolves through the
    // module map to the stdlib signature.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":12,"method":"textDocument/signatureHelp","params":{"textDocument":{"uri":"file:///c.ting"},"position":{"line":1,"character":19}}}"#,
    );
    let sig = recv(&mut reader);
    assert!(sig.contains("\"label\":\"median(xs)\""), "{sig}");
    assert!(sig.contains("lib/list.ting"), "{sig}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":4,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn unknown_stdlib_member_is_a_warning() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let _ = recv(&mut reader);

    // A misspelt stdlib function is a warning at the key; a correct
    // one and a non-function export (test.ting's `state`) are silent.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///w.ting","text":"let l = import(\"lib/list.ting\");\nlet t = import(\"../lib/test.ting\");\nprint(l[\"medain\"]([1]), l[\"median\"]([2]), t[\"state\"]);\n"}}}"#,
    );
    let diag = recv(&mut reader);
    assert!(diag.contains("\"severity\":2"), "{diag}");
    assert!(
        diag.contains("lib/list.ting has no `medain` (did you mean `median`?)"),
        "{diag}"
    );
    // Only the misspelling is diagnosed: one message, and `median`
    // appears in it only as the suggestion.
    assert_eq!(diag.matches("\"severity\"").count(), 1, "{diag}");
    assert!(!diag.contains("`state`"), "{diag}");
    // The key itself is the range: line 2, character after `l["`.
    assert!(
        diag.contains("\"start\":{\"character\":9,\"line\":2}"),
        "{diag}"
    );

    // A quickfix replaces the misspelt key with the nearest export.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":10,"method":"textDocument/codeAction","params":{"textDocument":{"uri":"file:///w.ting"},"range":{"start":{"line":2,"character":0},"end":{"line":2,"character":40}}}}"#,
    );
    let act = recv(&mut reader);
    assert!(act.contains("\"kind\":\"quickfix\""), "{act}");
    assert!(act.contains("Replace with `median`"), "{act}");
    assert!(act.contains("\"newText\":\"median\""), "{act}");
    // Other lines: nothing to offer.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":11,"method":"textDocument/codeAction","params":{"textDocument":{"uri":"file:///w.ting"},"range":{"start":{"line":0,"character":0},"end":{"line":1,"character":0}}}}"#,
    );
    let act = recv(&mut reader);
    assert!(act.contains("\"result\":[]"), "{act}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///w.ting","version":2},"contentChanges":[{"text":"let l = import(\"lib/list.ting\");\nprint(l[\"median\"]([1]), l[\"zzqqxx\"]);\n"}]}}"#,
    );
    let diag = recv(&mut reader);
    assert!(diag.contains("has no `zzqqxx`"), "{diag}");
    // A name unlike any export gets the warning but no quickfix.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":12,"method":"textDocument/codeAction","params":{"textDocument":{"uri":"file:///w.ting"},"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":60}}}}"#,
    );
    let act = recv(&mut reader);
    assert!(act.contains("\"result\":[]"), "{act}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn folding_ranges_cover_multiline_braces() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(init.contains("\"foldingRangeProvider\":true"), "{init}");

    // fn body lines 0-4 with a nested if on lines 1-3; a one-line
    // block and a one-line map fold nothing.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///f.ting","text":"fn f(n) {\n  if n > 1 {\n    return n;\n  }\n}\nfn g() { return 1; }\nlet m = {\"a\": 1};\n"}}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/foldingRange","params":{"textDocument":{"uri":"file:///f.ting"}}}"#,
    );
    let fold = recv(&mut reader);
    assert!(
        fold.contains("\"endLine\":4,\"kind\":\"region\",\"startLine\":0"),
        "{fold}"
    );
    assert!(
        fold.contains("\"endLine\":3,\"kind\":\"region\",\"startLine\":1"),
        "{fold}"
    );
    assert_eq!(fold.matches("startLine").count(), 2, "{fold}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn workspace_symbols_span_open_documents() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(init.contains("\"workspaceSymbolProvider\":true"), "{init}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///a.ting","text":"fn total(xs) { return 1; }\nlet limit = 3;\n"}}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///b.ting","text":"let subtotal = 2;\n"}}}"#,
    );
    let _ = recv(&mut reader);

    // "total" matches total (a.ting) and subtotal (b.ting), not limit.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"workspace/symbol","params":{"query":"total"}}"#,
    );
    let syms = recv(&mut reader);
    assert!(
        syms.contains("\"name\":\"total\"") && syms.contains("file:///a.ting"),
        "{syms}"
    );
    assert!(
        syms.contains("\"name\":\"subtotal\"") && syms.contains("file:///b.ting"),
        "{syms}"
    );
    assert!(!syms.contains("limit"), "{syms}");
    // Empty query lists everything.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"workspace/symbol","params":{"query":""}}"#,
    );
    let all = recv(&mut reader);
    assert_eq!(all.matches("\"name\":").count(), 3, "{all}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":4,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn broken_local_import_is_an_error_on_the_import_string() {
    let dir = std::env::temp_dir().join(format!("ting-lsp-import-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub").join("b.ting"), "fn broken( {\n").unwrap();
    std::fs::write(dir.join("sub").join("ok.ting"), "fn f() { return 1; }\n").unwrap();
    let dir_text = dir.display().to_string().replace('\\', "/");
    let uri = if dir_text.starts_with('/') {
        format!("file://{dir_text}/a.ting")
    } else {
        format!("file:///{dir_text}/a.ting")
    };
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let _ = recv(&mut reader);
    let text =
        "let ok = import(\"./sub/ok.ting\");\nlet b = import(\"./sub/b.ting\");\nprint(ok, b);\n";
    let open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","text":"{}"}}}}}}"#,
        text.replace('"', "\\\"").replace('\n', "\\n")
    );
    send(&mut stdin, &open);
    let diag = recv(&mut reader);
    assert!(
        diag.contains("b.ting:1:12: expected parameter name") && diag.contains("\"severity\":1"),
        "{diag}"
    );
    // On the import string of the broken file (line 1), and only there.
    assert!(
        diag.contains(r#""start":{"character":15,"line":1}"#),
        "{diag}"
    );
    assert_eq!(diag.matches("\"severity\":1").count(), 1, "{diag}");
    assert!(!diag.contains("ok.ting"), "{diag}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn document_links_point_at_importable_files() {
    let dir = std::env::temp_dir().join(format!("ting-lsp-links-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub").join("b.ting"), "fn f() { return 1; }\n").unwrap();
    // A proper file: URI on every platform: forward slashes, and the
    // extra leading slash a Windows drive letter needs.
    let dir_text = dir.display().to_string().replace('\\', "/");
    let uri = if dir_text.starts_with('/') {
        format!("file://{dir_text}/a.ting")
    } else {
        format!("file:///{dir_text}/a.ting")
    };
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(init.contains("\"documentLinkProvider\""), "{init}");

    // One import resolves through a `..`-free relative path, one is an
    // embedded module with no file on disk, one does not exist.
    let text = "let b = import(\"./sub/b.ting\");\nlet l = import(\"lib/list.ting\");\nlet x = import(\"missing.ting\");\n";
    let open = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{uri}","text":"{}"}}}}}}"#,
        text.replace('"', "\\\"").replace('\n', "\\n")
    );
    send(&mut stdin, &open);
    let _ = recv(&mut reader);
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/documentLink","params":{{"textDocument":{{"uri":"{uri}"}}}}}}"#
    );
    send(&mut stdin, &req);
    let links = recv(&mut reader);
    assert!(links.contains("sub/b.ting\""), "{links}");
    assert_eq!(links.matches("\"target\"").count(), 1, "{links}");
    assert!(links.contains("\"line\":0"), "{links}");

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A frame whose body is not JSON is skipped, not fatal: the next
/// well-formed request still gets its answer.
#[test]
fn malformed_message_does_not_end_the_session() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(&mut stdin, "this is not json");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(init.contains("\"capabilities\""), "{init}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn unused_parameter_is_a_warning() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///p.ting","text":"fn f(a, b) { return a; }\nprint(f(1, 2));\n"}}}"#,
    );
    let diag = recv(&mut reader);
    assert!(
        diag.contains("parameter `b` is never used") && diag.contains("\"severity\":2"),
        "{diag}"
    );
    assert!(!diag.contains("`a`"), "{diag}");
    // The range is the parameter: line 0, character 8.
    assert!(
        diag.contains(r#""start":{"character":8,"line":0}"#),
        "{diag}"
    );
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn unused_local_let_is_a_warning() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///l.ting","text":"fn f() {\n  let stale = 1;\n  return 2;\n}\nprint(f());\n"}}}"#,
    );
    let diag = recv(&mut reader);
    assert!(
        diag.contains("`stale` is never used") && diag.contains("\"severity\":2"),
        "{diag}"
    );
    assert!(
        diag.contains(r#""start":{"character":6,"line":1}"#),
        "{diag}"
    );
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn shadowed_builtin_is_a_warning() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///s.ting","text":"let keys = [1];\nprint(keys);\n"}}}"#,
    );
    let diag = recv(&mut reader);
    assert!(
        diag.contains("`keys` shadows a builtin") && diag.contains("\"severity\":2"),
        "{diag}"
    );
    assert!(
        diag.contains(r#""start":{"character":4,"line":0}"#),
        "{diag}"
    );
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn unused_top_level_let_is_a_warning() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///u.ting","text":"let a = 1;\nlet b = 2;\nprint(a);\n"}}}"#,
    );
    let diag = recv(&mut reader);
    assert!(
        diag.contains("`b` is never used") && diag.contains("\"severity\":2"),
        "{diag}"
    );
    assert!(!diag.contains("`a`"), "{diag}");
    // The range is the binding's name: line 1, character 4.
    assert!(
        diag.contains("\"start\":{\"character\":4,\"line\":1}"),
        "{diag}"
    );
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///u.ting","version":2},"contentChanges":[{"text":"let a = 1;\nlet b = 2;\nprint(a + b);\n"}]}}"#,
    );
    let diag = recv(&mut reader);
    assert!(diag.contains("\"diagnostics\":[]"), "{diag}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":{}}"#,
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
    assert!(init.contains("\"codeActionProvider\":true"), "{init}");

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
fn formatting_edit_ends_at_the_documents_last_position() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let _ = recv(&mut reader);
    // Two lines and a trailing newline: the document ends at line 2,
    // character 0 — not line 3.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///f.ting","text":"let   x=1;\nprint( x );\n"}}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{"textDocument":{"uri":"file:///f.ting"},"options":{}}}"#,
    );
    let fmt = recv(&mut reader);
    assert!(fmt.contains(r#""end":{"character":0,"line":2}"#), "{fmt}");
    assert!(
        fmt.contains(r#""newText":"let x = 1;\nprint(x);\n""#),
        "{fmt}"
    );
    // Without a trailing newline the end is the last line's length.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///f.ting","version":2},"contentChanges":[{"text":"let   x=1;"}]}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/formatting","params":{"textDocument":{"uri":"file:///f.ting"},"options":{}}}"#,
    );
    let fmt = recv(&mut reader);
    assert!(fmt.contains(r#""end":{"character":10,"line":0}"#), "{fmt}");
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":4,"method":"shutdown","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit"}"#);
    assert_eq!(child.wait().unwrap().code(), Some(0));
}

#[test]
fn document_highlight_marks_binding_sites_as_writes() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let init = recv(&mut reader);
    assert!(
        init.contains("\"documentHighlightProvider\":true"),
        "{init}"
    );
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///h.ting","text":"let count = 1;\ncount = count + 1;\nprint(count);\nfn count() { return 0; }\n"}}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/documentHighlight","params":{"textDocument":{"uri":"file:///h.ting"},"position":{"line":2,"character":8}}}"#,
    );
    let hl = recv(&mut reader);
    // Five occurrences: the let and the fn are writes, the rest reads.
    assert_eq!(hl.matches("\"kind\":3").count(), 2, "{hl}");
    assert_eq!(hl.matches("\"kind\":2").count(), 3, "{hl}");
    assert!(hl.contains(r#""start":{"character":4,"line":0}"#), "{hl}");
    assert!(!hl.contains("\"uri\""), "{hl}");
    // No identifier under the cursor: null.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/documentHighlight","params":{"textDocument":{"uri":"file:///h.ting"},"position":{"line":0,"character":12}}}"#,
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
fn prepare_rename_offers_the_identifier_or_declines() {
    let (mut child, mut stdin, mut reader) = spawn_server();
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///p.ting","text":"let total = len([1]);\nprint(total);\n"}}}"#,
    );
    let _ = recv(&mut reader);
    // On the binding: its range and its name as the placeholder.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/prepareRename","params":{"textDocument":{"uri":"file:///p.ting"},"position":{"line":0,"character":6}}}"#,
    );
    let ok = recv(&mut reader);
    assert!(
        ok.contains(r#""placeholder":"total""#)
            && ok.contains(r#""start":{"character":4,"line":0}"#)
            && ok.contains(r#""end":{"character":9,"line":0}"#),
        "{ok}"
    );
    // On a builtin (len) and on a keyword (let): null, so the editor
    // declines before prompting.
    for (id, ch) in [(3, 13), (4, 1)] {
        send(
            &mut stdin,
            &format!(
                r#"{{"jsonrpc":"2.0","id":{id},"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"file:///p.ting"}},"position":{{"line":0,"character":{ch}}}}}}}"#
            ),
        );
        let no = recv(&mut reader);
        assert!(no.contains("\"result\":null"), "{no}");
    }
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":5,"method":"shutdown","params":{}}"#,
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
    assert!(
        init.contains("\"renameProvider\":{\"prepareProvider\":true}"),
        "{init}"
    );

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

    // A second open document using the same name is renamed too; one
    // that does not mention it gets no change list.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///m.ting","text":"let m = n * 2;\n"}}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///o.ting","text":"let other = 1;\n"}}}"#,
    );
    let _ = recv(&mut reader);
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":20,"method":"textDocument/rename","params":{"textDocument":{"uri":"file:///n.ting"},"position":{"line":0,"character":4},"newName":"total"}}"#,
    );
    let edit = recv(&mut reader);
    assert_eq!(edit.matches("\"newText\":\"total\"").count(), 4, "{edit}");
    assert!(
        edit.contains("file:///m.ting") && !edit.contains("file:///o.ting"),
        "{edit}"
    );

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
