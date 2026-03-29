---
phase: 107-dynamic-invocation
plan: 02
subsystem: testing
tags: [reflection, dynamic-dispatch, intrinsics, writ-runtime, integration-tests, field-write, method-invoke]

requires:
  - phase: 107-dynamic-invocation
    provides: FieldInfo.set() and MethodInfo.invoke() intrinsics (Plan 01)

provides:
  - 6 integration tests covering DYN-01 (FieldInfo.set), DYN-02 (MethodInfo.invoke), DYN-04 (cooperative scheduling)
  - test_field_info_set_mut_field: proves mutable field write succeeds and reads back correctly
  - test_field_info_set_readonly_crashes: proves let-field write crashes with "Reflection write to immutable field"
  - test_field_info_set_wrong_instance_type_crashes: proves non-struct instance crashes with descriptive message
  - test_method_info_invoke_executes_method: proves invoke pushes CallFrame, callee writes field=100, reads back via GetField
  - test_method_info_invoke_wrong_argc_crashes: proves arg count mismatch crashes with "MethodInfo.invoke: expected"
  - test_method_info_invoke_cooperative_scheduling: proves invoked method is scheduler-driven (preempted by Instructions(3) limit)
  - Bug fix: resolve_intrinsic_id missing FieldInfoSet and MethodInfoInvoke entries (critical — without this, all Phase 107 intrinsics were broken)

affects: [future-reflection-callers, phase-108+]

tech-stack:
  added: []
  patterns:
    - "Integration test pattern: add_type_ref('writ-runtime', '1.0.0') + contract name → contract_idx for CALL_VIRT"
    - "Array-of-args pattern for MethodInfo.invoke: NewArray + ArrayAdd (param_count=0 in builder, so empty array for zero-param methods)"
    - "Preemption verification pattern: tick(Instructions(N)) then assert Ready or Running (not Completed)"

key-files:
  created: []
  modified:
    - writ-runtime/tests/reflection_tests.rs
    - writ-runtime/src/domain_dispatch.rs

key-decisions:
  - "Tests use param_count=0 (builder limitation) — target method takes only self, empty args array passed to invoke. Wrong-argc test passes 1-element array to 0-param method."
  - "Cooperative scheduling test asserts TaskState::Ready or Running (not Completed) after Instructions(3) tick — Ready is the preempted state in this scheduler"
  - "Bug fix for resolve_intrinsic_id folded into Task 1 commit — it was the root cause of the panic discovered immediately when running the first test"

patterns-established:
  - "Verify preemption with: tick(Instructions(N)) then assert state != Completed"
  - "Build args array for MethodInfo.invoke with NewArray + ArrayAdd elements"

requirements-completed: [DYN-01, DYN-02, DYN-04]

duration: 20min
completed: 2026-03-28
---

# Phase 107 Plan 02: Dynamic Invocation Integration Tests Summary

**6 integration tests proving FieldInfo.set() mutability enforcement and MethodInfo.invoke() cooperative dispatch — plus critical bug fix wiring FieldInfoSet/MethodInfoInvoke to the intrinsic dispatch resolver**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-03-28T17:20:00Z
- **Completed:** 2026-03-28T17:42:37Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Added 6 integration tests covering all DYN-01, DYN-02, DYN-04 requirements
- Fixed critical bug: `resolve_intrinsic_id` in `domain_dispatch.rs` was missing `FieldInfoSet` and `MethodInfoInvoke` entries, causing CALL_VIRT to resolve them as direct frame-push calls instead of intrinsics; discovered when the first test panicked with "slice::get_unchecked_mut requires that the index is within the slice" at calls.rs:102
- Full workspace test suite green: 0 failures across all crates

## Task Commits

Both tasks were committed together since the deviation fix was discovered during Task 1 and the MethodInfo.invoke tests were developed in the same implementation pass:

1. **Tasks 1+2 + deviation fix** - `0b8748d` (test)

## Files Created/Modified

- `writ-runtime/tests/reflection_tests.rs` — Added 6 new test functions (lines 900-1459); file grew from 898 to 1459 lines
- `writ-runtime/src/domain_dispatch.rs` — Added `("FieldInfo", "fieldinfo_set")` and `("MethodInfo", "methodinfo_invoke")` to `resolve_intrinsic_id` match arms

## Decisions Made

- `param_count=0` in ModuleBuilder builder API (pre-existing limitation). Tests work within this constraint: target methods take only self (no explicit args), and the wrong-argc test passes a 1-element array to a 0-param method to trigger the crash path.
- Cooperative scheduling test uses `Ready` state check (not `Running`) — the scheduler puts preempted tasks back to `Ready` state (ready for next tick), not `Running`. Test asserts `state != Completed` to prove scheduler-driven dispatch.
- Deviation fix committed with Task 1 since it was the blocking root cause discovered immediately during first test run.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] resolve_intrinsic_id missing FieldInfoSet and MethodInfoInvoke entries**
- **Found during:** Task 1 (first test run)
- **Issue:** `writ-runtime/src/domain_dispatch.rs` `resolve_intrinsic_id()` function mapped `(type_name, method_name)` pairs to `IntrinsicId` variants, but `("FieldInfo", "fieldinfo_set")` and `("MethodInfo", "methodinfo_invoke")` were never added. When CALL_VIRT resolved these contracts, `resolve_intrinsic_id` returned `None`, falling through to `DispatchTarget::Method { method_idx }` (direct frame dispatch). The virtual module's intrinsic methods have very few registers, so copying `argc=3` registers into a 1-register frame caused an unsafe out-of-bounds panic.
- **Fix:** Added two match arms to `resolve_intrinsic_id`: `("FieldInfo", "fieldinfo_set") => Some(IntrinsicId::FieldInfoSet)` and `("MethodInfo", "methodinfo_invoke") => Some(IntrinsicId::MethodInfoInvoke)`
- **Files modified:** `writ-runtime/src/domain_dispatch.rs`
- **Verification:** All 18 reflection_tests pass; full workspace passes
- **Committed in:** `0b8748d` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug)
**Impact on plan:** Critical correctness fix — without it, no Phase 107 intrinsic would have been callable. No scope creep.

## Issues Encountered

- Initial cooperative scheduling test asserted `TaskState::Running` but actual state after preemption is `TaskState::Ready` (scheduler puts preempted tasks back to ready queue). Fixed by relaxing assertion to `Ready || Running` and asserting `!= Completed`.

## Next Phase Readiness

- All 6 DYN requirements tests pass; DYN-01, DYN-02, DYN-04 are fully verified
- DYN-03 (Type.construct) remains explicitly deferred to v12+
- Phase 107 complete — both plans done

---
*Phase: 107-dynamic-invocation*
*Completed: 2026-03-28*
