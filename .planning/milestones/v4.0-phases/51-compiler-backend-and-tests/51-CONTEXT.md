# Phase 51: Compiler Backend and Tests - Context

**Gathered:** 2026-03-13
**Status:** Ready for planning

<domain>
## Phase Boundary

The compiler emits correct structural equality for value-type structs, migrates closure capture environments to class kind, and all golden tests pass with format_version=3 and updated kind values. Entity kind stays unchanged (kind=2). Covers COMP-05, COMP-07, TEST-01 through TEST-04. COMP-06 is dropped (entities remain kind=2).

</domain>

<decisions>
## Implementation Decisions

### Structural Equality Semantics
- Recursive field-by-field comparison for value-type structs — if struct A contains struct B, compare B's fields recursively
- Reference-type fields (class, entity, delegate, array) compared by reference identity (same heap object = equal)
- Primitive fields compared with CmpEqI/CmpEqF as appropriate
- All value-type structs are automatically equatable — no opt-in required, no restrictions based on field types
- == between different struct types is a compile-time error (type checker rejects)
- != gets its own field-by-field emission with early exit on first mismatch (not derived from == + NOT)
- Inline vs synthetic equality method: Claude's discretion

### Entity Kind Migration
- Entities remain kind=2 (Entity) in emitted IL — NOT changed to kind=4 (Class)
- COMP-06 requirement dropped: entities are conceptually specialized classes but keep distinct TypeDef kind for runtime identification
- VM already treats kind=2 and kind=4 identically for allocation (both heap-allocate via `alloc_struct`)
- Entity-specific features (SPAWN_ENTITY, component slots, lifecycle hooks) continue to key off kind=2

### Closure Capture Migration
- Closure capture environments change from kind=0 (Struct) to kind=4 (Class)
- Captures are heap-allocated reference objects — kind=4 is semantically correct
- Change location: `writ-compiler/src/emit/body/closure.rs` line ~81, `TypeDefKind::Struct` → `TypeDefKind::Class`

### Golden Test Coverage
- Comprehensive coverage: multiple .writ files for struct, class, entity, closures, recursive struct error
- IL snapshot verification: compile .writ → .writil, disassemble, snapshot text IL (matches existing golden test pattern)
- Separate test files for class (fields, methods, hooks, reference semantics) and entity (components, entity hooks, spawn)
- Existing golden tests bulk re-blessed with spot-check of 3-5 snapshots for expected changes only

### Claude's Discretion
- Inline comparison emission vs synthetic __eq method per struct type
- Exact .writ source file names and organization
- Which 3-5 existing snapshots to spot-check after re-blessing
- Test helper structure for new golden tests
- How structural equality integrates into existing emit/body/expr.rs comparison codegen

</decisions>

<specifics>
## Specific Ideas

- Structural equality rule: primitives by value, nested value-structs recursively, everything else (class, entity, delegate, array) by reference identity
- != should short-circuit on first mismatched field (early exit), not compute full == then negate
- Recursive struct error golden test captures the compile error (TEST-04) — already implemented in Phase 50, just needs a golden test .writ file
- Entity kind=2 stays because the runtime uses it for SPAWN_ENTITY, component access, and lifecycle hook dispatch — fast kind-check is valuable

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `emit/body/expr.rs` lines 540-610: existing comparison codegen (CmpEqI, CmpLtI, etc.) — extend for struct field comparison
- `emit/body/closure.rs` line ~81: `TypeDefKind::Struct` → change to `TypeDefKind::Class`
- `emit/collect.rs` lines 227-269: entity codegen — no changes needed (stays kind=2)
- Golden test infrastructure: insta snapshots in `writ-compiler/tests/snapshots/`, emit_tests.rs, emit_body_tests.rs
- `detect_recursive_structs` DFS already implemented in Phase 50 — golden test just needs a .writ source file

### Established Patterns
- Comparison codegen in expr.rs: match on BinaryOp variant, emit instruction(s), return destination register
- Golden tests use `lower_src()` / `emit_src()` helpers → `insta::assert_debug_snapshot!`
- Closure pre-scan runs before `builder.finalize()` — ordering constraint for any closure TypeDef changes
- `TypeDefKind` enum imported from `writ_module` (unified in Phase 48)

### Integration Points
- `writ-compiler/src/emit/body/expr.rs`: BinaryOp::Eq/NotEq arms — add struct field comparison branch
- `writ-compiler/src/emit/body/closure.rs`: TypeDefKind::Struct → TypeDefKind::Class
- `writ-compiler/src/check/check_decl.rs` or `check_expr.rs`: type-check == on different struct types (compile error)
- `writ-compiler/tests/`: new golden test .writ files and snapshot updates
- `.planning/REQUIREMENTS.md`: update COMP-06 to reflect entity kind=2 decision

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 51-compiler-backend-and-tests*
*Context gathered: 2026-03-13*
