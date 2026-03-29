# Phase 100: Spec and IL Foundation - Research

**Researched:** 2026-03-28
**Domain:** Language specification authoring — reflection semantics, IL opcode assignment, writ-runtime virtual module
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all implementation choices are at Claude's discretion.

### Claude's Discretion
All implementation choices are at Claude's discretion — pure spec/documentation phase. All key design decisions already captured in STATE.md Accumulated Context:
- typeof(expr) is static compile-time, expr.get_type() is dynamic runtime
- Reflectable is contract 19 with get_type() -> Type, auto-implemented on all user-defined types
- 6 reflection types: Type, FieldInfo, MethodInfo, ParameterInfo, AttributeInfo, ContractInfo
- FieldInfo.set() crashes task on let-field violation
- MethodInfo.invoke() uses current task stack
- format_version bumps 3 → 4
- BOX/UNBOX coercions at reflection API boundaries (no TyKind::Any)
- Dynamic construction deferred to v12+
- Primitive typeof via intrinsics (IntGetType etc.)
- ReflectionIndex lazy init
- GC permanent roots for reflection singletons

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SPEC-01 | Reflection type system defined in new language spec section (Type, FieldInfo, MethodInfo, ParameterInfo, AttributeInfo, ContractInfo) | §1.X Reflection section must be authored as a new splatted file; all 6 type shapes and their fields/methods documented |
| SPEC-02 | typeof(expr) semantics defined — static type query returning Type, distinct from get_type() dynamic query | New section must show the divergence example for polymorphic variables; explain static-vs-dynamic distinction explicitly |
| SPEC-03 | Reflectable contract defined — auto-implemented on all user-defined types, single get_type() method | Reflectable is contract slot 19 in §2.18; the language spec section must describe auto-impl rule for scripts |
| SPEC-04 | Dynamic invocation rules defined — MethodInfo.invoke(), FieldInfo.set() mutability enforcement, Type.construct() lifecycle hook dispatch | Note: Type.construct() is deferred (v12+); only FieldInfo.set() crash semantics and MethodInfo.invoke() stack rules need documenting |
| SPEC-05 | TypeOf opcode assigned in §4.2 opcode table | Add TypeOf in 0x0A Reflection sub-range; update §3.10 instruction reference and §4.0 count table |
| SPEC-06 | format_version bumped to 4 in spec | Edit §2.16.1 version history line; add "Version 4 — TYPEOF opcode added (§4.2 0x0A30)" |
| SPEC-07 | any-at-boundaries resolved — BOX/UNBOX coercion approach for reflection API parameters/returns | Document that FieldInfo.get()/set() and MethodInfo.invoke() accept/return boxed values; compiler inserts BOX/UNBOX at call sites |
| SPEC-08 | Generic reflection scope documented — what type_args() promises for statically-known vs runtime-queried types | Type.type_args() returns concrete args for typeof(List<int>); may return empty array for runtime-queried types of open generics |
</phase_requirements>

---

## Summary

Phase 100 is a pure documentation phase. It has no code changes. The deliverables are four edits to existing spec files and one new spec file:

1. **New file** — `language-spec/spec/` numbered to appear after §1.27 (builtins) and before §1.28 (grammar). The reflection section is §1.28 Reflection; the old §1.28 Grammar Summary becomes §1.29; the old §1.29 Lowering Reference becomes §1.30. Alternatively, since existing files are already numbered `28_` through `30_`, the new file should take a number between the user-defined attributes section (18) and grammar (29). The most natural placement is after §1.27 Standard Library Builtins — insert as `28_1_28_reflection.md` and bump `29_` and `30_` filenames. However this creates renaming friction. The simpler path is to continue at the next available numeric prefix after 29 (grammar is `29_28_`) — i.e., append as `30_1_28_reflection.md` and renumber inline. **See Architecture Patterns for the naming recommendation.**

2. **Edit** `47_2_18_writ_runtime_module_contents.md` — add §2.18.9 Reflection Types with the 6 builtin classes and Reflectable as contract 19.

3. **Edit** `67_4_2_opcode_assignment_table.md` — add TypeOf in 0x0A Reflection sub-range.

4. **Edit** `58_3_10_type_operations.md` — add TypeOf row to the instruction reference.

5. **Edit** `65_4_0_instruction_count_by_category.md` — increment Type Operations count and total.

