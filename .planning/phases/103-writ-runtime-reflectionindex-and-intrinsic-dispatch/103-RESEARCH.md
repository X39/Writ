# Phase 103: writ-runtime ReflectionIndex and Intrinsic Dispatch - Research

**Researched:** 2026-03-28
**Domain:** Rust runtime — GC-rooted cache, virtual dispatch, metadata traversal
**Confidence:** HIGH

## Summary

Phase 103 is a pure Rust runtime phase with no external dependencies. All research is based on direct inspection of the existing codebase — no external libraries or documentation are needed beyond what is already present.

The core task is: create a `ReflectionIndex` struct (a new module in `writ-runtime/src/`) that lazily allocates Type/FieldInfo/MethodInfo/ParameterInfo/AttributeInfo/ContractInfo heap objects on first access, registers them as permanent GC roots in `Runtime::collect_roots()`, wires the `TypeOf` opcode stub and 4 primitive `GetType` intrinsic stubs to return real heap objects, and implements all 28+ reflection `IntrinsicId` dispatch arms.

The key architectural facts are: (1) GC roots are just `Vec<HeapRef>` passed to `heap.collect()` — adding permanent roots means storing them somewhere and including them in `collect_roots()`; (2) heap object allocation uses `ctx.heap.alloc_struct(type_key, field_count)` + `ctx.heap.set_field()`; (3) the `ModuleAttributeView` / `Domain::query_attributes_on()` API for attribute data is already fully present; (4) `FieldDef.flags: u16` exists but no named constant for readonly is defined — the flag convention `0x01` is inferred from `add_field_def("length", ..., 0x01)` in `virtual_module.rs`.

**Primary recommendation:** Add `ReflectionIndex` as `writ-runtime/src/reflection.rs`, store it in `Runtime<H>`, and extend `collect_roots()` to drain its cached HeapRefs into the root slice. Wire TypeOf and intrinsic arms via `ctx.reflection` (passed into `ExecContext`).

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- ReflectionIndex lazy init: must NOT eagerly allocate Type objects for all types at domain load time — only on first access
- GC root strategy: reflection singleton HeapRefs (Type, FieldInfo, MethodInfo) must be registered as permanent GC roots in Runtime::collect_roots() so GC cannot free them
- AttributeInfo uses unified AttributeIndex shared with v10.0 ModuleAttributeView
- any-at-boundaries: compiler auto-inserts BOX/UNBOX coercions at reflection API parameter/return sites — no TyKind::Any needed
- TypeOf dispatch returns a Type heap object for any type index
- Primitive typeof via IntGetType/FloatGetType/BoolGetType/StringGetType intrinsics
- Phase 102 left intrinsic bodies as placeholder Value::Int(1) — this phase replaces with actual Type heap object allocation

### Claude's Discretion
All implementation choices are at Claude's discretion — infrastructure phase. All decisions are locked in STATE.md / CONTEXT.md.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RT-01 | ReflectionIndex with lazy FxHashMap caches for Type/FieldInfo/MethodInfo heap objects | ReflectionIndex struct pattern, FxHashMap available (rustc-hash 2.1.1), alloc_struct/set_field for construction |
| RT-02 | GC root registration for reflection singleton objects (permanent roots, not freed) | collect_roots() in runtime.rs returns Vec<HeapRef>; adding permanent roots = store in ReflectionIndex and collect there |
| RT-03 | TypeOf opcode dispatch in VM main loop | Stub is at dispatch/mod.rs line 505; needs ReflectionIndex access via ExecContext |
| RT-04 | All reflection IntrinsicId arms dispatched correctly (28+ new variants) | IntrinsicId enum at dispatch/mod.rs line 51; dispatch/intrinsics.rs contains the exhaustive match; stubs for 4 at lines 393-400 |
| RT-05 | Unified AttributeIndex shared with v10.0 ModuleAttributeView | Domain::query_attributes_on(module_idx, typedef_idx) in domain.rs line 421 is the existing API; ReflectionIndex AttributeInfo population uses this directly |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rustc-hash | 2.1.1 | FxHashMap for lazy caches | Already in Cargo.toml; all other caches in codebase use FxHashMap |

