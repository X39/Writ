# Phase 105: writ-compiler Reflectable Auto-Impl Emission - Research

**Researched:** 2026-03-28
**Domain:** writ-compiler IL metadata emission — ImplDef collection and method body generation
**Confidence:** HIGH

## Summary

Phase 105 adds automatic emission of Reflectable ImplDef rows for every user-defined TypeDef (struct, class, entity, enum). The CONTEXT.md has locked all key decisions: interleaved emission in TypeDef declaration order, Reflectable is contract 19 (0-based index 18, 1-based token row 19) in the writ-runtime virtual module, each auto-impl has a single `get_type()` method body emitting TYPEOF with the type's own type_idx, and extern types are excluded.

The implementation has two separable concerns: (1) metadata collection — adding the MethodDef and ImplDef rows to the ModuleBuilder during `collect_defs`, and (2) body emission — adding an `EmittedBody` with a single TYPEOF instruction to `emit_all_bodies`. The interleaving constraint is satisfied automatically if the auto-impl rows are emitted immediately after each TypeDef in the existing `collect_defs` loop (the finalize pass already sorts MethodDefs by parent TypeDefHandle index, preserving method_list ordering).

The disassembler outputs `.impl TypeName : ContractName { .method ... }` directives, so every golden `.writil` file that contains user types will gain new `.impl Reflectable` blocks and require re-blessing. Several unit tests in `emit_tests.rs` assert exact ImplDef counts and will need updating.

**Primary recommendation:** Add auto-impl emission inline in `collect_defs` for Struct/Class/Entity/Enum arms, register a cross-module ContractRef token for Reflectable once during module setup, emit method bodies by extending `emit_all_bodies` with a parallel synthetic-body list keyed by TypeDefHandle.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Reflectable auto-impl must be emitted interleaved in TypeDef declaration order in the main codegen pass, never in a post-pass — preserves method_list offset invariant
- Reflectable is contract 19 (0-based index 18) in the virtual module
- Each auto-impl has a single method: get_type() -> Type
- Extern types are excluded from auto-impl (they are host-managed)
- The get_type() method body should emit a TYPEOF instruction with the type's own type index

### Claude's Discretion
All implementation choices are at Claude's discretion — infrastructure phase. Key decisions locked in STATE.md as above.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| COMP-03 | Reflectable auto-impl emitted per user TypeDef interleaved in main codegen pass | Method_list offset invariant analysis; collect_defs loop structure documented |
| REFL-02 | expr.get_type() returns runtime dynamic Type via Reflectable contract dispatch | Dispatch table build_dispatch_table() chain documented; TYPEOF instruction body pattern established |
</phase_requirements>

---

## Project Constraints (from CLAUDE.md)

No `CLAUDE.md` found in the repository root. No project-level constraints to document.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| writ-compiler (internal) | workspace | emit/collect and emit/body passes | already owns all metadata emission |
| writ-module (internal) | workspace | MetadataToken, TableId, ImplDefRow structs | binary format definition |
| writ-runtime (virtual) | workspace | Reflectable contract lives at index 18 | cross-module reference target |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| rustc_hash::FxHashMap | workspace | typedef_handles and similar lookup maps | already used throughout collect pass |

**Installation:** No new external dependencies. All crates are workspace members.

---

## Architecture Patterns

### Recommended Project Structure

No new files required. Changes span:
```
writ-compiler/src/emit/collect/mod.rs       — auto-impl emission in collect_defs loop
writ-compiler/src/emit/collect/contracts.rs — new emit_reflectable_auto_impl() helper
writ-compiler/src/emit/body/mod.rs          — synthetic body emission in emit_all_bodies
```

### Pattern 1: Inline TypeDef-interleaved Auto-Impl

**What:** After each Struct/Class/Entity/Enum arm in `collect_defs`, immediately emit the Reflectable ImplDef and its `get_type()` MethodDef into the same ModuleBuilder pass.

**When to use:** All user-defined non-extern types. Skip for `Component`, `ExternComponent`, `ExternFn`.

