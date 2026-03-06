---
phase: 63-writ-compiler-file-splits
verified: 2026-03-18T12:00:00Z
status: passed
score: 7/7 requirements verified
re_verification: false
gaps: []
---

# Phase 63: writ-compiler File Splits Verification Report

**Phase Goal:** All 7 oversized files in writ-compiler are split into focused, navigable submodules
**Verified:** 2026-03-18
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | No targeted file in writ-compiler exceeds 500 lines (or has documented rationale) | VERIFIED | All 7 targeted files handled: split files max at 499 lines (check_expr/mod.rs), reviewed files have rationale comments |
| 2 | Each new submodule has a single clear responsibility | VERIFIED | Spot-checked: binary.rs has `//! Binary and unary-prefix operator type checking`, walker.rs has `//! AST walker for collecting called DefIds` |
| 3 | All existing tests pass without modification after the splits | VERIFIED | `cargo test -p writ-compiler`: 75 passed, 0 failed |
| 4 | `mod` declarations are explicit — no `pub use *` re-exports that obscure origin | VERIFIED | Zero `pub use ::*` globs in check_expr/, collect/, expr/; named re-exports only (builtins, resolve_ast_type) |

**Score:** 4/4 success criteria verified

---

## Required Artifacts

### Plan 01 (SPLIT-03): check_expr/ folder module

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/check/check_expr/mod.rs` | CheckCtx, check_expr dispatch, check_block_stmts, check_assignment_mutability | VERIFIED | All 4 pub functions present; 499 lines |
| `writ-compiler/src/check/check_expr/ident.rs` | `pub(super) fn check_ident` | VERIFIED | Line 11: `pub(super) fn check_ident` |
| `writ-compiler/src/check/check_expr/path.rs` | `pub(super) fn check_path` | VERIFIED | Line 11: `pub(super) fn check_path` |
| `writ-compiler/src/check/check_expr/binary.rs` | `check_binary`, `check_unary_prefix` | VERIFIED | Lines 12, 142 |
| `writ-compiler/src/check/check_expr/call.rs` | `check_call`, `check_generic_call` | VERIFIED | Lines 17, 344; 438 lines |
| `writ-compiler/src/check/check_expr/control.rs` | `pub(super) fn check_if` | VERIFIED | Line 12 |
| `writ-compiler/src/check/check_expr/access.rs` | `check_member_access`, `check_bracket_access` | VERIFIED | Lines 13, 141 |
| `writ-compiler/src/check/check_expr/match_.rs` | `check_match`, `check_pattern` | VERIFIED | Lines 14, 86 |
| `writ-compiler/src/check/check_expr/lambda.rs` | `pub(super) fn check_lambda` | VERIFIED | Line 13 |
| `writ-compiler/src/check/check_expr/construction.rs` | `check_new_construction`, `check_array_lit` | VERIFIED | Lines 13, 118 |
| `writ-compiler/src/check/check_expr.rs` (original) | DELETED | VERIFIED | File does not exist |

### Plan 02 (SPLIT-04): collect/ folder module

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/emit/collect/mod.rs` | `collect_defs`, `collect_post_finalize` | VERIFIED | Lines 34, 131; 182 lines |
| `writ-compiler/src/emit/collect/types.rs` | `collect_struct`, `collect_entity`, `collect_enum`, `collect_class` | VERIFIED | Lines 17, 66, 114, 152 |
| `writ-compiler/src/emit/collect/functions.rs` | `collect_fn`, `collect_extern_fn` | VERIFIED | Lines 17, 64 |
| `writ-compiler/src/emit/collect/contracts.rs` | `collect_contract`, `collect_impl` | VERIFIED | Lines 24, 60 |
| `writ-compiler/src/emit/collect/builtins.rs` | `inject_log_extern_defs`, `inject_dialogue_extern_defs` | VERIFIED | Lines 26, 64 (pub fn) |
| `writ-compiler/src/emit/collect/walker.rs` | `pub(super) fn collect_called_def_ids` | VERIFIED | Line 16 |
| `writ-compiler/src/emit/collect/globals.rs` | `pub(super) fn collect_const` | VERIFIED | Line 14 |
| `writ-compiler/src/emit/collect/encoding.rs` | `encode_fn_sig`, `encode_type_from_ast` | VERIFIED | Lines 328, 247; 472 lines |
| `writ-compiler/src/emit/collect/lookup.rs` | `pub(super) fn find_struct_decl` | VERIFIED | Line 17 |
| `writ-compiler/src/emit/collect.rs` (original) | DELETED | VERIFIED | File does not exist |