No new dependencies required. All tools are already in scope.

**Installation:** None needed.

---

## Architecture Patterns

### Pattern 1: Heap Object Allocation for Reflection Types
Reflection type heap objects (Type, FieldInfo, etc.) are `HeapObject::Struct` with `type_key = (0u32 << 16) | typedef_idx_0based`.

Virtual module is always at `domain.modules[0]`. The TypeDef indices for reflection types are:
- Type = 9 (0-based)
- ParameterInfo = 10
- AttributeInfo = 11
- ContractInfo = 12
- FieldInfo = 13
- MethodInfo = 14

So `type_key` for a `Type` heap object = `(0u32 << 16) | 9u32 = 9`.

**Field counts** per TypeDef (from virtual_module.rs):
- Type: 4 fields — name(string), namespace(string), kind(string), is_generic(bool)
- ParameterInfo: 2 fields — name(string), parameter_type(Type ref)
- AttributeInfo: 2 fields — name(string), args(Array ref)
- ContractInfo: 2 fields — name(string), type(Type ref)
- FieldInfo: 3 fields — name(string), declared_type(Type ref), is_mutable(bool)
- MethodInfo: 3 fields — name(string), return_type(Type ref), parameters(Array ref)

**Allocation pattern:**
```rust
// Source: writ-runtime/src/dispatch/objects.rs exec_new pattern
let href = ctx.heap.alloc_struct(type_key, field_count);
ctx.heap.set_field(href, 0, Value::Ref(name_href)).unwrap();
ctx.heap.set_field(href, 1, Value::Bool(true)).unwrap();
// ...
```

### Pattern 2: Lazy FxHashMap Cache in ReflectionIndex
```rust
// writ-runtime/src/reflection.rs (new file)
use rustc_hash::FxHashMap;
use crate::value::HeapRef;

pub struct ReflectionIndex {
    /// Keyed by (module_idx, typedef_0based_idx) — returns Type HeapRef
    type_cache: FxHashMap<(usize, usize), HeapRef>,
    /// Keyed by (module_idx, field_0based_idx) — returns FieldInfo HeapRef
    field_cache: FxHashMap<(usize, usize), HeapRef>,
    /// Keyed by (module_idx, method_0based_idx) — returns MethodInfo HeapRef
    method_cache: FxHashMap<(usize, usize), HeapRef>,
    // param_cache and contract_info_cache similarly
}

impl ReflectionIndex {
    pub fn new() -> Self { ... }
    /// Returns cached Type HeapRef or allocates one.
    pub fn get_or_alloc_type(
        &mut self,
        module_idx: usize,
        typedef_idx: usize,
        heap: &mut dyn GcHeap,
        modules: &[LoadedModule],
    ) -> HeapRef { ... }
    /// Collect all cached HeapRefs as GC roots.
    pub fn collect_roots(&self, out: &mut Vec<HeapRef>) { ... }
}
```

### Pattern 3: GC Root Registration (permanent roots)
`Runtime::collect_roots()` in `runtime.rs` line 597 returns `Vec<HeapRef>`. Pattern for adding permanent roots:

```rust
// runtime.rs — Runtime struct gains a field:
pub(crate) reflection: ReflectionIndex,  // added to Runtime<H>

// collect_roots() extension:
fn collect_roots(&self) -> Vec<HeapRef> {
    // ... existing task/global/entity root collection ...
    self.reflection.collect_roots(&mut roots);
    roots
}
```

Since `Runtime<H>` is built in `RuntimeBuilder::build()`, `reflection: ReflectionIndex::new()` is added to the `Runtime { ... }` struct literal there.

### Pattern 4: TypeOf Opcode — Wiring ReflectionIndex
Current stub in `dispatch/mod.rs` line 505:
```rust
Instruction::TypeOf { r_dst, type_idx: _ } => {
    // Phase 102 stub
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[*r_dst as usize] = Value::Int(1);
    ExecutionResult::Continue
}
```

