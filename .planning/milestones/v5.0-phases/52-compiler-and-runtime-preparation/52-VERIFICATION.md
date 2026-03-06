---
phase: 52-compiler-and-runtime-preparation
verified: 2026-03-14T00:00:00Z
status: passed
score: 16/16 must-haves verified
gaps: []
---

# Phase 52: Compiler and Runtime Preparation Verification Report

**Phase Goal:** The compiler and runtime have correct source position data and debug hooks so all subsequent LSP and DAP features are built on reliable foundations.
**Verified:** 2026-03-14
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | DebugLocal struct has a type_ref field pointing into the blob heap | VERIFIED | `writ-module/src/module.rs` line 38: `pub type_ref: u32, // blob heap offset` |
| 2 | Module format version is 4 (bumped from 3) | VERIFIED | `module.rs` `Module::new()` sets `format_version: 4`; reader at `reader.rs:57` checks `!= 4`; `builder.rs` also sets `format_version: 4` |
| 3 | DebugLocal round-trips correctly through writer/reader at 18 bytes per entry | VERIFIED | `writer.rs:240` comment: `18 bytes each`; writes `type_ref` at line 213; reader reads it at line 207; `test_debug_local_v4_roundtrip` test asserts type_ref survives round-trip |
| 4 | build_source_spans produces real 1-based line and column numbers from byte offsets | VERIFIED | `serialize.rs:533-553`: `build_source_spans` calls `byte_offset_to_line_col(span.start, line_starts)` which uses `partition_point` for O(log n) 1-based conversion; 7 unit tests in serialize.rs cover this |
| 5 | writ disasm output shows .locals section with register names and types | VERIFIED | `disassembler.rs:499-519`: `.locals { ... }` block emitted with `r{}: type "name" [start, end)` per named local |
| 6 | writ disasm output shows source location comments on instructions with SourceSpan entries | VERIFIED | `disassembler.rs:558-574`: `; line:N col:M` appended when `source_spans[span_cursor].pc == byte_offset` |
| 7 | RuntimeHost trait has debug_enabled(), before_instruction(), on_function_enter(), on_function_exit() methods with default no-op implementations | VERIFIED | `host.rs:122-140`: all four methods present with defaults; `debug_enabled` returns `false`; `before_instruction` returns `DebugAction::Continue` |
| 8 | NullHost and CliHost compile without implementing any debug methods | VERIFIED | `host.rs:149-167`: NullHost only implements `on_request` and `on_log`; no debug methods overridden |
| 9 | DebugAction enum has Continue, Break, StepOver, StepInto, StepOut, Disconnect variants | VERIFIED | `host.rs:6-19`: all 6 variants present |
| 10 | SuspendReason enum distinguishes HostRequest, Breakpoint, and DebugStep suspensions | VERIFIED | `task.rs:18-26`: `SuspendReason` with 3 variants including source location fields |
| 11 | Task struct has suspend_reason: Option<SuspendReason> field | VERIFIED | `task.rs:42`: `pub suspend_reason: Option<SuspendReason>`; initialized to `None` at line 67 |
| 12 | VM calls before_instruction only when debug_enabled() returns true | VERIFIED | `dispatch/mod.rs:244-276`: `if host.debug_enabled() { ... }` guards the entire debug hook block |
| 13 | Host-request suspensions set SuspendReason::HostRequest on the task | VERIFIED | `scheduler.rs:133`: `task.suspend_reason = Some(SuspendReason::HostRequest(req_id))` |
| 14 | Breakpoint/step suspensions set the appropriate SuspendReason variant with source location | VERIFIED | `dispatch/mod.rs:250-268`: `SuspendReason::Breakpoint` and `SuspendReason::DebugStep` set with method_idx, pc, line, col |
| 15 | A file with a syntax error in function A still produces compiled bodies for function B | VERIFIED | `emit/body/mod.rs:392-404`: `if expr_has_error(body) { ... continue; }` skips broken function and continues loop |
| 16 | A diagnostic is emitted for each skipped function body | VERIFIED | E9001 diagnostic emitted per Fn (line 400), per impl method (line 450), per Const (line 496), per Global (line 561) |

