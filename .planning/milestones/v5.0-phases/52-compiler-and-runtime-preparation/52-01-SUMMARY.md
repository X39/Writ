---
phase: 52-compiler-and-runtime-preparation
plan: 01
subsystem: compiler, module-format, disassembler
tags: [debug-info, binary-format, source-spans, lsp-prep, dap-prep]
dependency_graph:
  requires: []
  provides: [DebugLocal-v4-format, SourceSpan-line-col, build_line_starts, byte_offset_to_line_col, disasm-locals-section]
  affects: [writ-module, writ-compiler, writ-assembler, writ-cli, writ-golden]
tech_stack:
  added: []
  patterns: [tdd-red-green, line-offset-table, binary-format-versioning]
key_files:
  created: []
  modified:
    - writ-module/src/module.rs
    - writ-module/src/writer.rs
    - writ-module/src/reader.rs
    - writ-module/src/builder.rs
    - writ-module/tests/round_trip.rs
    - writ-compiler/src/emit/serialize.rs
    - writ-compiler/src/emit/mod.rs
    - writ-compiler/src/emit/body/debug.rs
    - writ-compiler/src/emit/body/stmt.rs
    - writ-cli/src/main.rs
    - writ-assembler/src/disassembler.rs
    - writ-golden/tests/golden/ (26 .writil files re-blessed)
decisions:
  - "DebugLocal type_ref is back-filled from register_types blob offsets after encode_type in translate()"
  - "Source text threading uses first available source file as fallback when body has no FileId"
  - "One SourceSpan per statement at emit_stmt entry point (sufficient for line-level stepping)"
  - "disasm source location comments only on instructions where pc exactly matches a SourceSpan.pc"
metrics:
  duration: ~11 minutes
  completed_date: "2026-03-14"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 14
---

# Phase 52 Plan 01: Extend DebugLocal Format and Fix SourceSpan Line Numbers Summary

**One-liner:** Format version 4 binary with type_ref on DebugLocal, real 1-based line:col in SourceSpan, and disassembler .locals section and source location comments.

## What Was Built

### PREP-05: DebugLocal Extended with type_ref

The `DebugLocal` struct in `writ-module/src/module.rs` gained a `type_ref: u32` field (blob heap offset) between `name` and `start_pc`. The binary format version was bumped from 3 to 4 in `module.rs`, `builder.rs`, and `serialize.rs`. The reader now rejects version 3 (previously accepted), and the writer emits 18 bytes per entry (up from 14). All four sites that set format_version were updated.

The `type_ref` field is populated in `serialize.rs`'s `translate()` function by back-filling from the `register_types` blob offset array after register type encoding completes.

### PREP-01: Real 1-based Line/Column in SourceSpan

Two new functions were added to `writ-compiler/src/emit/serialize.rs`:
- `build_line_starts(src: &str) -> Vec<u32>`: scans source bytes for `\n` to build a sorted offset table
- `byte_offset_to_line_col(offset: u32, line_starts: &[u32]) -> (u32, u16)`: binary search (partition_point) for O(log n) conversion

`emit_bodies` gained a `sources: &[(FileId, &str)]` parameter threaded from the CLI's `run_pipeline` through `serialize::serialize` to `build_source_spans`. The CLI constructs `sources` from `file_sources` and passes it. Call sites in compiler tests and golden tests pass `&[]` (graceful fallback: all text on line 1).

`emit_stmt` now pushes one `(instr_idx, stmt_span)` entry to `BodyEmitter.source_spans` at each statement boundary, fixing Pitfall 2 (source_spans was always empty).

### Disassembler: .locals Section and Source Location Comments

`disassemble_body` in `writ-assembler/src/disassembler.rs` now:
1. Emits a `.locals { }` block before register declarations, listing named locals (debug_locals with name != 0) with register index, type text (via `decode_type_sig`), variable name (via `read_string`), and scope range `[start_pc, end_pc)`
2. Appends `; line:N col:M` comments on instructions whose byte offset exactly matches a SourceSpan.pc entry

All 26 golden `.writil` files were re-blessed to reflect the new format.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Extend DebugLocal format and fix SourceSpan line numbers (TDD) | e021941 |
| 2 | Update disassembler and re-bless golden tests | 9b38060 |

## Verification

- `cargo test --workspace`: 0 failures across all test suites
- `cargo test -p writ-module`: 11 tests pass including test_debug_local_v4_roundtrip and test_format_version_rejection (patching to v3 is now rejected)
- `cargo test -p writ-compiler --lib`: 26 tests pass including 8 new serialize tests
- `cargo test -p writ-golden`: 34 golden tests pass after BLESS=1 re-bless
- Golden output sample: `fn_typed_params.writil` shows `.locals` section with `r0: int "a"` and `; line:1 col:42` comments

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] writ-module/src/builder.rs had format_version: 3**
- **Found during:** Task 1 verification
- **Issue:** `builder.rs` had a hardcoded `format_version: 3` in `ModuleBuilder::build()` not mentioned in the plan
- **Fix:** Updated to `format_version: 4`
- **Files modified:** `writ-module/src/builder.rs`
- **Commit:** e021941

**2. [Rule 3 - Blocking] Multiple emit_bodies and serialize::serialize call sites needed updating**
- **Found during:** Task 1 compile pass
- **Issue:** `writ-cli/tests/e2e_compile_tests.rs`, `writ-golden/tests/golden_tests.rs`, `writ-compiler/tests/emit_serialize_tests.rs` (4 sites), `writ-compiler/tests/emit_body_tests.rs` all called the old 4-argument `emit_bodies`/`serialize` signatures
- **Fix:** Added `&[]` as the `sources` argument to all call sites
- **Files modified:** `writ-cli/tests/e2e_compile_tests.rs`, `writ-golden/tests/golden_tests.rs`, `writ-compiler/tests/emit_serialize_tests.rs`, `writ-compiler/tests/emit_body_tests.rs`
- **Commit:** e021941

## Self-Check: PASSED

- writ-module/src/module.rs: FOUND (type_ref field present)
- writ-compiler/src/emit/serialize.rs: FOUND (build_line_starts present)
- writ-assembler/src/disassembler.rs: FOUND (.locals section present)
- Commit e021941: FOUND
- Commit 9b38060: FOUND
- cargo test --workspace: all green
