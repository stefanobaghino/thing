//! Interactive REPL: std-only, persistent session, multi-line input.

use crate::diag;
use crate::eval::Interpreter;
use crate::lexer;
use crate::parser;
use crate::value::Value;
use std::io::{BufRead, IsTerminal, Write};
use std::process::ExitCode;

/// What became of one accumulated input chunk.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Syntactically unfinished — keep reading lines.
    Incomplete,
    /// Ran fine, nothing to echo.
    Unit,
    /// A bare expression: its rendered value.
    Value(String),
    Error(String),
}

/// Evaluate one chunk of REPL input against a live interpreter.
///
/// A chunk that parses as a single expression is echoed; otherwise it runs
/// as statements. A parse that dies at end-of-input first retries with a
/// `;` appended (so `let x = 1` works), then reports Incomplete to request
/// another line.
pub fn eval_chunk<W: Write>(interp: &mut Interpreter<W>, src: &str) -> Outcome {
    eval_chunk_at(interp, "repl", src)
}

/// `eval_chunk` with the name diagnostics are rendered under: "repl"
/// for a typed chunk, the file's path for a `:load`.
pub fn eval_chunk_at<W: Write>(interp: &mut Interpreter<W>, path: &str, src: &str) -> Outcome {
    // Each line is its own source: what `try` names when a failure
    // happens in it.
    interp.set_source(path, src);
    let tokens = match lexer::lex(src) {
        Ok(t) => t,
        Err(e) => return Outcome::Error(diag::render(path, src, &e.message, e.span)),
    };
    if let Ok(expr) = parser::parse_expr(&tokens) {
        return match interp.eval(&expr) {
            Ok(Value::Nil) => Outcome::Unit,
            Ok(v) => Outcome::Value(render(&v)),
            Err(e) => Outcome::Error(e.render(path, src)),
        };
    }
    match parser::parse_program(&tokens) {
        Ok(prog) => run_program(interp, path, src, &prog),
        Err(e) if e.message.ends_with("found end of input") => {
            if let Ok(t2) = lexer::lex(&format!("{src};"))
                && let Ok(prog) = parser::parse_program(&t2)
            {
                return run_program(interp, path, src, &prog);
            }
            Outcome::Incomplete
        }
        Err(e) => Outcome::Error(diag::render(path, src, &e.message, e.span)),
    }
}

fn run_program<W: Write>(
    interp: &mut Interpreter<W>,
    path: &str,
    src: &str,
    prog: &[crate::ast::Stmt],
) -> Outcome {
    match interp.run(prog) {
        Ok(()) => Outcome::Unit,
        Err(e) => Outcome::Error(e.render(path, src)),
    }
}

/// How much of a value the prompt shows before it says how much is
/// left. A value is echoed to be looked at, and one that runs past a
/// screenful is not being looked at — it is scrolling: `range(100000)`
/// at the prompt is 688891 characters, and what it was is gone from
/// the session along with everything above it.
const ECHO_LIMIT: usize = 2000;

/// A binding is one line of `:vars`, which is a list to run an eye
/// down: a value too wide for a line is cut to fit and says how long
/// it really is. Typing the name then shows the value itself, up to
/// the ceiling above.
const VARS_LIMIT: usize = 60;

/// Echoed values quote strings, so `"a" + "b"` shows as `"ab"`.
fn render_text(v: &Value) -> String {
    match v {
        Value::Str(s) => format!("{s:?}"),
        v => v.to_string(),
    }
}

/// What `:vars` puts after the name: the value, on one line.
fn render_line(v: &Value) -> String {
    let text = render_text(v);
    let total = text.chars().count();
    if total <= VARS_LIMIT {
        return text;
    }
    let shown: String = text.chars().take(VARS_LIMIT).collect();
    format!("{shown}… ({total} characters)")
}

