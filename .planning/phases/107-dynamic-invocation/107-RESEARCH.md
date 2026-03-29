# Phase 107: Dynamic Invocation - Research

**Researched:** 2026-03-28
**Domain:** writ-runtime reflection — dynamic field write and method invocation
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- FieldInfo.set() on let-field crashes task with "Reflection write to immutable field '{name}'"
- FieldInfo.set() on mut-field writes the new value
- MethodInfo.invoke() uses the current task stack (not a new task)
- MethodInfo.invoke() participates in cooperative scheduling correctly
- FieldDef.flags readonly bit: 0x01 = readonly (let field). is_mutable = (flags & 0x01) == 0

### Claude's Discretion
All other implementation choices — virtual module contract layout, IntrinsicId names, method reverse-map design, invoke sub-loop design, test structure.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DYN-01 | FieldInfo.set(instance, value) writes field dynamically with runtime mutability enforcement (crash on let-field write) | FieldDef.flags bit 0x01 confirmed present; heap.set_field() is the write path; is_mutable already computed in FieldInfo heap object field 2 |
| DYN-02 | MethodInfo.invoke(instance, args) invokes method dynamically on current task stack with arg count/type validation | try_speaker_dispatch in calls.rs is the exact model; needs method_reverse map added to ReflectionIndex |
| DYN-03 | Type.construct(args) creates instance dynamically with on_create lifecycle hook dispatch | SCOPE CONFLICT — see Open Questions; STATE.md says deferred to v12+, but REQUIREMENTS.md assigns to Phase 107 |
| DYN-04 | Dynamic invocation correctly participates in cooperative scheduling (defer runs on unwind, spawn yields) | MethodInfo.invoke uses same call stack — defers unwind naturally on crash; spawn already handled by execute_one |
</phase_requirements>

## Summary

Phase 107 adds the write and invoke sides of the reflection API: `FieldInfo.set(instance, value)` for dynamic field writes, and `MethodInfo.invoke(instance, args)` for dynamic method calls on the current task stack.

The infrastructure is well-established. Phase 103 built the `ReflectionIndex` with lazy caches and reverse maps. Phase 106 implemented `FieldInfoGet` and all read-only intrinsics. The new intrinsics follow the same pattern with one important addition: `MethodInfo.invoke()` needs a `method_reverse` map in `ReflectionIndex` (analogous to the existing `field_reverse` map) so the intrinsic can recover `(module_idx, method_idx)` from a `MethodInfo` HeapRef.

The cooperative scheduling requirement (DYN-04) is satisfied naturally: `MethodInfo.invoke()` pushes a call frame onto `ctx.task.call_stack` and returns `ExecutionResult::Continue`, exactly like `exec_call`. The task scheduler's normal tick loop then executes the pushed frame. Defer handlers run on crash unwind because they are registered in the frame's `defer_stack` — no special handling needed. This is identical to the `try_speaker_dispatch` pattern already in `calls.rs`.

**Primary recommendation:** Add `FieldInfoSet` and `MethodInfoInvoke` to `IntrinsicId`; add two new contract defs to the virtual module; add `method_reverse` to `ReflectionIndex`; implement the intrinsics following FieldInfoGet and try_speaker_dispatch respectively. Address DYN-03 scope ambiguity before planning.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| writ-runtime (internal) | current | VM execution, heap, dispatch | The only runtime |
| writ-module (internal) | current | Module metadata, FieldDefRow.flags | Shared metadata format |

No external dependencies. This is a pure Rust implementation phase within the existing codebase.

### Installation
No new crates required.

## Architecture Patterns

### Recommended Project Structure
```
writ-runtime/src/
├── dispatch/mod.rs          # Add FieldInfoSet, MethodInfoInvoke to IntrinsicId
├── dispatch/intrinsics.rs   # Implement the two new intrinsic arms
├── reflection.rs            # Add method_reverse map; add lookup_method_identity()
├── virtual_module.rs        # Add FieldInfo.set and MethodInfo.invoke contract defs
└── tests/
    reflection_tests.rs      # New tests: field set (mut+let), method invoke, scheduling
```

