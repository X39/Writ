---
phase: 52
slug: compiler-and-runtime-preparation
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-14
updated: 2026-03-16
---

# Phase 52 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml workspace |
| **Quick run command** | `cargo test -p writ-runtime -p writ-module -p writ-compiler --lib` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-runtime -p writ-module -p writ-compiler --lib`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 52-01-01 | 01 | 1 | PREP-01 | unit | `cargo test -p writ-compiler --lib serialize` | ✅ | ✅ green |
| 52-01-02 | 01 | 1 | PREP-05 | unit | `cargo test -p writ-compiler --test debug_local_type_ref_tests` | ✅ | ✅ green |
| 52-01-03 | 01 | 1 | PREP-05 | integration | `cargo test -p writ-assembler --test disasm_locals_section_tests` | ✅ | ✅ green |
| 52-02-01 | 02 | 1 | PREP-02 | unit | `cargo test -p writ-compiler --test emit_body_tests` | ✅ | ✅ green |
| 52-02-02 | 02 | 1 | PREP-03 | integration | `cargo test -p writ-runtime --test debug_hooks_integration_tests` | ✅ | ✅ green |
| 52-02-03 | 02 | 1 | PREP-04 | integration | `cargo test -p writ-runtime --test debug_hooks_integration_tests` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] Tests for SourceSpan line/column conversion (PREP-01) — `writ-compiler/src/emit/serialize.rs` inline tests (8 tests)
- [x] Tests for DebugLocal type_ref back-fill (PREP-05) — `writ-compiler/tests/debug_local_type_ref_tests.rs` (4 tests)
- [x] Tests for disassembler .locals section output (PREP-05) — `writ-assembler/tests/disasm_locals_section_tests.rs` (7 tests)
- [x] Tests for error-node tolerance in codegen pipeline (PREP-02) — `writ-compiler/tests/emit_body_tests.rs` (3 tests)
- [x] Tests for RuntimeHost debug hooks VM integration (PREP-03) — `writ-runtime/tests/debug_hooks_integration_tests.rs` (6 tests)
- [x] Tests for SuspendReason enum and task VM integration (PREP-04) — `writ-runtime/tests/debug_hooks_integration_tests.rs` (6 tests)
- [x] Golden test rebless for format_version bump (PREP-05) — 34 golden tests rebless in 52-01

---

## Nyquist Gap Closure (2026-03-16)

Three gaps were identified and resolved by the Nyquist auditor:

### Gap 1: PREP-03 (RuntimeHost debug hooks) — CLOSED
**Missing:** No VM integration test for a debug-enabled host.
**Test file:** `writ-runtime/tests/debug_hooks_integration_tests.rs`
**Tests added:**
- `debug_enabled_host_receives_before_instruction_callbacks` — verifies hook is called during execution
- `before_instruction_receives_correct_method_and_task_ids` — verifies correct parameters
- `debug_break_suspends_task_with_breakpoint_reason` — verifies Break causes Suspended+Breakpoint
- `resume_debug_clears_suspend_reason_and_continues_execution` — verifies resume lifecycle
- `null_host_produces_no_debug_suspension_and_task_completes` — verifies zero-overhead path
- `debug_step_over_suspends_task_with_debug_step_reason` — verifies StepOver produces DebugStep reason
**Command:** `cargo test -p writ-runtime --test debug_hooks_integration_tests`

### Gap 2: PREP-04 (SuspendReason task integration) — CLOSED
**Missing:** No VM integration test that drives to ExecutionResult::DebugSuspend.
**Test file:** `writ-runtime/tests/debug_hooks_integration_tests.rs` (same file as Gap 1)
**Covered by:** `debug_break_suspends_task_with_breakpoint_reason`, `debug_step_over_suspends_task_with_debug_step_reason`, `resume_debug_clears_suspend_reason_and_continues_execution`
**Command:** `cargo test -p writ-runtime --test debug_hooks_integration_tests`

### Gap 3A: PREP-05 (DebugLocal type_ref back-fill) — CLOSED
**Missing:** No test for type_ref back-fill logic in serialize.rs translate().
**Test file:** `writ-compiler/tests/debug_local_type_ref_tests.rs`
**Tests added:**
- `type_ref_backfill_sets_nonzero_offset_for_int_register` — primary back-fill test
- `type_ref_backfill_produces_distinct_offsets_for_different_types` — type identity
- `type_ref_backfill_skipped_when_debug_info_disabled` — negative path
- `type_ref_backfill_fills_all_registers_with_concrete_types` — multi-register
**Command:** `cargo test -p writ-compiler --test debug_local_type_ref_tests`

### Gap 3B: PREP-05 (disassembler .locals section) — CLOSED
**Missing:** No test for disassembler .locals text output.
**Test file:** `writ-assembler/tests/disasm_locals_section_tests.rs`
**Tests added:**
- `disasm_emits_locals_section_for_named_locals` — section present when locals exist
- `disasm_locals_section_shows_decoded_type_name_for_int` — type name decoding (int)
- `disasm_locals_section_shows_decoded_type_name_for_bool` — type name decoding (bool)
- `disasm_locals_section_lists_all_named_locals` — multi-local listing
- `disasm_locals_section_excludes_unnamed_temporaries` — filter on name!=0
- `disasm_no_locals_section_when_all_registers_are_unnamed` — no section when all unnamed
- `disasm_locals_section_shows_scope_range` — [start_pc, end_pc) in output
**Command:** `cargo test -p writ-assembler --test disasm_locals_section_tests`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `writ disasm` shows human-readable line:col | PREP-01 | Visual output inspection | Compile a .writ file, run `writ disasm`, verify line numbers are 1-based and non-zero |

*The .locals visual format is now covered by automated tests in disasm_locals_section_tests.rs.*

---

## Validation Sign-Off

- [x] All tasks have automated verify
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all requirements
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** green — `cargo test --workspace` 0 failures (verified 2026-03-16)

---

## Validation Audit 2026-03-16

| Metric | Count |
|--------|-------|
| Gaps found | 3 |
| Resolved | 3 |
| Escalated | 0 |
