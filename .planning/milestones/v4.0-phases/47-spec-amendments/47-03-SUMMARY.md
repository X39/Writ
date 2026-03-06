---
phase: 47-spec-amendments
plan: "03"
subsystem: language-spec
tags: [il-spec, instructions, value-types, struct-class-split, decision-log]
dependency_graph:
  requires: [47-01, 47-02]
  provides: [normative-instruction-semantics-for-struct-class-split]
  affects: [phase-49-vm-runtime]
tech_stack:
  added: []
  patterns: [normative-prose, kind-dependent-dispatch, multi-word-copy, boxing-value-types]
key_files:
  created: []
  modified:
    - language-spec/spec/49_3_1_data_movement.md
    - language-spec/spec/56_3_8_object_model.md
    - language-spec/spec/63_3_15_boxing.md
    - language-spec/spec/69_b_il_decision_log.md
decisions:
  - "MOV performs multi-word copy for value-type structs (all fields copied inline, no heap indirection)"
  - "NEW is kind-dependent: kind=0 (struct) initializes value inline, kind=4 (class) allocates on GC heap"
  - "Value-type structs require BOX/UNBOX when passing through generic parameters"
  - "Appendix B decision log is normative present-tense with no version annotations"
  - "Lifecycle hooks (on create/finalize/serialize/deserialize) apply to classes and entities only -- structs have no lifecycle hooks"
  - "Closure mut captures use a shared capture class (not struct)"
metrics:
  duration: "~10 minutes"
  completed_date: "2026-03-12"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 4
---

# Phase 47 Plan 03: IL Instruction Sections and Decision Log Summary

MOV multi-word copy for value-type structs, NEW kind-dependent (inline vs heap), BOX/UNBOX extended to structs, and Appendix B decision log cleaned to normative prose with zero v4.0 annotations.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Update MOV, NEW, and BOX/UNBOX instruction sections | 871552e | 49_3_1_data_movement.md, 56_3_8_object_model.md, 63_3_15_boxing.md |
| 2 | Update Appendix B IL Decision Log | 2cbf5f4 | 69_b_il_decision_log.md |

## What Was Built

### Task 1: Instruction Section Updates

**`49_3_1_data_movement.md` (MOV):** Updated description to distinguish value-type copy (primitives, enums, structs) from reference copy (classes, strings, arrays, entities, delegates). Explicitly calls out multi-word copy for value-type structs.

**`56_3_8_object_model.md` (NEW):** Updated description to be kind-dependent. For structs (kind=0), NEW initializes the value inline in the destination register with no heap allocation. For classes (kind=4), NEW allocates zeroed memory on the GC heap. Added two new construction sequence examples before the existing entity example: `new Vec2 { x: 1.0, y: 2.0 }` (struct, no on_create) and `new Merchant { name: "Tim", gold: 100 }` (class, with on_create hook call). The existing Guard entity example is unchanged.

**`63_3_15_boxing.md` (BOX/UNBOX):** Two targeted changes: (1) Added `structs` to the value types list in the opening sentence. (2) Changed `structs` to `classes` in the reference types closing sentence.

### Task 2: Decision Log Cleanup

**`69_b_il_decision_log.md` (Appendix B):** Five rows updated:

- **Structs row:** Choice changed from "Value types (v4.0)" to "Value types". Rationale rewritten as normative prose: inline storage, copy-on-assign, shallow reference copy, structural equality auto-derived, no lifecycle hooks, recursive value-type structs illegal.
- **Classes row:** Choice changed from "Reference types (v4.0)" to "Reference types". Rationale rewritten: heap-allocated, GC-managed, shared-on-assign, lifecycle hooks supported, class keyword fills reference-type role.
- **Lifecycle hooks row:** Changed "structs and entities" to "classes and entities" (structs no longer have lifecycle hooks).
- **Closure mut captures row:** Choice and rationale both updated from "struct" to "class".
- **Construction syntax row:** Updated from "same syntax for structs and entities" to "same syntax for structs, classes, and entities".

Result: zero occurrences of "v4.0" in the decision log.

## Verification

All plan success criteria met:

- `49_3_1_data_movement.md` MOV description includes "multi-word copy" for structs: PASS
- `56_3_8_object_model.md` NEW is kind-dependent with struct and class construction examples: PASS
- `63_3_15_boxing.md` lists structs under value types requiring boxing, classes under reference types: PASS
- `69_b_il_decision_log.md` has zero occurrences of "v4.0": PASS
- All four files use normative present-tense prose with no version callouts: PASS

## Deviations from Plan

None - plan executed exactly as written.

## Self-Check

Files verified:
- language-spec/spec/49_3_1_data_movement.md: FOUND
- language-spec/spec/56_3_8_object_model.md: FOUND
- language-spec/spec/63_3_15_boxing.md: FOUND
- language-spec/spec/69_b_il_decision_log.md: FOUND

Commits verified:
- 871552e (Task 1): FOUND
- 2cbf5f4 (Task 2): FOUND

## Self-Check: PASSED
