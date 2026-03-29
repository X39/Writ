# Phase 77: Frame Register Pool - Research

**Researched:** 2026-03-22
**Domain:** Rust object pooling, VM call-frame memory management
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all implementation choices are at Claude's discretion.

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FRAME-01 | RegisterPool struct exists with acquire(reg_count) and release(vec) methods | Pool struct in frame.rs or new pool.rs; API design documented below |
| FRAME-02 | Pool acquire reuses a Vec from the free-list when capacity is sufficient | Scan free-list for `capacity >= reg_count`, pop and resize instead of allocating |
| FRAME-03 | Pool release clears Vec to Value::Void before storing in free-list | `v.fill(Value::Void)` or `v.iter_mut().for_each(|x| *x = Value::Void)` then push to free-list |
| FRAME-04 | Pool size is capped at 64 entries to prevent unbounded memory retention | Check `free_list.len() < 64` before pushing on release; drop Vec if cap exceeded |
| FRAME-05 | execute_ret returns popped frame's register Vec to the pool | Pool must be accessible from execute_ret call site in dispatch/mod.rs |
| FRAME-06 | Pool-correctness test verifies reused registers contain Value::Void | Integration test: acquire, write non-Void, release, re-acquire, assert all Void |
| VERIFY-01 | fib(40) produces correct output 102334155 | Correctness unchanged; pool is transparent to program semantics |
| VERIFY-02 | cargo test --release passes with zero failures | Full test suite must remain green |
| VERIFY-03 | cargo build --release produces zero warnings | No dead code, unused imports, or missing inlines |
</phase_requirements>

## Summary

Phase 77 introduces a `RegisterPool` — a thread-local free-list of `Vec<Value>` allocations recycled between call frames. Currently, every `CallFrame::new()` call allocates `vec![Value::Void; reg_count]`, and every `execute_ret` simply drops the popped frame (and its registers Vec). For recursive functions like fib(40), which makes 331,160,281 recursive calls, this is 331 million allocations and deallocations of small Vecs.

The fix is a size-capped free-list: when `execute_ret` pops a frame, it clears the registers Vec and puts it in the pool. When `CallFrame::new()` needs registers, it checks the pool first and reuses a Vec whose capacity already covers the required size instead of allocating. The pool is capped at 64 entries to bound memory retention.

The key implementation challenge is **ownership threading**: the pool must be reachable from both `CallFrame::new()` (acquire path) and `execute_ret` (release path). The cleanest solution is to pass the pool as a `&mut RegisterPool` parameter alongside the existing context, or to store it on `Scheduler` (which already owns tasks and globals). The pool is per-task-execution (single-threaded cooperative model), so no synchronization is required.

**Primary recommendation:** Store `RegisterPool` on `Scheduler`, thread it through `run_one_task` into `ExecContext`, and access it from `execute_ret` and `exec_call` family.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust stdlib `Vec<T>` | built-in | The pool free-list container | Already in use throughout codebase |
| `writ_runtime::value::Value` | project | Element type being pooled | The only element type in register Vecs |

No external libraries needed. This is a pure Rust stdlib pattern.

### Supporting
None required. The pool is a plain `Vec<Vec<Value>>` (free-list of Vecs).

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `Vec<Vec<Value>>` free-list | `arrayvec::ArrayVec` | No external dep needed; stdlib Vec is fine for 64-entry cap |
| Pool on Scheduler | Thread-local static | Thread-locals require `unsafe` or `RefCell`; Scheduler already owns execution state |
| Pool on ExecContext | Pool on Task | ExecContext lifetime is per-instruction; Task is per-task; Scheduler spans all tasks — Scheduler is cleanest for multi-task reuse |

## Architecture Patterns

### Recommended Project Structure

Pool lives in `frame.rs` alongside `CallFrame`. The pool is passed by `&mut` reference through the call chain.

```
writ-runtime/src/
├── frame.rs          # CallFrame + RegisterPool (new struct here)
├── scheduler.rs      # Scheduler gains pool: RegisterPool field
├── dispatch/
│   ├── mod.rs        # ExecContext gains pool: &mut RegisterPool; execute_ret uses pool
│   └── calls.rs      # exec_call/exec_call_virt/exec_call_indirect use pool to acquire
```

