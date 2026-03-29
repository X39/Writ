# Phase 108: Generic Reflection - Research

**Researched:** 2026-03-28
**Domain:** Runtime reflection — generic type queries, per-member attribute access
**Confidence:** HIGH

## Summary

Phase 108 implements the final four requirements of the v11.0 Runtime Reflection milestone:
GEN-01 (`Type.is_generic`), GEN-02 (`Type.type_args()`), GEN-03
(`MethodInfo.attributes()` and `FieldInfo.attributes()`), and GEN-04 (spec documentation
of the generic reflection limitation for runtime-queried types).

The spec text for all four is already written (§1.28.7, §1.28.8). The virtual module has
`TypeGetIsGeneric` registered as an IntrinsicId and the corresponding contract definition,
impl, and dispatch entry. What is missing is (1) the `is_generic` field being populated
with real data instead of `false`, (2) three new intrinsics for `type_args()`,
`MethodInfo.attributes()`, and `FieldInfo.attributes()`, and (3) the attr_cache key
scheme must be extended to cover method-scoped and field-scoped attribute lookups.

**Primary recommendation:** All four GEN requirements are additive changes to
`reflection.rs`, `intrinsics.rs`, `virtual_module.rs`, and `domain_dispatch.rs`. No
existing intrinsic arms, tests, or module-format rows need to be changed.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- type_args() for runtime-queried generics (via get_type()) returns partial or empty info
- type_args() for statically-known instantiations (via typeof()) returns correct Type array
- is_generic returns true for generic type instantiations, false for non-generic types
- Spec must document the limitation explicitly

### Claude's Discretion
All implementation choices are at Claude's discretion — infrastructure phase.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GEN-01 | Type.is_generic returns bool indicating whether the type has generic parameters | Field index 3 of the Type heap object already exists; `get_or_alloc_type` sets it to `false` unconditionally. Fix: scan `GenericParam` table for TypeDef owners; mark true when found. |
| GEN-02 | Type.type_args() returns Array of Type for bound generic arguments (statically-known instantiations) | TYPEOF opcode carries only the typedef token (no TypeSpec). For typeof(Array<int>), the compiler must encode a TypeSpec token. New `TypeTypeArgs` intrinsic reads type_args from the Type heap object (a lazily-allocated Array field that does not yet exist). |
| GEN-03 | Per-member attribute access — MethodInfo.attributes() and FieldInfo.attributes() return AttributeInfo arrays | New intrinsics `MethodInfoAttributes` and `FieldInfoAttributes`. Both scan `module.attribute_defs` filtering by `owner.table_id() == MethodDef` or `FieldDef`, using `method_reverse` / `field_reverse` maps to recover identity. The existing attr_cache key `(module_idx, typedef_idx, ordinal)` is type-scoped and must be extended or supplemented with method-scoped/field-scoped keys. |
| GEN-04 | Generic reflection limitations documented in spec for runtime-queried types | Already written in §1.28.7 and §1.28.8. Verify no gaps. |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| writ-runtime (intrinsics.rs) | local | New intrinsic arms | All reflection dispatch lives here |
| writ-runtime (virtual_module.rs) | local | New contract defs + impls | Pattern established in Phase 103/107 |
| writ-runtime (reflection.rs) | local | is_generic population, type_args array | ReflectionIndex is the central allocation point |
| writ-runtime (domain_dispatch.rs) | local | resolve_intrinsic_id mapping | New method name → IntrinsicId entries |
| language-spec/spec/28_1_28_reflection.md | local | Spec already written for §1.28.7 | Verify completeness only |

### No external dependencies.

## Architecture Patterns

### Pattern 1: Adding a New Reflection Intrinsic (established in Phases 103/107)

Every new intrinsic follows the identical 4-file pattern:

1. **`virtual_module.rs`** — `add_contract_def` + `add_contract_method`, then `add_impl_def` + `add_intrinsic_method`. Must appear in the correct ordered section (new contracts before TypeDef block, impls after TypeDef block).
2. **`domain_dispatch.rs`** — `resolve_intrinsic_id` match arm: `("TypeName", "method_name") => Some(IntrinsicId::Variant)`.
3. **`dispatch/mod.rs`** — New `IntrinsicId` variant in the `IntrinsicId` enum.
4. **`dispatch/intrinsics.rs`** — `match id { IntrinsicId::NewVariant => { ... } }` arm.

