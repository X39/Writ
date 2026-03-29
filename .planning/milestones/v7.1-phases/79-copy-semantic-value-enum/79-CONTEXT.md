# Phase 79: Copy-Semantic Value Enum - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase makes Value derive Copy by replacing Value::InlineStruct (which contains Vec<Value>) with Value::Struct(HeapRef) that stores struct fields on the GC heap. All match sites are updated. GC tracing updated to follow Struct(HeapRef). exec_get_field/exec_set_field read/write through HeapRef. Target: fib(40) under 30s.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- writ-runtime/src/value.rs contains the Value enum
- writ-runtime/src/gc.rs contains the GC heap
- dispatch handlers match on Value variants

### Established Patterns
- Value currently has InlineStruct variant with Vec<Value>
- GC heap already manages heap-allocated objects
- RegisterPool from Phase 77, execute_batch from Phase 78

### Integration Points
- value.rs Value enum (add Copy derive, replace InlineStruct with Struct(HeapRef))
- gc.rs (allocate struct fields on heap, trace Struct(HeapRef) in collect_value_refs)
- dispatch/helpers.rs (get_field, set_field through HeapRef)
- All match sites on Value across the workspace

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase

</specifics>

<deferred>
## Deferred Ideas

None

</deferred>
