# 1. Writ Language Specification
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

Entities lower to TypeDefs (kind=Entity) with fields, component slots, methods, and lifecycle hooks. Components are
extern and data-only — they are host-managed instances attached to the entity.

```
// entity Guard { name: string = "Guard", health: int = 80, use Sprite { ... }, ... }
// produces:
//   TypeDef(Guard, kind=Entity)
//     fields: [name: string, health: int, maxHealth: int, ...]
//     component_slots: [Speaker, Sprite, Collider]
//     component_overrides: [Speaker.displayName="Guard", Sprite.texture="res://...", ...]
//     lifecycle: [on_create, on_destroy, on_finalize, on_serialize, on_deserialize]

// Methods lower to:
//   fn Guard::greet(self: Guard) -> string { ... }
//   fn Guard::__on_create(mut self: Guard) { ... }     // from: on create { ... }
//   fn Guard::__on_interact(mut self: Guard, who: Entity) { ... }
//   fn Guard::__on_destroy(mut self: Guard) { ... }
//   fn Guard::__on_finalize(mut self: Guard) { ... }
//   fn Guard::__on_serialize(mut self: Guard) { ... }
//   fn Guard::__on_deserialize(mut self: Guard) { ... }
```

See [Section 14.7](15_14_entities.md#147-entity-lowering) for the full lowering specification.

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

The runtime must provide these core functions in the `Runtime` namespace. Dialogue functions are **transition points** —
they suspend execution until the host responds (see §13.9, IL spec §1.14.2).

| Function             | Signature                                                          | Behavior                                                                      |
|----------------------|--------------------------------------------------------------------|-------------------------------------------------------------------------------|
| `say`                | `fn say(speaker: Entity, text: string)`                            | Display text, **suspend** until player advances                               |
| `say_localized`      | `fn say_localized(speaker: Entity, key: string, fallback: string)` | Localized display with string table lookup, **suspend** until player advances |
| `choice`             | `fn choice(options: List<...>) -> int`                             | Present choices, **suspend** until player selects, return selected index      |
| `Entity.getOrCreate` | `fn getOrCreate<T>() -> T`                                         | Get or create singleton entity instance                                       |
| `Entity.findAll`     | `fn findAll<T>() -> EntityList<T>`                                 | Find all entities of a type                                                   |
| `Entity.destroy`     | `fn destroy(entity: Entity)`                                       | Destroy entity, fire `on destroy`, mark dead                                  |
| `Entity.isAlive`     | `fn isAlive(entity: Entity) -> bool`                               | Check if handle refers to a live entity                                       |

---

# Writ IL Specification

**Draft v0.1** — February 2026

---

The intermediate language specification for the Writ virtual machine. Defines the register-based IL design,
instruction set, binary module format, and execution model.

Architectural choices that govern the entire IL design are documented in sections 1.1–1.17. The instruction
set reference follows in sections 2.0–2.16. Instruction encoding tables and opcode assignments are in
sections 3.0–3.2.

---