The virtual module test `has_exactly_48_contract_defs` must be updated for each new contract.

### Pattern 2: TypeOf Token Carries TypeDef Index Only

```rust
// From dispatch/mod.rs line 529:
Instruction::TypeOf { r_dst, type_idx } => {
    let typedef_0based = ((*type_idx & 0x00FF_FFFF) as usize).saturating_sub(1);
    let href = ctx.reflection.get_or_alloc_type(
        ctx.current_module_idx, typedef_0based, ctx.heap, ctx.modules,
    );
    ...
}
```

`get_or_alloc_type` currently allocates the Type object with `is_generic = false` at field
index 3. For GEN-01, the fix is inside `get_or_alloc_type`: scan `module.generic_params`
looking for rows where `row.owner.table_id() == TableId::TypeDef.as_u8()` and
`row.owner.row_index() == Some((typedef_idx + 1) as u32)`. Set `is_generic = true` if any
such row exists.

### Pattern 3: GenericParam Table for is_generic Detection

```
GenericParamRow { owner: MetadataToken, owner_kind: u8, ordinal: u16, name: u32 }
```

`owner_kind = 0` means TypeDef owner (per `writ-compiler/src/emit/metadata.rs` line 307,
confirmed by the virtual_module pattern at line 69). To check if a TypeDef is generic:

```rust
let target_row = (typedef_idx + 1) as u32;
let is_generic = module.generic_params.iter().any(|p|
    p.owner.table_id() == TableId::TypeDef.as_u8()
    && p.owner.row_index() == Some(target_row)
);
```

### Pattern 4: type_args() — TypeSpec Blob Approach

The TYPEOF instruction bakes in a metadata token. For generic instantiations like
`typeof(Array<int>)`, the token should be a **TypeSpec token** (table 4) whose
`signature` blob encodes the instantiation (e.g., `[0x20, 0x01, 0x00, 0x00, 0x00]` for
`Array<int>` — tag 0x20=Array + TypeDef token for Int).

The TypeOf dispatch arm currently assumes a TypeDef token only. For GEN-02 it must
also handle TypeSpec tokens. The type_args array is stored as an **extra field** (field 4)
on the Type heap object, allocated lazily when a TypeSpec-backed Type is constructed.

**Key design decision (from CONTEXT.md locks):**
- `typeof(Array<int>)` → TypeSpec token → type_args() returns `[Type(Int)]`, is_generic = true
- `obj.get_type()` on a polymorphic generic → type_args() returns empty Array

The `get_or_alloc_type` signature needs a new variant (or overload) to handle TypeSpec
tokens. The simplest approach: a separate `get_or_alloc_typespec_type(module_idx,
typespec_idx)` that allocates a Type with the type_args array populated.

**IMPORTANT:** The Type TypeDef currently has 4 fields (`name`, `namespace`, `kind`,
`is_generic`). Adding `type_args` as field index 4 requires updating
`ReflectionIndex::TYPE_FIELD_COUNT` from 4 to 5, and `get_or_alloc_type` must set field
4 to an empty Array (non-generic types). `get_or_alloc_typespec_type` sets field 4 to the
populated array. All existing tests that assert exactly 4 heap objects (Type + 3 strings)
for a non-generic Type must be updated to account for the empty Array.

### Pattern 5: Attribute Cache Key for Method/Field-Scoped Attributes

The current `attr_cache` key is `(module_idx, typedef_idx, attr_ordinal)` — type-scoped
only. For `MethodInfo.attributes()` and `FieldInfo.attributes()`, the owner token is a
MethodDef (table 7) or FieldDef (table 5). The cache key must distinguish these cases.

Options:
- **Option A (recommended):** Add separate `method_attr_cache: FxHashMap<(usize, usize, usize), HeapRef>` and `field_attr_cache: FxHashMap<(usize, usize, usize), HeapRef>` fields — one per owner kind. Clean separation, no risk of key collision.
- **Option B:** Change `attr_cache` key to `(usize, u8, usize, usize)` (module_idx, owner_kind, owner_idx, ordinal). Backward compatible but changes existing signature.

