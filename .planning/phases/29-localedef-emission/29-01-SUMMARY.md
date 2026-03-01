---
phase: 29-localedef-emission
plan: 01
subsystem: compiler
tags: [localization, dialogue, emission, metadata, LocaleDef, lower, collect]

# Dependency graph
requires:
  - phase: 28-codegen-bug-fixes
    provides: "callee_def_id propagation, correct CALL/CALL_VIRT/TailCall emission"
provides:
  - "LocaleDef table (table 18) emission for [Locale(\"tag\")] dlg overrides"
  - "Attribute passthrough from CST DlgDecl through lower_dialogue() to AstFnDecl"
  - "Locale-override name suffixing: greet$ja prevents resolver duplicate-name collision"
affects:
  - runtime locale dispatch (reads LocaleDef table to select locale-specific dlg method)
  - future locale tooling phases

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "$tag suffix on locale-override dlg names; $ is not valid in user-written identifiers so greet$ja cannot collide with user-defined names"
    - "collect_post_finalize() extended with collect_locale_defs() pass — all token-dependent passes belong here after finalize()"

key-files:
  created:
    - ".planning/phases/29-localedef-emission/29-01-SUMMARY.md"
  modified:
    - "writ-compiler/src/lower/dialogue.rs"
    - "writ-compiler/src/emit/collect.rs"
    - "writ-compiler/tests/emit_tests.rs"
    - "writ-cli/tests/e2e_compile_tests.rs"

key-decisions:
  - "[Phase 29-01]: lower_dialogue() wires CST attrs via lower_attrs() and CST vis via lower_vis(); was previously hardcoded to attrs: vec![] and vis: None"
  - "[Phase 29-01]: Locale-override dlg names suffixed with $tag (e.g. greet$ja) in lower_dialogue() — $ char invalid in user identifiers ensures no collision; base name recovered via split('$').next()"
  - "[Phase 29-01]: collect_locale_defs() placed in collect_post_finalize() (not collect_defs()) because it needs finalized MethodDef tokens from token_for_def() and methoddef_token_by_name()"
  - "[Phase 29-01]: Tests use empty dlg bodies (dlg greet() {}) to avoid Entity/say resolution errors that would require extern entity declarations"

patterns-established:
  - "Locale override detection: name contains '$' => is override; split('$').next() recovers base name"

requirements-completed: [EMIT-25]

# Metrics
duration: 8min
completed: 2026-03-03
---

# Phase 29 Plan 01: LocaleDef Emission Summary

**LocaleDef table emission (EMIT-25): [Locale("ja")] dlg overrides now produce table-18 rows linking base dlg MethodDef to locale-specific MethodDef via $tag name suffixing**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-03T17:49:50Z
- **Completed:** 2026-03-03T17:57:50Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- `lower_dialogue()` now passes CST attributes and visibility through to `AstFnDecl`; [Locale("ja")] dlg overrides are suffixed to `greet$ja` preventing resolver duplicate-name collisions
- `collect_locale_defs()` implemented in `collect.rs` and called from `collect_post_finalize()`; scans all `TypedDecl::Fn` entries, finds `[Locale("tag")]` attrs, extracts base name via `$` split, and calls `builder.add_locale_def()`
- 3 unit tests + 1 E2E test covering: 1 override, 2 overrides, zero overrides (regression guard), and deserialized module check
- Full test suite passes with no regressions (347 tests across all crates)

## Task Commits

1. **Task 1: Wire dlg attributes through lowering and implement collect_locale_defs** - `be57046` (feat)
2. **Task 2: Add LocaleDef emission tests (unit + E2E)** - `9957e6b` (test)

**Plan metadata:** (docs commit below)

## Files Created/Modified
- `writ-compiler/src/lower/dialogue.rs` - lower_dialogue() now calls lower_attrs() and lower_vis(); detects [Locale] to produce $tag suffix
- `writ-compiler/src/emit/collect.rs` - collect_locale_defs() function added; called from collect_post_finalize(); AstExpr import added
- `writ-compiler/tests/emit_tests.rs` - 3 new tests: locale_override_dlg_emits_locale_def, two_locale_overrides_emit_two_locale_defs, no_locale_attr_emits_zero_locale_defs
- `writ-cli/tests/e2e_compile_tests.rs` - 1 new E2E test: test_locale_override_produces_locale_def_rows

## Decisions Made
- Used empty `dlg greet() {}` bodies in tests to avoid Entity/say resolution errors; the locale emission mechanism is independent of dlg body content
- `collect_locale_defs` placed in `collect_post_finalize()` (not `collect_defs()`) because it relies on `builder.token_for_def()` and `builder.methoddef_token_by_name()` which require finalized tokens
- `$` separator chosen for locale-override name suffix because `$` is not a valid character in user-written Writ identifiers (lexer rejects it), so `greet$ja` can never collide with a user-defined name

## Deviations from Plan

None - plan executed exactly as written.

The plan's pseudocode for `collect_locale_defs` referenced a redundant double-match pattern (`if let ... { match arg { ... } }`) — the implementation uses clean single-level destructuring instead, which is equivalent and more idiomatic Rust.

## Issues Encountered
- Parser constructs `DlgDecl { attrs: Vec::new(), vis: None }` inline (line 2115) but then patches `dd.attrs = attr_list` in the `attrs_vis_decl` combinator (line 3083). Confirmed attrs ARE correctly propagated before wiring lowering.
- First test iteration used `@narrator Hello.` in dlg body; this triggers Entity hoisting (`Entity.getOrCreate<Narrator>`) and `say()` calls which fail with `E0102 undefined variable Entity`. Switched to empty dlg bodies.

## Next Phase Readiness
- EMIT-25 is satisfied: LocaleDef table emission is complete
- Phase 29 Plan 01 is the only plan in Phase 29; phase is complete
- v3.0 milestone requirement EMIT-25 (last open requirement) is now closed

---
*Phase: 29-localedef-emission*
*Completed: 2026-03-03*
