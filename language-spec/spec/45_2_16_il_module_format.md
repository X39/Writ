# Writ IL Specification
## 2.16 IL Module Format

The compiled IL is stored in a binary module format. Each module is a self-contained compilation unit that may reference
types and methods from other modules. At load time, the runtime loads all modules into a single domain and resolves
cross-module references by name.

### 2.16.1 Binary Container

**Magic and version:**

```
Bytes 0–3:   0x57 0x52 0x49 0x54  ("WRIT")
Bytes 4–5:   u16 format_version    (starts at 1, bumps on incompatible layout changes)
Bytes 6–7:   u16 flags             (bit 0 = debug info present, rest reserved)
```

**Module header** (fixed layout, immediately after the magic):

```
module_name:        u32   // string heap offset
module_version:     u32   // string heap offset — semver string (§2.16.3)

string_heap_offset: u32
string_heap_size:   u32
blob_heap_offset:   u32
blob_heap_size:     u32

table_directory:    [offset: u32, row_count: u32] × 21
```

The table directory has a fixed-order entry for each of the 21 metadata tables (§2.16.5). Empty tables have
`row_count = 0`. Total header size: 8 (magic/version/flags) + 8 (name/version) + 16 (heaps) + 168 (table directory)
= **200 bytes**.

**String Heap:** Length-prefixed UTF-8. Each entry is `u32(byte_length)` followed by the string bytes. No null
terminator. Offset 0 is reserved as the empty/null string.

**Blob Heap:** Same encoding as the string heap (length-prefixed byte sequences). Stores TypeRef encodings (§2.15.3),
method signatures, constant values, and component override data.

**Byte order:** Little-endian throughout (§2.5). Table rows are aligned to 4-byte boundaries.

### 2.16.2 Multi-Module Architecture

Modules may depend on other modules. Dependencies are declared in the **ModuleRef** table and must form a directed
acyclic graph (DAG) — circular dependencies are forbidden. The runtime loads all modules into a single domain and
resolves cross-module references at load time.

**Cross-module references** are name-based:

- **TypeRef** rows reference a type in another module by `(ModuleRef, namespace, name)`. At load time, the runtime
  resolves each TypeRef to a TypeDef in the target module.
- **MethodRef** rows reference a method by `(parent type, name, signature)`. Resolved to a MethodDef at load time.
- **FieldRef** rows reference a field by `(parent type, name, type signature)`. Resolved to a FieldDef at load time.
  This provides ABI-safe cross-module field access — recompiling a dependency that reorders fields does not break
  dependent modules as long as field names and types are preserved.

After load-time resolution, cross-module references are equivalent to direct local references. The resolution cost is
paid once at load time.

**Compilation model:** The compiler needs access to referenced modules' metadata at compile time to emit correct
TypeRef, MethodRef, and FieldRef entries. This is analogous to include paths — the build system provides paths to
dependency modules, and the compiler reads their metadata tables.

### 2.16.3 Module Versioning

Each module declares its version as a **Semantic Versioning 3.0.0** (semver) string in the format `MAJOR.MINOR.PATCH`:

- **MAJOR** — incremented for breaking changes (removed types, changed signatures, incompatible behavior).
- **MINOR** — incremented for backwards-compatible additions (new types, new methods, new fields with defaults).
- **PATCH** — incremented for backwards-compatible bug fixes.

Semantic Versioning is a widely adopted convention that encodes compatibility information in a version number. The key
principle is that consumers can safely upgrade within the same major version. A change from `2.2.0` to `2.3.0` is safe
(new features, nothing removed). A change from `1.x` to `3.0.0` signals breaking changes that require consumer updates.

**Compatibility rule:** A loaded module with version `A.B.C` satisfies a dependency requirement of `>=X.Y.Z` when
`A == X` and `(A, B, C) >= (X, Y, Z)` by lexicographic comparison. The major version must match exactly (a major
version change signals breaking incompatibility); the minor and patch versions must be equal to or greater than the
requirement.

**ModuleRef** entries include a `min_version` field. At load time, the runtime checks that each dependency's version
satisfies the requirement. On failure, the runtime logs the mismatch (§2.14.7) and may refuse to load or proceed at the
host's discretion.

### 2.16.4 Metadata Tokens

Instructions and metadata entries reference types, methods, and fields via **metadata tokens** — u32 values encoding
both the target table and the row index:

