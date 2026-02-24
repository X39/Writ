# Writ IL Specification
## 2.15 IL Type System

The IL preserves the full Writ type system in metadata. Types are not erased — the runtime has access to complete type
information for dispatch, serialization, and reflection.

**Open items:**

- **Register sizing:** Whether registers are fixed-width or variable-width is deferred to D1.
- **Concrete encoding examples:** To be added in a future pass once instruction encoding (D1) is finalized.

**Resolved in §2.16:** The well-known type table proposed in the earlier B1 TODO is no longer needed. Core types
(Option, Result, etc.) are provided by the `writ-runtime` module (§2.16.8) and referenced via standard cross-module
TypeRef resolution. The blob heap format, register type table, and all metadata tables are specified in §2.16.

### 2.15.1 Register Model

IL functions operate on a set of **typed registers**. Each register holds exactly one value of its declared type. The
compiler emits type declarations for all registers in the method body metadata (see B3 in il-todo.md).

- For **value types** (int, float, bool, enums), the register holds the value directly.
- For **reference types** (string, structs, entities, arrays, closures/delegates), the register holds a GC-managed
  reference.

Registers are abstract — the spec does not mandate a physical size or layout. The runtime uses the register's declared
type to determine storage requirements. A `MOV` instruction copies the full value regardless of the underlying type's
physical size.

This means the IL does not concern itself with "how many bytes is an enum register." The compiler declares
`r3: QuestStatus`,
the runtime allocates whatever storage it needs for that type, and instructions like `NEW_ENUM`, `GET_TAG`, and
`EXTRACT_FIELD` operate on that register as a single unit.

### 2.15.2 Primitive Type Tags

Primitives have fixed type tags in the type reference encoding:

| Tag (u8) | Type     | Kind      | Register Contents                              |
|----------|----------|-----------|------------------------------------------------|
| `0x00`   | `void`   | —         | Zero-width, return-only                        |
| `0x01`   | `int`    | Value     | 64-bit signed integer                          |
| `0x02`   | `float`  | Value     | 64-bit IEEE 754                                |
| `0x03`   | `bool`   | Value     | Logical 0/1                                    |
| `0x04`   | `string` | Reference | GC pointer to heap-allocated, immutable string |

`bool` occupies a full register slot at runtime even though it is logically 1 bit. The spec does not mandate
bit-packing.

### 2.15.3 Type Reference Encoding

A **TypeRef** is a variable-length encoded type descriptor stored in the blob heap. TypeRefs appear wherever the
metadata references a type: field types, parameter types, return types, register type declarations, generic arguments.

| Kind (u8)     | Payload                      | Meaning                                                                                                               |
|---------------|------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| `0x00`–`0x04` | —                            | Primitive (void, int, float, bool, string)                                                                            |
| `0x10`        | TypeDef index (`u32`)        | Named type — struct, enum, entity, or component. The TypeDef entry carries a `kind` flag distinguishing these.        |
| `0x11`        | TypeSpec index (`u32`)       | Instantiated generic type (e.g., `List<int>`, `Option<Guard>`)                                                        |
| `0x12`        | GenericParam ordinal (`u16`) | Open type parameter — the Nth generic param on the enclosing TypeDef or MethodDef                                     |
| `0x20`        | element TypeRef              | `Array<T>` — recursive encoding. The element is itself a TypeRef.                                                     |
| `0x30`        | blob offset (`u32`)          | Function/delegate type — points to a signature blob: `param_count: u16, param_types: TypeRef[], return_type: TypeRef` |

**Design notes:**

- **Single TypeDef table.** All named types (structs, enums, entities, components) share one TypeDef table. The TypeDef
  entry's `kind` field distinguishes them. TypeRefs do not encode the kind — it is looked up from the TypeDef.
- **Option and Result are regular generic enums** in the type system. `Option<int>` is represented as a TypeSpec entry
  pointing to the `Option` TypeDef with type argument `int`. Their specialness exists only at the instruction level
  (`WRAP_SOME`, `IS_OK`, etc.), not in the type encoding.
- **Closure/delegate types.** A closure is a compiler-generated TypeDef (per §2.12). Its TypeRef is a `0x10` pointing
  to that generated TypeDef. The callable signature is encoded separately in the delegate metadata.
- **Recursive encoding.** TypeRefs nest: `Array<Option<int>>` encodes as `0x20` → `0x11` →
  TypeSpec(Option_TypeDef, [`0x01`]).

