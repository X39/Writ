---
phase: 111-assembler-completeness
plan: 01
subsystem: assembler
tags: [writ-assembler, disassembler, directive, round-trip, blob-heap, register-types]

# Dependency graph
requires:
  - phase: 100-spec-and-il-foundation
    provides: writ-module format, ModuleBuilder API, blob heap write_blob
provides:
  - ".export, .extern_fn, .component_slot, .locale, .attribute directives fully parse, assemble, and disassemble"
  - "Register type blob offsets are real heap positions (not 0 placeholders)"
  - "7 new round-trip integration tests covering all 5 directives and register type correctness"
affects:
  - writ-assembler
  - writ-module

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Directive parity: disassembler must emit parseable directives for every table entry"
    - "Post-build blob interning: encode_type_ref + write_blob after builder.build() for register types"

key-files:
  created:
    - writ-assembler/src/ast.rs (AsmExport, AsmComponentSlot, AsmLocaleDef, AsmAttributeDef structs)
  modified:
    - writ-assembler/src/lexer.rs (5 new known_directives entries)
    - writ-assembler/src/ast.rs (4 new fields on AsmModule)
    - writ-assembler/src/parser.rs (5 new parse_* methods, 5 new dispatch arms, 4 new AsmModule fields)
    - writ-assembler/src/assembler.rs (4 new builder call blocks, register type blob interning)
    - writ-assembler/src/disassembler.rs (sections 6/8/9/10/11 now emit real directives)
    - writ-assembler/tests/asm_round_trip.rs (7 new tests)

key-decisions:
  - "Parse .extern_fn as a separate directive from .extern (module ref) — lexer tokenizes as Directive(extern_fn)"
  - "Register type blob offsets intern post-build via write_blob into module.blob_heap — ModuleBuilder doesn't expose heap during build"
  - "Attribute disassembly includes owner_kind for round-trip fidelity — old format omitted it"

patterns-established:
  - "New directive pattern: lexer entry -> AST struct -> AsmModule field -> parser method + dispatch arm -> assembler builder call -> disassembler output"

requirements-completed: [ASM-01, ASM-02]

# Metrics
duration: 15min
completed: 2026-03-28
---

# Phase 111 Plan 01: Assembler Completeness Summary

**Full directive parity between disassembler and assembler for .export, .extern_fn, .component_slot, .locale, .attribute, plus real register type blob offsets replacing 0 placeholders**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-28T23:15:00Z
- **Completed:** 2026-03-28T23:30:03Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- 5 new directives parse (lexer tokens, AST nodes, parser methods) and assemble (builder calls)
- Disassembler sections 6/8/9/10/11 now emit real directives instead of commented-out stubs
- Register type blob offsets are real heap positions: `encode_type_ref` + `write_blob` post-build
- 7 new integration tests confirm round-trip fidelity; all 31+ existing tests continue to pass
- Full workspace builds cleanly

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend lexer, AST, parser, and disassembler for 5 directives** - `8450fc4` (feat)
2. **Task 2: Wire assembler for new directives and fix register type blob offsets** - `4940513` (feat)
3. **Task 3: Add round-trip integration tests for all new directives and register types** - `915ccc1` (test)

## Files Created/Modified

- `writ-assembler/src/lexer.rs` - Added extern_fn/export/component_slot/locale/attribute to known_directives
- `writ-assembler/src/ast.rs` - Added AsmExport/AsmComponentSlot/AsmLocaleDef/AsmAttributeDef structs + 4 new AsmModule fields
- `writ-assembler/src/parser.rs` - Added 5 parse_* methods and dispatch arms, updated AsmModule construction
- `writ-assembler/src/assembler.rs` - Added 4 builder call blocks, imported write_blob, fixed register type blob interning
- `writ-assembler/src/disassembler.rs` - Removed comment prefix from sections 6/8/9/10/11, added owner_kind to attribute output
- `writ-assembler/tests/asm_round_trip.rs` - 7 new tests covering all directives and register type correctness

## Decisions Made

- Parse `.extern_fn` as a separate directive from `.extern` (module ref) because they are semantically different table entries. The lexer sees `extern_fn` as a single token (underscore allowed in directive names).
- Register type blob offsets are interned post-build (after `builder.build()`) because ModuleBuilder doesn't expose the blob heap during the build phase. The `assemble_method_body` now returns `Vec::new()` as placeholder which gets overwritten.
- Added `owner_kind` to attribute disassembly output (`.attribute owner owner_kind "name"`) because the old format omitted it, breaking round-trip fidelity.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Assembler/disassembler are now feature-complete for all 21 table types
- Round-trip fidelity confirmed for all directive types
- Remaining v12.0 tech debt items: StrLen runtime bug, spec TOC entry, test registration, LSP/DAP fixes

---
*Phase: 111-assembler-completeness*
*Completed: 2026-03-28*
