# Writ IL Specification
## 3.8 Object Model

| Mnemonic          | Shape | Operands                           | Description                                                                                                                                                              |
|-------------------|-------|------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `NEW`             | RI32  | r_dst, type_idx:u32                | Allocate struct instance. Memory is zeroed; defaults and overrides applied via subsequent SET_FIELD instructions.                                                        |
| `GET_FIELD`       | var   | r_dst, r_obj, field_idx:u32        | Read a field from a struct/entity/component. Encoding: `u16(op) u16(r_dst) u16(r_obj) u32(field_idx)` = 10B.                                                             |
| `SET_FIELD`       | var   | r_obj, field_idx:u32, r_val        | Write a field. Encoding: `u16(op) u16(r_obj) u32(field_idx) u16(r_val)` = 10B.                                                                                           |
| `SPAWN_ENTITY`    | RI32  | r_dst, type_idx:u32                | Allocate entity instance, register with entity runtime, notify host to create components with defaults and overrides. Does NOT fire `on_create`.                         |
| `INIT_ENTITY`     | R     | r_entity                           | Fire the entity's `on_create` lifecycle hook. Must be called after field overrides (SET_FIELD) are applied.                                                              |
| `GET_COMPONENT`   | var   | r_dst, r_entity, comp_type_idx:u32 | Access a component on an entity by component type. Returns `Option<Component>`: Some if the entity has the component, None otherwise. Encoding: same as GET_FIELD (10B). |
| `GET_OR_CREATE`   | RI32  | r_dst, singleton_type_idx:u32      | `Entity.getOrCreate<T>()`. Returns the singleton instance, creating it if it doesn't exist.                                                                              |
| `FIND_ALL`        | RI32  | r_dst, entity_type_idx:u32         | `Entity.findAll<T>()`. Returns an EntityList of all live entities of the given type.                                                                                     |
| `DESTROY_ENTITY`  | R     | r_entity                           | Destroy an entity. Fires `on_destroy`, marks entity as dead in the registry, notifies host. Crashes if entity is already dead.                                           |
| `ENTITY_IS_ALIVE` | RR    | r_dst, r_entity                    | Check if entity handle refers to a live entity. Returns bool in r_dst. Does not crash on dead handles.                                                                   |

Construction sequence for `new Guard { name: "Steve" }`:

```
SPAWN_ENTITY  r0, Guard_type      // allocate, register, notify host for components
LOAD_STRING   r1, "Steve"_idx     // load the override value
SET_FIELD     r0, name_field, r1  // override the name field
INIT_ENTITY   r0                  // fire on_create
```

