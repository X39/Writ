---
phase: 22-name-resolution
plan: 03
status: complete
completed: "2026-03-02"
requirements-completed: [RES-09, RES-10, RES-11, RES-12]
---

# Plan 22-03 Summary: Validation & Error Quality

## Status: COMPLETE

## What Was Built
- **suggest.rs**: Fuzzy name suggestion engine using Jaro-Winkler similarity (strsim) with 0.8 threshold, max 3 suggestions. Two-phase search: visible names in scope, then cross-namespace search with import hints.
- **validate.rs**: Attribute target validation - `[Singleton]` only valid on entities, `[Conditional]` only valid on functions. Speaker validation structure in place (stub for future implementation).
- **Suggestion integration**: All 4 `UnresolvedName` error paths in resolver.rs now produce fuzzy "did you mean?" suggestions via `get_suggestion()` helper.
- **visible_names()**: New method on ScopeChain that collects all names visible from the current scope (generic params, current namespace members, file-private names, root namespace, using imports) for suggestion generation.

## Files Created/Modified
- `writ-compiler/src/resolve/suggest.rs` (new - 209 lines, 5 unit tests)
- `writ-compiler/src/resolve/validate.rs` (new - 92 lines)
- `writ-compiler/src/resolve/resolver.rs` (integrated suggestions into all error paths)
- `writ-compiler/src/resolve/scope.rs` (added visible_names())
- `writ-compiler/src/resolve/mod.rs` (added suggest + validate modules)
- `writ-compiler/tests/resolve_tests.rs` (9 new tests)

## Test Results
- 33 integration tests passing (24 from Waves 1+2 + 9 new)
- 13 unit tests passing (config, diagnostics, suggestions)
- Full workspace test suite: all tests pass across all crates
- New tests cover: [Singleton] on entity/struct/fn, [Conditional] on fn/entity, fuzzy suggestion quality, no-suggestion for unrelated names, generic shadow warning, generic no-shadow

## Requirements Addressed
- RES-09: Attribute target validation (E0006)
- RES-10: Speaker validation structure (E0007, implementation ready for future)
- RES-11: Fuzzy name suggestions for unresolved names
- RES-12: Error quality and diagnostic help text

## Self-Check: PASSED
