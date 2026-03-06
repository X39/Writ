---
phase: 47-spec-amendments
verified: 2026-03-12T18:45:00Z
status: passed
score: 11/11 must-haves verified
re_verification: false
---

# Phase 47: Spec Amendments Verification Report

**Phase Goal:** The language specification accurately describes the struct/class split — `struct` as value type, `class` as reference type, `entity` as specialized class — across all affected sections
**Verified:** 2026-03-12T18:45:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Section 8 Structs describes struct as a value type with copy-on-assign, structural equality, no lifecycle hooks, and recursive struct illegality | VERIFIED | `09_8_structs.md`: heading `## 1.8 Structs`, opening sentence "Structs are value types", sections 1.8.2–1.8.5 cover copy semantics, structural equality, passing, recursive illegality; no lifecycle hooks section; no `v4.0` or Design Record text |
| 2 | Section 9 Classes exists as a new section describing class as a reference type with heap allocation, lifecycle hooks, and shared-on-assign | VERIFIED | `10_9_classes.md` exists with heading `## 1.9 Classes`, opening "Classes are reference types — heap-allocated, GC-managed, shared-on-assign", full lifecycle hooks table in §1.9.2, heap-allocating construction sequence in §1.9.3 |
| 3 | The EBNF grammar includes a class_decl production and class is listed in the declaration production | VERIFIED | `29_28_grammar_summary_ebnf.md`: `class_decl` appears in `declaration`, as standalone `class_decl` production, and in `extern_decl`; `struct_member` has no `on_decl` |
| 4 | Section 2.9.1 memory model table shows Structs as Value type and includes a Classes row as Reference type | VERIFIED | `38_2_9_memory_model.md` §2.9.1 table: Structs row shows `Value / Register/inline (no heap alloc)`, Classes row shows `Reference / Heap (GC-managed)`; no `v4.0` callouts |
| 5 | Section 2.9.3 closure captures use class keyword for compiler-generated capture environment types | VERIFIED | `38_2_9_memory_model.md` §2.9.3: `class __closure_env_0 { count: int, }` — normative prose, no v4.0 Note blockquote |
| 6 | Section 2.9.8 IL implications describe MOV and NEW as type-dependent normative behavior without any v4.0 callout | VERIFIED | §2.9.8 MOV bullet: "For value types (int, float, bool, enums, structs), copies the full value. For value-type structs this is a multi-word copy of all fields." NEW bullet: "kind-dependent: for classes (kind=4), allocates on the GC heap; for structs (kind=0), initializes the value inline." Zero v4.0 hits confirmed by grep. |
| 7 | Section 2.11 construction model documents kind-dependent NEW semantics for structs (inline) and classes (heap) | VERIFIED | `40_2_11_construction_model.md`: "Struct construction (value type):" section (inline, no lifecycle hooks) and "Class construction (reference type):" section (heap, CALL __on_create); lifecycle hooks paragraph explicitly states structs have none |
| 8 | Section 2.15.1 register model lists structs as value types alongside int/float/bool/enums | VERIFIED | `44_2_15_il_type_system.md` §2.15.1: "For value types (int, float, bool, enums, structs), the register holds the value directly. For value-type structs, the register holds all fields inline as a single abstract typed slot." Classes listed under reference types. |
| 9 | Section 2.16.5 TypeDef kind table includes kind=4 for class and format_version history includes version 3 | VERIFIED | `45_2_16_il_module_format.md`: format_version history "Version 3 — TypeDef.kind=4 (class) added; kind=0 (struct) now means value type." TypeDef.kind line: `0 = struct (value type), 1 = enum, 2 = entity, 3 = component, 4 = class (reference type)`. |
| 10 | Section 3.1 MOV description includes multi-word copy semantics for value-type struct registers | VERIFIED | `49_3_1_data_movement.md` MOV row: "for value-type structs this is a multi-word copy of all fields." |
| 11 | Section 3.8 NEW instruction description includes kind-dependent behavior (inline for struct, heap for class) | VERIFIED | `56_3_8_object_model.md` NEW row: "kind-dependent: for structs (kind=0, value type), initializes the value inline...For classes (kind=4, reference type), allocates zeroed memory on the GC heap." Both Vec2 (struct) and Merchant (class) construction examples present. |

