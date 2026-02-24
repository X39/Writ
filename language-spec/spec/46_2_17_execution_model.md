# Writ IL Specification
## 2.17 Execution Model

The Writ runtime executes IL code as a set of concurrent **tasks**. Each task has its own call stack and executes
independently. The runtime schedules tasks cooperatively — tasks run until they voluntarily suspend at transition
points,
complete, or crash.

The task state machine and scheduling model described below are a **minimum viable reference design** for runtime
implementors. The spec mandates correctness (tasks must execute their IL correctly, defer handlers must fire, atomic
sections must provide exclusion) but does not mandate a specific scheduler, threading model, or state representation.
Runtime implementors may extend the state machine with additional states or transitions as needed for their host
environment.

### 2.17.1 Call Stack

Each task maintains a **managed call stack**: an ordered sequence of **call frames**. Each frame contains:

- **method**: The MethodDef token identifying the executing method.
- **pc**: The program counter — byte offset into the method body's code section.
- **registers**: An array of typed register slots, sized per the method body's `reg_count`.
- **defer_stack**: A LIFO stack of pending defer handler offsets (pushed by `DEFER_PUSH`, popped by `DEFER_POP`).

When a `CALL`, `CALL_VIRT`, `CALL_INDIRECT`, or `CALL_EXTERN` instruction executes, the runtime pushes a new frame.
When `RET` or `RET_VOID` executes, the runtime runs the frame's defer handlers (LIFO), then pops the frame and resumes
the caller. `TAIL_CALL` replaces the current frame rather than pushing a new one.

The call stack is not the native thread stack — it is a runtime-managed data structure. This is essential for
serialization (the runtime must be able to walk and snapshot all frames) and for suspension (the runtime can pause and
resume a task without unwinding native frames).

### 2.17.2 Task States

Each task is in exactly one of the following states:

| State         | Description                                                                                      |
|---------------|--------------------------------------------------------------------------------------------------|
| **Ready**     | Runnable. Waiting to be scheduled for execution.                                                 |
| **Running**   | Actively executing instructions.                                                                 |
| **Suspended** | Blocked. Waiting on a host response, a `JOIN` target, or an atomic lock held by another task.    |
| **Completed** | Finished normally via `RET`/`RET_VOID` from the top frame. Return value is available for `JOIN`. |
| **Cancelled** | Terminated by crash or external `CANCEL`. Defer handlers have run. No return value is produced.  |

Completed and Cancelled are terminal states. `JOIN` on a Completed task delivers the return value. `JOIN` on a
Cancelled task crashes the joining task — there is no return value to deliver.

**Valid transitions:**

| From      | To        | Trigger                                                                      |
|-----------|-----------|------------------------------------------------------------------------------|
| *(new)*   | Ready     | `SPAWN_TASK`, `SPAWN_DETACHED`, or host command (fire event, start dialogue) |
| Ready     | Running   | Scheduler selects the task for execution                                     |
| Running   | Suspended | Task hits a transition point (§2.17.3) or `JOIN` on an incomplete task       |
| Running   | Ready     | Execution limit reached (§2.17.5) — task paused mid-execution                |
| Running   | Completed | `RET`/`RET_VOID` from the top frame (defer handlers run first)               |
| Running   | Cancelled | Crash unwinds all frames (§2.17.7); or `CANCEL` targets this task            |
| Suspended | Ready     | Host confirms the pending request; or `JOIN` target completes/cancels        |
| Ready     | Cancelled | `CANCEL` from another task (defer handlers run)                              |
| Suspended | Cancelled | `CANCEL` from another task (defer handlers run)                              |

**Optional runtime states:** Runtimes may introduce additional states beyond this minimum set. For example, a runtime
implementing atomic sections via a drain-and-run strategy (§2.17.6) may add a **Draining** state: when a task enters
`ATOMIC_BEGIN`, all other Running tasks are moved to Draining (paused at their current instruction boundary), the atomic
section runs to completion, and drained tasks return to Ready. This is one valid approach — not the only one.

### 2.17.3 Transition Points

A **transition point** is an instruction where the runtime suspends the executing task to wait for an external response.
The task moves from Running to Suspended and does not resume until the host or another task provides the awaited result.

| Instruction                                      | Suspension Reason                                                    |
|--------------------------------------------------|----------------------------------------------------------------------|
| `CALL_EXTERN`                                    | Host executes native code. Suspends until host returns a result.     |
| `SET_FIELD` (component field, post-construction) | Proxied to host via suspend-and-confirm (§2.14.2).                   |
| `GET_FIELD` (component field)                    | Host provides the current native value.                              |
| `GET_COMPONENT`                                  | Host resolves whether the entity has the component.                  |
| `SPAWN_ENTITY`                                   | Host creates native representation for attached components.          |
| `INIT_ENTITY`                                    | Flushes buffered component field writes to host as a batch.          |
| `DESTROY_ENTITY`                                 | Notifies host of entity destruction after `on_destroy` completes.    |
| `GET_OR_CREATE`                                  | May trigger entity spawn if the singleton does not yet exist.        |
| `JOIN`                                           | Suspends until the target task reaches Completed or Cancelled state. |

`SET_FIELD` and `GET_FIELD` on **script fields** (not component fields) are direct memory operations and are not
transition points.

Runtime-provided functions (`Runtime.say`, `Runtime.choice`, etc.) suspend through their underlying mechanism — they
are extern calls or intrinsics that internally interact with the host API. They are not special at the IL instruction
level.

**Transition points are the only points where the runtime is guaranteed to be in a consistent, serializable state.**
Save operations (§2.13) should only occur when all running tasks have reached a transition point or are otherwise
suspended.

