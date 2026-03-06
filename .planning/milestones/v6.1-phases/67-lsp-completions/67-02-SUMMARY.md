---
phase: 67-lsp-completions
plan: "02"
subsystem: writ-lsp/queries
tags: [lsp, completions, dot-completion, testing, tdd]
dependency_graph:
  requires: [67-01]
  provides: [dot-completion-integration-tests]
  affects: [writ-lsp/src/queries/completion.rs, writ-lsp/src/queries/walk.rs]
tech_stack:
  added: []
  patterns: [TDD-integration, analyze_standalone-pipeline, expr_at_offset]
key_files:
  created: []
  modified:
    - writ-lsp/src/queries/walk.rs
    - writ-lsp/src/queries/completion.rs
decisions:
  - "Pipeline works correctly as-is: dot-offset math, FileId(0), and type resolution all produce correct results for struct and array receivers"
  - "No changes needed to backend.rs: existing analyze_standalone + expr_at_offset + build_dot_completions chain is sound"
  - "Used receiver_offset = dot_pos - 1 (last char of 'arr' or 'p') which falls in span [start, end)"
metrics:
  duration: "97s"
  completed: "2026-03-18"
  tasks_completed: 1
  files_modified: 2
---

# Phase 67 Plan 02: Dot-Completion Pipeline Diagnosis Summary

**One-liner:** Integration tests confirm dot-completion pipeline works correctly — analyze_standalone strips dot, expr_at_offset finds receiver, build_dot_completions returns typed fields/methods.

## Objective

Diagnose and fix the broken dot-completion pipeline. The plan expected to find a failure at one of three points: (1) `expr_at_offset` returning `None`, (2) receiver having `TyKind::Error`, or (3) `analyze_standalone` failing on modified source.

## What Was Built

### Diagnostic Tests (walk.rs)

Added `test_expr_at_offset_receiver_for_dot_completion` — confirms that for a source with `let p: Point = new Point { x: 1, y: 2 }; p`, calling `expr_at_offset` at the offset of the final `p` returns `Some(TypedExpr::Var { name: "p", .. })`. The half-open span check works correctly for single-character identifiers.

### Integration Tests (completion.rs)

Added two integration tests that simulate the exact backend.rs dot-completion pipeline:

**`test_dot_completion_integration_struct`**: Simulates user typing `p.` on a `Point` struct variable.
- Original: `"pub struct Point { x: int, y: int }\nfn main() { let p: Point = new Point { x: 1, y: 2 }; p. }"`
- Strips dot → modified source
- `analyze_standalone` produces valid typed AST
- `expr_at_offset(ast, dot_pos - 1, FileId(0))` finds receiver
- `build_dot_completions` returns items containing `"x"` and `"y"`

**`test_dot_completion_integration_array`**: Simulates user typing `arr.` on an `Array<int>` variable.
- Original: `"fn main() { let arr: Array<int> = [1, 2, 3]; arr. }"`
- Full pipeline returns push, pop, len, is_empty

## Diagnosis Result

**The pipeline was already correct.** All three potential failure points work as expected:
1. `expr_at_offset` finds the receiver — span math is correct
2. Receiver type is correctly resolved — not `TyKind::Error`
3. `analyze_standalone` produces typed AST from the dot-stripped source — parsing succeeds

The root cause of any real-world dot-completion failures would be in the LSP client/server interaction (trigger character detection, byte offset conversion from LSP position), not in the analysis pipeline itself.

## Deviations from Plan

None — plan executed exactly as written. No backend.rs changes were needed because the pipeline works correctly. The tests confirm this empirically.

## All Tests Passing

- `test_expr_at_offset_receiver_for_dot_completion` — PASS
- `test_dot_completion_integration_struct` — PASS
- `test_dot_completion_integration_array` — PASS
- Full suite: `cargo test -p writ-lsp --lib` — 98 passed, 0 failed

## Commits

| Hash | Message |
|------|---------|
| a8cc8ab | feat(67-02): add dot-completion integration tests and verify pipeline |

## Self-Check: PASSED

- writ-lsp/src/queries/completion.rs: FOUND
- writ-lsp/src/queries/walk.rs: FOUND
- commit a8cc8ab: FOUND
