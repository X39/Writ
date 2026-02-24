# 1. Writ Language Specification
## 14. Entities

Entities are game objects declared with the `entity` keyword. They combine properties, components (`use`), lifecycle
hooks (`on`), and methods. Entities lower to structs with component fields, auto-generated contract implementations, and
engine registrations.

### 14.1 Entity Declaration

```
entity Guard {
    // Properties (with defaults)
    name: string = "Guard",
    health: int = 80,
    maxHealth: int = 80,
    patrolRoute: List<vec2> = List::new(),

    // Components (extern, data-only — provided by host engine)
    use Speaker {
        displayName: "Guard",
    },
    use Sprite {
        texture: "res://sprites/guard.png",
    },
    use Collider {
        shape: "rect",
        width: 32,
        height: 48,
    },

    // Methods
    fn greet(self) -> string {
        $"Halt! I am {self.name}"
    }

    fn damage(mut self, amount: int) {
        self.health -= amount;
        if self.health <= 0 {
            Entity.destroy(self);
        }
    }

    fn heal(mut self, amount: int) {
        self.health = min(self.health + amount, self.maxHealth);
    }

    // Lifecycle hooks
    on create {
        log($"Guard spawned: {self.name}");
    }

    on interact(who: Entity) {
        -> guardDialog(self, who)
    }

    on destroy {
        dropLoot(self);
    }
}
```

### 14.2 Creating Entities

Entities are constructed with the `new` keyword and brace syntax. The compiler knows the type is an entity and generates
the appropriate IL (entity registration, component attachment, lifecycle hooks). Properties can be overridden.

```
let guard = new Guard {
    name: "Steve",
    patrolRoute: [new vec2 { x: 0, y: 0 }, new vec2 { x: 10, y: 0 }],
};

// Construct with all defaults
let defaultGuard = new Guard {};
```

### 14.3 Component Access

Components are accessed via `[]` indexing by type. Components are extern and data-only — script code reads and writes
their fields directly. For components declared in the entity definition, access is guaranteed non-null. For arbitrary
Entity references, component access returns `Option`.

```
// On a known entity type — guaranteed, no optional
guard[Sprite].visible = false;
guard[Collider].width = 48;

// On a generic Entity reference — returns Option
fn checkHealth(target: Entity) {
    if let Option::Some(hp) = target[Health] {
        if hp.current <= 0 {
            log("Target is dead");
        }
    }
}

// Unwrap if confident
let hp = target[Health]!.current;
```

> **Note:** If two components have a field with the same name, accessing it directly on the entity is a compile error.
> Use explicit component access: `self[Health].current` vs `self[Mana].current`.

### 14.4 Singleton Entities

Entities marked with the `[Singleton]` attribute are guaranteed to have at most one instance. They are accessed via
`Entity.getOrCreate<T>()`, which returns the existing instance or creates one. This is the mechanism used for
globally-referenced speakers in dialogue.

```
[Singleton]
entity Narrator {
    use Speaker {
        displayName: "The Narrator",
        color: "#CCCCCC",
    },
}

[Singleton]
entity OldTim {
    use Speaker {
        displayName: "Old Tim",
        color: "#AA8833",
        portrait: "res://portraits/tim.png",
    },
    use Sprite {
        texture: "res://sprites/tim.png",
    },
    gold: int = 500,

    on interact(who: Entity) {
        -> shopDialog(who)
    }
}

// Explicit access in code
let tim = Entity.getOrCreate<OldTim>();
tim.gold -= 10;

// In dialogue, @OldTim auto-resolves via Entity.getOrCreate<OldTim>()
dlg shopDialog(customer: Entity) {
    @Narrator You enter the shop.
    @OldTim Welcome, traveler!
    $ let tim = Entity.getOrCreate<OldTim>();
    $ tim.gold -= 10;
    @OldTim Here, a discount for you.
}
```

### 14.5 Entity References & EntityList

Entities reference each other by handle. The `EntityList<T>` type provides a typed collection for managing groups of
entities.

```
entity Party {
    leader: Player,
    members: EntityList<Entity> = EntityList::new(),

    fn addMember(mut self, e: Entity) {
        self.members.add(e);
    }

    fn healAll(self, amount: int) {
        for member in self.members {
            if let Option::Some(hp) = member[Health] {
                hp.current = min(hp.current + amount, hp.max);
            }
        }
    }
}
```

