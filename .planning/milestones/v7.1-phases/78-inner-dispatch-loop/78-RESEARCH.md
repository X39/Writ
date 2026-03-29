# Phase 78: Inner Dispatch Loop - Research

**Researched:** 2026-03-22
**Domain:** Rust VM batch execution, borrow-checker-safe task dispatch
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
| DISPATCH-01 | execute_batch function runs multiple instructions without returning to scheduler | New function in dispatch/mod.rs; holds `&mut Task` across the batch loop |
| DISPATCH-02 | execute_batch terminates batch on any non-Continue ExecutionResult | Match on result inside the batch loop; break on any non-Continue variant |
| DISPATCH-03 | execute_batch respects ExecutionLimit (clamps batch size to remaining limit) | Accept `limit: u64` (0 = unlimited); count instructions and stop at limit |
| DISPATCH-04 | execute_batch falls back to single-instruction dispatch when debug hooks are enabled | Check `host.debug_enabled()` at batch entry; delegate to existing `execute_one` loop if true |
| DISPATCH-05 | Frame reference is re-acquired after any stack-changing instruction (Call, Ret, TailCall) | After Call/Ret/TailCall variants return Continue, the next iteration of the batch loop fetches a fresh `task.call_stack.last_mut()` — no stale pointer survives between iterations |
| VERIFY-01 | fib(40) produces correct output 102334155 | No semantics change; fib(40) = 102334155 must still hold |
| VERIFY-02 | cargo test --release passes with zero failures | All existing tests unaffected; new batch tests added |
| VERIFY-03 | cargo build --release produces no warnings | No dead code, unused imports, or missing inline annotations |
</phase_requirements>

## Summary

Phase 78 introduces `execute_batch`, an inner dispatch loop that amortizes the per-instruction cost of the outer scheduler loop (one `FxHashMap` task lookup per instruction in `run_one_task`). Currently, `run_one_task` holds a task reference for exactly one instruction, releases it, returns to the outer loop, does a `self.tasks.get_mut(&task_id)` lookup, and re-enters dispatch. For fib(40) with ~300M instructions this is ~300M redundant HashMap lookups.

The fix is a batch loop that holds `&mut Task` continuously across N instructions, only breaking on non-`Continue` results (Crash, Completed, Suspended, DebugSuspend, LimitReached, spawn/join/cancel variants) or when the instruction budget is exhausted. When a DAP debugger is connected (`host.debug_enabled() == true`), the function falls back to single-instruction dispatch to preserve the per-instruction `before_instruction` hook behavior. `execute_batch` replaces the inner loop body in `run_one_task`.

The key design question raised in STATE.md is whether to use an `ExecContext` extension vs. a `BatchContext` wrapper. The answer is clear from source inspection: `execute_batch` is placed directly in `dispatch/mod.rs` alongside `execute_one`, takes the same parameter set as `execute_one`, and is called from `run_one_task` in place of the inner `execute_one` call. No new wrapper struct is needed — the existing per-parameter threading pattern is sufficient and avoids introducing new types.

**Primary recommendation:** Add `execute_batch` to `dispatch/mod.rs` with the same parameter signature as `execute_one`. Modify `run_one_task` in `scheduler.rs` to call `execute_batch` instead of the single-instruction `execute_one` loop. Fall back to `execute_one` internally when `host.debug_enabled()`.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust stdlib (no additions) | built-in | Batch loop control flow | Everything needed is already present in the codebase |
| `writ_runtime::dispatch::execute_one` | project | Single-instruction fallback when debug is active | Already the hot path; `execute_batch` wraps it |

No new external dependencies.

### Supporting
None. This is a pure restructuring of existing code.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Free function `execute_batch` in dispatch/mod.rs | Method on `ExecContext` | ExecContext is per-instruction (short lifetime); batch needs a longer scope. Free function mirrors `execute_one`'s placement exactly. |
| Replace `execute_one` call site in `run_one_task` | Inline batch logic in `run_one_task` | Keeping batch logic in dispatch/mod.rs preserves the module boundary; scheduler.rs stays thin |
| New `BatchContext` wrapper struct | Pass same individual parameters as `execute_one` | A wrapper adds a new type with no benefit; the existing 10-param pattern is already established and working |

## Architecture Patterns

### Recommended Project Structure

```
writ-runtime/src/
├── scheduler.rs      # run_one_task: replace single-instruction loop with execute_batch call
├── dispatch/
│   └── mod.rs        # add execute_batch; execute_one remains unchanged
└── (no other files change)
```

