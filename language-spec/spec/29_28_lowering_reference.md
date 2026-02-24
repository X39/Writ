# Writ Language Specification
## 28. Lowering Reference

All higher-level constructs lower to simpler primitives before execution.

### 28.1 Dialogue Lowering

| Dialogue Construct       | Lowers To                                          |
|--------------------------|----------------------------------------------------|
| `@speaker Text.`         | `say(speaker, "Text.");`                           |
| `@speaker` (default set) | Subsequent lines use set speaker in `say()` calls  |
| `$ choice { ... }`       | `choice([ ... ]);`                                 |
| `$ if cond { ... }`      | `if cond { ... }` (branches contain `say()` calls) |
| `$ match expr { ... }`   | `match expr { ... }` (arms contain `say()` calls)  |
| `-> otherDialog`         | `return otherDialog();`                            |
| `$ statement;`           | `statement;`                                       |
| `{expr}` in text         | Concatenation with `.into<string>()`               |
| `#key` on text line      | Overrides auto-generated key in `say_localized()`  |
| `@Singleton` (auto)      | `Entity.getOrCreate<T>()` for speaker              |

### 28.2 Full Dialogue Lowering Example

Source:

```
dlg greetPlayer(name: string) {
    @Narrator Hey, {name}.
    @Narrator
    How are you?
    $ choice {
        "Good!" {
            $ reputation += 1;
            Glad to hear it.
        }
        "Not great" {
            @Player Things are rough.
            @Narrator Sorry to hear that.
        }
    }
    -> farewellDialog
}
```

Lowered output:

```
fn greetPlayer(name: string) {
    let _narrator = Entity.getOrCreate<Narrator>();
    let _player = Entity.getOrCreate<Player>();
    say(_narrator, "Hey, " + name.into<string>() + ".");
    say(_narrator, "How are you?");
    choice([
        Option("Good!", fn() {
            reputation += 1;
            say(_narrator, "Glad to hear it.");
        }),
        Option("Not great", fn() {
            say(_player, "Things are rough.");
            say(_narrator, "Sorry to hear that.");
        }),
    ]);
    return farewellDialog();
}
```

### 28.3 Entity Lowering

Entities lower to structs with component fields, constructor functions, and registered lifecycle hooks. The exact
lowering is runtime-specific, but conceptually:

```
// entity Guard { use Health { current: 80, max: 80 }, ... }
// lowers to approximately:
struct Guard {
    _health: Health,
    _sprite: Sprite,
    name: string,
}
// + constructor with defaults
// + ComponentAccess<Health> impl
// + ComponentAccess<Sprite> impl
// + lifecycle hook registrations
```

### 28.4 Localized Dialogue Lowering

When localization is active, `say()` calls include key metadata for runtime lookup:

```
// say(_narrator, "Hey, " + name.into<string>() + ".");
// becomes:
say_localized(
    _narrator,
    "a3f7c012",                        // pre-computed FNV-1a key
    "Hey, " + name.into<string>() + ".",   // fallback (default locale)
);
```

The runtime's `say_localized` implementation:

1. Looks up `"a3f7c012"` in the active locale's string table.
2. If found, substitutes interpolation slots and displays the translation.
3. If not found, uses the fallback text.

### 28.5 Runtime Functions

The runtime must provide these core functions:

| Function             | Signature                                                          | Behavior                                        |
|----------------------|--------------------------------------------------------------------|-------------------------------------------------|
| `say`                | `fn say(speaker: Entity, text: string)`                            | Display text, yield until player advances       |
| `say_localized`      | `fn say_localized(speaker: Entity, key: string, fallback: string)` | Localized display with string table lookup      |
| `choice`             | `fn choice(options: List<...>)`                                    | Present choices, yield, execute selected branch |
| `Entity.getOrCreate` | `fn getOrCreate<T>() -> T`                                         | Get or create singleton entity instance         |
| `Entity.findAll`     | `fn findAll<T>() -> EntityList<T>`                                 | Find all entities of a type                     |

---

