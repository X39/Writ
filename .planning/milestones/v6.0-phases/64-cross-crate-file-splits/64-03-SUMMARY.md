---
phase: 64-cross-crate-file-splits
plan: 03
subsystem: compiler
tags: [rust, refactoring, file-split, writ-dap, writ-runtime, writ-cli]

# Dependency graph
requires:
  - phase: 64-01
    provides: writ-lsp/src/queries.rs split into queries/ folder
  - phase: 64-02
    provides: writ-lsp queries.rs sibling split patterns established

provides:
  - writ-dap/src/server/ folder module (4 files): mod.rs, handlers.rs, helpers.rs, inspection.rs
  - writ-runtime/src/domain_dispatch.rs (dispatch table + resolve_intrinsic_id split out)
  - writ-cli/src/pipeline.rs + commands/ folder (6 command files)

affects: [phase-65, writ-dap, writ-runtime, writ-cli]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "server/ folder module pattern: struct in mod.rs, handler methods in handlers.rs, free helpers in helpers.rs, inspection methods in inspection.rs"
    - "sibling dispatch module: domain_dispatch.rs declares impl Domain blocks in same crate, accessing private fields"
    - "CLI commands/ folder: one file per subcommand, mod.rs re-exports with pub use"

key-files:
  created:
    - writ-dap/src/server/mod.rs
    - writ-dap/src/server/handlers.rs
    - writ-dap/src/server/helpers.rs
    - writ-dap/src/server/inspection.rs
    - writ-runtime/src/domain_dispatch.rs
    - writ-cli/src/pipeline.rs
    - writ-cli/src/commands/mod.rs
    - writ-cli/src/commands/new.rs
    - writ-cli/src/commands/build.rs
    - writ-cli/src/commands/compile.rs
    - writ-cli/src/commands/assemble.rs
    - writ-cli/src/commands/disasm.rs
    - writ-cli/src/commands/run.rs
  modified:
    - writ-runtime/src/domain.rs
    - writ-runtime/src/lib.rs
    - writ-cli/src/main.rs

key-decisions:
  - "helpers.rs decode_frame_id and build_thread_list made pub(super) (not pub(crate)) — only called from inspection.rs and handlers.rs within the server/ module"
  - "DapServer struct fields changed from private to pub(super) — needed for impl blocks in handlers.rs and inspection.rs to access self.runtime, self.module, self.task_id etc."
  - "domain_dispatch.rs uses mod domain_dispatch (private, not pub mod) in lib.rs — dispatch fn is on Domain impl which is already pub through domain module"
  - "CLI cmd_ functions declared pub (not pub(crate)) — required by Rust's re-export rules for pub use in commands/mod.rs in a binary crate"
  - "Tests in domain.rs add explicit use crate::dispatch::{DispatchTarget, IntrinsicId} — no longer available via super::* after removing the top-level dispatch import"

patterns-established:
  - "Multi-file impl: Rust allows impl Domain blocks across multiple files within the same crate — domain_dispatch.rs contains impl Domain without re-declaring the struct"
  - "Server folder pattern: handle_request dispatch in mod.rs delegates to pub(super) methods in handlers.rs; inspection methods in inspection.rs; all share DapServer fields via pub(super)"

requirements-completed: [SPLIT-06, SPLIT-07, SPLIT-14]

# Metrics
duration: 23min
completed: 2026-03-18
---

# Phase 64 Plan 03: Cross-Crate File Splits (DAP + Runtime + CLI) Summary

**server.rs (1,140 lines) split into server/ folder module with 4 files; domain.rs dispatch group extracted to domain_dispatch.rs; main.rs (703 lines) split into main.rs + pipeline.rs + commands/ folder (8 files)**

## Performance

- **Duration:** ~23 min
- **Started:** 2026-03-18T03:33:29Z
- **Completed:** 2026-03-18T03:56:19Z
- **Tasks:** 3
- **Files modified:** 16 (3 modified, 13 created)

## Accomplishments
- Split writ-dap/src/server.rs into 4-file folder module: DapServer struct + dispatch in mod.rs, handler methods in handlers.rs, free helpers + tests in helpers.rs, inspection methods in inspection.rs
- Split writ-runtime/src/domain.rs dispatch group (build_dispatch_table + helpers + resolve_intrinsic_id) into sibling domain_dispatch.rs using Rust's multi-file impl block pattern
- Split writ-cli/src/main.rs into minimal CLI definition in main.rs, shared 5-stage pipeline in pipeline.rs, and 6 command files in commands/ folder
- All 3 crates: tests pass, zero clippy warnings with -D warnings

## Task Commits

Each task was committed atomically:

