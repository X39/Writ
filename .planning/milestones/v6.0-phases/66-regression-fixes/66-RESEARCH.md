# Phase 66: Regression Fixes - Research

**Researched:** 2026-03-18
**Domain:** Rust dead_code elimination, insta snapshot testing, dialogue lowering
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| WARN-02 | `cargo clippy` exits clean with zero warnings | Fix 6 dead_code warnings (infer.rs + mutability.rs + scope.rs) and restore say() 2-arg signature so type checker no longer errors on 1-arg calls |
</phase_requirements>

---

## Summary

Phase 66 closes two regressions introduced by earlier phases in the v6.0 milestone. Both regressions are fully characterised by the v6.0 milestone audit (`v6.0-MILESTONE-AUDIT.md`) and confirmed by running `cargo clippy --workspace` and the test suite against the current working tree.

**Regression 1 — 6 dead_code warnings** (introduced by Phase 65-03 `pub` -> `pub(crate)` narrowing): Narrowing the `check/infer`, `check/mutability`, and `resolve/scope` submodules from `pub mod` to `pub(crate) mod` allowed rustc to see that 6 items — never called from outside the crate — are genuinely dead. The items are leftover scaffolding, not intentional API surface. Correct fix is deletion.

**Regression 2 — say() emits 1 argument instead of 2** (introduced by Phase 62-01 `cargo clippy --fix`): The auto-fix pass renamed `speaker_ref` to `_speaker_ref` in `make_say` and `make_say_localized`. Because the parameter is now prefixed with `_`, it is no longer used in the function body — but the parameter was already present as `_speaker_ref`. The clippy fix also silently deleted `speaker_ref` from the `args` vec that builds the call. This changed `say(speaker, text)` to `say(text)` — a semantic regression visible in 29 snapshot failures and 1 emit test failure. The correct fix is restoring the 2-argument call and accepting the `.snap.new` files (which now show the 1-arg form) by deleting them so the original `.snap` baselines remain authoritative.

**Primary recommendation:** Delete the 6 dead items; restore the speaker argument in `make_say`/`make_say_localized`; delete all 29 `.snap.new` files so the green baselines are the authoritative snapshots.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| cargo clippy | bundled with rustc | Lint checking | Project requirement WARN-02 |
| insta | 1.x | Snapshot testing | Already used in writ-compiler snapshot tests |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cargo test | bundled | Running tests | Confirming all 112 lowering tests and emit tests pass |

**Run commands (verification):**
```bash
# Zero warnings check
cargo clippy --workspace

# Snapshot acceptance (delete stale .snap.new files, keep .snap baselines)
# No cargo insta review needed — the baselines are already correct
# The .snap.new files represent the regressed 1-arg format; we restore 2-arg

# Test confirmation
cargo test -p writ-compiler --test lowering_tests
cargo test -p writ-compiler --test emit_tests
```

---

## Architecture Patterns

### What Was Broken and Why

#### Dead Code: Phase 65-03 pub(crate) narrowing

Phase 65-03 narrowed three submodule declarations in `writ-compiler/src/check/mod.rs` and `writ-compiler/src/resolve/mod.rs` from `pub mod` to `pub(crate) mod`. Prior to narrowing, these modules were externally visible, so rustc suppressed dead_code analysis for their items (items on a `pub` module could always be used by an external crate). After narrowing to `pub(crate)`, rustc can see that nothing in the workspace uses these items.

**Confirmed dead items (cargo clippy --workspace output, 2026-03-18):**

1. `check/infer.rs:13` — `pub fn resolve_type_to_ty(...)` — called only from within `infer.rs` itself (recursive calls on lines 42, 48, 50). No external callers. The public-facing function that check_expr uses is `instantiate_generic_fn` and `substitute`, not `resolve_type_to_ty` directly.

2. `check/mutability.rs:20` — `pub fn check_method_mutation(...)` — defined, not called anywhere in the workspace. The mutation checking in `check_expr/mod.rs` uses `find_root_binding` from `check_expr/mod.rs` directly (its own private copy), not the one in `mutability.rs`.

3. `check/mutability.rs:47` — `fn find_root_binding(...)` — private to `mutability.rs`, only callable from `check_method_mutation` which itself is dead. Transitively dead.

4. `resolve/scope.rs:37` — `ScopeLayer::Locals(...)` — variant never constructed. The enum exists for `GenericParams`; `Locals` was scaffolding for future local-variable tracking in the resolver but resolver.rs never calls `push_locals`/`add_local`.

5. `resolve/scope.rs:62` — `LookupResult::BuiltinVariant(String)` — returned only from `resolve_value` which is itself dead (never called from resolver.rs). The resolver uses `scope.resolve_type()` for type resolution and does its own local-binding tracking.

