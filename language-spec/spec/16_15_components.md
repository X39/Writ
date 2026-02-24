# Writ Language Specification
## 15. Components

Components are composable behaviors that can be attached to entities via `use`. They can be engine-provided (extern) or
script-defined.

### 15.1 External Components (Engine-Provided)

```
extern component Sprite {
    texture: string,
    scale: float = 1.0,
    visible: bool = true,
}

extern component Collider {
    shape: string,
    width: float,
    height: float,
}

// The Speaker component is used for dialogue attribution
extern component Speaker {
    displayName: string,
    color: string = "#FFFFFF",
    portrait: string = "",
    voice: string = "",
}
```

### 15.2 Script-Defined Components

```
component Health {
    current: int,
    max: int,

    fn damage(amount: int) {
        self.current = self.current - amount;
        if self.current <= 0 {
            self.owner.destroy();
        }
    }

    fn heal(amount: int) {
        self.current = min(self.current + amount, self.max);
    }

    fn isDead() -> bool {
        self.current <= 0
    }
}
```

> **Note:** `self.owner` inside a component refers to the entity the component is attached to, typed as `Entity`. This
> allows components to interact with their owning entity.

---

