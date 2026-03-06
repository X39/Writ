---
phase: 49-vm-runtime
plan: "02"
subsystem: vm-runtime
tags: [vm, inline-struct, gc, value-types, kind-dispatch]
dependency_graph:
  requires: [49-01]
  provides: [inline-struct-vm-semantics, gc-inline-struct-tracing]
  affects: [writ-compiler, vm-dispatch]
tech_stack:
  added: []
  patterns: [kind-dispatch, collect_value_refs recursive helper, clone-then-match borrow pattern]
key_files:
  created: []
  modified:
    - writ-runtime/src/dispatch/objects.rs
    - writ-runtime/src/gc.rs
    - writ-runtime/src/runtime.rs
    - writ-runtime/tests/vm_tests.rs
decisions:
  - "Entity (kind=2) kept in heap-allocation path for NEW -- entities are reference types constructed via NEW, crashing would break existing entity tests"
  - "collect_value_refs made pub (not pub(crate)) to allow direct testing from vm_tests.rs integration tests"
  - "exec_set_field Ref arm: clone-then-match-with-let-_-to-drop pattern avoids drop(frame) on reference warning"
metrics:
  duration: "~15min"
  completed_date: "2026-03-12"
  tasks: 2
  files_modified: 4
---

# Phase 49 Plan 02: InlineStruct VM Dispatch and GC Summary

**One-liner:** Kind-dependent NEW dispatch creating InlineStruct for structs vs Ref for classes, GET/SET_FIELD variant dispatch, and recursive GC tracing of InlineStruct fields via collect_value_refs.

## What Was Built

### Task 1: NEW kind-dispatch and GET/SET_FIELD variant dispatch

Modified `exec_new` in `writ-runtime/src/dispatch/objects.rs` to dispatch by `TypeDefKind`:
- `Struct` (kind=0): creates `Value::InlineStruct { type_idx, fields: vec![Void; N] }` in the destination register — no heap allocation
- `Class` (kind=4) and `Entity` (kind=2): allocates on heap via `alloc_struct`, stores `Value::Ref(href)` — existing behavior preserved
- `Enum` (kind=1) and `Component` (kind=3): crash with `ExecutionResult::Crash` containing descriptive message

Modified `exec_get_field` to dispatch by value variant:
- `Value::InlineStruct`: reads `fields[idx]` directly, clones into destination register
- `Value::Ref` / `Value::Entity`: existing heap path via `ctx.heap.get_field()`
- Other: crash with "expected struct or class"

Modified `exec_set_field` to dispatch by value variant:
- `Value::InlineStruct`: mutates `fields[idx]` in place in the register
- `Value::Ref`: copies `href` (Copy type), uses `let _ = frame` to end borrow, then calls `ctx.heap.set_field()`
- Other: crash

### Task 2: GC root collection and trace_refs for InlineStruct

Added `pub fn collect_value_refs(val: &Value, refs: &mut Vec<HeapRef>)` to `gc.rs`:
- Recursively collects `HeapRef`s from any `Value`, including nested `InlineStruct` fields
- Handles `Value::Ref` (push), `Value::InlineStruct` (recurse into fields), all others (skip)

Updated `trace_refs` in `gc.rs` to use `collect_value_refs` for all `HeapObject` variants (Struct, Array, Delegate, Enum, Boxed) — previously only matched `Value::Ref` directly, missing `InlineStruct`-embedded refs.

Updated `collect_roots` in `runtime.rs` to use `collect_value_refs` for all register and global scans — previously only matched `if let Value::Ref(href) = reg`, missing `InlineStruct` registers holding refs.

## Tests Added (vm_tests.rs)

| Test | VM req | What it verifies |
|---|---|---|
| `test_new_struct_inline_no_heap` | VM-01 | NEW on Struct creates InlineStruct, heap size unchanged |
| `test_new_class_heap_alloc` | VM-03/06 | NEW on Class creates Ref, heap size increases by 1 |
| `test_new_enum_kind_crashes` | VM-03 | NEW on Enum returns Crash with descriptive message |
| `test_get_set_field_inline_struct` | — | GET/SET_FIELD on InlineStruct reads/writes fields directly |
| `test_get_set_field_class_ref` | VM-06 | GET/SET_FIELD on Ref (class) works through heap (regression) |
| `test_mov_inline_struct_independent_copy` | VM-02 | MOV of InlineStruct is deep copy; mutation of copy leaves original unchanged |
| `test_box_unbox_inline_struct` | VM-05 | BOX stores InlineStruct on heap; UNBOX recovers field values |
| `test_gc_traces_inline_struct_ref_fields` | VM-04 | InlineStruct in register keeps embedded Ref alive during GC |
| `test_gc_traces_nested_inline_struct_refs` | VM-04 | Nested InlineStruct (struct field = struct) traces deep Ref |
| `test_gc_traces_boxed_inline_struct` | VM-04 | Boxed(InlineStruct) keeps inner Ref alive during GC |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Test Regression] Updated `new_allocates_struct` test expectation**
- **Found during:** Task 1
- **Issue:** Pre-existing test `new_allocates_struct` expected `NEW` on a `Struct` typedef to return `Value::Ref`. After the kind-dispatch change, `NEW` on `Struct` correctly returns `Value::InlineStruct`. The test was asserting old, incorrect behavior.
- **Fix:** Updated assertion to `Value::InlineStruct { type_idx: 1, .. }` to match the now-correct semantics.
- **Files modified:** `writ-runtime/tests/vm_tests.rs`
- **Commit:** caf1d7b

**2. [Rule 2 - API visibility] Made `collect_value_refs` pub instead of pub(crate)**
- **Found during:** Task 2 test compilation
- **Issue:** Integration tests in `vm_tests.rs` access `writ_runtime::gc::collect_value_refs` directly to verify the recursive ref-collection logic. `pub(crate)` prevents external test access.
- **Fix:** Changed `pub(crate) fn collect_value_refs` to `pub fn collect_value_refs`.
- **Files modified:** `writ-runtime/src/gc.rs`
- **Commit:** 88aa2c0

## Self-Check: PASSED

- writ-runtime/src/dispatch/objects.rs: FOUND
- writ-runtime/src/gc.rs: FOUND
- writ-runtime/src/runtime.rs: FOUND
- .planning/phases/49-vm-runtime/49-02-SUMMARY.md: FOUND
- Commit caf1d7b: FOUND
- Commit 88aa2c0: FOUND
