---
phase: 116
plan: "03"
status: complete
started: 2026-03-29
completed: 2026-03-29
---

# Plan 116-03 Summary: Host Value Construction API

## What Was Built

- `Runtime::construct_value(type_name, fields)` — public method that looks up types by name, validates field count and types, allocates on heap
- `ExternHandler::ImmediateWithHeap` — new handler variant providing `&mut dyn GcHeap` access
- `ExternRegistry::on_with_heap()` — builder method for registering heap-aware handlers
- `RuntimeHost::on_extern_call_with_heap()` — default trait method (backward compatible)
- Dispatch loop in `exec_call_extern` tries heap-aware dispatch before `on_request`

## Key Files

- `writ-runtime/src/runtime.rs` — `construct_value` method
- `writ-runtime/src/extern_registry.rs` — `ImmediateWithHeap` variant + `on_with_heap`
- `writ-runtime/src/host.rs` — `on_extern_call_with_heap` trait method
- `writ-runtime/src/dispatch/calls.rs` — heap-aware dispatch in `exec_call_extern`

## Test Results

- All writ-runtime tests pass
- Type validation rejects wrong field count and mismatched types
- Enum types correctly rejected

## Deviations

- `alloc_struct` takes `(type_key, field_count)` not just `field_count` — adjusted call
- `set_field` returns Result — added `let _ =` wrapper
