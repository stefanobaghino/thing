//! expressions plus let/assign/index-assign/expression statements and
//! bare blocks. Everything else reports "not yet supported by --vm".

use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::lexer::Span;
use crate::value::Value;

#[derive(Debug, Clone)]
pub enum Op {
    /// Push consts[i].
    Const(u32),
    Nil,
    True,
    False,
    /// Push the variable's value (names[i]).
    GetVar(u32),
    /// Define names[i] in the current scope from the top of stack.
    Define(u32),
    /// Rebind names[i] from the top of stack.
    SetVar(u32),
    /// Push names[i], the read half of a compound assignment. Unlike
    /// GetVar, a missing name reports the assignment error, so
    /// `x += 1` and `x = x + 1` fail the same way `x = 1` does.
    GetVarToUpdate(u32),
    Unary(UnaryOp),
    Binary(BinaryOp),
    /// Pop n items into a fresh list.
    MakeList(u32),
    /// Pop 2n items (key/value pairs, in order) into a fresh map.
    MakeMap(u32),
    /// stack: [base, idx] -> [base[idx]]
    Index,
    /// stack: [base, idx] -> [base, idx, base[idx]] — the read half of
    /// a compound index assignment, which must not evaluate either
    /// operand a second time.
    IndexKeep,
    /// stack: [base, idx, value] -> []
    IndexSet,
    /// stack: [callee, arg0..argn-1] -> [result]; the span names the
    /// callee for not-callable errors (matching the tree-walker).
    Call(u8, Span),
    /// Error unless the top of stack is a list; the span names the
    /// expression that was spread.
    Spread(Span),
    /// Note that the statement starting at this offset ran. Emitted
    /// only when the chunk was compiled for coverage, so a plain run
    /// never executes one.
    Mark(usize),
    /// stack: [callee, list0..listn-1] -> [result]; the lists are
    /// concatenated in order into the arguments.
    CallSpread(u8, Span),
    /// Relative jump.
    Jump(i32),
    /// Pop a bool (strict); jump when false.
    JumpIfFalse(i32),
    /// Peek a bool (strict); jump when true, else pop. (for ||)
    OrJump(i32),
    /// Peek a bool (strict); jump when false, else pop. (for &&)
    AndJump(i32),
    /// Error unless the top of stack is a bool; leaves it in place.
    CheckBool,
    /// Error unless the top of stack is a string (a map key).
    CheckMapKey,
    Pop,
    /// Enter a fresh lexical scope.
    PushScope,
    /// Leave the current scope.
    PopScope,
    /// Pop the iterable, push the three loop slots: its snapshot
    /// list, nil, and an index of 0.
    IterNew,
    /// A for-loop over a call of n arguments whose callee is on the
    /// stack beneath them. When that callee turns out to BE the
    /// `range` builtin, the three loop slots become the counter, the
    /// limit and the step, and no list is built; otherwise the call
    /// happens and IterNew's slots are pushed instead. The check is at
    /// run time because `range` is an ordinary name a program may
    /// bind, and the REPL binds it in an earlier chunk than the loop.
    IterStart(u8),
    /// stack: [snap, nil, idx] or [cur, hi, step]. List: if idx ==
    /// len(snap) jump, else bump idx and push snap[idx]. Range: if the
    /// counter is past hi jump, else bump it by step and push what it
    /// was.
    IterNext(i32),
    /// Create a closure from protos[i], capturing the current env.
    MakeFn(u32),
    /// Pop the return value and leave the current function frame.
    Return,
    /// Push frame slot i (function locals resolved by the compiler).
    GetSlot(u16),
    /// Pop into frame slot i.
    SetSlot(u16),
    /// The write half of `slot op= rhs`: pop the right-hand side, take
    /// the slot's value out, apply the operator and put the answer
    /// back. Taking rather than reading means `s += c` appends to the
    /// string already in the slot instead of to a copy of it, and
    /// doing it after the right-hand side has run means `s += s` still
    /// sees the old s.
    UpdateSlot(u16, BinaryOp),
    /// Error unless names[i] is bound, the way GetVarToUpdate does,
    /// but without reading the value: the check has to happen before
    /// the right-hand side runs, and the read after it.
    CheckVar(u32),
    /// UpdateSlot for a name in the environment.
    UpdateVar(u32, BinaryOp),
    /// As CheckVar, but for the read half of `x = x + y`: the name is
    /// written as a read there, so an unbound one is reported as one.
    CheckVarRead(u32),
    /// `slot <op> literal` in one instruction. Reading a local and
    /// pushing a constant to compare them is the commonest three
    /// instructions in every program measured (LOG 800), and neither
    /// half can fail, so there is nothing to see in between.
    BinarySlotConst(u16, u32, BinaryOp),
    /// `slot <op> slot`, the same bargain with two locals: `i < n` in
    /// every loop, and the commonest three instructions in
    /// bench/stdlib.ting (LOG 801).
    BinarySlots(u16, u16, BinaryOp),
    /// The two above with the branch that follows them folded in:
    /// the answer to `c == ","` in an `if` or a `while` is looked at
    /// once and never reaches the stack. `JumpIfFalse` is a quarter
    /// of every instruction the CSV parse runs (LOG 802).
    JumpIfFalseSlotConst(u16, u32, BinaryOp, i32),
    JumpIfFalseSlots(u16, u16, BinaryOp, i32),
    /// Add the top of the stack to the value below it and store the
    /// result in the named binding -- Binary(Add) and SetVar in one
    /// step, so that the binding can be asked to let go of what was
    /// read before the add happens. It lets go only if it still holds
    /// that value, which is what keeps a right-hand side that
    /// reassigns the name working as it did.
    AppendVar(u32),
}

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<Op>,
    pub consts: Vec<Value>,
    pub names: Vec<String>,
    /// Frame slot count, at the top level as well as in a function.
    pub slots: u16,
    /// Where each parameter lives: Some(slot) or None (Env, captured).
    pub param_locs: Vec<Option<u16>>,
    /// Whether calls must allocate an Env frame (any captured binding).
    pub needs_env_frame: bool,
    /// Function bodies stay AST: the VM builds ordinary closures that
    /// the reference engine executes (docs/vm.md hybrid step).
    pub protos: Vec<FnProto>,
    /// spans[i] belongs to code[i]; used for diagnostics.
    pub spans: Vec<Span>,
    /// For each instruction that can raise "undefined variable", the
    /// innermost scope node there — the names in scope are that node
    /// and its ancestors. The tree-walker finds those in its
    /// environment and names the nearest one; a slot holds no name at
    /// runtime, so the compiler writes down what was in scope and the
    /// two engines say the same thing.
    ///
    /// A chain rather than a list per instruction: the list was
    /// copied for every instruction that can fail, so a program with
    /// n bindings and n statements copied n names n times. Nobody
    /// walks the chain until a diagnostic actually needs it.
    pub in_scope: Vec<(u32, Option<u32>)>,
    /// The chain itself: one node per slot-allocated binding, each
    /// naming the binding and pointing at the one enclosing it.
    pub scope_nodes: Vec<ScopeNode>,
}

