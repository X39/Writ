# Phase 18: Entity System and GC - Research

**Researched:** 2026-03-02
**Domain:** Entity registry with generation-indexed handles, lifecycle hooks, mark-and-sweep GC
**Confidence:** HIGH

## Summary

Phase 18 adds two major subsystems to writ-runtime: (1) an entity registry that manages entity lifecycle through generation-indexed handles with stale-handle detection and lifecycle hooks (on_create, on_destroy, on_interact, on_finalize), and (2) a precise mark-and-sweep garbage collector abstracted behind a `GcHeap` trait that replaces the existing `BumpHeap`.

The existing codebase provides strong foundations. `GenHandle<EntityTag>` (aliased as `EntityId`) already exists in value.rs. The `HostRequest` enum already has `EntitySpawn`, `InitEntity`, `DestroyEntity`, `GetOrCreate`, `FieldRead/Write`, and `GetComponent` variants. The dispatch loop has stub handlers for entity instructions (`SpawnEntity`, `InitEntity`, `DestroyEntity`, `EntityIsAlive`, `GetOrCreate`, `GetComponent`, `FindAll`). The scheduler has `create_task()` for spawning finalizer/hook tasks. The main work is: (a) building an `EntityRegistry` with generation-indexed slot management and construction buffering, (b) defining a `GcHeap` trait and implementing mark-and-sweep, (c) wiring entity lifecycle hooks to fire at correct points, and (d) replacing all `BumpHeap` usage with the trait-based heap.

**Primary recommendation:** Build in three waves: (1) Entity registry + GcHeap trait with BumpHeap adapter, (2) Entity instruction handlers + lifecycle hooks, (3) Mark-and-sweep GC implementation with finalization queue.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **GC Trigger Policy:** Host-configurable GC modes. At minimum: Manual mode where the host explicitly triggers collection. Additional modes at Claude's discretion.
- **GC Statistics & Host Interface:** Dedicated callback on RuntimeHost (`on_gc_complete`) with structured GC stats after each collection. Stats include: objects traced, objects freed, heap size before/after, finalization queue size.
- **GcHeap Trait Surface:** Trait includes allocation methods, collect(roots), object access, AND inspection methods (heap_size, object_count, is_reachable). Swapping implementation requires no VM dispatch loop changes.
- **Entity Destruction Edge Cases:** Self-destroy in on_create is allowed. Double destroy crashes the task. Cascading destruction is synchronous (nested, not queued).
- **Finalization Scheduling:** on_finalize crashes: object still collected, crash logged. Allocations in finalizers allowed. No finalizer budget limit.
- **Struct on_finalize:** Same mechanism as entity on_finalize via GC pathway.
- **Topological Finalization Order:** Objects with finalizers survive the sweep. Their references are kept alive until parent finalized. Children collectible only AFTER parent's finalizer completes. Two-cycle collection for finalizable objects.

### Claude's Discretion
- on_destroy execution model (inline vs separate task)
- BumpHeap retention as GcHeap implementor
- Finalization order among unrelated objects in the same cycle
- Finalizer task priority relative to regular tasks
- Additional GC trigger modes beyond Manual
- Stale handle crash diagnostics detail level

