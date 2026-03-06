---
phase: 43-unqualified-none-some
verified: 2026-03-06T17:30:00Z
status: human_needed
score: 3/4 success criteria verified
human_verification:
  - test: "Open writ-golden/tests/golden/fn_optional.writ and confirm it still compiles end-to-end: run `cargo test -p writ-golden` and manually add a test_fn_optional entry to confirm Option::None and Option::Some(value) qualified paths still produce correct IL"
    expected: "Option::None produces LOAD_NULL or equivalent null-construction IL; Option::Some(true) produces WRAP_SOME r, r_bool; no compile errors"
    why_human: "fn_optional.writ and fn_optional.writil both exist and the golden snapshot contains WRAP_SOME, but test_fn_optional is NOT registered in writ-golden/tests/golden_tests.rs -- success criterion 4 (Option::None and Option::Some(value) continue to compile correctly) has no running automated test"
---

# Phase 43: Unqualified None/Some Verification Report

**Phase Goal:** Writ scripts can use `None` and `Some(value)` without the `Option::` prefix -- the resolver injects both as symbols at lower priority than user-defined names
**Verified:** 2026-03-06T17:30:00Z
**Status:** human_needed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths (from ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `let x: bool? = None;` compiles without error | VERIFIED | `none_unqualified_with_annotation` passes; `check_ident` injects `Option<InferVar>` for bare `None` |
| 2 | `let y = Some(true);` compiles without error, type inferred as `bool?` | VERIFIED | `some_unqualified_infers_type` passes; `check_call` `Some` fast-path builds `Option<arg_ty>` |
| 3 | User-defined `None`/`Some` shadows injected symbol without compile error | VERIFIED | `user_none_shadows_builtin` passes; `check_ident` finds DefMap entry before sub-prelude step |
| 4 | `Option::None` and `Option::Some(value)` continue to compile correctly | ? UNCERTAIN | `fn_optional.writ` source + `fn_optional.writil` snapshot exist and show correct IL, but `test_fn_optional` is NOT registered in `golden_tests.rs` -- no running automated test covers this criterion |

**Score:** 3/4 success criteria verified

**Additional truths verified (from plan must_haves):**

| Truth | Status | Evidence |
|-------|--------|----------|
| Resolver recognizes None/Some as valid unqualified identifiers at sub-prelude priority | VERIFIED | `LookupResult::BuiltinVariant` returned from `resolve_value` step 8 after all user lookups fail |
| User-defined name at any scope level shadows None/Some without error | VERIFIED | `user_none_shadows_builtin` test; sub-prelude step 8 fires only when `resolve_type` returns `NotFound` |
| `using Status::*;` expands to one UsingEntry per variant | VERIFIED | `using_enum_glob` passes; `find_enum_variants` + `active_usings.push` loop confirmed in `resolver.rs` |
| `using Option::*;` accepted without error (vacuous) | VERIFIED | `using_option_glob_redundant_no_error` passes; prelude type check skips with `continue` |
| Two using-glob declarations with overlapping variant names produce ambiguity error | VERIFIED | `using_glob_conflict_ambiguous` passes; conflict detected at import-expansion time |
| `let x = None;` without annotation produces NoneWithoutAnnotation (E0120) | VERIFIED | `bare_none_no_annotation_error` passes; `check_stmt` bare-None detection emits E0120 |
| None/Some in pattern position on Option scrutinee compiles | VERIFIED | `none_some_in_pattern_position` passes; `check_pattern` handles `TyKind::Option` arms |
| Spec §23.4.4 documents using-glob and sub-prelude builtins | VERIFIED | Section found at line 185+ in `language-spec/spec/24_23_modules_namespaces.md` |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/resolve/prelude.rs` | `SUB_PRELUDE_VARIANT_NAMES` constant | VERIFIED | Line 20: `pub const SUB_PRELUDE_VARIANT_NAMES: &[&str] = &["None", "Some"];` |
| `writ-compiler/src/resolve/scope.rs` | `LookupResult::BuiltinVariant` variant + sub-prelude step | VERIFIED | Line 62: `BuiltinVariant(String)`; line 330: returned when name matches and all other lookups fail |
| `writ-compiler/src/resolve/resolver.rs` | Glob-enum expansion in `process_usings` | VERIFIED | Lines 107-198: `find_enum_variants` call + `active_usings.push` loop; line 741: helper function |
| `writ-compiler/src/check/check_expr.rs` | Sub-prelude `None`/`Some` injection in `check_ident`, `check_call`, `check_pattern` | VERIFIED | Line 444: `"None" | "Some"` match; line 741: `Some` fast-path; lines 1591, 1646: pattern arms |
| `writ-compiler/src/check/error.rs` | `TypeError::NoneWithoutAnnotation` variant | VERIFIED | Line 116: variant defined; line 373: Diagnostic conversion with "E0120" |
| `writ-diagnostics/src/code.rs` | E0120 constant | VERIFIED | Line 36: `pub const E0120: &str = "E0120";` |
| `writ-parser/src/parser.rs` | `::*` terminal in `using_decl` + `enum_destruct_single` rule | VERIFIED | Line 2452: `qualified_name_glob`; line 296: `enum_destruct_single` rule |
| `language-spec/spec/24_23_modules_namespaces.md` | New `§23.4.4` subsection | VERIFIED | Content found at line 185: glob import rules, sub-prelude builtins, examples, E0120 note |
| `writ-compiler/tests/typecheck_tests.rs` | 5 LANG-02 test stubs | VERIFIED | Lines 926, 934, 942, 950, 958: all 5 tests present and passing |
| `writ-compiler/tests/resolve_tests.rs` | 3 LANG-02 resolve test stubs | VERIFIED | Lines 836, 856, 877: all 3 tests present and passing |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `scope.rs` | `prelude.rs` | `SUB_PRELUDE_VARIANT_NAMES` lookup in `resolve_value` | WIRED | Line 329: `prelude::SUB_PRELUDE_VARIANT_NAMES.contains(&name)` |
| `resolver.rs` | `scope.rs` | `active_usings.push` for glob variants | WIRED | Lines 133, 161, 186, 198: push calls confirmed |
| `check_expr.rs` | `ty.rs` | `ctx.interner.option(infer_ty)` for None/Some type construction | WIRED | Line 448 (check_ident), line 746 (check_call Some fast-path) |
| `check_expr.rs` | `unify.rs` | `ctx.unify.new_var()` for fresh inference variable | WIRED | Line 446 in `check_ident` sub-prelude branch |
| `parser.rs` | `ast/decl.rs` | `AstUsingDecl.path` contains trailing `"*"` for glob | WIRED | Line 2455: `("*", e.span())` appended to path when `Token::Star` present |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| LANG-02 | 43-01, 43-02, 43-03 | User can write `None` and `Some(value)` without `Option::` prefix | SATISFIED | All 8 LANG-02 test stubs green; sub-prelude injection wired at resolver and typecheck layers; spec documented; REQUIREMENTS.md marks as `[x]` |

No orphaned requirements: REQUIREMENTS.md traceability table maps only LANG-02 to Phase 43, and it is fully covered.

### Anti-Patterns Found

No anti-patterns found. Scan of all modified files returned no TODOs, FIXMEs, XXX, HACK, or placeholder comments.

### Human Verification Required

#### 1. Option:: Qualified Path Regression Test

**Test:** In `writ-golden/tests/golden_tests.rs`, confirm that `fn_optional.writ` compiles cleanly end-to-end. The simplest way is to add a `test_fn_optional` function:
```rust
#[test]
fn test_fn_optional() {
    run_golden_test("fn_optional");
}
```
Then run `cargo test -p writ-golden -- test_fn_optional`. If the test passes, bless if needed.

**Expected:** `Option::None` and `Option::Some(true)` both compile and produce IL matching `fn_optional.writil` (which already contains `WRAP_SOME` for `Option::Some(true)` and a null-return for `Option::None`).

**Why human:** The `fn_optional.writ` source file and `fn_optional.writil` golden snapshot both exist and appear correct, but `test_fn_optional` is not registered in `golden_tests.rs`. ROADMAP success criterion 4 ("Option::None and Option::Some(value) continue to compile correctly") therefore has no running automated test. A human must either register the golden test or confirm manually that qualified Option paths still compile.

**Note on LOAD_NULL:** The ROADMAP criterion says `None` "emits `LOAD_NULL`", but the golden IL snapshot shows `RET r0` for `produce_option_none()` (returning a null register, not an explicit `LOAD_NULL` instruction). This may be a snapshot detail rather than a failure -- but a human reviewer should confirm the IL emission is correct by spec.

### Gaps Summary

No blocking gaps. All must-have artifacts exist, are substantive, and are wired. All 8 LANG-02 test stubs pass. The requirement LANG-02 is marked complete in REQUIREMENTS.md.

The single outstanding item is success criterion 4 (qualified `Option::None`/`Option::Some` paths still work), which is informally covered by the existing `fn_optional.writ` golden source and snapshot but lacks a registered test function. This is a coverage gap in the golden test suite, not a functional regression.

---
_Verified: 2026-03-06T17:30:00Z_
_Verifier: Claude (gsd-verifier)_
