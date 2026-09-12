//! Rendering of error diagnostics with a source excerpt.

use crate::lexer::Span;

/// What went wrong reading something, in ting's words rather than
/// std's. `read_to_string` on bytes that are not text hands back an
/// `InvalidData` error spelled "stream did not contain valid UTF-8",
/// which says "stream" about a file the message has just named and
/// never says what the trouble is. Every "cannot read" in the
/// program goes through here so they all say the same thing.
pub fn read_why(e: &std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::InvalidData {
        return "not UTF-8 text".to_string();
    }
    e.to_string()
}

/// `cannot read` about a path that came from a value rather than
/// from the command line. Such a path can be two things a path
/// never is: enormous, and text. A program that hands the CONTENTS
/// of a file to something wanting its NAME is told `No such file or
/// directory` about forty characters of its own data, which is true
/// and reads like nonsense — so a line break, which no path a
/// program means to open has in it, is diagnosed instead, and the
/// quote is cut to the width the traces use. A path with no line
/// break is named in full, however long: that is the file the reader
/// has to go and look at.
pub fn cannot_read(path: &str, why: &str) -> String {
    if !path.contains('\n') {
        return format!("cannot read {path:?}: {why}");
    }
    let mut shown: String = path.chars().take(30).collect();
    if path.chars().count() > 30 {
        shown.push_str("...");
    }
    format!("cannot read {shown:?}: that is text, not a path (a path cannot hold a line break)")
}

/// Bytes as text, or a message saying exactly where they stop being
/// text. A log is a gigabyte of good lines and one byte from some
/// older encoding; "not UTF-8 text" alone leaves nothing to search
/// for, so the byte, its offset, and the line it falls in are all
/// named. `from_utf8` already knows the offset — it is `valid_up_to`
/// — and the line is a count of the newlines before it.
pub fn text_of(bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|e| {
        let at = e.utf8_error().valid_up_to();
        let bytes = e.as_bytes();
        let line = bytes[..at].iter().filter(|b| **b == b'\n').count() + 1;
        let sol = bytes[..at]
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |i| i + 1);
        format!(
            "not UTF-8 text: byte {:#04x} at offset {at} (line {line}, byte {})",
            bytes[at],
            at - sol + 1
        )
    })
}

/// The same for one line read on its own: its offsets are its own,
/// and `whose` names it the way the reader can — "line 2" when the
/// reader is counting, "the line" when it is a stream nobody has
/// numbered.
pub fn text_of_line<'a>(bytes: &'a [u8], whose: &str) -> Result<&'a str, String> {
    std::str::from_utf8(bytes).map_err(|e| {
        let at = e.valid_up_to();
        format!(
            "not UTF-8 text: byte {:#04x} at byte {} of {whose}",
            bytes[at],
            at + 1
        )
    })
}

/// Bytes as text, or as text with the bad ones replaced when the
/// caller asked for that. "lossy" means here what it has always
/// meant for a child's output: a byte that is not UTF-8 becomes a
/// replacement character, and the read goes on.
pub fn text_or_lossy(bytes: Vec<u8>, lossy: bool) -> Result<String, String> {
    if lossy {
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    } else {
        text_of(bytes)
    }
}

/// The same choice for one line.
pub fn line_or_lossy<'a>(
    bytes: &'a [u8],
    whose: &str,
    lossy: bool,
) -> Result<std::borrow::Cow<'a, str>, String> {
    if lossy {
        Ok(String::from_utf8_lossy(bytes))
    } else {
        text_of_line(bytes, whose).map(std::borrow::Cow::Borrowed)
    }
}

/// A file as text, or as lossy text, with the read's own trouble
/// worded either way.
pub fn read_text_mode(path: impl AsRef<std::path::Path>, lossy: bool) -> Result<String, String> {
    match std::fs::read(path) {
        Ok(bytes) => text_or_lossy(bytes, lossy),
        Err(e) => Err(read_why(&e)),
    }
}