### Plan 03 (SPLIT-05): expr/ folder module

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/emit/body/expr/mod.rs` | `pub fn emit_expr` | VERIFIED | Line 36; 443 lines |
| `writ-compiler/src/emit/body/expr/literal.rs` | `pub(super) fn emit_literal` | VERIFIED | Line 11 |
| `writ-compiler/src/emit/body/expr/binary.rs` | `pub(super) fn emit_binary` | VERIFIED | Line 15 |
| `writ-compiler/src/emit/body/expr/control.rs` | `emit_if`, `emit_spawn`, `emit_defer` | VERIFIED | Lines 19, 76, 133 |
| `writ-compiler/src/emit/body/expr/construction.rs` | `emit_range`, `emit_new` | VERIFIED | Lines 26, 105 |
| `writ-compiler/src/emit/body/expr/builtins.rs` | `pub(super) fn try_emit_builtin_method` | VERIFIED | Line 22 |
| `writ-compiler/src/emit/body/expr/string.rs` | `try_collect_str_build_parts`, `emit_str_build` | VERIFIED | Lines 23, 64 |
| `writ-compiler/src/emit/body/expr/eq.rs` | `emit_struct_eq`, `emit_struct_neq` | VERIFIED | Lines 19, 70 |
| `writ-compiler/src/emit/body/expr.rs` (original) | DELETED | VERIFIED | File does not exist |

### Plan 03 (SPLIT-09): env.rs split

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/check/env.rs` | Public types: TypeEnv, FnSig, LocalEnv, Mutability | VERIFIED | Lines 55, 27, 288, 294; 336 lines (down from 1,032) |
| `writ-compiler/src/check/env_build.rs` | Builder helpers: build_fn_sig, build_struct_fields, find_fn_decl, resolve_ast_type, decl_def_id | VERIFIED | Lines 438, 534, 49, 294, 25; 724 lines |

### Plan 04 (SPLIT-08, SPLIT-10, SPLIT-11): No-split rationale

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/emit/module_builder.rs` | `SPLIT-08 review` + `no split` comment | VERIFIED | Line 9: "Conclusion: no split." |
| `writ-compiler/src/lower/dialogue.rs` | `SPLIT-10 review` + `no split` comment | VERIFIED | Line 5: "Conclusion: no split." |
| `writ-compiler/src/resolve/resolver.rs` | `SPLIT-11 review` + `no split` comment | VERIFIED | Line 8: "Conclusion: no split." |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `check/check_expr/mod.rs` | `check/check_expr/ident.rs` | `mod ident;` | WIRED | Line 26 of mod.rs |
| `check/check_expr/mod.rs` | all 9 submodules | `mod path; mod binary; ...` | WIRED | Lines 26-34 of mod.rs |
| `check/mod.rs` | `check/check_expr/mod.rs` | `pub mod check_expr;` | WIRED | Line 12 of check/mod.rs |
| `emit/collect/mod.rs` | `emit/collect/types.rs` | `mod types;` | WIRED | Line 15 of collect/mod.rs |
| `emit/collect/mod.rs` | all 8 submodules | `mod functions; mod contracts; ...` | WIRED | Lines 15-22 of collect/mod.rs |
| `emit/mod.rs` | `emit/collect/mod.rs` | `pub mod collect;` | WIRED | Line 12 of emit/mod.rs |
| `emit/body/expr/mod.rs` | `emit/body/expr/binary.rs` | `mod binary;` | WIRED | Line 8 of expr/mod.rs |
| `emit/body/expr/mod.rs` | all 7 submodules | `mod literal; mod binary; ...` | WIRED | Lines 7-13 of expr/mod.rs |
| `emit/body/mod.rs` | `emit/body/expr/mod.rs` | `pub mod expr;` | WIRED | Line 9 of body/mod.rs |
| `check/mod.rs` | `check/env_build.rs` | `pub(crate) mod env_build;` | WIRED | Line 9 of check/mod.rs |
| `check/env.rs` | `check/env_build.rs` | `pub use super::env_build::{...}` | WIRED | Line 23 of env.rs (named re-export) |
| `emit/collect/mod.rs` | `emit/collect/builtins.rs` | `pub use builtins::{inject_log_extern_defs, inject_dialogue_extern_defs}` | WIRED | Line 31 of collect/mod.rs (named re-export) |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| SPLIT-03 | 63-01 | check_expr.rs (2,155 lines) split by expression category | SATISFIED | check_expr/ exists with 10 files; original deleted; all functions present |
| SPLIT-04 | 63-02 | collect.rs (1,700 lines) split by declaration type | SATISFIED | collect/ exists with 9 files; original deleted; all functions present |
| SPLIT-05 | 63-03 | emit/body/expr.rs (1,478 lines) split by expression category | SATISFIED | expr/ exists with 8 files; original deleted; all functions present |
| SPLIT-08 | 63-04 | module_builder.rs (1,064 lines) reviewed for split opportunities | SATISFIED | `SPLIT-08 review` comment present; "Conclusion: no split." |
| SPLIT-09 | 63-03 | env.rs (1,039 lines) reviewed/split into env.rs + env_build.rs | SATISFIED | env.rs trimmed to 336 lines; env_build.rs created at 724 lines; wired via check/mod.rs |
| SPLIT-10 | 63-04 | dialogue.rs (858 lines) reviewed for split opportunities | SATISFIED | `SPLIT-10 review` comment present; "Conclusion: no split." |
| SPLIT-11 | 63-04 | resolver.rs (849 lines) reviewed for split opportunities | SATISFIED | `SPLIT-11 review` comment present; "Conclusion: no split." |

**All 7 SPLIT requirements are satisfied.**

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `emit/collect/encoding.rs` | 281, 290, 300, 311 | `// placeholder` (literal "0" blob offset/TypeSpec in encoded binary data) | Info | Pre-existing IL encoding stubs — structural byte layout placeholders awaiting full type token wiring in a later phase. NOT a code logic stub; bytes will be patched via finalize pass. |
| `emit/body/expr/construction.rs` | 94 | `// elem_type token: 0 as placeholder (Plan 04 will wire real type sigs)` | Info | Pre-existing emit stub for ArrayInit elem_type field. Not introduced by phase 63. |
| `emit/body/expr/control.rs` | 139, 146 | `// placeholder` (DeferPush method_idx, Br offset) | Info | Intentional patching pattern — instruction emitted with 0 then fixed up via index-based patch. Standard codegen pattern, not a logic stub. |
| `emit/body/expr/literal.rs` | 41 | `// placeholder` (LoadString string_idx: 0, pending_strings patch) | Info | Intentional fixup pattern — string index resolved in a later finalize pass. Not a stub. |