**Score:** 16/16 truths verified

---

## Required Artifacts

### Plan 52-01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-module/src/module.rs` | DebugLocal with type_ref: u32 field, format_version 4 | VERIFIED | Lines 35-41: `type_ref: u32` between `name` and `start_pc`; `Module::new()` sets `format_version: 4` |
| `writ-compiler/src/emit/serialize.rs` | build_line_starts + byte_offset_to_line_col + updated build_source_spans | VERIFIED | Lines 509-553: all three functions present and wired together; 7 unit tests at lines 556-618 |
| `writ-assembler/src/disassembler.rs` | .locals section and inline source location comments | VERIFIED | Lines 499-519 (.locals), 558-574 (source comments) |

### Plan 52-02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-runtime/src/host.rs` | DebugAction enum + debug methods on RuntimeHost trait | VERIFIED | Lines 6-19 (DebugAction), 122-140 (debug methods with defaults) |
| `writ-runtime/src/task.rs` | SuspendReason enum + suspend_reason field on Task | VERIFIED | Lines 18-26 (SuspendReason), 42 (field), 67 (init to None) |
| `writ-runtime/src/dispatch/mod.rs` | Debug hook call site in execute_one with source location lookup | VERIFIED | Lines 185-202 (lookup_source_location helper), 244-276 (debug hook in execute_one) |

### Plan 52-03 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/emit/body/mod.rs` | Per-function error node detection and skip logic in emit_all_bodies | VERIFIED | Lines 391-404: `expr_has_error(body)` check with `continue`; E9001 diagnostic at line 400 |
| `writ-compiler/src/emit/mod.rs` | Removed file-level has_error_nodes pre-pass | VERIFIED | No E9000 pre-pass present; line 116: only aborts when `bodies.is_empty() && !diags.is_empty()` |

---

## Key Link Verification

### Plan 52-01 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-cli/src/main.rs` | `writ-compiler/src/emit/mod.rs` | sources parameter passed to emit_bodies | VERIFIED | `main.rs:391-396`: constructs `sources` vec then passes `&sources` to `emit_bodies` |
| `writ-compiler/src/emit/serialize.rs` | `writ-module/src/module.rs` | DebugLocal construction with type_ref field | VERIFIED | `serialize.rs:494-501`: DebugLocal constructed with `type_ref: 0` initially, back-filled at lines 354-358 |
| `writ-compiler/src/emit/serialize.rs` | `writ-module/src/module.rs` | SourceSpan construction with real line/col from line_starts table | VERIFIED | `serialize.rs:543-550`: `byte_offset_to_line_col(span.start, line_starts)` called in build_source_spans |

### Plan 52-02 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-runtime/src/dispatch/mod.rs` | `writ-runtime/src/host.rs` | host.debug_enabled() guard + host.before_instruction() call | VERIFIED | `dispatch/mod.rs:244-246`: `if host.debug_enabled() { ... host.before_instruction(...) }` |
| `writ-runtime/src/dispatch/mod.rs` | `writ-runtime/src/task.rs` | task.suspend_reason = Some(SuspendReason::Breakpoint { .. }) | VERIFIED | `dispatch/mod.rs:250-255`: exact pattern present |
| `writ-runtime/src/scheduler.rs` | `writ-runtime/src/task.rs` | Sets SuspendReason::HostRequest on existing suspension path | VERIFIED | `scheduler.rs:133`: `task.suspend_reason = Some(SuspendReason::HostRequest(req_id))` |

