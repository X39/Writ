---
phase: 61-signature-help-diagnostics-extension-polish
verified: 2026-03-17T12:00:00Z
status: passed
score: 4/4 must-haves verified
re_verification: false
---

# Phase 61: Signature Help, Diagnostics, and Extension Polish Verification Report

**Phase Goal:** Signature help fires during real editing, top-level parse errors show squiggles, and entity names are visually distinct from struct names.
**Verified:** 2026-03-17T12:00:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| #   | Truth                                                                              | Status     | Evidence                                                                                                        |
| --- | ---------------------------------------------------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------- |
| 1   | Signature help returns Some when cursor is inside an incomplete call               | VERIFIED   | `extract_callee_name` in queries.rs (line 974), test `test_signature_help_incomplete_source` passes             |
| 2   | Active parameter index is correct after commas in incomplete calls                 | VERIFIED   | `comma_count` passed as `active_parameter` in primary text path; test `test_signature_help_active_param_incomplete` passes with `active_parameter == Some(1)` |
| 3   | Parse errors with zero-width spans produce visible squiggles                       | VERIFIED   | `saturating_sub(1)` expansion in `parse_error_to_diag` (convert.rs line 120); test `test_zero_width_span_expansion` passes with `entity Foo {` producing non-zero-width spans |
| 4   | Entity names and struct names use different semantic token scopes                  | VERIFIED   | `package.json` maps `entity` to `["support.class.writ"]` (not `entity.name.type.writ`); `configurationDefaults` provides `#4EC9B0` teal color under `[*]` wildcard |

**Score:** 4/4 truths verified

---

### Required Artifacts

| Artifact                        | Expected                                               | Status     | Details                                                                                                                 |
| ------------------------------- | ------------------------------------------------------ | ---------- | ----------------------------------------------------------------------------------------------------------------------- |
| `writ-lsp/src/queries.rs`       | Text-based callee name extraction for signature help   | VERIFIED   | Contains `fn extract_callee_name(source: &str, paren_offset: usize) -> Option<String>` at line 974. `build_signature_help` calls it as primary path before `find_enclosing_call` fallback (lines 1031-1088). Function is substantive: backward byte scan + DefMap lookup + FnSig resolution. |
| `writ-lsp/src/convert.rs`       | Zero-width span expansion in `parse_error_to_diag`    | VERIFIED   | Contains `saturating_sub` at line 120. Full expansion block: `raw_span.start == raw_span.end` branch with `start > 0` case and `start == 0` case. Used correctly in `parse_error_to_diag` before `Diagnostic::error(...).with_primary(file_id, span, ...)`. |
| `writ-vscode/package.json`      | Remapped semantic token scopes and configurationDefaults | VERIFIED | `semanticTokenScopes` entity scope is `["support.class.writ"]`. `configurationDefaults` block present with `editor.semanticTokenColorCustomizations` -> `[*]` -> rules with entity `#4EC9B0`, component `#9CDCFE`, dialogueSpeaker `#CE9178`. JSON is valid. |

---

### Key Link Verification

| From                            | To                                        | Via                                               | Status     | Details                                                                                                               |
| ------------------------------- | ----------------------------------------- | ------------------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------- |
| `writ-lsp/src/queries.rs`       | `DefMap.by_fqn` / `DefMap.file_private`   | `extract_callee_name` -> DefMap lookup -> fn_sigs | VERIFIED   | `extract_callee_name` called at line 1032. DefMap lookup at lines 1037-1048 (`ast.def_map.by_fqn.values()`, `ast.def_map.file_private.values()`). FnSig returned via `type_env.fn_sigs.get(&id)` at line 1051. Full chain present and substantive. |
| `writ-lsp/src/convert.rs`       | `lsp_types::Diagnostic` range             | span expansion before `with_primary`              | VERIFIED   | `saturating_sub(1)` expansion at line 120. Expanded `span` (not `raw_span`) is used in `Diagnostic::error(...).with_primary(file_id, span, ...)` at line 157. Link is complete. |
| `writ-vscode/package.json`      | VS Code semantic token renderer           | semanticTokenScopes + configurationDefaults       | VERIFIED   | `semanticTokenScopes[0].scopes.entity = ["support.class.writ"]` confirmed via `node -e` check. `configurationDefaults` present inside `"contributes"` as verified by Node.js JSON parse and `node -e` property access. |

---

### Requirements Coverage

