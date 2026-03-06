---
phase: 48-il-format
plan: 02
subsystem: compiler/runtime/assembler
tags: [writ-assembler, writ-compiler, writ-runtime, TypeDefKind, IL-format, class]

# Dependency graph
requires:
  - phase: 48-il-format/01
    provides: TypeDefKind::Class=4 in writ-module/tables.rs, add_type_def takes TypeDefKind

provides:
  - AsmTypeKind::Class variant in assembler AST
  - "class" directive parsing in assembler
  - TypeDefKind::Class arm in disassembler (None -> unreachable!())
  - Round-trip test for .class directive
  - Unified TypeDefKind: single definition in writ-module, re-exported from writ-compiler/metadata.rs
  - Runtime virtual_module.rs uses TypeDefKind enum (not raw integers)
  - All test files updated (vm_tests, gc_tests, task_tests, hook_dispatch_tests, domain.rs)
  - Compiler format_version=3 (was 2)
  - Full workspace: all tests pass, no duplicate TypeDefKind definitions

affects: [49-vm, 50-compiler, 51-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "TypeDefKind single-source: defined in writ_module::tables, re-exported where needed via pub use"
    - "Raw integer kind values replaced with TypeDefKind enum variants at all public API boundaries"
    - "Binary format stays u8 (TypeDefRow.kind); enum used at builder/reader API boundaries only"

key-files:
  created: []
  modified:
    - writ-assembler/src/ast.rs
    - writ-assembler/src/parser.rs
    - writ-assembler/src/assembler.rs
    - writ-assembler/src/disassembler.rs
    - writ-assembler/tests/asm_round_trip.rs
    - writ-compiler/src/emit/metadata.rs
    - writ-compiler/src/emit/serialize.rs
    - writ-runtime/src/virtual_module.rs
    - writ-runtime/src/domain.rs
    - writ-runtime/tests/vm_tests.rs
    - writ-runtime/tests/gc_tests.rs
    - writ-runtime/tests/task_tests.rs
    - writ-runtime/tests/hook_dispatch_tests.rs

key-decisions:
  - "Re-export pattern: delete local TypeDefKind from metadata.rs, add 'pub use writ_module::TypeDefKind' so all existing 'use crate::emit::metadata::TypeDefKind' imports continue resolving unchanged"
  - "disassembler None arm replaced with unreachable!() -- reader validates kinds so invalid values never reach disassembler"
  - "format_version bump from 2 to 3 in compiler serialize.rs (required for Module::from_bytes to accept compiled output)"
  - "domain.rs and all test files updated as auto-fix (Rule 3 blocking) since add_type_def API changed in Plan 01"

patterns-established:
  - "TypeDefKind unification: all crates import from writ_module::tables or writ_module (re-export); no local duplicates"

requirements-completed: [IL-01, IL-02, IL-03]

# Metrics
duration: 25min
completed: 2026-03-12
---

# Phase 48 Plan 02: Consumer Crate TypeDefKind Unification Summary

**TypeDefKind unified across all crates: assembler handles .class directives with round-trip fidelity, compiler re-exports from writ_module (no local duplicate), runtime uses enum variants throughout**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-03-12T19:00:00Z
- **Completed:** 2026-03-12T19:25:00Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments
- Added `AsmTypeKind::Class` to assembler AST and `"class"` arm to parser, enabling `.class` directives in `.writil` text files
- Updated assembler to pass `TypeDefKind` enum (not raw u8) to `builder.add_type_def()`; disassembler now emits `"class"` for kind=4 and uses `unreachable!()` for invalid kinds
- Added `test_class_round_trip` integration test: `.class MyClass {}` assembles to kind=4, round-trips through binary, disassembles back to `.type "MyClass" class`
- Deleted local `TypeDefKind` from `writ-compiler/src/emit/metadata.rs`; replaced with `pub use writ_module::TypeDefKind` — zero import path changes in downstream compiler code
- Updated `writ-runtime/src/virtual_module.rs` to use `TypeDefKind::Struct/Enum/Entity` instead of raw integers `0/1/2`
- Bumped compiler `format_version` from 2 to 3 so `Module::from_bytes` accepts compiled output
- Full workspace: 0 failures across all test suites

## Task Commits

Each task was committed atomically:

1. **Task 1: Update assembler and disassembler for class support** - `ca12a85` (feat)
2. **Task 2: Unify TypeDefKind across compiler and runtime** - `dc71a03` (feat)

**Plan metadata:** (see final metadata commit)

## Files Created/Modified
- `writ-assembler/src/ast.rs` - Added `AsmTypeKind::Class` variant
- `writ-assembler/src/parser.rs` - Added `"class"` arm to kind match
- `writ-assembler/src/assembler.rs` - Uses `TypeDefKind` enum, added import
- `writ-assembler/src/disassembler.rs` - Added `Class` match arm, `None => unreachable!()`
- `writ-assembler/tests/asm_round_trip.rs` - Added `test_class_round_trip`
- `writ-compiler/src/emit/metadata.rs` - Deleted local enum, added `pub use writ_module::TypeDefKind`
- `writ-compiler/src/emit/serialize.rs` - `format_version = 3` (was 2)
- `writ-runtime/src/virtual_module.rs` - All `add_type_def` calls use `TypeDefKind` enum
- `writ-runtime/src/domain.rs` - All test `add_type_def` calls use `TypeDefKind` enum
- `writ-runtime/tests/vm_tests.rs` - Added `TypeDefKind` import, fixed all kind values
- `writ-runtime/tests/gc_tests.rs` - Added `TypeDefKind` import, fixed all kind values
- `writ-runtime/tests/task_tests.rs` - Added `TypeDefKind` import, fixed all kind values
- `writ-runtime/tests/hook_dispatch_tests.rs` - Added `TypeDefKind` import, fixed all kind values

## Decisions Made
- Re-export pattern: `pub use writ_module::TypeDefKind` in metadata.rs preserves all existing `use crate::emit::metadata::TypeDefKind` import paths without modification
- `None => unreachable!()` in disassembler: Plan 01 made the reader validate kinds before passing to the disassembler, making the fallback structurally impossible
- `format_version = 3` required: reader (Plan 01) rejects version != 3 with `UnsupportedVersion`, so the compiler must emit version 3 for compiled modules to deserialize

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed writ-runtime/src/domain.rs raw integer add_type_def calls**
- **Found during:** Task 2 (full workspace cargo test)
- **Issue:** domain.rs test helpers used `builder.add_type_def(..., 0, 0)` with raw integer kind; builder API now requires `TypeDefKind`
- **Fix:** Added `use writ_module::tables::TypeDefKind;` in the test module block; replaced all `0` kind args with `TypeDefKind::Struct`
- **Files modified:** `writ-runtime/src/domain.rs`
- **Verification:** `cargo build -p writ-runtime` succeeded
- **Committed in:** `dc71a03` (Task 2 commit)

**2. [Rule 3 - Blocking] Fixed raw integer add_type_def calls in all runtime test files**
- **Found during:** Task 2 (cargo test full workspace)
- **Issue:** vm_tests.rs, gc_tests.rs, task_tests.rs, hook_dispatch_tests.rs all used raw integer kind values in `add_type_def` calls; 9+ compile errors
- **Fix:** Added `use writ_module::tables::TypeDefKind;` to each test file; replaced all raw integer kind values with appropriate `TypeDefKind::*` variants
- **Files modified:** `writ-runtime/tests/vm_tests.rs`, `gc_tests.rs`, `task_tests.rs`, `hook_dispatch_tests.rs`
- **Verification:** All tests pass
- **Committed in:** `dc71a03` (Task 2 commit)

**3. [Rule 3 - Blocking] Fixed compiler format_version=2 -> 3**
- **Found during:** Task 2 (writ-cli e2e tests failing with UnsupportedVersion(2))
- **Issue:** `writ-compiler/src/emit/serialize.rs` still wrote `format_version = 2`; Plan 01 made the reader reject anything except 3
- **Fix:** Changed `format_version = 2` to `format_version = 3` in serialize.rs
- **Files modified:** `writ-compiler/src/emit/serialize.rs`
- **Verification:** e2e compile tests pass (`test_compile_and_run_minimal`, `test_compile_produces_valid_module_header`, `test_locale_override_produces_locale_def_rows`)
- **Committed in:** `dc71a03` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 3 - Blocking)
**Impact on plan:** All auto-fixes were direct consequences of the Plan 01 API change (add_type_def now requires TypeDefKind). The format_version fix was required because Plan 01 introduced version validation. No scope creep.

## Issues Encountered
- Disk full (12.8 GiB build artifacts) before first test run — resolved with `cargo clean`
- Multiple background build processes competed for file lock — resolved by waiting and running synchronously

## Next Phase Readiness
- TypeDefKind is now a single source of truth in `writ_module::tables`; all crates use it
- Assembler/disassembler support `.class` directives with full round-trip fidelity
- Compiler emits `format_version=3` compatible with reader validation
- Ready for Phase 49: VM value-type struct implementation

---
*Phase: 48-il-format*
*Completed: 2026-03-12*
