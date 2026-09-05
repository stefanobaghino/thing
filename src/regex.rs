//! Regular expressions: a small pattern language, compiled to a
//! program and run by a Pike VM.
//!
//! The VM runs every alternative in lockstep rather than backtracking,
//! so a match costs the input times the program and no pattern can be
//! made to hang — `(a+)+b` against a line of a's is linear here. That
//! costs a little on easy patterns and buys the absence of a whole
//! class of failures, which is the right trade for a language whose
//! scripts read text nobody audited.
//!
//! Positions are character indices, because `len`, `slice` and `find`
//! count characters and a pattern engine that disagreed with them
//! would be a trap. There are no backreferences: they cannot be run in
//! lockstep, and buying them back would mean buying the backtracker
//! with them.

/// One member of a character class: a range, or one of the shorthands
/// that also stand alone outside brackets.
#[derive(Debug, Clone, PartialEq)]
enum Item {
    Range(char, char),
    Digit,
    Word,
    Space,
}

#[derive(Debug, Clone, PartialEq)]
struct Class {
    negated: bool,
    /// Each item, and whether it is the negated form (`\D` in a class).
    items: Vec<(Item, bool)>,
}

impl Class {
    fn matches(&self, c: char) -> bool {
        let mut hit = false;
        for (item, negated) in &self.items {
            let is = match item {
                Item::Range(lo, hi) => *lo <= c && c <= *hi,
                Item::Digit => c.is_ascii_digit(),
                Item::Word => c.is_alphanumeric() || c == '_',
                Item::Space => c.is_whitespace(),
            };
            if is != *negated {
                hit = true;
                break;
            }
        }
        hit != self.negated
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Empty,
    Char(char),
    Any,
    Class(Class),
    Start,
    End,
    /// A group, capturing into that index when it has one.
    Group(Option<usize>, Box<Node>),
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat {
        node: Box<Node>,
        min: u32,
        max: Option<u32>,
        greedy: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Inst {
    Char(char),
    Any,
    Class(Class),
    /// Prefer the first target; the second is the fallback.
    Split(usize, usize),
    Jmp(usize),
    Save(usize),
    Start,
    End,
    Match,
}

/// A counted repetition may not ask for more copies than this, and a
/// whole pattern may not compile to more instructions than that: two
/// numbers rather than one, so `a{1000}{1000}` is refused for the
/// second reason after passing the first.
const MAX_REPEAT: u32 = 1000;
const MAX_PROGRAM: usize = 100_000;

#[derive(Debug)]
pub struct Regex {
    prog: Vec<Inst>,
    /// Capturing groups, not counting the whole match.
    groups: usize,
}

struct Parser<'a> {
    src: &'a [char],
    pos: usize,
    groups: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            return true;
        }
        false
    }

    fn error(&self, msg: &str) -> String {
        format!("{msg} at {}", self.pos)
    }

    fn alt(&mut self) -> Result<Node, String> {
        let mut branches = vec![self.concat()?];
        while self.eat('|') {
            branches.push(self.concat()?);
        }
        if branches.len() == 1 {
            return Ok(branches.pop().unwrap());
        }
        Ok(Node::Alt(branches))
    }

