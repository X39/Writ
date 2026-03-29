---
phase: 86-lsp-support
plan: 01
subsystem: writ-lsp
tags: [lsp, completions, hover, diagnostics, contract-as-type, virtual-dispatch, runtime]
dependency_graph:
  requires:
    - phase: 84-01
      provides: TyKind::Contract, contract-as-type, E0122 removal
    - phase: 84-02
      provides: contract-method-resolution, contract_methods in TypeEnv
    - phase: 85-01
      provides: CALL_VIRT emission for contract-typed receivers
  provides:
    - TyKind::Contract arm in build_dot_completions (LSP-01)
    - DefKind::Contract arm in hover_text_for_def (LSP-02)
    - LSP diagnostics regression tests for contract-typed code (LSP-03)
    - HeapObject::Struct carries type_key for CALL_VIRT dispatch on class instances
    - collect_impl uses ContractDefHandle so ImplDef.contract is not NULL for user contracts
  affects: [v8.0-milestone-complete]
tech-stack:
  added: []
  patterns: [tdd-red-green, deviation-rule-1-auto-fix-bugs]
key-files:
  created: []
  modified:
    - writ-lsp/src/queries/completion.rs
    - writ-lsp/src/queries/hover.rs
    - writ-lsp/tests/test_protocol.rs
    - writ-runtime/src/heap.rs
    - writ-runtime/src/gc.rs
    - writ-runtime/src/dispatch/calls.rs
    - writ-runtime/src/dispatch/objects.rs
    - writ-runtime/src/dispatch/entities.rs
    - writ-compiler/src/emit/collect/contracts.rs
    - writ-compiler/src/emit/collect/mod.rs
key-decisions:
  - "HeapObject::Struct stores type_key=(module_idx<<16)|typedef_idx for class instances; u32::MAX for entity buffers"
  - "collect_impl resolves contract token from contractdef_handles before finalize, not from token_for_def (which is empty pre-finalize)"
  - "alloc_struct takes type_key as first arg; all entity/test callers pass u32::MAX"
patterns-established:
  - "Pattern: ContractDefHandle tracking in collect_defs for pre-finalize token resolution in collect_impl"
  - "Pattern: HeapObject carries type identity for dispatch — class instances embed type_key, value structs carry type_idx in Value::Struct"
requirements-completed: [LSP-01, LSP-02, LSP-03]

duration: 35min
completed: "2026-03-24"
---

# Phase 86 Plan 01: LSP Contract-as-Type Support Summary

**TyKind::Contract dot-completions and DefKind::Contract hover added to LSP, plus two runtime bugs fixed that caused CALL_VIRT to crash on all class-typed contract calls**

## Performance

- **Duration:** ~35 minutes
- **Started:** 2026-03-24T00:00:00Z
- **Completed:** 2026-03-24T00:35:00Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Added `TyKind::Contract` arm to `build_dot_completions` — reads from `type_env.contract_methods`, emits CompletionItem with kind=METHOD per method (LSP-01)
- Added `DefKind::Contract` arm to `hover_text_for_def` — returns `` ```writ\ncontract Name\n``` `` tooltip (LSP-02)
- Added 2 wire-protocol regression tests to `test_protocol.rs` confirming zero diagnostics for valid contract code and error diagnostic for invalid assignment (LSP-03)
- Fixed CALL_VIRT dispatch crash on class instances: `HeapObject::Struct` now carries `type_key` so `resolve_runtime_type_key` can identify the concrete type (Deviation Rule 1)
- Fixed null ImplDef contract token: `collect_impl` now resolves the contract token from `contractdef_handles` before `finalize()` is called (Deviation Rule 1)
- All 5 new tests pass; 20 LSP integration tests and full workspace suite green

## Task Commits

Each task was committed atomically:

1. **Task 1 (RED):** `21f7630` — test(86-01): add failing tests for TyKind::Contract dot-completions and DefKind::Contract hover
2. **Task 1 (GREEN):** `cc16430` — feat(86-01): add TyKind::Contract arm to build_dot_completions and DefKind::Contract arm to hover_text_for_def
3. **Task 2:** `95f85dd` — feat(86-01): add LSP diagnostics regression tests for contract-typed code + fix two runtime bugs

## Files Created/Modified