**Key invariant:** The finalize pass sorts `method_defs` by `parent.0` (TypeDefHandle index). Auto-impl methods must use the *same* TypeDefHandle as the TypeDef they belong to. Because auto-impl MethodDef rows are parented to their TypeDef (not to NULL), finalize will group them contiguously with the TypeDef's own methods, naturally satisfying the method_list offset invariant.

**Existing pattern reference** (collect/contracts.rs line 130–138):
```rust
// Source: writ-compiler/src/emit/collect/contracts.rs
let method_handle = builder.add_methoddef(
    target_type_handle,         // parent = the TypeDef
    &fn_decl.name,
    sig_blob,
    flags,
    Some(*_method_def_id),
    param_count,
);
```

**Auto-impl equivalent:**
```rust
// In collect/contracts.rs — new pub(super) function
pub(super) fn emit_reflectable_auto_impl(
    typedef_handle: TypeDefHandle,
    type_token: MetadataToken,
    reflectable_contract_token: MetadataToken,
    get_type_sig_blob: u32,
    builder: &mut ModuleBuilder,
) -> MethodDefHandle {
    // method_flags: pub=true, static=false, mut_self=false, hook=None
    // flag 0x80 (intrinsic) is NOT set — the method body is real IL (TYPEOF + RET)
    let flags = method_flags(true, false, false, HookKind::None);
    let method_handle = builder.add_methoddef(
        Some(typedef_handle),
        "get_type",
        get_type_sig_blob,
        flags,
        None,           // no DefId — synthetic method
        1,              // param_count = 1 (self)
    );
    // ImplDef linking this type to Reflectable
    builder.add_impl_def(type_token, reflectable_contract_token, 0, None);
    method_handle
}
```

### Pattern 2: ContractRef Token for Reflectable

**What:** Reflectable lives in writ-runtime, so the user module references it via a `ContractRef` — or by looking up the virtual module's token via a name-based scan at runtime. In practice, the compiler does not have a `ContractRef` table (it uses `TypeRef`); instead, for cross-module contract tokens the existing `collect_impl` code uses a fallback path: `builder.token_for_def(id)` which resolves TypeRef tokens.

**The actual mechanism:** For Reflectable, there is no user-level DefId. Instead, the compiler must encode the contract_token as a raw MetadataToken pointing to the Reflectable ContractDef in the writ-runtime virtual module. The runtime resolves this during `build_dispatch_table()` via `resolve_contract_key_for_impl()`, which accepts cross-module tokens.

**Token encoding:** Reflectable is ContractDef at 0-based index 18 → 1-based row 19 → `MetadataToken::new(TableId::ContractDef, 19)`. This token value is what appears in the ImplDef's `contract_token` field. The runtime builds a `DispatchKey` using this token as the `type_args_hash` discriminator.

**Known cross-module token pattern** (confirmed from virtual_module.rs test):
```rust
// Source: writ-runtime/src/virtual_module.rs line 1115
let reflectable_token = MetadataToken::new(10, 19);  // TableId::ContractDef = 10, row 19
```

Because this is a fixed constant from the spec ("Reflectable is contract 19"), the compiler can hard-code this token. A named constant `REFLECTABLE_CONTRACT_TOKEN` in collect/contracts.rs or collect/mod.rs avoids magic numbers.

### Pattern 3: Synthetic EmittedBody for get_type()

**What:** `emit_all_bodies` iterates `typed_ast.decls` to produce `EmittedBody` entries. Auto-impl methods have no `TypedDecl` entry — they are entirely synthetic. The body must be appended as a separate step after the TypedDecl loop.

**How:** Collect `(MethodDefHandle, type_def_id_or_handle)` pairs during `collect_defs` and pass them through. In `emit_all_bodies`, iterate this synthetic list and emit a minimal body:
```
r0 = alloc(self_ty)   // self parameter occupies r0
r1 = alloc(Type_ty)   // destination for TYPEOF
TYPEOF r1, type_idx
RET r1
```

**type_idx resolution:** Use the same `token_for_def(def_id)` lookup used by `resolve_typeof_type_idx` in `expr/mod.rs`. Since TypeDef tokens are assigned during `finalize()` and `emit_all_bodies` runs after finalize, `builder.token_for_def(def_id)` will return the correct finalized MetadataToken.