```
Bits 31–24:  table ID (0–20, matching the table directory order in §2.16.5)
Bits 23–0:   row index (1-based; 0 = null token)
```

This gives 24-bit row indices (up to 16,777,215 rows per table per module).

**Examples:**

| Token         | Meaning                                                        |
|---------------|----------------------------------------------------------------|
| `0x02_000005` | TypeDef row 5 (type defined in this module)                    |
| `0x03_000003` | TypeRef row 3 (type in another module — resolved at load time) |
| `0x07_00000A` | MethodDef row 10 (method defined here)                         |
| `0x08_000002` | MethodRef row 2 (method in another module)                     |

After load-time resolution, the runtime may remap cross-module tokens internally. The token encoding is a
storage/interchange format — the runtime's internal representation is implementation-defined.

### 2.16.5 Metadata Tables

All tables have **fixed-size rows**. References to heaps are u32 offsets. References to other tables are metadata tokens
(§2.16.4). Tables use the **list ownership** pattern: a parent's `xxx_list` field gives the index of the first child
row, and the range extends to the next parent's `xxx_list` value (or end of table).

| #  | Table                 | Key Fields                                                                               | Purpose                                               |
|----|-----------------------|------------------------------------------------------------------------------------------|-------------------------------------------------------|
| 0  | **ModuleDef**         | name(str), version(str), flags(u32)                                                      | Module identity (always 1 row)                        |
| 1  | **ModuleRef**         | name(str), min_version(str)                                                              | Dependencies on other modules                         |
| 2  | **TypeDef**           | name(str), namespace(str), kind(u8), flags(u16), field_list, method_list                 | Types defined in this module                          |
| 3  | **TypeRef**           | scope(token:ModuleRef), name(str), namespace(str)                                        | Types in other modules (resolved at load time)        |
| 4  | **TypeSpec**          | signature(blob)                                                                          | Instantiated generic types (TypeDef + type arguments) |
| 5  | **FieldDef**          | name(str), type_sig(blob), flags(u16)                                                    | Fields on types defined here                          |
| 6  | **FieldRef**          | parent(token), name(str), type_sig(blob)                                                 | Fields in other modules (resolved at load time)       |
| 7  | **MethodDef**         | name(str), signature(blob), flags(u16), body_offset(u32), body_size(u32), reg_count(u16) | Methods/functions defined here                        |
| 8  | **MethodRef**         | parent(token), name(str), signature(blob)                                                | Methods in other modules (resolved at load time)      |
| 9  | **ParamDef**          | name(str), type_sig(blob), sequence(u16)                                                 | Method parameters                                     |
| 10 | **ContractDef**       | name(str), namespace(str), method_list, generic_param_list                               | Contract declarations                                 |
| 11 | **ContractMethod**    | name(str), signature(blob), slot(u16)                                                    | Method slots within a contract                        |
| 12 | **ImplDef**           | type(token), contract(token), method_list                                                | Contract implementations                              |
| 13 | **GenericParam**      | owner(token), owner_kind(u8), ordinal(u16), name(str)                                    | Type parameters on types/methods                      |
| 14 | **GenericConstraint** | param(row:GenericParam), constraint(token)                                               | Bounds on type parameters                             |
| 15 | **GlobalDef**         | name(str), type_sig(blob), flags(u16), init_value(blob)                                  | Constants and `global mut` variables                  |
| 16 | **ExternDef**         | name(str), signature(blob), import_name(str), flags(u16)                                 | Extern function/type declarations                     |
| 17 | **ComponentSlot**     | owner_entity(token:TypeDef), component_type(token)                                       | Entity → component bindings                           |
| 18 | **LocaleDef**         | dlg_method(token:MethodDef), locale(str), loc_method(token:MethodDef)                    | Dialogue locale dispatch                              |
| 19 | **ExportDef**         | name(str), item_kind(u8), item(token)                                                    | Convenience index of pub-visible items                |
| 20 | **AttributeDef**      | owner(token), owner_kind(u8), name(str), value(blob)                                     | Metadata attributes ([Singleton], etc.)               |

**TypeDef.kind:** `0 = struct`, `1 = enum`, `2 = entity`, `3 = component`.