### Plan 52-03 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-compiler/src/emit/mod.rs` | `writ-compiler/src/emit/body/mod.rs` | emit_all_bodies now handles per-function error skipping internally | VERIFIED | `emit/mod.rs:109`: calls `body::emit_all_bodies(...)` which handles errors internally via `continue` |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| PREP-01 | 52-01 | Compiled .writil debug info contains real line/column numbers | SATISFIED | `build_line_starts` + `byte_offset_to_line_col` in serialize.rs; source text threaded from CLI through emit_bodies; `emit_stmt` pushes SourceSpan per statement |
| PREP-02 | 52-03 | Parser produces useful partial ASTs from incomplete or syntactically broken source files | SATISFIED | Per-function error skip in `emit_all_bodies` with E9001; `emit_bodies` returns Ok with partial results when at least one body compiles |
| PREP-03 | 52-02 | RuntimeHost trait has a before_instruction hook for debug breakpoint/stepping control | SATISFIED | `debug_enabled()`, `before_instruction()`, `on_function_enter()`, `on_function_exit()` on RuntimeHost with default no-ops |
| PREP-04 | 52-02 | Task distinguishes DAP debug suspension from host-request suspension via a SuspendReason discriminant | SATISFIED | `SuspendReason` enum with HostRequest/Breakpoint/DebugStep variants; `Task.suspend_reason` field; set at suspension site, cleared at resume |
| PREP-05 | 52-01 | Compiled .writil includes debug local variable info (register index → variable name + type) per function | SATISFIED | `DebugLocal.type_ref` field added at format version 4; back-filled from register_types blob offsets in serialize.rs; disassembler shows `.locals` section |

All 5 requirements for Phase 52 are satisfied. No orphaned requirements: REQUIREMENTS.md traceability table maps only PREP-01 through PREP-05 to Phase 52, and all five are claimed across the three plans.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `writ-runtime/src/runtime.rs` | 475 | TODO comment: `// TODO: For each HeapRef in finalization_queue...` | Info | Pre-existing TODO unrelated to Phase 52 goals; finalizer scheduling is out-of-scope for this phase |
| `writ-compiler/src/emit/body/mod.rs` | 186 | `#[allow(dead_code)]` on `has_error_nodes` | Info | Intentional retention per SUMMARY decisions: retained as utility for tests, not a gate |

No blocker or warning anti-patterns found. Both noted items are benign.

---

## Human Verification Required

The following behaviors require a running binary to verify fully, but all automated proxy checks pass:

### 1. Golden test disassembly output

**Test:** Run `writ build --debug` on a multi-function .writ file then `writ disasm` the output
**Expected:** `.locals` block with type-annotated register names appears before instructions; `; line:N col:M` comments appear on each statement-boundary instruction
**Why human:** Requires actual file compilation and binary execution; golden tests cover format correctness but not readability of live output

### 2. Partial compilation with broken function

**Test:** Create a .writ file with `fn broken() { ??? }` and `fn good() -> int { 42 }`, compile with `writ build`
**Expected:** Diagnostic for broken(), binary still contains good()'s body, no crash
**Why human:** Integration of full CLI pipeline including diagnostics rendering; unit tests cover the per-function skip logic but not the full CLI path

---

## Verification Summary

All 16 must-haves across three plans are fully verified at all three levels (exists, substantive, wired):

**Plan 52-01 (PREP-01, PREP-05):** DebugLocal carries `type_ref` at 18 bytes per entry, format version 4, `build_line_starts` and `byte_offset_to_line_col` implemented with 7 unit tests, source text threaded from CLI through to `build_source_spans`, `emit_stmt` pushes one SourceSpan per statement, disassembler emits `.locals` section and `; line:col` comments.

**Plan 52-02 (PREP-03, PREP-04):** `RuntimeHost` trait extended with four debug methods (all default no-ops), `DebugAction` enum with 6 variants, `SuspendReason` enum with 3 variants, `Task.suspend_reason` field initialized to None and set at every suspension site, `execute_one` calls `before_instruction` only under `debug_enabled()` guard, `on_function_enter`/`on_function_exit` wired into all frame push/pop sites, `SuspendReason::HostRequest` set in scheduler, `DebugSuspend` handled, `resume_debug()` clears suspend_reason.

**Plan 52-03 (PREP-02):** File-level `has_error_nodes` abort removed from both `emit_bodies` and `emit_all_bodies`; per-declaration `expr_has_error` check with `continue` added for Fn, impl method, Const, and Global; E9001 diagnostic emitted per skip; `emit_bodies` returns Ok with partial results when at least one body compiles; three TDD tests verify the behavior.

All commits (e021941, 9b38060, ae670a1, 558d60e, 9d721df, 3bdf961) are present in the repository.

---

_Verified: 2026-03-14_
_Verifier: Claude (gsd-verifier)_