### Pattern 1: RegisterPool Free-List

**What:** A capped Vec of recycled register Vecs. Acquire pops a Vec with sufficient capacity; release clears and pushes.

**When to use:** Every frame creation (acquire) and every frame destruction (release).

```rust
// In frame.rs — alongside CallFrame
pub struct RegisterPool {
    free_list: Vec<Vec<Value>>,
}

const POOL_CAP: usize = 64;

impl RegisterPool {
    pub fn new() -> Self {
        Self { free_list: Vec::new() }
    }

    /// Acquire a register Vec for a new frame.
    ///
    /// Returns a recycled Vec (resized, filled with Void) if one with sufficient
    /// capacity exists. Otherwise allocates fresh. O(n) scan of free-list.
    #[inline]
    pub fn acquire(&mut self, reg_count: usize) -> Vec<Value> {
        // Find any Vec with capacity >= reg_count (scan from back for recency)
        for i in (0..self.free_list.len()).rev() {
            if self.free_list[i].capacity() >= reg_count {
                let mut v = self.free_list.swap_remove(i);
                // Resize fills new slots with Void; existing slots already cleared by release
                v.resize(reg_count, Value::Void);
                return v;
            }
        }
        // Pool miss — allocate fresh
        vec![Value::Void; reg_count]
    }

    /// Release a register Vec back to the pool.
    ///
    /// Clears all registers to Void before pooling (safety: prevents stale values
    /// from leaking into reused frames). Drops the Vec if pool is at capacity.
    #[inline]
    pub fn release(&mut self, mut v: Vec<Value>) {
        if self.free_list.len() >= POOL_CAP {
            // Drop Vec — pool is full
            return;
        }
        // Clear to Void (required by FRAME-03, FRAME-06)
        v.fill(Value::Void);
        // Truncate length but keep capacity for reuse
        v.clear();
        self.free_list.push(v);
    }
}
```

**Notes on the clear strategy:**
- `v.fill(Value::Void)` writes Void to every element — this is required BEFORE `clear()` so the memory is clean.
- `v.clear()` sets len=0, keeping capacity intact. Reuse via `resize(reg_count, Value::Void)` is then O(reg_count).
- Alternative: skip `fill` and only do `resize` on acquire (which fills new slots). But `resize` only fills up to the new len; if the old len was larger, stale values remain. Therefore fill-then-clear on release is the correct approach for FRAME-03/FRAME-06 compliance.

### Pattern 2: Threading Pool Through Execution Context

**What:** `ExecContext` gains a `pool: &'a mut RegisterPool` field. All call sites that create or destroy frames receive the pool.

**When to use:** Anywhere a `CallFrame` is created (exec_call, exec_call_virt, exec_call_indirect) or destroyed (execute_ret, execute_crash).

```rust
// In dispatch/mod.rs
pub(super) struct ExecContext<'a> {
    pub task: &'a mut Task,
    pub modules: &'a [LoadedModule],
    pub current_module_idx: usize,
    pub dispatch_table: &'a DispatchTable,
    pub heap: &'a mut dyn GcHeap,
    pub host: &'a mut dyn RuntimeHost,
    pub globals: &'a mut Vec<Value>,
    pub next_request_id: &'a mut u32,
    pub entity_registry: &'a mut EntityRegistry,
    pub pool: &'a mut RegisterPool,  // NEW
}
```

`execute_ret` (standalone function, not on ExecContext) currently takes the same individual parameters as `execute_one`. It must also receive `pool: &mut RegisterPool`.

### Pattern 3: Pool Placement on Scheduler

**What:** `Scheduler` gains a `pool: RegisterPool` field. It is threaded into `run_one_task`, which passes it into `execute_one`, which passes it into `ExecContext`.

```rust
// In scheduler.rs
pub struct Scheduler {
    pub(crate) tasks: FxHashMap<TaskId, Task>,
    pub(crate) ready_queue: VecDeque<TaskId>,
    pub(crate) next_task_index: u32,
    pub(crate) globals: Vec<Value>,
    pub(crate) global_locks: FxHashMap<u32, TaskId>,
    pub(crate) join_waiters: FxHashMap<TaskId, Vec<(TaskId, u16)>>,
    pub(crate) entity_registry: EntityRegistry,
    pub(crate) pool: RegisterPool,  // NEW
}
```

