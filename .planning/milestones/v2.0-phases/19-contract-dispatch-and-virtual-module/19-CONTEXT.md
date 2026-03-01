# Phase 19: Contract Dispatch and Virtual Module - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the CALL_VIRT dispatch table (O(1) contract method resolution via (type, contract, slot) lookup built at module load time) and provide the `writ-runtime` virtual module in memory — containing all 17 contracts, Option/Result/Range types, primitive pseudo-TypeDefs, Array<T>, and Entity base type — without reading any file from disk. Cross-module TypeRef/MethodRef/FieldRef resolution must work across loaded modules.

</domain>

<decisions>
## Implementation Decisions

### All Areas — Spec Authority

The user has directed that all implementation decisions follow the Writ IL spec exactly. No user-specific preferences beyond spec compliance.

Key spec sections governing this phase:
- §2.18 (`writ-runtime` module contents) — types, contracts, primitives, arrays, entity base
- §3.7 (Calls) — CALL_VIRT encoding: `(r_dst, r_obj, contract_idx, slot, r_base, argc)`
- §2.15 (Type system) — primitive tags, TypeRef blobs, type encoding
- §2.4 (Metadata tables) — ImplDefRow, ContractDefRow, TypeDefRow, FieldRefRow, MethodRef structure
- §2.18.5 (Primitive contract implementations) — intrinsic instruction mapping for boxed primitives

### Claude's Discretion

All implementation details are at Claude's discretion as long as spec compliance is maintained:
- Dispatch table data structure (HashMap, flat array, etc.) — must achieve O(1) or amortized O(1) lookup
- Virtual module construction timing and API
- Cross-module resolution strategy and error reporting format
- Internal organization of intrinsic method routing
- How primitive type tags map to pseudo-TypeDef dispatch entries
- Test structure and coverage approach

</decisions>

<specifics>
## Specific Ideas

No specific requirements — strict spec adherence is the only constraint.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `LoadedModule` (`loader.rs`): Module with decoded instruction bodies — needs extension for dispatch table and cross-module refs
- `ImplDefRow`, `ContractDefRow`, `TypeDefRow` (`writ-module/tables.rs`): Metadata row types already defined
- `MetadataToken` (`writ-module`): 1-based indexing newtype for table references
- `dispatch.rs:execute_one()`: Main dispatch loop — CALL_VIRT stub at line 423 needs real implementation
- `RuntimeHost` trait, `GcHeap` trait: Existing abstractions the dispatch system must integrate with

### Established Patterns
- `LoadedModule::from_module()` does decode + reindex in loader.rs — dispatch table build should follow similar pattern
- `execute_one()` match arms handle each instruction — CALL_VIRT arm will need access to dispatch table
- Module uses string heap offsets for names — name-based resolution will need string heap lookups

### Integration Points
- `Runtime`/`RuntimeBuilder` (`runtime.rs`): Will need to hold the virtual module and dispatch tables
- `execute_one()` signature: Currently takes `&LoadedModule` singular — may need multi-module access for cross-module calls
- `Value` enum (`value.rs`): HeapRef values carry type information needed for dispatch

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 19-contract-dispatch-and-virtual-module*
*Context gathered: 2026-03-02*
