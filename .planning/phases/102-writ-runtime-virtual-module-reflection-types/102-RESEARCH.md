# Phase 102: writ-runtime Virtual Module Reflection Types - Research

**Researched:** 2026-03-28
**Domain:** writ-runtime virtual module extension — reflection TypeDefs, Reflectable contract, primitive intrinsics
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — pure infrastructure phase. Key decisions already locked in spec (§2.18.9):
- 6 reflection class TypeDefs with fields per spec
- Reflectable = contract slot 19, single method get_type() -> Type
- Auto-generated ImplDefs for all user-defined types (compiler phase, not this phase)
- Primitive intrinsics: IntGetType, FloatGetType, BoolGetType, StringGetType registered on pseudo-TypeDefs
- Reflection types are classes (heap-allocated, GC-managed)
- Fields on Type: name (string), kind (string), namespace (string), is_generic (bool)
- Fields on FieldInfo: name (string), declared_type (Type), is_mutable (bool)
- Fields on MethodInfo: name (string), return_type (Type), parameters (Array<ParameterInfo>)
- Fields on ParameterInfo: name (string), parameter_type (Type)
- Fields on AttributeInfo: name (string), args (Box[])
- Fields on ContractInfo: name (string), type (Type)

### Claude's Discretion
All implementation choices within the above constraints.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TYPE-01 | Type builtin class in writ-runtime virtual module with name, kind, namespace, is_generic fields | Add TypeDef with TypeDefKind::Class and 4 field_defs; type_sig bytes per §2.15.2 |
| TYPE-02 | FieldInfo builtin class with name, declared_type, is_mutable fields | Add TypeDef; declared_type references Type via TypeDef token (0x10 encoding) |
| TYPE-03 | MethodInfo builtin class with name, parameters, return_type fields | Add TypeDef; parameters is Array<ParameterInfo> (0x20 encoding) |
| TYPE-04 | ParameterInfo builtin class with name, declared_type fields | Add TypeDef; same pattern as FieldInfo |
| TYPE-05 | AttributeInfo builtin class with name, args fields | Add TypeDef; args is Array<Box> — use 0x20+0x10 pattern for boxed element |
| TYPE-06 | ContractInfo builtin class with name, type fields | Add TypeDef; type field references Type TypeDef |
| TYPE-07 | Reflectable contract (contract 19) with get_type() -> Type method | Add ContractDef as 24th entry; add contract_method "get_type" at slot 0 |
| TYPE-08 | Primitive get_type() intrinsics for Int, Float, Bool, String | Add ImplDef(prim, reflectable) + intrinsic method; add IntrinsicId variants; wire in resolve_intrinsic_id and execute_intrinsic |
</phase_requirements>

## Summary

Phase 102 is a pure extension of the existing `writ-runtime/src/virtual_module.rs` builder. The virtual module
currently contains 23 contract defs, 9 type defs, and assorted impl defs — all built programmatically with
`ModuleBuilder`. Phase 102 adds one more contract (Reflectable at slot 19/0-based index 23), 6 more TypeDefs
(reflection classes), and 4 more ImplDefs (primitive get_type intrinsics) to the same builder function.

In addition, the `Instruction::TypeOf` variant was added in Phase 101 but its VM dispatch arm is missing —
`writ-runtime` currently fails to compile with `E0004: non-exhaustive patterns`. Phase 102 must add a
`Instruction::TypeOf` arm to `dispatch/mod.rs` even though the full TYPEOF semantics (ReflectionIndex, lazy
singleton allocation) are deferred to Phase 103. A `todo!()` stub is not acceptable since it will panic;
a minimal "return null/void" or crash-with-message arm is needed to make tests pass.

