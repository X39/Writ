# Phase 47: Spec Amendments - Context

**Gathered:** 2026-03-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Update the language specification to accurately describe the struct/class split across all affected sections. `struct` becomes a value type, `class` is a new reference type keyword, `entity` is a specialized class. Covers SPEC-01 through SPEC-11 (11 requirements). No implementation code changes — spec documentation only.

</domain>

<decisions>
## Implementation Decisions

### v3.x Prose Cleanup
- Replace all forward-looking "v4.0 Change" callouts with normative prose — the spec just states what IS
- No version history notes anywhere — clean slate, as if struct was always a value type
- §2.9.1, §2.9.3, §2.9.8 callouts removed and surrounding text rewritten as normative v4.0 truth
- §2.15.1 Register Model: structs move to value-types bullet, classes take reference-types position (three-way split: value types include int/float/bool/enums/structs; reference types include string/classes/entities/arrays/closures)
- §2.9.8 IL Implications: rewrite MOV and NEW bullets as normative type-dependent behavior descriptions (no v4.0 callout)

### Section 8 Restructuring
- Full renumber: insert new §9 Classes, all sections from current §9 onward shift by +1 (§9 Enums → §10, §10 Contracts → §11, ... §28 Lowering Reference → §29)
- Spec splatted filenames updated to match new numbering
- Fix leading `1.` prefix: language spec sections (currently `## 8. Structs`) should be `## 1.8 Structs` to match the `# 1. Writ Language Specification` parent heading (IL spec already uses `## 2.9`, `## 3.1` etc. correctly)
- §8 Structs rewritten for value-type semantics:
  - §8.1 Construction stays (rewritten for inline/no-heap semantics)
  - §8.2 Lifecycle Hooks removed (value structs have none)
  - §8.3 Construction Sequence rewritten for inline NEW behavior
  - New content: shallow copy semantics, structural equality, passing semantics, no-size-limit, recursive struct illegality
- §8.4 Design Record removed entirely — redundant once normative sections carry v4.0 truth
- New §9 Classes mirrors struct structure:
  - §9.1 Construction (heap allocation with `new`)
  - §9.2 Lifecycle Hooks (on create/finalize/serialize/deserialize)
  - §9.3 Construction Sequence (IL for class NEW — heap alloc path)

### Class Grammar (SPEC-11)
- `class` declaration syntax mirrors `struct` exactly: fields, impl blocks, lifecycle hooks
- `class_decl` is a separate EBNF production (not shared with struct_decl)
- `class_decl` added to the `declaration` production list
- `extern class_decl` added alongside `extern struct_decl`

### Claude's Discretion
- Exact wording of normative prose rewrites
- How to handle cross-references between §8 and §9 (avoid duplication while keeping sections self-contained)
- Table formatting details in §2.9.1, Appendix B
- Order of content within the new §9 Classes section
- Whether to add a brief "entities are specialized classes" note in §9 or just cross-ref to Entities section

</decisions>

<specifics>
## Specific Ideas

- The user wants the spec to read as a clean document with no historical baggage — no "Changed in v4.0", no migration notes, no version history except format_version in §2.16.1
- Section numbering should be fully consistent: language spec sections prefixed with `1.` (e.g., `## 1.8 Structs`), IL spec sections already use `2.x`, `3.x`, `4.x`
- Class section should mirror struct section structure for consistency — readers finding one can predict the layout of the other

</specifics>

<code_context>
## Existing Code Insights

### Spec File Structure
- Splatted spec files in `language-spec/spec/` with naming convention `NN_section_name.md`
- Language spec files: `02_1_overview_design_philosophy.md` through `29_28_lowering_reference.md`
- IL spec files: `30_2_1_register_based_virtual_machine.md` through `69_b_il_decision_log.md`
- Table of contents in `01_table_of_contents.md`

### Sections Requiring Updates
- `09_8_structs.md` — rewrite for value types, remove §8.2/§8.4
- `38_2_9_memory_model.md` — §2.9.1 table, §2.9.3 closure captures, §2.9.8 IL implications
- `40_2_11_construction_model.md` — kind-dependent NEW semantics
- `44_2_15_il_type_system.md` — §2.15.1 register model three-way split
- `45_2_16_il_module_format.md` — §2.16.5 TypeDef kind values, format_version
- `49_3_1_data_movement.md` — MOV multi-word copy for value structs
- `56_3_8_object_model.md` — NEW instruction kind-dependent behavior
- `63_3_15_boxing.md` — BOX/UNBOX scope extended to value-type structs
- `28_27_grammar_summary_ebnf.md` — add class_decl production
- `69_b_il_decision_log.md` — already has Structs/Classes rows (verify accuracy)

### New File Required
- New spec file for §9 Classes (will be inserted into the splatted file sequence)

### Renumbering Scope
- All splatted filenames from current `10_9_enums.md` onward shift by +1 in section number
- All internal cross-references between sections must be updated
- Table of contents must be regenerated

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 47-spec-amendments*
*Context gathered: 2026-03-12*
