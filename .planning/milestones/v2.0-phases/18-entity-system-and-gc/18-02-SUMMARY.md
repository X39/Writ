# Plan 18-02 Summary: Dispatch Loop Migration and Entity Instruction Handlers

## What was done
1. **Dispatch loop migration to GcHeap**: All functions in dispatch.rs (`execute_one`, `execute_ret`, `execute_defer_handler`, `execute_crash`) now take `&mut dyn GcHeap` instead of `&mut BumpHeap`. EntityRegistry is passed through as an additional parameter.

2. **Scheduler migration**: Added `entity_registry: EntityRegistry` field to Scheduler. All task execution methods pass the registry through to dispatch functions. `cancel_task_tree` and `run_one_task` use `&mut dyn GcHeap`.

3. **Runtime migration**: `Runtime` struct stores `Box<dyn GcHeap>` instead of `BumpHeap`. `RuntimeBuilder::build()` boxes BumpHeap. `heap()`/`heap_mut()` return trait object references. Added `entity_registry()`/`entity_registry_mut()` accessors.

4. **Entity instruction handlers**:
   - `SPAWN_ENTITY`: Creates pending entity in registry, allocates heap struct, notifies host
   - `INIT_ENTITY`: Commits pending entity, flushes buffered field writes to heap, notifies host
   - `DESTROY_ENTITY`: Two-phase protocol using `EntityState::Destroying` - validates alive, begins destroy, re-executes to complete after hook frame would return, notifies host
   - `ENTITY_IS_ALIVE`: Checks entity registry for Alive state (not just Value::Entity variant)
   - `GET_OR_CREATE`: Checks singleton map, returns existing or creates new and registers singleton

5. **EntityState::Destroying**: New state for two-phase destroy protocol. `begin_destroy()` and `complete_destroy()` methods added to EntityRegistry.

## Test results
- 69 unit tests (21 entity + 14 GC + 34 existing)
- 26 task integration tests (unchanged)
- 72 VM instruction tests (64 existing + 8 new entity tests)
- **167 total, 0 failures, 0 warnings**

## New tests
- `spawn_entity_creates_pending_and_init_commits`
- `entity_is_alive_returns_true_for_alive`
- `entity_is_alive_returns_false_after_destroy`
- `destroy_stale_entity_crashes`
- `get_or_create_singleton_returns_same_entity`
- `entity_is_alive_on_uninitialized_handle_returns_false`
- `spawn_init_two_entities_both_alive`
- `destroy_one_entity_other_survives`

## Deviations from plan
- Hook lookup infrastructure (on_create, on_destroy, on_interact) deferred. The dispatch loop has the two-phase destroy protocol ready for hook frames, but actual method name scanning is not yet implemented. Hooks will be wired when Phase 19 (Contract Dispatch) provides proper method resolution. The entity lifecycle flow is correct without hooks -- entities are created, initialized, and destroyed correctly.
- on_interact dispatch mechanism deferred to Phase 19 (requires CALL_VIRT or dedicated interact instruction).

## Files modified
- `writ-runtime/src/dispatch.rs` (BumpHeap -> dyn GcHeap, entity handlers)
- `writ-runtime/src/scheduler.rs` (entity_registry field, GcHeap signatures)
- `writ-runtime/src/runtime.rs` (Box<dyn GcHeap>, entity_registry accessors)
- `writ-runtime/src/entity.rs` (EntityState::Destroying, begin_destroy/complete_destroy)
- `writ-runtime/tests/vm_tests.rs` (8 new entity tests)
