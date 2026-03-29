# Phase 103: writ-runtime ReflectionIndex and Intrinsic Dispatch - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — runtime reflection metadata loading and dispatch)

<domain>
## Phase Boundary

Implement the ReflectionIndex (lazy-loaded, GC-rooted cache of Type/FieldInfo/MethodInfo/etc. heap objects), wire all reflection intrinsic dispatch arms (Type.fields(), Type.methods(), Type.attributes(), Type.contracts(), Type.implements(), FieldInfo.get(), etc.), implement TypeOf opcode dispatch returning Type heap objects, and integrate with v10.0 ModuleAttributeView for AttributeInfo population. Satisfies RT-01 through RT-05.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — infrastructure phase. Key decisions locked in STATE.md:
- ReflectionIndex lazy init: must NOT eagerly allocate Type objects for all types at domain load time — only on first access
- GC root strategy: reflection singleton HeapRefs (Type, FieldInfo, MethodInfo) must be registered as permanent GC roots in Runtime::collect_roots() so GC cannot free them
- AttributeInfo uses unified AttributeIndex shared with v10.0 ModuleAttributeView
- any-at-boundaries: compiler auto-inserts BOX/UNBOX coercions at reflection API parameter/return sites — no TyKind::Any needed
- TypeOf dispatch returns a Type heap object for any type index
- Primitive typeof via IntGetType/FloatGetType/BoolGetType/StringGetType intrinsics
- Phase 102 left intrinsic bodies as placeholder Value::Int(1) — this phase replaces with actual Type heap object allocation

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- Phase 102's 4 IntrinsicId variants + dispatch stubs (IntGetType, FloatGetType, BoolGetType, StringGetType)
- Virtual module's 6 reflection TypeDefs (indices 9-14)
- Reflectable contract at 0-based index 18
- Existing GC infrastructure (MarkSweepHeap, GcHeap trait, root collection)
- v10.0 ModuleAttributeView for attribute data

### Established Patterns
- Intrinsic dispatch in dispatch/intrinsics.rs
- Heap object allocation via GcHeap
- Domain-level caching patterns

### Integration Points
- writ-runtime/src/dispatch/mod.rs — TypeOf opcode arm (currently stub)
- writ-runtime/src/dispatch/intrinsics.rs — reflection intrinsic bodies
- writ-runtime/src/ — new ReflectionIndex module
- writ-runtime/src/gc.rs or runtime.rs — GC root registration

</code_context>

<specifics>
## Specific Ideas

- Verify FieldDef.flags readonly bit exists in writ-module/src/tables.rs before implementing FieldInfo.get() mutability check
- Confirm v10.0 ModuleAttributeView surface before implementing AttributeInfo population

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
