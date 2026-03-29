---
phase: 95-deprecated-semantic-effects
verified: 2026-03-27T21:30:00Z
status: passed
score: 6/6 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 5/6
  gaps_closed:
    - "Hovering over a deprecated item in the LSP shows the deprecation message in the hover tooltip (TypedExpr::New arm)"
  gaps_remaining: []
  regressions: []
---

# Phase 95: Deprecated Semantic Effects Verification Report

**Phase Goal:** Referencing a deprecated item produces a compiler warning with the user's message string and the LSP surfaces that warning as a diagnostic with the message in hover
**Verified:** 2026-03-27T21:30:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure (commit 00f8a6d)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Calling a `[Deprecated('msg')]` function from a different file produces a W0006 warning containing the user's message | VERIFIED | `deprecated_call_cross_file_emits_w0006` passes; call.rs lines 340-357 emit W0006 |
| 2 | Referencing a deprecated const or type from a different file produces a W0006 warning | VERIFIED | `deprecated_construction_cross_file_emits_w0006` passes; construction.rs lines 62-80 and ident.rs `emit_deprecated_warning_if_cross_file` cover all non-call sites |
| 3 | Referencing a deprecated item from the same file produces NO warning | VERIFIED | `deprecated_call_same_file_no_warning` passes; file_id comparison in call.rs and ident.rs confirmed |
| 4 | Bare `[Deprecated]` with no message arg produces a W0006 warning with a default message | VERIFIED | `deprecated_bare_call_cross_file_default_message` passes; bare maps to empty string, formatted as `` `foo` is deprecated `` |
| 5 | LSP shows DiagnosticSeverity::Warning squiggle at call site of deprecated item | VERIFIED | Severity::Warning -> DiagnosticSeverity::WARNING in convert.rs; W0006 emitted as Severity::Warning; LSP pipeline confirmed |
| 6 | Hovering over a deprecated item in the LSP shows the deprecation message in the hover tooltip | VERIFIED | Commit 00f8a6d added `deprecation_notice()` call to `TypedExpr::New` arm (hover.rs lines 141-143); all four arms (Var, Const, Call, New) now consistent; 2 LSP hover tests pass |

**Score:** 6/6 truths verified

### Re-verification Focus: TypedExpr::New Arm (Previously Gap)

The previous verification identified that `hover_text_for_expr`'s `TypedExpr::New` arm (hover.rs line 138) returned the base format string without calling `deprecation_notice()`.

Commit 00f8a6d applied the fix:

```
-            format!("```writ\nnew {}\n```", entry.name)
+            let base = format!("```writ\nnew {}\n```", entry.name);
+            if let Some(notice) = deprecation_notice(*target_def_id, type_env) {
+                return format!("{}\n\n{}", notice, base);
+            }
+            base
```

The pattern is now consistent with the Call arm (lines 121-123) and Var/Const arms (lines 72-74, 94-95). The `deprecation_notice` helper at lines 18-26 queries `type_env.deprecated_items.get(&def_id)` and formats the result as `**Deprecated**` or `**Deprecated:** <msg>`.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-diagnostics/src/code.rs` | W0006 constant | VERIFIED | `pub const W0006: &str = "W0006"; // deprecated item reference` |
| `writ-compiler/src/check/env.rs` | `deprecated_items` field on TypeEnv | VERIFIED | `pub deprecated_items: FxHashMap<DefId, String>` |
| `writ-compiler/src/check/env_build.rs` | `extract_deprecated_msg` helper + population logic | VERIFIED | `extract_deprecated_msg` and `find_attrs_for_entry` helpers; populated in env.rs second pass |
| `writ-compiler/src/check/check_expr/call.rs` | W0006 emission at call sites | VERIFIED | Lines 340-357: W0006 emitted when `entry.file_id != ctx.current_file` |
| `writ-compiler/src/check/check_expr/ident.rs` | W0006 emission for non-call ident references | VERIFIED | `emit_deprecated_warning_if_cross_file` helper; called from check_ident |
| `writ-compiler/src/check/check_expr/construction.rs` | W0006 emission for construction sites | VERIFIED | Lines 62-80: W0006 emitted when constructed type is deprecated and from different file |
| `writ-compiler/tests/deprecated_tests.rs` | Integration tests | VERIFIED | 7/7 tests pass |
| `writ-lsp/src/queries/hover.rs` | Deprecation notice in hover tooltips | VERIFIED | `deprecation_notice()` called in Var (72), Const (94), Call (121), and New (141) arms; `hover_text_for_def` (line 314) also augmented |
| `writ-lsp/tests/test_protocol.rs` | LSP integration tests for deprecated hover | VERIFIED | `test_deprecated_hover_on_declaration` and `test_deprecated_hover_on_call_site` pass |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `env_build.rs` | `env.rs` | `deprecated_items.insert` in TypeEnv::build second pass | WIRED | Confirmed in previous verification; no changes |
| `check_expr/call.rs` | `env.rs` | `ctx.type_env.deprecated_items.get(&def_id)` | WIRED | Confirmed; no changes |
| `hover.rs` | `env.rs` | `deprecation_notice(def_id, type_env)` in all four expr arms | WIRED | Previously partial (New arm missing); now fully wired after commit 00f8a6d |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `deprecated_tests.rs` | `diags` (Vec<Diagnostic>) | typecheck pipeline -> W0006 emission | Yes — W0006 with user message produced for cross-file references | FLOWING |
| `test_protocol.rs` hover tests | hover response text | `hover_text_for_expr` -> `deprecation_notice` -> `type_env.deprecated_items` | Yes — "Deprecated" and "use bar instead" asserted present | FLOWING |
| `hover.rs` TypedExpr::New arm | `notice` (Option<String>) | `deprecation_notice(*target_def_id, type_env)` | Yes — same pipeline as Call/Var/Const arms | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 7 compiler deprecated tests pass | `cargo test -p writ-compiler` (deprecated_tests.rs) | 7/7 pass | PASS |
| 2 LSP deprecated hover tests pass | `cargo test -p writ-lsp deprecated` | 2/2 pass | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DEPR-01 | 95-01-PLAN.md | Referencing a `[Deprecated("msg")]` item produces a compiler warning containing the user's message string | SATISFIED | W0006 emitted in call.rs, ident.rs, and construction.rs; 7 integration tests pass covering all reference sites and same-file suppression |
| DEPR-02 | 95-02-PLAN.md | LSP surfaces `[Deprecated]` as `DiagnosticSeverity::Warning` and shows the deprecation message on hover | SATISFIED | DiagnosticSeverity::WARNING pipeline confirmed; hover notice present in all four `hover_text_for_expr` arms (Var, Const, Call, New) after commit 00f8a6d; 2 LSP tests pass |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `writ-lsp/tests/test_protocol.rs` | — | No test specifically exercises `TypedExpr::New` hover with a deprecated struct/entity | Info | The code fix is correct and structurally identical to the Call arm; absence of a dedicated regression test is a minor coverage gap but does not block goal achievement |

### Human Verification Required

None. All goal-critical behaviors are verified programmatically via compiler integration tests and LSP protocol tests.

### Gaps Summary

No gaps. The single gap from the previous verification (TypedExpr::New arm missing `deprecation_notice()` call in `hover_text_for_expr`) was closed in commit 00f8a6d. All six must-have truths are now verified. Both DEPR-01 and DEPR-02 requirements are fully satisfied.

---

_Verified: 2026-03-27T21:30:00Z_
_Verifier: Claude (gsd-verifier)_