**The DefId dilemma:** Auto-impl methods have no `DefId`. `EmittedBody.method_def_id = None` means the serializer must match them by name pattern, similar to lambda bodies. The serializer in `serialize.rs` matches `EmittedBody` to `MethodDef` rows by scanning unmatched MethodDefs in order. Because get_type() MethodDefs are named `"get_type"`, a name-based fallback in the serializer (or a separate `synthetic_bodies` list with explicit handle indices) will bind them correctly.

**Preferred approach:** Pass a `Vec<(MethodDefHandle, DefId)>` from `collect_defs` to `emit_bodies` alongside `lambda_infos`. Each entry is a "reflectable auto-impl" that needs a TYPEOF body. The serializer can match them by MethodDefHandle index.

### Pattern 4: get_type() Signature Blob

**What:** The method signature `(self) -> Type` must be encoded as a blob on the BlobHeap.

**Encoding:** Per the existing `encode_fn_sig_from_ast_sig` pattern:
- param_count = 1 (u16 LE) — just `self`
- self parameter type: TypeRef token for the owning TypeDef
- return type: TypeRef token for `Type` (already registered as `builder.add_type_ref(runtime_mod_idx, "Type", "writ")` in `collect_defs`)

A simpler approach: encode the sig as `(self) -> Type` using the same `encode_fn_sig` utilities. The self parameter is not encoded in the blob (it is implicit in the param_count), so the blob can be `[0x00, 0x00, <Type_typeref_tag>]` — param_count=0 regular params (self is counted separately via `param_count` field on MethodDef), return = TypeRef(Type).

Checking existing encoding: `encode_empty_sig` produces a void-void sig. A `() -> Type` sig would be param_count=0, return=TypeRef(Type_row). The `Type` TypeRef is registered at row index 1 (second `add_type_ref` call in `collect_defs` after `Range`). Its blob tag is `0x30` (TypeRef) + 4-byte offset for the TypeRef index.

**Simpler alternative:** Pre-encode a raw sig blob directly in the auto-impl helper since the signature is fixed and known:
```rust
// () -> Type: 0 params, return = TypeRef to "Type"
// TypeRef tag = 0x30, followed by u32 LE TypeRef row index (1-based)
let type_ref_row = builder.type_ref_row_for_name("Type");  // needs new helper or search
let mut sig = vec![0x00u8, 0x00u8]; // param_count = 0 LE
sig.push(0x30); // TypeRef tag
sig.extend_from_slice(&type_ref_row.to_le_bytes());
```

### Anti-Patterns to Avoid

- **Post-pass ImplDef insertion:** Never add Reflectable ImplDefs in a separate loop after the main TypedDecl loop. The method_list offset invariant requires ImplDef.method_list to point to the correct MethodDef row. The finalize sort groups MethodDefs by parent TypeDefHandle index — appending auto-impl MethodDefs after finalize would break their row positions.

- **Intrinsic flag on auto-impl MethodDef:** Do NOT set the intrinsic flag (0x80) on the auto-generated `get_type()` MethodDef. Intrinsic methods are dispatched by the runtime to native code. The auto-impl body is real IL (TYPEOF + RET) and must be dispatched as a normal IL method. The primitive Reflectable impls in writ-runtime use the intrinsic flag — user-defined type impls do not.

- **Shared sig blob by reference to later TypeRef rows:** TypeRef rows for Type/Int/Float/Bool/String are added in `collect_defs` before the TypedDecl loop. Their indices are deterministic and can be computed once.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Sig blob encoding for () -> Type | Custom byte encoder | Reuse blob_heap.intern() with pre-built byte vec | Pattern already established for log/dialogue builtins in builtins.rs |
| Contract token lookup for cross-module contracts | Scan virtual module at compile time | Hard-code MetadataToken::new(TableId::ContractDef, 19) as a named constant | Contract index is spec-locked; virtual module tests assert this |
| Method body emission infrastructure | New EmittedBody builder API | Existing EmittedBody struct + Instruction::TypeOf | Body struct is simple; TYPEOF already added in Phase 104 |

