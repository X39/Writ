//! Method body emission for IL codegen.
//!
//! This module orchestrates the emission of instruction sequences for all
//! method bodies in a TypedAst. It consumes the populated ModuleBuilder from
//! Phase 24 and the TypedAst/TyInterner from Phase 23.

pub mod reg_alloc;
pub mod labels;
pub mod expr;
pub mod stmt;
pub mod call;
pub mod closure;
pub mod patterns;
pub mod const_fold;
pub mod debug;

use chumsky::span::SimpleSpan;
use rustc_hash::FxHashMap;

use writ_module::instruction::Instruction;

use crate::check::ir::{TypedAst, TypedDecl, TypedExpr, TypedLiteral, TypedStmt};
use crate::check::ty::{Ty, TyInterner};
use crate::emit::module_builder::ModuleBuilder;
use crate::resolve::def_map::DefId;

use self::labels::{Label, LabelAllocator};
use self::reg_alloc::RegisterAllocator;

/// The result of emitting a single method body.
///
/// `method_def_id` is `Some(id)` for named methods (Fn, Impl, Const, Global),
/// or `None` for anonymous lambda bodies (which have no source DefId).
///
/// `pending_strings` holds `(instruction_index, string_value)` pairs for
/// LoadString instructions that need their `string_idx` filled in by the
/// string heap after emit_all_bodies returns (since body emission holds
/// `&ModuleBuilder` which does not allow mutable heap access).
pub struct EmittedBody {
    pub method_def_id: Option<DefId>,
    pub instructions: Vec<Instruction>,
    pub reg_count: u16,
    pub reg_types: Vec<Ty>,
    pub source_spans: Vec<(u32, SimpleSpan)>,
    pub debug_locals: Vec<(u16, String, u32, u32)>,
    /// Pending string interning: (instruction_index, string_value).
    /// After emit_all_bodies, the caller must intern these strings and patch
    /// the corresponding LoadString instructions with correct string_idx values.
    pub pending_strings: Vec<(usize, String)>,
    /// Label allocator carrying the instruction-index-keyed resolved labels
    /// and pending fixups for this body. Used by serialize.rs to compute and
    /// apply correct relative byte offsets for all branch instructions.
    pub label_allocator: LabelAllocator,
}

/// The execution context for emitting a single method body.
///
/// Holds all mutable state needed during instruction emission:
/// - Register allocator (sequential u16 indices)
/// - Label allocator (symbolic labels with fixup pass)
/// - Instruction buffer
/// - Local variable map (name -> register)
/// - Loop context stack (break/continue labels)
pub struct BodyEmitter<'a> {
    pub builder: &'a ModuleBuilder,
    pub interner: &'a TyInterner,
    pub regs: RegisterAllocator,
    pub labels: LabelAllocator,
    pub instructions: Vec<Instruction>,
    pub locals: FxHashMap<String, u16>,
    pub source_spans: Vec<(u32, SimpleSpan)>,
    pub debug_locals: Vec<(u16, String, u32, u32)>,
    pub current_method_def_id: Option<DefId>,
    /// Stack of (break_label, continue_label) for nested loops.
    pub loop_stack: Vec<(Label, Label)>,
    /// Lambda counter: tracks how many lambdas have been emitted in this body.
    /// Used by closure::emit_lambda to find the right synthetic TypeDef/MethodDef.
    pub lambda_counter: usize,
    /// Pending string literals awaiting interning.
    ///
    /// Collects (instruction_index, string_value) pairs for each LoadString emitted.
    /// The caller (emit_all_bodies) must intern these into the ModuleBuilder's string
    /// heap and patch the corresponding instructions after body emission completes.
    pub pending_strings: Vec<(usize, String)>,
}

impl<'a> BodyEmitter<'a> {
    /// Create a new BodyEmitter for a single method.
    pub fn new(builder: &'a ModuleBuilder, interner: &'a TyInterner) -> Self {
        Self {
            builder,
            interner,
            regs: RegisterAllocator::new(),
            labels: LabelAllocator::new(),
            instructions: Vec::new(),
            locals: FxHashMap::default(),
            source_spans: Vec::new(),
            debug_locals: Vec::new(),
            current_method_def_id: None,
            loop_stack: Vec::new(),
            lambda_counter: 0,
            pending_strings: Vec::new(),
        }
    }

