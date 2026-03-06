# Phase 46: Structs-as-Value-Types Design Discussion - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

A written decision record covering the YES/NO/MAYBE paths for structs as value types, GC implications, IL encoding changes, and v4.0 scope. No Rust source files are modified — the deliverable is a spec amendment section and a milestone entry.

</domain>

<decisions>
## Implementation Decisions

### Semantics Split: C# Model (struct + class)
- **Decision: YES** — Writ will adopt a struct/class split
- `struct` = value type (inline, copy-on-assign, no heap allocation for the struct itself)
- `class` = reference type (heap-allocated, GC-managed, shared-on-assign) — this is what `struct` is today
- `entity` keeps its own keyword but is conceptually a specialized class (entity -> class, but class !-> entity)
- Both `struct` and `class` can implement contracts (interfaces); value-type structs get boxed through generics, same as enums
- Value-type structs **can** contain reference-type fields (string, Array, class instances)
- Assignment of value-type structs is **shallow copy** — reference fields copy the pointer (both copies share the same object)
- **No lifecycle hooks** on value-type structs — no `on create`, no `on finalize`. If you need lifecycle hooks, use a class
- **Breaking change** in v4.0: existing `struct` keyword changes meaning from reference type to value type

### Copy vs Clone
- **Implicit copy always** — assignment copies the struct inline, same as int/float/enum today. No `.clone()` required
- **No size limit** — trust the developer. No compiler warning for large value-type structs
- **Structural equality by default** — value-type structs auto-derive field-by-field equality (like enums). Classes require explicit Eq impl
- **Passing semantics** — Claude's discretion (recommended: always by-copy for true value semantics; `mut self` mutates the local copy, not the caller's)

### GC & Performance Tradeoffs
- Reference fields inside value-type structs need GC tracing — **acknowledged, mechanism deferred to runtime** (same approach as enum payload tracing today)
- **No nesting depth limit** — compiler computes total size by walking the type graph. Recursive value-type structs are illegal (infinite size = compiler error)
- **Boxing** through generics works identically to enums today — brief mention in record, no special analysis
- **Include motivating examples** — Vec2, Vec3, Color, Rect showing why value semantics are natural for game scripting

### Migration Scope (v4.0)
- **Single v4.0 milestone**, phased internally (spec -> IL -> VM -> compiler -> tests)
- **Enumerate specific IL changes** in the decision record: TypeDef kind flag additions, NEW instruction behavior, MOV semantics for value structs, register sizing implications
- **format_version bump, no backward compatibility** — old .writil files incompatible. Pre-1.0, this is acceptable
- **Milestone entry** — Claude's discretion on detail level (scope boundary defined, phase breakdown deferred to v4.0 start)

### Claude's Discretion
- Passing semantics for value-type structs (recommended: always by-copy)
- Boxing analysis depth for generics
- v4.0 milestone entry detail level (estimated phase count vs scope boundary only)

</decisions>

<specifics>
## Specific Ideas

- Entity keyword marks a specialized kind of class — "entity -> class but class !-> entity"
- Motivating examples should include Vec2, Vec3, Color, Rect — common game scripting value types
- Structural equality for value-type structs matches enum behavior — consistency across value types
- No hooks on value structs is a clean separation: value types = pure data, classes = identity + behavior

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `HeapObject` enum in writ-runtime: currently has Struct variant for heap-allocated structs — will need a new inline/register representation for value-type structs
- `alloc_struct` in runtime heap: allocates struct on GC heap — classes will continue using this; value-type structs won't
- Enum value handling in VM: already handles inline tag+payload in registers — same pattern extends to value-type structs

### Established Patterns
- TypeDef table uses `kind` field to distinguish struct/enum/entity/component — adding `class` kind (or splitting struct kind into value-struct and class) is the natural extension
- Boxing for enums through generics (BOX/UNBOX instructions) — same mechanism applies to value-type structs
- `MOV` instruction copies register contents — for value-type structs, this becomes a multi-word copy instead of pointer copy

### Integration Points
- §2.9.1 Memory Model table: Structs row changes from "Reference" to "Value"; new "Classes" row added as "Reference"
- §2.15 IL Type System: Register model section needs to describe value-type struct registers
- TypeDef table: kind flag needs new values or split
- Compiler codegen: `new Type { ... }` must emit different IL for struct (inline init) vs class (heap alloc)
- Parser: `class` keyword added alongside existing `struct`

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 46-structs-as-value-types-design-discussion*
*Context gathered: 2026-03-06*
