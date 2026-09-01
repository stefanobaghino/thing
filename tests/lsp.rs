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

    // Unknown request gets a MethodNotFound error.
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":9,"method":"textDocument/hover","params":{}}"#,
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
