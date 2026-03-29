---
phase: 23-type-checking
plan: 02
status: complete
started: "2026-03-03"
completed: "2026-03-03"
---

# Plan 23-02 Summary: Field Access, Mutability, and Contract Bounds

## What Was Built
Extended `writ-compiler/src/check/` with field access resolution, component access typing, match expression checking, mutability enforcement, and contract bound verification:
- `check_member_access()`: resolves struct/entity fields from `TypeEnv.struct_fields`/`entity_fields`, falls through to `impl_index` for method access, emits `E0106` for unknown fields
- `check_bracket_access()`: handles array indexing (int index, returns elem type) and entity component access (guaranteed vs optional `Option<T>` wrapping)
- `check_match()`: checks scrutinee, binds pattern variables in scoped environments, unifies all arm body types
- `check_pattern()`: handles Wildcard, Variable (binds to scrutinee type), Literal (type-checked against scrutinee), EnumDestructure (resolves variant fields), Or, Range patterns
- `check_assignment_mutability()`: root-binding propagation via `find_root_binding()`, emits `E0108` for immutable reassignment and `E0107` for immutable field mutation with dual-span errors
- `check_contract_bounds()`: after generic type inference, verifies concrete types satisfy contract bounds via `impl_index` lookup, emits `E0103` with help suggestion
- `mutability.rs`: `check_method_mutation()` for `mut self` method calls on immutable bindings
- 21 new integration tests (49 total) covering field access, self typing, match, if/else unification, assignment, mutability, array indexing, void return, chained access

## Key Decisions
- Root-binding propagation walks `Field`/`Index`/`Var`/`SelfRef` chains to find the root `let` binding's mutability
- Component access on concrete entities with `use Component` returns the component type directly (guaranteed); on generic `Entity` wraps in `Option<T>`
- Match pattern variable bindings use the scrutinee type (or variant payload type for enum destructuring)
- Contract bound checking is wired into `check_call_with_sig()` after argument unification resolves InferVars
- `mutability.rs` is a separate module for the `mut self` method mutation check; assignment/field mutation checks are inline in `check_expr.rs` to avoid borrow conflicts with `CheckCtx`

## Commits
- (pending commit) feat(23-02): add field access, mutability enforcement, match checking, and contract bounds

## Self-Check: PASSED
- [x] All 49 typecheck tests pass
- [x] Full workspace tests pass (no regressions)
- [x] `s.field` on struct/entity resolves to declared field type
- [x] `s.unknown` produces E0106 unknown field error
- [x] `self` in method body resolves to enclosing type
- [x] `self` outside method produces E0102
- [x] Match arms with mismatched types produce E0100
- [x] Match variable binding gets scrutinee type
- [x] If/else branch types unified; mismatch produces E0100
- [x] `let x = 1; x = 2;` produces E0108 immutable reassignment
- [x] `let mut x = 1; x = 2;` no error
- [x] `s.x = 42` on immutable binding produces E0107
- [x] Array bracket access with non-int index produces E0100
- [x] Void function with return value produces E0100
- [x] Chained field access (o.inner.val) resolves correctly
- [x] Contract bound checking wired into generic call resolution

## Key Files
### Created
- `writ-compiler/src/check/mutability.rs` - Mutability enforcement, `mut self` method mutation checking

### Modified
- `writ-compiler/src/check/check_expr.rs` - Added `check_member_access`, `check_bracket_access`, `check_match`, `check_pattern`, `check_assignment_mutability`, `find_root_binding`, `check_contract_bounds`
- `writ-compiler/src/check/mod.rs` - Added `pub mod mutability;`
- `writ-compiler/tests/typecheck_tests.rs` - 21 new tests (49 total)