### 2.15.4 Generic Representation

**In metadata:**

- **Open generic types:** A TypeDef may have one or more `GenericParam` rows, each with a zero-based ordinal.
  `List<T>` has one GenericParam (ordinal 0). `Map<K, V>` has two (ordinals 0, 1).
- **Generic constraints:** `GenericConstraint` rows bind a GenericParam to required contracts. `T: Add + Eq` produces
  two constraint rows, each referencing the GenericParam and a contract TypeDef.
- **Instantiated types:** A `TypeSpec` entry references a TypeDef plus a list of concrete TypeRef arguments.
  `List<int>` = TypeSpec(List_TypeDef, [`0x01`]). `Map<string, int>` = TypeSpec(Map_TypeDef, [`0x04`, `0x01`]).
- **Generic methods:** Same mechanism — GenericParam rows are attached to the MethodDef instead of the TypeDef.
  Call sites provide type arguments in the `CALL` instruction's metadata.

**At runtime (generic dispatch):**

The spec requires that generic code executes correctly but does not mandate a specific dispatch mechanism. The
conceptual model:

When IL code calls a method on a generic type parameter `T` (e.g., `value.method()` where `value: T`), the runtime:

1. Determines the **concrete type tag** of the value. For reference types, the tag is stored on the heap object header.
   For value types passed through generic parameters, the value is **boxed** (see below).
2. Resolves the method via the **contract dispatch table**: a mapping from
   `(concrete_type_tag, contract_id, method_slot)` to a method entry point.
3. Calls the resolved method.

**Boxing:** When a value type (`int`, `float`, `bool`, enum) is passed to a generic parameter, the runtime boxes it —
allocating a small heap object that wraps the value and carries a type tag. This allows uniform representation through
generic code paths. The runtime may unbox or avoid boxing when the concrete type is statically known, but the spec does
not require such optimizations.

Runtimes may use any dispatch implementation: hash tables, vtable-style arrays, polymorphic inline caches, or
monomorphization of hot paths. The spec mandates correct behavior, not a specific strategy.

**Runtime guidance:** The runtime should build dispatch structures from `ImplDef` rows at module load time. Each
`ImplDef` maps a `(type, contract)` pair to a method list; the runtime should flatten these into a lookup structure
indexed by `(concrete_type_tag, contract_id, method_slot)` for O(1) dispatch. A flat table or hash map is the
recommended approach for predictable performance. Polymorphic inline caching at `CALL_VIRT` sites is a permitted
optimization for hot call sites.

### 2.15.5 Enum Representation

Enums are value types. An enum value consists of a **tag** and an optional **payload**.

**Tag:** A `u16` discriminant identifying the active variant. Supports up to 65535 variants per enum type.

**Layout by variant kind:**

- **Tag-only variants** (e.g., `QuestStatus::NotStarted`): Only the tag. No payload space.
- **Payload variants** (e.g., `QuestStatus::InProgress(step: int)`): Tag + inline payload fields, laid out
  consecutively per the variant's field definitions in the TypeDef.

**Total size:** An enum value's size is `sizeof(tag) + sizeof(largest_variant_payload)` across all variants. All
variants occupy the same total space; smaller variants are padded. The compiler calculates the layout from the TypeDef
at emit time.

**In registers:** An enum register holds the complete enum value (tag + payload) as a single abstract unit. The
runtime determines the physical storage from the register's declared type (see §2.15.1).

**Payload field types:** Payload fields follow the same rules as struct fields. Value-typed payload fields are stored
inline. Reference-typed payload fields store GC references and are traced by the garbage collector.

**Examples:**

```
enum QuestStatus {
    NotStarted,                    // tag-only: tag=0, payload=0 bytes
    InProgress(currentStep: int),  // tag=1, payload=8 bytes (one int)
    Completed,                     // tag-only: tag=2, payload=0 bytes
    Failed(reason: string),        // tag=3, payload=ref-size (one string ref)
}
// Total size: sizeof(u16) + max(0, 8, 0, ref-size) = tag + 8 bytes
```

**Option\<T\> null-pointer optimization:** When `T` is a non-nullable reference type, a runtime *may* represent
`Option<T>` as a bare reference where `null` = `None` and non-null = `Some(value)`. This is a permitted runtime
optimization, not mandated by the spec. IL code uses `WRAP_SOME` / `IS_NONE` / etc. regardless — the runtime may
elide them internally.

