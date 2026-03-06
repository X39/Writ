---
phase: 63-writ-compiler-file-splits
plan: "03"
subsystem: writ-compiler
tags: [refactor, file-split, emit, check, expr, env]
dependency_graph:
  requires: []
  provides: [SPLIT-05, SPLIT-09]
  affects: [writ-compiler/emit/body/expr, writ-compiler/check/env]
tech_stack:
  added: []
  patterns:
    - folder-module split (expr.rs -> expr/ with mod.rs + 7 submodules)
    - sibling-file split (env.rs trimmed + env_build.rs new sibling, mod.rs declares both)
key_files:
  created:
    - writ-compiler/src/emit/body/expr/mod.rs
    - writ-compiler/src/emit/body/expr/literal.rs
    - writ-compiler/src/emit/body/expr/binary.rs
    - writ-compiler/src/emit/body/expr/control.rs
    - writ-compiler/src/emit/body/expr/construction.rs
    - writ-compiler/src/emit/body/expr/builtins.rs
    - writ-compiler/src/emit/body/expr/string.rs
    - writ-compiler/src/emit/body/expr/eq.rs
    - writ-compiler/src/check/env_build.rs
  modified:
    - writ-compiler/src/check/env.rs
    - writ-compiler/src/check/mod.rs
  deleted:
    - writ-compiler/src/emit/body/expr.rs
decisions:
  - "expr/ folder module uses pub(super) on all submodule fns — binary.rs imports eq:: directly via super::eq, not via mod.rs re-exports"
  - "env_build.rs declared in check/mod.rs (not inside env.rs) per Approach A — Rust resolves check/env_build.rs from check/mod.rs correctly"
  - "resolve_ast_type and resolve_ast_type_with_file remain pub in env_build.rs and are re-exported via pub use in env.rs for backward compatibility"
metrics:
  duration: ~30min
  completed: "2026-03-18"
  tasks_completed: 2
  files_created: 9
  files_modified: 3
  files_deleted: 1
---

# Phase 63 Plan 03: Emit Body Expr and Check Env File Splits Summary

Split `emit/body/expr.rs` (1,470 lines) into an 8-file `expr/` folder module, and split `check/env.rs` (1,032 lines) into public types (`env.rs`, 336 lines) plus private builder helpers (`env_build.rs`, 724 lines). Both splits use explicit `pub(super)` visibility — no glob re-exports anywhere.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Split emit/body/expr.rs into 8-file expr/ folder module | f7f52ed | expr/mod.rs + 7 submodules (created), expr.rs (deleted) |
| 2 | Split check/env.rs into public types + builder helpers | 326b723 | env_build.rs (created), env.rs (trimmed), mod.rs (updated) |

## Verification Results

```
cargo test -p writ-compiler: 75 passed; 0 failed
cargo clippy -p writ-compiler: zero warnings
```

Line counts:
- `expr/binary.rs`: 261 lines
- `expr/builtins.rs`: 226 lines
- `expr/construction.rs`: 149 lines
- `expr/control.rs`: 170 lines
- `expr/eq.rs`: 163 lines
- `expr/literal.rs`: 46 lines
- `expr/mod.rs`: 443 lines (largest, under 500)
- `expr/string.rs`: 75 lines
- `check/env.rs`: 336 lines (down from 1,032)
- `check/env_build.rs`: 724 lines

## Decisions Made

1. **binary.rs cross-submodule import**: `binary.rs` imports `emit_struct_eq` and `emit_struct_neq` directly via `use super::eq::{emit_struct_eq, emit_struct_neq}`. The `mod.rs` does not re-export them — submodules import from siblings directly.

2. **env_build.rs location (Approach A)**: Declared as `pub(crate) mod env_build;` in `check/mod.rs` (not inside `env.rs`). This means `env_build.rs` lives at `check/env_build.rs` and is accessed as `super::env_build::` from `env.rs`. The alternative (Approach B — convert env.rs to env/mod.rs) was avoided to keep the change minimal.

3. **resolve_ast_type backward compat**: Both `resolve_ast_type` and `resolve_ast_type_with_file` are kept `pub` in `env_build.rs` and re-exported via `pub use super::env_build::{resolve_ast_type, resolve_ast_type_with_file}` in `env.rs`. Callers importing from `check::env` see no change.

## Deviations from Plan

None — plan executed exactly as written. The only minor deviation was removing the bogus `#[allow(unused_imports)]` trick in `expr/mod.rs` during initial compilation — the original plan's suggestion to use it was unnecessary once the imports were cleaned up correctly.

## Self-Check: PASSED

Files exist:
- `writ-compiler/src/emit/body/expr/mod.rs` — FOUND
- `writ-compiler/src/emit/body/expr/binary.rs` — FOUND
- `writ-compiler/src/emit/body/expr/eq.rs` — FOUND
- `writ-compiler/src/check/env_build.rs` — FOUND
- `writ-compiler/src/check/env.rs` — FOUND (trimmed)

Commits exist:
- `f7f52ed` — FOUND (Task 1)
- `326b723` — FOUND (Task 2)