### 14.5.1 Entity Handles

Entity references are runtime-managed **handles** — opaque identifiers that the runtime resolves against its internal
entity registry. Unlike structs (which are direct GC references to heap objects), entity handles add an indirection
layer
because entities can be explicitly destroyed while other code still holds references to them.

After `Entity.destroy(entity)` is called:

- Existing handles are **not** invalidated or nulled. They remain valid values that can be stored, passed, and compared.
- Accessing fields, components, or methods through a dead handle **crashes the task** — same severity as unwrapping
  None.
- Use `Entity.isAlive(entity)` to check whether a handle refers to a live entity without crashing.

The GC manages the handle objects themselves. An entity's memory is only collected after it is both destroyed (or never
explicitly destroyed) AND unreachable from all GC roots. A dead handle that is still referenced keeps the handle object
alive in the GC, but the underlying entity state is gone.

```
let guard = new Guard {};
let ref = guard;                // both guard and ref hold handles to the same entity
Entity.destroy(guard);          // entity destroyed — on_destroy fires, marked dead
Entity.isAlive(ref);            // false
// ref.name;                    // would crash — dead handle
```

### 14.5.2 Entity Static Methods

The `Entity` namespace provides static methods for entity lifecycle and queries:

| Method               | Signature                            | Behavior                                                                   |
|----------------------|--------------------------------------|----------------------------------------------------------------------------|
| `Entity.destroy`     | `fn destroy(entity: Entity)`         | Destroy an entity. Fires `on destroy`, marks dead, notifies host.          |
| `Entity.isAlive`     | `fn isAlive(entity: Entity) -> bool` | Check if a handle refers to a live entity. Does not crash on dead handles. |
| `Entity.getOrCreate` | `fn getOrCreate<T>() -> T`           | Get or create a singleton entity (see §14.4).                              |
| `Entity.findAll`     | `fn findAll<T>() -> EntityList<T>`   | Find all live entities of a type.                                          |

`Entity.destroy` and `Entity.isAlive` lower to dedicated IL instructions (`DESTROY_ENTITY`, `ENTITY_IS_ALIVE`).
`Entity.getOrCreate` and `Entity.findAll` lower to `GET_OR_CREATE` and `FIND_ALL` respectively.

### 14.6 Lifecycle Hooks

Entities support all the universal lifecycle hooks (shared with structs) plus entity-specific hooks. All hooks receive
an implicit `mut self` parameter.

#### 14.6.1 Universal Hooks

| Hook             | When                                            | Purpose                                 |
|------------------|-------------------------------------------------|-----------------------------------------|
| `on create`      | After all fields and components are initialized | Post-initialization logic               |
| `on finalize`    | GC is about to collect the entity               | Last-chance cleanup of native resources |
| `on serialize`   | Before the entity is serialized                 | Park native state                       |
| `on deserialize` | After the entity is deserialized                | Recreate native state                   |

#### 14.6.2 Entity-Specific Hooks

| Hook                       | When                               | Purpose                                           |
|----------------------------|------------------------------------|---------------------------------------------------|
| `on destroy`               | `Entity.destroy(entity)` is called | Deterministic cleanup, loot drops, deregistration |
| `on interact(who: Entity)` | Host fires interaction event       | Game-specific interaction logic                   |

`on destroy` is distinct from `on finalize`. Destruction is explicit and deterministic — the script calls
`Entity.destroy(entity)`. Finalization is implicit and non-deterministic — the GC collects the object when it becomes
unreachable. An entity that is destroyed will eventually be finalized by the GC, but finalization may also occur without
explicit destruction (e.g., if all references to the entity are dropped).

**Entity cleanup ordering:** When an entity is explicitly destroyed, the sequence is:

1. `on destroy` — deterministic cleanup (loot drops, deregistration, game logic).
2. The entity is marked destroyed. Accessing a destroyed entity handle crashes the task.
3. Eventually, when the GC determines the entity is unreachable: `on finalize` — native resource cleanup.
4. Runtime removes the entity from internal bookkeeping.

This ordering ensures `on finalize` always runs after `on destroy`, which is the expected pattern: `on destroy` handles
game logic, `on finalize` handles native resource cleanup (file handles, connections, etc.).

