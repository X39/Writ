# Writ IL Specification
## 2.18 `writ-runtime` Module Contents

The `writ-runtime` module is a virtual module provided by every conforming runtime. It is not compiled from Writ
source — the runtime supplies it as part of its implementation. The compiler references types, contracts, and methods
in `writ-runtime` via standard cross-module TypeRef and MethodRef resolution, exactly as it would reference any other
dependency module.

The spec mandates the types, contracts, and implementations listed below. The runtime must provide these with the
exact layouts specified. Additional types or methods beyond this list are permitted but not required.

### 2.18.1 Core Enums

#### Option\<T\>

```
enum Option<T> {
    None,           // tag 0 — no payload
    Some(value: T), // tag 1 — payload: T
}
```

Tag assignments are mandatory: `None = 0`, `Some = 1`. Zero-initialization of an Option register produces `None`.
The specialized IL instructions (`WRAP_SOME`, `UNWRAP`, `IS_SOME`, `IS_NONE`) depend on these tag values.

The `T?` syntax is sugar for `Option<T>`. The `null` literal is sugar for `Option::None`.

#### Result\<T, E: Error\>

```
enum Result<T, E: Error> {
    Ok(value: T),     // tag 0 — payload: T
    Err(error: E),    // tag 1 — payload: E
}
```

Tag assignments are mandatory: `Ok = 0`, `Err = 1`. The `E` parameter is constrained to the `Error` contract
(§2.18.3). Specialized IL instructions: `WRAP_OK`, `WRAP_ERR`, `UNWRAP_OK`, `IS_OK`, `IS_ERR`, `EXTRACT_ERR`.

### 2.18.2 Range\<T\>

```
struct Range<T> {
    start: T,
    end: T,
    start_inclusive: bool,
    end_inclusive: bool,
}
```

The `..` and `..=` operators construct Range values:

| Syntax  | start_inclusive | end_inclusive |
|---------|-----------------|---------------|
| `a..b`  | `true`          | `false`       |
| `a..=b` | `true`          | `true`        |

Range is a generic struct with no constraints on `T`. The type is deliberately general — while the current syntax
only produces ranges with `start_inclusive = true`, the struct supports all four inclusivity combinations for use by
library code and future language extensions.

**Range iteration:** The runtime provides `Iterable<T>` implementations for `Range<int>` (step by 1) and
`Range<float>` (step by 2.0). User types may provide their own `Iterable<T>` implementations for `Range<UserType>`
to support custom range iteration.

**Range indexing:** Arrays and strings use `Range<int>` for slice operations via `Index<Range<int>, T[]>` and
`Index<Range<int>, string>` (§6.9).

### 2.18.3 Contracts

The following contracts are defined in `writ-runtime`. The compiler maps operator syntax to these contracts
automatically (§10.1, §2.7). Each contract produces a `ContractDef` row in the `writ-runtime` module metadata.

**Arithmetic:**

| Contract    | Method Signature            | Operator  |
|-------------|-----------------------------|-----------|
| `Add<T, R>` | `operator +(other: T) -> R` | `+`       |
| `Sub<T, R>` | `operator -(other: T) -> R` | `-`       |
| `Mul<T, R>` | `operator *(other: T) -> R` | `*`       |
| `Div<T, R>` | `operator /(other: T) -> R` | `/`       |
| `Mod<T, R>` | `operator %(other: T) -> R` | `%`       |
| `Neg<R>`    | `operator -() -> R`         | unary `-` |
| `Not<R>`    | `operator !() -> R`         | `!`       |

**Comparison:**

| Contract | Method Signature                | Operator |
|----------|---------------------------------|----------|
| `Eq<T>`  | `operator ==(other: T) -> bool` | `==`     |
| `Ord<T>` | `operator <(other: T) -> bool`  | `<`      |

Derived operators `!=`, `>`, `<=`, `>=` are compiler desugaring from `Eq` and `Ord` (§17.4).

**Indexing:**

