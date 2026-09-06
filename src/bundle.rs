//! `--bundle`: a script and the local modules it imports, written out
//! as one file that runs the same way.
//!
//! Two facts about modules are what make this possible, and both were
//! measured before this file was written. A module is a map of what
//! its top level declares (the rule import_module follows), so a
//! module body can become a function that returns that map. And
//! importing the same file twice hands back the *same* map, so the
//! bundle binds each module once and every import site reads that one
//! binding — pasting a module per import site would give a program
//! that behaves differently the moment a module holds state.
//!
//! An import is inlined when its path names a file and left alone
//! when it does not, which is the order the interpreter resolves in:
//! filesystem first, and what has no file is a module embedded in the
//! binary. So `import("lib/list.ting")` normally stays — the binary
//! answers it, which is the whole reason one file is enough — while a
//! copy of that module sitting beside the script is inlined like any
//! other local module. Either way the bundle runs what the script
//! ran.
//!
//! One difference the bundle cannot hide: a module runs in a fresh
//! global environment, so a name it never defines is unbound there,
//! while inside the bundle the same name could find one of the
//! script's globals. That only separates programs that were already
//! erroring, and the bundler says nothing about it.

use crate::lexer;
use std::path::{Path, PathBuf};

/// The name a bundled module is bound to. Long and dull on purpose:
/// it shares a scope with the script's own top-level names.
fn binding(n: usize) -> String {
    format!("__ting_module_{n}")
}

/// `dir`-relative `path` the way the interpreter resolves it: absolute
/// paths as given, everything else joined onto the importing file's
/// own directory, then canonicalised so two spellings of one file are
/// one file (which is how the import cache decides identity too).
fn resolve(dir: &Path, path: &str) -> PathBuf {
    let raw = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        dir.join(path)
    };
    raw.canonicalize().unwrap_or(raw)
}

/// One `import("...")` call: the byte range of the whole call, and the
/// file it names when that file exists. No file means the embedded
/// standard library (or an import that will fail at run time, which is
/// the run's business and not the bundler's) — either way, left alone.
struct Import {
    range: std::ops::Range<usize>,
    target: Option<PathBuf>,
}

/// Every `import(...)` call in `src`. An import whose argument is not
/// a literal string is refused rather than guessed at: the bundler
/// cannot follow a path it will not see until the program runs.
fn imports_of(display: &str, src: &str, dir: &Path) -> Result<Vec<Import>, String> {
    let tokens =
        lexer::lex(src).map_err(|e| crate::diag::render(display, src, &e.message, e.span))?;
    let mut out = Vec::new();
    for i in 0..tokens.len() {
        let lexer::TokenKind::Ident(name) = &tokens[i].kind else {
            continue;
        };
        if name != "import"
            || !matches!(
                tokens.get(i + 1).map(|t| &t.kind),
                Some(lexer::TokenKind::LParen)
            )
        {
            continue;
        }
        let (Some(lexer::TokenKind::Str(path)), Some(lexer::TokenKind::RParen)) = (
            tokens.get(i + 2).map(|t| &t.kind),
            tokens.get(i + 3).map(|t| &t.kind),
        ) else {
            return Err(crate::diag::render(
                display,
                src,
                "cannot bundle: this import's path is not a literal string",
                tokens[i].span,
            ));
        };
        let target = resolve(dir, path);
        out.push(Import {
            range: tokens[i].span.start..tokens[i + 3].span.end,
            target: target.is_file().then_some(target),
        });
    }
    Ok(out)
}

/// `src` with each range replaced by its text. Ranges come from a
/// left-to-right token scan, so they are already in order and cannot
/// overlap.
fn splice(src: &str, subs: &[(std::ops::Range<usize>, String)]) -> String {
    let mut out = String::with_capacity(src.len());
    let mut at = 0;
    for (range, text) in subs {
        out.push_str(&src[at..range.start]);
        out.push_str(text);
        at = range.end;
    }
    out.push_str(&src[at..]);
    out
}

struct Bundler {
    /// The entry's directory, so the comment naming each module reads
    /// the way the script that imported it spelled it.
    root: PathBuf,
    done: std::collections::HashMap<PathBuf, String>,
    /// The chain of files currently being read, for the cycle check.
    open: Vec<PathBuf>,
    /// Module texts in the order they must run: a module is pushed
    /// after everything it imports.
    out: Vec<String>,
}

impl Bundler {
    /// How a module is named in the bundle's comments: relative to the
    /// entry script when it sits under it, so a bundle does not carry
    /// the author's home directory to whoever reads it.
    fn show(&self, path: &Path) -> String {
        match path.strip_prefix(&self.root) {
            Ok(rel) => rel.display().to_string(),
            Err(_) => path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
        }
    }