### Deferred Ideas (OUT OF SCOPE)
- None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| ENT-01 | Entity registry uses generation-indexed handles for stale-handle detection | GenHandle<EntityTag> exists; need EntityRegistry with slot array and generation bumping |
| ENT-02 | SPAWN_ENTITY/INIT_ENTITY construction buffering works per spec (batch flush on INIT) | Need PendingEntity struct to buffer field writes between SPAWN and INIT |
| ENT-03 | on_create lifecycle hook fires after INIT_ENTITY completes | After INIT_ENTITY confirms, look up on_create method, schedule as task |
| ENT-04 | on_destroy lifecycle hook fires before host entity removal | On DESTROY_ENTITY: fire on_destroy inline, then notify host |
| ENT-05 | on_interact(who) lifecycle hook dispatches as a new task | Look up on_interact method, spawn as new scheduler task with `who` argument |
| ENT-06 | Singleton entities support GET_OR_CREATE semantics | EntityRegistry tracks singleton type -> entity mapping |
| ENT-07 | ENTITY_IS_ALIVE correctly distinguishes alive/destroyed/collected states | Check entity slot generation against handle generation; destroyed flag |
| ENT-08 | Component field GET/SET suspends for host confirmation | Already implemented via HostRequest dispatch; verify suspend-and-confirm flow |
| GC-01 | Precise mark-and-sweep GC traces from register roots using TypeRef metadata | Walk all task registers, globals, pending entities for Ref values; mark reachable |
| GC-02 | GC heap allocates and manages reference-type objects | GcHeap trait wraps allocation + collection; MarkSweepHeap implements it |
| GC-03 | Finalization queue populated during tracing; on_finalize hooks fire as scheduled tasks | During sweep, move finalizable unreachable objects to finalization queue instead of freeing |
| GC-04 | GC runs at safe points (between ticks when all tasks are quiescent) | Host triggers via runtime.collect_garbage(); only valid when no tasks running |
| GC-05 | trait GcHeap abstraction allows replacing GC implementation without touching VM core | All heap access through trait; dispatch.rs parameterized on trait, not concrete type |
</phase_requirements>

## Architecture Patterns

### Entity Registry Design

```
EntityRegistry {
    slots: Vec<EntitySlot>,      // indexed by EntityId.index
    free_list: Vec<u32>,         // recycled slot indices
    singletons: HashMap<u32, EntityId>,  // type_idx -> entity for GET_OR_CREATE
    pending_entities: HashMap<EntityId, PendingEntity>,  // SPAWN..INIT buffering
}

EntitySlot {
    generation: u32,
    state: EntityState,  // Alive, Destroyed, Free
    type_idx: u32,       // which TypeDef this entity is
    data_ref: HeapRef,   // heap-allocated struct holding entity fields
}

PendingEntity {
    entity_id: EntityId,
    type_idx: u32,
    field_writes: Vec<(u32, Value)>,  // buffered field writes
}
```

**Construction flow:** `SPAWN_ENTITY` allocates a slot, creates a `PendingEntity` entry. Field writes during construction are buffered. `INIT_ENTITY` flushes buffered fields to the heap object, transitions entity to Alive, fires `on_create`, then notifies host.

### GcHeap Trait Design

```rust
pub trait GcHeap {
    fn alloc_string(&mut self, s: &str) -> HeapRef;
    fn alloc_struct(&mut self, field_count: usize) -> HeapRef;
    fn alloc_array(&mut self, elem_type: u32) -> HeapRef;
    fn alloc_delegate(&mut self, method_idx: usize, target: Option<Value>) -> HeapRef;
    fn alloc_enum(&mut self, type_idx: u32, tag: u16, fields: Vec<Value>) -> HeapRef;
    fn alloc_boxed(&mut self, val: Value) -> HeapRef;

    fn read_string(&self, href: HeapRef) -> Result<&str, RuntimeError>;
    fn get_field(&self, href: HeapRef, idx: usize) -> Result<Value, RuntimeError>;
    fn set_field(&mut self, href: HeapRef, idx: usize, val: Value) -> Result<(), RuntimeError>;
    fn get_object(&self, href: HeapRef) -> Result<&HeapObject, RuntimeError>;
    fn get_object_mut(&mut self, href: HeapRef) -> Result<&mut HeapObject, RuntimeError>;

    // GC operations
    fn collect(&mut self, roots: &[HeapRef]) -> GcStats;

    // Inspection
    fn heap_size(&self) -> usize;
    fn object_count(&self) -> usize;
    fn is_reachable(&self, href: HeapRef) -> bool;
}
```