/// A file as text, with either kind of trouble already worded: the
/// file could not be read at all, or it is not text and this is
/// where it stops being text.
pub fn read_text(path: impl AsRef<std::path::Path>) -> Result<String, String> {
    match std::fs::read(path) {
        Ok(bytes) => text_of(bytes),
        Err(e) => Err(read_why(&e)),
    }
}

/// Render a `path:line:col` header plus the offending line with a caret
/// underline covering the span (clamped to that line).
///
/// ```text
/// script.ting:3:9: error: undefined variable 'x'
///  3 | print(x + 1);
///    |       ^
/// ```
pub fn render(path: &str, src: &str, message: &str, span: Span) -> String {
    render_level(path, src, "error", message, span)
}

/// `render` with an explicit level word ("error", "warning").
pub fn render_level(path: &str, src: &str, level: &str, message: &str, span: Span) -> String {
    render_level_at(
        path,
        src,
        &crate::lexer::Lines::new(src),
        level,
        message,
        span,
    )
}

/// `render_level` against a line index built once by the caller. A
/// file's warnings are rendered together, and finding each one's line
/// from the top of the file made that quadratic.
pub fn render_level_at(
    path: &str,
    src: &str,
    lines: &crate::lexer::Lines,
    level: &str,
    message: &str,
    span: Span,
) -> String {
    let (line, col) = lines.line_col(src, span.start);
    let line_start = src[..span.start.min(src.len())]
        .rfind('\n')
        .map_or(0, |i| i + 1);
    let line_end = src[line_start..]
        .find('\n')
        .map_or(src.len(), |i| line_start + i);
    let text = &src[line_start..line_end];

    // A span past the line (a foreign offset) degrades to a one-wide
    // caret at the line's end rather than a panic.
    let span_start = span.start.min(line_end);
    let span_end = span.end.clamp(span_start, line_end);
    // COLUMNS, not characters: an ideograph is two columns wide and a
    // combining accent none, so a line holding either would put the
    // carets somewhere the token is not.
    let width = crate::width::width(&src[span_start..span_end]).max(1);
    // Tabs in the prefix stay tabs so the caret lines up in terminals.
    let prefix: String = src[line_start..span.start.min(line_end)]
        .chars()
        .map(|c| {
            if c == '\t' {
                "\t".to_string()
            } else {
                " ".repeat(crate::width::char_width(c))
            }
        })
        .collect();

    let gutter = line.to_string();
    let pad = " ".repeat(gutter.len());
    let carets = "^".repeat(width);
    format!(
        "{path}:{line}:{col}: {level}: {message}\n {gutter} | {text}\n {pad} | {prefix}{carets}"
    )
}

/// "1 argument", "2 arguments": a count and the word it counts, so a
/// message never has to say "argument(s)".
pub fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// A path the way the reader wrote it, where that is still true.
/// An imported module resolves to an absolute path — the right
/// identity, and the wrong thing to print in a table beside rows
/// named relative to the directory the command ran in.
pub fn shorten(path: &str) -> String {
    let Ok(cwd) = std::env::current_dir() else {
        return path.to_string();
    };
    // The path being shortened came from `canonicalize`, which on
    // Windows hands back a verbatim prefix the working directory does
    // not carry — so the two are compared canonicalised, and plain as
    // well, for a path that never went through it.
    let canonical = cwd.canonicalize().unwrap_or_else(|_| cwd.clone());
    let path = std::path::Path::new(path);
    for base in [&canonical, &cwd] {
        if let Ok(rest) = path.strip_prefix(base) {
            return rest.display().to_string();
        }
    }
    path.display().to_string()
}

