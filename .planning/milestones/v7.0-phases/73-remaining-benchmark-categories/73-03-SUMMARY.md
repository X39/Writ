---
phase: 73-remaining-benchmark-categories
plan: 03
subsystem: benchmark
tags: [writ, lua, squirrel, python, javascript, rust, oop, dispatch, object-allocation, contracts, impl]

requires:
  - phase: 73-remaining-benchmark-categories
    provides: benchmark harness and measurement infrastructure from plans 73-01 and 73-02

provides:
  - oop_dispatch benchmark: 6-language virtual dispatch comparison (contract/impl, metatables, class extends, trait objects)
  - object_create benchmark: 6-language object allocation comparison (class new with 3 fields, 1M iterations)

affects: [Phase 74, ROADMAP.md BENCH-06, ROADMAP.md BENCH-07, bench_runner.sh, generate.py]

tech-stack:
  added: []
  patterns:
    - "Writ contract/impl dispatch: define contract, impl per concrete type, call obj.method() — emitter resolves via methoddef_token_by_type_and_name"
    - "Impl method call resolution: IMPL-METHOD fix in emit/body/expr/mod.rs intercepts Field callee with Struct/Class receiver when callee_def_id is None"
    - "MetadataToken stripping: exec_new and get_type_field_count strip table bits (& 0x00FF_FFFF) before using as index"

key-files:
  created:
    - benchmark/cases/oop_dispatch/oop_dispatch.writ
    - benchmark/cases/oop_dispatch/oop_dispatch.lua
    - benchmark/cases/oop_dispatch/oop_dispatch.nut
    - benchmark/cases/oop_dispatch/oop_dispatch.py
    - benchmark/cases/oop_dispatch/oop_dispatch.js
    - benchmark/cases/oop_dispatch/oop_dispatch.rs
    - benchmark/cases/object_create/object_create.writ
    - benchmark/cases/object_create/object_create.lua
    - benchmark/cases/object_create/object_create.nut
    - benchmark/cases/object_create/object_create.py
    - benchmark/cases/object_create/object_create.js
    - benchmark/cases/object_create/object_create.rs
  modified:
    - writ-compiler/src/emit/body/expr/mod.rs
    - writ-compiler/src/emit/module_builder.rs
    - writ-compiler/src/emit/collect/contracts.rs
    - writ-compiler/src/emit/collect/lookup.rs

key-decisions:
  - "Contract/impl dispatch (IMPL-METHOD fix): when callee_def_id is None and callee is Field{receiver:Struct/Class}, look up MethodDef token by type+name via methoddef_token_by_type_and_name and emit direct CALL — prevents spurious CALL_INDIRECT"
  - "collect_impl iteration: iterate AST fn_decls by index (parallel to methods vec) using fn_decl.name directly, not method_entry.name which resolves to impl#N synthetic name"
  - "oop_dispatch uses concrete-type dispatch in separate if-branches (not dynamic base pointer) because Writ lacks else-if and base-type variable polymorphism — algorithmic equivalence maintained: same 4 types, same cycling, same sum"
  - "object_create uses pub class Point with 3 fields (x: int, y: int, label: string); Rust uses &'static str for label to avoid heap allocation (ceiling reference)"

patterns-established:
  - "Impl method smoke-test before full benchmark: compile+run minimal contract/impl snippet to validate the pipeline before writing 100-iteration benchmark"
  - "MetadataToken always strip table bits with (token & 0x00FF_FFFF) before treating as row index in runtime dispatch"

requirements-completed: [BENCH-06, BENCH-07]

duration: 45min
completed: 2026-03-20
---

# Phase 73 Plan 03: OOP Dispatch and Object Creation Benchmarks Summary

**OOP/dispatch and object creation benchmarks across 6 languages with Writ contract/impl dispatch pipeline fixes that enable direct method call compilation**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-03-20T~19:15Z (continuation from previous session)
- **Completed:** 2026-03-20T~20:00Z
- **Tasks:** 2
- **Files modified:** 14 (12 created, 2 modified)

## Accomplishments

- Fixed the Writ impl-method dispatch bug: `obj.method()` via `impl Contract for Type` now compiles to direct CALL instead of CALL_INDIRECT (which crashed with "not a delegate")
- Created 6-language oop_dispatch benchmark using native polymorphism per language; all output `250000`
- Created 6-language object_create benchmark allocating 1M 3-field objects; all output `499999500000`
- All 39 golden tests still pass after compiler changes

## Task Commits

1. **Task 1: Create oop_dispatch benchmark files for all 6 languages** - `2f71be4` (feat)
2. **Task 2: Create object_create benchmark files for all 6 languages** - `455b4ff` (feat)

**Plan metadata:** (final doc commit)

## Files Created/Modified

