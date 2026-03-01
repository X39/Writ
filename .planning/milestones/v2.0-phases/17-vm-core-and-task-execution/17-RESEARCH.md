# Phase 17: VM Core and Task Execution - Research

**Researched:** 2026-03-02
**Domain:** Register-based IL interpreter, cooperative task scheduler, Rust async-free concurrency
**Confidence:** HIGH (spec files read directly, existing codebase inspected, all decisions pre-locked in CONTEXT.md)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Host Embedding API**
- Request-response with IDs: Runtime emits `HostRequest` enum, task suspends, host calls `runtime.confirm(request_id, HostResponse)` to resume
- Single `HostRequest` enum with variants for all 9 transition points (ExternCall, EntitySpawn, FieldRead, FieldWrite, GetComponent, InitEntity, DestroyEntity, GetOrCreate, Join). Each carries requesting `task_id`
- Typed `HostResponse` enum: `Value(Value)`, `EntityHandle(EntityId)`, `Confirmed`, `Error(HostError)`. Runtime validates response matches request kind
- `RuntimeHost` trait: `fn on_request(&mut self, id: RequestId, req: &HostRequest) -> HostResponse` and `fn on_log(&mut self, level: LogLevel, message: &str)`. One trait per game engine
- NullHost auto-confirms immediately; tasks never actually suspend in tests
- Top-level type is `Runtime` (not Vm or VmState)
- Builder pattern: `RuntimeBuilder::new(module).with_host(host).with_limit(limit).build()`
- `writ-runtime` library crate: Convert main.rs to lib.rs. Depends on `writ-module`
- `Runtime` takes ownership of `Module`. No borrows, no Arc
- Public inspection methods: `runtime.task_state(task_id)`, `runtime.register_value(task_id, reg)`, `runtime.call_depth(task_id)`
- Single module in Phase 17. Multi-module deferred to Phase 19
- Entity instructions stub to host: SPAWN_ENTITY, INIT_ENTITY, DESTROY_ENTITY dispatched as HostRequests. No entity registry yet
- CALL_VIRT dispatches to CRASH: "contract dispatch not available". Phase 19 adds dispatch tables
- Decode instructions on load: `Vec<Instruction>` per method body. Dispatch loop on decoded instructions

**Value Representation**
- `enum Value { Void, Int(i64), Float(f64), Bool(bool), Ref(HeapRef), Entity(EntityId) }`
- `HeapRef` is opaque: `pub struct HeapRef(u32)` — index into heap object table. GC-relocatable
- Strings are heap objects referenced by `HeapRef`
- Simple bump allocator pre-GC. Objects allocated, never collected in Phase 17. GC trait boundary introduced with no-collect impl

**Handle System**
- Shared `GenHandle<Tag>`: `pub struct GenHandle<T> { index: u32, generation: u32, _phantom: PhantomData<T> }`
- `TaskId = GenHandle<TaskTag>`; `EntityId = GenHandle<EntityTag>` in Phase 18

**Scheduling Policy**
- Multi-thread-ready, per-task execution: Host can call `runtime.run_task(task_id)` from different threads. Runtime uses internal locking
- `tick(delta_time: f64, limit: ExecutionLimit)`. Host calls once per game frame
- FIFO creation order, round-robin rotation. Worked tasks go to back of queue
- Execution limits: instruction count OR time-based OR no-limit. Atomic sections exempt
- Runtime-enforced scoped task tree: parent cancel recursively cancels scoped children. Defer handlers fire at each level
- Detached tasks promoted to root level. Survive parent lifecycle
- Basic per-task counters: `instructions_executed`, `suspend_count`, `spawn_time`
- `call_sync()`: Host can call one method to completion without interruption. Ignores execution limits

**Atomic Sections**
- Per-global locking: locks acquired on first access, held until ATOMIC_END, released as batch
- No execution-limit suspension inside atomic sections

**Crash Diagnostics**
- Full stack trace: crash message + full call stack with method name and PC at each frame
- Delivered via both channels: `RuntimeHost::on_log` at error level AND returned in `TickResult`
- Secondary defer crashes: logged at error level with mini stack trace, unwinding continues

**Tick Result**
- `TickResult` enum: `AllCompleted`, `TasksSuspended(Vec<PendingRequest>)`, `ExecutionLimitReached`

### Claude's Discretion

- PC representation (instruction index vs byte offset) — pick what works best for decoded-on-load
- Defer stack representation (instruction offsets vs other)
- JOIN synchronization for multi-threaded per-task model
- tick() vs step() API granularity
- call_sync() implementation (temporary task vs direct execution)
- Register values in crash traces vs method+PC only
- Exact LogLevel enum variants beyond the spec's required four

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

