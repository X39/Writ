---
phase: 41-fix-fn-log-say-choice
verified: 2026-03-06T15:00:00Z
status: passed
score: 4/4 must-haves verified
re_verification: false
---

# Phase 41: Fix fn_log_say_choice Verification Report

**Phase Goal:** The fn_log_say_choice golden test compiles to non-empty method bodies; the stored snapshot is re-blessed with spec-correct IL in UTF-8 encoding
**Verified:** 2026-03-06
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `cargo test -p writ-golden` passes — fn_log_say_choice.writil contains non-empty method bodies with IL for log, say, and choice calls | VERIFIED | 9/9 golden tests pass; .writil has 4x CALL_EXTERN + RET_VOID in `main` body |
| 2 | Root cause of empty-method-body codegen failure is identified and documented | VERIFIED | 41-NOTES.md exists with 5-step root cause chain; root cause confirmed to be check_path :: normalization failure |
| 3 | Stored snapshot is UTF-8 encoded, no FF FE BOM — hex check confirms | VERIFIED | First 4 bytes: 2E 6D 6F 64 (`.mod`); no UTF-16 LE BOM; no UTF-8 BOM |
| 4 | Re-blessed IL is produced by compile_and_disassemble round-trip and matches snapshot byte-for-byte | VERIFIED | `BLESS=1 cargo test` wrote snapshot via bless_golden; subsequent `cargo test` passes (byte-for-byte match confirmed by green test) |

**Score:** 4/4 truths verified

---

### Required Artifacts

All artifacts checked at three levels: exists, substantive, wired.

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-golden/tests/golden/fn_log_say_choice.writil` | Blessed golden snapshot with CALL_EXTERN IL | VERIFIED | 27 lines; 4x CALL_EXTERN (log/say/choice); RET_VOID; UTF-8 no BOM |
| `writ-golden/tests/golden/fn_log_say_choice.writ` | BOM-free Writ source for golden test | VERIFIED | First byte: 0x70 ('p'); UTF-8 BOM (EF BB BF) absent; declares extern fn log/say/choice |
| `writ-golden/tests/golden_tests.rs` | Updated harness with BOM-strip, fixed bless extension, new test functions | VERIFIED | strip_utf16le_bom function at line 106; bless_golden writes .writil at line 119; test_fn_log_say_choice at line 314; test_harness_bom_strip at line 263 |
| `writ-compiler/src/check/check_expr.rs` | Fixed check_path with :: normalization; path fast-path in check_call | VERIFIED | Normalization block at lines 454-463; path fast-path in check_call at lines 726-739 |
| `language-spec/spec/27_26_standard_library_builtins.md` | Spec note on root-qualified inbuilt call forms | VERIFIED | Line 50: "The root-qualified forms `::log`, `::say`, and `::choice`..." |
| `.planning/phases/41-fix-fn-log-say-choice/41-NOTES.md` | Root cause documentation for BUG-01 | VERIFIED | Documents 5-step chain, fix applied, ancillary fixes, scope boundary |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `test_fn_log_say_choice` | `run_golden_test` | calls run_golden_test("fn_log_say_choice") | VERIFIED | Line 315 in golden_tests.rs: `run_golden_test("fn_log_say_choice")` |
| `run_golden_test` | `strip_utf16le_bom` | std::fs::read then strip before String::from_utf8 | VERIFIED | Lines 134-152 in golden_tests.rs: binary read, strip_utf16le_bom, from_utf8, CRLF normalize |
| `bless_golden` | `.writil` extension | format!("{name}.writil") | VERIFIED | Line 119: `golden_dir.join(format!("{name}.writil"))` |
| `check_path` | `normalized_segments` / `ctx.def_map.get` | strip_prefix("::") then join, then DefMap lookup | VERIFIED | Lines 454-464: normalization block followed by `ctx.def_map.get(&fqn)` |
| `check_call` path fast-path | `check_call_with_sig` | AstExpr::Path single-segment, strip ::, find_fn_def_id | VERIFIED | Lines 726-739: path fast-path sets callee_def_id enabling CALL_EXTERN |
| `fn_log_say_choice.writil` | `test_fn_log_say_choice` | run_golden_test reads .writil and compares | VERIFIED | Test passes (byte-for-byte match); file read at golden_dir.join("{name}.writil") |

---

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| BUG-01 | 41-01, 41-02, 41-03 | fn_log_say_choice golden test codegen failure diagnosed and fixed — method bodies non-empty; snapshot re-blessed with spec-correct IL in UTF-8 encoding | SATISFIED | 9/9 golden tests pass; .writil has CALL_EXTERN; UTF-8 confirmed; root cause in 41-NOTES.md |

**Orphaned requirements check:** REQUIREMENTS.md maps only BUG-01 to Phase 41. All three plans claim BUG-01. No orphaned requirements.

---

### Anti-Patterns Found

No blockers or warnings found.

| File | Pattern | Severity | Notes |
|------|---------|----------|-------|
| — | — | — | No TODO/FIXME/placeholder anti-patterns in any phase-41-modified files |

The pre-existing `// Could be an enum variant path -- Stub for now` comment in check_expr.rs is noted in 41-NOTES.md as out-of-scope and deferred to Phase 43. It does not affect the BUG-01 fix.

---

### Human Verification Required

None. All goal criteria are programmatically verifiable and confirmed:

- Test suite run confirmed live (9/9 pass)
- Encoding verified via byte-level inspection
- CALL_EXTERN instructions confirmed present in .writil
- check_path normalization code confirmed in source
- Spec note confirmed in spec file

---

### Gaps Summary

No gaps. All four ROADMAP success criteria are satisfied.

---

## Verification Detail

### Test Run Results (Live)

```
running 9 tests
test test_harness_bom_strip ... ok
test test_harness_fail_shows_diff ... ok
test test_bless_writes_file ... ok
test test_harness_pass ... ok
test test_fn_empty_main ... ok
test test_fn_basic_call ... ok
test test_fn_log_say_choice ... ok
test test_fn_typed_params ... ok
test test_fn_recursion ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

writ-compiler: 65/65 tests pass.

### Encoding Verification

- `fn_log_say_choice.writil` first 4 bytes: `2E 6D 6F 64` (`.mod`) — no BOM
- `fn_log_say_choice.writ` first 4 bytes: `70 75 62 20` (`pub `) — no BOM

### fn_log_say_choice.writil Contents Verification

File contains `main` method body with:
- 4x CALL_EXTERN instructions (log/say/log/choice)
- 3x LOAD_STRING instructions
- 1x RET_VOID
- Total: 8 instructions — non-empty confirmed

### Key Commits (All Verified in git log)

| Hash | Plan | Description |
|------|------|-------------|
| 4812a76 | 41-01 | feat: add strip_utf16le_bom, fix bless_golden extension, fix run_golden_test read path |
| ec13a56 | 41-01 | feat: add test_fn_log_say_choice and UTF-8 placeholder .writil |
| ebc2716 | 41-01 | fix: CRLF normalization in run_golden_test |
| ef2c054 | 41-02 | test: add failing tests for check_path :: normalization (RED) |
| 205cddf | 41-02 | feat: fix check_path to normalize root-qualified path segments |
| 1b5c95d | 41-02 | feat: strip UTF-8 BOM from fn_log_say_choice.writ and add spec note |
| e11209b | 41-03 | feat: bless fn_log_say_choice golden snapshot with correct CALL_EXTERN IL |
| 23e0005 | 41-03 | docs: write 41-NOTES.md documenting BUG-01 root cause and fix |

---

_Verified: 2026-03-06T15:00:00Z_
_Verifier: Claude (gsd-verifier)_
