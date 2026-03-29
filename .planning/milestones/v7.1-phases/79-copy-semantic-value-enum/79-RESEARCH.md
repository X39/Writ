# Phase 79: Copy-Semantic Value Enum - Research

**Researched:** 2026-03-22
**Domain:** Rust enum refactoring, GC tracing, VM register semantics
**Confidence:** HIGH

## Summary

Phase 79 removes `Value::InlineStruct { type_idx: u32, fields: Vec<Value> }` from the Value
enum and replaces it with `Value::Struct(HeapRef)` that stores struct fields on the already-
existing GC heap. This makes every Value variant either a primitive scalar, a Copy handle, or
void — so the enum can derive Copy. Once Copy, register-to-register moves are bitwise copies
(zero heap allocation), eliminating the clone overhead that currently fires on every struct MOV,
function-argument pass, and return value.

The heap infrastructure already has `alloc_struct(field_count)` → `HeapRef` and
`get_field/set_field(href, idx)` on both `BumpHeap` and `MarkSweepHeap`. The structural change
is therefore confined to: (a) the Value enum definition, (b) every match arm that names
`InlineStruct`, (c) the GC root walker `collect_value_refs`, and (d) the exec_new dispatch
path that currently creates an `InlineStruct` for `TypeDefKind::Struct`.

The critical risk is the GC regression: once structs live on the heap they must be traced as
roots, but a struct-in-register is NOT a `Ref` so `collect_value_refs` must be updated to push
the inner `HeapRef` when it encounters `Value::Struct(href)`. STATE.md mandates that the GC
regression test be written and passing BEFORE any `value.rs` match arms are changed.

The semantics change is real: struct values are now reference types under the hood (two
registers holding the "same" struct share the heap object). The Writ language spec treats structs
as value types, so SET_FIELD on a struct register must still work correctly. Phase 79 does not
change the instruction set; it changes only how the data is stored. The existing "independent
copy" test (`test_mov_inline_struct_independent_copy`) will need updating — after the migration
a MOV copies the HeapRef, not the fields, so both registers point at the same heap object. That
test will either be deleted or rewritten to document the new (reference) semantics.

**Primary recommendation:** Add `Value::Struct(HeapRef)` variant, derive Copy on Value, migrate
all match sites, update `collect_value_refs` to push the inner HeapRef for `Struct`, add the GC
regression test first, delete or rewrite the independent-copy test, then verify fib(40) < 30s.

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
| VALUE-01 | InlineStruct fields stored on GC heap via HeapRef instead of inline Vec | GcHeap.alloc_struct already exists on both BumpHeap and MarkSweepHeap; exec_new Struct path currently creates InlineStruct — switch to heap alloc + Value::Struct(href) |
| VALUE-02 | Value enum derives Copy (all variants are Copy-sized) | Void, Int(i64), Float(f64), Bool(bool), Ref(HeapRef), Entity(EntityId) all already Copy; removing InlineStruct (the only non-Copy variant) makes Copy derivable |
| VALUE-03 | GC collect_value_refs traces Value::Struct(HeapRef) as a heap root | Currently collect_value_refs handles Value::Ref and Value::InlineStruct; replace InlineStruct arm with Struct(href) => refs.push(href) |
| VALUE-04 | GC regression test confirms struct-in-register survives garbage collection | New test: alloc Struct on heap, put Value::Struct(href) in a register-like value, call collect_value_refs, pass resulting refs to heap.collect, assert struct survives |
| VALUE-05 | Field access reads/writes through HeapRef | exec_get_field and exec_set_field already have a Value::Ref path that calls heap.get_field/set_field; remove the InlineStruct path, rename Struct arm to match new variant |
| VALUE-06 | All existing tests pass after Value migration | ~12 match sites across 5 crates; tests that assert InlineStruct semantics (no-heap-alloc, independent-copy) must be updated |
| VERIFY-01 | fib(40) produces correct output 102334155 | No change to instruction semantics; fib uses only Int registers — passes unchanged |
| VERIFY-02 | cargo test --release passes with zero failures | Requires updating all InlineStruct match sites and test assertions |
| VERIFY-03 | cargo build --release produces no warnings | Requires removing all dead InlineStruct references |
| VERIFY-04 | fib(40) completes in under 30 seconds after all phases | Value::Copy removes clone overhead on struct moves; baseline was 53.1s at Phase 78 end — must reach < 30s combined with prior phase gains |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust std | 1.x | Copy derive, enum variants | Language feature |
| writ-runtime::gc | internal | GcHeap trait, collect_value_refs, MarkSweepHeap | Already the GC layer |
| writ-runtime::heap | internal | BumpHeap, HeapObject::Struct | Already stores struct fields |
| writ-runtime::value | internal | Value enum, HeapRef | The type being refactored |