/// One binding in the chain `in_scope` points into.
#[derive(Debug)]
pub struct ScopeNode {
    pub parent: Option<u32>,
    pub name: String,
}

impl Chunk {
    /// The slot-allocated names in scope at `ip`, for a diagnostic,
    /// outermost first as they were declared.
    pub fn in_scope_at(&self, ip: usize) -> Vec<String> {
        let Ok(i) = self
            .in_scope
            .binary_search_by_key(&(ip as u32), |(at, _)| *at)
        else {
            return Vec::new();
        };
        let mut names = Vec::new();
        let mut at = self.in_scope[i].1;
        while let Some(node) = at {
            let node = &self.scope_nodes[node as usize];
            names.push(node.name.clone());
            at = node.parent;
        }
        names.reverse();
        names
    }
}

#[derive(Debug)]
pub struct FnProto {
    /// The name the closure is bound to, when it is a `fn f(..)`
    /// definition rather than an anonymous literal: what a trace calls
    /// the frame.
    pub name: Option<String>,
    /// Where the literal starts, for a profile to name a line.
    pub def: Span,
    pub params: Vec<String>,
    /// The default for each parameter, carried so a compiled function
    /// fills a missing argument exactly as an interpreted one does.
    pub defaults: std::rc::Rc<Vec<Option<crate::ast::Expr>>>,
    /// True when the last parameter was written `...name`.
    pub rest: bool,
    pub chunk: std::rc::Rc<Chunk>,
}

pub struct CompileError {
    pub message: String,
    pub span: Span,
}

fn unsupported(what: &str, span: Span) -> CompileError {
    CompileError {
        message: format!("{what} is not yet supported by --vm"),
        span,
    }
}

pub fn compile_program(stmts: &[Stmt]) -> Result<Chunk, CompileError> {
    compile_stmts(stmts, None, false, false)
}

/// A module's top level, which is compiled but binds every name in the
/// environment. `import_module` reads a module's exports out of the
/// environment it ran in, and a frame slot holds no name — so the top
/// level of a module stays where the exports can be found, and only
/// its functions take slots. Nothing is given up for it: a module's
/// top level runs once, and the functions are where the work is.
pub fn compile_module(stmts: &[Stmt], coverage: bool) -> Result<Chunk, CompileError> {
    compile_stmts(stmts, None, coverage, true)
}

/// The same, with a `Mark` before every statement so a run can say
/// which ones happened.
pub fn compile_program_covered(stmts: &[Stmt]) -> Result<Chunk, CompileError> {
    compile_stmts(stmts, None, true, false)
}

/// Per-function resolver state: lexical scopes mapping names to frame
/// slots (or None for Env-allocated, i.e. captured, bindings).
struct FnCtx {
    scopes: Vec<Vec<(String, Option<u16>)>>,
    captured: std::collections::HashSet<String>,
    next_slot: u16,
    uses_env: bool,
    /// Where each name in scope is bound, innermost last, so
    /// resolving one is a lookup rather than a walk of every scope.
    /// A name can appear more than once: an inner binding shadows an
    /// outer one until its scope is left.
    at: std::collections::HashMap<String, Vec<Option<u16>>>,
    /// The innermost node of the scope chain, and the node each
    /// enclosing scope was left at.
    head: Option<u32>,
    heads: Vec<Option<u32>>,
}

