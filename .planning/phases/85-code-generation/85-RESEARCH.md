# Phase 85: Code Generation - Research

**Researched:** 2026-03-24
**Domain:** Writ compiler emit layer — CALL_VIRT instruction emission for contract-typed receivers
**Confidence:** HIGH

## Summary

Phase 85 closes the last gap in the contract-as-type pipeline that Phase 84 built. The type checker now correctly resolves `TyKind::Contract` and method calls on contract-typed receivers type-check through `contract_methods`. What is missing is that the **emitter treats `TyKind::Contract` like an unknown type and falls through to `CallKind::Direct`** — it needs to emit `CALL_VIRT` instead, carrying a correct `contract_idx` and `slot`.

The entire infrastructure for CALL_VIRT emission already exists and is tested: `contract_token_for_method_def_id`, `register_impl_method_contract`, `assign_vtable_slots`, and the `CallVirt` instruction path in `emit_expr`. What is missing is a single branch in the receiver-type dispatch inside `emit_expr` (in `writ-compiler/src/emit/body/expr/mod.rs`): when the receiver's type is `TyKind::Contract(def_id)`, the emitter must resolve the contract's `MetadataToken`, look up the method's slot, and emit `CALL_VIRT`. The `callee_def_id` from the typed expression will be `None` on this path (contract method calls do not propagate a DefId through the type checker today), so the emitter must look up the contract token and slot by name from the builder.

The "5-bug repro script" (`pub contract MyContract { fn implementedFunc(self); fn notImplementedFunc(self); } ...`) is the canonical end-to-end test. With Phase 84 complete, `let c: MyContract = new MyClass{}` compiles cleanly and `c.notImplementedFunc()` produces E0123 at compile time. The remaining success criterion for Phase 85 is: `c.implementedFunc()` compiles, emits `CALL_VIRT` with correct `contract_idx`/`slot`, and runs to completion; `c.notImplementedFunc()` (incomplete impl) is caught by E0123 before reaching the runtime.

**Primary recommendation:** Add `TyKind::Contract(contract_def_id)` to the receiver-type dispatch inside the `TypedExpr::Call` arm of `emit_expr`, resolve the contract token and method slot from the builder by name, and emit `CALL_VIRT`. Wire the collection phase to call `register_impl_method_contract` for every contract impl method so the contract_idx is non-zero.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — pure infrastructure phase.

### Claude's Discretion
Use ROADMAP phase goal, success criteria, and codebase conventions to guide all decisions.

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| EMIT-01 | Method calls on contract-typed receivers emit CALL_VIRT (not CALL) | Requires new branch in `emit_expr` TypedExpr::Call arm: detect `TyKind::Contract` receiver and route to `Instruction::CallVirt` |
| EMIT-02 | CALL_VIRT carries correct contract_idx and slot for contract-typed dispatch | contract_idx from `builder.token_for_def(contract_def_id)` after ensuring `collect_impl` calls `register_impl_method_contract`; slot from `builder.contract_method_slot_by_name` lookup |
| EMIT-04 | The original 5-bug repro script compiles and runs correctly (implementedFunc succeeds, notImplementedFunc caught by E0123) | E0123 is already working from Phase 84/quick-vkg. EMIT-01+02 complete the implementedFunc path. End-to-end test in emit_body_tests.rs or a new integration test |
</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| writ-compiler (internal) | workspace | Compiler pipeline: parse → lower → resolve → typecheck → emit | Only compiler crate |
| writ-module (internal) | workspace | IL module format, MetadataToken, Instruction types | Shared IL definition |
| writ-runtime (internal) | workspace | VM execution for end-to-end test | Required for EMIT-04 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| rustc-hash | workspace | FxHashMap for def_id lookups in builder | Used throughout emit layer |

### Alternatives Considered

None — this is a pure intra-codebase change.

**Installation:** No new dependencies required.

---

## Architecture Patterns

### The Call Dispatch Decision Tree (current state)

Inside `emit_expr` in `writ-compiler/src/emit/body/expr/mod.rs`, the `TypedExpr::Call` arm has two branches:

