---
phase: 105-writ-compiler-reflectable-auto-impl-emission
verified: 2026-03-28T15:00:00Z
status: gaps_found
score: 3/4 must-haves verified
gaps:
  - truth: "Calling get_type() on an instance of each user type returns a Type object with the correct name field (ROADMAP SC-2)"
    status: partial
    reason: "Golden tests compile and disassemble but do not execute the module in the VM. No end-to-end test links 'compile Writ source -> run in VM -> assert Type.name' for user-defined types. The writ-runtime reflection_tests.rs covers TYPEOF dispatch and CALL_VIRT but constructs modules via ModuleBuilder directly rather than through the compiler pipeline."
    artifacts:
      - path: "writ-runtime/tests/reflection_tests.rs"
        issue: "Tests use raw ModuleBuilder, not the writ-compiler pipeline — no full-pipeline get_type() dispatch test for user-defined types"
    missing:
      - "An end-to-end test (either in writ-runtime or a new writ-integration crate) that: compiles a .writ source with a user struct/entity, runs it in the runtime, calls get_type() via CALL_VIRT on a user-defined-type instance, and asserts the returned Type object has the correct name field"
human_verification:
  - test: "Compile a Writ source with a struct, run it in the writ-runtime VM, call get_type() and inspect the returned Type.name"
    expected: "Type.name field on the heap object equals the struct's name string"
    why_human: "writ-golden tests only disassemble; no existing test drives the full compiler->VM pipeline for a user-defined type's get_type() result"
---

# Phase 105: writ-compiler Reflectable Auto-Impl Emission Verification Report

**Phase Goal:** Every compiled user TypeDef (struct, class, enum, entity) has a Reflectable ImplDef in the output module, and calling get_type() on any value dispatches correctly via the virtual dispatch table
**Verified:** 2026-03-28T15:00:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Every user-defined TypeDef (struct, class, entity, enum) gets a Reflectable ImplDef in the compiled module | VERIFIED | `collect_defs` calls `emit_reflectable_auto_impl()` after each Struct/Class/Entity/Enum arm (collect/mod.rs lines 140-180). ExternComponent and Component are excluded. Golden files confirm: 8 .writil files all have `.impl TypeName : contract_167772179` blocks for every user type. |
| 2 | The get_type() method body emits TYPEOF with the type's own type_idx followed by RET | VERIFIED | `emit_all_bodies` in body/mod.rs lines 598-625 emits `Instruction::TypeOf { r_dst: 1, type_idx }` + `Instruction::Ret { r_src: 1 }` for each reflectable_info. Golden files confirm: every auto-impl block shows `TYPEOF r1, <token>` then `RET r1`. |
| 3 | ImplDef rows are interleaved in TypeDef declaration order, not appended in a post-pass | VERIFIED | `emit_reflectable_auto_impl()` is called inline after each collect_struct/class/entity/enum call in the main `collect_defs` loop. Post-finalize fixup (`set_impl_def_method_list`) corrects method_list values after `finalize()` sorts rows. `method_list_invariant_holds` test confirms TypeDef.method_list is non-zero after finalize for both types in a two-struct module. |
| 4 | Extern types do NOT get Reflectable auto-impls | VERIFIED | The `TypedDecl::ExternComponent` and `TypedDecl::Component` arms in collect/mod.rs (lines 206-213) do NOT call `emit_reflectable_auto_impl`. The `fn_log_say_choice.writil` golden file confirms: `__closure_0` and `__closure_1` (synthetic class TypeDefs from lambda pre-scan) also have no Reflectable impls, only the user entity `Narrator` gets one. |

