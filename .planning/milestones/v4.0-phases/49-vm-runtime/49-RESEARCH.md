# Phase 49: VM Runtime - Research

**Researched:** 2026-03-12
**Domain:** Rust VM runtime — value type semantics, inline struct registers, GC root tracing
**Confidence:** HIGH (all findings from direct source code inspection)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Each register is "a single abstract typed slot" per spec §2.15.1 — structs occupy ONE register containing all fields, not multiple contiguous registers
- Add `Value::InlineStruct { type_idx: u32, fields: Vec<Value> }` variant to the Value enum
- `Value` loses `Copy` trait, becomes `Clone`-only — mechanical ripple through runtime (add `.clone()` where compiler complains)
- MOV clones the Value — for InlineStruct this shallow-copies the Vec (independent field values, but Ref fields share the same heap objects)
- Shallow copy semantics: primitive fields are copied, reference fields copy the HeapRef (same heap object). Matches spec and C# value-type behavior
- Direct TypeDef table lookup at dispatch time: `module.type_defs[type_idx].kind` — one array index, negligible cost
- kind=0 (struct): create `Value::InlineStruct { type_idx, fields: vec![Value::Void; field_count] }` in destination register — no heap allocation
- kind=4 (class): heap-allocate via existing `alloc_struct(field_count)` path, store `Value::Ref(href)` — current behavior preserved
- Unexpected kinds (enum=1, entity=2, component=3): crash with descriptive message ("NEW: type_idx {} is an enum, not a struct or class")
- GET_FIELD/SET_FIELD: match on Value variant at dispatch time — no type metadata lookup needed
- `Value::InlineStruct { fields, .. }` → read/write `fields[field_idx]` directly in the register
- `Value::Ref(href)` → go through heap (existing path, unchanged)
- Other variants → crash with type mismatch message
- SET_FIELD on InlineStruct mutates in place — register owns its copy, other MOV'd copies are independent
- Classes (kind=4) reuse `HeapObject::Struct { fields }` on the heap — no new HeapObject variant needed
- `HeapObject::Boxed(Value)` unchanged — `Boxed(Value::InlineStruct { type_idx, fields })` stores the entire struct value
- UNBOX clones the InlineStruct value back out of the Boxed wrapper
- `trace_refs` updated to trace through `Boxed(Value::InlineStruct { fields, .. })` — scan fields for `Value::Ref`
- Register root collector must be updated to scan InlineStruct fields for Refs
- MOV just clones whatever Value is in the source register — no type metadata lookup

### Claude's Discretion
- Exact error message wording for crash scenarios
- Whether to add helper methods on Value (e.g., `value.as_inline_struct()`, `value.as_inline_struct_mut()`)
- Test organization (new test files vs extending existing test modules)
- Whether to optimize InlineStruct with SmallVec for common small structs (Vec2, Vec3) — not required, can be added later
- GC root collection implementation detail for scanning InlineStruct fields in registers

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| VM-01 | Value-type struct inline register representation — no heap alloc for kind=0 structs | `Value::InlineStruct` variant; NEW kind-check dispatches inline vs heap |
| VM-02 | MOV copies all fields for value-type struct registers (multi-word copy) | `exec_mov` clone path; `Value::InlineStruct.clone()` deep-copies fields Vec |
| VM-03 | NEW instruction kind-dependent — heap alloc for class (kind=4), inline init for struct (kind=0) | `TypeDefKind::from_u8()` in `helpers::get_type_kind()`; `exec_new` split |
| VM-04 | GC traces through value-struct registers to find embedded reference fields | `collect_roots()` in runtime.rs must scan InlineStruct fields; `trace_refs()` for Boxed InlineStruct |
| VM-05 | BOX/UNBOX extended to handle value-type struct boxing through generics | `exec_box` already calls `heap.alloc_boxed(val)` — works directly; `exec_unbox` needs clone |
| VM-06 | Class (kind=4) uses existing heap allocation path (current struct behavior preserved) | `alloc_struct(field_count)` unchanged; `HeapObject::Struct { fields }` unchanged |
</phase_requirements>

## Summary

