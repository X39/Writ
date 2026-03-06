---
phase: 68-dap-runtime-and-launch-fixes
plan: 01
subsystem: compiler
tags: [writ-compiler, serialize, il-encoding, byte-offsets, switch, defer, dap]

# Dependency graph
requires:
  - phase: 67-lsp-completions
    provides: prior LSP completion fixes
provides:
  - Pass 4 in encode_instructions() converts SWITCH and DeferPush instruction-index offsets to byte-position offsets
  - emit_defer Br skip uses label fixup pipeline (not direct instruction-index patch)
  - quest_system.writ compiles and runs through DAP pipeline without decode errors

affects:
  - 68-02 (further DAP fixes may build on corrected binary encoding)
  - writ-golden (golden files now reflect correct byte-relative SWITCH/DeferPush/Br offsets)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "encode_instructions() uses 4-pass approach: byte starts, encode, fixup labels, then SWITCH/DeferPush post-processing"
    - "Variable-length instruction types (SWITCH) and absolute-target types (DeferPush) require explicit Pass 4 byte-offset conversion"
    - "emit_defer Br skip now uses add_fixup/label pipeline; DeferPush method_idx still stored as instruction index, converted in Pass 4"

key-files:
  created: []
  modified:
    - writ-compiler/src/emit/serialize.rs
    - writ-compiler/src/emit/body/expr/control.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-golden/tests/golden/adv_defer.writil
    - writ-golden/tests/golden/quest_system.writil
    - writ-golden/tests/golden/type_enum_match.writil

key-decisions:
  - "Fix SWITCH offsets in serialize.rs Pass 4, not in patterns.rs — keeps encoding concerns in serializer, emitter stays index-based"
  - "Fix DeferPush byte offset in same Pass 4 — same class of bug, same fix site"
  - "Fix emit_defer Br skip to use label fixup pipeline (was direct instr-index patch bypassing Pass 3)"
  - "Bless golden files: byte-relative offsets are the correct observable output"

patterns-established:
  - "Pass 4 pattern: after Pass 3 label fixups, iterate instructions again for variable-layout types that can't use fixup pipeline"

requirements-completed:
  - DAP-01

# Metrics
duration: 14min
completed: 2026-03-18
---

# Phase 68 Plan 01: DAP Runtime and Launch Fixes Summary

**SWITCH and DeferPush byte-offset encoding fixed in encode_instructions() Pass 4, enabling quest_system.writ to compile and run through the DAP pipeline without decode errors**

## Performance

- **Duration:** 14 min
- **Started:** 2026-03-18T17:23:23Z
- **Completed:** 2026-03-18T17:37:00Z
- **Tasks:** 1
- **Files modified:** 6 (3 source, 3 golden)

## Accomplishments
- Added Pass 4 to `encode_instructions()` in `serialize.rs` that converts SWITCH instruction-index-relative offsets to byte-position offsets
- Fixed DeferPush byte-offset encoding in the same Pass 4 (same bug class: instruction index stored, byte offset expected by runtime)
- Fixed `emit_defer` to use label fixup pipeline for the Br skip instruction (was directly patching with instruction-index distance, bypassing Pass 3)
- quest_system.writ now compiles and runs through the full DAP debug session pipeline without "Switch target byte offset not found in offset map" or "DeferPush handler byte offset not found in offset map" decode errors
- Updated golden files to reflect correct byte-relative offsets

## Task Commits

1. **Task 1: Add SWITCH byte-offset post-processing pass to encode_instructions** - `9feb038` (feat)

**Plan metadata:** (included in task commit)

## Files Created/Modified
- `writ-compiler/src/emit/serialize.rs` - Added Pass 4 for SWITCH and DeferPush byte-offset conversion; added `test_encode_switch_byte_offsets` unit test
- `writ-compiler/src/emit/body/expr/control.rs` - Fixed `emit_defer` to use label fixup pipeline for Br skip instead of direct instruction-index patch
- `writ-compiler/tests/emit_body_tests.rs` - Updated `test_defer_handler_offset_matches_handler_start` to expect Br offset=0 (placeholder for fixup pipeline)
- `writ-golden/tests/golden/adv_defer.writil` - Blessed with correct byte-relative DEFER_PUSH and BR offsets
- `writ-golden/tests/golden/quest_system.writil` - Blessed with correct encoding output
- `writ-golden/tests/golden/type_enum_match.writil` - Blessed with correct byte-relative SWITCH offsets

