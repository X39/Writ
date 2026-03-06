---
phase: quick-260319-gjo
plan: 01
subsystem: compiler/emit
tags: [bug-fix, force-unwrap, TypedExpr, codegen, IL]
dependency_graph:
  requires: []
  provides: [force-unwrap-compilation]
  affects: [writ-compiler/check/ir, writ-compiler/check/desugar, writ-compiler/emit]
tech_stack:
  added: []
  patterns: [TypedExpr leaf node pattern, IL Crash instruction emission]
key_files:
  created:
    - writ-golden/tests/golden/force_unwrap.writ
    - writ-golden/tests/golden/force_unwrap.writil
  modified:
    - writ-compiler/src/check/ir.rs
    - writ-compiler/src/check/desugar.rs
    - writ-compiler/src/emit/body/mod.rs
    - writ-compiler/src/emit/body/expr/mod.rs
    - writ-compiler/src/emit/body/closure.rs
    - writ-compiler/src/emit/collect/walker.rs
    - writ-compiler/tests/typecheck_tests.rs
    - writ-golden/tests/golden_tests.rs
decisions:
  - "TypedExpr::Crash uses Ty(3) (String) for r_msg register — matches primitive pre-interning order"
  - "Crash result register allocated as *ty for type continuity even though it is unreachable at runtime"
metrics:
  duration: "~32 minutes"
  completed: "2026-03-19"
  tasks_completed: 2
  files_modified: 8
---

# Quick Task 260319-gjo: Fix Force-Unwrap Compilation Bug Summary

**One-liner:** Added TypedExpr::Crash variant to distinguish intentional runtime crashes from compilation errors, fixing the force-unwrap operator (`n!`) which was rejected by the `expr_has_error` pre-pass.

## What Was Done

The force-unwrap operator (`n!`) on `Option<T>` and `Result<T, E>` was desugaring correctly to a `match` expression, but the crash arm of that match was using `TypedExpr::Error` as a placeholder. `TypedExpr::Error` is the compilation-error sentinel — it triggers `expr_has_error` to return `true`, which causes `emit_all_bodies` to skip the entire function with an E9001 diagnostic.

**Root cause:** `build_unwrap_match` in `desugar.rs` emitted `TypedExpr::Error` for the "unreachable" None/Err arm, confusing a runtime-only crash with a type-checking failure.

**Fix:** Introduced a new `TypedExpr::Crash { ty, span, message }` variant that represents an intentional runtime panic (not a compiler error). The emitter converts this to `LoadString + Instruction::Crash` in the IL.

## Task 1: Add TypedExpr::Crash variant and update desugar.rs (commit 2b2dd1d)

1. Added `Crash { ty, span, message }` variant to `TypedExpr` in `writ-compiler/src/check/ir.rs`
2. Added `Crash` to the `ty()` and `span()` match arms in `impl TypedExpr`
3. Updated `build_unwrap_match` in `desugar.rs` to use `TypedExpr::Crash` instead of `TypedExpr::Error`

## Task 2: Handle TypedExpr::Crash in all emitter pattern matches (commit a577962)

1. **`expr_has_error` in `mod.rs`** — Added `TypedExpr::Crash { .. }` to the leaf-node arm that returns `false`. This is the critical fix that stops the pre-pass from treating `Crash` as a compilation error.
2. **`collect_lambda_bodies_from_expr` in `mod.rs`** — Added `Crash` to leaf arm (no children to recurse).
3. **`emit_expr` in `expr/mod.rs`** — Added a new match arm that emits `LoadString` (crash message) followed by `Instruction::Crash`.
4. **`scan_expr_for_lambdas` in `closure.rs`** — Added `Crash` to leaf arm.
5. **`walk_expr` in `walker.rs`** — Added `Crash` to leaf arm.
6. Added 2 typecheck regression tests (`force_unwrap_option_no_errors`, `force_unwrap_result_no_errors`) in `typecheck_tests.rs`.
7. Added golden test `force_unwrap` verifying the IL output contains `LOAD_STRING + CRASH` instructions.

## Generated IL

The force-unwrap `n!` on `int?` now compiles to:

```
BR 34              ; branch to success path
LOAD_STRING r4, 56 ; load crash message "unwrap failed: value is None/Err"
CRASH r4           ; emit runtime crash
MOV r3, r5         ; unreachable, but satisfies register continuity
BR 8               ; back to success
```

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

All key files exist: ir.rs, desugar.rs, mod.rs, expr/mod.rs, closure.rs, walker.rs, force_unwrap.writ, force_unwrap.writil

Commits verified: 2b2dd1d (Task 1), a577962 (Task 2)

Key code verified:
- `TypedExpr::Crash` variant in ir.rs (lines 168-173)
- `TypedExpr::Crash` in ty() and span() arms
- `build_unwrap_match` uses `TypedExpr::Crash` in desugar.rs
- `emit_expr` Crash arm emits LoadString + Instruction::Crash in expr/mod.rs
- `expr_has_error` treats Crash as non-error leaf node in mod.rs
