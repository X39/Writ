---
phase: 02-foundational-expression-lowering
plan: 02
subsystem: testing
tags: [insta, snapshots, rust, lowering, R3, R4, R5]

# Dependency graph
requires:
  - phase: 02-foundational-expression-lowering/02-01
    provides: lower_type, lower_fmt_string, lower_expr, lower_stmt, lower() pipeline

provides:
  - 14 insta snapshot tests covering all three Phase 2 requirements (R3, R4, R5)
  - Accepted snapshot files in writ-compiler/tests/snapshots/ (14 .snap files)
  - Quality gate: regression guards for T? desugaring, $"..." lowering, compound assignment expansion

affects:
  - 03-dialogue-block-parser
  - any phase that modifies lower_type, lower_expr, lower_fmt_string, lower_stmt

# Tech tracking
tech-stack:
  added: [cargo-insta (CI tooling), insta 1.46.3 already in dev-dependencies]
  patterns:
    - assert_debug_snapshot! for AST snapshot testing (SimpleSpan lacks Serialize without chumsky serde feature)
    - lower_src(&'static str) -> Ast test helper for end-to-end parse+lower in integration tests
    - let mut local variables for R5 compound assign tests (parser does not support mut in param position)

key-files:
  created:
    - writ-compiler/tests/lowering_tests.rs
    - writ-compiler/tests/snapshots/ (14 .snap files)
  modified: []

key-decisions:
  - "assert_debug_snapshot! over assert_ron_snapshot! — SimpleSpan does not implement Serialize without enabling chumsky serde feature; adding serde to AST types is overengineering for snapshot tests"
  - "lower_src takes &'static str — writ_parser::parse returns Rich<'static, Token<'src>, Span> errors; the Token<'src> borrows the source so the source must have 'static lifetime in tests"
  - "let mut local variables for R5 tests — Writ parser does not support mut in function parameter position (fn_param grammar is name: type only)"

patterns-established:
  - "Integration test pattern: parse source string with writ_parser::parse, lower with writ_compiler::lower, snapshot the Ast via assert_debug_snapshot!"
  - "Static source strings in tests: &'static str required due to CST borrow of source text propagating into error type lifetimes"

requirements-completed: [R3, R4, R5]

# Metrics
duration: 15min
completed: 2026-02-26
---

# Phase 02 Plan 02: Snapshot Test Suite for R3, R4, R5 Lowering Summary

**14 insta snapshot tests covering optional sugar (T?/null), formattable string ($"..."), and compound assignment (+=/-=/*=//=/%=) lowering with all snapshots accepted and cargo test passing 14/14**

## Performance

- **Duration:** 15 min
- **Started:** 2026-02-26T16:00:00Z
- **Completed:** 2026-02-26T16:15:00Z
- **Tasks:** 2
- **Files modified:** 15 (1 test file + 14 snapshot files)

## Accomplishments
- Created `writ-compiler/tests/lowering_tests.rs` with 14 snapshot tests covering all Phase 2 requirements
- R3 (4 tests): confirmed T? → `Generic { name: "Option", args: [T] }` and `null` → `Path { segments: ["Option", "None"] }`
- R4 (4 tests): confirmed `$"Hello {name}!"` → left-associative Binary Add chain with GenericCall `.into<string>()` wrappers
- R5 (6 tests): confirmed all five compound operators expand to `Assign { target, value: Binary { op } }` and plain `=` passes through without Binary wrapper
- All 14 snapshot files accepted; `cargo insta pending-snapshots` reports "No pending snapshots"

## Task Commits

Each task was committed atomically:

1. **Task 1: Decide serialization approach (no-op: use assert_debug_snapshot!)** — no commit needed (decision only)
2. **Task 2: R3 snapshot tests** - `d910fbe` (test)
3. **Task 2: R4/R5 snapshot tests + accepted snapshots** - `161dd73` (test)

## Files Created/Modified
- `writ-compiler/tests/lowering_tests.rs` — 14 integration snapshot tests: lower_src helper, R3/R4/R5 test functions
- `writ-compiler/tests/snapshots/lowering_tests__optional_param_type.snap` — T? in param → Option<T>
- `writ-compiler/tests/snapshots/lowering_tests__optional_return_type.snap` — T? in return + null → Option::None
- `writ-compiler/tests/snapshots/lowering_tests__null_literal_to_option_none.snap` — null → Path["Option","None"]
- `writ-compiler/tests/snapshots/lowering_tests__nested_optional_type.snap` — List<string?>? → nested Option
- `writ-compiler/tests/snapshots/lowering_tests__fmt_string_simple_interpolation.snap` — $"Hello {name}!" → Binary Add chain
- `writ-compiler/tests/snapshots/lowering_tests__fmt_string_no_interpolation.snap` — $"plain" → single StringLit
- `writ-compiler/tests/snapshots/lowering_tests__fmt_string_multiple_segments.snap` — multi-interpolation chain
- `writ-compiler/tests/snapshots/lowering_tests__fmt_string_escaped_braces.snap` — {{ }} handling documented
- `writ-compiler/tests/snapshots/lowering_tests__compound_add_assign.snap` — x += 1 expansion
- `writ-compiler/tests/snapshots/lowering_tests__compound_sub_assign.snap` — x -= 2 expansion
- `writ-compiler/tests/snapshots/lowering_tests__compound_mul_assign.snap` — x *= 3 expansion
- `writ-compiler/tests/snapshots/lowering_tests__compound_div_assign.snap` — x /= 4 expansion
- `writ-compiler/tests/snapshots/lowering_tests__compound_mod_assign.snap` — x %= 5 expansion
- `writ-compiler/tests/snapshots/lowering_tests__plain_assign_passthrough.snap` — x = 0 (no Binary wrapper)

## Decisions Made
- Used `assert_debug_snapshot!` instead of `assert_ron_snapshot!` — SimpleSpan (from chumsky) does not implement Serialize unless chumsky's `serde` feature is enabled. Adding that feature and serde derives to all AST types is overengineering for snapshot tests when `Debug` is already derived.
- `lower_src` takes `&'static str` — the CST borrows `&'src str` from the input; `writ_parser::parse` returns `Rich<'static, Token<'src>, Span>` errors which force the source to outlive the function call. Using `&'static str` (string literals) satisfies the lifetime constraint cleanly.
- R5 tests use `let mut` local variables — the Writ parser's `fn_param` grammar is `name: type` only; `mut` in parameter position is not supported. Using `let mut x: int = 0; x += 1;` in the body achieves the same test coverage.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed lifetime error in lower_src test helper**
- **Found during:** Task 2 (initial test compilation)
- **Issue:** `lower_src(src: &str)` — the CST borrows `&'src str` from the source, so `src` must outlive the `items` returned by `parse()`. Rust E0521/E0597 rejected the borrow escaping the function.
- **Fix:** Changed signature to `lower_src(src: &'static str)`. All test call sites use string literals which are `'static`, so no test changes were needed. Errors are eagerly converted to owned strings via `format!("{e:?}")` before the check.
- **Files modified:** writ-compiler/tests/lowering_tests.rs
- **Verification:** `cargo test -p writ-compiler` compiles and all 14 tests pass
- **Committed in:** d910fbe (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - lifetime bug in test helper)
**Impact on plan:** Required for compilation. No scope change. Plan's intent fully achieved.

## Issues Encountered
- `cargo insta accept` requires `cargo-insta` CLI tool — not pre-installed. Installed with `cargo install cargo-insta`. All 14 snapshots accepted successfully.

## Next Phase Readiness
- Phase 2 quality gate satisfied: all R3, R4, R5 lowering rules have accepted snapshot tests
- Phase 3 (dialogue block parser) can proceed; the lower() pipeline is tested end-to-end
- Snapshots serve as regression guards — any future change to optional/fmt_string/compound-assign lowering will fail these tests if behavior changes

## Self-Check: PASSED

- writ-compiler/tests/lowering_tests.rs: FOUND
- 14 snapshot files in writ-compiler/tests/snapshots/: FOUND
- Commit d910fbe: FOUND
- Commit 161dd73: FOUND

---
*Phase: 02-foundational-expression-lowering*
*Completed: 2026-02-26*