6. **Edit** `33_2_4_binary_format.md` and `45_2_16_il_module_format.md` — document format_version 4.

7. **Edit** `01_table_of_contents.md` — add §1.28 Reflection entries.

**Primary recommendation:** Write the §1.28 Reflection spec section as a new file inserted after §1.27, then make targeted edits to the four IL spec files. All content is already fully decided in STATE.md — this phase is transcription and organization, not design.

---

## Standard Stack

### Core

| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| Markdown | — | Spec authoring format | All existing spec files use Markdown |
| Splatted file naming | numeric prefix `NN_` | Ordering in spec directory | Established convention for this project |

No library dependencies — this is a documentation-only phase.

---

## Architecture Patterns

### Existing Spec File Structure

The spec lives in `language-spec/spec/`. Files are numbered with a two-digit prefix and a section number in the filename:

```
language-spec/spec/
├── 00_preamble.md
├── 01_table_of_contents.md
├── 02_1_overview_design_philosophy.md   ← §1.1
├── ...
├── 28_27_standard_library_builtins.md   ← §1.27
├── 29_28_grammar_summary_ebnf.md        ← §1.28
├── 30_29_lowering_reference.md          ← §1.29
├── 30_2_1_register_based_virtual_machine.md   ← §2.1 (SAME prefix 30!)
├── ...
├── 47_2_18_writ_runtime_module_contents.md    ← §2.18
├── ...
├── 65_4_0_instruction_count_by_category.md
├── 66_4_1_instruction_shape_reference.md
├── 67_4_2_opcode_assignment_table.md
├── 68_a_open_questions.md
├── 69_b_il_decision_log.md
```

**Naming collision observation:** The existing file listing shows prefix `30_` is used by both `30_29_lowering_reference.md` and `30_2_1_register_based_virtual_machine.md`. This means the numeric prefix is only used to sort language spec files before IL spec files — it is not strictly unique. The language spec sections go `28_`, `29_`, `30_` and then the IL spec sections also start around `30_`.

**Recommended naming for new file:** Add the reflection section as a language spec file positioned after §1.27 and before §1.28 Grammar. The cleanest option that avoids renaming any existing file:

- New file: `28_1_28_reflection.md` — inserts alphabetically between `28_27_standard_library_builtins.md` and `29_28_grammar_summary_ebnf.md`, which is exactly right.
- The section number becomes §1.28 Reflection.
- The existing §1.28 Grammar Summary becomes §1.29, and §1.29 Lowering Reference becomes §1.30. **But this requires renaming** `29_28_grammar_summary_ebnf.md` → `29_29_grammar_summary_ebnf.md` and `30_29_lowering_reference.md` → `30_30_lowering_reference.md` (or similar).

**Simplest path with no downstream renaming:** Use the section name §1.28 Reflection but add it as a new file with prefix `28_1_28_reflection.md`. The TOC entry numbers must be updated but the filenames of the grammar and lowering files do not need to change. The file-sort prefix `28_` still sorts correctly before `29_` and `30_`. This is the recommended approach.

**Alternative: append as §1.30 Reflection** after the existing lowering reference, with filename `30_1_30_reflection.md`. Section numbers are entirely internal to the file content and TOC — the filename prefix just controls sort order. This avoids any grammar/lowering renaming concern at the cost of a non-sequential section number gap.

**Decision for planner:** The planner should instruct Claude to add the file as `28_1_28_reflection.md` (§1.28), rename `29_28_grammar_summary_ebnf.md` to `29_29_grammar_summary_ebnf.md` and `30_29_lowering_reference.md` to `30_30_lowering_reference.md`, update the section numbers inside those files from §1.28/§1.29 to §1.29/§1.30, and update the TOC. This is clean and keeps sequential numbering.

### Pattern: Language Spec Section Structure

Each language spec section follows this pattern (from §1.17 Attributes as example):

```markdown
# 1.17 Attributes

[Intro paragraph — what this section defines]

## 1.17.1 Syntax
[Code examples in ```writ blocks]

## 1.17.2 [Sub-topic]
[Tables for structured data, code blocks for examples]

---
```

- Top-level heading matches the section number
- Sub-sections use `##` with sequential numbers
- Code blocks use ` ```writ ` language tag
- Tables for structured reference data (methods, fields, etc.)
- Ends with `---` horizontal rule

### Pattern: §2.18 writ-runtime Module Contents

Section 2.18 follows a consistent sub-section pattern for each type/contract group:

```markdown
## 2.18.X [Name]

