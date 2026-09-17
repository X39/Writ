# 2.11 Construction Model

**Decision:** Construction uses the `new` keyword with brace-syntax for all types. No user-defined constructors.
`spawn` is reserved for task concurrency only.

**Syntax:** `new Type { field: value, ... }` for structs, classes, and entities. The `new` keyword disambiguates
construction from block expressions, making the syntax unambiguous for the parser. The compiler determines the IL
sequence from the type's kind.

**Initializer evaluation:** The compiler produces exactly one value for every field in declaration order. A
construction-site initializer replaces that field's default expression. It evaluates those expressions in declaration
order and packs their results into consecutive registers. If evaluating any expression crashes, no construction
instruction executes and no object or entity is created.

**Version 9 cross-module defaults:** FieldDef records `has_default` but does not contain the default expression or an
initializer method. A compiler can inline a default only when it has the declaring source expression. When constructing
an imported type, the source must therefore provide every field explicitly; omitting an imported field whose metadata
only says `has_default` is a compile-time error. The compiler must not substitute `Void`, a primitive zero, or an
uninitialized slot. A future module version may replace this limitation with serialized initializer thunks.

`NEW` and `SPAWN_ENTITY` receive the complete initializer block:

```text
NEW/SPAWN_ENTITY r_dst, type_token, field_count, r_base
```

The block is `r_base..r_base+field_count-1`. `field_count` must exactly equal the resolved TypeDef's field count.
When it is zero, `r_base` is ignored and the compiler emits zero.

Before any allocation or externally visible state change, the runtime validates the destination register, type token,
permitted TypeDef kind, exact field count, and complete initializer register range and copies the values out of the
current frame. A validation failure is a runtime error: it crashes the current task without allocating, publishing an
entity, notifying the host, consuming a request id, or changing `r_dst`.

**Struct construction (value type):**

1. Evaluate and pack every field value.
2. `NEW r_dst, type_token, field_count, r_base` — create the complete struct value atomically.

No lifecycle hooks are called for structs. Construction is complete when `NEW` returns.

**Class construction (reference type):**

1. Evaluate and pack every field value.
2. `NEW r_dst, type_token, field_count, r_base` — allocate the class with its complete field vector.
3. `CALL __on_create` — run the `on create` hook body, if defined, after all fields are initialized.

**Entity construction:**

1. Evaluate and pack every script-field value.
2. `SPAWN_ENTITY r_dst, type_token, field_count, r_base` — validate first, then allocate a pending entity whose script
   fields are already complete, register its canonical type identity, and ask the host to provision components from
   ComponentSlot metadata.
3. `INIT_ENTITY r_dst` — transition the entity to alive, complete host initialization, and fire `on_create`.

The separation of `SPAWN_ENTITY` and `INIT_ENTITY` is a lifecycle/host boundary, not a mutable-field initialization
window. The handle produced by `SPAWN_ENTITY` already refers to complete script storage. The runtime must reject a
second `INIT_ENTITY`, an `INIT_ENTITY` for a non-pending handle, and use of a malformed spawn instruction.

**Read-only fields:** A field without the source `mut` modifier carries the FieldDef `READONLY` bit. Atomic
`NEW`/`SPAWN_ENTITY` initialization may establish its value. `SET_FIELD`, reflection, hooks, and all other
post-construction mechanisms have no initialization privilege and must crash before changing it.

**No constructors:** Construction is entirely compiler-generated. `new Type { ... }` produces initializer evaluation,
one atomic `NEW`/`SPAWN_ENTITY`, and the applicable lifecycle call. Fields without defaults are required at every
construction site. For convenience factories, use static methods: `Merchant::create("Tim")`.

**Lifecycle hooks:** Classes and entities support lifecycle hooks (`on create`, `on finalize`, `on serialize`,
`on deserialize`). Entities additionally support `on destroy` and `on interact`. Structs have no lifecycle hooks. All hooks receive implicit `mut self`.
Hooks lower to regular methods stored in the TypeDef metadata.