**Why Scheduler (not Task):** The pool's purpose is to share allocations across calls within and between tasks. A per-task pool would not reuse Vecs from completed tasks. A Scheduler-level pool sees all frames from all tasks, maximizing reuse.

**Why not Thread-local:** The codebase is single-threaded cooperative; thread-locals would work but add complexity (`RefCell`, `with()` boilerplate). Scheduler ownership is cleaner and aligns with the project's existing patterns.

### Pattern 4: CallFrame Creation via Pool

Replace `CallFrame::new()` (which always allocates) with a pool-aware creation path:

```rust
// In frame.rs
impl CallFrame {
    /// Create a new call frame, acquiring the register Vec from the pool.
    pub fn with_pool(pool: &mut RegisterPool, method_idx: usize, reg_count: usize, return_register: u16) -> Self {
        Self {
            method_idx,
            pc: 0,
            registers: pool.acquire(reg_count),
            defer_stack: Vec::new(),
            return_register,
        }
    }
}
```

`CallFrame::new()` can remain for use in `Scheduler::create_task` (where the pool reference is available via `&mut self.pool`) and test helpers that don't have a pool.

### Pattern 5: execute_ret Pool Integration

The critical path. After popping the frame, return its registers Vec to the pool:

```rust
fn execute_ret(
    task: &mut Task,
    ret_val: Value,
    pool: &mut RegisterPool,   // NEW parameter
    // ... other params unchanged ...
) -> ExecutionResult {
    // Step 1: Run defers (unchanged)
    // ...

    // Step 2: Fire debug hook, pop frame
    if host.debug_enabled() { ... }
    let mut popped = task.call_stack.pop().unwrap();

    // Step 3: Return registers to pool  ← NEW
    pool.release(popped.registers);
    // Note: popped.registers is now moved into pool.release()

    // Step 4: Deliver result (unchanged)
    if task.call_stack.is_empty() { ... }
    else { ... }
}
```

### Pattern 6: execute_crash Pool Integration

`execute_crash` also pops frames (during unwind). Those Vecs should also be returned to the pool to avoid leaking them during crash scenarios:

```rust
// In execute_crash, the frame-pop loop:
while !task.call_stack.is_empty() {
    // ... run defers ...
    let frame = task.call_stack.pop().unwrap();
    pool.release(frame.registers);  // NEW
}
```

### Anti-Patterns to Avoid

- **Pooling the defer_stack Vec:** The defer stack is typically empty at frame pop time (defers have already run). Not worth pooling — it's a rare, small allocation.
- **Pooling in exec_tail_call:** TailCall reuses the existing frame in-place (`current.registers.clear(); current.registers.resize(...)`). It never pops a frame, so the pool is not involved. Do NOT add pool calls to exec_tail_call.
- **Filling on acquire instead of release:** Filling on acquire requires knowing the old length; filling on release is simpler and correct. Always fill on release.
- **Using `truncate(0)` instead of `clear()`:** They're equivalent for Vec, but `clear()` is idiomatic.
- **Storing the entire CallFrame in the pool:** Only pool the `Vec<Value>` registers field. The frame struct itself is stack-allocated (inside `Vec<CallFrame>`) and not heap-managed.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Vec clearing | Custom unsafe memset | `v.fill(Value::Void)` | stdlib method, compiler generates optimal code |
| Pool capacity check | Separate counter field | `free_list.len()` | Vec tracks its own length, no dual-accounting needed |
| Concurrent pool | Mutex/RwLock | N/A — single-threaded cooperative scheduler | No synchronization needed; adds overhead |

**Key insight:** The VM's cooperative scheduling model means the pool is always accessed single-threaded. No synchronization primitives are needed or appropriate.

## Common Pitfalls

### Pitfall 1: Stale Values in Reused Registers
**What goes wrong:** A register Vec is released without clearing. The next frame that acquires it sees non-Void values in registers that should be zero-initialized. This causes incorrect behavior for any function that reads an uninitialized register.

