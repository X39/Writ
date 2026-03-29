# Phase 76: Zero-Allocation Call Convention - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase eliminates intermediate Vec allocations from all call instructions (exec_call, exec_call_virt, exec_call_indirect, exec_tail_call) by copying arguments directly from caller registers to callee registers.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- writ-runtime dispatch/calls.rs contains the call instruction handlers
- Phase 75 established inline annotations on call handlers

### Established Patterns
- Register-based VM with typed IL
- FxHashMap for dispatch table and scheduler tasks (from Phase 75)

### Integration Points
- dispatch/calls.rs call handlers (exec_call, exec_call_virt, exec_call_indirect, exec_tail_call)
- Frame creation and register allocation in frame.rs

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase

</specifics>

<deferred>
## Deferred Ideas

None

</deferred>
