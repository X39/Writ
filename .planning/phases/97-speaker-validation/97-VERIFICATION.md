---
phase: 97-speaker-validation
verified: 2026-03-27T22:00:00Z
status: passed
score: 4/4 must-haves verified
gaps: []
human_verification: []
---

# Phase 97: Speaker Validation Verification Report

**Phase Goal:** Dialogue blocks using `@speaker` syntax validate that the named speaker is a [Singleton] entity; non-singleton and non-existent entity speakers produce errors
**Verified:** 2026-03-27T22:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth                                                                             | Status     | Evidence                                                                                        |
|----|-----------------------------------------------------------------------------------|------------|-------------------------------------------------------------------------------------------------|
| 1  | `@speaker` targeting a [Singleton] entity compiles with no speaker-related diagnostics | ✓ VERIFIED | `singleton_speaker_valid` test asserts zero E0007; passes in resolve_tests run                  |
| 2  | `@speaker` targeting a non-[Singleton] entity produces E0007 with the entity name  | ✓ VERIFIED | `non_singleton_speaker_emits_e0007` asserts exactly one E0007 containing "Merchant"; passes     |
| 3  | `@speaker` targeting a non-existent entity produces E0003 (UnresolvedName)         | ✓ VERIFIED | `nonexistent_speaker_emits_error` asserts exactly one E0003 containing "Ghost"; passes          |
| 4  | Contract-typed param speakers (`dlg greet(npc: Entity)`) produce no false E0007    | ✓ VERIFIED | `contract_speaker_no_false_e0007` asserts zero E0007; passes — Tier 1 speakers never hoisted   |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact                                         | Expected                         | Status     | Details                                                                                    |
|--------------------------------------------------|----------------------------------|------------|--------------------------------------------------------------------------------------------|
| `writ-compiler/src/resolve/validate.rs`          | `validate_speakers()` implementation | ✓ VERIFIED | Full implementation (293 lines): `collect_entity_sets`, `collect_entities_in_items`, `validate_speakers_in_items`, `check_stmts_for_speakers`, `extract_get_or_create_entity` — not a stub |
| `writ-compiler/tests/resolve_tests.rs`           | Speaker validation test cases    | ✓ VERIFIED | 4 tests under `// === Speaker validation (Phase 97) ===` section; all pass                 |

### Key Link Verification

| From                                       | To                                      | Via                                        | Status    | Details                                                                                         |
|--------------------------------------------|-----------------------------------------|--------------------------------------------|-----------|-------------------------------------------------------------------------------------------------|
| `validate.rs`                              | `def_map.rs`                            | `DefKind::Entity` entity lookup            | ✓ WIRED   | `collect_entities_in_items` matches `AstDecl::Entity` branches directly — does not use `DefMap` at runtime but the entity-set approach is equivalent and correct |
| `validate.rs`                              | `error.rs`                              | `ResolutionError::InvalidSpeaker` / `UnresolvedName` emission | ✓ WIRED   | Lines 233-250 in validate.rs push both error variants; `From<ResolutionError>` in error.rs converts to `Diagnostic` |
| `resolve/mod.rs:114`                       | `validate::validate_speakers`           | Call in resolve pipeline                  | ✓ WIRED   | `validate::validate_speakers(asts, &def_map, &mut diags)` already present at line 114          |

### Data-Flow Trace (Level 4)

Not applicable — this phase produces diagnostics (compile-time errors), not rendered data. The "data" is diagnostic output flowing from `validate_speakers()` into the `diags` vec, which is passed by mutable reference and later consumed by callers. Verified by test assertions on diagnostic codes and message content.

### Behavioral Spot-Checks

| Behavior                                        | Command                                             | Result                               | Status  |
|-------------------------------------------------|-----------------------------------------------------|--------------------------------------|---------|
| 4 speaker validation tests pass                 | `cargo test -p writ-compiler speaker`               | 4 passed, 0 failed                   | ✓ PASS  |
| Full compiler test suite — zero regressions     | `cargo test -p writ-compiler`                       | 429 passed (across all binaries), 0 failed | ✓ PASS  |
| Golden tests — zero regressions                 | `cargo test -p writ-golden`                         | 48 passed, 0 failed                  | ✓ PASS  |

### Requirements Coverage

| Requirement | Source Plan | Description                                                                     | Status      | Evidence                                                                                            |
|-------------|-------------|---------------------------------------------------------------------------------|-------------|-----------------------------------------------------------------------------------------------------|
| SPKR-01     | 97-01-PLAN  | `@speaker` targeting a non-[Singleton] entity produces E0007                   | ✓ SATISFIED | `non_singleton_speaker_emits_e0007` test; `InvalidSpeaker` emission in `check_stmts_for_speakers`; E0007 code constant in `writ-diagnostics/src/code.rs:13` |
| SPKR-02     | 97-01-PLAN  | `@speaker` targeting a non-existent entity produces an error                   | ✓ SATISFIED | `nonexistent_speaker_emits_error` test; `UnresolvedName` emission in `check_stmts_for_speakers` for entities not in `all_entities` set |

No orphaned requirements — REQUIREMENTS.md marks both SPKR-01 and SPKR-02 as `[x]` complete and maps them to Phase 97.

### Anti-Patterns Found

| File                                          | Line | Pattern                  | Severity | Impact |
|-----------------------------------------------|------|--------------------------|----------|--------|
| `writ-compiler/src/resolve/validate.rs`       | 112  | `_def_map` unused param  | Info     | The `DefMap` parameter is accepted for API consistency but not used; entity sets are built directly from ASTs. No functional issue — the parameter is intentionally unused per the two-set design. |

No blockers or warnings. The `_def_map` underscore-prefix intentionally suppresses the unused-variable warning. The implementation uses direct AST traversal (two-set approach) which is correct and sufficient.

### Human Verification Required

None — all speaker validation behaviors are fully verifiable programmatically via the test suite.

### Gaps Summary

No gaps. All four observable truths are verified. Both requirements (SPKR-01, SPKR-02) are satisfied. Tests pass, no regressions in compiler or golden suites.

---

_Verified: 2026-03-27T22:00:00Z_
_Verifier: Claude (gsd-verifier)_
