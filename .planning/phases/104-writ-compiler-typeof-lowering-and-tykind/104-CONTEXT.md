# Phase 104: writ-compiler typeof() Lowering and TyKind - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — compiler pipeline for typeof())

<domain>
## Phase Boundary

Add typeof(expr) support through the full compiler pipeline: parser recognition, AST lowering to AstExpr::TypeOf, type checker TyKind::ReflectionType(Type), IL code generation emitting the TypeOf instruction with compile-time type index, and BOX/UNBOX auto-coercion at reflection API boundaries. Satisfies COMP-01, COMP-02, COMP-04, COMP-05, REFL-01.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — infrastructure phase. Key decisions locked in STATE.md:
- typeof(expr) is a static compile-time query that lowers to AstExpr::TypeOf and emits TYPEOF opcode with type_idx baked in — NOT a function call
- The type checker assigns TyKind::ReflectionType(Type) to typeof expressions
- BOX/UNBOX auto-coercions at reflection API parameter/return sites — no TyKind::Any needed
- typeof(Animal) on a polymorphic Dog variable returns Animal (static type); dog.get_type() returns Dog (dynamic)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- writ-compiler full pipeline: parser → name resolution → type checking → IL codegen
- Existing lowering patterns in writ-compiler/src/lower/
- TypeOf instruction (0x0A30) already in writ-module (Phase 101)
- TyKind enum in type checker

### Established Patterns
- AstExpr variants for expression nodes
- Lowering passes for sugar/builtins
- Type checker unification and TyKind matching
- IL emission patterns in emit/ module

### Integration Points
- writ-compiler/src/lower/ — lowering passes
- writ-compiler/src/resolve/ — name resolution
- writ-compiler/src/check/ — type checker
- writ-compiler/src/emit/ — IL code generation

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase. Follow existing compiler pipeline patterns.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
