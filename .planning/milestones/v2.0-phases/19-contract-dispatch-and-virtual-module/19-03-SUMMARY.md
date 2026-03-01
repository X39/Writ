# Plan 19-03 Summary: Dispatch Table and CALL_VIRT Wiring

## What was done
1. **DispatchTable** (`dispatch.rs`): HashMap-based O(1) lookup using `DispatchKey { type_key, contract_key, slot }` where keys encode `(module_idx << 16) | row_idx`. `DispatchTarget` is either `Intrinsic(IntrinsicId)` for native primitive operations or `Method { module_idx, method_idx }` for IL methods.

2. **IntrinsicId enum**: 36 variants covering all primitive contract implementations (Int: 13, Float: 10, Bool: 3, String: 6, Array: 4).

3. **Domain::build_dispatch_table()**: Iterates all ImplDef rows across all loaded modules. Resolves type/contract tokens to global keys, bounds method ranges by contract method count, and maps intrinsic-flagged methods to IntrinsicId via `resolve_intrinsic_id()`.

4. **CALL_VIRT handler**: Extracts runtime type from `r_obj`, resolves contract_key from `contract_idx` MetadataToken, looks up `(type_key, contract_key, slot)` in dispatch table, and dispatches to intrinsic execution or IL method call. Missing implementations produce a clear crash message.

5. **execute_intrinsic()**: Implements all 36 intrinsic operations inline (arithmetic, comparison, conversion, string operations, array indexing). Binary operators read self from `r_obj` and argument from `r_base+1`; unary operators read self from `r_obj`.

6. **Runtime migration to Domain**: Runtime now holds `Domain`, `DispatchTable`, and `user_module_idx` instead of a single `LoadedModule`. `RuntimeBuilder::build()` creates Domain with virtual module at index 0 and user module at index 1, resolves refs, and builds dispatch table.

7. **ResolvedContract**: New resolution type for TypeRefs that resolve to ContractDefs (not TypeDefs). Added `contracts` map to `ResolvedRefs` and `find_contract_def_by_name()` helper. This enables user modules to reference contracts like "Add", "Eq" via TypeRef for CALL_VIRT dispatch.

8. **Execution signature update**: `execute_one`, `execute_ret`, `execute_crash`, `execute_defer_handler` all now take `modules: &[LoadedModule]`, `current_module_idx`, and `dispatch_table: &DispatchTable` instead of a single module reference.

9. **5 CALL_VIRT integration tests**: Int+Int via Add, Float*Float via Mul, Bool==Bool via Eq, invalid dispatch (Bool:Neg) crashes gracefully, user-defined contract populates dispatch table.

10. **6 dispatch table unit tests**: Virtual module entry count (32 unique, 36 minus 4 generic collisions), Int:Add intrinsic lookup, Bool:Eq intrinsic lookup, nonexistent key returns None, user impl produces Method target, all intrinsic types covered.

## Known limitations
- Generic contract specializations collide in the dispatch table: when a type implements the same contract with different generic parameters (e.g., Int: Into<Float> vs Int: Into<String>), only the last-registered implementation is stored. Full generic dispatch requires a future phase.
- 32 unique dispatch entries from the virtual module instead of 36 due to 4 collisions (Int:Into x2, Float:Into x2, String:Index x2, Array:Index x2).

## Test results
- 107 unit tests (13 domain + 6 dispatch table + 14 virtual module + 74 existing)
- 9 GC integration tests (unchanged)
- 26 task integration tests (unchanged)
- 77 VM instruction tests (72 existing + 5 new CALL_VIRT)
- **219 total, 0 failures, 0 warnings**

## Deviations from plan
- Added `ResolvedContract` type and `contracts` map to `ResolvedRefs` to handle TypeRefs that resolve to ContractDefs. The original plan assumed TypeRef resolution would only target TypeDefs, but CALL_VIRT's contract_idx can be a TypeRef pointing to a contract name.
- `resolve_contract_key_from_idx` was simplified to use pre-resolved contracts map instead of runtime name-based lookup.
- Generic specialization collisions accepted as known limitation rather than implementing ImplDef-based keys (which would break virtual dispatch semantics).

## Files modified
- `writ-runtime/src/dispatch.rs` (+585 lines: DispatchTable, IntrinsicId, CALL_VIRT handler, intrinsic execution, type/contract key resolution)
- `writ-runtime/src/domain.rs` (+472 lines: build_dispatch_table, ResolvedContract, contract resolution, dispatch table tests)
- `writ-runtime/src/runtime.rs` (rewritten: Domain-based Runtime, RuntimeBuilder with virtual module)
- `writ-runtime/src/scheduler.rs` (updated signatures: modules/dispatch_table threading)
- `writ-runtime/tests/vm_tests.rs` (+212 lines: 5 CALL_VIRT integration tests)