| Requirement | Source Plan | Description                                                              | Status    | Evidence                                                                                                                       |
| ----------- | ----------- | ------------------------------------------------------------------------ | --------- | ------------------------------------------------------------------------------------------------------------------------------ |
| LSP-07      | 61-01-PLAN  | User sees signature help with active parameter highlighted during calls  | SATISFIED | `extract_callee_name` + text-based primary path in `build_signature_help` enables signature help on incomplete source. Tests `test_signature_help_incomplete_source` and `test_signature_help_active_param_incomplete` both pass. |
| LSP-01      | 61-01-PLAN  | Language server publishes diagnostics as inline editor squiggles         | SATISFIED | Zero-width span expansion in `parse_error_to_diag` ensures parse errors from EOF and recovery produce non-zero-width diagnostic ranges that VS Code can render. Test `test_zero_width_span_expansion` passes. |
| DIFF-01     | 61-01-PLAN  | Semantic highlighting distinguishes entity names, component types, etc.  | SATISFIED | `package.json` `semanticTokenScopes` maps entity to `support.class.writ` (distinct from struct's `entity.name.type`), and `configurationDefaults` guarantees color values in all themes. |

**Requirements from REQUIREMENTS.md traceability section for Phase 61:**
- `LSP-07 (gap) | Phase 61` — SATISFIED
- `LSP-01 (gap) | Phase 61` — SATISFIED
- `DIFF-01 (gap) | Phase 61` — SATISFIED

No orphaned requirements — all three IDs from the plan's `requirements` field are accounted for and satisfied.

---

### Anti-Patterns Found

No anti-patterns detected in the modified files.

- `writ-lsp/src/queries.rs` (lines 974-1088): No TODOs, FIXMEs, placeholders, empty handlers, or stub returns in the new code.
- `writ-lsp/src/convert.rs` (lines 113-134): No TODOs, FIXMEs, or stubs.
- `writ-vscode/package.json`: Valid JSON, no placeholder values.

---

### Human Verification Required

The following items cannot be fully verified programmatically and require manual VS Code testing to confirm the complete UAT scenario:

#### 1. Signature help UI in live editor session

**Test:** Open a `.writ` file in VS Code with the extension installed. Type `fn foo(a: int) -> int { a }` then on the next line type `fn main() {` and on the line below type `foo(` without a closing paren.
**Expected:** VS Code shows a signature help popup with `fn foo(a: int) -> int` and `a: int` highlighted as the active parameter.
**Why human:** The LSP `textDocument/signatureHelp` request is triggered by the editor on `(` keypress; the automated tests only call `build_signature_help` directly and do not exercise the full LSP request dispatch chain (`analysis_host.rs` -> `server.rs`).

#### 2. Squiggle visibility for top-level parse errors

**Test:** Open a `.writ` file and type `entity Foo {` without the closing `}` and save the file.
**Expected:** A red squiggle appears on the last character before the EOF error, not an invisible zero-width underline.
**Why human:** Automated test `test_zero_width_span_expansion` confirms the Diagnostic struct has non-zero-width `primary_span`, but rendering the squiggle requires the full LSP diagnostics publish path and the VS Code extension to be running.

#### 3. Visually distinct entity vs struct colors

**Test:** In VS Code with the extension installed, open a `.writ` file containing both a struct definition (e.g., `struct Foo {}`) and an entity definition (e.g., `entity Bar {}`). Observe the colors of `Foo` and `Bar` in usage sites.
**Expected:** `Bar` (entity) appears in teal (`#4EC9B0`) and `Foo` (struct) appears in a different color (e.g., green-yellow `entity.name.type` theme color), giving clear visual distinction.
**Why human:** Semantic token colors depend on the VS Code theme engine applying `configurationDefaults` and the `semanticTokenScopes` TextMate mapping. This requires a live VS Code session with the extension active.

---

### Commit Verification

Both commits documented in SUMMARY.md exist and are correct:

| Commit   | Description                                             | Files Changed                              |
| -------- | ------------------------------------------------------- | ------------------------------------------ |
| `c415b55` | feat(61-01): text-based signature help and zero-width span expansion | `writ-lsp/src/convert.rs`, `writ-lsp/src/queries.rs` |
| `534e2de` | feat(61-01): remap semantic token scopes and add color defaults in package.json | `writ-vscode/package.json` |

---

### Gaps Summary

No gaps. All four must-have truths are verified. All three requirement IDs (LSP-07, LSP-01, DIFF-01) are satisfied with substantive implementations that are wired end-to-end. The full writ-lsp test suite passes with 63 tests (4 new tests added in this phase). Three human-verification items remain but these require a live VS Code session and do not block automated goal assessment.

---

_Verified: 2026-03-17T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
