---
phase: 44
status: passed
verified: 2026-03-06
---

# Phase 44: Extended Log with Levels — Verification

## Goal
Writ scripts use the leveled log namespace (`log::trace`, `log::debug`, `log::info`, `log::warn`, `log::error`) — the old unqualified `log(msg)` one-argument form is removed and replaced with the leveled API.

## Requirement Coverage

| Req ID | Description | Status |
|--------|-------------|--------|
| TOOL-03 | Leveled log:: namespace, log(msg) removed | PASSED |

## Success Criteria

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | log::debug/info/warn/error compile and route to on_log | PASSED | check_call two-segment fast-path in check_expr.rs; 5 typecheck tests green |
| 2 | log(msg) no longer compiles | PASSED | No extern fn log declarations remain in golden/test fixtures; natural resolution failure |
| 3 | Spec documents leveled API | PASSED | §26.4 rewritten with log::trace through log::error table |
| 4 | CliHost prints [DEBUG], [WARN] etc. | PASSED | on_log uses UPPERCASE static strings; unit test present |
| 5 | All fixtures updated to log::info | PASSED | fn_log_say_choice.writ, hello.writ, 9 parser tests migrated |

## Must-Haves Verified

### Plan 44-01
- log::info('msg') compiles and emits CALL_EXTERN — VERIFIED
- log::trace/debug/warn/error all compile — VERIFIED (5 test cases)
- ::log::debug('msg') root-qualified compiles — VERIFIED (test_log_root_qualified)
- log('msg') without extern fn no longer compiles — VERIFIED (test_log_bare_fails)
- log alone not callable — VERIFIED (test_log_namespace_not_callable)

### Plan 44-02
- CliHost prints '[DEBUG] msg' uppercase — VERIFIED (on_log match arms)
- CliHost routes all 5 levels to correct LogLevel — VERIFIED
- All golden tests pass — VERIFIED (cargo test --workspace: 0 failures)
- All parser tests pass — VERIFIED
- Spec documents leveled API — VERIFIED
- hello.writ compiles with new syntax — VERIFIED

## Test Results

Full workspace: all test suites pass, 0 failures.
