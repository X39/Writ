---
phase: 54-lsp-navigation-and-completions
plan: 01
subsystem: writ-compiler, writ-lsp
tags: [compiler, lsp, typecheck, queries, analysis]
dependency_graph:
  requires: [53-03]
  provides: [typed-ast-in-analysis-result, display-named, queries-module, analysis-cache, server-capabilities]
  affects: [writ-compiler/src/check/mod.rs, writ-compiler/src/check/ty.rs, writ-lsp/src/analysis_host.rs, writ-lsp/src/backend.rs, writ-lsp/src/queries.rs, writ-lsp/src/lib.rs]
tech_stack:
  added: []
  patterns: [Arc<AnalysisResult> cache, DashMap cache per URI, UTF-16 position conversion, narrowest-span tree walking]
key_files:
  created:
    - writ-lsp/src/queries.rs
  modified:
    - writ-compiler/src/check/mod.rs
    - writ-compiler/src/check/ty.rs
    - writ-lsp/src/analysis_host.rs
    - writ-lsp/src/backend.rs
    - writ-lsp/src/lib.rs
    - writ-cli/src/main.rs
    - writ-cli/tests/e2e_compile_tests.rs
    - writ-golden/tests/golden_tests.rs
    - writ-compiler/tests/typecheck_tests.rs
    - writ-compiler/tests/emit_tests.rs
decisions:
  - "typecheck() returns 4-tuple (TypedAst, TyInterner, TypeEnv, Vec<Diagnostic>) — TypeEnv preserved for LSP use"
  - "AnalysisResult carries Option<TypedAst/TyInterner/TypeEnv> — None on panic or parse failure"
  - "analysis_cache uses Arc<AnalysisResult> per URI — zero-copy sharing between publish and handler paths"
  - "publish_grouped_diagnostics takes &Arc<AnalysisResult> — deref-transparent, no cloning needed"
metrics:
  duration: 6min
  completed: 2026-03-14
  tasks_completed: 2
  files_changed: 11
---

# Phase 54 Plan 01: LSP Navigation Infrastructure Summary

**One-liner:** Extended compiler and LSP with TypeEnv return, display_named, queries.rs position walker, Arc analysis cache, and full Phase 54 ServerCapabilities registration.

## What Was Built

### Task 1: Extend typecheck return type and add display_named

Modified `writ-compiler/src/check/mod.rs` to return `(TypedAst, TyInterner, TypeEnv, Vec<Diagnostic>)` from `typecheck()`. Previously the `TypeEnv` was dropped after struct field extraction; now it is returned as the 3rd tuple element so LSP handlers can access function signatures, struct/entity/enum fields, impl associations, and other type-level metadata.

Added `TyInterner::display_named()` in `writ-compiler/src/check/ty.rs`. Unlike `display()` which shows "struct"/"entity"/"enum" for named types, `display_named()` looks up the actual name from the `DefMap` (e.g., "Potion" instead of "struct"). Falls back to `display()` for primitives, GenericParam, Infer, and Error types.

Updated all 8 callers of `typecheck()` to use `_type_env` in the destructuring pattern (the LSP analysis host uses a proper variable name for later use).

### Task 2: queries.rs, extended AnalysisResult, analysis cache, ServerCapabilities

Created `writ-lsp/src/queries.rs` with three exported functions:

- `position_to_byte_offset(source, pos)` — converts LSP Position (0-based line, UTF-16 character) to byte offset; correctly handles multi-byte UTF-8 chars with 2-unit UTF-16 surrogates
- `expr_at_offset(ast, offset)` — walks all body-containing declarations (Fn, Impl methods, Const, Global) and finds the narrowest TypedExpr whose span contains the given offset; returns the best match across the full AST
- `find_def_id_at_offset(expr, def_map)` — extracts DefId from Call (callee_def_id), Var (name lookup), New (target_def_id), and Path (FQN join) expressions

Added 7 unit tests: 4 for position_to_byte_offset (start, second-line, UTF-16 emoji, out-of-bounds) and 3 for expr_at_offset (finds Var in tail position, finds Call/Var in call site, finds Literal inside impl method body).

Extended `AnalysisResult` in `analysis_host.rs` with `typed_ast`, `ty_interner`, and `type_env` optional fields. Both `analyze_standalone` and `analyze_project` now capture the typecheck output on Ok and store it; all early returns set all three to None.

Added `analysis_cache: DashMap<String, Arc<AnalysisResult>>` to `Backend`. On successful analysis, the result is Arc-wrapped and inserted keyed by URI string. `did_close` removes it. `publish_grouped_diagnostics` now accepts `&Arc<AnalysisResult>` (Arc derefs transparently, no behavior change).

Registered all Phase 54 `ServerCapabilities` in `initialize`: hover (Simple(true)), definition (Left(true)), references (Left(true)), completion (trigger chars ".", ":"), signature_help (trigger chars "(", ","), semantic_tokens (8 token types, full: Bool(true)).

## Files Modified

| File | Change |
|------|--------|
| `writ-compiler/src/check/mod.rs` | typecheck() returns 4-tuple including TypeEnv |
| `writ-compiler/src/check/ty.rs` | Added display_named() method |
| `writ-lsp/src/queries.rs` | New file: position_to_byte_offset, expr_at_offset, find_def_id_at_offset + tests |
| `writ-lsp/src/analysis_host.rs` | AnalysisResult extended; both analyze_ functions capture typed output |
| `writ-lsp/src/backend.rs` | analysis_cache field, Arc wrapping, ServerCapabilities |
| `writ-lsp/src/lib.rs` | Added pub mod queries |
| `writ-cli/src/main.rs` | Updated destructuring |
| `writ-cli/tests/e2e_compile_tests.rs` | Updated 2 destructurings |
| `writ-golden/tests/golden_tests.rs` | Updated 2 destructurings |
| `writ-compiler/tests/typecheck_tests.rs` | Updated 1 destructuring |
| `writ-compiler/tests/emit_tests.rs` | Updated 1 destructuring |

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 0cf7379 | feat(54-01): extend typecheck return type and add display_named |
| 2 | 666fa9a | feat(54-01): create queries.rs, extend AnalysisResult, add analysis cache and ServerCapabilities |

## Deviations from Plan

None — plan executed exactly as written.

## Verification

- `cargo test --workspace`: all test suites pass (zero failures)
- `cargo test -p writ-lsp`: 30 tests pass including 7 new query tests
- `cargo build -p writ-lsp`: compiles successfully
- AnalysisResult has typed_ast, ty_interner, type_env fields: confirmed
- TyInterner has display_named method: confirmed
- queries.rs exports position_to_byte_offset, expr_at_offset, find_def_id_at_offset: confirmed
- Backend has analysis_cache field: confirmed
- ServerCapabilities includes all Phase 54 capabilities: confirmed

## Self-Check: PASSED
