---
phase: 56
slug: dap-advanced-inspection
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-14
validated: 2026-03-16
---

# Phase 56 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`cargo test`) |
| **Config file** | `writ-dap/Cargo.toml` (standard workspace member) |
| **Quick run command** | `cargo test -p writ-dap && cargo test -p writ-runtime` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-dap && cargo test -p writ-runtime`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 56-01-01 | 01 | 1 | DAP-04 | unit | `cargo test -p writ-runtime --lib -- test_frame_registers` | ✅ writ-runtime/src/runtime.rs (3 tests) | ✅ green |
| 56-01-02 | 01 | 1 | DAP-07 | unit | `cargo test -p writ-runtime --lib -- test_all_task_ids` | ✅ writ-runtime/src/runtime.rs (5 tests) | ✅ green |
| 56-01-03 | 01 | 1 | DAP-04 | unit | `cargo test -p writ-dap --lib -- test_format_value` | ✅ writ-dap/src/variables.rs (11 tests) | ✅ green |
| 56-01-04 | 01 | 1 | DAP-04 | unit | `cargo test -p writ-dap --lib -- test_decode_type_blob` | ✅ writ-dap/src/variables.rs (11 tests) | ✅ green |
| 56-01-05 | 01 | 1 | DAP-04 | unit | `cargo test -p writ-dap --lib -- test_variables_ref` | ✅ writ-dap/src/variables.rs (4 tests) | ✅ green |
| 56-02-01 | 02 | 1 | DAP-04 | unit | `cargo test -p writ-dap --lib -- test_scopes_handler` | ✅ writ-dap/src/server.rs | ✅ green |
| 56-02-02 | 02 | 1 | DAP-04 | unit | `cargo test -p writ-dap --lib -- test_variables_handler` | ✅ writ-dap/src/server.rs | ✅ green |
| 56-02-03 | 02 | 1 | DAP-06 | unit | `cargo test -p writ-dap --lib -- test_evaluate_local_name` | ✅ writ-dap/src/server.rs | ✅ green |
| 56-02-04 | 02 | 1 | DAP-06 | unit | `cargo test -p writ-dap --lib -- test_evaluate_unknown` | ✅ writ-dap/src/server.rs | ✅ green |
| 56-02-05 | 02 | 1 | DAP-07 | unit | `cargo test -p writ-dap --lib -- test_threads_multi_task` | ✅ writ-dap/src/server.rs | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `writ-dap/src/variables.rs` — format_value, decode_type_blob, make/unpack_variables_ref with unit tests
- [x] `writ-runtime/src/runtime.rs` — add frame_registers and all_task_ids accessors with tests

*Existing test infrastructure covers framework needs — no new installs required.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Variables panel shows named locals in VS Code | DAP-04 | End-to-end VS Code UI | Set breakpoint in .writ file, pause, check Variables panel |
| Watch expression shows value | DAP-06 | End-to-end VS Code UI | Add variable name to Watch panel while paused |
| Threads panel shows multiple tasks | DAP-07 | Requires multi-task .writ program | Run program with `spawn`, check Threads panel |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated

---

## Validation Audit 2026-03-16

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

**Test inventory:** 39 automated tests covering all 10 tasks across 3 source files:
- `writ-runtime/src/runtime.rs`: 8 tests (frame_registers: 3, all_task_ids: 5)
- `writ-dap/src/variables.rs`: 25 tests (format_value: 11, decode_type_blob: 11, variables_ref: 4)
- `writ-dap/src/server.rs`: 6 tests (decode_frame_id: 1, threads_multi_task: 1, scopes_handler: 1, variables_handler: 1, evaluate_local_name: 1, evaluate_unknown: 1)

All tests green. Full workspace passes (124 + 53 + 11 tests, 0 failures).
