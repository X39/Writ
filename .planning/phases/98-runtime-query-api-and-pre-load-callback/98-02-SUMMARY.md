---
phase: 98-runtime-query-api-and-pre-load-callback
plan: 02
subsystem: runtime
tags: [writ-runtime, attribute-system, domain, query-api, metadata]

# Dependency graph
requires:
  - phase: 98-01
    provides: ModuleAttributeView, AttributeMatch, on_module_load pre-load callback

provides:
  - DomainAttributeMatch struct with module_idx, name, args, owner, owner_kind
  - Domain::query_attributes(attr_name) cross-module attribute lookup
  - Domain::query_attributes_on(module_idx, typedef_idx) per-type query
  - Domain::query_attribute_value(module_idx, owner_token, attr_name) single-value lookup
  - 7 integration tests in attr_query_tests.rs proving all three methods

affects:
  - Any host code that loads user modules and needs to inspect attributes at runtime
  - Phase 98 complete (QAPI-01, QAPI-02, QAPI-03 satisfied)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Domain attribute query: iterate modules.iter().enumerate(), filter owner_kind != ATTR_OWNER_KIND_DECL
    - Name comparison: read_string(...).ok() == Some(attr_name) avoids PartialEq on DecodeError
    - decode_row_args: blob offset 0 = empty vec (null blob), non-zero = read_blob + decode_attr_args, errors = empty vec

key-files:
  created: []
  modified:
    - writ-runtime/src/domain.rs
    - writ-runtime/src/lib.rs
    - writ-runtime/tests/attr_query_tests.rs

key-decisions:
  - "decode_row_args is a private free function (not a method), placed near the impl Domain query block for locality"
  - "query_attributes iterates all modules, accumulating matches with their module_idx — callers need no separate lookup"
  - "query_attribute_value returns Option<Vec<AttrValue>> with None for both out-of-range module_idx and missing attribute"

requirements-completed: [QAPI-01, QAPI-02, QAPI-03]

# Metrics
duration: 2min
completed: 2026-03-27
---

# Phase 98 Plan 02: Domain Query API Summary

**Domain query methods (query_attributes, query_attributes_on, query_attribute_value) with DomainAttributeMatch struct, allowing hosts to inspect attribute metadata on loaded modules at runtime**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-03-27T21:41:17Z
- **Completed:** 2026-03-27T21:43:27Z
- **Tasks:** 2 (both TDD; Task 2 tests appended to attr_query_tests.rs from Task 1)
- **Files modified:** 3

## Accomplishments

- `DomainAttributeMatch` struct in `writ-runtime/src/domain.rs` with `module_idx`, `name`, `args`, `owner`, `owner_kind`
- `Domain::query_attributes(&str)` — scans all loaded modules, returns all matching application rows
- `Domain::query_attributes_on(usize, usize)` — returns all attributes on a specific TypeDef by 0-based index
- `Domain::query_attribute_value(usize, MetadataToken, &str)` — returns `Option<Vec<AttrValue>>` for a specific attribute on a specific token
- `decode_row_args` private helper: null-blob safe (offset 0 = empty vec), decode failures return empty vec (no panics)
- `DomainAttributeMatch` re-exported from `writ-runtime/src/lib.rs`
- 7 integration tests appended to `writ-runtime/tests/attr_query_tests.rs` (total: 12 tests in file)
- Full workspace `cargo test` green

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Add failing tests for Domain query methods** — `db8d343` (test)
2. **Task 1+2 GREEN: Add Domain query methods and DomainAttributeMatch** — `6b10fd9` (feat)

_TDD tasks: 2 commits (1x RED + 1x GREEN covering both tasks)._

## Files Created/Modified

- `writ-runtime/src/domain.rs` — DomainAttributeMatch struct, query_attributes, query_attributes_on, query_attribute_value, decode_row_args helper
- `writ-runtime/src/lib.rs` — DomainAttributeMatch re-export added to pub use domain line
- `writ-runtime/tests/attr_query_tests.rs` — 7 new tests appended (domain_query_* family)

## Decisions Made

- `decode_row_args` is a private free function rather than a method — keeps it close to the query impl block in domain.rs, mirrors the `build_match`/`decode_args` pattern from ModuleAttributeView in host.rs
- `query_attributes` iterates all modules with `enumerate()` so each `DomainAttributeMatch` carries its `module_idx` — callers don't need a secondary lookup to know which module owns the match
- `query_attribute_value` returns `None` for both out-of-range `module_idx` and missing attribute name — consistent with the "gracefully handle missing data" contract from the plan

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None.

---
## Self-Check: PASSED

All files verified on disk. Both task commits (db8d343, 6b10fd9) present in git history.

*Phase: 98-runtime-query-api-and-pre-load-callback*
*Completed: 2026-03-27*