/// The candidate nearest to `name` by edit distance, if one is close
/// enough to be worth suggesting: at most a third of the name wrong
/// (and always at least one edit, so short names still get help), or
/// a name of three characters or more that one of the two starts with
/// — `lenght` is three edits from `len`, but nobody doubts the intent.
/// Equal distances are settled by the longer shared start (`medain`
/// means `median`, not `mean`) and then alphabetically, so the answer
/// never depends on the order the candidates arrive in.
/// A word another language uses where ting writes something else.
/// These are not typos, so `nearest` will not find them — `null` is
/// two edits from `nil` and `None` is three — and answering "did you
/// mean" would be the wrong shape anyway. The word is known; what is
/// wanted is ting's word for it.
///
/// `and`, `or` and `not` are here as well as in the parser, because
/// they only reach the parser where a token cannot go: `1 + and`
/// parses perfectly well, as the sum of a number and a name nothing
/// has bound.
pub fn spelt_here_as(name: &str) -> Option<&'static str> {
    Some(match name {
        "null" | "NULL" | "None" | "undefined" => "nil",
        "True" | "TRUE" => "true",
        "False" | "FALSE" => "false",
        "and" => "&&",
        "or" => "||",
        "not" => "!",
        _ => return None,
    })
}

/// What a module does not have, in one sentence: the module named the
/// way `--check` names a file, the key that was asked for, and the
/// nearest thing it does offer. `--check` says this about a lookup it
/// can see before the program runs, and a run says it about the same
/// lookup when it reaches it, so the two are this one function.
pub fn no_member<'a>(
    module: &str,
    key: &str,
    exports: impl IntoIterator<Item = &'a str>,
) -> String {
    // An exact builtin of that name is a certainty where the nearest
    // export is only a guess, so it wins: a module that retires a
    // function into a builtin leaves callers here.
    if crate::value::Builtin::ALL.iter().any(|b| b.name() == key) {
        return format!("{module} has no `{key}` (`{key}` is a builtin)");
    }
    let names: Vec<&str> = exports.into_iter().collect();
    // A near miss inside the module wins a tie — it is the thing
    // being indexed — but a builtin that is closer wins outright: the
    // stdlib leaves file IO and the rest to the builtins, so a reader
    // who goes looking in lib/fs.ting for `write` is one name away
    // from `write_file` and no distance at all from the module.
    let export = nearest_found(key, names.iter().copied());
    let builtin = nearest_found(key, crate::value::Builtin::ALL.iter().map(|b| b.name()));
    match (&export, &builtin) {
        (Some((ef, e)), Some((bf, b))) if (bf, distance(key, b)) < (ef, distance(key, e)) => {
            return format!("{module} has no `{key}` (did you mean the builtin `{b}`?)");
        }
        (Some((_, e)), _) => return format!("{module} has no `{key}` (did you mean `{e}`?)"),
        (None, Some((_, b))) => {
            return format!("{module} has no `{key}` (did you mean the builtin `{b}`?)");
        }
        (None, None) => {}
    }
    // No near miss: a reader who guessed wrong has nowhere to go from
    // the name alone, so the module says what it does have. A short
    // module names everything; a long one names how much there is and
    // the command that prints it, since eight of fifty-four names in
    // the order they happen to be declared help nobody.
    match names.len() {
        0 => format!("{module} has no `{key}`"),
        1..=8 => {
            // Sorted, so the checker reading a file top to bottom and
            // the run reading a map say the same sentence.
            let mut names = names.clone();
            names.sort_unstable();
            let list: Vec<String> = names.iter().map(|n| format!("`{n}`")).collect();
            format!("{module} has no `{key}` (it has {})", list.join(", "))
        }
        n => {
            format!("{module} has no `{key}` (it has {n} names — `ting --doc {module}` lists them)")
        }
    }
}

/// How an answer was found, best first. A shared start alone will
/// match a name of any distance, so `string_upper` finds `str` and
/// `list_sort` finds `list_dir` — both of which are further from what
/// was meant than the part sitting in the guess. How an answer was
/// found therefore ranks before how far it is: only a single slip,
/// which is what a typo is, beats a part that is a name outright.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum Found {
    /// The whole guess, one edit away or none.
    Slip,
    /// A part of the guess, which is a name exactly.
    Part,
    /// The whole guess, further off but sharing a start.
    Whole,
    /// The guess is a part of the name — the half a person keeps.
    Inside,
    /// A part of the guess, near a name.
    NearPart,
}

