---
phase: 63-writ-compiler-file-splits
plan: 02
subsystem: compiler
tags: [rust, refactoring, module-split, emit, collect]

# Dependency graph
requires:
  - phase: 63-01
    provides: check_expr split (parallel wave — no strict dependency)
provides:
  - collect/ folder module with 9 focused subfiles replacing collect.rs (1,687 lines)
  - mod.rs: collect_defs, collect_post_finalize, find_module_name, build_generic_map
  - types.rs: collect_struct, collect_entity, collect_enum, collect_class, collect_extern_struct
  - functions.rs: collect_fn, collect_extern_fn, collect_component
  - contracts.rs: collect_contract, collect_impl, collect_extern_class, collect_extern_component
  - builtins.rs: inject_log_extern_defs, inject_dialogue_extern_defs
  - walker.rs: collect_called_def_ids, walk_expr, walk_stmt
  - globals.rs: collect_const, collect_global
  - encoding.rs: all type encoding helpers, collect_exports, collect_attributes, collect_locale_defs, collect_component_slots
  - lookup.rs: all find_*_decl helpers and find_attrs_for_entry
affects: [63-03, 63-04, phase-65-duplication]

# Tech tracking
tech-stack:
  added: []
  patterns: [Rust module folder split — replace single .rs file with directory/mod.rs + submodules, pub(super) visibility for intra-module helpers, explicit imports in each submodule]

key-files:
  created:
    - writ-compiler/src/emit/collect/mod.rs
    - writ-compiler/src/emit/collect/types.rs
    - writ-compiler/src/emit/collect/functions.rs
    - writ-compiler/src/emit/collect/contracts.rs
    - writ-compiler/src/emit/collect/builtins.rs
    - writ-compiler/src/emit/collect/walker.rs
    - writ-compiler/src/emit/collect/globals.rs
    - writ-compiler/src/emit/collect/encoding.rs
    - writ-compiler/src/emit/collect/lookup.rs
  modified:
    - writ-compiler/src/emit/collect.rs (DELETED)

key-decisions:
  - "collect_extern_class placed in contracts.rs (not types.rs) alongside collect_extern_component — both are extern compound definitions that mirror the contract/impl boundary"
  - "find_attrs_for_entry placed in lookup.rs (not encoding.rs) — it's an AST lookup helper with same pattern as all other find_* functions"
  - "encoding.rs imports find_attrs_for_entry directly from lookup.rs — clean explicit import, no re-export needed"

patterns-established:
  - "Module folder split pattern: submodules use pub(super) fn, mod.rs re-exports only the pub API (inject_log_extern_defs, inject_dialogue_extern_defs via pub use builtins::*)"
  - "Cross-submodule calls: submodules import from super::encoding::, super::lookup:: using explicit paths"
  - "No glob re-exports (pub use *) in any submodule"

requirements-completed: [SPLIT-04]

# Metrics
duration: 8min
completed: 2026-03-18
---

# Phase 63 Plan 02: Collect Module Split Summary

**collect.rs (1,687 lines) split into collect/ folder with 9 focused subfiles by declaration category — all 75 tests pass, zero clippy warnings, no file exceeds 472 lines**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-18T01:29:50Z
- **Completed:** 2026-03-18T01:38:44Z
- **Tasks:** 1
- **Files modified:** 10 (9 created, 1 deleted)

## Accomplishments
- Deleted monolithic collect.rs (1,687 lines)
- Created collect/ directory with 9 submodules, each focused on a single declaration category
- All 75 writ-compiler tests pass without modification
- Zero clippy warnings after split
- No glob re-exports in any submodule; all cross-module calls use explicit import paths

## Task Commits

Each task was committed atomically:

1. **Task 1: Create collect/ folder module by splitting collect.rs into 9 focused subfiles** - `221fd40` (refactor)

## Files Created/Modified
- `writ-compiler/src/emit/collect/mod.rs` - Entry points: collect_defs, collect_post_finalize, find_module_name, build_generic_map; submodule declarations and re-exports
- `writ-compiler/src/emit/collect/types.rs` - Struct, entity, enum, class, extern struct collection (5 functions, 228 lines)
- `writ-compiler/src/emit/collect/functions.rs` - Fn, extern fn, component collection (3 functions, 122 lines)
- `writ-compiler/src/emit/collect/contracts.rs` - Contract, impl, extern class, extern component collection (4 functions, 219 lines)
- `writ-compiler/src/emit/collect/builtins.rs` - inject_log_extern_defs, inject_dialogue_extern_defs (107 lines)
- `writ-compiler/src/emit/collect/walker.rs` - collect_called_def_ids, walk_expr, walk_stmt (139 lines)
- `writ-compiler/src/emit/collect/globals.rs` - collect_const, collect_global (50 lines)
- `writ-compiler/src/emit/collect/encoding.rs` - Type encoding helpers + post-finalize passes (472 lines)
- `writ-compiler/src/emit/collect/lookup.rs` - All find_*_decl + find_attrs_for_entry (241 lines)
- `writ-compiler/src/emit/collect.rs` - DELETED

## Decisions Made
- Placed `collect_extern_class` and `collect_extern_component` in contracts.rs alongside `collect_contract` and `collect_impl`, since extern classes/components define the extern boundary analogous to contracts
- Kept `find_attrs_for_entry` in lookup.rs with the other `find_*` helpers (same pattern: scan asts by entry)
- `encoding.rs` has both type-encoding helpers AND post-finalize passes (collect_exports, collect_attributes, collect_locale_defs, collect_component_slots) since these all deal with type/token encoding

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None - the split was mechanical. Unused import warnings were cleaned up immediately after first build.

## Next Phase Readiness
- SPLIT-04 requirement fulfilled
- collect/ module is navigable; each file has a clear responsibility
- Wave 2 plans (63-03, 63-04) can now proceed

---
*Phase: 63-writ-compiler-file-splits*
*Completed: 2026-03-18*

## Self-Check: PASSED

- All 9 collect/ submodule files exist on disk
- writ-compiler/src/emit/collect.rs does NOT exist on disk
- Commit 221fd40 exists in git log
- 75 tests pass, zero clippy warnings