| Contract         | Method Signature                 | Operator   |
|------------------|----------------------------------|------------|
| `Index<K, V>`    | `operator [](key: K) -> V`       | `x[k]`     |
| `IndexSet<K, V>` | `operator []=(key: K, value: V)` | `x[k] = v` |

**Bitwise:**

| Contract       | Method Signature             | Operator |
|----------------|------------------------------|----------|
| `BitAnd<T, R>` | `operator &(other: T) -> R`  | `&`      |
| `BitOr<T, R>`  | `operator \|(other: T) -> R` | `\|`     |

**Iteration:**

| Contract      | Method Signature                   |
|---------------|------------------------------------|
| `Iterable<T>` | `fn iterator(self) -> Iterator<T>` |
| `Iterator<T>` | `fn next(mut self) -> T?`          |

**Conversion and error:**

| Contract  | Method Signature             |
|-----------|------------------------------|
| `Into<T>` | `fn into(self) -> T`         |
| `Error`   | `fn message(self) -> string` |

### 2.18.4 Primitive Pseudo-Types

Primitive types (`int`, `float`, `bool`, `string`) have fixed type tags (§2.15.2) and are not constructed via `NEW`.
To anchor contract implementations in the metadata, `writ-runtime` provides **pseudo-TypeDefs** for each primitive.
These TypeDefs exist solely as targets for `ImplDef` entries — they are not constructable by user code and carry no
user-visible fields or methods beyond their contract implementations.

| Pseudo-TypeDef | Primitive Tag | Purpose                          |
|----------------|---------------|----------------------------------|
| `Int`          | `0x01`        | Anchor for int contract impls    |
| `Float`        | `0x02`        | Anchor for float contract impls  |
| `Bool`         | `0x03`        | Anchor for bool contract impls   |
| `String`       | `0x04`        | Anchor for string contract impls |

The runtime maps primitive type tags to these pseudo-TypeDefs for contract dispatch. When generic code calls a
contract method on a boxed `int`, the runtime resolves via the `Int` pseudo-TypeDef's `ImplDef` entries.

### 2.18.5 Primitive Contract Implementations

All primitive contract implementations are intrinsic — the runtime provides native implementations that correspond
to dedicated IL instructions. For direct primitive operations, the compiler emits the dedicated instruction (§2.7).
The `ImplDef` entries exist for generic dispatch when primitives are boxed through generic parameters.

**int:**

| Contract           | Intrinsic Instruction      |
|--------------------|----------------------------|
| `Add<int, int>`    | `ADD_I`                    |
| `Sub<int, int>`    | `SUB_I`                    |
| `Mul<int, int>`    | `MUL_I`                    |
| `Div<int, int>`    | `DIV_I`                    |
| `Mod<int, int>`    | `MOD_I`                    |
| `Neg<int>`         | `NEG_I`                    |
| `Not<int>`         | `NOT` (bitwise complement) |
| `Eq<int>`          | `CMP_EQ_I`                 |
| `Ord<int>`         | `CMP_LT_I`                 |
| `BitAnd<int, int>` | `BIT_AND`                  |
| `BitOr<int, int>`  | `BIT_OR`                   |
| `Into<float>`      | `I2F`                      |
| `Into<string>`     | `I2S`                      |

**float:**

| Contract            | Intrinsic Instruction |
|---------------------|-----------------------|
| `Add<float, float>` | `ADD_F`               |
| `Sub<float, float>` | `SUB_F`               |
| `Mul<float, float>` | `MUL_F`               |
| `Div<float, float>` | `DIV_F`               |
| `Mod<float, float>` | `MOD_F`               |
| `Neg<float>`        | `NEG_F`               |
| `Eq<float>`         | `CMP_EQ_F`            |
| `Ord<float>`        | `CMP_LT_F`            |
| `Into<int>`         | `F2I`                 |
| `Into<string>`      | `F2S`                 |

**bool:**