**Primary recommendation:** Extend `build_writ_runtime_module()` with the 6 reflection TypeDefs and Reflectable
contract following the existing Section 3/4/5/6 patterns. Add `IntrinsicId` variants and wire them through
`resolve_intrinsic_id` and `execute_intrinsic`. Add a minimal `Instruction::TypeOf` stub arm. All existing tests
must continue to pass, and new tests must verify the exact structure specified in §2.18.9.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| writ-module (ModuleBuilder) | local crate | Building virtual module metadata | Established pattern for all 23 existing contract/type defs |
| writ-runtime (dispatch) | local crate | IntrinsicId enum, execute_intrinsic | Pattern: 4 new variants follow identical structure to IntAdd, BoolEq, etc. |

No external dependencies. This is a pure in-process Rust extension.

**Version verification:** SKIPPED — no external registry packages involved.

## Architecture Patterns

### Recommended Project Structure

Changes are confined to `writ-runtime/src/`:
```
writ-runtime/src/
├── virtual_module.rs    # ADD: Section 8.5 (reflection types) + Section 8.6 (Reflectable contract)
├── dispatch/
│   ├── mod.rs           # ADD: Instruction::TypeOf arm (minimal stub for Phase 102)
│   ├── intrinsics.rs    # ADD: IntrinsicId::IntGetType/FloatGetType/BoolGetType/StringGetType arms
│   └── mod.rs (IntrinsicId enum)  # ADD: 4 new variants
├── domain_dispatch.rs   # ADD: 4 new entries in resolve_intrinsic_id()
```

Tests live in `writ-runtime/tests/` — a new `reflection_types_tests.rs` file follows existing patterns.

### Pattern 1: Adding a TypeDef with Fields (Class Kind)

The existing `Range` type (Struct) and `Entity` type show the exact pattern. Reflection types use
`TypeDefKind::Class` (kind=4). Fields use `add_field_def(name, type_sig, flags)`.

**Type signature encoding (from §2.15.2 and §2.15.3):**

| Type | Encoding bytes |
|------|---------------|
| `string` | `[0x04]` |
| `bool` | `[0x03]` |
| `int` | `[0x01]` |
| `Array<T>` | `[0x20, <T_encoding>]` |
| Named TypeDef at 0-based index N in same module | `[0x10, N, 0x00, 0x00, 0x00]` |

The key encoding for fields that reference other TypeDefs (e.g., `declared_type: Type` on FieldInfo):
use `0x10` followed by the 4-byte LE TypeDef index. **This index is the 0-based row index into the
`type_defs` table of the virtual module.** The TypeDef index must be assigned after the type is added
to the builder, which means the 6 reflection TypeDefs must be added before adding fields that reference them.

**Example — Type TypeDef (4 fields, TypeDefKind::Class):**
```rust
// Source: virtual_module.rs existing pattern (Range, Array)
let type_type = builder.add_type_def("Type", "writ", TypeDefKind::Class, 0);
builder.add_field_def("name",       &[0x04], 0);          // string
builder.add_field_def("namespace",  &[0x04], 0);          // string
builder.add_field_def("kind",       &[0x04], 0);          // string
builder.add_field_def("is_generic", &[0x03], 0);          // bool
```

**Critical ordering constraint:** The TypeDef token for `Type` is assigned when `add_type_def` is called.
When FieldInfo needs a `declared_type: Type` field, it must reference `Type`'s 0-based index. Since
`build_writ_runtime_module` adds all TypeDefs to the builder in order, the 0-based index of `Type` in
the final `type_defs` table will be: (number of existing TypeDefs before the reflection section) + 0.

Currently the virtual module has 9 TypeDefs (indices 0–8):
- 0: Option, 1: Result, 2: Range, 3: Int, 4: Float, 5: Bool, 6: String, 7: Array, 8: Entity

Adding the 6 reflection TypeDefs in order:
- 9: Type, 10: FieldInfo, 11: MethodInfo, 12: ParameterInfo, 13: AttributeInfo, 14: ContractInfo

So `declared_type: Type` on FieldInfo would encode as `[0x10, 0x09, 0x00, 0x00, 0x00]` (0-based index 9).

**HOWEVER** — the `type_sig` encoding stored in the blob heap is the TypeDef row index as seen at build time.
The builder interned blobs are produced during `builder.build()`. Since the virtual module is self-contained
(no cross-module refs for fields), using the 0-based index directly is correct.

