# 1.11 Contracts

Contracts define a set of methods and/or operators that a type must implement. They serve the role of interfaces/traits
and are the foundation for bounded generics, operator overloading, and component polymorphism.

```writ
contract Interactable {
    fn onInteract(mut self, who: Entity);
}

contract Tradeable {
    fn getInventory(self) -> List<Item>;
    fn trade(mut self, item: Item, with: Entity);
}

// Implementation for a struct
impl Interactable for Merchant {
    fn onInteract(mut self, who: Entity) {
        // open trade dialog
    }
}

// Using contracts as bounds
fn interactWith(mut thing: Interactable) {
    thing.onInteract(player);
}
```

## 1.11.1 Builtin Contracts

These contracts are implicitly defined by the compiler and map to operator syntax or special behavior:

**Arithmetic operators:**

| Contract    | Operator     | Signature                   |
|-------------|--------------|-----------------------------|
| `Add<T, R>` | `+`          | `operator +(other: T) -> R` |
| `Sub<T, R>` | `-` (binary) | `operator -(other: T) -> R` |
| `Mul<T, R>` | `*`          | `operator *(other: T) -> R` |
| `Div<T, R>` | `/`          | `operator /(other: T) -> R` |
| `Mod<T, R>` | `%`          | `operator %(other: T) -> R` |
| `Neg<R>`    | `-` (unary)  | `operator -() -> R`         |
| `Not<R>`    | `!` (unary)  | `operator !() -> R`         |

**Comparison operators:**

| Contract | Operator | Signature                       |
|----------|----------|---------------------------------|
| `Eq<T>`  | `==`     | `operator ==(other: T) -> bool` |
| `Ord<T>` | `<`      | `operator <(other: T) -> bool`  |

> `!=` is auto-derived as `!(a == b)` from `Eq`. `>`, `<=`, `>=` are auto-derived from `Ord` and `Eq`. These cannot be
> overridden individually.

**Indexing operators:**

| Contract         | Operator           | Signature                        |
|------------------|--------------------|----------------------------------|
| `Index<K, V>`    | `x[k]` (read)      | `operator [](key: K) -> V`       |
| `IndexSet<K, V>` | `x[k] = v` (write) | `operator []=(key: K, value: V)` |

> A type may implement `Index` without `IndexSet` (read-only indexing). Attempting `x[k] = v` on a type that only
> implements `Index` is a compile error.

**Bitwise operators:**

| Contract       | Operator | Signature                    |
|----------------|----------|------------------------------|
| `BitAnd<T, R>` | `&`      | `operator &(other: T) -> R`  |
| `BitOr<T, R>`  | `\|`     | `operator \|(other: T) -> R` |

**Iteration:**

| Contract      | Behavior                        | Signature                          |
|---------------|---------------------------------|------------------------------------|
| `Iterable<T>` | Enables `for` loops (see 10.3)  | `fn iterator(self) -> Iterator<T>` |
| `Iterator<T>` | Produces elements one at a time | `fn next(mut self) -> T?`          |

**Conversion and special contracts:**

| Contract  | Behavior                   | Signature                    |
|-----------|----------------------------|------------------------------|
| `Into<T>` | Type conversion (see 10.2) | `fn into(self) -> T`         |
| `Error`   | Result `E` bound           | `fn message(self) -> string` |

> **Note:** When a user writes `operator +` in an `impl` block, the compiler automatically registers it as an
> implementation of the `Add` contract. Users never need to write `impl Add<...> for ...` directly.

**Compound assignment:** Operators `+=`, `-=`, `*=`, `/=`, `%=` are syntactic sugar. `a += b` desugars to `a = a + b`
and dispatches through the corresponding arithmetic contract. They are not independently overloadable.

## 1.11.2 Into\<T\> — Type Conversion

The `Into<T>` contract is the universal conversion mechanism. A type may implement `Into<T>` for multiple target types.

```writ
struct HealthInfo {
    current: int,
    max: int,
}

impl Into<string> for HealthInfo {
    fn into(self) -> string {
        $"{self.current}/{self.max}"
    }
}

impl Into<float> for HealthInfo {
    fn into(self) -> float {
        self.current / self.max
    }
}
```

**Calling convention:** Conversions are always invoked with an explicit type parameter on the call site:

```writ
let label = hp.into<string>();    // "75/100"
let ratio = hp.into<float>();     // 0.75
```

The `<T>` on the call disambiguates which `Into<T>` implementation to dispatch. There is no implicit conversion at
assignment or argument boundaries — the caller must be explicit.

**Exception — formattable strings and dialogue lines:** When an expression appears in an interpolation slot (`{expr}`
inside `$"..."` or dialogue text), the compiler implicitly calls `.into<string>()`. This is the only context where
`Into<T>` is invoked without an explicit call.

```writ
let hp = new HealthInfo { current: 75, max: 100 };
let msg = $"HP: {hp}";
// Equivalent to: $"HP: {hp.into<string>()}"
```

> **Note:** All primitive types (`int`, `float`, `bool`, `string`) have built-in `Into<string>` implementations provided
> by the compiler.

## 1.11.3 Iterable\<T\> — For Loop Support

The `Iterable<T>` and `Iterator<T>` contracts enable any type to be used with `for` loops.

`Iterable<T>` is implemented on the collection. It returns an `Iterator<T>`, which produces elements one at a time via
`next()`. When `next()` returns `null`, iteration ends.

```writ
// A for loop:
for item in collection {
    process(item);
}

// Desugars to:
{
    let mut _iter = collection.iterator();
    let mut _next = _iter.next();
    while _next != null {
        let item = _next!;
        process(item);
        _next = _iter.next();
    }
}
```

The following types have compiler-provided `Iterable<T>` implementations:

| Type           | Element Type | Behavior                                            |
|----------------|--------------|-----------------------------------------------------|
| `T[]`          | `T`          | Iterates elements in order                          |
| `Range<int>`   | `int`        | Iterates from start to end (exclusive or inclusive) |
| `Range<float>` | `float`      | Iterates in increments of 1.0                       |

User-defined types can implement `Iterable<T>` to participate in `for` loops:

```writ
impl Iterable<Entity> for Party {
    fn iterator(self) -> Iterator<Entity> {
        self.members.iterator()
    }
}

// Now usable in for loops:
for member in party {
    if let Option::Some(hp) = member[Health] {
        hp.current = min(hp.current + 10, hp.max);
    }
}
```

## 1.11.4 Contract-as-Type

A contract name may appear anywhere a type annotation is valid. This means contracts can be used as the declared type of variables, function parameters, and return types. When a contract name is used as a type, the binding holds a value of some concrete type that implements the contract, with the specific concrete type erased at the annotation site.

### Type Annotation Syntax

```writ
contract Speakable {
    fn speak(self) -> string;
}

// Variable declaration
let s: Speakable = someNpc;

// Parameter
fn greet(speaker: Speakable) {
    say(speaker.speak());
}

// Return type
fn getGreeter() -> Speakable {
    return someNpc;
}
```

### Assignability Rules

A value of concrete type `T` is assignable to a binding of contract type `C` if and only if `T` implements `C`. If `T` does not implement `C`, the compiler emits a type error. This rule applies uniformly at variable initialization, assignment, argument passing, and return statements.

```writ
contract Speakable {
    fn speak(self) -> string;
}

struct Villager { name: string }

impl Speakable for Villager {
    fn speak(self) -> string {
        return $"Hello, I'm {self.name}";
    }
}

struct Rock { weight: int }
// Rock does NOT implement Speakable

let a: Speakable = new Villager { name: "Ada" };  // OK — Villager implements Speakable
let b: Speakable = new Rock { weight: 50 };        // Error — Rock does not implement Speakable
```

### Virtual Dispatch

When a method is called on a value whose static type is a contract, the call dispatches virtually at runtime. The runtime determines the concrete type of the underlying value and invokes the corresponding method implementation. At the IL level, these calls are emitted as `CALL_VIRT` instructions rather than `CALL`, carrying the contract index and method slot to enable runtime dispatch through the contract's vtable.

```writ
fn makeSpeak(s: Speakable) {
    // s.speak() dispatches virtually — the runtime looks up the concrete
    // type's implementation of Speakable.speak and calls it via CALL_VIRT
    let message = s.speak();
    say(message);
}
```

> **Note:** Contract-typed values do not support direct field access — only methods declared in the contract are callable through the contract type. To access type-specific members, the value must be used through its concrete type.

---

