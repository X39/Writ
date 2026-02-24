# Writ Language Specification
## 26. Standard Library Builtins

### 26.1 Compiler-Known Types

| Type           | Sugar        | Purpose                                 |
|----------------|--------------|-----------------------------------------|
| `Option<T>`    | `T?`, `null` | Nullable values                         |
| `Result<T, E>` | —            | Fallible operations (`E: Error`)        |
| `Range<T>`     | `..`, `..=`  | Interval type for iteration and slicing |

### 26.2 Compiler-Known Contracts

| Contract                          | Special Behavior                                                                                                                                                                              |
|-----------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Error`                           | Required bound for Result's `E` parameter. Requires `message() -> string`.                                                                                                                    |
| `Into<T>`                         | Type conversion. Called explicitly via `.into<T>()`. Implicitly called as `.into<string>()` by `{expr}` interpolation in formattable strings (`$"..."`) and dialogue lines. See Section 10.2. |
| `Add`, `Sub`, `Mul`, `Div`, `Mod` | Mapped from `operator +`, `-`, `*`, `/`, `%` syntax.                                                                                                                                          |
| `Neg`, `Not`                      | Mapped from unary `-` and `!` syntax.                                                                                                                                                         |
| `Eq`, `Ord`                       | Mapped from `operator ==`, `<`. Derived: `!=`, `>`, `<=`, `>=`.                                                                                                                               |
| `Index<K, V>`, `IndexSet<K, V>`   | Mapped from `operator []` (read) and `operator []=` (write) syntax.                                                                                                                           |
| `BitAnd`, `BitOr`                 | Mapped from `operator &`, `\|`.                                                                                                                                                               |
| `Iterable<T>`, `Iterator<T>`      | Enable `for` loop iteration. `T[]` and `Range<T>` have compiler-provided implementations. See Section 10.3.                                                                                   |

### 26.3 Standard Library Types

These types are provided by the standard library with no special compiler support:

| Type            | Description                                                    |
|-----------------|----------------------------------------------------------------|
| `List<T>`       | Ordered, growable collection                                   |
| `Map<K, V>`     | Key-value associative collection                               |
| `Set<T>`        | Unordered unique collection                                    |
| `EntityList<T>` | Typed entity reference collection with component query support |

---

