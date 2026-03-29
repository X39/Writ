# Phase 80: Clone Cleanup and Micro-Optimizations - Research

**Researched:** 2026-03-22
**Domain:** Rust VM dispatch — Copy-type clone elimination, unsafe register indexing
**Confidence:** HIGH

## Summary

Phase 79 made `Value` a `Copy` type. Every `.clone()` call on a `Value` is now a no-op at the language level — the compiler generates the same machine code as a plain copy. However, the `.clone()` calls are still syntactic noise that obscures intent and, more importantly, signals to the reader that an allocation might occur. Removing them makes the code obviously correct and prevents future regressions where someone adds a non-Copy field to `Value` and reintroduces heap allocation without noticing.

The micro-optimisation half of this phase is register access in the argument-copy loops. The VM registers vector is sized exactly at frame creation time and never shrinks mid-dispatch. The bounds check on `registers[idx]` in a tight loop over `0..argc` is therefore redundant: `argc` is constrained to at most `reg_count` at call emit time by the compiler. Using `slice::get_unchecked` in the four argument-copy loops (exec_call, exec_call_virt, exec_call_indirect, exec_tail_call) eliminates the check without violating safety — provided the invariant is documented.

A secondary match-arm tightening opportunity exists in `resolve_runtime_type_key` and `exec_unbox` / heap-field reads, but these are off the fib(40) critical path and carry zero-to-negligible measured impact.

**Primary recommendation:** Replace every `.clone()` on `Value` with a plain copy assignment, then apply `unsafe { *registers.get_unchecked(idx) }` in the four argument-copy hot loops with a documented safety invariant.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — pure infrastructure phase.

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase.

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| VERIFY-04 (partial) | fib(40) completes in under 30 seconds after all phases | Clone removal and unsafe register access are the remaining opt levers before phase 81 |
| VERIFY-01 | fib(40) produces correct output 102334155 | Clone-to-copy is semantically identical; no logic changes; existing tests verify |
| VERIFY-02 | cargo test --release passes after the phase with zero failures | Replacing .clone() on Copy never changes semantics; existing suite is sufficient |
| VERIFY-03 | cargo build --release produces no warnings after the phase | Removing explicit .clone() on Copy types eliminates potential clippy::clone_on_copy warnings |
</phase_requirements>

---

## Standard Stack

No new dependencies required. This phase works entirely within the existing crate.

| Tool | Version | Purpose |
|------|---------|---------|
| Rust std `slice::get_unchecked` | stable | Bounds-check elimination in hot register-copy loops |
| `grep` / `cargo clippy` | project-installed | Verification that zero `.clone()` calls remain on `Value` |

**Installation:** none required.

**Version verification:** not applicable — no new crates.

---

## Architecture Patterns

### Clone Categorisation

After reading every `.clone()` call in the six dispatch files, they fall into three categories:

**Category A — Copy-on-Copy (35 calls): must be removed**

These call `.clone()` on a `Value` (which is `Copy` as of Phase 79). The compiler already treats them as bitwise copies, but the explicit `.clone()` is dead weight and a maintenance hazard.

Key locations:

| File | Line range | Pattern |
|------|-----------|---------|
| calls.rs | 33, 95, 231 | arg-copy loop `callee.registers[i] = caller.registers[...].clone()` |
| calls.rs | 53 | `obj_val = registers[r_obj].clone()` before type_key resolution |
| calls.rs | 129, 268, 273 | exec_call_extern and exec_tail_call arg collect |
| calls.rs | 190 | exec_new_delegate target capture |
| arith.rs | 25, 404 | exec_mov and exec_convert |
| arith.rs | 520 | exec_box val capture |
| objects.rs | 91, 137, 161, 182, 212, 248, 430, 463 | field/array/enum operations |
| objects.rs | 296, 308, 345, 353, 365, 404 | Option/Result wrap/unwrap |
| concurrency.rs | 18, 43 | spawn arg collect |
| concurrency.rs | 84, 95 | load_global and store_global |
| intrinsics.rs | 279, 333, 347, 388 | ArrayIndexSet, ArrayIndex, ArrayIterable |
| mod.rs | 370 | Ret val capture `registers[*r_src].clone()` |
| mod.rs | 597 | `task.return_value = Some(ret_val.clone())` |

**Category B — Clone on non-Value types (3 calls): leave or handle separately**

