# Quick Task 260320-3wc: Fix SetField using raw MetadataToken as field index — Summary

## Problem

`SetField: field index 83886081 out of range for struct with 2 fields` when running `new SomeStruct{ x: 0, y: Ok(1) }` via DAP.

83886081 = 0x05000001 = FieldDef table (0x05) + row 1. The compiler's `field_token_by_name` emitted MetadataTokens (with table prefix) as field indices, but the runtime uses them directly as 0-based array indices.

Same class of bug as the `exec_new` type_idx fix (0x02 table prefix).

## Fix

Changed the compiler to emit **0-based local field indices** instead of MetadataTokens:

- `field_token_by_name()` — counts fields within the parent type, returns local offset
- `field_token_by_name_on_closure()` — same fix for closure capture structs
- Assembler/disassembler updated to treat field_idx as plain integer, not token

This is consistent with Range construction (already uses 0-based) and EXTRACT_FIELD (spec says "zero-based").

## Files Changed

- `writ-compiler/src/emit/module_builder.rs` — field_token_by_name returns 0-based index
- `writ-assembler/src/assembler.rs` — GET_FIELD/SET_FIELD parse field_idx as int_lit
- `writ-assembler/src/disassembler.rs` — GET_FIELD/SET_FIELD display field_idx as plain number
- `writ-cli/tests/e2e_compile_tests.rs` — added test_struct_field_get_set

## Tests

All 178 tests pass (89 VM + 7 E2E + 81 typecheck + 1 assembler).