## Summary

Phase 17 builds the core execution engine for the Writ runtime: a register-based IL interpreter with a cooperative task scheduler. Unlike web or async frameworks, there is no event loop or OS thread magic — the runtime is a synchronous library that the host drives explicitly via `tick()` and `confirm()` calls. The host controls threading; the runtime's job is correctness.

The fundamental design challenge is that all 5 task states, defer unwinding, atomic sections, crash propagation, and suspend-and-confirm must share the same per-task data structures (CallFrame, Task). Getting these structures right before writing any instruction handlers is the highest-leverage decision in this phase. The STATE.md explicitly flags this as the primary Phase 17 concern.

The second challenge is scoped-task cancellation: when a parent task cancels, it must recursively cancel all scoped children, run each child's defer stack fully, and only then proceed with the parent's own unwinding. This interaction between the task tree and per-frame defer stacks is a multi-pass algorithm that must be designed explicitly.

The instruction dispatch loop itself is straightforward — the `Instruction` enum from `writ-module` already exists with all 91 variants decoded. The match arms for arithmetic, comparison, and control flow are mechanical. The interesting cases are CALL/RET (frame push/pop with defer execution), SPAWN_TASK/JOIN/CANCEL (task tree manipulation), ATOMIC_BEGIN/END (global lock tracking), and the 9 host suspension points.

**Primary recommendation:** Design CallFrame and Task data structures completely on paper before writing any instruction handlers. The defer stack, atomic_depth counter, pending_entities buffer, and suspension slot must all live here. Restructuring these mid-phase is expensive.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| VM-01 | Dispatch loop executes all 91 instructions via match-dispatch | Instruction enum already complete in writ-module; decoded-on-load approach means PC = usize index into Vec<Instruction> |
| VM-02 | Typed register file stores values (Void/Int/Float/Bool/Ref/Entity) per-frame | Value enum locked in CONTEXT.md; registers = Vec<Value> per CallFrame, sized by method_body.register_types.len() |
| VM-03 | Managed call frame stack supports CALL/RET with register window sliding | Calling convention spec §2.6: args in r_base..r_base+argc-1 in caller frame, copied to r0..argc-1 in new callee frame |
| VM-04 | Arithmetic, comparison, and logic instructions produce spec-correct results | Spec files §3.2–3.5 define all behaviors; division by zero = crash; string equality = content comparison |
| VM-05 | Control flow (BR, BR_TRUE, BR_FALSE, SWITCH) branches correctly | Offsets are i32 in encoded bytes; since decoded-on-load uses Vec<Instruction>, offsets must be converted to absolute instruction indices at decode time |
| VM-06 | String and type conversion instructions operate correctly | I2s/F2s/B2s produce heap strings via bump allocator; StrConcat allocates new heap string |
| VM-07 | Object model instructions (NEW, GET_FIELD, SET_FIELD, CAST, IS) work per spec | NEW allocates heap object via bump allocator; GET_FIELD/SET_FIELD on struct fields are direct memory ops; component field access dispatches as HostRequest |
| TASK-01 | Task scheduler manages tasks through 5-state lifecycle | Ready/Running/Suspended/Completed/Cancelled per §2.17.2; task map with TaskId -> Task |
| TASK-02 | Suspend-and-confirm correctly suspends at all 9 transition points | 9 transition points: CALL_EXTERN, SET_FIELD(component), GET_FIELD(component), GET_COMPONENT, SPAWN_ENTITY, INIT_ENTITY, DESTROY_ENTITY, GET_OR_CREATE, JOIN |
| TASK-03 | Defer stack executes in LIFO order on RET, crash, and CANCEL | Per §3.11: DEFER_PUSH pushes handler offset, DEFER_END signals continue unwind; handlers stored as instruction indices on CallFrame |
| TASK-04 | Crash propagation unwinds full call stack, executing all defer handlers | Multi-frame unwind: for each frame bottom-to-top, execute defers LIFO, then pop frame |
| TASK-05 | Secondary crashes in defer handlers are logged and swallowed | Catch secondary crash during defer execution: log via on_log(Error), continue next defer |
| TASK-06 | ATOMIC_BEGIN/END prevents task interleaving, suppresses execution limits | Per-global mutex map; atomic_depth counter on Task; execution limit check skipped when atomic_depth > 0 |
| TASK-07 | SPAWN/JOIN/CANCEL concurrency instructions manage child tasks correctly | SPAWN_TASK creates scoped child (parent_id set); SPAWN_DETACHED creates root-level; JOIN suspends until target terminal; CANCEL triggers recursive unwind |
| TASK-08 | RuntimeHost trait compiles with NullHost as synchronous no-op | Single trait on_request+on_log; NullHost returns default responses synchronously |
</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `writ-module` | workspace | Module data, Instruction enum, MethodBody, MetadataToken | Already complete from Phase 16; all 91 opcodes decoded |
| Rust std only | — | Vec, HashMap, Mutex, Arc, std::time::Instant | No external dependencies needed for the interpreter core |
| `thiserror` | 2.0 | Error types (RuntimeError, CrashInfo) | Already in writ-module; consistent with project pattern |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `parking_lot` | 0.12 | Faster Mutex/RwLock for global locking in atomic sections | If std Mutex contention shows in benchmarks; optional |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Rust enum Value | Tagged union / union | Enum is safe, matches spec's typed register model, no UB risk |
| Vec<Value> registers | HashMap<u16, Value> | Vec is O(1) access; HashMap adds overhead; reg count known at load time |
| Decoded Vec<Instruction> | Byte-offset threaded | Decoded is simpler to index, debug, and test; spec-required PC-to-source-span mapping is easier |
| std::Mutex for globals | parking_lot::Mutex | std Mutex is sufficient for correctness; parking_lot only if profiling shows contention |