### Mark-and-Sweep Implementation

```
MarkSweepHeap {
    objects: Vec<Option<HeapObject>>,  // None = free slot
    marks: Vec<bool>,                  // parallel mark bits
    free_list: Vec<u32>,               // recycled slots
    has_finalizer: Vec<bool>,          // per-object finalizer flag
    finalization_queue: Vec<HeapRef>,   // objects pending finalization
}
```

**Collection algorithm:**
1. Clear all mark bits
2. For each root ref, recursively mark reachable objects
3. Sweep: for each unmarked object:
   - If has finalizer: move to finalization_queue (keep alive for one more cycle)
   - If no finalizer: free (set slot to None, add to free_list)
4. Return GcStats

### Root Discovery

Roots come from:
- All task register files (walk all tasks, all frames, all registers)
- Global variables
- Pending entity field buffers
- Entity registry data_refs (for alive entities)
- Finalization queue (objects awaiting finalization keep their references alive)

### Lifecycle Hook Dispatch

| Hook | Trigger | Execution Model |
|------|---------|-----------------|
| on_create | After INIT_ENTITY completes | Inline in same task (synchronous) |
| on_destroy | Before host DestroyEntity notification | Inline in caller's task (synchronous, per decision: cascading destruction is synchronous) |
| on_interact(who) | Explicit instruction | New independent task via scheduler.create_task() |
| on_finalize | After GC sweep for unreachable finalizable objects | Scheduled as new tasks, fire after sweep completes |

### Anti-Patterns to Avoid
- **Putting GC logic in dispatch.rs:** All GC operations belong in the heap implementation. dispatch.rs should only call trait methods.
- **Running finalizer IL during collection sweep:** Spec explicitly forbids this. Finalizers fire as separate tasks AFTER sweep.
- **Freeing finalizable objects in one cycle:** Must survive one extra cycle per the topological finalization decision.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Generation-indexed handles | Custom unsafe arena | Existing GenHandle<T> in value.rs | Already correctly implemented |
| Host request dispatch | Custom protocol | Existing HostRequest/HostResponse pattern | Already tested and working |
| Task scheduling for hooks | Custom hook executor | Existing Scheduler::create_task() | Hooks are just tasks |

## Common Pitfalls

### Pitfall 1: Stale HeapRef After Collection
**What goes wrong:** GC frees an object, making its HeapRef index point to None or a new object.
**Why it happens:** HeapRef is a simple u32 index; unlike EntityId, it has no generation counter.
**How to avoid:** MarkSweepHeap uses `Vec<Option<HeapObject>>` with free-list recycling. Accessing a freed slot returns an error. Since GC only runs at safe points (no task is mid-execution), there should be no dangling refs if the mark phase is correct.
**Warning signs:** Tests that access heap objects after GC should verify they still exist.

### Pitfall 2: BumpHeap Removal Breaking All Tests
**What goes wrong:** Replacing BumpHeap with GcHeap trait changes every function signature in dispatch.rs.
**Why it happens:** BumpHeap is passed by concrete type throughout.
**How to avoid:** Make dispatch.rs generic over `GcHeap` trait. Keep BumpHeap as a GcHeap implementor (simple adapter) so all 64 existing tests continue to work unchanged. Then add MarkSweepHeap as the production implementation.
**Warning signs:** If you break existing tests in Wave 1, the refactor is too aggressive.

### Pitfall 3: Finalization Queue Keeping Entire Object Graphs Alive
**What goes wrong:** Objects in the finalization queue keep all their references alive, which keeps those references' references alive, etc.
**Why it happens:** The topological finalization decision requires this. But it means one finalizable object can prevent collection of a large subgraph.
**How to avoid:** This is by design (per CONTEXT.md decisions). Just ensure the two-cycle behavior is clearly tested. In practice, games have few finalizable objects.

