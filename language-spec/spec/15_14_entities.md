# Writ Language Specification
## 14. Entities

Entities are game objects declared with the `entity` keyword. They combine properties, components (`use`), lifecycle
hooks (`on`), and methods. Entities lower to structs with component fields, auto-generated contract implementations, and
engine registrations.

### 14.1 Entity Declaration

```
entity Guard {
    // Properties (with defaults)
    name: string = "Guard",
    patrolRoute: List<vec2> = List::new(),

    // Components
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
    use Health {
        current: 80,
        max: 80,
    },

    // Methods
    fn greet() -> string {
        $"Halt! I am {self.name}"
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

### 14.2 Spawning Entities

Entities are instantiated with `spawn`. Properties can be overridden at spawn time.

```
let guard = spawn Guard {
    name: "Steve",
    patrolRoute: [vec2(0, 0), vec2(10, 0), vec2(10, 10)],
};

// Spawn with all defaults
let defaultGuard = spawn Guard {};
```

### 14.3 Component Access

Components are accessed via `[]` indexing by type. For components declared in the entity definition, access is
guaranteed non-null. For arbitrary Entity references, component access returns `Option`.

```
// On a known entity type — guaranteed, no optional
guard[Health].damage(10);
guard[Sprite].visible = false;

// On a generic Entity reference — returns Option
fn healAnyone(target: Entity, amount: int) {
    if let Option::Some(hp) = target[Health] {
        hp.heal(amount);
    }
}

// Unwrap if confident
target[Health]!.damage(10);
```

> **Note:** If two components have a method with the same name, direct method call on the entity is a compile error. Use
> explicit component access: `self[Health].reset()` vs `self[Mana].reset()`.

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

    fn addMember(e: Entity) {
        self.members.add(e);
    }

    fn healAll(amount: int) {
        for member in self.members {
            if let Option::Some(hp) = member[Health] {
                hp.heal(amount);
            }
        }
    }
}
```

### 14.6 Entity Lowering

Entities lower to structs with component fields, plus auto-generated contract implementations from `use` declarations
and lifecycle hook registrations. The runtime handles component storage and entity lifecycle.

---

