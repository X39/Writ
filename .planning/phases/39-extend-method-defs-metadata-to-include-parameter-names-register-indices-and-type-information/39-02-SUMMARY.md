---
phase: 39-extend-method-defs-metadata-to-include-parameter-names-register-indices-and-type-information
plan: 02
subsystem: disassembler, golden-tests
tags: [writ-assembler, disassembler, golden, ParamDef, param_names]

requires:
  - phase: 39 plan 01
    provides: MethodDef.param_count in binary format enabling ParamDef lookup

provides:
  - Disassembler shows "name: type" for parameters using ParamDef table
  - All 4 golden .expected files re-blessed with named-parameter format
---

## Summary

Updated the disassembler to display parameter names alongside types in method signatures, using `MethodDef.param_count` to slice `ParamDef` rows. Re-blessed all 4 golden fixtures to lock the new output format.

## What Was Built

- `writ-assembler/src/disassembler.rs`:
  - Precomputes `method_param_start` cumulative offset table using `md.param_count` for each MethodDef
  - `get_param_names(method_idx)` closure slices `module.param_defs[start..end]` and returns `Vec<String>` of names
  - `decode_method_sig()` updated: accepts `param_names: &[String]` as new parameter; renders `"name: type"` when name is non-empty, falls back to `"type"` when name is empty
  - Section 3 (contract methods): passes `&[]` (no ParamDef for contract slots)
  - Section 4 (impl methods): passes `get_param_names(real_idx)`
  - Section 6 (extern defs): passes `&[]` (no MethodDef entry for extern fns)
  - Section 7 (top-level methods): passes `get_param_names(mi)`
- `writ-golden/tests/golden/fn_typed_params.expected`: re-blessed — shows `(a: int, b: int) -> int` and `(n: int) -> bool`
- `writ-golden/tests/golden/fn_recursion.expected`: re-blessed — shows `(n: int) -> int` for `factorial`
- `fn_basic_call.expected` and `fn_empty_main.expected`: unchanged (zero-param methods, no diff)

## Test Results

- `cargo test -p writ-golden`: 7/7 tests pass (after BLESS=1 run to re-generate)
- `cargo build -p writ-assembler`: compiles clean

## Self-Check: PASSED

All must_haves verified:
- Disassembled signatures show "name: type": YES (`(a: int, b: int)`, `(n: int)`)
- Param names from ParamDef table: YES (param_defs sliced by method_param_start)
- All four golden fixture tests pass: YES
- Zero-param methods unchanged: YES (`() -> void` still renders correctly)
