---
phase: 83-spec-review
plan: 01
subsystem: spec
tags: [contracts, contract-as-type, virtual-dispatch, assignability, CALL_VIRT]

requires: []

provides:
  - "§1.11.4 Contract-as-Type subsection in language-spec/spec/12_11_contracts.md"
  - "Authoritative spec text for contract type annotation syntax (SPEC-01)"
  - "Authoritative spec text for assignability rules T implements C (SPEC-02)"
  - "Authoritative spec text for CALL_VIRT virtual dispatch (SPEC-03)"

affects:
  - "84-type-system"
  - "85-codegen"
  - "86-lsp"

tech-stack:
  added: []
  patterns:
    - "Spec-first: define semantics in spec before implementing in compiler"

key-files:
  created: []
  modified:
    - "language-spec/spec/12_11_contracts.md"

key-decisions:
  - "§1.11.4 placed after §1.11.3 and before the trailing separator, keeping all existing content intact"
  - "Three sub-topics map 1:1 to requirements: type annotation syntax (SPEC-01), assignability rules (SPEC-02), virtual dispatch (SPEC-03)"
  - "CALL_VIRT reference in spec text ties spec semantics directly to IL instruction name"

patterns-established:
  - "Each spec requirement gets its own named subsection with prose + code example"

requirements-completed: [SPEC-01, SPEC-02, SPEC-03]

duration: 1min
completed: 2026-03-23
---

# Phase 83 Plan 01: Spec Review Summary

**§1.11.4 Contract-as-Type added to contracts spec with type annotation syntax, T-implements-C assignability rule, and CALL_VIRT virtual dispatch semantics**

## Performance

- **Duration:** ~1 min
- **Started:** 2026-03-23T22:50:52Z
- **Completed:** 2026-03-23T22:51:52Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Added §1.11.4 Contract-as-Type subsection after §1.11.3 in the contracts spec
- Documented contract names as valid type annotations in variable declarations, parameters, and return types (SPEC-01)
- Defined assignability rule: a value of concrete type T is assignable to contract type C if and only if T implements C (SPEC-02)
- Specified CALL_VIRT virtual dispatch for method calls on contract-typed values with note about no direct field access (SPEC-03)

## Task Commits

1. **Task 1: Add §1.11.4 Contract-as-Type subsection to spec** - `60bf315` (docs)

**Plan metadata:** (to be committed with this SUMMARY)

## Files Created/Modified

- `language-spec/spec/12_11_contracts.md` - Added §1.11.4 Contract-as-Type with three sub-topics (64 lines inserted)

## Decisions Made

- Three sub-topics structured as: "Type Annotation Syntax", "Assignability Rules", "Virtual Dispatch" — each with prose + code example
- Prose explicitly names `CALL_VIRT` to give Phase 85 (codegen) an unambiguous IL-level anchor
- Note block after virtual dispatch section documents that field access is not available through contract types

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 84 (type system): spec is authoritative reference for `TyKind::Contract(DefId)`, assignability checking, and method resolution on contract-typed receivers
- Phase 85 (codegen): `CALL_VIRT` named explicitly in §1.11.4 as the IL instruction for contract method calls
- Phase 86 (LSP): spec defines what completions and hover should surface for contract-typed variables

---
*Phase: 83-spec-review*
*Completed: 2026-03-23*
