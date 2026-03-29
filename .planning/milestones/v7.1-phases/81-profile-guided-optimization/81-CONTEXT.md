# Phase 81: Profile-Guided Optimization Build - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Apply profile-guided optimization (PGO) using fib(40) as the training workload to close the VERIFY-04 performance gap. Instrument build, profile run, optimized rebuild pipeline.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-runtime/` — the VM crate to be PGO-optimized
- `benchmark/cases/fibonacci/fib.writ` — the training workload
- Current fib(40) baseline: 43.765s (Phase 80 result)

### Established Patterns
- Cargo workspace with multiple crates
- Release profile in Cargo.toml
- `cargo run --release -- run benchmark/cases/fibonacci/fib.writ` for benchmarking

### Integration Points
- Cargo build system (rustc PGO flags via RUSTFLAGS or cargo config)
- LLVM PGO instrumentation (llvm-profdata merge)

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase.

</specifics>

<deferred>
## Deferred Ideas

None — infrastructure phase.

</deferred>
