---
gsd_state_version: 1.0
milestone: v14.0
milestone_name: milestone
status: verifying
stopped_at: Completed 122-02-PLAN.md (CLI wiring, config, virtual module injection, integration tests)
last_updated: "2026-03-29T22:59:57.509Z"
last_activity: 2026-03-29
progress:
  total_phases: 3
  completed_phases: 3
  total_plans: 8
  completed_plans: 8
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-29)

**Core value:** Correct, spec-compliant implementation at every layer — lowering matches Section 28 exactly, runtime matches the IL spec exactly — structured so each layer can be extended independently.
**Current focus:** Phase 122 — cross-module-type-resolution

## Current Position

Phase: 122
Plan: Not started
Status: Phase complete — ready for verification
Last activity: 2026-03-29

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Prior milestones:** 246 plans across 119 phases (v1.0-v13.0)
**v14.0 so far:** 0 plans across 0 phases complete

## Accumulated Context

### Decisions

(carried forward from v13.0 where relevant)

- Array primitives surfaced as compiler dot-call methods on `T[]` receivers — existing VM opcodes 0x0900-0x0908 need compiler resolution wiring only
- String utilities implemented as Rust intrinsics using `std::str` — no pure-Writ implementation, avoids Unicode byte-indexing corruption
- Collections written in pure Writ source files, loaded as a library module before user code — no compiler special-casing of List/Map/Set
- [v14.0] Arrays must be fixed-size — `add`/`remove_at`/`insert` removed from `T[]`; dynamic behavior belongs exclusively on List<T>
- [v14.0] `contains` should work on Iterable, not arrays — explicit impl specialization for arrays deferred to future milestone
- [v14.0] Cross-module type resolution: compiler loads DefMap from `.writc` for compile-time validation; virtual module types use same mechanism but built-in
- [v14.0 roadmap] Phase 120 must complete before Phase 121 — stdlib rewrite depends on resize+copy API being in place
- [v14.0 roadmap] Phase 122 (XMOD) is independent of 120/121 — can be executed in parallel or after
- [Phase 120-array-semantics-correction]: Clean break (D-01): ArrayAdd/Remove/Insert/Contains removed entirely; ArrayResize/Copy/NewArraySized/NewArrayFilled added; ArraySlice renumbered 0x0908->0x0907; format_version bumped to 5
- [Phase 120]: Type checker TyKind::Array match must mirror builtins.rs exactly — both files must be updated together when adding/removing array methods
- [Phase 120]: serialize.rs format_version override must be updated when format_version changes — it overrides the builder default
- [Phase 120]: writ-cli/build.rs made tolerant of stdlib compilation failure via empty placeholder .writc — Phase 121 will fix the stdlib source
- [Phase 120-03]: array_primitives.writ extended to include indexed access and overlap copy_from per plan D-09
- [Phase 121-stdlib-rewrite]: Golden test .writ drivers are standalone (no stdlib import) — they required the same resize API update as collections.writ
- [Phase 122-01]: collect_declarations accepts &mut DefMap to allow pre-injection of library types before user declarations
- [Phase 122-01]: inject_module_types called before collect_declarations so library FQNs are in DefMap before user declarations are processed
- [Phase 122-cross-module-type-resolution]: New sections appended as 1.2.8-1.2.10 to existing writ.toml spec file; using declarations documented as the standard mechanism for unqualified library namespace access
- [Phase 122]: Virtual module injection at CLI (cmd_build) level — writ-compiler cannot depend on writ-runtime; writ-cli depends on both
- [Phase 122]: inject_library_sigs top-level fn fix: rebuild type/impl method ownership ranges to detect non-owned methods and inject FnSigs into type_env.fn_sigs

### Critical Architecture Notes

- Array methods added in v13.0 (add, remove_at, insert) made arrays behave as lists — must be removed and stdlib rewritten to use resize+index
- Cross-module gap: DefMap is single-compilation-unit only; no mechanism to load from pre-compiled library modules
- `coll_with_library_separate_modules` test is `#[ignore]` — proof target for cross-module resolution (XMOD-04)
- ariadne panics if secondary label references a FileId absent from the renderer's sources slice — audit `render_diagnostics` sources slice construction before enabling cross-file secondary labels

### Pending Todos

None — ready to begin Phase 120.

### Blockers/Concerns

None.

## Session Continuity

Last activity: 2026-03-29 — Roadmap created, 3 phases (120-122), 17/17 requirements mapped
Stopped at: Completed 122-02-PLAN.md (CLI wiring, config, virtual module injection, integration tests)
Resume file: None
Next step: `/gsd:plan-phase 120`
