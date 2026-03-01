# Phase 24: IL Codegen — Metadata Skeleton - Research

**Researched:** 2026-03-03
**Domain:** Rust compiler internals — metadata table emission from typed IR
**Confidence:** HIGH

## Summary

Phase 24 introduces the `emit` module to `writ-compiler`, translating the TypedAst + DefMap (from Phase 23) into a ModuleBuilder populated with all 21 metadata table rows. This is a "skeleton" pass — it creates the metadata structure that Phase 25 (method body emission) will reference, but does not emit any IL instructions.

The primary technical challenge is the two-pass collect-then-assign strategy that ensures all DefIds have stable MetadataTokens before any cross-references are written. The secondary challenge is correctly deriving CALL_VIRT slot numbers from contract declaration order (not impl block traversal order). The codebase has established patterns for arena-based ID allocation (id_arena), interning with structural dedup (TyInterner), and module-per-phase organization (ast/, lower/, resolve/, check/) that the new emit/ module should follow.

**Primary recommendation:** Create a new `writ-compiler/src/emit/` module with ModuleBuilder as the central struct, a two-pass emission function (collect all defs, then assign row indices), and per-table row structs mirroring the 21 spec tables.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **Token Assignment Strategy:** Two-pass approach: Pass 1 collects all definitions and groups them by table. Pass 2 assigns row indices in table order. No fixup needed — all row indices known before cross-references are written.
- **DefId to MetadataToken mapping:** Stored in a separate `HashMap<DefId, MetadataToken>` — TypedAst stays immutable, mapping lives in ModuleBuilder or a side table.
- **ModuleBuilder API Surface:** New `emit` module at `writ-compiler/src/emit/` — alongside existing `ast`, `lower`, `resolve`, `check`. Generic table-level API: ModuleBuilder has methods like `add_typedef(name, kind, flags)` that don't know about TypedAst. A separate pass translates TypedAst to ModuleBuilder calls.
- **Contract Vtable Slot Ordering:** Separate vtable pass: collect all contract info first, then a dedicated pass assigns slot numbers from contract declaration order. Slot assignment is fixed before any method body can emit CALL_VIRT.
- **Lifecycle Hook Registration:** Hooks apply to BOTH structs and entities. Universal hooks (structs + entities): on_create, on_finalize, on_serialize, on_deserialize. Entity-specific hooks: on_destroy, on_interact. Hook registration via MethodDef.flags only — set hook_kind bits in the flags field. No separate fast-lookup slots or side tables. Absent hooks = simply no MethodDef row with that hook_kind.

