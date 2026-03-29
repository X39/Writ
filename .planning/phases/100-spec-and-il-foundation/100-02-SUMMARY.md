---
phase: 100-spec-and-il-foundation
plan: "02"
subsystem: language-spec
tags: [il-spec, reflection, opcode, format-version, writ-runtime]
dependency_graph:
  requires: []
  provides: [TYPEOF-opcode-spec, reflection-types-spec, format-version-4]
  affects: [writ-module, writ-runtime, writ-compiler]
tech_stack:
  added: []
  patterns: [spec-driven-design, opcode-category-partitioning, lazy-singleton-pattern]
key_files:
  created: []
  modified:
    - language-spec/spec/67_4_2_opcode_assignment_table.md
    - language-spec/spec/58_3_10_type_operations.md
    - language-spec/spec/65_4_0_instruction_count_by_category.md
    - language-spec/spec/45_2_16_il_module_format.md
    - language-spec/spec/47_2_18_writ_runtime_module_contents.md
decisions:
  - "TYPEOF assigned opcode 0x0A30 in Reflection sub-range (0x0A30-0x0A3F) of the Type Operations category (0x0A)"
  - "format_version bumped to 4; format_version=3 modules rejected at load time with UnsupportedVersion"
  - "Reflectable contract assigned slot 19 in writ-runtime; auto-generated ImplDef for every user-defined type"
  - "Primitive get_type uses separate intrinsic dispatch (IntGetType etc.) on pseudo-TypeDefs, not Reflectable ImplDef"
  - "AttributeInfo unified with v10.0 ModuleAttributeView — shared AttributeIndex, no duplicate scan paths"
metrics:
  duration_minutes: 2
  completed_date: "2026-03-28"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 5
---

# Phase 100 Plan 02: TypeOf Opcode and Reflection Types Spec Summary

**One-liner:** IL spec updated with TYPEOF opcode at 0x0A30, format_version 4, and all 6 reflection class TypeDefs plus Reflectable contract 19 in section 2.18.9.

## What Was Done

This plan established the complete IL-level binary specification for the TYPEOF instruction and the reflection type system, providing the stable contract that Phases 101-103 implement against.

### Task 1: TypeOf Opcode, Instruction Reference, and Count

- Added `Reflection (0x0A30-0x0A3F)` sub-range to section 4.2 (opcode assignment table) with TYPEOF at 0x0A30, shape RI32
- Documented TYPEOF in section 3.10 with full operand description: `r_dst, type_idx:u32`, 8-byte encoding (`u16(0x0A30) u16(r_dst) u32(type_idx)`)
- Added Reflection category row (1 instruction: TYPEOF) to section 4.0 instruction count table
- Bumped total instruction count from 91 to 92

### Task 2: Format Version 4 and Reflection Types

- Extended format version history in section 2.16.1 with Version 4 entry documenting the TYPEOF opcode addition and UnsupportedVersion rejection for format_version=3 modules
- Added complete section 2.18.9 Reflection Types to the writ-runtime module spec (section 2.18) covering:
  - **Type** — 4 fields (name, namespace, kind, is_generic), 6 intrinsic methods (TypeFields, TypeMethods, TypeAttributes, TypeContracts, TypeImplements, TypeTypeArgs); lazy-allocated singletons, permanent GC roots
  - **FieldInfo** — 3 fields (name, declared_type, is_mutable), 3 intrinsic methods (FieldGet, FieldSet, FieldAttributes); set() on immutable field crashes task
  - **MethodInfo** — 3 fields (name, parameters, return_type), 2 intrinsic methods (MethodInvoke, MethodAttributes); invoke() participates in cooperative scheduling
  - **ParameterInfo** — 2 fields (name, declared_type); data-only
  - **AttributeInfo** — 2 fields (name, args); shares AttributeIndex with ModuleAttributeView (v10.0 integration)
  - **ContractInfo** — 2 fields (name, type); data-only
  - **Reflectable contract** — slot 19, method `fn get_type(self) -> Type`; compiler auto-generates ImplDef for every user-defined type; primitives use IntGetType/FloatGetType/BoolGetType/StringGetType intrinsics on pseudo-TypeDefs

## Deviations from Plan

None — plan executed exactly as written.

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| TYPEOF in Reflection sub-range (0x0A30-0x0A3F) | Leaves Enum range (0x0A20-0x0A2F) fully intact; clean sub-category partition |
| format_version=3 modules rejected (not warned) | Pre-1.0 project; strict rejection matches existing format_version policy (format_version=3 strict from v4.0 decision) |
| Reflectable as contract slot 19 | Follows on from 17 original contracts (§2.18.3) and contract slot numbering; sequential assignment |
| Primitive get_type via separate intrinsics | Primitives use pseudo-TypeDefs (§2.18.4) as anchor; same pattern as other primitive contract impls |
| AttributeInfo unified with ModuleAttributeView | Single attribute scan path; aligns with v10.0 design decision to avoid duplicate attribute scan paths |

## Self-Check: PASSED

Files confirmed present:
- language-spec/spec/67_4_2_opcode_assignment_table.md — TYPEOF and 0x0A30 entries present
- language-spec/spec/58_3_10_type_operations.md — TYPEOF instruction documented
- language-spec/spec/65_4_0_instruction_count_by_category.md — Reflection row, Total = 92
- language-spec/spec/45_2_16_il_module_format.md — Version 4 with TYPEOF, 0x0A30, UnsupportedVersion
- language-spec/spec/47_2_18_writ_runtime_module_contents.md — section 2.18.9, all 6 TypeDefs, Reflectable contract 19

Commits confirmed:
- f4d8234: feat(100-02): add TYPEOF opcode 0x0A30 and update instruction count to 92
- ae33977: feat(100-02): add format_version 4 and section 2.18.9 Reflection Types
