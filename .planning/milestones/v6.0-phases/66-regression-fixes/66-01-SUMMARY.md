---
phase: 66-regression-fixes
plan: 01
subsystem: compiler
tags: [rust, clippy, dead-code, dialogue-lowering, snapshot-tests]

# Dependency graph
requires:
  - phase: 65-code-duplication-and-module-boundaries
    provides: pub narrowing that exposed 6 genuinely dead items
  - phase: 62-clippy-warning-elimination
    provides: cargo clippy --fix that silently regressed say() to 1-arg
provides:
  - Zero dead_code warnings in writ-compiler
  - Correct 2-argument say(speaker, text) emission in dialogue lowering
  - All 112 lowering snapshot tests passing against original baselines
  - No stale .snap.new files in working tree
affects:
  - writ-runtime (depends on say/say_localized 2-arg ABI)
  - future dialogue-related plans

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "After removing all variants but one from an enum, if-let patterns become irrefutable — convert to let destructure"

key-files:
  created: []
  modified:
    - writ-compiler/src/check/infer.rs
    - writ-compiler/src/check/mutability.rs
    - writ-compiler/src/resolve/scope.rs
    - writ-compiler/src/resolve/resolver.rs
    - writ-compiler/src/lower/dialogue.rs

key-decisions:
  - "Removed FxHashMap import from infer.rs — only resolve_type_to_ty used it, instantiate_generic_fn and substitute do not"
  - "Converted irrefutable if-let ScopeLayer::GenericParams patterns to plain let destructure after Locals variant removed"
  - "Deleted 29 stale .snap.new files (untracked) — they recorded regressed 1-arg format; original .snap baselines are correct"

patterns-established:
  - "When removing enum variants, audit all match arms and if-let patterns across the crate for irrefutability"

requirements-completed: [WARN-02]

# Metrics
duration: 15min
completed: 2026-03-18
---

# Phase 66 Plan 01: Regression Fixes Summary

**Dead-code purge (6 items across 4 files) and say() ABI restoration to 2-argument (speaker, text) emission — cargo clippy exits clean, all 112 lowering tests pass**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-18T12:00:00Z
- **Completed:** 2026-03-18T12:15:00Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Removed 6 dead code items: `resolve_type_to_ty` (infer.rs), `check_method_mutation` + `find_root_binding` (mutability.rs), `ScopeLayer::Locals` + `LookupResult::BuiltinVariant` + `push_locals` + `add_local` + `resolve_value` (scope.rs), `BuiltinVariant` match arm (resolver.rs)
- Restored `say(speaker, text)` 2-argument emission — Phase 62 `cargo clippy --fix` had renamed `speaker_ref` to `_speaker_ref` and silently dropped the speaker from the args vec
- Restored `say_localized(speaker, key, fallback)` 3-argument emission by the same fix
- Deleted all 29 stale `.snap.new` files that recorded the regressed 1-argument format
- `cargo clippy --workspace` exits with zero warnings and zero errors
- All 112 lowering snapshot tests pass against original baselines
- All 30 emit tests pass including `choice_option_emits_externdef`

## Task Commits

Each task was committed atomically:

1. **Task 1: Delete 6 dead code items across 4 files** - `72627fe` (fix)
2. **Task 2: Restore say() 2-argument emission and delete stale snapshots** - `83d56df` (fix)

**Plan metadata:** (final commit — see below)

## Files Created/Modified

- `writ-compiler/src/check/infer.rs` — Removed `resolve_type_to_ty` function (lines 13-77) and its exclusive imports (`FxHashMap`, `DefKind`, `DefMap`, `PrimitiveTag`, `ResolvedType`)
- `writ-compiler/src/check/mutability.rs` — Reduced to doc header only (removed `check_method_mutation`, `find_root_binding`, and all use imports)
- `writ-compiler/src/resolve/scope.rs` — Removed `ScopeLayer::Locals` variant, `LookupResult::BuiltinVariant` variant, `push_locals`, `add_local`, `resolve_value` methods; fixed irrefutable if-let patterns
- `writ-compiler/src/resolve/resolver.rs` — Removed `LookupResult::BuiltinVariant(_)` match arm in `resolve_type`
- `writ-compiler/src/lower/dialogue.rs` — Restored `speaker_ref` parameter (removed `_` prefix) in `make_say` and `make_say_localized`; added speaker as first arg in both args vecs; updated doc comments

## Decisions Made

- Removed `FxHashMap` import from `infer.rs` after deleting `resolve_type_to_ty` (only that function used it; `instantiate_generic_fn` and `substitute` do not).
- After removing `ScopeLayer::Locals`, two `if let ScopeLayer::GenericParams(params) = layer` patterns became irrefutable warnings — converted to `let ScopeLayer::GenericParams(params) = layer;` plain destructuring.
- Deleted 29 `.snap.new` files (untracked, never committed) — they recorded the regressed 1-arg say() format; the original `.snap` baselines already have the correct 2-arg format.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed FxHashMap import after resolve_type_to_ty deletion**
- **Found during:** Task 1 (Delete 6 dead code items) — cargo clippy flagged unused import
- **Issue:** `use rustc_hash::FxHashMap` was exclusive to `resolve_type_to_ty` which was deleted; leaving it would generate an `unused_imports` warning
- **Fix:** Removed the `use rustc_hash::FxHashMap;` line from infer.rs
- **Files modified:** `writ-compiler/src/check/infer.rs`
- **Verification:** `cargo clippy --workspace` exits clean
- **Committed in:** `72627fe` (Task 1 commit)

**2. [Rule 1 - Bug] Fixed irrefutable if-let patterns in scope.rs**
- **Found during:** Task 1 (Delete 6 dead code items) — cargo clippy flagged `irrefutable_let_patterns` warning
- **Issue:** After removing `ScopeLayer::Locals`, `ScopeLayer` has only one variant (`GenericParams`). Two `if let ScopeLayer::GenericParams(params) = layer` patterns became irrefutable (always match), triggering clippy warnings.
- **Fix:** Converted both to `let ScopeLayer::GenericParams(params) = layer;` plain destructuring
- **Files modified:** `writ-compiler/src/resolve/scope.rs`
- **Verification:** `cargo clippy --workspace` exits with zero warnings
- **Committed in:** `72627fe` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — bugs surfaced by clippy after planned deletions)
**Impact on plan:** Both fixes were direct consequences of the planned deletions. No scope creep.

## Issues Encountered

- Pre-existing `writ-lsp` test compilation failures (9 errors in `test_hover_protocol` — tokio feature flags). These are unrelated to this plan's changes and were present before execution.

## Next Phase Readiness

- `cargo clippy --workspace` is clean — WARN-02 requirement satisfied
- Dialogue lowering ABI restored to spec: `say(speaker, text)` and `say_localized(speaker, key, fallback)`
- All snapshot tests passing — no stale `.snap.new` files remain
- Phase 66 is complete

---
*Phase: 66-regression-fixes*
*Completed: 2026-03-18*
