---
phase: 25-il-codegen-method-bodies
plan: 05
subsystem: compiler
tags: [codegen, IL, emit, patterns, switch, const-fold, closures, strings, writ-compiler]

# Dependency graph
requires:
  - phase: 25-il-codegen-method-bodies
    provides: emit_all_bodies, BodyEmitter, EmittedBody, emit_bodies — Phase 25 Plans 01-04 baseline

provides:
  - "SWITCH instruction offsets correctly resolved via post-arm-emission patching (not no-op)"
  - "TypedDecl::Const wired to const_fold() producing LOAD_INT/LOAD_FLOAT for folded literals"
  - "Lambda bodies emitted as separate EmittedBody entries via lambda_infos parameter"
  - "String literals use pending_strings deferred interning; emit_bodies patches LoadString string_idx"
  - "EmittedBody.method_def_id changed to Option<DefId> (None for anonymous lambda bodies)"
  - "LabelAllocator.resolve() method for instruction-index lookup by label"

affects:
  - phase-26-cli-integration
  - phase-27-runtime-vm

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Deferred string interning: BodyEmitter collects pending_strings; caller patches after mutable builder available"
    - "Post-emission SWITCH patching: arm labels marked first, offsets patched via emitter.labels.resolve() after all arms"
    - "Option<DefId> for EmittedBody.method_def_id: None for synthetic lambda bodies, Some for named methods"

key-files:
  created: []
  modified:
    - writ-compiler/src/emit/body/labels.rs
    - writ-compiler/src/emit/body/patterns.rs
    - writ-compiler/src/emit/body/mod.rs
    - writ-compiler/src/emit/body/expr.rs
    - writ-compiler/src/emit/mod.rs
    - writ-compiler/src/emit/serialize.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-compiler/tests/emit_serialize_tests.rs

key-decisions:
  - "SWITCH fixup uses post-emission direct patching (not add_fixup) because Switch has variable-length Vec<i32> offsets"
  - "EmittedBody.method_def_id changed to Option<DefId> to support anonymous lambda bodies (None) cleanly"
  - "String interning uses pending_strings deferred pattern: collect during emission, intern after emit_all_bodies via mutable builder"
  - "emit_all_bodies signature extended with lambda_infos: &[LambdaInfo] parameter to enable closure body emission"
  - "Lambda bodies use method_def_id: None; serializer already skips None-DefId MethodDefs (def_id: None in builder)"

patterns-established:
  - "Deferred mutation pattern: when BodyEmitter holds immutable builder, collect side effects in EmittedBody fields for post-emission fixup"
  - "SWITCH label fixup must occur AFTER all arms emitted; labels.resolve() used not add_fixup()"

requirements-completed:
  - EMIT-07
  - EMIT-08
  - EMIT-09
  - EMIT-10
  - EMIT-11
  - EMIT-12
  - EMIT-13
  - EMIT-14
  - EMIT-15
  - EMIT-16
  - EMIT-17
  - EMIT-18
  - EMIT-19
  - EMIT-21
  - EMIT-23
  - EMIT-26
  - EMIT-27
  - EMIT-28

# Metrics
duration: 22min
completed: 2026-03-03
---

# Phase 25 Plan 05: Gap Closure — SWITCH Fixup, const_fold, Closure Bodies, String Interning Summary

**Four verification gaps closed: SWITCH offsets now non-zero via post-arm patching, const_fold wired into emit_all_bodies, lambda bodies emitted as separate EmittedBody entries, string literals use deferred pending_strings interning with emit_bodies fixup pass**

## Performance

- **Duration:** ~22 min
- **Started:** 2026-03-03T13:00:00Z
- **Completed:** 2026-03-03T13:22:35Z
- **Tasks:** 2 (TDD: 2 RED commits + 2 GREEN commits combined)
- **Files modified:** 8