fn compile_stmts(
    stmts: &[Stmt],
    func: Option<(&[crate::ast::Param], FnCtx)>,
    coverage: bool,
    env_top: bool,
) -> Result<Chunk, CompileError> {
    // A script's top level gets a resolver of its own. Its bindings are
    // as local as a function's — nothing outside the chunk reads them
    // by name, since the REPL runs on the tree-walker — so the ones no
    // nested closure captures live in frame slots instead of the
    // environment. A module's top level is the exception, and
    // `env_top` names it: its bindings ARE read from outside, by
    // `import_module`, which collects the exports out of the
    // environment the module ran in.
    let in_function = func.is_some();
    let (params, fn_ctx) = match func {
        Some((p, ctx)) => (p.to_vec(), Some(ctx)),
        None if env_top => (Vec::new(), None),
        None => {
            let mut captured = std::collections::HashSet::new();
            captured_names(stmts, &mut captured);
            let ctx = FnCtx {
                scopes: vec![Vec::new()],
                captured,
                next_slot: 0,
                uses_env: false,
                at: std::collections::HashMap::new(),
                head: None,
                heads: Vec::new(),
            };
            (Vec::new(), Some(ctx))
        }
    };
    let mut c = Compiler {
        chunk: Chunk {
            code: Vec::new(),
            consts: Vec::new(),
            names: Vec::new(),
            slots: 0,
            param_locs: Vec::new(),
            needs_env_frame: false,
            protos: Vec::new(),
            spans: Vec::new(),
            in_scope: Vec::new(),
            scope_nodes: Vec::new(),
        },
        loops: Vec::new(),
        scope_depth: 0,
        in_function,
        fn_ctx,
        coverage,
        const_at: std::collections::HashMap::new(),
        name_at: std::collections::HashMap::new(),
    };
    // Parameters are the function's outermost bindings.
    for p in &params {
        let loc = c.bind(&p.name);
        c.chunk.param_locs.push(loc);
    }
    for s in stmts {
        c.stmt(s)?;
    }
    if let Some(ctx) = &c.fn_ctx {
        c.chunk.slots = ctx.next_slot;
        c.chunk.needs_env_frame = ctx.uses_env;
    }
    Ok(c.chunk)
}

/// Every identifier mentioned inside nested fn literals of `stmts` —
/// a conservative over-approximation of what those closures capture.
fn captured_names(stmts: &[Stmt], out: &mut std::collections::HashSet<String>) {
    fn walk_stmt(s: &Stmt, in_fn: bool, out: &mut std::collections::HashSet<String>) {
        match &s.kind {
            StmtKind::Let(n, e) => {
                if in_fn {
                    out.insert(n.clone());
                }
                walk_expr(e, in_fn, out);
            }
            StmtKind::Assign(n, _, e) => {
                if in_fn {
                    out.insert(n.clone());
                }
                walk_expr(e, in_fn, out);
            }
            StmtKind::IndexAssign(a, b, _, c) => {
                walk_expr(a, in_fn, out);
                walk_expr(b, in_fn, out);
                walk_expr(c, in_fn, out);
            }
            StmtKind::Expr(e) => walk_expr(e, in_fn, out),
            StmtKind::Block(ss) => ss.iter().for_each(|s| walk_stmt(s, in_fn, out)),
            StmtKind::If(c, t, e) => {
                walk_expr(c, in_fn, out);
                walk_stmt(t, in_fn, out);
                if let Some(e) = e {
                    walk_stmt(e, in_fn, out);
                }
            }
            StmtKind::While(c, b) => {
                walk_expr(c, in_fn, out);
                walk_stmt(b, in_fn, out);
            }
            StmtKind::For(v, i, b) => {
                if in_fn {
                    out.insert(v.clone());
                }
                walk_expr(i, in_fn, out);
                walk_stmt(b, in_fn, out);
            }
            StmtKind::Break | StmtKind::Continue => {}
            StmtKind::Return(e) => {
                if let Some(e) = e {
                    walk_expr(e, in_fn, out);
                }
            }
        }
    }
    /// Every name an expression mentions, by worklist rather than by
    /// recursion. Statements nest no deeper than the parser allows,
    /// but an operator chain does not nest at all syntactically and
    /// still leans left as a tree, so a long enough one walked this
    /// by 500000 host frames and killed the process (854, 857).
    fn walk_expr(root: &Expr, root_in_fn: bool, out: &mut std::collections::HashSet<String>) {
        let mut todo = vec![(root, root_in_fn)];
        while let Some((e, in_fn)) = todo.pop() {
            match &e.kind {
                ExprKind::Var(n) => {
                    if in_fn {
                        out.insert(n.clone());
                    }
                }
                ExprKind::List(xs) => todo.extend(xs.iter().map(|x| (x, in_fn))),
                ExprKind::Map(kvs) => {
                    for (k, v) in kvs {
                        todo.push((k, in_fn));
                        todo.push((v, in_fn));
                    }
                }
                ExprKind::Unary(_, x) | ExprKind::Spread(x) => todo.push((x, in_fn)),
                ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
                    todo.push((a, in_fn));
                    todo.push((b, in_fn));
                }
                ExprKind::Call(c, args) => {
                    todo.push((c, in_fn));
                    todo.extend(args.iter().map(|a| (a, in_fn)));
                }
                // Everything inside a nested fn literal is "captured".
                ExprKind::Fn(params, body) => {
                    if in_fn {
                        out.extend(params.iter().map(|p| p.name.clone()));
                    }
                    // A default is evaluated at the call, against the
                    // closure's env, so whatever it names has to live
                    // there rather than in a slot of the frame that
                    // made the closure.
                    for p in params {
                        if let Some(d) = &p.default {
                            todo.push((d, true));
                        }
                    }
                    body.iter().for_each(|s| walk_stmt(s, true, out));
                }
                _ => {}
            }
        }
    }
    stmts.iter().for_each(|s| walk_stmt(s, false, out));
}

