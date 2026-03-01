# Plan 18-03 Summary: GC Wired into Runtime

## What was done
1. **MarkSweepHeap already implemented** in gc.rs from Plan 01 Task 1 (precise mark-and-sweep, free-list slot reuse, finalization queue with two-cycle collection).

2. **RuntimeBuilder::with_gc()**: Added `use_gc: bool` flag to RuntimeBuilder. When set, `build()` creates `Box::new(MarkSweepHeap::new())` instead of `Box::new(BumpHeap::new())`.

3. **Runtime::collect_garbage()**: Collects roots from all task registers (all frames in all tasks), global variables, and entity data_refs for alive entities. Calls `heap.collect(roots)`, reports stats via `host.on_gc_complete()`, and drains the finalization queue. Finalizer task scheduling is stubbed with a TODO for Phase 19 (requires on_finalize hook method lookup).

4. **Runtime::collect_roots()**: Private helper that iterates all tasks' call stacks, return values, globals, and entity registry alive entities to build the root set.

5. **Scheduler::schedule_finalizer()**: Helper method that creates a detached task with the given method and self argument. Ready for Phase 19 wiring.

6. **EntityRegistry fix**: `set_data_ref()` and `get_data_ref()` now use `validate_active()` instead of `validate_alive()`, accepting Pending, Alive, and Destroying states. This fixes a bug where SPAWN_ENTITY could not set the data_ref on a Pending entity (the `let _ =` silently discarded the error).

7. **9 GC integration tests** in `writ-runtime/tests/gc_tests.rs`:
   - `gc_collects_unreachable_string`: Verify string freed after register overwrite
   - `gc_preserves_reachable_global`: String in global survives collection
   - `gc_preserves_entity_data_ref`: Alive entity's heap data survives collection
   - `gc_frees_destroyed_entity_data`: Destroyed entity's data is freed
   - `gc_on_gc_complete_callback_fires`: RecordingHost receives callback
   - `gc_stats_accurate_counts`: heap_before, objects_traced, objects_freed, heap_after all correct
   - `gc_with_bump_heap_is_noop`: BumpHeap collect returns zeros, objects persist
   - `gc_empty_heap_collection`: Zero stats on empty heap
   - `gc_multiple_collections_progressive`: Multiple GC cycles work correctly

## Test results
- 69 unit tests (21 entity + 14 GC + 34 existing)
- 9 GC integration tests (new)
- 26 task integration tests (unchanged)
- 72 VM instruction tests (unchanged)
- **176 total, 0 failures, 0 warnings**

## Deviations from plan
- Finalizer task scheduling is deferred to Phase 19 (requires on_finalize hook method lookup which depends on contract dispatch). The `schedule_finalizer()` helper is ready, and the TODO in `collect_garbage()` marks the integration point.
- type_idx on HeapObject::Struct was not added (would be needed for finalizer type lookup in Phase 19). Current implementation drains the finalization queue without scheduling tasks.
- `validate_active()` was added as a new validation method to EntityRegistry to fix the SPAWN_ENTITY data_ref bug. This was a latent bug in Plan 02 that was masked because `set_data_ref` errors were silently discarded with `let _ =`.

## Files modified
- `writ-runtime/src/runtime.rs` (with_gc, collect_garbage, collect_roots)
- `writ-runtime/src/scheduler.rs` (schedule_finalizer)
- `writ-runtime/src/entity.rs` (validate_active, set_data_ref/get_data_ref use validate_active)
- `writ-runtime/tests/gc_tests.rs` (9 new integration tests)
