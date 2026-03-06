---
phase: 47-spec-amendments
plan: "01"
subsystem: language-spec
tags: [spec, structs, classes, value-types, reference-types, grammar, ebnf]
dependency_graph:
  requires: []
  provides: [Section-8-value-type-structs, Section-9-reference-type-classes, class-decl-grammar]
  affects: [language-spec/spec/09_8_structs.md, language-spec/spec/10_9_classes.md, language-spec/spec/28_27_grammar_summary_ebnf.md]
tech_stack:
  added: []
  patterns: [value-type-semantics, reference-type-semantics, EBNF-grammar]
key_files:
  created:
    - language-spec/spec/10_9_classes.md
  modified:
    - language-spec/spec/09_8_structs.md
    - language-spec/spec/28_27_grammar_summary_ebnf.md
decisions:
  - "Section 8 rewritten as pure value-type spec: copy-on-assign, no heap, no lifecycle hooks, structural equality"
  - "Section 9 (Classes) created as reference-type spec: heap allocation, lifecycle hooks, shared-on-assign"
  - "class_decl is a separate EBNF production from struct_decl (not shared)"
  - "struct_member no longer includes on_decl — value structs have no lifecycle hooks"
  - "class_member includes on_decl — reference classes have lifecycle hooks"
metrics:
  duration: ~20 minutes
  completed: 2026-03-12
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 2
---

# Phase 47 Plan 01: Structs as Value Types — Spec Section 8/9 Summary

Section 8 rewritten for value-type struct semantics and Section 9 created for reference-type class semantics, with class_decl EBNF grammar production added.

## What Was Built

### Task 1: Rewrite Section 8 Structs and Create Section 9 Classes

**`language-spec/spec/09_8_structs.md`** — Complete rewrite for value-type semantics:
- Opening: structs are value types with inline storage and copy-on-assign
- Opening example changed from `Merchant` (reference pattern) to `Vec2` (natural value type)
- §8.1 Construction: describes inline initialization, no heap allocation, contrasts with Section 9 classes
- §8.2 Shallow Copy Semantics: field-by-field copy, reference fields copy pointer only
- §8.3 Structural Equality: auto-derived field-by-field equality, classes require explicit Eq
- §8.4 Passing Semantics: always passed by copy, `mut self` mutates local copy
- §8.5 Recursive Structs: illegal (infinite size), compiler error with cycle named, suggests class
- §8.6 Construction Sequence (IL): inline NEW with no CALL __on_create step
- Removed: old §8.2 Lifecycle Hooks, old §8.4 Design Record (entire forward-looking section gone)
- No v4.0 callouts, no version history notes — clean normative prose

**`language-spec/spec/10_9_classes.md`** — New file for reference-type class semantics:
- Opening: classes are reference types, heap-allocated, GC-managed, shared-on-assign
- `Merchant` example migrated here using `class` keyword
- §9.1 Construction: heap allocation via `new`, reference copy semantics
- §9.2 Lifecycle Hooks: on create/finalize/serialize/deserialize, NativeConnection example rewritten as class
- Full hooks table, implicit on create, finalize semantics, hook failure semantics
- §9.3 Construction Sequence (IL): heap-allocating NEW followed by CALL __on_create
- Note: entities are specialized classes, see Section 14

### Task 2: Add class_decl to EBNF Grammar

**`language-spec/spec/28_27_grammar_summary_ebnf.md`** — Four targeted updates:
- `struct_member` updated to remove `on_decl` (value structs have no lifecycle hooks)
- New `class_decl` and `class_member` productions added after `struct_member`
- `class_member` includes `on_decl` (reference classes have lifecycle hooks)
- `declaration` production updated to include `class_decl`
- `extern_decl` updated to include `class_decl` alongside `struct_decl`
- `new_expr` comment updated to note it works for both structs (inline) and classes (heap)

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | `8f69ae0` | feat(47-01): rewrite Section 8 Structs and create Section 9 Classes |
| 2 | `afe98bb` | feat(47-01): add class_decl EBNF production and update grammar rules |

## Verification

- `09_8_structs.md`: 2 "value type" hits, 0 "Design Record" hits, 0 "v4.0" hits — clean value-type spec
- `10_9_classes.md`: 10 "class" hits, lifecycle hooks fully described — clean reference-type spec
- `28_27_grammar_summary_ebnf.md`: 3 `class_decl` hits (production, declaration, extern_decl), `struct_member` has no `on_decl`, `class_member` has `on_decl`
- No version history notes or "v4.0 Change" callouts in any file

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

All files confirmed present. Both task commits verified in git history.
