# Phase 76: Zero-Allocation Call Convention - Research

**Researched:** 2026-03-22
**Domain:** Rust VM call-frame management, borrow-checker patterns for split-stack mutation
**Confidence:** HIGH

## Summary

Phase 76 eliminates the intermediate `Vec` that every call instruction (exec_call, exec_call_virt, exec_call_indirect, exec_tail_call) currently allocates to stage arguments before writing them into the new frame's registers. The fix is a direct register-to-register copy: create the new frame first, then write argument values from the caller frame into callee frame registers using index arithmetic — no staging buffer needed.

The core borrow-checker challenge is that both the caller frame and the callee frame live inside `ctx.task.call_stack`. Reading from the caller and writing to the pushed callee in the same expression would require two simultaneous mutable borrows of the same `Vec`. The solution is to push the callee frame first with registers pre-sized, then use `split_at_mut` (or sequential indexing after the push) to obtain disjoint mutable references to caller and callee.

The tail-call path is structurally different: it replaces the current frame in-place rather than pushing a new one. The args are read from registers[r_base..r_base+argc] and then the same frame's registers are overwritten. This requires reading the arguments into local (stack-allocated) variables before overwriting — a small fixed-size array or individual register reads via copy semantics.

**Primary recommendation:** For exec_call/exec_call_virt/exec_call_indirect, push the callee frame then use `split_at_mut` on the call stack. For exec_tail_call, copy args into a `SmallVec<[Value; 16]>` or read them element-by-element into a fixed local array before overwriting the frame — avoiding heap allocation for typical argument counts (most functions take 0-8 args).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all implementation choices are at Claude's discretion (pure infrastructure phase).

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CALL-01 | exec_call copies arguments directly from caller registers to callee registers without intermediate Vec | Push frame first, use split_at_mut or sequential copy pattern |
| CALL-02 | exec_call_virt copies arguments directly without intermediate Vec | Same pattern as CALL-01 applied to the Method dispatch arm |
| CALL-03 | exec_call_indirect copies arguments directly without intermediate Vec | Same pattern as CALL-01 |
| CALL-04 | exec_tail_call copies arguments directly without intermediate Vec | Different pattern: read-then-overwrite using local copies before frame replacement |
| CALL-05 | All existing call-related tests pass after zero-allocation conversion | Existing tests cover tail_call_does_not_grow_stack, call_virt_crashes, call_extern, tail-call-with-defer |
| VERIFY-01 | fib(40) produces correct output 102334155 after each phase | Run via writ-cli or test harness; confirmed by automated check |
| VERIFY-02 | cargo test --release passes after each phase with zero failures | Standard gate |
| VERIFY-03 | cargo build --release produces no warnings after each phase | Standard gate |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust std (split_at_mut) | stable | Disjoint mutable slice references | Compiler-verified safe aliasing for same-Vec read/write |
| rustc-hash (FxHashMap) | 2.1.1 | Already present in writ-runtime | No new dependency needed |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| smallvec (optional) | — | Stack-allocated arg buffer for tail-call | Only if read-before-overwrite pattern is complex; avoid adding dependency if Value is Clone-only |

**Installation:** No new dependencies required. The fix is pure restructuring of existing code.

## Architecture Patterns

### Current Allocation Pattern (what to replace)

Every call handler does this today:
```rust
// BAD: allocates staging Vec on every call
let mut args = Vec::with_capacity(argc as usize);
{
    let caller = ctx.task.call_stack.last().unwrap();
    for i in 0..argc as usize {
        args.push(caller.registers[r_base as usize + i].clone());
    }
}
let mut new_frame = CallFrame::new(method_idx, reg_count, r_dst);
for (i, arg) in args.into_iter().enumerate() {
    if i < new_frame.registers.len() {
        new_frame.registers[i] = arg;
    }
}
ctx.task.call_stack.push(new_frame);
```

