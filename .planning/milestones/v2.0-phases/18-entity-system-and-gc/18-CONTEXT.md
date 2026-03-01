# Phase 18: Entity System and GC - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Entities are created, destroyed, and queried through generation-indexed handles with no stale-handle aliasing. Lifecycle hooks fire at correct points. A precise mark-and-sweep GC collects unreachable objects at safe points without running finalizer IL inside the collection sweep. The GC heap is abstracted behind a trait for swappable implementations.

</domain>

<decisions>
## Implementation Decisions

### GC Trigger Policy
- Host-configurable GC modes. At minimum: **Manual mode** where the host explicitly triggers collection
- Additional modes (e.g., between-ticks automatic, memory-pressure threshold) are at Claude's discretion if easy to add
- GC does NOT run autonomously by default — host controls timing

### GC Statistics & Host Interface
- Dedicated callback on RuntimeHost (e.g., `on_gc_complete`) with structured GC stats after each collection
- Stats should include: objects traced, objects freed, heap size before/after, finalization queue size
- This is a new method on the RuntimeHost trait, not routed through on_log

### GcHeap Trait Surface
- Trait includes allocation methods, collect(roots), object access, AND inspection methods
- Inspection: heap_size(), object_count(), is_reachable(ref) or similar
- Swapping GC implementation requires changing only the trait implementor — no VM dispatch loop changes (success criterion 5)

### BumpHeap Retention
- Claude's discretion whether to keep BumpHeap as a GcHeap implementor (for tests/no-GC scenarios) or replace entirely

### Entity Destruction Edge Cases
- **Self-destroy in on_create:** Allowed. `Entity.destroy(self)` inside `on create` is legal. Entity ends up immediately dead after construction. Useful for conditional spawning
- **Double destroy:** Crashes the task. Calling `Entity.destroy()` on an already-dead handle follows the spec's "dead handle = crash" rule
- **Cascading destruction:** Synchronous chain. When on_destroy calls `Entity.destroy()` on other entities, the inner destruction runs immediately as a nested call, not queued
- **on_destroy execution model:** Claude's discretion whether on_destroy runs inline in the caller's task or spawns a separate task

### Finalization Scheduling
- on_finalize crashes: Object is still collected. Crash is logged. Finalizer failure cannot prevent collection (matches spec: "on_finalize resurrection is UB")
- Allocations in finalizers: Allowed. Finalizers can allocate new heap objects normally. If this triggers another GC, it runs with the being-finalized objects already swept
- Finalizer budget: No budget limit — finalizers run to completion once started
- Struct on_finalize: Same mechanism as entity on_finalize. Both types support finalization via the same GC pathway. Only difference: structs never have on_destroy
- Finalizer priority (before vs interleaved with regular tasks): Claude's discretion

### Topological Finalization Order
- **Key decision:** Objects with finalizers survive the sweep. Their references are kept alive until the parent is finalized
- Children only become collectible AFTER the parent's finalizer completes
- This means: unreachable objects with finalizers are placed in a finalization queue (not freed). Objects reachable from the finalization queue are also kept alive. After the finalizer runs, the parent and its formerly-held references are eligible for collection in the NEXT GC cycle
- This prevents the invalid-state problem: finalizers always see valid referenced objects because those references haven't been collected yet
- Finalization order among peers (unrelated objects): Claude's discretion

### Claude's Discretion
- on_destroy execution model (inline vs separate task)
- BumpHeap retention as GcHeap implementor
- Finalization order among unrelated objects in the same cycle
- Finalizer task priority relative to regular tasks
- Additional GC trigger modes beyond Manual
- Stale handle crash diagnostics detail level

</decisions>

<specifics>
## Specific Ideas

- GC modes should be host-configurable, with Manual as the baseline — this mirrors how game engines want explicit control over frame-sensitive operations
- Topological finalization is inspired by .NET's model where finalizable objects survive one extra GC cycle to ensure valid references during finalization
- The dedicated `on_gc_complete` callback gives hosts structured data for performance monitoring dashboards

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `GenHandle<T>` (value.rs): Type-safe generation-indexed handles already implemented. `EntityId = GenHandle<EntityTag>` ready to use
- `Value::Entity(EntityId)` variant: Entity handles already representable as runtime values
- `HeapObject` enum (heap.rs): Already has String, Struct, Array, Delegate, Enum, Boxed variants — needs Entity variant or separate registry
- `HostRequest` enum (host.rs): Already has EntitySpawn, InitEntity, DestroyEntity, GetOrCreate, FieldRead/Write, GetComponent variants
- `RuntimeHost` trait (host.rs): on_request/on_log methods — needs on_gc_complete addition
- `NullHost` (host.rs): Test helper that auto-confirms all requests — needs GC callback stub

### Established Patterns
- Dispatch loop (`dispatch.rs`): `execute_one()` handles instructions and returns `ExecutionResult` enum. Entity/GC instructions will follow this pattern
- Scheduler (`scheduler.rs`): Task lifecycle management with ready queue. Finalizer tasks can use `create_task()` to schedule
- Task state machine (task.rs): Ready/Running/Suspended/Completed/Cancelled states with atomic depth tracking
- Host request/response model: Runtime emits `HostRequest`, host returns `HostResponse`. Entity operations already use this pattern

### Integration Points
- `BumpHeap` → replaced/wrapped by `GcHeap` trait. All heap access in dispatch.rs goes through the trait
- `Scheduler::create_task()` → used to schedule finalizer tasks after GC sweep
- `execute_one()` → needs handlers for SPAWN_ENTITY, INIT_ENTITY, DESTROY_ENTITY, ENTITY_IS_ALIVE, GET_OR_CREATE, GET_COMPONENT instructions
- `RuntimeHost` trait → needs `on_gc_complete()` method with GC stats struct
- `lib.rs` exports → new entity registry module, GcHeap trait, GC stats types

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 18-entity-system-and-gc*
*Context gathered: 2026-03-02*
