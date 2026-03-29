---
phase: 105-writ-compiler-reflectable-auto-impl-emission
plan: 01
subsystem: compiler
tags: [reflectable, impl-def, method-def, type-system, codegen, golden-tests]

# Dependency graph
requires:
  - phase: 104-writ-compiler-typeof-lowering-and-tykind
    provides: TYPEOF instruction in the IL and resolve_typeof_type_idx in body/expr
provides:
  - emit_reflectable_auto_impl() helper in collect/contracts.rs
  - ReflectableInfo struct for passing auto-impl data from collect to body phases
  - Synthetic get_type() MethodDef per user TypeDef interleaved in declaration order
  - Synthetic ImplDef linking each TypeDef to Reflectable contract (token 167772179)
  - TYPEOF + RET body for each auto-impl in emit_all_bodies
affects:
  - any phase that reads ImplDef.method_list or method_def_count
  - writ-runtime dispatch tests that call get_type() on user-defined types
  - golden test suite (all .writil files with user types now have .impl Reflectable blocks)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Synthetic metadata emission: add method/impl rows during collect_defs, return handle info, fix up post-finalize"
    - "Post-finalize method_list fixup: update ImplDef.method_list after finalize() using typedef_method_list_by_handle()"
    - "Orphaned body ordering: reflectable bodies emitted before lambda bodies so positional matching in serialize.rs is correct"
    - "param_count=0 for self-only synthetic methods: ParamDef rows are emitted only for regular params"