`ExecContext` does not currently hold a `ReflectionIndex` reference. Two approaches:
- **Approach A (preferred):** Pass `&mut ReflectionIndex` through `ExecContext` (add field `reflection: &'a mut ReflectionIndex`). This mirrors how `heap`, `host`, `entity_registry`, and `pool` are passed.
- **Approach B:** Thread `reflection` as an additional parameter only to the reflection arms.

Approach A is cleaner and consistent with the codebase pattern. `ExecContext` gains one field; `execute_one()` and `execute_batch()` gain one parameter. The call chain (`scheduler::run_one_task` → `execute_batch`) must also be updated.

### Pattern 5: IntrinsicId Dispatch Arms (28+ new variants)
New `IntrinsicId` variants needed (in `dispatch/mod.rs`):
```
// Type methods (6):
TypeFields, TypeMethods, TypeAttributes, TypeContracts, TypeImplements, TypeName,
// FieldInfo methods (2):
FieldInfoGet, FieldInfoName,
// MethodInfo methods (2):
MethodInfoName, MethodInfoParameters,
// AttributeInfo methods (1):
AttributeInfoName,
// ContractInfo methods (1):
ContractInfoName,
// Primitive get_type (already have 4):
IntGetType, FloatGetType, BoolGetType, StringGetType  (replace stub with real impl)
```

Exact count depends on how many Type accessors (name, namespace, kind, is_generic, fields, methods, attributes, contracts, implements) are wired as methods vs field reads. The method-per-accessor pattern is standard for the virtual module.

### Pattern 6: AttributeInfo Population via Domain::query_attributes_on
`Domain::query_attributes_on(module_idx, typedef_idx)` returns `Vec<DomainAttributeMatch>` (domain.rs line 421). Each `DomainAttributeMatch` has `.name: String`, `.args: Vec<AttrValue>`, `.owner`, `.owner_kind`.

For each attribute match, construct an `AttributeInfo` heap object:
```rust
let attr_href = ctx.heap.alloc_struct(attr_info_type_key, 2);
let name_href = ctx.heap.alloc_string(&attr.name);
ctx.heap.set_field(attr_href, 0, Value::Ref(name_href)).unwrap();
// args field: alloc an Array<Boxed> containing boxed AttrValues
ctx.heap.set_field(attr_href, 1, Value::Ref(args_arr_href)).unwrap();
```

This satisfies RT-05 because `Domain::query_attributes_on` is the v10.0 attribute query path — no duplicate scan.

### Pattern 7: FieldDef readonly bit
`FieldDefRow.flags: u16` exists (writ-module/src/tables.rs line 153). No named constant is defined in the codebase. The convention is inferred from `virtual_module.rs` line 343:
```rust
builder.add_field_def("length", &[0x01], 0x01);  // int type, read-only flag
```
This means `flags & 0x01 == 1` = readonly (let-field). The `FieldInfo.is_mutable` boolean is computed as `(field_def.flags & 0x01) == 0`.

**Note:** This is the only evidence of the readonly flag convention. Confidence: MEDIUM (single source). A named constant `FIELD_FLAG_READONLY: u16 = 0x01` should be defined in `writ-module/src/tables.rs` as part of this phase.

### Recommended Project Structure (new file)
```
writ-runtime/src/
├── reflection.rs        # NEW: ReflectionIndex, lazy caches, root collection
├── dispatch/
│   ├── mod.rs           # EDIT: TypeOf arm, new IntrinsicId variants, ExecContext field
│   └── intrinsics.rs    # EDIT: replace 4 stubs, add 20+ new arms
├── runtime.rs           # EDIT: add reflection field, extend collect_roots()
└── lib.rs               # EDIT: pub mod reflection; pub use reflection::ReflectionIndex
```

