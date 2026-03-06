---
phase: 54-lsp-navigation-and-completions
plan: 03
subsystem: lsp
tags: [tower-lsp, completions, signature-help, lsp-types, writ-compiler]

# Dependency graph
requires:
  - phase: 54-01
    provides: AnalysisResult with TypedAst/TyInterner/TypeEnv, expr_at_offset, position_to_byte_offset, analysis_cache in Backend
provides:
  - build_identifier_completions: keywords + prelude names + DefMap public entries
  - build_dot_completions: struct fields, methods, entity components (DIFF-02), array/option built-ins
  - build_signature_help: backward paren scan + comma count for active parameter tracking
  - Backend::completion: dot-completion (re-analyze stripped source) and identifier-completion (cached DefMap)
  - Backend::signature_help: uses cached TypeEnv fn_sigs for parameter info
affects: [54-04, 54-05, 57-bundle]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Dot-completion strips trailing dot and re-analyzes modified source via analyze_standalone — avoids parser crash on trailing dot"
    - "Identifier completions always return keyword + prelude names as minimum, even without cached typed data"
    - "Signature help backward-scans source bytes for '(' counting comma depth to find active parameter"
    - "find_call_in_expr: returns narrowest-span Call node containing offset for enclosing-call detection"

key-files:
  created: []
  modified:
    - writ-lsp/src/queries.rs
    - writ-lsp/src/backend.rs

key-decisions:
  - "Dot-completion re-runs analyze_standalone on modified source (dot stripped) rather than using cached analysis — accepts slightly degraded cross-file type resolution in exchange for speed and simplicity during active typing"
  - "build_identifier_completions skips DefMap entries with FileId(u32::MAX) (synthetic builtins: log::*, dialogue builtins) to avoid redundant entries — prelude lists them separately with correct kinds"
  - "find_call_in_expr returns innermost Call to handle nested calls (e.g., foo(bar(|))) correctly — innermost wins"

patterns-established:
  - "Query functions in queries.rs: pure functions over TypedAst/TyInterner/TypeEnv — no async, no side effects"
  - "Backend handlers: thin glue that reads cache/source, delegates to queries.rs, returns LSP response"

requirements-completed: [LSP-02, LSP-03, LSP-07, DIFF-02]

# Metrics
duration: 9min
completed: 2026-03-14
---

# Phase 54 Plan 03: Completions and Signature Help Summary

**Identifier + dot completions and signature help LSP handlers using TypeEnv struct_fields, entity_components, and fn_sigs — with DIFF-02 entity component names in dot-completions**

## Performance

- **Duration:** 9 min
- **Started:** 2026-03-14T09:04:20Z
- **Completed:** 2026-03-14T09:13:17Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- `build_identifier_completions`: returns 34 keywords + 5 primitive types + 5 prelude types + 17 contract names + all public non-synthetic DefMap entries with correct CompletionItemKind
- `build_dot_completions`: returns struct/class fields and methods, entity fields/methods/components (DIFF-02), enum variants, array built-ins (`push`/`pop`/`len`/`is_empty`), Option built-ins (`is_some`/`is_none`/`unwrap`)
- `build_signature_help`: backward byte-scan from cursor to find enclosing `(`, comma-counts for `active_parameter`, looks up `FnSig` from `TypeEnv::fn_sigs`
- `Backend::completion`: dispatches on dot trigger — strips dot and re-analyzes for receiver type, falls back to identifier completions
- `Backend::signature_help`: reads cached TypedAst + TypeEnv, delegates to `build_signature_help`
- 5 new tests covering keywords, prelude names, struct field completions, entity component completions, and signature active-parameter tracking

## Task Commits

Each task was committed atomically:

1. **Task 1: Add completion and signature help query functions to queries.rs** - `73dd5ea` (feat)
2. **Task 2: Implement completion and signature_help handlers in Backend** - `dd20641` (feat)

**Plan metadata:** (docs commit pending)