Phase 49 makes the VM runtime aware of the struct/class split introduced in the spec (Phase 47) and IL format (Phase 48). The change is entirely within `writ-runtime`; no changes to `writ-module`, `writ-compiler`, or `writ-assembler` are needed.

The core change is adding `Value::InlineStruct { type_idx: u32, fields: Vec<Value> }` to the `Value` enum and removing the `Copy` derive. This is the only data structure change — everything else (dispatch functions, GC, BOX/UNBOX) flows from it. The existing `HeapObject::Struct { fields }` is reused unchanged for class heap objects (kind=4).

The mechanical Copy-to-Clone ripple is the broadest change in scope. Every place that implicitly copies a `Value` by assignment — all the `let val = frame.registers[i]` extractions, the `get_field` return `.copied()` calls, the `exec_mov` direct assignment, the `Ret` return value extraction, `return_value: Option<Value>` reads — must be updated. The compiler will enumerate all such sites when `#[derive(Copy)]` is removed.

**Primary recommendation:** Structure the work in three waves: (1) `value.rs` — add InlineStruct variant, remove Copy, keep Clone; (2) mechanical `.clone()` fixes rippling from compiler errors; (3) targeted changes to NEW dispatch, GET/SET_FIELD dispatch, trace_refs, and collect_roots.

## Standard Stack

### Core Files Modified
| File | Change Type | Scope |
|------|-------------|-------|
| `writ-runtime/src/value.rs` | Add variant, remove Copy | Small — Value enum only |
| `writ-runtime/src/dispatch/objects.rs` | exec_new kind-check, exec_get/set_field split | ~50 lines changed |
| `writ-runtime/src/dispatch/arith.rs` | exec_mov clone, exec_unbox clone | ~4 lines changed |
| `writ-runtime/src/gc.rs` | trace_refs Boxed arm, GcHeap trait get_field | ~15 lines changed |
| `writ-runtime/src/runtime.rs` | collect_roots InlineStruct scan | ~10 lines changed |
| `writ-runtime/src/heap.rs` | get_field return type (`.copied()` → `.cloned()`) | ~4 lines changed |
| All files using Value | Mechanical .clone() additions | Compiler-guided |

### No New Dependencies
This phase requires no new crates. All changes are pure Rust refactoring within `writ-runtime`.

## Architecture Patterns

### Recommended File Structure (unchanged)
```
writ-runtime/src/
├── value.rs          # ADD InlineStruct variant, REMOVE Copy
├── heap.rs           # .copied() -> .cloned() in get_field returns
├── gc.rs             # trace_refs Boxed arm + GcHeap get_field return type
├── runtime.rs        # collect_roots: scan InlineStruct fields in registers
├── dispatch/
│   ├── arith.rs      # exec_mov clone, exec_unbox clone
│   └── objects.rs    # exec_new kind-check, exec_get_field/exec_set_field split
└── tests/
    └── vm_tests.rs   # Extend with struct/class tests
```

### Pattern 1: Value::InlineStruct Variant

**What:** New enum variant that holds all struct fields inline within the register.
**When to use:** Represents any value-type struct (TypeDefKind::Struct, kind=0) instance.

```rust
// Source: writ-runtime/src/value.rs — after change
#[derive(Debug, Clone)]  // NOTE: Copy removed
pub enum Value {
    Void,
    Int(i64),
    Float(f64),
    Bool(bool),
    Ref(HeapRef),
    Entity(EntityId),
    InlineStruct { type_idx: u32, fields: Vec<Value> },  // NEW
}

impl Default for Value {
    fn default() -> Self { Value::Void }
}
```

Shallow copy semantics on `.clone()`: `Vec<Value>::clone()` deep-copies the Vec but each element is cloned — primitives are value-copied, `Ref` copies just the `HeapRef(u32)` pointer (same heap object, no deep clone of heap data).

### Pattern 2: NEW Kind-Dispatch

**What:** `exec_new` checks TypeDefKind before deciding allocation strategy.
**When to use:** Every NEW instruction execution.