### Anti-Patterns to Avoid
- **Eager allocation at domain load time:** Must not pre-populate `ReflectionIndex` in `RuntimeBuilder::build()`. Violates the lazy-init decision and inflates startup cost.
- **Storing HeapRefs outside the reflection index:** Any cached reflection HeapRef not collected in `collect_roots()` will be freed by GC. The cache IS the root list.
- **Separate attribute scan in ReflectionIndex:** Do not re-iterate `attribute_defs` directly. Use `domain.query_attributes_on()` — it is the unified path (RT-05).
- **New GcHeap variant for reflection objects:** Reflection objects are plain `HeapObject::Struct`. No new variant needed.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Attribute data for AttributeInfo | Custom attribute_defs scan | `Domain::query_attributes_on()` | Already implemented, correct filtering |
| Heap allocation for struct objects | Custom allocator | `GcHeap::alloc_struct()` + `set_field()` | The only allocation path; everything else is a BumpHeap/MarkSweepHeap detail |
| String allocation for names | Direct heap manipulation | `GcHeap::alloc_string()` | Same as all other string alloc in codebase |
| FxHashMap | std HashMap | `rustc_hash::FxHashMap` | Already a dep; all other caches use it |
| type_key encoding | Custom key | `(module_idx as u32) << 16 | typedef_idx as u32` | The established convention in the entire codebase (domain.rs, dispatch) |

---

## Common Pitfalls

### Pitfall 1: GC Frees Cached Reflection Objects
**What goes wrong:** ReflectionIndex caches HeapRef values but they are not listed in `collect_roots()`. After a GC cycle, the HeapRefs point to freed slots. Next access to a cached Type object crashes with "heap object at N has been freed".

**Why it happens:** `MarkSweepHeap::collect()` frees any object not reachable from the root list. The root list is built by `collect_roots()`. If `reflection.collect_roots()` is not called there, all cached HeapRefs become dangling.

**How to avoid:** Every HeapRef stored in a `ReflectionIndex` cache must be included in the `Vec<HeapRef>` returned by `collect_roots()`. The `ReflectionIndex::collect_roots(&self, out: &mut Vec<HeapRef>)` method iterates all values in all caches.

**Warning signs:** Tests with `.with_gc()` pass on first call but fail after `rt.collect_garbage()` is called. Test `success_criteria_2` (GC cycle does not free cached Type objects) explicitly verifies this.

### Pitfall 2: ExecContext Borrow Conflicts with ReflectionIndex
**What goes wrong:** `ExecContext<'a>` holds `heap: &'a mut dyn GcHeap`. If `ReflectionIndex` is stored on `Runtime<H>` and `ExecContext` tries to borrow it alongside `heap`, Rust's borrow checker may complain about multiple mutable borrows of `Runtime`.

**Why it happens:** `execute_one()` takes `heap: &mut dyn GcHeap` and other fields as independent mutable parameters. These are already separate fields of `Runtime`, which is why the borrow checker accepts them. Adding `reflection: &'a mut ReflectionIndex` follows the same pattern — it must be passed as a separate parameter extracted from `Runtime` before creating `ExecContext`.

**How to avoid:** In `scheduler::run_one_task`, extract `runtime.reflection` (or `&mut self.reflection`) as a local mutable reference before creating `ExecContext`, just as `self.heap.as_mut()` and `&mut self.scheduler.entity_registry` are extracted.

**Warning signs:** Compile error mentioning "cannot borrow `*self` as mutable more than once at a time".

### Pitfall 3: type_key Encoding Mismatch
**What goes wrong:** A Type heap object for a user TypeDef in module 1 is allocated with `type_key = typedef_idx` (forgetting the module_idx shift). Virtual dispatch lookups using this type_key fail.

**Why it happens:** The `type_key` encoding is `(module_idx << 16) | typedef_idx`. This is documented in `heap.rs` line 11 and the dispatch table comment in `dispatch/mod.rs` line 24. It is not enforced by the type system.

