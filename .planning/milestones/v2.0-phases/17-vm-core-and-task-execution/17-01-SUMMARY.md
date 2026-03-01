---
phase: 17-vm-core-and-task-execution
plan: 01
subsystem: runtime
tags: [vm, register-file, heap, host-trait, module-loader, value-representation]

requires:
  - phase: 16-module-format-foundation
    provides: Instruction enum, Module struct, ModuleBuilder, MethodBody, string/blob heaps
provides:
  - Value enum (Void/Int/Float/Bool/Ref/Entity) with Copy semantics
  - GenHandle<T> type-safe generation-indexed handles (TaskId, EntityId)
  - BumpHeap allocator for strings, structs, arrays, delegates, enums, boxed values
  - CallFrame with method_idx, pc, registers, defer_stack, return_register
  - Task struct with 5-state lifecycle, call stack, parent/child refs, atomic tracking
  - RuntimeHost trait and NullHost auto-confirm implementation for all 9 request types
  - LoadedModule with decoded instruction bodies and branch offset reindexing
  - RuntimeError/CrashInfo/HostError error types
affects: [17-02, 17-03, 18-entity-gc, 19-contract-dispatch]

tech-stack:
  added: [thiserror 2.0]
  patterns: [generation-indexed handles, bump heap allocation, branch offset reindexing at load time]

key-files:
  created:
    - writ-runtime/src/lib.rs
    - writ-runtime/src/value.rs
    - writ-runtime/src/heap.rs
    - writ-runtime/src/frame.rs
    - writ-runtime/src/task.rs
    - writ-runtime/src/host.rs
    - writ-runtime/src/error.rs
    - writ-runtime/src/loader.rs
  modified:
    - writ-runtime/Cargo.toml

key-decisions:
  - "Tag types (TaskTag, EntityTag) derive all standard traits for GenHandle compatibility"
  - "DeferPush method_idx treated as byte offset from method start, converted to instruction index at load time"
  - "BumpHeap stores HeapObject enum variants in flat Vec indexed by HeapRef(u32)"
  - "Value implements Copy since all variants (including Ref and Entity) are just u32/handle copies"

patterns-established:
  - "HeapRef(u32) as opaque index into BumpHeap.objects Vec"
  - "LoadedModule.decoded_bodies parallel to Module.method_bodies for O(1) lookup"
  - "Branch reindexing at load time: byte offsets -> instruction indices in one pass"

requirements-completed: [VM-02, TASK-08]

duration: 3min
completed: 2026-03-02
---

# Phase 17 Plan 01: Foundation Types and Module Loader Summary

**writ-runtime library crate with Value/HeapRef/GenHandle types, BumpHeap allocator, CallFrame/Task structures, RuntimeHost trait with NullHost, and LoadedModule with branch offset reindexing**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-02T00:00:00Z
- **Completed:** 2026-03-02T00:03:00Z
- **Tasks:** 2 (merged into single implementation pass with inline tests)
- **Files modified:** 10

## Accomplishments
- Converted writ-runtime from binary stub to library crate with writ-module dependency
- All 8 core type modules created: value, heap, frame, task, host, error, loader, lib
- BumpHeap round-trips string allocation and struct field read/write
- NullHost implements RuntimeHost returning correct default responses for all 9 request types
- LoadedModule decodes and reindexes branch targets (Br, BrTrue, BrFalse, Switch, DeferPush)
- 34 unit tests passing across all modules

## Task Commits

1. **Task 1+2: Core types, heap, host, loader with tests** - `f517617` (feat)

## Files Created/Modified
- `writ-runtime/Cargo.toml` - Added writ-module and thiserror dependencies
- `writ-runtime/src/lib.rs` - Module declarations and public re-exports
- `writ-runtime/src/value.rs` - Value enum, HeapRef, GenHandle, TaskId, EntityId
- `writ-runtime/src/heap.rs` - BumpHeap with string/struct/array/delegate/enum/boxed allocation
- `writ-runtime/src/frame.rs` - CallFrame with registers, PC, defer stack
- `writ-runtime/src/task.rs` - Task struct with 5-state lifecycle
- `writ-runtime/src/host.rs` - RuntimeHost trait, HostRequest/HostResponse, NullHost
- `writ-runtime/src/error.rs` - RuntimeError, CrashInfo, StackFrame, HostError
- `writ-runtime/src/loader.rs` - LoadedModule with decode_and_reindex

## Decisions Made
- Tag types (TaskTag, EntityTag) derive Debug/Clone/Copy/PartialEq/Eq/Hash for GenHandle compatibility
- Value implements PartialEq manually to handle f64 comparison via to_bits()
- Loader tests merged into module as unit tests (simpler than separate integration test file)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All foundational types ready for Plan 02 dispatch loop implementation
- Value, HeapRef, CallFrame, Task, RuntimeHost all publicly exported
- LoadedModule provides decoded instruction vectors for dispatch

---
*Phase: 17-vm-core-and-task-execution*
*Completed: 2026-03-02*