/// Per-loop bookkeeping for break/continue lowering.
struct LoopCtx {
    /// Where continue jumps (cond for while, IterNext for for).
    continue_target: usize,
    /// Jump ops to patch to the loop's end.
    break_patches: Vec<usize>,
    /// Compiler scope depth just outside the loop body; break/continue
    /// emit PopScope down to it before jumping. Break needs no extra
    /// stack cleanup: a for-loop's [snapshot, index] slots are popped
    /// at the shared end label both break and exhaustion jump to.
    scope_depth: usize,
}

/// A scalar literal by value, for finding it in the constant pool
/// without walking the pool. Only the three kinds `konst` dedups have
/// a key; a float is keyed by its bits, as the pool compares them.
#[derive(PartialEq, Eq, Hash)]
enum ConstKey {
    Int(i64),
    Str(String),
    Float(u64),
}

impl ConstKey {
    fn of(v: &Value) -> Option<ConstKey> {
        match v {
            Value::Int(n) => Some(ConstKey::Int(*n)),
            Value::Str(t) => Some(ConstKey::Str(t.as_str().to_string())),
            Value::Float(x) => Some(ConstKey::Float(x.to_bits())),
            _ => None,
        }
    }
}

struct Compiler {
    chunk: Chunk,
    loops: Vec<LoopCtx>,
    scope_depth: usize,
    in_function: bool,
    fn_ctx: Option<FnCtx>,
    /// Emit a `Mark` before every statement, for `--coverage`.
    coverage: bool,
    /// Where each pooled constant and name already sits. The pools
    /// used to be searched by scanning them, under a comment saying
    /// they stay tiny — true of every program in the corpus and false
    /// of a generated one. Measured at 843: 8000 functions cost
    /// 650 ms to compile against the tree-walker's 80 ms to run them,
    /// and `--check` paid it too, since it compiles to find static
    /// errors.
    const_at: std::collections::HashMap<ConstKey, u32>,
    name_at: std::collections::HashMap<String, u32>,
}

/// The value of a literal, or `None` for anything that has to be run
/// to know. Only literals may be folded into the instruction that
/// uses them: nothing else is guaranteed to be free of effects.
fn literal_value(e: &Expr) -> Option<Value> {
    match &e.kind {
        ExprKind::Int(n) => Some(Value::Int(*n)),
        ExprKind::Float(x) => Some(Value::Float(*x)),
        ExprKind::Str(t) => Some(Value::str(t.clone())),
        ExprKind::Bool(b) => Some(Value::Bool(*b)),
        ExprKind::Nil => Some(Value::Nil),
        _ => None,
    }
}

impl Compiler {
    fn emit(&mut self, op: Op, span: Span) {
        self.chunk.code.push(op);
        self.chunk.spans.push(span);
    }

    fn konst(&mut self, v: Value) -> u32 {
        // Scalar literals are deduped; anything else is pushed as it
        // comes, exactly as when the pool was searched by scanning it.
        let Some(key) = ConstKey::of(&v) else {
            self.chunk.consts.push(v);
            return (self.chunk.consts.len() - 1) as u32;
        };
        if let Some(&i) = self.const_at.get(&key) {
            return i;
        }
        self.chunk.consts.push(v);
        let at = (self.chunk.consts.len() - 1) as u32;
        self.const_at.insert(key, at);
        at
    }

    /// Note the slot names in scope for the instruction just emitted,
    /// which is one that can fail with "undefined variable".
    fn note_scope(&mut self) {
        let Some(ctx) = &self.fn_ctx else { return };
        let Some(head) = ctx.head else { return };
        let at = (self.chunk.code.len() - 1) as u32;
        self.chunk.in_scope.push((at, Some(head)));
    }

    fn name(&mut self, n: &str) -> u32 {
        if let Some(&i) = self.name_at.get(n) {
            return i;
        }
        self.chunk.names.push(n.to_string());
        let at = (self.chunk.names.len() - 1) as u32;
        self.name_at.insert(n.to_string(), at);
        at
    }

    /// Bind a fresh local: a frame slot when possible, Env when the
    /// name is captured by a nested closure.
    fn bind(&mut self, n: &str) -> Option<u16> {
        let Some(ctx) = &mut self.fn_ctx else {
            return None;
        };
        let loc = if ctx.captured.contains(n) {
            ctx.uses_env = true;
            None
        } else {
            let slot = ctx.next_slot;
            ctx.next_slot += 1;
            Some(slot)
        };
        ctx.scopes
            .last_mut()
            .expect("resolver scope")
            .push((n.to_string(), loc));
        ctx.at.entry(n.to_string()).or_default().push(loc);
        // Only a slot-allocated name joins the chain a diagnostic
        // reads: the others are in the environment, where the
        // tree-walker finds them by name anyway.
        if loc.is_some() {
            self.chunk.scope_nodes.push(ScopeNode {
                parent: self.fn_ctx.as_ref().expect("resolver context").head,
                name: n.to_string(),
            });
            let node = (self.chunk.scope_nodes.len() - 1) as u32;
            self.fn_ctx.as_mut().expect("resolver context").head = Some(node);
        }
        loc
    }

    /// Resolve a name: innermost local first, else Env (outer/global).
    /// A name bound nowhere and a name bound in the environment both
    /// answer None, as they did when this walked the scopes.
    fn resolve(&self, n: &str) -> Option<u16> {
        let ctx = self.fn_ctx.as_ref()?;
        ctx.at.get(n).and_then(|bound| bound.last().copied())?
    }

