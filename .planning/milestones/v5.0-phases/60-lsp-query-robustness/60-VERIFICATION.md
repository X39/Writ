---
phase: 60-lsp-query-robustness
verified: 2026-03-17T10:00:00Z
status: passed
score: 4/4 must-haves verified
---

# Phase 60: LSP Query Robustness Verification Report

**Phase Goal:** Hover, go-to-definition, and find-references work on variable declarations, type annotations, and definition-site names — not just expression nodes.
**Verified:** 2026-03-17
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| #  | Truth                                                                                   | Status     | Evidence                                                                                                           |
|----|-----------------------------------------------------------------------------------------|------------|--------------------------------------------------------------------------------------------------------------------|
| 1  | Hovering a variable name in a let binding shows its type (not void)                    | VERIFIED   | `binding_at_offset` in queries.rs (line 542) returns BindingInfo from `TypedStmt::Let::name_span`; wired into hover fallback in backend.rs lines 208-219 |
| 2  | Hovering a function parameter name shows its type                                       | VERIFIED   | `binding_at_offset` checks `TypedDecl::Fn::param_name_spans` + `TypeEnv.fn_sigs` (queries.rs lines 549-563); wired into hover handler |
| 3  | Go-to-definition on a type name in a let declaration jumps to the type's declaration    | VERIFIED   | `type_ann_def_id_at_offset` in queries.rs (line 668) reads `TypedStmt::Let::type_ann_def_id`; wired as second fallback in goto_definition handler (backend.rs lines 257-260) |
| 4  | Find-all-references from a function/struct/entity declaration name returns all references including the declaration itself | VERIFIED   | `def_at_offset` in queries.rs (line 650) skips FileId(u32::MAX) builtins; wired as fallback in references handler (backend.rs lines 337-340) |

**Score:** 4/4 truths verified

---

## Required Artifacts

| Artifact                                          | Expected                                                          | Status    | Details                                                                                                            |
|---------------------------------------------------|-------------------------------------------------------------------|-----------|--------------------------------------------------------------------------------------------------------------------|
| `writ-compiler/src/check/ir.rs`                   | TypedStmt::Let with `type_ann_span` and `type_ann_def_id` fields | VERIFIED  | Lines 246-249: `type_ann_span: Option<SimpleSpan>` and `type_ann_def_id: Option<DefId>` both present with doc comments |
| `writ-compiler/src/check/ir.rs`                   | TypedDecl::Fn with `param_name_spans: Vec<SimpleSpan>` field      | VERIFIED  | Lines 292-298: `param_name_spans: Vec<SimpleSpan>` present with doc comment                                        |
| `writ-compiler/src/check/check_stmt.rs`           | TypedStmt::Let constructed with `type_ann_span` and `type_ann_def_id` | VERIFIED  | Lines 92-93: both fields populated from annotation span/DefId extraction; `AstType` imported at line 4             |
| `writ-compiler/src/check/check_decl.rs`           | TypedDecl::Fn constructed with `param_name_spans` from AstFnDecl  | VERIFIED  | Lines 92-99: `param_name_spans` built by filtering `fn_decl.params` for Regular and SelfParam variants            |
| `writ-lsp/src/queries.rs`                         | `BindingInfo` struct, `binding_at_offset`, `def_at_offset`, `type_ann_def_id_at_offset` | VERIFIED  | All four present at lines 532, 542, 650, 668 respectively                                                          |
| `writ-lsp/src/backend.rs`                         | Fallback chains in hover, goto_definition, and references handlers | VERIFIED  | Lines 208-219 (hover), 257-265 (goto_def two fallbacks), 337-340 (references one fallback)                         |

---

## Key Link Verification

| From                                    | To                                        | Via                                                             | Status  | Details                                                                                        |
|-----------------------------------------|-------------------------------------------|-----------------------------------------------------------------|---------|-----------------------------------------------------------------------------------------------|
| `writ-lsp/src/backend.rs`               | `writ-lsp/src/queries.rs`                 | `binding_at_offset`, `def_at_offset`, `type_ann_def_id_at_offset` fallback calls | WIRED   | Lines 209, 259, 264, 339 — all three functions called with correct arguments including `type_env` for `binding_at_offset` |
| `writ-compiler/src/check/check_stmt.rs` | `writ-compiler/src/check/ir.rs`           | TypedStmt::Let construction with `type_ann_span` and `type_ann_def_id` | WIRED   | Fields set at lines 92-93 from local `ann_span`/`ann_def_id` computed at lines 27-65          |
| `writ-lsp/src/queries.rs`               | `writ-compiler/src/check/ir.rs`           | TypedStmt::Let destructuring for `binding_at_offset` and `type_ann` queries | WIRED   | Line 608 (`name_span` in find_binding_in_stmt), line 715 (`type_ann_span`/`type_ann_def_id` in find_type_ann_in_stmt) |

