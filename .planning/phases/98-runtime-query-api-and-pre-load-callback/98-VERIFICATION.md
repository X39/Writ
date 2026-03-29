---
phase: 98-runtime-query-api-and-pre-load-callback
verified: 2026-03-27T22:00:00Z
status: passed
score: 9/9 must-haves verified
re_verification: false
---

# Phase 98: Runtime Query API and Pre-Load Callback Verification Report

**Phase Goal:** The host can inspect all attribute data on a loaded module before any code executes, query attributes by name or by type, and reject modules that do not meet attribute-based requirements
**Verified:** 2026-03-27
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | `on_module_load` fires after binary parse but before `Domain::add_module` for the user module | VERIFIED | `runtime.rs` lines 128-136: view created from `self.module`, callback fired, then `domain.add_module(self.module)` at line 138. Integration test `on_module_load_fires_for_user_module` passes. |
| 2  | Host returning `Err` from `on_module_load` causes `RuntimeBuilder::build` to return `Err(RuntimeError::LoadError(...))` containing the reason string | VERIFIED | `runtime.rs` line 131-135: `Err(reason)` maps to `RuntimeError::LoadError(format!("module rejected by host: {}", reason))`. Integration test `on_module_load_rejection_prevents_loading` asserts both `"module rejected by host"` and `"bad module"` in error string. |
| 3  | `on_module_load` does NOT fire for virtual module or library modules | VERIFIED | `runtime.rs`: hook inserted after library loop (lines 124-126), before user module add (line 138). No hook call for virtual or library paths. Test `on_module_load_fires_for_user_module` records call count == 1 even though virtual module is always loaded. |
| 4  | `ModuleAttributeView` provides read-only attribute inspection with no side effects | VERIFIED | `host.rs` lines 184-289: `ModuleAttributeView<'a>` holds `module: &'a writ_module::Module` (immutable borrow). All methods return owned `Vec<AttributeMatch>` or `Option<Vec<AttrValue>>`. No mutation methods. |
| 5  | Existing `RuntimeHost` implementors compile without changes (default method) | VERIFIED | `host.rs` line 157-159: `fn on_module_load` has default body `Ok(())`. `NullHost` impl has no `on_module_load` override and compiles. All 90 writ-runtime tests pass. |
| 6  | `domain.query_attributes("Quest")` returns all application rows tagged Quest with decoded arguments, across all loaded modules | VERIFIED | `domain.rs` lines 389-414: iterates all modules with `enumerate()`, filters `ATTR_OWNER_KIND_DECL`, matches name. Test `domain_query_attributes_by_name` asserts len==1 with `AttrValue::String("Chapter1")`. |
| 7  | `domain.query_attributes_on(module_idx, typedef_idx)` returns all attributes on that specific type | VERIFIED | `domain.rs` lines 421-457: bounds-checks `module_idx`, converts `typedef_idx` to 1-based, filters by `TableId::TypeDef` and `row_index`. Test `domain_query_attributes_on_typedef` asserts correct result; `domain_query_attributes_on_wrong_typedef` asserts empty. |
| 8  | `domain.query_attribute_value(module_idx, owner_token, "level")` returns decoded args for that specific attribute on that definition | VERIFIED | `domain.rs` lines 463-482: bounds-checks `module_idx`, finds first matching row by owner token + name + non-DECL filter, decodes args. Tests `domain_query_attribute_value_found` and `domain_query_attribute_value_not_found` both pass. |
| 9  | Declaration rows (`owner_kind==3`) are never included in query results | VERIFIED | All three query methods in `domain.rs` filter `row.owner_kind == ATTR_OWNER_KIND_DECL` before collecting. All three `ModuleAttributeView` methods likewise filter in `host.rs`. Test `domain_query_attributes_excludes_declarations` and `on_module_load_allows_attribute_query` both confirm exactly 1 result when 1 application + 1 declaration row exist. |

