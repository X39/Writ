---
phase: 26-cli-integration-e2e-validation
plan: 01
subsystem: runtime-bugs
tags: [runtime, dispatch, hooks, generics, cli-host, fix]
dependency_graph:
  requires: []
  provides: [FIX-01, FIX-02, FIX-03]
  affects: [writ-runtime/dispatch, writ-runtime/domain, writ-runtime/host, writ-cli/cli_host]
tech_stack:
  added: []
  patterns:
    - "HOOK_RETURN_SINK sentinel (u16::MAX) prevents hook return from clobbering entity register"
    - "find_hook_by_name scans TypeDef method range by name via string_heap"
    - "DispatchKey.type_args_hash structural extension for future generic specialization"
    - "display_args pre-resolved parallel to args in HostRequest::ExternCall"
key_files:
  created:
    - writ-runtime/tests/hook_dispatch_tests.rs
  modified:
    - writ-runtime/src/dispatch.rs
    - writ-runtime/src/domain.rs
    - writ-runtime/src/host.rs
    - writ-cli/src/cli_host.rs
    - writ-cli/tests/cli_integration.rs
decisions:
  - "type_args_hash=0 in both build_dispatch_table and CALL_VIRT lookup; compiler support needed before field is functional"
  - "HOOK_RETURN_SINK=u16::MAX as return_register sentinel so push_hook_frame never corrupts entity handle in r0"
  - "display_args uses Vec<String> parallel field on HostRequest::ExternCall (not Value::Str variant or heap threading)"
  - "two-phase DESTROY_ENTITY uses begin_destroy before hook push; entity gets type_idx before state transition"
metrics:
  duration: ~35 min
  completed: 2026-03-03
  tasks: 2
  files_changed: 5
requirements-completed: [FIX-01, FIX-03]
---

# Phase 26 Plan 01: Runtime Bug Fixes (FIX-01, FIX-02, FIX-03) Summary

**One-liner:** Lifecycle hook dispatch via find_hook_by_name+push_hook_frame, generic dispatch key type_args_hash extension, and GC heap string pre-resolution for ExternCall display_args.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | FIX-01 lifecycle hook dispatch + FIX-02 generic dispatch key | 8cb6141 | dispatch.rs, domain.rs, hook_dispatch_tests.rs |
| 2 | FIX-03 string dereferencing in CliHost | 1d3e62f | host.rs, dispatch.rs, cli_host.rs, cli_integration.rs |

## What Was Built

### FIX-01: Lifecycle Hook Dispatch