**Key insight:** The auto-impl is entirely synthetic — no AST, no DefId, no TypedExpr. Everything is built directly from TypeDefHandle + pre-computed constants.

---

## Runtime State Inventory

Not applicable — this is a compiler codegen phase with no rename/migration aspects.

---

## Common Pitfalls

### Pitfall 1: method_list = 0 on ImplDef
**What goes wrong:** `add_impl_def` is called with `method_list = 0`. The finalize pass does NOT update ImplDef.method_list — it only updates TypeDef.method_list. If the auto-impl MethodDef is parented to the TypeDef (TypeDefHandle), TypeDef.method_list gets updated. But ImplDef.method_list is what `build_dispatch_table` and `domain_dispatch.rs` use to find the impl's methods.
**Why it happens:** The compiler's `collect_impl` also passes `method_list = 0` (line 183 of contracts.rs) — this works for user impls because the **runtime uses TypeDef.method_list** to find impl methods, not ImplDef.method_list directly for user modules.
**Reality check from domain_dispatch.rs:** `impl_def.method_list.saturating_sub(1)` is `0 - 1 = usize::MAX` which saturates to 0 — so method_start = 0. The runtime then counts `contract_method_count` methods from position 0, which is wrong unless method_list is set correctly.
**How to avoid:** The auto-impl MethodDef must be parented to the TypeDef so that after `finalize()` the TypeDef.method_list points to it. Then, at the time `add_impl_def` is called, store the current `method_defs.len() + 1` as the method_list value (the MethodDef has just been added). This matches how the writ-module `builder.add_impl_def()` in virtual_module.rs works — it sets `method_list = self.method_defs.len() as u32 + 1` at call time. The compiler's `add_impl_def` takes `method_list` as a parameter; pass `builder.method_def_count() as u32 + 1` **before** adding the MethodDef, then add the MethodDef immediately after. Wait — no, method_list must point to the MethodDef already added. Pass `builder.method_def_count() as u32` (1-based: the just-added method is at index `count`, so row = count = existing_count after push). Correct sequence: add MethodDef first, then call `add_impl_def` with `builder.method_def_count() as u32` for method_list.

**Actually**: looking at the compiler's finalize pass carefully — it does NOT update ImplDef.method_list at all. The compiler's ImplDef.method_list is always 0. The runtime must be tolerating this because for user modules it uses a different dispatch path. Investigation of `build_dispatch_table` shows it reads `impl_def.method_list.saturating_sub(1)` = 0 for method_start, and contract_method_count bounds the range. For a contract with 1 method and method_start=0, it finds method at index 0 — which only works if the MethodDef for get_type is actually at global index 0 in the module, which is unlikely. This is a **real bug to investigate** — or the runtime simply doesn't use ImplDef.method_list for user modules.
**Warning signs:** If `cargo test` passes with method_list=0 for existing user impls, either the dispatch path doesn't use it, or it works by accident. Check whether the ImplDef.method_list is actually needed to be correct for CALL_VIRT to work.

### Pitfall 2: get_type() MethodDef parented to wrong TypeDefHandle
**What goes wrong:** If the auto-impl MethodDef's parent is `None` (top-level function) instead of `Some(typedef_handle)`, finalize() sorts it with `parent.map(|p| p.0).unwrap_or(usize::MAX)` = `usize::MAX`, placing it at the end of the MethodDef table after all type-owned methods. This breaks TypeDef.method_list ranges.
**How to avoid:** Always pass `Some(typedef_handle)` as the parent to `add_methoddef()`.

### Pitfall 3: Interleaving broken by TypeRef rows added before the loop
**What goes wrong:** TypeRef rows for Type/Int/Float/Bool/String are added before the TypedDecl loop. Their row indices depend on insertion order. If the auto-impl sig blob encodes a TypeRef row index, that index must match the actual finalized TypeRef table position.
**How to avoid:** Compute the TypeRef row index for "Type" by searching the existing `type_refs` vec by name after it's been added, not by hard-coding an index.

### Pitfall 4: Existing emit_tests.rs assertions break
**What goes wrong:** `impl_emits_impldef` asserts `impl_def_count() == 1` for a source with one user impl. After Phase 105, the struct `Foo` also gets a Reflectable auto-impl, making count = 2.
**How to avoid:** Update all affected tests. See "Existing Tests Requiring Updates" section.