- `calls.rs:214` — `target.clone()` where `target: Option<Value>`. `Option<Value>` is `Copy` when `V: Copy`, so this `.clone()` is also redundant and can be removed.
- `calls.rs:330` — `inner.clone()` in `resolve_runtime_type_key` for `HeapObject::Boxed(inner)`. `inner` is `Value` (Copy), removable.
- `mod.rs:680` — `msg.clone()` where `msg: String`. This is the crash message — genuinely needs a clone because `String` is not `Copy`. Leave it.
- `mod.rs:722` — `f.registers.clone()` for crash info stack frame. This is `Vec<Value>`, not a hot path (crash only). Leave it.

**Category C — Genuinely necessary clones (2): untouched**

- `mod.rs:680` — `msg.clone()` (String, crash path)
- `mod.rs:722` — `f.registers.clone()` (Vec<Value>, crash path, not hot)

**Net removable `.clone()` count:** approximately 37 (35 direct Value + 2 Option<Value>/Value-in-boxed).

---

### Pattern 1: Copy Assignment Replacement

**What:** Replace `x.clone()` with `x` (or `*x` when the source is behind a reference that is already coercible) everywhere `Value: Copy` makes the clone redundant.

**When to use:** Any `registers[idx].clone()` assignment, any `val.clone()` where `val: Value`.

**Example:**
```rust
// Before (Category A):
callee.registers[i] = caller.registers[r_base as usize + i].clone();

// After (plain copy, identical codegen):
callee.registers[i] = caller.registers[r_base as usize + i];
```

```rust
// Before:
let obj_val = ctx.task.call_stack.last().unwrap().registers[r_obj as usize].clone();

// After:
let obj_val = ctx.task.call_stack.last().unwrap().registers[r_obj as usize];
```

---

### Pattern 2: Unsafe Register Indexing in Argument-Copy Loops

**What:** Replace `registers[r_base as usize + i]` with `unsafe { *registers.get_unchecked(r_base as usize + i) }` in the four argument-copy loops.

**When to use:** Only in the hot-path arg-copy loops inside exec_call, exec_call_virt, exec_call_indirect, and exec_tail_call. NOT in general register access.

**Safety invariant (must be documented in a `// SAFETY:` comment):**

The compiler guarantees `argc <= reg_count` when emitting a CALL instruction. `CallFrame::with_pool` initialises `reg_count` registers. Therefore `r_base + i < registers.len()` for all `i in 0..argc`.

The existing guarded form `if i < callee.registers.len()` is redundant but also a mutable borrow of the already-borrowed callee frame. The unchecked form removes both the check and the conditional.

**Example:**
```rust
// Before:
for i in 0..argc as usize {
    if i < callee.registers.len() {
        callee.registers[i] = caller.registers[r_base as usize + i].clone();
    }
}

// After:
// SAFETY: argc <= callee reg_count (compiler invariant); r_base + argc <= caller reg_count
// (caller emits with correct argc); both bounds were verified at frame creation time.
for i in 0..argc as usize {
    unsafe {
        *callee.registers.get_unchecked_mut(i) =
            *caller.registers.get_unchecked(r_base as usize + i);
    }
}
```

The `if i < callee.registers.len()` guard can be deleted as part of this change. That guard was present because the original code was defensive; now that we understand the invariant and document it, the guard is dead code.

---

### Pattern 3: exec_tail_call Inline Buffer

The tail-call path copies args into a `[Value; 32]` stack buffer then moves them out. The moves use `std::mem::replace(&mut arg_buf[i], Value::Void)`. Since `Value` is `Copy`, this can be simplified:

```rust
// Before:
current.registers[i] = std::mem::replace(&mut arg_buf[i], Value::Void);

// After:
current.registers[i] = arg_buf[i];
```

`std::mem::replace` on a `Copy` type is never needed — no destructor runs, nothing needs clearing. The old `arg_buf[i]` value is simply abandoned on the stack when the buffer goes out of scope.

---

### Recommended Change Sequence

1. **Wave 1:** Remove all Category A and Category B `.clone()` calls (mechanical find-replace per file). No logic changes.
2. **Wave 2:** Apply unsafe indexing in the four arg-copy loops with safety comments. Remove the `if i < callee.registers.len()` guards.
3. **Wave 3:** Simplify `exec_tail_call` `std::mem::replace` to plain copy.
4. **Verification:** `grep -n '\.clone()' writ-runtime/src/dispatch/` must return zero results for Value-typed expressions. `cargo test --release`. Measure fib(40) median.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Bounds elimination | Custom bounds-check-free wrapper type | `slice::get_unchecked` | Already in std, audited, no abstraction cost |
| Detecting Copy clones | Custom lint | `cargo clippy -- -W clippy::clone_on_copy` | Built-in lint, instant feedback |