### Pattern 1: FieldInfoSet follows FieldInfoGet exactly
**What:** Read the FieldInfo object's identity via `field_reverse`, check `FieldDef.flags & 0x01`, then call `heap.set_field(instance_href, field_offset, new_value)`.
**When to use:** DYN-01 implementation.
**Example:**
```rust
// Source: writ-runtime/src/dispatch/intrinsics.rs — FieldInfoGet arm (lines 662-691)
IntrinsicId::FieldInfoSet => {
    // r_obj = FieldInfo heap object
    // r_base+1 = instance (mut or ref)
    // r_base+2 = new value
    let fi_href = helpers::extract_ref(
        &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
    );
    let instance_val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1];
    let new_val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 2];

    let instance_href = match instance_val {
        Value::Ref(href) => href,
        Value::Struct { href, .. } => href,
        _ => return ExecutionResult::Crash(
            "FieldInfo.set: instance argument is not a struct or ref".into()
        ),
    };

    let (module_idx, _typedef_idx, field_offset) =
        match ctx.reflection.lookup_field_identity(fi_href) {
            Some(id) => id,
            None => return ExecutionResult::Crash(
                "FieldInfo.set: not a FieldInfo object".into()
            ),
        };

    // Check mutability via FieldDef.flags bit 0x01
    let field_start = {
        let module = &ctx.modules[module_idx].module;
        let td_idx = _typedef_idx;
        let td = &module.type_defs[td_idx];
        td.field_list.saturating_sub(1) as usize
    };
    let abs_idx = field_start + field_offset;
    let flags = ctx.modules[module_idx].module.field_defs[abs_idx].flags;
    if flags & 0x01 != 0 {
        // 0x01 = FIELD_FLAG_READONLY — crash on let-field write
        let name = writ_module::heap::read_string(
            &ctx.modules[module_idx].module.string_heap,
            ctx.modules[module_idx].module.field_defs[abs_idx].name,
        ).unwrap_or("?");
        return ExecutionResult::Crash(
            format!("Reflection write to immutable field '{}'", name)
        );
    }

    match ctx.heap.set_field(instance_href, field_offset, new_val) {
        Ok(()) => {
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Void;
            ExecutionResult::Continue
        }
        Err(e) => ExecutionResult::Crash(format!("FieldInfo.set: {}", e)),
    }
}
```

### Pattern 2: MethodInfoInvoke follows try_speaker_dispatch
**What:** Recover `(module_idx, method_idx)` from the `method_reverse` map, validate arg count, push a CallFrame, copy args including the instance in r0.
**When to use:** DYN-02 implementation.
**Example:**
```rust
// Source: writ-runtime/src/dispatch/calls.rs — try_speaker_dispatch (lines 480-597)
// Key steps only — full implementation expands these
IntrinsicId::MethodInfoInvoke => {
    let mi_href = helpers::extract_ref(
        &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
    );
    let instance_val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1];
    let args_arr_val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 2];

    let (method_module_idx, method_idx) =
        match ctx.reflection.lookup_method_identity(mi_href) {
            Some(id) => id,
            None => return ExecutionResult::Crash(
                "MethodInfo.invoke: not a MethodInfo object".into()
            ),
        };

    // Validate method exists
    let target_module = &ctx.modules[method_module_idx];
    if method_idx >= target_module.decoded_bodies.len() {
        return ExecutionResult::Crash(
            format!("MethodInfo.invoke: method index {} out of range", method_idx)
        );
    }

    // Extract args array elements
    let args_href = helpers::extract_ref(&args_arr_val);
    let args: Vec<Value> = match ctx.heap.get_object(args_href) {
        Ok(HeapObject::Array { elements, .. }) => elements.clone(),
        _ => vec![],
    };

    // Validate arg count against method param_count
    let expected_params = target_module.module.method_defs[method_idx].param_count as usize;
    // self is r0, user args fill r1..r(param_count). Total registers = param_count + 1 at minimum.
    if args.len() != expected_params {
        return ExecutionResult::Crash(format!(
            "MethodInfo.invoke: expected {} args, got {}",
            expected_params, args.len()
        ));
    }

    let reg_count = target_module.module.method_bodies[method_idx].register_types.len();

    // Push callee frame — r_dst is the return destination in the *current* (caller) frame
    ctx.task.call_stack.push(
        crate::frame::CallFrame::with_pool(ctx.pool, method_idx, reg_count, r_dst)
    );

    // Set self (r0) = instance, args = r1..
    if let Some(frame) = ctx.task.call_stack.last_mut() {
        if !frame.registers.is_empty() {
            frame.registers[0] = instance_val;
        }
        for (i, v) in args.iter().enumerate() {
            let ri = i + 1;
            if ri < frame.registers.len() {
                frame.registers[ri] = *v;
            }
        }
    }

    ExecutionResult::Continue
    // The task scheduler's tick loop will now execute the pushed frame.
    // Defers registered in that frame unwind automatically on crash.
}
```

