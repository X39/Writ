---
phase: 75-baseline-build-config-and-inline-annotations
plan: 01
subsystem: runtime
tags: [rustc-hash, FxHashMap, inline, lto, release-profile, writ-runtime, dispatch]

requires: []
provides:
  - Release profile with LTO fat, codegen-units=1, panic=abort in Cargo.toml
  - rustc-hash 2.1.1 as writ-runtime dependency
  - FxHashMap replacing HashMap in all 5 writ-runtime hot-path files
  - inline(always) on 5 extract_* helpers in dispatch/helpers.rs
  - inline on 49 exec_* arithmetic/control-flow functions in dispatch/arith.rs
  - inline on 5 call-dispatch functions in dispatch/calls.rs
  - inline on execute_ret in dispatch/mod.rs
  - execute_one intentionally has NO inline annotation
affects: [76-zero-alloc-call-convention, 77-frame-register-pool, 78-inner-dispatch-loop, 79-copy-value]

tech-stack:
  added: [rustc-hash 2.1.1]
  patterns:
    - FxHashMap used for all hash maps in writ-runtime hot paths (consistent with rustc-hash crate)
    - inline(always) on value extraction helpers (5 simple match arms, guaranteed inlineable)
    - inline on exec_* dispatch handlers (optimizer hint, not forced)
    - execute_one has no inline (large match dispatch; forcing inline would bloat call sites)

key-files:
  created: []
  modified:
    - Cargo.toml
    - writ-runtime/Cargo.toml
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/dispatch/helpers.rs
    - writ-runtime/src/dispatch/arith.rs
    - writ-runtime/src/dispatch/calls.rs
    - writ-runtime/src/scheduler.rs
    - writ-runtime/src/domain.rs
    - writ-runtime/src/entity.rs
    - writ-runtime/src/loader.rs

key-decisions:
  - "FxHashMap replaces std HashMap throughout writ-runtime — non-cryptographic hash is safe and faster for integer/struct keys used in dispatch and scheduling"
  - "inline(always) for extract_* helpers (5-10 instructions each, on the absolute hot path), inline for exec_* (let optimizer decide per call site)"
  - "execute_one has NO inline — forcing inline of a 300-arm match would bloat all callers (scheduler, defer handler, crash unwind)"

patterns-established:
  - "use rustc_hash::FxHashMap + FxHashMap::default() pattern for all hot-path hash maps in writ-runtime"
  - "#[inline(always)] for trivial value extraction helpers; #[inline] for larger dispatch handlers"

requirements-completed: [BUILD-01, BUILD-02, BUILD-03, BUILD-04, INLINE-01, INLINE-02, INLINE-03, INLINE-04]

duration: 14min
completed: 2026-03-22
---

# Phase 75 Plan 01: Baseline Build Config and Inline Annotations Summary

**Release profile with LTO/fat/panic=abort, FxHashMap across 5 writ-runtime files, and inline annotations on all dispatch helpers — 88 tests pass in release mode**

## Performance

- **Duration:** 14 min
- **Started:** 2026-03-22T05:09:23Z
- **Completed:** 2026-03-22T05:23:31Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Configured release profile with `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` in workspace Cargo.toml
- Replaced `std::collections::HashMap` with `rustc_hash::FxHashMap` in all 5 writ-runtime hot-path files (dispatch/mod.rs, scheduler.rs, domain.rs, entity.rs, loader.rs)
- Applied `#[inline(always)]` to all 5 `extract_*` value extraction helpers in dispatch/helpers.rs
- Applied `#[inline]` to all 49 `exec_*` functions in dispatch/arith.rs (arithmetic, control flow, conversions, strings, boxing)
- Applied `#[inline]` to 5 call-dispatch functions in dispatch/calls.rs and `execute_ret` in dispatch/mod.rs
- Confirmed `execute_one` has no inline annotation (intentional — prevents bloating 3 call sites with a 300-arm match)

## Task Commits

1. **Task 1: Release profile and FxHashMap substitution** - `c3c7ee3` (feat)
2. **Task 2: Inline annotations on dispatch helpers** - `06ece38` (feat)

## Files Created/Modified

- `Cargo.toml` - Added [profile.release] with lto/codegen-units/panic settings
- `writ-runtime/Cargo.toml` - Added rustc-hash 2.1.1 dependency
- `writ-runtime/src/dispatch/mod.rs` - FxHashMap for DispatchTable; #[inline] on execute_ret
- `writ-runtime/src/dispatch/helpers.rs` - #[inline(always)] on all 5 extract_* functions
- `writ-runtime/src/dispatch/arith.rs` - #[inline] on all 49 exec_* functions
- `writ-runtime/src/dispatch/calls.rs` - #[inline] on 5 call-dispatch functions
- `writ-runtime/src/scheduler.rs` - FxHashMap for tasks, global_locks, join_waiters
- `writ-runtime/src/domain.rs` - FxHashMap for ResolvedRefs (types, contracts, methods, fields)
- `writ-runtime/src/entity.rs` - FxHashMap for singletons, pending
- `writ-runtime/src/loader.rs` - FxHashMap for local offset_map in decode_and_reindex

## Decisions Made

- FxHashMap replaces std HashMap throughout writ-runtime — non-cryptographic hash is safe and faster for integer/struct keys (DispatchKey, TaskId, u32 row indices) used in dispatch and scheduling
- `#[inline(always)]` for `extract_*` helpers (5-10 instructions each, on the absolute hot path every instruction dispatch), `#[inline]` for exec_* (let optimizer decide per call site based on code size)
- `execute_one` has NO inline — forcing inline of a 300+ arm match would bloat all callers (scheduler loop, defer handler, crash unwind)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## Next Phase Readiness

- Build config foundation complete; release profile will be used for all Phase 75 Plan 02 baseline measurements
- FxHashMap substitution complete; hot-path dispatch and scheduling now use identity-hash-based maps
- Inline hints in place; compiler will respect them in the next release build
- Phase 76 (zero-alloc call convention) can begin immediately

---
*Phase: 75-baseline-build-config-and-inline-annotations*
*Completed: 2026-03-22*
