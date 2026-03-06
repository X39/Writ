---
phase: quick
plan: 260319-flh
subsystem: language-spec
tags: [spec, il-spec, option, result, intrinsic-methods]
dependency_graph:
  requires: []
  provides: ["Option<T> intrinsic method documentation", "Result<T,E> intrinsic method documentation"]
  affects: ["language-spec/spec/47_2_18_writ_runtime_module_contents.md"]
tech_stack:
  added: []
  patterns: ["Methods (intrinsic) table format matching §2.18.6 Array<T>"]
key_files:
  created: []
  modified:
    - language-spec/spec/47_2_18_writ_runtime_module_contents.md
decisions:
  - "Placed Method tables after the sugar paragraph (T?/null) in Option, and after the Specialized IL instructions sentence in Result, to keep prose flow logical"
  - "Used exact table column names and bold heading format from §2.18.6 Array<T> for consistency"
  - "Documented ! postfix operator equivalence inline after each table as explanatory prose"
metrics:
  duration: "72 seconds"
  completed_date: "2026-03-19"
  tasks_completed: 1
  files_modified: 1
---

# Quick Task 260319-flh: Document Intrinsic Methods on Option<T> and Result<T, E>

**One-liner:** Documented `is_some`, `is_none`, `unwrap`, `is_ok`, `is_err`, `unwrap_err` as intrinsic methods with IL mappings and `!` operator equivalence in §2.18.1.

## Summary

The IL spec section 2.18.1 previously defined Option and Result with tag assignments and a list of specialized IL instructions, but did not expose those instructions as callable methods. The compiler and LSP already implement these as intrinsic method calls. This task closes the spec-to-implementation gap.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add intrinsic method tables to Option and Result in §2.18.1 | 5a3f1ad | language-spec/spec/47_2_18_writ_runtime_module_contents.md |

## Changes Made

**Option\<T\> (§2.18.1):**
- Updated "specialized IL instructions" sentence to note methods are exposed — "see table below"
- Added `**Methods (intrinsic):**` table with `is_some` (`IS_SOME`), `is_none` (`IS_NONE`), `unwrap` (`UNWRAP`)
- Added prose: `unwrap` crashes on `None`; equivalent to `!` postfix operator (`opt.unwrap()` and `opt!` produce identical IL)

**Result\<T, E\> (§2.18.1):**
- Updated "Specialized IL instructions" sentence to note methods are exposed — "see table below"
- Added `**Methods (intrinsic):**` table with `is_ok` (`IS_OK`), `is_err` (`IS_ERR`), `unwrap` (`UNWRAP_OK`), `unwrap_err` (`EXTRACT_ERR`)
- Added prose: `unwrap` crashes on `Err`; equivalent to `!` postfix operator (`result.unwrap()` and `result!` produce identical IL)

Sections 2.18.2 through 2.18.8 are unchanged. The new tables use the same format as the existing Array\<T\> methods table in §2.18.6.

## Verification

All checks passed:
- 3 "Methods (intrinsic)" headings present (Option, Result, Array)
- Option table: 3 rows (is_some, is_none, unwrap)
- Result table: 4 rows (is_ok, is_err, unwrap, unwrap_err)
- 2 postfix operator equivalence paragraphs
- 2 "see table below" references
- All sections 2.18.2-2.18.8 intact

## Deviations from Plan

None - plan executed exactly as written.

## Self-Check: PASSED

- `language-spec/spec/47_2_18_writ_runtime_module_contents.md` — exists and contains all required content
- Commit `5a3f1ad` — present in git log