```rust
// Source: writ-runtime/src/dispatch/objects.rs — after change
pub(super) fn exec_new(ctx: &mut ExecContext<'_>, r_dst: u16, type_idx: u32) -> ExecutionResult {
    let module = &ctx.modules[ctx.current_module_idx];
    let kind = {
        let idx = type_idx.saturating_sub(1) as usize;
        module.module.type_defs.get(idx).map(|t| t.kind)
    };
    let field_count = helpers::get_type_field_count(&module.module, type_idx);

    match kind.and_then(|k| writ_module::TypeDefKind::from_u8(k)) {
        Some(writ_module::TypeDefKind::Struct) => {
            // kind=0: inline in register, no heap allocation
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::InlineStruct {
                type_idx,
                fields: vec![Value::Void; field_count],
            };
            ExecutionResult::Continue
        }
        Some(writ_module::TypeDefKind::Class) => {
            // kind=4: heap allocation (current behavior preserved)
            let href = ctx.heap.alloc_struct(field_count);
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = Value::Ref(href);
            ExecutionResult::Continue
        }
        Some(other) => ExecutionResult::Crash(format!(
            "NEW: type_idx {} has kind {:?}, not a struct or class",
            type_idx, other
        )),
        None => ExecutionResult::Crash(format!(
            "NEW: type_idx {} is out of range", type_idx
        )),
    }
}
```

**Key detail:** `TypeDefRow.kind` is a raw `u8` — use `TypeDefKind::from_u8()` from `writ_module::tables` (already re-exported via `writ_module`). Access: `module.module.type_defs[idx].kind` where `idx = type_idx.saturating_sub(1)`.

### Pattern 3: GET_FIELD/SET_FIELD Variant Dispatch

**What:** Match on Value variant first; only call into heap for Ref.
**When to use:** Every GET_FIELD and SET_FIELD execution.

```rust
// Source: writ-runtime/src/dispatch/objects.rs — after change

pub(super) fn exec_get_field(
    ctx: &mut ExecContext<'_>,
    r_dst: u16, r_obj: u16, field_idx: u32,
) -> ExecutionResult {
    let val = ctx.task.call_stack.last().unwrap().registers[r_obj as usize].clone();
    match val {
        Value::InlineStruct { fields, .. } => {
            let idx = field_idx as usize;
            let field_val = fields.get(idx).cloned().unwrap_or_else(|| {
                // crash path — return as ExecutionResult::Crash
                return Value::Void; // placeholder, see full impl
            });
            // ... store to r_dst
        }
        Value::Ref(href) => {
            // existing heap path unchanged
        }
        other => ExecutionResult::Crash(format!(
            "GetField: expected struct or class, got {:?}", other
        )),
    }
}

pub(super) fn exec_set_field(
    ctx: &mut ExecContext<'_>,
    r_obj: u16, field_idx: u32, r_val: u16,
) -> ExecutionResult {
    let val = ctx.task.call_stack.last().unwrap().registers[r_val as usize].clone();
    let idx = field_idx as usize;
    let frame = ctx.task.call_stack.last_mut().unwrap();
    match &mut frame.registers[r_obj as usize] {
        Value::InlineStruct { fields, .. } => {
            if idx < fields.len() {
                fields[idx] = val;
                ExecutionResult::Continue
            } else {
                ExecutionResult::Crash(format!(
                    "SetField: field index {} out of range", idx
                ))
            }
        }
        Value::Ref(href) => {
            // existing heap path — need href copy before borrow
            let href = *href;
            match ctx.heap.set_field(href, idx, val) {
                Ok(()) => ExecutionResult::Continue,
                Err(e) => ExecutionResult::Crash(format!("SetField: {}", e)),
            }
        }
        _ => ExecutionResult::Crash("SetField: not a struct or class".into()),
    }
}
```

**Critical borrow pattern for SET_FIELD on InlineStruct:** The register is mutated in-place. Use `&mut frame.registers[r_obj as usize]` with a match on the mutable reference. For the Ref path, copy the href out before calling back into ctx.heap (to avoid double-borrow of ctx).

### Pattern 4: GC Root Collection for InlineStruct

**What:** `collect_roots()` must recurse into InlineStruct fields to find embedded Refs.
**When to use:** GC root collection pass in `runtime.rs`.