    /// Push an instruction onto the buffer.
    pub fn emit(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }

    /// Allocate a new register with the given type.
    pub fn alloc_reg(&mut self, ty: Ty) -> u16 {
        self.regs.alloc(ty)
    }

    /// Allocate a "void" register (uses Void type).
    ///
    /// Void is always Ty(4) since primitives are pre-interned in a fixed order
    /// (Int=0, Float=1, Bool=2, String=3, Void=4) by TyInterner::new().
    pub fn alloc_void_reg(&mut self) -> u16 {
        let void_ty = Ty(4);
        self.regs.alloc(void_ty)
    }

    /// Create a new label.
    pub fn new_label(&mut self) -> Label {
        self.labels.new_label()
    }

    /// Mark a label at the current instruction byte position (instruction count-based).
    ///
    /// Note: for the fixup pass we track instruction *count* positions, not byte positions,
    /// since serialization happens after codegen. The label system uses instruction-index
    /// positions, and the fixup pass uses those same indices.
    ///
    /// For tests using code-byte fixups directly, the labels module's apply_fixups
    /// works with raw byte buffers. The instruction-level system marks by instruction index.
    pub fn mark_label_here(&mut self, label: Label) {
        let pos = self.instructions.len();
        self.labels.mark(label, pos);
    }

    /// Add a fixup for a branch instruction at the given instruction index.
    pub fn add_fixup(&mut self, instr_idx: usize, label: Label) {
        self.labels.add_fixup(instr_idx, label);
    }

    /// Push the current break/continue label pair onto the loop stack.
    pub fn push_loop(&mut self, break_lbl: Label, continue_lbl: Label) {
        self.loop_stack.push((break_lbl, continue_lbl));
    }

    /// Pop the innermost loop's label pair.
    pub fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    /// Get the innermost loop's break label, panicking if not in a loop.
    pub fn break_label(&self) -> Label {
        self.loop_stack.last().expect("break outside loop").0
    }

    /// Get the innermost loop's continue label, panicking if not in a loop.
    pub fn continue_label(&self) -> Label {
        self.loop_stack.last().expect("continue outside loop").1
    }
}

// ─── Error pre-pass ───────────────────────────────────────────────────────────