**Why it happens:** Forgetting to call `fill(Value::Void)` (or doing it after `clear()` instead of before) during release.

**How to avoid:** The release sequence is strictly: `v.fill(Value::Void)` → `v.clear()` → `free_list.push(v)`. FRAME-06's test directly verifies this.

**Warning signs:** Tests with functions that use registers they haven't explicitly initialized start failing or producing garbage results.

### Pitfall 2: Pool Not Reached from execute_crash
**What goes wrong:** `execute_crash` pops frames in a loop but never returns registers to the pool. Under crash scenarios, all the frame Vecs from the crashed task are dropped without pooling, wasting the freed memory. More importantly, the pool grows stale after crashes if it was designed to track outstanding borrows.

**Why it happens:** `execute_crash` is a distinct code path from `execute_ret` and is easy to miss.

**How to avoid:** Add pool.release() calls in the `execute_crash` frame-unwind loop, parallel to the `execute_ret` change.

### Pitfall 3: Borrow Conflicts When Threading Pool Through ExecContext
**What goes wrong:** `ExecContext` borrows `pool: &'a mut RegisterPool` from `Scheduler`. But `Scheduler::run_one_task` also needs `&mut self` to access tasks. Rust's borrow checker may reject simultaneous `&mut self.pool` and `&mut self.tasks`.

**Why it happens:** Mutable borrows of two fields of the same struct via `&mut self` require split-borrow, which Rust supports within a single function but not across function boundaries.

