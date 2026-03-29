---
phase: 96-conditional-semantic-effects
verified: 2026-03-27T21:30:00Z
status: passed
score: 7/7 must-haves verified
re_verification: false
---

# Phase 96: Conditional Semantic Effects Verification Report

**Phase Goal:** Functions marked `[Conditional("name")]` are emitted only when the named condition is active, with type-checking of call-site arguments always occurring regardless of elision, and a verified fallback function.
**Verified:** 2026-03-27T21:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A `[Conditional('x')]` fn with no matching-signature fallback produces E0009 | VERIFIED | `env.rs:320` E0009 diagnostic emitted in fallback verification pass; `code.rs:15` constant defined |
| 2 | The checker excludes conditional fn DefIds from candidate resolution, always resolving calls to the fallback | VERIFIED | `check_expr/mod.rs:504,518,534,539` — four filter sites in `find_fn_candidates` |
| 3 | `conditional_fns` and `fallback_for_conditional` maps are populated in TypeEnv and transferred to TypedAst | VERIFIED | `env.rs:72,75` fields; `mod.rs:100,101` transfer; `ir.rs:25,28` fields |
| 4 | With `--condition debug`, only the `[Conditional("debug")]` variant is emitted; fallback suppressed | VERIFIED | `conditional_active.writil` contains one `.method "greet"` with LOAD_STRING offset 110 ("Debug greeting"); `test_conditional_active` passes |
| 5 | With no active condition, only the fallback function is emitted; conditional variant absent | VERIFIED | `conditional_inactive.writil` contains one `.method "greet"` with LOAD_STRING offset 113 ("Default greeting"); `test_conditional_inactive` passes |
| 6 | Multiple active conditions matching the same fallback produce E0010 | VERIFIED | `collect/mod.rs:83-96` active_for_fallback map scanned post-skip; E0010 emitted when `active_conds.len() > 1` |
| 7 | Golden tests prove COND-01 and COND-02 via round-trip compile+disassemble | VERIFIED | `compile_and_disassemble_with_conditions` helper at `golden_tests.rs:104`; both tests pass (`cargo test -p writ-golden conditional` 2/2 ok) |

**Score:** 7/7 truths verified

---

### Required Artifacts

#### Plan 01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-diagnostics/src/code.rs` | E0009 and E0010 error codes | VERIFIED | Lines 15-16: `pub const E0009` and `pub const E0010` |
| `writ-compiler/src/check/env.rs` | `conditional_fns` and `fallback_for_conditional` fields on TypeEnv | VERIFIED | Lines 72, 75 — fields declared; lines 98, 99 — initialized; lines 234-319 — populated in build passes |
| `writ-compiler/src/check/env_build.rs` | `extract_conditional_name` helper | VERIFIED | Line 709: `pub(super) fn extract_conditional_name(attrs: &[AstAttribute]) -> Option<String>` |
| `writ-compiler/src/check/ir.rs` | `conditional_fns` and `fallback_for_conditional` on TypedAst | VERIFIED | Lines 25, 28 — both fields present with correct types |
| `writ-compiler/src/check/check_expr/mod.rs` | `find_fn_candidates` filters out conditional fn DefIds | VERIFIED | Lines 504, 518, 534, 539 — filter applied on all three return paths plus direct DefId check |

#### Plan 02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/emit/collect/mod.rs` | Emit-time filtering with `skipped_def_ids` | VERIFIED | Lines 65-96: pre-scan computes `skipped_def_ids`; line 127: skip check at Fn arm |
| `writ-cli/src/main.rs` | `--condition` CLI flag on Compile and Build subcommands | VERIFIED | Lines 62-78: `condition: Vec<String>` with `ArgAction::Append` on both subcommands |
| `writ-cli/src/pipeline.rs` | `active_conditions` parameter threaded through pipeline | VERIFIED | Lines 9, 21, 93: parameter declared, doc comment, and passed to `emit_bodies` |
| `writ-golden/tests/golden/conditional_active.writ` | Golden source for condition-active scenario | VERIFIED | File exists; 13 lines with `[Conditional("debug")]` greet + fallback greet + main |
| `writ-golden/tests/golden/conditional_inactive.writ` | Golden source for condition-inactive scenario | VERIFIED | Identical source to active (proves emit-time-only elision) |
| `writ-golden/tests/golden/conditional_active.writil` | Blessed output: one greet (debug variant) | VERIFIED | One `.method "greet"` with LOAD_STRING 110 ("Debug greeting"); `// .attribute` present |
| `writ-golden/tests/golden/conditional_inactive.writil` | Blessed output: one greet (fallback variant) | VERIFIED | One `.method "greet"` with LOAD_STRING 113 ("Default greeting"); no attribute comment |
| `writ-golden/tests/golden_tests.rs` | `compile_and_disassemble_with_conditions` helper and test entries | VERIFIED | Lines 104-187: helper defined; lines 883-895: `test_conditional_active` and `test_conditional_inactive` |

---

### Key Link Verification

