---
phase: 26-cli-integration-e2e-validation
plan: 03
subsystem: testing
tags: [writ-cli, writ-compiler, writ-parser, e2e, integration-tests, emit_bodies, pipeline]

# Dependency graph
requires:
  - phase: 26-cli-integration-e2e-validation/26-02
    provides: "writ compile subcommand wiring full 5-stage pipeline"
  - phase: 25-il-codegen-method-bodies
    provides: emit_bodies() function producing binary .writil output
provides:
  - "4 e2e integration tests validating full source->compile->run pipeline"
  - "writ-cli/tests/e2e_compile_tests.rs exercising parse->lower->resolve->typecheck->codegen->load->execute"
  - "Fixed emit_bodies() to produce complete modules (TypeDef/MethodDef/ExportDef tables)"
  - "Fixed ExportDef item_kind encoding: Fn=0 (method), Struct/Entity=1 (type)"
affects:
  - "All future writ compile users expecting exports in .writil output"

# Tech tracking
tech-stack:
  added:
    - writ-compiler dev-dependency in writ-cli/Cargo.toml
    - writ-diagnostics dev-dependency in writ-cli/Cargo.toml
    - writ-parser dev-dependency in writ-cli/Cargo.toml
  patterns:
    - "Box::leak for src string reused from Plan 02 to satisfy 'static lifetime in e2e tests"
    - "compile_source() helper wraps full 5-stage pipeline for test reuse"
    - "compile_expect_error() walks through type check stage for E0102 UndefinedVariable"
    - "emit_bodies(typed_ast, interner, asts) now calls collect_defs+collect_post_finalize for complete modules"

key-files:
  created:
    - writ-cli/tests/fixtures/hello.writ
    - writ-cli/tests/fixtures/error_resolution.writ
    - writ-cli/tests/e2e_compile_tests.rs
  modified:
    - writ-cli/Cargo.toml
    - writ-cli/src/main.rs
    - writ-compiler/src/emit/mod.rs
    - writ-compiler/src/emit/collect.rs
    - writ-compiler/tests/emit_body_tests.rs
    - writ-compiler/tests/emit_serialize_tests.rs

key-decisions:
  - "emit_bodies() extended to accept asts parameter and call full metadata collection (collect_defs + collect_post_finalize); existing unit tests pass &[] for programmatic TypedAst"
  - "ExportDef item_kind: 0=method (Fn/ExternFn), 1=type (Struct/Entity/Enum), 2=global (Const/Global) — was reversed in collect_exports"
  - "Undefined variable references (let x = undefined_name) produce E0102 at typecheck stage, not E0003 at resolution — resolver only validates type-level names"

patterns-established:
  - "E2E test pattern: compile_source(src) -> Result<Vec<u8>> wraps all 5 pipeline stages for reuse"
  - "Module deserialization test: Module::from_bytes(&bytes) validates serialized output is spec-compliant"
  - "Export lookup: read_string(&module.string_heap, export.name) + item_kind==0 finds pub fn main()"

requirements-completed: [CLI-01, CLI-02, CLI-03, FIX-01, FIX-02, FIX-03]

# Metrics
duration: 12min
completed: 2026-03-03
---

# Phase 26 Plan 03: E2E Compile Pipeline Tests Summary

**4 end-to-end integration tests validate the full Writ source->compile->run pipeline, with emit_bodies() fixed to produce complete modules including ExportDef tables via collect_defs+collect_post_finalize**

## Performance

- **Duration:** 12 min
- **Started:** 2026-03-03T14:36:22Z
- **Completed:** 2026-03-03T14:48:00Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Created `writ-cli/tests/fixtures/` with `hello.writ` (minimal valid program) and `error_resolution.writ` (undefined name error case)
- Wrote 4 e2e integration tests in `writ-cli/tests/e2e_compile_tests.rs` — all pass, exercising the full source-to-execution pipeline
- Fixed `emit_bodies()` to call `collect_defs` and `collect_post_finalize`, producing complete modules with TypeDef/MethodDef/ExportDef tables
- Fixed `collect_exports` ExportDef item_kind encoding (Fn=0 method, not 1; Struct=1 type, not 0)
- 1063 total tests pass across workspace (all 334 compiler + 235 runtime + 15 cli + others)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Writ fixture files for e2e testing** - `11c114c` (feat)
2. **Task 2: End-to-end compile and run integration tests (TDD)** - `bead270` (feat)

**Plan metadata:** (to be committed as docs metadata)

