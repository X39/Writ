---
phase: 115-generic-constraints
plan: 02
subsystem: compiler
tags: [il-emission, generic-constraints, metadata, module-builder]

# Dependency graph
requires:
  - phase: 115-generic-constraints plan 01
    provides: "Generic bound enforcement in type checker (E0103) — ensures bounds are semantically valid before emission"
provides:
  - "GenericConstraint table (table 14) rows emitted with correct 1-based param_row and resolved contract MetadataToken"
  - "generic_constraint_contract_ids side-table in ModuleBuilder for deferred DefId resolution"
  - "collect_fn emits GenericConstraint rows for every bound on every generic param"
affects:
  - runtime-reflection
  - il-binary-format

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Side-table pattern for deferred resolution: store DefIds during Pass 1 (collect), resolve to MetadataTokens during Pass 2 (finalize)"
    - "AstGenericParam.bounds iteration via zip with entry.generics for constraint emission"

key-files:
  created: []
  modified:
    - writ-compiler/src/emit/module_builder.rs
    - writ-compiler/src/emit/collect/functions.rs
    - writ-compiler/tests/emit_tests.rs

key-decisions:
  - "Side-table generic_constraint_contract_ids stores DefIds parallel to generic_constraints Vec, resolved to MetadataTokens in finalize step 9 via def_token_map — same pattern as type_def_def_ids, contract_def_def_ids, etc."
  - "param_row stored as 0-based provisional index during collection, remapped to 1-based during finalize step 9 (spec section 2.16.5)"
  - "Bound resolution uses def_map.get(name) on simple name (not FQN) — consistent with build_generic_bounds in env_build.rs"
  - "AstType::Named { name, .. } is the only matchable bound type — Generic/Array/Func/Void bounds are silently skipped (no valid bound forms in current spec)"

patterns-established:
  - "TDD RED-GREEN: write failing tests first, then implement fixes"
  - "Contract syntax in Writ tests: fn method_name(self) -> type; (self as keyword param, not type)"
  - "Struct construction in Writ tests: new StructName { field: value } (not StructName(field: value))"

requirements-completed: [GEN-04]

# Metrics
duration: 15min
completed: 2026-03-29
---

# Phase 115 Plan 02: Generic Constraints — IL Emission Summary

**GenericConstraint table (table 14) now emitted with 1-based param_row and resolved contract MetadataToken for every bound-constrained generic parameter**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-29T00:00:00Z
- **Completed:** 2026-03-29T00:15:00Z
- **Tasks:** 1 (TDD: RED + GREEN)
- **Files modified:** 3

## Accomplishments
- Added `generic_constraint_contract_ids: Vec<DefId>` side-table to `ModuleBuilder` for deferred contract token resolution
- Fixed `add_generic_constraint` to store the constraint's `DefId` (was silently discarding it with `let _ = constraint_def_id`)
- Fixed finalize step 9 to remap `param_row` from 0-based to 1-based and resolve contract `DefId` to `MetadataToken` via `def_token_map`
- Updated `collect_fn` in `functions.rs` to zip `entry.generics` (names) with `fn_decl.generics` (AST with bounds) and call `add_generic_constraint` for each `AstType::Named` bound
- Added two passing emit tests: `emit_generic_constraint_table` (single bound) and `emit_generic_multi_constraint` (two bounds)

## Task Commits

1. **Task 1 (RED): Failing emit tests** - `dc0a963` (test)
2. **Task 1 (GREEN): Fix ModuleBuilder + collect_fn** - `bb0bf7d` (feat)

## Files Created/Modified
- `writ-compiler/src/emit/module_builder.rs` - Added `generic_constraint_contract_ids` field, fixed `add_generic_constraint`, fixed finalize step 9
- `writ-compiler/src/emit/collect/functions.rs` - Import `AstType`, replace GenericParam-only loop with combined GenericParam+GenericConstraint loop
- `writ-compiler/tests/emit_tests.rs` - Added `emit_generic_constraint_table` and `emit_generic_multi_constraint` tests

## Decisions Made
- Used `def_map.get(name)` (simple name lookup) for bound contract resolution — same pattern as `build_generic_bounds` in `env_build.rs`
- `AstType::Named` is the only valid bound form; `Generic`, `Array`, `Func`, `Void` silently skipped (no valid multi-arg or non-named bounds in spec)
- `param_row` stored provisionally as 0-based GenericParam index during collection, remapped to 1-based in finalize (spec-correct)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Test source syntax corrections**
- **Found during:** Task 1 (RED — writing tests)
- **Issue:** Plan's test source used `fn eq(other: self)` (self as type name — not valid Writ), `Bar(x: 1)` struct construction (not valid — `E0102 undefined variable Bar`), and `Eq`/`Ord` contract names (E0002 — reserved prelude names)
- **Fix:** Changed contract method to `fn is_eq(self) -> bool`, struct construction to `new Bar { x: 1 }`, contract names to `Equivalent`/`Comparable`
- **Files modified:** `writ-compiler/tests/emit_tests.rs`
- **Verification:** Tests parse and compile cleanly; failures are now for the correct semantic reason (0 constraint rows)
- **Committed in:** dc0a963 (RED commit)

---

**Total deviations:** 1 auto-fixed (test syntax corrections)
**Impact on plan:** Necessary corrections to plan's test source examples. No scope creep. Implementation unchanged from plan.

## Issues Encountered
- Test source syntax in the plan used invalid Writ constructs (`self` as type name, struct-call syntax, prelude-reserved names). Auto-fixed by matching existing test patterns in the codebase.

## Next Phase Readiness
- GenericConstraint table emission complete — GEN-04 satisfied
- Phase 115 plan 01 (typecheck generic bound enforcement, E0103) may be in parallel execution
- IL binary serialization already writes the GenericConstraint table from `finalized_generic_constraints()` — no further plumbing needed

## Self-Check: PASSED

- FOUND: writ-compiler/src/emit/module_builder.rs
- FOUND: writ-compiler/src/emit/collect/functions.rs
- FOUND: writ-compiler/tests/emit_tests.rs
- FOUND: .planning/phases/115-generic-constraints/115-02-SUMMARY.md
- FOUND: commit dc0a963 (test: RED)
- FOUND: commit bb0bf7d (feat: GREEN)

---
*Phase: 115-generic-constraints*
*Completed: 2026-03-29*