**Score:** 11/11 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `language-spec/spec/09_8_structs.md` | Value-type struct semantics | VERIFIED | Contains "value type", 6 subsections, no lifecycle hooks, no Design Record, no v4.0 callouts |
| `language-spec/spec/10_9_classes.md` | Reference-type class semantics | VERIFIED | Contains "class", lifecycle hooks table, heap-allocating IL sequence, entity cross-ref |
| `language-spec/spec/29_28_grammar_summary_ebnf.md` | class_decl EBNF production | VERIFIED | 3 occurrences of `class_decl` (production, declaration, extern_decl); renamed from 28_27 to 29_28 by Plan 04 |
| `language-spec/spec/38_2_9_memory_model.md` | Memory model with struct=value, class=reference | VERIFIED | Structs=Value, Classes=Reference in table; struct value semantics paragraph; closure capture uses `class` |
| `language-spec/spec/40_2_11_construction_model.md` | Kind-dependent NEW semantics | VERIFIED | Separate struct (inline) and class (heap) construction sections; normative prose; zero v4.0 hits |
| `language-spec/spec/44_2_15_il_type_system.md` | Register model with value-type structs | VERIFIED | Structs in value types bullet, classes in reference types bullet; TypeRef encoding mentions class |
| `language-spec/spec/45_2_16_il_module_format.md` | TypeDef kind=4 class, format_version 3 | VERIFIED | kind=4 confirmed at line 143; format version 3 confirmed at line 18 |
| `language-spec/spec/49_3_1_data_movement.md` | MOV with multi-word copy for value structs | VERIFIED | MOV row includes "multi-word copy of all fields" for value-type structs |
| `language-spec/spec/56_3_8_object_model.md` | NEW with kind-dependent behavior | VERIFIED | NEW row is kind-dependent; both struct (Vec2) and class (Merchant) IL examples present |
| `language-spec/spec/63_3_15_boxing.md` | BOX/UNBOX for value-type structs | VERIFIED | Opening sentence lists structs in value types; closing sentence lists classes (not structs) in reference types |
| `language-spec/spec/69_b_il_decision_log.md` | Updated decision log rows | VERIFIED | Structs row = "Value types" (normative), Classes row = "Reference types" (normative), Lifecycle hooks = "classes and entities", Closure capture = "Shared capture class"; zero v4.0 hits |
| `language-spec/spec/01_table_of_contents.md` | TOC with Classes section and correct numbering | VERIFIED | Entry `[1.9 Classes]` present; subsections 1.9.1–1.9.3 listed; all sections from 1.10 onward renumbered correctly |
| `language-spec/spec/06_5_type_system.md` | Type categories table with Structs/Classes rows | VERIFIED | Two distinct rows: "Structs: Value types -- user-defined composite types with copy semantics" and "Classes: Reference types -- user-defined composite types with heap allocation"; Entities row says "specialized classes" |
| `language-spec/spec/16_15_entities.md` | Entities reference "classes" for lowering | VERIFIED | Opening: "Entities lower to classes with component fields"; uses "Unlike classes" not "Unlike structs" |
| `language-spec/spec/11_10_enums.md` | Renamed from 10_9_enums.md; heading 1.10 | VERIFIED | File exists at new name; heading is `## 1.10 Enums` |
| `language-spec/spec/30_29_lowering_reference.md` | Renamed from 29_28; heading 1.29 | VERIFIED | File exists; heading is `## 1.29 Lowering Reference` |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `09_8_structs.md` | `10_9_classes.md` | cross-reference for lifecycle hooks and recursive types | VERIFIED | §1.8.1 "contrasts with classes (see Section 1.9)"; §1.8.3 "Classes require explicit Eq contract implementation (see Section 1.9)"; §1.8.5 "Use a class for recursive data structures (see Section 1.9)" |
| `29_28_grammar_summary_ebnf.md` | `10_9_classes.md` | class_decl production matches section 9 syntax | VERIFIED | Grammar `class_decl` uses `'class' IDENT [generic_params] '{' { class_member } '}'` matching §1.9 syntax |
| `38_2_9_memory_model.md` | `44_2_15_il_type_system.md` | memory model references register model for value-type struct storage | VERIFIED | §2.9.8: "For value-type structs, this is a multi-word copy of all fields" — consistent with §2.15.1 register model |
| `40_2_11_construction_model.md` | `45_2_16_il_module_format.md` | construction model references TypeDef.kind for NEW behavior | VERIFIED | Construction model references `type_idx` and kind-dispatch; §2.16.5 defines the kind table |
| `56_3_8_object_model.md` | `45_2_16_il_module_format.md` | NEW references TypeDef.kind for dispatch | VERIFIED | NEW row explicitly states "kind=0" and "kind=4" matching §2.16.5 TypeDef.kind definitions |
| `63_3_15_boxing.md` | `44_2_15_il_type_system.md` | BOX/UNBOX references value-type register model | VERIFIED | Boxing file references "register type table (§2.16.6)" — consistent with value-type register model |
| `01_table_of_contents.md` | all spec files | TOC entries match section headings | VERIFIED | TOC has 1.1 through 1.29; actual files have matching `## 1.N` headings; Classes at 1.9, Enums at 1.10 |
| `16_15_entities.md` | `10_9_classes.md` | entities section references classes | VERIFIED | "Entities lower to classes", "Unlike classes (which are direct GC references)", "Entities support all the universal lifecycle hooks (shared with classes)" |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| SPEC-01 | Plans 01, 04 | Section 8 updated — struct as value type, class keyword, entity as specialized class | SATISFIED | `09_8_structs.md` rewritten; `10_9_classes.md` created; `06_5_type_system.md` and `16_15_entities.md` updated |
| SPEC-02 | Plans 02, 04 | Section 2.9.1 Memory Model table updated | SATISFIED | Structs=Value, Classes=Reference in `38_2_9_memory_model.md` table |
| SPEC-03 | Plan 02 | Section 2.9.3 Closure captures updated | SATISFIED | `class __closure_env_0` in normative prose, v4.0 Note blockquote removed |
| SPEC-04 | Plan 02 | Section 2.11 Construction Model — NEW semantics kind-dependent | SATISFIED | Struct (inline) and Class (heap) sections in `40_2_11_construction_model.md` |
| SPEC-05 | Plan 02 | Section 2.15 IL Type System — register model for value-type structs | SATISFIED | Structs moved to value types in §2.15.1 register model |
| SPEC-06 | Plan 02 | Section 2.16.5 TypeDef table — kind=0 reinterpreted, kind=4 added | SATISFIED | `kind=4 = class (reference type)` in `45_2_16_il_module_format.md`; format_version 3 documented |
| SPEC-07 | Plan 03 | Section 3.1 MOV semantics — multi-word copy for value-struct registers | SATISFIED | MOV row in `49_3_1_data_movement.md` includes "multi-word copy of all fields" |
| SPEC-08 | Plan 03 | Section 3.8 NEW instruction — kind-dependent heap vs inline | SATISFIED | NEW row in `56_3_8_object_model.md` is fully kind-dependent with both construction examples |
| SPEC-09 | Plan 03 | Section 3.15 BOX/UNBOX — scope extended to value-type structs | SATISFIED | `63_3_15_boxing.md`: structs added to value types, structs replaced with classes in reference types |
| SPEC-10 | Plan 03 | Appendix B Decision Log — Structs row updated, Classes row added | SATISFIED | Zero v4.0 hits; Structs=Value types, Classes=Reference types, normative present-tense prose throughout |
| SPEC-11 | Plan 01 | `class` keyword added to grammar | SATISFIED | `class_decl` in EBNF `declaration`, `extern_decl`, standalone production in `29_28_grammar_summary_ebnf.md` |

