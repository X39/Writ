# Phase 24: IL Codegen — Metadata Skeleton - Context

**Gathered:** 2026-03-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Populate all 21 metadata tables in a ModuleBuilder with correct token assignment from TypedAst output. Every DefId gets a stable MetadataToken before any method body is emitted. This is the "skeleton" pass — it creates the metadata structure that method body emission (Phase 25) will reference.

Input: TypedAst (from Phase 23 type checker) + DefMap
Output: ModuleBuilder with all Def rows populated, DefId→MetadataToken mapping complete, CALL_VIRT slot numbers fixed

</domain>

<decisions>
## Implementation Decisions

### Token Assignment Strategy
- Two-pass approach: Pass 1 collects all definitions and groups them by table. Pass 2 assigns row indices in table order. No fixup needed — all row indices known before cross-references are written.
- DefId → MetadataToken mapping stored in a separate `HashMap<DefId, MetadataToken>` — TypedAst stays immutable, mapping lives in ModuleBuilder or a side table.

### ModuleBuilder API Surface
- New `emit` module at `writ-compiler/src/emit/` — alongside existing `ast`, `lower`, `resolve`, `check`
- Generic table-level API: ModuleBuilder has methods like `add_typedef(name, kind, flags)` that don't know about TypedAst. A separate pass translates TypedAst → ModuleBuilder calls. Clean separation of concerns.

### Contract Vtable Slot Ordering
- Separate vtable pass: collect all contract info first, then a dedicated pass assigns slot numbers from contract declaration order. Slot assignment is fixed before any method body can emit CALL_VIRT.

### Lifecycle Hook Registration
- Hooks apply to BOTH structs and entities:
  - Universal hooks (structs + entities): on_create, on_finalize, on_serialize, on_deserialize
  - Entity-specific hooks: on_destroy, on_interact
- Hook registration via MethodDef.flags only — set hook_kind bits in the flags field. No separate fast-lookup slots or side tables.
- Absent hooks = simply no MethodDef row with that hook_kind. Runtime detects presence by scanning MethodDefs.

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

</decisions>

<specifics>
## Specific Ideas

- Two-pass collect-then-assign avoids placeholder fixups entirely — all cross-table references can be resolved in the assign pass
- Generic table-level API on ModuleBuilder enables potential reuse for module inspection/verification tooling later
- Spec's list-ownership pattern (TypeDef.field_list / method_list) constrains child row ordering — children must be contiguous under parent

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `DefMap` with arena-allocated `DefId` (`writ-compiler/src/resolve/def_map.rs`): Central symbol table — Phase 24 maps these DefIds to MetadataTokens
- `TypedAst` / `TypedDecl` / `TypedExpr` (`writ-compiler/src/check/ir.rs`): Input to Phase 24 — all types resolved, every node carries `Ty`
- `Ty` / `TyKind` / `TyInterner` (`writ-compiler/src/check/ty.rs`): Interned type system — used to generate TypeRef blobs and FieldDef/MethodDef type signatures
- `DefEntry` / `DefKind` (`writ-compiler/src/resolve/def_map.rs`): Definition metadata (name, kind, visibility, namespace) feeds directly into TypeDef/MethodDef/etc. row construction

### Established Patterns
- Arena-based ID allocation (`id_arena`): DefId pattern can inform MetadataToken allocation approach
- Interning with structural dedup (`TyInterner`): Pattern available for string/blob heap dedup
- Module structure: `ast/`, `lower/`, `resolve/`, `check/` — new `emit/` follows same pattern
- Error handling: `writ-diagnostics` crate with `Diagnostic` type for error reporting

### Integration Points
- Pipeline: `parse()` → `lower()` → `resolve()` → `check()` → **`emit()`** — new emit stage takes TypedAst + DefMap
- `writ-compiler/src/lib.rs`: Will need to export new `emit` module
- Phase 25 (method body emission): Will consume ModuleBuilder + DefId→MetadataToken mapping to emit instructions with correct token references

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 24-il-codegen-metadata-skeleton*
*Context gathered: 2026-03-03*