/// A value as the prompt echoes it: whole while it can be read, and
/// cut with a note when it cannot. The cut is the prompt's alone:
/// `print(x)` writes the whole value, here as in a script, and
/// nothing a program prints passes through this.
fn render(v: &Value) -> String {
    let text = render_text(v);
    let total = text.chars().count();
    if total <= ECHO_LIMIT {
        return text;
    }
    let shown: String = text.chars().take(ECHO_LIMIT).collect();
    format!("{shown}…\n({ECHO_LIMIT} of {total} characters; print() writes all of it)")
}

pub fn run() -> ExitCode {
    // Same big-stack thread as script execution (deep ting recursion),
    // and the same declaration, so a REPL session and a script refuse
    // to recurse at exactly the same depth.
    const STACK: usize = 32 * 1024 * 1024;
    crate::eval::set_stack_budget(STACK);
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(run_inner)
        .expect("failed to spawn REPL thread")
        .join()
        .expect("REPL thread panicked")
}

/// All REPL output goes through here so a reader that went away
/// (`echo ':help' | ting | head`) ends the session quietly instead of
/// panicking inside the standard print macros, matching what print()
/// does in scripts.
fn emit(text: &str) {
    let mut out = std::io::stdout().lock();
    if let Err(e) = out.write_all(text.as_bytes()) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("ting: {e}");
        std::process::exit(1);
    }
}

pub fn say(text: &str) {
    emit(text);
    emit("\n");
}

/// `:doc NAME` — a builtin's signature and doc line, or a stdlib
/// function's module, signature and leading comment (every embedded
/// module is searched, imported or not). A word that names nothing is
/// SEARCHED for, and a function is followed by what else its name
/// finds, exactly as `--doc` does: the same question typed two ways
/// has to give the same answer, and a module index here is already
/// forty lines, so the length of a search is nothing new.
fn print_doc(name: &str) {
    if let Some(text) = doc_text(name) {
        say(&text);
        if let Some(more) = doc_mentions(name, Some(name)) {
            say("");
            say("also mentioned by:");
            say(&more);
        }
        return;
    }
    if let Some(text) = doc_index(Some(name)) {
        say(&text);
        return;
    }
    if let Some(found) = doc_search(name, None) {
        say(&format!("matching {name}:"));
        say(&found);
        return;
    }
    let names = doc_names();
    match crate::diag::nearest(name, names.iter().map(String::as_str)) {
        Some(near) => say(&format!(
            "(no builtin, stdlib function or module matches {name}; did you mean {near}?)"
        )),
        None => say(&format!(
            "(no builtin, stdlib function or module matches {name})"
        )),
    }
}

/// The documentation for a builtin (signature, doc line) or a stdlib
/// function (signature, module, leading comment — every embedded
/// module searched; a name in several modules lists all), or None.
/// Shared by the REPL's :doc and the CLI's --doc.
/// COLUMNS every doc line is kept within, so nothing runs past an
/// 80-column terminal — measured the way a terminal measures, where
/// an ideograph takes two and a combining mark none.
const DOC_WIDTH: usize = 78;

/// `text` word-wrapped to DOC_WIDTH columns, every line prefixed with
/// `indent` spaces; an empty text gives no lines.
fn wrap_indented(text: &str, indent: usize) -> Vec<String> {
    let pad = " ".repeat(indent);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let (have, next) = (crate::width::width(&line), crate::width::width(word));
        if !line.is_empty() && indent + have + 1 + next > DOC_WIDTH {
            lines.push(format!("{pad}{line}"));
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(format!("{pad}{line}"));
    }
    lines
}

/// The module whose short name is this word. Two of the thirteen
/// collide with something that answers first — `map` is the builtin
/// map(xs, f) and also lib/map.ting, `args` the builtin args() and
/// also lib/args.ting — so the answer about the builtin has to say
/// the module is there, or the module is unreachable by the name a
/// reader would try (908).
fn module_named(name: &str) -> Option<&'static str> {
    crate::eval::embedded_stdlib()
        .iter()
        .map(|(path, _)| *path)
        .find(|path| path.trim_start_matches("lib/").trim_end_matches(".ting") == name)
}

