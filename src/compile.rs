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
    /// Pop the iterable, push its snapshot list (for-loop semantics).
    IterNew,
    /// stack: [snap, idx]. If idx == len(snap): jump. Else: bump idx
    /// and push snap[idx].
    IterNext(i32),
    /// Create a closure from protos[i], capturing the current env.
    MakeFn(u32),
    /// Pop the return value and leave the current function frame.
    Return,
    /// Push frame slot i (function locals resolved by the compiler).
    GetSlot(u16),
    /// Pop into frame slot i.
    SetSlot(u16),
}

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<Op>,
    pub consts: Vec<Value>,
    pub names: Vec<String>,
    /// Frame slot count (function chunks; 0 at top level).
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
    compile_stmts(stmts, None, false)
}

/// The same, with a `Mark` before every statement so a run can say
/// which ones happened.
pub fn compile_program_covered(stmts: &[Stmt]) -> Result<Chunk, CompileError> {
    compile_stmts(stmts, None, true)
}

/// Per-function resolver state: lexical scopes mapping names to frame
/// slots (or None for Env-allocated, i.e. captured, bindings).
struct FnCtx {
    scopes: Vec<Vec<(String, Option<u16>)>>,
    captured: std::collections::HashSet<String>,
    next_slot: u16,
    uses_env: bool,
}

