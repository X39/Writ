---
phase: 41-fix-fn-log-say-choice
plan: 01
subsystem: testing
tags: [golden-tests, bom-strip, utf-16, harness, rust]

# Dependency graph
requires: []
provides:
  - Golden test harness with UTF-16 LE BOM stripping on read path
  - bless_golden fixed to write .writil extension (was .expected)
  - run_golden_test reads with std::fs::read + BOM strip + CRLF normalization
  - test_harness_bom_strip unit test for BOM-strip logic
  - test_fn_log_say_choice test function anchored to fn_log_say_choice.writil
  - UTF-8 placeholder fn_log_say_choice.writil (replaces corrupt UTF-16 LE file)
affects: [41-02, 41-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Binary read + BOM strip + CRLF normalize pattern for golden expected file reads

key-files:
  created:
    - writ-golden/tests/golden/fn_log_say_choice.writil (replaced with UTF-8 placeholder)
  modified:
    - writ-golden/tests/golden_tests.rs

key-decisions:
  - "strip_utf16le_bom strips only UTF-16 LE BOM (0xFF 0xFE) on read — never modifies files on disk"
  - "CRLF normalization applied after BOM strip so Windows-saved .writil files compare correctly against LF disassembler output"
  - "bless_golden writes .writil extension (not .expected) to match what run_golden_test reads"
  - "fn_log_say_choice.writil replaced with UTF-8 comment placeholder so test fails with mismatch not file-not-found"

patterns-established:
  - "Golden expected file read pattern: std::fs::read -> strip_utf16le_bom -> from_utf8 -> replace CRLF with LF"

requirements-completed: [BUG-01]

# Metrics
duration: 4min
completed: 2026-03-06
---

# Phase 41 Plan 01: Fix fn_log_say_choice Harness Summary

**Golden test harness hardened with UTF-16 LE BOM stripping, CRLF normalization, fixed bless extension (.expected -> .writil), and regression-anchor test for BUG-01 fix**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-06T14:06:30Z
- **Completed:** 2026-03-06T14:10:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Added `strip_utf16le_bom` helper and wired it into `run_golden_test` so UTF-16 LE hand-edited files compare correctly
- Fixed `bless_golden` to write `.writil` extension — was incorrectly writing `.expected`, creating a dead file nobody reads
- Added `test_harness_bom_strip` and `test_fn_log_say_choice` tests; all four Section C harness tests pass
- Replaced corrupt UTF-16 LE `fn_log_say_choice.writil` with clean UTF-8 placeholder so the new test reaches comparison stage (fails with parse error from BOM in source, not "file not found")
- Auto-fixed CRLF regression: switching from `read_to_string` to `read` (binary) exposed that existing `.writil` files use CRLF; added normalization to preserve pre-existing golden test results

## Task Commits

Each task was committed atomically:

1. **Task 1: Add strip_utf16le_bom, fix bless_golden extension, fix run_golden_test read path, add BOM-strip unit test** - `4812a76` (feat)
2. **Task 2: Add test_fn_log_say_choice and UTF-8 placeholder .writil** - `ec13a56` (feat)
3. **Auto-fix: CRLF normalization in run_golden_test** - `ebc2716` (fix)

**Plan metadata:** _(docs commit follows)_

## Files Created/Modified

- `writ-golden/tests/golden_tests.rs` - Added strip_utf16le_bom helper, fixed bless_golden extension, fixed run_golden_test to binary read + BOM strip + CRLF normalize, updated test_bless_writes_file assertion, added test_harness_bom_strip, added test_fn_log_say_choice
- `writ-golden/tests/golden/fn_log_say_choice.writil` - Replaced UTF-16 LE buggy snapshot with UTF-8 single-line placeholder comment

## Decisions Made

- Strip only UTF-16 LE BOM on read; never write BOM (bless_golden uses `std::fs::write` which produces UTF-8 without BOM)
- Normalize CRLF to LF after BOM-strip decode — necessary because Windows text editors save CRLF while the Rust disassembler emits LF
- Placeholder `.writil` replaces existing UTF-16 LE file rather than leaving it (the UTF-16 content would cause `from_utf8` to panic after BOM strip, defeating the goal of a clean mismatch failure)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] CRLF mismatch regression from binary read change**
- **Found during:** Task 2 verification (run full writ-golden suite)
- **Issue:** Switching from `read_to_string` (text mode, auto-normalizes CRLF on Windows) to `read` (binary) caused all existing golden tests to fail — expected files have CRLF, actual has LF
- **Fix:** Added `.replace("\r\n", "\n")` after `from_utf8` decode in `run_golden_test`
- **Files modified:** `writ-golden/tests/golden_tests.rs`
- **Verification:** Full `cargo test -p writ-golden` shows 8/9 pass; only `test_fn_log_say_choice` fails (expected — codegen not yet fixed)
- **Committed in:** `ebc2716`

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug introduced by plan's prescribed binary read change)
**Impact on plan:** Necessary for correctness. Restores 4 previously-passing golden tests. No scope creep.

## Issues Encountered

- The existing `fn_log_say_choice.writil` was UTF-16 LE (not absent) — plan assumed it was missing or could be overwritten. Replaced it with the UTF-8 placeholder per plan intent.
- `test_fn_log_say_choice` fails with "1 parse error(s): Some(found 'Error' at 0..3 expected declaration)" — this is the UTF-8 BOM in `fn_log_say_choice.writ` confusing the parser, consistent with the root cause identified in 41-CONTEXT.md. Plan 02 will address this.

## Next Phase Readiness

- Harness is ready for Plan 02 (codegen fix) — all infrastructure is in place
- Plan 02 must strip the UTF-8 BOM from `fn_log_say_choice.writ` source file (the parse error at 0..3 confirms this is the immediate blocker)
- After Plan 02 fixes codegen: run `BLESS=1 cargo test -p writ-golden -- test_fn_log_say_choice` for Plan 03

---
*Phase: 41-fix-fn-log-say-choice*
*Completed: 2026-03-06*

## Self-Check: PASSED

- `writ-golden/tests/golden_tests.rs` — FOUND
- `writ-golden/tests/golden/fn_log_say_choice.writil` — FOUND
- `.planning/phases/41-fix-fn-log-say-choice/41-01-SUMMARY.md` — FOUND
- Commit `4812a76` — FOUND
- Commit `ec13a56` — FOUND
- Commit `ebc2716` — FOUND