**How to avoid:** Always compute `let type_key = (module_idx as u32) << 16 | typedef_idx as u32`. Virtual module is always `module_idx = 0`, so reflection TypeDef objects (Type=9, FieldInfo=13, etc.) use `type_key = 9`, `13`, etc. User TypeDefs in module 1 use `(1 << 16) | typedef_idx`.

### Pitfall 4: TypeDef Field Count Requires Range Arithmetic
**What goes wrong:** `alloc_struct(type_key, field_count)` is called with a hardcoded `field_count` that doesn't match the actual number of fields in the TypeDef. Field reads crash with "field index out of range".

**Why it happens:** Virtual module TypeDefs are defined in `virtual_module.rs` with specific field counts (Type=4, FieldInfo=3, etc.) but this is not queried from the loaded module at runtime.

**How to avoid:** Either hardcode the field counts as named constants in `reflection.rs` (matching virtual_module.rs), or compute them from `td.field_list` range arithmetic (same pattern as `Domain::find_field_in_type`). Hardcoding is simpler for the 6 reflection TypeDefs since they never change.

### Pitfall 5: ReflectionIndex Not Added to ExecContext for Intrinsics
**What goes wrong:** `execute_intrinsic()` in `intrinsics.rs` is called from `dispatch/calls.rs` via `execute_intrinsic(ctx, id, ...)`. If `ctx` doesn't have a `reflection` field, the new reflection intrinsic arms can't call `ctx.reflection.get_or_alloc_type(...)`.

**How to avoid:** Add `reflection: &'a mut ReflectionIndex` to `ExecContext<'a>` before writing any intrinsic arms. The new field must be threaded through `execute_one()`, `execute_batch()`, and all callers back to `scheduler::run_one_task`.

---

## Code Examples

### Allocating a Type heap object
```rust
// Source: writ-runtime/src/dispatch/objects.rs exec_new pattern + reflection fields from virtual_module.rs
fn alloc_type_object(
    heap: &mut dyn GcHeap,
    modules: &[LoadedModule],
    module_idx: usize,
    typedef_idx: usize,
) -> HeapRef {
    // type_key for "Type" TypeDef (index 9) in virtual module (index 0)
    let type_type_key: u32 = 9; // (0 << 16) | 9
    let href = heap.alloc_struct(type_type_key, 4); // 4 fields: name, namespace, kind, is_generic

    let module = &modules[module_idx].module;
    let td = &module.type_defs[typedef_idx];

    let name_str = writ_module::heap::read_string(&module.string_heap, td.name).unwrap_or("");
    let name_href = heap.alloc_string(name_str);
    heap.set_field(href, 0, Value::Ref(name_href)).unwrap();

    let ns_str = writ_module::heap::read_string(&module.string_heap, td.namespace).unwrap_or("");
    let ns_href = heap.alloc_string(ns_str);
    heap.set_field(href, 1, Value::Ref(ns_href)).unwrap();

    let kind_str = match td.kind {
        0 => "struct", 1 => "enum", 2 => "class", 3 => "entity", _ => "unknown",
    };
    let kind_href = heap.alloc_string(kind_str);
    heap.set_field(href, 2, Value::Ref(kind_href)).unwrap();

    let has_generic = !module.generic_params.is_empty(); // simplified
    heap.set_field(href, 3, Value::Bool(has_generic)).unwrap();

    href
}
```

### Extending collect_roots() for permanent reflection roots
```rust
// Source: writ-runtime/src/runtime.rs collect_roots() lines 597-627
fn collect_roots(&self) -> Vec<HeapRef> {
    use crate::gc::collect_value_refs;
    let mut roots = Vec::new();

    // ... existing task/global/entity roots ...

    // Permanent reflection singleton roots (RT-02)
    self.reflection.collect_roots(&mut roots);

    roots
}
```