fn compile_stmts(
    stmts: &[Stmt],
    func: Option<(&[crate::ast::Param], FnCtx)>,
    coverage: bool,
) -> Result<Chunk, CompileError> {
    let (params, fn_ctx) = match func {
        Some((p, ctx)) => (p.to_vec(), Some(ctx)),
        None => (Vec::new(), None),
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
        },
        loops: Vec::new(),
        scope_depth: 0,
        in_function: fn_ctx.is_some(),
        fn_ctx,
        coverage,
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
    fn walk_expr(e: &Expr, in_fn: bool, out: &mut std::collections::HashSet<String>) {
        match &e.kind {
            ExprKind::Var(n) => {
                if in_fn {
                    out.insert(n.clone());
                }
            }
            ExprKind::List(xs) => xs.iter().for_each(|x| walk_expr(x, in_fn, out)),
            ExprKind::Map(kvs) => kvs.iter().for_each(|(k, v)| {
                walk_expr(k, in_fn, out);
                walk_expr(v, in_fn, out);
            }),
            ExprKind::Unary(_, x) => walk_expr(x, in_fn, out),
            ExprKind::Binary(_, a, b) => {
                walk_expr(a, in_fn, out);
                walk_expr(b, in_fn, out);
            }
            ExprKind::Index(a, b) => {
                walk_expr(a, in_fn, out);
                walk_expr(b, in_fn, out);
            }
            ExprKind::Call(c, args) => {
                walk_expr(c, in_fn, out);
                args.iter().for_each(|a| walk_expr(a, in_fn, out));
            }
            ExprKind::Spread(x) => walk_expr(x, in_fn, out),
            // Everything inside a nested fn literal is "captured".
            ExprKind::Fn(params, body) => {
                if in_fn {
                    out.extend(params.iter().map(|p| p.name.clone()));
                }
                // A default is evaluated at the call, against the
                // closure's env, so whatever it names has to live
                // there rather than in a slot of the frame that made
                // the closure.
                for p in params {
                    if let Some(d) = &p.default {
                        walk_expr(d, true, out);
                    }
                }
                body.iter().for_each(|s| walk_stmt(s, true, out));
            }
            _ => {}
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

struct Compiler {
    chunk: Chunk,
    loops: Vec<LoopCtx>,
    scope_depth: usize,
    in_function: bool,
    fn_ctx: Option<FnCtx>,
    /// Emit a `Mark` before every statement, for `--coverage`.
    coverage: bool,
}

impl Compiler {
    fn emit(&mut self, op: Op, span: Span) {
        self.chunk.code.push(op);
        self.chunk.spans.push(span);
    }

    fn konst(&mut self, v: Value) -> u32 {
        // Dedup scalar literals; the pool stays tiny so a scan is fine.
        let dup = self.chunk.consts.iter().position(|c| match (c, &v) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        });
        if let Some(i) = dup {
            return i as u32;
        }
        self.chunk.consts.push(v);
        (self.chunk.consts.len() - 1) as u32
    }

    fn name(&mut self, n: &str) -> u32 {
        if let Some(i) = self.chunk.names.iter().position(|x| x == n) {
            return i as u32;
        }
        self.chunk.names.push(n.to_string());
        (self.chunk.names.len() - 1) as u32
    }

    /// Bind a fresh local: a frame slot when possible, Env when the
    /// name is captured by a nested closure (or at top level).
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
        loc
    }

    /// Resolve a name: innermost local first, else Env (outer/global).
    fn resolve(&self, n: &str) -> Option<u16> {
        let ctx = self.fn_ctx.as_ref()?;
        for scope in ctx.scopes.iter().rev() {
            for (name, loc) in scope.iter().rev() {
                if name == n {
                    return *loc;
                }
            }
        }
        None
    }

    fn enter_scope(&mut self) {
        if let Some(ctx) = &mut self.fn_ctx {
            ctx.scopes.push(Vec::new());
        }
    }

    fn leave_scope(&mut self) {
        if let Some(ctx) = &mut self.fn_ctx {
            ctx.scopes.pop();
        }
    }

    /// Does this block need a runtime Env scope? Only when it directly
    /// declares an Env-allocated (captured) binding.
    fn block_needs_env(&self, stmts: &[Stmt]) -> bool {
        stmts.iter().any(|st| match &st.kind {
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
                    Some(slot) => self.emit(Op::SetSlot(slot), s.span),
                    None => {
                        let i = self.name(name);
                        self.emit(Op::Define(i), s.span);
                    }
                }
            }
            StmtKind::Assign(name, op, value) => {
                let slot = self.resolve(name);
                if let Some(op) = op {
                    match slot {
                        Some(slot) => self.emit(Op::GetSlot(slot), s.span),
                        None => {
                            let i = self.name(name);
                            self.emit(Op::GetVarToUpdate(i), s.span);
                        }
                    }
                    self.expr(value)?;
                    self.emit(Op::Binary(*op), s.span);
                } else {
                    self.expr(value)?;
                }
                match slot {
                    Some(slot) => self.emit(Op::SetSlot(slot), s.span),
                    None => {
                        let i = self.name(name);
                        self.emit(Op::SetVar(i), s.span);
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
                self.expr(cond)?;
                let to_else = self.chunk.code.len();
                self.emit(Op::JumpIfFalse(0), cond.span);
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
                self.expr(cond)?;
                let to_end = self.chunk.code.len();
                self.emit(Op::JumpIfFalse(0), cond.span);
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
                self.expr(iterable)?;
                self.emit(Op::IterNew, iterable.span);
                let zero = self.konst(Value::Int(0));
                self.emit(Op::Const(zero), s.span);
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
                // The loop owned [snapshot, index] on the stack; both
                // jump-to-end paths (done and break) land here.
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
                let i = self.konst(Value::Str(s.clone()));
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
            ExprKind::Binary(op, lhs, rhs) => {
                self.expr(lhs)?;
                self.expr(rhs)?;
                self.emit(Op::Binary(*op), e.span);
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
        };
        let chunk = compile_stmts(body, Some((params, ctx)), self.coverage)?;
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

    fn patch(&mut self, at: usize, target: i32) {
        let rel = target - at as i32 - 1;
        match &mut self.chunk.code[at] {
            Op::Jump(o) | Op::JumpIfFalse(o) | Op::OrJump(o) | Op::AndJump(o) | Op::IterNext(o) => {
                *o = rel
            }
            _ => unreachable!("patched op is not a jump"),
        }
    }
}
