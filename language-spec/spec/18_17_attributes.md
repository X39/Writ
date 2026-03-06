# 1. Writ Language Specification
## 1.17 Attributes

Attributes provide metadata on declarations using `[]` syntax. They are placed on the line before the declaration they
modify. The parser collects pending attributes and attaches them when it encounters the next declaration keyword.

### 1.17.1 Syntax

Attributes accept positional arguments, named arguments, or both. Positional arguments must appear before named
arguments.

```
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

### 1.17.2 Builtin Attributes

| Attribute             | Applies To         | Parameters                               | Effect                                                                                                       |
|-----------------------|--------------------|------------------------------------------|--------------------------------------------------------------------------------------------------------------|
| `[Singleton]`         | entity             | *(none)*                                 | Enforces at most one instance. Enables `Entity.getOrCreate<T>()` and auto-resolution in `@speaker` dialogue. |
| `[Deprecated(msg)]`   | any declaration    | `msg`: string (positional)               | Compiler warning when referenced. Message shown in language server.                                          |
| `[Locale(tag)]`       | dlg                | `tag`: string (positional)               | Marks this `dlg` as a locale-specific structural override. See [Section 1.26](#126-localization).               |
| `[Import(lib, ...)]`  | extern declaration | See [Section 1.25.2](#1252-library-imports) | Marks an extern as loaded from a native library rather than provided by the runtime.                         |
| `[Conditional(name)]` | fn                 | `name`: string (positional)              | Marks a function as a conditional override. See [Section 1.17.4](#1174-conditional-compilation).                |

### 1.17.3 Parser Disambiguation

The `[` token at statement level could be either an attribute or an array expression. The parser resolves this by
checking whether the token after the closing `]` is a declaration keyword (`entity`, `fn`, `struct`, etc.). If yes, it
is an attribute. Otherwise, it is an expression. This requires only one token of lookahead past the `]`.

### 1.17.4 Conditional Compilation

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

```
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

```
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

---