### Pattern 1: execute_batch Signature and Placement

`execute_batch` sits in `dispatch/mod.rs` alongside `execute_one`. It has the same parameter signature plus a `limit: u64` (0 = unlimited). It returns the same `ExecutionResult` as `execute_one`.

```rust
// Source: derived from execute_one signature in dispatch/mod.rs
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_batch(
    task: &mut Task,
    modules: &[LoadedModule],
    current_module_idx: usize,
    dispatch_table: &DispatchTable,
    heap: &mut dyn GcHeap,
    host: &mut dyn RuntimeHost,
    globals: &mut Vec<Value>,
    next_request_id: &mut u32,
    entity_registry: &mut EntityRegistry,
    pool: &mut RegisterPool,
    limit: u64,
) -> ExecutionResult {
    // Debug path: fall back to single-instruction dispatch
    if host.debug_enabled() {
        return execute_one(task, modules, current_module_idx, dispatch_table,
                          heap, host, globals, next_request_id, entity_registry, pool);
    }

    let effective_limit = if limit == 0 { u64::MAX } else { limit };
    let mut executed: u64 = 0;

    loop {
        if executed >= effective_limit {
            return ExecutionResult::LimitReached;
        }
        let result = execute_one(task, modules, current_module_idx, dispatch_table,
                                heap, host, globals, next_request_id, entity_registry, pool);
        executed += 1;
        match result {
            ExecutionResult::Continue => continue,
            other => return other,
        }
    }
}
```

**Important:** This shape (calling `execute_one` internally) is the safe starting point. It eliminates the per-instruction HashMap lookup because `task` is already held as `&mut Task` — there is no re-lookup between iterations. The reason the existing loop in `run_one_task` does a HashMap lookup each iteration is that it needs `&mut Task` from `self.tasks.get_mut(&task_id)`. By passing `task` directly into `execute_batch`, the borrow lives for the entire batch.

### Pattern 2: run_one_task Restructuring

The current `run_one_task` loop does:
1. Check limit (with a `self.tasks.get(&task_id)` lookup)
2. `self.tasks.get_mut(&task_id)` to get the task reference
3. Call `execute_one(task, ...)`
4. `instructions_run += 1`
5. Match on result, potentially doing more `self.tasks.get_mut(&task_id)` lookups

After this phase, `run_one_task` extracts the task reference once, calls `execute_batch`, and matches on the single result:

```rust
// Simplified run_one_task inner logic (after Phase 78)
let task = self.tasks.get_mut(&task_id)?;
let result = execute_batch(
    task,
    modules,
    current_module_idx,
    dispatch_table,
    heap,
    host,
    &mut self.globals,
    next_request_id,
    &mut self.entity_registry,
    &mut self.pool,
    per_task_limit,
);
// One match on result — no per-instruction HashMap lookups
```

The tricky part: some `ExecutionResult` variants in the current `run_one_task` match arms cause further scheduler operations (spawn child, join, cancel) that need `&mut self`. These variants will continue to be handled in `run_one_task`'s match on `execute_batch`'s return value. The key insight is that `execute_batch` returns immediately on any non-Continue result, so by the time the scheduler handles SpawnChild/JoinTask/CancelTask, the task borrow from `execute_batch` has already been released.

### Pattern 3: DISPATCH-05 — Frame Re-acquisition is Automatic

DISPATCH-05 requires that the frame reference is re-acquired after stack-changing instructions (Call, Ret, TailCall). This is already satisfied by the design: each call to `execute_one` fetches `task.call_stack.last_mut()` at the start of `execute_one`. There is no cached frame reference between `execute_one` invocations in the batch loop. The frame is fetched fresh on every call to `execute_one`, so Call/Ret/TailCall that modify the call stack are immediately reflected on the next iteration.

No special code is needed for DISPATCH-05. It is satisfied automatically by calling `execute_one` per iteration rather than passing a frame reference into the batch.

### Pattern 4: Limit Accounting

