# Phase 77: Frame Register Pool - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase introduces a RegisterPool that caches deallocated register Vecs for reuse on the next call, eliminating per-call register Vec allocation for the common case. Pool is capped at 64 entries. execute_ret returns the popped frame's Vec to the pool.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- writ-runtime frame.rs contains CallFrame with registers Vec
- Phase 76 established zero-allocation call convention in calls.rs

### Established Patterns
- Register-based VM, FxHashMap for dispatch/scheduler
- Zero-allocation call convention from Phase 76

### Integration Points
- frame.rs CallFrame creation (acquire from pool)
- dispatch/mod.rs execute_ret (release to pool)
- scheduler.rs or task.rs (pool ownership)

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase

</specifics>

<deferred>
## Deferred Ideas

None

</deferred>