6. `resolve/scope.rs:100,105,302` — `push_locals`, `add_local`, `resolve_value` methods — never called from `resolver.rs` or any other file outside `scope.rs`. Confirmed by grep: zero external callers.

**Correct fix:** Delete all 6 items. They are scaffolding, not suppressed intentional API. Suppressing with `#[allow(dead_code)]` would be incorrect since the audit recommends removal ("they appear to be leftover scaffolding" per the audit report).

**Impact of deletion:**
- `resolve_type_to_ty`: Deleting the function also means removing the recursive self-calls on lines 42, 48, 50 within `infer.rs` — but those are its own recursive calls, not external uses. Deletion is clean.
- `check_method_mutation` + `find_root_binding` in `mutability.rs`: Entire module body becomes empty (only the `use` imports remain). Can remove the unused imports too. The module declaration (`pub(crate) mod mutability;`) can remain but the file content reduces to just the `//!` doc header — or the module can be removed entirely if there's no planned future use. Safest: remove the two functions, keep the module with its doc header.
- `ScopeLayer::Locals`, `push_locals`, `add_local`, `resolve_value`, `LookupResult::BuiltinVariant`: Remove the variant from the enum, remove the three methods from the impl block. Also remove `resolve_value`'s internal usage of `ScopeLayer::Locals` (that match arm is the only place `Locals` is matched). Also remove `LookupResult::BuiltinVariant` from the match in `resolver.rs:708` if it references this variant.

#### Say() Argument: Phase 62-01 clippy --fix

`cargo clippy --fix` renamed `speaker_ref` -> `_speaker_ref` in `make_say` and `make_say_localized` to silence an "unused variable" warning. This was correct for the lint but incorrect semantically — the parameter was used to pass the speaker entity as the first argument to the `say()` call. With `_speaker_ref`, clippy's auto-fix also deleted the argument from the `args` vec in both functions.

**Current state of `make_say` (line 563-578, `lower/dialogue.rs`):**
```rust
fn make_say(
    _speaker_ref: AstExpr,   // <- prefixed with _  (clippy "fixed")
    text: AstExpr,
    span: SimpleSpan,
) -> AstExpr {
    AstExpr::Call {
        callee: ...,
        args: vec![
            AstArg { name: None, value: text, span },  // <- only text, speaker MISSING
        ],
        ...
    }
}
```

**Required state (restoring pre-Phase-62 behavior):**
```rust
fn make_say(
    speaker_ref: AstExpr,    // <- no _ prefix, parameter IS used
    text: AstExpr,
    span: SimpleSpan,
) -> AstExpr {
    AstExpr::Call {
        callee: ...,
        args: vec![
            AstArg { name: None, value: speaker_ref, span },  // <- speaker FIRST
            AstArg { name: None, value: text, span },          // <- text SECOND
        ],
        ...
    }
}
```

Same pattern for `make_say_localized` (lines 580-605): restore `speaker_ref` parameter name and restore it as the first element of the `args` vec. The function also has `_speaker_ref` as the first parameter currently.

**Snapshot handling:** The 29 `.snap.new` files record the regressed 1-argument format. Once we restore 2-argument emission, the tests will match the original `.snap` baselines again. The `.snap.new` files must be deleted — not accepted — because they represent the wrong behavior.

```bash
# Delete all stale .snap.new files
rm writ-compiler/tests/snapshots/*.snap.new
```

After deletion + code fix, `cargo test -p writ-compiler --test lowering_tests` must pass all 112 tests against the original `.snap` baselines.

### Verification Dependencies

The emit test `choice_option_emits_externdef` declares `say(speaker: Entity, text: string)` (2 params) and calls a dlg that emits 3 `say()` calls. With the 1-arg emitter, the type checker raises E0101 "function `say` expects 2 arguments but 1 provided" causing `diags.is_empty()` to fail. After restoring 2-arg emission, the type checker sees matching arity and the test passes.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Snapshot acceptance | Script to rewrite .snap content | Delete .snap.new files; let existing .snap baselines stand | The baselines are already correct |
| Suppressing dead_code | `#[allow(dead_code)]` annotations | Delete the dead items | Suppression masks real problems; deletion is correct cleanup |

---

## Common Pitfalls

### Pitfall 1: Attempting to Suppress Instead of Delete

**What goes wrong:** Adding `#[allow(dead_code)]` to the 6 items instead of removing them.
**Why it happens:** Suppression feels "safer" than deletion.
**How to avoid:** The audit explicitly says "Removal preferred — they appear to be leftover scaffolding." Deletion is the correct action.
**Warning signs:** If any `#[allow(dead_code)]` appears in the diff, it's wrong.

### Pitfall 2: Accepting .snap.new Files Instead of Deleting Them