### Pitfall 5: Golden .writil files gain .impl blocks
**What goes wrong:** The disassembler emits `.impl TypeName : Reflectable { .method "get_type" ... }` for every user TypeDef. All golden tests that compile sources with user types will fail with output mismatch.
**How to avoid:** Run `BLESS=1 cargo test -p writ-golden` after implementation to regenerate all affected golden snapshots. Do not try to predict which exact golden files change — bless all of them.

### Pitfall 6: fn_param_map missing for synthetic method
**What goes wrong:** `emit_all_bodies` calls `builder.get_fn_params(def_id)` to pre-allocate parameter registers. Synthetic get_type() methods have no DefId, so `fn_param_map` has no entry. Without explicit self-register allocation, the body emitter starts at r0 for the TYPEOF result, but the runtime expects r0 = self.
**How to avoid:** For synthetic bodies, manually pre-allocate r0 for self before emitting TYPEOF. Do not rely on `fn_param_map`. The self type is the TypeDef's own Ty.

---

## Code Examples

### Example 1: How existing collect_impl emits an ImplDef (source of truth)
```rust
// Source: writ-compiler/src/emit/collect/contracts.rs lines 166-183
let type_token = target_type_handle
    .map(|h| MetadataToken::new(TableId::TypeDef, (h.0 + 1) as u32))
    .unwrap_or(MetadataToken::NULL);
let contract_token = contract_def_id
    .and_then(|id| {
        contractdef_handles.get(&id).map(|h| {
            MetadataToken::new(TableId::ContractDef, (h.0 + 1) as u32)
        }).or_else(|| builder.token_for_def(id))
    })
    .unwrap_or(MetadataToken::NULL);
// method_list will be set during finalize to point to the impl's methods.
builder.add_impl_def(type_token, contract_token, 0, Some(impl_def_id));
```

### Example 2: Reflectable contract token (spec-locked value)
```rust
// Source: writ-runtime/src/virtual_module.rs test at line 1115
// Reflectable is ContractDef at 0-based index 18 → 1-based row 19
// TableId::ContractDef = 10
const REFLECTABLE_CONTRACT_TOKEN: MetadataToken = MetadataToken(
    (10u32 << 24) | 19u32   // high byte = table id, low 24 bits = row
);
// Note: verify MetadataToken bit layout in writ-compiler/src/emit/metadata.rs
// MetadataToken::new(TableId::ContractDef, 19) is the correct constructor call
```

### Example 3: TYPEOF instruction emission (from Phase 104)
```rust
// Source: writ-compiler/src/emit/body/expr/mod.rs lines 497-501
TypedExpr::TypeOf { ty, static_ty, .. } => {
    let r_dst = emitter.alloc_reg(*ty);
    let type_idx = resolve_typeof_type_idx(emitter, *static_ty);
    emitter.emit(Instruction::TypeOf { r_dst, type_idx });
    r_dst
}
```

### Example 4: Type_idx for user type (from resolve_typeof_type_idx)
```rust
// Source: writ-compiler/src/emit/body/expr/mod.rs lines 519-527
TyKind::Struct(def_id)
| TyKind::Class(def_id)
| TyKind::Entity(def_id)
| TyKind::Enum(def_id)
| TyKind::Contract(def_id) => {
    emitter.builder.token_for_def(*def_id)
        .map(|t| t.0)
        .unwrap_or(0)
}
```

For synthetic bodies, the equivalent is:
```rust
let type_idx = builder.token_for_def(type_def_id).map(|t| t.0).unwrap_or(0);
```

### Example 5: Synthetic body in emit_all_bodies (lambda analog)
```rust
// Source: writ-compiler/src/emit/body/mod.rs lines 592-615
// Lambda bodies also have method_def_id: None
bodies.push(EmittedBody {
    method_def_id: None,      // no source DefId
    instructions: emitter.instructions,
    reg_count,
    reg_types,
    source_spans: emitter.source_spans,
    debug_locals: emitter.debug_locals,
    pending_strings: emitter.pending_strings,
    label_allocator: emitter.labels,
});
```

