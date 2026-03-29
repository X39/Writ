---
phase: 120-array-semantics-correction
plan: 03
subsystem: testing
tags: [golden-tests, writ-golden, language-spec, il-spec, array-semantics]

# Dependency graph
requires:
  - phase: 120-01
    provides: removed array growth opcodes, added resize/copy_from compiler emission
  - phase: 120-02
    provides: new opcodes wired through runtime and assembler (ArrayResize, ArrayCopy, ArraySlice, NewArraySized, NewArrayFilled)
provides:
  - Golden test array_primitives exercises new API (len, indexed access/write, resize grow/shrink, slice, copy_from cross-array and overlap)
  - Golden test type_array_ops updated to resize(5) per spec
  - Collection golden tests (coll_list_basic, coll_map_basic, coll_set_basic, coll_hashmap_basic, coll_list_map, coll_list_filter, coll_list_reduce) marked #[ignore] with Phase 121 comments
  - Language spec 07_6_primitive_types.md describes arrays as allocation-explicit, not growable
  - IL spec 57_3_9_arrays.md documents 10 opcodes: NEW_ARRAY through NEW_ARRAY_FILLED
  - Opcode assignment table 67_4_2 and instruction count 65_4_0 reflect new layout
affects: [121-stdlib-rewrite, future-golden-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Collection tests #[ignore]-d with Phase comment pointing to re-enable milestone"
    - "BLESS=1 to regenerate writil snapshots after .writ fixture changes"

key-files:
  created: []
  modified:
    - writ-golden/tests/golden/array_primitives.writ
    - writ-golden/tests/golden/array_primitives.writil
    - writ-golden/tests/golden/type_array_ops.writ
    - writ-golden/tests/golden/type_array_ops.writil
    - writ-golden/tests/golden_tests.rs
    - language-spec/spec/07_6_primitive_types.md
    - language-spec/spec/57_3_9_arrays.md
    - language-spec/spec/67_4_2_opcode_assignment_table.md
    - language-spec/spec/65_4_0_instruction_count_by_category.md

key-decisions:
  - "array_primitives.writ extended beyond Plan 02 version to include indexed access, overlap copy, and all new-API elements per D-09"
  - "5 pre-existing golden test failures (generic_inherent_impl, lib_preload_stub, expr_string_escapes, fn_overload, string_utilities) are out of scope — deferred per Phase 120 boundaries"
  - "Instruction count total updated 92->93 (arrays 9->10)"

patterns-established:
  - "After rewriting .writ fixtures, BLESS=1 must be re-run from the repo root (not a worktree) to regenerate .writil snapshots"

requirements-completed: [ARR-01, ARR-02, ARR-03, ARR-04, ARR-05, ARR-06]

# Metrics
duration: 15min
completed: 2026-03-29
---

# Phase 120 Plan 03: Array Semantics Correction — Golden Tests and Spec Update Summary

**Array golden tests rewritten to exercise resize/copy_from/len/slice API; collection tests ignored for Phase 121; language and IL specs rewritten to describe arrays as allocation-explicit with 10 opcodes (0x0900-0x0909)**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-29T00:00:00Z
- **Completed:** 2026-03-29
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- Rewrote `array_primitives.writ` with comprehensive new-API fixture covering `len()`, indexed read/write, `resize()` grow and shrink, `slice()`, `copy_from()` cross-array, and `copy_from()` same-array overlap
- Rewrote `type_array_ops.writ` to use `resize(5)` per acceptance criteria
- Blessed `.writil` snapshots for both fixtures — both golden tests pass
- Added `#[ignore] // Phase 121: re-enable after stdlib rewrite` inline comment to 7 collection test functions
- Rewrote spec section 1.6.1 from "growable" to "allocation-explicit" with resize(n) model
- Rewrote spec section 1.6.3 to document only the 6 compiler-known operations (indexed access read/write, len, slice, resize, copy_from)
- Rewrote IL spec 57_3_9_arrays.md with 10 opcodes replacing old 9-opcode table
- Updated opcode assignment table 0x09 section (ARRAY_RESIZE/COPY/SLICE/NEW_ARRAY_SIZED/NEW_ARRAY_FILLED)
- Updated instruction count table: Arrays 9->10, Total 92->93

## Task Commits

1. **Task 1: Rewrite golden test fixtures and ignore collection tests** - `f3d6d4d` (feat)
2. **Task 2: Update language spec and IL spec documents** - `e008387` (docs)

## Files Created/Modified

- `writ-golden/tests/golden/array_primitives.writ` - Comprehensive new-API fixture
- `writ-golden/tests/golden/array_primitives.writil` - Blessed snapshot (regenerated)
- `writ-golden/tests/golden/type_array_ops.writ` - Updated to resize(5)
- `writ-golden/tests/golden/type_array_ops.writil` - Blessed snapshot (regenerated)
- `writ-golden/tests/golden_tests.rs` - 7 collection tests marked #[ignore] with inline Phase 121 comment
- `language-spec/spec/07_6_primitive_types.md` - Sections 1.6.1 and 1.6.3 rewritten
- `language-spec/spec/57_3_9_arrays.md` - Complete opcode table rewrite (10 opcodes)
- `language-spec/spec/67_4_2_opcode_assignment_table.md` - 0x09 section updated
- `language-spec/spec/65_4_0_instruction_count_by_category.md` - Arrays 9->10, total 92->93

## Decisions Made

- Extended `array_primitives.writ` beyond the Plan 02 partial version to include all required elements per the plan's acceptance criteria (indexed access, overlap copy, all ARR-0x operations)
- `#[ignore]` attributes now carry inline `// Phase 121` comments as required by acceptance criteria (doc comments alone were insufficient)
- 5 pre-existing golden test failures are out of scope (different column positions in string/generic tests — regression not introduced by Phase 120)

## Deviations from Plan

None - plan executed exactly as written. Pre-existing failures in `golden_generic_inherent_impl`, `golden_lib_preload_stub`, `test_expr_string_escapes`, `test_fn_overload`, `test_string_utilities` are pre-existing and out of scope for Phase 120.

## Issues Encountered

- The `BLESS=1` run done before `git stash` generated snapshots for the old `.writ` content. After `git stash pop`, the snapshots were stale. Required a second `BLESS=1` run against the new fixture content.

## Known Stubs

None — all golden tests exercise real compiler output, no placeholder data.

## Next Phase Readiness

- Phase 120 is fully complete: opcodes wired (plans 01-02), golden tests updated and passing, spec documents updated
- Phase 121 can proceed to rewrite the stdlib collections (`coll_list_basic.writ` etc.) to use `resize`/`copy_from` instead of `add`/`remove_at`, then re-enable the 7 ignored tests

---
*Phase: 120-array-semantics-correction*
*Completed: 2026-03-29*