### Pattern 2: Adding a Contract (Reflectable at slot 19)

Reflectable is contract number 19 (1-indexed), meaning it is the 19th ContractDef row. Currently there
are 23 contract defs (0-indexed 0–22). Reflectable must be the 24th (0-indexed 23). The spec says
"contract slot 19" which refers to the `writ-runtime` ContractDef table index (1-based row 24, 0-based 23).

Wait — let us re-check. The spec says "Reflectable is contract index 19 in the writ-runtime virtual module."
Looking at the existing 23 contracts in order:
1–18: Add, Sub, Mul, Div, Mod, Neg, Not, Eq, Ord, Index, IndexSet, BitAnd, BitOr, Iterable, Iterator, Into, Error, Speaker
19–23: Into<Float>, Into<Int>, Into<String>, Index<Int>, Index<Range> (specializations)

The spec says Reflectable is at "contract slot 19" but the current module already has 23 contract defs
with the specializations occupying slots 19–23. This is a conflict that must be resolved carefully.

**Resolution:** The spec says slot 19. The current 5 specialization contracts (slots 19–23) were added as
a workaround for FIX-02 (generic dispatch key). They must be renumbered — Reflectable should be inserted
at slot 19 (0-based index 18), pushing the specialization contracts to slots 20–24. The dispatch table
uses contract token raw values as `type_args_hash`, so reordering these contract defs will change their
token values. The `build_dispatch_table` code uses `impl_def.contract.0` directly as `type_args_hash`,
which means the dispatch table will still work correctly after reordering — as long as the ImplDef rows
are updated to reference the new contract tokens (which they are, because the builder adds ImplDefs
referencing the returned contract tokens, not hard-coded indices).

**Pattern for adding Reflectable:**
```rust
// Insert BEFORE the specialization contracts (currently slots 19-23)
let reflectable_contract = builder.add_contract_def("Reflectable", "writ");
builder.add_contract_method("get_type", &[], 0);
```

Then the primitive ImplDefs for Reflectable:
```rust
builder.add_impl_def(int_type,    reflectable_contract);
add_intrinsic_method(&mut builder, "int_get_type");

builder.add_impl_def(float_type,  reflectable_contract);
add_intrinsic_method(&mut builder, "float_get_type");

builder.add_impl_def(bool_type,   reflectable_contract);
add_intrinsic_method(&mut builder, "bool_get_type");

builder.add_impl_def(string_type, reflectable_contract);
add_intrinsic_method(&mut builder, "string_get_type");
```

### Pattern 3: Adding IntrinsicId Variants

In `dispatch/mod.rs`, the `IntrinsicId` enum needs 4 new variants:
```rust
// Reflection get_type (4)
IntGetType, FloatGetType, BoolGetType, StringGetType,
```

In `domain_dispatch.rs`, `resolve_intrinsic_id` needs 4 new arms:
```rust
("Int",    "int_get_type")    => Some(IntrinsicId::IntGetType),
("Float",  "float_get_type")  => Some(IntrinsicId::FloatGetType),
("Bool",   "bool_get_type")   => Some(IntrinsicId::BoolGetType),
("String", "string_get_type") => Some(IntrinsicId::StringGetType),
```

In `dispatch/intrinsics.rs`, `execute_intrinsic` needs 4 new arms. For Phase 102, these return a
placeholder non-null value (e.g., `Value::Int(0)` tagged as a sentinel) since the full Type heap object
allocation is Phase 103. The success criteria only requires "returns a non-null value without panicking":
```rust
IntrinsicId::IntGetType | IntrinsicId::FloatGetType |
IntrinsicId::BoolGetType | IntrinsicId::StringGetType => {
    // Phase 102: return sentinel non-null. Phase 103 replaces with Type heap object.
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[r_dst as usize] = Value::Int(1); // placeholder — non-null sentinel
    ExecutionResult::Continue
}
```