### Pattern 3: method_reverse map in ReflectionIndex
**What:** Add `method_reverse: FxHashMap<HeapRef, (usize, usize)>` parallel to the existing `field_reverse` map. Populate it in `get_or_alloc_method_info()`.
**When to use:** Before implementing MethodInfoInvoke.
```rust
// Source: writ-runtime/src/reflection.rs — get_or_alloc_method_info (lines 299-333)
// Add to ReflectionIndex struct:
pub(crate) method_reverse: FxHashMap<HeapRef, (usize, usize)>,

// Add at end of get_or_alloc_method_info():
self.method_reverse.insert(href, (module_idx, method_idx));

// New lookup method:
pub fn lookup_method_identity(&self, href: HeapRef) -> Option<(usize, usize)> {
    self.method_reverse.get(&href).copied()
}
```

### Pattern 4: Virtual module contract defs
**What:** Add `FieldInfo.set` and `MethodInfo.invoke` contract defs to `virtual_module.rs`, add ImplDef entries for each, and add intrinsic method stubs.
**When to use:** Required before dispatch table can route CALL_VIRT to the new arms.
```rust
// Source: writ-runtime/src/virtual_module.rs — Section 10 contract defs (lines 462-503)
let fieldinfo_set_contract = builder.add_contract_def("FieldInfo.set", "writ");
// ... add slot definition ...
let methodinfo_invoke_contract = builder.add_contract_def("MethodInfo.invoke", "writ");
// ... add slot definition ...

// In ImplDef section:
builder.add_impl_def(field_info_type, fieldinfo_set_contract);
add_intrinsic_method(&mut builder, "fieldinfo_set");
builder.add_impl_def(method_info_type, methodinfo_invoke_contract);
add_intrinsic_method(&mut builder, "methodinfo_invoke");
```

### Pattern 5: IntrinsicId registration in dispatch table
**What:** The dispatch table is populated during domain load. The `"fieldinfo_set"` and `"methodinfo_invoke"` method names must map to the new `IntrinsicId` variants.
```rust
// Source: writ-runtime/src/dispatch/mod.rs — IntrinsicId enum (lines 52-81)
// Add after FieldInfoGetIsMutable:
FieldInfoSet,
// Add after MethodInfoGetParameters:
MethodInfoInvoke,
```

### Anti-Patterns to Avoid
- **Blocking on MethodInfo.invoke():** The intrinsic must NOT run an inner execution loop. It pushes the frame and returns `Continue`. The outer tick loop in the scheduler drives execution. Running an inner loop (as `try_speaker_dispatch` does) would prevent cooperative scheduling and break DYN-04.
- **Recovering mutability from the FieldInfo heap object field 2 (is_mutable bool):** The is_mutable cached value is correct, but re-reading from the FieldDef.flags raw byte is more authoritative and avoids any stale-cache concern. Either approach works but reading from flags is the defensive choice.
- **Skipping arg count validation in MethodInfo.invoke:** param_count is the number of regular params excluding self. args.len() must equal param_count exactly.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Field identity lookup | Custom reverse-scan of field_defs | `reflection.lookup_field_identity(fi_href)` | Already implemented — returns (module_idx, typedef_idx, field_offset) |
| Field name for crash message | Re-query heap fields | `module.field_defs[abs_idx].name` + `read_string` | Direct metadata read, no heap traversal |
| Method existence check | Manual method_defs scan | `decoded_bodies.len()` bound check | Already the pattern in exec_call |
| Task crash for immutability | Custom error type | `ExecutionResult::Crash(string)` | Same mechanism used by div-by-zero, entity errors |