```rust
// Source: writ-runtime/src/runtime.rs — collect_roots() after change
fn collect_roots(&self) -> Vec<HeapRef> {
    let mut roots = Vec::new();

    for task in self.scheduler.tasks.values() {
        for frame in &task.call_stack {
            for reg in &frame.registers {
                collect_value_refs(reg, &mut roots);
            }
        }
        if let Some(ref rv) = task.return_value {
            collect_value_refs(rv, &mut roots);
        }
    }

    for global in &self.scheduler.globals {
        collect_value_refs(global, &mut roots);
    }

    for (_entity_id, slot) in self.scheduler.entity_registry.alive_entities() {
        if let Some(href) = slot.data_ref {
            roots.push(href);
        }
    }

    roots
}

// Helper (can be a free function in runtime.rs or gc.rs)
fn collect_value_refs(val: &Value, roots: &mut Vec<HeapRef>) {
    match val {
        Value::Ref(href) => roots.push(*href),
        Value::InlineStruct { fields, .. } => {
            for field in fields {
                collect_value_refs(field, roots);  // recursive for nested structs
            }
        }
        _ => {}
    }
}
```

**Depth:** Recursion handles nested InlineStructs (a struct whose field is another struct). The spec does not prohibit nested value types.

### Pattern 5: trace_refs for Boxed InlineStruct

**What:** `trace_refs()` in gc.rs must extract Refs from a boxed InlineStruct.
**Current state:** `HeapObject::Boxed(v)` arm only handles `Value::Ref(href)` directly — misses InlineStruct fields.

```rust
// Source: writ-runtime/src/gc.rs — trace_refs after change
HeapObject::Boxed(v) => {
    collect_value_refs(v, &mut refs);  // reuse the same helper
}
```

This requires either moving `collect_value_refs` to gc.rs or making it accessible from both.

### Pattern 6: MOV Clone

**What:** `exec_mov` must clone instead of copy-assign.
**Current code (arith.rs line 23):**
```rust
frame.registers[r_dst as usize] = frame.registers[r_src as usize];
```
**After change:**
```rust
frame.registers[r_dst as usize] = frame.registers[r_src as usize].clone();
```
Similarly `exec_convert` (arith.rs line 363).

### Pattern 7: UNBOX Clone

**What:** `exec_unbox` must clone the value out of the Boxed wrapper.
**Current code (arith.rs):**
```rust
Ok(crate::heap::HeapObject::Boxed(val)) => {
    let val = *val;  // Copy dereference — breaks when Value loses Copy
```
**After change:**
```rust
Ok(crate::heap::HeapObject::Boxed(val)) => {
    let val = val.clone();
```

### Pattern 8: get_field Return Type Change

**What:** `get_field` in heap.rs and gc.rs returns `Result<Value, RuntimeError>`. Currently uses `.copied()` to copy from Vec<Value>. After removing Copy, use `.cloned()`.

**heap.rs BumpHeap::get_field:**
```rust
// Before:
fields.get(idx).copied().ok_or_else(|| ...)
// After:
fields.get(idx).cloned().ok_or_else(|| ...)
```

Same change needed in `MarkSweepHeap::get_field` (gc.rs lines 214, 221).

Note: Since `get_field` on the `GcHeap` trait returns `Result<Value, RuntimeError>` (by value), it will work correctly with cloned Values — callers receive owned Values as before.

### Anti-Patterns to Avoid