    /// Whether a binary node with this left side is one the
    /// superinstructions below fuse: both of them want a left side
    /// that is a local, so anything else is compiled the long way and
    /// is safe to walk iteratively.
    fn fusible(&self, lhs: &Expr) -> bool {
        matches!(&lhs.kind, ExprKind::Var(n) if self.resolve(n).is_some())
    }

    fn enter_scope(&mut self) {
        if let Some(ctx) = &mut self.fn_ctx {
            ctx.scopes.push(Vec::new());
            ctx.heads.push(ctx.head);
        }
    }

    fn leave_scope(&mut self) {
        if let Some(ctx) = &mut self.fn_ctx {
            // Every name this scope bound stops shadowing whatever it
            // hid, and the chain goes back to where the scope began.
            if let Some(gone) = ctx.scopes.pop() {
                for (name, _) in gone {
                    let empty = match ctx.at.get_mut(&name) {
                        Some(bound) => {
                            bound.pop();
                            bound.is_empty()
                        }
                        None => false,
                    };
                    if empty {
                        ctx.at.remove(&name);
                    }
                }
            }
            if let Some(head) = ctx.heads.pop() {
                ctx.head = head;
            }
        }
    }

    /// Does this block need a runtime Env scope? Only when it directly
    /// declares an Env-allocated (captured) binding.
    fn block_needs_env(&self, stmts: &[Stmt]) -> bool {
        stmts.iter().any(|st| match &st.kind {
            // Every top-level `let` binds a name in the environment,
            // slot or not, so the block it sits in needs a scope to
            // pop — that is what restores a shadowed builtin.
            StmtKind::Let(n, _) if !self.in_function => {
                let _ = n;
                true
            }
            StmtKind::Let(n, _) => match &self.fn_ctx {
                Some(ctx) => ctx.captured.contains(n),
                None => true,
            },
            _ => false,
        })
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), CompileError> {
        if self.coverage {
            self.emit(Op::Mark(s.span.start), s.span);
        }
        match &s.kind {
            StmtKind::Let(name, init) => {
                // `fn f(..) {..}` parses as a let of a fn literal, so
                // this is where a function learns its name.
                match &init.kind {
                    ExprKind::Fn(params, body) => {
                        self.closure(params, body, init.span, Some(name))?
                    }
                    _ => self.expr(init)?,
                }
                match self.bind(name) {
                    Some(slot) => {
                        self.emit(Op::SetSlot(slot), s.span);
                        // At the top level the environment is also what
                        // every diagnostic means by "in scope": it is
                        // where the nearest-name suggestion looks. A
                        // slot holds no name, so the name is bound here
                        // too, to nil. Nothing can read it — a name any
                        // closure mentions is captured, and captured
                        // names never get a slot — so the binding is
                        // only ever a name, and both engines go on
                        // seeing the same scope.
                        if !self.in_function {
                            let i = self.name(name);
                            self.emit(Op::Nil, s.span);
                            self.emit(Op::Define(i), s.span);
                        }
                    }
                    None => {
                        let i = self.name(name);
                        self.emit(Op::Define(i), s.span);
                    }
                }
            }
            StmtKind::Assign(name, op, value) => {
                let slot = self.resolve(name);
                if let Some(op) = op {
                    // Read, operate and write are one instruction, so
                    // the old value is moved out of its home rather
                    // than copied onto the stack. The name check for a
                    // variable still comes first, because `x += 1` has
                    // to fail the way `x = 1` does before the
                    // right-hand side runs.
                    // A frame slot is a binding no nested closure
                    // even mentions, so nothing a call could do
                    // reaches it: the right-hand side may be
                    // anything, calls included. An Env binding is
                    // reachable by any function, so there the
                    // expression must not name it.
                    let fuse = *op == BinaryOp::Add
                        && (slot.is_some() || crate::eval::cannot_reach(value, name));
                    match (slot, fuse) {
                        (Some(slot), true) => {
                            self.expr(value)?;
                            self.emit(Op::UpdateSlot(slot, *op), s.span);
                        }
                        (None, true) => {
                            let i = self.name(name);
                            self.emit(Op::CheckVar(i), s.span);
                            self.note_scope();
                            self.expr(value)?;
                            self.emit(Op::UpdateVar(i, *op), s.span);
                            self.note_scope();
                        }
                        (None, false) if *op == BinaryOp::Add => {
                            let i = self.name(name);
                            self.emit(Op::GetVarToUpdate(i), s.span);
                            self.note_scope();
                            self.expr(value)?;
                            self.emit(Op::AppendVar(i), s.span);
                            self.note_scope();
                        }
                        (slot, false) => {
                            match slot {
                                Some(slot) => self.emit(Op::GetSlot(slot), s.span),
                                None => {
                                    let i = self.name(name);
                                    self.emit(Op::GetVarToUpdate(i), s.span);
                                    self.note_scope();
                                }
                            }
                            self.expr(value)?;
                            self.emit(Op::Binary(*op), s.span);
                            match slot {
                                Some(slot) => self.emit(Op::SetSlot(slot), s.span),
                                None => {
                                    let i = self.name(name);
                                    self.emit(Op::SetVar(i), s.span);
                                    self.note_scope();
                                }
                            }
                        }
                    }
                } else if let Some((rhs, read)) = crate::eval::folds_into_append(name, value)
                    .filter(|(rhs, _)| slot.is_some() || crate::eval::cannot_reach(rhs, name))
                {
                    // The long spelling of `x += y`, fused the same
                    // way. The operator's span rides the instruction,
                    // so a type error still points at `x + y` rather
                    // than at the whole statement.
                    match slot {
                        Some(slot) => {
                            self.expr(rhs)?;
                            self.emit(Op::UpdateSlot(slot, BinaryOp::Add), value.span);
                        }
                        None => {
                            let i = self.name(name);
                            self.emit(Op::CheckVarRead(i), read);
                            self.note_scope();
                            self.expr(rhs)?;
                            self.emit(Op::UpdateVar(i, BinaryOp::Add), value.span);
                            self.note_scope();
                        }
                    }
                } else if slot.is_none()
                    && let Some((rhs, read)) = crate::eval::folds_into_append(name, value)
                {
                    // `x = x + f()`, where the call could reassign
                    // `x`. The read is an ordinary read, at its own
                    // span, and comes first as it always did; what
                    // follows lets the binding let go of it once the
                    // call has had its chance.
                    let i = self.name(name);
                    self.emit(Op::GetVar(i), read);
                    self.note_scope();
                    self.expr(rhs)?;
                    self.emit(Op::AppendVar(i), value.span);
                    self.note_scope();
                } else {
                    self.expr(value)?;
                    match slot {
                        Some(slot) => self.emit(Op::SetSlot(slot), s.span),
                        None => {
                            let i = self.name(name);
                            self.emit(Op::SetVar(i), s.span);
                            self.note_scope();
                        }
                    }
                }
            }
            StmtKind::IndexAssign(base, idx, op, value) => {
                self.expr(base)?;
                self.expr(idx)?;
                if let Some(op) = op {
                    // IndexKeep leaves base and index where they are,
                    // so the write below uses the ones already read.
                    self.emit(Op::IndexKeep, s.span);
                    self.expr(value)?;
                    self.emit(Op::Binary(*op), s.span);
                } else {
                    self.expr(value)?;
                }
                self.emit(Op::IndexSet, s.span);
            }
            StmtKind::Expr(e) => {
                self.expr(e)?;
                self.emit(Op::Pop, s.span);
            }
            // A runtime Env scope only when the block directly declares
            // an Env-allocated binding; slot locals need no scope ops.
            StmtKind::Block(stmts) => {
                let scoped = self.block_needs_env(stmts);
                self.enter_scope();
                if scoped {
                    self.emit(Op::PushScope, s.span);
                    self.scope_depth += 1;
                }
                for st in stmts {
                    self.stmt(st)?;
                }
                if scoped {
                    self.scope_depth -= 1;
                    self.emit(Op::PopScope, s.span);
                }
                self.leave_scope();
            }
            StmtKind::If(cond, then, els) => {
                let to_else = self.jump_if_false(cond)?;
                self.stmt(then)?;
                match els {
                    Some(els) => {
                        let to_end = self.chunk.code.len();
                        self.emit(Op::Jump(0), s.span);
                        let here = self.chunk.code.len() as i32;
                        self.patch(to_else, here);
                        self.stmt(els)?;
                        let here = self.chunk.code.len() as i32;
                        self.patch(to_end, here);
                    }
                    None => {
                        let here = self.chunk.code.len() as i32;
                        self.patch(to_else, here);
                    }
                }
            }
            StmtKind::While(cond, body) => {
                let loop_start = self.chunk.code.len();
                let to_end = self.jump_if_false(cond)?;
                self.loops.push(LoopCtx {
                    continue_target: loop_start,
                    break_patches: vec![to_end],
                    scope_depth: self.scope_depth,
                });
                self.stmt(body)?;
                let back = self.chunk.code.len();
                self.emit(Op::Jump(0), s.span);
                self.patch(back, loop_start as i32);
                let ctx = self.loops.pop().expect("loop ctx");
                let end = self.chunk.code.len() as i32;
                for at in ctx.break_patches {
                    self.patch(at, end);
                }
            }
            StmtKind::For(var, iterable, body) => {
                // `for x in range(...)` counts instead of building the
                // list, when the callee turns out to be the builtin —
                // which IterStart decides, because `range` is a name a
                // program may bind. A spread makes the argument count a
                // runtime fact, so those keep the general path.
                match &iterable.kind {
                    ExprKind::Call(callee, args)
                        if (1..=3).contains(&args.len())
                            && !args.iter().any(|a| matches!(a.kind, ExprKind::Spread(_)))
                            && matches!(&callee.kind, ExprKind::Var(n) if n == "range") =>
                    {
                        self.expr(callee)?;
                        for a in args {
                            self.expr(a)?;
                        }
                        self.emit(Op::IterStart(args.len() as u8), iterable.span);
                    }
                    _ => {
                        self.expr(iterable)?;
                        self.emit(Op::IterNew, iterable.span);
                    }
                }
                let next_ip = self.chunk.code.len();
                self.emit(Op::IterNext(0), s.span);
                self.loops.push(LoopCtx {
                    continue_target: next_ip,
                    break_patches: vec![next_ip],
                    scope_depth: self.scope_depth,
                });
                self.enter_scope();
                match self.bind(var) {
                    // Uncaptured loop var: a slot reused per iteration
                    // is observationally identical to a fresh binding.
                    Some(slot) => {
                        self.emit(Op::SetSlot(slot), s.span);
                        self.stmt(body)?;
                    }
                    // Captured (or top-level): fresh scope per
                    // iteration, like the tree-walker.
                    None => {
                        self.emit(Op::PushScope, s.span);
                        self.scope_depth += 1;
                        let vi = self.name(var);
                        self.emit(Op::Define(vi), s.span);
                        self.stmt(body)?;
                        self.scope_depth -= 1;
                        self.emit(Op::PopScope, s.span);
                    }
                }
                self.leave_scope();
                let back = self.chunk.code.len();
                self.emit(Op::Jump(0), s.span);
                self.patch(back, next_ip as i32);
                let ctx = self.loops.pop().expect("loop ctx");
                let end = self.chunk.code.len() as i32;
                for at in ctx.break_patches {
                    self.patch(at, end);
                }
                // The loop owned three slots on the stack; both
                // jump-to-end paths (done and break) land here.
                self.emit(Op::Pop, s.span);
                self.emit(Op::Pop, s.span);
                self.emit(Op::Pop, s.span);
            }
            StmtKind::Break => {
                let Some(ctx_depth) = self.loops.last().map(|c| c.scope_depth) else {
                    return Err(CompileError {
                        message: "break outside loop".to_string(),
                        span: s.span,
                    });
                };
                for _ in ctx_depth..self.scope_depth {
                    self.emit(Op::PopScope, s.span);
                }
                let at = self.chunk.code.len();
                self.emit(Op::Jump(0), s.span);
                self.loops
                    .last_mut()
                    .expect("loop ctx")
                    .break_patches
                    .push(at);
            }
            StmtKind::Continue => {
                let Some(ctx_depth) = self.loops.last().map(|c| c.scope_depth) else {
                    return Err(CompileError {
                        message: "continue outside loop".to_string(),
                        span: s.span,
                    });
                };
                for _ in ctx_depth..self.scope_depth {
                    self.emit(Op::PopScope, s.span);
                }
                let target = self.loops.last().expect("loop ctx").continue_target;
                let at = self.chunk.code.len();
                self.emit(Op::Jump(0), s.span);
                self.patch(at, target as i32);
            }
            StmtKind::Return(value) => {
                if !self.in_function {
                    // Same message as the tree-walker, surfaced at
                    // compile time (accepted divergence).
                    return Err(CompileError {
                        message: "return outside function".to_string(),
                        span: s.span,
                    });
                }
                match value {
                    Some(e) => self.expr(e)?,
                    None => self.emit(Op::Nil, s.span),
                }
                self.emit(Op::Return, s.span);
            }
        }
        Ok(())
    }