**Hook failure semantics:** If a lifecycle hook crashes, the crash unwinds and terminates the calling task's call stack.
The runtime must log the failure to the host via the runtime logging interface (see IL spec §1.14.7). An `on destroy`
crash still marks the entity as destroyed — the entity does not "survive" a failed destructor.

### 14.7 Entity Lowering

An entity declaration lowers to a TypeDef with fields, component slots, methods, and lifecycle hook registrations.

#### 14.7.1 TypeDef Generation

Each `entity` declaration produces a TypeDef in the IL metadata with kind `Entity`. The TypeDef contains:

- **Fields:** All entity properties (`name: string`, etc.) become regular fields on the TypeDef, with default values
  stored in the metadata.
- **Component slots:** Each `use Component { ... }` declaration registers a component type index on the entity type.
  Component instances are allocated and attached by the host engine during `SPAWN_ENTITY` — they are not stored as
  inline fields on the entity struct.
- **Component overrides:** Field overrides specified in `use Health { current: 80, max: 80 }` are stored in the TypeDef
  metadata and applied to the component instance during entity construction.

```
// entity Guard { name: string = "Guard", health: int = 80, use Sprite { ... }, ... }
// produces:
//   TypeDef(Guard, kind=Entity)
//     fields: [name: string, health: int, maxHealth: int, ...]
//     component_slots: [Speaker, Sprite, Collider]
//     component_overrides: [Speaker.displayName="Guard", Sprite.texture="res://...", ...]
```

#### 14.7.2 Method Lowering

Entity methods lower to regular functions with the entity handle as explicit `self`:

```
// fn greet(self) -> string { $"Halt! I am {self.name}" }
// lowers to:
//   MethodDef(Guard::greet, params=[self: Guard], returns=string)
```

#### 14.7.3 Lifecycle Hook Lowering

Lifecycle hooks lower to registered callback functions with implicit `mut self`:

| Hook                               | Lowered Signature                                | Registration                                |
|------------------------------------|--------------------------------------------------|---------------------------------------------|
| `on create { ... }`                | `fn __on_create(mut self: Guard)`                | Called after field init during construction |
| `on interact(who: Entity) { ... }` | `fn __on_interact(mut self: Guard, who: Entity)` | Called by host via "fire event"             |
| `on destroy { ... }`               | `fn __on_destroy(mut self: Guard)`               | Called by `DESTROY_ENTITY`                  |
| `on finalize { ... }`              | `fn __on_finalize(mut self: Guard)`              | Called by GC before collection              |
| `on serialize { ... }`             | `fn __on_serialize(mut self: Guard)`             | Called before serialization snapshot        |
| `on deserialize { ... }`           | `fn __on_deserialize(mut self: Guard)`           | Called after deserialization restore        |

The runtime stores these as method indices in the TypeDef metadata. `INIT_ENTITY` invokes `__on_create`.
`DESTROY_ENTITY` invokes `__on_destroy`. The host fires `on_interact` through the runtime-host interface.

#### 14.7.4 Component Access Lowering

Component access via `[]` lowers to IL instructions based on context:

- `guard[Health]` on a known entity type (component declared in entity) → `GET_COMPONENT r_dst, r_guard, Health_type`.
  The compiler knows the component exists, so the result is `Health` (not `Option<Health>`).
- `target[Health]` on a generic `Entity` reference → `GET_COMPONENT r_dst, r_target, Health_type`. Returns
  `Option<Health>` because the entity may not have that component.

#### 14.7.5 Construction Sequence

`new Guard { name: "Steve" }` compiles to the following IL:

```
SPAWN_ENTITY  r0, Guard_type      // 1. Allocate entity, notify host to create components
                                   //    with defaults and overrides
LOAD_STRING   r1, "Steve"_idx     // 2. Load override value
SET_FIELD     r0, name_field, r1  // 3. Override entity field
INIT_ENTITY   r0                  // 4. Fire on_create (calls Guard::__on_create)
```

The full sequence:

1. **SPAWN_ENTITY** — allocates the entity object, notifies the host to create all declared component instances with
   their TypeDef defaults and component overrides, and registers the entity with the entity runtime. Does NOT fire
   `on_create`.
2. **SET_FIELD** (zero or more) — applies field overrides from the construction expression.
3. **INIT_ENTITY** — fires `on_create`. At this point, all fields and components are fully initialized.

---