#### Plan 01 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `check/env_build.rs` | `check/env.rs` | `extract_conditional_name` populates `TypeEnv.conditional_fns` | WIRED | `env.rs:244`: `env.conditional_fns.insert(def_id, cond_name)` — confirmed call path |
| `check/mod.rs` | `check/ir.rs` | `typecheck()` transfers conditional maps from TypeEnv to TypedAst | WIRED | `mod.rs:100-101`: both maps cloned into TypedAst after type-checking |
| `check/check_expr/mod.rs` | `check/env.rs` | `find_fn_candidates` reads `type_env.conditional_fns` to filter | WIRED | 4 filter sites; `contains_key` called against `ctx.type_env.conditional_fns` |

#### Plan 02 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-cli/src/main.rs` | `writ-cli/src/pipeline.rs` | CLI `--condition Vec<String>` converted to HashSet, passed to `run_pipeline` | WIRED | `commands/compile.rs:16,40`: HashSet constructed; `commands/build.rs:29-34,71`: merged with toml |
| `writ-cli/src/pipeline.rs` | `writ-compiler/src/emit/mod.rs` | `run_pipeline` passes `active_conditions` to `emit_bodies` | WIRED | `pipeline.rs:93`: `emit_bodies(..., active_conditions)` |
| `writ-compiler/src/emit/mod.rs` | `writ-compiler/src/emit/collect/mod.rs` | `emit_bodies` passes `active_conditions` to `collect_defs` | WIRED | `emit/mod.rs:90`: `collect::collect_defs(..., active_conditions)` |
| `writ-compiler/src/emit/collect/mod.rs` | `writ-compiler/src/check/ir.rs` | `collect_defs` reads `typed_ast.conditional_fns` to compute `skipped_def_ids` | WIRED | `collect/mod.rs:69`: iterates `&typed_ast.conditional_fns` in pre-scan |

---

### Data-Flow Trace (Level 4)

The phase output is not a UI/render component — it is a compiler pipeline that produces binary IL. Data flow is verified via the golden test round-trip: source writ file -> compile with active_conditions -> binary bytes -> Module::from_bytes -> disassemble -> compare with .writil snapshot. Both golden tests pass, confirming real data flows end-to-end.

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `conditional_active.writil` | `greet` MethodDef body | `collect_defs` with `active_conditions={"debug"}` | Yes — LOAD_STRING 110 = "Debug greeting" string pool offset | FLOWING |
| `conditional_inactive.writil` | `greet` MethodDef body | `collect_defs` with `active_conditions={}` | Yes — LOAD_STRING 113 = "Default greeting" string pool offset | FLOWING |
| `TypeEnv.conditional_fns` | DefId -> String map | Third pass in `TypeEnv::build` scanning `[Conditional]` attrs | Yes — populated from attribute scan | FLOWING |
| `TypeEnv.fallback_for_conditional` | DefId -> DefId map | Fourth pass signature-matching against overload set | Yes — populated or E0009 emitted | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Both conditional golden tests pass | `cargo test -p writ-golden conditional` | 2/2 ok | PASS |
| Full workspace test suite | `cargo test --workspace` | All result lines show `0 failed` across all test suites | PASS |
| Conditional fn DefIds filtered from candidates | `grep "conditional_fns.contains_key" writ-compiler/src/check/check_expr/mod.rs` | 4 matches | PASS |
| skipped_def_ids pre-scan present | `grep "skipped_def_ids" writ-compiler/src/emit/collect/mod.rs` | 4 matches | PASS |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| COND-01 | 96-02 | `[Conditional("name")]` function emits only the winning variant when condition active via `writ.toml` or `--condition` | SATISFIED | `test_conditional_active` passes; `conditional_active.writil` has one greet with "Debug greeting"; `--condition` CLI flag wired through pipeline |
| COND-02 | 96-02 | When no condition is active, the non-conditional fallback function is emitted | SATISFIED | `test_conditional_inactive` passes; `conditional_inactive.writil` has one greet with "Default greeting" |
| COND-03 | 96-01, 96-02 | Resolver verifies fallback exists with matching signature; emitter errors on multiple conditions matching same signature | SATISFIED | E0009 emitted by fallback verification pass (Plan 01); E0010 emitted by `active_for_fallback` check in `collect_defs` (Plan 02) |
| COND-04 | 96-01 | Arguments at a `[Conditional]` call site still type-check even when the call is elided | SATISFIED | `find_fn_candidates` filters conditional fns from resolution, always returning fallback DefId — type-checking runs against fallback's FnSig regardless of conditions |

All four requirements confirmed in REQUIREMENTS.md as `[x]` Complete.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | — |

No TODOs, FIXMEs, placeholders, or stub implementations found in the modified files (`env.rs`, `env_build.rs`, `ir.rs`, `mod.rs`, `check_expr/mod.rs`, `emit/collect/mod.rs`, `emit/mod.rs`, `pipeline.rs`, `main.rs`).

---

### Human Verification Required

None. All acceptance criteria are verifiable programmatically.

The golden snapshots (`conditional_active.writil`, `conditional_inactive.writil`) serve as the primary regression guard for emit-time correctness. Both are substantive: each contains exactly one `.method "greet"` block with distinct string pool offsets corresponding to "Debug greeting" and "Default greeting" respectively.

---

### Gaps Summary

No gaps. All 7 observable truths are verified, all 12 artifacts exist and are substantive, all 7 key links are wired, all 4 requirements are satisfied, and `cargo test --workspace` produces zero failures.

---

_Verified: 2026-03-27T21:30:00Z_
_Verifier: Claude (gsd-verifier)_
