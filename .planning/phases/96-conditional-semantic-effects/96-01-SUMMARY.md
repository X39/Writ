---
phase: 96-conditional-semantic-effects
plan: 01
subsystem: compiler
tags: [rust, type-checker, attributes, conditional-compilation, semantic-effects]

# Dependency graph
requires:
  - phase: 95-deprecated-semantic-effects
    provides: find_attrs_for_entry, extract_deprecated_msg patterns in env_build.rs; deprecated_items field on TypeEnv

provides:
  - E0009 and E0010 error codes in writ-diagnostics
  - conditional_fns and fallback_for_conditional fields on TypeEnv (populated by attribute scan + fallback verification pass)
  - extract_conditional_name helper in env_build.rs
  - conditional_fns and fallback_for_conditional fields on TypedAst (transferred from TypeEnv)
  - find_fn_candidates filters out conditional fn DefIds (checker resolves calls to fallback only)

affects:
  - 96-02 (emit-time filtering depends on TypedAst.conditional_fns and TypedAst.fallback_for_conditional)
  - writ-lsp (LSP queries receive TypeEnv with new fields)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Fourth pass in TypeEnv::build: fallback verification — parallel to deprecated_items pass (third pass)"
    - "Additive filtering in find_fn_candidates: conditional fns excluded from candidate resolution"
    - "TypedAst carries conditional maps as passthrough for downstream emit consumption"

key-files:
  created: []
  modified:
    - writ-diagnostics/src/code.rs
    - writ-compiler/src/check/env.rs
    - writ-compiler/src/check/env_build.rs
    - writ-compiler/src/check/ir.rs
    - writ-compiler/src/check/mod.rs
    - writ-compiler/src/check/check_expr/mod.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-compiler/tests/emit_serialize_tests.rs
    - writ-lsp/src/queries/completion.rs

key-decisions:
  - "Fallback verification pass runs after conditional_fns is populated so it can check env.conditional_fns to skip other conditional overloads"
  - "Diags variable rebound once before fallback verification pass; removed duplicate rebind before validate_contract_impls"
  - "Private fn overload lookup uses fn_overloads key format 'name@file_id' as fallback for missing file_private entries"
  - "LSP completion test TypeEnv literals fixed with two new Default fields (Rule 1: auto-fix)"

patterns-established:
  - "Attribute-driven TypeEnv pass pattern: scan all decls, call find_attrs_for_entry, call extract_X helper, insert into map"
  - "Checker candidate filtering: apply .filter(|id| !ctx.type_env.conditional_fns.contains_key(id)) to all three return paths"

requirements-completed: [COND-03, COND-04]

# Metrics
duration: 18min
completed: 2026-03-27
---

# Phase 96 Plan 01: Conditional Semantic Effects Infrastructure Summary

**TypeEnv + TypedAst carry [Conditional] metadata maps; checker always resolves calls to fallback; E0009 fires for missing fallback.**

## Performance

- **Duration:** 18 min
- **Started:** 2026-03-27T20:34:00Z
- **Completed:** 2026-03-27T20:52:00Z
- **Tasks:** 2/2
- **Files modified:** 9

## Accomplishments

- Added E0009 (missing [Conditional] fallback) and E0010 (ambiguous active conditions) to writ-diagnostics/src/code.rs
- TypeEnv gains `conditional_fns: FxHashMap<DefId, String>` and `fallback_for_conditional: FxHashMap<DefId, DefId>` populated in two new build passes: third pass scans [Conditional] attrs, fourth pass verifies fallback existence and emits E0009 when absent
- TypedAst carries both maps transferred from TypeEnv; all three return paths in find_fn_candidates filter out conditional fn DefIds so checker always resolves to fallback (COND-04)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add error codes, TypeEnv fields, env_build extraction, and fallback verification** - `3178cd7` (feat)
2. **Task 2: Embed conditional maps in TypedAst, filter candidates in checker, wire typecheck** - `2b71067` (feat)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed LSP completion test TypeEnv struct literal initialization**
- **Found during:** Task 2 (cargo test --workspace failure)
- **Issue:** Three `TypeEnv { ... }` struct literals in writ-lsp/src/queries/completion.rs were missing `conditional_fns` and `fallback_for_conditional` fields
- **Fix:** Added `conditional_fns: Default::default()` and `fallback_for_conditional: Default::default()` to all three
- **Files modified:** writ-lsp/src/queries/completion.rs
- **Commit:** 2b71067

**2. [Rule 1 - Bug] Fixed emit test TypedAst struct literal initialization**
- **Found during:** Task 2 (cargo test --workspace failure — 14 missing field errors in emit tests)
- **Issue:** 12 TypedAst struct literals in emit_body_tests.rs and 2 in emit_serialize_tests.rs missing new conditional fields
- **Fix:** Added `conditional_fns: FxHashMap::default()` and `fallback_for_conditional: FxHashMap::default()` to all 14 literals
- **Files modified:** writ-compiler/tests/emit_body_tests.rs, writ-compiler/tests/emit_serialize_tests.rs
- **Commit:** 2b71067

**3. [Rule 1 - Bug] Fixed duplicate `let mut diags = diags` rebinding**
- **Found during:** Task 1 implementation — the fourth pass already rebound `diags`, but the original validate_contract_impls block also rebound it, causing a type conflict (Vec<DiagnosticBuilder> vs Vec<Diagnostic>)
- **Fix:** Removed the duplicate `let mut diags = diags` before validate_contract_impls since diags was already bound in the fourth pass
- **Files modified:** writ-compiler/src/check/env.rs
- **Commit:** 3178cd7

**4. [Rule 1 - Bug] Fixed DiagnosticBuilder not calling .build()**
- **Found during:** Task 1 build failure (E0308 type mismatch)
- **Issue:** E0009 diagnostic push used `.with_primary(...)` without `.build()`, causing type inference to treat the vec as `Vec<DiagnosticBuilder>` rather than `Vec<Diagnostic>`
- **Fix:** Added `.build()` call at end of builder chain
- **Files modified:** writ-compiler/src/check/env.rs
- **Commit:** 3178cd7

## Known Stubs

None — all maps are fully populated; E0009 fires correctly for missing fallbacks.

## Self-Check: PASSED
- `writ-diagnostics/src/code.rs` contains E0009 and E0010 constants
- `writ-compiler/src/check/env.rs` contains conditional_fns and fallback_for_conditional fields
- `writ-compiler/src/check/env_build.rs` contains extract_conditional_name
- `writ-compiler/src/check/ir.rs` contains conditional_fns and fallback_for_conditional on TypedAst
- `writ-compiler/src/check/mod.rs` transfers maps from TypeEnv to TypedAst
- `writ-compiler/src/check/check_expr/mod.rs` filters conditional fns from find_fn_candidates
- All workspace tests pass (0 failures across all test suites)
- Commits 3178cd7 and 2b71067 exist in git log
