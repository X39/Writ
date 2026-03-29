# v7.1 Pre-Optimization Baseline

**Measured:** 2026-03-22
**Build:** cargo build --release (LTO=fat, codegen-units=1, panic=abort)
**Platform:** Windows 11, AMD Ryzen 9 5900X 12-Core Processor
**Commit:** 97618a2

## fib(40) Timing

| Run | Time (s) |
|-----|----------|
| 1   | 78.798s  |
| 2   | 83.297s  |
| 3   | 93.202s  |
| **Median** | **83.297s** |

**Output:** 102334155 (correct)

## Build Configuration at Baseline

- `lto = "fat"` (cross-crate inlining)
- `codegen-units = 1` (maximum optimization)
- `panic = "abort"` (no unwind tables)
- FxHashMap for DispatchTable, Scheduler.tasks, and all writ-runtime hash maps
- `#[inline(always)]` on extract_* helpers
- `#[inline]` on exec_* dispatch functions
- No `#[inline]` on execute_one

## Prior Reference

- Pre-Phase-75 (quick-task 260322-6om): ~141s (default release profile, no LTO, no FxHashMap)
- After quick-task 260322-6om hot-path optimizations (no LTO/FxHashMap): ~28s on a prior measurement
  - Note: quick-task 260322-6om applied instruction cloning elimination + scheduler HashMap lookup bypass
  - Phase 75 Plan 01 adds LTO fat + FxHashMap on top of those optimizations

## Notes

This baseline represents the fully-configured release build BEFORE any algorithmic
optimizations (Phases 76-79). All subsequent phase measurements should compare against
the median time recorded here (83.297s).

The measured times show some variance (78-93s) typical of a Windows environment.
Three runs were taken; the median of 83.297s is the authoritative baseline figure.

---

## Phase 76: Zero-Allocation Call Convention

**Measured:** 2026-03-22
**Build:** cargo build --release (LTO=fat, codegen-units=1, panic=abort)
**Platform:** Windows 11, AMD Ryzen 9 5900X 12-Core Processor
**Commit:** a645e98

### Change Summary

Eliminated intermediate `Vec::with_capacity` staging in all four call instruction handlers:
- `exec_call`: push-then-split_at_mut pattern for direct register-to-register copy
- `exec_call_virt` (Method arm): same push-then-split_at_mut pattern
- `exec_call_indirect`: same push-then-split_at_mut pattern
- `exec_tail_call`: stack-resident `[Value; 32]` buffer (heap fallback only for argc > 32)

For fib(40), which makes ~300M recursive calls, this eliminates ~300M heap allocations in the common case.

### fib(40) Timing

| Run | Time (s) |
|-----|----------|
| 1   | 66.979s  |
| 2   | 67.346s  |
| 3   | 66.509s  |
| **Median** | **66.979s** |

**Output:** 102334155 (correct)

### Delta vs Phase 75 Baseline

| Metric | Value |
|--------|-------|
| Phase 75 baseline | 83.297s |
| Phase 76 median | 66.979s |
| Improvement | -16.318s |
| Percentage improvement | **19.6%** |

---

## Phase 77: Frame Register Pool

**Measured:** 2026-03-22
**Build:** cargo build --release (LTO=fat, codegen-units=1, panic=abort)
**Platform:** Windows 11, AMD Ryzen 9 5900X 12-Core Processor
**Commit:** 190ed2f

### Change Summary

Threaded a free-list `RegisterPool` through the entire execution pipeline:
- `Scheduler` owns `pool: RegisterPool`; `create_task` acquires from pool
- `ExecContext` borrows `pool: &mut RegisterPool` on every instruction dispatch
- `execute_ret` releases the popped frame's register Vec back to the pool
- `execute_crash` releases each unwound frame's register Vec back to the pool
- `exec_call`, `exec_call_virt` (Method arm), `exec_call_indirect` acquire from pool via `CallFrame::with_pool`
- `exec_tail_call` unchanged — it reuses the existing frame in-place via `clear()+resize()`

RegisterPool eliminates per-call Vec allocation for recursive functions. For fib(40) with ~300M
recursive calls, released Vecs are reused immediately on the next call, eliminating allocator
round-trips entirely for the steady-state recursion.

### fib(40) Timing

| Run | Time (s) |
|-----|----------|
| 1   | 63.908s  |
| 2   | 59.800s  |
| 3   | 59.634s  |
| **Median** | **59.800s** |

**Output:** 102334155 (correct)

### Delta vs Phase 76 Baseline

| Metric | Value |
|--------|-------|
| Phase 76 baseline | 66.979s |
| Phase 77 median | 59.800s |
| Improvement | -7.179s |
| Percentage improvement | **10.7%** |
| Cumulative improvement vs Phase 75 | -23.497s (-28.2%) |

---

## Phase 78: Inner Dispatch Loop

**Measured:** 2026-03-22
**Build:** cargo build --release (LTO=fat, codegen-units=1, panic=abort)
**Platform:** Windows 11, AMD Ryzen 9 5900X 12-Core Processor
**Commit:** 3293ff6

### Change Summary

Introduced execute_batch inner dispatch loop that holds &mut Task across multiple
instructions, eliminating the per-instruction FxHashMap task lookup in run_one_task.
Falls back to single-instruction dispatch when debug hooks are enabled.

### fib(40) Timing

| Run | Time (s) |
|-----|----------|
| 1   | 56.447s  |
| 2   | 52.839s  |
| 3   | 53.134s  |
| **Median** | **53.134s** |

**Output:** 102334155 (correct)

### Delta vs Phase 77 Baseline

| Metric | Value |
|--------|-------|
| Phase 77 baseline | 59.800s |
| Phase 78 median | 53.134s |
| Improvement | -6.666s |
| Percentage improvement | **11.1%** |
| Cumulative improvement vs Phase 75 | -30.163s (-36.2%) |

---

## Phase 79: Copy-Semantic Value Enum

**Measured:** 2026-03-22
**Build:** cargo build --release (LTO=fat, codegen-units=1, panic=abort)
**Platform:** Windows 11, AMD Ryzen 9 5900X 12-Core Processor
**Commit:** 4e53552

### Change Summary

Replaced `Value::InlineStruct { type_idx, fields: Vec<Value> }` with `Value::Struct { type_idx, href: HeapRef }`.
Value now derives Copy — all register-to-register moves (MOV, argument passing, return) are zero-allocation
bitwise copies. Struct fields live on the GC heap via HeapRef. Eliminates `.clone()` overhead on every
register operation for struct values.

### fib(40) Timing

| Run | Time (s) |
|-----|----------|
| 1   | 44.873s  |
| 2   | 44.569s  |
| 3   | 45.151s  |
| **Median** | **44.873s** |

**Output:** 102334155 (correct)

### Delta vs Phase 78 Baseline

| Metric | Value |
|--------|-------|
| Phase 78 baseline | 53.134s |
| Phase 79 median | 44.873s |
| Improvement | -8.261s |
| Percentage improvement | **15.5%** |
| Cumulative improvement vs Phase 75 | -38.424s (-46.1%) |

### v7.1 Performance Target Assessment

Target: fib(40) under 30 seconds.
Result: 44.873s median — **target not met** (14.873s above target).

All five v7.1 optimization phases (75-79) are complete. Cumulative improvement: -38.424s (-46.1% vs Phase 75 baseline of 83.297s). The 30s target requires further optimization beyond Phase 79 scope.