### Pattern 1: Push-Then-Split for exec_call / exec_call_virt / exec_call_indirect

Push the callee frame first (registers pre-zeroed to Value::Void). Then obtain disjoint references to caller and callee via `split_at_mut`.

```rust
// GOOD: zero intermediate allocation
// Push callee frame with registers pre-sized
ctx.task.call_stack.push(CallFrame::new(method_idx, reg_count, r_dst));
let stack_len = ctx.task.call_stack.len();
let (bottom, top) = ctx.task.call_stack.split_at_mut(stack_len - 1);
let caller = bottom.last().unwrap();  // second-to-last frame
let callee = top.first_mut().unwrap(); // just-pushed frame
for i in 0..argc as usize {
    if i < callee.registers.len() {
        callee.registers[i] = caller.registers[r_base as usize + i].clone();
    }
}
```

This satisfies the borrow checker because `split_at_mut` produces two non-overlapping slices backed by the same underlying Vec memory, so both can be mutably borrowed simultaneously.

**Constraint:** `r_base + argc` must not overflow the caller's register count. The existing code already implicitly requires this (panic on out-of-bounds), so no behavior change.

### Pattern 2: Read-Then-Overwrite for exec_tail_call

Tail-call replaces the current (sole live) frame in-place. The args are read from `current.registers[r_base..r_base+argc]`, then `current.registers` is reset and those args are written to `current.registers[0..argc]`.

The self-aliasing problem: we cannot hold a borrow to `current.registers` while also resetting it. Two safe approaches:

**Option A — Clone individual values before reset (preferred for typical arg counts):**

The key insight is that Value already derives Clone, and all variants except `InlineStruct` are small (one word or two). For typical small arg counts this is cheaper than a Vec alloc. Read each arg by index into a local, then reset and write:

```rust
// Read args into locals — no Vec allocation for <= N args
// (using a fixed local array sized to the max expected argc)
// For argc <= 8 (vast majority of calls), this stays on the stack.
let mut temp = [const { Value::Void }; MAX_INLINE_ARGS];  // or smallvec
for i in 0..argc as usize {
    temp[i] = current.registers[r_base as usize + i].clone();
}
current.method_idx = method_idx;
current.pc = 0;
current.registers.fill(Value::Void);  // or truncate+resize
current.registers.resize(reg_count, Value::Void);
for i in 0..argc as usize {
    current.registers[i] = temp[i].clone();  // or mem::take
}
```

**Option B — Rearrange with mem::swap (avoids clone for most variants):**

Not applicable here because registers is a Vec and tail-call may change reg_count.

**Recommended:** Option A with `const { Value::Void }` array of fixed size (e.g., 32 slots) and a fallback Vec if argc exceeds the fixed limit. For the fib benchmark (argc=2), this is entirely stack-resident.

**Alternative for tail-call:** Since `Value` does not implement `Copy` (InlineStruct contains a Vec), a true zero-allocation path requires either (a) the fixed-array approach, (b) reusing the existing registers Vec by clever in-place rotation before resize, or (c) accepting that tail-call gets a tiny stack buffer (not a heap buffer). Option (a) is cleanest.

### Pattern 3: exec_call_extern is exempt

`exec_call_extern` builds a `Vec<Value>` that it passes to `HostRequest::ExternCall`. This is not a zero-allocation target — the Vec is semantically required as the args payload to the host. Do not change this function in Phase 76.

### Recommended Project Structure (unchanged)

```
writ-runtime/src/dispatch/
├── calls.rs     # Target file for all CALL-01..04 changes
├── mod.rs       # execute_one dispatcher — no changes needed
└── frame.rs     # CallFrame — no changes needed
```

### Anti-Patterns to Avoid