**Branch A** (`!is_static_call && Func-typed callee`): handles delegate/indirect calls. Also has an early-exit for concrete struct/class receivers found via `methoddef_token_by_type_and_name` — emits direct `CALL`.

**Branch B** (all other calls): dispatches based on `callee` being a `TypedExpr::Field`:
- `TyKind::Struct(_) | TyKind::Class(_) | TyKind::Entity(_)` → `CallKind::Direct`
- `TyKind::GenericParam(_)` → `CallKind::Virtual { slot: 0 }` (slot hardcoded 0, contract_idx from `maybe_def_id`)
- `TyKind::Contract(_)` → **falls through to `_` arm → `CallKind::Direct`** — this is the bug

The fix for EMIT-01 is to add `TyKind::Contract(contract_def_id)` as an explicit arm in Branch B.

### How Contract Token Resolution Works

The pipeline for CALL_VIRT contract_idx has two halves:

**Compile time (collect phase):**
1. `collect_impl` in `emit/collect/contracts.rs` emits `ImplDef` rows that link type → contract.
2. `register_impl_method_contract(method_def_id, contract_token)` on `ModuleBuilder` stores `method_def_id → contract_token` in `method_to_contract: FxHashMap<DefId, MetadataToken>`.
3. `contract_token_for_method_def_id(def_id)` retrieves this mapping at call site.

**Problem:** `collect_impl` currently does NOT call `register_impl_method_contract`. The existing tests that validate non-zero `contract_idx` (`test_call_virt_emits_non_zero_contract_idx_when_registered`) set up this mapping manually. The full-pipeline path never registers it, so `contract_idx` will be 0 unless the collection phase is wired.

**For the contract-typed-receiver path**, however, `callee_def_id` is `None` — the type checker in `check_member_access` for `TyKind::Contract` does not attach a `callee_def_id` to the call. This means `contract_token_for_method_def_id` cannot be used. Instead, the emitter must:
1. Extract `contract_def_id` from the receiver's `TyKind::Contract(def_id)`.
2. Look up the contract's `MetadataToken` via `builder.token_for_def(contract_def_id)`.
3. Look up the method's slot from `contract_methods` by contract_def_id + method name.

### Slot Lookup by Name Pattern

`ModuleBuilder` currently exposes `contract_method_slot_by_def_id(def_id)` which always returns `None` (documented stub). For the contract-receiver path, slot lookup must go through the contract's method list by name. The builder has `contract_method_range(contract_idx)` and `contract_method_slot(cm_idx)` — but the slot is the position in the contract's method list, which corresponds to the `slot` assigned during `assign_vtable_slots`.

New method needed on `ModuleBuilder`:
```rust
pub fn contract_method_slot_by_name(&self, contract_def_id: DefId, method_name: &str) -> Option<u16>
```
This finds the ContractDef row for `contract_def_id`, iterates its ContractMethod entries in order, and returns the slot index of the entry whose name matches `method_name`. Since `assign_vtable_slots` assigns slot = position in the range, the slot equals the 0-based position within the contract's method list.

### CALL_VIRT Argument Layout

From `call.rs` lines 89-113 (existing CallKind::Virtual arm):
```rust
// CALL_VIRT layout: r_obj = receiver (self), r_base = first extra arg, argc = n-1
let r_obj = r_base;                               // receiver register
let r_args_base = if argc > 0 { r_base + 1 } else { r_base };
let n_args = if argc > 0 { argc - 1 } else { 0 };
emitter.emit(Instruction::CallVirt {
    r_dst,
    r_obj,
    contract_idx,  // MetadataToken.0 for the ContractDef
    slot,          // 0-based index in contract's method list
    r_base: r_args_base,
    argc: n_args,
});
```

For the contract-receiver path in `emit_expr`, this same layout applies. The receiver is the first argument (already emitted as self).

### End-to-End Repro Script

The "5-bug repro script" from quick-260323-vkg is:
```
pub contract MyContract {
    fn implementedFunc(self);
    fn notImplementedFunc(self);
}
pub class MyClass {}
impl MyContract for MyClass {
    fn implementedFunc(self) {}
}
pub fn main() {
    let c: MyContract = new MyClass{};
    c.implementedFunc();
    c.notImplementedFunc();
}
```

