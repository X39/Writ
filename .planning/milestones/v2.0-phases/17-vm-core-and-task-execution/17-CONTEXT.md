# Phase 17: VM Core and Task Execution - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the VM dispatch loop (91 instructions), typed register file, managed call frame stack, cooperative task scheduler (5-state lifecycle), suspend-and-confirm protocol, defer/crash unwinding, atomic section enforcement, and RuntimeHost trait. Single-module execution only. Entity system, GC, and contract dispatch are separate phases (18, 19).

</domain>

<decisions>
## Implementation Decisions

### Host Embedding API
- **Request-response with IDs**: Runtime emits `HostRequest` enum, task suspends, host calls `runtime.confirm(request_id, HostResponse)` to resume
- **Single `HostRequest` enum**: One enum with variants for all 9 transition points (ExternCall, EntitySpawn, FieldRead, FieldWrite, GetComponent, InitEntity, DestroyEntity, GetOrCreate, Join). Each request carries the requesting `task_id`
- **Typed `HostResponse` enum**: `Value(Value)`, `EntityHandle(EntityId)`, `Confirmed`, `Error(HostError)`. Runtime validates response matches request kind
- **`RuntimeHost` trait**: Single trait with `fn on_request(&mut self, id: RequestId, req: &HostRequest) -> HostResponse` and `fn on_log(&mut self, level: LogLevel, message: &str)`. One trait per game engine implementation
- **NullHost auto-confirms immediately**: Receives request, returns default response (0 for values, no-op for spawns, stdout for say). Tasks never actually suspend in tests
- **`Runtime` type name**: Top-level type is `Runtime` (not Vm or VmState)
- **Builder pattern**: `RuntimeBuilder::new(module).with_host(host).with_limit(limit).build()` for construction
- **`writ-runtime` library crate**: Convert existing stub from binary (main.rs) to library (lib.rs). Depends on `writ-module` crate. Phase 21 creates `writ-runner` for CLI
- **Owns the Module**: `Runtime` takes ownership of `Module`. No borrows, no Arc
- **Public inspection methods**: `runtime.task_state(task_id)`, `runtime.register_value(task_id, reg)`, `runtime.call_depth(task_id)` for test assertions
- **Single module in Phase 17**: One module loaded and executed. Multi-module resolution deferred to Phase 19
- **Entity instructions stub to host**: SPAWN_ENTITY, INIT_ENTITY, DESTROY_ENTITY dispatched as HostRequests. No entity registry yet. Phase 18 adds the real registry
- **CALL_VIRT dispatches to CRASH**: Contract dispatch not available in Phase 17. Instruction handled in loop but crashes with "contract dispatch not available". Phase 19 adds dispatch tables
- **Decode instructions on load**: All MethodBody code bytes decoded into `Vec<Instruction>` at module load time. Dispatch loop operates on decoded instructions

### Value Representation
- **Rust enum**: `enum Value { Void, Int(i64), Float(f64), Bool(bool), Ref(HeapRef), Entity(EntityId) }`
- **HeapRef is opaque handle**: `pub struct HeapRef(u32)` — index into heap's object table. GC can relocate objects without invalidating references
- **Strings are heap objects**: Referenced by `HeapRef`. Consistent with spec's reference-type semantics
- **Simple bump allocator for pre-GC**: Objects allocated but never collected in Phase 17. GC trait boundary introduced but only a no-collect impl. Phase 18 adds real GC

### Handle System
- **Shared `GenHandle<Tag>`**: Generic `pub struct GenHandle<T> { index: u32, generation: u32, _phantom: PhantomData<T> }`. `TaskId = GenHandle<TaskTag>`, `EntityId = GenHandle<EntityTag>` in Phase 18. One implementation, type-safe

