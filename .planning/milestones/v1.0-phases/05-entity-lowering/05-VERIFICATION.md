---
phase: 05-entity-lowering
verified: 2026-02-27T09:00:00Z
status: passed
score: 7/7 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 5/7
  gaps_closed:
    - "entity Name { ... } lowers to AstDecl::Struct + AstDecl::Impl blocks (no AstDecl::Entity in the output)"
    - "[Singleton] attribute propagates to the generated AstStructDecl"
    - "Duplicate use clauses, duplicate property names, unknown lifecycle events, and property-component collisions emit LoweringError without halting"
  gaps_remaining: []
  regressions: []
---

# Phase 05: Entity Lowering Verification Report

**Phase Goal:** Entity declarations are fully lowered to a struct, per-component ComponentAccess<T> impls, and lifecycle hook registrations.
**Verified:** 2026-02-27T09:00:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure via plan 05-03 (parser trailing-comma defect fix + three snapshot re-acceptances)

## Summary of Gap Closure

The previous verification (score 5/7) identified three gaps, all caused by a single root defect: `entity_property` in `writ-parser/src/parser.rs` line 2583 used `just(Token::Comma)` (required trailing comma), causing any entity whose last member is a property to fail parsing and fall through to expression-statement error recovery.

Plan 05-03 applied the one-line fix (`just(Token::Comma).or_not()`) and re-accepted three broken snapshots. Commit `e6c90a8` contains all four changed files.