## Files Created/Modified
- `writ-lsp/src/queries.rs` - Added `build_identifier_completions`, `build_dot_completions`, `build_signature_help`, `find_enclosing_call`, `find_call_in_expr`, `find_call_in_stmts`, `find_call_in_stmt`, `format_fn_sig_oneliner`; 5 new tests
- `writ-lsp/src/backend.rs` - Added `completion` and `signature_help` to `LanguageServer` impl; added `identifier_completion` helper to `impl Backend`

## Decisions Made
- Dot-completion re-runs `analyze_standalone` on source with dot stripped rather than walking the cached AST. This is a deliberate simplification: dot introduces a parse error in the cached AST, and re-analysis is fast enough for single-file interactive completion.
- `FileId(u32::MAX)` synthetic entries (log::*, dialogue builtins) are excluded from identifier completions since they are already provided as prelude-level names and would appear as duplicates.
- `find_call_in_expr` returns the innermost Call node to handle nested calls correctly — when cursor is inside `bar(|)` inside `foo(bar())`, signature help shows `bar`'s parameters.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed test: struct fields not found in by_fqn**
- **Found during:** Task 1 (test_dot_completions_struct_fields)
- **Issue:** Test used `struct Point { ... }` (private) — private definitions are not in `by_fqn`. Dot-completion test could not find the DefId.
- **Fix:** Changed struct declaration to `pub struct Point { ... }`
- **Files modified:** writ-lsp/src/queries.rs
- **Verification:** Test passes
- **Committed in:** 73dd5ea (Task 1 commit)

**2. [Rule 1 - Bug] Fixed test: extern component field missing trailing comma**
- **Found during:** Task 1 (test_dot_completions_entity_components)
- **Issue:** Writ extern component fields require trailing comma syntax `name: type,` — test used `{ hp: int }` without comma.
- **Fix:** Changed to `{ hp: int, }`
- **Files modified:** writ-lsp/src/queries.rs
- **Verification:** Test passes, parse errors gone
- **Committed in:** 73dd5ea (Task 1 commit)

**3. [Rule 1 - Bug] Fixed test: incorrect `on create` placement inside `impl` block**
- **Found during:** Task 1 (test_dot_completions_entity_components)
- **Issue:** Initial test placed `on create { }` inside `impl Player {}` — parse error because lifecycle hooks belong in entity bodies, not impl blocks.
- **Fix:** Simplified test to not use lifecycle hooks; used `pub entity Player { use Health { hp: 100 }, }` directly
- **Files modified:** writ-lsp/src/queries.rs
- **Verification:** Test passes
- **Committed in:** 73dd5ea (Task 1 commit)

**4. [Rule 1 - Bug] Fixed: `interner.intern()` requires `&mut self`**
- **Found during:** Task 1 (test_dot_completions_struct_fields, test_dot_completions_entity_components)
- **Issue:** Tests called `interner.intern(TyKind::Struct(...))` without `mut` binding — compiler error E0596
- **Fix:** Changed to `let (ast, mut interner, type_env) = build_typed_ast_full(src)`
- **Files modified:** writ-lsp/src/queries.rs
- **Verification:** Compiles and tests pass
- **Committed in:** 73dd5ea (Task 1 commit)

---

**Total deviations:** 4 auto-fixed (all Rule 1 bugs in tests, caught during verify)
**Impact on plan:** All auto-fixes were in test code to match actual Writ language syntax and Rust borrow rules. No changes to implementation logic.

## Issues Encountered
None

## Next Phase Readiness
- Completions and signature help are fully wired in the Backend; all `cargo test -p writ-lsp` pass (38 tests)
- Phase 54-04 (semantic tokens) can proceed — it reads the same TypedAst and TypeEnv from the analysis cache
- The dot-completion standalone re-analysis approach is a simplification: cross-file type resolution during dot-completion uses only the single file's types. Phase 54-05 or later can upgrade to project-mode dot-completion.

---
*Phase: 54-lsp-navigation-and-completions*
*Completed: 2026-03-14*

## Self-Check: PASSED

- `writ-lsp/src/queries.rs` - FOUND
- `writ-lsp/src/backend.rs` - FOUND
- Commit `73dd5ea` (Task 1) - FOUND
- Commit `dd20641` (Task 2) - FOUND
