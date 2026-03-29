---
phase: 98-runtime-query-api-and-pre-load-callback
plan: 01
subsystem: runtime
tags: [writ-runtime, attribute-system, module-loading, host-callback, metadata]

# Dependency graph
requires:
  - phase: 93-blob-encoding-foundation
    provides: encode_attr_args/decode_attr_args in writ-module/src/attr.rs
  - phase: 94-attribute-declarations-and-builtin-names
    provides: ATTR_OWNER_KIND_DECL=3, AttributeDefRow, attribute_defs in Module

provides:
  - ModuleAttributeView<'a> struct with query_attributes, query_attributes_on, query_attribute_value
  - AttributeMatch struct with name, args, owner, owner_kind fields
  - RuntimeHost::on_module_load default method (Ok(()) - backward compatible)
  - Pre-load hook wired in RuntimeBuilder::build for user module only
  - Integration tests proving callback fires, view works, rejection works

affects:
  - 98-02 (query_attributes API on running runtime)
  - Any future RuntimeHost implementors (they now have on_module_load available)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Pre-load inspection: host receives read-only ModuleAttributeView before domain.add_module
    - ATTR_OWNER_KIND_DECL filtering: query methods never return declaration rows
    - Blob offset 0 = no args: decode_args returns empty Vec without calling read_blob

key-files:
  created:
    - writ-runtime/tests/attr_query_tests.rs
  modified:
    - writ-runtime/src/host.rs
    - writ-runtime/src/runtime.rs
    - writ-runtime/src/lib.rs

key-decisions:
  - "ModuleAttributeView::new is pub(crate) — only RuntimeBuilder creates instances; hosts only receive &ModuleAttributeView"
  - "build() takes mut self so the host can be called mutably before being consumed by Runtime"
  - "Pre-load hook fires before domain.add_module so the module is never partially loaded on rejection"

patterns-established:
  - "Attribute query pattern: filter owner_kind != ATTR_OWNER_KIND_DECL, compare name via .ok() == Some(attr_name)"
  - "Blob decode pattern: offset 0 = empty vec, non-zero = read_blob then decode_attr_args, errors = empty vec"

requirements-completed: [QAPI-04, QAPI-05, QAPI-06]

# Metrics
duration: 6min
completed: 2026-03-27
---

# Phase 98 Plan 01: ModuleAttributeView and Pre-Load Callback Summary

**ModuleAttributeView with attribute query API and on_module_load pre-load callback on RuntimeHost, allowing hosts to inspect and reject user modules before they enter the domain**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-03-27T21:32:37Z
- **Completed:** 2026-03-27T21:38:22Z
- **Tasks:** 2 (both TDD)
- **Files modified:** 4

## Accomplishments

- `AttributeMatch` and `ModuleAttributeView<'a>` structs in `writ-runtime/src/host.rs`
- `query_attributes(name)`, `query_attributes_on(typedef_idx)`, `query_attribute_value(token, name)` on the view
- `on_module_load` default method on `RuntimeHost` — all existing implementations compile unchanged
- Pre-load hook wired in `RuntimeBuilder::build` before `domain.add_module(user_module)`
- Rejection returns `RuntimeError::LoadError("module rejected by host: <reason>")`
- 5 integration tests in `attr_query_tests.rs` covering all behaviors

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Add failing tests for ModuleAttributeView** - `bee5208` (test)
2. **Task 1 GREEN: ModuleAttributeView, AttributeMatch, on_module_load** - `a263bcb` (feat)
3. **Task 2 RED: Add failing integration tests for pre-load callback** - `31d45cb` (test)
4. **Task 2 GREEN: Wire pre-load hook in RuntimeBuilder::build** - `98f539b` (feat)

_TDD tasks produced 4 commits (2x RED + 2x GREEN)._

## Files Created/Modified

- `writ-runtime/src/host.rs` — AttributeMatch, ModuleAttributeView<'a>, on_module_load default on RuntimeHost
- `writ-runtime/src/runtime.rs` — pre-load hook before domain.add_module; build() changed to mut self
- `writ-runtime/src/lib.rs` — re-exports for ModuleAttributeView and AttributeMatch
- `writ-runtime/tests/attr_query_tests.rs` — 5 integration tests for pre-load callback

## Decisions Made

- `build()` takes `mut self` so the host field can be borrowed mutably for `on_module_load` before the builder is consumed into the `Runtime` struct
- `ModuleAttributeView::new` is `pub(crate)` — hosts only ever receive `&ModuleAttributeView<'_>`, never construct one directly
- Pre-load hook fires before `domain.add_module` so a rejected module never partially enters the domain

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed ModuleBuilder::new call in test helper**
- **Found during:** Task 1 GREEN (first compile attempt)
- **Issue:** Plan interface doc showed `ModuleBuilder::new("name", "version")` but the actual API is `ModuleBuilder::new("name")` (version is a builder method)
- **Fix:** Changed test helper to `ModuleBuilder::new("TestModule")` (single arg)
- **Files modified:** writ-runtime/src/host.rs
- **Verification:** Compiled and tests passed
- **Committed in:** a263bcb (Task 1 feat commit)

**2. [Rule 1 - Bug] Fixed Result comparison in query filter**
- **Found during:** Task 1 GREEN (first compile attempt)
- **Issue:** `DecodeError` does not implement `PartialEq`, so `read_string(...) == Ok(attr_name)` failed to compile
- **Fix:** Changed to `read_string(...).ok() == Some(attr_name)` which compares `Option<&str>` (which does implement PartialEq)
- **Files modified:** writ-runtime/src/host.rs
- **Verification:** Compiled cleanly
- **Committed in:** a263bcb (Task 1 feat commit)

**3. [Rule 1 - Bug] Added mut self to RuntimeBuilder::build**
- **Found during:** Task 2 GREEN (first compile attempt)
- **Issue:** `on_module_load` requires `&mut self` but `build(self)` didn't declare `mut`, so the host borrow was rejected by the borrow checker
- **Fix:** Changed signature to `pub fn build(mut self)`
- **Files modified:** writ-runtime/src/runtime.rs
- **Verification:** Compiled and all tests passed
- **Committed in:** 98f539b (Task 2 feat commit)

---

**Total deviations:** 3 auto-fixed (all Rule 1 - compile-time bugs from doc/API mismatch)
**Impact on plan:** All fixes resolved immediately at compile time. No scope creep.

## Issues Encountered

- Cargo test filter `attr_query` matched test binary names but not test function names — had to use `--test attr_query_tests` to run integration tests directly

## Next Phase Readiness

- Pre-load callback fully operational — hosts can inspect and reject user modules before any code executes
- `ModuleAttributeView` API established and tested — ready for use in 98-02 (runtime query API)
- All existing `RuntimeHost` implementors (NullHost, ExternHost, CliHost, DAP backend) compile without changes

---
## Self-Check: PASSED

All files found on disk. All 4 task commits verified in git history.

*Phase: 98-runtime-query-api-and-pre-load-callback*
*Completed: 2026-03-27*