### 2.17.4 Entry Points

The host drives execution through commands (§2.14.3). The following commands affect the task state machine:

| Host Command       | Effect                                                                                                      |
|--------------------|-------------------------------------------------------------------------------------------------------------|
| **Tick**           | Resume scheduling. The runtime executes Ready tasks until all tasks are Suspended, Completed, or Cancelled. |
| **Fire event**     | Create a new task for the event handler (e.g., `on_interact`). The task enters Ready.                       |
| **Start dialogue** | Create a new task for the dialogue function. The task enters Ready.                                         |
| **Confirm**        | Fulfill a Suspended task's pending host request. The task moves to Ready.                                   |

Tasks created by host commands enter the Ready state. Whether they are scheduled within the current tick or deferred to
the next tick is implementation-defined.

### 2.17.5 Scheduling and Execution Limits

The runtime must schedule Ready tasks for execution. The spec does not mandate a scheduling algorithm — the runtime may
use any strategy (FIFO, priority-based, round-robin, work-stealing, etc.). The order in which Ready tasks are selected
is implementation-defined.

**Threading:** The runtime may execute tasks on a single thread or dispatch them across multiple threads concurrently.
When multiple tasks execute in parallel, the runtime must ensure that `ATOMIC_BEGIN`/`ATOMIC_END` sections provide the
guarantees specified in §2.17.6. The spec recommends multi-threaded dispatch for runtimes targeting modern hardware, but
does not require it.

**Execution limits (recommended):** To prevent runaway scripts from blocking the host engine, runtimes should enforce an
execution limit per tick to bound the total time spent in script execution. This may be:

- An **instruction budget** per task (e.g., 10,000 instructions before yielding).
- A **wall-clock time limit** per tick (e.g., 16ms total across all tasks).
- Any other mechanism appropriate to the host environment.

When a task exceeds the execution limit, the runtime pauses it at the current instruction boundary and moves it from
Running back to Ready. The task resumes from the same instruction on the next scheduling pass. The runtime may log a
warning when a task is repeatedly limit-paused, as this typically indicates a script bug (tight loop with no transition
points).

**Exception:** A task inside an `ATOMIC_BEGIN`/`ATOMIC_END` section must not be paused by execution limits. See §2.17.6.

### 2.17.6 Atomic Sections

`ATOMIC_BEGIN` and `ATOMIC_END` create a region where the runtime **must** guarantee exclusive access to the globals
read or written by the executing task. The following requirements are mandatory:

1. **No interleaving.** While a task is inside an atomic section, no other task may read or write the involved global
   variables until `ATOMIC_END` executes.
2. **No execution-limit suspension.** The runtime must not pause a task due to execution limits while it is inside an
   atomic section. The section runs to `ATOMIC_END` without interruption.
3. **Proper nesting.** Every `ATOMIC_BEGIN` must have a matching `ATOMIC_END` before the enclosing frame returns. The
   runtime may detect unpaired atomics at module load time (verification) or at runtime.

**Implementation guidance:** The spec does not prescribe how the runtime achieves these guarantees. Approaches include
but are not limited to:

- **Drain-and-run:** When a task hits `ATOMIC_BEGIN`, the runtime pauses all other tasks at their next instruction
  boundary (or transition point), runs the atomic section to completion, then resumes normal scheduling. This may use
  an additional task state (e.g., Draining) beyond the minimum set in §2.17.2.
- **Per-global locking:** The atomic section acquires locks on globals as they are accessed. Other tasks block only if
  they attempt to access a locked global.
- **Single-threaded non-preemption:** If all tasks run on a single thread, atomic sections are inherently
  non-interleaved. The runtime simply disables execution-limit pausing for the duration.

**Transition points inside atomic sections:** If a transition point (e.g., `CALL_EXTERN`) occurs inside an atomic
section, the task suspends while holding the atomic guarantee. Other tasks attempting to access the guarded globals will
block until the atomic section completes. This can cause deadlocks if the host response depends on another blocked task.
The compiler **must** emit a warning when a transition point occurs inside an atomic block. A future language mechanism
may allow suppressing this warning for cases where the author has verified safety (see TODO).

### 2.17.7 Crash Propagation and Defer Unwinding

When a task crashes (`CRASH` instruction, failed `UNWRAP`/`UNWRAP_OK`, out-of-bounds array access, dead entity handle
access, division by zero, etc.), the runtime unwinds the **entire task call stack**:

1. Execute all defer handlers on the current frame's defer stack in LIFO order.
2. Pop the frame.
3. Repeat for the next frame down the stack, executing its defer handlers.
4. Continue until all frames are unwound.
5. The task enters the Cancelled state.
6. Log the crash to the host via the runtime logging interface (§2.14.7).

Defer handlers that themselves crash do not halt the unwinding process. The runtime logs the secondary crash and
continues unwinding the remaining defers and frames.

`CANCEL` from another task triggers the same unwinding sequence — the target task's stack is fully unwound with defer
handlers firing at each frame.

### 2.17.8 Task Tree

Tasks form a tree based on their spawn relationships:

- **Scoped tasks** (`SPAWN_TASK`): Children of the spawning task. When the parent completes, crashes, or is cancelled,
  all scoped children are automatically cancelled first (defer handlers run). The parent's own completion is deferred
  until all scoped children have terminated.
- **Detached tasks** (`SPAWN_DETACHED`): Independent of the spawning task. They are not affected by the parent's
  lifecycle and must be explicitly cancelled or allowed to run to completion.

Scoped task cancellation is recursive: cancelling a parent cancels its scoped children, which cancels their scoped
children, and so on. Defer handlers fire at each level during unwinding.

