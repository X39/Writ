---
phase: 20-text-assembler
plan: 01
subsystem: assembler
tags: [lexer, parser, ast, writil, text-il]

# Dependency graph
requires:
  - phase: 16-module-format-foundation
    provides: ModuleBuilder API, Module struct, Instruction enum
provides:
  - writ-assembler crate with lexer, AST, and recursive-descent parser
  - tokenize() function for .writil text format
  - parse() function producing structured AsmModule AST
  - Multi-error collection with line:column diagnostics
affects: [20-02-PLAN, 21-disassembler-and-runner-cli]

# Tech tracking
tech-stack:
  added: [thiserror 2.0]
  patterns: [recursive-descent parser, multi-error collection, line-oriented lexer]

key-files:
  created:
    - writ-assembler/Cargo.toml
    - writ-assembler/src/lib.rs
    - writ-assembler/src/error.rs
    - writ-assembler/src/lexer.rs
    - writ-assembler/src/ast.rs
    - writ-assembler/src/parser.rs
    - writ-assembler/tests/parse_tests.rs
  modified:
    - Cargo.toml

key-decisions:
  - "Used peek_kind() -> &TokenKind helper to avoid Rust 2024 edition borrow issues with ref patterns"
  - "Directives recognized via known-directive list in lexer; unknown dot-prefixed tokens become LabelRef"
  - "Instruction mnemonics tokenized as Ident and matched case-insensitively in parser"

patterns-established:
  - "Pattern: peek_kind()/advance() parser style for Rust 2024 edition compatibility"
  - "Pattern: synchronize() error recovery — skip to }, directive, or EOF on parse error"
  - "Pattern: clone directive name before match block to avoid borrow conflict with &mut self"

requirements-completed: [ASM-01]

# Metrics
duration: ~20min
completed: 2026-03-02
---

# Phase 20 Plan 01: Lexer, AST, and Parser Summary

**Recursive-descent parser for .writil text IL with 22 token types, full AST node coverage, and multi-error collection with line:column diagnostics**

## Performance

- **Duration:** ~20 min
- **Tasks:** 2
- **Files created:** 7
- **Files modified:** 1

## Accomplishments
- Created writ-assembler crate with complete lexer supporting 22 token types (directives, registers, labels, strings, ints, floats, symbols)
- Implemented recursive-descent parser handling all directive types: .module, .type, .field, .contract, .impl, .method, .reg, .extern, .global
- Multi-error collection with synchronization: parser recovers from errors and continues collecting additional diagnostics
- Full AST node types covering the entire .writil grammar including type references, method references, field references, and generic types

## Task Commits

Each task was committed atomically:

1. **Task 1: Create writ-assembler crate with lexer, AST, and parser** - `105f0de` (feat)
2. **Task 2: Add comprehensive parser tests** - `8131ea6` (test)

## Files Created/Modified
- `Cargo.toml` - Added writ-assembler to workspace members
- `writ-assembler/Cargo.toml` - Crate manifest with writ-module and thiserror dependencies
- `writ-assembler/src/lib.rs` - Public API: assemble(src) -> Result<Module, Vec<AssembleError>>
- `writ-assembler/src/error.rs` - AssembleError with line:col and descriptive message
- `writ-assembler/src/lexer.rs` - Token enum and tokenize() function with 10 unit tests
- `writ-assembler/src/ast.rs` - AST node types for all directives and instructions
- `writ-assembler/src/parser.rs` - Recursive-descent parser with multi-error collection
- `writ-assembler/tests/parse_tests.rs` - 24 integration tests covering all directive types

## Decisions Made
- Used `peek_kind() -> &TokenKind` helper pattern to work with Rust 2024 edition's stricter borrow rules (no `ref` in implicitly-borrowing patterns)
- Lexer recognizes known directives (.module, .type, .field, etc.) explicitly; unknown `.xyz` tokens treated as LabelRef
- Instruction mnemonics are tokenized as plain Ident tokens; case-insensitive matching handled in parser

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Rust 2024 edition borrow checker compatibility**
- **Found during:** Task 1 (parser implementation)
- **Issue:** Rust 2024 edition disallows `ref` in implicitly-borrowing patterns, causing 3 compile errors. Removing `ref` caused 11 move-out-of-shared-reference errors.
- **Fix:** Introduced `peek_kind() -> &TokenKind` method and restructured all pattern matches to borrow through it. Clone directive names before match blocks to avoid &mut self borrow conflicts.
- **Files modified:** writ-assembler/src/parser.rs
- **Verification:** `cargo build -p writ-assembler` succeeds
- **Committed in:** 105f0de (Task 1 commit)

**2. [Rule 3 - Blocking] Multi-error test using unknown directives**
- **Found during:** Task 2 (parse tests)
- **Issue:** Test using `.unknown_directive` didn't produce parser errors because lexer tokenizes unknown dot-prefixed tokens as LabelRef, not Directive
- **Fix:** Changed test to use `BADTOKEN1`/`BADTOKEN2` inside .type blocks, which correctly triggers multiple parser errors via synchronization
- **Files modified:** writ-assembler/tests/parse_tests.rs
- **Verification:** All 24 tests pass
- **Committed in:** 8131ea6 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both fixes necessary for correctness. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Lexer and parser pipeline complete, ready for Plan 02 (two-pass assembler)
- AST types are the input contract for the assembler
- assemble() function stub wired up, ready to be completed

---
*Phase: 20-text-assembler, Plan: 01*
*Completed: 2026-03-02*