Option A is preferable: no existing code changes to `attr_cache`, `get_or_alloc_attribute_info`, or `TypeAttributes` intrinsic.

The `collect_roots` method must include the new cache values.

### Pattern 6: MethodInfo.attributes() Implementation

```rust
IntrinsicId::MethodInfoAttributes => {
    let mi_href = helpers::extract_ref(&ctx.task.call_stack.last()...);
    let (method_module_idx, method_idx) =
        match ctx.reflection.lookup_method_identity(mi_href) { ... };

    // AttributeDef owner_kind = 1 means MethodDef
    let target_row = (method_idx + 1) as u32;
    let mut attr_data = Vec::new();
    for row in &ctx.modules[method_module_idx].module.attribute_defs {
        if row.owner_kind == ATTR_OWNER_KIND_DECL { continue; }
        if row.owner.table_id() != TableId::MethodDef.as_u8() { continue; }
        if row.owner.row_index() != Some(target_row) { continue; }
        // collect name + args
    }
    // allocate AttributeInfo objects via ctx.reflection.get_or_alloc_method_attribute_info(...)
    // build Array, store in r_dst
}
```

### Pattern 7: FieldInfo.attributes() Implementation

Same pattern as MethodInfo.attributes() but uses `field_reverse` to recover the
**absolute field index** (not the typedef-scoped field_offset). The owner token in
AttributeDef uses the 1-based absolute field index.

```rust
let (module_idx, typedef_idx, field_offset) =
    ctx.reflection.lookup_field_identity(fi_href);
let (field_start, _) = ReflectionIndex::typedef_field_range_pub(
    ctx.modules, module_idx, typedef_idx);
let abs_field_idx = field_start + field_offset;
let target_row = (abs_field_idx + 1) as u32;
// Filter AttributeDef rows: owner.table_id() == TableId::FieldDef.as_u8()
//                          && owner.row_index() == Some(target_row)
```

### Anti-Patterns to Avoid

- **Setting is_generic to false unconditionally:** The existing code in `get_or_alloc_type` at `reflection.rs:122` does this — must be replaced with the GenericParam scan.
- **Changing TYPE_FIELD_COUNT without updating existing tests:** Three tests assert exactly 4 heap objects for Type allocation; adding field 4 (empty Array) changes the count.
- **Using TypeDef token for TypeSpec typeof:** The TypeOf dispatch arm strips `bits 23-0` assuming a TypeDef token (table 2). TypeSpec tokens use table 4. The dispatch must branch on `type_idx >> 24` to distinguish.
- **Forgetting to update has_exactly_48_contract_defs test:** Phase 108 adds 3 new contracts (TypeTypeArgs, MethodInfoAttributes, FieldInfoAttributes). Update the assertion from 48 to 51.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| AttributeDef scan logic | Custom per-intrinsic scanner | Replicate the existing TypeAttributes pattern exactly | TypeAttributes intrinsic at intrinsics.rs:472 is the proven template |
| GenericParam lookup | Ad-hoc binary search | Linear scan of `module.generic_params` | Table is small; O(n) is fine, matches all other table scans |
| TypeSpec signature parsing | Full blob decoder | Minimal: extract the first token from the signature blob (type tag + TypeDef token bytes) | Only need to extract type_arg TypeDef indices for type_args() |

## Common Pitfalls

### Pitfall 1: attr_cache Key Collision (type vs method vs field)
**What goes wrong:** If `MethodInfo.attributes()` reuses `get_or_alloc_attribute_info` with `(module_idx, typedef_idx, ordinal)`, the cache key may collide with a TypeDef attribute at the same ordinal.
**Why it happens:** The cache was designed for type-scoped attributes only.
**How to avoid:** Add `method_attr_cache` and `field_attr_cache` as separate FxHashMap fields in ReflectionIndex.
**Warning signs:** AttributeInfo.name returns the wrong attribute name; test with a type that has both type-level and method-level attributes with the same ordinal.

