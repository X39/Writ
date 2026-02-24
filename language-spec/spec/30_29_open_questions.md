# Writ Language Specification
## 29. Open Questions

The following design questions remain unresolved and are tracked for future specification revisions.

**String formatting**
Should there be a format specifier syntax within interpolation? E.g., `{price:.2f}` for float formatting.

**Tuple types**
Are anonymous tuple types `(int, string)` useful, or are named structs sufficient?

**Destructuring**
Should `let (x, y) = getPosition();` or `let { name, gold } = merchant;` be supported?

**Visibility modifiers**
Should `pub`/`priv` exist on struct fields and functions, or is everything public?

**Standard library API**
Exact API surface for `List<T>`, `Map<K,V>`, `Set<T>`, `EntityList<T>`, iterators, string utilities, math functions.
Core array operations (`T[]`) are defined in Section 6.3. Relationship between arrays and `List<T>` (wrapper, alias, or
distinct type) is TBD.

**REPL / hot reload**
Should the runtime support live script reloading during development?

**Entity destruction semantics**
What does `Entity.getOrCreate` return for a destroyed singleton? Recreate silently, return Option, or crash?

**Component dynamic add/remove**
Can components be added to or removed from entities at runtime, or is the component set fixed at spawn time?

**EntityList.with\<T\>() narrowing**
Should `.with<Component>()` refine the type so that component access is guaranteed non-optional within the filtered set?

**Custom attributes**
Can users define their own attributes, or are they limited to builtin ones?

**Serialization**
How are entity and game state serialized for save/load? Automatic, opt-in via attribute, or manual?

**Localization pluralization**
How should pluralization rules be handled across locales (e.g., `{count} items` where some languages have complex plural
forms)?
