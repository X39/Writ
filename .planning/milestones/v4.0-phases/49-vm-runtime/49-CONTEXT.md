# Phase 49: VM Runtime - Context

**Gathered:** 2026-03-12
**Status:** Ready for planning

<domain>
## Phase Boundary

The VM executes struct and class values with correct semantics: structs live inline in registers and copy on assignment (value semantics), classes allocate on the heap and share references (reference semantics). GC correctly traces both. BOX/UNBOX handles value-type struct boxing for generics. Covers VM-01 through VM-06.

</domain>

<decisions>
## Implementation Decisions

### Register Model for Inline Structs
- Each register is "a single abstract typed slot" per spec §2.15.1 — structs occupy ONE register containing all fields, not multiple contiguous registers
- Add `Value::InlineStruct { type_idx: u32, fields: Vec<Value> }` variant to the Value enum
- `Value` loses `Copy` trait, becomes `Clone`-only — mechanical ripple through runtime (add `.clone()` where compiler complains)
- MOV clones the Value — for InlineStruct this shallow-copies the Vec (independent field values, but Ref fields share the same heap objects)
- Shallow copy semantics: primitive fields are copied, reference fields copy the HeapRef (same heap object). Matches spec and C# value-type behavior

### NEW Instruction Kind-Dependent Behavior
- Direct TypeDef table lookup at dispatch time: `module.type_defs[type_idx].kind` — one array index, negligible cost
- kind=0 (struct): create `Value::InlineStruct { type_idx, fields: vec![Value::Void; field_count] }` in destination register — no heap allocation
- kind=4 (class): heap-allocate via existing `alloc_struct(field_count)` path, store `Value::Ref(href)` — current behavior preserved
- Unexpected kinds (enum=1, entity=2, component=3): crash with descriptive message ("NEW: type_idx {} is an enum, not a struct or class")

### GET_FIELD/SET_FIELD Dispatch Split
- Match on Value variant at dispatch time — no type metadata lookup needed:
  - `Value::InlineStruct { fields, .. }` → read/write `fields[field_idx]` directly in the register
  - `Value::Ref(href)` → go through heap (existing path, unchanged)
  - `Value::Entity(eid)` → existing entity path (unchanged)
  - Other variants → crash with type mismatch message
- SET_FIELD on InlineStruct mutates in place (modify `fields[field_idx]` directly) — register owns its copy, other MOV'd copies are independent
- Classes (kind=4) reuse `HeapObject::Struct { fields }` on the heap — same heap object type, no new variant needed. The only difference from old structs is allocation path (NEW kind-check)

### BOX/UNBOX for Struct Values
- `HeapObject::Boxed(Value)` unchanged — `Boxed(Value::InlineStruct { type_idx, fields })` stores the entire struct value
- No new HeapObject variant needed
- UNBOX clones the InlineStruct value back out of the Boxed wrapper
- UNBOX type mismatch crashes (consistent with existing behavior)

### GC Tracing
- `trace_refs` updated to trace through `Boxed(Value::InlineStruct { fields, .. })` — scan fields for `Value::Ref` to keep referenced heap objects alive
- Register root collection already scans all registers — `Value::InlineStruct` fields containing `Value::Ref` are NOT directly visible to the root collector (they're inside the InlineStruct). The `trace_refs` path handles this when the struct is boxed. For register-inline structs, the root collector must be updated to also scan InlineStruct fields for Refs.

### MOV Behavior
- MOV just clones whatever Value is in the source register — no type metadata lookup
- `Value::InlineStruct.clone()` deep-copies the fields Vec (shallow copy of each Value within)
- `Value::Ref.clone()` copies the pointer (trivial, same as before)

### Claude's Discretion
- Exact error message wording for crash scenarios
- Whether to add helper methods on Value (e.g., `value.as_inline_struct()`, `value.as_inline_struct_mut()`)
- Test organization (new test files vs extending existing test modules)
- Whether to optimize InlineStruct with SmallVec for common small structs (Vec2, Vec3) — not required, can be added later
- GC root collection implementation detail for scanning InlineStruct fields in registers

</decisions>

<specifics>
## Specific Ideas

- User confirmed registers are "virtually endless in size" / auto-scaled — one register per value, runtime determines physical storage
- Spec §2.15.1 is authoritative: "the register holds all fields inline as a single abstract typed slot"
- MOV "multi-word copy" means copying all field data within one register, NOT spanning multiple register indices
- Strict crash-on-error model consistent with existing runtime patterns (SET_FIELD on wrong type, UNBOX mismatch, etc.)

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Value` enum in `value.rs`: add InlineStruct variant, remove `Copy` derive, keep `Clone`
- `HeapObject::Struct { fields }`: reused for class heap allocation (kind=4) — no rename needed
- `HeapObject::Boxed(Value)`: works for boxing InlineStruct values directly
- `TypeDefKind::from_u8()` in writ-module: already available for kind lookup at NEW dispatch
- `helpers::get_type_field_count()`: already looks up field count from TypeDef table

### Established Patterns
- Dispatch functions in `dispatch/objects.rs` match on `Value` variant or call `helpers::extract_ref()`
- `ExecutionResult::Crash(msg)` for runtime errors — consistent error model
- `ExecContext` provides `ctx.modules[ctx.current_module_idx]` for module metadata access
- `GcHeap` trait abstracts heap implementations — `BumpHeap` and `MarkSweepHeap` both implement it

### Integration Points
- `value.rs`: Value enum change (add InlineStruct, drop Copy)
- `dispatch/objects.rs`: exec_new (kind-check), exec_get_field/exec_set_field (variant dispatch)
- `dispatch/arith.rs`: exec_mov (clone semantics), exec_box/exec_unbox (InlineStruct support)
- `gc.rs`: trace_refs (scan InlineStruct fields in Boxed), root collection (scan InlineStruct in registers)
- `frame.rs`: CallFrame.registers Vec<Value> — no change needed (Value is still the element type)
- `heap.rs`: BumpHeap and MarkSweepHeap — alloc_struct still used for class allocation
- All files using `Value` — mechanical .clone() additions where Copy was relied upon

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 49-vm-runtime*
*Context gathered: 2026-03-12*