### Pitfall 2: is_generic Stale on Cached Type Objects
**What goes wrong:** `get_or_alloc_type` caches Type objects on first access. If is_generic was set to `false` on a previous call (before the GenericParam scan was added), the cache returns the stale object.
**Why it happens:** ReflectionIndex cache is permanent — objects are never reallocated.
**How to avoid:** Fix `get_or_alloc_type` at the source. Since Phase 108 is the first phase where is_generic is correct, no migration is needed for existing cached objects (only tests create Types in tests). The fix must be in `get_or_alloc_type`, not in a post-hoc update.

### Pitfall 3: TYPE_FIELD_COUNT Mismatch After Adding type_args Field
**What goes wrong:** `alloc_struct(TYPE_TYPE_KEY, TYPE_FIELD_COUNT)` allocates a struct with N slots. If `TYPE_FIELD_COUNT` is still 4 after adding field 4 (type_args array), `set_field(href, 4, ...)` will fail or write out of bounds.
**Why it happens:** Struct allocation requires the correct field count upfront.
**How to avoid:** Bump `TYPE_FIELD_COUNT` from 4 to 5 before adding any code that sets field 4.
**Warning signs:** `heap.set_field` returns Err; GC corruption on type_args access.

### Pitfall 4: TypeOf Dispatch Breaks on TypeSpec Tokens
**What goes wrong:** Current TYPEOF arm: `let typedef_0based = ((*type_idx & 0x00FF_FFFF) as usize).saturating_sub(1)` — this interprets the row index as a TypeDef index regardless of table_id. A TypeSpec token (table 4) will resolve to the wrong TypeDef.
**Why it happens:** The dispatch does not check `*type_idx >> 24` for the table ID.
**How to avoid:** Branch on table ID: if `(type_idx >> 24) == 2`, use `get_or_alloc_type`; if `== 4`, use `get_or_alloc_typespec_type`.
**Warning signs:** typeof(Array<int>) returns a Type with name="" or name="Int" instead of name="Array".

### Pitfall 5: Empty Array Field Count in GC Survival Tests
**What goes wrong:** Existing test `test_type_object_survives_gc` asserts `before >= 4` (Type + 3 strings). After adding field 4 (empty Array), each Type allocation creates one more heap object.
**Why it happens:** GC tests count heap objects; the count changes when Type gains a new Array field.
**How to avoid:** Update assertion to `before >= 5` (Type + 3 strings + 1 empty Array).

## Code Examples

### GEN-01: Populating is_generic in get_or_alloc_type

```rust
// In reflection.rs, get_or_alloc_type — replace the hardcoded false:
// Source: writ-module/src/tables.rs (TableId::TypeDef = 2, TableId::GenericParam = 13)
use writ_module::tables::TableId;
let target_row = (typedef_idx + 1) as u32;
let is_generic = module.generic_params.iter().any(|p|
    p.owner.table_id() == TableId::TypeDef.as_u8()
    && p.owner.row_index() == Some(target_row)
);
let _ = heap.set_field(href, 3, Value::Bool(is_generic));
```

### GEN-01: TypeGetIsGeneric intrinsic (already works — reads field 3)

```rust
// intrinsics.rs:650 — TypeGetIsGeneric reads heap field 3. No change needed here.
// The fix is entirely in get_or_alloc_type populating field 3 correctly.
IntrinsicId::TypeGetIsGeneric => {
    let type_href = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
    let val = ctx.heap.get_field(type_href, 3).unwrap_or(Value::Void);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = val;
    ExecutionResult::Continue
}
```

### GEN-02: TypeTypeArgs intrinsic

```rust
// Source: matches TypeAttributes pattern (intrinsics.rs:472)
IntrinsicId::TypeTypeArgs => {
    let type_href = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
    // Field 4 is the type_args Array (populated in get_or_alloc_type or get_or_alloc_typespec_type)
    let val = ctx.heap.get_field(type_href, 4).unwrap_or(Value::Void);
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = val;
    ExecutionResult::Continue
}
```

### GEN-03: MethodInfoAttributes intrinsic (sketch)

