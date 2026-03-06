---
phase: quick-260320-h4s
plan: 01
subsystem: runtime, lsp
tags: [runtime, lsp, crash, stacktrace, diagnostics, error-reporting]
dependency_graph:
  requires: []
  provides: [writ_runtime::CrashInfo::format_stacktrace, writ_runtime::StackFrame::line, writ_runtime::StackFrame::column, lsp::R0001-runtime-crash-diagnostic]
  affects: [writ-runtime, writ-lsp, writ-dap]
tech_stack:
  added: [writ-module (as writ-lsp dep), writ-runtime (as writ-lsp dep)]
  patterns: [runtime-level-crash-enrichment, consumer-pattern-for-crash-info]
key_files:
  created: []
  modified:
    - writ-runtime/src/error.rs
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/lib.rs
    - writ-lsp/Cargo.toml
    - writ-lsp/src/analysis_host.rs
decisions:
  - "CrashInfo enrichment happens at crash time in execute_crash (not at query time), so all consumers get rich data without re-implementing resolution"
  - "LSP uses catch_unwind around runtime execution to prevent panics from crashing the server"
  - "ExecutionLimit::Instructions(100_000) safety cap; ExecutionLimitReached treated as no-crash (game loop)"
  - "StackFrame is re-exported from writ-runtime public API so any consumer (DAP, CLI) can use it"
metrics:
  duration: "~10 minutes"
  completed: "2026-03-20"
  tasks: 3
  files: 5
---

# Quick Task 260320-h4s: Runtime Crash Stacktrace for LSP Summary

**One-liner:** Runtime-level CrashInfo enrichment with method_name and source locations at crash time, consumed by LSP as R0001 diagnostics with full stack trace.

## What Was Done

When a Writ script crashes at runtime (force-unwrap on None, division by zero, etc.), the runtime now produces rich `CrashInfo` with resolved method names and source locations. Any consumer of `writ-runtime` (LSP, DAP, CLI) can call `crash_info.format_stacktrace()` to get a human-readable string. The LSP analysis pipeline now runs the script after successful typecheck and surfaces crashes as `R0001` diagnostics in the editor.

## Tasks Completed

### Task 1: Enrich StackFrame and CrashInfo (commit 3ed8d31)

**Files:** `writ-runtime/src/error.rs`, `writ-runtime/src/dispatch/mod.rs`, `writ-runtime/src/lib.rs`

- Added `line: u32` and `column: u32` fields to `StackFrame` (1-based, 0 = unknown)
- Added `CrashInfo::format_stacktrace()` method with format: `Runtime crash: <msg>\n\nStack trace:\n  at <method> (line N, col M)`
- Updated `execute_crash` in `dispatch/mod.rs` to:
  - Resolve `method_name` from the module's string heap via `writ_module::heap::read_string`
  - Convert instruction-index `pc` to byte-offset via `byte_offsets[method_idx][pc]`
  - Find source location by scanning `source_spans` for largest `span.pc <= byte_pc`
  - Populate `line` and `column` from the matched span
- Re-exported `StackFrame` from `writ-runtime/src/lib.rs`
- Added 4 unit tests inline in `error.rs` covering all format_stacktrace cases

### Task 2: Wire LSP to consume enriched CrashInfo (commit b388299)

**Files:** `writ-lsp/Cargo.toml`, `writ-lsp/src/analysis_host.rs`

- Added `writ-module` and `writ-runtime` as dependencies
- Added Stage 5+6 to `analyze_standalone` and `analyze_project`:
  - Guard: only runs when no compile errors exist
  - Calls `writ_compiler::emit_bodies(typed_ast, interner, asts, true, sources)`
  - Parses module bytes: `writ_module::Module::from_bytes`
  - Finds `main` entry point by scanning method_defs
  - Runs with `ExecutionLimit::Instructions(100_000)` safety cap
  - Reports crash as `R0001` diagnostic using `crash.format_stacktrace()`
- Added `try_runtime_diagnostic()` free function (Stage 5+6 implementation)
- Added `line_col_to_offset()` helper for converting 1-based line/col to byte offset
- All runtime execution wrapped in `catch_unwind` to prevent server crashes

### Task 3: Tests (commit f42e5d1)

**Files:** `writ-lsp/src/analysis_host.rs` (runtime tests already added inline in error.rs in Task 1)

LSP-level tests (all using 16MB thread stack to avoid stack overflow from emit pipeline):

- `test_runtime_crash_force_unwrap_shows_stacktrace`: R0001 with "Runtime crash" and "main" in message
- `test_runtime_crash_nested_call_shows_full_stacktrace`: full call chain with correct ordering (crash_here before main)
- `test_no_runtime_diagnostic_when_compile_errors`: type error blocks runtime execution
- `test_no_runtime_diagnostic_clean_script`: no false positives
- `test_no_runtime_diagnostic_no_main`: no execution without main entry point
- `test_runtime_crash_primary_span_points_to_crash_site`: primary span is non-zero

## Verification Results

| Check | Status |
|-------|--------|
| `cargo build -p writ-runtime` | PASS |
| `cargo build -p writ-lsp` | PASS |
| `cargo test -p writ-runtime --lib` (129 tests) | PASS |
| `cargo test -p writ-lsp` (115 tests) | PASS |
| `cargo test -p writ-dap` (7 tests) | PASS |
| format_stacktrace unit tests (4 tests) | PASS |
| LSP runtime_crash tests (3 tests) | PASS |
| LSP no_runtime_diagnostic tests (3 tests) | PASS |

## Deviations from Plan

None - plan executed exactly as written.

The DAP `types::StackFrame` (lsp-types crate) was not the runtime's `error::StackFrame`, so no DAP source changes were needed for compatibility.

## Self-Check: PASSED

- `writ-runtime/src/error.rs` - exists with StackFrame.line, StackFrame.column, CrashInfo::format_stacktrace()
- `writ-runtime/src/dispatch/mod.rs` - execute_crash populates method_name and source location
- `writ-runtime/src/lib.rs` - StackFrame re-exported
- `writ-lsp/Cargo.toml` - writ-module and writ-runtime dependencies added
- `writ-lsp/src/analysis_host.rs` - try_runtime_diagnostic + line_col_to_offset + Stage 5+6 in both analyze functions
- Commits 3ed8d31, b388299, f42e5d1 all exist in git history