**Score:** 3/4 truths verified (truth #2 is verified at the compiler/disassembler level; end-to-end VM dispatch for user-defined types is unverified per ROADMAP SC-2)

### ROADMAP Success Criteria vs Verification

| # | ROADMAP Success Criterion | Status | Evidence |
|---|--------------------------|--------|----------|
| 1 | A compiled module with three user types contains exactly three Reflectable ImplDefs | VERIFIED | `reflectable_auto_impl_three_types` test: `struct Point + enum Color + entity Guard` -> `assert_eq!(builder.impl_def_count(), 3)` PASS. |
| 2 | Calling get_type() on an instance of each user type returns a Type object with the correct name field — verified by a golden test that runs the compiled module | PARTIAL | Golden tests are compile+disassemble only, not VM execution. `reflection_tests.rs` covers TYPEOF and CALL_VIRT dispatch but uses raw ModuleBuilder, not the compiler pipeline. No test drives `compile .writ -> run VM -> assert Type.name` for a user-defined type. |
| 3 | The dispatch table method_list offset invariant holds for all auto-generated ImplDef rows — confirmed by a test with multiple user types that runs a full round-trip through the VM without a dispatch panic | PARTIAL | `method_list_invariant_holds` confirms non-zero method_list at metadata level. Golden tests round-trip through Module::from_bytes (serializer/deserializer) without panic. No VM execution test for multi-type dispatch round-trip. |
| 4 | `cargo test` passes with zero failures in writ-compiler | VERIFIED | `cargo test -p writ-compiler`: 95/95 tests pass. Full `cargo test`: all test suites show 0 failures. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/emit/collect/contracts.rs` | `emit_reflectable_auto_impl()` helper function | VERIFIED | Function exists at line 209. Returns `(MethodDefHandle, ImplDefHandle)`. Also defines `REFLECTABLE_CONTRACT_TOKEN` constant. Sig blob encoding: `[0x00, 0x00, 0x10, <TypeRef token LE>]`. param_count=0 (no ParamDef rows for self-only method). |
| `writ-compiler/src/emit/collect/mod.rs` | Auto-impl calls after each Struct/Class/Entity/Enum collect; returns `Vec<ReflectableInfo>` | VERIFIED | `ReflectableInfo` struct defined (lines 47-54). `collect_defs` returns `Vec<ReflectableInfo>` (line 63). Auto-impl called after Struct (140-147), Class (151-158), Entity (161-169), Enum (172-180). |
| `writ-compiler/src/emit/body/mod.rs` | Synthetic get_type() body emission in `emit_all_bodies` | VERIFIED | `reflectable_infos: &[ReflectableInfo]` parameter added (line 382). Synthetic body loop at lines 598-625. Emitted BEFORE lambda bodies per ordering requirement. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `collect/mod.rs` | `collect/contracts.rs` | `emit_reflectable_auto_impl()` call after each type collection | VERIFIED | `use contracts::emit_reflectable_auto_impl` at line 28; called 4 times in main loop |
| `body/mod.rs` | `collect/mod.rs` (ReflectableInfo) | `reflectable_infos` Vec passed from `collect_defs` through `emit_bodies` to `emit_all_bodies` | VERIFIED | `emit/mod.rs` captures `reflectable_infos` from `collect_defs` (line 44 and 100); passes to `emit_all_bodies` (lines 132); body/mod.rs imports `ReflectableInfo` from `crate::emit::collect` (line 24) |
| `emit/mod.rs` | `module_builder.rs` | Post-finalize `set_impl_def_method_list` and `typedef_method_list_by_handle` | VERIFIED | Both methods exist in module_builder.rs (lines 425, 433). Called post-finalize in both `emit()` (lines 57-60) and `emit_bodies()` (lines 119-122). |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `body/mod.rs` synthetic body loop | `type_idx` in `Instruction::TypeOf` | `builder.token_for_def(info.def_id)` post-finalize | Yes — resolved from finalized MetadataToken for each TypeDef | FLOWING |
| `collect/contracts.rs` sig blob | TypeRef token for `"Type"` return type | `builder.type_ref_token_by_name("Type")` | Yes — returns registered TypeRef row 2 (1-based) for "Type" from writ-runtime | FLOWING |
| `emit/mod.rs` method_list fixup | `impl_handle.method_list` | `builder.typedef_method_list_by_handle(info.typedef_handle)` post-finalize | Yes — non-zero value confirmed by `method_list_invariant_holds` test | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 3 user types produce exactly 3 Reflectable ImplDefs | `cargo test -p writ-compiler reflectable_auto_impl_three_types` | PASS (32/32 emit_tests pass) | PASS |
| TypeDef.method_list is non-zero after finalize | `cargo test -p writ-compiler method_list_invariant_holds` | PASS | PASS |
| impl_emits_impldef count=2 (1 user + 1 auto-impl) | `cargo test -p writ-compiler impl_emits_impldef` | PASS | PASS |
| Golden tests round-trip serialization without panic | `cargo test -p writ-golden` (48/48 pass) | PASS | PASS |
| Full compiler test suite | `cargo test -p writ-compiler` (95/95 pass) | PASS | PASS |
| Full workspace test suite | `cargo test` (all suites 0 failures) | PASS | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| COMP-03 | 105-01-PLAN.md | Reflectable auto-impl emitted per user TypeDef interleaved in main codegen pass | SATISFIED | `emit_reflectable_auto_impl()` called inline in `collect_defs` after each user-type arm; `reflectable_auto_impl_three_types` test confirms count; golden files show interleaved `.impl` blocks |
| REFL-02 | 105-01-PLAN.md | expr.get_type() returns runtime dynamic Type via Reflectable contract dispatch | PARTIAL | Compiler emits TYPEOF+RET bodies correctly (golden tests confirm). Runtime-level CALL_VIRT dispatch tested in `reflection_tests.rs` for primitives and raw ModuleBuilder modules. Full-pipeline Writ->VM dispatch test for user-defined types is absent. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `body/mod.rs` | 607-608 | `let self_ty = Ty(0)` and `let type_ty = Ty(0)` — placeholder type values for synthetic body reg_types | Info | Documented as intentional: "type doesn't affect IL execution". No real impact since TYPEOF uses only `r_dst` and `type_idx`, not register types. Not a stub — the instructions are real. |

### Human Verification Required

#### 1. End-to-End get_type() Dispatch for User-Defined Types

**Test:** Write a Writ source file with a user struct/entity, compile it with `emit_bodies`, run it in the writ-runtime VM, call get_type() on an instance via CALL_VIRT (Reflectable contract, slot 0), and read the returned Type heap object's `name` field.

**Expected:** The `name` field on the Type heap object equals the struct/entity name string (e.g., `"Point"` for `struct Point`).

**Why human:** writ-golden tests are compile+disassemble only (no VM execution). writ-runtime reflection tests use raw ModuleBuilder, not the compiler pipeline. Verifying the full chain "Writ source -> compiler -> binary -> VM -> Type.name == struct name" requires writing a new integration test or running manually.

### Gaps Summary

One gap against the ROADMAP success criteria:

**Success Criterion 2** — "Calling get_type() on an instance of each user type returns a Type object with the correct name field — verified by a golden test that runs the compiled module" — is not fully satisfied. The golden tests verify correct IL emission (disassembler output) but do not execute the compiled module in the runtime. The writ-runtime has reflection dispatch tests, but these are constructed using raw ModuleBuilder rather than the compiler pipeline. There is no end-to-end test that: (1) compiles Writ source with user-defined types through the full pipeline, (2) runs the resulting module in the VM, (3) dispatches CALL_VIRT get_type() on a user-type instance, and (4) asserts the returned Type object has the correct name.

The compiler-side implementation is complete and correct (TYPEOF+RET bodies emitted, ImplDef.method_list wired, all tests pass). The gap is a missing integration test, not a missing feature. The runtime dispatch infrastructure (from Phase 103) is already in place.

**Note on missing test:** The PLAN's task 2 done criteria listed `get_type_body_is_typeof_ret` test in `emit_body_tests.rs`. This test was not created. Coverage for TYPEOF+RET body correctness is indirect: golden round-trip tests serialize/deserialize the instructions and the disassembler output shows `TYPEOF r1, <token> / RET r1`. The existing `emit_typeof_struct` test in emit_body_tests.rs covers expression-level TypeOf but not the synthetic auto-impl body emission path specifically.

---

_Verified: 2026-03-28T15:00:00Z_
_Verifier: Claude (gsd-verifier)_