```rust
// Source: TypeAttributes intrinsic pattern (intrinsics.rs:472)
IntrinsicId::MethodInfoAttributes => {
    use writ_module::tables::{TableId, ATTR_OWNER_KIND_DECL};
    let mi_href = helpers::extract_ref(&ctx.task.call_stack.last().unwrap().registers[r_obj as usize]);
    let (method_module_idx, method_idx) = match ctx.reflection.lookup_method_identity(mi_href) {
        Some(id) => id,
        None => return ExecutionResult::Crash("MethodInfoAttributes: not a MethodInfo".into()),
    };
    let target_row = (method_idx + 1) as u32;
    let mut attr_data: Vec<(String, Vec<writ_module::attr::AttrValue>)> = Vec::new();
    {
        let module = &ctx.modules[method_module_idx].module;
        for row in &module.attribute_defs {
            if row.owner_kind == ATTR_OWNER_KIND_DECL { continue; }
            if row.owner.table_id() != TableId::MethodDef.as_u8() { continue; }
            if row.owner.row_index() != Some(target_row) { continue; }
            // collect name + args (same as TypeAttributes)
        }
    }
    // allocate AttributeInfo objects, build Array
    ...
}
```

### Virtual Module: Adding 3 New Contracts for Phase 108

```rust
// In virtual_module.rs, after Phase 107 contracts:
// Phase 108: Generic reflection + per-member attributes
let type_type_args_contract     = builder.add_contract_def("Type.type_args",           "writ");
builder.add_contract_method("type_type_args", &[], 0);

let methodinfo_attrs_contract   = builder.add_contract_def("MethodInfo.attributes",    "writ");
builder.add_contract_method("methodinfo_attributes", &[], 0);

let fieldinfo_attrs_contract    = builder.add_contract_def("FieldInfo.attributes",     "writ");
builder.add_contract_method("fieldinfo_attributes", &[], 0);

// ... (after TypeDef block) ...
builder.add_impl_def(type_type, type_type_args_contract);
add_intrinsic_method(&mut builder, "type_type_args");

builder.add_impl_def(method_info_type, methodinfo_attrs_contract);
add_intrinsic_method(&mut builder, "methodinfo_attributes");

builder.add_impl_def(field_info_type, fieldinfo_attrs_contract);
add_intrinsic_method(&mut builder, "fieldinfo_attributes");
```

### domain_dispatch.rs: 3 New resolve_intrinsic_id Arms

```rust
("Type",       "type_type_args")        => Some(IntrinsicId::TypeTypeArgs),
("MethodInfo", "methodinfo_attributes") => Some(IntrinsicId::MethodInfoAttributes),
("FieldInfo",  "fieldinfo_attributes")  => Some(IntrinsicId::FieldInfoAttributes),
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| is_generic hardcoded false | is_generic from GenericParam table scan | Phase 108 | Correctness fix |
| TypeOf dispatch assumes TypeDef token | TypeOf dispatch branches on table_id | Phase 108 | Enables typeof(Array<int>) |
| No method/field attribute access | MethodInfo.attributes(), FieldInfo.attributes() | Phase 108 | Per-member attribute queries |

**Deprecated/outdated:**
- `// Field 3: is_generic (bool) — false for now (generic reflection is Phase 108)` comment in `reflection.rs:121` — remove when implementing.

## Open Questions

1. **TypeSpec token encoding in TYPEOF for typeof(Array<int>)**
   - What we know: The compiler lowers `typeof(Array<int>)` to a TYPEOF instruction. The type_idx must encode either a TypeSpec row (table 4) for the instantiation, or the TypeDef for Array plus out-of-band type arg information.
   - What's unclear: The compiler is not in scope for this phase (it's a runtime-only phase per CONTEXT.md). How are tests exercising type_args()? Tests must construct TypeSpec rows manually in test modules using `builder.add_type_spec()`.
   - Recommendation: In integration tests, manually create a TypeSpec row and a custom TYPEOF instruction using the TypeSpec token. The TypeSpec signature blob encodes `[0x20, 0x10, typedef_token_bytes...]` (Array tag + TypeRef/TypeDef element type). This mirrors how the compiler would emit it.