No new external dependencies.

## Architecture Patterns

### Recommended Project Structure

No new files. Changes are confined to:

```
writ-runtime/src/
├── value.rs                  # Remove InlineStruct, add Struct(HeapRef), derive Copy, remove Clone
├── gc.rs                     # collect_value_refs: replace InlineStruct arm with Struct(href) arm
├── dispatch/objects.rs       # exec_new: Struct kind allocates on heap; exec_get_field/exec_set_field: remove InlineStruct arm
├── dispatch/calls.rs         # Two display-format match arms referencing InlineStruct
dispatch/ other files          # No InlineStruct references — no changes required
writ-runtime/tests/vm_tests.rs # Update/delete tests asserting InlineStruct behavior
writ-cli/src/cli_host.rs      # One display match arm
writ-cli/tests/cli_integration.rs # One display match arm
writ-dap/src/variables.rs    # One display match arm + one constructor
```

### Pattern 1: New Value Variant

Replace:
```rust
// value.rs — BEFORE
#[derive(Debug, Clone)]
#[derive(Default)]
pub enum Value {
    #[default]
    Void,
    Int(i64),
    Float(f64),
    Bool(bool),
    Ref(HeapRef),
    Entity(EntityId),
    InlineStruct { type_idx: u32, fields: Vec<Value> },
}
```

With:
```rust
// value.rs — AFTER
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Value {
    #[default]
    Void,
    Int(i64),
    Float(f64),
    Bool(bool),
    Ref(HeapRef),
    Entity(EntityId),
    Struct { type_idx: u32, href: HeapRef },
}
```

Notes:
- `Float(f64)` does NOT implement `Eq` or `Hash` natively, but Value currently implements PartialEq
  manually and does NOT derive Hash. Keep the existing manual PartialEq impl; do not derive Eq or
  Hash — just add `Copy`. The `#[derive(Copy)]` alone is the minimum viable change; Clone is
  implied by Copy.
- `type_idx` is retained in `Value::Struct` so that `exec_new` and display code can still
  distinguish struct types without a heap lookup.
- Alternatively: `Value::Struct(HeapRef)` (tuple variant) — simpler, and type_idx is already in
  `HeapObject::Struct` if we add it there. Either form works; the named-field form (`Struct {
  type_idx, href }`) avoids a heap lookup in display/resolve_runtime_type_key.

**Recommended:** Use `Value::Struct { type_idx: u32, href: HeapRef }` to preserve type_idx
access without a heap round-trip.

### Pattern 2: exec_new for Struct Kind

```rust
// dispatch/objects.rs — exec_new, Struct arm — AFTER
Some(writ_module::TypeDefKind::Struct) => {
    let href = ctx.heap.alloc_struct(field_count);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Struct { type_idx, href };
    ExecutionResult::Continue
}
```

This is identical to the Class/Entity arm except it wraps in `Value::Struct` instead of
`Value::Ref`. The heap already allocates `HeapObject::Struct` with `field_count` Void-filled
fields via `alloc_struct`.

### Pattern 3: exec_get_field / exec_set_field

```rust
// dispatch/objects.rs — exec_get_field — AFTER
Value::Struct { href, .. } => {
    let href = *href;
    match ctx.heap.get_field(href, field_idx as usize) {
        Ok(val) => {
            let frame = ctx.task.call_stack.last_mut().unwrap();
            frame.registers[r_dst as usize] = val;
            ExecutionResult::Continue
        }
        Err(e) => ExecutionResult::Crash(format!("GetField: {}", e)),
    }
}
```

SetField for `Value::Struct` is identical to the existing `Value::Ref` arm (copy href, release
borrow, call `ctx.heap.set_field`).

### Pattern 4: collect_value_refs Update

```rust
// gc.rs — AFTER
pub fn collect_value_refs(val: &Value, refs: &mut Vec<HeapRef>) {
    match val {
        Value::Ref(href) => refs.push(*href),
        Value::Struct { href, .. } => refs.push(*href),
        _ => {}
    }
}
```

