# Phase 107: Dynamic Invocation - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — runtime dynamic field write and method invoke)

<domain>
## Phase Boundary

Implement FieldInfo.set() for dynamic field writes (with let-field immutability enforcement) and MethodInfo.invoke() for dynamic method invocation (using current task stack, cooperative scheduling). Satisfies DYN-01 through DYN-04.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — infrastructure phase. Key decisions locked:
- FieldInfo.set() on let-field crashes task with "Reflection write to immutable field '{name}'"
- FieldInfo.set() on mut-field writes the new value
- MethodInfo.invoke() uses the current task stack (not a new task)
- MethodInfo.invoke() participates in cooperative scheduling correctly
- FieldDef.flags readonly bit: 0x01 = readonly (let field). is_mutable = (flags & 0x01) == 0

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- FieldInfoGet intrinsic already reads fields dynamically (Phase 103)
- ReflectionIndex with field/method caches
- Existing IntrinsicId::FieldInfoGet dispatch pattern
- FieldDef.flags in writ-module/src/tables.rs

### Integration Points
- writ-runtime/src/dispatch/intrinsics.rs — FieldInfoSet, MethodInfoInvoke dispatch
- writ-runtime/src/dispatch/mod.rs — new IntrinsicId variants
- writ-runtime/src/reflection.rs — helper methods for field/method metadata lookup

</code_context>

<specifics>
## Specific Ideas

- Verify FieldDef.flags readonly bit exists before implementing FieldInfo.set() mutability check
- Confirm exact ExecContext/call-stack shape for MethodInfo.invoke() frame construction

</specifics>

<deferred>
## Deferred Ideas

None.

</deferred>