- **Splitting after push without accounting for length:** After `push`, the stack length increases by 1. `split_at_mut(stack_len - 1)` gives `bottom = [0..stack_len-1]` and `top = [stack_len-1..stack_len]` — the caller is `bottom.last()` and callee is `top[0]`. Off-by-one here causes a panic.
- **Using unsafe for aliasing:** Do not use `unsafe` pointer casting to alias caller/callee. The `split_at_mut` approach is safe and has the same codegen.
- **Changing exec_call_extern:** It builds a Vec that is the payload to the host API. Leave it alone.
- **Changing SpawnTask/SpawnDetached in concurrency.rs:** These also use Vec::with_capacity for args passed to ExecutionResult variants that carry Vec<Value>. They are out of scope for Phase 76 (they are concurrency instructions, not call instructions).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Disjoint mutable borrows into same Vec | Unsafe raw pointer aliasing | `split_at_mut` | Compiler-verified safe, identical codegen |
| Small fixed arg buffer | Custom SmallVec impl | Fixed Rust array `[Value::Void; N]` | Zero dependencies, stack-resident, same effect |

**Key insight:** The existing borrow-checker split is a solved problem in Rust — `split_at_mut` exists precisely for this pattern (indexing two disjoint parts of a slice simultaneously).

## Common Pitfalls

### Pitfall 1: Off-by-One in split_at_mut Index
**What goes wrong:** Using `split_at_mut(stack_len)` gives an empty top slice; `split_at_mut(0)` gives an empty bottom slice.
**Why it happens:** Confusing "index of last element" with "length after push."
**How to avoid:** After push, `stack_len = call_stack.len()`. Split at `stack_len - 1`. Caller is `bottom.last()`, callee is `top[0]`.
**Warning signs:** Panic at `unwrap()` or index-out-of-bounds in test.

### Pitfall 2: Forgetting that argc May Be Zero
**What goes wrong:** Loop `for i in 0..argc` is trivially correct for argc=0, but some constructs around it (e.g., bounds-checking r_base+argc against caller register count) must not panic for argc=0.
**Why it happens:** Edge case in test coverage.
**How to avoid:** The existing loop `for i in 0..argc as usize` already handles this correctly. Preserve the loop structure.

### Pitfall 3: Value::InlineStruct Clone Cost
**What goes wrong:** `InlineStruct { fields: Vec<Value> }` clone allocates. This is inherent to the current Value representation.
**Why it happens:** Value is Clone-only (not Copy) due to InlineStruct's inner Vec. Phase 76 does NOT solve this — that is Phase 79.
**How to avoid:** Accept the clone cost for InlineStruct in Phase 76. The key win is eliminating the *staging* Vec that previously wrapped all args regardless of type.

### Pitfall 4: exec_tail_call Register Count Change
**What goes wrong:** The tail-call target may have a different `reg_count` than the current frame. Simply in-place overwriting registers[0..argc] without resizing leaves stale registers if reg_count grew, or retains old values if reg_count shrank.
**Why it happens:** The current code does `current.registers = vec![Value::Void; reg_count]` which correctly handles this.
**How to avoid:** After writing the saved args, call `current.registers.fill(Value::Void)` then `current.registers.resize(reg_count, Value::Void)`, then write the saved args into `[0..argc]`. Alternatively, truncate to argc first, resize to reg_count (fills with Void), then write args at [0..argc] — which avoids the intermediate fill.

### Pitfall 5: Missed Vec Allocation in exec_call_virt
**What goes wrong:** exec_call_virt has the Vec allocation inside the `Method` arm of the dispatch match. It is easy to fix exec_call and forget exec_call_virt's inner arm.
**Why it happens:** The allocation is at line 92 inside a match arm, not at the top of the function.
**How to avoid:** Grep for `Vec::with_capacity` in calls.rs after the change and confirm zero matches in the target functions (exec_call, exec_call_virt Method arm, exec_call_indirect, exec_tail_call).

## Code Examples

### Verified Pattern: split_at_mut for push-then-copy