**Current state (post-Phase 84):**
- `let c: MyContract = new MyClass{}` — compiles (contract assignability works)
- `c.notImplementedFunc()` — should produce E0123 (incomplete impl validation works)
- `c.implementedFunc()` — may compile (type-check resolves the method) but emits wrong CALL (not CALL_VIRT)

**Phase 85 success state:**
- `c.implementedFunc()` compiles and emits `CALL_VIRT` that dispatches correctly at runtime
- `c.notImplementedFunc()` produces E0123 at compile time

**Note on the repro flow:** The repro has both a compile-time error path (`notImplementedFunc`) and a valid dispatch path (`implementedFunc`). For end-to-end test purposes, the test should separate them: one test validates that a complete impl + contract receiver call runs, and another validates that E0123 stops the incomplete impl at compile time.

### Anti-Patterns to Avoid

- **Slot hardcoding:** Do not hardcode `slot: 0` for contract-receiver calls. The slot must come from the contract's method order.
- **Assuming callee_def_id is Some:** For contract-typed receiver calls, `callee_def_id` is `None`. The emitter must use contract_def_id + method name for both token and slot lookup.
- **Bypassing the existing CALL_VIRT path:** Do not duplicate the CALL_VIRT emission logic. Reuse the existing `Instruction::CallVirt` emission pattern from the `CallKind::Virtual` arm in `call.rs`.
- **Emitting receiver twice:** When building arg_regs for a method call, the receiver (`self`) is the first element. Do not re-emit the receiver object when constructing `r_base`/`r_obj`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Contract token lookup by DefId | Custom HashMap scan | `builder.token_for_def(contract_def_id)` | Already in `def_token_map` after finalize |
| Method slot for CALL_VIRT | Custom slot computation | Add `contract_method_slot_by_name` to ModuleBuilder using `contract_method_range` + `contract_methods` | Range/slot data already tracked |
| Argument packing | Custom consecutive-block logic | `pack_args_consecutive` in `call.rs` | Handles the BUG-06 consecutive-args optimization |
| Contract dispatch table population | Custom dispatch table | Runtime's `exec_call_virt` — already looks up by `(type_key, contract_key, slot)` | Runtime side is complete and correct |

---

## Common Pitfalls

### Pitfall 1: callee_def_id is None for contract method calls
**What goes wrong:** The type checker's `check_member_access` for `TyKind::Contract` builds a `TypedExpr::Field` with no `callee_def_id` threaded through. When the emitter later sees `TypedExpr::Call { callee_def_id: None, ... }`, it cannot use `contract_token_for_method_def_id` to retrieve the contract token.
**Why it happens:** The contract method resolution in `access.rs` (lines 90–106) returns `TypedExpr::Field` with just the Func type — no DefId for the specific contract method is stored.
**How to avoid:** Extract contract_def_id directly from the receiver's type (`TyKind::Contract(contract_def_id)`) in the emitter, then call `builder.token_for_def(contract_def_id)` for the contract token and a new `contract_method_slot_by_name` helper for the slot.
**Warning signs:** `contract_idx = 0` in emitted CALL_VIRT, or `slot = 0` for all method calls regardless of declaration order.

### Pitfall 2: Arg packing double-emits the receiver
**What goes wrong:** The receiver is emitted first (as `self` / r_obj), then if the args include the receiver again, it gets emitted twice, producing an extra register and incorrect argc.
**Why it happens:** The contract-typed call path must emit self explicitly (Branch A in `emit_expr` already handles this for the concrete receiver path — see lines 283–290). The contract path needs the same pattern: emit receiver → emit remaining args → pack_args_consecutive.
**How to avoid:** Follow the exact pattern used for concrete-receiver calls in Branch A of `emit_expr` (lines 281–292): `emit_expr(receiver)` first, then chain it with the rest of the args.
**Warning signs:** Runtime crash on CALL_VIRT with wrong argc or r_obj pointing to wrong register.

