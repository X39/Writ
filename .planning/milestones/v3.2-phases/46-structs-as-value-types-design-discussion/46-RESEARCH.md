# Phase 46: Structs-as-Value-Types Design Discussion - Research

**Researched:** 2026-03-06
**Domain:** Language spec design record — type system semantics, IL encoding, GC implications
**Confidence:** HIGH (all findings from the project's own spec files and Rust source)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Decision: YES** — Writ will adopt a struct/class split
- `struct` = value type (inline, copy-on-assign, no heap allocation for the struct itself)
- `class` = reference type (heap-allocated, GC-managed, shared-on-assign) — this is what `struct` is today
- `entity` keeps its own keyword; conceptually a specialized class (entity -> class, but class !-> entity)
- Both `struct` and `class` can implement contracts; value-type structs get boxed through generics, same as enums
- Value-type structs **can** contain reference-type fields (string, Array, class instances)
- Assignment is **shallow copy** — reference fields copy the pointer (both copies share the same object)
- **No lifecycle hooks** on value-type structs — no `on create`, no `on finalize`; use a class if hooks are needed
- **Breaking change** in v4.0: existing `struct` keyword changes meaning from reference type to value type
- **Implicit copy always** — assignment copies the struct inline; no `.clone()` required
- **No size limit** — trust the developer; no compiler warning for large value-type structs
- **Structural equality by default** — value-type structs auto-derive field-by-field equality (like enums)
- **Passing semantics** — Claude's discretion (recommended: always by-copy; `mut self` mutates the local copy)
- Reference fields inside value-type structs need GC tracing — mechanism deferred to runtime (same as enum payload tracing today)
- **No nesting depth limit** — compiler computes total size by walking the type graph; recursive value-type structs are illegal (infinite size = compiler error)
- **Boxing** through generics works identically to enums today
- **Motivating examples** — Vec2, Vec3, Color, Rect
- **Single v4.0 milestone**, phased internally (spec -> IL -> VM -> compiler -> tests)
- **Enumerate specific IL changes** in the decision record
- **format_version bump, no backward compatibility** — old .writil files incompatible; pre-1.0, acceptable

### Claude's Discretion

- Passing semantics for value-type structs (recommended: always by-copy)
- Boxing analysis depth for generics
- v4.0 milestone entry detail level (estimated phase count vs scope boundary only)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DES-01 | Spec amendment section on structs-as-value-types with a written decision record covering YES/NO/MAYBE paths, GC implications, and a v4.0 scope if YES | All findings below directly enable writing the amendment, decision record, and MILESTONES.md entry |
</phase_requirements>

---

## Summary

Phase 46 produces a single documentation artifact: a spec amendment section (to be inserted into the language spec and/or IL spec) recording the YES decision to adopt a C# model where `struct` becomes a value type and `class` takes the reference-type role that `struct` plays today. The decision record must be a complete standalone reference — any future implementor reading it must understand what changes, why, and at what scope.

The research phase has done a thorough audit of the existing spec and Rust implementation to determine the exact text that the decision record must contain. The findings fall into four groups: (1) the current state that must be described as the baseline, (2) the new semantics that must be defined precisely, (3) the IL changes that must be enumerated for the v4.0 milestone scope, and (4) the GC implications that must be acknowledged.

**Primary recommendation:** Write the decision record as a new section `§X.Y Structs as Value Types — Design Record` appended to the language spec, cross-referencing the existing §8 (Structs), §2.9 (Memory Model), §2.15 (IL Type System), and §2.16 (Module Format). Also add a MILESTONES.md entry for v4.0.

---

## Current State — What the Record Must Describe as Baseline

These are the facts the decision record must state clearly as "the old model" so the change is unambiguous.

### Structs Today (Reference Types)

From `language-spec/spec/38_2_9_memory_model.md`, §2.9.1 Value Types vs Reference Types:

| Type | Kind | Storage | Assignment | GC Traced |
|------|------|---------|-----------|-----------|
| Structs | **Reference** | Heap (GC-managed) | Copy reference (shared object) | Yes |
| Enums | **Value** | Register/stack (tag + inline payload) | Copy tag + payload | Payload fields traced if references |

The decision log in `language-spec/spec/69_b_il_decision_log.md` records:
- **Structs: Reference types** — Heap-allocated, GC-managed. Assignment copies reference (shared object).
- **Enums: Value types** — Tag + inline payload. Copied on assignment. Reference payloads are GC-traced.

This is the baseline. The amendment explicitly reverses the Structs entry.

### Current TypeDef.kind Encoding

From `writ-module/src/tables.rs`, the `TypeDefKind` enum (values live in the binary format):

```rust
pub enum TypeDefKind {
    Struct = 0,   // currently means reference type
    Enum = 1,
    Entity = 2,
    Component = 3,
}
```

And from spec §2.16.5: `TypeDef.kind: 0 = struct, 1 = enum, 2 = entity, 3 = component`.

This is what v4.0 must extend.

### Current Heap Representation

From `writ-runtime/src/heap.rs`:

```rust
pub enum HeapObject {
    String(String),
    Struct { fields: Vec<Value> },         // always heap-allocated today
    Array { elem_type: u32, elements: Vec<Value> },
    Delegate { method_idx: usize, target: Option<Value> },
    Enum { type_idx: u32, tag: u16, fields: Vec<Value> },
    Boxed(Value),
}
```

`HeapObject::Struct` is the reference-type struct representation. Value-type structs (post-v4.0) will not use `HeapObject::Struct` for their normal representation — they will be stored inline in registers, like `HeapObject::Enum` is today.

### Current Value Enum (Register Representation)

From `writ-runtime/src/value.rs`:

```rust
pub enum Value {
    Void,
    Int(i64),
    Float(f64),
    Bool(bool),
    Ref(HeapRef),    // structs, strings, arrays, delegates land here
    Entity(EntityId),
}
```

For value-type structs (v4.0), there is no current `Value` variant that holds an inline multi-field struct. This is one of the VM representation changes that v4.0 must address.

### Current MOV Instruction Semantics

From spec §3.1 (Data Movement):

> `MOV` — Copy register to register. Semantics depend on type: value copy for primitives, reference copy for heap types.

For value-type structs, MOV must copy all fields, not just a reference. This is a VM execution change.

### Current NEW Instruction Semantics

From spec §3.8 (Object Model):

> `NEW` — Allocate struct instance. Memory is zeroed; defaults and overrides applied via subsequent SET_FIELD instructions.

For value-type structs, `NEW` must change to in-place initialization rather than heap allocation. The amendment must specify a new instruction or updated semantics.

### Current Lifecycle Hooks on Structs

From `language-spec/spec/09_8_structs.md`, §8.2 Lifecycle Hooks:

Structs currently support `on create`, `on finalize`, `on serialize`, `on deserialize`. All four hooks are defined on the current struct type. Under the new model, **value-type structs (struct) lose all hooks**; **class** (the renamed reference type) retains all four hooks.

---

## New Semantics — What the Decision Record Must Define

### The Struct/Class Split

| Keyword | Kind | Storage | Assignment | GC Traced | Lifecycle Hooks |
|---------|------|---------|-----------|-----------|-----------------|
| `struct` | Value | Register/inline (no heap alloc for struct itself) | Shallow copy — copies all fields by value; ref fields copy pointer | Payload ref fields traced | None |
| `class` | Reference | Heap (GC-managed) | Copy reference (shared object) | Yes | `on create`, `on finalize`, `on serialize`, `on deserialize` |
| `entity` | Reference (specialized class) | Entity runtime + GC heap | Copy handle | Yes | `on create`, `on finalize`, `on serialize`, `on deserialize`, `on destroy`, `on interact` |

**Key rule:** `entity` is a subkind of class for conceptual purposes, but does NOT use the `class` keyword — it retains `entity`. The spec should note "entity -> class but class !-> entity" meaning entity is always a reference type with identity, but a plain class is not an entity.

### Shallow Copy Semantics

The decision record should include this illustrative example:

```
struct Vec2 { x: float, y: float }
struct Color { r: float, g: float, b: float, a: float }
struct Note { text: string, priority: int }  // string field is a ref

let a = new Vec2 { x: 1.0, y: 2.0 };
let b = a;           // b is a fresh copy: {x: 1.0, y: 2.0}
// a and b are independent; mutation to b.x does NOT affect a.x

let c = new Note { text: "hello", priority: 1 };
let d = c;           // d.text and c.text point to the same string object
                     // but the Note struct itself is two independent copies
```

### Structural Equality

The decision record must state: value-type structs auto-derive field-by-field equality, matching the existing enum behavior. Two structs are equal if and only if all corresponding fields compare equal using the standard equality rules for each field's type (reference equality for ref-typed fields). Classes require explicit `Eq` contract implementation.

### Passing Semantics (Claude's Discretion — Recommended)

**Recommendation: always by-copy.** When a value-type struct is passed as a function argument, the callee receives a copy. `mut self` on a method that takes a value-type struct mutates the local copy, not the caller's value. This is identical to how `int` and `float` behave today. The decision record should state this recommendation explicitly.

This means: from the IL perspective, passing a value-type struct is a multi-register or multi-word copy at the call boundary, not a pointer pass.

### Recursive Value-Type Structs Are Illegal

```
struct Bad {
    child: Bad,   // ERROR: infinite size — value-type struct cannot contain itself
}

struct Node {
    data: int,
    next: Node?,  // Still illegal for value types — Option<Node> has infinite size too
                  // (would need Node to be a class for this to work)
}
```

The compiler must detect this via a size-computation walk of the type graph and emit a compile error.

### Motivating Examples (Required by CONTEXT.md)

The decision record must include these game scripting examples showing why value semantics are natural:

```
// Vec2 — two floats, always copied, no sharing needed
struct Vec2 { x: float, y: float }

// Vec3 — three floats
struct Vec3 { x: float, y: float, z: float }

// Color — four floats, RGBA
struct Color { r: float, g: float, b: float, a: float }

// Rect — two points or position+size
struct Rect { x: float, y: float, w: float, h: float }

// Usage pattern that proves the value
fn move_entity(pos: Vec2, delta: Vec2) -> Vec2 {
    new Vec2 { x: pos.x + delta.x, y: pos.y + delta.y }
}
// No mutation of the caller's pos — clean functional style natural in game scripting
```

---

## IL Changes — Enumeration for v4.0 Scope

This is the section the CONTEXT.md requires to be enumerated in the decision record. Each item must appear with enough precision that a v4.0 planner can break it into tasks.

### IL Change 1: TypeDef.kind Flag Extension

Current spec (§2.16.5): `TypeDef.kind: 0 = struct, 1 = enum, 2 = entity, 3 = component`

v4.0 change: add `class` as a new kind, and reinterpret the existing kinds:

| New kind value | Meaning |
|---------------|---------|
| 0 | `struct` — value type (NEW semantics) |
| 1 | `enum` — unchanged |
| 2 | `entity` — unchanged |
| 3 | `component` — unchanged |
| 4 | `class` — reference type (what `struct` meant before) |

**Why a new value 4 (not reuse 0):** Old .writil files with kind=0 mean reference-type struct. The format_version bump makes old files incompatible anyway, but being explicit about the semantic change in the kind encoding is cleaner.

**Impact:** `TypeDefKind` enum in `writ-module/src/tables.rs` gains a `Class = 4` variant. All sites that match on `TypeDefKind::Struct` must be updated to distinguish value-struct from class.

### IL Change 2: NEW Instruction Behavior

Current (§3.8): `NEW r_dst, type_idx:u32` — Allocate struct instance on the GC heap.

v4.0 change: `NEW` behavior depends on the TypeDef's kind:
- If kind = `class` (4): allocate on GC heap, exactly as today.
- If kind = `struct` (0): initialize value in-place — no heap allocation. The compiler emits field initialization into consecutive registers (or a struct register). No heap allocation occurs for the struct itself.

**Alternative considered:** Introduce a separate `NEW_STRUCT` opcode for value-type initialization. This is cleaner but adds opcode count. The decision record should note both options and recommend the kind-check approach to avoid opcode proliferation (inline with how MOV already has type-dependent semantics).

### IL Change 3: MOV Instruction — Multi-Word Copy for Value Structs

Current (§3.1): `MOV r_dst, r_src` — copies a register. For references, this is a pointer copy.

v4.0 change: For value-type struct registers, `MOV` must copy all fields. The spec already states registers are abstract typed slots (§2.15.1) and "the runtime determines physical storage from the register's declared type." This means MOV semantics are already type-dependent by spec; the v4.0 change makes value-type structs behave correctly under this rule.

From the runtime implementation perspective, a value-type struct register would need to be represented as either:
- A sequence of N registers (one per field, recursively flattened), OR
- A new `Value::Struct { fields: Vec<Value> }` variant

The decision record should note that the VM representation is a runtime concern — the spec does not mandate which approach — but the IL spec must clarify that `MOV` for value-struct registers copies all fields.

### IL Change 4: GC Tracing — Reference Fields in Value Structs

Current behavior: GC traces struct heap objects by scanning their `fields: Vec<Value>` for `Value::Ref` entries.

v4.0 requirement: Value-type struct values held in registers (or as inline fields of other structs) must be GC-traced through. The mechanism is the same as enum payload tracing today (already described in §2.15.5):

> Payload field types follow the same rules as struct fields. Value-typed payload fields are stored inline. Reference-typed payload fields store GC references and are traced by the garbage collector.

The decision record should state explicitly: "The runtime must trace through value-type struct registers to find GC roots. This is the same mechanism used for enum payload tracing. The register type table (§2.16.6) provides the type information needed to identify which registers or fields are GC references."

### IL Change 5: BOX/UNBOX — Value Structs Through Generics

Current (§3.15): `BOX` and `UNBOX` are defined for value types (int, float, bool, enums). The boxing section notes: "Reference types (string, structs, arrays, entities, delegates) are already references and pass through generics without boxing."

v4.0 change: `struct` (value type) joins `int`, `float`, `bool`, enums as types that require boxing through generics. The boxing section must be updated to include value-type structs. The BOX/UNBOX instructions are already defined and correct — only the documentation changes.

### IL Change 6: Structural Equality Instruction or Convention

Current: No dedicated struct equality instruction. Reference equality for structs uses pointer comparison via `CMP_EQ` on `Ref` values.

v4.0 requirement: Value-type structs use structural equality (field-by-field comparison). The implementation options are:
- (a) Emit a sequence of field comparison instructions at each equality check site (compiler concern).
- (b) Add a `STRUCT_EQ` instruction that performs structural comparison at runtime.

The decision record should recommend option (a) — the compiler emits field comparisons explicitly, consistent with how enum structural equality is handled through match patterns. This keeps the instruction set minimal and avoids a runtime special case.

### IL Change 7: format_version Bump

From spec §2.16.1: "format_version starts at 1, bumps on incompatible layout changes." Current version is 2 (added param_count to MethodDef in Phase 39).

v4.0 change: format_version must bump to 3 (minimum), reflecting that TypeDef.kind values have new semantics. Old .writil files with kind=0 meant reference-type struct; they are incompatible with v4.0 runtimes that treat kind=0 as value-type struct.

---

## GC Implications — What the Decision Record Must Acknowledge

### Tracing Through Value Structs

A value-type struct that contains reference fields (string, Array, class instance) must have those references traced by the GC. Because the struct is stored inline in a register (not as a heap object), the GC cannot reach it through a heap traversal — it must be found through register scanning.

The existing GC root rule (§2.9.6): "All registers in all active task call stacks — the IL type metadata tells the GC exactly which registers hold references at any PC."

For v4.0, the register type table (§2.16.6) must encode value-type struct registers with enough type information that the GC can walk their fields to find embedded references. The mechanism is the same as for enum registers today: the register's TypeRef encodes the struct TypeDef, and the GC consults the TypeDef's FieldDef list to find reference-typed fields.

**Key point for the decision record:** This does not require a new mechanism. It requires the runtime's GC to apply its existing enum-payload-tracing logic to value-type struct registers as well. The spec already has the hooks (typed register table + TypeDef.kind); the runtime just needs to handle kind=0 (value struct) the way it handles kind=1 (enum) in the field-tracing pass.

### Closure Capture Struct Complication

From §2.9.3 (Closure Captures): The compiler generates `struct __closure_env_0 { count: int }` to hold mutable closure captures.

Under v4.0, the word `struct` in the closure capture comment means value-type struct. This is fine (the capture env is meant to be heap-allocated via the delegate mechanism), but the spec must clarify that compiler-generated closure capture types use the `class` keyword (reference semantics), not `struct`. The decision record should include this note to prevent ambiguity in the closure model.

### No GC Finalization on Value Structs

Since value-type structs are not heap objects, the GC does not manage their lifetime and cannot fire `on finalize`. This is consistent with the locked decision (no lifecycle hooks on value structs). The decision record should state this explicitly: "Value-type structs are not GC-managed objects. Their storage is reclaimed when the enclosing scope exits (register is deallocated). There is no finalization."

---

## Architecture Patterns for the Decision Record Document

### Where the Amendment Lives in the Spec

The spec is organized with splatted files in `language-spec/spec/`. The decision record should be added as a new file adjacent to the existing spec sections. Two reasonable locations:

**Option A:** As an amendment to §8 (Structs). Add `§8.4 Value-Type Structs and Classes — Design Record` to `language-spec/spec/09_8_structs.md`.

**Option B:** As a new standalone file, e.g., `language-spec/spec/09b_8_struct_class_design_record.md`, so it does not clutter the main §8 file.

**Recommendation:** Option A — inline with §8. The design record is part of the language spec, not a separate document. The planner should add §8.4 directly to `09_8_structs.md` and cross-reference IL changes in `38_2_9_memory_model.md` (update the §2.9.1 table).

Additionally, the IL decision log (`69_b_il_decision_log.md`) must be updated to flip the Structs row and add a Class row.

### Where the MILESTONES.md Entry Lives

`D:/dev/git/Writ/.planning/MILESTONES.md` — prepend a new `## v4.0 Structs as Value Types` section. The current milestone entries follow a consistent format. The v4.0 entry should be concise (scope boundary only, per Claude's discretion), listing the five implementation layers: spec -> IL -> VM -> compiler -> tests.

---

## Common Pitfalls for the Document Author

### Pitfall 1: Conflating "class" the new keyword with the old "struct" documentation

The spec currently uses "struct" to mean reference type in §2.9.1, §2.9.3 (closure captures), §8.x (lifecycle hooks), and §2.11 (construction model). Every occurrence of "struct" in those sections that discusses reference-type behavior must either be updated to say "class" or annotated as "class (formerly struct in v3.x)".

**Prevention:** The decision record should include a "Migration Notes" subsection listing each spec section that uses the word "struct" in a reference-type context.

### Pitfall 2: The Decision Log's Structs Row

`69_b_il_decision_log.md` has: `Structs | Reference types | Heap-allocated, GC-managed.`

This row must be updated in the decision record to reflect the split. The amendment must include a specific instruction to update this table.

### Pitfall 3: Closure Capture Structs

`38_2_9_memory_model.md` §2.9.3 shows compiler-generated `struct __closure_env_0` — this uses the `struct` keyword and will need to change to `class` (or an implementation-internal notation) in a post-v4.0 world. The decision record should flag this as a known follow-on change, not resolve it.

### Pitfall 4: Construction Syntax Ambiguity

`new Merchant { ... }` currently means heap-allocate a reference-type struct. Under v4.0:
- `new Vec2 { x: 1.0, y: 2.0 }` — inline-initializes a value-type struct (no heap alloc)
- `new Merchant { ... }` — heap-allocates a class

The syntax is identical; the behavior differs based on the type's kind. The decision record must state this clearly. The compiler determines which IL to emit from the TypeDef's kind field, not the syntax.

### Pitfall 5: The format_version History Table

`38_2_16_il_module_format.md` §2.16.1 states: "Format version history: Version 1 — initial format. Version 2 — added param_count(u16) to MethodDef."

The decision record must note that v4.0 will add: "Version 3 — TypeDef.kind=4 (class) added; kind=0 (struct) now means value type. Old files with kind=0 are incompatible."

---

## Code Examples (Verified Patterns)

All examples below are grounded in the existing spec or implementation and are directly usable in the decision record.

### Example 1: Value-Type Struct vs Class Declaration

```
// v4.0 syntax — struct = value type
struct Vec2 { x: float, y: float }          // value type: copied on assignment
struct Color { r: float, g: float, b: float, a: float }

// v4.0 syntax — class = reference type (what 'struct' meant in v3.x)
class Merchant {
    name: string,
    gold: int,
    reputation: float = 0.5,

    on create {
        // post-initialization logic (class retains lifecycle hooks)
    }
}
```

### Example 2: Assignment Semantics Contrast

```
// Value type (struct) — independent copies
let a = new Vec2 { x: 1.0, y: 2.0 };
let b = a;         // b is a fresh copy
// modifying b.x does NOT affect a.x

// Reference type (class) — shared object
let m1 = new Merchant { name: "Tim", gold: 100 };
let m2 = m1;       // m2 and m1 point to the same Merchant object
// modifying m2.gold DOES affect m1.gold (same as current struct behavior)
```

### Example 3: IL Encoding Change for value-type struct Construction

```
// v3.x (current): new Vec2 { x: 1.0, y: 2.0 } emits:
NEW           r0, Vec2_type             // heap allocation
LOAD_FLOAT    r1, 1.0
SET_FIELD     r0, x_field, r1
LOAD_FLOAT    r1, 2.0
SET_FIELD     r0, y_field, r1
// r0 holds a Ref (HeapRef) — reference type

// v4.0: new Vec2 { x: 1.0, y: 2.0 } emits:
// (implementation: register-based inline or NEW with value-type flag)
// r0 holds an inline Vec2 value — NOT a heap reference
// Exact encoding TBD in v4.0 spec work — decision record marks this as a known IL change
```

### Example 4: TypeDef.kind Extension

From `writ-module/src/tables.rs` (current state), the v4.0 change:

```rust
// Current (v3.x):
pub enum TypeDefKind {
    Struct = 0,   // reference type — CHANGES MEANING in v4.0
    Enum = 1,
    Entity = 2,
    Component = 3,
}

// v4.0 additions:
pub enum TypeDefKind {
    Struct = 0,   // NOW: value type
    Enum = 1,     // unchanged
    Entity = 2,   // unchanged
    Component = 3, // unchanged
    Class = 4,    // NEW: reference type (what Struct=0 meant in v3.x)
}
```

### Example 5: GC Tracing — Value Struct with Ref Field

```
// This value-type struct contains a reference field
struct Note { text: string, priority: int }

// In a register: r5 = Note { text: <HeapRef to string>, priority: 3 }
// The GC must trace r5 to find the string reference.
// Mechanism: same as enum payload tracing (§2.15.5) —
//   GC reads r5's TypeRef → finds Note TypeDef (kind=0, value struct)
//   → walks FieldDef list → finds text:string (ref field) → adds to trace roots
//   → finds priority:int (value field) → skips
```

---

## State of the Art

| Old Approach (v3.x) | New Approach (v4.0) | Impact |
|---------------------|---------------------|--------|
| `struct` = reference type | `struct` = value type | Breaking keyword meaning change |
| No `class` keyword | `class` = reference type | New keyword, parser change |
| Structs allocated on GC heap | value structs stored inline in registers | VM representation change |
| Structs support lifecycle hooks | value structs: no hooks; classes: hooks | Semantic clarification |
| MOV struct = pointer copy | MOV value struct = multi-field copy | VM execution change |
| NEW always heap-allocates | NEW for value struct: in-place init | IL semantics change |
| TypeDef.kind: 0=struct (ref) | TypeDef.kind: 0=struct (value), 4=class | Binary format change; format_version=3 |
| Struct equality = identity (same ref) | value struct equality = structural (field-by-field) | Language semantic change |
| Boxing: only int/float/bool/enum | Boxing: int/float/bool/enum/value-struct | §3.15 doc update; BOX/UNBOX unchanged |

**Deprecated:**
- `struct` meaning reference type: superseded by `class` in v4.0
- `HeapObject::Struct` as the sole struct representation: joined by inline/register representation for value structs (class continues using heap-based representation)

---

## Open Questions

1. **STRUCT_EQ instruction vs compiler-emitted comparisons**
   - What we know: enums use GET_TAG + field extraction for equality; value structs could follow the same pattern
   - What's unclear: whether a generic `STRUCT_EQ` instruction would be more efficient at the VM level
   - Recommendation: record both options in the decision record; recommend compiler-emitted field comparisons (option a) to keep the instruction set minimal; defer final choice to v4.0 spec work

2. **Register representation for multi-field value structs**
   - What we know: the spec says registers are abstract typed slots; the runtime determines physical storage
   - What's unclear: whether the runtime uses one register per struct (with internal multi-field storage) or flattened registers (one per field)
   - Recommendation: note this as an implementation decision for the v4.0 VM chapter; the spec mandates correct semantics, not physical layout. Record the two options: (a) single abstract struct register with runtime-managed layout, (b) register flattening (N fields → N registers). Option (a) mirrors enum handling and is consistent with §2.15.1.

3. **Closure capture env type after v4.0**
   - What we know: §2.9.3 documents compiler-generated `struct __closure_env_0` using the `struct` keyword
   - What's unclear: whether the spec should say "class" for these generated types in v4.0 (since they are reference types)
   - Recommendation: flag in the decision record as a known follow-on spec update; the decision record itself does not need to resolve this — it is sufficient to note "compiler-generated capture environments are reference types (class) regardless of the user-facing keyword used in the spec prose"

---

## Validation Architecture

> Note: `workflow.nyquist_validation` key is absent from `.planning/config.json`, so it is treated as enabled. However, this phase produces documentation only — no code files are written. All validation is by human review of the written documents.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | N/A — documentation phase |
| Quick run command | N/A |
| Full suite command | `cargo test --workspace` (confirm no Rust tests broken by the doc change) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DES-01 | Spec amendment section exists with decision record | manual | human review of written spec file | ❌ Wave 0 (new file/section) |
| DES-01 | Decision record covers GC tracing implications | manual | human review | ❌ part of same doc |
| DES-01 | Decision record covers IL encoding changes | manual | human review | ❌ part of same doc |
| DES-01 | Decision record covers VM value representation changes | manual | human review | ❌ part of same doc |
| DES-01 | Decision record covers estimated v4.0 scope | manual | human review | ❌ part of same doc |
| DES-01 | v4.0 milestone entry in MILESTONES.md | manual | human review of MILESTONES.md | ❌ Wave 0 (new entry) |
| DES-01 | No Rust source file modified | automated | `git diff --name-only HEAD \| grep '\.rs$'` — must be empty | n/a (verify) |

### Wave 0 Gaps

- [ ] `language-spec/spec/09_8_structs.md` — add §8.4 design record section
- [ ] `language-spec/spec/38_2_9_memory_model.md` — update §2.9.1 table and §2.9.8 IL implications
- [ ] `language-spec/spec/69_b_il_decision_log.md` — update Structs row, add Class row
- [ ] `.planning/MILESTONES.md` — add v4.0 milestone entry

*(No test infrastructure needed — documentation-only phase)*

---

## Sources

### Primary (HIGH confidence)

- `language-spec/spec/38_2_9_memory_model.md` — current Memory Model spec including §2.9.1 type table, §2.9.6 GC roots, §2.9.8 IL implications
- `language-spec/spec/09_8_structs.md` — current Struct spec including lifecycle hooks, construction sequence
- `language-spec/spec/44_2_15_il_type_system.md` — TypeRef encoding, register model, enum representation (§2.15.5 is the GC tracing template for value types with ref fields)
- `language-spec/spec/45_2_16_il_module_format.md` — TypeDef.kind definition, format_version history, metadata tables
- `language-spec/spec/56_3_8_object_model.md` — NEW and SET_FIELD instruction specs
- `language-spec/spec/49_3_1_data_movement.md` — MOV instruction spec
- `language-spec/spec/63_3_15_boxing.md` — BOX/UNBOX instruction spec
- `language-spec/spec/69_b_il_decision_log.md` — existing decision log showing Structs=Reference, Enums=Value
- `writ-module/src/tables.rs` — TypeDefKind enum (current: Struct=0, Enum=1, Entity=2, Component=3)
- `writ-runtime/src/heap.rs` — HeapObject enum (current: Struct always heap-allocated)
- `writ-runtime/src/value.rs` — Value enum (no inline struct variant currently)
- `.planning/phases/46-structs-as-value-types-design-discussion/46-CONTEXT.md` — locked decisions from discuss-phase
- `.planning/MILESTONES.md` — milestone format reference

### Secondary (MEDIUM confidence)

- `language-spec/spec/40_2_11_construction_model.md` — construction model (NEW + SET_FIELD sequence); shows what v4.0 must change for value-type structs
- `language-spec/spec/68_a_open_questions.md` — confirms structs-as-value-types not previously in open questions (no prior design work to reconcile)

---

## Metadata

**Confidence breakdown:**
- User decisions (locked): HIGH — from CONTEXT.md, result of explicit /gsd:discuss-phase session
- Current spec baseline: HIGH — read directly from spec files
- IL changes enumeration: HIGH — derived from spec files + Rust source, no ambiguity
- GC tracing mechanism: HIGH — §2.15.5 (enum payload tracing) is the exact template
- VM representation options: MEDIUM — implementation options, spec deliberately does not mandate physical layout
- v4.0 milestone scope: MEDIUM — scope boundary is locked; phase breakdown is Claude's discretion

**Research date:** 2026-03-06
**Valid until:** Stable — this is internal project documentation, not external library research. Valid until the spec itself changes.