### ReflectionIndex collect_roots implementation
```rust
// Source: pattern from codebase (gc.rs collect_value_refs)
impl ReflectionIndex {
    pub fn collect_roots(&self, out: &mut Vec<HeapRef>) {
        out.extend(self.type_cache.values().copied());
        out.extend(self.field_cache.values().copied());
        out.extend(self.method_cache.values().copied());
        out.extend(self.param_cache.values().copied());
        out.extend(self.attr_cache.values().copied());
        out.extend(self.contract_info_cache.values().copied());
    }
}
```

### FieldDef readonly bit check
```rust
// Source: virtual_module.rs line 343 — flags=0x01 convention
// flags & 0x01 == 1 means readonly (let field)
const FIELD_FLAG_READONLY: u16 = 0x01;

let is_mutable = (field_def.flags & FIELD_FLAG_READONLY) == 0;
heap.set_field(field_info_href, 2, Value::Bool(is_mutable)).unwrap();
```

### TypeOf opcode dispatch (wired)
```rust
// Source: writ-runtime/src/dispatch/mod.rs line 505 (current stub)
Instruction::TypeOf { r_dst, type_idx } => {
    // type_idx is a TypeDef token: table_id byte + 1-based row index
    // Decode to 0-based typedef index:
    let typedef_0based = ((*type_idx & 0x00FF_FFFF) as usize).saturating_sub(1);
    // For a cross-module token, module_idx comes from type resolution;
    // for a same-module token, use current_module_idx
    let href = ctx.reflection.get_or_alloc_type(
        ctx.current_module_idx, typedef_0based, ctx.heap, ctx.modules
    );
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[*r_dst as usize] = Value::Ref(href);
    ExecutionResult::Continue
}
```

---

## Runtime State Inventory

This is a greenfield infrastructure phase — no runtime state inventory applies. No stored data, live service config, OS-registered state, secrets, or build artifacts contain reflection-related strings that need renaming.

---

## Environment Availability

Step 2.6: SKIPPED — this is a pure Rust code/config change with no external tool dependencies. The only requirement is `cargo test` within the existing workspace.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Compilation | ✓ | existing workspace | — |
| rustc-hash | FxHashMap in ReflectionIndex | ✓ | 2.1.1 | — |

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (cargo test) |
| Config file | Cargo.toml (workspace) |
| Quick run command | `cargo test -p writ-runtime reflection` |
| Full suite command | `cargo test -p writ-runtime` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RT-01 | ReflectionIndex lazy cache: Type heap object allocated on first access | integration | `cargo test -p writ-runtime reflection::tests::type_object_lazy_alloc` | ❌ Wave 0 |
| RT-01 | FieldInfo objects contain correct name and declared_type | integration | `cargo test -p writ-runtime tests::reflection_field_info_populated` | ❌ Wave 0 |
| RT-02 | Cached Type heap objects survive GC cycle with no script-side roots | integration | `cargo test -p writ-runtime tests::reflection_roots_survive_gc` | ❌ Wave 0 |
| RT-03 | TypeOf opcode returns a non-null Value::Ref (Type heap object) | integration | `cargo test -p writ-runtime tests::typeof_opcode_returns_type_ref` | ❌ Wave 0 |
| RT-04 | All IntrinsicId arms compile (exhaustive match — compile-time) | unit | `cargo build -p writ-runtime` | ❌ Wave 0 |
| RT-04 | IntGetType/FloatGetType/BoolGetType/StringGetType return Value::Ref | unit | `cargo test -p writ-runtime tests::primitive_get_type_returns_ref` | ❌ Wave 0 |
| RT-05 | AttributeInfo objects populated from Domain::query_attributes_on | integration | `cargo test -p writ-runtime tests::attribute_info_populated` | ❌ Wave 0 |

Tests should be added to `writ-runtime/tests/reflection_tests.rs` (new integration test file) following the pattern in `writ-runtime/tests/gc_tests.rs`.

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime reflection`
- **Per wave merge:** `cargo test -p writ-runtime`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-runtime/tests/reflection_tests.rs` — covers RT-01 through RT-05
- [ ] `writ-runtime/src/reflection.rs` — ReflectionIndex module itself (the implementation)

