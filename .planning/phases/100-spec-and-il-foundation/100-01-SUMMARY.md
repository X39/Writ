---
phase: 100-spec-and-il-foundation
plan: 01
subsystem: spec
tags: [reflection, spec, language-spec, il-spec, typeof, Reflectable]

# Dependency graph
requires: []
provides:
  - "Section 1.28 Reflection language spec (6 reflection types, typeof/get_type semantics, Reflectable contract, dynamic invocation, BOX/UNBOX boundaries, generic reflection scope)"
  - "Renumbered grammar spec from 1.28 to 1.29"
  - "Renumbered lowering reference from 1.29 to 1.30"
  - "Updated TOC with 1.28 Reflection entries and 2.18.9 Reflection Types"
affects: [100-02, 101-typeof-opcode, 102-reflection-types, 103-reflectable-contract, 104-dynamic-invocation]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Reflection spec pattern: static typeof(expr) vs dynamic get_type() divergence example"
    - "BOX/UNBOX boundary rule at reflection API parameters/return values"

key-files:
  created:
    - "language-spec/spec/28_1_28_reflection.md"
    - "language-spec/spec/29_29_grammar_summary_ebnf.md"
    - "language-spec/spec/30_30_lowering_reference.md"
  modified:
    - "language-spec/spec/01_table_of_contents.md"
    - "language-spec/spec/15_14_dialogue_blocks_dlg.md"
    - "language-spec/spec/28_27_standard_library_builtins.md"

key-decisions:
  - "typeof(expr) is static compile-time query — returns static declared type, baked into IL via TYPEOF opcode"
  - "get_type() is dynamic runtime query via Reflectable contract dispatch — returns actual runtime type"
  - "Reflectable auto-implemented on all user-defined types; primitives use separate intrinsics (IntGetType etc.)"
  - "FieldInfo.set() on let-field crashes task with message 'Reflection write to immutable field {name}'"
  - "BOX/UNBOX at reflection API boundaries — no TyKind::Any introduced"
  - "Type.construct() deferred to v12+"
  - "String-based type lookup (Type.for_name) is explicit anti-feature — serialization injection risk"
  - "type_args() may return empty array for runtime-queried open generic types — documented limitation"

patterns-established:
  - "Reflection scope rule: only pub fields/methods visible; extern types excluded"
  - "Type singletons are GC-permanent roots, lazily allocated on first typeof/get_type call"

requirements-completed: [SPEC-01, SPEC-02, SPEC-03, SPEC-04, SPEC-07, SPEC-08]

# Metrics
duration: 15min
completed: 2026-03-28
---

# Phase 100 Plan 01: Spec and IL Foundation Summary

**Section 1.28 Reflection language spec with 6 reflection types, typeof/get_type divergence semantics, Reflectable auto-impl contract, BOX/UNBOX dynamic invocation boundaries, and section renumbering of grammar (1.29) and lowering (1.30)**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-28T00:00:00Z
- **Completed:** 2026-03-28T00:15:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Created complete `language-spec/spec/28_1_28_reflection.md` with all 8 sub-sections covering all 6 reflection types (Type, FieldInfo, MethodInfo, ParameterInfo, AttributeInfo, ContractInfo)
- Documented the static/dynamic divergence between `typeof(expr)` (compile-time, TYPEOF opcode) and `get_type()` (dynamic, Reflectable contract dispatch) with polymorphic example
- Specified the Reflectable contract as contract index 19 with auto-impl on all user-defined types and separate primitive intrinsics
- Documented BOX/UNBOX coercion rules at all reflection API boundaries without introducing TyKind::Any
- Renamed grammar spec file 29_28 -> 29_29 and lowering reference 30_29 -> 30_30 with updated section numbers
- Updated TOC with 1.28 Reflection section entries (1.28.1-1.28.8) and 2.18.9 Reflection Types

## Task Commits

Each task was committed atomically:

1. **Task 1: Write section 1.28 Reflection spec and rename bumped sections** - `730fcf7` (feat)
2. **Task 2: Update table of contents for section renumbering** - `b192c12` (feat)

## Files Created/Modified

- `language-spec/spec/28_1_28_reflection.md` - New section 1.28 Reflection (complete spec)
- `language-spec/spec/29_29_grammar_summary_ebnf.md` - Renamed from 29_28, heading updated to 1.29, added typeof_expr EBNF production
- `language-spec/spec/30_30_lowering_reference.md` - Renamed from 30_29, all sub-sections updated to 1.30.X
- `language-spec/spec/01_table_of_contents.md` - Inserted 1.28 Reflection entries, renumbered grammar/lowering, added 2.18.9
- `language-spec/spec/15_14_dialogue_blocks_dlg.md` - Updated cross-reference from §1.29.5 to §1.30.5
- `language-spec/spec/28_27_standard_library_builtins.md` - Updated cross-reference from §1.29.1-§1.29.5 to §1.30.1-§1.30.5

## Decisions Made

- `typeof(expr)` is a compile-time query: returns static declared type baked into IL via TYPEOF opcode; expression evaluated for type only, not executed
- `get_type()` is dynamic: dispatched via Reflectable contract (CALL_VIRT), returns concrete runtime type
- Reflectable is contract 19 in writ-runtime virtual module; auto-implemented on all user-defined types; extern types excluded
- Primitives use separate intrinsics (IntGetType, FloatGetType, BoolGetType, StringGetType) registered in section 2.18.9
- `FieldInfo.set()` crashes task with `"Reflection write to immutable field '{field_name}'"` when field is `let`
- `MethodInfo.invoke()` runs on current task stack (not a separate task); cooperative scheduling applies
- All reflection API parameters/returns use Box type; compiler auto-inserts BOX/UNBOX at call sites; no TyKind::Any
- `Type.construct()` deferred to v12+ — calling it crashes with UnsupportedOperation
- `Type.for_name(string)` is an explicit anti-feature — host controls type resolution, string lookup is injection risk
- `type_args()` may return empty array for runtime-queried open generic types — documented limitation, not a bug

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated cross-references in two spec files**
- **Found during:** Task 1 (rename step 4 — search for cross-references)
- **Issue:** `15_14_dialogue_blocks_dlg.md` referenced `§1.29.5` and `28_27_standard_library_builtins.md` referenced `§1.29.1–§1.29.5` — both now point to the renamed 1.30.x sections
- **Fix:** Updated both references to §1.30.5 and §1.30.1–§1.30.5 respectively
- **Files modified:** language-spec/spec/15_14_dialogue_blocks_dlg.md, language-spec/spec/28_27_standard_library_builtins.md
- **Verification:** grep found no remaining §1.28 Grammar or §1.29 Lowering references in non-TOC files
- **Committed in:** 730fcf7 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 — missing cross-reference updates)
**Impact on plan:** Fix necessary for spec consistency; no scope creep.

## Issues Encountered

None — pure documentation phase, no code changes.

## Next Phase Readiness

- Section 1.28 Reflection is the stable written contract all downstream implementation phases (101-108) implement against
- Plan 100-02 can now proceed to write the IL spec sections (TYPEOF opcode, reflection types in writ-runtime §2.18.9, format_version 4)
- No blockers

---
*Phase: 100-spec-and-il-foundation*
*Completed: 2026-03-28*
