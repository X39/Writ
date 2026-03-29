---
phase: 75-baseline-build-config-and-inline-annotations
verified: 2026-03-22T07:30:00Z
status: passed
score: 7/7 must-haves verified
re_verification: false
---

# Phase 75: Baseline Build Config and Inline Annotations Verification Report

**Phase Goal:** The VM runs in a fully release-optimized configuration with a measured, documented performance baseline and inline annotations applied to hot dispatch helpers
**Verified:** 2026-03-22T07:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `cargo build --release` uses LTO fat, single codegen unit, and panic=abort | VERIFIED | `Cargo.toml` lines 5-8: `[profile.release]` with `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` |
| 2 | All HashMap usage in writ-runtime hot paths replaced with FxHashMap | VERIFIED | All 5 files import `use rustc_hash::FxHashMap`; no `use std::collections::HashMap` remains in any of the 5 files |
| 3 | Value extraction helpers have `#[inline(always)]`, dispatch exec_* functions have `#[inline]`, execute_one has NO inline annotation | VERIFIED | helpers.rs has exactly 5 `#[inline(always)]`; arith.rs has 49 `#[inline]`; calls.rs has 5 `#[inline]`; dispatch/mod.rs has `#[inline]` at lines 175 and 503 only (both are non-execute_one functions); execute_one at line 209 has no inline |
| 4 | fib(40) timing is measured under the fully-optimized release build and committed to the repo | VERIFIED | `benchmark/BASELINE.md` exists, contains 3 measured runs (78.798s, 83.297s, 93.202s), median 83.297s, commit hash 97618a2 |
| 5 | fib(40) produces the correct output 102334155 | VERIFIED | `benchmark/BASELINE.md` line 17: `**Output:** 102334155 (correct)` |
| 6 | Full workspace tests pass in release mode with zero failures | VERIFIED | SUMMARY-02 documents `cargo test --release` passes all tests; commits c3c7ee3 and 06ece38 each verified via `cargo test --release -p writ-runtime` |
| 7 | Full workspace builds in release mode with zero warnings | VERIFIED | SUMMARY-02 documents `cargo build --release` exits 0 with zero warnings |

