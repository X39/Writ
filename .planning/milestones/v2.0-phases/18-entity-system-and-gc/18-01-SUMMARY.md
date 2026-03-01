# Plan 18-01 Summary: GcHeap Trait, BumpHeap Adapter, EntityRegistry

## What was done
1. **GcHeap trait** (`gc.rs`): Defined trait abstraction with 16 methods covering allocation (6), access (5), collection (1), finalization (2), and inspection (2). BumpHeap and MarkSweepHeap both implement this trait, enabling GC swap without dispatch loop changes (GC-05).

2. **GcStats struct** (`gc.rs`): Structured statistics with `objects_traced`, `objects_freed`, `heap_before`, `heap_after`, `finalization_queue_size` fields.

3. **MarkSweepHeap** (`gc.rs`): Full mark-and-sweep GC with free-list slot reuse, two-phase finalization (unmarked finalizable objects survive one extra cycle with their references kept alive via topological finalization), and explicit work-stack tracing.

4. **BumpHeap adapter** (`heap.rs`): `impl GcHeap for BumpHeap` delegates all methods to existing inherent methods. `collect()` returns `GcStats::default()` (no-op). All 64 existing tests pass unchanged.

5. **EntityRegistry** (`entity.rs`): Generation-indexed entity slots with:
   - `allocate()` / `begin_spawn()` / `commit_init()` lifecycle
   - Stale-handle detection via generation counter comparison
   - `destroy()` with generation bump and free-list recycling
   - Singleton entity map (`register_singleton` / `get_singleton`)
   - PendingEntity buffering for field writes between SPAWN and INIT
   - `alive_entities()` iterator for GC root collection

6. **RuntimeHost callback** (`host.rs`): Added `on_gc_complete(&mut self, _stats: &GcStats)` with default no-op implementation.

## Test results
- 69 unit tests (21 entity + 14 GC + 34 existing)
- 26 task integration tests (unchanged)
- 64 VM instruction tests (unchanged)
- **159 total, 0 failures, 0 warnings**

## Deviations from plan
- MarkSweepHeap was included in gc.rs during Plan 01 instead of being deferred to Plan 03. This simplifies Plan 03 to focus solely on wiring GC into Runtime with root collection and finalization scheduling.

## Files modified
- `writ-runtime/src/gc.rs` (new)
- `writ-runtime/src/entity.rs` (new)
- `writ-runtime/src/heap.rs` (added `impl GcHeap for BumpHeap`)
- `writ-runtime/src/host.rs` (added `on_gc_complete`)
- `writ-runtime/src/lib.rs` (added `gc`, `entity` modules and exports)
