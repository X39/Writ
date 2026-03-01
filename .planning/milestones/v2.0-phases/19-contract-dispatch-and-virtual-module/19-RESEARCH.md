# Phase 19: Contract Dispatch and Virtual Module - Research

**Researched:** 2026-03-02
**Domain:** Rust runtime internals / spec-compliant dispatch tables
**Confidence:** HIGH

## Summary

Phase 19 builds three tightly coupled systems: (1) a dispatch table that maps `(type_tag, contract_id, slot)` to concrete method implementations for O(1) CALL_VIRT resolution, (2) the `writ-runtime` virtual module providing all 17 contracts, Option/Result/Range types, primitive pseudo-TypeDefs, Array, and Entity base type in memory without a file on disk, and (3) cross-module name resolution for TypeRef/MethodRef/FieldRef across loaded modules.

The existing codebase has a clean stub at `dispatch.rs:423` (`Instruction::CallVirt { .. } => Crash("CALL_VIRT: contract dispatch not available")`) and the runtime currently loads a single module. Phase 19 extends this to multi-module with a domain-level dispatch table and virtual module.

**Primary recommendation:** Build the virtual module first (it populates the metadata tables that the dispatch table consumes), then build the dispatch table builder that processes ImplDef rows from all loaded modules, then implement cross-module resolution and wire CALL_VIRT into the dispatch loop. A separate intrinsic dispatch path handles primitive contract methods without IL method bodies.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- All implementation decisions follow the Writ IL spec exactly. No user-specific preferences beyond spec compliance.
- Key spec sections: section 2.18 (writ-runtime contents), section 3.7 (CALL_VIRT encoding), section 2.15 (type system), section 2.4/2.16 (metadata tables, module format), section 2.18.5 (primitive intrinsics).

### Claude's Discretion
- Dispatch table data structure (HashMap, flat array, etc.) -- must achieve O(1) or amortized O(1) lookup
- Virtual module construction timing and API
- Cross-module resolution strategy and error reporting format
- Internal organization of intrinsic method routing
- How primitive type tags map to pseudo-TypeDef dispatch entries
- Test structure and coverage approach

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DISP-01 | CALL_VIRT dispatches through contract table built from ImplDef rows at load time | Dispatch table architecture section; HashMap<(TypeTag, ContractIdx, Slot), DispatchTarget> built during module loading |
| DISP-02 | writ-runtime virtual module provides Option, Result, Range types | Virtual module construction section; TypeDef/FieldDef rows for all three types with spec-mandated tag assignments |
| DISP-03 | writ-runtime virtual module provides all 17 contract definitions | Virtual module construction section; 17 ContractDef rows with ContractMethod slots enumerated |
| DISP-04 | writ-runtime virtual module provides primitive pseudo-TypeDefs (Int/Float/Bool/String) | Virtual module construction section; 4 pseudo-TypeDef rows with ImplDef entries for intrinsic dispatch |
| DISP-05 | writ-runtime virtual module provides Array<T> methods and Entity base type | Virtual module construction section; Array and Entity TypeDef rows with method and contract impl entries |
| DISP-06 | Cross-module name resolution resolves TypeRef/MethodRef/FieldRef across modules | Cross-module resolution section; name-based lookup at load time with error reporting |
</phase_requirements>

## Architecture Patterns

### Pattern 1: Domain-Level Module Registry

The runtime currently holds a single `LoadedModule`. Phase 19 extends this to a `Domain` or module-level registry that holds the virtual module plus all loaded user modules. The dispatch table is domain-wide.

**Current state:**
```rust
pub struct Runtime<H: RuntimeHost = NullHost> {
    pub(crate) loaded_module: LoadedModule,
    // ... scheduler, heap, host
}
```

**After Phase 19:**
```rust
pub struct Runtime<H: RuntimeHost = NullHost> {
    pub(crate) modules: Vec<LoadedModule>,    // index 0 = writ-runtime virtual module
    pub(crate) dispatch_table: DispatchTable,  // domain-wide dispatch
    pub(crate) scheduler: Scheduler,
    pub(crate) heap: Box<dyn GcHeap>,
    pub(crate) host: H,
    pub(crate) next_request_id: u32,
}
```

The `execute_one` function signature changes from `module: &LoadedModule` to something that provides access to the dispatch table and multiple modules (e.g., `domain: &Domain` or the dispatch table + module vec).

### Pattern 2: Dispatch Table as HashMap

**Structure:** `HashMap<(u32, u32, u16), DispatchTarget>` where the key is `(type_def_index, contract_def_index, method_slot)`.

```rust
/// A resolved dispatch target.
pub enum DispatchTarget {
    /// Normal IL method body -- module_idx + method_idx within that module.
    Method { module_idx: usize, method_idx: usize },
    /// Intrinsic -- runtime-provided native implementation.
    Intrinsic(IntrinsicId),
}
```