### Pitfall 4: Entity Registry and Heap Getting Out of Sync
**What goes wrong:** Entity destroyed but its heap object not freed, or heap object freed but entity still marked alive.
**Why it happens:** Entity lifecycle and GC lifecycle are two separate systems.
**How to avoid:** When an entity is destroyed, mark its slot as Destroyed but leave the heap object to be collected by GC. The entity's data_ref becomes unreachable once no registers reference it.

### Pitfall 5: on_destroy Cascading While In Atomic Section
**What goes wrong:** Cascading destruction (entity A's on_destroy destroys entity B) could run arbitrary IL during what should be an atomic operation.
**Why it happens:** Synchronous cascading destruction means nested calls to on_destroy handlers.
**How to avoid:** on_destroy runs inline in the caller's task. If the caller is in an atomic section, the destruction (including cascading) happens atomically. This is actually correct behavior -- the atomic section prevents other tasks from interleaving.

## Code Examples

### GcHeap Trait Implementation Pattern

```rust
// BumpHeap implements GcHeap as a no-collection adapter
impl GcHeap for BumpHeap {
    fn collect(&mut self, _roots: &[HeapRef]) -> GcStats {
        GcStats::default() // no-op: bump heap never collects
    }
    // ... delegate all alloc/read methods to existing implementations
}
```

### Root Collection Pattern

```rust
fn collect_roots(
    scheduler: &Scheduler,
    entity_registry: &EntityRegistry,
) -> Vec<HeapRef> {
    let mut roots = Vec::new();

    // Task registers
    for task in scheduler.tasks.values() {
        for frame in &task.call_stack {
            for reg in &frame.registers {
                if let Value::Ref(href) = reg {
                    roots.push(*href);
                }
            }
        }
    }

    // Globals
    for global in &scheduler.globals {
        if let Value::Ref(href) = global {
            roots.push(*href);
        }
    }

    // Entity data refs
    for slot in &entity_registry.slots {
        if slot.state == EntityState::Alive {
            roots.push(slot.data_ref);
        }
    }

    roots
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| BumpHeap (never frees) | GcHeap trait + MarkSweepHeap | Phase 18 | Enables memory reclamation |
| Entity stubs (always alive) | EntityRegistry with generation checks | Phase 18 | Enables stale-handle detection |
| No lifecycle hooks | Hook dispatch via scheduler tasks | Phase 18 | Enables game-engine entity patterns |

## Open Questions

1. **Method index lookup for lifecycle hooks**
   - What we know: Lifecycle hooks (on_create, on_destroy, etc.) need to find the method index for a given entity type's hook
   - What's unclear: How to look up "on_create for TypeDef X" from the module metadata
   - Recommendation: Use a lookup table built at module load time mapping (type_idx, hook_kind) -> method_idx. Scan MethodDef names during LoadedModule construction.

2. **Object tracing depth for mark phase**
   - What we know: HeapObjects contain Values, which may contain Refs to other HeapObjects
   - What's unclear: Whether recursive marking needs a stack limit
   - Recommendation: Use an explicit work-stack (not recursion) for the mark phase to avoid stack overflow on deep object graphs.

## Sources

### Primary (HIGH confidence)
- Existing writ-runtime source code: value.rs, heap.rs, host.rs, dispatch.rs, scheduler.rs, task.rs, runtime.rs
- Phase 18 CONTEXT.md decisions
- REQUIREMENTS.md (ENT-01 through ENT-08, GC-01 through GC-05)

### Secondary (MEDIUM confidence)
- .NET GC finalization model (inspiration for topological finalization design)
- Generation-indexed handle pattern (common in ECS/game engine literature)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - building on existing Rust codebase with clear patterns
- Architecture: HIGH - all key decisions locked in CONTEXT.md, codebase well-understood
- Pitfalls: HIGH - identified from codebase analysis, known GC implementation patterns

**Research date:** 2026-03-02
**Valid until:** 2026-04-02 (stable -- internal codebase, no external dependencies)
