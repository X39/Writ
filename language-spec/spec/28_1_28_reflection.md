# 1.28 Reflection

Reflection gives scripts the ability to inspect type metadata at runtime. Writ provides two complementary
query mechanisms — a static compile-time query (`typeof(expr)`) and a dynamic runtime query (`expr.get_type()`) —
along with a set of reflection types that describe the shape of any user-defined type.

## 1.28.1 Reflection Types

The reflection system exposes six types. All are class types (GC-allocated, reference semantics).

| Type | Kind | Purpose |
|------|------|---------|
| `Type` | class | Represents a type's metadata (name, kind, namespace, fields, methods, attributes, contracts) |
| `FieldInfo` | class | Describes a single public field (name, declared_type, is_mutable) |
| `MethodInfo` | class | Describes a single public method (name, parameters, return_type) |
| `ParameterInfo` | class | Describes a method parameter (name, declared_type) |
| `AttributeInfo` | class | Describes an applied attribute (name, args) |
| `ContractInfo` | class | Describes an implemented contract (name, type) |

---

**`Type` — type metadata**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Unqualified type name |
| `namespace` | `string` | Fully-qualified namespace |
| `kind` | `string` | One of: `"struct"`, `"class"`, `"enum"`, `"entity"` |
| `is_generic` | `bool` | Whether the type has generic parameters |

| Method | Signature | Description |
|--------|-----------|-------------|
| `fields` | `fn fields(self) -> FieldInfo[]` | All public fields |
| `methods` | `fn methods(self) -> MethodInfo[]` | All public methods |
| `attributes` | `fn attributes(self) -> AttributeInfo[]` | All applied attributes |
| `contracts` | `fn contracts(self) -> ContractInfo[]` | All implemented contracts |
| `implements` | `fn implements(self, contract: Type) -> bool` | Runtime capability check |
| `type_args` | `fn type_args(self) -> Type[]` | Bound generic type arguments |

---

**`FieldInfo` — field descriptor**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Field name |
| `declared_type` | `Type` | Type of the field |
| `is_mutable` | `bool` | `true` if declared with `mut`, `false` if `let` |

| Method | Signature | Description |
|--------|-----------|-------------|
| `get` | `fn get(self, instance: Box) -> Box` | Read field value dynamically (boxed) |
| `set` | `fn set(self, instance: Box, value: Box)` | Write field value dynamically (boxed). Crashes task if field is immutable (`let`). |

---

**`MethodInfo` — method descriptor**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Method name |
| `parameters` | `ParameterInfo[]` | Parameter descriptors |
| `return_type` | `Type` | Return type |

| Method | Signature | Description |
|--------|-----------|-------------|
| `invoke` | `fn invoke(self, instance: Box, args: Box[]) -> Box` | Invoke method dynamically on the current task's call stack. The invoked method participates in cooperative scheduling (defer runs on unwind, spawn yields). |
| `attributes` | `fn attributes(self) -> AttributeInfo[]` | Per-member attributes |

---

**`ParameterInfo` — parameter descriptor**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Parameter name |
| `declared_type` | `Type` | Parameter type |

---

**`AttributeInfo` — attribute descriptor**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Attribute name |
| `args` | `Box[]` | Attribute arguments (boxed values) |

---

**`ContractInfo` — contract descriptor**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | Contract name |
| `type` | `Type` | The contract's `Type` object |

---

## 1.28.2 `typeof(expr)` — Static Type Query

`typeof(expr)` is a **compile-time** query that returns a `Type` object representing the **static** (declared)
type of the expression. The type index is baked into the IL at compile time via the `TYPEOF` opcode (see section
3.10). The expression `expr` is evaluated only for its type — it is not executed.

The result is a lazily-allocated singleton `Type` heap object. `typeof(T) == typeof(T)` is always true (identity
by TypeDef index). The runtime allocates the `Type` object on first access and roots it permanently in the GC.

```writ
let t: Type = typeof(Player);
log($"Type name: {t.name}");         // "Player"
log($"Namespace: {t.namespace}");    // e.g., "game.entities"

for field in t.fields() {
    log($"  {field.name}: {field.declared_type.name}");
}
```

`typeof` accepts any expression whose static type is known at compile time — type names, variable references,
or expressions. The compiler resolves the type at compile time and emits a `TYPEOF` instruction with a literal
TypeDef token.

## 1.28.3 `get_type()` — Dynamic Type Query

`get_type()` is a **dynamic** runtime query dispatched via the `Reflectable` contract (section 1.28.4). It
returns the **runtime** actual type of the receiver, which may differ from the static declared type for
polymorphic variables.

```writ
fn inspect(a: Animal) {
    let static_t  = typeof(a);       // Always: Type for Animal (compile-time static type)
    let dynamic_t = a.get_type();    // May be: Type for Dog (runtime actual type)
    let same = static_t == dynamic_t; // false when a is a subtype
}
```

The call `a.get_type()` lowers to a `CALL_VIRT` on the `Reflectable` contract. The runtime dispatches to the
concrete type's auto-generated `Reflectable` implementation, which returns the pre-allocated `Type` singleton
for that concrete type.

## 1.28.4 Reflectable Contract

The `Reflectable` contract has a single method:

```writ
contract Reflectable {
    fn get_type(self) -> Type;
}
// Auto-implemented on all user-defined types.
// For primitives, the runtime provides intrinsic dispatch.
```

**Auto-implementation rule:** The compiler automatically generates a `Reflectable` implementation for every
**user-defined** type — structs, classes, entities, and enums. The generated method returns the pre-allocated
`Type` singleton for that type. No user code is required.