pub fn nearest<'a>(name: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    nearest_found(name, candidates).map(|(_, c)| c)
}

/// The nearest name and how it was found, for a caller weighing two
/// of these against each other.
pub fn nearest_found<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<(Found, String)> {
    let candidates: Vec<&str> = candidates.into_iter().collect();
    let whole = closest(name, &candidates, false);
    if let Some((d, c)) = whole
        && d <= 1
    {
        return Some((Found::Slip, c.to_string()));
    }
    // A compound guess holds the name inside it: `to_float` is `float`
    // with a habit from another language in front of it, `array_len` is
    // `len`, `list_median` is `median` with the module's name repeated.
    // The parts are asked LAST first: the qualifier goes in front in
    // every language that spells names this way, so the name is the
    // part at the end.
    let mut parts: Vec<&str> = name
        .split('_')
        .filter(|p| *p != name && p.chars().count() >= 3)
        .collect();
    parts.reverse();
    if let Some(p) = parts.iter().find(|p| candidates.contains(*p)) {
        return Some((Found::Part, (*p).to_string()));
    }
    if let Some((_, c)) = whole {
        return Some((Found::Whole, c.to_string()));
    }
    // The guess is a part OF the name, which is the way round a
    // person usually remembers it: the head of a compound name is the
    // part carrying no information — `check_`, `list_`, `str_` — and
    // the tail is what distinguishes it, so `check_approx` is
    // remembered as `approx` and `check_err` as `err`. A candidate
    // ending in the guess is preferred over one merely holding it.
    // No length floor here, unlike every rule above: those measure a
    // distance, and under three characters a distance means nothing
    // because every short name is a slip or two from every other.
    // Being a whole part of a name is identity, not distance, so `eq`
    // finds `check_eq` — and what this tier offers is a suggestion
    // there would otherwise be none of.
    let holds = |c: &str, last: bool| match last {
        true => c.rsplit('_').next() == Some(name),
        false => c.split('_').any(|p| p == name),
    };
    if let Some((_, c)) = whole {
        return Some((Found::Whole, c.to_string()));
    }
    if let Some(c) = candidates
        .iter()
        .find(|c| **c != name && holds(c, true))
        .or_else(|| candidates.iter().find(|c| **c != name && holds(c, false)))
    {
        return Some((Found::Inside, (*c).to_string()));
    }
    parts
        .iter()
        .find_map(|p| closest(p, &candidates, false))
        .map(|(_, c)| (Found::NearPart, c.to_string()))
}

/// The nearest of `candidates` to `name` by edit distance, with a name
/// that starts the other kept however far apart they are. `itself` says
/// whether `name` may be its own answer: a whole guess never is, but a
/// part of one is exactly the answer wanted.
fn closest<'a>(name: &str, candidates: &[&'a str], itself: bool) -> Option<(usize, &'a str)> {
    // Under three characters every name is one edit from every other,
    // so a suggestion would be noise rather than help.
    if name.chars().count() < 3 {
        return None;
    }
    let limit = (name.chars().count() / 3).max(1);
    let mut best: Option<(usize, usize, &str)> = None;
    for c in candidates.iter().copied() {
        if c == name && !itself {
            continue;
        }
        let d = distance(name, c);
        let shares_start = c.chars().count() >= 3
            && name.chars().count() >= 3
            && (name.starts_with(c) || c.starts_with(name));
        if d > limit && !shares_start {
            continue;
        }
        let shared = name
            .chars()
            .zip(c.chars())
            .take_while(|(a, b)| a == b)
            .count();
        match best {
            // Nearer wins; then the longer shared start; then the name.
            Some((bd, bs, bc))
                if (bd, std::cmp::Reverse(bs), bc) <= (d, std::cmp::Reverse(shared), c) => {}
            _ => best = Some((d, shared, c)),
        }
    }
    best.map(|(d, _, c)| (d, c))
}