### Pitfall 3: Contract token is 0 because register_impl_method_contract was never called
**What goes wrong:** The runtime's `resolve_type_args_hash` returns 0 when `contract_idx = 0`, causing `DispatchKey.type_args_hash = 0`. The dispatch table lookup fails because the impl was registered with a non-zero hash.
**Why it happens:** `collect_impl` does not call `register_impl_method_contract` in the full pipeline.
**How to avoid:** For the contract-receiver CALL_VIRT path, the contract token is resolved directly from `builder.token_for_def(contract_def_id)` — NOT from `contract_token_for_method_def_id`. This path does not depend on `register_impl_method_contract` at all, bypassing the problem. However, if the `callee_def_id`-based path (for GenericParam receivers) is also used, `register_impl_method_contract` needs to be called in `collect_impl` for that path.
**Warning signs:** CALL_VIRT emits non-zero `contract_idx` in unit tests but 0 in the full pipeline.

### Pitfall 4: Slot assignment happens before or after finalize at wrong time
**What goes wrong:** `contract_method_slot_by_name` is called before `assign_vtable_slots` runs, returning uninitialized slot values (all 0).
**Why it happens:** `assign_vtable_slots` is called from `emit/mod.rs` or `emit/collect/mod.rs` before body emission starts. If the slot lookup happens during collection (before vtable slot assignment), the values will be 0.
**How to avoid:** Verify the call order in `emit/mod.rs`: slots should be assigned before `emit_all_bodies` runs. The current call to `slots::assign_vtable_slots(builder)` happens in the collect phase before body emission. Confirm this is the case by checking `writ-compiler/src/emit/mod.rs`.
**Warning signs:** All contract method calls emit `slot: 0` regardless of declaration position.

### Pitfall 5: Branch A short-circuit intercepts contract-receiver calls
**What goes wrong:** Branch A of the `TypedExpr::Call` arm checks `!is_static_call && Func-typed callee`. For contract method calls, `callee_def_id` is `None` (so `is_static_call = false`) and the callee type IS `TyKind::Func`. The Branch A inner check for `receiver_def_id` (via `extract_type_def_id`) will return `None` for a contract receiver (since `TyKind::Contract` is not handled by `extract_type_def_id`), so it falls through to `emit_call_indirect`. This produces a CALL_INDIRECT instead of CALL_VIRT.
**Why it happens:** `extract_type_def_id` only handles `TyKind::Struct | Class | Entity | Enum` — not `TyKind::Contract`.
**How to avoid:** Either: (a) handle the contract receiver case before the `!is_static_call` branch by checking if the receiver type is `TyKind::Contract` and emitting CALL_VIRT immediately; or (b) extend `extract_type_def_id` to return `None` but add a separate `is_contract_receiver` check in Branch A that routes to CALL_VIRT. Option (a) is cleaner.
**Warning signs:** Contract method calls emit CALL_INDIRECT instead of CALL_VIRT.

---

## Code Examples

### Pattern: Detecting contract receiver and emitting CALL_VIRT

This is the pattern to add inside `TypedExpr::Call` in `emit_expr`, before Branch A's `!is_static_call` check:

```rust
// Source: writ-compiler/src/emit/body/expr/mod.rs (TypedExpr::Call arm)
// Contract-typed receiver dispatch (EMIT-01, EMIT-02)
if !is_static_call {
    if let TypedExpr::Field { receiver, field, .. } = callee.as_ref() {
        if let TyKind::Contract(contract_def_id) = emitter.interner.kind(receiver.ty()).clone() {
            let TypedExpr::Call { ty, args, .. } = expr else { unreachable!() };
            let r_dst_call = emitter.alloc_reg(*ty);

            // Emit self (receiver) first, then remaining args
            let r_self = emit_expr(emitter, receiver);
            let arg_regs: Vec<u16> = std::iter::once(r_self)
                .chain(args.iter().map(|arg| emit_expr(emitter, arg)))
                .collect();
            let r_base = pack_args_consecutive(emitter, &arg_regs);

            // Resolve contract token and slot
            let contract_token = emitter.builder.token_for_def(contract_def_id)
                .map(|t| t.0)
                .unwrap_or(0);
            let slot = emitter.builder.contract_method_slot_by_name(contract_def_id, field)
                .unwrap_or(0);

            // CALL_VIRT layout: r_obj = receiver, r_base = first extra arg
            let r_obj = r_base;
            let r_args_base = if arg_regs.len() > 1 { r_base + 1 } else { r_base };
            let n_args = (arg_regs.len() as u16).saturating_sub(1);
            emitter.emit(Instruction::CallVirt {
                r_dst: r_dst_call,
                r_obj,
                contract_idx: contract_token,
                slot,
                r_base: r_args_base,
                argc: n_args,
            });
            return r_dst_call;
        }
    }
}
```

