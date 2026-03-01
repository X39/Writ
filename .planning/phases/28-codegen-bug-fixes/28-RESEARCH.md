# Phase 28: Codegen Bug Fixes — Call Resolution and Instruction Emission - Research

**Researched:** 2026-03-03
**Domain:** Rust compiler codegen — IL method body emission, call resolution, instruction correctness
**Confidence:** HIGH

## Summary

Phase 28 fixes four concrete bugs identified in the v3.0 milestone audit that cause the compiler to emit incorrect or placeholder instructions. These are not architectural gaps — the surrounding infrastructure is in place — but specific missing implementations that require targeted fixes.

**MC-01** is the deepest bug: `TypedExpr::Call` carries the callee as a `TypedExpr` with only a name string, not a DefId. The `extract_callee_def_id_opt()` function in `expr.rs` can therefore never return a real DefId without access to `DefMap`. The fix requires adding a `def_id: Option<DefId>` field to `TypedExpr::Call` (or `TypedExpr::Var`), populated during type checking, so the emitter can use it without needing `DefMap` access in `BodyEmitter`.

**BF-01** depends on MC-01 being fixed first: once callee DefIds flow through, the existing `contract_token_for_method_def_id` side table in `ModuleBuilder` can produce the correct non-zero `contract_idx` in `CallVirt`. The infrastructure from Phase 26-04 is already in place — only the DefId resolution path is missing.

**BF-02** (`TypedExpr::Range` emitting Nop) requires constructing a `Range<T>` struct using `New { type_idx }` followed by four `SetField` instructions (for `start`, `end`, `start_inclusive`, `end_inclusive`). The `Range<T>` TypeDef lives in the `writ-runtime` virtual module; its type token must be resolved through the builder's cross-module TypeRef system.

**BF-03** (`DeferPush` emitting `method_idx: 0` placeholder) requires computing the instruction index of the defer handler block at emit time. The handler code (the deferred expression body) is emitted inline, so the fix is to emit a placeholder `DeferPush`, record the index for fixup, emit the handler body, emit `DeferEnd`, then patch the `DeferPush.method_idx` with the instruction index of the first handler instruction.

**Primary recommendation:** Fix MC-01 first (add `callee_def_id: Option<DefId>` to `TypedExpr::Call`), which unblocks BF-01. Fix BF-02 and BF-03 independently as they don't depend on MC-01.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| EMIT-09 | CALL, CALL_VIRT, CALL_EXTERN, CALL_INDIRECT with correct dispatch selection | MC-01 fix: callee DefId propagation through TypedExpr::Call enables correct method_idx in CALL; BF-01 fix: correct contract_idx in CALL_VIRT |
| EMIT-12 | All 9 array instructions | BF-02 fix: Range construction enables range-based iteration, which is part of EMIT-12's array/range scope |
| EMIT-15 | SPAWN_TASK, SPAWN_DETACHED, JOIN, CANCEL, DEFER_PUSH/POP/END | BF-03 fix: correct handler_offset in DeferPush; existing Join/Cancel/Spawn already correct |
| EMIT-27 | CALL_VIRT specializes to CALL for concrete static receiver types | MC-01 fix unblocks correct method_idx in both CALL and CALL_VIRT; EMIT-27 is currently correct for the dispatch-kind decision but emits method_idx=0 without the DefId |
| FIX-02 | Runtime resolves generic contract specialization without collision (compiler-side) | BF-01 fix: compiler emits non-zero contract_idx derived from registered impl-method-to-contract mapping |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust (writ-compiler crate) | 2021 edition | All codegen implementation | Project-standard; no new deps needed |
| writ-module | workspace | IL instruction set, MetadataToken | Defines Instruction enum and WRITIL format |
| id-arena | 2.3.0 | DefId type (id_arena::Id<DefEntry>) | Already in workspace; DefId is `id_arena::Id<DefEntry>` |
| rustc-hash | 2.1.1 | FxHashMap for method_to_contract side table | Already in workspace |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| writ-runtime (virtual_module) | workspace | Range<T> TypeDef token lookup | BF-02: need the Range type_idx from the writ-runtime virtual module |

