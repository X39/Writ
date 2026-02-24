---
phase: 03-operator-and-concurrency-lowering
plan: 01
subsystem: compiler
tags: [rust, lowering, operator-overloading, desugaring, impl-blocks, contract-impls]

# Dependency graph
requires:
  - phase: 02-foundational-expression-lowering
    provides: "lower_fn, lower_param, lower_vis, lower_type, lower_stmt, LoweringContext — all helpers operator.rs uses"
provides:
  - "lower_operator_impls function: extracts Op members from impl blocks and re-emits as standalone contract impls"
  - "Operator-to-contract mapping: all 10 OpSymbol variants mapped to contract names and method names"
  - "Sub disambiguation: param count distinguishes unary Neg from binary Sub"
  - "Derived operator generation: Ne from Eq, Gt from Ord, LtEq+GtEq from Eq+Ord"
affects:
  - 03-02-concurrency-lowering
  - future downstream phases that consume lowered AST

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Operator desugaring via dedicated module: operator.rs extracts Op members, never passes AstImplMember::Op downstream"
    - "Split impl lowering: base impl emitted only when fn_members non-empty or explicit contract present"
    - "Derived operator synthesis from impl_span: all synthetic nodes reuse the source span, never SimpleSpan::new(0,0)"

key-files:
  created:
    - writ-compiler/src/lower/operator.rs
  modified:
    - writ-compiler/src/lower/mod.rs

key-decisions:
  - "lower_operator_impls replaces lower_impl entirely — no call site left using lower_impl"
  - "lower_impl and lower_op_decl removed (not just hidden) to keep dead code warnings clean"
  - "Sub disambiguation uses match guard (OpSymbol::Sub if op_decl.params.is_empty()) not a separate pre-check"
  - "Empty base impl suppressed: only emitted when fn_members non-empty OR contract is Some"
  - "Derived ops use eq_param_type for LtEq/GtEq when both Eq and Ord are present"
  - "lower_fn, lower_param, lower_vis promoted to pub(crate) so operator.rs can import via super::"

patterns-established:
  - "Operator module pattern: dedicated lower/operator.rs for each operator desugaring phase"
  - "Vec<AstDecl> return: lower_operator_impls returns a vec (1..N decls) rather than a single AstDecl"

requirements-completed: [R6]

# Metrics
duration: 4min
completed: 2026-02-26
---

# Phase 03 Plan 01: Operator-to-Contract Desugaring Summary

**Operator impl blocks now lower to standalone contract impls (Add, Sub/Neg, Mul, Div, Mod, Eq, Ord, Not, Index, IndexMut) with derived Ne, Gt, LtEq, GtEq synthesized automatically — AstImplMember::Op no longer appears in lowered output.**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-26T19:01:32Z
- **Completed:** 2026-02-26T19:05:53Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Created `writ-compiler/src/lower/operator.rs` with `lower_operator_impls`, `op_to_contract_impl`, `op_symbol_to_contract`, and `generate_derived_operators`
- All 10 `OpSymbol` variants handled exhaustively (no `_ =>` wildcard) — Add, Sub/Neg, Mul, Div, Mod, Eq, Ord, Not, Index, IndexMut
- Derived operators automatically generated: `Ne` from `Eq`, `Gt` from `Ord`, `LtEq`+`GtEq` from `Eq+Ord`
- Wired both `Item::Impl` dispatch sites in `lower/mod.rs` to `lower_operator_impls` using `decls.extend(...)`
- All 14 Phase 2 snapshot tests continue to pass without regression

## Task Commits

Each task was committed atomically:

1. **Task 1: Create lower/operator.rs with operator-to-contract mapping** - `19b02a6` (feat)
2. **Task 2: Wire lower_operator_impls into lower/mod.rs at both call sites** - `f6ea779` (feat)

## Files Created/Modified

- `writ-compiler/src/lower/operator.rs` - New module with operator-to-contract desugaring logic (398 lines)
- `writ-compiler/src/lower/mod.rs` - Wired new module: pub mod operator, pub(crate) helpers, updated dispatch, removed lower_impl/lower_op_decl

## Decisions Made

- `lower_impl` and `lower_op_decl` removed entirely (not left as dead code) to maintain zero-warning build
- `lower_fn`, `lower_param`, `lower_vis` promoted to `pub(crate)` rather than re-exported, as the `super::` import pattern is clean and idiomatic for sibling modules
- Empty base impl suppression: an impl block with ONLY operators and NO contract does not emit a spurious empty `AstDecl::Impl`
- Derived operators use `impl_span` for all synthetic AST nodes — never `SimpleSpan::new(0, 0)`, which would create invalid debug information

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. Build and tests passed on first attempt.

## Self-Check: PASSED

- FOUND: writ-compiler/src/lower/operator.rs
- FOUND: .planning/phases/03-operator-and-concurrency-lowering/03-01-SUMMARY.md
- FOUND: commit 19b02a6 (Task 1)
- FOUND: commit f6ea779 (Task 2)
- FOUND: commit ced96d3 (metadata)

## Next Phase Readiness

- Phase 03 Plan 01 complete — operator desugaring fully implemented and wired
- Ready for Phase 03 Plan 02: concurrency lowering (spawn/join/cancel/defer/detached pass-through)
- No blockers

---
*Phase: 03-operator-and-concurrency-lowering*
*Completed: 2026-02-26*