2. **TypeSpec signature blob format for type_args extraction**
   - What we know: TypeSpec.signature is a blob heap offset. The blob encodes an instantiated generic type. Array's signature uses tag 0x20. The element type is encoded as a TypeRef blob (tag 0x10 + 4-byte TypeDef index) or a primitive tag.
   - What's unclear: Is the full TypeSpec signature format documented in the spec?
   - Recommendation: Read `language-spec/spec/45_2_16_il_module_format.md` section on TypeSpec blobs before implementation. The existing Array field type sigs (`[0x20, 0x10, 0x0A, 0x00, 0x00, 0x00]` for `Array<ParameterInfo>`) are the authoritative examples.

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — pure Rust code changes in existing crates).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) |
| Config file | none — workspace Cargo.toml |
| Quick run command | `cargo test -p writ-runtime -- generic_reflection` |
| Full suite command | `cargo test -p writ-runtime` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GEN-01 | `Type.is_generic` is `true` for generic typedef, `false` for non-generic | integration | `cargo test -p writ-runtime -- test_is_generic` | ❌ Wave 0 |
| GEN-01 | `Type.is_generic` is `false` for primitives | integration | `cargo test -p writ-runtime -- test_primitive_is_not_generic` | ❌ Wave 0 |
| GEN-02 | `type_args()` returns correct Types for typeof(Array<int>) | integration | `cargo test -p writ-runtime -- test_type_args_static` | ❌ Wave 0 |
| GEN-02 | `type_args()` returns empty Array for runtime-queried generic | integration | `cargo test -p writ-runtime -- test_type_args_runtime_empty` | ❌ Wave 0 |
| GEN-03 | `MethodInfo.attributes()` returns correct AttributeInfo array | integration | `cargo test -p writ-runtime -- test_method_info_attributes` | ❌ Wave 0 |
| GEN-03 | `FieldInfo.attributes()` returns correct AttributeInfo array | integration | `cargo test -p writ-runtime -- test_field_info_attributes` | ❌ Wave 0 |
| GEN-04 | Spec §1.28.7 documents type_args limitation for runtime types | manual review | n/a | ✅ (already written) |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime -- generic_reflection`
- **Per wave merge:** `cargo test -p writ-runtime`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-runtime/tests/reflection_tests.rs` — add test functions for GEN-01, GEN-02, GEN-03 (append to existing file — do NOT create a new file)
- [ ] No new test file needed: all generic reflection tests go in the existing `reflection_tests.rs` per the Phase 106 precedent

## Sources

### Primary (HIGH confidence)
- `writ-runtime/src/reflection.rs` — complete file read; confirmed TYPE_FIELD_COUNT=4, is_generic hardcoded false, attr_cache key scheme
- `writ-runtime/src/dispatch/intrinsics.rs` — read lines 472-870; confirmed TypeAttributes, TypeGetIsGeneric, MethodInfoInvoke patterns
- `writ-runtime/src/dispatch/mod.rs` — complete file read; confirmed IntrinsicId enum (85 variants through Phase 107), TypeOf dispatch at line 526
- `writ-runtime/src/virtual_module.rs` — read lines 440-700; confirmed 48 contracts, 15 TypeDefs, Phase 107 additions
- `writ-runtime/src/domain_dispatch.rs` — read lines 265-302; confirmed all 28 IntrinsicId mappings through Phase 107
- `writ-module/src/tables.rs` — complete file read; confirmed GenericParamRow, TypeSpecRow, AttributeDefRow, owner_kind semantics
- `writ-module/src/builder.rs` — read lines 260-380; confirmed add_generic_param, add_type_spec APIs
- `language-spec/spec/28_1_28_reflection.md` — complete file read; confirmed §1.28.7 and §1.28.8 are complete

### Secondary (MEDIUM confidence)
- `writ-compiler/src/emit/metadata.rs` lines 303-310 — confirmed `owner_kind: 0=TypeDef, 1=MethodDef, 2=ContractDef` for GenericParam
- Prior research doc `.planning/phases/98-runtime-query-api-and-pre-load-callback/98-RESEARCH.md` — confirmed `owner_kind: 0=type, 1=method, 2=field/global, 3=declaration` for AttributeDef

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all files read directly from source
- Architecture: HIGH — patterns derived from existing Phase 103/107 code
- Pitfalls: HIGH — identified from direct inspection of reflection.rs constants and intrinsics patterns
- GEN-02 TypeSpec token handling: MEDIUM — TypeSpec blob format inferred from existing virtual_module.rs examples; exact sig format for test construction needs verification against spec §2.16

**Research date:** 2026-03-28
**Valid until:** 2026-04-28 (stable internal codebase, no external deps)