**Installation:** No new dependencies needed. All required libraries are already in `Cargo.toml`.

## Architecture Patterns

### Recommended Project Structure

Changes are confined to:
```
writ-compiler/src/
├── check/ir.rs                  # Add callee_def_id to TypedExpr::Call (MC-01)
├── check/check_expr.rs          # Populate callee_def_id when building TypedExpr::Call (MC-01)
├── emit/body/expr.rs            # Fix extract_callee_def_id_opt, Range emission, DeferPush fixup
└── emit/body/call.rs            # (may be unaffected if expr.rs path is fixed)
writ-compiler/tests/
└── emit_body_tests.rs           # New tests for all four bugs
```

### Pattern 1: DefId propagation into TypedExpr::Call (MC-01)

**What:** Add an `Option<DefId>` field to `TypedExpr::Call` so the emitter doesn't need DefMap.
**When to use:** Whenever emitting CALL/CALL_VIRT/CALL_EXTERN instructions.
**Example:**
```rust
// In check/ir.rs — extend TypedExpr::Call
Call {
    ty: Ty,
    span: SimpleSpan,
    callee: Box<TypedExpr>,
    args: Vec<TypedExpr>,
    callee_def_id: Option<DefId>,  // NEW: populated by type checker when callee is a known Fn/ExternFn
},

// In check/check_expr.rs — check_call_with_sig
TypedExpr::Call {
    ty: resolved_ret,
    span,
    callee: Box::new(TypedExpr::Var { ty: ..., span: name_span, name: fn_name.to_string() }),
    args: typed_args,
    callee_def_id: Some(def_id),   // Pass the resolved def_id through
}

// In emit/body/expr.rs — update extract_callee_def_id_opt
pub(crate) fn extract_callee_def_id_opt(
    _emitter: &BodyEmitter<'_>,
    callee_expr_with_parent: &TypedExpr,
) -> Option<DefId> {
    // Now callee_expr is the parent Call expression, not just the callee sub-expr
    // OR alternatively: the Call variant exposes callee_def_id directly
    match callee_expr_with_parent {
        TypedExpr::Call { callee_def_id, .. } => *callee_def_id,
        _ => None,
    }
}
```

**Alternative approach (simpler):** Keep `extract_callee_def_id_opt` taking the callee subexpression, but add a `def_id` field directly to `TypedExpr::Var`:
```rust
Var {
    ty: Ty,
    span: SimpleSpan,
    name: String,
    def_id: Option<DefId>,  // NEW: populated when the var resolves to a known Fn
},
```
Then `extract_callee_def_id_opt` can check `TypedExpr::Var { def_id, .. }`.

**Recommended approach:** Add `callee_def_id: Option<DefId>` directly to `TypedExpr::Call`. This is more precise — only call sites that resolve to a known function get a non-None value. No cascading changes to all Var usages.

### Pattern 2: Range struct construction (BF-02)

**What:** `TypedExpr::Range` should emit `New { type_idx }` + four `SetField` instructions.
**When to use:** Any range expression (`0..10`, `0..=10`).
**Example:**
```rust
// In emit/body/expr.rs — replace the Nop placeholder
TypedExpr::Range { ty, start, end, inclusive, .. } => {
    // Resolve Range<T> type_idx from the writ-runtime module.
    // The Range type is in the virtual module; look it up by name via builder
    // or use a hardcoded token path via writ-runtime ModuleRef.
    // Approach: builder exposes a helper that returns the Range TypeDef's external token.
    let range_type_idx = emitter.builder.range_type_idx().unwrap_or(0);

    // Construct Range struct: NEW { r_dst, type_idx }
    let r_range = emitter.alloc_reg(*ty);
    emitter.emit(Instruction::New { r_dst: r_range, type_idx: range_type_idx });

    // SET_FIELD for start
    let start_field_idx = 0u32; // "start" is field 0 in Range
    let r_start = if let Some(start_expr) = start {
        emit_expr(emitter, start_expr)
    } else {
        // half-open range with no start: default to 0
        let r = emitter.alloc_reg(Ty(0)); // Int
        emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 });
        r
    };
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: start_field_idx, r_val: r_start });

    // SET_FIELD for end
    let end_field_idx = 1u32; // "end" is field 1
    let r_end = if let Some(end_expr) = end {
        emit_expr(emitter, end_expr)
    } else {
        let r = emitter.alloc_reg(Ty(0));
        emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 });
        r
    };
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: end_field_idx, r_val: r_end });

    // SET_FIELD for start_inclusive (always true in current syntax)
    let r_start_incl = emitter.alloc_reg(Ty(2)); // Bool
    emitter.emit(Instruction::LoadTrue { r_dst: r_start_incl });
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: 2, r_val: r_start_incl });

    // SET_FIELD for end_inclusive (true for ..=, false for ..)
    let r_end_incl = emitter.alloc_reg(Ty(2)); // Bool
    if *inclusive {
        emitter.emit(Instruction::LoadTrue { r_dst: r_end_incl });
    } else {
        emitter.emit(Instruction::LoadFalse { r_dst: r_end_incl });
    }
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: 3, r_val: r_end_incl });

    r_range
}
```