**What goes wrong:** Running `cargo insta review` or `cargo insta accept` to accept the 1-arg snapshots, then restoring the 2-arg code.
**Why it happens:** Confusion about which is the "new" truth.
**How to avoid:** The `.snap.new` files represent the regressed output. The `.snap` baselines are the correct expected output. Delete the `.snap.new` files, restore the code, tests pass against `.snap`.

### Pitfall 3: Removing resolve_value Without Cleaning Up Its Dependencies

**What goes wrong:** Removing `resolve_value` but leaving `LookupResult::BuiltinVariant` in the enum and/or leaving the match arm in `resolver.rs:708`.
**Why it happens:** Forgetting to grep for usages of the removed item.
**How to avoid:** After removing `resolve_value`, verify `LookupResult::BuiltinVariant` has no remaining construction sites. The only construction site was `resolve_value` itself (`return LookupResult::BuiltinVariant(...)`). The match arm in `resolver.rs` at line 708 matches on `LookupResult::BuiltinVariant(_)` — that arm must also be removed.

### Pitfall 4: Leaving Unused Imports in mutability.rs After Deleting Functions

**What goes wrong:** Deleting `check_method_mutation` and `find_root_binding` from `mutability.rs` but leaving the `use` imports for `CheckCtx`, `TypedExpr`, `LocalEnv`, `Mutability`, `TypeError`.
**Why it happens:** Deleting function bodies but not cleaning up imports.
**How to avoid:** After removing both functions, delete all `use` statements that were only there to support them. Leaving them will produce "unused import" clippy warnings.

### Pitfall 5: Breaking the mutability.rs doc header

**What goes wrong:** Deleting the file content so completely that the `//!` doc comment is also removed.
**Why it happens:** Blanket deletion.
**How to avoid:** Retain the `//!` module doc header. The module declaration still exists in `check/mod.rs` and should have its module file present.

---

## Code Examples

Verified from current source files:

### Fix 1: make_say restored (lower/dialogue.rs, ~line 563)

```rust
// Source: writ-compiler/src/lower/dialogue.rs (restore to pre-Phase-62 state)
fn make_say(
    speaker_ref: AstExpr,    // RESTORED: no _ prefix
    text: AstExpr,
    span: SimpleSpan,
) -> AstExpr {
    AstExpr::Call {
        callee: Box::new(AstExpr::Ident {
            name: "say".to_string(),
            span,
        }),
        args: vec![
            AstArg { name: None, value: speaker_ref, span },  // RESTORED: speaker first
            AstArg { name: None, value: text, span },
        ],
        span,
    }
}
```

### Fix 2: make_say_localized restored (lower/dialogue.rs, ~line 580)

```rust
// Source: writ-compiler/src/lower/dialogue.rs (restore to pre-Phase-62 state)
fn make_say_localized(
    speaker_ref: AstExpr,    // RESTORED: no _ prefix
    loc_key: String,
    fallback: AstExpr,
    span: SimpleSpan,
) -> AstExpr {
    AstExpr::Call {
        callee: Box::new(AstExpr::Ident {
            name: "say_localized".to_string(),
            span,
        }),
        args: vec![
            AstArg { name: None, value: speaker_ref, span },  // RESTORED: speaker first
            AstArg {
                name: None,
                value: AstExpr::StringLit { value: loc_key, span },
                span,
            },
            AstArg { name: None, value: fallback, span },
        ],
        span,
    }
}
```

### Fix 3: Delete 6 dead items

```
writ-compiler/src/check/infer.rs
  - Remove: pub fn resolve_type_to_ty(...) { ... }  (lines 13-77)
    NOTE: The recursive self-calls on lines 42, 48, 50 are inside this function.
    Removing the entire function removes them too. instantiate_generic_fn and
    substitute remain — they have real callers.

writ-compiler/src/check/mutability.rs
  - Remove: pub fn check_method_mutation(...) { ... }  (lines 20-44)
  - Remove: fn find_root_binding(...) { ... }  (lines 47-66)
  - Remove: all use statements (lines 12-18) that are only used by these two functions
  - Keep: //! module doc header

writ-compiler/src/resolve/scope.rs
  - Remove: ScopeLayer::Locals(Vec<(String, SimpleSpan)>),  (line 37)
  - Remove: LookupResult::BuiltinVariant(String),  (line 62 with doc comment above)
  - Remove: push_locals method  (lines 100-102)
  - Remove: add_local method  (lines 105-115)
  - Remove: resolve_value method  (lines 302-330)

writ-compiler/src/resolve/resolver.rs
  - Remove: LookupResult::BuiltinVariant(_) match arm  (around line 708)
```

### Fix 4: Delete stale snapshot files

```bash
rm writ-compiler/tests/snapshots/*.snap.new
```