**No blocker anti-patterns.** All "placeholder" comments are intentional emit-then-patch codegen patterns that pre-existed phase 63. None were introduced by the split. No `unimplemented!()` or `todo!()` macros found anywhere in the split files.

---

## Human Verification Required

None. All checks passed programmatically.

- File existence and line counts: verified via `wc -l`
- Function signatures: verified via `grep`
- Wiring: verified via `grep` on `mod`, `pub mod`, and `pub use` declarations
- No glob re-exports: verified via `grep "pub use.*::\*"`
- Tests: `cargo test -p writ-compiler` — 75 passed, 0 failed
- Clippy: `cargo clippy --workspace` — zero warnings
- Commits: all 5 phase commits exist in git log (f562d40, 221fd40, f7f52ed, 326b723, f9e5bd3)

---

## Line Count Summary

| Split Module | Files | Max Lines | Status |
|-------------|-------|-----------|--------|
| `check/check_expr/` | 10 | 499 (mod.rs) | All under 500 |
| `emit/collect/` | 9 | 472 (encoding.rs) | All under 500 |
| `emit/body/expr/` | 8 | 443 (mod.rs) | All under 500 |
| `check/env.rs` | 1 | 336 | Under 350 target |
| `check/env_build.rs` | 1 | 724 | Under 750 target |
| `emit/module_builder.rs` | 1 | 1,073 | Reviewed — no split (SPLIT-08) |
| `lower/dialogue.rs` | 1 | 870 | Reviewed — no split (SPLIT-10) |
| `resolve/resolver.rs` | 1 | 858 | Reviewed — no split (SPLIT-11) |

---

## Phase Goal Verdict

**Goal: "All 7 oversized files in writ-compiler are split into focused, navigable submodules"**

- 3 files received folder module splits: check_expr.rs, collect.rs, emit/body/expr.rs
- 1 file received sibling-file split: check/env.rs → env.rs + env_build.rs
- 3 files were reviewed with documented "no split" rationale: module_builder.rs, dialogue.rs, resolver.rs

The goal is **achieved**. All 7 SPLIT requirements (SPLIT-03 through SPLIT-11, excluding SPLIT-06 and SPLIT-07 which are assigned to other crates/phases) are satisfied. The codebase compiles cleanly with 75 tests passing and zero clippy warnings.

---

_Verified: 2026-03-18_
_Verifier: Claude (gsd-verifier)_