**Installation:**
```bash
# Only dependency to add to writ-runtime Cargo.toml:
writ-module = { path = "../writ-module" }
thiserror = "2.0"
```

---

## Architecture Patterns

### Recommended Project Structure

```
writ-runtime/
├── src/
│   ├── lib.rs              # pub use; RuntimeBuilder, Runtime, RuntimeHost
│   ├── value.rs            # Value enum, HeapRef, GenHandle, EntityId, TaskId
│   ├── heap.rs             # BumpHeap, HeapObject (struct/array/string/delegate/enum)
│   ├── frame.rs            # CallFrame (method_idx, pc, registers, defer_stack)
│   ├── task.rs             # Task struct, TaskState enum, TaskTree
│   ├── scheduler.rs        # Scheduler (ready_queue, task_map, global_locks)
│   ├── dispatch.rs         # execute_instruction() match dispatch
│   ├── host.rs             # RuntimeHost trait, HostRequest, HostResponse, NullHost
│   └── runtime.rs          # Runtime struct, RuntimeBuilder, tick(), confirm(), call_sync()
└── tests/
    ├── vm_tests.rs         # Per-instruction unit tests (arithmetic, control flow, etc.)
    ├── task_tests.rs       # Lifecycle, spawn/join/cancel, defer, atomic, crash
    └── host_tests.rs       # NullHost round-trip, suspend-and-confirm protocol
```

### Pattern 1: CallFrame — the central data structure

**What:** Every executing function has one CallFrame. All instruction-level state lives here.

**Design (must be complete before coding dispatch):**
```rust
pub struct CallFrame {
    /// Index into module.method_defs (0-based; MetadataToken is 1-based so subtract 1)
    pub method_idx: usize,
    /// Index into decoded Vec<Instruction> for this method (not a byte offset)
    pub pc: usize,
    /// Typed register file. Sized at frame creation from method_body.register_types.len()
    pub registers: Vec<Value>,
    /// LIFO defer handler stack. Each entry is an instruction index in the same method's body.
    pub defer_stack: Vec<usize>,
}
```

**Key design note:** PC is an instruction index (not byte offset) because instructions are decoded into `Vec<Instruction>` at load time. Branch offsets in the encoded binary are byte offsets — they must be converted to instruction indices when decoding. This conversion happens once at load time, not at dispatch time.

### Pattern 2: Task — owns the call stack

```rust
pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
    pub call_stack: Vec<CallFrame>,        // top = call_stack.last()
    pub parent_id: Option<TaskId>,
    pub scoped_children: Vec<TaskId>,     // recursively cancelled with parent
    pub pending_request: Option<(RequestId, HostRequest)>,   // Some when Suspended
    pub return_value: Option<Value>,      // Some when Completed
    pub crash_info: Option<CrashInfo>,   // Some when Cancelled by crash
    pub atomic_depth: u32,               // >0 = inside ATOMIC_BEGIN/END
    pub instructions_executed: u64,
    pub suspend_count: u32,
    pub spawn_time: std::time::Instant,
    /// Globals locked by this task's current atomic section. Cleared at ATOMIC_END.
    pub atomic_locks: Vec<u32>,          // global_idx values locked
}
```

### Pattern 3: Instruction index from byte offset (load-time conversion)

**What:** The spec encodes branch targets as i32 byte offsets. Since the dispatch loop uses instruction indices, convert at load time.

