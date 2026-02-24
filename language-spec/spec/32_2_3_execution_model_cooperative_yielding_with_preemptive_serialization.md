# Writ IL Specification
## 2.3 Execution Model: Cooperative Yielding with Preemptive Serialization

**Decision:** Functions are normal imperative code in the IL. The runtime manages suspension transparently.

**How it works:**

- The runtime interprets IL instruction-by-instruction on a managed call stack (not the native stack).
- At **transition points** — calls to `say()`, `choice()`, `wait()`, extern calls, `spawn`/`join` — the runtime *may*
  suspend execution and yield to the game engine.
- At **any point**, the runtime can snapshot the entire VM state for save/load.
- The IL itself contains no "yield here" instructions. Yielding is a runtime decision.

**What this is NOT:**

- Functions are NOT compiled into state machines.
- There is no async/await transformation.
- There are no explicit coroutine instructions in the IL.

**Implications:**

- The entire VM state must always be serializable: call stacks, register files, heap objects, globals.
- All data in registers must be of serializable types (no raw native pointers in script-visible state).
- Serialization only occurs at transition points (suspend-and-confirm model, §2.14.2). Native handles and GPU state are
  the host's responsibility.