## Decisions Made
- Fixed SWITCH in `serialize.rs` Pass 4 rather than `patterns.rs` — the emitter correctly stores instruction-index distances for internal use; the serializer is responsible for the binary encoding transformation.
- Fixed DeferPush in the same Pass 4 pass — same bug class (instruction index stored, byte offset expected), same fix site.
- Fixed `emit_defer`'s Br to use `add_fixup` + label pipeline — this makes the Br consistent with all other branch instructions that go through Pass 3; DeferPush.method_idx remains instruction-index-based and is converted in Pass 4.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed DeferPush byte-offset encoding (unmasked by SWITCH fix)**
- **Found during:** Task 1 (verification of quest_system.writ DAP session)
- **Issue:** After fixing SWITCH, a new error appeared: "DeferPush handler byte offset 3 not found in offset map". emit_defer stored the handler instruction index as method_idx, but the runtime's decode_and_reindex expects a byte offset from method start. This bug was always present but was hidden because the SWITCH error came first.
- **Fix:** Extended Pass 4 in `encode_instructions()` to also patch DeferPush.method_idx with the correct byte offset.
- **Files modified:** `writ-compiler/src/emit/serialize.rs`
- **Verification:** test_quest_system_compiles and test_quest_system_full_debug_session both pass
- **Committed in:** 9feb038 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed emit_defer Br skip to use label fixup pipeline**
- **Found during:** Task 1 (verification after DeferPush fix — "Br target byte offset 14 not found in offset map")**
- **Issue:** emit_defer directly patched Br.offset with an instruction-index-relative value, bypassing the fixup pipeline (Pass 3). After DeferPush was fixed, the runtime now reached the Br and failed on its incorrect offset.
- **Fix:** Changed emit_defer to use `emitter.add_fixup(br_skip_idx, after_handler_label)` and `emitter.mark_label_here(after_handler_label)`, making it consistent with all other branch instructions. Updated the existing test to expect offset=0 placeholder.
- **Files modified:** `writ-compiler/src/emit/body/expr/control.rs`, `writ-compiler/tests/emit_body_tests.rs`
- **Verification:** All compiler tests pass; DAP tests pass
- **Committed in:** 9feb038 (Task 1 commit)

**3. [Rule 1 - Bug] Blessed golden files with correct byte-relative offsets**
- **Found during:** Task 1 (full workspace test run revealed 3 golden test failures)
- **Issue:** adv_defer.writil, type_enum_match.writil, and quest_system.writil golden files had instruction-index-relative SWITCH/DeferPush/Br offset values. After the fix, the correct byte-relative values no longer matched.
- **Fix:** Ran `BLESS=1 cargo test -p writ-golden` to update golden files with correct output.
- **Files modified:** 3 .writil golden files
- **Verification:** All 34 golden tests pass
- **Committed in:** 9feb038 (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 1 — same bug class, same root cause: instruction-index values encoded where byte offsets are required)
**Impact on plan:** All fixes necessary for correctness. The DeferPush and Br bugs were always present but hidden; fixing SWITCH unmasked them in sequence.

## Issues Encountered
- The plan focused on SWITCH only, but DeferPush had the identical bug and the Br in emit_defer had a related bug. All three needed to be fixed to pass the acceptance criteria. Each fix unmasked the next one in sequence (SWITCH -> DeferPush -> Br).

## Next Phase Readiness
- quest_system.writ now compiles and runs through the DAP pipeline without byte-offset decode errors
- The DAP full debug session test passes with launch_success=true (program runs to completion without hitting breakpoints, which is expected behavior)
- Further DAP work (breakpoints, stepping, variable inspection) can proceed on a solid encoding foundation

## Self-Check: PASSED

- writ-compiler/src/emit/serialize.rs: FOUND
- writ-compiler/src/emit/body/expr/control.rs: FOUND
- .planning/phases/68-dap-runtime-and-launch-fixes/68-01-SUMMARY.md: FOUND
- Commit 9feb038: FOUND
- "Pass 4" comment in serialize.rs: FOUND (line 465)
- Instruction::Switch match in serialize.rs: FOUND (line 478)
- copy_from_slice byte patch in serialize.rs: FOUND (line 491)
- test_encode_switch_byte_offsets in serialize.rs: FOUND (line 628)

---
*Phase: 68-dap-runtime-and-launch-fixes*
*Completed: 2026-03-18*
