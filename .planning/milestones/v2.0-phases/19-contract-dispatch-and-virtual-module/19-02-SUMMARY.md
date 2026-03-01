# Plan 19-02 Summary: Cross-Module Resolution Infrastructure

## What was done
1. **Domain struct** (`domain.rs`, 591 lines): Multi-module container holding all loaded modules. Convention: virtual module at index 0, user module at index 1+.

2. **ResolvedRefs**: Per-module resolution results stored in `LoadedModule::resolved_refs`. Contains `types`, `methods`, and `fields` HashMaps mapping 0-based TypeRef/MethodRef/FieldRef row indices to resolved (module_idx, target_idx) pairs.

3. **Domain::resolve_refs()**: Resolves all cross-module references across all loaded modules at load time. For each module, every TypeRef is resolved by name against the target module's type_defs, every MethodRef is resolved by method name within the type's method range, and every FieldRef by field name within the type's field range.

4. **Name-matching helpers**: `find_module_by_name()`, `find_type_def_by_name()`, `find_method_in_type()`, `find_field_in_type()` -- all use string heap reads to match by (namespace, name).

5. **resolve_parent_type()**: Maps parent MetadataToken (table 2 = TypeDef, table 3 = TypeRef) to (module_idx, typedef_idx) pair, using already-resolved TypeRef results for cross-module parents.

6. **LoadedModule extension**: Added `resolved_refs: ResolvedRefs` field with `Default::default()` initialization.

7. **13 unit tests**: domain creation, TypeRef/MethodRef/FieldRef resolution, unresolvable reference error messages, virtual module types resolvable from user modules, local typedef MethodRef resolution.

## Test results
- 13 new domain unit tests
- All existing tests pass unchanged
- **203 total, 0 failures, 0 warnings**

## Files modified
- `writ-runtime/src/domain.rs` (new, 591 lines)
- `writ-runtime/src/lib.rs` (added `pub mod domain;`)
- `writ-runtime/src/loader.rs` (added `resolved_refs` field to `LoadedModule`)
