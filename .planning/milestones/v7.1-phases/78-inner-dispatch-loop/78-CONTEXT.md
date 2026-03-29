# Phase 78: Inner Dispatch Loop - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase introduces an execute_batch function that runs multiple instructions per task slice without returning to the outer scheduler loop, eliminating the per-instruction HashMap task lookup. Falls back to single-instruction dispatch when debug hooks are enabled (DAP). Respects ExecutionLimit.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- writ-runtime scheduler.rs run_one_task loop
- dispatch/mod.rs execute_one function
- RegisterPool from Phase 77

### Established Patterns
- FxHashMap for task lookup in scheduler
- ExecContext threading pool, globals, entity_registry
- ExecutionLimit for step-limited execution

### Integration Points
- scheduler.rs run_one_task (outer loop that calls execute_one per instruction)
- dispatch/mod.rs (execute_one returns ExecutionResult)
- DAP debug hooks path

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase

</specifics>

<deferred>
## Deferred Ideas

None

</deferred>