**MethodDef.flags** includes: visibility (pub/private), is_static, is_mut_self, hook_kind (0=none, 1=create, 2=destroy,
3=finalize, 4=serialize, 5=deserialize, 6=interact), and an **intrinsic** flag for `writ-runtime` native
implementations (§2.16.8).

**FieldDef.flags** includes: visibility (pub/private), has_default, is_component_field.

### 2.16.6 Method Body Layout

Each method body starts at the MethodDef's `body_offset` and occupies `body_size` bytes:

```
MethodBody {
    register_types: u32[reg_count]    // blob heap offsets — one TypeRef per register
    code_size:      u32
    code:           u8[code_size]     // instruction stream

    // Present only if module flags bit 0 (debug) is set:
    debug_local_count:  u16
    debug_locals:       DebugLocal[debug_local_count]
    source_span_count:  u32
    source_spans:       SourceSpan[source_span_count]
}
```

**Register type table:** `reg_count` (from MethodDef) entries, each a u32 blob heap offset pointing to a TypeRef
encoding (§2.15.3). The runtime reads these at method load to determine per-register storage requirements. Common
TypeRefs are naturally deduplicated in the blob heap.

**Debug info** (optional):

```
DebugLocal  { register: u16, name: u32(str_offset), start_pc: u32, end_pc: u32 }
SourceSpan  { pc: u32, line: u32, column: u16 }
```

No defer table or exception table is needed in the method body. The defer stack is runtime state managed by
`DEFER_PUSH`/`DEFER_POP` instructions. Writ has no try/catch, so no exception handler table.

### 2.16.7 Entity Construction Buffering

During entity construction, component field writes are **buffered** by the runtime and delivered to the host as a single
batch when `INIT_ENTITY` executes. This avoids per-field round-trips through suspend-and-confirm (§2.14.2) during
construction.

**Construction sequence:**

1. `SPAWN_ENTITY r, type_token` — Allocate entity in the runtime's heap. Set the entity's internal "under construction"
   flag. Notify the host with the component list (from the ComponentSlot table) so it can prepare native
   representations.
2. `SET_FIELD r, field_token, r_val` on **script fields** — Written directly to the script heap. No host involvement.
3. `SET_FIELD r, field_token, r_val` on **component fields** — **Buffered** by the runtime. Not sent to host.
4. `INIT_ENTITY r` — Flush all buffered component field values to the host as a single batch. Clear the "under
   construction" flag. Fire the `on_create` lifecycle hook.

**Safety invariant:** Every `SPAWN_ENTITY` must be followed by exactly one `INIT_ENTITY` for the same entity before the
enclosing frame returns. If a frame exits with an entity still in "under construction" state, the runtime crashes the
task and logs the error (§2.14.7). The compiler guarantees this pairing — `INIT_ENTITY` is always emitted as part of
the `new Entity { ... }` lowering.

**After construction:** `SET_FIELD` on component fields goes to the host immediately via suspend-and-confirm (§2.14.2).
Buffering applies only during the SPAWN_ENTITY → INIT_ENTITY construction window.

### 2.16.8 The `writ-runtime` Module

The `writ-runtime` module is a **runtime-provided module** containing core type definitions that the compiler and IL
instructions depend on. Unlike normal modules, `writ-runtime` is not compiled from Writ source — the runtime provides
it as part of its implementation. The spec mandates what types this module must contain and what layouts they must have.
The runtime is free to implement them however it chooses internally.

Methods on `writ-runtime` types may carry an **intrinsic** flag on their MethodDef entries, indicating that the runtime
provides a native implementation rather than IL bytecode. This allows core operations (such as contract implementations
on primitive types) to execute as optimized native code while appearing as normal methods in the metadata for generic
dispatch, reflection, and cross-module referencing.

A separate **`writ-std`** module (a standard library written in Writ) may provide utility types like `List<T>`,
`Map<K, V>`, and common helper functions. Unlike `writ-runtime`, `writ-std` is ordinary Writ code compiled to a normal
module. It imports from `writ-runtime` via standard ModuleRef resolution. `writ-std` is not required for the language to
function — it is a convenience library that can be implemented incrementally.

From the module format's perspective, `writ-runtime` is an ordinary module — its specialness is that the runtime
provides it and the spec mandates its contents.

**Contents of `writ-runtime`:** See §2.18 for the complete manifest of types, contracts, and intrinsic methods that
this module must provide.

