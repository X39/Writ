# Phase 102: writ-runtime Virtual Module Reflection Types - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — plumbing reflection types into virtual module)

<domain>
## Phase Boundary

Add the 6 reflection class TypeDefs (Type, FieldInfo, MethodInfo, ParameterInfo, AttributeInfo, ContractInfo) to the writ-runtime virtual module, register Reflectable as contract 19 with one method slot (get_type), and register primitive get_type intrinsics (IntGetType, FloatGetType, BoolGetType, StringGetType). Satisfies TYPE-01 through TYPE-08.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Key decisions already locked in spec (§2.18.9):
- 6 reflection class TypeDefs with fields per spec
- Reflectable = contract slot 19, single method get_type() -> Type
- Auto-generated ImplDefs for all user-defined types (compiler phase, not this phase)
- Primitive intrinsics: IntGetType, FloatGetType, BoolGetType, StringGetType registered on pseudo-TypeDefs
- Reflection types are classes (heap-allocated, GC-managed)
- Fields on Type: name (string), kind (int), namespace (string)
- Fields on FieldInfo: name (string), declared_type (Type), is_mutable (bool)
- Fields on MethodInfo: name (string), return_type (Type), parameters (Array<ParameterInfo>)
- Fields on ParameterInfo: name (string), parameter_type (Type)
- Fields on AttributeInfo: name (string), values (Array<string>)
- Fields on ContractInfo: name (string), methods (Array<MethodInfo>)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- writ-runtime virtual module builder (existing 17 contracts + entity base type)
- IntrinsicId enum for registering built-in operations
- Domain dispatch table for contract resolution

### Established Patterns
- Virtual module TypeDef registration with field definitions
- Contract slot assignment and dispatch table wiring
- Intrinsic registration for primitive operations

### Integration Points
- `writ-runtime/src/virtual_module.rs` or equivalent — virtual module builder
- `writ-runtime/src/intrinsics.rs` or equivalent — intrinsic dispatch
- `writ-runtime/src/domain.rs` — domain dispatch table

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase. Follow existing virtual module patterns.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