### Pattern 4: Instruction::TypeOf Stub in dispatch/mod.rs

The VM match in `execute_one` is non-exhaustive after Phase 101 added `Instruction::TypeOf`. A minimal
stub is required for Phase 102 compilation. Full semantics are Phase 103 (ReflectionIndex):
```rust
Instruction::TypeOf { r_dst, type_idx: _ } => {
    // Phase 102 stub: return placeholder. Phase 103 wires to ReflectionIndex.
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[*r_dst as usize] = Value::Int(1); // placeholder
    ExecutionResult::Continue
}
```

### Anti-Patterns to Avoid

- **Using `todo!()`** in the TypeOf arm — this compiles but panics immediately, causing test failures.
- **Hard-coding 0-based TypeDef indices** as literal byte values without a comment explaining the mapping.
  Use a comment like `// TypeDef index 9 = Type in writ-runtime module`.
- **Inserting Reflectable after the specialization contracts** — this would put it at slot 24, violating the
  spec requirement of slot 19. It must be inserted before the 5 specialization contracts.
- **Forgetting to add a `has_exactly_24_contract_defs` test update** — the existing test
  `has_exactly_23_contract_defs` will fail if Reflectable is added without updating the assertion.
- **Not handling the `parameters` field on MethodInfo** — the spec says `parameters: ParameterInfo[]`
  but REQUIREMENTS.md (TYPE-03) shows `parameters, return_type`. Use `Array<ParameterInfo>` encoding
  (`[0x20, 0x10, <ParameterInfo_idx_LE>]`).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Type signature bytes | Custom encoding logic | Direct byte literals per §2.15.3 | The virtual module already uses hardcoded blobs (`[0x01]`, `[0x03]`, `[0x12, 0x00, 0x00]`); keep consistent |
| Contract slot numbering | Dynamic contract lookup | Insertion order in builder | The ModuleBuilder assigns slots sequentially; insertion order IS the slot order |
| TypeDef index lookup | Runtime name scan | Compile-time knowledge of insertion order | Module is built in one function; index = insertion order |

## Runtime State Inventory

Step 2.5: SKIPPED — this is not a rename/refactor/migration phase.

## Environment Availability Check

Step 2.6: SKIPPED — this is a pure Rust code change. All dependencies are local crates already present.

Current build status: `cargo test -p writ-runtime` FAILS due to `E0004: non-exhaustive patterns` on
`Instruction::TypeOf`. This is the first blocker Phase 102 must fix.

## Common Pitfalls

### Pitfall 1: Contract Slot Conflict

**What goes wrong:** Adding Reflectable at the end of the contract list puts it at slot 24 instead of 19,
violating the spec requirement. The compiler will reference Reflectable using the TypeRef/ContractDef
token for slot 19.

**Why it happens:** The builder adds contracts sequentially; 5 specialization contracts were inserted at
slots 19–23 for FIX-02. Reflectable must go before them.

**How to avoid:** Insert the `add_contract_def("Reflectable", ...)` call BEFORE the `into_float_spec` block
in `virtual_module.rs`.

**Warning signs:** The `has_exactly_23_contract_defs` test will still pass (wrong), but integration tests
with the compiler will fail because the compiler references Reflectable by token index.

### Pitfall 2: TypeDef Index Encoding for Cross-Reference Fields

**What goes wrong:** `FieldInfo.declared_type` must encode the `Type` TypeDef index (9) as `[0x10, 0x09, 0x00, 0x00, 0x00]`. Using the wrong index silently produces wrong metadata — the field's type appears as a different type.

**Why it happens:** The TypeDef table is flat; index 9 for Type is only correct if all 9 prior TypeDefs are inserted in the same order as today. Any reordering breaks this.

**How to avoid:** Add a compile-time comment mapping each reflection type's 0-based index. Verify in tests
by checking `field_list` ownership and blob heap content (as existing `range_is_struct_with_four_fields_and_one_generic_param` test does).

**Warning signs:** No compile error — only runtime reflection queries will return wrong type metadata.

