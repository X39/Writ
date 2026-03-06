---
phase: 47-spec-amendments
plan: 02
subsystem: language-spec
tags: [spec, il, memory-model, construction, type-system, module-format, struct-class-split]
dependency_graph:
  requires: []
  provides: [normative-memory-model, normative-construction-model, normative-type-system, normative-module-format]
  affects: [47-03-PLAN.md, phase-48-vm, phase-49-compiler]
tech_stack:
  added: []
  patterns: [struct-value-type, class-reference-type, kind-dependent-NEW]
key_files:
  created: []
  modified:
    - language-spec/spec/38_2_9_memory_model.md
    - language-spec/spec/40_2_11_construction_model.md
    - language-spec/spec/44_2_15_il_type_system.md
    - language-spec/spec/45_2_16_il_module_format.md
decisions:
  - "Structs are value types (inline, copy-on-assign); classes are reference types (GC heap) as normative spec truth"
  - "TypeDef.kind=4 reserved for class; format_version=3 documents this as a binary-incompatible change"
  - "Closure capture environments use class keyword (always reference types) in normative prose"
  - "Lifecycle hooks on classes and entities only; structs have no lifecycle hooks"
metrics:
  duration: "3 minutes"
  completed_date: "2026-03-12"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 4
---

# Phase 47 Plan 02: IL Foundation Sections Summary

**One-liner:** Promoted struct=value/class=reference split from design record to normative prose across memory model, construction model, type system, and module format sections.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Update memory model and construction model | 87c972c | 38_2_9_memory_model.md, 40_2_11_construction_model.md |
| 2 | Update IL type system and module format | 4a76da6 | 44_2_15_il_type_system.md, 45_2_16_il_module_format.md |

## What Was Done

### Task 1: Memory Model and Construction Model

**`38_2_9_memory_model.md` (Section 2.9 Memory Model):**
- Section 2.9.1: Replaced old table (Structs=Reference) with v4.0 table (Structs=Value, Classes=Reference added)
- Added "Struct value semantics" paragraph analogous to the existing "Enum value semantics" paragraph
- Section 2.9.2: Updated Merchant example to use `new Merchant { ... }` syntax with class comment
- Section 2.9.3: Removed v4.0 Note blockquote; capture env now uses `class __closure_env_0 { ... }` normatively
- Section 2.9.8: Replaced v4.0 Note bullet with normative MOV and NEW descriptions (kind-dependent behavior)

**`40_2_11_construction_model.md` (Section 2.11 Construction Model):**
- Opening syntax paragraph updated from "structs and entities" to "structs, classes, and entities"
- Default field values paragraph now distinguishes struct (inline) vs class/entity (zeroed heap alloc)
- Replaced single "Struct construction" section with two sections: struct (value type, no lifecycle hooks) and class (reference type, with on_create hook)
- Entity construction section kept unchanged
- No constructors paragraph updated to mention structs, classes, and entities
- Lifecycle hooks paragraph updated: classes+entities have hooks, structs explicitly have none

### Task 2: IL Type System and Module Format

**`44_2_15_il_type_system.md` (Section 2.15 IL Type System):**
- Section 2.15.1: structs moved from reference types to value types; classes added to reference types
- Value types bullet: added structs with explanation of inline multi-word register storage
- Reference types bullet: structs removed, classes added
- Section 2.15.3 TypeRef encoding: kind 0x10 now includes "class" in the named-type description
- Design notes: Single TypeDef table paragraph now lists structs, classes, enums, entities, components

**`45_2_16_il_module_format.md` (Section 2.16 Module Format):**
- Section 2.16.1: Format version history extended with Version 3 (TypeDef.kind=4 class, kind=0 struct=value type)
- Section 2.16.5 TypeDef table row: Purpose column notes kind distinguishes struct/class/enum/entity/component
- Section 2.16.5 TypeDef.kind line: added kind=4 (class reference type), clarified kind=0 (struct value type)
- Section 2.16.6 register type table paragraph: added sentence about value-type struct multi-word copy and GC tracing

## Deviations from Plan

None -- plan executed exactly as written.

## Verification Results

- Zero "v4.0" callouts across all four files (verified via grep -c)
- Memory model table: Structs=Value, Classes=Reference row added
- Construction model: kind-dependent struct (inline) vs class (heap) sections present
- Register model: structs listed under value types
- TypeDef.kind: includes kind=4 for class
- Format version history: Version 3 present

## Self-Check: PASSED

Files modified confirmed:
- language-spec/spec/38_2_9_memory_model.md -- exists and updated
- language-spec/spec/40_2_11_construction_model.md -- exists and updated
- language-spec/spec/44_2_15_il_type_system.md -- exists and updated
- language-spec/spec/45_2_16_il_module_format.md -- exists and updated

Commits confirmed:
- 87c972c -- Task 1 (memory model + construction model)
- 4a76da6 -- Task 2 (type system + module format)