*(Existing test infrastructure in `gc_tests.rs`, `vm_tests.rs` covers the GC and dispatch patterns needed as reference)*

---

## Open Questions

1. **type_idx encoding in TypeOf opcode**
   - What we know: `TypeOf { r_dst: u16, type_idx: u32 }` — the type_idx is a metadata token (table_id 0x02 for TypeDef, row_index 1-based), consistent with the RI32 shape documented in STATE.md
   - What's unclear: Whether type_idx can reference a TypeRef (cross-module) or only a local TypeDef token. The Phase 101 decision states TypeOf emits "compile-time type index baked into instruction" — this strongly implies local TypeDef tokens only, not cross-module TypeRef tokens
   - Recommendation: Implement for local TypeDef tokens first. Use `decode_method_token()` pattern (strip table_id, subtract 1) to get 0-based typedef_idx. For cross-module types, the compiler would have resolved the TypeRef to a local typedef index at compile time

2. **ExecContext threading through the call chain**
   - What we know: `execute_one()` takes 10 parameters; adding `reflection: &mut ReflectionIndex` makes 11. `execute_batch()` and `run_one_task()` in scheduler.rs must also be updated
   - What's unclear: Whether the planner should treat this as a single wave (all files touched together) or split into a "wire ExecContext" wave and an "implement intrinsics" wave
   - Recommendation: Wire ExecContext in Wave 1, implement intrinsics in Wave 2 — keeps diff reviewable

3. **MethodInfo parameter population complexity**
   - What we know: MethodInfo has a `parameters: Array<ParameterInfo>` field. ParamDef rows exist in the module for parameters
   - What's unclear: Whether `param_def` rows are present for all method types in the virtual module, or only for user-defined methods
   - Recommendation: For RT-04, stub the `parameters` field as an empty Array for now; full param population is required for REFL-04 (Phase 106)

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Value::Int(1) sentinel for TypeOf | Actual Type HeapRef via ReflectionIndex | This phase (103) | typeof(x) returns a usable heap object |
| No reflection IntrinsicId variants | 28+ new IntrinsicId arms | This phase (103) | Type.fields(), Type.methods(), etc. callable |

---

## Sources

### Primary (HIGH confidence)
- `writ-runtime/src/gc.rs` — MarkSweepHeap::collect(), collect_roots pattern, GC root semantics
- `writ-runtime/src/runtime.rs` — Runtime struct, collect_roots() implementation (lines 597-627), GC integration
- `writ-runtime/src/dispatch/mod.rs` — IntrinsicId enum (line 51), ExecContext struct (line 155), TypeOf stub (line 505)
- `writ-runtime/src/dispatch/intrinsics.rs` — execute_intrinsic() match pattern, reflection stubs (lines 393-400)
- `writ-runtime/src/virtual_module.rs` — Reflection TypeDef indices (9-14), field counts, FieldDef flags convention (line 343)
- `writ-runtime/src/host.rs` — ModuleAttributeView, query_attributes(), query_attributes_on()
- `writ-runtime/src/domain.rs` — DomainAttributeMatch, Domain::query_attributes_on() (line 421)
- `writ-module/src/tables.rs` — FieldDefRow.flags: u16 (line 153), ATTR_OWNER_KIND_DECL constant

### Secondary (MEDIUM confidence)
- `writ-runtime/tests/gc_tests.rs` — Integration test structure and patterns for reflection tests
- STATE.md decisions section — TypeOf opcode encoding, ReflectionIndex constraints

### Tertiary (LOW confidence)
- None — all claims are verified against source code

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — direct code inspection; no new dependencies
- Architecture: HIGH — all patterns verified against existing codebase implementation
- Pitfalls: HIGH — pitfalls derived from direct analysis of GC implementation and borrow patterns
- FieldDef readonly flag: MEDIUM — single source (virtual_module.rs line 343 comment)

**Research date:** 2026-03-28
**Valid until:** No expiry — pure code research, not dependent on external sources
