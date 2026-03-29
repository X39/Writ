---
phase: 116-array-primitives-string-utilities-host-value-construction
plan: "01"
subsystem: compiler+runtime
tags: [array-methods, opcodes, array-contains, golden-test]
dependency_graph:
  requires: []
  provides: [array-add, array-remove_at, array-insert, array-contains, array-slice]
  affects: [writ-module, writ-compiler, writ-runtime, writ-golden]
tech_stack:
  added: []
  patterns: [opcode-extension, builtin-method, dot-call-resolution]
key_files:
  created:
    - writ-golden/tests/golden/array_primitives.writ
    - writ-golden/tests/golden/array_primitives.writil
  modified:
    - writ-module/src/instruction.rs
    - writ-compiler/src/check/check_expr/access.rs
    - writ-compiler/src/emit/body/expr/builtins.rs
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/dispatch/objects.rs
    - writ-golden/tests/golden/type_array_ops.writ
    - writ-golden/tests/golden_tests.rs
decisions:
  - "ArrayContains uses opcode 0x0909 (next after ArraySlice 0x0908) — RRR shape, 8B"
  - "String element equality in ArrayContains uses heap.read_string content comparison, not address equality"
  - "push renamed to add in both access.rs and builtins.rs simultaneously to avoid partial-rename failure"
  - "Golden test uses let _ bindings instead of print() since print is not a built-in in the test harness"
  - "type_array_ops.writ updated from push to add to fix pre-existing golden test failure"
metrics:
  duration: "~20 min"
  completed_date: "2026-03-29"
  tasks_completed: 2
  files_changed: 9
---

# Phase 116 Plan 01: Array Primitives Summary

Wired array mutation methods (add, remove_at, insert, contains, slice) through the compiler type-checker and emitter, added the `ArrayContains` opcode (0x0909) end-to-end through instruction set, emitter, and runtime dispatch, and proved correctness with a golden test.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add ArrayContains opcode and wire all array methods | da50835 + 62d74d7 | instruction.rs, access.rs, builtins.rs, dispatch/mod.rs, dispatch/objects.rs |
| 2 | Golden test for all array primitives | 6686b08 | array_primitives.writ, array_primitives.writil, type_array_ops.writ, golden_tests.rs |

## What Was Built

### ArrayContains Opcode (0x0909)

New `ArrayContains { r_dst: u16, r_arr: u16, r_val: u16 }` instruction variant:
- **Encoding:** opcode 0x0909, Shape RRR (8B) — three u16 registers LE-encoded
- **Runtime:** `exec_array_contains` in `dispatch/objects.rs` performs a linear scan comparing each element against the target. For `Value::Ref` (strings): reads string content via `heap.read_string` and compares character content (not heap addresses). For `Value::Int`/`Value::Float`/`Value::Bool`: uses PartialEq directly.

### Array Method Wiring (Compiler Layers)

All five array methods wired through the type checker (`TyKind::Array` arm in `access.rs`) and emitter (`TyKind::Array` arm in `builtins.rs`):

| Method | Type Signature | Emitted Instruction |
|--------|---------------|---------------------|
| `add(x: T)` | `(T) -> void` | `ArrayAdd { r_arr, r_val }` |
| `remove_at(i: int)` | `(int) -> void` | `ArrayRemove { r_arr, r_idx }` |
| `insert(i: int, x: T)` | `(int, T) -> void` | `ArrayInsert { r_arr, r_idx, r_val }` |
| `contains(x: T)` | `(T) -> bool` | `ArrayContains { r_dst, r_arr, r_val }` |
| `slice(s: int, e: int)` | `(int, int) -> T[]` | `ArraySlice { r_dst, r_arr, r_start, r_end }` |

The `push` alias was renamed to `add` in both `access.rs` and `builtins.rs` simultaneously.

### Golden Test

`writ-golden/tests/golden/array_primitives.writ` exercises all five methods in a single function, including:
- Integer array add/remove/insert/contains/slice operations
- String array `contains` with content equality (not address equality)

The generated `.writil` confirms all five opcodes appear in the disassembly: `ARRAY_ADD`, `ARRAY_REMOVE`, `ARRAY_INSERT`, `ARRAY_CONTAINS`, `ARRAY_SLICE`.

## Verification Results

- `cargo test -p writ-module --lib` — 0 tests (no unit tests), compiles clean
- `cargo test -p writ-compiler --lib` — 27/27 pass
- `cargo test -p writ-runtime --lib` — 156/156 pass
- `cargo test -p writ-golden -- array_primitives` — 1/1 pass
- `cargo test -p writ-golden` — 60/60 pass (was 58/59 before fix)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] type_array_ops.writ used renamed `push` method**
- **Found during:** Task 2 (running full golden suite exposed failure)
- **Issue:** `writ-golden/tests/golden/type_array_ops.writ` called `arr.push(99)` which no longer exists — the method was renamed to `add` in Task 1
- **Fix:** Updated `type_array_ops.writ` to use `arr.add(99)` and regenerated the `.writil` snapshot
- **Files modified:** `writ-golden/tests/golden/type_array_ops.writ`
- **Commit:** 6686b08

**2. [Rule 1 - Bug] Plan-specified golden test used print() which is not a built-in**
- **Found during:** Task 2 (discovered from 116-02 SUMMARY — same issue occurred there)
- **Issue:** Plan's sample golden test code used `print()` calls, but `print` is not a built-in function in the Writ golden test harness
- **Fix:** Rewrote golden test to assign results to `let _var = ...` bindings — still exercises all 5 opcodes in the IL output
- **Files modified:** `writ-golden/tests/golden/array_primitives.writ`
- **Commit:** 6686b08

**3. [Info - Merge] 116-01 Task 1 code was included in 116-02 commit da50835**
- Due to parallel worktree execution, the array method wiring (access.rs, builtins.rs, instruction.rs, dispatch/mod.rs) was committed as part of 116-02 Task 1 commit. The `exec_array_contains` runtime function landed in 116-03 commit 62d74d7. The net effect is identical — all code is present and correct on the branch.

## Self-Check: PASSED

- writ-module/src/instruction.rs: FOUND (ArrayContains at line 157)
- writ-compiler/src/check/check_expr/access.rs: FOUND (add/remove_at/insert/contains at lines 185-191)
- writ-compiler/src/emit/body/expr/builtins.rs: FOUND (add/remove_at/insert/contains at lines 96-120)
- writ-runtime/src/dispatch/objects.rs: FOUND (exec_array_contains)
- writ-golden/tests/golden/array_primitives.writ: FOUND
- writ-golden/tests/golden/array_primitives.writil: FOUND
- Commit 6686b08: FOUND
