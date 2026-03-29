---
phase: 86-lsp-support
verified: 2026-03-24T00:00:00Z
status: passed
score: 5/5 must-haves verified
re_verification: false
---

# Phase 86: LSP Contract-as-Type Support — Verification Report

**Phase Goal:** The language server provides correct completions, hover information, and diagnostics for contract-typed variables
**Verified:** 2026-03-24
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Dot-completion on a contract-typed variable shows the contract's methods | VERIFIED | `TyKind::Contract(def_id)` arm at line 295 of `completion.rs`; reads `type_env.contract_methods.get(def_id)`, emits METHOD items; `test_dot_completions_contract` and `test_dot_completion_integration_contract` both pass |
| 2 | Hovering over a contract declaration shows 'contract Name' tooltip | VERIFIED | `DefKind::Contract` arm at line 272 of `hover.rs`; returns `"```writ\ncontract {}\n```"`; `test_hover_contract_def` passes |
| 3 | Hovering over a contract-typed variable shows the contract type name | VERIFIED | `display_named` in `ty.rs` handles `TyKind::Contract(def_id)` (pre-existing since Phase 84); `hover_text_for_expr` calls `interner.display_named` for `TypedExpr::Var`, so contract type name appears in hover |
| 4 | Valid contract code produces zero diagnostics in LSP | VERIFIED | `test_diagnostics_contract_valid_no_errors` passes — 0 diagnostics for full contract+impl+call source |
| 5 | Invalid contract assignment produces error diagnostics in LSP | VERIFIED | `test_diagnostics_contract_invalid_produces_error` passes — at least one error-severity diagnostic for non-implementing type assignment |

**Score:** 5/5 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-lsp/src/queries/completion.rs` | `TyKind::Contract` arm in `build_dot_completions` | VERIFIED | Arm present at line 295; reads `contract_methods`, emits `CompletionItemKind::METHOD` items with `sort_text` prefix `"1_"` |
| `writ-lsp/src/queries/hover.rs` | `DefKind::Contract` arm in `hover_text_for_def` | VERIFIED | Arm present at line 272; returns `` ```writ\ncontract {name}\n``` `` |
| `writ-lsp/tests/test_protocol.rs` | Diagnostics regression tests for contract-typed code | VERIFIED | `test_diagnostics_contract_valid_no_errors` and `test_diagnostics_contract_invalid_produces_error` both present and passing |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-lsp/src/queries/completion.rs` | `type_env.contract_methods` | `FxHashMap<DefId, Vec<FnSig>>` lookup | WIRED | `contract_methods.get(def_id)` at line 296; result iterated to build `CompletionItem` entries |
| `writ-lsp/src/queries/hover.rs` | `DefKind::Contract` | match arm in `hover_text_for_def` | WIRED | Match arm present at line 272; produces non-empty tooltip string |

---

### Data-Flow Trace (Level 4)

The primary artifacts are query functions (not UI components), so the data-flow concern is whether contract method data reaches the completion response. Verified:

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `completion.rs` — contract arm | `type_env.contract_methods` | TypeEnv populated by `writ_compiler::check::typecheck` | Yes — `test_dot_completion_integration_contract` confirms method `"speak"` appears from real typecheck output | FLOWING |
| `hover.rs` — DefKind::Contract arm | `entry.name` | `def_map.get_entry(def_id).name` | Yes — `test_hover_contract_def` confirms `"Greetable"` appears in output | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Contract dot-completion unit test | `cargo test -p writ-lsp -- test_dot_completions_contract` | 1 passed | PASS |
| Contract dot-completion integration test | `cargo test -p writ-lsp -- test_dot_completion_integration_contract` | 1 passed | PASS |
| Contract hover def test | `cargo test -p writ-lsp -- test_hover_contract` | 1 passed | PASS |
| Diagnostics: valid contract no errors | `cargo test -p writ-lsp -- test_diagnostics_contract_valid_no_errors` | 1 passed | PASS |
| Diagnostics: invalid contract produces error | `cargo test -p writ-lsp -- test_diagnostics_contract_invalid_produces_error` | 1 passed | PASS |
| Full workspace suite | `cargo test --workspace` | 0 failures across all crates | PASS |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| LSP-01 | 86-01-PLAN.md | Dot-completion on contract-typed variables shows contract methods | SATISFIED | `TyKind::Contract` arm in `build_dot_completions`; 2 tests pass |
| LSP-02 | 86-01-PLAN.md | Hover on contract-typed variables shows the contract type name | SATISFIED | `DefKind::Contract` arm in `hover_text_for_def`; 1 test passes; variable-use hover pre-existing via `display_named` |
| LSP-03 | 86-01-PLAN.md | Diagnostics for contract-typed code work correctly (no false positives or missing errors) | SATISFIED | 2 wire-protocol tests pass; also required fixing 2 runtime bugs (CALL_VIRT crash + null ImplDef contract token) that were unblocking this requirement |

**Orphaned requirement check:** REQUIREMENTS.md traceability table assigns only LSP-01, LSP-02, LSP-03 to Phase 86. No additional IDs are mapped to this phase. No orphaned requirements.

---

### Additional Scope: Runtime Bug Fixes

The executor auto-fixed two pre-existing bugs found during Phase 86 testing, under Deviation Rule 1:

1. **CALL_VIRT crash on class instances** — `HeapObject::Struct` gained a `type_key: u32` field; `alloc_struct` signature updated; `exec_new` for Class kind computes and stores `(module_idx<<16)|typedef_idx`. Verified: `writ-runtime/src/heap.rs` line 15 shows `Struct { type_key: u32, fields: Vec<Value> }`; `dispatch/calls.rs` reads `Ok(HeapObject::Struct { type_key, .. }) => *type_key`.

2. **Null ImplDef contract token** — `writ-compiler/src/emit/collect/contracts.rs` now accepts `contractdef_handles` and resolves user-defined contract tokens from the handle map before `finalize()`; `collect/mod.rs` builds and passes `contractdef_handles`. Verified: both files contain the `contractdef_handles` lookups.

These fixes are correctness-critical for the phase goal — without them `test_diagnostics_contract_valid_no_errors` would crash at runtime.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | None found | — | — |

No TODOs, FIXMEs, placeholder returns, or stub implementations in any modified file.

---

### Human Verification Required

None. All five truths are verifiable programmatically and confirmed by passing tests.

---

### Gaps Summary

No gaps. All five must-have truths are verified at all four levels (exists, substantive, wired, data-flowing). All three requirement IDs are satisfied. The full workspace test suite is green with zero failures.

---

_Verified: 2026-03-24_
_Verifier: Claude (gsd-verifier)_
