# 1. Writ Language Specification
## 6. Primitive Types

| Type     | Description                      | Default Value |
|----------|----------------------------------|---------------|
| `int`    | 64-bit signed integer            | `0`           |
| `float`  | 64-bit IEEE 754 floating point   | `0.0`         |
| `bool`   | Boolean value                    | `false`       |
| `string` | UTF-8 encoded string (immutable) | `""`          |

### 6.1 Arrays

Arrays are ordered, growable, homogeneous collections. They are a compiler-known semi-primitive — not a single machine
word, but with literal syntax and built-in operations. Like `string`, the compiler understands arrays directly; they are
not a standard library type.

The array type is written with postfix `[]` notation: `int[]`, `string[]`, `vec2[]`.

| Type  | Description                         | Default Value | Literal Syntax |
|-------|-------------------------------------|---------------|----------------|
| `T[]` | Ordered, growable collection of `T` | `[]`          | `[expr, ...]`  |

### 6.2 Array Literals

The `[expr, expr, ...]` syntax constructs an array. All elements must be the same type. The element type is inferred
from the contents or from the expected type context.

```
let numbers = [1, 2, 3];              // int[]
let names = ["Alice", "Bob"];          // string[]
let empty: int[] = [];                 // empty array, type from annotation
let mixed = [1, 2.0];                 // COMPILE ERROR: int and float are not the same type
```

An empty literal `[]` requires a type context — either a variable type annotation or an expected parameter type. Without
context, it is a compile error.

```
let items: int[] = [];                 // ok: type from annotation
fn process(items: int[]) { ... }
process([]);                           // ok: type from parameter

let unknown = [];                      // COMPILE ERROR: cannot infer element type
```

### 6.3 Array Operations

The following operations are compiler-known and provided by the runtime. Arrays are mutable — elements can be added and
removed. The array itself must be `let mut` to allow structural mutation (add/remove). Element assignment through
indexing also requires `let mut`.

| Operation              | Signature                 | Description                                                                                |
|------------------------|---------------------------|--------------------------------------------------------------------------------------------|
| `.length`              | `int` (read-only)         | Number of elements.                                                                        |
| `[index]`              | `T`                       | Access element by zero-based index. Out-of-bounds crashes the task (with defer unwinding). |
| `.add(item)`           | `fn(item: T)`             | Append an element to the end.                                                              |
| `.removeAt(index)`     | `fn(index: int)`          | Remove the element at the given index. Out-of-bounds crashes the task.                     |
| `.insert(index, item)` | `fn(index: int, item: T)` | Insert an element at the given index, shifting subsequent elements.                        |
| `.contains(item)`      | `fn(item: T) -> bool`     | Returns `true` if the item is in the array. `T` must implement `Eq`.                       |
| `.iterator()`          | `Iterator<T>`             | Returns an iterator over elements. Arrays implement `Iterable<T>` (see Section 10.3).      |

```
let mut inventory = ["Sword", "Shield"];
inventory.add("Potion");              // ["Sword", "Shield", "Potion"]
inventory.removeAt(0);                // ["Shield", "Potion"]
let count = inventory.length;         // 2
let has = inventory.contains("Shield"); // true

// Immutable array — structure cannot change
let fixed = [1, 2, 3];
fixed.add(4);                         // COMPILE ERROR: fixed is not mutable
let x = fixed[0];                     // ok: reading is always permitted
```

### 6.4 Array Indexing

Array indexing uses the `[]` operator. It returns `T` directly (not `Option<T>`). Out-of-bounds access crashes the
current task with defer unwinding — this matches the crash semantics of `!` and failed library loads.

```
let items = [10, 20, 30];
let first = items[0];                 // 10
let bad = items[99];                  // RUNTIME CRASH: index out of bounds
```

### 6.5 Parser Disambiguation

The `[` token has three roles depending on context:

1. **Array literal** — at the start of an expression (after `=`, as argument, etc.): `[1, 2, 3]`
2. **Index / component access** — as a postfix operator after an expression: `items[0]`, `guard[Health]`
3. **Attribute** — at statement level before a declaration keyword: `[Singleton]`

The parser resolves (1) vs (2) by position: `[` at the start of an expression is a literal, `[` after an expression is
postfix. For (3), see [Section 16.3](#163-parser-disambiguation).

> **Note:** `string` is listed in the primitive types table despite not being a machine word, because it is a language
> keyword. Arrays follow the same pattern — compiler-known, with dedicated syntax, but not a single machine word.
> Standard
> library types like `List<T>` may provide higher-level collection abstractions on top of arrays.

### 6.6 Ranges

`Range<T>` is a compiler-known type representing an interval between two values. It is created with the `..` (exclusive
end) or `..=` (inclusive end) operators.

```
let r = 0..10;        // Range<int>, exclusive: [0, 10)
let ri = 0..=10;      // Range<int>, inclusive: [0, 10]
let pct = 0.0..1.0;   // Range<float>, exclusive
```

Start or end may be omitted when used inside `[]` indexing to mean "from the beginning" or "to the end":

```
let items = [10, 20, 30, 40, 50];
items[1..4]     // [20, 30, 40]
items[..3]      // [10, 20, 30]
items[2..]      // [30, 40, 50]
```

### 6.7 From-End Indexing with ^

Inside `[]` indexing, the `^n` syntax means "n from the end." The compiler desugars `^n` to `collection.length - n` at
the call site. `^` is only valid inside `[]` — it is not a general-purpose operator.

```
let items = [10, 20, 30, 40, 50];
items[^1]       // 50 (last element, desugars to items[items.length - 1])
items[^2]       // 40 (second from end)
items[..^1]     // [10, 20, 30, 40] (everything except last)
items[^3..^1]   // [30, 40] (third from end to second from end)

let text = "Hello, world!";
text[..^1]      // "Hello, world" (drop last char)
text[7..]       // "world!"
```

### 6.8 Range in For Loops

Ranges are iterable. When used with `for`, exclusive ranges (`..`) iterate up to but not including the end, and
inclusive ranges (`..=`) include the end:

```
for i in 0..5 {
    // i = 0, 1, 2, 3, 4
}

for i in 1..=5 {
    // i = 1, 2, 3, 4, 5
}
```

### 6.9 Range Indexing Contract

Types that support range-based slicing implement `Index<Range<int>, R>` where `R` is the return type of the slice.
Arrays and strings have compiler-provided implementations:

| Type     | Index Key    | Returns  | Description      |
|----------|--------------|----------|------------------|
| `T[]`    | `int`        | `T`      | Single element   |
| `T[]`    | `Range<int>` | `T[]`    | Sub-array (copy) |
| `string` | `Range<int>` | `string` | Substring        |

User-defined types may implement `Index<Range<int>, R>` to support range-based slicing via the standard operator
overloading mechanism.

---

