---
phase: 13-lowering-entity-model-and-misc
status: passed
verified: 2026-03-01
verifier: orchestrator
score: 5/5
---

# Phase 13: Lowering — Entity Model and Misc — Verification

## Phase Goal

Entity lowering handles all six lifecycle hooks, models components as host-managed slots (not inline struct fields), injects implicit self into operator and hook methods, emits a distinct `AstDecl::Entity` variant, and the `IndexSet` contract name is corrected.

## Success Criteria Verification

### 1. Entity lowering produces hook registrations for all six hooks: `create`, `destroy`, `interact`, `finalize`, `serialize`, and `deserialize` — no hook is silently skipped

**Status: PASS**

- `partition_entity_members()` in `lower/entity.rs` updated to recognize all six lifecycle hooks: `create`, `destroy`, `interact`, `finalize`, `serialize`, `deserialize`
- Each hook produces an `AstEntityHook` with the corresponding contract name and method declaration
- Previously only create/finalize/serialize/deserialize were recognized; destroy and interact were silently dropped
- Plan 13-01 evidence: `partition_entity_members` accepts 6 hooks in entity.rs; AstEntityHook types added
- Plan 13-02 tests: `entity_lifecycle_on_finalize`, `entity_lifecycle_on_serialize`, `entity_lifecycle_on_deserialize` (3 individual hook tests); `entity_all_six_hooks` (verifies none are skipped) — 4 snapshot tests pass

### 2. Entity component declarations lower to host-managed component slot descriptors rather than inline struct field declarations

**Status: PASS**

- `AstComponentSlot` type added to AST: carries component name and overrides vector
- Component lowering changed from `$ComponentName` struct field generation to `AstComponentSlot` descriptors
- `ComponentAccess<T>` impl generation removed — components are host-managed; codegen will emit `GET_COMPONENT` directly
- Plan 13-01 evidence: `AstComponentSlot` in ast/decl.rs; ComponentAccess removed from entity.rs
- Plan 13-02 tests: `entity_component_slot_model` (with overrides), `entity_component_slot_no_overrides`, `entity_multiple_component_slots` — 3 snapshot tests pass confirming slot descriptor structure

### 3. Operator methods and lifecycle hook methods on entities receive an implicit `self` or `mut self` parameter injected by the lowering pass

**Status: PASS**

- Implicit `mut self` injected as first param in all 6 lifecycle hook methods (per spec §14.6, including serialize)
- Implicit `self`/`mut self` injected into operator methods via `op_to_contract_impl` in `lower/operator.rs`
- Operator self mutability: `IndexSet` gets `mut self` (modifies container), all other operators get immutable `self`
- Plan 13-01 evidence: implicit self injection in entity.rs and operator.rs; mut self for hooks/IndexSet
- Plan 13-02 tests: `entity_hook_implicit_mut_self` (hook gets mut self); `operator_implicit_self_immutable` (non-IndexSet op); `operator_implicit_mut_self_index_set` (IndexSet op) — 3 snapshot tests pass

### 4. The lowered output for an entity declaration uses `AstDecl::Entity` rather than reusing `AstDecl::Struct`

**Status: PASS**

- `AstDecl::Entity` variant with `AstEntityDecl` struct added to `ast/decl.rs`
- `AstEntityDecl` carries properties, component_slots, hooks, and inherent_impl as separate structured fields
- Entity lowering entirely rewritten in `lower/entity.rs` to emit `AstDecl::Entity` instead of the old `AstDecl::Struct + Impl chain` approach
- 28 existing lowering snapshots updated to reflect new Entity variant
- Plan 13-01 evidence: `AstDecl::Entity` variant in decl.rs; rewritten `lower_entity` in entity.rs; 28 snapshots updated
- Plan 13-02 test: `entity_full_with_component_slots_and_all_hooks` — full entity test confirms AstDecl::Entity structure in snapshot

### 5. The `op_symbol_to_contract` mapping emits `"IndexSet"` for index-assignment operations, not `"IndexMut"`

**Status: PASS**

- `op_symbol_to_contract` mapping in `lower/operator.rs` corrected: `"IndexMut"` -> `"IndexSet"`
- Plan 13-01 evidence: IndexSet fix in operator.rs `op_symbol_to_contract` function
- Plan 13-02 test: `operator_index_set_contract_name` — snapshot confirms emitted contract name is `"IndexSet"` not `"IndexMut"`

## Requirement Coverage

All 5 phase requirements accounted for:

| Requirement | Plan    | Status   | Evidence |
|-------------|---------|----------|----------|
| ENT-01      | 13-01   | Verified | `partition_entity_members` accepts all 6 hooks; 3 individual tests + all-six-hooks test pass |
| ENT-02      | 13-01   | Verified | `AstComponentSlot` descriptors; ComponentAccess removed; 3 component slot snapshot tests pass |
| ENT-03      | 13-01   | Verified | Implicit mut self in hooks; implicit self/mut-self in operators; 3 implicit self snapshot tests pass |
| ENT-04      | 13-01   | Verified | `AstDecl::Entity` variant added; entity lowering rewritten; full entity snapshot test passes |
| MISC-01     | 13-01   | Verified | `op_symbol_to_contract` corrected to `"IndexSet"`; contract name snapshot test passes |

## Test Results

```
cargo test --workspace: 437 passed, 0 failed
  109 lowering tests
  13 unit tests (string_utils)
  74 lexer tests
  239 parser tests
  2 doc tests
```

New tests added: 12 lowering snapshot tests (Plan 13-02)
- ENT-01: 3 individual hook tests + 1 all-six-hooks test
- ENT-02: 3 component slot tests
- ENT-03: 3 implicit self tests (1 hook, 1 operator, 1 IndexSet)
- ENT-04: 1 full entity test (combined)
- MISC-01: 1 IndexSet contract name test

28 existing snapshot files updated to reflect new AstDecl::Entity structure (Plan 13-01).

## Gaps Found

None.

---
*Phase: 13-lowering-entity-model-and-misc*
*Verified: 2026-03-01*