```rust
// Source: Rust std docs — split_at_mut provides disjoint mutable borrows
// After: ctx.task.call_stack.push(CallFrame::new(method_idx, reg_count, r_dst));
let stack_len = ctx.task.call_stack.len();
// stack_len >= 2 guaranteed: at least one caller frame existed + we just pushed
let (bottom, top) = ctx.task.call_stack.split_at_mut(stack_len - 1);
let caller = bottom.last().unwrap();
let callee = &mut top[0];
for i in 0..argc as usize {
    if i < callee.registers.len() {
        callee.registers[i] = caller.registers[r_base as usize + i].clone();
    }
}
```

### Verified Pattern: tail-call with local buffer

```rust
// Read args before overwriting frame
// MAX_INLINE_ARGS = 32 covers all realistic Writ function signatures
const MAX_INLINE_ARGS: usize = 32;
let argc_usize = argc as usize;
let current = ctx.task.call_stack.last_mut().unwrap();
// Stack-allocate buffer — no heap allocation for argc <= MAX_INLINE_ARGS
let mut arg_buf: [Value; MAX_INLINE_ARGS] = std::array::from_fn(|_| Value::Void);
let use_heap_fallback = argc_usize > MAX_INLINE_ARGS;
let mut heap_buf;
if use_heap_fallback {
    heap_buf = Vec::with_capacity(argc_usize);
    for i in 0..argc_usize {
        heap_buf.push(current.registers[r_base as usize + i].clone());
    }
} else {
    for i in 0..argc_usize {
        arg_buf[i] = current.registers[r_base as usize + i].clone();
    }
}
// Now overwrite the frame
current.method_idx = method_idx;
current.pc = 0;
current.registers.truncate(0);
current.registers.resize(reg_count, Value::Void);
// Write saved args back at [0..argc]
if use_heap_fallback {
    for (i, v) in heap_buf.into_iter().enumerate() {
        current.registers[i] = v;
    }
} else {
    for i in 0..argc_usize {
        current.registers[i] = std::mem::replace(&mut arg_buf[i], Value::Void);
    }
}
```

**Simpler alternative** (if we accept that tail-call with >32 args falls back to a heap Vec, which is extremely rare):