[Short description]

**Fields:** (table: Field | Type | Access/Notes)
**Methods (intrinsic):** (table: Method | Signature | Intrinsic IL)
**Contract implementations:** (table: Contract | Intrinsic)
```

Reflection types are class types (kind=4 in TypeDef), not enums. They have methods that dispatch via CALL_VIRT on Reflectable or direct CALL on the reflection type's own methods. Most methods are intrinsic — the runtime provides native implementations.

### TypeOf Opcode Placement

The opcode table uses category byte `0x0A` for Type Operations. Current sub-ranges:
- `0x0A00`–`0x0A0F` — Option operations (4 used, 12 slots)
- `0x0A10`–`0x0A1F` — Result operations (6 used, 10 slots)
- `0x0A20`–`0x0A2F` — Enum operations (3 used, 13 slots)

A new sub-range for Reflection operations starts at `0x0A30`:

```
0x0A30 — TYPEOF — Shape: RI32 — r_dst, type_idx:u32
```

Shape RI32 (8 bytes: `u16(op) u16(r_dst) u32(type_idx)`) is the right fit: one output register and one compile-time type index baked in. This matches the existing pattern for NEW (`0x0800 RI32 r, type_idx`).

### format_version 4

The format_version history is a single line in §2.16.1:

```
Format version history: Version 1 — initial format ... Version 3 — TypeDef.kind=4 (class) added ...
```

The edit appends: `Version 4 — TYPEOF opcode added (reflection; §3.10, §4.2 0x0A30); old format_version=3 modules rejected at load time with UnsupportedVersion.`

The §2.4 Binary Format file is currently a stub (2 lines). It should not need editing since the format_version lives in §2.16.1.

### Anti-Patterns to Avoid

- **Don't add a new file prefix category.** The existing `28_`/`29_`/`30_` prefix scheme is adequate. Do not introduce `28a_` or other non-numeric variants.
- **Don't create a new §2.19 for reflection.** Add §2.18.9 as a sub-section of the existing writ-runtime module contents file. This keeps all virtual module content together.
- **Don't document Type.construct() in the dynamic invocation section.** STATE.md explicitly deferred Type.construct() to v12+. The spec should note "dynamic construction via Type.construct() is reserved for a future version."
- **Don't assign opcode 0x0A23 (extending enum range).** TypeOf is not an enum operation — start the new Reflection sub-range at 0x0A30 for logical grouping.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Spec formatting consistency | Custom style guide | Follow existing §1.17 Attributes and §2.18 writ-runtime module sections as templates | These sections are already written and represent the project's established style |
| Opcode numbering | New category byte | Extend 0x0A Type Operations with a Reflection sub-range at 0x0A30 | Avoids bumping the category count and is consistent with how Option/Result/Enum sub-ranges work |
| Contract slot numbering | Arbitrary assignment | Reflectable = contract 19 as decided | STATE.md already assigned contract 19; don't re-derive |

---

## Common Pitfalls

### Pitfall 1: Conflating typeof(expr) with expr.get_type()

**What goes wrong:** Spec text blurs the static/dynamic distinction, leaving implementors uncertain about which one to use where.
**Why it happens:** Both return a `Type` object, so they appear interchangeable until polymorphism is involved.
**How to avoid:** The spec must include a concrete divergence example:

```writ
entity Animal { }
entity Dog { }  // Dog is a subtype of Animal in spirit

