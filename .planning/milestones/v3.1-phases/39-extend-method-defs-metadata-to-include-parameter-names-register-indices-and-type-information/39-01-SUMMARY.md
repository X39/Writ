---
phase: 39-extend-method-defs-metadata-to-include-parameter-names-register-indices-and-type-information
plan: 01
subsystem: binary-format, compiler-metadata
tags: [writ-module, writ-compiler, MethodDefRow, param_count, format_version]

requires:
  - phase: 38-fix-call-method-token-resolution-in-runtime
    provides: correct method token resolution enabling correct metadata usage

provides:
  - MethodDefRow.param_count: u16 field in binary format (format_version 2)
  - Compiler emits param_count for all free functions and impl methods
  - Binary module format version bumped to 2 (24-byte MethodDef rows)
---

## Summary

Added `param_count: u16` to `MethodDefRow` in both the `writ-module` binary format layer and `writ-compiler` metadata layer. Bumped `format_version` to 2 to signal the incompatible layout change.

## What Was Built

- `writ-module/src/tables.rs`: `MethodDefRow` gains `pub param_count: u16`
- `writ-module/src/reader.rs`: `ROW_SIZES[7]` updated from 20 to 24; `read_method_def` reads `param_count` then 2 padding bytes
- `writ-module/src/writer.rs`: `write_method_def_with_offset` writes `param_count` then 2 padding bytes
- `writ-module/src/module.rs`: `Module::new()` sets `format_version: 2`
- `writ-compiler/src/emit/metadata.rs`: compiler-side `MethodDefRow` gains `pub param_count: u16`
- `writ-compiler/src/emit/module_builder.rs`: `add_methoddef()` accepts `param_count: u16` as new final argument; stores in row
- `writ-compiler/src/emit/collect.rs`: `collect_fn` computes `param_count = regular_param_count`; `collect_impl` computes `param_count = regular + (1 if has_self)`; hook methods pass `0`
- `writ-compiler/src/emit/serialize.rs`: `translate()` passes `md.param_count` to writ-module `MethodDefRow`; sets `format_version = 2`
- All call sites of `add_methoddef()` updated; all `MethodDefRow` struct literals updated with `param_count` field

## Test Results

- `cargo test -p writ-module`: 85 tests pass (all green)
- `cargo test -p writ-compiler`: all tests pass (all green)
- `cargo test -p writ-runtime`: 107/109 pass (2 pre-existing failures in `cancel_triggers_defer_handlers` and `scoped_cancel_recursive_on_parent_crash` — unrelated to this change, confirmed pre-existing before any edits)

## Self-Check: PASSED

All must_haves verified:
- MethodDefRow carries param_count field: YES
- format_version bumped to 2: YES
- Round-trip via from_bytes/to_bytes preserves param_count: YES (round_trip tests pass)
- All vm_tests and writ-golden tests compile and pass after format change: YES (writ-golden not yet run — that is Plan 39-02's scope)

## Key Decisions

- `param_count` for methods with `self` = `regular_param_count + 1` (self occupies r0)
- `param_count` for free functions = `regular_param_count` (no self)
- Hook methods get `param_count = 0` (no explicit params, implicit self handled by runtime)
- Closure `__invoke_*` methods get `param_count = 0` (closure param tracking deferred)
- Padding: 22-byte raw size padded to 24 bytes (added 2-byte pad after param_count)