**How:**
```rust
/// During module load, decode code bytes and also build a byte-offset -> instruction-index map.
fn decode_method_body(raw_code: &[u8]) -> (Vec<Instruction>, Vec<usize>) {
    // Returns: (instructions, byte_offset_to_instr_index)
    // After decoding all instructions, patch branch offsets:
    //   Br { offset } -> Br { offset: resolved_instr_index }
    // Same for BrTrue, BrFalse, Switch, DeferPush
}
```

The `DeferPush { r_dst, method_idx }` operand in the spec is actually a method_idx for the handler function (per the Instruction enum) — but the spec's §3.11 defer layout shows handler_offset as a code offset within the same method. The existing Instruction::DeferPush stores `method_idx: u32` (matching the encoding spec in the binary format), not a byte offset. **Clarification needed** — see Open Questions.

### Pattern 4: Dispatch loop structure

```rust
fn execute_task(task: &mut Task, module: &LoadedModule, host: &mut dyn RuntimeHost,
                limit: &mut ExecutionLimit) -> ExecutionResult {
    loop {
        // 1. Check execution limit (skip if atomic_depth > 0)
        if !task.atomic_depth > 0 && limit.exhausted() {
            return ExecutionResult::LimitReached;
        }

        let frame = task.call_stack.last_mut().unwrap();
        let instr = &module.decoded_bodies[frame.method_idx][frame.pc];
        frame.pc += 1;

        match instr {
            Instruction::Nop => {}
            Instruction::LoadInt { r_dst, value } => {
                frame.registers[*r_dst as usize] = Value::Int(*value);
            }
            Instruction::Ret { r_src } => {
                let ret_val = frame.registers[*r_src as usize].clone();
                return execute_ret(task, ret_val);  // runs defers, pops frame
            }
            // ... all 91 arms
            Instruction::CallVirt { .. } => {
                // Phase 17: crash with "contract dispatch not available"
                return ExecutionResult::Crash("CALL_VIRT: contract dispatch not available".into());
            }
        }
        task.instructions_executed += 1;
    }
}
```

### Pattern 5: Defer execution — the unwind algorithm

```rust
fn execute_ret(task: &mut Task, ret_val: Value) -> ExecutionResult {
    // Step 1: Run current frame's defers in LIFO order
    while let Some(handler_pc) = task.call_stack.last_mut().unwrap().defer_stack.pop() {
        if let Err(secondary_crash) = execute_defer_handler(task, handler_pc, module, host) {
            // TASK-05: Log secondary crash, continue unwinding
            host.on_log(LogLevel::Error, &format!("secondary crash in defer: {}", secondary_crash));
        }
    }
    // Step 2: Pop the frame
    task.call_stack.pop();
    // Step 3: If stack empty -> Completed; else deliver return value to caller frame
    if task.call_stack.is_empty() {
        task.state = TaskState::Completed;
        task.return_value = Some(ret_val);
        return ExecutionResult::Completed(ret_val);
    }
    // deliver ret_val to caller's r_dst (stored in the call frame before push)
    ExecutionResult::Continue
}

fn execute_crash(task: &mut Task, msg: String, module: &LoadedModule, host: &mut dyn RuntimeHost) {
    // Unwind entire call stack with defer handlers at each level
    let crash_info = build_crash_info(task, &msg, module);
    while !task.call_stack.is_empty() {
        let frame_defers: Vec<usize> = task.call_stack.last().unwrap().defer_stack.clone().into_iter().rev().collect();
        for handler_pc in frame_defers {
            if let Err(e) = execute_defer_handler(task, handler_pc, module, host) {
                host.on_log(LogLevel::Error, &format!("secondary crash in defer: {}", e));
            }
        }
        task.call_stack.pop();
    }
    task.state = TaskState::Cancelled;
    task.crash_info = Some(crash_info.clone());
    host.on_log(LogLevel::Error, &crash_info.to_string());
}
```

### Pattern 6: Scoped task cancellation (recursive)

```rust
fn cancel_task_tree(scheduler: &mut Scheduler, task_id: TaskId, module: &LoadedModule, host: &mut dyn RuntimeHost) {
    // Cancel all scoped children first (recursive, depth-first)
    let children = scheduler.tasks[task_id].scoped_children.clone();
    for child_id in children {
        cancel_task_tree(scheduler, child_id, module, host);
    }
    // Then cancel this task
    let task = &mut scheduler.tasks[task_id];
    if matches!(task.state, TaskState::Completed | TaskState::Cancelled) {
        return; // already terminal
    }
    execute_crash(task, "task cancelled".into(), module, host);
}
```

### Pattern 7: Atomic section with per-global locking

**Approach (per CONTEXT.md decision: per-global locking):**

