---
phase: 103-writ-runtime-reflectionindex-and-intrinsic-dispatch
plan: 01
subsystem: runtime
tags: [reflection, gc, dispatch, intrinsics, lazy-cache, type-objects]

# Dependency graph
requires:
  - phase: 102-writ-runtime-virtual-module-reflection-types
    provides: virtual module with 6 reflection TypeDefs (Type=index 9), Reflectable contract, 4 primitive GetType intrinsics with Value::Int(1) stubs

provides:
  - ReflectionIndex struct with lazy FxHashMap caches for Type/FieldInfo/MethodInfo/ParameterInfo/AttributeInfo/ContractInfo
  - get_or_alloc_type(): allocates Type class heap objects on demand from module metadata
  - get_or_alloc_primitive_type(): allocates Type heap objects for Int/Float/Bool/String
  - collect_roots(): registers all cached HeapRefs as permanent GC roots
  - TypeOf opcode returns Value::Ref(href) pointing to real Type heap object (replaces Value::Int(1) stub)
  - IntGetType/FloatGetType/BoolGetType/StringGetType intrinsics return Value::Ref(href) (replace Value::Int(1) stubs)
  - reflection field threaded through Runtime, Scheduler, ExecContext, execute_batch, execute_one

affects:
  - 103-02 (next plan: full reflection intrinsic arms)
  - 104 (typeof() lowering in compiler)
  - 105 (Reflectable auto-impl)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Lazy singleton heap objects: allocate on first access, cache HeapRef, register as GC root"
    - "Synthetic cache key (usize::MAX, ordinal) for primitive types with no module TypeDef"
    - "Thread reflection through entire call chain: Runtime -> Scheduler -> execute_batch -> execute_one -> ExecContext"

key-files:
  created:
    - writ-runtime/src/reflection.rs
  modified:
    - writ-runtime/src/lib.rs
    - writ-runtime/src/runtime.rs
    - writ-runtime/src/scheduler.rs
    - writ-runtime/src/dispatch/mod.rs
    - writ-runtime/src/dispatch/intrinsics.rs
    - writ-runtime/src/dispatch/calls.rs

key-decisions:
  - "Primitive type cache key uses (usize::MAX, ordinal) as synthetic module_idx to avoid special-casing in get_or_alloc_type"
  - "reflection parameter placed after pool and before limit in execute_batch/execute_one for readability"
  - "cancel_task_tree and execute_crash also take reflection since they call execute_defer_handler which calls execute_one"

patterns-established:
  - "Lazy GC-root pattern: allocate on first access, cache permanently, register all cached HeapRefs in collect_roots()"
  - "Type kind mapping: 0=struct, 1=enum, 2=entity, 3=component, 4=class — matches TypeDefKind discriminants"

requirements-completed: [RT-01, RT-02, RT-03]

# Metrics
duration: 18min
completed: 2026-03-28
---

# Phase 103 Plan 01: ReflectionIndex lazy cache and intrinsic dispatch Summary

**Lazy singleton ReflectionIndex replacing Value::Int(1) stubs with real Type heap objects — TypeOf opcode and all 4 primitive GetType intrinsics now return Value::Ref(href)**

## Performance

- **Duration:** 18 min
- **Started:** 2026-03-28T10:52:00Z
- **Completed:** 2026-03-28T11:10:07Z
- **Tasks:** 2
- **Files modified:** 7 (1 created, 6 modified)

## Accomplishments

- Created `ReflectionIndex` with 6 lazy FxHashMap caches, permanent GC root registration, and lazy alloc from module metadata
- Replaced TypeOf opcode stub: now calls `get_or_alloc_type()` and stores `Value::Ref(href)` instead of `Value::Int(1)`
- Replaced combined GetType stub arm with 4 separate arms calling `get_or_alloc_primitive_type()` for Int/Float/Bool/String
- Threaded `reflection: &mut ReflectionIndex` through the entire execution pipeline (7 function signatures updated)
- All 90 existing tests pass with zero regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Create ReflectionIndex module and wire into Runtime and GC roots** - `0395c6c` (feat)
2. **Task 2: Wire ReflectionIndex into ExecContext, scheduler, and dispatch — replace TypeOf and primitive GetType stubs** - `279b683` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `writ-runtime/src/reflection.rs` - New: ReflectionIndex struct with 6 lazy caches, get_or_alloc_type(), get_or_alloc_primitive_type(), collect_roots(), unit tests
- `writ-runtime/src/lib.rs` - Added `pub mod reflection` and `pub use reflection::ReflectionIndex`
- `writ-runtime/src/runtime.rs` - Added `reflection: ReflectionIndex` field, wired collect_roots and 3 run_one_task call sites
- `writ-runtime/src/scheduler.rs` - Updated run_one_task and cancel_task_tree signatures; forwarded reflection to execute_batch and execute_crash
- `writ-runtime/src/dispatch/mod.rs` - Added reflection field to ExecContext; updated execute_one/execute_batch/execute_ret/execute_defer_handler/execute_crash; replaced TypeOf stub
- `writ-runtime/src/dispatch/intrinsics.rs` - Replaced combined GetType stub with 4 separate arms calling get_or_alloc_primitive_type()
- `writ-runtime/src/dispatch/calls.rs` - Updated try_speaker_dispatch signature and execute_defer_handler call site

## Decisions Made

- Synthetic cache key `(usize::MAX, ordinal)` for primitives: avoids special-casing in get_or_alloc_type(), ordinals are Int=0, Float=1, Bool=2, String=3
- `reflection` parameter placed after `pool` in function signatures to group execution-context parameters together
- `cancel_task_tree` and `execute_crash` also received the parameter because they call down into `execute_defer_handler` which calls `execute_one`

## Deviations from Plan

None - plan executed exactly as written. The propagation of `reflection` through `execute_defer_handler` and `execute_crash` was implied by the plan's note about the `execute_batch`/`execute_one` chain, and the `calls.rs` tail-call defer handler was a natural extension.

## Issues Encountered

None.

## Next Phase Readiness

- Phase 103-02 can now build on ExecContext.reflection for all remaining reflection intrinsic arms (FieldInfo, MethodInfo, TypeName, TypeKind, TypeFields, TypeMethods, etc.)
- GC correctness verified: cached HeapRefs are registered as permanent roots in collect_roots()
- Lazy allocation semantics preserved: no Type objects allocated at domain load time

---
*Phase: 103-writ-runtime-reflectionindex-and-intrinsic-dispatch*
*Completed: 2026-03-28*