    fn concat(&mut self) -> Result<Node, String> {
        let mut parts = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            parts.push(self.repeat()?);
        }
        match parts.len() {
            0 => Ok(Node::Empty),
            1 => Ok(parts.pop().unwrap()),
            _ => Ok(Node::Concat(parts)),
        }
    }

    fn repeat(&mut self) -> Result<Node, String> {
        let atom = self.atom()?;
        let (min, max) = match self.peek() {
            Some('*') => {
                self.pos += 1;
                (0, None)
            }
            Some('+') => {
                self.pos += 1;
                (1, None)
            }
            Some('?') => {
                self.pos += 1;
                (0, Some(1))
            }
            Some('{') if self.counted_ahead(self.pos) => {
                self.pos += 1;
                let min = self.number()?;
                let max = if self.eat(',') {
                    if self.peek() == Some('}') {
                        None
                    } else {
                        Some(self.number()?)
                    }
                } else {
                    Some(min)
                };
                if !self.eat('}') {
                    return Err(self.error("unclosed {"));
                }
                if min > MAX_REPEAT || max.is_some_and(|m| m > MAX_REPEAT) {
                    return Err(self.error("repetition count is too large"));
                }
                if max.is_some_and(|m| m < min) {
                    return Err(self.error("repetition counts are the wrong way round"));
                }
                (min, max)
            }
            _ => return Ok(atom),
        };
        if matches!(atom, Node::Start | Node::End) {
            return Err(self.error("an anchor cannot be repeated"));
        }
        let greedy = !self.eat('?');
        Ok(Node::Repeat {
            node: Box::new(atom),
            min,
            max,
            greedy,
        })
    }

    /// Whether a `{` opens a counted repetition. One that does not —
    /// `a{b}` — is an ordinary character, as it is in every engine
    /// people have used before this one.
    fn counted_ahead(&self, brace: usize) -> bool {
        let mut i = brace + 1;
        let mut digits = 0;
        while self.src.get(i).is_some_and(|c| c.is_ascii_digit()) {
            i += 1;
            digits += 1;
        }
        if digits == 0 {
            return false;
        }
        if self.src.get(i) == Some(&',') {
            i += 1;
            while self.src.get(i).is_some_and(|c| c.is_ascii_digit()) {
                i += 1;
            }
        }
        self.src.get(i) == Some(&'}')
    }

    fn number(&mut self) -> Result<u32, String> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        if start == self.pos {
            return Err(self.error("expected a number"));
        }
        let text: String = self.src[start..self.pos].iter().collect();
        text.parse::<u32>()
            .map_err(|_| self.error("repetition count is too large"))
    }

    fn atom(&mut self) -> Result<Node, String> {
        let c = match self.peek() {
            Some(c) => c,
            None => return Ok(Node::Empty),
        };
        self.pos += 1;
        match c {
            '.' => Ok(Node::Any),
            '^' => Ok(Node::Start),
            '$' => Ok(Node::End),
            '(' => {
                let index = if self.pos + 1 < self.src.len()
                    && self.src[self.pos] == '?'
                    && self.src[self.pos + 1] == ':'
                {
                    self.pos += 2;
                    None
                } else {
                    self.groups += 1;
                    Some(self.groups)
                };
                let inner = self.alt()?;
                if !self.eat(')') {
                    return Err(self.error("unclosed ("));
                }
                Ok(Node::Group(index, Box::new(inner)))
            }
            ')' => Err(self.error("unmatched )")),
            '[' => self.class(),
            ']' => Ok(Node::Char(']')),
            '*' | '+' | '?' => Err(self.error("nothing to repeat")),
            // A brace that opens a count here has nothing before it to
            // count; one that opens nothing is an ordinary character.
            '{' if self.counted_ahead(self.pos - 1) => Err(self.error("nothing to repeat")),
            '\\' => self.escape(),
            _ => Ok(Node::Char(c)),
        }
    }

    /// An escape, wherever one may appear. A shorthand becomes a
    /// one-item class; anything else stands for itself, so `\.` is a
    /// dot and `\\` is a backslash.
    fn escape(&mut self) -> Result<Node, String> {
        let c = match self.peek() {
            Some(c) => c,
            None => return Err(self.error("a backslash needs something after it")),
        };
        self.pos += 1;
        if let Some((item, negated)) = shorthand(c) {
            return Ok(Node::Class(Class {
                negated: false,
                items: vec![(item, negated)],
            }));
        }
        Ok(Node::Char(match c {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            '0' => '\0',
            other => other,
        }))
    }

    fn class(&mut self) -> Result<Node, String> {
        let negated = self.eat('^');
        let mut items = Vec::new();
        // A ']' first is a literal bracket, the usual convention.
        if self.eat(']') {
            items.push((Item::Range(']', ']'), false));
        }
        loop {
            let c = match self.peek() {
                Some(']') => {
                    self.pos += 1;
                    break;
                }
                Some(c) => c,
                None => return Err(self.error("unclosed [")),
            };
            self.pos += 1;
            let lo = if c == '\\' {
                let e = match self.peek() {
                    Some(e) => e,
                    None => return Err(self.error("a backslash needs something after it")),
                };
                self.pos += 1;
                if let Some((item, neg)) = shorthand(e) {
                    items.push((item, neg));
                    continue;
                }
                match e {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    other => other,
                }
            } else {
                c
            };
            // A '-' at the end of a class is a literal dash.
            if self.peek() == Some('-') && self.src.get(self.pos + 1) != Some(&']') {
                self.pos += 1;
                let hi = match self.peek() {
                    Some('\\') => {
                        self.pos += 1;
                        let e = match self.peek() {
                            Some(e) => e,
                            None => {
                                return Err(self.error("a backslash needs something after it"));
                            }
                        };
                        self.pos += 1;
                        match e {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '0' => '\0',
                            other => other,
                        }
                    }
                    Some(e) => {
                        self.pos += 1;
                        e
                    }
                    None => return Err(self.error("unclosed [")),
                };
                if hi < lo {
                    return Err(self.error("a range runs the wrong way"));
                }
                items.push((Item::Range(lo, hi), false));
            } else {
                items.push((Item::Range(lo, lo), false));
            }
        }
        if items.is_empty() {
            return Err(self.error("an empty class matches nothing"));
        }
        Ok(Node::Class(Class { negated, items }))
    }
}