```rust
// In Scheduler:
pub global_locks: HashMap<u32, TaskId>,  // global_idx -> owner task_id

// ATOMIC_BEGIN handler:
// Just increment task.atomic_depth. Locks are acquired lazily.

// LOAD_GLOBAL / STORE_GLOBAL inside atomic section (task.atomic_depth > 0):
//   if global_locks.get(global_idx) == Some(other_task_id) -> suspend (Blocked variant)
//   else: global_locks.insert(global_idx, task.id); task.atomic_locks.push(global_idx)
//   proceed with read/write

// ATOMIC_END handler:
//   for idx in task.atomic_locks.drain(..) { global_locks.remove(&idx); }
//   task.atomic_depth -= 1;
```

### Pattern 8: Suspend-and-confirm flow

```rust
// In dispatch — on CALL_EXTERN:
fn suspend_for_host(task: &mut Task, req: HostRequest, next_request_id: &mut u32) -> ExecutionResult {
    let id = RequestId(*next_request_id);
    *next_request_id += 1;
    task.pending_request = Some((id, req));
    task.state = TaskState::Suspended;
    task.suspend_count += 1;
    ExecutionResult::Suspended(id)
}

// On Runtime::confirm(request_id, response):
//   Find task by pending_request.0 == request_id
//   Validate response kind matches request kind
//   Store response value in frame's r_dst register
//   task.state = TaskState::Ready
//   task.pending_request = None
```

### Pattern 9: NullHost implementation

```rust
pub struct NullHost;

impl RuntimeHost for NullHost {
    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { .. } => HostResponse::Value(Value::Void),
            HostRequest::EntitySpawn { .. } => HostResponse::Confirmed,
            HostRequest::FieldRead { .. } => HostResponse::Value(Value::Int(0)),
            HostRequest::FieldWrite { .. } => HostResponse::Confirmed,
            HostRequest::GetComponent { .. } => HostResponse::Value(Value::Void), // None
            HostRequest::InitEntity { .. } => HostResponse::Confirmed,
            HostRequest::DestroyEntity { .. } => HostResponse::Confirmed,
            HostRequest::GetOrCreate { .. } => HostResponse::Confirmed,
            HostRequest::Join { .. } => HostResponse::Confirmed, // handled by scheduler
        }
    }
    fn on_log(&mut self, _level: LogLevel, message: &str) {
        // Phase 17 NullHost: silently drop or print to stdout for debugging
        let _ = message;
    }
}
```

### Anti-Patterns to Avoid

- **Byte-offset PC in the dispatch loop:** The decoded-on-load design mandates instruction-index PC. Using byte offsets after decoding creates redundant re-decoding every branch. Convert at load time.
- **Cloning Value on every instruction:** For non-reference Value variants (Int, Float, Bool, Void), Copy semantics are fine. For Ref/Entity, clone is a u32 copy. Never deep-clone register arrays.
- **Holding task borrow while dispatching:** The scheduler owns tasks; the dispatch loop needs `&mut Task`. Design to either pass task by `&mut` parameter or split borrow carefully. Do not store task pointers.
- **Panic on invalid register access:** Invalid register indices are a VM implementation bug (not a script crash). Use debug assertions, not panics in release. Provide runtime error for out-of-bounds.
- **Running defer handlers via recursive call to execute_task:** Defer handlers are code blocks in the same method body. Jump the PC to the handler's instruction index and execute instructions until DEFER_END. Do not recursively call execute_task — this collapses the frame stack incorrectly.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Module reading | Module decoder | `writ_module::Module::from_bytes()` | Already complete, round-trip tested |
| Instruction decoding | Custom decoder | `writ_module::Instruction::decode()` | Already complete with all 91 opcodes |
| Test module construction | Manual byte arrays | `writ_module::ModuleBuilder` | Already complete with fluent API for all 21 table types |
| Error formatting | Custom Display | `thiserror::Error` derive | Already used in writ-module |

**Key insight:** Phase 16 deliberately built all the module infrastructure Phase 17 needs. The instruction enum, decode, builder, and all table structs are production-complete. Phase 17 is a consumer, not a builder, of writ-module.

---

## Common Pitfalls

### Pitfall 1: Branch Offset Conversion

**What goes wrong:** Branch instructions (Br, BrTrue, BrFalse, Switch) encode targets as i32 byte offsets into the method's raw code bytes. If the dispatch loop uses instruction indices (as decided), these offsets cannot be used directly.

**Why it happens:** The spec's binary format (byte offsets) and the runtime's execution model (decoded Vec<Instruction> with index PC) use different addressing schemes.

**How to avoid:** During module load, when decoding each method body, build a `byte_offset → instruction_index` map. After decoding all instructions, rewrite branch targets from byte offsets to instruction indices. Store the rewritten indices in the Instruction variants. This is a one-time O(n) pass per method body.