**How to avoid:** Use field split-borrows explicitly in `run_one_task`: borrow `&mut self.pool`, `&mut self.tasks`, etc. separately before passing them into `execute_one`. The existing pattern for `globals` and `entity_registry` demonstrates this is already done (they're passed as `&mut self.globals`, `&mut self.entity_registry`). Adding `&mut self.pool` follows the same pattern.

**Example of the safe pattern (already used in scheduler.rs):**
```rust
execute_one(
    task,
    modules,
    current_module_idx,
    dispatch_table,
    heap,
    host,
    &mut self.globals,        // field borrow — safe
    next_request_id,
    &mut self.entity_registry, // field borrow — safe
    &mut self.pool,            // NEW field borrow — safe
)
```

### Pitfall 4: execute_ret Signature Has Many Parameters — Easy to Miss Pool
**What goes wrong:** `execute_ret` is a standalone free function that mirrors `execute_one`'s parameter set. Adding `pool` to `execute_one` also requires threading it to `execute_ret` (called from the `Ret`/`RetVoid` match arms). The same applies to `execute_crash` and `execute_defer_handler` (which calls `execute_one`).

**Why it happens:** The parameter chain is long; it's easy to add `pool` to `execute_one` and forget the callee functions.

**How to avoid:** Grep for every site that calls `execute_ret` or `execute_crash` after the signature change. Compile errors will catch missed sites.

### Pitfall 5: Pool Scan Complexity on Acquire
**What goes wrong:** The free-list scan is O(n) where n ≤ 64. For the fib benchmark, `reg_count` is always the same small number (fib uses ~3 registers). The scan will hit immediately (the first entry will always have sufficient capacity), so the O(n) concern is irrelevant in practice.

**Why it happens:** Theoretical concern about scan overhead with a large pool.

**How to avoid:** For the common case (reg_count ≤ capacity of most pooled Vecs), scanning from the back (`rev()`) hits a match quickly. The 64-entry cap bounds worst-case to 64 comparisons — negligible.

### Pitfall 6: create_task Also Creates Frames — Must Use Pool
**What goes wrong:** `Scheduler::create_task` calls `CallFrame::new(method_idx, reg_count, 0)` directly. If this path bypasses the pool, task-initial frames never benefit from pooling.

**Why it happens:** `create_task` is separate from the hot path and easy to miss.

**How to avoid:** Change `create_task` to call `CallFrame::with_pool(&mut self.pool, ...)` using the Scheduler's own pool field.

## Code Examples

### Complete RegisterPool Implementation

```rust
// Source: analysis of writ-runtime/src/frame.rs + requirements
use crate::value::Value;

const POOL_CAP: usize = 64;

pub struct RegisterPool {
    free_list: Vec<Vec<Value>>,
}

impl RegisterPool {
    pub fn new() -> Self {
        Self { free_list: Vec::new() }
    }

    #[inline]
    pub fn acquire(&mut self, reg_count: usize) -> Vec<Value> {
        for i in (0..self.free_list.len()).rev() {
            if self.free_list[i].capacity() >= reg_count {
                let mut v = self.free_list.swap_remove(i);
                v.resize(reg_count, Value::Void);
                return v;
            }
        }
        vec![Value::Void; reg_count]
    }

    #[inline]
    pub fn release(&mut self, mut v: Vec<Value>) {
        if self.free_list.len() >= POOL_CAP {
            return;
        }
        v.fill(Value::Void);
        v.clear();
        self.free_list.push(v);
    }
}

impl Default for RegisterPool {
    fn default() -> Self { Self::new() }
}
```

### Pool Correctness Test (FRAME-06)

```rust
// Source: derived from requirements — lives in writ-runtime/tests/vm_tests.rs
// or a new writ-runtime/tests/pool_tests.rs
#[test]
fn pool_reuse_clears_registers() {
    use writ_runtime::frame::RegisterPool;
    use writ_runtime::Value;

    let mut pool = RegisterPool::new();

    // Acquire a Vec, write non-Void values, release it
    let mut v = pool.acquire(4);
    v[0] = Value::Int(99);
    v[1] = Value::Bool(true);
    v[2] = Value::Int(-1);
    v[3] = Value::Float(3.14);
    pool.release(v);

    // Re-acquire — must get back a cleared Vec
    let v2 = pool.acquire(4);
    assert_eq!(v2.len(), 4);
    for (i, val) in v2.iter().enumerate() {
        assert!(
            matches!(val, Value::Void),
            "register {} was not Void after pool reuse: {:?}", i, val
        );
    }
}

#[test]
fn pool_cap_prevents_unbounded_growth() {
    use writ_runtime::frame::RegisterPool;
    use writ_runtime::Value;

    let mut pool = RegisterPool::new();
    // Release 70 Vecs — only 64 should be retained
    for _ in 0..70 {
        pool.release(vec![Value::Void; 4]);
    }
    // Acquire 65 times — 64th should hit pool, 65th should allocate fresh
    let mut acquired = Vec::new();
    for _ in 0..65 {
        acquired.push(pool.acquire(4));
    }
    // All 65 must succeed (either from pool or fresh allocation)
    assert_eq!(acquired.len(), 65);
}
```

### fib(40) Benchmark Verification

The benchmark uses the existing `benchmark/cases/fib/fib.writc` compiled bytecode. The procedure from Phase 76:

```bash
# Release build
cargo build --release

# Run fib(40) three times, take median
# (use writ-runtime binary or the benchmark harness)
# Compare median against Phase 76 result (66.979s)
# Record delta in STATE.md Accumulated Context
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `vec![Value::Void; reg_count]` on every call | Pool-reused Vec, only allocate on miss | Phase 77 | Eliminates per-call heap allocation for recursive functions |
| Drop frame Vec on ret | Clear + return to pool | Phase 77 | Reuses backing storage across calls |

**Prior art in Rust ecosystems:** This pattern is used in tokio's buffer pool, wasm runtimes (Wasmtime uses a similar slab for stacks), and JVM's frame pool. The Rust stdlib `Vec::resize` and `Vec::fill` methods are designed for exactly this use case.

## Open Questions

1. **Should `RegisterPool` be `pub` or `pub(crate)`?**
   - What we know: It's tested from integration tests (tests/ directory), which require `pub` visibility on the type itself.
   - What's unclear: Whether to expose the full API publicly or only what tests need.
   - Recommendation: Make `RegisterPool` and its `acquire`/`release` methods `pub` so integration tests can construct one directly. `POOL_CAP` constant can be `pub(crate)`.

2. **Does `execute_defer_handler` need the pool?**
   - What we know: `execute_defer_handler` calls `execute_one` recursively to run defer code. If `execute_one` calls frames, those calls will use the pool. `execute_defer_handler` itself does not pop frames — it runs within an existing frame. No pool threading needed here beyond what `execute_one` already receives.
   - Recommendation: Thread pool to `execute_defer_handler` only if it calls `execute_ret` internally (it does not — defers end on `DeferComplete`). So no change needed to `execute_defer_handler`'s signature.

3. **Should the pool scan prefer largest-capacity or most-recent?**
   - What we know: Scanning from back (`rev()`) prefers most-recently-released, which has good cache locality. Fib uses uniform reg_count, so any entry matches.
   - Recommendation: Scan from back. Simple and cache-friendly.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) |
| Config file | none (workspace Cargo.toml) |
| Quick run command | `cargo test -p writ-runtime --release` |
| Full suite command | `cargo test --release` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FRAME-01 | RegisterPool::new(), acquire(), release() exist and compile | unit | `cargo test -p writ-runtime --release pool` | ❌ Wave 0 |
| FRAME-02 | acquire reuses Vec with sufficient capacity | unit | `cargo test -p writ-runtime --release pool_reuse` | ❌ Wave 0 |
| FRAME-03 | release fills with Void before storing | unit | `cargo test -p writ-runtime --release pool_reuse_clears` | ❌ Wave 0 |
| FRAME-04 | pool capped at 64 entries | unit | `cargo test -p writ-runtime --release pool_cap` | ❌ Wave 0 |
| FRAME-05 | execute_ret returns Vec to pool | integration | `cargo test -p writ-runtime --release` (all existing vm_tests pass) | ✅ existing |
| FRAME-06 | correctness test: reused regs are Void | unit | `cargo test -p writ-runtime --release pool_correctness` | ❌ Wave 0 |
| VERIFY-01 | fib(40) = 102334155 | smoke | `cargo test -p writ-runtime --release fib` (or manual run) | ✅ existing |
| VERIFY-02 | full suite passes | regression | `cargo test --release` | ✅ existing |
| VERIFY-03 | zero warnings | build | `cargo build --release 2>&1 \| grep warning` | ✅ existing |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime --release`
- **Per wave merge:** `cargo test --release`
- **Phase gate:** Full suite green + fib(40) faster than 66.979s before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] Pool tests — unit tests for `RegisterPool` (FRAME-01 through FRAME-06). Likely added to a new `writ-runtime/tests/pool_tests.rs` or appended to `vm_tests.rs`.