### Scheduling Policy
- **Multi-thread-ready, per-task execution**: Host can call `runtime.run_task(task_id)` from different threads concurrently. Runtime uses internal locking for shared state. Runtime does NOT own a thread pool — host drives threading
- **Discrete ticks with delta_time**: `tick(delta_time: f64, limit: ExecutionLimit)`. Host calls once per game frame. Runtime processes Ready tasks within budget
- **FIFO creation order, round-robin rotation**: Ready tasks scheduled in creation order. When a task hits execution limit, it goes to back of queue. No task can starve others
- **Execution limits per tick/run_task**: Accepts instruction count OR time-based limit. No-limit option also supported. Atomic sections exempt from limits per spec
- **Runtime-enforced scoped task tree**: Parent completion/cancellation automatically cancels scoped children recursively. Defer handlers fire at each level. Host doesn't manage the tree
- **Detached tasks promoted to root level**: SPAWN_DETACHED creates root-level tasks with no parent reference. Survive parent lifecycle
- **Basic per-task counters**: Each task tracks `instructions_executed`, `suspend_count`, `spawn_time`. Accessible via public inspection methods
- **Synchronous call_sync() for render-time scripting**: Host can call a single method to completion without interruption. Ignores execution limits. For render-time scripting where a method must finish in one shot

### Atomic Sections
- **Per-global locking**: Locks acquired on first access within atomic section, held until ATOMIC_END. Locks released as a batch. Other tasks block only if they access a locked global
- **No execution-limit suspension**: Tasks inside atomic sections run to ATOMIC_END regardless of limits per spec

### Crash Diagnostics
- **Full stack trace**: Crash message + full call stack with method name and PC at each frame
- **Delivered through both channels**: Crash details logged via `RuntimeHost::on_log` at error level AND returned in `TickResult` (e.g., `TasksCancelled` with crash info)
- **Secondary defer crashes**: Logged at error level with mini stack trace, unwinding continues. Original crash is the primary report

### Tick Result
- **`TickResult` enum**: `AllCompleted`, `TasksSuspended(Vec<PendingRequest>)`, `ExecutionLimitReached`. Host knows exactly what to do next

### Claude's Discretion
- PC representation (instruction index vs byte offset) — pick what works best for the decoded-on-load approach
- Defer stack representation (instruction offsets vs other)
- JOIN synchronization for multi-threaded per-task model
- tick() vs step() API granularity
- Synchronous call_sync() implementation (temporary task vs direct execution)
- Register values in crash traces vs method+PC only
- Exact LogLevel enum variants beyond the spec's required four

</decisions>

<specifics>
## Specific Ideas

- "The runtime has to be able to call one method to completion without interruption — for render-time scripting of things"
- "Multi-threaded in mind. Thread pool may not be managed by runtime. Reference implementation should be 'ready for use' with a game, which will have its own thread pooling"
- "A task may act in a hostile way. That must be prevented by using a round-robin approach, appending worked tasks to the back of the queue again"
- "Per-global locking during the whole atomic session. Lock must be acquired on-request basis and released when the atomic session ends"
- "Tick should accept an instruction or time limit. A time limit would be better, as long as clock access is fast enough (e.g., to achieve a max script engine runtime of 10ms). No limit must also be accepted"

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-module::Instruction` enum: All 91 opcodes with field operands — direct input to dispatch loop
- `writ-module::Module` struct: Complete in-memory module with 21 tables, string/blob heaps, method bodies
- `writ-module::ModuleBuilder`: Programmatic module construction for test authoring
- `writ-module::MethodBody`: Contains `register_types`, raw `code` bytes, debug locals, source spans
- `writ-module::MetadataToken`: 1-based token newtype for table lookups
- `writ-module::tables::*`: All 21 table row structs (TypeDefRow, MethodDefRow, FieldDefRow, etc.)

### Established Patterns
- Workspace-based Rust project with independent crates (writ-parser, writ-compiler, writ-runtime, writ-module)
- `writ-module` uses byteorder for binary encoding, derives Debug/Clone on data types
- Tests use `#[cfg(test)] mod tests` pattern within crate, plus integration tests in `tests/` directory
- No external logging framework in use yet — Phase 17 will introduce LogLevel
- `writ-runtime` currently a binary stub (main.rs) — must be converted to library (lib.rs)

### Integration Points
- `writ-runtime` depends on `writ-module` via Cargo.toml workspace dependency
- Phase 18 (Entity/GC) extends the Runtime with entity registry and real GC, replacing the bump allocator
- Phase 19 (Contract Dispatch) fills in CALL_VIRT with real dispatch tables
- Phase 21 (Runner CLI) creates `writ-runner` binary that consumes `writ-runtime::Runtime`
- Module loading: `Module::read()` from writ-module provides the starting point; Runtime decodes instruction bytes on load

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 17-vm-core-and-task-execution*
*Context gathered: 2026-03-02*