key-files:
  created: []
  modified:
    - writ-compiler/src/emit/collect/contracts.rs
    - writ-compiler/src/emit/collect/mod.rs
    - writ-compiler/src/emit/body/mod.rs
    - writ-compiler/src/emit/mod.rs
    - writ-compiler/src/emit/module_builder.rs
    - writ-compiler/tests/emit_tests.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-golden/tests/golden/*.writil (8 golden files updated)

key-decisions:
  - "param_count=0 on synthetic get_type() MethodDef — ParamDef rows only count regular params, not self; using 1 caused disassembler panic (range out of bounds in method_param_start accumulation)"
  - "Post-finalize ImplDef.method_list fixup via set_impl_def_method_list() after builder.finalize() — cannot compute correct 1-based row at add_impl_def() time since finalize sort hasn't run yet"
  - "Reflectable bodies emitted before lambda bodies in emit_all_bodies — reflectable MethodDefs are added during collect_defs (before pre_scan_lambdas), so they appear first in finalized MethodDef table"
  - "TypeRef return type encoding: [0x00, 0x00, 0x10, token_bytes_le] where token = type_ref_token_by_name('Type') — matches disassembler decode_type_ref tag 0x10 with TypeRef table lookup"

patterns-established:
  - "Synthetic MethodDef with parent TypeDefHandle: gets grouped by finalize stable-sort into TypeDef's method range"
  - "Returning Vec<ReflectableInfo> from collect_defs to thread post-finalize fixup data and body emission data"

requirements-completed: [COMP-03, REFL-02]

# Metrics
duration: 35min
completed: 2026-03-28
---

# Phase 105 Plan 01: Reflectable Auto-Impl Emission Summary

**Compiler auto-generates Reflectable ImplDef + TYPEOF+RET get_type() body for every user-defined TypeDef, satisfying COMP-03 and REFL-02 with interleaved metadata emission and correct post-finalize method_list wiring**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-03-28T13:00:00Z
- **Completed:** 2026-03-28T13:36:01Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments

- Every user-defined TypeDef (struct, class, entity, enum) now produces a Reflectable ImplDef in the compiled module with a synthetic get_type() method body
- TYPEOF instruction emitted with the TypeDef's own finalized MetadataToken as type_idx, followed by RET — correct per spec
- ImplDef.method_list correctly wired post-finalize so domain_dispatch.rs find the right method for CALL_VIRT dispatch
- 8 golden .writil files blessed with new .impl TypeName : Reflectable blocks; 48/48 golden tests pass
- 95/95 writ-compiler tests pass; full cargo test suite green

## Task Commits

1. **Task 1: Collect-pass auto-impl emission and body plumbing** - `142951a` (feat)
2. **Task 2: Update tests and bless golden snapshots** - `5243087` (test)

## Files Created/Modified

- `writ-compiler/src/emit/collect/contracts.rs` - Added emit_reflectable_auto_impl() and REFLECTABLE_CONTRACT_TOKEN constant
- `writ-compiler/src/emit/collect/mod.rs` - Added ReflectableInfo struct; collect_defs() returns Vec<ReflectableInfo>; auto-impl calls after Struct/Class/Entity/Enum
- `writ-compiler/src/emit/body/mod.rs` - Added reflectable_infos parameter; synthetic TYPEOF+RET body emission before lambda bodies
- `writ-compiler/src/emit/mod.rs` - Capture reflectable_infos from collect_defs; post-finalize method_list fixup; pass to emit_all_bodies
- `writ-compiler/src/emit/module_builder.rs` - Added set_impl_def_method_list() and typedef_method_list_by_handle() methods
- `writ-compiler/tests/emit_tests.rs` - Updated impl_emits_impldef count; added reflectable_auto_impl_three_types and method_list_invariant_holds tests
- `writ-compiler/tests/emit_body_tests.rs` - Updated all emit_all_bodies calls with empty reflectable_infos param
- `writ-golden/tests/golden/*.writil` - 8 golden files blessed with .impl Reflectable blocks

## Decisions Made

- **param_count=0 for synthetic methods**: The MethodDef.param_count field counts ParamDef table rows. Self has no ParamDef row (it is implicit). Setting param_count=1 caused the disassembler to accumulate a wrong offset into the param_defs table, causing slice-out-of-bounds panics for all golden tests with user types.

- **Post-finalize ImplDef.method_list fixup**: At add_impl_def() call time, the MethodDef table hasn't been sorted yet. The correct 1-based row index for get_type() is only known after finalize() runs the stable sort. Added set_impl_def_method_list() to ModuleBuilder and fix up in both emit() and emit_bodies().

- **Reflectable body ordering before lambdas**: serialize.rs matches orphaned bodies (method_def_id=None) to orphaned MethodDefs (def_id=None) positionally. Reflectable MethodDefs are added during collect_defs (before pre_scan_lambdas), so they come first in the finalized table. Emitting reflectable bodies before lambda bodies preserves the positional match.

- **TypeRef return type encoding**: For () -> Type, the sig blob is [0x00, 0x00, 0x10, type_ref_token_le]. Tag 0x10 is the named/reference type tag; the 4-byte value is the TypeRef MetadataToken for "Type" (registered as second add_type_ref in collect_defs).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed param_count=0 for synthetic get_type() MethodDef**
- **Found during:** Task 2 (running golden tests)
- **Issue:** Plan specified param_count=1 (for self), but MethodDef.param_count in the binary counts ParamDef rows (not self). With param_count=1 and 0 ParamDef rows, the disassembler's cumulative param_def offset exceeded param_defs.len(), causing slice-out-of-bounds panic on 8 golden tests.
- **Fix:** Set param_count=0 in emit_reflectable_auto_impl() to match the 0 ParamDef rows actually emitted.
- **Files modified:** writ-compiler/src/emit/collect/contracts.rs
- **Committed in:** 142951a (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Essential correctness fix. No scope creep. The plan's spec for param_count was incorrect; the fix aligns with how all other self-only methods are handled.

## Issues Encountered

None beyond the param_count bug documented above.

## Known Stubs

None — all auto-impl methods emit real IL (TYPEOF + RET with correct type_idx).

## Next Phase Readiness

- Reflectable auto-impl emitted for all user types: ready for runtime get_type() dispatch testing
- ImplDef.method_list correctly wired: CALL_VIRT on get_type() can be dispatched by domain_dispatch.rs
- Golden tests locked with .impl Reflectable blocks as regression anchors

## Self-Check: PASSED

Files checked:
- writ-compiler/src/emit/collect/contracts.rs: emit_reflectable_auto_impl() present
- writ-compiler/src/emit/collect/mod.rs: returns Vec<ReflectableInfo>, auto-impl calls present
- writ-compiler/src/emit/body/mod.rs: reflectable_infos parameter, synthetic body emission present
- Commits 142951a and 5243087 verified in git log

---
*Phase: 105-writ-compiler-reflectable-auto-impl-emission*
*Completed: 2026-03-28*