/// The class shorthands, and whether the letter names the negated
/// form. `\d` and `\D` differ only in that flag.
fn shorthand(c: char) -> Option<(Item, bool)> {
    match c {
        'd' => Some((Item::Digit, false)),
        'D' => Some((Item::Digit, true)),
        'w' => Some((Item::Word, false)),
        'W' => Some((Item::Word, true)),
        's' => Some((Item::Space, false)),
        'S' => Some((Item::Space, true)),
        _ => None,
    }
}

struct Compiler {
    prog: Vec<Inst>,
}

impl Compiler {
    fn push(&mut self, inst: Inst) -> Result<usize, String> {
        if self.prog.len() >= MAX_PROGRAM {
            return Err("pattern is too large".to_string());
        }
        self.prog.push(inst);
        Ok(self.prog.len() - 1)
    }

    fn emit(&mut self, node: &Node) -> Result<(), String> {
        match node {
            Node::Empty => Ok(()),
            Node::Char(c) => self.push(Inst::Char(*c)).map(|_| ()),
            Node::Any => self.push(Inst::Any).map(|_| ()),
            Node::Class(cl) => self.push(Inst::Class(cl.clone())).map(|_| ()),
            Node::Start => self.push(Inst::Start).map(|_| ()),
            Node::End => self.push(Inst::End).map(|_| ()),
            Node::Group(index, inner) => {
                if let Some(i) = index {
                    self.push(Inst::Save(i * 2))?;
                }
                self.emit(inner)?;
                if let Some(i) = index {
                    self.push(Inst::Save(i * 2 + 1))?;
                }
                Ok(())
            }
            Node::Concat(parts) => {
                for part in parts {
                    self.emit(part)?;
                }
                Ok(())
            }
            Node::Alt(branches) => {
                let mut ends = Vec::new();
                for (i, branch) in branches.iter().enumerate() {
                    if i + 1 == branches.len() {
                        self.emit(branch)?;
                        break;
                    }
                    let split = self.push(Inst::Split(0, 0))?;
                    let here = self.prog.len();
                    self.emit(branch)?;
                    ends.push(self.push(Inst::Jmp(0))?);
                    let next = self.prog.len();
                    self.prog[split] = Inst::Split(here, next);
                }
                let end = self.prog.len();
                for jump in ends {
                    self.prog[jump] = Inst::Jmp(end);
                }
                Ok(())
            }
            Node::Repeat {
                node,
                min,
                max,
                greedy,
            } => self.repeat(node, *min, *max, *greedy),
        }
    }

    fn repeat(
        &mut self,
        node: &Node,
        min: u32,
        max: Option<u32>,
        greedy: bool,
    ) -> Result<(), String> {
        // The required copies, spelled out.
        for _ in 0..min {
            self.emit(node)?;
        }
        match max {
            // Unbounded: a loop after the required copies.
            None => {
                let split = self.push(Inst::Split(0, 0))?;
                let body = self.prog.len();
                self.emit(node)?;
                self.push(Inst::Jmp(split))?;
                let after = self.prog.len();
                self.prog[split] = if greedy {
                    Inst::Split(body, after)
                } else {
                    Inst::Split(after, body)
                };
                Ok(())
            }
            // Bounded: each further copy guarded by its own split, so
            // leaving early skips every copy that would have followed.
            Some(max) => {
                let mut splits = Vec::new();
                for _ in min..max {
                    let split = self.push(Inst::Split(0, 0))?;
                    let body = self.prog.len();
                    self.emit(node)?;
                    splits.push((split, body));
                }
                let after = self.prog.len();
                for (split, body) in splits {
                    self.prog[split] = if greedy {
                        Inst::Split(body, after)
                    } else {
                        Inst::Split(after, body)
                    };
                }
                Ok(())
            }
        }
    }
}