/// The pointer to that module, appended to whatever answered first.
fn also_a_module(out: &mut Vec<String>, name: &str) {
    if let Some(path) = module_named(name) {
        out.push(String::new());
        out.push(format!(
            "({path} is a module of the same name: --doc {path})"
        ));
    }
}

pub fn doc_text(name: &str) -> Option<String> {
    if let Some(b) = crate::value::Builtin::ALL.iter().find(|b| b.name() == name) {
        let (sig, text) = b.doc();
        let mut out = vec![sig.to_string()];
        out.extend(wrap_indented(text, 2));
        also_a_module(&mut out, name);
        return Some(out.join("\n"));
    }
    // A source that imports every module makes the LSP's scanner
    // return all stdlib functions.
    let everything: String = crate::eval::embedded_stdlib()
        .iter()
        .map(|(path, _)| format!("import(\"{path}\");\n"))
        .collect();
    let hits: Vec<_> = crate::lsp::imported_stdlib_functions(&everything)
        .into_iter()
        .filter(|(_, n, _, _)| n == name)
        .collect();
    if hits.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for (path, _, sig, comment) in hits {
        out.push(format!("{sig}  [{path}]"));
        out.extend(wrap_indented(&comment, 2));
    }
    also_a_module(&mut out, name);
    Some(out.join("\n"))
}

/// Every name `--doc` and `:doc` answer to: the builtins, the stdlib
/// functions, and each module under both its short name and its path.
/// The candidate list for a suggestion when a name is not one of them.
pub fn doc_names() -> Vec<String> {
    let mut out: Vec<String> = crate::value::Builtin::ALL
        .iter()
        .map(|b| b.name().to_string())
        .collect();
    let everything: String = crate::eval::embedded_stdlib()
        .iter()
        .map(|(path, _)| format!("import(\"{path}\");\n"))
        .collect();
    out.extend(
        crate::lsp::imported_stdlib_functions(&everything)
            .into_iter()
            .map(|(_, name, _, _)| name),
    );
    for (path, _) in crate::eval::embedded_stdlib() {
        out.push((*path).to_string());
        out.push(
            path.trim_start_matches("lib/")
                .trim_end_matches(".ting")
                .to_string(),
        );
    }
    out
}

/// `--doc` with no name (a table of contents: every builtin, then
/// every stdlib function grouped by module) or with a module name
/// (`list` or `lib/list.ting`: that module's members). One line per
/// function: the signature, then the first sentence of its comment.
/// None when the module does not exist.
pub fn doc_index(module: Option<&str>) -> Option<String> {
    let mut out = Vec::new();
    if module.is_none() {
        out.push("builtins:".to_string());
        let mut docs: Vec<_> = crate::value::Builtin::ALL.iter().map(|b| b.doc()).collect();
        docs.sort();
        for (sig, text) in docs {
            out.push(member_line(sig, text));
        }
    }
    let everything: String = crate::eval::embedded_stdlib()
        .iter()
        .map(|(path, _)| format!("import(\"{path}\");\n"))
        .collect();
    let all = crate::lsp::imported_stdlib_functions(&everything);
    let mut found = false;
    for (path, _) in crate::eval::embedded_stdlib() {
        let short = path.trim_start_matches("lib/").trim_end_matches(".ting");
        if let Some(m) = module
            && m != *path
            && m != short
        {
            continue;
        }
        found = true;
        if !out.is_empty() {
            out.push(String::new());
        }
        let source = crate::eval::embedded_stdlib()
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, src)| crate::lsp::source_header(src))
            .unwrap_or_default();
        // Asked for by name, the module answers with what it is for,
        // in full; in the table of contents that would be a wall, so
        // there it is the first sentence on the module's own line.
        if module.is_some() {
            out.push(format!("{path}:"));
            out.extend(header_lines(&source));
        } else {
            out.push(module_line(path, &source));
        }
        for (p, _, sig, comment) in &all {
            if p != path {
                continue;
            }
            out.push(member_line(sig, comment));
        }
    }
    if !found {
        return None;
    }
    Some(out.join("\n"))
}

