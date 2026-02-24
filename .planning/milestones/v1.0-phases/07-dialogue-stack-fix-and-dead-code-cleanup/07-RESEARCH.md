# Phase 7: Dialogue Stack Fix and Dead Code Cleanup - Research

**Researched:** 2026-02-27
**Domain:** Rust compiler internals — dialogue lowering pass, dead code removal, doc comment accuracy
**Confidence:** HIGH (all findings from direct source inspection, no external dependencies)

## Summary

Phase 7 is a pure codebase cleanup phase targeting three surgical changes to `writ-compiler/src/lower/`. No new functionality is introduced. All three changes were catalogued in the v1.0 milestone audit (`SPEAKER_STACK_LEAK`, dead `next_loc_key()` method, and stale `stmt.rs` doc comment). The changes are independent and low-risk: the speaker stack fix is a save/restore pattern already used in `lower_choice`, the dead code removal deletes fields and a method with zero call sites, and the doc comment fix is a text edit.

The most important planning constraint is **no snapshot changes**: the speaker stack leak is currently unobservable by tests (each test uses a fresh `LoweringContext`), and the dead code removal touches no code path that emits AST nodes. The planner should verify this with a `cargo test` run after each change.

**Primary recommendation:** Three independent tasks, each committed atomically. Stack fix first (highest risk), dead code second, doc comment third.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| R8 | Dialogue Lowering — `lower_dialogue()` correctly handles all dialogue constructs including speaker scoping | Stack-drain fix ensures speaker scopes from `SpeakerTag` lines in one `dlg` item do not persist into the next `dlg` item in the same `lower()` call |
| R11 | Dialogue Transition Validation — transition semantics correct | Dead code removal of `next_loc_key()` does not affect transition validation; doc comment fix in `stmt.rs` makes the comment accurate for `Stmt::Transition` handling |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust (std) | 2024 edition | All implementation | Project language |
| insta | existing | Snapshot test framework | Already in use, all 69 tests use it |
| cargo test | built-in | Test runner | Standard Rust toolchain |

No new dependencies. No installation required.

## Architecture Patterns

### Pattern 1: Save/Restore Stack Depth (already used in lower_choice)

**What:** Record `ctx.speaker_stack_depth()` before entering a sub-scope, then drain the stack back to that depth after returning.

**When to use:** Any boundary that should not leak speaker state into the caller.

**Example — existing pattern in `dialogue.rs` `lower_choice`:**
```rust
// Source: writ-compiler/src/lower/dialogue.rs lines 517–538
let depth = ctx.speaker_stack_depth();

// Lower arm body (may push SpeakerTag scopes)
let body = lower_dlg_lines(&arm.body, state, ctx);

// Restore speaker scope
while ctx.speaker_stack_depth() > depth {
    ctx.pop_speaker();
}
```

**Application for Phase 7:** The same pattern must be applied in `lower_dialogue()` itself, around the entire call to `lower_dlg_lines`, so that any `SpeakerTag` pushes inside the dialogue body do not leak into the next `Item::Dlg` processed by the `lower()` loop.

### Pattern 2: Dead Field + Method Removal

**What:** Delete `loc_key_counter: u32` field from `LoweringContext`, delete `next_loc_key()` method, delete the `loc_key_counter: 0` initializer in `LoweringContext::new()`. Verify zero call sites exist before removing.

**Verification before removal:**
```bash
# Source: confirmed by audit — zero call sites
grep -rn "next_loc_key\|loc_key_counter" writ-compiler/src/
```

The audit confirmed `next_loc_key()` is never called. Actual key generation uses `fnv1a_32()` privately in `dialogue.rs`. The field and method are dead code introduced during Phase 1 infrastructure setup, before Phase 4 decided to use content-based FNV-1a hashing instead of a sequential counter.

### Pattern 3: Doc Comment Text Edit

**What:** Replace the stale comment in `stmt.rs` that claims `DlgDecl`/`Transition` use `todo!()`.

**Current comment (lines 8–14 in `stmt.rs`):**
```rust
/// Folds a CST `Stmt` into a lowered `AstStmt`.
///
/// Calls `lower_expr` on all expression sub-nodes and `lower_type` on
/// any type annotation sub-nodes.
///
/// `Stmt::DlgDecl` and `Stmt::Transition` use `todo!()` — these are
/// dialogue-specific statements handled in Phase 4.
```

**Accurate replacement:** The comment should note that both variants are fully implemented — `DlgDecl` lowers to a `let` binding wrapping a lambda, and `Transition` lowers to `AstStmt::Return`.

### Anti-Patterns to Avoid

