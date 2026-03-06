---
phase: 41-fix-fn-log-say-choice
plan: "02"
subsystem: writ-compiler/check, writ-golden, language-spec
tags: [bug-fix, check_path, root-qualified-paths, BOM, spec-note]
dependency_graph:
  requires: [41-01]
  provides: [check_path normalization for ::-prefixed paths, BOM-free .writ source]
  affects: [writ-compiler/check/check_expr.rs, writ-golden/tests/golden/fn_log_say_choice.writ, language-spec/spec/27_26_standard_library_builtins.md]
tech_stack:
  added: []
  patterns: [TDD red-green-refactor, check_path normalization via strip_prefix]
key_files:
  created: []
  modified:
    - writ-compiler/src/check/check_expr.rs
    - writ-compiler/tests/typecheck_tests.rs
    - writ-golden/tests/golden/fn_log_say_choice.writ
    - language-spec/spec/27_26_standard_library_builtins.md
decisions:
  - "check_path normalization strips '::' from first segment only — matches lower/expr.rs encoding where only segs[0] gets the :: prefix"
  - "callee_def_id is None for path-form calls even after fix — set only in check_call Ident fast-path; path-form calls use type-based dispatch which doesn't need it"
  - "TDD tests verify call type is non-error, not callee_def_id — callee_def_id=None is a pre-existing behavior for path calls, not a bug introduced by this fix"
  - "emit panic from Option::None enum variant resolution is pre-existing stub behavior — out of scope for this plan"
metrics:
  duration: "~7 minutes"
  completed_date: "2026-03-06"
  tasks_completed: 2
  tasks_total: 2
  files_changed: 4
---

# Phase 41 Plan 02: Fix check_path :: Normalization Summary

**One-liner:** Normalized root-qualified path segments (::log → log) in check_path before DefMap lookup by stripping the leading :: prefix from the first segment.

## What Was Built

### Task 1: Fix check_path to normalize root-qualified path segments (TDD)

**Problem:** `lower/expr.rs` encodes `::log` (root-qualified path) as `AstExpr::Path { segments: ["::log"] }` — prepending `::` to the first segment. But `check_path` joined segments naively: `["::log"].join("::")` = `"::log"`, and `def_map.get("::log")` returned `None` (items are registered as `"log"`, not `"::log"`). The function fell through to the stub path returning `ty: error()` with no diagnostic. The call then got an error-typed callee, codegen silently produced zero instructions.

**Fix:** Added a normalization block before the DefMap lookup that strips the leading `::` from the first segment only:

```rust
let normalized_segments: Vec<String> = {
    let mut segs = segments.to_vec();
    if let Some(first) = segs.first_mut() {
        if let Some(stripped) = first.strip_prefix("::") {
            *first = stripped.to_string();
        }
    }
    segs
};
let fqn = normalized_segments.join("::");
```

**TDD:** RED tests verified the call produces error type before fix. GREEN tests verify the call type matches the extern fn's declared return type after fix.

### Task 2: Strip UTF-8 BOM and add spec note

**BOM fix:** `fn_log_say_choice.writ` had a UTF-8 BOM (EF BB BF) that caused a parse error: `found 'Error' at 0..3 expected declaration`. Rewrote as plain UTF-8 using the Write tool.

**Spec note:** Added to `language-spec/spec/27_26_standard_library_builtins.md §26.4`:

> The root-qualified forms `::log`, `::say`, and `::choice` (with a leading `::`) are also valid — `::` means "resolve from the root namespace" (see §23.9). They are equivalent to the unqualified names and produce identical IL. Both forms are accepted from any `fn` or `dlg` context.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed as written.

### Out-of-Scope Discoveries (Deferred)

**1. [Pre-existing stub] Option::None enum variant path resolution**

- **Found during:** Task 2 verification (golden test run)
- **Issue:** The `.writ` test file uses `Option::None` as an argument to `::say(Option::None, "...")`. The `check_path` stub for multi-segment paths (enum variants like `Direction::North`) returns `ty: error()` silently. After the BOM fix, parsing succeeds, but this error type propagates into lambda captures, causing an emit panic: `Error type should not appear in emit output` in `writ-compiler/src/emit/type_sig.rs:90`.
- **Impact:** `cargo test -p writ-golden -- test_fn_log_say_choice` fails with an emit panic instead of the plan-expected "golden mismatch". The difference: parse succeeds (BOM fixed), check_path resolves `::log`/`::say`/`::choice` correctly, but `Option::None` still hits the unimplemented enum variant stub.
- **Status:** Pre-existing limitation, out of scope. Deferred to Phase 43 (None/Some) which resolves enum variant paths.
- **Files:** `writ-compiler/src/check/check_expr.rs:484` (the `// Could be an enum variant path -- Stub for now` comment)

## Commits

| Hash | Description |
|------|-------------|
| ef2c054 | test(41-02): add failing tests for check_path :: normalization (RED) |
| 205cddf | feat(41-02): fix check_path to normalize root-qualified path segments |
| 1b5c95d | feat(41-02): strip UTF-8 BOM from fn_log_say_choice.writ and add spec note |

## Verification Results

| Check | Result | Notes |
|-------|--------|-------|
| `cargo test -p writ-compiler` | PASS (65 tests) | All 4 new root-qualified tests pass |
| `cargo test -p writ-golden -- test_fn_log_say_choice` | FAIL (emit panic, not parse error) | BOM fixed, ::log resolves; fails at Option::None enum variant stub |
| `cargo test -p writ-golden` | 8/9 pass | No regressions in other golden tests |
| fn_log_say_choice.writ first byte | 0x70 ('p') | BOM (0xEF) confirmed absent |
| Spec §26.4 ::log note | Present | "root-qualified forms ::log, ::say, ::choice are valid" |

## Success Criteria Assessment

| Criterion | Met |
|-----------|-----|
| check_path normalizes ::log → log before DefMap lookup | YES |
| fn_log_say_choice golden test runs to comparison stage | PARTIAL — reaches emit stage (past parse/typecheck), but panics in emit due to Option::None stub |
| Spec §26.4 documents ::log/::say/::choice as valid root-qualified forms | YES |
| fn_log_say_choice.writ is BOM-free | YES |

## Self-Check: PASSED

All created/modified files exist. All 3 task commits (ef2c054, 205cddf, 1b5c95d) confirmed in git log.
