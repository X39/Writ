# 3.8 Object Model

| Mnemonic          | Shape | Operands                                                     | Description                                                                                                                                                              |
|-------------------|-------|--------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `NEW`             | var   | r_dst, type_token:u32, field_count:u16, r_base                | Validate and construct a complete struct or class from the declaration-ordered initializer block. Encoding: `u16(op) u16(r_dst) u32(type_token) u16(field_count) u16(r_base)` = 12B. |
| `GET_FIELD`       | var   | r_dst, r_obj, field_token:u32                                | Read a field selected by a FieldDef or FieldRef metadata token. Encoding: `u16(op) u16(r_dst) u16(r_obj) u32(field_token)` = 10B.                                         |
| `SET_FIELD`       | var   | r_obj, field_token:u32, r_val                                | Perform a post-construction write selected by a FieldDef or FieldRef token. A read-only field crashes before mutation. Encoding: `u16(op) u16(r_obj) u32(field_token) u16(r_val)` = 10B. |
| `SPAWN_ENTITY`    | var   | r_dst, type_token:u32, field_count:u16, r_base                | Validate and create a pending entity with its complete script-field vector, then register it and notify the host. Does not fire `on_create`. Encoding is 12B as for `NEW`. |
| `INIT_ENTITY`     | R     | r_entity                                                     | Transition a complete pending entity to alive, complete host initialization, and fire `on_create`.                                                                        |
| `GET_COMPONENT`   | var   | r_dst, r_entity, comp_type_idx:u32 | Access a component on an entity by component type. Returns `Option<Component>`: Some if the entity has the component, None otherwise. Encoding: same as GET_FIELD (10B). |
| `GET_OR_CREATE`   | RI32  | r_dst, singleton_type_idx:u32      | `Entity.getOrCreate<T>()`. Returns an existing singleton. The current creation form is valid only for a zero-script-field entity; otherwise it crashes and source must use `new` with explicit/default initializers. |
| `FIND_ALL`        | RI32  | r_dst, entity_type_idx:u32         | `Entity.findAll<T>()`. Returns an EntityList of all live entities of the given type.                                                                                     |
| `DESTROY_ENTITY`  | R     | r_entity                           | Destroy an entity. Fires `on_destroy`, marks entity as dead in the registry, notifies host. Crashes if entity is already dead.                                           |
| `ENTITY_IS_ALIVE` | RR    | r_dst, r_entity                    | Check if entity handle refers to a live entity. Returns bool in r_dst. Does not crash on dead handles.                                                                   |

The `field_token` operand is always a non-null metadata token:

- A table-5 FieldDef token names an absolute row in the current module's FieldDef table. It is not the field's local
  offset in an object.
- A table-6 FieldRef token names a FieldRef row in the current module. The loader resolves that row to the defining
  module, declaring TypeDef, FieldDef row, and local object-layout offset.

For both forms, the runtime validates the table tag, non-zero row, row bounds, resolution result, and receiver owner
before accessing heap or entity storage. The receiver's canonical `(module, TypeDef)` identity must equal the field's
declaring owner. A forged token, unresolved reference, owner mismatch, receiver without a canonical owner, or token
from any other metadata table is a runtime error and crashes the current task. Raw zero-based field ordinals are not
accepted. For `SET_FIELD`, the runtime also resolves the final FieldDef and checks its `READONLY` bit before touching
storage. This check applies equally to local FieldDef and cross-module FieldRef operands.

For `NEW` and `SPAWN_ENTITY`, `field_count` must exactly equal the resolved TypeDef field count and the complete
`r_base..r_base+field_count-1` range must exist. The runtime validates these conditions, the destination register, the
type token, and the permitted kind before allocation or any other side effect. `NEW` accepts only Struct and Class;
`SPAWN_ENTITY` accepts only Entity. A zero count makes `r_base` irrelevant. Invalid operands crash the current task
without changing `r_dst`.

Construction sequence for `new Vec2 { x: 1.0, y: 2.0 }` (struct -- value type):

```
LOAD_FLOAT    r1, 1.0
LOAD_FLOAT    r2, 2.0
NEW           r0, Vec2_type, 2, r1 // complete x/y initializer block
// No on_create call -- value-type structs have no lifecycle hooks
```

Construction sequence for `new Merchant { name: "Tim", gold: 100 }` (class -- reference type):

```
LOAD_STRING   r1, "Tim"_idx
LOAD_INT      r2, 100
NEW           r0, Merchant_type, 2, r1
CALL          r_, Merchant::__on_create, r0  // run on_create hook
```

Construction sequence for `new Guard { name: "Steve" }` (entity):

```
LOAD_STRING   r1, "Steve"_idx
SPAWN_ENTITY  r0, Guard_type, 1, r1 // complete script fields, then register/notify
INIT_ENTITY   r0                     // publish as alive and fire on_create
```