Added `find_hook_by_name(module, type_idx, name) -> Option<usize>` in `dispatch.rs`. Scans the method range for a TypeDef (bounded by the next TypeDef's `method_list` pointer) to find a method by name via `string_heap`.

Added `push_hook_frame(task, hook_method_idx, module, entity_handle)`. Creates a `CallFrame` with `return_register = HOOK_RETURN_SINK (u16::MAX)` to prevent the hook's void return from clobbering the entity handle in `r0` of the caller's frame. Updated `execute_ret` to skip register write when `return_register == u16::MAX`.

- `INIT_ENTITY` handler: After `commit_init()` and host notification, calls `find_hook_by_name(..., "on_create")` and pushes hook frame if found.
- `DESTROY_ENTITY` handler: Gets `type_idx` before `begin_destroy()` changes entity state. After `begin_destroy()`, decrements PC (for re-execution), then calls `find_hook_by_name(..., "on_destroy")` and pushes hook frame if found. On second execution (entity in Destroying state), calls `complete_destroy()` and notifies host.

All 3 hook integration tests pass (`init_entity_dispatches_on_create_hook`, `destroy_entity_dispatches_on_destroy_hook`, `entity_without_hooks_inits_and_destroys_ok`).

### FIX-02: Generic Dispatch Key Extension

Added `type_args_hash: u32` field to `DispatchKey` struct. This structurally supports distinguishing generic specializations (e.g., `Int:Into<Float>` vs `Int:Into<String>`). Added `DispatchTable::get_any()` method for backward-compatible lookups that ignore `type_args_hash`.

Both `build_dispatch_table()` in `domain.rs` and the `CALL_VIRT` handler in `dispatch.rs` use `type_args_hash = 0u32`. Full functional use requires compiler support to emit per-specialization tokens — the field is reserved for that future extension.

Added two regression tests: `two_same_contract_different_token_specializations_produce_two_entries` and `non_generic_impl_still_works_after_fix02`.

### FIX-03: String Dereferencing for ExternCall

Added `display_args: Vec<String>` field to `HostRequest::ExternCall` in `host.rs`. The `CallExtern` handler in `dispatch.rs` pre-resolves each arg before issuing the request:
- `Value::Ref(href)` → `heap.read_string(href).map(|s| s.to_string()).unwrap_or("<ref>")`
- `Value::Int(i)` → `i.to_string()`
- `Value::Float(f)` → `f.to_string()`
- `Value::Bool(b)` → `b.to_string()`
- `Value::Void` → `"void"`
- `Value::Entity(e)` → `"<entity@{index}>"`

`CliHost::on_request` now uses `display_args[0]` for `say()` output with fallback to `format_value()` for backward compat. Removed the "Known Limitation" doc comment since FIX-03 resolves it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] HOOK_RETURN_SINK to prevent entity register corruption**
- **Found during:** Task 1 (GREEN phase debugging)
- **Issue:** `push_hook_frame` used `return_register = 0`, so `execute_ret` wrote `Value::Void` to `caller.registers[0]` (the entity handle). On DESTROY_ENTITY re-execution, `extract_entity(&registers[0])` got `Value::Void`, causing a crash.
- **Fix:** Added `HOOK_RETURN_SINK = u16::MAX` constant. `push_hook_frame` uses it as `return_register`. `execute_ret` skips register write when `return_register == u16::MAX`.
- **Files modified:** dispatch.rs
- **Commit:** 8cb6141

**2. [Rule 1 - Bug] FIX-02 CALL_VIRT dispatch failure (type_args_hash mismatch)**
- **Found during:** Task 1 (GREEN phase)
- **Issue:** Initially set `type_args_hash = impl_def.contract.0` in `build_dispatch_table` and `type_args_hash = contract_idx` in CALL_VIRT handler. These produced different values (ImplDefs store ContractDef tokens, CALL_VIRT carries TypeRef tokens) causing 3 CALL_VIRT tests to fail.
- **Fix:** Changed both to `type_args_hash = 0u32`. Full discrimination deferred until compiler emits matching tokens at both sites.
- **Files modified:** dispatch.rs, domain.rs
- **Commit:** 8cb6141

**3. [Rule 2 - Missing Test Infrastructure] I2s-based string test for FIX-03**
- **Found during:** Task 2 (RED phase)
- **Issue:** `ModuleBuilder` has no `add_string()` method, and `LoadString` instruction requires an offset from the string heap at module build time. For FIX-03 RED test, used `I2s` (int-to-string) instruction instead — produces `Value::Ref` via GC heap without needing pre-interned string in the module's string heap.
- **Fix:** Test uses `LOAD_INT r0 99 + I2S r1 r0 + CALL_EXTERN r2 say(r1) + RET_VOID` to produce a `Value::Ref` for say()'s arg.
- **Files modified:** writ-cli/tests/cli_integration.rs
- **Commit:** 1d3e62f

## Verification

```
cargo test -p writ-runtime  -- 224 tests pass
cargo test -p writ-cli      -- 11 tests pass (including new FIX-03 integration test)
cargo build -p writ-cli     -- no compilation errors
```

## Self-Check

### Files Exist
- [x] writ-runtime/tests/hook_dispatch_tests.rs
- [x] writ-runtime/src/dispatch.rs (modified)
- [x] writ-runtime/src/domain.rs (modified)
- [x] writ-runtime/src/host.rs (modified)
- [x] writ-cli/src/cli_host.rs (modified)
- [x] writ-cli/tests/cli_integration.rs (modified)

### Commits Exist
- [x] 8cb6141 feat(26-01): FIX-01 lifecycle hook dispatch + FIX-02 generic dispatch key
- [x] 1d3e62f feat(26-01): FIX-03 string dereferencing for ExternCall display_args