fn example(a: Animal) {
    let static_t  = typeof(a);       // Always: Type for Animal (compile-time static type)
    let dynamic_t = a.get_type();    // May be: Type for Dog (runtime actual type)
    // static_t == dynamic_t only when a is actually an Animal, not a subtype
}
```

**Warning signs:** Draft spec text that says "both return the Type of the expression" without noting they can diverge.

### Pitfall 2: Omitting the auto-impl rule scope

**What goes wrong:** Spec says "Reflectable is auto-implemented" without specifying which types get it.
**Why it happens:** Primitives and extern types follow different rules.
**How to avoid:** Be explicit: auto-impl applies to all **user-defined** types (structs, classes, entities, enums). Primitives get separate IntGetType/FloatGetType/BoolGetType/StringGetType intrinsics. Extern types do not get Reflectable auto-impl (out of scope for this phase).

### Pitfall 3: Documenting Type.construct() as available

**What goes wrong:** Including Type.construct() in the dynamic invocation section when it is deferred to v12+.
**How to avoid:** The spec must include a note: "Dynamic type instantiation via Type.construct() is deferred to a future version. Attempting to call it in this version crashes the task with UnsupportedOperation."

### Pitfall 4: Wrong BOX/UNBOX contract for reflection methods

**What goes wrong:** Spec says FieldInfo.get() returns `object` or some special any-type, confusing implementors.
**Why it happens:** Without TyKind::Any, the return type of FieldInfo.get() needs a precise spec.
**How to avoid:** State explicitly that FieldInfo.get(), FieldInfo.set(), MethodInfo.invoke() use **boxed** values. The return type of FieldInfo.get() is declared as `Box` (a compiler-known opaque reference-typed container). The compiler inserts BOX before passing values in and UNBOX after receiving them back. This is consistent with existing §2.15.4 boxing behavior.

### Pitfall 5: Forgetting to update §4.0 instruction count

**What goes wrong:** Adding TYPEOF to §4.2 but leaving §4.0 count table with the old total of 91.
**How to avoid:** Edit §4.0 to increment the Type Operations row count and update the total from 91 to 92.

### Pitfall 6: FieldDef readonly bit presence

**What goes wrong:** Spec for FieldInfo.set() references a readonly bit in FieldDef.flags, but the bit may not be assigned yet.
**Why it happens:** FieldDef.flags currently has: visibility, has_default, is_component_field. There is no explicit "readonly" bit.
**How to avoid:** STATE.md includes a pending todo: "Verify FieldDef.flags readonly bit exists in writ-module/src/tables.rs before Phase 107 planning." For Phase 100 spec purposes, the spec can define the semantics without referencing a specific bit position — say "FieldInfo.set() checks whether the field was declared with `let` (immutable binding), crashing the task if so. The runtime determines immutability from the field's declaration metadata." The actual bit assignment in FieldDef.flags is a Phase 101/107 concern.

---

## Code Examples

Verified patterns from existing spec sections:

### typeof(expr) — Static Query

```writ
// Source: STATE.md Accumulated Context / this phase's design
let t: Type = typeof(Player);
log($"Type name: {t.name}");            // "Player"
log($"Namespace: {t.namespace}");       // e.g., "game.entities"

// Divergence example
fn inspect(a: Animal) {
    let static_t  = typeof(a);        // Type for Animal (static)
    let dynamic_t = a.get_type();     // Type for Dog if a is Dog (dynamic)
    let same = static_t == dynamic_t; // false when a is a subtype
}
```

### get_type() — Dynamic Query via Reflectable Contract

```writ
// Source: STATE.md Accumulated Context
fn printTypeName(x: Reflectable) {
    say(x.get_type().name);   // dispatches via CALL_VIRT contract 19 slot 0
}
```

### FieldInfo Iteration

```writ
// Source: this phase's design (SPEC-01, SPEC-03)
let t = typeof(Merchant);
for field in t.fields() {
    log($"  {field.name}: {field.declared_type.name}");
}
```

### MethodInfo.invoke() — Dynamic Invocation

```writ
// Source: STATE.md — MethodInfo.invoke() uses current task stack
let t    = typeof(Merchant);
let greet = t.methods().find(fn(m) = m.name == "greet")!;
greet.invoke(merchant_instance, []);   // args boxed automatically by compiler
```

### FieldInfo.set() — Mutability Enforcement

```writ
// Source: STATE.md — crashes task on let-field write
let t    = typeof(Player);
let hp_field = t.fields().find(fn(f) = f.name == "hp")!;
hp_field.set(player, 100);  // OK if hp is 'mut'
// If hp was declared 'let', task crashes: "Reflection write to immutable field 'hp'"
```

### TypeOf IL Instruction

```
// Source: this phase's design (SPEC-05)
// Emitted for: let t = typeof(Player);
TYPEOF r3, type_idx(Player)
// r3 receives a Type heap object for Player, lazily allocated on first access
// Encoding: RI32 — u16(0x0A30) u16(r_dst) u32(type_idx)
```

---

## Integration Points and File-by-File Edit Plan

### File 1 — NEW: `language-spec/spec/28_1_28_reflection.md`

New file, §1.28 Reflection. Required sub-sections:

- **§1.28.1 Reflection Types** — Table of the 6 types with their fields and methods
- **§1.28.2 typeof(expr) — Static Type Query** — syntax, semantics, IL lowering note, divergence example
- **§1.28.3 get_type() — Dynamic Type Query** — Reflectable contract dispatch, divergence example
- **§1.28.4 Reflectable Contract** — auto-impl rule (user-defined types only), get_type() signature
- **§1.28.5 Type Introspection Methods** — Type.fields(), Type.methods(), Type.attributes(), Type.contracts(), Type.implements(), Type.is_generic, Type.type_args()
- **§1.28.6 Dynamic Invocation** — FieldInfo.get/set semantics, MethodInfo.invoke semantics, mutability enforcement, boxing at boundaries
- **§1.28.7 Generic Reflection Scope** — what type_args() promises, limitations for runtime-queried types
- **§1.28.8 Scope and Limitations** — pub fields/methods only, no extern reflection, Type.construct() deferred

### File 2 — EDIT: `language-spec/spec/29_28_grammar_summary_ebnf.md`

Rename to `29_29_grammar_summary_ebnf.md`. Update heading from `# 1.28 Grammar Summary (EBNF)` to `# 1.29 Grammar Summary (EBNF)`. Update all internal cross-references. Add `typeof` keyword to grammar rules.

