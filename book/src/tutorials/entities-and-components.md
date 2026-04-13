# Entities and Components

Entities are Writ's game objects. They combine properties, components, methods, and lifecycle hooks into a single declaration. This tutorial builds up from basic entity declaration to the full entity-component pattern.

## Declaring an Entity

An entity is declared with the `entity` keyword. Properties have names, types, and optional defaults:

```writ
entity Guard {
    name: string = "Guard",
    health: int = 80,
    maxHealth: int = 80,
}
```

## Creating Entities

Construct entities with `new` and brace syntax. Override any properties you want:

```writ
let guard = new Guard {
    name: "Steve",
};

// Or use all defaults
let defaultGuard = new Guard {};
```

## Singleton Entities

Entities marked `[Singleton]` have at most one instance. Access them with `Entity.getOrCreate<T>()`:

```writ
[Singleton]
entity Narrator {}

pub fn main() {
    let narrator = Entity.getOrCreate<Narrator>();
    ::say(narrator, "Hello, World!");
}
```

This is the standard pattern for dialogue speakers -- the Narrator from [Hello World](../getting-started/hello-world.md) is a singleton entity.

`getOrCreate` returns the existing instance if one exists, or creates a new one. This makes singletons safe to reference from anywhere without worrying about initialization order.

## Entity Methods

Entities can have methods. Use `self` to access the entity's properties:

```writ
entity Guard {
    name: string = "Guard",
    health: int = 80,
    maxHealth: int = 80,

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
}
```

Use `mut self` when the method modifies the entity's properties.

## Components

Components are extern, data-only types provided by the host engine. Declare them with `use` inside an entity:

```writ
entity Guard {
    name: string = "Guard",
    health: int = 80,

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
}
```

Access components with `[]` indexing by type:

```writ
guard[Sprite].visible = false;
guard[Collider].width = 48;
```

On a known entity type, component access is guaranteed non-null. On a generic `Entity` reference, it returns `Option`:

```writ
fn checkHealth(target: Entity) {
    if let Option::Some(hp) = target[Health] {
        if hp.current <= 0 {
            ::log::info("Target is dead");
        }
    }
}
```

## Lifecycle Hooks

Entities support `on` hooks for lifecycle events:

```writ
entity Guard {
    name: string = "Guard",

    on create {
        ::log::info($"Guard spawned: {self.name}");
    }

    on interact(who: Entity) {
        -> guardDialog(self, who)
    }

    on destroy {
        dropLoot(self);
    }
}
```

| Hook | When it fires |
|------|---------------|
| `on create` | After the entity is fully constructed |
| `on destroy` | When `Entity.destroy()` is called |
| `on interact(who)` | When another entity interacts with this one |

## Entities and Dialogue

Entities are the natural speakers in dialogue blocks. Singleton entities can be referenced by name with `@`:

```writ
[Singleton]
entity OldTim {
    use Speaker { displayName: "Old Tim" },
}

dlg shopScene(customer: Entity) {
    @OldTim Welcome, traveler!
    @customer Who, me?
    @OldTim Yes, you! Come see my wares.
}
```

Speaker resolution in `@`:
1. Check local variables and parameters first
2. Check `[Singleton]` entities with a `Speaker` component
3. Otherwise, compile error

For the full entity reference, see [Entities](../language-ref/entities.md) and [Components](../language-ref/components.md).