### Pitfall 3: Forgetting to Update Existing Tests

**What goes wrong:** `has_exactly_23_contract_defs` and `type_defs_include_all_nine_types` will fail after
adding Reflectable and the 6 reflection TypeDefs.

**Why it happens:** These tests assert exact counts.

**How to avoid:** Update both tests as part of the same plan task that extends the builder.

### Pitfall 4: Reflection Type Fields Referencing Each Other During Build

**What goes wrong:** MethodInfo has `parameters: ParameterInfo[]`. To encode the field type, `ParameterInfo`'s
TypeDef index must be known. If ParameterInfo is not yet added when MethodInfo's fields are being defined,
the index is wrong.

**Why it happens:** Fields are added immediately after each `add_type_def` call in the builder.

**How to avoid:** Add all 6 TypeDefs first (without fields), then add fields in a second pass — OR add them
in dependency order: Type, ParameterInfo, AttributeInfo, ContractInfo, FieldInfo, MethodInfo (so each type
exists before any type that references it adds its fields).

**Correct dependency order:**
1. `Type` (no deps)
2. `ParameterInfo` (fields: name: string, declared_type: Type)
3. `AttributeInfo` (fields: name: string, args: Box[])
4. `ContractInfo` (fields: name: string, type: Type)
5. `FieldInfo` (fields: name: string, declared_type: Type, is_mutable: bool)
6. `MethodInfo` (fields: name: string, return_type: Type, parameters: ParameterInfo[])

This ordering ensures every referenced TypeDef is defined before the referencing field is added.

### Pitfall 5: TYPEOF Dispatch Arm Missing (Compilation Blocker)

**What goes wrong:** `cargo test -p writ-runtime` fails with E0004 immediately. No tests run at all.

**Why it happens:** Phase 101 added `Instruction::TypeOf` to writ-module but did not add the dispatch arm.

**How to avoid:** The very first task in any plan MUST be adding the stub arm to `dispatch/mod.rs`.

## Code Examples

Verified patterns from existing `virtual_module.rs`:

### Adding a Class TypeDef with String Fields

```rust
// Source: writ-runtime/src/virtual_module.rs Section 6 (Entity)
// TypeDefKind::Class = 4, matches spec §2.18.9
let type_type = builder.add_type_def("Type", "writ", TypeDefKind::Class, 0);
builder.add_field_def("name",      &[0x04], 0);   // string
builder.add_field_def("namespace", &[0x04], 0);   // string
builder.add_field_def("kind",      &[0x04], 0);   // string
builder.add_field_def("is_generic",&[0x03], 0);   // bool
```

### Encoding a Field Whose Type is Another TypeDef

```rust
// Source: writ-runtime/src/virtual_module.rs — Range uses GenericParam (0x12)
// For a concrete TypeDef reference use 0x10 + 4-byte LE index
// Example: declared_type: Type, where Type is at 0-based index 9
let type_idx_9: u32 = 9;
let type_ref_sig: [u8; 5] = [0x10,
    (type_idx_9 & 0xFF) as u8,
    ((type_idx_9 >> 8) & 0xFF) as u8,
    ((type_idx_9 >> 16) & 0xFF) as u8,
    ((type_idx_9 >> 24) & 0xFF) as u8,
];
builder.add_field_def("declared_type", &type_ref_sig, 0);
```

### Adding Reflectable Contract Before Specializations

```rust
// Source: pattern from virtual_module.rs — insert before into_float_spec block
// Reflectable: contract slot 19 (1-indexed), 0-based index 18
let reflectable_contract = builder.add_contract_def("Reflectable", "writ");
builder.add_contract_method("get_type", &[], 0);

// Then specialization contracts follow (they shift to slots 20-24):
let into_float_spec = builder.add_contract_def("Into<Float>", "writ");
// ...
```

### Adding Primitive Reflectable ImplDefs