- **Multi-register approach:** Never span struct fields across multiple register indices. The spec is explicit: one register = one abstract typed slot. Only one register index is written by NEW.
- **New HeapObject variant for class:** Classes (kind=4) reuse `HeapObject::Struct { fields }` — the same heap object type used by the old struct model. No new HeapObject variant.
- **Type metadata lookup in GET/SET_FIELD:** Do NOT look up TypeDef kind on every field access. Match on Value variant only — it's O(1) pattern match with no table lookup.
- **Deep clone of heap objects on MOV:** MOV performs a shallow clone of InlineStruct — primitive fields are value-copied, Ref fields copy the HeapRef pointer (same heap object). Do NOT deep-clone heap objects.
- **Forgetting exec_convert:** `exec_convert` (arith.rs:363) also copies a register value. It must also use `.clone()`.
- **Missing recursion in collect_value_refs:** Nested structs (InlineStruct containing another InlineStruct as a field) must be handled recursively. Flat iteration is insufficient.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Kind lookup | Custom kind cache | `TypeDefRow.kind` + `TypeDefKind::from_u8()` | Already implemented in Phase 48 |
| Field count | Re-implement field counting | `helpers::get_type_field_count()` | Already correct, handles 1-based token math |
| Heap object for class | New HeapObject variant | `HeapObject::Struct { fields }` | Identical structure, no rename needed |
| GcHeap abstraction | Direct heap mutation | `ctx.heap.alloc_struct()` / `ctx.heap.set_field()` | Both BumpHeap and MarkSweepHeap must work |

**Key insight:** The existing infrastructure (TypeDefKind enum, get_type_field_count, HeapObject::Struct, GcHeap trait) is exactly what Phase 49 needs. The work is wiring new dispatch paths, not building new abstractions.

## Common Pitfalls

### Pitfall 1: Borrow Conflict in SET_FIELD on InlineStruct
**What goes wrong:** Reading `r_val` register and mutating `r_obj` register in the same frame — Rust borrow checker rejects `&frame.registers[r_val]` and `&mut frame.registers[r_obj]` simultaneously.
**Why it happens:** `Vec<Value>` doesn't allow simultaneous immutable and mutable indexing.
**How to avoid:** Clone the value from `r_val` before taking the mutable reference to `r_obj`.
```rust
let val = frame.registers[r_val as usize].clone();  // extract first
// then match &mut frame.registers[r_obj as usize]
```
**Warning signs:** `cannot borrow frame.registers as mutable because it is also borrowed as immutable`

### Pitfall 2: Double Borrow for SET_FIELD Ref Path
**What goes wrong:** In the `Value::Ref(href)` arm of exec_set_field, calling `ctx.heap.set_field(href, ...)` while `frame` (which holds the registers) is still borrowed.
**Why it happens:** `frame = ctx.task.call_stack.last_mut()` and `ctx.heap` are both behind `ctx`, so the borrow checker sees a conflict.
**How to avoid:** Copy `href` out (HeapRef is `Copy`) before ending the frame borrow:
```rust
let href = match &frame.registers[r_obj as usize] {
    Value::Ref(h) => *h,
    // ...
};
drop(frame_borrow);  // or restructure to not hold frame across heap call
ctx.heap.set_field(href, idx, val)
```
**Warning signs:** `cannot borrow ctx as mutable more than once at a time`

### Pitfall 3: Incomplete Copy Removal
**What goes wrong:** Some `.copied()` calls on `Vec<Value>` elements compile fine with the old Value (Copy) but fail after removing Copy derive. Missing a site means the build fails at an unexpected location.
**Why it happens:** Copy impls are implicit — they're used everywhere without explicit syntax.
**How to avoid:** Let `cargo build` enumerate all failures after removing `#[derive(Copy)]`. Fix them systematically. Key sites:
- `heap.rs`: `get_field` — 2 `.copied()` calls (lines 101, 110)
- `gc.rs`: `get_field` in MarkSweepHeap — 2 `.copied()` calls (lines 214, 221)
- `runtime.rs`: `register_value()` — `.copied()` on `f.registers.get(reg)`
- `dispatch/arith.rs`: `exec_mov` (line 23), `exec_convert` (line 363)
- `dispatch/objects.rs`: multiple `let val = frame.registers[i]` direct copies
- `dispatch/mod.rs:289`: `ret_val` extraction from registers
**Warning signs:** Compiler error at each `let x = some_value_ref;` or `.copied()`

### Pitfall 4: return_value Option<Value> needs .clone()
**What goes wrong:** `task.return_value = Some(ret_val)` stores the Value. Later `t.return_value` is read with `.and_then(|t| t.return_value)` — this moves out of Option<Value>, which requires Copy.
**Why it happens:** `Option<Value>` pattern matching moves Value when Value is not Copy.
**How to avoid:** Use `t.return_value.clone()` in read paths; the write path stores an owned Value so is fine.