This removes all 29 `.snap.new` files. The original `.snap` baselines (2-arg format) remain authoritative.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `pub fn resolve_type_to_ty` | Delete — never called | Phase 65-03 exposed | -1 clippy warning |
| `pub fn check_method_mutation` | Delete — dead scaffolding | Phase 65-03 exposed | -1 clippy warning |
| `fn find_root_binding` in mutability.rs | Delete — transitively dead | Phase 65-03 exposed | -1 clippy warning |
| `ScopeLayer::Locals` | Delete — never constructed | Phase 65-03 exposed | -1 clippy warning |
| `LookupResult::BuiltinVariant` | Delete — only from dead code | Phase 65-03 exposed | -1 clippy warning |
| `push_locals`, `add_local`, `resolve_value` | Delete — zero callers | Phase 65-03 exposed | -1 clippy warning |
| `say(_speaker_ref, text)` (1 arg) | `say(speaker_ref, text)` (2 args) | Phase 62-01 regressed | 29 snapshot + 1 emit test fixed |

---

## Open Questions

1. **Should the `mutability` module be removed entirely from `check/mod.rs`?**
   - What we know: After removing both functions, the file contains only a `//!` doc header. The module declaration `pub(crate) mod mutability;` in `check/mod.rs` references it.
   - What's unclear: Whether future work will add mutability checking back to this module.
   - Recommendation: Keep the module file and declaration but empty its function content. The doc header serves as a placeholder explaining the module's purpose. This avoids removing a module boundary that may be used again.

2. **Does `LookupResult::BuiltinVariant` have any remaining construction or match sites after removing `resolve_value`?**
   - What we know: `resolve_value` is the only function that constructs `BuiltinVariant(...)`. One match arm in `resolver.rs:708` consumes it.
   - What's unclear: Whether any other file pattern-matches on this variant.
   - Recommendation: Before removing the variant, `grep -rn "BuiltinVariant" src/` to confirm the only sites are in `scope.rs` (construction) and `resolver.rs` (consumption). Both must be removed.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test + insta 1.x |
| Config file | writ-compiler/Cargo.toml (dev-dep: insta) |
| Quick run command | `cargo clippy --workspace && cargo test -p writ-compiler --test lowering_tests && cargo test -p writ-compiler --test emit_tests` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WARN-02 | `cargo clippy --workspace` exits with zero warnings | lint | `cargo clippy --workspace` | N/A |
| WARN-02 | say() emits 2 args (speaker, text) | snapshot | `cargo test -p writ-compiler --test lowering_tests` | Snapshots exist in `writ-compiler/tests/snapshots/` |
| WARN-02 | All 112 lowering snapshot tests pass | snapshot | `cargo test -p writ-compiler --test lowering_tests` | Yes |
| WARN-02 | choice_option_emits_externdef emit test passes | integration | `cargo test -p writ-compiler --test emit_tests` | Yes (`writ-compiler/tests/emit_tests.rs:250`) |
| WARN-02 | No stale .snap.new files in working tree | artifact | `git status writ-compiler/tests/snapshots/` | Files exist, must be deleted |

### Sampling Rate
- **Per task commit:** `cargo clippy --workspace` (fast, ~5s)
- **Per wave merge:** `cargo test -p writ-compiler --test lowering_tests && cargo test -p writ-compiler --test emit_tests`
- **Phase gate:** `cargo clippy --workspace && cargo test --workspace` before closing

### Wave 0 Gaps
None — existing test infrastructure covers all phase requirements. The snapshot baselines already exist. No new test files, fixtures, or framework installs needed.

---

## Sources

### Primary (HIGH confidence)
- Direct `cargo clippy --workspace` run (2026-03-18) — 6 warnings, exact locations confirmed
- Direct file reads: `writ-compiler/src/check/infer.rs`, `check/mutability.rs`, `resolve/scope.rs`, `lower/dialogue.rs`
- `.planning/v6.0-MILESTONE-AUDIT.md` — authoritative audit documenting both regressions with commit hashes
- `writ-compiler/tests/snapshots/lowering_tests__dlg_say_without_key.snap` + `.snap.new` — confirmed 2-arg vs 1-arg format mismatch
- `cargo test -p writ-compiler --test emit_tests choice_option_emits_externdef` — confirmed failure with E0101 arity error

### Secondary (MEDIUM confidence)
- `writ-compiler/tests/emit_tests.rs:250-281` — emit test fixture confirming `say(speaker: Entity, text: string)` 2-param signature is the intended contract

### Tertiary (LOW confidence)
None.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — cargo/insta are the existing stack; confirmed by Cargo.toml
- Architecture: HIGH — regressions confirmed by running the tools against current code
- Pitfalls: HIGH — derived from direct code inspection and test output

**Research date:** 2026-03-18
**Valid until:** 2026-04-17 (stable Rust toolchain; insta snapshot format does not change)
