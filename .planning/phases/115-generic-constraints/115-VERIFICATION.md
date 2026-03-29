---
phase: 115-generic-constraints
verified: 2026-03-29T00:00:00Z
status: passed
score: 7/7 must-haves verified
gaps: []
human_verification:
  - test: "E0103 error message reads well in rendered output"
    expected: "Primary label at call site, secondary label pointing to bound declaration with message 'bound `EqBound` declared here', help text 'consider adding `impl EqBound for Foo { ... }`'"
    why_human: "Diagnostic formatting and readability is a UX judgment that requires visual inspection of rendered output"
---

# Phase 115: Generic Constraints Verification Report

**Phase Goal:** Users can declare and have compiler-enforced contract bounds on generic type parameters
**Verified:** 2026-03-29
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Calling a bound-constrained generic fn with a type that implements the required contract produces no error | VERIFIED | `generic_single_bound_satisfied` passes — struct with `impl EqBound` passes bound check with no diagnostics |
| 2 | Calling a bound-constrained generic fn with a type that does NOT implement the contract produces E0103 | VERIFIED | `generic_bound_not_satisfied_emits_e0103` passes — struct without impl yields E0103 |
| 3 | Multi-bound syntax `<T: Eq + Ord>` enforces both contracts independently | VERIFIED | `generic_multi_bound_both_satisfied` (no error) and `generic_multi_bound_missing_one_emits_e0103` (E0103 when one is missing) both pass |
| 4 | The E0103 error includes a secondary label pointing to the bound declaration | VERIFIED | `generic_bound_error_has_secondary_label` passes — `e0103.secondary_labels` is non-empty |
| 5 | The E0103 error includes a help suggestion: consider adding impl ContractName for TypeName | VERIFIED | `generic_bound_error_has_help_suggestion` passes — `e0103.help.contains("consider adding \`impl")` is true |
| 6 | A compiled binary from source with `<T: Eq>` has a non-empty GenericConstraint table (table 14) | VERIFIED | `emit_generic_constraint_table` passes — 1 constraint row, `param_row == 1`, `constraint != MetadataToken::NULL` |
| 7 | The GenericConstraint rows have correct 1-based param_row and resolved constraint MetadataToken | VERIFIED | `emit_generic_multi_constraint` passes — 2 rows for `<T: Equivalent + Comparable>`; `param_row` remapped from 0-based in finalize step 9 via `row.param_row + 1`; `constraint` resolved via `def_token_map` |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/check/env.rs` | FnSig with `bound_decl_spans` and `fn_file` fields | VERIFIED | Lines 39–41: `pub bound_decl_spans: Vec<SimpleSpan>` and `pub fn_file: FileId` present in `FnSig` struct |
| `writ-compiler/src/check/error.rs` | `UnsatisfiedBound` with `bound_decl_span` and `bound_decl_file` | VERIFIED | Lines 35–36: both fields present; `From<TypeError>` arm at line 210 calls `.with_secondary(bound_decl_file, bound_decl_span, ...)` |
| `writ-compiler/src/check/check_expr/call.rs` | `check_contract_bounds` threading `bound_decl_span` into `UnsatisfiedBound` | VERIFIED | Lines 472–482: `sig.bound_decl_spans[i]` used with fallback to `call_span`; `sig.fn_file` passed as `bound_decl_file` |
| `writ-compiler/tests/typecheck_tests.rs` | 6 generic bound tests (GEN-01 through GEN-06 minus GEN-04) | VERIFIED | All 6 functions present at lines 1406–1480; all 6 pass |
| `writ-compiler/src/emit/module_builder.rs` | `generic_constraint_contract_ids` side-table; finalize step 9 resolves to MetadataToken | VERIFIED | Line 106: `generic_constraint_contract_ids: Vec<DefId>`; lines 681–692: finalize step 9 remaps `param_row` to 1-based and resolves via `def_token_map` |
| `writ-compiler/src/emit/collect/functions.rs` | `collect_fn` calls `add_generic_constraint` for each bound on each generic param | VERIFIED | Lines 62–65: iterates `ast_gp.bounds`, matches `AstType::Named { name, .. }`, calls `builder.add_generic_constraint(param_idx, contract_def_id)` |
| `writ-compiler/tests/emit_tests.rs` | Tests verifying GenericConstraint table rows | VERIFIED | Lines 629–667: `emit_generic_constraint_table` (1 row) and `emit_generic_multi_constraint` (2 rows) both pass |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-compiler/src/check/env_build.rs` | `writ-compiler/src/check/env.rs` | `build_fn_sig` populates `FnSig.bound_decl_spans` and `FnSig.fn_file` | WIRED | Lines 440, 449–450: `bound_decl_spans` from `fn_decl.generics.iter().map(|gp| gp.span)`, `fn_file: entry.file_id`; also populated at lines 482/491–492, 610–611, 673–674 for all FnSig constructors |
| `writ-compiler/src/check/check_expr/call.rs` | `writ-compiler/src/check/error.rs` | `check_contract_bounds` emits `UnsatisfiedBound` with `bound_decl_span` | WIRED | Line 472–482: `bound_decl_span = sig.bound_decl_spans[i]` threaded into `TypeError::UnsatisfiedBound { ..., bound_decl_span, bound_decl_file: sig.fn_file }` |
| `writ-compiler/src/emit/collect/functions.rs` | `writ-compiler/src/emit/module_builder.rs` | `collect_fn` calls `builder.add_generic_constraint(param_idx, contract_def_id)` | WIRED | Line 65: `builder.add_generic_constraint(param_idx, contract_def_id)` inside `ast_gp.bounds` iteration |
| `writ-compiler/src/emit/module_builder.rs` finalize step 9 | `def_token_map` | Resolves constraint `DefId` to `MetadataToken` | WIRED | Lines 685–690: `self.def_token_map.get(&contract_def_id).copied().unwrap_or(MetadataToken::NULL)` |

