---
phase: 16-module-format-foundation
plan: 02
subsystem: runtime
tags: [rust, il, binary-format, serialization, round-trip]

requires:
  - phase: 16-module-format-foundation plan 01
    provides: All data types (MetadataToken, table row structs, Instruction, Module)
provides:
  - Module::from_bytes binary deserialization
  - Module::to_bytes binary serialization
  - Round-trip identity (write -> read -> write = identical bytes)
  - Error handling for malformed input
affects: [vm-core, assembler, disassembler, module-builder]

tech-stack:
  added: []
  patterns: [cursor-based-reading, layout-computation-then-write, verbatim-heap-round-trip]

key-files:
  created:
    - writ-module/src/reader.rs
    - writ-module/src/writer.rs
    - writ-module/tests/round_trip.rs
  modified:
    - writ-module/src/module.rs
    - writ-module/src/lib.rs

key-decisions:
  - "Writer recomputes all offsets (tables, heaps, bodies) — header offset fields from Module struct are ignored"
  - "Heaps written verbatim for byte-exact round-trip — no re-interning or reordering"
  - "Raw code bytes round-tripped — not re-encoded from decoded instructions"
  - "Row padding bytes written/read symmetrically for 4-byte alignment"

patterns-established:
  - "Binary layout: header(200) -> tables(0-20) -> string heap -> blob heap -> method bodies"
  - "Table row alignment: each row type has a compile-time ROW_SIZES constant for 4-byte alignment"
  - "Writer patches MethodDef body_offset/body_size during serialization"

requirements-completed: [MOD-01, MOD-02, MOD-06]

duration: 4min
completed: 2026-03-02
---

# Phase 16 Plan 02: Binary Reader/Writer Summary

**Module::from_bytes and Module::to_bytes with byte-exact round-trip identity for all 21 table types and method bodies**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-01T23:29:18Z
- **Completed:** 2026-03-01T23:33:11Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Implemented from_bytes reader parsing 200-byte header, all 21 tables with 4-byte aligned rows, string/blob heaps, and method bodies
- Implemented to_bytes writer with layout computation (tables -> heaps -> bodies), offset patching, and alignment padding
- Round-trip identity verified for empty modules, modules with strings, type defs, method bodies, and multiple tables
- Error handling for bad magic bytes and truncated input
- 77 total tests passing across all test files

## Task Commits

1. **Task 1: Binary reader and writer** - `02aa7f9` (feat)
2. **Task 2: Round-trip identity tests** - `38e4e96` (test)

## Files Created/Modified
- `writ-module/src/reader.rs` - from_bytes with 21 table row readers and method body parsing
- `writ-module/src/writer.rs` - to_bytes with layout computation and 21 table row writers
- `writ-module/src/module.rs` - Added from_bytes/to_bytes delegation methods
- `writ-module/src/lib.rs` - Added reader/writer module declarations
- `writ-module/tests/round_trip.rs` - 7 round-trip and error case tests

## Decisions Made
None - followed plan as specified

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Binary reader/writer ready for Plan 03 (ModuleBuilder)
- Builder-produced modules can now be verified through to_bytes -> from_bytes -> to_bytes round-trip

## Self-Check: PASSED

---
*Phase: 16-module-format-foundation*
*Completed: 2026-03-02*
