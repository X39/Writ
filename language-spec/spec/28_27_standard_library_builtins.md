# 1. Writ Language Specification
## 1.27 Standard Library Builtins

### 1.27.1 Compiler-Known Types

| Type           | Sugar        | Purpose                                 |
|----------------|--------------|-----------------------------------------|
| `Option<T>`    | `T?`, `null` | Nullable values                         |
| `Result<T, E>` | —            | Fallible operations (`E: Error`)        |
| `Range<T>`     | `..`, `..=`  | Interval type for iteration and slicing |

### 1.27.2 Compiler-Known Contracts

| Contract                          | Special Behavior                                                                                                                                                                              |
|-----------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Error`                           | Required bound for Result's `E` parameter. Requires `message() -> string`.                                                                                                                    |
| `Into<T>`                         | Type conversion. Called explicitly via `.into<T>()`. Implicitly called as `.into<string>()` by `{expr}` interpolation in formattable strings (`$"..."`) and dialogue lines. See Section 1.11.2. |
| `Add`, `Sub`, `Mul`, `Div`, `Mod` | Mapped from `operator +`, `-`, `*`, `/`, `%` syntax.                                                                                                                                          |
| `Neg`, `Not`                      | Mapped from unary `-` and `!` syntax.                                                                                                                                                         |
| `Eq`, `Ord`                       | Mapped from `operator ==`, `<`. Derived: `!=`, `>`, `<=`, `>=`.                                                                                                                               |
| `Index<K, V>`, `IndexSet<K, V>`   | Mapped from `operator []` (read) and `operator []=` (write) syntax.                                                                                                                           |
| `BitAnd`, `BitOr`                 | Mapped from `operator &`, `\|`.                                                                                                                                                               |
| `Iterable<T>`, `Iterator<T>`      | Enable `for` loop iteration. `T[]` and `Range<T>` have compiler-provided implementations. See Section 1.11.3.                                                                                   |

### 1.27.3 Standard Library Types

These types are provided by the standard library with no special compiler support:

| Type            | Description                                                    |
|-----------------|----------------------------------------------------------------|
| `List<T>`       | Ordered, growable collection                                   |
| `Map<K, V>`     | Key-value associative collection                               |
| `Set<T>`        | Unordered unique collection                                    |
| `EntityList<T>` | Typed entity reference collection with component query support |

### 1.27.4 Root-Namespace Inbuilt Calls

The following functions and namespaces are always available from the root namespace without any
qualifier. No `writ::`, `Runtime::`, or any other qualifier is needed or accepted.

#### say and choice

| Function | Signature | Purpose |
|----------|-----------|---------|
| `say`    | `fn say(speaker: Entity, text: string)` | Display dialogue (transition point — suspends) |
| `choice` | `fn choice(options: ...) -> int` | Present choices (transition point — suspends) |

These are **inbuilt calls** — the compiler resolves them from the root namespace. They are callable as
`say(speaker, text)` and `choice(options)`.

The root-qualified forms `::say` and `::choice` (with a leading `::`) are also valid — `::` means
"resolve from the root namespace" (see §1.24.9). They are equivalent to the unqualified names and
produce identical IL. Both forms are accepted from any `fn` or `dlg` context.

`say` and `choice` are dialogue transition points — the VM suspends until the host responds (§1.14.9).

The compiler lowers `dlg` syntax (`@Speaker text`, `$ choice { ... }`) into calls to `say` and
`choice` automatically — user code in `dlg` blocks does not call them directly.

#### log:: namespace

`log` is a **compiler-known namespace**, not a callable function. It provides five leveled logging
functions:

| Function       | Signature                   | Purpose                                       |
|----------------|-----------------------------|-----------------------------------------------|
| `log::trace`   | `fn log::trace(msg: string)` | Extremely verbose trace output                |
| `log::debug`   | `fn log::debug(msg: string)` | Debug-level messages for development          |
| `log::info`    | `fn log::info(msg: string)`  | Informational messages (most common)          |
| `log::warn`    | `fn log::warn(msg: string)`  | Warnings about unexpected but handled states  |
| `log::error`   | `fn log::error(msg: string)` | Errors and failures                           |

Each function accepts a single `string` argument and returns `void`. All five are fire-and-forget —
they do not suspend the VM.

The root-qualified forms (`::log::trace(msg)`, `::log::info(msg)`, etc.) are also valid and
equivalent. Both forms are accepted from any `fn` or `dlg` context.

All five log functions route to `RuntimeHost::on_log(level, message)` with the corresponding
`LogLevel` variant (`Trace`, `Debug`, `Info`, `Warn`, `Error`). What the host does with the
message — printing, filtering, forwarding — is host-defined.

Standard lexical shadowing applies: `let log = 5;` shadows the `log` namespace at that scope.

---

