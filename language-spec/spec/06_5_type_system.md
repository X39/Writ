# 1. Writ Language Specification
## 1.5 Type System

Writ uses a static type system with type inference for local variables. Types are checked at compile time. The runtime
carries type tags for dynamic dispatch of contract methods.

### 1.5.1 Type Categories

| Category       | Examples                          | Notes                                                        |
|----------------|-----------------------------------|--------------------------------------------------------------|
| Primitives     | `int`, `float`, `bool`, `string`  | Value types, always non-null, lowercase keywords             |
| Arrays         | `T[]`                             | Fixed-type, growable ordered collections with literal syntax |
| Structs        | `struct Vec2 { ... }`             | Value types -- user-defined composite types with copy semantics |
| Classes        | `class Merchant { ... }`          | Reference types -- user-defined composite types with heap allocation |
| Entities       | `entity Guard { ... }`            | Game objects (specialized classes) with components and lifecycle |
| Components     | `extern component Health { ... }` | Extern data schemas attached to entities via `use`           |
| Enums          | `enum QuestStatus { ... }`        | Tagged unions with variant data                              |
| Ranges         | `Range<T>`                        | Compiler-known interval type, created with `..` and `..=`    |
| Nullable       | `T?`                              | Sugar for `Option<T>`                                        |
| Result         | `Result<T, E>`                    | `E` must implement `Error` contract                          |
| Function types | `fn(int, int) -> int`             | First-class function references                              |
| Generic        | `T`, `T: Contract`                | Bounded or unbounded type parameters                         |

### 1.5.2 Type Inference

Local variable types are inferred from their initializer. Function signatures, struct fields, and entity properties must
be fully annotated.

```
let x = 42;                // inferred as int
let name = "hello";        // inferred as string
let pos = new vec2 { x: 1.0, y: 2.0 };  // inferred as vec2

// Function signatures require explicit types
fn add(a: int, b: int) -> int {
    a + b
}
```

---