## Files Created/Modified
- `writ-cli/tests/fixtures/hello.writ` - Minimal compilable Writ program: `pub fn main() { let x: int = 42; }`
- `writ-cli/tests/fixtures/error_resolution.writ` - Program with undefined name: `let x = undefined_name;`
- `writ-cli/tests/e2e_compile_tests.rs` - 4 integration tests: compile_minimal, valid_header, compile_and_run, compile_error
- `writ-cli/Cargo.toml` - Added writ-compiler, writ-diagnostics, writ-parser to [dev-dependencies]
- `writ-cli/src/main.rs` - Updated cmd_compile to pass &[(file_id, &ast)] to emit_bodies
- `writ-compiler/src/emit/mod.rs` - emit_bodies() signature extended with asts parameter; now calls collect_defs, assign_vtable_slots, pre_scan_lambdas, finalize, collect_post_finalize in correct order
- `writ-compiler/src/emit/collect.rs` - Fixed collect_exports item_kind mapping (Fn=0, Struct=1)
- `writ-compiler/tests/emit_body_tests.rs` - Updated emit_bodies call to pass &[]
- `writ-compiler/tests/emit_serialize_tests.rs` - Updated emit_bodies calls to pass &[]

## Decisions Made

- **emit_bodies asts parameter:** Added `asts: &[(FileId, &Ast)]` to enable full metadata collection. Unit tests that construct TypedAst programmatically pass `&[]`, which correctly produces no metadata rows (but still emits method bodies). CLI and e2e tests pass real ASTs, producing complete modules.
- **ExportDef item_kind fix:** The collect_exports function had `Struct/Entity=0, Fn=1` (reversed). Correct per disassembler: `Fn=0 (method)`, `Struct/Entity=1 (type)`. Fixed as part of auto-fix (Rule 1 Bug).
- **Error stage for undefined names:** `undefined_name` in a let binding produces E0102 (TypeError::UndefinedVariable) at the typechecking stage, not E0003 at resolution. The resolver only validates type-level names. compile_expect_error() was updated to run through typecheck stage.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] emit_bodies produced empty modules (no TypeDef/MethodDef/ExportDef)**
- **Found during:** Task 2 (TDD RED phase)
- **Issue:** `emit_bodies()` was a "simplified path" that skipped `collect_defs` and `collect_post_finalize`. The serializer had no MethodDef rows to iterate, so no method bodies were serialized. The output module was valid bytes but useless for `writ run` (no exports).
- **Fix:** Added `asts: &[(FileId, &Ast)]` parameter. emit_bodies now calls the full collection pipeline: `collect_defs` -> `assign_vtable_slots` -> `pre_scan_lambdas` -> `finalize` -> `collect_post_finalize` -> `emit_all_bodies`. Updated cmd_compile and unit tests.
- **Files modified:** writ-compiler/src/emit/mod.rs, writ-cli/src/main.rs, writ-compiler/tests/emit_body_tests.rs, writ-compiler/tests/emit_serialize_tests.rs
- **Verification:** `test_compile_and_run_minimal` passes — export found, method_idx resolved, VM executes main
- **Committed in:** bead270 (Task 2 commit)

**2. [Rule 1 - Bug] ExportDef item_kind encoding reversed in collect_exports**
- **Found during:** Task 2 (GREEN phase — test_compile_and_run_minimal failed with item_kind=1 != 0)
- **Issue:** `collect_exports` assigned `item_kind=0` to Struct/Entity (should be 1=type) and `item_kind=1` to Fn (should be 0=method). The encoding was exactly backwards from the disassembler and cmd_run conventions.
- **Fix:** Corrected mapping: `Fn/ExternFn => 0` (method), `Struct/Entity/Enum/... => 1` (type), `Const/Global => 2` (global).
- **Files modified:** writ-compiler/src/emit/collect.rs
- **Verification:** `test_compile_and_run_minimal` now finds export with item_kind=0, method_idx works
- **Committed in:** bead270 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 - bugs in emit pipeline design)
**Impact on plan:** Both fixes were essential for the e2e test to pass. No scope creep — the fixes corrected existing bugs that would have broken `writ compile` for all users.

## Issues Encountered
- `cargo test -p writ-cli e2e` filter syntax doesn't match test file names — correct syntax is `cargo test -p writ-cli --test e2e_compile_tests`

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full compile+run pipeline validated end-to-end: `pub fn main() { let x: int = 42; }` compiles and executes
- Compilation errors display at the correct pipeline stage (resolution or type checking)
- All 1063 tests pass across the workspace
- Phase 26 is complete: all 3 plans done (01 runtime bugs, 02 compile subcommand, 03 e2e tests)
- v3.0 milestone "Writ Compiler" is now fully validated

## Self-Check: PASSED

### Files Exist
- [x] writ-cli/tests/fixtures/hello.writ
- [x] writ-cli/tests/fixtures/error_resolution.writ
- [x] writ-cli/tests/e2e_compile_tests.rs

### Commits Exist
- [x] 11c114c feat(26-03): add Writ e2e test fixture files
- [x] bead270 feat(26-03): e2e compile pipeline tests + emit_bodies full metadata fix

---
*Phase: 26-cli-integration-e2e-validation*
*Completed: 2026-03-03*