## Common Pitfalls

### Pitfall 1: method_reverse not populated on existing MethodInfo objects
**What goes wrong:** If `method_reverse` is added to `ReflectionIndex` but existing cached MethodInfo HeapRefs were allocated before the map existed, `lookup_method_identity()` returns None for them.
**Why it happens:** Tests from Phase 106 create MethodInfo objects without `method_reverse`. This is not a runtime issue because the cache is rebuilt fresh each process start, but unit tests that create MethodInfo objects before the reverse map is populated will fail.
**How to avoid:** Add `method_reverse` to `ReflectionIndex::new()` initializer and to `get_or_alloc_method_info()` simultaneously in a single commit. Do not defer the `method_reverse.insert()` call.
**Warning signs:** `lookup_method_identity` returning None for a valid MethodInfo HeapRef in tests.

### Pitfall 2: FieldInfoSet argc mismatch
**What goes wrong:** The `execute_intrinsic` signature receives `r_obj`, `r_base`, `argc`. For FieldInfoSet: r_obj = the FieldInfo receiver, r_base+1 = instance, r_base+2 = new value. If the compiler emits argc=2 but the intrinsic reads r_base+2, it works — but if argc=1 (wrong), r_base+2 is undefined.
**Why it happens:** The FieldInfo.set contract must declare the correct parameter count so the compiler emits the right argc.
**How to avoid:** The virtual module contract slot for `FieldInfo.set` must declare 2 parameters (instance + value). Verify the contract method slot signature includes both.
**Warning signs:** Crash "instance argument is not a struct or ref" when the new value register is read as instance.

### Pitfall 3: MethodInfo.invoke uses inner execute loop (breaks scheduling)
**What goes wrong:** If `MethodInfo.invoke` runs its own execution loop (like `try_speaker_dispatch`), the invoked method runs synchronously to completion before the intrinsic returns. This means cooperative yielding and suspension never happen for the invoked method — DYN-04 fails.
**Why it happens:** `try_speaker_dispatch` uses an inner loop because it needs the result synchronously (for a string display value). MethodInfo.invoke has no such requirement — the result is written to the caller's r_dst register when the callee returns via RET.
**How to avoid:** Push frame, return `ExecutionResult::Continue`. Let the scheduler drive the callee.
**Warning signs:** Methods invoked via MethodInfo.invoke never suspend; tests that await host responses inside invoked methods fail or timeout.

### Pitfall 4: DYN-03 (Type.construct) scope ambiguity
**What goes wrong:** REQUIREMENTS.md assigns DYN-03 to Phase 107, but STATE.md accumulated context (line: "Dynamic construction (Type.construct()) deferred to v12+") marks it deferred. CONTEXT.md does not mention DYN-03 at all.
**Why it happens:** The deferred decision was made before requirements were finalized, and REQUIREMENTS.md was not updated to reflect it.
**How to avoid:** Confirm with the user before planning whether DYN-03 is in scope for Phase 107 or deferred. If deferred, mark DYN-03 as `Pending (deferred)` in REQUIREMENTS.md.
**Warning signs:** Planner attempts to implement Type.construct() and discovers no NEW instruction exists in the module, no allocation strategy for dynamic construction, and no on_create hook dispatch path.

### Pitfall 5: virtual_module.rs contract count assertion failure
**What goes wrong:** `has_exactly_46_contract_defs()` test in virtual_module.rs asserts exactly 46 contract defs. Adding 2 new contracts (FieldInfo.set + MethodInfo.invoke) without updating this assertion causes test failure.
**Why it happens:** The test was written to catch accidental contract additions. It is correct behavior — it just needs to be updated.
**How to avoid:** Update the assertion in the test to 48 (or the correct new count) when adding the new contracts.
**Warning signs:** `has_exactly_46_contract_defs` test fails immediately after virtual_module.rs change.

## Code Examples