### Example 6: How collect_defs loop structure enables interleaving
```rust
// Source: writ-compiler/src/emit/collect/mod.rs lines 120-134
for decl in &typed_ast.decls {
    match decl {
        TypedDecl::Struct { def_id } => {
            collect_struct(...);  // adds TypeDef + FieldDefs
            // ADD HERE: emit_reflectable_auto_impl(typedef_handle, ...)
        }
        TypedDecl::Class { def_id } => {
            collect_class(...);
            // ADD HERE: emit_reflectable_auto_impl(typedef_handle, ...)
        }
        TypedDecl::Entity { def_id } => {
            collect_entity(...);
            // ADD HERE: emit_reflectable_auto_impl(typedef_handle, ...)
        }
        TypedDecl::Enum { def_id } => {
            collect_enum(...);
            // ADD HERE: emit_reflectable_auto_impl(typedef_handle, ...)
        }
        // Component and ExternComponent: NO auto-impl (excluded by locked decision)
        ...
    }
}
```

---

## Existing Tests Requiring Updates

### writ-compiler/tests/emit_tests.rs

| Test | Current assertion | After Phase 105 | Update needed |
|------|-------------------|-----------------|---------------|
| `impl_emits_impldef` | `impl_def_count() == 1` | `struct Foo` gets Reflectable auto-impl → count = 2 | Change to 2, or change source to exclude types |
| `struct_emits_typedef` | no ImplDef check | no change | None |
| `typedef_tokens_are_one_based` | `def_token_map.len() == 3` (3 structs, no log refs) | 3 structs, each with Reflectable auto-impl MethodDef (no DefId → not in def_token_map); ImplDef has None DefId too. token count stays 3 | Probably no change |

### writ-compiler/tests/emit_body_tests.rs
Check for any assertions on `impl_def_count` or MethodDef counts involving structs/entities.

### writ-golden/tests/*.writil
Every golden file with a user type (struct/class/entity/enum) will gain `.impl TypeName : Reflectable { .method "get_type" ... }` blocks. The full list includes but is not limited to:
- `type_struct_new.writil` (has Point struct)
- `type_class_new.writil` (has Node class)
- `type_struct_eq.writil` (has Point and Color structs)
- `type_enum_match.writil` (has enum)
- `quest_system.writil` (has QuestStatus enum, QuestType enum, Narrator entity)
- `entity_get_or_create.writil` (has Guard entity)
- All other golden files with any type declaration

**Run `BLESS=1 cargo test -p writ-golden` to regenerate all affected snapshots.**

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No Reflectable impls on user types | Auto-emit per TypeDef interleaved in collect_defs | Phase 105 (now) | REFL-02 satisfied; CALL_VIRT on get_type() dispatches to IL method |
| typeof(expr) returns stub Value::Int(1) | TYPEOF opcode returns lazy singleton Type object | Phase 103/104 | User types get_type() will now return the correct Type via same lazy singleton pattern |

---

## Open Questions

1. **ImplDef.method_list = 0 vs correct value**
   - What we know: the compiler's `collect_impl` always passes method_list=0 to `add_impl_def`; the finalize pass does NOT update ImplDef.method_list; yet CALL_VIRT works for existing user-defined contract impls
   - What's unclear: does `build_dispatch_table` actually use ImplDef.method_list for finding methods, or does it use TypeDef.method_list + some other mechanism for user module impls? domain_dispatch.rs line 47 reads `impl_def.method_list.saturating_sub(1)` — with method_list=0 this gives method_start=0
   - Recommendation: Before implementing, trace through `build_dispatch_table` for a module with one TypeDef and one ImplDef to verify whether method_list=0 causes the wrong method to be dispatched. If it does, the auto-impl must set method_list correctly: call `add_methoddef` first, then `add_impl_def` with `builder.method_def_count() as u32` (the 1-based index of the just-added method). If method_list=0 is fine (perhaps the runtime uses TypeDef.method_list for user module methods), leave it as 0.