/// One index entry: the signature, then the first sentence of the
/// comment — on the same line when it fits in DOC_WIDTH, otherwise
/// wrapped underneath, indented further than the signature.
fn member_line(sig: &str, comment: &str) -> String {
    let first = match comment.find(". ") {
        Some(i) => &comment[..=i],
        None => comment,
    };
    if first.is_empty() {
        return format!("  {sig}");
    }
    let one = format!("  {sig}  {first}");
    if crate::width::width(&one) <= DOC_WIDTH {
        return one;
    }
    let mut lines = vec![format!("  {sig}")];
    lines.extend(wrap_indented(first, 6));
    lines.join("\n")
}

/// The first sentence of a module's header comment — its one-line
/// answer to "what is this for". None when the file has no header.
fn module_summary(header: &[String]) -> Option<String> {
    let joined = header
        .iter()
        .filter(|l| !l.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if joined.is_empty() {
        return None;
    }
    Some(match joined.find(". ") {
        Some(i) => joined[..=i].to_string(),
        // No sentence break to cut at: the first line, rather than a
        // header's worked examples run together on one line.
        None => header.first()?.clone(),
    })
}

/// A module's line in the index: its path, then that one-line answer
/// where there is one — on the same line when it fits, wrapped
/// underneath otherwise, indented past where a member would start.
fn module_line(path: &str, header: &[String]) -> String {
    let Some(summary) = module_summary(header) else {
        return format!("{path}:");
    };
    let one = format!("{path}: {summary}");
    if one.len() <= DOC_WIDTH {
        return one;
    }
    let mut lines = vec![format!("{path}:")];
    lines.extend(wrap_indented(&summary, 4));
    lines.join("\n")
}

/// A header printed above a module's members: every line indented,
/// its own shape kept, and a blank line before the list starts.
fn header_lines(header: &[String]) -> Vec<String> {
    let mut out: Vec<String> = header
        .iter()
        .map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("  {l}")
            }
        })
        .collect();
    if !out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Every entry `--doc` can print: the module path (empty for a
/// builtin), the name, the signature and the comment.
fn doc_entries() -> Vec<(&'static str, String, String, String)> {
    let mut out: Vec<(&'static str, String, String, String)> = crate::value::Builtin::ALL
        .iter()
        .map(|b| {
            let (sig, text) = b.doc();
            ("", b.name().to_string(), sig.to_string(), text.to_string())
        })
        .collect();
    let everything: String = crate::eval::embedded_stdlib()
        .iter()
        .map(|(path, _)| format!("import(\"{path}\");\n"))
        .collect();
    out.extend(crate::lsp::imported_stdlib_functions(&everything));
    out
}

/// Whether a query finds an entry. A name matches on any substring,
/// so `sort` finds `sort_with`. A comment matches only where a WORD
/// of it starts with the query, so `len` finds "length" and not
/// "silently", and `sort` finds "sorted" and "sorting".
fn doc_matches(query: &str, name: &str, comment: &str) -> bool {
    if name.to_lowercase().contains(query) {
        return true;
    }
    comment
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| word.starts_with(query))
}

/// `--doc TEXT` and `:doc TEXT` searching for TEXT rather than
/// looking it up: every entry whose name or comment matches, grouped
/// and formatted the way the index is. `skip` is a name already
/// printed in full, so an exact hit is not repeated underneath
/// itself. None when nothing matches.
pub fn doc_search(query: &str, skip: Option<&str>) -> Option<String> {
    let hits = doc_hits(query, skip);
    let mut out = Vec::new();
    for (path, entries) in hits {
        if !out.is_empty() {
            out.push(String::new());
        }
        out.push(format!(
            "{}:",
            if path.is_empty() { "builtins" } else { path }
        ));
        out.extend(entries.into_iter().map(|(_, line)| line));
    }
    (!out.is_empty()).then(|| out.join("\n"))
}

/// The same search, as names rather than entries: what else mentions
/// a word, for a reader who already has the answer to the word
/// itself. `--doc map` spelled out forty-four entries under the two
/// lines that answered the question; the names are the part that says
/// where else to look, and `--doc NAME` is how to look.
pub fn doc_mentions(query: &str, skip: Option<&str>) -> Option<String> {
    let hits = doc_hits(query, skip);
    let mut out = Vec::new();
    for (path, entries) in hits {
        let names: Vec<String> = entries.into_iter().map(|(name, _)| name).collect();
        out.push(format!(
            "  {}: {}",
            if path.is_empty() { "builtins" } else { path },
            names.join(", ")
        ));
    }
    (!out.is_empty()).then(|| out.join("\n"))
}

/// Every entry a query finds, grouped by where it lives: builtins
/// first under the empty path, then each module. Each entry is its
/// name and the line an index would print for it, so a caller can
/// show either.
fn doc_hits(query: &str, skip: Option<&str>) -> Vec<(&'static str, Vec<(String, String)>)> {
    let query = query.to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let mut builtins: Vec<(String, String)> = Vec::new();
    let mut modules: Vec<(&'static str, Vec<(String, String)>)> = Vec::new();
    for (path, name, sig, comment) in doc_entries() {
        if Some(name.as_str()) == skip || !doc_matches(&query, &name, &comment) {
            continue;
        }
        let entry = (name, member_line(&sig, &comment));
        if path.is_empty() {
            builtins.push(entry);
        } else if let Some((_, entries)) = modules.iter_mut().find(|(p, _)| *p == path) {
            entries.push(entry);
        } else {
            modules.push((path, vec![entry]));
        }
    }
    builtins.sort();
    let mut out = Vec::new();
    if !builtins.is_empty() {
        out.push(("", builtins));
    }
    out.extend(modules);
    out
}

/// `--doc FILE.ting` — the file's own top-level functions, one line
/// each, the way a stdlib module is listed. None when the file cannot
/// be read.
pub fn doc_file(path: &str) -> Option<String> {
    let source = std::fs::read_to_string(path).ok()?;
    let mut out = vec![format!("{path}:")];
    out.extend(header_lines(&crate::lsp::source_header(&source)));
    for (_, sig, comment) in crate::lsp::source_functions(&source) {
        out.push(member_line(&sig, &comment));
    }
    Some(out.join("\n"))
}

/// Pairs laid out as two columns, the first padded to the widest
/// entry. COLUMNS, and padded by hand: Rust's own `{:width$}` counts
/// characters, which is the same answer only while every signature is
/// ASCII.
fn two_columns(pairs: &[(&str, &str)]) -> Vec<String> {
    let width = pairs
        .iter()
        .map(|(sig, _)| crate::width::width(sig))
        .max()
        .unwrap_or(0);
    pairs
        .iter()
        .map(|(sig, text)| {
            let pad = " ".repeat(width - crate::width::width(sig));
            format!("{sig}{pad}  {text}")
        })
        .collect()
}

/// `:help` — every builtin's signature and one-liner, in name order.
fn print_help() {
    let mut docs: Vec<_> = crate::value::Builtin::ALL.iter().map(|b| b.doc()).collect();
    docs.sort();
    for line in two_columns(&docs) {
        say(&line);
    }
    say(
        "(:doc NAME explains a builtin or stdlib function, :doc MODULE lists a module, :doc alone lists everything; :vars bindings; :load <file> runs a file here; :time EXPR evaluates and reports milliseconds; :fmt reprints the last chunk formatted; :history lists the chunks that ran without error; :save <file> writes them as a script; :clear resets; ctrl-d exits)",
    );
}

fn run_inner() -> ExitCode {
    let stdin = std::io::stdin();
    let tty = stdin.is_terminal();
    if tty {
        say(&format!(
            "ting {} — :help, :doc NAME, :vars, :load <file>, :time EXPR, :fmt, :history, :save <file>, :clear",
            env!("CARGO_PKG_VERSION")
        ));
    }
    let mut interp = Interpreter::new(std::io::stdout());
    let mut buffer = String::new();
    // The last complete chunk that was evaluated, for :fmt.
    let mut last = String::new();
    // Every chunk that evaluated without error, in order: the session
    // transcript behind :history (and :save).
    let mut history: Vec<String> = Vec::new();
    loop {
        if tty {
            emit(if buffer.is_empty() { "> " } else { ".. " });
            std::io::stdout().flush().ok();
        }
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                if tty {
                    say("");
                }
                return ExitCode::SUCCESS;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("ting: {e}");
                return ExitCode::FAILURE;
            }
        }
        if !buffer.is_empty() && line.trim().is_empty() {
            buffer.clear();
            continue;
        }
        // Meta-commands: only at the start of a fresh chunk.
        if buffer.is_empty() && line.trim() == ":help" {
            print_help();
            continue;
        }
        if buffer.is_empty() && line.trim() == ":clear" {
            interp = Interpreter::new(std::io::stdout());
            history.clear();
            say("(session cleared)");
            continue;
        }
        if buffer.is_empty()
            && let Some(path) = line.trim().strip_prefix(":save ")
        {
            // The transcript as a runnable script: chunks in order, a
            // blank line between them, so `ting FILE` replays the session.
            let path = path.trim();
            if history.is_empty() {
                say("(nothing to save yet)");
                continue;
            }
            let script: String = history
                .iter()
                .map(|c| c.trim_end().to_string())
                .collect::<Vec<_>>()
                .join("\n\n")
                + "\n";
            match std::fs::write(path, script) {
                Ok(()) => say(&format!("(saved {} chunk(s) to {path})", history.len())),
                Err(e) => eprintln!("ting: cannot write {path:?}: {e}"),
            }
            continue;
        }
        if buffer.is_empty() && line.trim() == ":history" {
            if history.is_empty() {
                say("(nothing evaluated yet)");
            }
            for (i, chunk) in history.iter().enumerate() {
                let mut lines = chunk.trim_end().lines();
                if let Some(first) = lines.next() {
                    say(&format!("{:>3}  {first}", i + 1));
                }
                for rest in lines {
                    say(&format!("     {rest}"));
                }
            }
            continue;
        }
        if buffer.is_empty() && line.trim() == ":doc" {
            // The table of contents, as `ting --doc` prints it.
            say(&doc_index(None).unwrap_or_default());
            continue;
        }
        if buffer.is_empty()
            && let Some(name) = line.trim().strip_prefix(":doc ")
        {
            print_doc(name.trim());
            continue;
        }
        if buffer.is_empty() && line.trim() == ":fmt" {
            if last.is_empty() {
                say("(nothing to format yet)");
            } else {
                match crate::fmt::format(&last) {
                    Ok(formatted) => emit(&formatted),
                    Err(e) => eprintln!("{}", diag::render("repl", &last, &e.message, e.span)),
                }
            }
            continue;
        }
        if buffer.is_empty() && line.trim() == ":vars" {
            let bindings = interp.user_bindings();
            if bindings.is_empty() {
                say("(no bindings yet)");
            }
            for (name, value) in bindings {
                say(&format!("{name}: {}", render_line(&value)));
            }
            continue;
        }
        if buffer.is_empty()
            && let Some(src) = line.trim().strip_prefix(":time ")
        {
            // One-line chunk, timed: the value (if any) then the elapsed
            // wall-clock milliseconds on its own line.
            let started = std::time::Instant::now();
            let outcome = eval_chunk(&mut interp, src);
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            match outcome {
                Outcome::Incomplete => eprintln!("ting: :time needs a complete expression"),
                Outcome::Unit => {}
                Outcome::Value(v) => say(&v.to_string()),
                Outcome::Error(msg) => eprintln!("{msg}"),
            }
            say(&format!("({ms:.1} ms)"));
            continue;
        }
        if buffer.is_empty()
            && let Some(path) = line.trim().strip_prefix(":load ")
        {
            let path = path.trim();
            match crate::diag::read_text(path) {
                Ok(src) => {
                    // The loaded file's relative imports resolve against
                    // its own directory, as they would under `ting FILE`;
                    // the session's base comes back afterwards.
                    let saved = interp.base_dir();
                    if let Some(dir) = std::path::Path::new(path).parent() {
                        interp.set_base_dir(dir.to_path_buf());
                    }
                    let before = interp.user_bindings().len();
                    let outcome = eval_chunk_at(&mut interp, path, &src);
                    interp.set_base_dir(saved);
                    let loaded = !matches!(outcome, Outcome::Error(_) | Outcome::Incomplete);
                    match outcome {
                        Outcome::Incomplete => eprintln!("ting: {path}: incomplete program"),
                        Outcome::Unit => {}
                        Outcome::Value(v) => say(&v.to_string()),
                        Outcome::Error(msg) => eprintln!("{msg}"),
                    }
                    // What the load added to the session, so a file of
                    // definitions is not loaded in silence.
                    if loaded {
                        let added = interp.user_bindings().len().saturating_sub(before);
                        say(&format!("(loaded {path}: {added} new binding(s))"));
                    }
                }
                Err(why) => eprintln!("ting: cannot read {path:?}: {why}"),
            }
            continue;
        }
        buffer.push_str(&line);
        if buffer.trim().is_empty() {
            buffer.clear();
            continue;
        }
        let ok = match eval_chunk(&mut interp, &buffer) {
            Outcome::Incomplete => continue,
            Outcome::Unit => true,
            Outcome::Value(s) => {
                say(&s);
                true
            }
            Outcome::Error(msg) => {
                eprintln!("{msg}");
                false
            }
        };
        last = std::mem::take(&mut buffer);
        if ok {
            history.push(last.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Interpreter<Vec<u8>> {
        Interpreter::new(Vec::new())
    }

    /// The two columns line up on a terminal, which means the first
    /// one is measured in columns: an ideograph takes two of them and
    /// a combining accent none.
    #[test]
    fn the_signature_column_is_as_wide_as_a_terminal_makes_it() {
        let pairs = [
            ("\u{65e5}()", "wide"),
            ("abcd()", "plain"),
            ("e\u{301}()", "mark"),
        ];
        let lines = two_columns(&pairs);
        // The widest signature is six columns, so every second column
        // starts at the eighth.
        let starts: Vec<_> = lines
            .iter()
            .zip(pairs)
            .map(|(l, (_, text))| crate::width::width(l) - crate::width::width(text))
            .collect();
        assert_eq!(starts, vec![8, 8, 8], "{lines:?}");
        assert_eq!(lines[0], "\u{65e5}()    wide");
        assert_eq!(lines[2], "e\u{301}()     mark");
    }

    /// A doc line is wrapped to fit a terminal, so it packs by
    /// columns: measured in bytes, wide text would break a third of
    /// the way across the page.
    #[test]
    fn doc_text_wraps_on_columns() {
        let word = "\u{65e5}\u{672c}\u{8a9e}"; // six columns, nine bytes
        let text = vec![word; 30].join(" ");
        let lines = wrap_indented(&text, 6);
        for l in &lines {
            assert!(crate::width::width(l) <= DOC_WIDTH, "{l}");
        }
        // Ten words fit: 6 of indent, then ten sixes and nine spaces.
        assert_eq!(lines[0].split_whitespace().count(), 10, "{lines:?}");
        assert_eq!(crate::width::width(&lines[0]), 75, "{lines:?}");
    }

    #[test]
    fn expressions_echo_their_value() {
        let mut i = fresh();
        assert_eq!(eval_chunk(&mut i, "1 + 2"), Outcome::Value("3".into()));
        assert_eq!(
            eval_chunk(&mut i, "\"a\" + \"b\""),
            Outcome::Value("\"ab\"".into())
        );
        assert_eq!(eval_chunk(&mut i, "{}"), Outcome::Value("{}".into()));
    }

    #[test]
    fn nil_expressions_do_not_echo() {
        let mut i = fresh();
        assert_eq!(eval_chunk(&mut i, "nil"), Outcome::Unit);
        assert_eq!(eval_chunk(&mut i, "print(7)"), Outcome::Unit);
    }

    #[test]
    fn state_persists_across_chunks() {
        let mut i = fresh();
        assert_eq!(eval_chunk(&mut i, "let x = 40;"), Outcome::Unit);
        assert_eq!(eval_chunk(&mut i, "x = x + 2;"), Outcome::Unit);
        assert_eq!(eval_chunk(&mut i, "x"), Outcome::Value("42".into()));
    }

    #[test]
    fn missing_final_semicolon_is_forgiven() {
        let mut i = fresh();
        assert_eq!(eval_chunk(&mut i, "let y = 1"), Outcome::Unit);
        assert_eq!(eval_chunk(&mut i, "y"), Outcome::Value("1".into()));
    }

    #[test]
    fn open_constructs_request_more_input() {
        let mut i = fresh();
        assert_eq!(eval_chunk(&mut i, "fn f() {"), Outcome::Incomplete);
        assert_eq!(eval_chunk(&mut i, "if true {"), Outcome::Incomplete);
        assert_eq!(eval_chunk(&mut i, "1 +"), Outcome::Incomplete);
    }

    #[test]
    fn completed_multiline_function_runs() {
        let mut i = fresh();
        let chunk = "fn double(x) {\n  return x * 2;\n}";
        assert_eq!(eval_chunk(&mut i, chunk), Outcome::Unit);
        assert_eq!(
            eval_chunk(&mut i, "double(21)"),
            Outcome::Value("42".into())
        );
    }

    #[test]
    fn errors_are_rendered_with_carets_and_session_survives() {
        let mut i = fresh();
        assert_eq!(
            eval_chunk(&mut i, "xyz"),
            Outcome::Error("repl:1:1: error: undefined variable 'xyz'\n 1 | xyz\n   | ^^^".into())
        );
        match eval_chunk(&mut i, "1 = 2;") {
            Outcome::Error(msg) => assert!(msg.contains("invalid assignment target")),
            other => panic!("expected error, got {other:?}"),
        }
        assert_eq!(eval_chunk(&mut i, "2 + 2"), Outcome::Value("4".into()));
    }

    #[test]
    fn multiline_chunk_errors_point_at_the_right_line() {
        let mut i = fresh();
        match eval_chunk(&mut i, "fn f() {\n  return zzz;\n}\nf()") {
            Outcome::Error(msg) => {
                assert!(
                    msg.starts_with("repl:2:10: error: undefined variable 'zzz'"),
                    "got:\n{msg}"
                );
                assert!(msg.contains("  return zzz;"), "got:\n{msg}");
            }
            other => panic!("expected error, got {other:?}"),
        }
    }

    #[test]
    fn prints_go_to_the_interpreter_writer() {
        let mut i = fresh();
        eval_chunk(&mut i, "print(\"hi\");");
        assert_eq!(String::from_utf8(i.into_out()).unwrap(), "hi\n");
    }
}