**Score:** 9/9 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-runtime/src/host.rs` | `ModuleAttributeView` struct, `AttributeMatch` struct, `on_module_load` default method | VERIFIED | All three present, substantive, and used. Lines 162-289. |
| `writ-runtime/src/runtime.rs` | Pre-load hook call in `RuntimeBuilder::build` before `domain.add_module` | VERIFIED | Lines 127-136. Scoped block creates view, calls `self.host.on_module_load(&view)`, returns `LoadError` on rejection, then adds module at line 138. |
| `writ-runtime/src/domain.rs` | Domain query methods: `query_attributes`, `query_attributes_on`, `query_attribute_value` | VERIFIED | Lines 383-500. All three methods present, substantive (not stubs), filtering DECL rows. `DomainAttributeMatch` struct at line 369. |
| `writ-runtime/src/lib.rs` | Re-exports for `ModuleAttributeView`, `AttributeMatch`, `DomainAttributeMatch` | VERIFIED | Line 44: `pub use host::{..., ModuleAttributeView, AttributeMatch}`. Line 46: `pub use domain::{..., DomainAttributeMatch}`. |
| `writ-runtime/tests/attr_query_tests.rs` | Integration tests for pre-load callback and Domain query methods | VERIFIED | 12 tests: 5 covering QAPI-04/05/06 (pre-load) and 7 covering QAPI-01/02/03 (domain query). All pass. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-runtime/src/runtime.rs` | `writ-runtime/src/host.rs` | `self.host.on_module_load(&view)` | WIRED | Pattern present at line 130 in `RuntimeBuilder::build`. |
| `writ-runtime/src/host.rs` | `writ-module/src/attr.rs` | `decode_attr_args` for query results | WIRED | `host.rs` line 284: `writ_module::attr::decode_attr_args(blob).unwrap_or_default()`. |
| `writ-runtime/src/domain.rs` | `writ-module/src/attr.rs` | `decode_attr_args` for query results | WIRED | `domain.rs` line 497: `writ_module::attr::decode_attr_args(blob).unwrap_or_default()`. |
| `writ-runtime/src/domain.rs` | `writ-module/src/heap.rs` | `read_string` and `read_blob` for name and blob access | WIRED | `domain.rs` line 11: `use writ_module::heap::read_string`; line 399: `writ_module::heap::read_string(...)`, line 496: `writ_module::heap::read_blob(...)`. |

### Data-Flow Trace (Level 4)

The artifacts in this phase are query APIs (library-style, not UI components). They accept input data and return owned collections. Data flows from the module's `attribute_defs` table through filtering and decoding to the caller. No rendering layer; Level 4 trace is not applicable.

The integration tests prove real data flows end-to-end: a `ModuleBuilder` constructs a module with an encoded `AttrValue::String("Chapter1")` blob, the Runtime loads it, and query methods decode and return that exact value — confirming real data, not static returns.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All 12 integration tests pass | `cargo test --package writ-runtime --test attr_query_tests` | `12 passed; 0 failed` | PASS |
| No regressions in writ-runtime | `cargo test --package writ-runtime` | `90 passed; 0 failed` | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| QAPI-01 | 98-02 | `query_attributes(name)` returning all matching declarations with decoded arguments | SATISFIED | `Domain::query_attributes` in `domain.rs` lines 389-414. Tests `domain_query_attributes_by_name`, `domain_query_attributes_excludes_declarations`, `domain_query_attributes_no_match` all pass. |
| QAPI-02 | 98-02 | `query_attributes_on(type_id)` returning all attributes on a specific type | SATISFIED | `Domain::query_attributes_on` in `domain.rs` lines 421-457. Tests `domain_query_attributes_on_typedef`, `domain_query_attributes_on_wrong_typedef` pass. |
| QAPI-03 | 98-02 | `query_attribute_value(def, name)` returning decoded args for a specific attribute on a specific definition | SATISFIED | `Domain::query_attribute_value` in `domain.rs` lines 463-482. Tests `domain_query_attribute_value_found`, `domain_query_attribute_value_not_found` pass. |
| QAPI-04 | 98-01 | Pre-load callback that fires before any module code executes, giving the host full attribute inspection | SATISFIED | `on_module_load` in `RuntimeHost` trait (`host.rs` line 157). Wired in `RuntimeBuilder::build` at lines 127-136. Test `on_module_load_fires_for_user_module` proves call_count == 1. |
| QAPI-05 | 98-01 | Pre-load callback returns allow/reject decision; rejected modules are not loaded | SATISFIED | `Err(reason)` from `on_module_load` returns `RuntimeError::LoadError`. Test `on_module_load_rejection_prevents_loading` confirms build fails with correct message. |
| QAPI-06 | 98-01 | No attribute causes automatic instantiation or invocation — host must explicitly act on query results | SATISFIED | `ModuleAttributeView` is read-only (`&'a Module`), all methods return owned data. No side-effect methods exist. Query results require explicit host action. Test `on_module_load_allows_attribute_query` confirms data returned, no side effects triggered. |

No orphaned requirements found. All six QAPI requirements from REQUIREMENTS.md are claimed by the two plans and verified by implementation evidence.

### Anti-Patterns Found

No anti-patterns found. Scan of `host.rs`, `runtime.rs`, `domain.rs`, and `attr_query_tests.rs` found no TODO/FIXME markers in the new query code, no empty implementations returning `null`/`[]`/`{}` in live paths, and no placeholder comments.

Note: `runtime.rs` line 589 has a `TODO` for finalizer task scheduling in `collect_garbage`, but that is pre-existing code unrelated to this phase.

### Human Verification Required

None. All phase behaviors are verifiable programmatically through the integration test suite and code inspection.

### Gaps Summary

No gaps. All nine observable truths are verified, all five artifacts are present and substantive, all four key links are wired, all six requirements are satisfied, and 90 test suite passes with zero regressions.

---

_Verified: 2026-03-27T22:00:00Z_
_Verifier: Claude (gsd-verifier)_