    fn expr(&mut self, e: &Expr) -> Result<(), CompileError> {
        match &e.kind {
            ExprKind::Int(n) => {
                let i = self.konst(Value::Int(*n));
                self.emit(Op::Const(i), e.span);
            }
            ExprKind::Float(x) => {
                let i = self.konst(Value::Float(*x));
                self.emit(Op::Const(i), e.span);
            }
            ExprKind::Str(s) => {
                let i = self.konst(Value::str(s.clone()));
                self.emit(Op::Const(i), e.span);
            }
            ExprKind::Bool(true) => self.emit(Op::True, e.span),
            ExprKind::Bool(false) => self.emit(Op::False, e.span),
            ExprKind::Nil => self.emit(Op::Nil, e.span),
            ExprKind::Var(name) => match self.resolve(name) {
                Some(slot) => self.emit(Op::GetSlot(slot), e.span),
                None => {
                    let i = self.name(name);
                    self.emit(Op::GetVar(i), e.span);
                    self.note_scope();
                }
            },
            ExprKind::List(items) => {
                for it in items {
                    self.expr(it)?;
                }
                self.emit(Op::MakeList(items.len() as u32), e.span);
            }
            ExprKind::Map(entries) => {
                for (k, v) in entries {
                    self.expr(k)?;
                    self.emit(Op::CheckMapKey, k.span);
                    self.expr(v)?;
                }
                self.emit(Op::MakeMap(entries.len() as u32), e.span);
            }
            ExprKind::Unary(op, operand) => {
                self.expr(operand)?;
                self.emit(Op::Unary(*op), e.span);
            }
            ExprKind::Binary(BinaryOp::And, lhs, rhs) => {
                self.expr(lhs)?;
                let patch = self.chunk.code.len();
                self.emit(Op::AndJump(0), lhs.span);
                self.expr(rhs)?;
                self.emit(Op::CheckBool, rhs.span);
                let target = self.chunk.code.len() as i32;
                self.patch(patch, target);
            }
            ExprKind::Binary(BinaryOp::Or, lhs, rhs) => {
                self.expr(lhs)?;
                let patch = self.chunk.code.len();
                self.emit(Op::OrJump(0), lhs.span);
                self.expr(rhs)?;
                self.emit(Op::CheckBool, rhs.span);
                let target = self.chunk.code.len() as i32;
                self.patch(patch, target);
            }
            // A left spine of plain binaries, emitted without
            // recursing down it: `a + b + c + ...` leans left, and one
            // host frame per term is what killed an unoptimized build
            // at 10000 terms (854). The descent stops wherever a node
            // could fuse, so the superinstructions below still see the
            // shapes they match on.
            ExprKind::Binary(op, lhs, rhs)
                if !matches!(op, BinaryOp::And | BinaryOp::Or)
                    && !self.fusible(lhs)
                    && matches!(&lhs.kind, ExprKind::Binary(op, l, _)
                        if !matches!(op, BinaryOp::And | BinaryOp::Or) && !self.fusible(l)) =>
            {
                let mut spine = vec![(*op, rhs, e.span)];
                let mut node = lhs;
                while let ExprKind::Binary(op, l, r) = &node.kind {
                    if matches!(op, BinaryOp::And | BinaryOp::Or) || self.fusible(l) {
                        break;
                    }
                    spine.push((*op, r, node.span));
                    node = l;
                }
                self.expr(node)?;
                while let Some((op, rhs, span)) = spine.pop() {
                    self.expr(rhs)?;
                    self.emit(Op::Binary(op), span);
                }
            }
            ExprKind::Binary(op, lhs, rhs) => {
                let slot = match &lhs.kind {
                    ExprKind::Var(n) => self.resolve(n),
                    _ => None,
                };
                let right = match &rhs.kind {
                    ExprKind::Var(n) => self.resolve(n),
                    _ => None,
                };
                match (slot, right, literal_value(rhs)) {
                    (Some(slot), _, Some(v)) => {
                        let k = self.konst(v);
                        self.emit(Op::BinarySlotConst(slot, k, *op), e.span);
                    }
                    (Some(a), Some(b), None) => {
                        self.emit(Op::BinarySlots(a, b, *op), e.span);
                    }
                    _ => {
                        self.expr(lhs)?;
                        self.expr(rhs)?;
                        self.emit(Op::Binary(*op), e.span);
                    }
                }
            }
            ExprKind::Index(base, idx) => {
                self.expr(base)?;
                self.expr(idx)?;
                self.emit(Op::Index, e.span);
            }
            ExprKind::Call(callee, args) => {
                self.expr(callee)?;
                if args.len() > u8::MAX as usize {
                    return Err(unsupported("more than 255 arguments", e.span));
                }
                // A spread makes the argument count a runtime fact, so
                // every argument becomes a list and the call flattens
                // them. Calls without one keep the direct path.
                if args.iter().any(|a| matches!(a.kind, ExprKind::Spread(_))) {
                    for a in args {
                        match &a.kind {
                            ExprKind::Spread(inner) => {
                                self.expr(inner)?;
                                self.emit(Op::Spread(inner.span), a.span);
                            }
                            _ => {
                                self.expr(a)?;
                                self.emit(Op::MakeList(1), a.span);
                            }
                        }
                    }
                    self.emit(Op::CallSpread(args.len() as u8, callee.span), e.span);
                } else {
                    for a in args {
                        self.expr(a)?;
                    }
                    self.emit(Op::Call(args.len() as u8, callee.span), e.span);
                }
            }
            ExprKind::Spread(_) => {
                return Err(unsupported("'...' outside a call", e.span));
            }
            ExprKind::Fn(params, body) => self.closure(params, body, e.span, None)?,
        }
        Ok(())
    }