### Pitfall 5: GC Root Missing Nested Struct Refs
**What goes wrong:** A struct has a field that is itself an InlineStruct, which has a Ref field. Flat iteration over register values finds the outer InlineStruct but misses the nested Ref.
**Why it happens:** The original root collector only matches `Value::Ref(href)` — one level deep.
**How to avoid:** Use the recursive `collect_value_refs` helper that recurses into InlineStruct fields.
**Warning signs:** GC frees objects that are still referenced via nested structs.

### Pitfall 6: array_init and similar Read from Consecutive Registers
**What goes wrong:** `exec_array_init` reads `frame.registers[r_base + i]` in a loop. After Value loses Copy, this must use `.clone()`.
**Why it happens:** Direct index assignment implicitly copies with Copy; becomes a move with Clone.
**How to avoid:** `.clone()` on each element in the loop.
**Warning signs:** `cannot move out of index` compiler error

## Code Examples

### Full Value Enum After Change
```rust
// Source: writ-runtime/src/value.rs

#[derive(Debug, Clone)]  // Copy removed
pub enum Value {
    Void,
    Int(i64),
    Float(f64),
    Bool(bool),
    Ref(HeapRef),
    Entity(EntityId),
    InlineStruct { type_idx: u32, fields: Vec<Value> },
}
```

### exec_mov After Change
```rust
// Source: writ-runtime/src/dispatch/arith.rs
pub(super) fn exec_mov(ctx: &mut ExecContext<'_>, r_dst: u16, r_src: u16) -> ExecutionResult {
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = frame.registers[r_src as usize].clone();
    ExecutionResult::Continue
}
```

### TypeDef Kind Lookup Pattern
```rust
// Source: writ-runtime/src/dispatch/helpers.rs (existing pattern, extend for kind)
// TypeDefRow.kind is raw u8; type_idx is 1-based MetadataToken
let idx = type_idx.saturating_sub(1) as usize;
let kind = module.type_defs.get(idx).map(|t| t.kind);
let type_kind = kind.and_then(writ_module::tables::TypeDefKind::from_u8);
```

### collect_value_refs Helper
```rust
// Can live in gc.rs or runtime.rs
pub fn collect_value_refs(val: &Value, refs: &mut Vec<HeapRef>) {
    match val {
        Value::Ref(href) => refs.push(*href),
        Value::InlineStruct { fields, .. } => {
            for field in fields {
                collect_value_refs(field, refs);
            }
        }
        _ => {}
    }
}
```

### trace_refs Boxed Arm After Change
```rust
// Source: writ-runtime/src/gc.rs — trace_refs function
HeapObject::Boxed(v) => {
    // Before: if let Value::Ref(href) = v { refs.push(*href); }
    // After: handle InlineStruct inside Boxed
    match v {
        Value::Ref(href) => refs.push(*href),
        Value::InlineStruct { fields, .. } => {
            for field in fields {
                if let Value::Ref(href) = field {
                    refs.push(*href);
                }
                // Note: deeply nested structs in a Boxed are unusual but possible
                // For safety, recurse if needed
            }
        }
        _ => {}
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| All structs heap-allocated via `alloc_struct` | kind=0 structs inline in register; kind=4 classes heap-allocated | Phase 49 | NEW dispatch branches; GC root scan must recurse into registers |
| `Value` has `#[derive(Copy)]` | `Value` is `Clone`-only (Copy removed) | Phase 49 | Mechanical `.clone()` additions throughout runtime; all implicit copies become explicit |
| `exec_get_field` always goes through heap | Variant dispatch: InlineStruct reads fields directly | Phase 49 | No heap round-trip for struct field reads |
| `trace_refs(Boxed(v))` only handles `Value::Ref` | Must also handle `Value::InlineStruct { fields }` | Phase 49 | GC correctness for boxed structs with embedded refs |

## Open Questions

