---
phase: 49-vm-runtime
plan: "01"
subsystem: writ-runtime
tags: [value-types, inline-struct, copy-removal, clone, vm]
dependency_graph:
  requires: []
  provides: [Value::InlineStruct variant, Clone-only Value enum]
  affects: [writ-runtime, writ-cli]
tech_stack:
  added: []
  patterns: [Clone-only enum with Vec fields, mechanical .clone() at register reads]
key_files:
  created: []
  modified:
    - writ-runtime/src/value.rs
    - writ-runtime/src/heap.rs
    - writ-runtime/src/gc.rs
    - writ-runtime/src/runtime.rs
    - writ-runtime/src/dispatch/arith.rs
    - writ-runtime/src/dispatch/objects.rs
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/dispatch/calls.rs
    - writ-runtime/src/dispatch/intrinsics.rs
    - writ-runtime/src/dispatch/concurrency.rs
    - writ-runtime/src/scheduler.rs
    - writ-cli/src/cli_host.rs
decisions:
  - "InlineStruct { type_idx: u32, fields: Vec<Value> } added after Entity variant"
  - "Copy derive removed from Value; all register reads use .clone() for explicit multi-word copy semantics"
  - "writ-cli cli_host.rs InlineStruct arm added to format_value to keep workspace building"
metrics:
  duration: "~15 min"
  completed: "2026-03-12T21:41:39Z"
  tasks_completed: 2
  files_modified: 12
---

# Phase 49 Plan 01: Value InlineStruct Variant and Copy Removal Summary

**One-liner:** Added `Value::InlineStruct { type_idx, fields }` variant, removed `Copy` derive, and mechanically fixed all 35 compilation errors across writ-runtime and writ-cli with `.clone()` at every register read site.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Add InlineStruct variant and remove Copy from Value | abf290a | writ-runtime/src/value.rs |
| 2 | Fix all Copy-removal compilation errors across writ-runtime | 72f9c3e | 11 files (heap, gc, runtime, dispatch/*, scheduler, cli_host) |

## What Was Built

### Task 1: Value::InlineStruct variant

`writ-runtime/src/value.rs`:
- Removed `Copy` from `#[derive(Debug, Clone, Copy)]`
- Added `InlineStruct { type_idx: u32, fields: Vec<Value> }` after `Entity(EntityId)` variant
- Updated doc comment: "Clone-only. InlineStruct holds fields inline (value-type struct semantics)."
- Added `PartialEq` arm: `(InlineStruct { type_idx: a, fields: fa }, InlineStruct { type_idx: b, fields: fb }) => a == b && fa == fb`

### Task 2: Compilation fix categories

**A. `.copied()` to `.cloned()` (4 sites):**
- `heap.rs` lines 101, 110: `fields.get(idx).copied()` in BumpHeap::get_field (Struct and Enum)
- `gc.rs` lines 214, 221: same pattern in MarkSweepHeap::get_field

**B. Register reads (30+ sites across dispatch files):**
- `dispatch/arith.rs`: exec_mov, exec_convert, exec_box register reads; exec_unbox `*val` deref
- `dispatch/objects.rs`: all `frame.registers[idx]` reads in set_field, array_init/store/add/insert/load, wrap_some/ok/err, unwrap/unwrap_ok/extract_err, new_enum, extract_field
- `dispatch/calls.rs`: arg collection in exec_call, exec_call_virt, exec_call_extern, exec_call_indirect, exec_tail_call; new_delegate target capture; call_virt obj_val; Boxed inner deref
- `dispatch/intrinsics.rs`: StringIntoString, ArrayIndex, ArrayIndexSet, ArrayIterable
- `dispatch/concurrency.rs`: spawn arg loops, LoadGlobal, StoreGlobal
- `dispatch/mod.rs`: Ret ret_val; return_value assignment before Completed

**C. Option<Value> fields (scheduler, runtime):**
- `runtime.rs`: register_value `.copied()` to `.cloned()`; HostResponse::Value `*val` to `.clone()`; two `t.return_value` reads to `.clone()`
- `scheduler.rs`: JoinTask target_info `.return_value` to `.clone()`; wake_joiners `return_value.clone()`; Completed val used twice

**D. Non-exhaustive patterns:**
- `dispatch/calls.rs` display_args match: added `Value::InlineStruct { type_idx, .. }` arm
- `dispatch/calls.rs` resolve_runtime_type_key: added `Value::InlineStruct { .. }` arm
- `writ-cli/src/cli_host.rs` format_value: added `Value::InlineStruct { type_idx, .. }` arm

## Verification

```
cargo test -p writ-runtime  →  78 passed; 0 failed
cargo build --workspace     →  0 errors
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing coverage] writ-cli cli_host.rs non-exhaustive InlineStruct arm**
- **Found during:** Task 2 (workspace build verification)
- **Issue:** `format_value` in writ-cli had a non-exhaustive match on Value that did not cover `InlineStruct`
- **Fix:** Added `Value::InlineStruct { type_idx, .. } => format!("<struct@{}>", type_idx)` arm
- **Files modified:** `writ-cli/src/cli_host.rs`
- **Commit:** 72f9c3e

## Self-Check: PASSED

- `writ-runtime/src/value.rs` — FOUND, contains InlineStruct variant, no Copy derive
- `writ-runtime/src/dispatch/arith.rs` — FOUND, contains `.clone()` at exec_mov
- `writ-runtime/src/heap.rs` — FOUND, contains `.cloned()` in get_field
- `writ-runtime/src/gc.rs` — FOUND, contains `.cloned()` in get_field
- Commit abf290a — FOUND (Task 1)
- Commit 72f9c3e — FOUND (Task 2)
- 78 tests pass, 0 failures
- Full workspace build: 0 errors
