---
phase: 47-spec-amendments
plan: "04"
subsystem: language-spec
tags: [spec, renumbering, classes, toc, cross-references]
dependency_graph:
  requires: [47-01, 47-02, 47-03]
  provides: [internally-consistent-spec-numbering]
  affects: [language-spec/spec/]
tech_stack:
  added: []
  patterns: [1.N section prefix format, splatted spec files]
key_files:
  created: []
  modified:
    - language-spec/spec/01_table_of_contents.md
    - language-spec/spec/02_1_overview_design_philosophy.md
    - language-spec/spec/03_2_project_configuration_writ_toml.md
    - language-spec/spec/04_3_naming_conventions_style_guide.md
    - language-spec/spec/05_4_lexical_structure.md
    - language-spec/spec/06_5_type_system.md
    - language-spec/spec/07_6_primitive_types.md
    - language-spec/spec/08_7_variables_constants.md
    - language-spec/spec/09_8_structs.md
    - language-spec/spec/10_9_classes.md
    - language-spec/spec/11_10_enums.md (renamed from 10_9_enums.md)
    - language-spec/spec/12_11_contracts.md (renamed from 11_10_contracts.md)
    - language-spec/spec/13_12_generics.md (renamed from 12_11_generics.md)
    - language-spec/spec/14_13_functions_fn.md (renamed from 13_12_functions_fn.md)
    - language-spec/spec/15_14_dialogue_blocks_dlg.md (renamed from 14_13_dialogue_blocks_dlg.md)
    - language-spec/spec/16_15_entities.md (renamed from 15_14_entities.md)
    - language-spec/spec/17_16_components.md (renamed from 16_15_components.md)
    - language-spec/spec/18_17_attributes.md (renamed from 17_16_attributes.md)
    - language-spec/spec/19_18_operators_overloading.md (renamed from 18_17_operators_overloading.md)
    - language-spec/spec/20_19_error_handling.md (renamed from 19_18_error_handling.md)
    - language-spec/spec/21_20_nullability_optionals.md (renamed from 20_19_nullability_optionals.md)
    - language-spec/spec/22_21_concurrency.md (renamed from 21_20_concurrency.md)
    - language-spec/spec/23_22_scoping_rules.md (renamed from 22_21_scoping_rules.md)
    - language-spec/spec/24_23_globals_atomic_access.md (renamed from 23_22_globals_atomic_access.md)
    - language-spec/spec/25_24_modules_namespaces.md (renamed from 24_23_modules_namespaces.md)
    - language-spec/spec/26_25_external_declarations.md (renamed from 25_24_external_declarations.md)
    - language-spec/spec/27_26_localization.md (renamed from 26_25_localization.md)
    - language-spec/spec/28_27_standard_library_builtins.md (renamed from 27_26_standard_library_builtins.md)
    - language-spec/spec/29_28_grammar_summary_ebnf.md (renamed from 28_27_grammar_summary_ebnf.md)
    - language-spec/spec/30_29_lowering_reference.md (renamed from 29_28_lowering_reference.md)
    - language-spec/spec/43_2_14_runtime_host_interface.md
    - language-spec/spec/47_2_18_writ_runtime_module_contents.md
    - language-spec/spec/68_a_open_questions.md
decisions:
  - "All language spec section headings use 1.N prefix format (e.g., 1.8 Structs, 1.9 Classes, 1.10 Enums)"
  - "Splatted filenames shifted +1 (10_9_enums through 29_28_lowering) to accommodate Classes at position 9"
  - "Type Categories table has separate Structs (value type) and Classes (reference type) rows"
  - "Entities section uses 'classes' (not 'structs') for lowering context — entities are specialized classes"
metrics:
  duration: "~45 minutes"
  completed_date: "2026-03-12"
  tasks: 2
  files: 33
---

# Phase 47 Plan 04: Spec Section Renumbering Summary

Mechanical renumbering pass to make the spec internally consistent after the Classes section insertion from Plans 01-03. All 21 shifted splatted files renamed, all section headings prefixed with 1.N, TOC regenerated with 29 language spec sections, Type Categories table updated, cross-references fixed throughout.

## What Was Done

### Task 1: File Renames and Section Heading Prefix Fixes

Renamed 20 language spec splatted files from `10_9_enums.md` through `29_28_lowering_reference.md` to `11_10_enums.md` through `30_29_lowering_reference.md` using `git mv` for rename tracking.

Updated all section headings in all 28 language spec files (02_ through 30_) to use the `1.N` prefix format:
- `## 8. Structs` became `## 1.8 Structs`
- `## 9. Enums` (now section 10) became `## 1.10 Enums`
- All subsections updated to match (e.g., `### 9.1 Builtin Enums` became `### 1.10.1 Builtin Enums`)
- Deep subsections updated (e.g., `#### 9.3.4 Enum Destructuring` became `#### 1.10.3.4 Enum Destructuring`)

Updated cross-references in IL spec files:
- `§13.9` (dialogue suspension) -> `§1.14.9`
- `§26.4` (inbuilt calls) -> `§1.27.4`
- `§28.5` (lowering) -> `§1.29.5`
- `§10.1` (builtin contracts) -> `§1.11.1`
- `§14.4` (singleton entities) -> `§1.15.4`

Updated entities section to say "classes" instead of "structs" in the lowering context, since entities are specialized classes per the v4.0 redesign.

### Task 2: TOC, Type Categories Table, and Cross-References

Regenerated `01_table_of_contents.md` with all 29 language spec sections (was 28), inserting Classes at position 1.9 with its three subsections (1.9.1 Construction, 1.9.2 Lifecycle Hooks, 1.9.3 Construction Sequence). All subsection anchors updated to match the new 1.N format.

Updated `06_5_type_system.md` Section 1.5.1 Type Categories table:
- Split single Structs row into separate Structs (value types) and Classes (reference types) rows
- Updated Entities row description to "Game objects (specialized classes) with components and lifecycle"

Fixed all inline cross-references across language spec files:
- `Section 10.2` -> `Section 1.11.2` (Into<T>)
- `Section 10.3` -> `Section 1.11.3` (Iterable<T>)
- `[Section 13]` -> `[Section 1.14]` (Dialogue Blocks)
- `[Section 16.4]` -> `[Section 1.17.4]` (Conditional Compilation)
- `[Section 24.2]` -> `[Section 1.25.2]` (Library Imports)
- `Section 23` -> `Section 1.24` (Modules & Namespaces)
- etc.

## Deviations from Plan

None - plan executed exactly as written. The only deviation was discovering that `entities.md` had two additional struct-to-class wording fixes beyond the opening paragraph ("entity struct" -> "entity class" and "Unlike structs" -> "Unlike classes") — these were Rule 2 fixes for correctness (entities are classes in the v4.0 model).

## Self-Check: PASSED

- FOUND: language-spec/spec/11_10_enums.md
- FOUND: language-spec/spec/30_29_lowering_reference.md
- FOUND: language-spec/spec/10_9_classes.md
- FOUND: language-spec/spec/01_table_of_contents.md
- FOUND: .planning/phases/47-spec-amendments/47-04-SUMMARY.md
- FOUND commit: c5cdf16 feat(47-04): rename splatted spec files and fix section heading prefixes
- FOUND commit: 094c2e3 feat(47-04): update TOC, type categories table, and cross-references