The dispatch table is built once at domain load time by iterating all ImplDef rows across all loaded modules. Each ImplDef maps a `(type_token, contract_token)` pair to a method list; the builder resolves the tokens to global indices and inserts one entry per contract method slot.

**Why HashMap over flat array:** The type x contract x slot space is sparse (most types implement few contracts). A HashMap provides O(1) amortized lookup without wasting memory on empty slots. For a spec reference implementation, this is the right tradeoff.

### Pattern 3: Virtual Module as In-Memory Module

The `writ-runtime` module is constructed programmatically using the existing `ModuleBuilder` API (or direct `Module` construction). It produces a real `Module` struct with populated metadata tables, which is then loaded via `LoadedModule::from_module()` like any other module. This means the virtual module participates in dispatch table building and cross-module resolution identically to user modules.

**Benefits:**
- Reuses existing module infrastructure
- No special-casing for virtual module in dispatch or resolution
- Tests can inspect the virtual module's tables like any other module

### Pattern 4: Intrinsic Dispatch

Primitive contract implementations (section 2.18.5) don't have IL method bodies -- they map to dedicated instructions. The dispatch table stores `DispatchTarget::Intrinsic(IntrinsicId)` for these entries.

When CALL_VIRT resolves to an intrinsic, the runtime executes the corresponding native operation directly instead of pushing a new call frame.

**Intrinsic enumeration (from spec section 2.18.5):**

**int (13 entries):** Add, Sub, Mul, Div, Mod, Neg, Not, Eq, Ord, BitAnd, BitOr, Into<float>, Into<string>
**float (10 entries):** Add, Sub, Mul, Div, Mod, Neg, Eq, Ord, Into<int>, Into<string>
**bool (3 entries):** Eq, Not, Into<string>
**string (6 entries):** Add<string,string>, Eq, Ord, Index<int,string>, Index<Range<int>,string>, Into<string>
**Array (4 contract impls):** Index<int,T>, IndexSet<int,T>, Index<Range<int>,Array<T>>, Iterable<T>

Total: ~36 intrinsic entries.

### Pattern 5: Cross-Module Resolution

At load time, for each module:
1. For each `TypeRefRow`: resolve `(scope: ModuleRef, namespace, name)` to a TypeDef in the target module by string matching.
2. For each `MethodRefRow`: resolve `(parent type, name, signature)` to a MethodDef in the resolved parent type's method list.
3. For each `FieldRefRow`: resolve `(parent type, name, type_sig)` to a FieldDef in the resolved parent type's field list.

The resolution result is stored in a `ResolvedRef` map per module, allowing instructions to look up the resolved target at runtime.

**Error handling:** Unresolvable references produce a named runtime error at load time (not a panic). The error includes the unresolved name and the target module name.

## Common Pitfalls