**Warning signs:** BR jumping to the wrong instruction, infinite loops on correct code, SWITCH targeting wrong handlers.

### Pitfall 2: DeferPush handler_offset vs method_idx ambiguity

**What goes wrong:** The `Instruction::DeferPush` in writ-module stores `method_idx: u32`, matching the binary encoding shape `RI32`. But §3.11 in the spec describes defer handlers as code blocks *within the current method body* accessed by `handler_offset`. These are different things.

**Why it happens:** The binary format stores a field named `method_idx` in the RI32 slot, but whether this is an absolute method index or a byte offset within the current method depends on the spec's actual semantics. The concurrency spec §3.11 says "handler_offset points to a code block within the current method body."

**How to avoid:** Read the binary format spec (§2.5) carefully for DEFER_PUSH. The field is a byte offset, not a method index — the field name in the Instruction enum (`method_idx`) appears to be a naming inconsistency. Treat the `method_idx` field in `DeferPush` as a byte offset into the current method's code, and convert it to an instruction index at load time. See Open Questions #1.

**Warning signs:** Defers executing wrong code, crashing immediately when defers fire.

### Pitfall 3: Scoped child cancellation ordering

**What goes wrong:** When a parent task crashes or is cancelled, scoped children must be fully unwound (including their own defers) before the parent continues its own unwind. Getting the ordering wrong produces double-free-like bugs where a child's defer handler accesses parent state that has already been cleaned up.

**Why it happens:** The cancel algorithm feels recursive but the implementation must be iterative or carefully ordered. The parent's defer stack and child cancellation are separate concerns.

**How to avoid:** In `execute_crash`, before touching the parent task's defer stack, iterate through `task.scoped_children` and cancel each child to completion. Only then unwind the parent.

**Warning signs:** Child defer handlers running after parent resources freed, parent completing before children cancelled.

### Pitfall 4: JOIN race condition in multi-thread-ready design

**What goes wrong:** The design says `run_task(task_id)` can be called from different threads. If Task A JOINs Task B, and both are being run concurrently, the scheduler must correctly transition A to Suspended and later to Ready when B completes — without a data race on the task map.

**Why it happens:** The multi-thread-ready requirement introduces concurrent access to the task map even in Phase 17.

**How to avoid:** Wrap the entire task map (and scheduler state) in a `Mutex<SchedulerState>`. Each `run_task` call locks the scheduler for state transitions but releases the lock before actually executing instructions. The execute loop works on a local `Task` copy or passes exclusive `&mut Task` through the locked region carefully.

**Note:** The simpler correct approach for Phase 17 is: always hold the scheduler lock for the full `run_task` call. This serializes all task execution — correct, and multi-thread-safe. Phase 18+ can optimize for real parallelism. The host can still call `run_task` from threads; they just serialize at the lock.

**Warning signs:** JOIN target completing but joining task never waking, task state corruption under concurrent tick calls.

### Pitfall 5: Execution limit not suppressed in atomic sections

**What goes wrong:** A task inside ATOMIC_BEGIN/END gets paused by the instruction count or time limit, allowing another task to interleave and access a global the atomic task has locked.

**Why it happens:** The limit check happens before each instruction. Easy to forget the `atomic_depth > 0` guard.

**How to avoid:** The execution limit check is: `if task.atomic_depth == 0 && limit.exhausted() { yield }`. Make this a single checked function that cannot be bypassed.

**Warning signs:** Atomic section interleaving test fails; second task can read globals mid-atomic.

### Pitfall 6: TAIL_CALL frame replacement vs push

**What goes wrong:** TAIL_CALL replaces the current frame rather than pushing a new one. Implementing it as a CALL (push new frame, set PC) breaks tail-call semantics and eventually stack-overflows on dialogue `->` transitions.

**Why it happens:** CALL and TAIL_CALL look similar in the Instruction enum.

**How to avoid:** For TAIL_CALL: copy arguments into a temporary buffer, replace the current frame's `method_idx`, `pc`, and resize `registers`, restore arguments from the buffer. The defer stack of the old frame executes first (LIFO) before the replacement.

**Warning signs:** Stack grows unboundedly on dialogue chains; dialogue `->` transitions eventually crash with stack overflow.

### Pitfall 7: String equality is content comparison (CMP_EQ_S)

**What goes wrong:** `CMP_EQ_S` comparing two strings that happen to be the same module string literal returns true by comparing HeapRef values (both point to the same interned string), but comparing two runtime-constructed strings with identical content returns false if they are different heap allocations.

