---
phase: 50-compiler-frontend
plan: 02
subsystem: compiler
tags: [rust, type-checking, codegen, diagnostics, struct, class]

# Dependency graph
requires:
  - phase: 50-compiler-frontend-01
    provides: class AST, lowering, name resolution, type checking; TypeDefKind::Class in writ-module
provides:
  - collect_class emitting TypeDefKind::Class (kind=4) in binary modules
  - detect_recursive_structs DFS pass rejecting infinite-size value-type struct cycles
  - E0121 error code with chain description and "use class" suggestion
affects: [50-03, compiler-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Post-type-check DFS pass for structural cycle detection (globally_visited + in_path sets)
    - Path-based cycle chain builder producing human-readable field-level diagnostics

key-files:
  created: []
  modified:
    - writ-diagnostics/src/code.rs
    - writ-compiler/src/check/error.rs
    - writ-compiler/src/check/mod.rs

key-decisions:
  - "Task 1 (class codegen wire-up) was already complete from Plan 01 — collect_class, collect_extern_class, find_class_decl, TypeDefKind::Class dispatch, and export dispatch were all committed in feat(50-01)"
  - "detect_recursive_structs uses two sets: globally_visited (avoid re-walking subtrees) and in_path (per-DFS cycle detection); class fields are explicitly excluded from recursion"
  - "Path entries store (from_def_id, field_name) where field_name is the field on from_def_id that leads to the next struct, enabling clear chain messages"

patterns-established:
  - "DFS cycle detection: push (id, field_name) before recursing, pop after; check in_path for cycle, globally_visited to skip already-explored subtrees"
  - "Post-type-check passes run after all decl checks but before interner extraction, using the same def_map/type_env/interner"

requirements-completed: [COMP-02, COMP-03, COMP-04]

# Metrics
duration: 12min
completed: 2026-03-13
---

# Phase 50 Plan 02: Class Codegen and Recursive Struct Detection Summary

**TypeDefKind::Class codegen (COMP-03) confirmed from Plan 01 plus DFS recursive-struct error pass (COMP-04) emitting E0121 with field chain and "use class" suggestion**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-03-13T00:00:00Z
- **Completed:** 2026-03-13T00:12:00Z
- **Tasks:** 2 (Task 1 confirmed from Plan 01, Task 2 implemented)
- **Files modified:** 3

## Accomplishments
- Confirmed Task 1 (class codegen) was already complete in Plan 01 commits: collect_class emits TypeDefKind::Class (kind=4), export dispatch includes DefKind::Class and DefKind::ExternClass, TyKind::Class in extract_type_def_id
- Implemented detect_recursive_structs DFS pass in check/mod.rs running post-type-check, checking only DefKind::Struct (excluding classes, entities, enums)
- Added E0121 error code and TypeError::RecursiveStruct variant with struct name, cycle chain, span, file, and "use class" suggestion
- All 94+ existing tests pass without regression

## Task Commits

1. **Task 1: Wire class through codegen** - Already in `d094550` from Plan 01 (feat(50-01))
2. **Task 2: Recursive struct detection** - `f41a79c` (feat(50-02))

**Plan metadata:** (pending docs commit)

## Files Created/Modified
- `writ-diagnostics/src/code.rs` - Added E0121 constant for recursive struct error
- `writ-compiler/src/check/error.rs` - Added RecursiveStruct variant to TypeError, added From<TypeError> conversion producing E0121 diagnostics
- `writ-compiler/src/check/mod.rs` - Added detect_recursive_structs(), dfs_struct(), emit_recursive_struct_error(); call inserted as step 4b in typecheck()

## Decisions Made
- Task 1 was already complete from Plan 01. No re-implementation needed.
- DFS path stores `(from_def_id, field_name_on_from_struct)` tuples — this makes chain building straightforward: `path[i] = (A, "x")` means struct A has field x pointing to the next node.
- globally_visited prevents re-walking already-explored subtrees (O(n) total work instead of O(n^2)).
- Classes excluded from cycle detection because they store a heap pointer (not an inline value), so a class field cannot contribute to infinite size.

## Deviations from Plan

None — plan executed as written. Task 1 was discovered already complete, which was confirmed by reading Plan 01's commits before beginning any coding.

## Issues Encountered

None — build was clean throughout, all tests passed immediately after implementation.

## Next Phase Readiness
- COMP-02 (struct emits kind=0), COMP-03 (class emits kind=4), COMP-04 (recursive struct error) are all satisfied
- Ready for Phase 50 Plan 03 (compiler integration tests for struct/class/recursive-struct)

---
*Phase: 50-compiler-frontend*
*Completed: 2026-03-13*
