---
phase: 46-structs-as-value-types-design-discussion
plan: "01"
subsystem: language-spec
tags: [spec, design-record, value-types, structs, classes, v4.0]
dependency_graph:
  requires: []
  provides: [section-8.4-design-record, v4.0-milestone-entry]
  affects: [language-spec/spec/09_8_structs.md, language-spec/spec/38_2_9_memory_model.md, language-spec/spec/69_b_il_decision_log.md, .planning/MILESTONES.md]
tech_stack:
  added: []
  patterns: [spec-amendment, forward-design-record, inline-annotation]
key_files:
  created: []
  modified:
    - language-spec/spec/09_8_structs.md
    - language-spec/spec/38_2_9_memory_model.md
    - language-spec/spec/69_b_il_decision_log.md
    - .planning/MILESTONES.md
decisions:
  - "struct/class split adopted (YES): struct=value type, class=reference type, entity=specialized class"
  - "Structural equality auto-derived for value-type structs (field-by-field, matching enum behavior)"
  - "Passing semantics: always by-copy for value-type structs (mut self mutates local copy)"
  - "Compiler-emitted field comparisons recommended over STRUCT_EQ instruction (option a)"
  - "Single abstract struct register recommended over register flattening (option a, mirrors enum handling)"
  - "format_version bumps to 3 for v4.0; no backward compatibility (pre-1.0, acceptable)"
metrics:
  duration: "8 minutes"
  completed_date: "2026-03-07"
  tasks_completed: 2
  files_modified: 4
---

# Phase 46 Plan 01: Structs as Value Types -- Design Record Summary

**One-liner:** Struct/class split design record with all seven IL changes, GC tracing mechanism, and v4.0 milestone scope in language spec section 8.4.

## What Was Built

Two tasks executed, documenting the YES decision to adopt a C# model struct/class split in a future v4.0 milestone.

### Task 1: Write the design record and update spec cross-references

Added section 8.4 "Value-Type Structs and Classes -- Design Record (v4.0)" to `language-spec/spec/09_8_structs.md` with seven subsections:

- **8.4.1 Decision** -- YES with struct=value, class=reference, entity=specialized class
- **8.4.2 New Type Semantics** -- table: struct/class/entity with Storage, Assignment, GC Traced, Lifecycle Hooks columns
- **8.4.3 Value-Type Struct Semantics** -- shallow copy, structural equality, passing semantics, no lifecycle hooks, no size limit, recursive structs illegal
- **8.4.4 Motivating Examples** -- Vec2, Vec3, Color, Rect; assignment semantics contrast; move_entity functional style
- **8.4.5 IL Changes Required (v4.0)** -- seven changes: TypeDef.kind extension, NEW kind-dependent behavior, MOV multi-word copy, GC tracing mechanism, BOX/UNBOX scope, structural equality (compiler-emitted), format_version=3
- **8.4.6 GC Implications** -- register tracing (same as enum payload tracing), closure capture env must use class, no finalization on value structs
- **8.4.7 Migration Notes** -- phased implementation plan, spec sections requiring updates, cross-references

Added three v4.0 annotation notes to `language-spec/spec/38_2_9_memory_model.md`:
- Section 2.9.1: note that Structs row reflects v3.x; v4.0 splits to struct=value / class=reference
- Section 2.9.3: note that compiler-generated closure capture environments are reference types (class) in v4.0
- Section 2.9.8: note that MOV for value-struct registers becomes multi-word copy; NEW becomes kind-dependent

Updated `language-spec/spec/69_b_il_decision_log.md`:
- Structs row: updated to show v3.x (reference) / v4.0 (value) with reference to section 8.4
- Classes row: new row for v4.0 reference type keyword

### Task 2: Add v4.0 milestone entry to MILESTONES.md

Added `## v4.0 Structs as Value Types (Planned)` at the top of `.planning/MILESTONES.md` with:
- Goal statement referencing section 8.4
- Scope boundary across five layers: spec, IL, VM, compiler, tests
- Estimated phase count (5-7), prerequisite (v3.2 complete), breaking change note

## Deviations from Plan

None -- plan executed exactly as written.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 8b1683e | feat(46-01): add section 8.4 value-type structs/classes design record |
| 2 | 88ec87f | feat(46-01): add v4.0 Structs as Value Types milestone entry |

## Verification Results

- `grep "8.4" language-spec/spec/09_8_structs.md` -- 8 matches (section header + all subsections + cross-references)
- `grep "Class" language-spec/spec/69_b_il_decision_log.md` -- 1 match (Classes row)
- `grep "v4.0" language-spec/spec/38_2_9_memory_model.md` -- 5 matches (3 annotation notes + internal references)
- `grep "v4.0" .planning/MILESTONES.md` -- 1 match (milestone heading)
- `git diff --name-only HEAD -- "*.rs" | wc -l` -- 0 (no Rust source files modified)
- `cargo test --workspace` -- all tests pass

## Self-Check: PASSED

Files verified:
- FOUND: language-spec/spec/09_8_structs.md (section 8.4 with 7 subsections)
- FOUND: language-spec/spec/38_2_9_memory_model.md (v4.0 notes in 2.9.1, 2.9.3, 2.9.8)
- FOUND: language-spec/spec/69_b_il_decision_log.md (updated Structs row, new Classes row)
- FOUND: .planning/MILESTONES.md (v4.0 entry at top with scope boundary)

Commits verified:
- FOUND: 8b1683e (Task 1)
- FOUND: 88ec87f (Task 2)
