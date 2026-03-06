---
phase: 43-unqualified-none-some
plan: 03
subsystem: typecheck
tags: [rust, typecheck, resolver, parser, option, none, some, writ-compiler, writ-parser]

requires:
  - phase: 43-unqualified-none-some
    plan: 01
    provides: eight RED test stubs defining the LANG-02 contract
  - phase: 43-unqualified-none-some
    plan: 02
    provides: resolver sub-prelude injection, using-glob expansion, parser ::* extension

provides:
  - "check_ident sub-prelude injection: None/Some return Option<InferVar> after all user lookups fail"
  - "check_call Some(expr) builtin: produces Option<T> from argument type"
  - "check_pattern Option arm handling: None/Some in EnumDestructure and Variable patterns on Option scrutinee"
  - "check_stmt bare-None detection: NoneWithoutAnnotation (E0120) when None has no constraining annotation"
  - "Parser single-segment enum destructure: Ident(patterns) accepted in pattern position"
  - "Spec §23.4.4: glob enum imports, sub-prelude builtins documented"

affects: []

tech-stack:
  added: []
  patterns:
    - "Sub-prelude check in check_ident: match name after all DefMap lookups fail -- avoids going through ScopeChain"
    - "Some() call special-cased in check_call before General case: constructs Option<T> from arg type"
    - "check_pattern Option arm: inspect TyKind::Option(inner) before TyKind::Enum check -- no fake DefId needed"
    - "Single-segment enum destructure: enum_destruct_single parser rule with mandatory parens separates from variable"

key-files:
  created: []
  modified:
    - writ-compiler/src/check/check_expr.rs
    - writ-compiler/src/check/check_stmt.rs
    - writ-compiler/src/check/error.rs
    - writ-diagnostics/src/code.rs
    - writ-compiler/tests/typecheck_tests.rs
    - writ-parser/src/parser.rs
    - language-spec/spec/24_23_modules_namespaces.md

key-decisions:
  - "NoneWithoutAnnotation is E0120 (next free code after E0119)"
  - "Some(expr) call handled in check_call Ident fast-path -- fired before General callee check; user-defined Some() shadows via find_fn_def_id early return"
  - "None/Some Variable patterns on Option scrutinee return TypedPattern::Wildcard -- no local binding created, suppresses false errors"
  - "EnumDestructure single-segment Some(v) returns TypedPattern::Wildcard with inner check_pattern call to bind v -- emitter falls back to literal_match for explicit Option matches"
  - "enum_destruct_single parser rule: mandatory LParen prevents ambiguity with plain variable binding"

patterns-established:
  - "Sub-prelude name injection in check layer: match-by-name after all user lookups, before UndefinedVariable error"
  - "Single-segment enum destructure in parser: enum_destruct_single rule placed before variable rule in single choice"

requirements-completed: [LANG-02]

duration: 12min
completed: 2026-03-06
---

# Phase 43 Plan 03: Type-Checker Sub-Prelude, Pattern Position, Parser, and Spec Summary

**check_ident/check_call None/Some sub-prelude injection plus single-segment Some(v) parser rule and §23.4.4 spec subsection -- all 8 LANG-02 stubs turn GREEN.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-03-06T16:47:00Z
- **Completed:** 2026-03-06T17:01:53Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Five typecheck stubs green: none_unqualified_with_annotation, some_unqualified_infers_type, bare_none_no_annotation_error, user_none_shadows_builtin, none_some_in_pattern_position
- NoneWithoutAnnotation (E0120) error variant and diagnostic added
- Sub-prelude injection in check_ident: None/Some return Option<InferVar> after all user lookups fail
- Some(expr) call special-cased in check_call: produces Option<arg_ty> without going through NotCallable path
- check_pattern handles None/Some on Option scrutinee -- both Variable and EnumDestructure arms
- check_stmt detects bare None without annotation and emits E0120
- Parser extended with enum_destruct_single rule: Ident(patterns) accepted in pattern position
- Spec §23.4.4 added: glob enum import syntax, conflict rules, sub-prelude builtins documented
- No regressions: 9 golden, 70 typecheck, 239 parser, 36 resolve tests all green

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend check_ident, check_pattern, check_call, check_stmt and add NoneWithoutAnnotation error** - `6434fd9` (feat)
2. **Task 2: Add §23.4.4 spec subsection for glob enum imports and sub-prelude builtins** - `65806a5` (feat)