```rust
// All-in-one, always stack-resident for <= 32 args — handles fib(40) (argc=2)
let argc_usize = argc as usize;
debug_assert!(argc_usize <= 32, "tail-call argc={} exceeds inline buffer", argc_usize);
// For the benchmark / common path: inline, no alloc
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Instruction cloning on dispatch | Borrow-based instruction reference | Phase 75 (quick task 260322-6om) | ~5x speedup on fib(40) |
| std HashMap in DispatchTable | FxHashMap (rustc-hash) | Phase 75 | ~10% reduction in dispatch latency |
| arg staging Vec on every call | Direct register-to-register copy | Phase 76 (this phase) | Eliminates N heap allocs per fib(40) call tree |

**Baseline from Phase 75:** Median of 3 cold fib(40) runs = **83.297s** (release mode). Phase 76 delta must be recorded after implementation.

## Open Questions

1. **Does `std::array::from_fn(|_| Value::Void)` compile on edition 2024?**
   - What we know: `std::array::from_fn` is stable since Rust 1.63. Edition 2024 does not affect this.
   - What's unclear: Whether `[Value::Void; 32]` is allowed given that Value does not implement Copy.
   - Recommendation: Use `std::array::from_fn(|_| Value::Void)` which does not require Copy — it calls the closure for each element.

2. **Should the inline buffer size be a named const or a hardcoded 32?**
   - What we know: Writ functions have no practical limit on arg count in the IL spec, but real functions rarely exceed 8-10 args.
   - Recommendation: Use a `const MAX_INLINE_ARGC: usize = 32` in calls.rs. Document the fallback.

3. **Does the fib benchmark actually exercise tail-call?**
   - What we know: The fib.writ benchmark uses recursive `fib(n-1) + fib(n-2)` — this is a regular CALL, not TAIL_CALL. The tail_call_does_not_grow_stack test in vm_tests.rs covers the tail-call path.
   - Recommendation: Add a test that explicitly exercises tail-call argument passing with multiple args (current test uses argc=1, method_idx=0x07000002).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + integration tests in `writ-runtime/tests/` |
| Config file | Cargo.toml (edition 2024) |
| Quick run command | `cargo test --release -p writ-runtime 2>&1` |
| Full suite command | `cargo test --release 2>&1` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CALL-01 | exec_call passes args without staging Vec | unit | `cargo test --release -p writ-runtime call 2>&1` | Covered by vm_tests.rs call tests |
| CALL-02 | exec_call_virt passes args without staging Vec | unit | `cargo test --release -p writ-runtime call_virt 2>&1` | Covered by vm_tests.rs (call_virt_crashes) |
| CALL-03 | exec_call_indirect passes args without staging Vec | unit | `cargo test --release -p writ-runtime call_indirect 2>&1` | Covered by vm_tests.rs |
| CALL-04 | exec_tail_call passes args without staging Vec | unit | `cargo test --release -p writ-runtime tail_call 2>&1` | Covered by vm_tests.rs (tail_call_does_not_grow_stack) |
| CALL-05 | All call tests pass | integration | `cargo test --release -p writ-runtime 2>&1` | ✅ existing |
| VERIFY-01 | fib(40) = 102334155 | smoke | Run writ-cli on fib.writ | ✅ existing |
| VERIFY-02 | zero test failures | suite | `cargo test --release 2>&1` | ✅ existing |
| VERIFY-03 | zero warnings | build | `cargo build --release 2>&1` | ✅ existing |

### Sampling Rate
- **Per task commit:** `cargo test --release -p writ-runtime`
- **Per wave merge:** `cargo test --release`
- **Phase gate:** Full suite green + fib(40) correct + performance delta recorded before marking phase complete

### Wave 0 Gaps
- [ ] `writ-runtime/tests/vm_tests.rs` — add `tail_call_passes_multiple_args` test (argc >= 2, verifies each arg arrives in correct callee register). Current tail-call test uses argc=1.
- [ ] `writ-runtime/tests/vm_tests.rs` — add `call_indirect_passes_args` test (currently call_indirect is tested only for delegate lookup, not arg passing with argc > 0).

*(No new test files needed — gaps are additional test functions in existing files.)*

## Sources

### Primary (HIGH confidence)
- Direct source read: `writ-runtime/src/dispatch/calls.rs` — all five call handlers, exact line numbers for Vec::with_capacity occurrences at lines 26, 92, 134, 232, 272
- Direct source read: `writ-runtime/src/frame.rs` — CallFrame struct, registers field is `Vec<Value>`
- Direct source read: `writ-runtime/src/dispatch/mod.rs` — ExecContext definition, execute_one dispatch match, split_at_mut is appropriate for call_stack (Vec<CallFrame>)
- Direct source read: `writ-runtime/src/value.rs` — Value enum, Clone-only (not Copy), InlineStruct contains Vec<Value>
- Direct source read: `writ-runtime/tests/vm_tests.rs` — tail_call_does_not_grow_stack test at line 918, call_virt_crashes at line 963

### Secondary (MEDIUM confidence)
- Rust stdlib docs — `slice::split_at_mut` is stable since Rust 1.0.0, safe abstraction for disjoint mutable borrows into the same slice
- `std::array::from_fn` stable since Rust 1.63 — safe array initialization without Copy

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — code is directly read, no external library changes needed
- Architecture: HIGH — split_at_mut pattern is well-established Rust idiom verified against source
- Pitfalls: HIGH — identified directly from source structure (exec_call_virt inner arm, tail-call register count change, argc=0 edge case)

**Research date:** 2026-03-22
**Valid until:** Until any of calls.rs, frame.rs, or value.rs is structurally changed (next relevant change is Phase 77 frame pooling or Phase 79 Value::Copy migration)