1. **collect_value_refs placement**
   - What we know: needed in both `runtime.rs` (collect_roots) and `gc.rs` (trace_refs Boxed arm)
   - What's unclear: whether to put it in `gc.rs` (as a public/pub(crate) function) or duplicate it
   - Recommendation: Place in `gc.rs` alongside `trace_refs`, make it `pub(crate)`, import in `runtime.rs`

2. **Intrinsic register copies (dispatch/intrinsics.rs lines 279, 345)**
   - What we know: Two sites copy registers by direct assignment (like exec_mov)
   - What's unclear: Whether InlineStructs would ever appear in these paths
   - Recommendation: Add `.clone()` defensively — the cost for non-InlineStruct values is negligible

3. **PartialEq/Eq for InlineStruct**
   - What we know: `Value` implements `PartialEq` and `Eq` via custom impl
   - What's unclear: Whether the new `InlineStruct` variant needs equality (used in tests comparing values)
   - Recommendation: Add `(Value::InlineStruct { type_idx: a, fields: fa }, Value::InlineStruct { type_idx: b, fields: fb }) => a == b && fa == fb` arm to the PartialEq impl

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test framework (cargo test) |
| Config file | Cargo.toml workspace |
| Quick run command | `cargo test -p writ-runtime` |
| Full suite command | `cargo test -p writ-runtime` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VM-01 | NEW on kind=0 creates InlineStruct in register (no heap alloc) | unit | `cargo test -p writ-runtime inline_struct_no_heap_alloc` | Wave 0 |
| VM-02 | MOV of InlineStruct register produces independent copy | unit | `cargo test -p writ-runtime mov_inline_struct_independent_copy` | Wave 0 |
| VM-03 | NEW on kind=4 allocates heap object; NEW on kind=0 does not | unit | `cargo test -p writ-runtime new_kind_dispatch` | Wave 0 |
| VM-04 | GC does not free heap objects referenced by InlineStruct fields | unit | `cargo test -p writ-runtime gc_traces_inline_struct_ref_fields` | Wave 0 |
| VM-05 | BOX on InlineStruct produces Boxed(InlineStruct); UNBOX recovers it | unit | `cargo test -p writ-runtime box_unbox_inline_struct` | Wave 0 |
| VM-06 | GET_FIELD/SET_FIELD on class (kind=4) Ref still works (regression) | unit | `cargo test -p writ-runtime class_field_access_regression` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime`
- **Per wave merge:** `cargo test -p writ-runtime`
- **Phase gate:** Full suite green (all writ-runtime tests) before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] New tests in `writ-runtime/tests/vm_tests.rs` or a new `writ-runtime/tests/struct_semantics_tests.rs` covering VM-01 through VM-06
- [ ] Framework already installed — no setup needed

## Sources

### Primary (HIGH confidence)
- Direct source inspection of `writ-runtime/src/value.rs` — current Value enum, Copy/Clone derives
- Direct source inspection of `writ-runtime/src/heap.rs` — HeapObject variants, BumpHeap, alloc_struct
- Direct source inspection of `writ-runtime/src/gc.rs` — trace_refs, MarkSweepHeap, GcHeap trait
- Direct source inspection of `writ-runtime/src/dispatch/objects.rs` — exec_new, exec_get_field, exec_set_field
- Direct source inspection of `writ-runtime/src/dispatch/arith.rs` — exec_mov, exec_box, exec_unbox
- Direct source inspection of `writ-runtime/src/runtime.rs` — collect_roots, collect_garbage
- Direct source inspection of `writ-module/src/tables.rs` — TypeDefKind::from_u8, TypeDefRow.kind
- `.planning/phases/49-vm-runtime/49-CONTEXT.md` — all locked implementation decisions
- `grep` of `.copied()` usage across writ-runtime — 6 sites identified

### Secondary (MEDIUM confidence)
- None needed — all findings from direct source inspection

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all types and functions verified by reading source
- Architecture: HIGH — patterns derived from existing dispatch function structure
- Pitfalls: HIGH — borrow conflicts derived from reading actual Rust code patterns; Copy removal sites enumerated by grep

**Research date:** 2026-03-12
**Valid until:** 2026-04-12 (stable codebase, no external dependencies)
