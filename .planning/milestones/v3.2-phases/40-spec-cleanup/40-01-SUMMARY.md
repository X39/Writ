---
phase: 40-spec-cleanup
plan: "01"
subsystem: spec
tags: [spec-cleanup, serialization, toc]
dependency_graph:
  requires: []
  provides: [SPEC-01]
  affects: [language-spec/spec/01_table_of_contents.md]
tech_stack:
  added: []
  patterns: []
key_files:
  created: []
  modified:
    - language-spec/spec/01_table_of_contents.md
  deleted:
    - language-spec/spec/37_2_8_serialization_critical_sections_removed.md
    - (entry removed) language-spec/spec/.split_config.json
decisions:
  - "Split config entry for deleted file also removed to prevent tooling breakage"
metrics:
  duration: "5 minutes"
  completed: "2026-03-06T13:10:00Z"
  tasks_completed: 1
  tasks_total: 1
  files_changed: 3
---

# Phase 40 Plan 01: Delete §2.8 Serialization Critical Sections Stub Summary

Deleted the §2.8 Serialization Critical Sections — REMOVED stub file and its TOC entry, fully erasing a superseded design that was replaced by the suspend-and-confirm model.

## What Was Done

### Files Deleted

- `language-spec/spec/37_2_8_serialization_critical_sections_removed.md` — The §2.8 stub that documented the now-superseded CRITICAL_BEGIN/CRITICAL_END design. Deleted entirely. The file number gap at 37_ is acceptable; splatted files are consumed alphabetically and gaps are fine.

### TOC Changes Made

Removed from `language-spec/spec/01_table_of_contents.md`:
```
  * [2.8 Serialization Critical Sections — REMOVED](#28-serialization-critical-sections--removed)
```
The surrounding entries (2.7 Operator Dispatch and 2.9 Memory Model) remain unchanged and adjacent.

### Section 3.16 Left Untouched

`language-spec/spec/64_3_16_serialization_control_removed.md` and its TOC entry (`3.16 Serialization Control — REMOVED`) were not modified. That section documents the removal of CRITICAL_BEGIN/CRITICAL_END opcodes from the instruction table, which is separate from the §2.8 design-level stub.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Removed dangling .split_config.json entry**
- **Found during:** Task 1
- **Issue:** `.split_config.json` contained an entry for the deleted file. If left in place, the split tool would attempt to read a non-existent file and fail.
- **Fix:** Removed the `37_2_8_serialization_critical_sections_removed.md` block from `.split_config.json`.
- **Files modified:** `language-spec/spec/.split_config.json`
- **Commit:** 736a1e3

## Verification Results

- `language-spec/spec/37_2_8_serialization_critical_sections_removed.md` does not exist on disk: PASS
- `language-spec/spec/01_table_of_contents.md` contains no line matching "2_8" or "2.8 Serialization Critical Sections": PASS
- `language-spec/spec/.split_config.json` contains no entry for the deleted file: PASS
- `language-spec/spec/64_3_16_serialization_control_removed.md` still exists: PASS
- TOC still contains the 3.16 entry: PASS
- Zero matches for "2_8" or "2.8 Serialization" across all spec files: PASS

## Self-Check: PASSED

All claimed changes verified:
- Deleted file confirmed absent from disk
- TOC entry confirmed absent from 01_table_of_contents.md
- Split config entry confirmed absent from .split_config.json
- 3.16 file and TOC entry confirmed present
- Commit 736a1e3 exists in git log
