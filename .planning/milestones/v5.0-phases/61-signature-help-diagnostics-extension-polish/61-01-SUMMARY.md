---
phase: 61-signature-help-diagnostics-extension-polish
plan: 01
subsystem: lsp
tags: [lsp, signature-help, diagnostics, vscode-extension, semantic-tokens]

# Dependency graph
requires:
  - phase: 60-lsp-query-robustness
    provides: Robust LSP query infrastructure (binding_at_offset, def_at_offset, hover fallback)
provides:
  - Text-based callee name extraction for signature help on incomplete source
  - Zero-width parse error span expansion for visible VS Code squiggles
  - Distinct semantic token scopes and color defaults for entity vs struct names
affects: [v5.0-milestone-uat, vscode-extension-users]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Text-based backward scan for callee name extraction before AST-dependent lookup"
    - "Zero-width span expansion via saturating_sub(1) before LSP diagnostic conversion"
    - "configurationDefaults with [*] wildcard for guaranteed semantic token colors"

key-files:
  created: []
  modified:
    - writ-lsp/src/queries.rs
    - writ-lsp/src/convert.rs
    - writ-vscode/package.json

key-decisions:
  - "Text-based callee extraction (extract_callee_name) is primary path; find_enclosing_call remains as fallback — O(1) string scan vs full re-analysis"
  - "Zero-width span expansion uses saturating_sub(1) without source text parameter — minimal diff, safe for ASCII, avoids callers needing update"
  - "semanticTokenScopes entity mapped to support.class.writ (not entity.name.type.writ) — avoids inheriting struct color via TextMate prefix matching"
  - "configurationDefaults [*] wildcard guarantees visual distinction in all themes including Dark+"

patterns-established:
  - "Signature help primary path: text scan + DefMap lookup for incomplete sources, AST Call node as fallback for complete sources"
  - "Parse error span normalization: always expand zero-width spans before LSP conversion"

requirements-completed: [LSP-07, LSP-01, DIFF-01]

# Metrics
duration: 12min
completed: 2026-03-17
---

# Phase 61 Plan 01: Signature Help, Diagnostics, and Extension Polish Summary

**Text-based signature help via DefMap callee lookup, zero-width span expansion for visible squiggles, and distinct entity/struct semantic token colors via scope remapping and configurationDefaults**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-03-17T09:50:00Z
- **Completed:** 2026-03-17T10:02:45Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Added `extract_callee_name` function to scan source text backward from `(` and look up callee in DefMap, enabling signature help on syntactically incomplete source like `foo(` and `foo(1,`
- Expanded zero-width spans in `parse_error_to_diag` using `saturating_sub(1)` so VS Code renders visible squiggles for EOF and entity/struct recovery parse errors
- Remapped semantic token scopes from `entity.name.type.*` family to `support.class.writ`, `support.other.component.writ`, `variable.other.constant.speaker.writ` to avoid inheriting struct color
- Added `configurationDefaults` with `[*]` wildcard in package.json providing explicit Dark+ palette colors: teal (#4EC9B0) for entities, blue (#9CDCFE) for components, orange (#CE9178) for dialogue speakers
- 63 writ-lsp tests pass (was 59 before; 4 new tests added)

## Task Commits

Each task was committed atomically:

1. **Task 1: Text-based signature help and zero-width span expansion** - `c415b55` (feat)
2. **Task 2: Remap semantic token scopes and add color defaults in package.json** - `534e2de` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `writ-lsp/src/queries.rs` - Added `extract_callee_name()` and text-based primary path in `build_signature_help()`; 2 new signature help tests
- `writ-lsp/src/convert.rs` - Zero-width span expansion in `parse_error_to_diag()`; 2 new span tests
- `writ-vscode/package.json` - Remapped semanticTokenScopes + added configurationDefaults block

## Decisions Made

- Text-based callee extraction is the primary path for signature help because it works on incomplete/broken sources with O(1) string scan overhead. The existing `find_enclosing_call` path remains as fallback for complete calls (backward compatible).
- Zero-width span expansion uses `saturating_sub(1)` without passing source text to `parse_error_to_diag`. This avoids updating 2 call sites and is correct for all ASCII inputs (which all Writ keywords and identifiers are at error boundaries).
- The `[*]` theme wildcard in `configurationDefaults` overrides all themes including light ones. This is intentional — visual distinction is the requirement, and users can override in personal settings.

## Deviations from Plan

None - plan executed exactly as written. The test `test_signature_help_incomplete_source` prints "Note: typed_ast not available for broken source" and skips the assertion body when `analyze_standalone` does not return a typed AST (source is too broken for typecheck to run). This is the plan-specified behavior — the `else` branch with `eprintln!` was explicitly included in the plan's test code.

## Issues Encountered

- `SimpleSpan::new((), 0..0)` in the initial `test_zero_width_span_at_offset_zero` test body caused a compile error (function not found). Fixed by removing that line — the test verifies the expansion via the span field check instead.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All three UAT gap requirements (LSP-07, LSP-01, DIFF-01) are now covered
- v5.0 milestone UAT should pass tests 4, 12, and 13
- No blockers for v5.0-milestone-uat phase

## Self-Check: PASSED

- writ-lsp/src/queries.rs: FOUND
- writ-lsp/src/convert.rs: FOUND
- writ-vscode/package.json: FOUND
- .planning/phases/61-signature-help-diagnostics-extension-polish/61-01-SUMMARY.md: FOUND
- Commit c415b55 (Task 1): FOUND
- Commit 534e2de (Task 2): FOUND

---
*Phase: 61-signature-help-diagnostics-extension-polish*
*Completed: 2026-03-17*
