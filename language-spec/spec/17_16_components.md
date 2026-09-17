# 1.16 Components

Components are data schemas for composable behaviors that can be attached to entities via `use`. Components are always
engine-provided (`extern`) and contain only field declarations — no methods. The host engine owns component storage and
behavior; the script language defines the schema for compile-time type checking and field access.

## 1.16.1 Component Declarations

```writ
extern component Sprite {
    mut texture: string,
    scale: float = 1.0,
    mut visible: bool = true,
}

extern component Collider {
    shape: string,
    mut width: float,
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
    mut current: int,
    max: int,
}
```

## 1.16.2 Component Access

Script code reads and writes component fields directly. Components have no script-defined methods — any logic involving
component data is written as entity methods or free functions.

```writ
// Direct field access
let mut health = guard[Health];
health.current -= 10;
if health.current <= 0 {
    Entity.destroy(guard);
}

// Reading component fields
let isVisible = guard[Sprite].visible;
let mut sprite = guard[Sprite];
sprite.texture = "res://sprites/guard_alert.png";
```

## 1.16.3 Runtime Behavior

Component field reads and writes on extern components are proxied through the host API. When script code writes
`sprite.visible = false`, the runtime sends the field change to the host engine, which updates the native
representation. The runtime suspends execution until the host confirms the change has been processed, ensuring
consistency with the game engine's logic loop.

> **Note:** Components are not GC-managed script objects. They are host-owned data accessed through the entity handle.
> During `SPAWN_ENTITY`, the runtime and host associate each component instance with its owning entity. Lowered
> component access uses that hidden association; it is not a source field, cannot be named by Writ code, and is not
> initialized through `SET_FIELD`.

---