```rust
// Source: pattern from int_type ImplDef block (Section 4 of virtual_module.rs)
builder.add_impl_def(int_type, reflectable_contract);
add_intrinsic_method(&mut builder, "int_get_type");

builder.add_impl_def(float_type, reflectable_contract);
add_intrinsic_method(&mut builder, "float_get_type");

builder.add_impl_def(bool_type, reflectable_contract);
add_intrinsic_method(&mut builder, "bool_get_type");

builder.add_impl_def(string_type, reflectable_contract);
add_intrinsic_method(&mut builder, "string_get_type");
```

### New IntrinsicId Variants

```rust
// Source: dispatch/mod.rs IntrinsicId enum — append to existing list
// Reflection (4) — get_type() on primitive pseudo-TypeDefs
IntGetType, FloatGetType, BoolGetType, StringGetType,
```

### resolve_intrinsic_id Entries

```rust
// Source: domain_dispatch.rs resolve_intrinsic_id match arms
("Int",    "int_get_type")    => Some(IntrinsicId::IntGetType),
("Float",  "float_get_type")  => Some(IntrinsicId::FloatGetType),
("Bool",   "bool_get_type")   => Some(IntrinsicId::BoolGetType),
("String", "string_get_type") => Some(IntrinsicId::StringGetType),
```

### TypeOf Stub in execute_one

