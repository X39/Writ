# Writ IL Specification
## 2.11 Construction Model

**Decision:** Construction uses the `new` keyword with brace-syntax for all types. No user-defined constructors.
`spawn` is reserved for task concurrency only.

**Syntax:** `new Type { field: value, ... }` for both structs and entities. The `new` keyword disambiguates
construction from block expressions, making the syntax unambiguous for the parser. The compiler determines the IL
sequence from the type's kind.

**Default field values:** Defaults can be runtime expressions (e.g., `List::new()`). The compiler inlines the default
expression at every construction site that doesn't override the field. `NEW` allocates zeroed memory — the compiler
emits explicit code for all field initialization.

**Struct construction:**

1. `NEW type_idx` — allocate zeroed memory.
2. `SET_FIELD` / `LOAD_*` for every field (defaults + overrides).
3. `CALL __on_create` — run the `on create` hook body, if defined.

**Entity construction:**

1. `SPAWN_ENTITY type_token` — allocate entity. Set the entity's internal "under construction" flag. Notify the host
   with the component list (from ComponentSlot metadata) so it can prepare native representations.
2. `SET_FIELD` for script fields (written to heap directly) and component fields (**buffered**, not sent to host).
3. `INIT_ENTITY` — flush all buffered component field values to the host as a single batch. Clear the "under
   construction" flag. Fire the `on_create` lifecycle hook.

The separation of SPAWN_ENTITY and INIT_ENTITY ensures field overrides are visible inside `on_create`. Component field
buffering avoids per-field round-trips through suspend-and-confirm during construction. See §2.16.7 for the full
buffering specification and safety invariants.

**No constructors:** Construction is entirely compiler-generated. `new Type { ... }` produces `NEW`/`SPAWN_ENTITY` +
`SET_FIELD` + `on_create`. Fields without defaults are required at every construction site. For convenience factories,
use static methods: `Merchant::create("Tim")`.

**Lifecycle hooks:** Both structs and entities support lifecycle hooks (`on create`, `on finalize`, `on serialize`,
`on deserialize`). Entities additionally support `on destroy` and `on interact`. All hooks receive implicit `mut self`.
Hooks lower to regular methods stored in the TypeDef metadata.