- **Touching snapshots:** Neither the stack fix nor the dead code removal should change any snapshot output. If a snapshot changes, something has gone wrong.
- **Removing `speaker_stack_depth()` from context.rs:** This method is used by `lower_choice` and will be used by the stack fix. Do NOT remove it.
- **Removing `push_speaker`/`pop_speaker`:** These are used by both the stack fix and `lower_choice`. Only `next_loc_key()` and `loc_key_counter` are dead.
- **Changing `loc_key_counter` doc comment on `LoweringContext`:** The struct's doc comment in `context.rs` also mentions `loc_key_counter` ("generates deterministic localization keys via `next_loc_key()`"). Update this doc comment too when removing the field/method.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Stack save/restore | Custom scope guard type | Existing `speaker_stack_depth()` + drain loop | Pattern already established in `lower_choice`, consistent with codebase |
| Dead code detection | Manual audit | `grep` + Rust compiler warnings | Compiler will flag unused `loc_key_counter` field if `#[allow(dead_code)]` is removed; search confirms zero call sites |

## Common Pitfalls

### Pitfall 1: Over-Draining the Stack

**What goes wrong:** The save/restore in `lower_dialogue()` saves depth at function entry. If the caller (`lower()`) already has items on the speaker stack from prior operations (shouldn't happen, but defensively), over-draining could corrupt the caller's state.

**Why it happens:** The drain loop `while ctx.speaker_stack_depth() > depth` uses the saved `depth`, not zero. This is correct and safe — it only removes what `lower_dialogue` pushed.

**How to avoid:** Save depth at the top of `lower_dialogue()` before `lower_dlg_lines` is called, not before the function entry checks. The save goes after the `param_names` collection and `singleton_speakers` pre-scan (neither of which touch the speaker stack), but before `lower_dlg_lines`.

**Warning signs:** If the stack is over-drained, the drain loop would pop entries pushed by callers — but since `lower()` starts with an empty stack and processes items sequentially, this cannot happen in practice.

### Pitfall 2: Snapshot Regression from Stack Fix

**What goes wrong:** Adding the drain in `lower_dialogue()` changes the `speaker_stack` state at the end of a `lower()` call, which could in theory affect any test that passes multiple `dlg` items in sequence.

**Why it happens:** The drain itself emits no AST nodes — it only modifies `ctx.speaker_stack`. The lowered output is already fully produced by `lower_dlg_lines` before the drain runs.

**How to avoid:** Confirm no snapshot changes after the fix. Run `cargo test -p writ-compiler` and verify all 69 tests still pass with no `INSTA_UPDATE` needed.

**Warning signs:** Any `insta` snapshot mismatch indicates the fix touched the AST output path, which would be a bug.

### Pitfall 3: Missing the Struct Doc Comment

**What goes wrong:** `LoweringContext`'s struct-level doc comment in `context.rs` (lines 13–18) lists `loc_key_counter` as a field and references `next_loc_key()`. If only the method and field are removed but the doc comment is left unchanged, the doc comment becomes stale again.

**Current doc comment in context.rs:**
```rust
/// Shared mutable state threaded through every lowering pass.
///
/// Passes receive `&mut LoweringContext` and:
/// - Append errors via `emit_error()` (pipeline never halts)
/// - Push/pop speaker scopes (dialogue lowering)
/// - Generate deterministic localization keys via `next_loc_key()`
```

**How to avoid:** When removing `next_loc_key()` and `loc_key_counter`, also update this struct-level doc comment. Remove the third bullet point or replace it with an accurate statement (keys are generated by `fnv1a_32()` internally in `dialogue.rs`).

### Pitfall 4: Zero-Warning Build Discipline

**What goes wrong:** After removing `loc_key_counter` and `next_loc_key()`, if a `#[allow(dead_code)]` attribute was added somewhere to suppress the warning, it must also be removed.

**How to avoid:** Search for any `allow(dead_code)` annotations in `context.rs` before committing. The project uses a zero-warning build policy (confirmed by audit).

## Code Examples

### Stack Fix — Where to Insert in lower_dialogue()

```rust
// Source: writ-compiler/src/lower/dialogue.rs
pub fn lower_dialogue(
    dlg: DlgDecl<'_>,
    dlg_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstFnDecl {
    // ... (params, singleton pre-scan, hoisted_stmts) ...

    // --- INSERT: save speaker stack depth before lowering body ---
    let speaker_depth_before = ctx.speaker_stack_depth();

    // Lower the dialogue body
    let body_stmts = lower_dlg_lines(&dlg.body, &mut state, ctx);

    // --- INSERT: restore speaker stack to pre-call depth ---
    while ctx.speaker_stack_depth() > speaker_depth_before {
        ctx.pop_speaker();
    }

    // Combine hoisted + body
    hoisted_stmts.extend(body_stmts);

    AstFnDecl { /* ... */ }
}
```

### Dead Code Removal — Fields/Methods to Delete in context.rs

```rust
// REMOVE these from LoweringContext:
//   Field:    loc_key_counter: u32,
//   Init:     loc_key_counter: 0,
//   Method:   pub fn next_loc_key(&mut self) -> u32 { ... }

// KEEP (used by lower_choice and the stack fix):
//   speaker_stack: Vec<SpeakerScope>,
//   push_speaker(), pop_speaker(), current_speaker(), speaker_stack_depth()
```

### Doc Comment Fix — stmt.rs

```rust
// BEFORE (lines 8–14):
/// `Stmt::DlgDecl` and `Stmt::Transition` use `todo!()` — these are
/// dialogue-specific statements handled in Phase 4.

// AFTER:
/// `Stmt::DlgDecl` lowers to a `let` binding wrapping the dialogue body as a lambda.
/// `Stmt::Transition` lowers to `AstStmt::Return` with the target as a `Call` expression.
```

## Validation Architecture

> `workflow.nyquist_validation` is not set (key absent from config.json) — treating as false. Skipping Validation Architecture section.

Note: The config.json has `"workflow": { "research": true, "plan_check": true, "verifier": true }` with no `nyquist_validation` key. Section omitted per instructions.

### Test Continuity Contract

All 69 existing tests must pass with zero snapshot changes. The test command is:

```bash
cargo test -p writ-compiler
```

The stack fix specifically should be verified by observing that:
1. Tests still pass (output unchanged)
2. A new regression test can be added that lowers two sequential `dlg` items and confirms the second item's speaker state is clean (optional — the phase success criteria does not require a new test)

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `next_loc_key()` counter-based key gen | `fnv1a_32()` content-hash key gen | Phase 4 | Counter is dead; remove it |
| `todo!()` stubs for DlgDecl/Transition | Fully implemented in stmt.rs | Phase 4 | Doc comment must be updated |
| No stack drain after lower_dialogue | Should drain to pre-call depth | Phase 7 (this phase) | Prevents stack leak across sequential dlg items |

## Open Questions

1. **Should a new regression test be added for the stack fix?**
   - What we know: Phase 7 success criteria item 4 says "existing 69 tests continue to pass with no snapshot changes" — it does not mandate a new test
   - What's unclear: Whether proving the fix without a dedicated regression test is sufficient
   - Recommendation: Add a focused regression test that lowers two sequential `dlg` items with `SpeakerTag` lines and verifies the second processes correctly. This is low-effort (single test function, no snapshot needed for verification of the invariant) and proves the fix at the unit level. However, this is discretionary — the phase success criteria does not require it.

2. **Should `AstImplMember::Op` dead variant also be cleaned up?**
   - What we know: The audit flagged it as "Low (may be intentional)" and Phase 7 scope explicitly excludes it (not listed in phase success criteria)
   - Recommendation: Leave it for a future phase. Removing enum variants is a breaking change if downstream code pattern-matches on them.

## Sources

### Primary (HIGH confidence)

All findings sourced directly from the codebase via source inspection:

- `writ-compiler/src/lower/context.rs` — `LoweringContext` struct, `speaker_stack`, `loc_key_counter`, `next_loc_key()`, `speaker_stack_depth()`
- `writ-compiler/src/lower/dialogue.rs` — `lower_dialogue()`, `lower_dlg_lines()`, `lower_choice()` (save/restore pattern at lines 517–538)
- `writ-compiler/src/lower/stmt.rs` — stale doc comment at lines 8–14
- `.planning/v1.0-MILESTONE-AUDIT.md` — SPEAKER_STACK_LEAK gap, dead code items, doc comment item
- `writ-compiler/tests/lowering_tests.rs` — 69 existing tests, `lower_src` and `lower_src_with_errors` helpers

### Secondary (MEDIUM confidence)

- `.planning/STATE.md` — Key decision "assert_debug_snapshot over assert_ron_snapshot", zero-warning build discipline
- `.planning/REQUIREMENTS.md` — R8, R11 acceptance criteria

### Tertiary (LOW confidence)

None — all claims are verifiable from source.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, all Rust stdlib + existing insta
- Architecture: HIGH — stack fix pattern already in codebase at `lower_choice`
- Pitfalls: HIGH — all sourced from direct code inspection and audit findings

**Research date:** 2026-02-27
**Valid until:** Indefinite — this is internal code, not a moving target