**Coverage: 11/11 requirements satisfied. No orphaned requirements detected.**

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `44_2_15_il_type_system.md` | 12 | "B1 TODO" in a "Resolved" note | Info | Pre-existing note documenting a resolved decision. Not a placeholder. No impact on phase 47 goal. |
| `69_b_il_decision_log.md` | 67 | "TODO A8" in atomic warning row | Info | Pre-existing open item A8 documented in MEMORY.md as a known open issue. Not introduced by phase 47. No impact on phase 47 goal. |

No blocker or warning anti-patterns introduced by phase 47.

---

### Human Verification Required

None. All phase 47 deliverables are spec document content changes verifiable by text inspection.

---

### Gaps Summary

No gaps. All 11 requirements are satisfied. The spec accurately describes the struct/class split:

- `09_8_structs.md` is a clean value-type specification with no lifecycle hooks, no Design Record, no v4.0 callouts.
- `10_9_classes.md` exists as a complete reference-type specification with heap allocation, lifecycle hooks, and cross-references.
- The EBNF grammar formally includes `class_decl` in declarations and extern declarations, with `struct_member` correctly excluding `on_decl`.
- All four IL foundation files (memory model, construction model, type system, module format) describe the split as normative prose.
- All three IL instruction files (MOV, NEW, BOX/UNBOX) and the decision log carry correct normative semantics.
- The TOC has 29 language spec sections (was 28), all splatted filenames use correct section numbers, all headings use the `1.N` prefix, the Type Categories table has separate Structs and Classes rows, and the Entities section references classes for lowering.
- Zero v4.0 callouts remain in any of the 11 phase artifacts.

---

_Verified: 2026-03-12T18:45:00Z_
_Verifier: Claude (gsd-verifier)_
