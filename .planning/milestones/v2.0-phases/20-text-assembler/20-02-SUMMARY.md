---
phase: 20-text-assembler
plan: 02
subsystem: assembler
tags: [assembler, two-pass, label-resolution, modulebuilder, writil]

# Dependency graph
requires:
  - phase: 20-text-assembler
    provides: Lexer, AST, and parser (Plan 01)
  - phase: 16-module-format-foundation
    provides: ModuleBuilder API, Instruction encode/decode, Module from_bytes/to_bytes
provides:
  - Complete text IL to binary module assembler pipeline
  - Two-pass label resolution for forward and backward branch references
  - Name resolution for types, methods, fields, and contracts
  - Integration tests covering all four ASM requirements
affects: [21-disassembler-and-runner-cli]

# Tech tracking
tech-stack:
  added: []
  patterns: [two-pass label resolution, pre-register then patch method bodies]

key-files:
  created:
    - writ-assembler/src/assembler.rs
    - writ-assembler/tests/asm_basic.rs
    - writ-assembler/tests/asm_labels.rs
    - writ-assembler/tests/asm_round_trip.rs
    - writ-assembler/tests/asm_errors.rs
  modified:
    - writ-assembler/src/lib.rs

key-decisions:
  - "Pre-register all methods with placeholder bodies before assembling bodies -- enables forward method references"
  - "Two-pass label resolution: pass 1 collects label byte offsets with placeholder branch offsets; pass 2 patches branches"
  - "Branch offset = target_offset - (current_offset + instruction_size) -- relative to byte after branch instruction"
  - "Register types stored as placeholder 0 offsets -- ModuleBuilder doesn't expose blob heap for external interning"
  - "Method bodies patched into module after builder.build() since builder consumes placeholders"

patterns-established:
  - "Pattern: pre-register + patch for forward references in two-phase assembly"
  - "Pattern: instruction_size() via encode-to-temp-buffer for byte offset tracking"
  - "Pattern: label patches tracked alongside instructions as Vec<String> label names"

requirements-completed: [ASM-01, ASM-02, ASM-03, ASM-04]

# Metrics
duration: ~15min
completed: 2026-03-02
---

# Phase 20 Plan 02: Two-pass Assembler and Integration Tests Summary

**Full text IL to binary module pipeline with two-pass label resolution, all 91 instruction mnemonics, and round-trip verification through Module::from_bytes**

## Performance

- **Duration:** ~15 min
- **Tasks:** 2 (assembler core + integration tests, committed together)
- **Files created:** 5
- **Files modified:** 1

## Accomplishments
- Implemented two-pass assembler converting AST to spec-valid binary modules via ModuleBuilder
- All 91 instruction mnemonics mapped to Instruction variants with case-insensitive matching
- Forward and backward label resolution with correct branch offset computation (relative to byte after branch)
- Name resolution for types, methods, fields, and contracts across all directive types
- Binary round-trip verified: text IL -> assemble -> to_bytes -> from_bytes succeeds with structure preserved
- 21 integration tests covering all four ASM requirements plus error diagnostics

## Task Commits

Each task was committed atomically:

1. **Task 1+2: Assembler core and integration tests** - `cabb453` (feat)

## Files Created/Modified
- `writ-assembler/src/assembler.rs` - Two-pass assembler: name resolution, label resolution, ModuleBuilder integration
- `writ-assembler/src/lib.rs` - Wired assembler into tokenize -> parse -> assemble pipeline
- `writ-assembler/tests/asm_basic.rs` - 7 tests: minimal module, types, contracts, impls, methods
- `writ-assembler/tests/asm_labels.rs` - 4 tests: forward/backward/mixed label resolution
- `writ-assembler/tests/asm_round_trip.rs` - 4 tests: binary validity, structure preservation, instruction decode
- `writ-assembler/tests/asm_errors.rs` - 6 tests: undefined labels, unknown mnemonics, multi-error collection

## Decisions Made
- Pre-register methods with placeholder bodies before assembling instruction operands -- this enables forward method references within the same module
- Register types stored as placeholder 0 blob heap offsets since ModuleBuilder doesn't expose its blob heap for external interning. This is a known limitation; register type metadata is not execution-critical for the assembler's validation purpose
- Method bodies are patched into the Module struct after builder.build() since the builder stores placeholder bodies during pre-registration

## Deviations from Plan
None - plan executed as specified.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Complete assembler pipeline available for Phase 21 (Disassembler and Runner CLI)
- All four ASM requirements satisfied and verified by tests
- 55 total tests in writ-assembler crate (10 lexer + 24 parse + 21 integration)

---
*Phase: 20-text-assembler, Plan: 02*
*Completed: 2026-03-02*