## Files Created/Modified

- `writ-compiler/src/check/check_expr.rs` - Sub-prelude injection in check_ident; Some() call fast-path in check_call; None/Some handling in check_pattern (Variable + EnumDestructure arms)
- `writ-compiler/src/check/check_stmt.rs` - Bare None detection in let-binding arm
- `writ-compiler/src/check/error.rs` - NoneWithoutAnnotation variant + Diagnostic conversion
- `writ-diagnostics/src/code.rs` - E0120 constant added
- `writ-compiler/tests/typecheck_tests.rs` - none_some_in_pattern_position assertion added (has_no_errors)
- `writ-parser/src/parser.rs` - enum_destruct_single rule for Ident(patterns) in pattern position
- `language-spec/spec/24_23_modules_namespaces.md` - §23.4.4 added

## Decisions Made

- **E0120 for NoneWithoutAnnotation**: Next free code after E0119 (closure capture error).
- **Some() call in check_call Ident fast-path**: The user-defined `Some` function case is already handled above by `find_fn_def_id` + early return; the sub-prelude case fires only when that lookup fails.
- **Wildcard for None/Some patterns**: The explicit `match x { None => {}, Some(v) => {} }` case falls through to `emit_literal_match` (not `is_option_propagation`) because neither arm has a `Return` body. Emitting `Wildcard` is correct for the typecheck layer; the semantic exactness for IL is deferred to the desugar layer (`?`/`!` operators).
- **enum_destruct_single parser rule**: Mandatory `LParen` distinguishes `Some(v)` from plain variable `Some`. Placed before `variable` in the `single` choice so `Some(v)` is parsed as destructure.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] check_call Some(expr) special-case added**
- **Found during:** Task 1 (running some_unqualified_infers_type)
- **Issue:** `Some(true)` reached the General case in `check_call` where `callee_ty = Option<InferVar>` is not `Func`, triggering NotCallable error. The plan's text only mentioned check_ident injection without mentioning the call site.
- **Fix:** Added a `name == "Some"` branch in check_call's Ident fast-path that checks args length and returns `TypedExpr::Call { ty: Option<arg_ty> }`.
- **Files modified:** `writ-compiler/src/check/check_expr.rs`
- **Verification:** `some_unqualified_infers_type` passes
- **Committed in:** `6434fd9` (Task 1 commit)

**2. [Rule 3 - Blocking] Parser single-segment enum destructure added in Task 1**
- **Found during:** Task 1 (none_some_in_pattern_position parse failure)
- **Issue:** `Some(v)` in pattern position panicked at parse stage -- parser requires `at_least(2)` segments for EnumDestructure, so `Some(v)` was parsed as `Variable("Some")` followed by unexpected `(`.
- **Fix:** Added `enum_destruct_single` rule: `Ident (patterns)` with mandatory parens, placed before `variable` in the `single` choice. Task 2's plan description of a parser fix was pre-empted here because it was needed to get Task 1's test green.
- **Files modified:** `writ-parser/src/parser.rs`
- **Verification:** `none_some_in_pattern_position` no longer panics; 239 parser tests still green
- **Committed in:** `6434fd9` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Both fixes necessary for the tests to compile. No scope creep -- check_expr.rs pattern changes only affect Option-scrutinee match arms.

## Issues Encountered

None.

## Next Phase Readiness

- All 8 LANG-02 test stubs are GREEN (5 typecheck + 3 resolve)
- LANG-02 requirement fully satisfied: unqualified None/Some compile in expression and pattern position, bare None errors gracefully, using Enum::* parses and resolves, spec documented
- No pending blockers for Phase 43

## Self-Check: PASSED

- `writ-compiler/src/check/check_expr.rs`: FOUND (Sub-prelude injection present)
- `writ-compiler/src/check/error.rs`: FOUND (NoneWithoutAnnotation present)
- `writ-diagnostics/src/code.rs`: FOUND (E0120 present)
- `language-spec/spec/24_23_modules_namespaces.md`: FOUND (§23.4.4 present)
- Commit `6434fd9`: FOUND (Task 1)
- Commit `65806a5`: FOUND (Task 2)

---
*Phase: 43-unqualified-none-some*
*Completed: 2026-03-06*