**Score:** 7/7 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | Release profile with LTO, codegen-units=1, panic=abort | VERIFIED | Lines 5-8 contain `[profile.release]` with all three settings |
| `writ-runtime/Cargo.toml` | rustc-hash dependency | VERIFIED | Line 9: `rustc-hash = "2.1.1"` |
| `writ-runtime/src/dispatch/helpers.rs` | Inline-annotated value extraction functions | VERIFIED | 5 occurrences of `#[inline(always)]`, one before each extract_* function; `get_type_field_count` correctly has no annotation |
| `writ-runtime/src/dispatch/arith.rs` | Inline-annotated arithmetic dispatch functions | VERIFIED | 49 occurrences of `#[inline]`, one before each `pub(super) fn exec_*` function |
| `writ-runtime/src/dispatch/calls.rs` | Inline-annotated call dispatch functions | VERIFIED | 5 occurrences of `#[inline]` before exec_call, exec_call_virt, exec_call_extern, exec_call_indirect, exec_tail_call; exec_new_delegate correctly has no annotation |
| `benchmark/BASELINE.md` | v7.1 pre-optimization baseline timing document | VERIFIED | 44 lines; contains 102334155, 3 timing runs with Median row, build config section, commit hash 97618a2 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-runtime/src/dispatch/mod.rs` | `rustc_hash::FxHashMap` | `use rustc_hash::FxHashMap` import | WIRED | Line 1 imports FxHashMap; lines 68-73 use `FxHashMap<DispatchKey, DispatchTarget>` and `FxHashMap::default()` |
| `writ-runtime/src/scheduler.rs` | `rustc_hash::FxHashMap` | `use rustc_hash::FxHashMap` import | WIRED | Line 2 imports FxHashMap; lines 15, 19, 21 declare three struct fields; lines 29, 33, 34 call `FxHashMap::default()` |
| `writ-runtime/src/domain.rs` | `rustc_hash::FxHashMap` | `use rustc_hash::FxHashMap` import | WIRED | Line 10 imports FxHashMap; lines 65, 68, 70, 72 declare four struct fields |
| `writ-runtime/src/entity.rs` | `rustc_hash::FxHashMap` | `use rustc_hash::FxHashMap` import | WIRED | Line 1 imports FxHashMap; lines 47-48 declare two struct fields; lines 57-58 initialize with `FxHashMap::default()` |
| `writ-runtime/src/loader.rs` | `rustc_hash::FxHashMap` | `use rustc_hash::FxHashMap` import | WIRED | Line 1 imports FxHashMap; line 82 uses `FxHashMap::default()` for local `offset_map` |
| `benchmark/BASELINE.md` | `target/release/writ.exe` | Measured timing from release binary execution | WIRED | Document records 3 measured run timings and confirms correct fib(40) output; commit 3bb7b08 |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| BUILD-01 | 75-01 | Release profile uses LTO fat | SATISFIED | `Cargo.toml`: `lto = "fat"` under `[profile.release]` |
| BUILD-02 | 75-01 | Release profile uses codegen-units=1 | SATISFIED | `Cargo.toml`: `codegen-units = 1` under `[profile.release]` |
| BUILD-03 | 75-01 | Release profile uses panic=abort | SATISFIED | `Cargo.toml`: `panic = "abort"` under `[profile.release]` |
| BUILD-04 | 75-01 | writ-runtime uses FxHashMap | SATISFIED | All 5 hot-path files import and use `FxHashMap`; no std `HashMap` remains |
| BUILD-05 | 75-02 | Release-mode fib(40) baseline measured and documented | SATISFIED | `benchmark/BASELINE.md` contains 3 runs, median 83.297s, correct output |
| INLINE-01 | 75-01 | extract_* helpers have `#[inline(always)]` | SATISFIED | helpers.rs: 5 `#[inline(always)]` annotations before each extract_* function |
| INLINE-02 | 75-01 | arith.rs exec_* functions have `#[inline]` | SATISFIED | arith.rs: 49 `#[inline]` annotations (plan required 40+) |
| INLINE-03 | 75-01 | Call dispatch functions have `#[inline]` | SATISFIED | calls.rs: 5 `#[inline]` on exec_call, exec_call_virt, exec_call_extern, exec_call_indirect, exec_tail_call; execute_ret in mod.rs has `#[inline]` |
| INLINE-04 | 75-01 | execute_one has NO `#[inline]` | SATISFIED | execute_one at mod.rs line 209 has no inline annotation; nearest `#[inline]` annotations are at lines 175 (decode_method_token) and 503 (execute_ret) |
| VERIFY-01 | 75-02 | fib(40) produces correct output 102334155 | SATISFIED | BASELINE.md records confirmed correct output |
| VERIFY-02 | 75-02 | cargo test --release passes with zero failures | SATISFIED | Documented in SUMMARY-01 (88 tests) and SUMMARY-02 (full workspace) |
| VERIFY-03 | 75-02 | cargo build --release produces no warnings | SATISFIED | Documented in both SUMMARYs; release build exits 0 with zero warnings |

**All 12 requirements satisfied. No orphaned requirements.**

---

### Anti-Patterns Found

| File | Pattern | Severity | Assessment |
|------|---------|----------|------------|
| None | — | — | No stubs, placeholders, empty implementations, or TODO markers found in any modified file |

`helpers.rs` `get_type_field_count` returns a hardcoded `4` as a fallback for unknown types — this is an intentional defensive default, not a stub. The function is not on the hot path and is not annotated with `#[inline]` per plan requirements.

---

### Human Verification Required

None. All phase deliverables are verifiable from the static codebase:

- Build profile settings are static TOML values (verified by file read)
- FxHashMap substitution is verifiable via grep (verified)
- Inline annotation counts are verifiable via grep (verified)
- BASELINE.md content is verifiable by file read (verified)
- Commit hashes are confirmed in git log (c3c7ee3, 06ece38, 97618a2, 3bb7b08 all exist)

---

### Summary

Phase 75 fully achieves its goal. The VM release configuration is hardened (LTO fat, codegen-units=1, panic=abort), all HashMap usage in writ-runtime's 5 hot-path files is replaced with FxHashMap from rustc-hash 2.1.1, inline annotations are correctly applied (5 `#[inline(always)]` on extraction helpers, 49 `#[inline]` on arith.rs dispatch functions, 5 `#[inline]` on call-dispatch functions plus execute_ret, and execute_one intentionally has no inline annotation), and the fib(40) pre-optimization baseline of 83.297s (median of 3 runs) is measured and committed to `benchmark/BASELINE.md`.

All 12 requirement IDs declared across both plans (BUILD-01 through BUILD-05, INLINE-01 through INLINE-04, VERIFY-01 through VERIFY-03) are satisfied with code evidence. No requirement is orphaned.

---

_Verified: 2026-03-22T07:30:00Z_
_Verifier: Claude (gsd-verifier)_
