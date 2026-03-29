---
phase: quick
plan: 260322-6om
subsystem: writ-runtime
tags: [performance, vm, dispatch, hot-path]
dependency_graph:
  requires: []
  provides: [optimized-vm-dispatch]
  affects: [writ-runtime, benchmark]
tech_stack:
  added: []
  patterns: [borrow-based-dispatch, zero-clone-hot-path]
key_files:
  created: []
  modified:
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/dispatch/arith.rs
    - writ-runtime/src/scheduler.rs
    - benchmark/cases/fib/fib.writc
decisions:
  - Borrow instruction reference instead of cloning — eliminates per-instruction enum clone with Vec heap allocation for Switch variant
  - exec_switch takes &[i32] instead of Vec<i32> to match the borrow-based approach
  - byte_pc lookup moved inside debug guard — saves a Vec index per instruction in non-debug mode
  - Scheduler limit check gated on limit > 0 — saves a HashMap lookup per instruction in unlimited mode (common case)
  - Did not use unsafe raw pointer to task — safer approach achieved similar gains
metrics:
  duration: ~45 minutes
  completed: "2026-03-22T04:19:00Z"
  tasks: 3
  files_modified: 4
---

# Quick Task 260322-6om: VM Performance Optimization for fib Benchmark

Optimized the Writ VM hot path to achieve ~5x speedup on compute-intensive recursive workloads. The fib(40) benchmark now runs in ~141 seconds vs. the projected ~750 second baseline.

## What Was Done

### Root Cause
The VM was 50-75x slower than Lua for compute-intensive recursive workloads (fib). Root cause: on every single instruction dispatch, the VM was:
1. Cloning the entire `Instruction` enum (which includes a `Switch { offsets: Vec<i32> }` variant — heap allocation per instruction)
2. Doing a Vec lookup for `byte_pc` even in non-debug mode
3. Doing a HashMap lookup in the scheduler limit check even when limit=0 (unlimited)

### Optimization 1: Eliminate instruction cloning (Task 1)

Changed `execute_one` in `writ-runtime/src/dispatch/mod.rs` from:
```rust
let instr = body[pc].clone();   // heap alloc on every instruction
frame.pc += 1;
```
To:
```rust
frame.pc = pc + 1;
// frame borrow ends via Rust NLL
let instr = &modules[current_module_idx].decoded_bodies[method_idx][pc]; // zero-cost borrow
```

All 91 instruction match arms changed from value-based pattern matching to reference-based (`*r_dst`, `*r_a`, etc.). The `Switch` variant now passes `offsets` as `&[i32]` to `exec_switch` (signature updated in arith.rs).

Also moved `byte_pc` computation inside `if host.debug_enabled()` block — saves a Vec index lookup per instruction in non-debug (production) mode.

**Result:** fib(35) 23s → 18s (1.3x), fib(40) ~750s → 181s (4.1x)

### Optimization 2: Hot-path arithmetic already optimal (Task 2)

Audited `writ-runtime/src/dispatch/arith.rs` — all arithmetic handlers already use `helpers::extract_int(&frame.registers[...])` (borrow-based, returns Copy `i64`). No changes needed.

### Optimization 3: Scheduler HashMap lookup (Task 2, deviation)

In `scheduler.rs::run_one_task`, the per-iteration limit check was:
```rust
let task = self.tasks.get(&task_id).unwrap();  // HashMap lookup
if task.atomic_depth == 0 && limit > 0 && instructions_run >= limit { ... }
```

Changed to gate on `limit > 0` first:
```rust
if limit > 0 {
    let task = self.tasks.get(&task_id).unwrap();  // skipped when limit==0
    ...
}
```

When the CLI runs `writ run` with no step limit (`ExecutionLimit::None → limit=0`), the entire limit check (including the HashMap lookup) is skipped on every instruction. This is the common production case.

**Result:** fib(35) 18s → 12s (additional 1.5x), fib(40) 181s → 141s

## Performance Results

| Benchmark | Baseline | After | Speedup |
|-----------|----------|-------|---------|
| fib(35)   | 23s      | 12s   | 1.9x    |
| fib(40)   | ~750s    | 141s  | ~5.3x   |

Correctness verified:
- fib(10) = 55
- fib(30) = 832040
- fib(35) = 9227465
- fib(40) = 102334155

## Remaining Bottleneck

fib(40) at 141s is still above the 120s target from the plan. The remaining overhead is dominated by:

1. **Per-instruction HashMap lookup** — `self.tasks.get_mut(&task_id)` before every `execute_one` call. Eliminating this safely requires a structural change (unsafe raw pointer or moving the task out of the HashMap temporarily).

2. **Recursive call overhead** — `exec_call` clones all arguments into a Vec for the new frame. For fib this happens on every call. Avoiding this would require a register-sharing or stack-pointer approach rather than per-frame Vec allocation.

3. **Match dispatch on 91 variants** — Rust should compile this to a jump table, but branch prediction on deep recursion may still hurt.

Further speedup options (not implemented):
- Computed goto / jump table with function pointers
- NaN-boxing Value representation (pack all scalars into 64 bits, no enum overhead)
- JIT compilation

## Deviations from Plan

### Auto-added: Scheduler limit-check optimization
- **Found during:** Task 2 review of all per-instruction overhead
- **Issue:** Two HashMap lookups per instruction (one for limit check, one for execute_one)
- **Fix:** Gated the limit check on `limit > 0`, eliminating one HashMap lookup per instruction in unlimited mode
- **Files modified:** `writ-runtime/src/scheduler.rs`
- **Commit:** a34525e

### Task 2 note: arith.rs already optimal
The plan anticipated needing to change arith.rs to borrow-based patterns. These were already in place (using `helpers::extract_int(&frame.registers[...])`) — no changes were needed.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1    | 131ba98 | Eliminate instruction cloning in VM hot path |
| 2    | a34525e | Skip HashMap lookup in unlimited-run mode |
| 3    | 74ec631 | Validate performance and update precompiled binary |

## Self-Check: PASSED

Files verified:
- writ-runtime/src/dispatch/mod.rs: modified (borrow-based dispatch)
- writ-runtime/src/dispatch/arith.rs: modified (exec_switch takes &[i32])
- writ-runtime/src/scheduler.rs: modified (limit > 0 guard)
- benchmark/cases/fib/fib.writc: created (recompiled)

Commits verified in git log:
- 131ba98 present
- a34525e present
- 74ec631 present
