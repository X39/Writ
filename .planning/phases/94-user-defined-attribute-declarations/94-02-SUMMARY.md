---
phase: 94-user-defined-attribute-declarations
plan: 02
subsystem: compiler-pipeline
tags: [resolver, type-checker, emitter, attribute-system, virtual-module]
dependency_graph:
  requires: [94-01, 93-blob-encoding-foundation]
  provides: [attribute-decl-binary-emission, builtin-attribute-reservation]
  affects: [writ-diagnostics, writ-module, writ-compiler, writ-runtime]
tech_stack:
  added: []
  patterns: [guard-before-insert, post-finalize-collection, virtual-module-section]
key_files:
  created: []
  modified:
    - writ-diagnostics/src/code.rs
    - writ-compiler/src/resolve/prelude.rs
    - writ-compiler/src/resolve/error.rs
    - writ-compiler/src/resolve/collector.rs
    - writ-compiler/src/check/check_decl.rs
    - writ-module/src/tables.rs
    - writ-compiler/src/emit/collect/mod.rs
    - writ-compiler/src/emit/collect/encoding.rs
    - writ-runtime/src/virtual_module.rs
    - writ-compiler/tests/resolve_tests.rs
    - writ-compiler/tests/emit_body_tests.rs
decisions:
  - E0008 reuses the ResolutionError From-impl pattern exactly matching PreludeShadow (E0002)
  - Param type validation uses E0006 (invalid attribute target) as the error code — no new code added since the plan did not define one; the check verifies string/int/bool only via AstType::Named match
  - collect_attribute_decl_defs encodes param signature as u16 count + tag bytes, always interning even a zero-param sig (2-byte count)
  - virtual_module uses inline block scope to build sig Vec before passing &[u8] to add_attribute_def
metrics:
  duration_minutes: 12
  completed_date: "2026-03-27"
  tasks_completed: 2
  files_modified: 11
---

# Phase 94 Plan 02: Attribute Declaration Binary Emission Summary

**One-liner:** Attribute declarations emit AttributeDef rows (owner_kind=3) in the binary module, builtin names (Deprecated/Conditional/Singleton/Locale) are reserved in the resolver with E0008, param types validated to string/int/bool, and four builtin rows registered in the virtual module.

## What Was Built

**Task 1 — Builtin name reservation, param type validation, E0008:**

- Added `pub const E0008: &str = "E0008"` (builtin attribute shadow) to `writ-diagnostics/src/code.rs`
- Added `BUILTIN_ATTRIBUTE_NAMES` constant and `is_builtin_attribute_name` predicate to `writ-compiler/src/resolve/prelude.rs`
- Added `BuiltinAttributeShadow { name, file, span }` variant to `ResolutionError` with a `From<ResolutionError> for Diagnostic` arm using E0008 (format mirrors PreludeShadow arm)
- Modified the `AstDecl::Attribute` arm in `collector.rs`: checks `is_builtin_attribute_name` before `try_insert`; emits E0008 and `continue`s (skipping insertion) if reserved
- Added `check_attribute_decl` in `check_decl.rs`: validates each param type is `string`, `int`, or `bool` via `AstType::Named` match; emits E0006 with message "attribute parameters must be `string`, `int`, or `bool`"
- Added two resolve tests: `attribute_decl_collected` (Quest collects as AttributeDef), `builtin_attribute_shadow` (Deprecated produces E0008 and is absent from DefMap)

**Task 2 — Emitter AttributeDef rows and virtual module builtins:**

- Added `pub const ATTR_OWNER_KIND_DECL: u8 = 3` near `AttributeDefRow` in `writ-module/src/tables.rs`
- Added `collect_attribute_decl_defs` to `encoding.rs`: iterates `TypedDecl::AttributeDef` entries, finds the matching `AstAttributeDecl` by (name, name_span, file_id), encodes param type signature (u16 count + tag bytes), interns blob, calls `builder.add_attribute_def(MetadataToken::NULL, ATTR_OWNER_KIND_DECL, name, blob_offset)`
- Wired `collect_attribute_decl_defs` into `collect_post_finalize` in `mod.rs`, after `collect_attributes`
- Added Section 7 to `build_writ_runtime_module()` in `virtual_module.rs`: four `add_attribute_def` calls for Deprecated (1 string), Conditional (1 string), Singleton (0 params), Locale (1 string) — all with `ATTR_OWNER_KIND_DECL`
- Added `attribute_decl_emits_def_row` emit test: compiles `pub attribute Quest(name: string, level: int);`, reads the binary module, asserts AttributeDef row with name "Quest", owner_kind=3, value!=0

## Commits

| Task | Name | Commit |
|------|------|--------|
| 1 | Builtin attribute name reservation, param type validation, E0008 error | 29dcba7 |
| 2 | Emitter AttributeDef rows for user-defined attrs and virtual module builtins | 7746115 |

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None. All acceptance criteria satisfied:
- E0008 is produced for any of the four builtin attribute names when declared as user-defined
- Attribute parameter types validated to string/int/bool in check_decl
- User-defined attributes emit AttributeDef rows with owner_kind=3 in the binary module
- Virtual module contains four builtin AttributeDef declaration rows

## Self-Check: PASSED