    /// Compile a fn literal into a proto and emit the MakeFn for it.
    fn closure(
        &mut self,
        params: &[crate::ast::Param],
        body: &std::rc::Rc<Vec<Stmt>>,
        span: Span,
        name: Option<&str>,
    ) -> Result<(), CompileError> {
        let mut captured = std::collections::HashSet::new();
        captured_names(body, &mut captured);
        let ctx = FnCtx {
            scopes: vec![Vec::new()],
            captured,
            next_slot: 0,
            uses_env: false,
            at: std::collections::HashMap::new(),
            head: None,
            heads: Vec::new(),
        };
        let chunk = compile_stmts(body, Some((params, ctx)), self.coverage, false)?;
        self.chunk.protos.push(FnProto {
            name: name.map(str::to_string),
            def: span,
            params: params.iter().map(|p| p.name.clone()).collect(),
            defaults: std::rc::Rc::new(params.iter().map(|p| p.default.clone()).collect()),
            rest: params.last().is_some_and(|p| p.rest),
            chunk: std::rc::Rc::new(chunk),
        });
        let i = (self.chunk.protos.len() - 1) as u32;
        self.emit(Op::MakeFn(i), span);
        Ok(())
    }

    /// Emit the test for a condition and the jump that skips the body
    /// when it is false, and answer where to patch the offset. A
    /// comparison against a local or a literal becomes ONE
    /// instruction: the answer is looked at where it is made instead
    /// of being pushed, popped and asked whether it is true. It is
    /// still asked -- `if x + 1 {}` is an error, and the same one.
    fn jump_if_false(&mut self, cond: &Expr) -> Result<usize, CompileError> {
        if let ExprKind::Binary(op, lhs, rhs) = &cond.kind
            && !matches!(op, BinaryOp::And | BinaryOp::Or)
            && let ExprKind::Var(n) = &lhs.kind
            && let Some(a) = self.resolve(n)
        {
            let right = match &rhs.kind {
                ExprKind::Var(n) => self.resolve(n),
                _ => None,
            };
            let at = self.chunk.code.len();
            if let Some(v) = literal_value(rhs) {
                let k = self.konst(v);
                self.emit(Op::JumpIfFalseSlotConst(a, k, *op, 0), cond.span);
                return Ok(at);
            }
            if let Some(b) = right {
                self.emit(Op::JumpIfFalseSlots(a, b, *op, 0), cond.span);
                return Ok(at);
            }
        }
        self.expr(cond)?;
        let at = self.chunk.code.len();
        self.emit(Op::JumpIfFalse(0), cond.span);
        Ok(at)
    }

    fn patch(&mut self, at: usize, target: i32) {
        let rel = target - at as i32 - 1;
        match &mut self.chunk.code[at] {
            Op::Jump(o)
            | Op::JumpIfFalse(o)
            | Op::OrJump(o)
            | Op::AndJump(o)
            | Op::IterNext(o)
            | Op::JumpIfFalseSlotConst(_, _, _, o)
            | Op::JumpIfFalseSlots(_, _, _, o) => *o = rel,
            _ => unreachable!("patched op is not a jump"),
        }
    }
}
