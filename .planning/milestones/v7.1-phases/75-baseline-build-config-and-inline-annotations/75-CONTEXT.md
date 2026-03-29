# Phase 75: Baseline, Build Config, and Inline Annotations - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers a fully release-optimized build configuration (LTO, single codegen unit, panic=abort), replaces std HashMap with FxHashMap in hot paths, measures and documents the fib(40) baseline, and applies inline annotations to hot dispatch helpers.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- writ-runtime crate contains the VM dispatch loop and scheduler
- Existing benchmark infrastructure in benchmark/ directory

### Established Patterns
- Cargo workspace with multiple crates
- VM uses HashMap for DispatchTable and Scheduler.tasks

### Integration Points
- Cargo.toml profile configuration for release builds
- writ-runtime/src/ dispatch handlers for inline annotations

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase

</specifics>

<deferred>
## Deferred Ideas

None

</deferred>