/// Scan a TypedAst for any TypedExpr::Error or TypedStmt::Error nodes.
///
/// Returns true if any error nodes are found. The caller should abort
/// codegen and return only the collected diagnostics.
pub fn has_error_nodes(typed_ast: &TypedAst) -> bool {
    for decl in &typed_ast.decls {
        match decl {
            TypedDecl::Fn { body, .. } => {
                if expr_has_error(body) {
                    return true;
                }
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    if expr_has_error(body) {
                        return true;
                    }
                }
            }
            TypedDecl::Const { value, .. } | TypedDecl::Global { value, .. } => {
                if expr_has_error(value) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Recursively check a TypedExpr for any Error nodes.
fn expr_has_error(expr: &TypedExpr) -> bool {
    match expr {
        TypedExpr::Error { .. } => return true,
        TypedExpr::Block { stmts, tail, .. } => {
            for stmt in stmts {
                if stmt_has_error(stmt) {
                    return true;
                }
            }
            if let Some(t) = tail {
                if expr_has_error(t) {
                    return true;
                }
            }
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            if expr_has_error(condition) || expr_has_error(then_branch) {
                return true;
            }
            if let Some(e) = else_branch {
                if expr_has_error(e) {
                    return true;
                }
            }
        }
        TypedExpr::Binary { left, right, .. } => {
            if expr_has_error(left) || expr_has_error(right) {
                return true;
            }
        }
        TypedExpr::UnaryPrefix { expr: inner, .. } => {
            if expr_has_error(inner) {
                return true;
            }
        }
        TypedExpr::Call { callee, args, .. } => {
            if expr_has_error(callee) {
                return true;
            }
            for arg in args {
                if expr_has_error(arg) {
                    return true;
                }
            }
        }
        TypedExpr::Field { receiver, .. }
        | TypedExpr::ComponentAccess { receiver, .. } => {
            if expr_has_error(receiver) {
                return true;
            }
        }
        TypedExpr::Index { receiver, index, .. } => {
            if expr_has_error(receiver) || expr_has_error(index) {
                return true;
            }
        }
        TypedExpr::Assign { target, value, .. } => {
            if expr_has_error(target) || expr_has_error(value) {
                return true;
            }
        }
        TypedExpr::New { fields, .. } => {
            for (_, v) in fields {
                if expr_has_error(v) {
                    return true;
                }
            }
        }
        TypedExpr::ArrayLit { elements, .. } => {
            for e in elements {
                if expr_has_error(e) {
                    return true;
                }
            }
        }
        TypedExpr::Range { start, end, .. } => {
            if let Some(s) = start {
                if expr_has_error(s) {
                    return true;
                }
            }
            if let Some(e) = end {
                if expr_has_error(e) {
                    return true;
                }
            }
        }
        TypedExpr::Spawn { expr: inner, .. }
        | TypedExpr::SpawnDetached { expr: inner, .. }
        | TypedExpr::Join { expr: inner, .. }
        | TypedExpr::Cancel { expr: inner, .. }
        | TypedExpr::Defer { expr: inner, .. } => {
            if expr_has_error(inner) {
                return true;
            }
        }
        TypedExpr::Lambda { body, .. } => {
            if expr_has_error(body) {
                return true;
            }
        }
        TypedExpr::Match { scrutinee, arms, .. } => {
            if expr_has_error(scrutinee) {
                return true;
            }
            for arm in arms {
                if expr_has_error(&arm.body) {
                    return true;
                }
            }
        }
        TypedExpr::Return { value, .. } => {
            if let Some(v) = value {
                if expr_has_error(v) {
                    return true;
                }
            }
        }
        // Leaf nodes with no children to recurse into
        TypedExpr::Literal { .. }
        | TypedExpr::Var { .. }
        | TypedExpr::SelfRef { .. }
        | TypedExpr::Path { .. } => {}
    }
    false
}

/// Recursively check a TypedStmt for any Error nodes.
fn stmt_has_error(stmt: &TypedStmt) -> bool {
    match stmt {
        TypedStmt::Error { .. } => true,
        TypedStmt::Let { value, .. } => expr_has_error(value),
        TypedStmt::Expr { expr, .. } => expr_has_error(expr),
        TypedStmt::Return { value, .. } => value.as_ref().map_or(false, expr_has_error),
        TypedStmt::For { iterable, body, .. } => {
            if expr_has_error(iterable) {
                return true;
            }
            body.iter().any(stmt_has_error)
        }
        TypedStmt::While { condition, body, .. } => {
            if expr_has_error(condition) {
                return true;
            }
            body.iter().any(stmt_has_error)
        }
        TypedStmt::Atomic { body, .. } => body.iter().any(stmt_has_error),
        TypedStmt::Break { value, .. } => value.as_ref().map_or(false, expr_has_error),
        TypedStmt::Continue { .. } => false,
    }
}

// ─── emit_all_bodies ──────────────────────────────────────────────────────────

/// Emit all method bodies from the TypedAst.
///
/// Pre-pass: if any Error nodes exist, return empty Vec with diagnostic.
/// Otherwise, iterate all Fn, Impl, Const, and Global decls, emit each.
/// Lambda bodies are also emitted using the provided `lambda_infos`.
pub fn emit_all_bodies(
    typed_ast: &TypedAst,
    interner: &TyInterner,
    builder: &ModuleBuilder,
    lambda_infos: &[closure::LambdaInfo],
) -> (Vec<EmittedBody>, Vec<writ_diagnostics::Diagnostic>) {
    let mut diags = Vec::new();

    if has_error_nodes(typed_ast) {
        diags.push(
            writ_diagnostics::Diagnostic::error(
                "E9000",
                "Codegen aborted: TypedAst contains error nodes",
            ).build()
        );
        return (Vec::new(), diags);
    }

    let mut bodies = Vec::new();

    for decl in &typed_ast.decls {
        match decl {
            TypedDecl::Fn { def_id, body } => {
                let mut emitter = BodyEmitter::new(builder, interner);
                emitter.current_method_def_id = Some(*def_id);
                // Pre-allocate parameter registers r0..r(n-1) per IL spec section 2.16.2.
                // Parameters are allocated in declaration order before any body emission,
                // ensuring all branches of the body find parameters in stable registers.
                if let Some(params) = builder.get_fn_params(*def_id) {
                    for (name, ty) in params.clone() {
                        let r = emitter.alloc_reg(ty);
                        emitter.locals.insert(name.clone(), r);
                        // Parameters are live from the start of the method body.
                        emitter.debug_locals.push((r, name, 0, u32::MAX));
                    }
                }
                let result_reg = expr::emit_expr(&mut emitter, body);
                // Append implicit trailing return so the VM never falls off the end.
                if body.ty() == Ty(4) {
                    emitter.emit(Instruction::RetVoid);
                } else {
                    emitter.emit(Instruction::Ret { r_src: result_reg });
                }
                let reg_count = emitter.regs.reg_count();
                let reg_types = emitter.regs.types().to_vec();
                bodies.push(EmittedBody {
                    method_def_id: Some(*def_id),
                    instructions: emitter.instructions,
                    reg_count,
                    reg_types,
                    source_spans: emitter.source_spans,
                    debug_locals: emitter.debug_locals,
                    pending_strings: emitter.pending_strings,
                    label_allocator: emitter.labels,
                });
            }
            TypedDecl::Impl { methods, .. } => {
                for (def_id, body) in methods {
                    let mut emitter = BodyEmitter::new(builder, interner);
                    emitter.current_method_def_id = Some(*def_id);
                    // Pre-allocate parameter registers r0..r(n-1) per IL spec section 2.16.2.
                    if let Some(params) = builder.get_fn_params(*def_id) {
                        for (name, ty) in params.clone() {
                            let r = emitter.alloc_reg(ty);
                            emitter.locals.insert(name.clone(), r);
                            // Parameters are live from the start of the method body.
                            emitter.debug_locals.push((r, name, 0, u32::MAX));
                        }
                    }
                    let result_reg = expr::emit_expr(&mut emitter, body);
                    // Append implicit trailing return so the VM never falls off the end.
                    if body.ty() == Ty(4) {
                        emitter.emit(Instruction::RetVoid);
                    } else {
                        emitter.emit(Instruction::Ret { r_src: result_reg });
                    }
                    let reg_count = emitter.regs.reg_count();
                    let reg_types = emitter.regs.types().to_vec();
                    bodies.push(EmittedBody {
                        method_def_id: Some(*def_id),
                        instructions: emitter.instructions,
                        reg_count,
                        reg_types,
                        source_spans: emitter.source_spans,
                        debug_locals: emitter.debug_locals,
                        pending_strings: emitter.pending_strings,
                        label_allocator: emitter.labels,
                    });
                }
            }
            TypedDecl::Const { def_id, value } => {
                let mut emitter = BodyEmitter::new(builder, interner);
                emitter.current_method_def_id = Some(*def_id);

                // Try constant folding first — emit a single load instruction.
                let r = if let Some(folded) = const_fold::const_fold(value, interner) {
                    match &folded {
                        TypedLiteral::Int(v) => {
                            let r = emitter.alloc_reg(Ty(0)); // Int
                            emitter.emit(Instruction::LoadInt { r_dst: r, value: *v });
                            r
                        }
                        TypedLiteral::Float(v) => {
                            let r = emitter.alloc_reg(Ty(1)); // Float
                            emitter.emit(Instruction::LoadFloat { r_dst: r, value: *v });
                            r
                        }
                        TypedLiteral::Bool(true) => {
                            let r = emitter.alloc_reg(Ty(2)); // Bool
                            emitter.emit(Instruction::LoadTrue { r_dst: r });
                            r
                        }
                        TypedLiteral::Bool(false) => {
                            let r = emitter.alloc_reg(Ty(2)); // Bool
                            emitter.emit(Instruction::LoadFalse { r_dst: r });
                            r
                        }
                        TypedLiteral::String(_) => {
                            // String constants: emit via normal expr path (handles interning)
                            expr::emit_expr(&mut emitter, value)
                        }
                    }
                } else {
                    // Non-foldable: emit the full expression
                    expr::emit_expr(&mut emitter, value)
                };

                emitter.emit(Instruction::Ret { r_src: r });

                let reg_count = emitter.regs.reg_count();
                let reg_types = emitter.regs.types().to_vec();
                bodies.push(EmittedBody {
                    method_def_id: Some(*def_id),
                    instructions: emitter.instructions,
                    reg_count,
                    reg_types,
                    source_spans: emitter.source_spans,
                    debug_locals: emitter.debug_locals,
                    pending_strings: emitter.pending_strings,
                    label_allocator: emitter.labels,
                });
            }
            TypedDecl::Global { def_id, value } => {
                // Global initializers: emit without const folding (may be non-constant).
                let mut emitter = BodyEmitter::new(builder, interner);
                emitter.current_method_def_id = Some(*def_id);
                let r = expr::emit_expr(&mut emitter, value);
                emitter.emit(Instruction::Ret { r_src: r });
                let reg_count = emitter.regs.reg_count();
                let reg_types = emitter.regs.types().to_vec();
                bodies.push(EmittedBody {
                    method_def_id: Some(*def_id),
                    instructions: emitter.instructions,
                    reg_count,
                    reg_types,
                    source_spans: emitter.source_spans,
                    debug_locals: emitter.debug_locals,
                    pending_strings: emitter.pending_strings,
                    label_allocator: emitter.labels,
                });
            }
            _ => {}
        }
    }

    // Emit lambda bodies as separate EmittedBody entries.
    // lambda_infos[i] corresponds to the i-th Lambda node discovered by pre_scan_lambdas.
    // Walk the TypedAst in the same order as pre_scan_lambdas to collect lambda bodies.
    let mut lambda_bodies: Vec<&TypedExpr> = Vec::new();
    collect_lambda_bodies_from_ast(typed_ast, &mut lambda_bodies);

    for (i, lambda_body) in lambda_bodies.iter().enumerate() {
        if i >= lambda_infos.len() {
            break;
        }
        let _info = &lambda_infos[i];
        let mut emitter = BodyEmitter::new(builder, interner);
        let r = expr::emit_expr(&mut emitter, lambda_body);
        emitter.emit(Instruction::Ret { r_src: r });

        // Lambda bodies have no source DefId — use None.
        // The serializer matches lambda MethodDefs by name pattern, not DefId.
        let reg_count = emitter.regs.reg_count();
        let reg_types = emitter.regs.types().to_vec();
        bodies.push(EmittedBody {
            method_def_id: None,
            instructions: emitter.instructions,
            reg_count,
            reg_types,
            source_spans: emitter.source_spans,
            debug_locals: emitter.debug_locals,
            pending_strings: emitter.pending_strings,
            label_allocator: emitter.labels,
        });
    }

    (bodies, diags)
}

/// Walk the TypedAst in the same pre-order as `pre_scan_lambdas` and collect
/// references to each Lambda's body expression.
fn collect_lambda_bodies_from_ast<'a>(typed_ast: &'a TypedAst, out: &mut Vec<&'a TypedExpr>) {
    for decl in &typed_ast.decls {
        match decl {
            TypedDecl::Fn { body, .. } => {
                collect_lambda_bodies_from_expr(body, out);
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    collect_lambda_bodies_from_expr(body, out);
                }
            }
            TypedDecl::Const { value, .. } | TypedDecl::Global { value, .. } => {
                collect_lambda_bodies_from_expr(value, out);
            }
            _ => {}
        }
    }
}

/// Walk an expression in the same pre-order as `scan_expr_for_lambdas`,
/// collecting the body of each Lambda encountered.
fn collect_lambda_bodies_from_expr<'a>(expr: &'a TypedExpr, out: &mut Vec<&'a TypedExpr>) {
    match expr {
        TypedExpr::Lambda { body, .. } => {
            // Collect the body expression for this lambda, then recurse into it.
            out.push(body.as_ref());
            collect_lambda_bodies_from_expr(body, out);
        }
        TypedExpr::Block { stmts, tail, .. } => {
            for stmt in stmts {
                collect_lambda_bodies_from_stmt(stmt, out);
            }
            if let Some(t) = tail {
                collect_lambda_bodies_from_expr(t, out);
            }
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            collect_lambda_bodies_from_expr(condition, out);
            collect_lambda_bodies_from_expr(then_branch, out);
            if let Some(e) = else_branch {
                collect_lambda_bodies_from_expr(e, out);
            }
        }
        TypedExpr::Binary { left, right, .. } => {
            collect_lambda_bodies_from_expr(left, out);
            collect_lambda_bodies_from_expr(right, out);
        }
        TypedExpr::UnaryPrefix { expr: inner, .. } => {
            collect_lambda_bodies_from_expr(inner, out);
        }
        TypedExpr::Call { callee, args, .. } => {
            collect_lambda_bodies_from_expr(callee, out);
            for arg in args {
                collect_lambda_bodies_from_expr(arg, out);
            }
        }
        TypedExpr::Field { receiver, .. } | TypedExpr::ComponentAccess { receiver, .. } => {
            collect_lambda_bodies_from_expr(receiver, out);
        }
        TypedExpr::Index { receiver, index, .. } => {
            collect_lambda_bodies_from_expr(receiver, out);
            collect_lambda_bodies_from_expr(index, out);
        }
        TypedExpr::Assign { target, value, .. } => {
            collect_lambda_bodies_from_expr(target, out);
            collect_lambda_bodies_from_expr(value, out);
        }
        TypedExpr::New { fields, .. } => {
            for (_, v) in fields {
                collect_lambda_bodies_from_expr(v, out);
            }
        }
        TypedExpr::ArrayLit { elements, .. } => {
            for e in elements {
                collect_lambda_bodies_from_expr(e, out);
            }
        }
        TypedExpr::Range { start, end, .. } => {
            if let Some(s) = start {
                collect_lambda_bodies_from_expr(s, out);
            }
            if let Some(e) = end {
                collect_lambda_bodies_from_expr(e, out);
            }
        }
        TypedExpr::Spawn { expr: inner, .. }
        | TypedExpr::SpawnDetached { expr: inner, .. }
        | TypedExpr::Join { expr: inner, .. }
        | TypedExpr::Cancel { expr: inner, .. }
        | TypedExpr::Defer { expr: inner, .. } => {
            collect_lambda_bodies_from_expr(inner, out);
        }
        TypedExpr::Match { scrutinee, arms, .. } => {
            collect_lambda_bodies_from_expr(scrutinee, out);
            for arm in arms {
                collect_lambda_bodies_from_expr(&arm.body, out);
            }
        }
        TypedExpr::Return { value, .. } => {
            if let Some(v) = value {
                collect_lambda_bodies_from_expr(v, out);
            }
        }
        // Leaf nodes
        TypedExpr::Literal { .. }
        | TypedExpr::Var { .. }
        | TypedExpr::SelfRef { .. }
        | TypedExpr::Path { .. }
        | TypedExpr::Error { .. } => {}
    }
}

/// Walk a statement for lambda bodies.
fn collect_lambda_bodies_from_stmt<'a>(stmt: &'a TypedStmt, out: &mut Vec<&'a TypedExpr>) {
    match stmt {
        TypedStmt::Let { value, .. } => collect_lambda_bodies_from_expr(value, out),
        TypedStmt::Expr { expr, .. } => collect_lambda_bodies_from_expr(expr, out),
        TypedStmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_lambda_bodies_from_expr(v, out);
            }
        }
        TypedStmt::For { iterable, body, .. } => {
            collect_lambda_bodies_from_expr(iterable, out);
            for s in body {
                collect_lambda_bodies_from_stmt(s, out);
            }
        }
        TypedStmt::While { condition, body, .. } => {
            collect_lambda_bodies_from_expr(condition, out);
            for s in body {
                collect_lambda_bodies_from_stmt(s, out);
            }
        }
        TypedStmt::Atomic { body, .. } => {
            for s in body {
                collect_lambda_bodies_from_stmt(s, out);
            }
        }
        TypedStmt::Break { value, .. } => {
            if let Some(v) = value {
                collect_lambda_bodies_from_expr(v, out);
            }
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => {}
    }
}
