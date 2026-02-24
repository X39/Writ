# Writ Language Specification
## 19. Nullability & Optionals

`T?` is syntactic sugar for `Option<T>`. All types are non-nullable by default. The `null` keyword is sugar for
`Option::None`.

```
struct Party {
    leader: Entity,         // never null
    healer: Entity?,        // may be null (= Option<Entity>)
}

// Safe access with ?
let name = party.healer?.name;       // string?

// Assert non-null with !
let name = party.healer!.name;       // string (crash if null)

// Pattern matching
match party.healer {
    Option::Some(h) => { h.heal(player); }
    Option::None => { log("No healer!"); }
}

// if let
if let Option::Some(h) = party.healer {
    h.heal(player);
}

// Null assignment
let mut healer: Entity? = null;      // null = Option::None
```

---