**Key insight:** `slice::get_unchecked` is the right tool — it conveys the intent precisely, keeps the safety boundary explicit, and generates the same code as a direct array access with no bounds check.

---

## Common Pitfalls

### Pitfall 1: Removing Clone on Non-Copy Heap Contents
**What goes wrong:** `HeapObject::Boxed(inner)` — at first glance `inner.clone()` looks like a Value clone, but `inner` is the `Value` stored *inside* the Boxed heap object. Since `Value: Copy`, this is still removable. However, `Vec<Value>` (as in `f.registers.clone()` for crash info) is NOT Copy and must be kept.
**How to avoid:** Check the type, not just the variable name. `Value` = removable. `Vec<Value>`, `String`, `Option<...>` where inner is non-Copy = keep.
**Warning signs:** Compile error "the trait `Copy` is not implemented for..." after removal.

### Pitfall 2: Forgetting the `if i < callee.registers.len()` Guard
**What goes wrong:** Deleting the unsafe block but keeping the guard, or vice versa. The guard is dead if the invariant holds; keeping it with `get_unchecked` is misleading (implies the check was needed).
**How to avoid:** Delete the guard and the unsafe body together as a single change, with the SAFETY comment explaining why the guard is unnecessary.
**Warning signs:** Code compiles but has `if i < ... { unsafe { ... } }` — contradictory pattern.

### Pitfall 3: Applying Unsafe Indexing Outside the Four Arg-Copy Loops
**What goes wrong:** Generalising the unsafe indexing to all register access. Other sites (branch targets, field indices, array indices) do NOT have the same compiler-guaranteed-bounds invariant.
**How to avoid:** Restrict `get_unchecked` to the four identified loops only.
**Warning signs:** Panics in tests on unusual inputs that exercise out-of-range registers.

### Pitfall 4: Benchmarking in Debug Mode
**What goes wrong:** The micro-optimisations are invisible in debug mode (bounds checks are always present, optimiser is off). The fib(40) delta must be measured with `cargo build --release` and run via the release binary.
**How to avoid:** Always benchmark with `--release`. The VERIFY-04 target of < 30 s is a release-mode measurement.

### Pitfall 5: mod.rs execute_ret Double Clone
**What goes wrong:** Line 597 reads `task.return_value = Some(ret_val.clone()); ExecutionResult::Completed(ret_val)`. Since `Value: Copy`, `ret_val.clone()` can be replaced with `ret_val` — but more precisely both uses can consume the copy directly. No structural change needed, just drop the `.clone()`.
**How to avoid:** Treat this like every other Category A clone: `Some(ret_val)` is fine because Copy means the variable is still usable after the move (the compiler inserts the bitwise copy automatically).

---

## Code Examples

### Verified Pattern: exec_mov before and after

```rust
// arith.rs exec_mov — BEFORE (Clone on Copy):
pub(super) fn exec_mov(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = frame.registers[r_src as usize].clone();
    ExecutionResult::Continue
}

// AFTER (plain copy, semantically identical):
pub(super) fn exec_mov(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = frame.registers[r_src as usize];
    ExecutionResult::Continue
}
```

### Verified Pattern: exec_call arg-copy loop before and after

```rust
// calls.rs exec_call arg-copy — BEFORE:
for i in 0..argc as usize {
    if i < callee.registers.len() {
        callee.registers[i] = caller.registers[r_base as usize + i].clone();
    }
}

// AFTER:
// SAFETY: The compiler guarantees argc <= callee reg_count and r_base + argc <= caller
// reg_count for every CALL instruction it emits. Both frames were sized from these
// values at creation time, so all indices are in-bounds.
for i in 0..argc as usize {
    unsafe {
        *callee.registers.get_unchecked_mut(i) =
            *caller.registers.get_unchecked(r_base as usize + i);
    }
}
```

### Verified Pattern: exec_tail_call std::mem::replace simplification

```rust
// BEFORE:
for i in 0..argc_usize {
    current.registers[i] = std::mem::replace(&mut arg_buf[i], Value::Void);
}

// AFTER (Value is Copy — replace is unnecessary):
for i in 0..argc_usize {
    current.registers[i] = arg_buf[i];
}
```

---

## Validation Architecture