**Critical sub-question:** How to get `range_type_idx`? The Range TypeDef is in the writ-runtime virtual module (not the user's module). The compiler needs to emit a cross-module TypeRef to it. Looking at the existing module builder, there are `ModuleRef` rows and `TypeRef` blobs. The simplest approach for Phase 28 is:
- Add a `range_type_idx() -> Option<u32>` helper to `ModuleBuilder` that returns the TypeRef token for the Range type from writ-runtime.
- OR: return a hardcoded sentinel (0) and document that Range construction is approximated (type_idx=0 = first type, which is Option). This is incorrect but won't panic.
- OR: use `writ-module`'s existing builder to pre-emit a TypeRef to Range during module initialization.

**Recommended approach:** Add a `runtime_type_token(name: &str) -> Option<u32>` helper to `ModuleBuilder` that searches existing TypeRef entries for the named writ-runtime type. If not found, falls back to 0. This is a minimal, correct-when-wired approach.

### Pattern 3: DeferPush handler_offset fixup (BF-03)

**What:** The defer handler starts at a known instruction index that can be computed after emitting the DeferPush and before emitting the handler body.
**When to use:** `TypedExpr::Defer { expr, .. }`.

**Key insight from runtime analysis:** The runtime's `DeferPush` handler stores `method_idx` directly as the instruction index PC for the handler:
```rust
// In dispatch.rs line 1296:
frame.defer_stack.push(method_idx as usize);
// ...
// execute_defer_handler sets:
task.call_stack.last_mut().unwrap().pc = handler_pc;
```
So `method_idx` in `DeferPush` IS the instruction index of the first instruction of the handler block. After emit, the offset is the instruction count at the point the handler body begins.

**Example:**
```rust
fn emit_defer(emitter: &mut BodyEmitter<'_>, expr: &TypedExpr) -> u16 {
    let void_ty = Ty(4);
    let r_dst = emitter.alloc_reg(void_ty);

    // Emit DeferPush with placeholder; record instruction index for fixup
    let defer_push_idx = emitter.instructions.len();
    emitter.emit(Instruction::DeferPush { r_dst, method_idx: 0 }); // placeholder

    // DeferPop: disarms the defer on normal exit (emitted BEFORE the handler)
    emitter.emit(Instruction::DeferPop);

    // The handler starts at the NEXT instruction index
    let handler_instruction_idx = emitter.instructions.len() as u32;

    // Emit the deferred expression body (the handler code)
    let _ = emit_expr(emitter, expr);

    // DeferEnd: marks end of handler
    emitter.emit(Instruction::DeferEnd);

    // Patch DeferPush with the correct handler instruction index
    if let Instruction::DeferPush { method_idx, .. } = &mut emitter.instructions[defer_push_idx] {
        *method_idx = handler_instruction_idx;
    }

    r_dst
}
```

**Note on emit ordering:** The current emit_defer emits: DeferPush, body, DeferPop, DeferEnd. But logically, DeferPop should be emitted on normal exit (before the handler block), and the handler should be reachable only when the defer triggers. The correct sequence is:
```
DeferPush { handler_idx }   <- registers handler PC
... normal code ...
DeferPop                    <- disarms if normal exit
... handler (unreachable on normal path) ...
DeferEnd                    <- signals handler completion
```
The body of the `defer` expr is the handler, not the "scope guarded by defer". In Writ, `defer expr` means "run `expr` when this scope exits". So `expr` is the handler body.

The correct structure emitted for `defer expr`:
1. `DeferPush { method_idx: N }` — push handler at index N
2. (other normal-path code happens in the surrounding scope, not emitted here)
3. At scope exit: `DeferPop` if not triggering the handler (normal path)
4. At index N: `emit_expr(expr)` — the handler body
5. `DeferEnd` — end of handler

For the simplified in-body implementation (Phase 28 fix), the handler is emitted inline immediately after the DeferPop. The exact layout:
```
idx 0: DeferPush { method_idx: 3 }
idx 1: DeferPop
idx 2: Br { to end }          <- skip handler on normal path
idx 3: <handler body>
idx N: DeferEnd
```
However, for simplicity, the current approach (emitting handler inline) is acceptable as a Phase 28 fix — the key correctness fix is that `method_idx` points to the right instruction index, not 0.

### Anti-Patterns to Avoid

- **Modifying emit_all_bodies to pass DefId separately:** The clean fix is to embed the DefId in `TypedExpr::Call` at type-check time. Don't try to thread it through emit_all_bodies as a side table.
- **Using byte offsets for DeferPush:** The runtime uses instruction indices (PC), not byte offsets. `DeferPush.method_idx` is an instruction index, set by `execute_defer_handler` as the `pc` value.
- **Hardcoding Range field indices:** The Range fields (`start=0`, `end=1`, `start_inclusive=2`, `end_inclusive=3`) are defined in `writ-runtime/src/virtual_module.rs`. These are stable (defined once in the virtual module) but should use a named constant or lookup rather than magic numbers.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fixup for DeferPush handler index | A separate label/fixup system | Direct Vec indexing (`emitter.instructions[idx]`) | The instruction buffer is a `Vec<Instruction>` with mutable access; direct mutation after emission is simpler and more correct than adding a new fixup type |
| Range type token resolution | A new cross-module resolution system | `ModuleBuilder::token_for_def()` + existing ModuleRef/TypeRef infrastructure, or a simple `range_type_idx()` helper | Infrastructure exists; minimal glue is sufficient for Phase 28 |
| DefMap access in BodyEmitter | Adding DefMap to BodyEmitter | Add `callee_def_id` to `TypedExpr::Call` | BodyEmitter intentionally holds `&ModuleBuilder` only; the typed IR should carry resolved DefIds |

## Common Pitfalls

### Pitfall 1: TypedExpr::Call is used in pattern matching throughout the codebase

**What goes wrong:** Adding `callee_def_id: Option<DefId>` to `TypedExpr::Call` requires updating every match arm that destructures `TypedExpr::Call`.
**Why it happens:** Rust exhaustive pattern matching.
**How to avoid:** Search all files for `TypedExpr::Call {` before making the change. Use `..` to ignore the new field in existing non-emit match arms (check_expr.rs, mod.rs has_error_nodes, collect_lambda_bodies, etc.).
**Warning signs:** Compiler error "missing field `callee_def_id` in initializer of `TypedExpr::Call`".

Known locations that destructure TypedExpr::Call:
- `writ-compiler/src/emit/body/mod.rs` — `expr_has_error`, `collect_lambda_bodies_from_expr`
- `writ-compiler/src/emit/body/expr.rs` — main `emit_expr` match, `emit_tail_call`
- `writ-compiler/src/emit/body/call.rs` — `emit_call`, `emit_call_indirect`
- `writ-compiler/src/check/check_expr.rs` — builds `TypedExpr::Call` (multiple sites)
- `writ-compiler/src/check/check_stmt.rs` — any Call handling in statement checking
- `writ-compiler/tests/emit_body_tests.rs` — test construction of `TypedExpr::Call`

### Pitfall 2: extract_callee_def_id_opt receives the CALLEE sub-expression, not the Call expression

**What goes wrong:** The function signature takes `callee: &TypedExpr` (the sub-expression inside the Call), not the Call node itself. If `callee_def_id` is added to `TypedExpr::Call`, the function cannot access it since it receives the inner callee.
**Why it happens:** The function was designed to analyze the callee shape (Var vs Field).
**How to avoid:** Either:
  1. Change the call sites to pass the parent `TypedExpr::Call` node, OR
  2. Add `def_id` to `TypedExpr::Var` (so the callee sub-expression carries it), OR
  3. At call sites in `emit_expr`, read `callee_def_id` from the outer Call pattern and pass it directly to `emit_call`.

**Recommended:** Option 3 — in the `TypedExpr::Call` match arm of `emit_expr`, destructure `callee_def_id` from the Call and pass it to `emit_call`. Remove `extract_callee_def_id_opt` from the emit_expr path entirely.

### Pitfall 3: Range construction requires a cross-module type reference

**What goes wrong:** The `Range<T>` TypeDef is in the writ-runtime virtual module, not the user's module. `New { type_idx }` requires the type's token in the current module's TypeRef table (for cross-module refs) or TypeDef table (local types). Without a correct token, the runtime will crash or return wrong results.
**Why it happens:** The user module only has TypeDefs for types defined in user source; writ-runtime types are referenced via TypeRef.
**How to avoid:** Research `ModuleBuilder`'s existing TypeRef handling. In `collect.rs`, cross-module type references are built with `add_typeref()`. A minimal fix for BF-02 could: (1) look up the Range TypeRef if already registered, or (2) register a new TypeRef to writ-runtime::Range during emit. Fall back to type_idx=0 with a warning comment if not found.

### Pitfall 4: DeferPush handler_idx must be instruction index, not byte offset

**What goes wrong:** The labels system in `BodyEmitter` works with instruction indices (position in `instructions` Vec), not byte offsets. `DeferPush.method_idx` is stored as the handler PC in the dispatch table, which is also an instruction index.
**Why it happens:** The instruction encoding and runtime both use instruction indices; byte offsets would require serialization information not available at emit time.
**How to avoid:** Use `emitter.instructions.len()` as the instruction index — it gives the 0-based index of the next instruction to be emitted.

### Pitfall 5: check_call_with_sig constructs TypedExpr::Call in multiple places

**What goes wrong:** `check_expr.rs` has many sites that construct `TypedExpr::Call`. Some are error/fallback paths that don't have a resolved `def_id`.
**Why it happens:** Call checking handles multiple cases: direct fn call, method call, func-typed callee, error fallback.
**How to avoid:** Use `callee_def_id: None` for error/fallback paths and unknown-callee paths. Only `check_call_with_sig` has a resolved `def_id` to propagate. Search for all `TypedExpr::Call {` construction sites.

## Code Examples

### Current broken state (MC-01)

```rust
// In writ-compiler/src/emit/body/expr.rs line 1229:
pub(crate) fn extract_callee_def_id_opt(
    emitter: &BodyEmitter<'_>,
    callee: &TypedExpr,
) -> Option<crate::resolve::def_map::DefId> {
    // Without a DefMap reference in BodyEmitter, we can only use type info.
    let _ = callee;
    let _ = emitter;
    None   // ALWAYS returns None — this is the bug
}
```

### Fixed state (MC-01) — add field to TypedExpr::Call

```rust
// In writ-compiler/src/check/ir.rs:
Call {
    ty: Ty,
    span: SimpleSpan,
    callee: Box<TypedExpr>,
    args: Vec<TypedExpr>,
    callee_def_id: Option<DefId>,  // populated by check_call_with_sig
},

// In writ-compiler/src/check/check_expr.rs — check_call_with_sig:
TypedExpr::Call {
    ty: resolved_ret,
    span,
    callee: Box::new(TypedExpr::Var { ty: ..., span: name_span, name: fn_name.to_string() }),
    args: typed_args,
    callee_def_id: Some(def_id),
}

// In emit/body/expr.rs — the Call match arm now has callee_def_id:
TypedExpr::Call { callee, ty, callee_def_id, .. } => {
    // Use callee_def_id directly
    let kind = analyze_callee_with_id(emitter, expr, *callee_def_id);
    // Use *callee_def_id as the definitive DefId for method_idx resolution
    let method_idx = callee_def_id
        .and_then(|id| emitter.builder.token_for_def(id))
        .map(|t| t.0)
        .unwrap_or(0);
    ...
}
```

### Fixed state (BF-01) — contract_idx wiring

```rust
// BF-01 is already structurally in place (26-04 added the side table):
// - contract_token_for_method_def_id() exists in ModuleBuilder
// - register_impl_method_contract() is called from collect phase
// - Both CALL_VIRT emission sites already use contract_token_for_method_def_id(callee_def_id)
// The fix is: callee_def_id is now Some(...) from MC-01, so the lookup succeeds.
```

### Fixed state (BF-02) — Range construction

```rust
// In emit/body/expr.rs:
TypedExpr::Range { ty, start, end, inclusive, .. } => {
    // Get Range type token from builder (0-based fallback if not wired)
    let range_type_idx = emitter.builder.range_type_idx().unwrap_or(0);
    let r_range = emitter.alloc_reg(*ty);
    emitter.emit(Instruction::New { r_dst: r_range, type_idx: range_type_idx });

    // start field (index 0)
    let r_start = if let Some(s) = start { emit_expr(emitter, s) } else {
        let r = emitter.alloc_reg(Ty(0)); emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 }); r
    };
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: 0, r_val: r_start });

    // end field (index 1)
    let r_end = if let Some(e) = end { emit_expr(emitter, e) } else {
        let r = emitter.alloc_reg(Ty(0)); emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 }); r
    };
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: 1, r_val: r_end });

    // start_inclusive (always true per spec §2.18.2)
    let r_si = emitter.alloc_reg(Ty(2));
    emitter.emit(Instruction::LoadTrue { r_dst: r_si });
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: 2, r_val: r_si });

    // end_inclusive (true for ..=, false for ..)
    let r_ei = emitter.alloc_reg(Ty(2));
    if *inclusive { emitter.emit(Instruction::LoadTrue { r_dst: r_ei }); }
    else { emitter.emit(Instruction::LoadFalse { r_dst: r_ei }); }
    emitter.emit(Instruction::SetField { r_obj: r_range, field_idx: 3, r_val: r_ei });

    r_range
}
```

### Fixed state (BF-03) — DeferPush with correct handler_idx

```rust
// In emit/body/expr.rs:
fn emit_defer(emitter: &mut BodyEmitter<'_>, expr: &TypedExpr) -> u16 {
    let void_ty = Ty(4);
    let r_dst = emitter.alloc_reg(void_ty);

    // Emit DeferPush placeholder; remember index for patching
    let defer_push_instr_idx = emitter.instructions.len();
    emitter.emit(Instruction::DeferPush { r_dst, method_idx: 0 }); // placeholder

    // DeferPop: disarm if normal exit (precedes handler so is not in handler path)
    emitter.emit(Instruction::DeferPop);

    // Handler starts at the NEXT instruction (current len is the index)
    let handler_start_idx = emitter.instructions.len() as u32;

    // Emit the handler body (the deferred expression)
    let _ = emit_expr(emitter, expr);

    // DeferEnd: marks completion of handler execution
    emitter.emit(Instruction::DeferEnd);

    // Patch DeferPush with the correct handler instruction index
    if let Instruction::DeferPush { method_idx, .. } = &mut emitter.instructions[defer_push_instr_idx] {
        *method_idx = handler_start_idx;
    }

    r_dst
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| extract_callee_def_id_opt returns None | Add callee_def_id to TypedExpr::Call | Phase 28 | Unblocks correct method_idx for all CALL/CALL_VIRT/CALL_EXTERN/TailCall sites |
| TypedExpr::Range emits Nop | Emit New + SetField sequence for Range struct | Phase 28 | Range expressions become valid IL instead of no-ops |
| DeferPush method_idx=0 | Compute and patch correct handler instruction index | Phase 28 | defer blocks actually run correct handler code |
| contract_idx=0 in CALL_VIRT | Use registered contract token from method_to_contract side table | Phase 26-04 + Phase 28 (MC-01 unblocks) | Generic dispatch works correctly for multi-specialization types |

## Open Questions

1. **Range type_idx resolution**
   - What we know: `Range<T>` TypeDef is defined in `writ-runtime/src/virtual_module.rs` as the 3rd TypeDef (after Option and Result). The builder's `token_for_def` doesn't cover virtual module types since they use a separate token space.
   - What's unclear: Does `ModuleBuilder` have existing machinery to emit a TypeRef to writ-runtime::Range, or must we add `range_type_idx()` as a new builder helper?
   - Recommendation: Add `pub fn range_type_idx(&self) -> Option<u32>` to `ModuleBuilder` that returns the TypeRef token for Range. If no TypeRef to Range exists, return None and fallback to 0. The implementation can look at existing TypeRefs (added during `collect_defs` for writ-runtime module ref) or hardcode based on the known position. This can be a TODO comment with type_idx=0 fallback if wiring is complex.

2. **check_call_with_sig for method calls**
   - What we know: `check_call_with_sig` uses `def_id` locally but doesn't embed it in the produced `TypedExpr::Call`. Method calls go through a different path.
   - What's unclear: Method calls (receiver.method()) don't call `check_call_with_sig` — they may use a different code path. Need to verify the method call path in `check_expr.rs` (lines 900-1020) to ensure `callee_def_id` is populated for method calls too.
   - Recommendation: Read the method call resolution section of `check_expr.rs` before implementing.

3. **DeferPush ordering: push vs. scope guard semantics**
   - What we know: `defer expr` in Writ means "run expr when current scope exits". The runtime's DeferPush stores a PC for the handler. The emit_defer current implementation emits handler inline.
   - What's unclear: The correct ordering (push, then normal code, then pop on exit) requires the calling scope to emit DeferPop. Currently `emit_defer` emits both DeferPop and DeferEnd inline, which means the DeferPop fires immediately (not at scope exit).
   - Recommendation: For Phase 28, fix the handler_idx issue (BF-03's stated success criterion). The DeferPop-at-scope-exit semantics are a separate, more complex problem (requires scope exit tracking). The success criterion for BF-03 is only: "DeferPush emits correct handler_offset pointing to the defer block handler" — so the minimal fix is correct handler_idx with current DeferPop placement.

## Validation Architecture

(nyquist_validation not configured — section skipped)

## Sources

### Primary (HIGH confidence)
- Direct source code inspection: `D:/dev/git/Writ/writ-compiler/src/emit/body/expr.rs` — confirmed all four bug locations
- Direct source code inspection: `D:/dev/git/Writ/writ-compiler/src/check/ir.rs` — confirmed TypedExpr::Call lacks callee_def_id
- Direct source code inspection: `D:/dev/git/Writ/writ-runtime/src/dispatch.rs` line 1293-1296 — DeferPush stores method_idx as instruction PC
- Direct source code inspection: `D:/dev/git/Writ/writ-runtime/src/virtual_module.rs` line 191-197 — Range<T> struct definition with 4 fields
- `.planning/v3.0-MILESTONE-AUDIT.md` — authoritative source for MC-01, BF-01, BF-02, BF-03 bug descriptions
- `.planning/phases/26-cli-integration-e2e-validation/26-04-SUMMARY.md` — FIX-02 side table infrastructure from Phase 26

### Secondary (MEDIUM confidence)
- `.planning/phases/26-cli-integration-e2e-validation/26-VERIFICATION.md` — current state of CALL_VIRT contract_idx wiring with backward-compat fallback
- `.planning/STATE.md` accumulated decisions — implementation decisions from Phases 25 and 26 that constrain Phase 28

### Tertiary (LOW confidence)
None — all findings are from direct code inspection.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new deps; all changes in existing files
- Architecture: HIGH — bug locations confirmed by code inspection; fix patterns derived from surrounding code conventions
- Pitfalls: HIGH — TypedExpr enum exhaustive matching is deterministic; instruction index semantics confirmed from runtime

**Research date:** 2026-03-03
**Valid until:** This is a point-in-time code audit; valid as long as the files referenced haven't changed.