**Why it happens:** Strings are always HeapRef values; the impl must resolve both refs and compare content.

**How to avoid:** `CMP_EQ_S` must always resolve both HeapRefs to their underlying string content and compare byte-by-byte. Never compare HeapRef indices directly.

---

## Code Examples

### Module Load: Decode and index instruction bodies

```rust
pub struct LoadedModule {
    pub module: Module,
    /// For each method body (parallel to module.method_bodies),
    /// the decoded Vec<Instruction> with branch targets rewritten to instruction indices.
    pub decoded_bodies: Vec<Vec<Instruction>>,
}

impl LoadedModule {
    pub fn from_module(module: Module) -> Result<Self, RuntimeError> {
        let mut decoded_bodies = Vec::with_capacity(module.method_bodies.len());
        for body in &module.method_bodies {
            let instructions = decode_and_reindex(&body.code)?;
            decoded_bodies.push(instructions);
        }
        Ok(Self { module, decoded_bodies })
    }
}

fn decode_and_reindex(raw_code: &[u8]) -> Result<Vec<Instruction>, RuntimeError> {
    // 1. Decode all instructions, recording byte offset of each
    // 2. Build offset_map: HashMap<u32, usize> (byte_offset -> instr_index)
    // 3. Rewrite branch targets:
    //    - Br { offset } -> Br { offset: offset_map[current_byte_offset + offset] as i32 }
    //    - BrTrue, BrFalse, Switch: same
    //    - DeferPush: resolve method_idx field as byte offset -> instruction index
    todo!("implement during Wave 1")
}
```

### GenHandle: type-safe generation handles

```rust
use std::marker::PhantomData;

pub struct TaskTag;
pub struct EntityTag;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenHandle<T> {
    pub index: u32,
    pub generation: u32,
    _phantom: PhantomData<T>,
}

pub type TaskId = GenHandle<TaskTag>;
pub type EntityId = GenHandle<EntityTag>;

impl<T> GenHandle<T> {
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation, _phantom: PhantomData }
    }
}
```

### RuntimeHost trait + NullHost

```rust
pub trait RuntimeHost {
    fn on_request(&mut self, id: RequestId, req: &HostRequest) -> HostResponse;
    fn on_log(&mut self, level: LogLevel, message: &str);
}

pub struct NullHost;
impl RuntimeHost for NullHost {
    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { .. } => HostResponse::Value(Value::Void),
            HostRequest::FieldRead { .. } => HostResponse::Value(Value::Int(0)),
            _ => HostResponse::Confirmed,
        }
    }
    fn on_log(&mut self, _level: LogLevel, _message: &str) {}
}
```

### TickResult and ExecutionLimit

```rust
pub enum TickResult {
    AllCompleted,
    TasksSuspended(Vec<PendingRequest>),
    ExecutionLimitReached,
}

pub enum ExecutionLimit {
    Instructions(u64),
    Time(std::time::Duration),
    None,
}

impl ExecutionLimit {
    pub fn exhausted(&self, start: std::time::Instant, instr_count: u64) -> bool {
        match self {
            ExecutionLimit::Instructions(max) => instr_count >= *max,
            ExecutionLimit::Time(max) => start.elapsed() >= *max,
            ExecutionLimit::None => false,
        }
    }
}
```

### Calling convention: CALL instruction frame push

