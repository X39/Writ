---
phase: 10-parser-core-syntax
plan: 02
subsystem: parser, compiler
tags: [tests, parser-tests, lowering-tests, snapshots]

requires:
  - phase: 10-parser-core-syntax
    plan: 01
    provides: "All CST/AST type definitions, parser combinators, and lowering for 6 features"
provides:
  - "23 parser tests covering all 6 Phase 10 requirements"
  - "10 lowering snapshot tests covering all 6 Phase 10 requirements"
affects: []

tech-stack:
  added: []
  patterns: []

key-files:
  created:
    - writ-compiler/tests/snapshots/lowering_tests__lower_new_construction_basic.snap
    - writ-compiler/tests/snapshots/lowering_tests__lower_new_construction_empty.snap
    - writ-compiler/tests/snapshots/lowering_tests__lower_new_construction_generic.snap
    - writ-compiler/tests/snapshots/lowering_tests__lower_hex_binary_literals.snap
    - writ-compiler/tests/snapshots/lowering_tests__lower_struct_lifecycle_hook.snap
    - writ-compiler/tests/snapshots/lowering_tests__lower_struct_multiple_hooks.snap
    - writ-compiler/tests/snapshots/lowering_tests__lower_self_param.snap
    - writ-compiler/tests/snapshots/lowering_tests__lower_mut_self_with_regular_param.snap
    - writ-compiler/tests/snapshots/lowering_tests__lower_shift_operators.snap
    - writ-compiler/tests/snapshots/lowering_tests__lower_bitand_bitor_operators.snap
  modified:
    - writ-parser/tests/parser_tests.rs
    - writ-compiler/tests/lowering_tests.rs

key-decisions:
  - "Parser tests use direct CST node assertions (not snapshots) for fine-grained control"
  - "Lowering tests use insta snapshots consistent with existing lowering test patterns"
  - "Existing test regressions from Plan 01 were already fixed during Plan 01 execution"

patterns-established: []

requirements-completed: [PARSE-01, PARSE-02, DECL-01, DECL-02, EXPR-01, EXPR-02]

duration: ~5min
completed: 2026-03-01
---

# Plan 10-02: Comprehensive Tests Summary

**Added 33 tests (23 parser + 10 lowering) covering all 6 Phase 10 syntax features with zero regressions**

## Performance

- **Duration:** ~5 min
- **Tasks:** 2 (Task 2 was a no-op since regressions were already fixed in Plan 01)
- **Files modified:** 2 (+ 10 snapshot files created)

## Accomplishments

### Parser Tests Added (23 tests in parser_tests.rs)

**PARSE-01: new keyword construction (6 tests)**
- `new_construction_basic` - basic `new Point { x: 1, y: 2 }`
- `new_construction_empty` - empty fields `new Point {}`
- `new_construction_trailing_comma` - trailing comma support
- `new_construction_generic_type` - generic type `new List<int> {}`
- `new_construction_rooted_path` - rooted path `new ::module::Type { x: 1 }`
- `old_construction_syntax_rejected` - bare `Point { x: 1 }` produces errors

**PARSE-02: hex/binary literals (4 tests)**
- `hex_literal_parses_as_int` - `0xFF` -> IntLit
- `binary_literal_parses_as_int` - `0b1010` -> IntLit
- `hex_literal_uppercase` - `0XFF` -> IntLit
- `binary_literal_uppercase` - `0B1010` -> IntLit

**DECL-01: struct lifecycle hooks (4 tests)**
- `struct_with_on_create_hook` - single hook
- `struct_interleaved_fields_and_hooks` - fields and hooks mixed
- `struct_all_four_lifecycle_hooks` - create, finalize, serialize, deserialize
- `struct_hook_with_body` - hook with body statements

**DECL-02: self/mut self parameters (3 tests)**
- `fn_with_self_param` - immutable self
- `fn_with_mut_self_param` - mutable self
- `fn_self_with_regular_params` - self + regular params

**EXPR-01: shift operators (4 tests)**
- `shift_left_operator` - `a << b` -> Binary(Shl)
- `shift_right_operator` - `a >> b` -> Binary(Shr)
- `shift_precedence_below_additive` - `a << b + c` -> `a << (b + c)`
- `shift_precedence_above_comparison` - `a << b < c` -> `(a << b) < c`

**EXPR-02: BitAnd/BitOr in OpSymbol (2 tests)**
- `operator_bitand_in_impl` - `operator &` in impl block
- `operator_bitor_in_impl` - `operator |` in impl block

### Lowering Tests Added (10 snapshot tests in lowering_tests.rs)

- `lower_new_construction_basic` - new expr with fields
- `lower_new_construction_empty` - new expr empty
- `lower_new_construction_generic` - new expr with generic type
- `lower_hex_binary_literals` - hex/bin literals through lowering
- `lower_struct_lifecycle_hook` - struct with field + on hook
- `lower_struct_multiple_hooks` - struct with multiple hooks
- `lower_self_param` - fn with self param
- `lower_mut_self_with_regular_param` - fn with mut self + regular param
- `lower_shift_operators` - << and >> binary ops
- `lower_bitand_bitor_operators` - operator & and operator | in impl

## Test Results
- All 380 tests pass: 79 lowering + 13 unit + 74 lexer + 212 parser + 2 doctest
- Zero regressions from existing tests

## Deviations from Plan
- Task 2 (fix regressions) was already completed during Plan 01 execution, so it was a no-op here.

## Issues Encountered
- Minor: format string `{ mutable: true }` in panic message conflicted with Rust's `{}` formatting syntax. Fixed by using parentheses instead of braces.

## User Setup Required
None.

---
*Phase: 10-parser-core-syntax*
*Completed: 2026-03-01*