### FieldInfoGet (existing — model for FieldInfoSet)
```rust
// Source: writ-runtime/src/dispatch/intrinsics.rs lines 662-691
IntrinsicId::FieldInfoGet => {
    let fi_href = helpers::extract_ref(
        &ctx.task.call_stack.last().unwrap().registers[r_obj as usize]
    );
    let instance_val = ctx.task.call_stack.last().unwrap().registers[r_base as usize + 1];
    let instance_href = match instance_val {
        Value::Ref(href) => href,
        Value::Struct { href, .. } => href,
        _ => return ExecutionResult::Crash(
            "FieldInfo.get: instance argument is not a struct or ref".into()
        ),
    };
    let (_module_idx, _typedef_idx, field_offset) =
        match ctx.reflection.lookup_field_identity(fi_href) {
            Some(id) => id,
            None => return ExecutionResult::Crash(
                "FieldInfo.get: not a FieldInfo object".into()
            ),
        };
    let val = ctx.heap.get_field(instance_href, field_offset).unwrap_or(Value::Void);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = val;
    ExecutionResult::Continue
}
```

### FieldDef.flags readonly bit
```rust
// Source: writ-module/src/tables.rs lines 148-154
pub struct FieldDefRow {
    pub name: u32,     // string heap offset
    pub type_sig: u32, // blob heap offset
    pub flags: u16,    // bit 0x01 = FIELD_FLAG_READONLY (let field)
}

// In reflection.rs get_or_alloc_field_info (line 280):
// 0x01 = FIELD_FLAG_READONLY; is_mutable = (flags & 0x01) == 0
let is_mutable = (fd.flags & 0x01) == 0;
let _ = heap.set_field(href, 2, Value::Bool(is_mutable));
```

### heap.set_field signature
```rust
// Source: writ-runtime/src/gc.rs line 36
fn set_field(&mut self, href: HeapRef, idx: usize, val: Value) -> Result<(), RuntimeError>;

// Concrete impl in heap.rs line 141:
pub fn set_field(&mut self, href: HeapRef, idx: usize, val: Value) -> Result<(), RuntimeError>
// Returns Err if href is not a Struct or idx is out of range.
```

### CallFrame construction (from exec_call)
```rust
// Source: writ-runtime/src/dispatch/calls.rs lines 26-44
let reg_count = module.module.method_bodies[method_idx].register_types.len();
ctx.task.call_stack.push(
    crate::frame::CallFrame::with_pool(ctx.pool, method_idx, reg_count, r_dst)
);
let stack_len = ctx.task.call_stack.len();
let (bottom, top) = ctx.task.call_stack.split_at_mut(stack_len - 1);
let caller = bottom.last().unwrap();
let callee = &mut top[0];
for i in 0..argc as usize {
    callee.registers[i] = caller.registers[r_base as usize + i];
}
```

### Crash testing pattern (existing tests)
```rust
// Source: writ-runtime/tests/vm_tests.rs lines 1770-1772
assert_eq!(rt.task_state(tid), Some(TaskState::Cancelled));
let crash = rt.crash_info(tid).unwrap();
assert!(crash.message.contains("not alive"), "crash message: {}", crash.message);
// Note: crashed tasks have state Cancelled (not a separate Crashed state)
```

### ReflectionIndex reverse map pattern (field_reverse existing)
```rust
// Source: writ-runtime/src/reflection.rs lines 29-31
pub(crate) field_reverse: FxHashMap<HeapRef, (usize, usize, usize)>,
// Populated in get_or_alloc_field_info:
self.field_reverse.insert(href, (module_idx, typedef_idx, field_offset));
// Queried by:
pub fn lookup_field_identity(&self, href: HeapRef) -> Option<(usize, usize, usize)>
```

## Runtime State Inventory

Step 2.5: SKIPPED — this is a new feature implementation phase, not a rename/refactor/migration.

## Environment Availability