All three gaps are now closed. The test suite is stable: 177 parser tests pass, 62 compiler lowering tests pass, zero regressions.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `entity Name { ... }` lowers to `AstDecl::Struct` + `AstDecl::Impl` blocks (no `AstDecl::Entity` in output) | VERIFIED | `entity_property_fields` snapshot now shows `Ast { items: [Struct(AstStructDecl { name: "Guard", fields: [name: string, level: int = 1] })] }` — parser produces EntityDecl, `lower_entity()` produces correct Struct. `entity_full_declaration` snapshot shows Struct + inherent Impl + ComponentAccess Impl + OnCreate Impl — all four emission categories confirmed. |
| 2 | `use Component { field: val }` produces a `$ComponentName` struct field with `StructLit` initializer containing only specified overrides | VERIFIED | Confirmed by `entity_component_use_clause`, `entity_empty_use_clause`, `entity_multiple_use_clauses`, `entity_component_partial_override`, `entity_component_no_override`, and `entity_full_declaration` snapshots. `$Health` naming, empty fields vec, partial override all confirmed. |
| 3 | `on create/interact/destroy` hooks lower to contract impl blocks (`OnCreate`/`OnInteract`/`OnDestroy`) | VERIFIED | Confirmed by `entity_lifecycle_on_create`, `entity_lifecycle_on_interact_with_params`, `entity_lifecycle_on_destroy` snapshots — all produce correct `AstDecl::Impl` with correct contract name, method name, and lowered body/params. |
| 4 | `[Singleton]` attribute propagates to the generated `AstStructDecl` | VERIFIED | `entity_singleton_attribute` snapshot now shows `Ast { items: [Struct(AstStructDecl { attrs: [AstAttribute { name: "Singleton" }], name: "Narrator", fields: [name: string] })] }`. The `[Singleton]` attribute is present in `AstStructDecl.attrs`. Previously showed array-literal fallback; now shows correct struct output. |
| 5 | Duplicate use clauses, duplicate property names, unknown lifecycle events, and property-component collisions emit `LoweringError` without halting | VERIFIED | All four error variants confirmed: `DuplicateUseClause` (entity_duplicate_use_clause_error), `DuplicateProperty` (entity_duplicate_property_error — now shows `[DuplicateProperty { property: "name", entity: "Guard" }]` instead of `[]`), `UnknownLifecycleEvent` (entity_unknown_lifecycle_event_error), `PropertyComponentCollision` (entity_property_component_collision_error). All emit errors and do not halt. |
| 6 | Entity methods go in a separate inherent impl block (`contract: None`); empty methods list emits no inherent impl | VERIFIED | Confirmed by `entity_methods_inherent_impl` snapshot: `AstImplDecl { contract: None }` contains the `greet` method. `entity_lifecycle_on_create` (no methods) confirms no inherent impl is emitted. |
| 7 | Entity lowering works in both top-level `lower()` and `lower_namespace()` (both `todo!` arms replaced) | VERIFIED | Both `Item::Entity` arms in `lower()` (line 114) and in `lower_namespace()` (line 336) call `decls.extend(lower_entity(...))`. Zero `todo!` instances in `lower/mod.rs`. `cargo check` passes clean. |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-parser/src/parser.rs` | `entity_property` parser with optional trailing comma | VERIFIED | Line 2583: `.then_ignore(just(Token::Comma).or_not())`. Confirmed by direct read and by grep. Matches the pattern used by `entity_use` at line 2545. Commit `e6c90a8` contains the one-line change. |
| `writ-compiler/src/lower/entity.rs` | `lower_entity() -> Vec<AstDecl>` and `partition_entity_members()` | VERIFIED | File exists, 351 lines. `pub fn lower_entity(entity: EntityDecl<'_>, entity_span: SimpleSpan, ctx: &mut LoweringContext) -> Vec<AstDecl>` confirmed. All 4 validation checks (DuplicateUseClause, DuplicateProperty, UnknownLifecycleEvent, PropertyComponentCollision) present. |
| `writ-compiler/src/lower/mod.rs` | `Item::Entity` wiring in both `lower()` and `lower_namespace()` | VERIFIED | Line 113-115 and line 335-337 both call `decls.extend(lower_entity(...))`. `pub mod entity;` at line 9. `lower_attrs` and `lower_struct_field` are `pub(crate)`. |
| `writ-compiler/src/lower/error.rs` | Entity-specific error variants: `DuplicateUseClause`, `DuplicateProperty`, `UnknownLifecycleEvent`, `PropertyComponentCollision` | VERIFIED | All four variants present at lines 32-57 with correct field names, types, and `#[error(...)]` messages. |
| `writ-compiler/tests/snapshots/lowering_tests__entity_property_fields.snap` | Corrected snapshot showing `AstDecl::Struct` with two property fields | VERIFIED | Snapshot shows `Struct(AstStructDecl { name: "Guard", fields: [name: string, level: int = 1] })`. No `Stmt`/`Expr` fallback content. |
| `writ-compiler/tests/snapshots/lowering_tests__entity_singleton_attribute.snap` | Corrected snapshot showing `AstDecl::Struct` with `[Singleton]` in attrs | VERIFIED | Snapshot shows `Struct(AstStructDecl { attrs: [AstAttribute { name: "Singleton" }], name: "Narrator", fields: [name: string] })`. No array-literal fallback content. |
| `writ-compiler/tests/snapshots/lowering_tests__entity_duplicate_property_error.snap` | Corrected snapshot showing `(Struct, [DuplicateProperty error])` | VERIFIED | Snapshot shows `(Ast { items: [Struct(AstStructDecl { name: "Guard" })] }, [DuplicateProperty { property: "name", entity: "Guard" }])`. The error list is non-empty. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `writ-parser/src/parser.rs` line 2583 | `writ-compiler/src/lower/entity.rs` | `just(Token::Comma).or_not()` makes property-only entities parse as EntityDecl which reaches `lower_entity()` | WIRED | Pattern `just(Token::Comma).or_not()` confirmed at parser.rs:2583. `entity_property_fields` snapshot shows `AstDecl::Struct` output, proving the entity reached `lower_entity()`. |
| `writ-compiler/src/lower/mod.rs` | `writ-compiler/src/lower/entity.rs` | `Item::Entity` arm calls `lower_entity()` with `decls.extend()` | WIRED | `decls.extend(lower_entity(` confirmed at lines 114 and 336. |
| `writ-compiler/src/lower/entity.rs` | `writ-compiler/src/lower/mod.rs` | `use super::lower_fn, lower_param, lower_vis, lower_attrs` | WIRED | `use super::{lower_fn, lower_param, lower_vis, lower_attrs}` at line 17. All four helpers are `pub(crate)` in mod.rs. |
| `writ-compiler/src/lower/entity.rs` | `writ-compiler/src/ast/expr.rs` | `AstExpr::StructLit` for component field default initializers | WIRED | `AstExpr::StructLit { ... }` at line 224-230 of entity.rs. `entity_full_declaration` snapshot shows `StructLit { name: "Health", fields: [(current, 80), (max, 80)] }`. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| R12 | 05-01, 05-02, 05-03 | Entity Lowering: `entity Name { ... }` to struct + impls; use clause, lifecycle hooks, [Singleton], member partitioning | VERIFIED | All acceptance criteria satisfied: property fields lower to AstStructField (entity_property_fields), [Singleton] propagates to attrs (entity_singleton_attribute), DuplicateProperty emitted for duplicate names (entity_duplicate_property_error), lifecycle hooks produce contract impls (entity_lifecycle_* snapshots), methods produce inherent impl (entity_methods_inherent_impl). |
| R13 | 05-01, 05-02 | Component Field Flattening: `use Health { current: 80 }` → `$Health: Health` field with StructLit initializer; partial overrides; non-overridden fields absent | VERIFIED | All R13 acceptance criteria confirmed: `$Health` naming, StructLit initializer with overrides, partial override (only `current: 50`, no `max`), no-override (empty fields vec) — all confirmed across entity_component_* and entity_full_declaration snapshots. |

**Orphaned requirements check:** No additional requirements mapped to Phase 5 in REQUIREMENTS.md beyond R12 and R13.

**Note on R13 naming:** REQUIREMENTS.md specifies `_health: Health` (underscore prefix) but the implementation uses `$Health` (dollar prefix). The plan explicitly specifies `$ComponentName` naming. This is a deliberate design decision documented in the plan ("Dollar-prefix ($ComponentName) for user-unreachable component struct fields") and consistent across all code and snapshots. Not a defect.

### Anti-Patterns Found

None. The three previously-flagged blocker anti-patterns (broken snapshots recording parser-fallback output) are all resolved:

- `entity_property_fields.snap` — now shows `AstDecl::Struct` with two `AstStructField` entries
- `entity_singleton_attribute.snap` — now shows `AstDecl::Struct` with `attrs: [AstAttribute { name: "Singleton" }]`
- `entity_duplicate_property_error.snap` — now shows `(Struct, [DuplicateProperty { property: "name" }])`

No TODO/FIXME/placeholder patterns found in entity.rs or the corrected snapshot files.

### Human Verification Required

None — all items are verified programmatically.

### Re-verification: Gap Resolution Detail

**Gap 1 (Truth #1, previously PARTIAL):** "entity Name { ... } lowers to AstDecl::Struct + AstDecl::Impl blocks"

- Root cause was: `just(Token::Comma)` at parser.rs:2583 requiring trailing comma on every entity property.
- Fix applied: `just(Token::Comma).or_not()` at parser.rs:2583 (commit e6c90a8).
- Verified closed: `entity_property_fields` snapshot shows `Struct(AstStructDecl { name: "Guard", fields: [name: string, level: int = 1] })` — no fallback content.

**Gap 2 (Truth #4, previously FAILED):** "[Singleton] attribute propagates to the generated AstStructDecl"

- Root cause was: same trailing-comma defect prevented `[Singleton] entity Narrator { name: string }` from being parsed as an EntityDecl.
- Fix applied: same one-line parser fix.
- Verified closed: `entity_singleton_attribute` snapshot shows `attrs: [AstAttribute { name: "Singleton" }]` in the emitted `AstStructDecl`.

**Gap 3 (Truth #5, previously PARTIAL):** "DuplicateProperty emits LoweringError"

- Root cause was: `entity Guard { name: string, name: int }` failed to parse as EntityDecl (last property `name: int` lacked trailing comma before `}`), so DuplicateProperty code path in `partition_entity_members` was never reached.
- Fix applied: same one-line parser fix.
- Verified closed: `entity_duplicate_property_error` snapshot shows `[DuplicateProperty { property: "name", entity: "Guard", span: 29..33 }]` in the errors list.

### Test Suite Results

| Suite | Tests | Pass | Fail | Regressions |
|-------|-------|------|------|-------------|
| `writ-parser` | 177 | 177 | 0 | 0 |
| `writ-compiler` | 62 | 62 | 0 | 0 |

Both suites run without `INSTA_UPDATE` — snapshots are stable.

---

_Verified: 2026-02-27T09:00:00Z_
_Verifier: Claude (gsd-verifier)_
