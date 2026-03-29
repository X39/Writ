# 1.17 Attributes

Attributes provide metadata on declarations using `[]` syntax. They are placed on the line before the declaration they
modify. The parser collects pending attributes and attaches them when it encounters the next declaration keyword.

## 1.17.1 Syntax

Attributes accept positional arguments, named arguments, or both. Positional arguments must appear before named
arguments.

```writ
// No arguments
[Singleton]
entity Narrator { ... }

// Positional argument
[Deprecated("Use NewMerchant instead")]
entity OldMerchant { ... }

// Named arguments
[Import("physics", symbol = "phys_raycast_2d")]
extern fn raycast(origin: vec2, dir: vec2, dist: float) -> HitResult?;

// Multiple attributes (separate lines)
[Singleton]
[Deprecated("Use NewMerchant instead")]
entity OldMerchant { ... }

// Multiple attributes (comma-separated)
[Singleton, Deprecated("Use NewMerchant")]
entity OldMerchant { ... }
```

## 1.17.2 Builtin Attributes

| Attribute             | Applies To         | Parameters                               | Effect                                                                                                       |
|-----------------------|--------------------|------------------------------------------|--------------------------------------------------------------------------------------------------------------|
| `[Singleton]`         | entity             | *(none)*                                 | Enforces at most one instance. Enables `Entity.getOrCreate<T>()` and auto-resolution in `@speaker` dialogue. |
| `[Deprecated(msg)]`   | any declaration    | `msg`: string (positional)               | Compiler warning when referenced. Message shown in language server.                                          |
| `[Locale(tag)]`       | dlg                | `tag`: string (positional)               | Marks this `dlg` as a locale-specific structural override. See [Section 1.26](#126-localization).               |
| `[Import(lib, ...)]`  | extern declaration | See [Section 1.25.2](#1252-library-imports) | Marks an extern as loaded from a native library rather than provided by the runtime.                         |
| `[Conditional(name)]` | fn                 | `name`: string (positional)              | Marks a function as a conditional override. See [Section 1.17.4](#1174-conditional-compilation).                |

## 1.17.3 Parser Disambiguation

The `[` token at statement level could be either an attribute or an array expression. The parser resolves this by
checking whether the token after the closing `]` is a declaration keyword (`entity`, `fn`, `struct`, etc.). If yes, it
is an attribute. Otherwise, it is an expression. This requires only one token of lookahead past the `]`.

## 1.17.4 Conditional Compilation

The `[Conditional("name")]` attribute marks a function as a **conditional override**. The condition name is a string
that is either active or inactive at compile time (defined in `writ.toml` or via compiler flags).

**Rules:**

1. Every conditional function **must** have a non-conditional counterpart with the same name and signature. A
   conditional function without a fallback is a compile error.
2. When the named condition is active, the conditional version replaces the fallback at compile time. When inactive, the
   fallback stands and the conditional version is excluded entirely.
3. `[Conditional]` applies only to functions (`fn`). It cannot be used on structs, entities, components, or other
   declarations.
4. Multiple conditional overrides for the same function are allowed with different condition names, but at most one
   condition may be active for a given function at compile time. Overlapping active conditions on the same function
   signature is a compile error.

```writ
// Non-conditional fallback (always required)
fn rumbleController(intensity: float) {
    // generic fallback — could be a no-op
}

// PlayStation-specific override
[Conditional("playstation")]
fn rumbleController(intensity: float) {
    // DualSense haptics via native import
}

// Xbox-specific override
[Conditional("xbox")]
fn rumbleController(intensity: float) {
    // Xbox trigger rumble
}
```

```writ
// Debug logging — no-op fallback in release
fn writeDebugLine(msg: string) { }

[Conditional("debug")]
fn writeDebugLine(msg: string) {
    runtime.log(msg);
}
```

This model mirrors dialogue localization: the non-conditional function is the "default locale" and conditional overrides
are locale-specific translations. Code that calls `writeDebugLine(...)` always compiles — the compiler selects the
appropriate implementation based on active conditions.

Conditions are defined in `writ.toml` (see [Section 1.2.5](#125-conditions)) or passed as compiler flags.

## 1.17.5 User-Defined Attributes

In addition to builtins, users can declare custom attribute types with typed parameters:

```writ
attribute MinLevel(level: int);
attribute Tag(name: string);
attribute Debug();
```

**Rules:**

1. Parameters use the same type syntax as function parameters. Supported parameter types are `string`, `int`, and `bool`.
2. The declaration ends with a semicolon — there is no body.
3. Builtin attribute names (`Singleton`, `Deprecated`, `Locale`, `Import`, `Conditional`) are reserved. Declaring a user-defined attribute with a builtin name produces a name collision error (E0008).
4. User-defined attributes pass through the full compiler pipeline and are recorded in the module's `AttributeDef` metadata table. The attribute's typed arguments are serialized into the blob heap (see [Section 1.17.6](#1176-attribute-argument-encoding)).
5. User-defined attributes have no automatic semantic effects at compile time or runtime. The host application must explicitly query and act on them through the runtime query API (see [Section 1.17.7](#1177-runtime-query-api)).

```writ
// Declaration
attribute MinLevel(level: int);

// Application
[MinLevel(5)]
fn advancedMove() {
    // ...
}

// The compiler records MinLevel(5) in the AttributeDef table.
// The host can query it at load time via the runtime query API.
```

## 1.17.6 Attribute Argument Encoding

Attribute arguments are serialized into the module's blob heap using a tagged binary format. Each argument is prefixed with a 1-byte tag identifying its type, followed by its payload. A multi-argument attribute produces a sequential concatenation of individually encoded arguments.

| Tag | Constant | Payload |
|-----|----------|---------|
| `0x01` | `ATTR_TAG_STRING` | `u32` byte length (little-endian) + UTF-8 bytes |
| `0x02` | `ATTR_TAG_INT` | `i64` little-endian (8 bytes) |
| `0x03` | `ATTR_TAG_BOOL` | `u8` (`0x00` = false, any non-zero = true) |
| `0x04` | `ATTR_TAG_NAMED` | `u32` name byte length (LE) + name UTF-8 bytes + inner argument encoding |

An empty argument list (e.g., `[Debug()]`) encodes to an empty byte sequence stored at blob offset 0 (the null blob).

**Example:** `[Deprecated("use bar instead")]` encodes as:

```text
0x01                         // ATTR_TAG_STRING
0x13 0x00 0x00 0x00          // byte length = 19 (little-endian u32)
"use bar instead"            // 19 UTF-8 bytes
```

**Named argument example:** `[Import("physics", symbol = "phys_raycast")]` encodes as:

```text
0x01                         // ATTR_TAG_STRING (positional arg "physics")
0x07 0x00 0x00 0x00          // byte length = 7
"physics"                    // 7 bytes
0x04                         // ATTR_TAG_NAMED
0x06 0x00 0x00 0x00          // name byte length = 6
"symbol"                     // 6 name bytes
0x01                         // ATTR_TAG_STRING (inner value)
0x0C 0x00 0x00 0x00          // byte length = 12
"phys_raycast"               // 12 bytes
```

The tag constants and encoding/decoding functions are defined in `writ-module` and shared by both the compiler (encoding) and the runtime (decoding).

## 1.17.7 Runtime Query API

The runtime provides three query methods on the `Domain` (multi-module) and `ModuleAttributeView` (single-module) types. These return decoded attribute arguments — the host never interacts with raw blob bytes.

### Domain-level queries (multi-module)

```
Domain::query_attributes(attr_name: &str) -> Vec<DomainAttributeMatch>
```

Returns all attribute application rows across all loaded modules whose name matches `attr_name`. Declaration rows (attribute declarations, not applications) are excluded.

```
Domain::query_attributes_on(module_idx: usize, typedef_idx: usize) -> Vec<DomainAttributeMatch>
```

Returns all attribute applications on the TypeDef at the given index in the given module.

```
Domain::query_attribute_value(module_idx: usize, owner_token: MetadataToken, attr_name: &str) -> Option<Vec<AttrValue>>
```

Returns the decoded argument list for the first attribute matching `attr_name` on the given owner token, or `None` if no match exists.

Each match is returned as a `DomainAttributeMatch`:

| Field | Type | Description |
|-------|------|-------------|
| `module_idx` | `usize` | Index into the domain's module list |
| `name` | `String` | Attribute name |
| `args` | `Vec<AttrValue>` | Decoded arguments (empty for no-arg attributes) |
| `owner` | `MetadataToken` | The definition this attribute is applied to |
| `owner_kind` | `u8` | `0` = type, `1` = method, `2` = other, `3` = declaration |

### Pre-load callback

The `RuntimeHost` trait includes a pre-load hook that fires before any module code executes:

```
fn on_module_load(&mut self, view: &ModuleAttributeView) -> Result<(), String>
```

- Returning `Ok(())` allows the module to load normally.
- Returning `Err(reason)` rejects the module — it is never added to the domain.
- The callback fires only for the user module (not for the virtual module or library modules).

`ModuleAttributeView` provides the same three query methods as `Domain`, scoped to the single module being loaded:

```
ModuleAttributeView::query_attributes(attr_name: &str) -> Vec<AttributeMatch>
ModuleAttributeView::query_attributes_on(typedef_idx: usize) -> Vec<AttributeMatch>
ModuleAttributeView::query_attribute_value(owner_token: MetadataToken, attr_name: &str) -> Option<Vec<AttrValue>>
```

### Design principle

No attribute causes automatic instantiation or invocation. The runtime query API is purely reflective — the host must explicitly read attribute data and decide what actions to take. This keeps the attribute system predictable and gives the host full control over attribute-driven behavior.

---