Step 2.6: SKIPPED — no external tool dependencies. All implementation is pure Rust within the existing workspace.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`#[test]`) + cargo test |
| Config file | Cargo.toml workspace |
| Quick run command | `cargo test -p writ-runtime reflection` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DYN-01 | FieldInfo.set writes mut field | unit | `cargo test -p writ-runtime test_field_info_set_mut` | ❌ Wave 0 |
| DYN-01 | FieldInfo.set crashes on let-field | unit | `cargo test -p writ-runtime test_field_info_set_readonly_crashes` | ❌ Wave 0 |
| DYN-02 | MethodInfo.invoke pushes frame and executes | unit | `cargo test -p writ-runtime test_method_info_invoke` | ❌ Wave 0 |
| DYN-02 | MethodInfo.invoke arg count mismatch crashes | unit | `cargo test -p writ-runtime test_method_info_invoke_wrong_argc` | ❌ Wave 0 |
| DYN-03 | Type.construct (if in scope) | unit | `cargo test -p writ-runtime test_type_construct` | ❌ Wave 0 |
| DYN-04 | Invoked method participates in scheduling | unit | `cargo test -p writ-runtime test_method_info_invoke_schedules_normally` | ❌ Wave 0 |
| DYN-04 | Defers run when invoked method crashes | unit | `cargo test -p writ-runtime test_method_info_invoke_defer_on_crash` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime reflection`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] New test functions in `writ-runtime/tests/reflection_tests.rs` — covers DYN-01, DYN-02, DYN-04
- [ ] Update `has_exactly_46_contract_defs` assertion in `writ-runtime/src/virtual_module.rs` when new contracts are added

## Open Questions

1. **DYN-03 scope: Is Type.construct() in or out of Phase 107?**
   - What we know: REQUIREMENTS.md lists DYN-03 as Pending for Phase 107. STATE.md accumulated context says "Dynamic construction (Type.construct()) deferred to v12+." CONTEXT.md does not mention DYN-03.
   - What's unclear: Which document takes precedence — the deliberate deferral in STATE.md context, or the assignment in REQUIREMENTS.md?
   - Recommendation: Ask the user before planning. If deferred, update REQUIREMENTS.md to mark DYN-03 with a note. If in scope, research the NEW instruction and on_create dispatch path as a separate task since it is significantly more complex than DYN-01/DYN-02.

2. **MethodInfoInvoke: should module_idx be stored in the MethodInfo heap object or only in method_reverse?**
   - What we know: `field_reverse` stores `(module_idx, typedef_idx, field_offset)`. The parallel design for `method_reverse` would store `(module_idx, method_idx)`.
   - What's unclear: Whether method_idx here is the absolute module-level method index or needs to be qualified differently for cross-module invocations.
   - Recommendation: Use the same pattern as `field_reverse` — store `(module_idx, method_idx)` where method_idx is the 0-based index into `modules[module_idx].decoded_bodies`. This is exactly how `try_speaker_dispatch` resolved method identity.

## Sources

### Primary (HIGH confidence)
- `writ-runtime/src/dispatch/intrinsics.rs` — FieldInfoGet implementation (lines 662-691), complete model
- `writ-runtime/src/dispatch/calls.rs` — try_speaker_dispatch (lines 480-597), MethodInfo.invoke model
- `writ-runtime/src/reflection.rs` — ReflectionIndex: field_reverse pattern, get_or_alloc_method_info
- `writ-module/src/tables.rs` — FieldDefRow.flags (line 153): confirms bit 0x01 = readonly
- `writ-runtime/src/dispatch/mod.rs` — IntrinsicId enum (lines 52-81), ExecContext shape
- `writ-runtime/src/virtual_module.rs` — Contract def layout, 46 contract count assertion
- `writ-runtime/src/frame.rs` — CallFrame structure, with_pool constructor
- `writ-runtime/src/task.rs` — TaskState::Cancelled is the crashed state, crash_info field
- `writ-runtime/tests/reflection_tests.rs` — Existing test patterns for reflection intrinsics
- `writ-runtime/tests/vm_tests.rs` — Crash testing pattern (crash_info, TaskState::Cancelled)
- `.planning/phases/107-dynamic-invocation/107-CONTEXT.md` — Locked decisions
- `.planning/STATE.md` — Accumulated context, DYN-03 deferral note

### Secondary (MEDIUM confidence)
- None — all findings are from direct source code inspection.

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all code is internal, directly read
- Architecture: HIGH — FieldInfoGet is the direct model; try_speaker_dispatch is the direct model for invoke
- Pitfalls: HIGH — derived from direct code inspection of the existing patterns
- DYN-03 scope: LOW — conflicting signals between REQUIREMENTS.md and STATE.md

**Research date:** 2026-03-28
**Valid until:** Stable (code does not change without commits)