### Pattern: New ModuleBuilder helper for slot lookup by name

```rust
// Source: writ-compiler/src/emit/module_builder.rs
/// Look up the CALL_VIRT slot for a contract method by contract DefId and method name.
///
/// Searches the ContractMethod entries for the ContractDef registered with `contract_def_id`,
/// returning the 0-based slot index of the entry whose name matches `method_name`.
/// Slots are assigned by `assign_vtable_slots` in declaration order (0, 1, 2, ...).
///
/// Returns None if the contract is not registered or the method is not found.
pub fn contract_method_slot_by_name(&self, contract_def_id: DefId, method_name: &str) -> Option<u16> {
    // Find the ContractDef index for this DefId
    let contract_idx = self.contract_def_def_ids
        .iter()
        .position(|id| id.as_ref() == Some(&contract_def_id))?;
    // Iterate the contract's methods in range order; slot = 0-based position
    let range = self.contract_method_range(contract_idx);
    for (slot, cm_idx) in range.enumerate() {
        let name_in_heap = self.string_heap.get_str(self.contract_methods[cm_idx].row.name);
        if name_in_heap == method_name {
            return Some(slot as u16);
        }
    }
    None
}
```

### Pattern: End-to-end test structure

Tests for EMIT-04 should follow the pattern used in `writ-runtime/tests/speaker_dispatch_tests.rs` and `emit_body_tests.rs`:

```rust
// In writ-compiler/tests/emit_body_tests.rs (or a new integration test)
// Test: contract-typed receiver call emits CALL_VIRT with correct contract_idx and slot
#[test]
fn test_contract_receiver_emits_call_virt() {
    // 1. Build module with a contract and an impl
    // 2. Register method -> contract mapping via register_impl_method_contract
    //    OR rely on the new name-based lookup via contract_method_slot_by_name
    // 3. Build a TypedExpr::Call with Field on a Contract-typed receiver
    // 4. Call emit_expr
    // 5. Assert: Instruction::CallVirt is present
    // 6. Assert: contract_idx != 0 and equals the contract's MetadataToken.0
    // 7. Assert: slot matches the method's position in the contract's declaration order
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Contract-as-type was E0122 error | Contract-as-type resolves to TyKind::Contract(DefId) | Phase 84-01 (2026-03-23) | Phase 85 can now reach codegen with contract-typed receivers |
| Contract method calls fell through to error | Contract method calls resolve via contract_methods in access.rs | Phase 84-02 (2026-03-23) | TypedExpr::Field + TypedExpr::Call now produced for contract method calls |
| CALL_VIRT for GenericParam receivers only | CALL_VIRT also needed for Contract receivers | Phase 85 (this phase) | Adds TyKind::Contract arm to emit_expr dispatch |

---

## Open Questions

1. **Is there a full pipeline integration test harness?**
   - What we know: `emit_body_tests.rs` tests emit units in isolation. The runtime tests (e.g., `speaker_dispatch_tests.rs`) run full IL through the VM.
   - What's unclear: Whether there's a "compile Writ source and run" integration test that covers the full path from source to execution. Such a test would be the cleanest EMIT-04 coverage.
   - Recommendation: Check `writ-runtime/tests/vm_tests.rs` for an existing pattern; if a "compile + run" helper exists there, use it for EMIT-04. Otherwise, a unit test in `emit_body_tests.rs` verifying CALL_VIRT emission on a contract-receiver TypedExpr is sufficient for EMIT-01/EMIT-02, and a typecheck test can verify that the repro script with a complete impl produces no errors (EMIT-04 compile half).

2. **Does collect_impl need to call register_impl_method_contract for the GenericParam CALL_VIRT path?**
   - What we know: The contract-receiver CALL_VIRT path (EMIT-01/02) does NOT need `register_impl_method_contract` — it uses `token_for_def(contract_def_id)` directly. The existing GenericParam path in `emit_expr` (line 309-311) uses `contract_token_for_method_def_id(callee_def_id)` which requires the registration.
   - What's unclear: Whether Phase 85 needs to fix the GenericParam path too or just the Contract-receiver path.
   - Recommendation: Focus Phase 85 on the Contract-receiver path (EMIT-01/02). Fix GenericParam path wiring (register_impl_method_contract in collect_impl) only if needed for EMIT-04.

---

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — pure Rust source changes within existing crates).

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness |
| Config file | none — `cargo test` |
| Quick run command | `cargo test -p writ-compiler --test emit_body_tests -- contract` |
| Full suite command | `cargo test -p writ-compiler` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| EMIT-01 | Contract-typed receiver call emits CALL_VIRT not CALL | unit | `cargo test -p writ-compiler --test emit_body_tests -- test_contract_receiver_emits_call_virt` | ❌ Wave 0 |
| EMIT-02 | CALL_VIRT carries correct contract_idx and slot | unit | `cargo test -p writ-compiler --test emit_body_tests -- test_contract_receiver_call_virt_correct_idx_and_slot` | ❌ Wave 0 |
| EMIT-04 | Repro script: implementedFunc path compiles and runs; notImplementedFunc catches E0123 | unit | `cargo test -p writ-compiler --test typecheck_tests -- test_contract_receiver_repro_script` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-compiler --test emit_body_tests -- contract`
- **Per wave merge:** `cargo test -p writ-compiler`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] Tests in `writ-compiler/tests/emit_body_tests.rs` for EMIT-01 and EMIT-02 — contract receiver CALL_VIRT emission
- [ ] Test in `writ-compiler/tests/typecheck_tests.rs` for EMIT-04 — end-to-end repro script (complete impl compiles, incomplete impl catches E0123)
- [ ] `contract_method_slot_by_name` method on `ModuleBuilder` in `writ-compiler/src/emit/module_builder.rs`

---

## Sources

### Primary (HIGH confidence)

- Direct source inspection — `writ-compiler/src/emit/body/expr/mod.rs` TypedExpr::Call arm (lines 253–368)
- Direct source inspection — `writ-compiler/src/emit/body/call.rs` analyze_callee and CallKind::Virtual arm
- Direct source inspection — `writ-compiler/src/emit/module_builder.rs` ModuleBuilder fields and query methods
- Direct source inspection — `writ-compiler/src/emit/collect/contracts.rs` collect_impl
- Direct source inspection — `writ-compiler/src/emit/slots.rs` assign_vtable_slots
- Direct source inspection — `writ-runtime/src/dispatch/calls.rs` exec_call_virt (runtime side)
- Direct source inspection — `writ-compiler/src/check/check_expr/access.rs` TyKind::Contract arm
- Direct source inspection — Phase 84 verification report (all 8 truths verified, 88 tests passing)
- Direct source inspection — `writ-compiler/tests/emit_body_tests.rs` existing CALL_VIRT tests (lines 2851–3303)

### Secondary (MEDIUM confidence)

- Quick task summary 260323-vkg — confirms E0123 and E0122 are working; repro script behavior documented
- Phase 84-02 summary — confirms contract method resolution via contract_methods is wired in access.rs

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — pure internal crate changes, no new dependencies
- Architecture: HIGH — source inspected directly; all relevant code read and cross-referenced
- Pitfalls: HIGH — identified from direct code reading of the dispatch branches and known patterns from prior bug-fixes (BUG-06, BUG-07, FIX-02)

**Research date:** 2026-03-24
**Valid until:** Indefinite — no external dependencies; stale only if emit_expr architecture changes