/// Edit distance in characters: insert, delete and substitute each
/// cost one, and so does swapping two neighbours — `lsp` typed `lps`
/// is one slip, not two. Two rows of state, since a transposition
/// looks back a row further than Levenshtein does.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev2: Vec<usize> = vec![0; b.len() + 1];
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let mut d = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
            if i > 0 && j > 0 && *ca == b[j - 1] && a[i - 1] == *cb {
                d = d.min(prev2[j - 1] + 1);
            }
            cur[j + 1] = d;
        }
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stdlib leaves whole jobs to the builtins, so the name a
    /// reader guessed at inside a module is often one of those.
    #[test]
    fn a_builtin_nearer_than_any_export_is_named_as_a_builtin() {
        let fs = ["with_ext", "base", "size"];
        assert_eq!(
            no_member("lib/fs.ting", "write", fs),
            "lib/fs.ting has no `write` (did you mean the builtin `write_file`?)"
        );
        // An export that is nearer wins, and so does one that ties:
        // the module is what the reader was indexing.
        assert_eq!(
            no_member("lib/list.ting", "medain", ["median", "map"]),
            "lib/list.ting has no `medain` (did you mean `median`?)"
        );
        assert_eq!(
            no_member("lib/x.ting", "prin", ["prinx"]),
            "lib/x.ting has no `prin` (did you mean `prinx`?)"
        );
        // The module's own export sits inside the guess, and the
        // builtin `list_dir` is only near the whole of it: how the
        // answer was found ranks before how far away it is.
        assert_eq!(
            no_member("lib/list.ting", "list_median", ["median", "map"]),
            "lib/list.ting has no `list_median` (did you mean `median`?)"
        );
        // An exact builtin still beats every guess.
        assert_eq!(
            no_member("lib/list.ting", "len", ["lens"]),
            "lib/list.ting has no `len` (`len` is a builtin)"
        );
        // Nothing near in either place keeps the sentence 1046 wrote.
        assert_eq!(
            no_member("lib/fs.ting", "zzqqxx", fs),
            "lib/fs.ting has no `zzqqxx` (it has `base`, `size`, `with_ext`)"
        );
    }

    /// A key a module does not have, with no near miss to offer: what
    /// the module DOES have is the only help left, and a long module
    /// hands over the command that prints it rather than an arbitrary
    /// handful of names.
    #[test]
    fn a_member_with_no_near_miss_says_what_the_module_has() {
        let short = ["parse", "help", "main"];
        assert_eq!(
            no_member("lib/args.ting", "zzqqxx", short),
            "lib/args.ting has no `zzqqxx` (it has `help`, `main`, `parse`)"
        );
        let long: Vec<String> = (0..9).map(|i| format!("name{i}")).collect();
        assert_eq!(
            no_member("lib/big.ting", "zzqqxx", long.iter().map(String::as_str)),
            "lib/big.ting has no `zzqqxx` (it has 9 names — `ting --doc lib/big.ting` lists them)"
        );
        // Eight is still few enough to name.
        assert!(
            no_member(
                "lib/big.ting",
                "zzqqxx",
                long[..8].iter().map(String::as_str)
            )
            .ends_with(
                "(it has `name0`, `name1`, `name2`, `name3`, `name4`, `name5`, `name6`, `name7`)"
            )
        );
        // A near miss is better than either, and a builtin better than
        // that.
        assert_eq!(
            no_member("lib/args.ting", "pares", short),
            "lib/args.ting has no `pares` (did you mean `parse`?)"
        );
        assert_eq!(
            no_member("lib/args.ting", "print", short),
            "lib/args.ting has no `print` (`print` is a builtin)"
        );
        // A module with nothing in it has nothing to say.
        assert_eq!(
            no_member("lib/empty.ting", "zzqqxx", Vec::<&str>::new()),
            "lib/empty.ting has no `zzqqxx`"
        );
    }

    #[test]
    fn caret_under_mid_line_span() {
        let src = "let x = 1;\nprint(y + 1);\n";
        let start = src.find('y').unwrap();
        let out = render(
            "t.ting",
            src,
            "undefined variable 'y'",
            Span::new(start, start + 1),
        );
        assert_eq!(
            out,
            "t.ting:2:7: error: undefined variable 'y'\n \
             2 | print(y + 1);\n   |       ^"
        );
    }

    #[test]
    fn caret_width_matches_span() {
        let src = "1 + true;";
        let out = render("t.ting", src, "boom", Span::new(4, 8));
        assert!(out.ends_with(" | 1 + true;\n   |     ^^^^"), "got:\n{out}");
    }

    /// A line holding text wider than one column each: the carets
    /// have to be placed in COLUMNS, or they point somewhere the
    /// token is not. Every ideograph before the span pushes it one
    /// column right, and the span's own are two columns each.
    #[test]
    fn carets_line_up_under_wide_characters() {
        let src = "print(\"\u{65e5}\u{672c}\", totl);";
        let start = src.find("totl").unwrap();
        let out = render("t.ting", src, "boom", Span::new(start, start + 4));
        let last = out.lines().next_back().unwrap();
        let before = last.split('^').next().unwrap();
        // 5 for the gutter, then print(" is 7, the two ideographs 4,
        // and ", is 3.
        assert_eq!(before.chars().count(), 5 + 7 + 4 + 3, "got:\n{out}");
        assert!(last.ends_with("^^^^"), "got:\n{out}");

        // The span itself is two columns per character.
        let wide = "\u{65e5}\u{672c}";
        let out = render("t.ting", wide, "boom", Span::new(0, wide.len()));
        assert!(out.ends_with("| ^^^^"), "got:\n{out}");

        // A combining accent takes no column of its own, so the caret
        // does not drift right of the letter it rides on.
        let src = "e\u{301}x = 1;";
        let start = src.find('x').unwrap();
        let out = render("t.ting", src, "boom", Span::new(start, start + 1));
        assert!(out.ends_with("|  ^"), "got:\n{out}");
    }

    #[test]
    fn span_at_end_of_input_gets_one_caret() {
        let src = "let x =";
        let out = render("t.ting", src, "expected expression", Span::new(7, 7));
        assert!(out.ends_with(" | let x =\n   |        ^"), "got:\n{out}");
    }

    #[test]
    fn multi_line_span_clamps_to_first_line() {
        let src = "if true {\n  1;\n}";
        let out = render("t.ting", src, "msg", Span::new(0, src.len()));
        assert!(out.contains(" | if true {\n"), "got:\n{out}");
        assert!(out.ends_with(" | ^^^^^^^^^"), "got:\n{out}");
    }

    #[test]
    fn tabs_keep_caret_aligned() {
        let src = "\tprint(z);";
        let start = src.find('z').unwrap();
        let out = render(
            "t.ting",
            src,
            "undefined variable 'z'",
            Span::new(start, start + 1),
        );
        assert!(out.ends_with(" | \t      ^"), "got:\n{out}");
    }

    #[test]
    fn unicode_before_span_counts_chars_not_bytes() {
        let src = "let héllo = wörld;";
        let start = src.find('w').unwrap();
        let out = render(
            "t.ting",
            src,
            "undefined variable 'wörld'",
            Span::new(start, start + "wörld".len()),
        );
        assert!(out.ends_with(" |             ^^^^^"), "got:\n{out}");
    }

    #[test]
    fn shorten_names_paths_under_the_working_directory() {
        let cwd = std::env::current_dir().expect("cwd");
        let want = std::path::Path::new("lib")
            .join("test.ting")
            .display()
            .to_string();
        let inside = cwd.join("lib").join("test.ting");
        assert_eq!(shorten(&inside.display().to_string()), want);
        // The shape the reports really pass: what `import` resolved
        // to, which is canonical, and on Windows verbatim with it.
        let resolved = cwd
            .canonicalize()
            .expect("canonicalize")
            .join("lib")
            .join("test.ting");
        assert_eq!(shorten(&resolved.display().to_string()), want);
        // A path that is not under it needs all of itself to say
        // where it is, and a relative one is already there.
        assert_eq!(shorten("lib/test.ting"), "lib/test.ting");
        let outside = cwd
            .parent()
            .map_or_else(|| "/".to_string(), |p| p.display().to_string());
        assert_eq!(shorten(&outside), outside);
    }

    /// Read the other way round: the guess is a part of the name.
    /// The head of a compound name carries no information, so the
    /// tail is the half a person keeps.
    #[test]
    fn nearest_finds_a_name_the_guess_is_part_of() {
        assert_eq!(
            nearest("approx", ["check_approx", "check_err"]),
            Some("check_approx".to_string())
        );
        assert_eq!(
            nearest("err", ["check_err", "check_approx"]),
            Some("check_err".to_string())
        );
        // A name ENDING in the guess beats one merely holding it.
        assert_eq!(
            nearest("dirs", ["a_dirs_b", "x_dirs"]),
            Some("x_dirs".to_string())
        );
        // A shared start still wins: `med` is the start of `median`,
        // which is nearer than a name with `med` buried in it.
        assert_eq!(
            nearest("med", ["median", "check_med"]),
            Some("median".to_string())
        );
        // No length floor on this tier: two characters are too few to
        // measure a distance with, and being a whole part of a name
        // is identity rather than distance.
        assert_eq!(
            nearest("eq", ["check_eq", "check_err"]),
            Some("check_eq".to_string())
        );
        assert_eq!(nearest("eq", ["check_err"]), None);
        // Identity, not a prefix: `er` is two characters of `err` and
        // no part of anything, so it stays unanswered.
        assert_eq!(nearest("er", ["check_err"]), None);
        // The tail is preferred at this length too.
        assert_eq!(
            nearest("eq", ["eq_of", "check_eq"]),
            Some("check_eq".to_string())
        );
        // A name is still never its own suggestion, at any length.
        assert_eq!(nearest("eq", ["eq"]), None);
    }

    /// A guess built out of a habit from another language holds the
    /// name inside it: the parts are tried when the whole is nothing.
    #[test]
    fn nearest_looks_inside_a_compound_guess() {
        assert_eq!(
            nearest("to_float", ["float", "int"]),
            Some("float".to_string())
        );
        assert_eq!(
            nearest("array_len", ["len", "map"]),
            Some("len".to_string())
        );
        assert_eq!(
            nearest("list_median", ["median", "list_dir"]),
            Some("median".to_string())
        );
        // A part that is near a name answers too: `string` is not a
        // name, and `str` starts it.
        assert_eq!(
            nearest("to_string", ["str", "print"]),
            Some("str".to_string())
        );
        // A part that IS a name beats a part that is merely near one,
        // whichever comes first and whichever is longer.
        assert_eq!(
            nearest("fetch_upper", ["str", "upper"]),
            Some("upper".to_string())
        );
        // And it beats a whole-guess match resting on a shared start,
        // however far off that is: `str` starts `string_upper` and
        // `list_dir` is three edits from `list_sort`, but `upper` and
        // `sort` are the names sitting in the guess.
        assert_eq!(
            nearest("string_upper", ["str", "upper"]),
            Some("upper".to_string())
        );
        assert_eq!(
            nearest("list_sort", ["list_dir", "sort"]),
            Some("sort".to_string())
        );
        assert_eq!(nearest("str_len", ["str", "len"]), Some("len".to_string()));
        // One slip is what a typo is, and it still wins: a plural of a
        // real name is not a compound guess.
        assert_eq!(
            nearest("list_dirs", ["list_dir", "list"]),
            Some("list_dir".to_string())
        );
        // With no part that is a name, the last part is asked first:
        // the qualifier goes in front, so the name is at the end.
        assert_eq!(
            nearest("fetch_records", ["etch", "record"]),
            Some("record".to_string())
        );
        // Parts under three characters are no more help than short
        // names are, and a name with no parts is only itself.
        assert_eq!(nearest("to_x", ["to", "x"]), None);
        assert_eq!(nearest("elephant", ["print", "len"]), None);
    }

    /// A path a program computed can be text it meant to read, and
    /// `No such file or directory` about a spreadsheet says nothing.
    /// A real path keeps its whole name however long it is.
    #[test]
    fn a_path_with_a_line_break_in_it_is_text() {
        assert_eq!(
            cannot_read("a,b\nc,d\n", "No such file or directory"),
            "cannot read \"a,b\\nc,d\\n\": that is text, not a path (a path cannot hold a line break)"
        );
        // Cut to the width the traces use, with the ellipsis inside
        // the quotes so the message cannot be mistaken for the name.
        let long = "date,category,amount\n2026-01-03,food,12.50\n";
        assert!(
            cannot_read(long, "No such file or directory")
                .starts_with("cannot read \"date,category,amount\\n2026-01-0...\": that is text"),
            "{}",
            cannot_read(long, "No such file or directory")
        );
        // No line break: the name in full, and the reason as given.
        let path = "/a/very/long/path/that/somebody/has/to/go/and/look/at.ting";
        assert_eq!(
            cannot_read(path, "No such file or directory"),
            format!("cannot read {path:?}: No such file or directory")
        );
    }

    #[test]
    fn nearest_suggests_only_close_names() {
        assert_eq!(
            nearest("cont", ["count", "print", "len"]),
            Some("count".to_string())
        );
        assert_eq!(
            nearest("lenght", ["len", "length", "left"]),
            Some("length".to_string())
        );
        // Three edits, but "len" starts the name: still the answer.
        assert_eq!(nearest("lenght", ["len", "map"]), Some("len".to_string()));
        assert_eq!(
            nearest("prnt", ["print", "push"]),
            Some("print".to_string())
        );
        // Nothing within a third of the name is no suggestion at all.
        assert_eq!(nearest("elephant", ["print", "len"]), None);
        // The name itself is never its own suggestion.
        assert_eq!(nearest("len", ["len"]), None);
        // Ties go to the longer shared start, then to the first name.
        assert_eq!(
            nearest("medain", ["mean", "median"]),
            Some("median".to_string())
        );
        assert_eq!(
            nearest("medain", ["median", "mean"]),
            Some("median".to_string())
        );
        // A swap of neighbours is one slip.
        assert_eq!(nearest("lps", ["lsp", "map"]), Some("lsp".to_string()));
        // Under three characters, no suggestion at all.
        assert_eq!(nearest("ab", ["ac", "bb"]), None);
    }

    #[test]
    fn where_the_bytes_stop_being_text() {
        assert_eq!(text_of(b"plain".to_vec()), Ok("plain".to_string()));
        // The first byte, and a byte in the middle of the third line.
        assert_eq!(
            text_of(vec![0xff]).unwrap_err(),
            "not UTF-8 text: byte 0xff at offset 0 (line 1, byte 1)"
        );
        assert_eq!(
            text_of(b"a\nbb\nccc\xe9d".to_vec()).unwrap_err(),
            "not UTF-8 text: byte 0xe9 at offset 8 (line 3, byte 4)"
        );
        // Half a character at the end is where it stops too.
        assert_eq!(
            text_of("é".as_bytes()[..1].to_vec()).unwrap_err(),
            "not UTF-8 text: byte 0xc3 at offset 0 (line 1, byte 1)"
        );
        // A line names itself however the reader can name it.
        assert_eq!(text_of_line(b"fine", "line 7"), Ok("fine"));
        assert_eq!(
            text_of_line(b"ca\xe9 bad", "line 7").unwrap_err(),
            "not UTF-8 text: byte 0xe9 at byte 3 of line 7"
        );
        assert_eq!(
            text_of_line(b"\xe9", "the line").unwrap_err(),
            "not UTF-8 text: byte 0xe9 at byte 1 of the line"
        );
    }
}