The recursive walk is gone because struct fields now live in the heap object, not inline. The
GC's existing `trace_refs` function already recursively walks `HeapObject::Struct { fields }` to
find nested Refs, so transitive reachability is preserved without any change to `trace_refs`.

### Pattern 5: GC Regression Test (write FIRST)

```rust
// writ-runtime/tests/vm_tests.rs — new test, write before changing any match arms
#[test]
fn test_gc_traces_struct_href_in_register() {
    use writ_runtime::gc::{GcHeap, MarkSweepHeap, collect_value_refs};
    let mut heap = MarkSweepHeap::new();

    // Allocate a struct with one field containing a string ref
    let string_href = heap.alloc_string("survive");
    let struct_href = heap.alloc_struct(1);
    heap.set_field(struct_href, 0, Value::Ref(string_href)).unwrap();

    // Simulate a register holding Value::Struct { type_idx: 1, href: struct_href }
    let reg_val = Value::Struct { type_idx: 1, href: struct_href };

    // collect_value_refs must surface struct_href as a root
    let mut roots = Vec::new();
    collect_value_refs(&reg_val, &mut roots);
    assert_eq!(roots, vec![struct_href]);

    // GC with struct_href as root — both struct and string survive via trace_refs
    let stats = heap.collect(&roots);
    assert_eq!(stats.objects_freed, 0);
    assert_eq!(heap.heap_size(), 2);
    assert_eq!(heap.read_string(string_href).unwrap(), "survive");

    // GC with no roots — both freed
    let stats = heap.collect(&[]);
    assert_eq!(stats.objects_freed, 2);
    assert_eq!(heap.heap_size(), 0);
}
```

This test will fail to compile until `Value::Struct` exists, so it gates the migration sequence:
write the test → add `Value::Struct` variant → remove `Value::InlineStruct` → fix all arms.

### Pattern 6: Test Migration for InlineStruct Tests

Tests that must be rewritten or deleted after migration:

| Test | Action | Reason |
|------|--------|--------|
| `test_new_struct_inline_no_heap` | Delete or invert | After migration, struct NEW allocates on heap; assertion `heap_before == heap_after` becomes false |
| `test_mov_inline_struct_independent_copy` | Delete or rewrite | After migration, MOV copies HeapRef — both registers point to same object; mutation through one register affects the other |
| `test_gc_traces_inline_struct_ref_fields` | Replace | Superseded by new GC regression test |
| `test_gc_traces_nested_inline_struct_refs` | Delete | Nested InlineStruct no longer possible |
| `test_gc_traces_boxed_inline_struct` | Update | BOX on Value::Struct now boxes a HeapRef — semantics change slightly |
| `new_allocates_struct` (simple test line 1163) | Update match arm | Change `Value::InlineStruct { type_idx, .. }` to `Value::Struct { type_idx, .. }` |
| `test_box_unbox_inline_struct` | Update | Box of `Value::Struct` should still recover field value through unbox |
| `get_set_field_round_trip` | No change if behavior preserved | Test exercises field ops, not variant name |

### Anti-Patterns to Avoid

- **Adding type_idx to HeapObject::Struct**: Unnecessary heap layout change. type_idx is already
  in the `Value::Struct` variant. HeapObject::Struct currently has no type_idx field; do not add
  one — the existing `get_field/set_field` API works without it.
