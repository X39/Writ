---
phase: 120-array-semantics-correction
plan: 01
subsystem: module-format
tags: [instruction-set, opcodes, binary-format, writ-module, array-semantics]

# Dependency graph
requires: []
provides:
  - "Instruction enum with array opcode set: ArrayResize (0x0905), ArrayCopy (0x0906), ArraySlice (0x0907), NewArraySized (0x0908), NewArrayFilled (0x0909)"
  - "format_version 5 in builder, module default, and reader validation"
  - "All new opcodes encode and decode correctly (round-trip verified)"
  - "Removed opcodes: ArrayAdd, ArrayRemove, ArrayInsert, ArrayContains"
affects: [120-02, 120-03, writ-assembler, writ-runtime, writ-compiler]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "New opcode variants follow the existing enum+opcode()+encode()+decode() 4-part pattern"
    - "format_version bump requires coordinated 3-file change: builder.rs + module.rs + reader.rs"

key-files:
  created: []
  modified:
    - writ-module/src/instruction.rs
    - writ-module/src/builder.rs
    - writ-module/src/module.rs
    - writ-module/src/reader.rs
    - writ-module/tests/instruction_tests.rs

key-decisions:
  - "Clean break (D-01): old array opcodes removed entirely — no tombstones, no deprecation shims"
  - "format_version bumped to 5 (D-02): reader rejects any version != 5"
  - "ArraySlice renumbered from 0x0908 to 0x0907 to compact the block (D-03)"
  - "Instruction test count updated to 100 (was 99) since array group grew from 9 to 10 opcodes"

patterns-established:
  - "Opcode addition: enum variant → opcode() number → encode() arm → decode() arm → update tests"
  - "format_version bump: 3-file atomic change (builder, module, reader) in a single commit"

requirements-completed: [ARR-01, ARR-02, ARR-03, ARR-04, ARR-05]

# Metrics
duration: 15min
completed: 2026-03-29
---

# Phase 120 Plan 01: Array Semantics Correction — Module Foundation Summary

**Replaced four growth opcodes (ArrayAdd/Remove/Insert/Contains) with four allocation-explicit opcodes (ArrayResize/ArrayCopy/NewArraySized/NewArrayFilled), renumbered ArraySlice from 0x0908 to 0x0907, and bumped format_version to 5 in writ-module**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-29T00:00:00Z
- **Completed:** 2026-03-29
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Removed ArrayAdd, ArrayRemove, ArrayInsert, ArrayContains from Instruction enum (clean break per D-01)
- Added ArrayResize (0x0905), ArrayCopy (0x0906), NewArraySized (0x0908), NewArrayFilled (0x0909) with correct encode/decode
- Renumbered ArraySlice from 0x0908 to 0x0907 (compaction per D-03), producing a gapless 0x0900-0x0909 block
- Bumped format_version from 4 to 5 across builder.rs, module.rs, and reader.rs (D-02)
- Updated instruction_tests.rs: replaced old round-trip tests, added new ones, updated the comprehensive all-opcodes test (now 100 instructions)
- All writ-module tests pass (100 instructions verified round-trip)

## Task Commits

Each task was committed atomically:

1. **Task 1: Remove old opcodes, add new opcodes to Instruction enum** - `164d2cb` (feat)
2. **Task 2: Bump format_version from 4 to 5** - `1d17b6a` (feat)

## Files Created/Modified

- `writ-module/src/instruction.rs` - Instruction enum, opcode(), encode(), decode() for all 10 array opcodes
- `writ-module/src/builder.rs` - format_version: 5 in ModuleBuilder::build()
- `writ-module/src/module.rs` - format_version: 5 in Module::new(), updated doc comment
- `writ-module/src/reader.rs` - Reject format_version != 5
- `writ-module/tests/instruction_tests.rs` - Updated tests: replaced 3 old round-trip tests, added 4 new round-trip tests, updated comprehensive all-opcodes list (100 items)

## Decisions Made

- D-01 clean break applied: old opcodes removed without deprecation shims or error stubs
- D-02 format_version=5: reader unconditionally rejects non-5 versions
- D-03 compaction: ArraySlice moved from 0x0908 to 0x0907 so 0x0905-0x0909 are all new/moved opcodes
- Instruction count in test updated to 100 (not 99): the old list was missing ArrayContains, so the net add was +1 item when replacing with the correct 10-opcode array group

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Test count mismatch: comprehensive opcode list had 9 array entries, not 10**
- **Found during:** Task 1 (array opcode overhaul)
- **Issue:** The existing comprehensive test listed 9 array opcodes (missing ArrayContains), asserted len==99. After the overhaul we have 10 array opcodes, so the list becomes 100 items.
- **Fix:** Updated assertion from 99 to 100 and added all 10 new array opcodes to the list
- **Files modified:** writ-module/tests/instruction_tests.rs
- **Verification:** `cargo test -p writ-module` passes with 100-item assertion
- **Committed in:** 164d2cb (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Fix necessary for test correctness. No scope creep.

## Issues Encountered

None — straightforward enum variant replacement with encode/decode updates.

## Next Phase Readiness

- writ-module instruction set is now the authoritative definition of the array opcode table
- Downstream crates (writ-compiler, writ-runtime, writ-assembler) will have compile errors until Plan 02 updates them
- Plan 02 can now proceed: update assembler/disassembler mnemonics for all 10 array opcodes
- Plan 03 can proceed after Plan 02: update runtime dispatch and compiler emission

---
*Phase: 120-array-semantics-correction*
*Completed: 2026-03-29*

## Self-Check: PASSED

- [x] writ-module/src/instruction.rs exists and contains `ArrayResize`
- [x] writ-module/src/builder.rs contains `format_version: 5`
- [x] writ-module/src/module.rs contains `format_version: 5`
- [x] writ-module/src/reader.rs contains `!= 5`
- [x] Commits 164d2cb and 1d17b6a exist in git log
- [x] All writ-module tests pass (100 instructions round-trip verified)
