---
phase: quick-260322-wfw
plan: 01
subsystem: compiler/emitter, compiler/checker, lsp
tags: [for-range, loop, compiler-bug, lsp-crash]
dependency_graph:
  requires: []
  provides: [for-range-compilation, range-iteration-emission]
  affects: [writ-compiler, writ-lsp, writ-golden]
tech_stack:
  added: []
  patterns: [counter-based-range-iteration, expression-before-type-dispatch]
key_files:
  created:
    - writ-golden/tests/golden/ctrl_for_range.writ
    - writ-golden/tests/golden/ctrl_for_range.writil
    - writ-golden/tests/golden/ctrl_for_range.writc
  modified:
    - writ-compiler/src/check/check_stmt.rs
    - writ-compiler/src/check/check_expr/mod.rs
    - writ-compiler/src/emit/body/stmt.rs
    - writ-golden/tests/golden_tests.rs
    - writ-lsp/src/analysis_host.rs
decisions:
  - "Range iteration uses counter-based loop (no Range struct allocation needed)"
  - "Inclusive ranges use end+1 with CmpLtI (no CmpLeI in the VM)"
  - "Expression-based dispatch (TypedExpr::Range) before type-based dispatch in emitter"
  - "Inclusive flag propagated from AST RangeKind::Inclusive into TypedExpr::Range"
metrics:
  duration: "5m 32s"
  completed: "2026-03-22"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 8
---

# Quick Task 260322-wfw: Fix LSP Stacktrace and Compiler Bug with For-Range Loops

For-range loops (`for i in 0..5`, `for i in 1..=5`) now compile to counter-based iteration IL with correct inclusive flag propagation from AST through checker to emitter.

## What Changed

### Task 1: Fix type checker and emitter (130ff05)

**Type checker** (`check_stmt.rs`): The for-loop handler only recognized `TyKind::Array` iterables, falling through to error type for everything else. Added detection for `TypedExpr::Range` expressions, assigning `int` as the binding type (range elements are always integers).

**Inclusive flag** (`check_expr/mod.rs`): The Range expression handler hardcoded `inclusive: false`. Fixed to propagate `RangeKind::Inclusive` from the AST, enabling `..=` syntax to work correctly.

**Emitter** (`stmt.rs`): Added expression-based dispatch in `emit_for_loop` -- checks for `TypedExpr::Range` before the type-based match (necessary because Range has ty=int, same as TyKind::Int). New `emit_for_range` helper generates:
- LOAD_INT start (default 0), LOAD_INT end
- For inclusive: ADD_I end+1 to get limit (no CmpLeI in the VM)
- CMP_LT_I / BR_FALSE loop, body, ADD_I increment, BR back

### Task 2: Golden test and LSP regression test (0acb9a6)

- Golden test `ctrl_for_range`: exclusive range `0..5` with sum accumulation
- LSP unit test `test_for_range_no_runtime_crash`: verifies no error diagnostics
- User's original script (`for i in 2..n`) confirmed to compile and run without crash

## Deviations from Plan

None -- plan executed exactly as written.

## Known Stubs

None.

## Verification Results

- `cargo build -p writ-compiler` -- compiles cleanly
- `cargo test -p writ-golden` -- 40/40 pass (including new ctrl_for_range)
- `cargo test -p writ-lsp --lib` -- 112/112 pass (including new for_range test)
- `cargo test -p writ-runtime` -- passes (no regressions)
- User script `for i in 2..n { ... }` compiles and runs without crash

## Self-Check: PASSED

All created files exist. All commits verified.