`workflow.nyquist_validation` is not set in `.planning/config.json` — treated as enabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`#[test]`) |
| Config file | `writ-runtime/Cargo.toml` (test integration under `tests/`) |
| Quick run command | `cargo test -p writ-runtime --release 2>&1 \| tail -5` |
| Full suite command | `cargo test --release 2>&1 \| tail -20` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VERIFY-01 | fib(40) output == 102334155 | smoke/manual | run fib benchmark binary | manual |
| VERIFY-02 | cargo test --release zero failures | integration | `cargo test --release` | existing suite |
| VERIFY-03 | cargo build --release zero warnings | build check | `cargo build --release 2>&1 \| grep warning` | N/A |
| VERIFY-04 | fib(40) < 30 s release mode | benchmark | run `writ-runtime` fib(40) binary, record median of 3 runs | manual |

### Clone-Zero Grep Verification

Post-implementation, this command must return no lines matching `Value`-typed `.clone()`:

```bash
grep -n "\.clone()" writ-runtime/src/dispatch/calls.rs writ-runtime/src/dispatch/arith.rs writ-runtime/src/dispatch/objects.rs writ-runtime/src/dispatch/mod.rs writ-runtime/src/dispatch/concurrency.rs writ-runtime/src/dispatch/intrinsics.rs
```

Acceptable remaining calls (non-Value types, non-hot paths):
- `mod.rs`: `msg.clone()` (String), `f.registers.clone()` (Vec<Value>, crash info)

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime --release 2>&1 | tail -5`
- **Per wave merge:** `cargo test --release 2>&1 | tail -20`
- **Phase gate:** Full suite green + fib(40) median measured before `/gsd:verify-work`

### Wave 0 Gaps
None — existing test infrastructure covers all phase requirements. The clone cleanup is a refactor; existing vm_tests.rs, pool_tests.rs, gc_tests.rs, and task_tests.rs provide sufficient coverage.

---

## Open Questions

1. **Will fib(40) break through 30 s after this phase alone?**
   - What we know: Phase 79 median was 44.873 s. The gap is 14.873 s. Clone-to-copy is already a no-op in release codegen; the gains come from unsafe indexing (removes bounds check in the arg-copy loop) and possibly better inlining from simpler code.
   - What's unclear: How much of the remaining time is register-copy overhead vs. heap allocation, GC pressure, or instruction decode.
   - Recommendation: Measure immediately after Phase 80. If fib(40) is still above 30 s, Phase 81 (further optimisations) proceeds. The success criterion for Phase 80 is "delta from Phase 79 recorded", not "< 30 s".

2. **exec_call_extern args Vec: keep or remove?**
   - What we know: `exec_call_extern` builds a `Vec<Value>` to pass to the host. This is intentional — the host needs a owned arg list. The `.clone()` calls inside this Vec construction are removable (Value is Copy, so `args.push(frame.registers[...])` works without `.clone()`).
   - What's unclear: Nothing — this is straightforward.
   - Recommendation: Remove the `.clone()` in the push loop. The Vec itself stays (host needs it).

---

## Sources

### Primary (HIGH confidence)
- Direct source inspection of `writ-runtime/src/dispatch/` (all 8 files read in full)
- `writ-runtime/src/value.rs` — `Value: Copy` confirmed at line 59
- `writ-runtime/src/frame.rs` — `registers: Vec<Value>`, pool acquire/release logic
- `writ-runtime/tests/vm_tests.rs` — existing test infrastructure confirmed
- Rust Reference on `slice::get_unchecked` — stable API since Rust 1.0
- Rust Reference on `Copy` — `clone()` on a `Copy` type is a bitwise copy with zero additional behaviour

### Secondary (MEDIUM confidence)
- Phase 79 STATE.md entry: "fib(40) median 44.873s" — baseline for delta measurement
- REQUIREMENTS.md VERIFY-04: "< 30 seconds after all phases" — confirms this phase is partial contributor

---

## Metadata

**Confidence breakdown:**
- Clone identification: HIGH — every `.clone()` call enumerated by grep with file/line
- Unsafe indexing pattern: HIGH — standard Rust stable API, safety invariant directly derivable from call-convention spec
- Performance prediction: MEDIUM — clone-to-copy is already no-op in release; unsafe indexing gain depends on loop trip count (typically 1-3 for fib, potentially larger for other workloads)
- Test sufficiency: HIGH — existing suite exercises all affected paths

**Research date:** 2026-03-22
**Valid until:** 2026-04-22 (stable codebase, no external dependencies)
