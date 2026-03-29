---
phase: 106-read-only-introspection-integration-tests-and-lsp
plan: "02"
subsystem: compiler + LSP
tags: [golden-tests, lsp, typeof, reflection, type-checker, unification]
dependency_graph:
  requires: [104-writ-compiler-typeof-lowering-and-tykind, 105-writ-compiler-reflectable-auto-impl-emission]
  provides: [golden regression anchors for TYPEOF IL emission, LSP hover for typeof, LSP type diagnostics for reflection types]
  affects: [writ-golden, writ-compiler, writ-lsp]
tech_stack:
  added: []
  patterns: [golden test bless pattern, LSP unit test with typed AST helper]
key_files:
  created:
    - writ-golden/tests/golden/refl_typeof_basic.writ
    - writ-golden/tests/golden/refl_typeof_basic.writil
    - writ-golden/tests/golden/refl_typeof_equality.writ
    - writ-golden/tests/golden/refl_typeof_equality.writil
  modified:
    - writ-golden/tests/golden_tests.rs
    - writ-compiler/src/check/check_expr/mod.rs
    - writ-compiler/src/check/unify.rs
    - writ-lsp/src/queries/hover.rs
decisions:
  - "typeof(TypeName) in source: TypeOf arm in check_expr resolves bare Ident that names a type def (struct/class/entity/enum/contract) directly to its Ty rather than calling check_expr (which would fail with undefined variable)"
  - "ReflectionType unification: typeof(T) == typeof(U) is valid at the type level because both are runtime Type objects; unify.rs now accepts any two ReflectionType(_) as compatible"
  - "TypeOf hover arm: explicit match arm for TypedExpr::TypeOf in hover_text_for_expr produces 'Type', preventing silent regressions if the catch-all changes"
metrics:
  duration: "~11 minutes"
  completed_date: "2026-03-28"
  tasks_completed: 2
  files_modified: 6
  files_created: 4
---

# Phase 106 Plan 02: Golden Tests for typeof Compilation and LSP Hover Summary

Golden compiler pipeline tests for typeof and LSP typeof hover — two golden test pairs (.writ + .writil) lock TYPEOF IL emission, explicit TypeOf hover arm locks LSP-02, and two compiler bug fixes enable typeof(TypeName) to compile correctly.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Add golden test files and test registrations for typeof compilation | efe3f05 |
| 2 | Add explicit TypeOf hover arm and LSP unit tests | 7d0bc6e |

## What Was Built

### Golden Tests (Task 1)

Two new golden test pairs were added to `writ-golden/tests/golden/`:

**refl_typeof_basic**: Compiles `typeof(Point)` inside a function. The blessed `.writil` confirms a `TYPEOF r0, 33554433` instruction is emitted with the correct TypeDef token for `Point`.

**refl_typeof_equality**: Compiles `typeof(Alpha) == typeof(Alpha)` and `typeof(Alpha) == typeof(Beta)`. The blessed `.writil` confirms two separate `TYPEOF` instructions are emitted (not register aliasing), followed by `CMP_EQ_I` — exactly the pattern needed for REFL-09 interning correctness.

Both tests are registered in `golden_tests.rs` under Section O: Reflection golden tests.

### LSP Changes (Task 2)

Added explicit `TypedExpr::TypeOf` match arm to `hover_text_for_expr` in `hover.rs`. The arm returns `"```writ\nType\n```"` — consistent with the locked LSP-02 decision. Added two unit tests:

- `test_hover_typeof_shows_type`: builds typed AST, finds typeof expression by offset, asserts hover contains "Type"
- `test_typeof_type_error_diagnostic`: runs pipeline on `typeof(Foo) + 1`, asserts at least one type error diagnostic is produced (LSP-01)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] typeof(TypeName) failed with "undefined variable"**
- **Found during:** Task 1, first bless attempt
- **Issue:** `check_expr` on `AstExpr::TypeOf` calls `check_expr` on the inner expression. When inner is `AstExpr::Ident { name: "Point" }`, it goes to `check_ident` which only handles `Fn`, `ExternFn`, `Const`, `Global` def kinds — not `Struct`, `Class`, `Entity`, `Enum`, or `Contract`. Fell through to "undefined variable" error.
- **Fix:** In the `AstExpr::TypeOf` arm of `check_expr`, added a special case: when inner expression is an `AstExpr::Ident`, look up the name in the def map and, if it resolves to a type definition (Struct/Class/Entity/Enum/Contract), return the corresponding `Ty` directly instead of calling `check_expr`. Other expression forms fall through to the normal path.
- **Files modified:** `writ-compiler/src/check/check_expr/mod.rs`
- **Commit:** efe3f05

**2. [Rule 1 - Bug] typeof(Alpha) == typeof(Beta) failed with "type mismatch: expected Type, found Type"**
- **Found during:** Task 1, second bless attempt (after fix 1)
- **Issue:** Unification of `ReflectionType(Struct(Alpha_id))` with `ReflectionType(Struct(Beta_id))` failed because there was no case in `unify.rs` for two `ReflectionType(_)` with different inner types. The error message showed both as "Type" (correct display) but they were different `Ty` IDs.
- **Fix:** Added `(TyKind::ReflectionType(_), TyKind::ReflectionType(_)) => Ok(())` case to `unify.rs`. Any two reflection types are unifiable because at runtime they are both `Type` objects — the inner type is used only by the emitter to select the TypeDef token, not to distinguish runtime compatibility.
- **Files modified:** `writ-compiler/src/check/unify.rs`
- **Commit:** efe3f05

## Verification

```
cargo test -p writ-golden golden_refl_typeof   → 2 passed
cargo test -p writ-golden                      → 50 passed (no regressions)
cargo test -p writ-lsp test_hover_typeof       → 1 passed
cargo test -p writ-lsp test_typeof_type_error  → 1 passed
```

## Known Stubs

None. The golden `.writil` files are fully blessed with real compiler output. The hover arm and diagnostic tests exercise real pipeline paths.

## Self-Check: PASSED

- `writ-golden/tests/golden/refl_typeof_basic.writ` — FOUND
- `writ-golden/tests/golden/refl_typeof_basic.writil` — FOUND (contains TYPEOF)
- `writ-golden/tests/golden/refl_typeof_equality.writ` — FOUND
- `writ-golden/tests/golden/refl_typeof_equality.writil` — FOUND (contains TYPEOF + CMP_EQ_I)
- Commit efe3f05 — FOUND (feat(106-02): add golden tests for typeof compilation...)
- Commit 7d0bc6e — FOUND (feat(106-02): add explicit TypeOf hover arm...)
