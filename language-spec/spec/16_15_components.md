# 1. Writ Language Specification
## 15. Components

Components are data schemas for composable behaviors that can be attached to entities via `use`. Components are always
engine-provided (`extern`) and contain only field declarations — no methods. The host engine owns component storage and
behavior; the script language defines the schema for compile-time type checking and field access.

### 15.1 Component Declarations

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

extern component Health {
    current: int,
    max: int,
}
```

### 15.2 Component Access

Script code reads and writes component fields directly. Components have no script-defined methods — any logic involving
component data is written as entity methods or free functions.

```
// Direct field access
guard[Health].current -= 10;
if guard[Health].current <= 0 {
    Entity.destroy(guard);
}

// Reading component fields
let isVisible = guard[Sprite].visible;
guard[Sprite].texture = "res://sprites/guard_alert.png";
```

### 15.3 Runtime Behavior

Component field reads and writes on extern components are proxied through the host API. When script code writes
`guard[Sprite].visible = false`, the runtime sends the field change to the host engine, which updates the native
representation. The runtime suspends execution until the host confirms the change has been processed, ensuring
consistency with the game engine's logic loop.

> **Note:** Components are not GC-managed script objects. They are host-owned data accessed through the entity handle.
> The `self.entity` back-reference is a compiler-emitted hidden field (using an internal name like `@entity` that is
> unreachable from Writ source code). The compiler sets this field during `SPAWN_ENTITY` and uses it when lowering
> component access expressions. It is not a user-facing language feature.

---