---

## Requirements Coverage

| Requirement     | Source Plan | Description                                                              | Status    | Evidence                                                                                                               |
|-----------------|-------------|--------------------------------------------------------------------------|-----------|-----------------------------------------------------------------------------------------------------------------------|
| LSP-04 (gap)    | 60-01-PLAN  | User can hover any identifier to see its type, signature, or definition info | SATISFIED | `binding_at_offset` covers let-binding names and fn param names; hover fallback wired in backend.rs                   |
| LSP-05 (gap)    | 60-01-PLAN  | User can go-to-definition on any identifier to jump to its declaration   | SATISFIED | `type_ann_def_id_at_offset` and `def_at_offset` cover type annotations and declaration names; both wired in goto_definition |
| LSP-06 (gap)    | 60-01-PLAN  | User can find all references of a definition across all files            | SATISFIED | `def_at_offset` fallback wired in references handler; `include_declaration` path already uses `entry.name_span`       |

**Note:** `REQUIREMENTS.md` lines 116-118 still show LSP-04/05/06 (gap) as "Pending". The code implementation is complete and verified, but the tracking table was not updated to "Complete" after phase execution. This is a documentation tracking gap only — the code delivers all three requirements.

---

## Anti-Patterns Found

| File                                          | Line | Pattern      | Severity | Impact                                                                                 |
|-----------------------------------------------|------|--------------|----------|----------------------------------------------------------------------------------------|
| `writ-compiler/src/check/check_decl.rs`       | 196  | `placeholder` | Info     | Pre-existing comment in impl method processing: "Use the impl_def_id as a placeholder for the method DefId" — describes real logic, not a code stub. Not introduced by this phase. |

No blockers or warnings found. The "placeholder" comment is a pre-existing code comment describing a legitimate design decision, not introduced by phase 60.

---

## Human Verification Required

### 1. Hover on let-binding name in VS Code

**Test:** Open a .writ file, write `let x: int = 42;`, position cursor on the `x` identifier.
**Expected:** Hover popup shows `x: int` in a markdown code block.
**Why human:** LSP protocol exchange, editor UI popup rendering, and byte-offset-to-position conversion cannot be verified by grep alone.

### 2. Go-to-definition on type annotation in VS Code

**Test:** Write `struct MyStruct {} fn main() { let x: MyStruct = new MyStruct {}; }`, position cursor on the `MyStruct` type annotation in the let binding.
**Expected:** Editor jumps to the `struct MyStruct {}` declaration.
**Why human:** Requires live LSP client-server exchange and file navigation.

### 3. Find-references from a declaration name in VS Code

**Test:** Define `fn foo() {}` and call `foo()` elsewhere, position cursor on the `foo` in the function declaration line.
**Expected:** Find-references returns both the declaration and the call site.
**Why human:** Requires live LSP client with `includeDeclaration: true` context flag.

---

## Test Results

All automated tests passed:

| Test Suite                                          | Tests | Status |
|-----------------------------------------------------|-------|--------|
| `cargo test -p writ-lsp binding_at_offset --lib`    | 4     | passed |
| `cargo test -p writ-lsp def_at_offset --lib`        | 2     | passed |
| `cargo test -p writ-lsp type_ann --lib`             | 2     | passed |
| `cargo test -p writ-compiler --test emit_body_tests`| 94    | passed |
| `cargo test -p writ-lsp -p writ-compiler` (full)    | 446   | passed |

Commits verified in repository: `3d5e6ec` (Task 1) and `18b9b65` (Task 2).

---

## Gaps Summary

No implementation gaps. All four must-have truths verified, all six artifacts substantive and wired, all three key links confirmed active. The only outstanding item is:

1. **REQUIREMENTS.md tracking not updated** (documentation only): Lines 116-118 still list LSP-04/05/06 (gap) as "Pending". This does not affect implementation correctness — it is a docs-only update needed in REQUIREMENTS.md.

---

_Verified: 2026-03-17T10:00:00Z_
_Verifier: Claude (gsd-verifier)_