`execute_batch` accepts `limit: u64` (mirrors `run_one_task`'s existing `limit: u64` convention where 0 means unlimited). Inside the batch:

- If `limit == 0`: run until a non-Continue result, no counter check
- If `limit > 0`: count instructions; return `LimitReached` when count reaches limit

The `atomic_depth` check (don't yield mid-atomic-section) is currently in `run_one_task`. For the batch, the simplest approach: only clamp at the start if we already know we're at the limit — or check inside the loop. The current code in `run_one_task` checks `task.atomic_depth == 0` before comparing `instructions_run >= limit`. This check must be preserved in the batch loop to avoid yielding mid-atomic.

```rust
// Limit check with atomic awareness:
if limit > 0 && executed >= limit && task.atomic_depth == 0 {
    return ExecutionResult::LimitReached;
}
```

### Pattern 5: Debug Fallback (DISPATCH-04)

When `host.debug_enabled()` is true, `execute_batch` runs exactly one instruction via `execute_one` and returns its result. This means the outer `run_one_task` loop continues to call `execute_batch` on each outer iteration, but `execute_batch` delegates to a single `execute_one`. The behavior is functionally identical to the pre-Phase-78 single-instruction loop, preserving all `before_instruction` hook semantics.

The test for DISPATCH-04 constructs a host with `debug_enabled() = true` and verifies that `execute_batch` called with a large limit still executes only one instruction per outer-loop call (or equivalently, that the DAP breakpoint fires correctly).

### Anti-Patterns to Avoid

- **Caching `task.call_stack.last_mut()` across `execute_one` calls:** Call/Ret/TailCall modify the stack. Never hold a frame reference across batch iterations.
- **Inlining the instruction match in `execute_batch`:** The full 90-arm match is already in `execute_one`. Duplicating it in a batch function would double maintenance burden. Call `execute_one` from the batch loop instead.
- **Adding `#[inline]` to `execute_batch`:** Like `execute_one`, `execute_batch` is large enough (it contains `execute_one` in its body) that inlining would bloat callers. Leave it without `#[inline]`.
- **Handling concurrency results (SpawnChild, JoinTask, CancelTask) inside `execute_batch`:** These require scheduler state (`self.tasks`, `self.ready_queue`, etc.) that is not available inside `execute_batch`. Return them to `run_one_task` unchanged, where the existing match arms handle them. This means `execute_batch` terminates the batch on any concurrency result.
- **Moving the `instructions_run` counter from `run_one_task` to only inside `execute_batch`:** `run_one_task` still needs to know how many instructions ran for TickResult classification. Return the count, or restructure `run_one_task` to not need it (currently it doesn't actually use `instructions_run` for TickResult — only for the limit check, which moves into `execute_batch`).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Per-instruction HashMap elimination | Custom unsafe pointer tricks | Hold `&mut Task` through the batch naturally via parameter passing | Rust's borrow checker already allows this; no unsafe needed |
| Batch frame management | Separate "batch frame cache" struct | Re-acquire via `task.call_stack.last_mut()` on each `execute_one` call | Already safe; call stack is `Vec<CallFrame>` which doesn't reallocate under Call/Ret because Phase 76/77 made that efficient |
| Instruction counting | AtomicU64 or separate counter struct | Plain `u64` local variable in the batch loop | Single-threaded cooperative model; no synchronization needed |

**Key insight:** The entire optimization is structural — by not returning to the outer scheduler loop between instructions, the per-instruction HashMap lookup is eliminated. No changes to the instruction set, handler implementations, or data structures are required.

## Common Pitfalls

### Pitfall 1: Borrow Conflict Between Task Reference and Scheduler State
**What goes wrong:** `execute_batch` holds `&mut Task`. Inside `run_one_task`, this comes from `self.tasks.get_mut(&task_id)`. Rust's borrow checker rejects holding this borrow while also accessing `self.ready_queue`, `self.globals`, etc.

**Why it happens:** `self.tasks` is a field of `Scheduler`. Borrowing `&mut task` from `self.tasks` and simultaneously mutating `self.ready_queue` (also a field of `self`) looks like two mutable borrows of `self`.

**How to avoid:** Extract the task reference into a local variable before calling `execute_batch`. Pass other scheduler fields (`&mut self.globals`, `&mut self.entity_registry`, `&mut self.pool`) individually, not via `&mut self`. The borrow checker accepts simultaneous mutable borrows of disjoint struct fields when accessed by field name (not through `self`). This is exactly the pattern already used for `globals`, `entity_registry`, and `pool` in the current `run_one_task`.

**Concretely:** After `execute_batch` returns (with a non-Continue result), the `task` borrow is released. Only then can `run_one_task` do `self.tasks.get_mut(&task_id)` for state updates (setting `task.state = TaskState::Ready`, etc.).

**Warning signs:** Compile error "cannot borrow `self.tasks` as mutable more than once" or "cannot borrow `*self` while it is already borrowed".

### Pitfall 2: Handling the LimitReached Loop Exit Incorrectly
**What goes wrong:** The current `run_one_task` loop checks the limit at the TOP of each iteration (before calling `execute_one`). If `execute_batch` checks only at the bottom (after `execute_one`), it could execute one extra instruction beyond the limit.

**Why it happens:** Off-by-one in counter check placement.

**How to avoid:** Check the limit at the TOP of the batch loop before calling `execute_one`, matching the existing semantics. Specifically: if `limit > 0 && executed >= limit && task.atomic_depth == 0 { return ExecutionResult::LimitReached; }` goes before the `execute_one` call.

**Warning signs:** The DISPATCH-03 test (limit not overshot) fails.

### Pitfall 3: Concurrency Results Must Escape the Batch
**What goes wrong:** SpawnChild, SpawnDetachedTask, JoinTask, CancelTask variants need scheduler operations that cannot happen inside `execute_batch`. If the batch loop handles them internally (e.g., ignoring them with `continue`), tasks won't be spawned or joined correctly.

**Why it happens:** Copy-pasting the `run_one_task` match arms into `execute_batch`.

**How to avoid:** `execute_batch` treats every non-Continue result as a batch terminator and returns it immediately. The caller (`run_one_task`) handles all variants exactly as before. The batch simply reduces how often we exit to the scheduler — not whether we exit when needed.

### Pitfall 4: execute_batch Called With Stale atomic_depth
**What goes wrong:** If `execute_batch` is called when `task.atomic_depth > 0`, the limit check must not yield (can't break out of an atomic section). The batch must continue executing until either the section ends (AtomicEnd brings depth to 0) or a non-limit non-Continue result occurs.

**Why it happens:** The limit check doesn't account for atomic depth.

**How to avoid:** Mirror the existing `run_one_task` condition: `if limit > 0 && executed >= limit && task.atomic_depth == 0`. This is already the semantics; preserve it in the batch.

### Pitfall 5: Missing Benchmark Delta Recording
**What goes wrong:** The DISPATCH-01 through DISPATCH-05 requirements are satisfied but the fib(40) timing delta (success criterion 4) is not recorded in BASELINE.md.

**Why it happens:** Implementation focus on code correctness; benchmark step overlooked.

**How to avoid:** After `cargo test --release` passes, run the three-measurement fib(40) protocol and append the Phase 78 section to `benchmark/BASELINE.md` before closing the phase.

## Code Examples

### execute_batch Implementation

```rust
// Source: derived from dispatch/mod.rs execute_one and scheduler.rs run_one_task
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_batch(
    task: &mut Task,
    modules: &[LoadedModule],
    current_module_idx: usize,
    dispatch_table: &DispatchTable,
    heap: &mut dyn GcHeap,
    host: &mut dyn RuntimeHost,
    globals: &mut Vec<Value>,
    next_request_id: &mut u32,
    entity_registry: &mut EntityRegistry,
    pool: &mut RegisterPool,
    limit: u64,
) -> ExecutionResult {
    // DISPATCH-04: debug path — single instruction to preserve per-instruction hooks
    if host.debug_enabled() {
        return execute_one(task, modules, current_module_idx, dispatch_table,
                          heap, host, globals, next_request_id, entity_registry, pool);
    }

    let mut executed: u64 = 0;

    loop {
        // DISPATCH-03: respect limit, with atomic-section awareness
        if limit > 0 && executed >= limit && task.atomic_depth == 0 {
            return ExecutionResult::LimitReached;
        }

        let result = execute_one(task, modules, current_module_idx, dispatch_table,
                                heap, host, globals, next_request_id, entity_registry, pool);
        executed += 1;

        match result {
            // DISPATCH-02: continue only on Continue; terminate on everything else
            ExecutionResult::Continue => continue,
            other => return other,
        }
    }
}
```

### run_one_task After Phase 78

The outer loop in `run_one_task` collapses from a per-instruction loop to a call to `execute_batch` with its result matched against the full `ExecutionResult` set. The task borrow is extracted once before the `execute_batch` call and is not re-borrowed inside it:

```rust
// Outline only — full match arms remain identical to the current run_one_task
{
    let task = self.tasks.get_mut(&task_id)?;
    // task borrow is held for the entire batch
    let result = execute_batch(
        task, modules, current_module_idx, dispatch_table, heap, host,
        &mut self.globals, next_request_id, &mut self.entity_registry,
        &mut self.pool, limit,
    );
    // task borrow released here

    // Scheduler state mutations happen after the borrow is released:
    match result {
        // ... existing match arms unchanged ...
    }
}
// NOTE: The outer loop in run_one_task is now driven by execute_batch's non-Continue returns.
// For concurrency results (SpawnChild, JoinTask, etc.), run_one_task re-enters the loop
// (just as it did before), calling execute_batch again for the same task.
```

**Critical detail:** The current `run_one_task` uses a `loop` that `continue`s on concurrency results. After Phase 78, `execute_batch` handles the inner Continue loop; the outer `run_one_task` loop only iterates on concurrency results (which require scheduler-level handling). This is a correct and important distinction.

### Test for DISPATCH-04 (Debug Fallback)

```rust
// In writ-runtime/tests/dispatch_tests.rs (new file) or appended to vm_tests.rs
// Build a debug host that records before_instruction calls.
// Verify that with limit=1000 but debug enabled, execute_batch only runs one instruction.
struct CountingDebugHost { instruction_count: usize }
impl RuntimeHost for CountingDebugHost {
    fn debug_enabled(&self) -> bool { true }
    fn before_instruction(&mut self, ...) -> DebugAction {
        self.instruction_count += 1;
        DebugAction::Continue
    }
    // ... other methods ...
}
// Test: spawn fib(1), run with execute_batch(limit=1000, debug=true).
// The batch returns after exactly one instruction because debug_enabled() is true.
// The outer loop in run_one_task continues until task completes.
// Assert: task completes with correct result; debug hooks fired for every instruction.
```

### Test for DISPATCH-03 (Limit Not Overshot)

```rust
// Spawn a task that runs many instructions.
// Call runtime.run_task(task_id, ExecutionLimit::Instructions(10)).
// Assert: exactly 10 instructions executed, task is still Ready/Running.
// Assert: task.instructions_executed == 10 (or check via some observable state).
```

### fib(40) Benchmark Protocol (same as Phase 76/77)

```bash
# Release build
cargo build --release

# Run fib(40) three times and record times
# Append Phase 78 section to benchmark/BASELINE.md
# Compare median against Phase 77 result (59.800s)
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Per-instruction `self.tasks.get_mut(&task_id)` lookup in `run_one_task` | Single lookup per batch; task held for N instructions | Phase 78 | Eliminates ~300M HashMap lookups for fib(40) |
| Single-instruction dispatch path always active | Batch path for non-debug; single-instruction path preserved for DAP | Phase 78 | Zero overhead on debug path; batch speedup on hot path |

**Prior context:** Quick-task 260322-6om already eliminated instruction cloning and the per-instruction scheduler HashMap lookup in a different way (passed task directly into execute_one). Phase 78 formalizes and extends this by introducing the explicit batch loop with limit accounting, DAP fallback, and proper test coverage.

## Open Questions

1. **Where does `instructions_run` accounting go?**
   - What we know: The current `run_one_task` increments `instructions_run` after each `execute_one` call and uses it for limit checking. After Phase 78, limit checking moves inside `execute_batch`.
   - What's unclear: Does `run_one_task` need `instructions_run` for anything else? Currently it does not — it's only used for the limit comparison.
   - Recommendation: Remove `instructions_run` from `run_one_task` entirely. The limit logic lives in `execute_batch`. The `task.instructions_executed` field (incremented inside `execute_one`) remains for external instrumentation.

2. **Should `execute_batch` accept the limit from `run_one_task` directly, or should it compute remaining budget?**
   - What we know: The current `run_one_task` passes `limit` (per-task instructions allowed per tick) and tracks `instructions_run` locally to compute remaining budget. With a batch, the batch knows how many it ran.
   - Recommendation: Pass `limit` directly as the absolute batch size. `execute_batch` counts from 0 up to `limit`. This is simpler than passing a "remaining" counter and correct for the current single-batch-per-task-per-tick pattern.

3. **Should concurrency results cause `run_one_task` to re-enter `execute_batch` for the same task?**
   - What we know: Currently, SpawnChild/SpawnDetachedTask return `continue` in the outer loop (task keeps running after spawn). After Phase 78, these results return from `execute_batch`, and `run_one_task` must decide whether to re-enter `execute_batch` for the same task.
   - Recommendation: Yes — preserve the existing `continue` behavior for concurrency results by having `run_one_task` re-enter the batch loop after handling spawn/join/cancel. This is identical to the current behavior; the task continues running in its current tick. The per-task `limit` is passed unchanged (or could be reduced by the count executed so far — either is correct, but simplicity favors passing it unchanged since concurrency operations are rare).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) |
| Config file | none (workspace Cargo.toml) |
| Quick run command | `cargo test -p writ-runtime --release` |
| Full suite command | `cargo test --release` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DISPATCH-01 | execute_batch runs multiple instructions holding &mut Task | integration | `cargo test -p writ-runtime --release execute_batch` | ❌ Wave 0 |
| DISPATCH-02 | execute_batch terminates on non-Continue result | unit | `cargo test -p writ-runtime --release batch_terminates_on_crash` | ❌ Wave 0 |
| DISPATCH-03 | execute_batch does not overshoot ExecutionLimit | integration | `cargo test -p writ-runtime --release batch_respects_limit` | ❌ Wave 0 |
| DISPATCH-04 | execute_batch falls back to single-instruction when debug enabled | integration | `cargo test -p writ-runtime --release batch_debug_fallback` | ❌ Wave 0 |
| DISPATCH-05 | Frame re-acquired after Call/Ret/TailCall | structural (compiler-verified) | `cargo build --release` (no unsafe pointer caching possible) | N/A — guaranteed by design |
| VERIFY-01 | fib(40) = 102334155 | smoke | existing `cargo test -p writ-runtime --release fib` | ✅ existing |
| VERIFY-02 | full suite passes | regression | `cargo test --release` | ✅ existing |
| VERIFY-03 | zero warnings | build | `cargo build --release` | ✅ existing |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime --release`
- **Per wave merge:** `cargo test --release`
- **Phase gate:** Full suite green + fib(40) faster than Phase 77 result (59.800s) before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-runtime/tests/dispatch_tests.rs` — new file with DISPATCH-01 through DISPATCH-04 tests (or append to `vm_tests.rs`)
- [ ] `execute_batch` function does not yet exist in `dispatch/mod.rs`

## Sources

### Primary (HIGH confidence)
- Direct source inspection: `writ-runtime/src/scheduler.rs` — `run_one_task` inner loop; per-instruction HashMap lookup pattern; `instructions_run` counter; concurrency result handling
- Direct source inspection: `writ-runtime/src/dispatch/mod.rs` — `execute_one` signature (10 parameters + return type); `ExecContext` struct; atomic_depth check pattern; `ExecutionResult` variants
- Direct source inspection: `writ-runtime/src/dispatch/calls.rs` — exec_call/exec_call_virt/exec_call_indirect stack manipulation (confirms DISPATCH-05 is structurally guaranteed)
- Direct source inspection: `writ-runtime/src/host.rs` — `debug_enabled()` method on `RuntimeHost`; `before_instruction` hook; `DebugAction` variants
- Direct source inspection: `writ-runtime/src/task.rs` — `atomic_depth: u32` field; `instructions_executed: u64` field
- Direct source inspection: `.planning/REQUIREMENTS.md` — DISPATCH-01 through DISPATCH-05 exact requirement text
- Direct source inspection: `benchmark/BASELINE.md` — Phase 77 fib(40) median 59.800s is the performance baseline for this phase
- Direct source inspection: `.planning/STATE.md` — Research flag about ExecContext extension vs BatchContext wrapper

### Secondary (MEDIUM confidence)
- Phase 77 research pattern: same parameter-threading approach (individual field borrows) applies here
- Quick-task 260322-6om precedent: passing task directly into execute_one already demonstrated that the per-instruction HashMap lookup is the bottleneck

### Tertiary (LOW confidence)
- General VM optimization knowledge: interpreter batch/superblock dispatch is a well-known technique; the specific speedup for this workload is estimated at 5-15% on fib(40) based on the proportion of time spent in scheduler overhead vs. instruction execution

## Metadata

**Confidence breakdown:**
- execute_batch design: HIGH — derived directly from source inspection; no ambiguity in signature or placement
- run_one_task restructuring: HIGH — clear from reading run_one_task; borrow patterns already established in Phase 77
- Pitfalls: HIGH — derived from actual code analysis of borrow conflicts, atomic depth, concurrency results
- Performance estimate: MEDIUM — prior phases removed larger bottlenecks; HashMap elimination is smaller in magnitude; 5-15% estimate is speculative

**Research date:** 2026-03-22
**Valid until:** 2026-04-22 (stable internal domain, no external dependencies)