/// A thread: where it is in the program, and what it has captured.
/// A thread's capture slots. Threads share one set until a `Save`
/// writes to it, so the copies a search makes are reference counts:
/// only a thread that actually records a group position pays for a
/// vector, and only when it does not already hold the last reference.
type Caps = std::rc::Rc<Vec<Option<usize>>>;

type Thread = (usize, Caps);

/// What a search reuses at every position: which instructions this
/// step has already reached, and the stack `add` walks the epsilon
/// closure with. Both are cleared per use rather than reallocated.
struct Scratch {
    seen: Vec<bool>,
    stack: Vec<Thread>,
    /// Capture sets whose last reference has come back, ready to be
    /// filled again. The leftmost restart needs a fresh set at every
    /// position, which is once per character: taking those from here
    /// is what keeps a search from allocating per character.
    pool: Vec<Caps>,
}

/// A capture set of `slots` empty slots, refilled from the pool when
/// one has come back and allocated only when it has not.
fn empty_caps(scratch: &mut Scratch, slots: usize) -> Caps {
    match scratch.pool.pop() {
        Some(mut caps) => {
            let v = std::rc::Rc::get_mut(&mut caps).expect("a pooled set is unheld");
            v.clear();
            v.resize(slots, None);
            caps
        }
        None => std::rc::Rc::new(vec![None; slots]),
    }
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Regex, String> {
        let src: Vec<char> = pattern.chars().collect();
        let mut parser = Parser {
            src: &src,
            pos: 0,
            groups: 0,
        };
        let node = parser.alt()?;
        if parser.pos != src.len() {
            // Past the character, like every other message here.
            parser.pos += 1;
            return Err(parser.error("unmatched )"));
        }
        let mut compiler = Compiler { prog: Vec::new() };
        compiler.push(Inst::Save(0))?;
        compiler.emit(&node)?;
        compiler.push(Inst::Save(1))?;
        compiler.push(Inst::Match)?;
        Ok(Regex {
            prog: compiler.prog,
            groups: parser.groups,
        })
    }

    /// How many capturing groups the pattern has, the whole match
    /// excluded.
    pub fn groups(&self) -> usize {
        self.groups
    }

    /// The leftmost match at or after `from`, as capture slots: pairs
    /// of character indices, slot 0 and 1 being the whole match. A
    /// group that took no part in the match is None.
    pub fn find_at(&self, text: &[char], from: usize) -> Option<Vec<Option<usize>>> {
        let slots = self.groups * 2 + 2;
        // Two thread lists and one scratch, reused at every position.
        // Advancing one character used to allocate a list, a seen
        // vector and a stack; the search is the same, the allocation
        // is not per character any more.
        let mut clist: Vec<Thread> = Vec::new();
        let mut nlist: Vec<Thread> = Vec::new();
        let mut scratch = Scratch {
            seen: vec![false; self.prog.len()],
            stack: Vec::new(),
            pool: Vec::new(),
        };
        let first = empty_caps(&mut scratch, slots);
        self.add(&mut clist, &mut scratch, 0, from, text, first);
        let mut matched: Option<Caps> = None;
        let mut pos = from;
        loop {
            // The sets nobody else still holds go back to the pool
            // rather than being dropped, so the restart below can
            // refill one instead of allocating another.
            for (_, caps) in nlist.drain(..) {
                if std::rc::Rc::strong_count(&caps) == 1 {
                    scratch.pool.push(caps);
                }
            }
            // `seen` belongs to the list being built, and from here on
            // that is nlist: what it recorded for clist is spent.
            scratch.seen.fill(false);
            for thread in &clist {
                let (pc, caps) = thread.clone();
                let step = match &self.prog[pc] {
                    Inst::Char(c) => pos < text.len() && text[pos] == *c,
                    // A dot stops at a line end, as it does elsewhere.
                    Inst::Any => pos < text.len() && text[pos] != '\n',
                    Inst::Class(cl) => pos < text.len() && cl.matches(text[pos]),
                    Inst::Match => {
                        // Threads are in priority order, so the first
                        // to match wins and the rest of this list is
                        // no longer interesting.
                        matched = Some(caps);
                        break;
                    }
                    _ => unreachable!("epsilon instruction left in a thread list"),
                };
                if step {
                    self.add(&mut nlist, &mut scratch, pc + 1, pos + 1, text, caps);
                }
            }
            // A fresh start at the next position, at lowest priority,
            // and only while nothing has matched: that is what makes
            // the search leftmost.
            if matched.is_none() && pos < text.len() {
                let fresh = empty_caps(&mut scratch, slots);
                self.add(&mut nlist, &mut scratch, 0, pos + 1, text, fresh);
            }
            if pos >= text.len() {
                break;
            }
            pos += 1;
            std::mem::swap(&mut clist, &mut nlist);
            if clist.is_empty() {
                break;
            }
        }
        // The winner's slots come back owned: unwrap when this is the
        // last reference, copy when other threads still share it.
        matched.map(|caps| std::rc::Rc::try_unwrap(caps).unwrap_or_else(|rc| (*rc).clone()))
    }

    /// Adds a thread and everything reachable from it without reading
    /// a character, in priority order, skipping any instruction this
    /// step has already reached.
    fn add(
        &self,
        list: &mut Vec<Thread>,
        scratch: &mut Scratch,
        pc: usize,
        pos: usize,
        text: &[char],
        caps: Caps,
    ) {
        let Scratch { seen, stack, .. } = scratch;
        stack.clear();
        stack.push((pc, caps));
        while let Some((pc, caps)) = stack.pop() {
            if seen[pc] {
                continue;
            }
            seen[pc] = true;
            match &self.prog[pc] {
                Inst::Jmp(t) => stack.push((*t, caps)),
                Inst::Split(a, b) => {
                    // Pushed second, popped first: the preferred
                    // branch is explored before the fallback.
                    stack.push((*b, caps.clone()));
                    stack.push((*a, caps));
                }
                Inst::Save(slot) => {
                    let mut caps = caps;
                    if *slot < caps.len() {
                        // The one place a thread's slots are written,
                        // and so the only place a copy can be needed.
                        std::rc::Rc::make_mut(&mut caps)[*slot] = Some(pos);
                    }
                    stack.push((pc + 1, caps));
                }
                Inst::Start => {
                    if pos == 0 {
                        stack.push((pc + 1, caps));
                    }
                }
                Inst::End => {
                    if pos == text.len() {
                        stack.push((pc + 1, caps));
                    }
                }
                _ => list.push((pc, caps)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    fn find(pattern: &str, text: &str) -> Option<(usize, usize)> {
        let re = Regex::new(pattern).unwrap();
        let caps = re.find_at(&chars(text), 0)?;
        Some((caps[0].unwrap(), caps[1].unwrap()))
    }

    #[test]
    fn literals_and_dots() {
        assert_eq!(find("abc", "xxabcxx"), Some((2, 5)));
        assert_eq!(find("abc", "ab"), None);
        assert_eq!(find("a.c", "abc"), Some((0, 3)));
        assert_eq!(find("a.c", "a\nc"), None);
        assert_eq!(find("", "abc"), Some((0, 0)));
    }

    #[test]
    fn positions_count_characters_not_bytes() {
        assert_eq!(find("b", "héllo b"), Some((6, 7)));
        assert_eq!(find("é", "héllo"), Some((1, 2)));
    }

    #[test]
    fn classes_ranges_and_shorthands() {
        assert_eq!(find("[abc]+", "xxbcax"), Some((2, 5)));
        assert_eq!(find("[^a-z]", "abcQd"), Some((3, 4)));
        assert_eq!(find(r"\d+", "abc 1234 z"), Some((4, 8)));
        assert_eq!(find(r"\w+", "  hi_there!"), Some((2, 10)));
        assert_eq!(find(r"\s", "ab cd"), Some((2, 3)));
        assert_eq!(find(r"[\d-]+", "x12-3y"), Some((1, 5)));
        assert_eq!(find("[]a]+", "x]aa"), Some((1, 4)));
        assert_eq!(find(r"[\n]", "a\nb"), Some((1, 2)));
    }

    #[test]
    fn anchors_hold_the_ends() {
        assert_eq!(find("^abc", "abcd"), Some((0, 3)));
        assert_eq!(find("^abc", "xabc"), None);
        assert_eq!(find("abc$", "xabc"), Some((1, 4)));
        assert_eq!(find("abc$", "abcx"), None);
        assert_eq!(find("^$", ""), Some((0, 0)));
    }

    #[test]
    fn quantifiers_greedy_and_lazy() {
        assert_eq!(find("a*", "aaa"), Some((0, 3)));
        assert_eq!(find("a*?", "aaa"), Some((0, 0)));
        assert_eq!(find("<.*>", "<a><b>"), Some((0, 6)));
        assert_eq!(find("<.*?>", "<a><b>"), Some((0, 3)));
        assert_eq!(find("a+", "baaa"), Some((1, 4)));
        assert_eq!(find("ab?c", "ac"), Some((0, 2)));
        assert_eq!(find("a{2,3}", "aaaa"), Some((0, 3)));
        assert_eq!(find("a{2}", "aaaa"), Some((0, 2)));
        assert_eq!(find("a{2,}", "aaaa"), Some((0, 4)));
        assert_eq!(find("a{2,3}?", "aaaa"), Some((0, 2)));
        // A brace that opens nothing countable is an ordinary one.
        assert_eq!(find("a{b}", "xa{b}"), Some((1, 5)));
    }

    #[test]
    fn alternation_prefers_the_earlier_branch() {
        assert_eq!(find("foo|foobar", "foobar"), Some((0, 3)));
        assert_eq!(find("foobar|foo", "foobar"), Some((0, 6)));
        assert_eq!(find("(?:ab|cd)+", "abcdx"), Some((0, 4)));
    }

    #[test]
    fn groups_capture_and_non_capturing_ones_do_not() {
        let re = Regex::new(r"(\w+)@(\w+)").unwrap();
        let caps = re.find_at(&chars("mail: me@here!"), 0).unwrap();
        assert_eq!(re.groups(), 2);
        assert_eq!((caps[2], caps[3]), (Some(6), Some(8)));
        assert_eq!((caps[4], caps[5]), (Some(9), Some(13)));

        let re = Regex::new("(?:a)(b)").unwrap();
        assert_eq!(re.groups(), 1);

        // A group that took no part in the match stays unset.
        let re = Regex::new("(a)|(b)").unwrap();
        let caps = re.find_at(&chars("b"), 0).unwrap();
        assert_eq!((caps[2], caps[4]), (None, Some(0)));
    }

    #[test]
    fn the_search_is_leftmost() {
        assert_eq!(find("a+", "bbaaa"), Some((2, 5)));
        let re = Regex::new("a").unwrap();
        assert_eq!(re.find_at(&chars("aXa"), 1).unwrap()[0], Some(2));
    }

    #[test]
    fn nothing_can_be_made_to_hang() {
        // The pattern that eats a backtracking engine alive.
        let text: String = "a".repeat(2000);
        let re = Regex::new("(a+)+b").unwrap();
        assert_eq!(re.find_at(&chars(&text), 0), None);
    }

    #[test]
    fn bad_patterns_are_refused_with_a_position() {
        for (pattern, message) in [
            ("(ab", "unclosed ( at 3"),
            ("ab)", "unmatched ) at 3"),
            ("[a", "unclosed [ at 2"),
            ("[]", "unclosed [ at 2"),
            ("[z-a]", "a range runs the wrong way at 4"),
            ("*a", "nothing to repeat at 1"),
            ("a{2,1}", "repetition counts are the wrong way round at 6"),
            ("a{1001}", "repetition count is too large at 7"),
            ("a\\", "a backslash needs something after it at 2"),
            ("^*", "an anchor cannot be repeated at 2"),
        ] {
            assert_eq!(Regex::new(pattern).unwrap_err(), message, "for {pattern}");
        }
        assert_eq!(
            Regex::new("(a{1000}){1000}").unwrap_err(),
            "pattern is too large"
        );
        // A second count has nothing left to count.
        assert_eq!(Regex::new("a{2}{3}").unwrap_err(), "nothing to repeat at 5");
    }
}
