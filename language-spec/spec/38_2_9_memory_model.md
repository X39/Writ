# Writ IL Specification
## 2.9 Memory Model

**Decision:** The IL and language assume a **garbage-collected runtime**. The spec does not mandate a specific GC
algorithm (generational, tracing, etc.) — runtime implementors choose — but language semantics are designed for GC and
do not expose manual memory management.

### 2.9.1 Value Types vs Reference Types

| Type                   | Kind                     | Storage                                  | Assignment                     | GC Traced                           |
|------------------------|--------------------------|------------------------------------------|--------------------------------|-------------------------------------|
| `int`, `float`, `bool` | **Value**                | Register (direct bits)                   | Copy bits                      | No                                  |
| `string`               | **Reference, immutable** | Heap (GC-managed)                        | Copy reference                 | Yes                                 |
| Structs                | **Reference**            | Heap (GC-managed)                        | Copy reference (shared object) | Yes                                 |
| Enums                  | **Value**                | Register/stack (tag + inline payload)    | Copy tag + payload             | Payload fields traced if references |
| Arrays                 | **Reference**            | Heap (GC-managed)                        | Copy reference (shared)        | Yes                                 |
| Entities               | **Reference (handle)**   | Entity runtime + GC heap                 | Copy handle                    | Yes                                 |
| Components             | **Extern (host-owned)**  | Host-managed, accessed via entity handle | Via entity reference           | Host responsibility                 |
| Closures/Delegates     | **Reference**            | Heap (GC-managed)                        | Copy reference                 | Yes                                 |

**Enum value semantics:** Enums are value types with inline payloads. The tag is a small integer. Payload fields are
stored inline (for value types) or as references (for reference-typed fields). `Option<int>` is just a tag + an int — no
heap allocation. `Option<string>` is a tag + a string reference. Assignment copies the tag + all payload
bits/references.

### 2.9.2 Assignment and Mutability

`let` / `let mut` controls **binding mutability**, not object mutability:

- `let a = thing` — immutable binding. Cannot reassign `a`. Cannot mutate fields through `a`.
- `let mut a = thing` — mutable binding. Can reassign `a`. Can mutate fields through `a`.

For reference types, assignment copies the reference. Both bindings point to the same object:

```
let mut a = Merchant(name: "Tim", gold: 100);
let mut b = a;     // b and a point to the same object
b.gold += 50;      // a.gold is ALSO now 150
```

This is standard GC-language behavior (Java classes, C# classes, Lua tables).

### 2.9.3 Closure Captures

**Immutable captures (`let`):** The value is copied into the capture struct. For value types, this is a bit copy. For
reference types, this copies the reference (closure and outer scope share the same object, but neither can reassign the
binding).

**Mutable captures (`let mut`):** The compiler generates a **shared capture struct** on the heap that holds the mutable
variable. Both the outer scope and the closure hold a reference to this same struct. The outer scope is rewritten to
access the variable through the struct. Mutations through either side are visible to both.

```
// Source:
let mut count = 0;
let process = fn(x: int) -> int {
    count += 1;    // mutates shared capture
    x + count
};
process(10);       // count is now 1
log(count);        // also 1 — same struct

// Compiler rewrites to (conceptually):
let __env = __closure_env_0 { count: 0 };
let process = Delegate(__closure_body_0, __env);
// process(10) calls __closure_body_0 with __env as first arg
__env.count;       // outer scope accesses through the struct too
```

The capture struct is a compiler-generated type, not a special runtime type:

```
struct __closure_env_0 {
    count: int,   // shared mutable field
}
```

No special runtime types are needed — this is purely a compiler transformation using standard structs.

### 2.9.4 String Handling

- **Literals:** Stored in the module's string heap. Shared, interned. Zero GC pressure at runtime.
- **Runtime strings** (concatenation, format strings, `Into<string>` results): Heap-allocated, GC-managed, not interned.
- **Comparison:** Always by value (character content), regardless of interning. `CMP_EQ_S` compares content.
- **Immutable:** All strings are immutable. Operations like concatenation produce new strings.

### 2.9.5 Entity Lifecycle

Entities have **dual reachability** — they exist in the entity runtime's registry AND as GC-managed objects:

- **Alive:** In the entity registry. Handle is valid. Fields readable/writable.
- **Destroyed:** `DESTROY_ENTITY` called. `on_destroy` fires, defer handlers run, entity removed from registry. Handle
  becomes **dead**.
- **Collected:** GC reclaims memory once the entity is both destroyed AND unreachable from any GC root.

Accessing a dead entity handle (reading fields, calling methods, component access) is a **crash** — same severity as
unwrapping None. Use `Entity.isAlive(entity)` (`ENTITY_IS_ALIVE` instruction) to check liveness without crashing.

Entity handles are opaque runtime-managed identifiers, not direct GC pointers. The runtime resolves handles against its
entity registry. Destruction marks the registry entry as dead but does not invalidate or null existing handles — they
remain valid values that can be stored, passed, and compared. The GC manages the handle objects; an entity's memory is
only collected after it is destroyed AND unreachable from all GC roots.

`Entity.getOrCreate<T>()` for a destroyed singleton **recreates it** (the semantics are get-or-*create*).

**Runtime guidance (entity storage):** Entities should be stored in a registry keyed by opaque handle IDs — a
generation-indexed array is recommended, where each slot holds the entity's script state and a generation counter for
handle validation (stale handles detected by generation mismatch). Component access via `GET_COMPONENT` should resolve
against the entity's `ComponentSlot` list from the TypeDef metadata — a type-tag lookup returning the component
reference or `None`. Singleton entities (marked `[Singleton]`) should be maintained in a per-type registry indexed by
TypeDef token; `GET_OR_CREATE` checks this registry first and falls through to full entity construction if absent.

### 2.9.6 GC Roots

The GC traces from these roots:

1. **All registers in all active task call stacks** — the IL type metadata tells the GC exactly which registers hold
   references at any PC (precise scanning, not conservative).
2. **All global variables** — `global mut` and `const` holding reference types.
3. **The entity registry** — all live (non-destroyed) entities.
4. **The task handle tree** — handles to spawned tasks.

### 2.9.7 Garbage Collection

The spec assumes garbage collection but does not prescribe a specific algorithm. The typed register model (§2.15.1)
provides complete type information for every register at every program point, enabling **precise root scanning** — the
runtime should use this to identify GC references exactly rather than conservatively scanning memory. A generational or
incremental collector is recommended for game workloads to minimize stop-the-world pause times.

**Finalization ordering:** When multiple unreachable objects are collected in the same GC cycle, the order in which
`on finalize` hooks execute is implementation-defined. Finalizers must not assume that other managed objects referenced
by the finalizing object are in a valid state or have not yet been finalized. Deterministic cleanup logic belongs in
`on destroy` (for entities) or application-level teardown, not in finalizers.

**Finalization execution:** Finalizer hooks should be queued during GC tracing and executed as tasks during a subsequent
scheduling pass — not during the GC pause itself. This allows finalizer code to execute normally, including suspension
at transition points.

### 2.9.8 IL Implications

- `MOV` copies register contents. For references, this copies the pointer — no deep copy, no clone.
- No `FREE` / `DEALLOC` instructions exist. The GC handles all reclamation.
- `NEW`, `NEW_ARRAY`, `NEW_ENUM`, `SPAWN_ENTITY` are allocation points. The GC may trigger during any allocation.
- **GC safepoints** are a runtime concern, not an IL concern. The runtime can GC at any instruction boundary because
  type metadata enables precise root scanning.
- Dead entity access requires a liveness check in the runtime on field/method access through entity handles.