- `benchmark/cases/oop_dispatch/oop_dispatch.writ` - Writ contract/impl dispatch: TypeA-D each impl Computable.compute()
- `benchmark/cases/oop_dispatch/oop_dispatch.lua` - Lua metatable inheritance (__index chain)
- `benchmark/cases/oop_dispatch/oop_dispatch.nut` - Squirrel class extends mechanism
- `benchmark/cases/oop_dispatch/oop_dispatch.py` - Python class inheritance
- `benchmark/cases/oop_dispatch/oop_dispatch.js` - JS class extends
- `benchmark/cases/oop_dispatch/oop_dispatch.rs` - Rust dyn Computable trait objects
- `benchmark/cases/object_create/object_create.writ` - Writ pub class Point; new Point{} 1M times
- `benchmark/cases/object_create/object_create.lua` - Lua table literal creation
- `benchmark/cases/object_create/object_create.nut` - Squirrel class with constructor
- `benchmark/cases/object_create/object_create.py` - Python class with __slots__
- `benchmark/cases/object_create/object_create.js` - JS class with constructor
- `benchmark/cases/object_create/object_create.rs` - Rust stack-allocated struct (ceiling reference)
- `writ-compiler/src/emit/body/expr/mod.rs` - IMPL-METHOD fix: intercept method calls on concrete types
- `writ-compiler/src/emit/module_builder.rs` - Add methoddef_token_by_type_and_name lookup
- `writ-compiler/src/emit/collect/contracts.rs` - Remove unused find_method_in_impl import
- `writ-compiler/src/emit/collect/lookup.rs` - Remove unused find_method_in_impl function

## Decisions Made

- Contract/impl dispatch for Writ oop_dispatch uses concrete-type dispatch in separate if-branches (not dynamic base pointer). This is because Writ lacks `else if` chaining and base-type variable polymorphism in the current compiler. Algorithmic equivalence is maintained: same 4 types, same cycling pattern, same expected sum.
- Python object_create uses `__slots__` for fair struct-like comparison with other languages. This reduces per-object memory overhead to match the fixed-field semantics of Writ/Rust/JS classes.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed impl method calls compiling to CALL_INDIRECT instead of direct CALL**
- **Found during:** Task 1 (oop_dispatch Writ smoke test)
- **Issue:** `f.compute()` where `compute` is defined via `impl Computable for Foo` was compiling to `GET_FIELD r3, r0, 0; CALL_INDIRECT r4, r3, 0` — CALL_INDIRECT crashed with "not a delegate" because `r3` contained the integer value of field `val` (42), not a delegate
- **Fix:** Added IMPL-METHOD interception in `emit/body/expr/mod.rs`: when `callee_def_id` is None and callee is `Field { receiver with Struct/Class type, field: method_name }`, look up the MethodDef token via `builder.methoddef_token_by_type_and_name(receiver_def_id, field)` and emit a direct CALL. Added `methoddef_token_by_type_and_name` to `module_builder.rs`.
- **Files modified:** writ-compiler/src/emit/body/expr/mod.rs, writ-compiler/src/emit/module_builder.rs
- **Verification:** Smoke test `f.compute()` outputs `[INFO] 42`; oop_dispatch outputs `[INFO] 250000`; 39 golden tests pass
- **Committed in:** 2f71be4 (Task 1 commit, also included cleanup of lookup.rs)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Essential fix — the entire oop_dispatch.writ benchmark would not compile without it. No scope creep.

## Issues Encountered

The impl method dispatch bug had multiple root causes that were fixed in sequence (some in the prior session, completed here):

1. `collect_impl` used `method_entry.name` = "impl#N" (the impl block's synthetic name from the resolver) to search the AST for method declarations — always failing, so no MethodDef rows were emitted. Fixed by iterating AST fn_decls by index in parallel with the `methods` vec.
2. MetadataToken was not stripped before use as a row index in `exec_new` and `get_type_field_count` — the table bits (0x02000000 for TypeDef) caused out-of-range indexing.
3. `check_generic_call` did not handle `expr.into<T>()` when callee was a MemberAccess — caused "type int has no field into" during string interpolation.
4. S2i/S2f/S2b instruction variants were missing from the Instruction enum.

Items 1-4 were all committed in prior session commit `da68b0b`. The final piece fixed here was the emitter-level IMPL-METHOD interception.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All 5 benchmark suites (fib, sieve, string_concat, array_sort, hash_map, oop_dispatch, object_create) are ready for Docker container execution
- bench_runner.sh already handles missing .writ files (null entries in raw.json) so hash_map.writ absence is graceful
- Phase 73 plan 03 is the last plan in phase 73; milestone v7.0 benchmark suite is complete

## Self-Check: PASSED

- All 12 benchmark files exist in correct directories
- SUMMARY.md created at .planning/phases/73-remaining-benchmark-categories/73-03-SUMMARY.md
- Commits 2f71be4 and 455b4ff verified in git log
- Writ oop_dispatch outputs [INFO] 250000 (verified)
- Writ object_create outputs [INFO] 499999500000 (verified)
- 39 golden tests pass after compiler changes

---
*Phase: 73-remaining-benchmark-categories*
*Completed: 2026-03-20*