### Data-Flow Trace (Level 4)

Not applicable — this phase produces compiler diagnostics and IL metadata, not UI components or dynamic rendering. Data flows are verified through the behavioral spot-checks below.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 6 typecheck tests pass (GEN-01, GEN-02, GEN-03, GEN-05, GEN-06) | `cargo test -p writ-compiler -- generic_bound generic_single_bound_satisfied generic_multi_bound` | 6 passed, 0 failed | PASS |
| 2 emit tests pass (GEN-04) | `cargo test -p writ-compiler emit_generic` | 2 passed, 0 failed | PASS |
| Full writ-compiler suite — no regressions | `cargo test -p writ-compiler` | 101 passed (typecheck) + all other suites green; 0 failed total | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| GEN-01 | 115-01-PLAN.md | User can declare single contract bounds on generic type params (`<T: Eq>`) | SATISFIED | `generic_single_bound_satisfied` passes — `<T: EqBound>` is parsed, resolved, and enforced |
| GEN-02 | 115-01-PLAN.md | User can declare multiple contract bounds on a single type param (`<T: Eq + Ord>`) | SATISFIED | `generic_multi_bound_both_satisfied` and `generic_multi_bound_missing_one_emits_e0103` pass — `<T: EqBound + OrdBound>` enforces both bounds |
| GEN-03 | 115-01-PLAN.md | Compiler enforces bounds at call sites — error when passing a type that doesn't implement the required contract | SATISFIED | `generic_bound_not_satisfied_emits_e0103` passes — call with non-implementing type yields E0103 |
| GEN-04 | 115-02-PLAN.md | Compiler emits generic constraints to IL GenericConstraint table rows | SATISFIED | `emit_generic_constraint_table` (1 row, correct `param_row=1`, non-NULL token) and `emit_generic_multi_constraint` (2 rows) pass |
| GEN-05 | 115-01-PLAN.md | Constraint violation errors show multi-span diagnostics (call site + constraint declaration) | SATISFIED | `generic_bound_error_has_secondary_label` passes — `secondary_labels` is non-empty, pointing to bound declaration via `bound_decl_span`/`bound_decl_file` |
| GEN-06 | 115-01-PLAN.md | Constraint violation errors include fix suggestion ("add `impl Eq for Foo`") | SATISFIED | `generic_bound_error_has_help_suggestion` passes — `e0103.help.contains("consider adding \`impl")` is true |

**Orphaned requirements check:** REQUIREMENTS.md tracking table still shows GEN-01, GEN-02, GEN-03, GEN-05, GEN-06 as `Pending` (only GEN-04 is marked `Complete`). The checkbox list at the top of REQUIREMENTS.md has all five as `[ ]`. This is a **tracking document discrepancy only** — the code satisfies all six requirements as confirmed by passing tests. The REQUIREMENTS.md status column was not updated after phase 115 completion. This does not block the phase goal.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `writ-compiler/src/check/env.rs` | 367, 404 | `bound_decl_spans: vec![]` in two FnSig constructors | Info | These are contract method and impl entry constructors that have no generic params — empty vec is correct behavior, not a stub |
| `writ-compiler/src/check/check_expr/call.rs` | 472–475 | Fallback `call_span` when `i >= sig.bound_decl_spans.len()` | Info | Defensive fallback for edge cases; does not suppress the error, only degrades secondary span precision |

No blockers or warnings found. The `vec![]` and fallback patterns are intentional defensive code, not stubs.

### Human Verification Required

#### 1. Rendered Diagnostic Output

**Test:** Compile a Writ source file containing `pub fn check<T: EqBound>(a: T) -> bool { true }` and call it with a non-implementing type. View the rendered diagnostic in a terminal.
**Expected:** Primary error at call site ("unsatisfied bound here"), secondary label at the `<T: EqBound>` declaration site ("bound `EqBound` declared here"), help text "consider adding `impl EqBound for Foo { ... }`". All three spans should point to accurate source locations with no off-by-one errors.
**Why human:** Diagnostic rendering (color, span accuracy in multi-line source, label alignment) requires visual inspection. The code paths are verified but rendered quality is a UX judgment.

### Gaps Summary

No gaps. All must-haves verified at all levels (existence, substance, wiring, behavioral). The REQUIREMENTS.md tracking document has a stale status for GEN-01..03, GEN-05..06 (shows Pending when implementation is complete) — this is a cosmetic documentation issue, not a code gap.

---

_Verified: 2026-03-29_
_Verifier: Claude (gsd-verifier)_
