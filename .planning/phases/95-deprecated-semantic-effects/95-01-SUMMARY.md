---
phase: 95-deprecated-semantic-effects
plan: 01
subsystem: compiler
tags: [rust, writ-compiler, writ-diagnostics, writ-lsp, type-checking, warnings]

requires:
  - phase: 94-user-attribute-declarations
    provides: AstAttribute/AstAttributeArg types, attribute_decl pipeline, E0008 builtin shadow check

provides:
  - W0006 warning code in writ-diagnostics
  - TypeEnv.deprecated_items map (DefId -> deprecation message)
  - extract_deprecated_msg helper in env_build.rs
  - W0006 emission at function call sites (check_call_with_sig)
  - W0006 emission at ident references (check_ident — fn-as-value, const, global)
  - W0006 emission at construction sites (check_new_construction)
  - Self-deprecation suppression (same FileId = no warning)
  - 7 integration tests in deprecated_tests.rs

affects:
  - 95-02 (LSP diagnostics surfacing for W0006)
  - future phases querying deprecated_items for hover/completion markers

tech-stack:
  added: []
  patterns:
    - "Self-deprecation suppression: compare entry.file_id != ctx.current_file before emitting W0006"
    - "Two-pass TypeEnv build: main decl pass + second deprecated-items pass"
    - "find_attrs_for_entry duplicated in env_build.rs (pub(super)) as emit::collect::lookup is pub(super) to emit)"

key-files:
  created:
    - writ-compiler/tests/deprecated_tests.rs
  modified:
    - writ-diagnostics/src/code.rs
    - writ-compiler/src/check/env.rs
    - writ-compiler/src/check/env_build.rs
    - writ-compiler/src/check/check_expr/call.rs
    - writ-compiler/src/check/check_expr/ident.rs
    - writ-compiler/src/check/check_expr/construction.rs
    - writ-lsp/src/queries/completion.rs

key-decisions:
  - "find_attrs_for_entry duplicated in env_build.rs rather than making emit::collect::lookup pub(crate) — keeps emit module boundary clean"
  - "ident.rs emits W0006 only for non-call references; call sites handled exclusively in check_call_with_sig to prevent double-emission"
  - "Bare [Deprecated] with no args maps to empty string in deprecated_items, displayed as default message at call sites"

requirements-completed:
  - DEPR-01

duration: 25min
completed: 2026-03-27
---

# Phase 95 Plan 01: [Deprecated] Semantic Effects Summary

**W0006 deprecated-item warnings emitted at call/ident/construction sites with same-file suppression, backed by TypeEnv.deprecated_items populated from [Deprecated] attributes**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-03-27
- **Completed:** 2026-03-27
- **Tasks:** 2 (TDD: RED tests first, then GREEN implementation)
- **Files modified:** 7

## Accomplishments

- Added W0006 constant to writ-diagnostics and populated TypeEnv.deprecated_items from all [Deprecated] attributes during TypeEnv::build second pass
- Emits W0006 at function call sites, identifier references (fn-as-value, const, global), and new-construction sites
- Self-deprecation suppression: callers in the same file as the deprecated definition see no warning
- Bare `[Deprecated]` (no args) defaults to `` `name` is deprecated ``; with message shows `` `name` is deprecated: msg ``
- 7 integration tests cover all scenarios including cross-file call, same-file suppression, bare attribute, and struct construction

## Task Commits

1. **Task 1: Add W0006 code and deprecated_items map** - `ee897a4` (feat)
2. **Task 2: Emit W0006 at call/ident/construction sites** - `ab6199e` (feat)

## Files Created/Modified

- `writ-diagnostics/src/code.rs` - Added W0006 constant
- `writ-compiler/src/check/env.rs` - Added deprecated_items field to TypeEnv; second-pass population in build()
- `writ-compiler/src/check/env_build.rs` - Added extract_deprecated_msg helper and find_attrs_for_entry duplicate
- `writ-compiler/src/check/check_expr/call.rs` - W0006 emission in check_call_with_sig
- `writ-compiler/src/check/check_expr/ident.rs` - W0006 emission for non-call ident references
- `writ-compiler/src/check/check_expr/construction.rs` - W0006 emission for new-construction
- `writ-lsp/src/queries/completion.rs` - Fixed 3 TypeEnv struct literals missing deprecated_items field
- `writ-compiler/tests/deprecated_tests.rs` - 7 integration tests for all deprecated warning scenarios

## Decisions Made

- `find_attrs_for_entry` duplicated in `env_build.rs` (not made `pub(crate)` in `emit::collect::lookup`) — keeps emit module boundary clean and avoids cross-module coupling
- `check_ident` only emits W0006 for non-call references; direct calls handled in `check_call_with_sig` — eliminates double-emission risk since `resolve_overloaded_call` bypasses `check_ident` for function calls
- Bare `[Deprecated]` stores `""` in deprecated_items; call site formats as `` `name` is deprecated `` — consistent with Rust convention

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed writ-lsp TypeEnv struct literals missing deprecated_items**
- **Found during:** Task 2 verification (workspace-wide cargo test)
- **Issue:** 3 TypeEnv struct-literal initializers in writ-lsp/src/queries/completion.rs missing the new `deprecated_items` field, causing compile errors
- **Fix:** Added `deprecated_items: Default::default()` to all 3 TypeEnv literals in the LSP completion test helpers
- **Files modified:** writ-lsp/src/queries/completion.rs
- **Verification:** `cargo test --workspace` passes fully
- **Committed in:** ab6199e (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — missing struct field in consuming crate)
**Impact on plan:** Necessary correctness fix; no scope creep.

## Issues Encountered

None — implementation matched plan design exactly.

## Next Phase Readiness

- W0006 warning infrastructure complete; Plan 02 can surface these in LSP diagnostics and hover text
- deprecated_items is a public field on TypeEnv, accessible to LSP hover/completion handlers
- No blockers

---
*Phase: 95-deprecated-semantic-effects*
*Completed: 2026-03-27*
