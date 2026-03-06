---
phase: quick
plan: 260320-4ms
subsystem: writ-lsp
tags: [lsp, completion, new-keyword, context-aware]
dependency_graph:
  requires: []
  provides: [LSP-NEW-COMPLETION]
  affects: [writ-lsp/src/queries/completion.rs, writ-lsp/src/backend.rs]
tech_stack:
  added: []
  patterns: [context-aware completion dispatch, backward text scan for keyword detection]
key_files:
  created: []
  modified:
    - writ-lsp/src/queries/completion.rs
    - writ-lsp/src/queries/mod.rs
    - writ-lsp/src/backend.rs
    - writ-lsp/tests/test_protocol.rs
decisions:
  - "is_after_new_keyword scans only spaces/tabs (not newlines) as whitespace to avoid cross-line false positives"
  - "Entity uses CLASS completion kind; Struct/ExternStruct/Class/ExternClass use STRUCT kind — mirrors existing identifier_completions mapping"
  - "Empty CompletionResponse (not None) returned when cache unavailable in new-keyword path, so LSP client receives a valid but empty list rather than no response"
metrics:
  duration: "~8 minutes"
  completed: "2026-03-20"
  tasks: 2
  files: 4
---

# Phase quick Plan 260320-4ms: LSP Add Auto-Completion After `new` Keyword Summary

**One-liner:** Context-aware LSP completion after `new ` keyword — filters to constructable types only (Struct, Class, Entity) using backward text scan with word-boundary check, wired into identifier_completion dispatch.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add is_after_new_keyword and build_new_keyword_completions | c56032d | completion.rs, mod.rs |
| 2 | Wire context-aware completion into backend.rs and add E2E tests | d840024 | backend.rs, test_protocol.rs |

## What Was Built

### Task 1: Helper functions in completion.rs

**`is_after_new_keyword(source: &str, byte_offset: usize) -> bool`**
- Scans backward from `byte_offset` skipping spaces/tabs
- Requires at least one whitespace consumed (user pressed Space after `new`)
- Checks preceding 3 bytes are `n`, `e`, `w`
- Verifies character before `new` (if any) is not alphanumeric or `_` (avoids `renew`, `fnew`, etc.)

**`build_new_keyword_completions(def_map: &DefMap, _interner: &TyInterner) -> Vec<CompletionItem>`**
- Iterates `def_map.by_fqn` values
- Filters to `Struct`, `ExternStruct`, `Class`, `ExternClass` (STRUCT kind), `Entity` (CLASS kind)
- Skips synthetic entries (`file_id == FileId(u32::MAX)`)
- Excludes all other kinds: Fn, Enum, Contract, Impl, Const, Global, Component, etc.

**5 unit tests added:**
- `test_is_after_new_keyword_basic` — `"let x = new "` returns true
- `test_is_after_new_keyword_multiple_spaces` — `"new   "` returns true
- `test_is_after_new_keyword_not_partial` — `"renew "` returns false
- `test_is_after_new_keyword_no_space` — `"new"` with no trailing space returns false
- `test_new_keyword_completions_filters_to_constructable` — struct included, enum/fn excluded

**mod.rs:** Added re-exports for both new functions.

### Task 2: Backend wiring and E2E tests

**`identifier_completion` signature change:**
- Added `source: &str` and `pos: lsp_types::Position` parameters
- Both call sites updated (main completion handler + dot-completion fallback)

**New dispatch logic:**
- Computes `byte_offset` from `source` + `pos`
- If `is_after_new_keyword(source, byte_offset)` → dispatches to `build_new_keyword_completions`
- Falls through to existing full identifier completion otherwise
- Returns empty list (not `None`) when cache unavailable in new-keyword context

**2 E2E tests added:**
- `test_completion_after_new_keyword` — asserts Point (struct) present; Color (enum), helper (fn), fn/let/if (keywords) absent
- `test_completion_not_after_new_still_returns_all` — asserts `new`, `if`, `true` keywords present in regular completion

## Verification

All 16 writ-lsp tests pass:
- 14 pre-existing tests: no regressions
- 2 new E2E tests: both pass
- 5 new unit tests: all pass

```
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Second identifier_completion call site in dot-completion fallback**
- **Found during:** Task 2 compilation
- **Issue:** The dot-completion path has a fallback at line 496 that also called `identifier_completion(&uri_str)` without the new `source` and `pos` parameters
- **Fix:** Updated that call site to pass `&source, pos` (both already in scope)
- **Files modified:** writ-lsp/src/backend.rs
- **Commit:** d840024

## Self-Check: PASSED

- `writ-lsp/src/queries/completion.rs` - modified (functions added + tests) - FOUND
- `writ-lsp/src/queries/mod.rs` - modified (re-exports added) - FOUND
- `writ-lsp/src/backend.rs` - modified (signature + dispatch logic) - FOUND
- `writ-lsp/tests/test_protocol.rs` - modified (2 E2E tests) - FOUND
- Commit c56032d (Task 1) - FOUND
- Commit d840024 (Task 2) - FOUND