**Primitives:** `int`, `float`, `bool`, and `string` do not receive an auto-impl. Instead, the runtime virtual
module registers separate intrinsics (`IntGetType`, `FloatGetType`, `BoolGetType`, `StringGetType`) in section
2.18.9. These are dispatched via the same `CALL_VIRT` mechanism.

**Extern types** do not receive a `Reflectable` auto-impl. Host-provided types are outside the script metadata
boundary.

`Reflectable` is contract index 19 in the writ-runtime virtual module (see section 2.18.3).

## 1.28.5 Type Introspection Methods

The `Type` type exposes the following introspection methods:

- `fields()` — returns all **public** fields as `FieldInfo[]`. Private fields are excluded.
- `methods()` — returns all **public** methods as `MethodInfo[]`. Private methods are excluded.
- `attributes()` — returns all applied attributes as `AttributeInfo[]`. Integrates with the attribute system
  (section 1.17) and shares underlying data with the `ModuleAttributeView` query API (section 2.18.9).
- `contracts()` — returns all implemented contracts as `ContractInfo[]`.
- `implements(contract: Type) -> bool` — runtime capability check. Returns `true` if this type implements the
  given contract. Useful for safe dynamic dispatch.
- `is_generic` — `bool` field, `true` if the type has generic parameters.
- `type_args()` — see section 1.28.7.

```writ
let t = typeof(Merchant);
for field in t.fields() {
    log($"  {field.name}: {field.declared_type.name}");
}

if t.implements(typeof(Serializable)) {
    log("Merchant is serializable");
}
```

## 1.28.6 Dynamic Invocation

Reflection supports dynamic field access and method invocation through `FieldInfo` and `MethodInfo`.

**`FieldInfo.get(instance)` and `FieldInfo.set(instance, value)`:**

- `get()` reads the field value from the given instance and returns it as a `Box`.
- `set()` writes a new value. If the field was declared with `let` (immutable binding), calling `set()` **crashes
  the current task** with the message: `"Reflection write to immutable field '{field_name}'"`.
- If the field was declared with `mut`, `set()` writes the new value.
- The runtime determines immutability from the `is_mutable` flag stored in the field's declaration metadata.

**`MethodInfo.invoke(instance, args)`:**

- Invocation executes on the **current task's call stack** (not a separate task).
- The invoked method participates in cooperative scheduling: `defer` blocks run on frame unwind, `spawn` within
  the invoked method yields as expected.
- Argument count and type validation occur at the call site; mismatches crash the task.

**Boxing at API boundaries:**

All reflection API parameters and return values use `Box` (see section 3.15).

- `FieldInfo.get()` returns a `Box`. The compiler auto-inserts `UNBOX` at the call site when the result is
  assigned to a concrete type.
- `FieldInfo.set()` accepts a `Box` value. The compiler auto-inserts `BOX` at the call site when a concrete
  value is passed.
- `MethodInfo.invoke()` accepts `Box[]` args and returns `Box`. The same auto-coercion applies at the call site.
- This uses the existing `BOX`/`UNBOX` IL instructions (section 3.15). No new `TyKind::Any` is introduced.

```writ
let t    = typeof(Player);
let hp   = t.fields().find(fn(f) = f.name == "hp")!;
hp.set(player, 100);  // OK if hp is 'mut'
// If hp was declared 'let', task crashes: "Reflection write to immutable field 'hp'"

let greet = t.methods().find(fn(m) = m.name == "greet")!;
greet.invoke(player, []);  // args boxed automatically by compiler
```

**Note on `Type.construct()`:** Dynamic type instantiation via `Type.construct()` is reserved for a future
version. Attempting to call it in this version crashes the task with `UnsupportedOperation`.

## 1.28.7 Generic Reflection Scope

`Type.is_generic` and `Type.type_args()` describe generic instantiation:

- `is_generic` is `true` when the type was originally declared with generic parameters (e.g., `Array<T>`).
- `type_args()` returns the bound type arguments for the instantiation.

**Static instantiations via `typeof`:** Type arguments are fully known at compile time and encoded in the
`TYPEOF` instruction's TypeRef blob. `type_args()` returns the concrete type arguments.

```writ
let t = typeof(Array<int>);
log($"is_generic: {t.is_generic}");   // true
for arg in t.type_args() {
    log($"  arg: {arg.name}");        // "int"
}
```

**Runtime instantiations via `get_type()`:** For types queried through `get_type()` on a polymorphic variable
whose concrete generic instantiation is not statically known, `type_args()` **may return an empty array**. This
is a documented limitation of the runtime reflection model, not a bug. The type is correctly identified; only
the generic arguments are unavailable in this case.

## 1.28.8 Scope and Limitations

The following constraints govern the reflection system:

- **Visibility:** Only `pub` fields and methods are visible through reflection. Private members are excluded from
  `fields()` and `methods()` output. This is not a security mechanism — it is a API-surface contract that mirrors
  the visibility rules of direct access.
- **Extern types:** Extern declarations are not reflectable. Host-provided types exist outside the script
  metadata boundary; they do not appear in `ModuleDef` tables and cannot be queried.
- **No private access:** Accessing non-public fields via reflection is explicitly disallowed. The host trust
  model does not allow scripts to bypass visibility via indirect means.
- **`Type.construct()` deferred:** Dynamic type instantiation is a v12+ feature. Calling it in the current
  version crashes the task with `UnsupportedOperation`.
- **No string-based type lookup:** `Type.for_name(string)` is an **explicit anti-feature**. String-based
  type resolution creates a serialization injection attack vector (a hostile save file names an arbitrary type)
  and shifts type resolution authority from the host to the script. Type resolution is always host-controlled.
- **Open generic type_args:** As noted in section 1.28.7, `type_args()` on a runtime-queried open generic
  type may return an empty array. Callers must handle this case.

---
