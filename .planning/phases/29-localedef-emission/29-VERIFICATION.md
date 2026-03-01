---
phase: 29-localedef-emission
verified: 2026-03-03T18:30:00Z
status: passed
score: 5/5 must-haves verified
re_verification: false
---

# Phase 29: LocaleDef Emission Verification Report

**Phase Goal:** The compiler emits LocaleDef table rows for all localization keys generated during lowering; the locale manifest is populated instead of returning 0 rows
**Verified:** 2026-03-03T18:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A Writ source containing a Locale dlg override compiles without errors | VERIFIED | `locale_override_dlg_emits_locale_def` test passes with zero errors in diagnostics; confirmed by `cargo test -p writ-compiler locale` output |
| 2 | The compiled module contains at least one LocaleDef row | VERIFIED | `builder.locale_defs.len() == 1` assertion passes in unit test; `module.locale_defs.len() > 0` assertion passes in E2E test `test_locale_override_produces_locale_def_rows` |
| 3 | The LocaleDef dlg_method points to the base un-attributed dlg MethodDef | VERIFIED | `collect_locale_defs` uses `builder.methoddef_token_by_name(base_name)` to look up the base method; base_name is extracted from `entry.name.split('$').next()` which strips the locale suffix from the override's suffixed name (e.g. `greet$ja` -> `greet`); the base method is registered before the override in document order |
| 4 | The LocaleDef loc_method points to the locale-override dlg MethodDef | VERIFIED | `collect_locale_defs` uses `builder.token_for_def(def_id)` for the override's MethodDef token; `add_locale_def(base, &tag, loc)` places override token in `loc_method` field |
| 5 | The LocaleDef locale string heap offset resolves to the locale tag | VERIFIED | `builder.add_locale_def(base, &tag, loc)` interns the locale tag string into the string heap inside `add_locale_def()` (confirmed via `module_builder.rs` line 443-455); E2E round-trip through `Module::from_bytes` confirms the serialized locale string is preserved |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/emit/collect.rs` | `collect_locale_defs()` implementation | VERIFIED | Function present at line 768; contains full `[Locale("tag")]` attribute scanning, `$`-split base-name recovery, `methoddef_token_by_name` + `token_for_def` lookup, and `add_locale_def()` call |
| `writ-compiler/src/lower/dialogue.rs` | Attribute passthrough and locale-aware name suffixing | VERIFIED | `lower_attrs(dlg.attrs, ctx)` called at line 56; `[Locale]` detection + `$tag` suffix at lines 61-77; `lower_vis(dlg.vis)` wired at line 157; all via `use super::{lower_attrs, lower_param, lower_vis}` import at line 15 |
| `writ-compiler/tests/emit_tests.rs` | Unit test for LocaleDef emission | VERIFIED | Three tests: `locale_override_dlg_emits_locale_def` (1 row), `two_locale_overrides_emit_two_locale_defs` (2 rows), `no_locale_attr_emits_zero_locale_defs` (0 rows); all pass |
| `writ-cli/tests/e2e_compile_tests.rs` | E2E test for LocaleDef in compiled module | VERIFIED | `test_locale_override_produces_locale_def_rows` compiles source to bytes, deserializes via `Module::from_bytes`, asserts `module.locale_defs.len() > 0`; passes |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-compiler/src/lower/dialogue.rs` | `writ-compiler/src/emit/collect.rs` | `[Locale]` attributes flow from lowering through AST to collection pass | WIRED | `lower_dialogue()` writes `[Locale]` attributes into `AstFnDecl.attrs` and suffixes the name with `$tag`; `collect_locale_defs()` reads `AstFnDecl.attrs` via `find_attrs_for_entry()` and detects the `[Locale]` attribute; the `$` in the name signals it is an override, enabling base-name recovery |
| `writ-compiler/src/emit/collect.rs` | `writ-compiler/src/emit/module_builder.rs` | `collect_locale_defs` calls `builder.add_locale_def()` | WIRED | `collect_locale_defs` at line 812 calls `builder.add_locale_def(base, &tag, loc)`; `add_locale_def` exists at `module_builder.rs` line 443; `locale_defs: Vec<LocaleDefRow>` public field confirmed at line 97 |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| EMIT-25 | 29-01-PLAN.md | Compiler emits LocaleDef table rows for localization keys | SATISFIED | `collect_locale_defs()` fully implemented and called from `collect_post_finalize()`; 3 unit tests + 1 E2E test all pass; REQUIREMENTS.md line 73 shows `[x] EMIT-25` checked; line 180 maps EMIT-25 to Phase 29 with status Complete |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `writ-compiler/src/emit/collect.rs` | 96 | Comment update: old TODO stub replaced with `// 5. LocaleDef: collected in collect_post_finalize() after token assignment.` | Info | None — the misleading TODO is gone; accurate comment now documents the design decision |
| `writ-compiler/src/emit/collect.rs` | 145-200 | Pre-existing `placeholder` comments in `encode_ty`/`resolve_type` helper functions | Info | These are pre-existing from earlier phases and are unrelated to LocaleDef emission; both functions are marked `dead_code` by the compiler (warnings present but not new) |

No blockers or warnings introduced by this phase.

### Human Verification Required

None. All observable behaviors were verifiable programmatically:

- Diagnostic absence: checked via assertion in tests
- LocaleDef row count: checked via `builder.locale_defs.len()` assertions
- Serialization round-trip: checked via E2E test with `Module::from_bytes`
- No regression: full test suite (347+ tests) passes with zero failures

### Gaps Summary

No gaps. All 5 must-have truths are verified, all 4 required artifacts are substantive and wired, both key links are confirmed active, and EMIT-25 is fully satisfied. The full test suite passes without regressions.

---

## Verification Evidence Summary

**Commits verified:**
- `be57046` — `feat(29-01): wire dlg attributes through lowering and implement collect_locale_defs`
- `9957e6b` — `test(29-01): add LocaleDef emission unit tests and E2E test`

**Test results:**
- `cargo test -p writ-compiler locale` — 3/3 passed
  - `locale_override_dlg_emits_locale_def` ... ok
  - `two_locale_overrides_emit_two_locale_defs` ... ok
  - `no_locale_attr_emits_zero_locale_defs` ... ok
- `cargo test -p writ-cli locale` — 1/1 passed
  - `test_locale_override_produces_locale_def_rows` ... ok
- `cargo test` (full suite) — all suites pass, 0 failures across all crates

**Key implementation facts confirmed by direct code read:**
- `lower_dialogue()` line 56: `let attrs = lower_attrs(dlg.attrs, ctx);` — attributes wired
- `lower_dialogue()` lines 61-77: `[Locale]` detection + `$tag` suffix — name disambiguation
- `lower_dialogue()` line 157: `vis: lower_vis(dlg.vis)` — visibility passthrough
- `collect_locale_defs()` line 768: function exists, called from `collect_post_finalize()` line 119
- `collect_locale_defs()` lines 781-796: `[Locale("tag")]` attribute scan with correct `AstAttributeArg::Positional(AstExpr::StringLit)` destructuring
- `collect_locale_defs()` line 801: base name recovery via `entry.name.split('$').next()`
- `collect_locale_defs()` lines 804-812: `methoddef_token_by_name` + `token_for_def` + `add_locale_def` call

---

_Verified: 2026-03-03T18:30:00Z_
_Verifier: Claude (gsd-verifier)_