| Contract       | Intrinsic Instruction    |
|----------------|--------------------------|
| `Eq<bool>`     | `CMP_EQ_B`               |
| `Not<bool>`    | `NOT` (logical negation) |
| `Into<string>` | `B2S`                    |

**string:**

| Contract                    | Intrinsic                    |
|-----------------------------|------------------------------|
| `Add<string, string>`       | `STR_CONCAT`                 |
| `Eq<string>`                | `CMP_EQ_S`                   |
| `Ord<string>`               | Intrinsic (lexicographic)    |
| `Index<int, string>`        | Intrinsic (single character) |
| `Index<Range<int>, string>` | Intrinsic (substring)        |
| `Into<string>`              | Identity (returns self)      |

### 2.18.6 Array Type

The `Array<T>` TypeDef provides methods and contract implementations for the built-in array type. The `T[]` syntax
is sugar for `Array<T>`. In the type encoding, arrays use kind `0x20` (§2.15.3); the runtime maps this to the
`Array<T>` TypeDef for method resolution and contract dispatch.

**Fields:**

| Field    | Type  | Access    | Intrinsic   |
|----------|-------|-----------|-------------|
| `length` | `int` | Read-only | `ARRAY_LEN` |

**Methods (intrinsic):**

| Method     | Signature                                  | Intrinsic IL   |
|------------|--------------------------------------------|----------------|
| `add`      | `fn add(mut self, item: T)`                | `ARRAY_ADD`    |
| `removeAt` | `fn removeAt(mut self, index: int)`        | `ARRAY_REMOVE` |
| `insert`   | `fn insert(mut self, index: int, item: T)` | `ARRAY_INSERT` |
| `contains` | `fn contains(self, item: T) -> bool`       | Intrinsic      |
| `slice`    | `fn slice(self, range: Range<int>) -> T[]` | `ARRAY_SLICE`  |
| `iterator` | `fn iterator(self) -> Iterator<T>`         | Intrinsic      |

The `contains` method requires `T: Eq` at the call site. This constraint is enforced by the compiler — it is not
encoded on the `Array<T>` TypeDef's generic parameter, which would incorrectly restrict all array usage to
Eq-implementing types.

**Contract implementations:**

| Contract                      | Intrinsic                                         |
|-------------------------------|---------------------------------------------------|
| `Index<int, T>`               | `ARRAY_LOAD` — crashes on out-of-bounds           |
| `IndexSet<int, T>`            | `ARRAY_STORE` — crashes on out-of-bounds          |
| `Index<Range<int>, Array<T>>` | `ARRAY_SLICE`                                     |
| `Iterable<T>`                 | Returns a runtime-provided iterator over elements |

### 2.18.7 Entity Base Type

The `Entity` TypeDef (kind=Entity) serves as the base handle type for all entity references. When a variable is
typed as `Entity` (rather than a specific entity type like `Guard`), it refers to this base type. All user-defined
entity types are assignable to `Entity` for handle operations.

**Static methods (intrinsic):**

| Method        | Signature                            | Intrinsic IL      |
|---------------|--------------------------------------|-------------------|
| `destroy`     | `fn destroy(entity: Entity)`         | `DESTROY_ENTITY`  |
| `isAlive`     | `fn isAlive(entity: Entity) -> bool` | `ENTITY_IS_ALIVE` |
| `getOrCreate` | `fn getOrCreate<T>() -> T`           | `GET_OR_CREATE`   |
| `findAll`     | `fn findAll<T>() -> EntityList<T>`   | `FIND_ALL`        |

### 2.18.8 Versioning

The `writ-runtime` module version tracks the IL specification version. A major version bump in the IL spec
corresponds to a major version bump in `writ-runtime`. Compiled modules reference `writ-runtime` via `ModuleRef`
with a `min_version` matching the IL spec version they were compiled against.

Since `writ-runtime` is provided by the runtime rather than loaded from disk, the runtime ensures its provided module
matches the version expected by loaded user modules. On version mismatch, the runtime reports the conflict via the
logging interface (§2.14.7).