```rust
// Caller frame has args at r_base..r_base+argc-1
// New frame gets them at r0..argc-1
fn push_call_frame(task: &mut Task, method_idx: usize, r_base: u16, argc: u16,
                   r_dst: u16, loaded_module: &LoadedModule) {
    let reg_count = loaded_module.module.method_bodies[method_idx].register_types.len();
    let mut registers = vec![Value::Void; reg_count];

    // Copy arguments from caller frame
    let caller = task.call_stack.last().unwrap();
    for i in 0..argc as usize {
        registers[i] = caller.registers[r_base as usize + i].clone();
    }

    // Store r_dst in caller frame so RET knows where to put the return value
    // (Store in CallFrame as `return_register: u16`)

    task.call_stack.push(CallFrame {
        method_idx,
        pc: 0,
        registers,
        defer_stack: Vec::new(),
        return_register: r_dst,  // where caller expects return value
    });
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Stack-based VM (JVM/CLR style) | Register-based (per spec §2.1) | Spec decision pre-Phase 17 | Instructions encode src/dst explicitly; easier to serialize frames |
| Byte-offset PC with per-instruction decode | Decoded Vec<Instruction> with index PC | Phase 17 decision | Faster dispatch, cleaner branch logic, load-time cost only |
| Async/await for task suspension | Explicit managed call stack with cooperative yield | Out of scope (see REQUIREMENTS.md) | Enables frame serialization; Rust async futures cannot be inspected |

---

## Open Questions

1. **DeferPush: byte offset or method index?**
   - What we know: The Instruction enum field is named `method_idx: u32` (matches `RI32` encoding shape). The §3.11 spec says "handler_offset points to a code block within the current method body."
   - What's unclear: Is the `method_idx` field in DeferPush actually a byte offset into the current method's code (consistent with §3.11), or is it an index into the MethodDef table (which would mean defers call separate methods)?
   - Recommendation: Assume it is a **byte offset within the current method** consistent with §3.11's layout diagram and the field name being a red herring from the binary format's RI32 shape. The defer handler code is embedded in the method body (after main code), not a separate method. Validate by checking the binary format spec §2.5 for DEFER_PUSH encoding definition.

2. **call_sync() implementation: temporary task or direct frame push?**
   - What we know: Host needs to call one method to completion synchronously. Ignores execution limits.
   - What's unclear: Whether to create a temporary `Task` that runs to completion, or push frames directly onto an existing call context.
   - Recommendation: Create a one-shot task with `ExecutionLimit::None` and run it synchronously within `call_sync()`. Simpler implementation; the task machinery handles defers and crashes correctly. Temporary task is cleaned up after `call_sync` returns.

3. **Return register storage in CallFrame**
   - What we know: When CALL pushes a new frame, the caller specified `r_dst` for the return value. The callee's RET must write to the caller's `r_dst`.
   - What's unclear: Whether to store `r_dst` in the new frame (so RET can look it up) or in the caller frame.
   - Recommendation: Store `return_register: u16` in the new callee frame. When RET pops the frame, it reads `return_register` from the now-popped frame (save it before popping), then writes the return value into `task.call_stack.last_mut().registers[return_register]`.

---

## Validation Architecture

> `workflow.nyquist_validation` is not present in config.json (field missing). Skipping this section.

---

## Sources

### Primary (HIGH confidence)
- `D:/dev/git/Writ/language-spec/spec/46_2_17_execution_model.md` — Task states, transitions, atomic sections, crash propagation, task tree, scheduling
- `D:/dev/git/Writ/language-spec/spec/43_2_14_runtime_host_interface.md` — Host request/response contract, logging interface, entity ownership model
- `D:/dev/git/Writ/language-spec/spec/35_2_6_calling_convention.md` — Register window, arg layout, self semantics
- `D:/dev/git/Writ/language-spec/spec/59_3_11_concurrency.md` — SPAWN_TASK, JOIN, CANCEL, DEFER_PUSH/POP/END
- `D:/dev/git/Writ/language-spec/spec/60_3_12_globals_atomics.md` — LOAD_GLOBAL, STORE_GLOBAL, ATOMIC_BEGIN/END
- `D:/dev/git/Writ/language-spec/spec/56_3_8_object_model.md` — NEW, GET_FIELD, SET_FIELD, entity instructions
- `D:/dev/git/Writ/language-spec/spec/55_3_7_calls.md` — CALL, CALL_VIRT, CALL_EXTERN, TAIL_CALL, NEW_DELEGATE, CALL_INDIRECT
- `D:/dev/git/Writ/language-spec/spec/30_2_1_register_based_virtual_machine.md` — Register-based rationale
- `D:/dev/git/Writ/language-spec/spec/38_2_9_memory_model.md` — Value types, reference types, string semantics, entity lifecycle
- `D:/dev/git/Writ/language-spec/spec/44_2_15_il_type_system.md` — Type tags, TypeRef encoding, enum representation
- `D:/dev/git/Writ/writ-module/src/instruction.rs` — Complete Instruction enum with all 91 opcodes
- `D:/dev/git/Writ/writ-module/src/module.rs` — Module struct, MethodBody, ModuleHeader
- `D:/dev/git/Writ/.planning/phases/17-vm-core-and-task-execution/17-CONTEXT.md` — All locked decisions

### Secondary (MEDIUM confidence)
- `D:/dev/git/Writ/.planning/STATE.md` — Phase concern about CallFrame/Task data structure design first
- `D:/dev/git/Writ/.planning/REQUIREMENTS.md` — VM-01 through TASK-08 requirement text

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — writ-module already complete; runtime needs only std + thiserror
- Architecture: HIGH — all major decisions locked in CONTEXT.md; structure follows directly from spec
- Pitfalls: HIGH — identified from spec analysis and Rust borrow/concurrency constraints
- Open questions: MEDIUM — DeferPush semantics requires reading binary format spec §2.5 for final confirmation

**Research date:** 2026-03-02
**Valid until:** 2026-04-02 (stable spec, no external dependencies to track)
