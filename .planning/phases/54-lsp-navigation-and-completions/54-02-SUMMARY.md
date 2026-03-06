---
phase: 54-lsp-navigation-and-completions
plan: 02
subsystem: lsp
tags: [tower-lsp, rust, lsp, hover, goto-definition, find-references, TypedAst]

# Dependency graph
requires:
  - phase: 54-01
    provides: "queries.rs with expr_at_offset, find_def_id_at_offset; AnalysisResult with TypedAst/TyInterner/TypeEnv; Backend analysis_cache; TyInterner::display_named"
provides:
  - "hover handler: returns markdown type/signature tooltip for any TypedExpr at cursor"
  - "goto_definition handler: navigates to DefEntry name_span location; returns None for synthetic builtins"
  - "references handler: collects all use-sites of a DefId across the TypedAst"
  - "hover_text_for_expr query: builds markdown hover string for Var, Call, Field, ComponentAccess, New, SelfRef, Path"
  - "collect_references query: walks entire TypedAst collecting SimpleSpan use-sites for a given DefId"
affects: [54-03, 54-04, 54-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cache-then-query: all navigation handlers read from analysis_cache first, gracefully return None if cache empty or typed data unavailable"
    - "Builtin sentinel: FileId(u32::MAX) is the sentinel for synthetic builtins — navigation returns None instead of crashing"
    - "Span-containment file matching: references handler maps spans to file sources by checking span.end <= src.len()"

key-files:
  created: []
  modified:
    - writ-lsp/src/queries.rs
    - writ-lsp/src/backend.rs

key-decisions:
  - "collect_references searches both by_fqn and file_private for Var name resolution — functions defined in a single-file test are file-private"
  - "References handler uses span containment (span.end <= src.len()) to match multi-file spans to file sources; falls back to trigger URI"
  - "hover returns None for TypedExpr::Error nodes — no useful type info to show"

patterns-established:
  - "Navigation handler pattern: read document_map -> read analysis_cache -> position_to_byte_offset -> expr_at_offset -> extract answer -> return None on any None"

requirements-completed: [LSP-04, LSP-05, LSP-06]

# Metrics
duration: 8min
completed: 2026-03-14
---

# Phase 54 Plan 02: LSP Navigation Handlers Summary

**Hover shows type/signature tooltips, goto-definition navigates to declaration spans, find-references collects all use-sites — all three using TypedAst analysis cache**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-03-14T09:00:00Z
- **Completed:** 2026-03-14T09:07:49Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `hover_text_for_expr` builds markdown code blocks for Var (name + type), Call (full fn signature with generics), Field/ComponentAccess (field + type), New (struct name), SelfRef (self + type), Path (path + type)
- `collect_references` walks all Fn/Impl/Const/Global bodies collecting SimpleSpan use-sites for a given DefId
- `hover` handler wired into Backend LanguageServer impl — reads analysis_cache, finds expr at cursor, returns Hover with MarkupContent::Markdown
- `goto_definition` handler — resolves DefId, guards against FileId(u32::MAX) builtins, converts name_span to Location
- `references` handler — collects ref spans, maps to file sources by containment, optionally includes declaration site
- 3 new tests: test_hover_text_var, test_hover_text_fn_call, test_collect_references_finds_uses (all pass)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add hover_text_for_expr and collect_references to queries.rs** - `bf081ee` (feat)
2. **Task 2: Implement hover, goto_definition, and references handlers in Backend** - `b79678b` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified
- `writ-lsp/src/queries.rs` - Added hover_text_for_expr, format_fn_sig_hover, collect_references, collect_refs_in_expr/stmts/stmt helpers, plus 3 new tests and build_typed_ast_full test helper
- `writ-lsp/src/backend.rs` - Added hover, goto_definition, and references methods to LanguageServer impl

## Decisions Made
- `collect_references` searches both `by_fqn` and `file_private` for Var name resolution — discovered during test that top-level functions in single-file test sources are file-private (not public)
- References handler uses span containment (span.end <= src.len()) to match multi-file spans to file sources; falls back to trigger URI for unmatched spans
- hover returns empty string (mapped to None) for TypedExpr::Error nodes since there is no useful type info to show

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] File-private function not found by by_fqn lookup in test**
- **Found during:** Task 1 (test_collect_references_finds_uses)
- **Issue:** Top-level functions in single-file test sources are inserted as file-private (not by_fqn), causing the test helper that searched only by_fqn to fail
- **Fix:** Updated test to search both by_fqn and file_private maps when looking up a definition by name
- **Files modified:** writ-lsp/src/queries.rs
- **Verification:** test_collect_references_finds_uses passes
- **Committed in:** bf081ee (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Necessary to fix the test — the implementation logic in collect_refs_in_expr already checks both maps correctly, only the test helper needed the fix.

## Issues Encountered
None beyond the auto-fixed deviation above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Navigation handlers complete: hover, goto-definition, find-references fully wired
- Plan 54-03 (completions) can build on the same analysis_cache pattern established here
- No blockers

---
*Phase: 54-lsp-navigation-and-completions*
*Completed: 2026-03-14*
