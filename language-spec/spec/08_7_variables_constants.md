# 1. Writ Language Specification
## 7. Variables & Constants

### 7.1 Variable Declarations

Variables are declared with `let` (immutable) or `let mut` (mutable). Immutability is the default. Type is inferred from
the initializer or can be explicitly annotated.

```
let name = "Aria";              // immutable, inferred string
let mut health = 100;            // mutable, inferred int
let pos: vec2 = vec2 { x: 0.0, y: 0.0 };  // explicit type annotation

name = "Bob";      // COMPILE ERROR: name is immutable
health += 10;       // ok: health is mutable
```

### 7.2 Shadowing

Variables can be shadowed by a new `let` declaration in the same scope. The new binding can have a different type. This
allows transformations without mutability.

```
let x = 10;
let x = x * 2;      // shadows, x is now 20 (still immutable)
let x = "hello";    // shadows again, different type
```

### 7.3 Constants

Constants are declared with `const` at the top level. They must have a compile-time known value.

```
const MAX_HEALTH: int = 100;
const GAME_TITLE: string = "My RPG";
const DEFAULT_SPEED: float = 5.0;
```

---