1. **Task 1: Split server.rs into server/ folder module** - `a490350` (feat)
2. **Task 2: Split domain.rs into domain.rs + domain_dispatch.rs** - `08827e4` (feat)
3. **Task 3: Split main.rs into main.rs + pipeline.rs + commands/** - `4b3edd1` (feat)

## Files Created/Modified
- `writ-dap/src/server/mod.rs` - DapServer struct, run(), handle_request() dispatch, stdio_server()
- `writ-dap/src/server/handlers.rs` - Handler methods per DAP command (handle_launch, handle_set_breakpoints, etc.)
- `writ-dap/src/server/helpers.rs` - Free helpers (decode_frame_id, build_thread_list, collect_frame_variables, instr_to_byte_pc) + all tests
- `writ-dap/src/server/inspection.rs` - Runtime inspection methods (run_until_stop, build_stack_frames, get_variables, etc.)
- `writ-runtime/src/domain.rs` - Resolution types + Domain struct + resolution impl methods + tests (dispatch section removed)
- `writ-runtime/src/domain_dispatch.rs` - build_dispatch_table, resolve_type_key, resolve_contract_key_for_impl, get_contract_method_count, get_type_name, resolve_intrinsic_id
- `writ-runtime/src/lib.rs` - Added mod domain_dispatch (private)
- `writ-cli/src/main.rs` - CLI definition (Cli + Commands structs) and main() dispatch only
- `writ-cli/src/pipeline.rs` - Shared run_pipeline() 5-stage compile pipeline
- `writ-cli/src/commands/{mod,new,build,compile,assemble,disasm,run}.rs` - One file per CLI subcommand

## Decisions Made
- `helpers.rs` free functions made `pub(super)` so they're accessible from `handlers.rs` and `inspection.rs` within the `server/` module boundary
- `DapServer` struct fields changed to `pub(super)` to allow the multiple `impl` blocks in submodule files to access them
- `domain_dispatch.rs` declared as `mod domain_dispatch` (not `pub mod`) in lib.rs — dispatch functions are methods on the already-public `Domain` type
- CLI `cmd_*` functions declared `pub` (not `pub(crate)`) — required by Rust re-export rules: `pub use` in `commands/mod.rs` requires the items to be `pub`
- Tests in `domain.rs` add explicit `use crate::dispatch::{DispatchTarget, IntrinsicId}` — these types were previously available via the top-level module import which was removed

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] DapServer struct fields visibility**
- **Found during:** Task 1 (server.rs split)
- **Issue:** Original code had private fields on DapServer; handlers.rs and inspection.rs impl blocks need field access
- **Fix:** Changed fields to `pub(super)` so all submodules within `server/` can access them
- **Files modified:** writ-dap/src/server/mod.rs
- **Verification:** cargo build -p writ-dap passes
- **Committed in:** a490350 (Task 1 commit)

**2. [Rule 1 - Bug] domain.rs tests lost DispatchTarget/IntrinsicId imports**
- **Found during:** Task 2 (domain.rs split)
- **Issue:** Removing `use crate::dispatch::{...}` from module level meant tests could no longer use DispatchTarget and IntrinsicId via `super::*`
- **Fix:** Added explicit `use crate::dispatch::{DispatchTarget, IntrinsicId}` inside the `#[cfg(test)]` mod
- **Files modified:** writ-runtime/src/domain.rs
- **Verification:** cargo test -p writ-runtime: 88 tests pass
- **Committed in:** 08827e4 (Task 2 commit)

**3. [Rule 1 - Bug] pub(crate) re-export error in commands/mod.rs**
- **Found during:** Task 3 (main.rs split)
- **Issue:** Rust requires `pub` (not `pub(crate)`) for items that are re-exported via `pub use` in a module
- **Fix:** Changed all cmd_* functions from `pub(crate)` to `pub` in command files; same for run_pipeline
- **Files modified:** writ-cli/src/commands/*.rs, writ-cli/src/pipeline.rs
- **Verification:** cargo build -p writ-cli passes
- **Committed in:** 4b3edd1 (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 1 - Bug)
**Impact on plan:** All auto-fixes were correctness requirements for the split to compile. No scope creep.

## Issues Encountered
None — all issues were handled via deviation rules.

## Next Phase Readiness
- All three crates pass tests and clippy with zero warnings
- SPLIT-06, SPLIT-07, SPLIT-14 requirements satisfied
- Phase 64 complete — ready for Phase 65 (module boundary cleanup / consolidation)

---
*Phase: 64-cross-crate-file-splits*
*Completed: 2026-03-18*

## Self-Check: PASSED

All files verified present. All commits verified in git history. Key content patterns confirmed.
