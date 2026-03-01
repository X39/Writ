---
phase: 16-module-format-foundation
plan: 01
subsystem: runtime
tags: [rust, il, binary-format, metadata, instruction-set]

requires: []
provides:
  - writ-module crate with all IL data types
  - MetadataToken newtype with 1-based indexing
  - 98-opcode Instruction enum with encode/decode round-trip
  - 21 metadata table row structs matching spec section 2.16.5
  - Module struct with all table Vecs, heaps, and method bodies
  - String/blob heap utilities
  - ModuleError/DecodeError/EncodeError types
affects: [vm-core, assembler, disassembler, module-reader-writer, module-builder]

tech-stack:
  added: [byteorder, thiserror]
  patterns: [newtype-pattern, encode-decode-round-trip, spec-driven-data-modeling]

key-files:
  created:
    - writ-module/src/token.rs
    - writ-module/src/tables.rs
    - writ-module/src/instruction.rs
    - writ-module/src/module.rs
    - writ-module/src/heap.rs
    - writ-module/src/error.rs
    - writ-module/src/lib.rs
    - writ-module/tests/token_tests.rs
    - writ-module/tests/instruction_tests.rs
  modified:
    - Cargo.toml

key-decisions:
  - "Spec defines 98 distinct opcodes (plan estimated 91) — implemented all 98 with correct opcode assignments"
  - "PartialEq for Instruction uses byte-comparison via encode for non-float variants and bit-exact comparison for LoadFloat"
  - "Heap functions are standalone pub fns rather than methods on a heap struct — keeps heap as simple Vec<u8>"

patterns-established:
  - "MetadataToken: u32 newtype with table_id in bits 31-24 and 1-based row_index in bits 23-0"
  - "Instruction encode/decode: match on opcode u16, read/write operands per spec shape"
  - "Table row structs: simple data structs with heap offsets as u32, tokens as MetadataToken"

requirements-completed: [MOD-04, MOD-05]

duration: 7min
completed: 2026-03-02
---

# Phase 16 Plan 01: Core Data Types Summary

**writ-module crate with 98-opcode Instruction enum, 21 metadata table structs, MetadataToken newtype, and heap utilities**

## Performance

- **Duration:** 7 min
- **Started:** 2026-03-01T23:20:43Z
- **Completed:** 2026-03-01T23:27:54Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments
- Created standalone writ-module crate as workspace member with byteorder and thiserror dependencies
- Implemented MetadataToken with null semantics, 1-based indexing, and table_id/row_index extraction
- Built all 21 metadata table row structs matching spec section 2.16.5 exactly
- Implemented 98-opcode Instruction enum with full binary encode/decode and round-trip identity
- Created Module struct holding all 21 table Vecs, string/blob heaps, method bodies, and 200-byte header
- 70 tests passing (9 token + 61 instruction)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create writ-module crate with data types** - `cab7809` (feat)
2. **Task 2: Instruction encode/decode round-trip tests** - `c7a86a0` (test)

## Files Created/Modified
- `Cargo.toml` - Added writ-module to workspace members
- `writ-module/Cargo.toml` - Crate manifest with byteorder + thiserror
- `writ-module/src/lib.rs` - Module declarations and re-exports
- `writ-module/src/error.rs` - ModuleError, DecodeError, EncodeError types
- `writ-module/src/token.rs` - MetadataToken newtype with NULL, new(), table_id(), row_index()
- `writ-module/src/tables.rs` - 21 row structs, TypeDefKind, TableId enums
- `writ-module/src/instruction.rs` - 98-variant Instruction enum with encode/decode
- `writ-module/src/module.rs` - Module, ModuleHeader, MethodBody, DebugLocal, SourceSpan structs
- `writ-module/src/heap.rs` - String/blob heap init, intern, read utilities
- `writ-module/tests/token_tests.rs` - 9 MetadataToken tests
- `writ-module/tests/instruction_tests.rs` - 61 instruction round-trip tests

## Decisions Made
- Spec section 4.2 actually defines 98 distinct opcodes, not the 91 estimated in the plan. All 98 were implemented.
- PartialEq for Instruction uses encode-to-bytes comparison for all variants except LoadFloat (which uses bit-exact f64 comparison)
- Heap functions kept as standalone pub fns on Vec<u8> rather than wrapping in a struct

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected opcode count from 91 to 98**
- **Found during:** Task 2 (instruction tests)
- **Issue:** Plan stated "91 opcodes" but spec section 4.2 defines 98 distinct opcodes
- **Fix:** Implemented all 98 opcodes, updated test assertion to match
- **Files modified:** writ-module/tests/instruction_tests.rs
- **Verification:** All 61 instruction tests pass including the comprehensive all-98 test
- **Committed in:** c7a86a0

**2. [Rule 3 - Blocking] Removed unused std::fmt import**
- **Found during:** Task 1 (initial build)
- **Issue:** error.rs imported std::fmt but didn't use it (compiler warning)
- **Fix:** Removed the unused import
- **Files modified:** writ-module/src/error.rs
- **Verification:** cargo build -p writ-module compiles with no warnings
- **Committed in:** cab7809

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Opcode count correction was necessary for spec accuracy. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All data types ready for Plan 02 (binary reader/writer)
- Module struct, all table structs, instruction encode/decode, and heaps are fully operational
- No blockers for proceeding

## Self-Check: PASSED

---
*Phase: 16-module-format-foundation*
*Completed: 2026-03-02*