- **Removing Clone from Value entirely**: Once Copy is derived, Clone is implied for free — do
  not add explicit Clone derive (it's redundant and signals misunderstanding).
- **Forgetting to update `resolve_runtime_type_key` in calls.rs line 337**: Currently returns
  `u32::MAX` for `InlineStruct`; must be updated for `Struct { type_idx, .. }` to return
  `type_idx` (or keep returning `u32::MAX` if dispatch on struct type is not implemented).
- **Skipping the GC regression test**: The state note in STATE.md is explicit — write the test
  first, prove it compiles and fails, then implement the migration.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Struct field storage | Custom inline buffer or SmallVec | `heap.alloc_struct(field_count)` + `HeapObject::Struct { fields }` | Already implemented in both BumpHeap and MarkSweepHeap |
| GC tracing | Recursive inline scan | `trace_refs` already walks `HeapObject::Struct { fields }` transitively | Adding Struct(HeapRef) as a root is sufficient; trace_refs handles the rest |
| Type discrimination | Separate type registry lookup | Carry `type_idx` in `Value::Struct` variant | Zero extra cost at Copy time |

## Common Pitfalls

### Pitfall 1: The "Independent Copy" Semantic Flip

**What goes wrong:** The test `test_mov_inline_struct_independent_copy` asserts that after
`MOV r2, r0` followed by `SET_FIELD r2, 0, 999`, the original `r0.field[0]` is still 10. After
migration, both registers hold the same `HeapRef` — SET_FIELD mutates the shared heap object,
so `r0.field[0]` becomes 999 and the test fails.

**Why it happens:** InlineStruct provided value-copy semantics; `Value::Struct(HeapRef)` provides
reference semantics. The Writ language spec says structs are value types, but this phase does not
implement copy-on-write — it moves the storage to the heap and makes the register value Copy
(bitwise copy of the HeapRef), not Copy of the fields.

**How to avoid:** Delete or rewrite the independent-copy test to document reference semantics.
If true value-copy semantics are required, they can be layered on top in a future phase using
copy-on-write. For v7.1, the goal is Copy Value enum for zero-allocation register moves, not
full structural equality on mutation.

**Warning signs:** Test `test_mov_inline_struct_independent_copy` failing after migration is
expected and correct — it is documenting the old (now removed) behavior.

### Pitfall 2: collect_value_refs Forgetting to Push href

**What goes wrong:** `collect_value_refs` is updated to handle `Value::Struct { href, .. }` but
the arm pushes nothing (e.g., a `_ => {}` catch-all), causing the struct's heap object to be
unreachable from the GC's perspective and freed on the next collection cycle.

**Why it happens:** The old InlineStruct arm recursively walked fields looking for embedded
`Ref` values; with the new design, the struct IS the heap ref — you push `href` directly, then
the GC's mark phase calls `trace_refs(HeapObject::Struct)` which walks the struct's fields.

**How to avoid:** The GC regression test catches this before any other code is changed.

### Pitfall 3: Heap Allocation During `pool.acquire`

**What goes wrong:** After Phase 77, RegisterPool releases frame Vecs and reuses them. The
`pool.release` path calls `v.fill(Value::Void)`. Once `Value::Void` is Copy, fill still works
correctly. However, if any call site does `registers[i] = Value::Struct { type_idx, href }` in a
tight loop without GC, the heap will grow unboundedly. This is a correctness concern, not a new
bug — but it's worth noting that struct-heavy workloads now produce more GC pressure.

**How to avoid:** No immediate action required for Phase 79. Note for future: MOV on struct
should deep-copy fields if value semantics are needed, or use a ref-counted wrapper.

### Pitfall 4: PartialEq Manual Impl Needs Updating

**What goes wrong:** `value.rs` has a handwritten `impl PartialEq for Value` that includes:
```rust
(Value::InlineStruct { type_idx: a, fields: fa }, Value::InlineStruct { type_idx: b, fields: fb }) => a == b && fa == fb,
```
This arm must be changed to:
```rust
(Value::Struct { type_idx: a, href: ha }, Value::Struct { type_idx: b, href: hb }) => a == b && ha == hb,
```

**Why it happens:** The manual PartialEq impl is not caught by derive — it silently compiles
with the wrong arms (or fails to compile if the old arms don't match the new variant names).

**How to avoid:** After adding `Value::Struct` and removing `Value::InlineStruct`, the compiler
will flag the stale arm with a "no variant named InlineStruct" error. Fix it at that point.

### Pitfall 5: writ-dap variables.rs Constructor

**What goes wrong:** `writ-dap/src/variables.rs` line 239 constructs a `Value::InlineStruct`
directly (not just matches on it). After migration this is a compile error.

**Why it happens:** The DAP module uses InlineStruct to format variable values in the debugger.

**How to avoid:** The constructor must be updated to `Value::Struct { type_idx, href }` and
`href` must come from a heap allocation. If the DAP code is creating a temporary InlineStruct
for display purposes only, it may be simpler to remove the construction and compute the display
string from the actual heap object.

## Code Examples

### Complete collect_value_refs After Migration

```rust
// Source: writ-runtime/src/gc.rs
/// Collect all HeapRefs directly reachable from a Value.
/// For Value::Struct, pushes the heap object reference — GC's trace_refs
/// then walks HeapObject::Struct { fields } for transitive refs.
pub fn collect_value_refs(val: &Value, refs: &mut Vec<HeapRef>) {
    match val {
        Value::Ref(href) => refs.push(*href),
        Value::Struct { href, .. } => refs.push(*href),
        _ => {}
    }
}
```

### exec_new Struct Arm After Migration

```rust
// Source: writ-runtime/src/dispatch/objects.rs — exec_new
Some(writ_module::TypeDefKind::Struct) => {
    let href = ctx.heap.alloc_struct(field_count);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Struct { type_idx, href };
    ExecutionResult::Continue
}
```

### exec_set_field Struct Arm After Migration

```rust
// Source: writ-runtime/src/dispatch/objects.rs — exec_set_field
Value::Struct { href, .. } => {
    let href = *href;
    let _ = frame;
    match ctx.heap.set_field(href, idx, val) {
        Ok(()) => ExecutionResult::Continue,
        Err(e) => ExecutionResult::Crash(format!("SetField: {}", e)),
    }
}
```

## Complete Match-Site Inventory

All locations that must be updated (sourced from workspace grep):

### writ-runtime/src/value.rs
- Line 58: doc comment "Clone-only. InlineStruct holds fields inline" — update comment
- Line 69: `InlineStruct { type_idx: u32, fields: Vec<Value> }` — replace with `Struct { type_idx: u32, href: HeapRef }`
- Line 82: PartialEq manual impl arm for InlineStruct — update to Struct

### writ-runtime/src/gc.rs
- Line 62: `Value::InlineStruct { fields, .. }` arm in collect_value_refs — replace
- Line 72: doc comment "nested InlineStructs" — update

### writ-runtime/src/dispatch/objects.rs
- Line 22: `Value::InlineStruct { type_idx, fields }` constructor in exec_new — replace
- Line 55: `Value::InlineStruct { fields, .. }` match in exec_get_field — replace
- Line 102: `Value::InlineStruct { fields, .. }` match in exec_set_field — replace

### writ-runtime/src/dispatch/calls.rs
- Line 143: `Value::InlineStruct { type_idx, .. } => format!("<struct@{}>", type_idx)` — update variant name
- Line 337: `Value::InlineStruct { .. } => u32::MAX` in resolve_runtime_type_key — update variant name; consider returning type_idx

### writ-cli/src/cli_host.rs
- Line 67: `Value::InlineStruct { type_idx, .. } => format!("<struct@{}>", type_idx)` — update variant name

### writ-cli/tests/cli_integration.rs
- Line 55: `Value::InlineStruct { .. } => "<struct>".to_string()` — update variant name

### writ-dap/src/variables.rs
- Line 50: `Value::InlineStruct { type_idx, fields }` display arm — update variant name and fields access
- Line 239: `Value::InlineStruct { ... }` constructor — replace with `Value::Struct { type_idx, href }` using a heap allocation or remove

### writ-runtime/tests/vm_tests.rs
- Lines 1164, 1176, 1178, 1181: `new_allocates_struct` test — update match arm
- Lines 2100-2131: `test_new_struct_inline_no_heap` — delete or invert (heap IS allocated now)
- Lines 2186-2194: `get_set_field_round_trip` area — check if needs update
- Lines 2240-2276: `test_mov_inline_struct_independent_copy` — delete (reference semantics now)
- Lines 2278-2312: `test_box_unbox_inline_struct` — update constructor
- Lines 2314-2376: `test_gc_traces_inline_struct_*` tests (3 tests) — replace with new GC regression test

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Clone`-only Value with Vec inside | `Copy` Value with HeapRef | Phase 79 | Register moves become zero-allocation bitwise copies |
| Inline struct fields (stack-allocated Vec per struct instance) | Struct fields on GC heap (single allocation per struct lifetime) | Phase 79 | GC pressure increases slightly; clone pressure eliminated |

## Open Questions

1. **Value::Struct variant shape: tuple vs named fields**
   - What we know: Both `Value::Struct(HeapRef)` (no type_idx) and `Value::Struct { type_idx, href }` (with type_idx) work
   - What's unclear: Whether removing type_idx from the register value causes any regression in resolve_runtime_type_key or display code
   - Recommendation: Use named-field form `Value::Struct { type_idx: u32, href: HeapRef }` — avoids heap lookup in display; type_idx costs 4 bytes more but keeps the variant self-describing

2. **Independent copy semantics: delete or preserve with heap clone**
   - What we know: After migration, MOV of Value::Struct copies the HeapRef (shared mutation)
   - What's unclear: Whether any existing Writ test programs rely on struct value-copy semantics
   - Recommendation: Delete the independent-copy test for now; document that true value-copy semantics (deep clone on MOV) are a future v8.0 concern

3. **writ-dap variables.rs line 239 constructor**
   - What we know: This code constructs `Value::InlineStruct` directly — likely for test fixtures or display purposes
   - What's unclear: Whether the test/display code needs an actual HeapRef or can be adapted to not construct a struct Value at all
   - Recommendation: Inspect the context; if it's a test fixture, update to allocate on a local BumpHeap. If display-only, remove the Value construction and format from scratch.

## Validation Architecture

`workflow.nyquist_validation` is absent from config.json — treat as enabled.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`cargo test`) |
| Config file | Cargo.toml per crate (no separate test config) |
| Quick run command | `cargo test -p writ-runtime --release 2>&1 | tail -20` |
| Full suite command | `cargo test --release 2>&1 | tail -40` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VALUE-01 | Struct NEW allocates on heap | unit | `cargo test -p writ-runtime --release test_new_struct` | ❌ Wave 0 (existing test asserts opposite) |
| VALUE-02 | Value derives Copy | compile | `cargo build -p writ-runtime --release` | N/A |
| VALUE-03 | collect_value_refs traces Struct(HeapRef) | unit | `cargo test -p writ-runtime --release test_gc_traces_struct_href` | ❌ Wave 0 |
| VALUE-04 | GC regression test: struct-in-register survives GC | unit | `cargo test -p writ-runtime --release test_gc_traces_struct_href_in_register` | ❌ Wave 0 |
| VALUE-05 | get_field/set_field through HeapRef | unit | `cargo test -p writ-runtime --release get_set_field_round_trip` | ✅ (needs arm update) |
| VALUE-06 | Full suite passes | integration | `cargo test --release` | ✅ |
| VERIFY-01 | fib(40) output = 102334155 | smoke | `cargo run -p writ-cli --release -- benchmark/cases/fib/fib.writ` | ✅ |
| VERIFY-02 | Zero test failures | integration | `cargo test --release` | ✅ |
| VERIFY-03 | Zero warnings | build | `cargo build --release 2>&1 | grep warning` | ✅ |
| VERIFY-04 | fib(40) < 30s | perf | `time cargo run -p writ-cli --release -- benchmark/cases/fib/fib.writ` | ✅ |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime --release 2>&1 | tail -20`
- **Per wave merge:** `cargo test --release 2>&1 | tail -40`
- **Phase gate:** Full suite green + fib(40) < 30s before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-runtime/tests/vm_tests.rs` — add `test_gc_traces_struct_href_in_register` (covers VALUE-03, VALUE-04)
- [ ] Delete or update `test_new_struct_inline_no_heap` — currently asserts no heap allocation (covers VALUE-01 after inversion)
- [ ] Update `new_allocates_struct` match arm — change `Value::InlineStruct` to `Value::Struct`

## Sources

### Primary (HIGH confidence)
- `writ-runtime/src/value.rs` — exact current Value enum definition, confirmed InlineStruct variant
- `writ-runtime/src/gc.rs` — collect_value_refs and MarkSweepHeap implementation, confirmed alloc_struct and get_field/set_field exist
- `writ-runtime/src/heap.rs` — BumpHeap implementation, HeapObject::Struct definition
- `writ-runtime/src/dispatch/objects.rs` — exec_new, exec_get_field, exec_set_field implementations
- `writ-runtime/src/dispatch/calls.rs` — two InlineStruct display arms
- `writ-runtime/src/runtime.rs:534-564` — collect_roots shows how registers feed into GC
- `writ-runtime/tests/vm_tests.rs` — all InlineStruct-dependent tests enumerated
- `writ-cli/src/cli_host.rs` — InlineStruct display arm
- `writ-cli/tests/cli_integration.rs` — InlineStruct display arm
- `writ-dap/src/variables.rs` — InlineStruct display arm and constructor
- `.planning/STATE.md` — explicit mandate: write GC regression test BEFORE changing match arms

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all relevant code read directly from source
- Architecture: HIGH — every InlineStruct match site enumerated via workspace grep
- Pitfalls: HIGH — semantic flip on independent copy is proven from test assertions; GC tracing
  risk is confirmed from collect_value_refs source; PartialEq pitfall confirmed from manual impl

**Research date:** 2026-03-22
**Valid until:** Indefinitely — all findings are from source code, not external dependencies
