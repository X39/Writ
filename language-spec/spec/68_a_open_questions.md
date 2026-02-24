# Appendix
## A. Open Questions

The following design questions remain unresolved and are tracked for future specification revisions.

**String formatting**
Should there be a format specifier syntax within interpolation? E.g., `{price:.2f}` for float formatting.

**Tuple types**
Are anonymous tuple types `(int, string)` useful, or are named structs sufficient?

**Destructuring**
Should `let (x, y) = getPosition();` or `let { name, gold } = merchant;` be supported?

**Standard library API**
Exact API surface for `List<T>`, `Map<K,V>`, `Set<T>`, `EntityList<T>`, iterators, string utilities, math functions.
Core array operations (`T[]`) are defined in Section 6.3. Relationship between arrays and `List<T>` (wrapper, alias, or
distinct type) is TBD.

**REPL / hot reload**
Should the runtime support live script reloading during development?

**Entity destruction semantics**
What does `Entity.getOrCreate` return for a destroyed singleton? Recreate silently, return Option, or crash?

**Component dynamic add/remove**
Components are extern and data-only (host-provided). The component set is fixed at construction time by the entity
declaration. Dynamic add/remove would require host engine support and is not a language-level feature.

**EntityList.with\<T\>() narrowing**
Should `.with<Component>()` refine the type so that component access is guaranteed non-optional within the filtered set?

**Custom attributes**
Can users define their own attributes, or are they limited to builtin ones?

**Serialization**
How are entity and game state serialized for save/load? Automatic, opt-in via attribute, or manual?

**Localization pluralization**
How should pluralization rules be handled across locales (e.g., `{count} items` where some languages have complex plural
forms)?

**Properties & UI binding**
Should Writ have a `property` keyword with get/set accessors, an `[Observable]` attribute, or both? How should game UI
bind to script-side state — push-based (change notifications), pull-based (polling), or a declarative binding syntax?

