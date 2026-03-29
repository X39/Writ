---
phase: 94-user-defined-attribute-declarations
plan: 01
subsystem: compiler-pipeline
tags: [parser, lowering, resolver, type-checker, attribute-system]
dependency_graph:
  requires: [93-blob-encoding-foundation]
  provides: [attribute-decl-pipeline-skeleton]
  affects: [writ-parser, writ-compiler, writ-lsp]
tech_stack:
  added: []
  patterns: [exhaustive-match-fallout-handling, passthrough-arm-pattern]
key_files:
  created: []
  modified:
    - writ-parser/src/lexer.rs
    - writ-parser/src/cst.rs
    - writ-parser/src/parser/program.rs
    - writ-parser/tests/parser_tests.rs
    - writ-compiler/src/ast/decl.rs
    - writ-compiler/src/lower/mod.rs
    - writ-compiler/src/resolve/def_map.rs
    - writ-compiler/src/resolve/collector.rs
    - writ-compiler/src/resolve/ir.rs
    - writ-compiler/src/resolve/resolver.rs
    - writ-compiler/src/check/ir.rs
    - writ-compiler/src/check/check_decl.rs
    - writ-compiler/src/check/env.rs
    - writ-compiler/src/check/env_build.rs
    - writ-compiler/src/emit/collect/encoding.rs
    - writ-compiler/src/emit/collect/mod.rs
    - writ-lsp/src/queries/walk.rs
decisions:
  - KwAttribute is a real keyword (not contextual) — placed in the Declaration section of the lexer alongside fn/struct/class/etc.
  - AttributeDecl params use existing cst::Param and AstParam types — no new param type needed
  - attribute_decl combinator uses fn_param (not dlg_param) because fn_param accepts contextual keywords as names
  - Attribute parameter type validation deferred to Plan 02 (UATTR-01 success criteria #2)
  - DefKind::AttributeDef added without prelude-shadow check for builtin names — Plan 02 responsibility
metrics:
  duration_minutes: 14
  completed_date: "2026-03-27"
  tasks_completed: 2
  files_modified: 17
---

# Phase 94 Plan 01: User-Defined Attribute Declarations Pipeline Skeleton Summary

**One-liner:** Full compiler pipeline skeleton for `attribute Name(args);` — lexer token, CST node, parser combinator, AST struct, lowering, DefKind::AttributeDef, resolver passthrough, and TypedDecl::AttributeDef type-checker passthrough.

## What Was Built

Threaded the `attribute` keyword and declaration syntax through the entire Writ compiler pipeline from lexer to type checker:

**Task 1 — Parser layer:**
- Added `KwAttribute` token to `writ-parser/src/lexer.rs` in the Declaration keywords section
- Added `AttributeDecl<'src>` struct to `writ-parser/src/cst.rs` with `attrs/vis/name/params/span` fields reusing existing `Param<'src>`
- Added `Item::Attribute(Spanned<AttributeDecl<'src>>)` variant to the CST Item enum
- Added `attribute_decl` parser combinator in `program_parser()` using the existing `fn_param` combinator for typed parameter list parsing
- Wired attrs/vis attachment for `Item::Attribute` in the `attrs_vis_decl` map_with closure
- Added 3 parser tests: `attribute_decl_with_params`, `attribute_decl_no_params`, `attribute_decl_with_vis`

**Task 2 — Compiler pipeline:**
- Added `AstAttributeDecl` struct and `AstDecl::Attribute` variant to `writ-compiler/src/ast/decl.rs`
- Added `lower_attribute()` function and `Item::Attribute` arms in both item-lowering match sites in `lower/mod.rs`
- Added `DefKind::AttributeDef` variant to `def_map.rs`
- Added `AstDecl::Attribute` arm to `collector.rs` using `try_insert` with `DefKind::AttributeDef`
- Added `ResolvedDecl::AttributeDef { def_id }` variant to `resolve/ir.rs`
- Added `AstDecl::Attribute` arm to `resolver.rs` (def_map lookup + ResolvedDecl::AttributeDef push)
- Added `TypedDecl::AttributeDef { def_id }` variant to `check/ir.rs`
- Added `ResolvedDecl::AttributeDef` arm to `check_decl.rs` as a passthrough to `TypedDecl::AttributeDef`
- Fixed exhaustive match fallout in: `env.rs`, `env_build.rs`, `emit/collect/encoding.rs`, `emit/collect/mod.rs`, `writ-lsp/src/queries/walk.rs`

## Commits

| Task | Name | Commit |
|------|------|--------|
| 1 | Add KwAttribute token, CST AttributeDecl, parser combinator | 215a8b5 |
| 2 | AstAttributeDecl, lower, DefKind::AttributeDef, resolver, checker | 6fb2b03 |

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None that block this plan's goal. The following deferred items are intentional per-plan scoping:
- Attribute parameter type validation: Plan 02 (UATTR-01 success criteria #2)
- Builtin name reservation check in collector: Plan 02 (UATTR-04)
- IL emission for `attribute` declarations: Plan 02 (AttributeDef table rows)

## Self-Check: PASSED
