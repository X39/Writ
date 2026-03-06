---
phase: 66-regression-fixes
verified: 2026-03-18T15:00:00Z
status: passed
score: 5/5 must-haves verified
re_verification: false
---

# Phase 66: Regression Fixes Verification Report

**Phase Goal:** Fix cross-phase regressions — eliminate dead_code warnings from pub(crate) narrowing and restore say() speaker argument accidentally removed by clippy auto-fix
**Verified:** 2026-03-18T15:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | `cargo clippy --workspace` exits with zero warnings and zero errors | VERIFIED | `cargo clippy --workspace` produced zero output — no warnings, no errors; `Finished dev profile` only |
| 2 | `say()` emits 2 arguments (speaker, text) in dialogue lowering | VERIFIED | `make_say` at dialogue.rs:558 takes `speaker_ref: AstExpr` (no `_` prefix) and places it as args[0] at line 569; `make_say_localized` does the same at line 589. Grep for `_speaker_ref` returns 0 matches. |
| 3 | All 112 lowering snapshot tests pass against the original .snap baselines | VERIFIED | `cargo test -p writ-compiler --test lowering_tests` → `test result: ok. 112 passed; 0 failed` |
| 4 | The emit test `choice_option_emits_externdef` passes | VERIFIED | `cargo test -p writ-compiler --test emit_tests` → `test result: ok. 30 passed; 0 failed`, including `choice_option_emits_externdef ... ok` |
| 5 | No stale .snap.new files remain in the working tree | VERIFIED | Glob `writ-compiler/tests/snapshots/*.snap.new` returns no files; git status confirms none untracked |

**Score:** 5/5 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/check/infer.rs` | Type inference helpers without dead `resolve_type_to_ty` | VERIFIED | File is 85 lines. Contains `pub fn instantiate_generic_fn` (line 9) and `pub fn substitute` (line 47). No `resolve_type_to_ty`, no `FxHashMap` import, no `DefKind`/`DefMap`/`PrimitiveTag`/`ResolvedType` imports. |
| `writ-compiler/src/check/mutability.rs` | Mutability module with doc header only (no dead functions) | VERIFIED | File is exactly 11 lines — the `//! Mutability enforcement` doc block only. No `use` lines, no `fn` lines, no `check_method_mutation`, no `find_root_binding`. |
| `writ-compiler/src/resolve/scope.rs` | Scope chain without `Locals` variant and dead methods | VERIFIED | `ScopeLayer` enum has only `GenericParams` variant (line 33-36). No `Locals` variant, no `push_locals`, no `add_local`, no `resolve_value` anywhere in file. |
| `writ-compiler/src/resolve/resolver.rs` | Resolver without `BuiltinVariant` match arm in `resolve_type` | VERIFIED | Grep for `BuiltinVariant` across all of `writ-compiler/src` returns 0 matches. |
| `writ-compiler/src/lower/dialogue.rs` | Dialogue lowering with 2-argument say() and say_localized() calls | VERIFIED | `make_say` (line 558) has `speaker_ref: AstExpr` parameter and `value: speaker_ref` at args[0] (line 569). `make_say_localized` (line 577) has `speaker_ref: AstExpr` parameter and `value: speaker_ref` at args[0] (line 589). `value: speaker_ref` appears exactly 2 times. |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `lower/dialogue.rs make_say` | `say()` call args vec | `speaker_ref` parameter (not `_speaker_ref`) | VERIFIED | Grep for `_speaker_ref` returns 0 matches. `value: speaker_ref` appears at lines 569 and 589. |
| `resolve/scope.rs` | `ScopeLayer` enum | Only `GenericParams` variant remains | VERIFIED | `ScopeLayer` enum contains only `GenericParams(Vec<(String, SimpleSpan)>)`. No `Locals` variant. Irrefutable `if let` patterns were converted to plain `let` destructures (commit 72627fe). |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| WARN-02 | 66-01-PLAN.md | `cargo clippy` exits clean with zero warnings | SATISFIED | `cargo clippy --workspace` produces zero warnings and zero errors. No `#[allow(dead_code)]` suppressions were introduced in any of the 5 modified files. Two pre-existing suppressions in `check/pattern.rs` and `emit/body/mod.rs` are unrelated to this phase. |

**Orphaned requirements:** None. REQUIREMENTS.md maps only `WARN-02` to Phase 66. The plan declares only `[WARN-02]`. Full coverage, no gaps.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `check/pattern.rs` | 149 | `#[allow(dead_code)]` on `pattern_is_exhaustive` | Info | Pre-existing; not introduced by Phase 66; not in any file this phase touched |
| `emit/body/mod.rs` | 186 | `#[allow(dead_code)]` on `has_error_nodes` | Info | Pre-existing; not introduced by Phase 66; not in any file this phase touched |

No blockers. No warnings introduced by this phase. The plan's success criterion "No new `#[allow(dead_code)]` suppressions introduced" is satisfied.

---

### Human Verification Required

None. All phase behaviors have automated verification. Clippy output, test counts, and file contents are fully machine-verifiable.

---

### Commit Verification

Both task commits are present and correct:

| Commit | Description | Files Changed |
|--------|-------------|---------------|
| `72627fe` | Delete 6 dead code items | `check/infer.rs`, `check/mutability.rs`, `resolve/scope.rs`, `resolve/resolver.rs` — 204 deletions |
| `83d56df` | Restore say() 2-argument emission; delete 29 stale .snap.new files | `lower/dialogue.rs` — 12 deletions, 6 insertions |

---

### Gaps Summary

No gaps. All 5 must-have truths are verified. All 5 required artifacts pass existence, substance, and wiring checks. The single declared requirement (WARN-02) is satisfied with concrete evidence. No stale snapshot files, no dead code suppressions introduced, no underscore-prefixed speaker parameter.

---

_Verified: 2026-03-18T15:00:00Z_
_Verifier: Claude (gsd-verifier)_