- `writ-lsp/src/queries/completion.rs` — Added TyKind::Contract arm before catch-all; 2 new unit/integration tests in #[cfg(test)] module
- `writ-lsp/src/queries/hover.rs` — Added DefKind::Contract arm before catch-all; 1 new unit test in #[cfg(test)] module
- `writ-lsp/tests/test_protocol.rs` — Added test_diagnostics_contract_valid_no_errors and test_diagnostics_contract_invalid_produces_error
- `writ-runtime/src/heap.rs` — HeapObject::Struct gains type_key field; alloc_struct(type_key, field_count) signature
- `writ-runtime/src/gc.rs` — Updated GcHeap trait alloc_struct signature; updated test call sites
- `writ-runtime/src/dispatch/calls.rs` — resolve_runtime_type_key handles HeapObject::Struct via stored type_key
- `writ-runtime/src/dispatch/objects.rs` — exec_new for Class kind computes and stores class_type_key in HeapObject::Struct
- `writ-runtime/src/dispatch/entities.rs` — Updated alloc_struct calls to pass u32::MAX for entity data buffers
- `writ-compiler/src/emit/collect/contracts.rs` — collect_contract returns ContractDefHandle; collect_impl accepts contractdef_handles and uses it for pre-finalize token resolution
- `writ-compiler/src/emit/collect/mod.rs` — Tracks contractdef_handles, stores result from collect_contract, passes to collect_impl

## Decisions Made

1. **HeapObject::Struct carries type_key** — class instances now embed their type identity in the heap object so CALL_VIRT can resolve the dispatch table entry without walking the TypeDef table.
2. **u32::MAX sentinel for non-dispatch allocations** — entity data buffers and test allocations don't need virtual dispatch; they pass u32::MAX and resolve_runtime_type_key returns u32::MAX (dispatch miss handled gracefully).
3. **contractdef_handles for pre-finalize resolution** — follows the same pattern as typedef_handles. Contracts appear before impls in TypedDecl order, so the handle is always available when collect_impl runs.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CALL_VIRT crashed on all class-typed contract method calls (type_key=0xffffffff)**
- **Found during:** Task 2 (diagnostics regression test showed R0001 crash on valid contract code)
- **Issue:** `HeapObject::Struct` had no `type_key` field. For class instances (`Value::Ref(href)`), `resolve_runtime_type_key` returned `u32::MAX` because `HeapObject::Struct` only had fields. Dispatch table lookup always missed.
- **Fix:** Added `type_key: u32` to `HeapObject::Struct`; `alloc_struct(type_key, field_count)`; `exec_new` for Class kind computes `(module_idx<<16)|typedef_idx` and stores it; `resolve_runtime_type_key` reads it back.
- **Files modified:** `writ-runtime/src/heap.rs`, `writ-runtime/src/gc.rs`, `writ-runtime/src/dispatch/calls.rs`, `writ-runtime/src/dispatch/objects.rs`, `writ-runtime/src/dispatch/entities.rs`, `writ-runtime/tests/vm_tests.rs`, `writ-dap/src/variables.rs`
- **Verification:** test_diagnostics_contract_valid_no_errors passes; full workspace green
- **Committed in:** `95f85dd` (Task 2 commit)

**2. [Rule 1 - Bug] ImplDef.contract was MetadataToken::NULL for all user-defined contracts**
- **Found during:** Task 2 (even with fix 1, dispatch table still had no entry for the impl)
- **Issue:** `collect_impl` called `builder.token_for_def(contract_def_id)` to get the contract token, but `token_for_def` is only populated after `builder.finalize()`. During collection (before finalize), it always returned None, so `contract_token = MetadataToken::NULL`. The ImplDef carried a null contract, so `build_dispatch_table` skipped it.
- **Fix:** Added `contractdef_handles: FxHashMap<DefId, ContractDefHandle>` tracking in `collect_defs`. `collect_contract` now returns the handle; it's stored in the map. `collect_impl` first checks `contractdef_handles` (for user-defined contracts), then falls back to `token_for_def` (for cross-module contracts from writ-runtime).
- **Files modified:** `writ-compiler/src/emit/collect/contracts.rs`, `writ-compiler/src/emit/collect/mod.rs`
- **Verification:** test_diagnostics_contract_valid_no_errors passes; full workspace green
- **Committed in:** `95f85dd` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 - Bug)
**Impact on plan:** Both bugs were pre-existing in the codebase and directly caused the plan's regression test to fail. Both fixes are correctness-critical: without them, CALL_VIRT on any user-defined contract method crashes at runtime. No scope creep.

## Known Stubs

None — all features fully implemented. Contract-as-type is complete through the full stack: spec, type system, codegen, runtime dispatch, and LSP.

## Self-Check

### Files Exist Check
- writ-lsp/src/queries/completion.rs — exists
- writ-lsp/src/queries/hover.rs — exists
- writ-lsp/tests/test_protocol.rs — exists
- writ-runtime/src/heap.rs — exists
- writ-compiler/src/emit/collect/contracts.rs — exists

### Commits Exist Check
- 21f7630 — test(86-01): add failing tests
- cc16430 — feat(86-01): implement completions and hover
- 95f85dd — feat(86-01): diagnostics tests + runtime bug fixes

## Self-Check: PASSED
