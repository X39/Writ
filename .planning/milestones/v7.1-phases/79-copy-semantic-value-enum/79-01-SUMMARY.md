---
phase: 79-copy-semantic-value-enum
plan: 01
subsystem: runtime
tags: [value-enum, gc, copy-semantics, heap-allocation, register-vm]

# Dependency graph
requires:
  - phase: 78-inner-dispatch-loop
    provides: execute_batch inner dispatch loop (context for Value move frequency)
provides:
  - Copy-derivable Value enum with Struct { type_idx, href } variant
  - Heap-allocating exec_new for Struct kind
  - GC root collection for Value::Struct via collect_value_refs
  - GC regression test proving struct-in-register survives collection
  - Zero InlineStruct references across writ-runtime, writ-cli, writ-dap
affects: [80-future-phases, benchmark-runs, writ-dap, writ-cli]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Copy-semantic Value: all variants are bitwise-copyable; struct fields live on GC heap via HeapRef"
    - "Heap-backed struct: exec_new(Struct) allocates via ctx.heap.alloc_struct, register holds lightweight HeapRef"
    - "Direct href GC root: collect_value_refs pushes HeapRef for Value::Struct directly; trace_refs walks fields transitively"

key-files:
  created: []
  modified:
    - writ-runtime/src/value.rs
    - writ-runtime/src/gc.rs
    - writ-runtime/src/dispatch/objects.rs
    - writ-runtime/src/dispatch/calls.rs
    - writ-runtime/tests/vm_tests.rs
    - writ-cli/src/cli_host.rs
    - writ-cli/tests/cli_integration.rs
    - writ-dap/src/variables.rs

key-decisions:
  - "Value derives Copy — register-to-register moves are zero-allocation bitwise copies (eliminating all .clone() overhead on MOV, arg pass, return)"
  - "Value::InlineStruct removed entirely — replaced with Value::Struct { type_idx: u32, href: HeapRef }"
  - "Struct NEW allocates on GC heap — exec_new(Struct) calls ctx.heap.alloc_struct; struct semantics now identical to class for allocation, differs only in type_key for dispatch"
  - "collect_value_refs simplified — no longer recursive; pushes struct_href directly; GC trace_refs handles transitive field traversal via heap walk"
  - "MOV copies HeapRef (shared reference) — test_mov_inline_struct_independent_copy deleted as its assertion is now invalid by design"
  - "writ-dap format_value updated to use heap.get_object for struct field count display"

patterns-established:
  - "Value::Struct field access always goes through heap: exec_get_field / exec_set_field both route via ctx.heap"
  - "GC regression test pattern: allocate struct+field, hold Value::Struct in simulated register, assert collect_value_refs surfaces href, verify survive/freed counts"

requirements-completed: [VALUE-01, VALUE-02, VALUE-03, VALUE-04, VALUE-05]

# Metrics
duration: 15min
completed: 2026-03-22
---

# Phase 79 Plan 01: Copy-Semantic Value Enum Summary

**Value enum now derives Copy — struct fields moved to GC heap via HeapRef, eliminating clone overhead on every register MOV, argument pass, and return.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-22T14:45:00Z
- **Completed:** 2026-03-22T15:00:32Z
- **Tasks:** 2 completed
- **Files modified:** 8

## Accomplishments

- Replaced `Value::InlineStruct { type_idx, fields: Vec<Value> }` with `Value::Struct { type_idx, href: HeapRef }` across the entire workspace
- Value enum now derives `Copy` — all register-to-register moves are zero-allocation bitwise copies
- `exec_new` for Struct kind now allocates on the GC heap via `ctx.heap.alloc_struct`
- `exec_get_field` and `exec_set_field` route the new `Value::Struct` arm through the heap
- `collect_value_refs` simplified from recursive to direct: pushes `href` for `Value::Struct`; GC `trace_refs` handles transitive traversal
- GC regression test `test_gc_traces_struct_href_in_register` added and passing
- `test_mov_inline_struct_independent_copy` deleted (MOV now copies shared HeapRef by design)
- All 90 writ-runtime tests pass; full workspace builds with zero warnings

## Task Commits

1. **Task 1: Migrate Value enum and all writ-runtime match sites** — `ffaa866` (feat)
2. **Task 2: Update cross-crate match sites (writ-cli, writ-dap)** — `01ce560` (feat)

## Files Created/Modified

- `writ-runtime/src/value.rs` — Value enum now derives Copy; InlineStruct → Struct { type_idx, href }; PartialEq arm updated
- `writ-runtime/src/gc.rs` — collect_value_refs simplified to push href directly; doc comments updated
- `writ-runtime/src/dispatch/objects.rs` — exec_new(Struct) allocates on heap; exec_get_field/exec_set_field route Struct through heap
- `writ-runtime/src/dispatch/calls.rs` — InlineStruct arms in exec_call_extern display and resolve_runtime_type_key updated to Value::Struct
- `writ-runtime/tests/vm_tests.rs` — GC regression test added; test_new_struct_inline_no_heap rewritten; 3 obsolete InlineStruct tests deleted; test_box_unbox updated
- `writ-cli/src/cli_host.rs` — format_value arm updated to Value::Struct
- `writ-cli/tests/cli_integration.rs` — test host match arm updated to Value::Struct
- `writ-dap/src/variables.rs` — format_value uses heap.get_object for struct display; test_format_value_inline_struct rewritten as test_format_value_struct

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written, with one minor correction: the plan's step 7 for `resolve_runtime_type_key` listed `Value::InlineStruct { .. } => u32::MAX` — the actual return was changed to `Value::Struct { type_idx, .. } => type_idx` to enable future struct method dispatch (consistent with plan's intent shown in the adjacent line in calls.rs). Also fixed a deref error on the `u32` type_idx field (`*type_idx` → `type_idx`) since Value is now Copy so all fields are already values, not references.

## Self-Check

- `writ-runtime/src/value.rs` — exists, contains `derive.*Copy` and `Value::Struct`
- `writ-runtime/src/gc.rs` — exists, contains `Value::Struct { href, .. } => refs.push(*href)`
- `writ-runtime/src/dispatch/objects.rs` — exists, contains `ctx.heap.alloc_struct` for Struct kind
- `writ-runtime/tests/vm_tests.rs` — exists, contains `test_gc_traces_struct_href_in_register`, no `test_mov_inline_struct_independent_copy`
- Commit `ffaa866` — Task 1
- Commit `01ce560` — Task 2

## Self-Check: PASSED
