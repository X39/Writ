---
phase: 16-module-format-foundation
plan: 03
subsystem: runtime
tags: [rust, il, builder-api, module-construction, string-interning]

requires:
  - phase: 16-module-format-foundation plan 01
    provides: All data types (MetadataToken, table row structs, Instruction, Module)
  - phase: 16-module-format-foundation plan 02
    provides: Module::from_bytes and Module::to_bytes for round-trip verification
provides:
  - ModuleBuilder fluent API for programmatic Module construction
  - Automatic string/blob heap interning (users pass &str, not offsets)
  - Builder integration tests proving round-trip through serialization
affects: [assembler, compiler-backend, vm-tests]

tech-stack:
  added: []
  patterns: [builder-pattern, string-interning, list-ownership-tracking]

key-files:
  created:
    - writ-module/src/builder.rs
    - writ-module/tests/builder_tests.rs
  modified:
    - writ-module/src/lib.rs

key-decisions:
  - "Builder row types use String/Vec<u8> for ergonomics; build() interns into heaps"
  - "field_list/method_list computed via next-index pattern at add_type_def time"
  - "Header offset fields set to 0 in builder — writer is authoritative for layout"
  - "body_size set to 1 (non-zero) as placeholder — writer recomputes during serialization"

patterns-established:
  - "Builder pattern: add_* methods return MetadataToken; build() interns all strings/blobs"
  - "List ownership: add type first, then its fields/methods, before adding next type"

requirements-completed: [MOD-03]

duration: 5min
completed: 2026-03-02
---

# Phase 16 Plan 03: ModuleBuilder API Summary

**ModuleBuilder fluent API with automatic string/blob interning for all 21 table types, verified by 8 integration tests and round-trip serialization**

## Performance

- **Duration:** 5 min
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Implemented ModuleBuilder with 21 add_* methods covering all table types
- Automatic string heap interning (users pass &str, builder handles offsets)
- Automatic blob heap interning (users pass &[u8], builder handles offsets)
- field_list/method_list ownership tracking via next-index pattern
- 8 integration tests covering: empty builder, version, types+fields, method bodies, round-trip serialization, multiple types, header name, serialization output
- All 85 tests passing across the crate (9 token + 61 instruction + 7 round_trip + 8 builder)

## Task Commits

1. **Task 1: Implement ModuleBuilder with fluent API** - `a7198ed` (feat)
2. **Task 2: ModuleBuilder integration tests** - `45ef2f6` (test)

## Files Created/Modified
- `writ-module/src/builder.rs` - ModuleBuilder struct with 21 builder row types and build() method
- `writ-module/tests/builder_tests.rs` - 8 integration tests for builder functionality
- `writ-module/src/lib.rs` - Added `pub mod builder` and `pub use builder::ModuleBuilder`

## Decisions Made
- Builder row types mirror table row structs but use String/Vec<u8> for ergonomics
- field_list/method_list computed at add_type_def time using current counts (next-index pattern)
- Header offset fields set to 0 since the writer recomputes them during to_bytes

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- ModuleBuilder ready for Phase 20 (Text Assembler) and future compiler backend
- All Phase 16 plans complete: data types, binary reader/writer, and builder API
- 85 total tests providing comprehensive coverage

## Self-Check: PASSED

---
*Phase: 16-module-format-foundation*
*Completed: 2026-03-02*
