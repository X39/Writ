# Phase 80: Clone Cleanup and Micro-Optimizations - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Remove all residual .clone() on Copy Value and apply micro-optimizations (unsafe register access, tighter match arms) to squeeze remaining performance from the current architecture.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-runtime/src/dispatch/` — 8 dispatch module files (arith.rs, calls.rs, objects.rs, mod.rs, helpers.rs, intrinsics.rs, entities.rs, concurrency.rs)
- Value enum already derives Copy (Phase 79)

### Established Patterns
- 40 `.clone()` calls remain across 6 dispatch files (calls.rs: 10, objects.rs: 14, arith.rs: 4, mod.rs: 4, concurrency.rs: 4, intrinsics.rs: 4)
- 160 Value references across 7 dispatch files

### Integration Points
- Register access patterns in dispatch handlers
- Frame register pool from Phase 77

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase.

</specifics>

<deferred>
## Deferred Ideas

None — infrastructure phase.

</deferred>