    /// Read `path`, inline whatever it imports, and append it. Returns
    /// the name the module is bound to; a file already inlined returns
    /// its existing name and is not read again.
    fn add(
        &mut self,
        path: &Path,
        from: Option<(&str, &str, lexer::Span)>,
    ) -> Result<String, String> {
        if let Some(name) = self.done.get(path) {
            return Ok(name.clone());
        }
        if self.open.iter().any(|p| p == path) {
            let (display, src, span) = from.expect("a cycle needs an importer");
            return Err(crate::diag::render(
                display,
                src,
                &format!("cannot bundle: circular import of {:?}", self.show(path)),
                span,
            ));
        }
        let display = self.show(path);
        let src = std::fs::read_to_string(path)
            .map_err(|e| format!("ting: cannot read {}: {e}", path.display()))?;
        self.open.push(path.to_path_buf());
        let body = self.inline(&display, &src, path)?;
        self.open.pop();

        let exports = exports_of(&display, &src)?;
        let name = binding(self.done.len());
        let entries: Vec<String> = exports.iter().map(|n| format!("{n:?}: {n}")).collect();
        let mut text = format!("# {display}\nlet {name} = fn() {{\n");
        for line in body.lines() {
            if line.trim().is_empty() {
                text.push('\n');
            } else {
                text.push_str("  ");
                text.push_str(line);
                text.push('\n');
            }
        }
        text.push_str(&format!("  return {{{}}};\n}}();\n", entries.join(", ")));
        self.done.insert(path.to_path_buf(), name.clone());
        self.out.push(text);
        Ok(name)
    }

    /// `src` with every local import replaced by the name of the
    /// module it asks for, the modules themselves having been added
    /// first.
    fn inline(&mut self, display: &str, src: &str, path: &Path) -> Result<String, String> {
        let dir = path.parent().unwrap_or(Path::new("."));
        let imports = imports_of(display, src, dir)?;
        let mut subs = Vec::new();
        for import in imports {
            let Some(target) = import.target else {
                continue;
            };
            let span = lexer::Span {
                start: import.range.start,
                end: import.range.end,
            };
            let name = self.add(&target, Some((display, src, span)))?;
            subs.push((import.range, name));
        }
        Ok(splice(src, &subs))
    }
}

/// The names a module's top level declares, sorted, which is the map
/// an import of it hands back. A module that returns from its top
/// level is refused: the bundle would return that value instead of
/// the module's map, and quietly.
fn exports_of(display: &str, src: &str) -> Result<Vec<String>, String> {
    let render = |m: &str, s: lexer::Span| crate::diag::render(display, src, m, s);
    let tokens = lexer::lex(src).map_err(|e| render(&e.message, e.span))?;
    let program = crate::parser::parse_program(&tokens).map_err(|e| render(&e.message, e.span))?;
    let mut names = std::collections::BTreeSet::new();
    for stmt in &program {
        match &stmt.kind {
            crate::ast::StmtKind::Let(name, _) => {
                names.insert(name.clone());
            }
            crate::ast::StmtKind::Return(_) => {
                return Err(render(
                    "cannot bundle: this module returns from its top level",
                    stmt.span,
                ));
            }
            _ => {}
        }
    }
    Ok(names.into_iter().collect())
}

/// The whole program in one file: every local module the script
/// imports, directly or through another module, inlined once and in
/// dependency order, then the script itself.
pub fn bundle(path: &Path) -> Result<String, String> {
    let path = path
        .canonicalize()
        .map_err(|e| format!("ting: cannot read {}: {e}", path.display()))?;
    let root = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut bundler = Bundler {
        root,
        done: std::collections::HashMap::new(),
        open: vec![path.clone()],
        out: Vec::new(),
    };
    let display = bundler.show(&path);
    let src = std::fs::read_to_string(&path)
        .map_err(|e| format!("ting: cannot read {}: {e}", path.display()))?;
    // The entry is parsed for the same reasons a module is: a file
    // that does not parse cannot be bundled, and saying so here beats
    // handing back something that only fails when it is run.
    exports_of(&display, &src)?;
    let main = bundler.inline(&display, &src, &path)?;

    let mut out = format!(
        "# {display}, bundled by `ting --bundle`.\n\
         #\n\
         # Each local module is inlined once, after the modules it\n\
         # imports, as a function returning what its top level\n\
         # declared; every import of it reads that one binding, which\n\
         # is what importing a file twice already gives. An import\n\
         # whose path is not a file was left as it was: the binary\n\
         # answers it.\n\n"
    );
    for text in &bundler.out {
        out.push_str(text);
        out.push('\n');
    }
    out.push_str(&format!("# {display}\n"));
    out.push_str(&main);
    Ok(out)
}