## Accomplishments
- Fixed SWITCH no-op fixup loop: arm labels marked after emission, then directly patch `Switch.offsets` Vec via `labels.resolve()` for each arm
- Added `LabelAllocator::resolve()` method for instruction-index lookup (supports SWITCH multi-slot patching without add_fixup)
- Wired `const_fold()` into `emit_all_bodies()` via new `TypedDecl::Const` arm; foldable constants emit `LoadInt/LoadFloat/LoadTrue/LoadFalse + Ret`; non-foldable falls back to `emit_expr()`
- Added `TypedDecl::Global` arm to `emit_all_bodies()` for completeness
- Lambda bodies emitted as separate `EmittedBody` entries: extended `emit_all_bodies` signature with `lambda_infos: &[LambdaInfo]`, walk AST in pre-scan order to collect lambda bodies, emit each as `EmittedBody { method_def_id: None, .. }`
- String literal interning: `BodyEmitter.pending_strings` collects `(instr_idx, string_value)` pairs during emission; `emit_bodies()` does fixup pass after `emit_all_bodies` with mutable builder
- `EmittedBody.method_def_id` changed from `DefId` to `Option<DefId>` to support anonymous lambda bodies
- 8 new tests added (4 per task); all 73 emit_body_tests + 10 emit_serialize_tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix SWITCH offset fixup and wire const_fold for TypedDecl::Const** - `4d52c15` (feat)
2. **Task 2: Wire closure body emission and implement string literal interning** - `5082f34` (test)

## Files Created/Modified
- `writ-compiler/src/emit/body/labels.rs` - Added `resolve()` method for instruction-index lookup
- `writ-compiler/src/emit/body/patterns.rs` - SWITCH fixup: post-arm patching using `labels.resolve()` + direct Vec mutation
- `writ-compiler/src/emit/body/mod.rs` - emit_all_bodies signature + lambda_infos param; TypedDecl::Const/Global arms; EmittedBody.method_def_id: Option<DefId>; pending_strings field; lambda body walker
- `writ-compiler/src/emit/body/expr.rs` - String literals use pending_strings deferred collection instead of LoadString { string_idx: 0 }
- `writ-compiler/src/emit/mod.rs` - Pass lambda_infos to emit_all_bodies; string interning fixup pass
- `writ-compiler/src/emit/serialize.rs` - Update body match to use Option<DefId>
- `writ-compiler/tests/emit_body_tests.rs` - 8 new tests for all 4 gap fixes
- `writ-compiler/tests/emit_serialize_tests.rs` - Updated for EmittedBody struct changes

## Decisions Made
- **SWITCH patching approach:** Direct `Vec<i32>` mutation post-emission rather than add_fixup (which handles single-offset branches only). Arm labels are marked after SWITCH, so the fixup loop moved to after all arms are emitted. Resolution via `labels.resolve(label)`.
- **EmittedBody.method_def_id: Option<DefId>:** Lambda bodies have no source DefId (builder MethodDefs created with `def_id: None`). Using `Option<DefId>` cleanly models this. Serializer already skips None-DefId MethodDefs.
- **Deferred string interning pattern:** BodyEmitter holds `&'a ModuleBuilder` (immutable). Rather than change to `&'a mut ModuleBuilder` (which would require every test to use `let mut builder`), we collect `pending_strings: Vec<(usize, String)>` in BodyEmitter/EmittedBody, then patch instructions in `emit_bodies()` which holds the builder exclusively and can mutate.
- **lambda_infos parameter added to emit_all_bodies:** Direct parameter rather than threading through context struct, consistent with existing function signature style.

## Deviations from Plan

None - plan executed exactly as written. All four gaps closed as specified.

The approach for string interning used Option A from the plan (pending_strings deferred collection) rather than changing BodyEmitter to hold `&'a mut ModuleBuilder`, which correctly avoids a large test refactor.

## Issues Encountered
- `DefId::from_raw()` does not exist (id_arena Id<T> is opaque). Resolved by changing `EmittedBody.method_def_id` to `Option<DefId>` so lambda bodies use `None` cleanly.
- `TypedPattern::EnumVariant` fields in tests were wrong (`ty`, `type_name` don't exist; correct fields are `enum_def_id`, `variant_name`, `bindings`, `span`). Fixed by consulting existing lambda test patterns.
- Duplicate `use` imports for `Capture`/`CaptureMode` in test file (already imported from Plan 03 section). Removed duplicate.

## Next Phase Readiness
- All 4 verification gaps from Phase 25 VERIFICATION.md are closed
- emit_bodies() produces spec-compliant .writil binary with correct SWITCH dispatch, const-folded values, closure bodies, and properly indexed string literals
- TAIL_CALL (EMIT-24) and STR_BUILD (EMIT-20) remain deferred (not part of this plan)
- Phase 26 (CLI integration) can now consume emit_bodies()

---
*Phase: 25-il-codegen-method-bodies*
*Completed: 2026-03-03*