### Claude's Discretion
- Row ordering within tables (source order vs grouped by parent — must satisfy spec's list-ownership pattern)
- Whether to emit TypeRef/MethodRef/FieldRef rows for writ-runtime types in Phase 24 or defer to Phase 25
- Whether ModuleBuilder owns string/blob heaps internally or uses separate heap builders
- Whether to include binary serialization of metadata tables or keep in-memory only
- MetadataToken representation (newtype struct vs type alias)
- TypeRef blob encoding approach (separate TypeSigBuilder vs inline in ModuleBuilder)
- Per-table Rust row struct types vs generic row representation
- ImplDef method ordering relative to contract slots
- Generic contract representation (single ContractDef + GenericParam vs per-instantiation)
- Validation level for inter-phase invariants (trust Phase 23 vs debug-assert safety nets)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| EMIT-01 | Compiler emits ModuleDef, ModuleRef (including writ-runtime), and ExportDef metadata | ModuleBuilder needs add_module_def(), add_module_ref(), add_export_def() methods. ModuleDef is always 1 row. ModuleRef for writ-runtime emitted unconditionally. ExportDef rows for all pub-visible items. |
| EMIT-02 | Compiler emits TypeDef, FieldDef, MethodDef, and ParamDef rows for all named types | Two-pass strategy: Pass 1 collects all TypeDefs from DefMap entries (structs, enums, entities, components), Pass 2 assigns row indices. Children (FieldDef, MethodDef, ParamDef) must be contiguous under parent per list-ownership pattern. |
| EMIT-03 | Compiler emits ContractDef, ContractMethod, and ImplDef rows with correct CALL_VIRT slot ordering from contract declaration | Dedicated vtable pass collects contracts first, assigns slot numbers from contract member declaration order. ImplDef references both type and contract tokens. |
| EMIT-04 | Compiler emits GenericParam and GenericConstraint rows | GenericParam rows attach to TypeDefs and MethodDefs via owner token. GenericConstraint rows reference GenericParam row + constraint contract token. |
| EMIT-05 | Compiler emits GlobalDef and ExternDef rows | GlobalDef for const and global mut declarations. ExternDef for extern fn/struct/component declarations. |
| EMIT-06 | Compiler emits ComponentSlot rows with override blobs for entity component slots | ComponentSlot rows link entity TypeDef token to component type token. Override blobs stored in blob heap (Phase 25 concern for actual blob encoding, Phase 24 can emit placeholder empty blobs or skip override data). |
| EMIT-22 | Compiler registers lifecycle hook MethodDefs with correct hook_kind flags in entity TypeDefs | MethodDef.flags hook_kind field: 0=none, 1=create, 2=destroy, 3=finalize, 4=serialize, 5=deserialize, 6=interact. Set during MethodDef row creation for struct/entity hooks. |
| EMIT-25 | Compiler emits LocaleDef table rows for localization keys | LocaleDef rows reference dialogue MethodDef tokens. Localization keys are generated during lowering (dialogue.rs). Phase 24 needs to collect these and emit LocaleDef rows. |
| EMIT-29 | Compiler emits AttributeDef rows for runtime attribute inspection | AttributeDef rows: owner token, owner_kind, name, value blob. Attributes like [Singleton], [Conditional] from AST declarations. |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rustc-hash | 2.1.1 | FxHashMap for DefId/token lookups | Already in project; fastest hash for integer keys |
| id-arena | 2.3.0 | Arena allocation pattern reference | Already used for DefId; MetadataToken may follow similar pattern but spec uses u32 encoding |

### Supporting
No new dependencies needed. The emit module uses only existing project dependencies.

## Architecture Patterns

### Recommended Module Structure
```
writ-compiler/src/emit/
├── mod.rs              # Public API: emit() entry point
├── module_builder.rs   # ModuleBuilder struct + table-level add_* methods
├── metadata.rs         # MetadataToken, TableId, row structs for all 21 tables
├── collect.rs          # Pass 1: walk TypedAst+DefMap, call ModuleBuilder methods
├── slots.rs            # Vtable slot assignment pass (contract declaration order)
├── heaps.rs            # String heap + blob heap builders with dedup
├── type_sig.rs         # TypeRef blob encoding (Ty -> blob bytes)
└── error.rs            # Emit-phase error types
```

### Pattern 1: Two-Pass Collect-Then-Assign
**What:** Pass 1 walks the TypedAst and DefMap, calling ModuleBuilder::add_* methods to register all definitions. Each add_* returns a provisional handle. Pass 2 (internal to ModuleBuilder) assigns contiguous row indices per table, respecting parent-child ordering for the list-ownership pattern.
**When to use:** Always — this is the locked decision from CONTEXT.md.
**Example:**
```rust
// Pass 1: collect
let typedef_handle = builder.add_typedef("Potion", "survival", TypeDefKind::Struct, flags);
let field_handle = builder.add_fielddef(typedef_handle, "potency", type_sig_blob, field_flags);
let method_handle = builder.add_methoddef(typedef_handle, "use", sig_blob, method_flags);

// Pass 2: finalize assigns row indices
let token_map = builder.finalize(); // DefId -> MetadataToken
```

### Pattern 2: MetadataToken Encoding
**What:** u32 where bits 31-24 = table ID (0-20), bits 23-0 = row index (1-based; 0 = null token).
**When to use:** For all inter-table references.
**Example:**
```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MetadataToken(pub u32);

impl MetadataToken {
    pub const NULL: MetadataToken = MetadataToken(0);

    pub fn new(table: TableId, row: u32) -> Self {
        debug_assert!(row > 0 && row <= 0x00FF_FFFF);
        MetadataToken((table as u32) << 24 | row)
    }

    pub fn table(self) -> TableId { /* bits 31-24 */ }
    pub fn row(self) -> u32 { /* bits 23-0 */ }
}

#[repr(u8)]
pub enum TableId {
    ModuleDef = 0, ModuleRef = 1, TypeDef = 2, TypeRef = 3,
    TypeSpec = 4, FieldDef = 5, FieldRef = 6, MethodDef = 7,
    MethodRef = 8, ParamDef = 9, ContractDef = 10, ContractMethod = 11,
    ImplDef = 12, GenericParam = 13, GenericConstraint = 14,
    GlobalDef = 15, ExternDef = 16, ComponentSlot = 17,
    LocaleDef = 18, ExportDef = 19, AttributeDef = 20,
}
```

### Pattern 3: List Ownership (Spec Section 2.16.5)
**What:** Parent table entries (TypeDef, ContractDef) use `xxx_list` fields pointing to the first child row. The range extends to the next parent's `xxx_list` value. This means children must be contiguous in their table.
**When to use:** TypeDef.field_list/method_list, ContractDef.method_list/generic_param_list, MethodDef implicit param_list.
**Implementation:** Sort children by parent before assigning row indices. Each parent's xxx_list = first child's row index. If no children, xxx_list = next parent's xxx_list (or end of table).

### Pattern 4: Hook Kind Flag Encoding
**What:** MethodDef.flags u16 field encodes hook_kind in a sub-field.
**Values:** 0=none, 1=create, 2=destroy, 3=finalize, 4=serialize, 5=deserialize, 6=interact.
**Implementation:** When walking struct/entity AST members and finding `on create { body }`, the corresponding MethodDef gets hook_kind=1 in its flags.

### Anti-Patterns to Avoid
- **Assigning tokens during collection:** Don't assign row indices in Pass 1. The list-ownership pattern requires children to be grouped contiguously under parents, so row indices can only be assigned after all rows are collected.
- **Walking impl blocks for slot ordering:** CALL_VIRT slots must come from contract declaration order, not impl traversal. Never derive slot indices from ImplDef methods.
- **Mutating TypedAst:** The TypedAst is immutable. All token mappings go in ModuleBuilder or a side HashMap<DefId, MetadataToken>.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| String dedup in heaps | Manual hash tracking | FxHashMap<String, u32> (offset) | Already proven pattern in TyInterner |
| Type signature encoding | Ad-hoc byte arrays | Dedicated TypeSigBuilder | TypeRef blobs have recursive structure (2.15.3) that needs a builder |

## Common Pitfalls

### Pitfall 1: List Ownership Ordering
**What goes wrong:** If FieldDef rows for different TypeDefs are interleaved, the list-ownership pattern breaks — a TypeDef's field_list would span rows belonging to other types.
**Why it happens:** Collecting fields in source order across all types produces interleaved rows.
**How to avoid:** Group all children by parent TypeDef before assigning row indices. Sort the collected rows so all fields of TypeDef A come before all fields of TypeDef B.
**Warning signs:** Tests show field_list indices that don't form contiguous ranges.

### Pitfall 2: Contract Slot vs Method Order
**What goes wrong:** CALL_VIRT dispatch uses wrong slot index because slots were assigned from impl block method order rather than contract declaration order.
**Why it happens:** It's natural to assign slots while walking impl methods, but contracts define the canonical slot ordering.
**How to avoid:** Walk ContractDef members first, assign slot indices 0..N, then ImplDef methods are mapped to slots by name matching against the contract.
**Warning signs:** Test with reordered impl methods produces different slot indices.

### Pitfall 3: Missing writ-runtime ModuleRef
**What goes wrong:** Types like Option, Result, Array that live in writ-runtime cannot be referenced without a ModuleRef row.
**Why it happens:** Forgetting to emit the implicit writ-runtime dependency.
**How to avoid:** Always emit ModuleRef row 1 for writ-runtime with name "writ-runtime" and min_version "1.0.0".
**Warning signs:** TypeRef for Option/Result/Array has no valid scope token.

### Pitfall 4: Off-by-One in Row Indices
**What goes wrong:** MetadataToken uses 1-based row indices (0 = null token), but Vec indices are 0-based.
**Why it happens:** Natural programming in Rust uses 0-based indexing.
**How to avoid:** MetadataToken::new always adds 1 to the Vec index. Add debug_assert!(row > 0) in token construction.
**Warning signs:** Token row index 0 appears in non-null positions.

### Pitfall 5: Locale Key Collection Gap
**What goes wrong:** LocaleDef rows need to reference dialogue MethodDef tokens, but localization keys are generated during lowering (Phase 20), not during emit.
**Why it happens:** The lowering phase generates loc keys as string literals embedded in function call args. By the time we reach emit, these keys are buried in TypedExpr trees.
**How to avoid:** Either (a) track loc keys in a side table during lowering that persists to emit, or (b) scan TypedAst for dialogue-related patterns during emit collection. Option (a) is cleaner — add a `loc_keys: Vec<LocKeyEntry>` to the lowering context that survives into the TypedAst or DefMap.
**Warning signs:** LocaleDef table is empty despite dialogue source code present.

## Code Examples

### ModuleBuilder Core Structure
```rust
pub struct ModuleBuilder {
    // Heaps
    pub string_heap: StringHeap,
    pub blob_heap: BlobHeap,

    // Table rows (collected, not yet assigned indices)
    pub module_def: Option<ModuleDefRow>,
    pub module_refs: Vec<ModuleRefRow>,
    pub type_defs: Vec<TypeDefRow>,
    pub type_refs: Vec<TypeRefRow>,
    pub type_specs: Vec<TypeSpecRow>,
    pub field_defs: Vec<FieldDefRow>,
    pub field_refs: Vec<FieldRefRow>,
    pub method_defs: Vec<MethodDefRow>,
    pub method_refs: Vec<MethodRefRow>,
    pub param_defs: Vec<ParamDefRow>,
    pub contract_defs: Vec<ContractDefRow>,
    pub contract_methods: Vec<ContractMethodRow>,
    pub impl_defs: Vec<ImplDefRow>,
    pub generic_params: Vec<GenericParamRow>,
    pub generic_constraints: Vec<GenericConstraintRow>,
    pub global_defs: Vec<GlobalDefRow>,
    pub extern_defs: Vec<ExternDefRow>,
    pub component_slots: Vec<ComponentSlotRow>,
    pub locale_defs: Vec<LocaleDefRow>,
    pub export_defs: Vec<ExportDefRow>,
    pub attribute_defs: Vec<AttributeDefRow>,

    // Mappings
    pub def_token_map: FxHashMap<DefId, MetadataToken>,
}
```

### Emit Entry Point Pattern (following resolve/check convention)
```rust
pub fn emit(typed_ast: &TypedAst, asts: &[(FileId, &Ast)]) -> (ModuleBuilder, Vec<Diagnostic>) {
    let mut builder = ModuleBuilder::new();
    let mut diags = Vec::new();

    // Pass 1: Collect all definitions
    collect::collect_defs(&typed_ast, asts, &mut builder, &mut diags);

    // Pass 1.5: Assign vtable slots from contract declaration order
    slots::assign_vtable_slots(&mut builder);

    // Pass 2: Finalize row indices and cross-references
    builder.finalize();

    (builder, diags)
}
```

## Open Questions

1. **LocaleDef population strategy**
   - What we know: Localization keys are generated during dialogue lowering (Phase 20). They appear as string literals in the lowered AST. The LocaleDef table needs (dlg_method token, locale string, loc_method token).
   - What's unclear: How to connect lowered loc keys back to specific dialogue method DefIds. The lowering context doesn't persist a loc key manifest.
   - Recommendation: Add a `loc_keys: Vec<(String, String)>` field to the LoweringContext or TypedAst that maps (dlg_fn_name, loc_key) pairs. In Phase 24, emit LocaleDef rows from this manifest. If the manifest doesn't exist yet, Phase 24 should create a minimal stub and add a TODO for Phase 25 to handle multi-locale dispatch.

2. **TypeRef/MethodRef/FieldRef for writ-runtime types**
   - What we know: Phase 25 will need TypeRef rows for Option, Result, Array etc. These reference the writ-runtime ModuleRef.
   - What's unclear: Whether Phase 24 should pre-emit all known writ-runtime TypeRefs or let Phase 25 emit them on demand.
   - Recommendation: Emit the ModuleRef for writ-runtime in Phase 24. Defer TypeRef/MethodRef/FieldRef rows to Phase 25 when they're actually needed for instruction operands. This keeps Phase 24 focused on local definitions.

3. **ComponentSlot override blob encoding**
   - What we know: ComponentSlot rows link entity to component. Override values exist in AST as AstExpr.
   - What's unclear: The exact blob format for override data.
   - Recommendation: In Phase 24, emit ComponentSlot rows with empty override blobs (blob heap offset 0). Phase 25 can fill in actual override data when emitting entity construction sequences.

## Sources

### Primary (HIGH confidence)
- IL spec section 2.16 (Module Format) — read directly from language-spec/spec/45_2_16_il_module_format.md
- IL spec section 2.15 (IL Type System) — read directly from language-spec/spec/44_2_15_il_type_system.md
- Existing codebase patterns — DefMap, TyInterner, TypedAst, TypeEnv

### Secondary (MEDIUM confidence)
- Dialogue lowering code (writ-compiler/src/lower/dialogue.rs) — loc key generation patterns

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no external dependencies, internal compiler module
- Architecture: HIGH - two-pass strategy is locked decision, module structure follows established codebase patterns
- Pitfalls: HIGH - derived from direct spec reading and codebase analysis

**Research date:** 2026-03-03
**Valid until:** 2026-04-03 (stable — internal compiler, no external dependency churn)