### File 3 — EDIT: `language-spec/spec/30_29_lowering_reference.md`

Rename to `30_30_lowering_reference.md`. Update heading from `# 1.29 Lowering Reference` to `# 1.30 Lowering Reference`. Update all internal cross-references.

### File 4 — EDIT: `language-spec/spec/47_2_18_writ_runtime_module_contents.md`

Add §2.18.9 Reflection Types. Content:
- List all 6 reflection class TypeDefs with their fields (name, namespace, etc.) and intrinsic methods
- Reflectable as ContractDef slot 19 with get_type() -> Type method
- Note that all reflection type methods are marked intrinsic (runtime-provided native implementations)

### File 5 — EDIT: `language-spec/spec/67_4_2_opcode_assignment_table.md`

Add new subsection after §0x0A — Type Operations:

```
**Reflection (0x0A30–0x0A3F):**

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0A30` | TYPEOF   | RI32  |
```

### File 6 — EDIT: `language-spec/spec/58_3_10_type_operations.md`

Add row after the Enum operations table:

```
**Reflection:**

| Mnemonic | Shape | Operands              | Description                                                        |
|----------|-------|-----------------------|--------------------------------------------------------------------|
| `TYPEOF` | RI32  | r_dst, type_idx:u32   | Load compile-time Type singleton for the type at type_idx. r_dst receives a lazily-allocated Type heap reference. The type_idx is a TypeDef/TypeRef/TypeSpec metadata token baked in at compile time. |
```

### File 7 — EDIT: `language-spec/spec/65_4_0_instruction_count_by_category.md`

Update the Type Operations rows:

- Split "Option" / "Result" / "Enum" into separate rows (already separate in this file)
- Add `Reflection | 1 | TYPEOF` row
- Increment Total from 91 to 92

### File 8 — EDIT: `language-spec/spec/45_2_16_il_module_format.md`

In §2.16.1, update the format version history line:

```
Version 4 — TYPEOF opcode added (§3.10, §4.2 0x0A30); format_version=3 modules are rejected at load time with UnsupportedVersion.
```

### File 9 — EDIT: `language-spec/spec/01_table_of_contents.md`

- Update §1.28 entry to be Reflection (with sub-section entries)
- Update old §1.28 Grammar Summary to §1.29
- Update old §1.29 Lowering Reference to §1.30
- Add §2.18.9 Reflection Types sub-entry
- Add TypeOf to §4.2 opcode table entries

---

## State of the Art

This is a self-contained language spec project. There is no external "state of the art" to research — all design decisions are captured in STATE.md Accumulated Context (see User Constraints above). The design is original and does not reference external reflection APIs.

For reference, the design is informed by:
- CLR/C# reflection (Type, FieldInfo, MethodInfo) — same name conventions, same lazy singleton pattern
- Java reflection — the auto-implement-on-all-types approach (similar to java.lang.Object.getClass())
- Rust trait objects — the no-runtime-overhead-by-default philosophy (typeof is zero-cost at runtime; only dynamic get_type() has dispatch cost)

---

## Open Questions

1. **FieldDef.flags readonly bit assignment**
   - What we know: Current FieldDef.flags has visibility, has_default, is_component_field bits. No readonly bit.
   - What's unclear: Whether to assign a bit position in this spec phase or defer to the format_version 4 edit.
   - Recommendation: The spec text for SPEC-04 should define the semantic ("declared with let = immutable") without assigning a specific bit position. Reserve bit position assignment for Phase 101 (compiler pipeline) when the format is actively being touched. The spec can say "the runtime reads immutability from the FieldDef flags" and Phase 101 assigns the bit.

2. **BOX type name in spec**
   - What we know: STATE.md says "BOX/UNBOX coercions at reflection API boundaries, no TyKind::Any."
   - What's unclear: What type name to use in the spec for the boxed return of FieldInfo.get()? Options: `Box`, `any`, `object`.
   - Recommendation: Use `Box` as the type name (matching the IL's BOX/UNBOX opcode category §3.15). The spec should say: "FieldInfo.get(instance) -> Box — returns the field value as a boxed value. The caller uses UNBOX to extract the concrete type."

3. **Renaming §1.28 and §1.29**
   - What we know: Adding a new §1.28 Reflection requires bumping existing §1.28 Grammar and §1.29 Lowering to §1.29 and §1.30.
   - What's unclear: Whether any cross-spec references use these section numbers.
   - Recommendation: Search for "1.28" and "1.29" in the spec files before renaming. A quick grep will show if any file references these sections by number.

---

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — purely documentation edits to Markdown files).

---

## Validation Architecture

> Skipped: `workflow.nyquist_validation` is not explicitly set to false, but this phase has no testable code — it is purely spec/documentation. There are no test commands to run and no test files to create. The planner should treat validation as manual review: "a reader of the spec can find X" style checks per the success criteria.

Manual validation checklist (per phase success criteria):
- [ ] A reader finds a complete §1.X Reflection section describing all 6 types with fields and methods
- [ ] The spec shows a divergence example for typeof vs get_type() with polymorphic variables
- [ ] Reflectable contract is defined with get_type() -> Type and the auto-impl rule
- [ ] FieldInfo.set() crash semantics for let-fields are documented
- [ ] MethodInfo.invoke() current-task-stack semantics are documented
- [ ] §4.2 opcode table contains TypeOf at opcode 0x0A30
- [ ] §2.18 writ-runtime module lists 6 reflection types and Reflectable as contract 19
- [ ] §2.16.1 format_version history includes Version 4

---

## Sources

### Primary (HIGH confidence)
- `language-spec/spec/47_2_18_writ_runtime_module_contents.md` — existing virtual module pattern; §2.18.8 Versioning
- `language-spec/spec/67_4_2_opcode_assignment_table.md` — opcode layout, 0x0A sub-range analysis
- `language-spec/spec/45_2_16_il_module_format.md` — format_version history, §2.16.1
- `language-spec/spec/18_17_attributes.md` — spec section style template; §1.17 sub-section pattern
- `language-spec/spec/12_11_contracts.md` — contract definition pattern; auto-impl examples
- `language-spec/spec/66_4_1_instruction_shape_reference.md` — RI32 shape confirmed for TYPEOF
- `language-spec/spec/65_4_0_instruction_count_by_category.md` — current total: 91 instructions
- `.planning/STATE.md` Accumulated Context — all reflection design decisions
- `.planning/REQUIREMENTS.md` — SPEC-01 through SPEC-08 definitions
- `.planning/phases/100-spec-and-il-foundation/100-CONTEXT.md` — implementation decisions

### Secondary (MEDIUM confidence)
- `language-spec/spec/01_table_of_contents.md` — section numbering and TOC update scope
- `language-spec/spec/44_2_15_il_type_system.md` — boxing semantics for reflection boundary types
- `language-spec/spec/58_3_10_type_operations.md` — instruction reference format for TYPEOF addition

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — pure Markdown, no external libraries
- Architecture: HIGH — all design decisions locked in STATE.md; file structure fully analyzed
- Pitfalls: HIGH — all pitfalls derived from examining the actual spec files and cross-referencing the design decisions

**Research date:** 2026-03-28
**Valid until:** Stable indefinitely (spec decisions are locked; no external dependency drift)