## Sources

### Primary (HIGH confidence)
- Direct source inspection: `writ-runtime/src/frame.rs` — CallFrame struct; `Vec<Value>` as the register store
- Direct source inspection: `writ-runtime/src/dispatch/mod.rs` — `execute_ret` function, `ExecContext` struct, frame pop pattern
- Direct source inspection: `writ-runtime/src/scheduler.rs` — Scheduler struct fields, field-borrow pattern for globals and entity_registry
- Direct source inspection: `writ-runtime/src/dispatch/calls.rs` — exec_call, exec_call_virt, exec_call_indirect, exec_tail_call implementations
- Direct source inspection: `.planning/REQUIREMENTS.md` — FRAME-01 through FRAME-06 exact requirements
- `.planning/STATE.md` — Phase 76 fib(40) median 66.979s is the performance baseline for this phase

### Secondary (MEDIUM confidence)
- Phase 76 pattern: `exec_tail_call` already uses in-place Vec reuse (`clear + resize`) — confirms Vec reuse is safe and correct in this codebase

### Tertiary (LOW confidence)
- General Rust ecosystem knowledge: pool pattern with `Vec<Vec<T>>` free-list is well-established

## Metadata

**Confidence breakdown:**
- RegisterPool design: HIGH — derived directly from source inspection; no ambiguity
- Threading pattern: HIGH — mirrors existing field-borrow pattern for globals/entity_registry
- Pitfalls: HIGH — derived from actual code analysis (execute_crash gap, borrow checker concern)
- Performance estimate: MEDIUM — Phase 76 eliminated zero-copy arg passing; pool eliminates the one remaining per-call allocation; speedup likely 10-25% on fib(40)

**Research date:** 2026-03-22
**Valid until:** 2026-04-22 (stable internal domain, no external dependencies)