```rust
// Source: dispatch/mod.rs execute_one match — after Instruction::Unbox arm
Instruction::TypeOf { r_dst, type_idx: _ } => {
    // Phase 102: placeholder return. Phase 103 wires to ReflectionIndex for
    // lazy singleton Type heap object allocation.
    let frame = ctx.task.call_stack.last_mut().unwrap();
    frame.registers[*r_dst as usize] = Value::Int(1); // non-null sentinel
    ExecutionResult::Continue
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No Reflectable contract | Reflectable at contract slot 19 | Phase 102 | CALL_VIRT on Reflectable can now resolve for primitives |
| No reflection TypeDefs | 6 class TypeDefs for Type/FieldInfo/etc | Phase 102 | Metadata visible to compiler's TypeRef resolution |
| TypeOf arm missing | TypeOf stub arm in execute_one | Phase 102 | writ-runtime crate compiles again |

## Open Questions

1. **Contract Slot "19" — 0-based or 1-based?**
   - What we know: Spec says "contract index 19 in the writ-runtime virtual module." The existing module has 23 contracts. Primitives use intrinsics, so user modules reference Reflectable by TypeRef resolved to a ContractDef token in writ-runtime.
   - What's unclear: Whether "index 19" is 0-based (the 20th contract) or 1-based (the 19th contract). Based on how the dispatch table builds `contract_key = (mod_idx << 16) | contractdef_row_0based`, the 0-based position matters.
   - Recommendation: Treat "contract slot 19" as the 19th contract definition (1-indexed), placing it at 0-based index 18. This makes Reflectable the 19th ContractDef row when the module is serialized. The 5 specialization contracts move from 0-based 18–22 to 0-based 19–23. Insert Reflectable immediately after Speaker (the 18th base contract) and before the specializations.

2. **Box[] encoding for AttributeInfo.args**
   - What we know: `args: Box[]` — an array of boxed values. Spec says §3.15 BOX/UNBOX coercion. Box is encoded how?
   - What's unclear: The type encoding for `Box` in a field signature. Looking at §2.15.3, there's no explicit `Box` kind byte — Box is a runtime wrapping, not a distinct type kind in the TypeRef encoding.
   - Recommendation: For Phase 102, encode `args` as a generic `Array` with element type void (`[0x20, 0x00]`) as a placeholder. The actual Box encoding is a Phase 103/104 concern. The field metadata only needs to exist at this phase; semantic correctness of the encoding is enforced when the compiler references it (Phase 104+).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) |
| Config file | none (Rust default) |
| Quick run command | `cargo test -p writ-runtime` |
| Full suite command | `cargo test -p writ-runtime` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TYPE-01 | Type TypeDef exists with 4 fields (name, namespace, kind, is_generic), kind=class | unit | `cargo test -p writ-runtime type_typedef` | ❌ Wave 0 |
| TYPE-02 | FieldInfo TypeDef exists with 3 fields | unit | `cargo test -p writ-runtime fieldinfo_typedef` | ❌ Wave 0 |
| TYPE-03 | MethodInfo TypeDef exists with 3 fields | unit | `cargo test -p writ-runtime methodinfo_typedef` | ❌ Wave 0 |
| TYPE-04 | ParameterInfo TypeDef exists with 2 fields | unit | `cargo test -p writ-runtime parameterinfo_typedef` | ❌ Wave 0 |
| TYPE-05 | AttributeInfo TypeDef exists with 2 fields | unit | `cargo test -p writ-runtime attributeinfo_typedef` | ❌ Wave 0 |
| TYPE-06 | ContractInfo TypeDef exists with 2 fields | unit | `cargo test -p writ-runtime contractinfo_typedef` | ❌ Wave 0 |
| TYPE-07 | Reflectable is at ContractDef 0-based index 18, method "get_type" at slot 0 | unit | `cargo test -p writ-runtime reflectable_contract_slot` | ❌ Wave 0 |
| TYPE-08 | CALL_VIRT on Int value with Reflectable contract dispatches to IntGetType without panic | integration | `cargo test -p writ-runtime int_get_type_intrinsic` | ❌ Wave 0 |

The success criterion "calls get_type() on an Int value dispatches to IntrinsicId::IntGetType without panic" requires an integration test that:
1. Hand-assembles a module with a CALL_VIRT instruction targeting the Reflectable contract
2. Loads it via RuntimeBuilder alongside the virtual module
3. Spawns a task with an Int value
4. Ticks to completion and asserts Completed (not Crash)

This follows the pattern used in `hook_dispatch_tests.rs` for verifying contract dispatch.

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime`
- **Per wave merge:** `cargo test -p writ-runtime`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-runtime/tests/reflection_types_tests.rs` — covers TYPE-01 through TYPE-08
  - Tests for virtual module structure (6 new TypeDefs with correct kind/field counts)
  - Test for Reflectable at 0-based index 18 with "get_type" method
  - Integration test for CALL_VIRT Int + Reflectable → IntrinsicId::IntGetType dispatch without crash

*(No framework install needed — Rust's built-in test harness is already configured.)*

## Sources

### Primary (HIGH confidence)
- `writ-runtime/src/virtual_module.rs` — existing patterns for TypeDef, contract, ImplDef, intrinsic method registration
- `writ-runtime/src/dispatch/mod.rs` — IntrinsicId enum, execute_one match structure
- `writ-runtime/src/domain_dispatch.rs` — resolve_intrinsic_id function, build_dispatch_table
- `writ-runtime/src/dispatch/calls.rs` — resolve_runtime_type_key (confirms Int/Float/Bool/String → primitive pseudo-TypeDef lookup)
- `language-spec/spec/47_2_18_writ_runtime_module_contents.md §2.18.9` — definitive spec for all 6 reflection TypeDefs + Reflectable
- `language-spec/spec/28_1_28_reflection.md §1.28.4` — Reflectable contract semantics
- `language-spec/spec/44_2_15_il_type_system.md §2.15.2/2.15.3` — type tag encoding (0x01=int, 0x03=bool, 0x04=string, 0x10=TypeDef ref)
- `writ-module/src/tables.rs` — confirms TypeDefKind::Class = 4 exists

### Secondary (MEDIUM confidence)
- Build error output: `E0004: non-exhaustive patterns Instruction::TypeOf` — confirms Phase 102 must add the dispatch stub

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all changes are local Rust crate extensions following verified existing patterns
- Architecture: HIGH — insertion order, type encoding, and dispatch mechanism fully verified against existing code
- Pitfalls: HIGH — most pitfalls discovered by direct code inspection, not speculation

**Research date:** 2026-03-28
**Valid until:** 2026-06-28 (stable internal crate — no external dependency churn)
