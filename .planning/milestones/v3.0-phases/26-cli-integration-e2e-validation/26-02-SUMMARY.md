---
phase: 26-cli-integration-e2e-validation
plan: 02
subsystem: cli
tags: [writ-cli, writ-compiler, writ-diagnostics, writ-parser, ariadne, pipeline]

# Dependency graph
requires:
  - phase: 25-il-codegen-method-bodies
    provides: emit_bodies() function producing binary .writil output
  - phase: 23-type-checking
    provides: typecheck() producing TypedAst
  - phase: 22-name-resolution
    provides: resolve() producing NameResolvedAst
provides:
  - "writ compile <input.writ> [--output <out.writil>] CLI subcommand"
  - "LoweringError::to_diagnostic() covering all 9 variants with codes L0001-L0099"
  - "Full 5-stage pipeline: parse -> lower -> resolve -> typecheck -> emit_bodies"
  - "Error-stop discipline with ariadne rendering at each stage"
affects:
  - "26-cli-integration-e2e-validation (e2e tests)"

# Tech tracking
tech-stack:
  added:
    - writ-compiler = { path = "../writ-compiler" } in writ-cli
    - writ-diagnostics = { path = "../writ-diagnostics" } in writ-cli
    - writ-parser = { path = "../writ-parser" } in writ-cli
  patterns:
    - "Box::leak src string for 'static lifetime required by writ_parser::parse return type"
    - "Error-stop pipeline: each stage returns early if errors exist before proceeding"
    - "Parse errors format via {:?} since Token lacks Display"
    - "LoweringError::to_diagnostic uses DiagnosticBuilder fluent API: with_primary/with_secondary/with_help"

key-files:
  created: []
  modified:
    - writ-compiler/src/lower/error.rs
    - writ-cli/Cargo.toml
    - writ-cli/src/main.rs

key-decisions:
  - "Box::leak used for src string to satisfy 'static lifetime bound from writ_parser::parse (Rich<'static, Token<'src>, Span> requires 'src = 'static in practice)"
  - "Parse errors use {:?} debug format since writ_parser::Token lacks Display impl"
  - "Real DiagnosticBuilder API uses with_primary/with_secondary/with_help (not .file/.span/.label as in plan pseudo-code)"

patterns-established:
  - "cmd_compile() follows cmd_assemble() pattern: read -> process -> write -> eprintln success"
  - "LoweringError::to_diagnostic(file_id) converts all 9 variants to well-formed Diagnostics"

requirements-completed: [CLI-01, CLI-02, CLI-03]

# Metrics
duration: 10min
completed: 2026-03-03
---

# Phase 26 Plan 02: CLI Compile Subcommand Summary

**`writ compile foo.writ` wires all 5 pipeline stages into a user-facing command, with ariadne error rendering at every stage and LoweringError::to_diagnostic covering all 9 variants**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-03T14:01:49Z
- **Completed:** 2026-03-03T14:11:49Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Implemented `LoweringError::to_diagnostic(&self, file_id: FileId) -> Diagnostic` for all 9 error variants (L0001-L0099) using the real DiagnosticBuilder API (`with_primary`, `with_secondary`, `with_help`)
- Added `writ-compiler`, `writ-diagnostics`, `writ-parser` dependencies to `writ-cli/Cargo.toml`
- Added `Commands::Compile` variant and `cmd_compile()` function chaining parse -> lower -> resolve -> typecheck -> emit_bodies with error-stop discipline
- All 334 existing writ-compiler tests continue to pass

## Task Commits

Each task was committed atomically:

1. **Task 1: LoweringError to Diagnostic conversion** - `8fe48e7` (feat)
2. **Task 2: `writ compile` subcommand and pipeline wiring** - `f979237` (feat)

**Plan metadata:** (to be committed as docs metadata)

## Files Created/Modified
- `writ-compiler/src/lower/error.rs` - Added `use writ_diagnostics::{Diagnostic, FileId}` import and `to_diagnostic` impl block with all 9 variants
- `writ-cli/Cargo.toml` - Added writ-compiler, writ-diagnostics, writ-parser dependencies
- `writ-cli/src/main.rs` - Added `Commands::Compile`, `cmd_compile()` function, updated doc comment

## Decisions Made
- **Box::leak for parse source:** `writ_parser::parse` returns `Vec<Rich<'static, Token<'src>, Span>>` which requires `'src = 'static` in practice. Used `Box::leak(src_owned.into_boxed_str())` to promote the source string to `'static`. This is acceptable for a CLI process that reads one file per invocation.
- **Parse error format:** Used `{:?}` debug format for parse errors since `writ_parser::Token` doesn't implement `Display`. This gives verbose but correct output showing token names and spans.
- **Real DiagnosticBuilder API:** The plan's pseudo-code used `.file()`, `.span()`, `.label()` but the actual API uses `.with_primary(file_id, span, label)`, `.with_secondary(file_id, span, msg)`, `.with_help(text)`. Adapted implementation to match.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Token Display not implemented — used {:?} instead**
- **Found during:** Task 2 (cmd_compile implementation)
- **Issue:** `writ_parser::Token<'_>` doesn't implement `Display`, so `format!("{}", err)` fails since `Rich<'_, T, S>: Display` requires `T: Display`
- **Fix:** Changed parse error formatting to `{:?}` debug format
- **Files modified:** writ-cli/src/main.rs
- **Verification:** Cargo build succeeds
- **Committed in:** f979237 (Task 2 commit)

**2. [Rule 1 - Bug] parse() requires 'static source string**
- **Found during:** Task 2 (cmd_compile implementation)
- **Issue:** `writ_parser::parse` return type `Vec<Rich<'static, Token<'src>, Span>>` requires that `'src = 'static`, but a local `String` read from disk cannot satisfy this
- **Fix:** Used `Box::leak(src_owned.into_boxed_str())` to obtain `&'static str` before calling `parse()`
- **Files modified:** writ-cli/src/main.rs
- **Verification:** Cargo build succeeds, `writ compile hello.writ` produces `hello.writil`
- **Committed in:** f979237 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 - bugs in API assumptions in plan)
**Impact on plan:** Both auto-fixes necessary for compilation. No scope creep.

## Issues Encountered
- Stack overflow in debug build when parsing even minimal `.writ` files due to deeply recursive chumsky parser combinators. Release build (`cargo build --release`) works correctly. This is a pre-existing characteristic of debug chumsky parsers on Windows, not introduced by this plan.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `writ compile` subcommand fully functional and produces `.writil` binary output
- Full pipeline wired: parse -> lower -> resolve -> typecheck -> emit_bodies
- Ariadne error rendering works at every stage with source spans
- Ready for Phase 26 Plan 03: end-to-end validation and integration tests

---
*Phase: 26-cli-integration-e2e-validation*
*Completed: 2026-03-03*