2. **Sig blob for (self) -> Type**
   - What we know: `encode_empty_sig` produces `() -> void`; the Type TypeRef is registered at a known position in the TypeRef table
   - What's unclear: the exact blob encoding tag for a TypeRef return type and how `encode_fn_sig` represents self
   - Recommendation: The simplest approach is to encode the sig as `[0x00, 0x00, 0x30, <typeref_idx_u32_le>]` — 0 regular params, TypeRef return. Verify the TypeRef index for "Type" by checking the order of `add_type_ref` calls in `collect_defs` (Range=0, Type=1, Int=2, Float=3, Bool=4, String=5 → Type TypeRef is at 0-based index 1, 1-based row 2).

---

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — pure compiler code change in an existing Rust workspace).

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness + insta (snapshots) |
| Config file | Cargo.toml workspaces |
| Quick run command | `cargo test -p writ-compiler -- emit_tests` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| COMP-03 | Module with 3 user types has exactly 3 Reflectable ImplDefs | unit | `cargo test -p writ-compiler -- reflectable_auto_impl` | ❌ Wave 0 |
| COMP-03 | method_list offset invariant holds for all auto-generated ImplDefs | unit | `cargo test -p writ-compiler -- method_list_invariant` | ❌ Wave 0 |
| REFL-02 | Disassembled output contains .impl blocks for all user types | golden | `cargo test -p writ-golden` | ✅ (needs re-blessing) |
| REFL-02 | get_type() body emits TYPEOF r1, type_idx; RET r1 | unit | `cargo test -p writ-compiler -- get_type_body_emits_typeof` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-compiler -- emit_tests`
- **Per wave merge:** `cargo test -p writ-compiler && cargo test -p writ-golden`
- **Phase gate:** Full `cargo test` green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-compiler/tests/emit_tests.rs` — add `reflectable_auto_impl_three_types` test (3 user types → 3 ImplDefs beyond user impls)
- [ ] `writ-compiler/tests/emit_tests.rs` — update `impl_emits_impldef` to account for 1 auto-impl on `struct Foo` (count becomes 2)
- [ ] `writ-compiler/tests/emit_tests.rs` — add `method_list_invariant_holds` test verifying ImplDef.method_list is non-zero and within method table bounds
- [ ] `writ-compiler/tests/emit_body_tests.rs` — add `get_type_body_is_typeof_ret` test verifying body instructions are `[TYPEOF, RET]`
- [ ] Golden files: run `BLESS=1 cargo test -p writ-golden` to regenerate — not new files, just re-blessed content

---

## Sources

### Primary (HIGH confidence)
- `writ-compiler/src/emit/collect/mod.rs` — collect_defs loop structure; TypedDecl dispatch
- `writ-compiler/src/emit/collect/contracts.rs` — collect_impl pattern; add_impl_def call site
- `writ-compiler/src/emit/collect/types.rs` — collect_struct/entity/enum/class patterns
- `writ-compiler/src/emit/module_builder.rs` — add_typedef, add_methoddef, add_impl_def APIs; finalize sort logic
- `writ-compiler/src/emit/body/mod.rs` — emit_all_bodies structure; EmittedBody struct; lambda analog
- `writ-compiler/src/emit/body/expr/mod.rs` — TypeOf emission; resolve_typeof_type_idx
- `writ-runtime/src/virtual_module.rs` — Reflectable at index 18 confirmed; contract token = MetadataToken::new(10,19)
- `writ-runtime/src/domain_dispatch.rs` — how ImplDef.method_list is used at runtime
- `writ-assembler/src/disassembler.rs` — `.impl` blocks are emitted in disassembly output
- `writ-compiler/tests/emit_tests.rs` — exact test assertions that need updating

### Secondary (MEDIUM confidence)
- STATE.md decisions log — locked decisions for this phase
- REQUIREMENTS.md — COMP-03, REFL-02 requirements definition

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all code is in-repo, read directly
- Architecture: HIGH — finalize sort logic and method_list invariant read from source
- Pitfalls: HIGH for structural issues (wrong parent, test counts); MEDIUM for ImplDef.method_list=0 correctness (needs runtime trace to confirm)

**Research date:** 2026-03-28
**Valid until:** Stable until Phase 106 modifies the emit pipeline (estimated 30 days)