### Pitfall 1: Token Space Collision Across Modules
**What goes wrong:** MetadataTokens are module-local (1-based indices into that module's tables). Using a raw token from one module to index into another module's tables produces wrong results.
**How to avoid:** All cross-module references must be resolved to a `(module_idx, local_idx)` pair. The dispatch table key must use globally-unique type identifiers, not raw module-local tokens.

### Pitfall 2: Primitive Type Tags vs TypeDef Tokens
**What goes wrong:** Primitive values (Int/Float/Bool) in registers have no heap object with a type tag. When CALL_VIRT operates on a boxed primitive, the runtime needs to determine the primitive type to look up the correct pseudo-TypeDef.
**How to avoid:** For boxed primitives, the dispatch path inspects the Value variant inside the box: `Value::Int` maps to the Int pseudo-TypeDef, `Value::Float` to Float, etc. The Boxed heap object must preserve the original Value variant for this mapping.

### Pitfall 3: execute_one Signature Change
**What goes wrong:** `execute_one` currently takes a single `&LoadedModule`. Changing this to multi-module access requires updating every call site (scheduler, defer handler, crash propagation).
**How to avoid:** Introduce a `ModuleContext` or similar struct that bundles the dispatch table + module list, minimizing the number of parameter changes. Alternatively, store the dispatch table and modules on the scheduler or a shared context.

### Pitfall 4: Virtual Module String Heap Isolation
**What goes wrong:** The virtual module has its own string heap. Name-based resolution needs to compare strings across modules, which means reading from different string heaps.
**How to avoid:** Resolution code must be explicitly module-aware: read the name from module A's string heap, compare against names read from module B's string heap.

### Pitfall 5: Method List Ownership Pattern
**What goes wrong:** The spec uses a "list ownership" pattern where a TypeDef's `method_list` field gives the index of the first MethodDef row, and the range extends to the next TypeDef's `method_list` value. Same for `field_list`, `method_list` on ContractDef, etc. Off-by-one errors are common.
**How to avoid:** Create utility functions like `methods_of_type(module, type_idx) -> &[MethodDefRow]` that handle the range calculation correctly. Test edge cases (last type in table, single-method types).

## Code Examples

### Dispatch Table Key
```rust
/// Global key for dispatch table lookup.
/// type_key is a globally-unique identifier for a type across all loaded modules.
/// contract_key is a globally-unique identifier for a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DispatchKey {
    pub type_key: u32,      // global type identifier
    pub contract_key: u32,  // global contract identifier
    pub slot: u16,          // method slot within the contract
}
```

### Virtual Module Construction (Sketch)
```rust
pub fn build_writ_runtime_module() -> Module {
    let mut builder = ModuleBuilder::new("writ-runtime", "1.0.0");

    // 17 contracts
    let add_contract = builder.add_contract("Add", "writ");
    builder.add_contract_method(add_contract, "op_add", /* sig */, 0);
    // ... repeat for all 17

    // Core types: Option<T>, Result<T,E>, Range<T>
    let option_type = builder.add_type("Option", "writ", TypeDefKind::Enum);
    // ... with generic params and variants

    // 4 primitive pseudo-TypeDefs
    let int_type = builder.add_type("Int", "writ", TypeDefKind::Struct);
    // ... with ImplDef entries mapping to intrinsic methods

    // Array<T>, Entity base type
    // ...

    builder.build()
}
```

### CALL_VIRT Dispatch (Sketch)
```rust
Instruction::CallVirt { r_dst, r_obj, contract_idx, slot, r_base, argc } => {
    let obj_val = frame.registers[r_obj as usize];
    let type_key = resolve_type_key(obj_val, heap, modules);
    let contract_key = resolve_contract_key(contract_idx, current_module_idx, modules);

    let key = DispatchKey { type_key, contract_key, slot };
    match dispatch_table.get(&key) {
        Some(DispatchTarget::Method { module_idx, method_idx }) => {
            // Push new call frame in target module
        }
        Some(DispatchTarget::Intrinsic(id)) => {
            // Execute intrinsic inline
        }
        None => {
            ExecutionResult::Crash(format!(
                "no implementation of contract {} for type {}",
                contract_name, type_name
            ))
        }
    }
}
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Module construction | Raw Module struct manipulation | Existing ModuleBuilder API | Already handles string heap, blob heap, table indices correctly |
| Instruction decoding | Custom decoder | LoadedModule::from_module() | Already handles decode + reindex in two passes |

## State of the Art

This is an internal spec implementation -- no external dependencies or evolving ecosystem to track. The Rust standard library provides all needed data structures (HashMap, Vec, etc.).

## Open Questions

1. **Hook dispatch wiring:** The STATE.md notes that Phase 18 deferred hook dispatch (on_create, on_destroy, on_interact, on_finalize) to Phase 19 because it needs method resolution. Phase 19 should wire these hooks using the new dispatch infrastructure.
   - What we know: Entity lifecycle hooks are already partially implemented (entity.rs handles state transitions, but hook method lookup is stubbed)
   - Recommendation: Include hook method lookup wiring as part of dispatch table integration

2. **Boxed type tagging for dispatch:** When a boxed value goes through CALL_VIRT, the runtime must determine the concrete type. Currently `HeapObject::Boxed(Value)` stores just the value.
   - What we know: The Value enum already carries type info (Int/Float/Bool/Ref/Entity variants)
   - Recommendation: Use the Value variant discriminant to map to primitive pseudo-TypeDef keys

## Sources

### Primary (HIGH confidence)
- Writ IL Spec section 2.18 (writ-runtime module contents) -- complete manifest of virtual module
- Writ IL Spec section 3.7 (Calls) -- CALL_VIRT encoding and semantics
- Writ IL Spec section 2.15 (IL type system) -- type tags, TypeRef encoding, dispatch guidance
- Writ IL Spec section 2.16 (IL module format) -- metadata tables, token encoding, cross-module refs
- Codebase analysis of dispatch.rs, loader.rs, runtime.rs, tables.rs, value.rs, heap.rs, module.rs

## Metadata

**Confidence breakdown:**
- Dispatch table architecture: HIGH -- spec provides explicit guidance (section 2.15.4)
- Virtual module contents: HIGH -- spec section 2.18 is exhaustive
- Cross-module resolution: HIGH -- spec section 2.16.2 is clear
- Intrinsic enumeration: HIGH -- spec section 2.18.5 lists every entry

**Research date:** 2026-03-02
**Valid until:** N/A (spec-driven, no external dependencies)
