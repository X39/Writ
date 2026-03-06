# 1. Writ Language Specification
## 1.8 Structs

Structs are value types — named composite types with named fields, inline storage, copy-on-assign. They support methods
and operator overloading via `impl` blocks. Structs have no lifecycle hooks and no heap allocation; they are pure data
with copy semantics. For reference semantics, heap allocation, or lifecycle hooks, use `class` (see Section 1.9).

```
struct Vec2 {
    x: float,
    y: float,
}

impl Vec2 {
    fn length(self) -> float {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

// Construction uses the `new` keyword with named fields
let v = new Vec2 { x: 1.0, y: 2.0 };
```

### 1.8.1 Construction

Structs are constructed with the `new` keyword followed by the type name and brace-enclosed field initializers. Fields
with default values may be omitted. Fields without defaults are required at every construction site.

`new` for a struct does **not** allocate heap memory. The value is initialized inline — the struct lives in a register
or on the stack, not on the GC heap. This contrasts with classes, where `new` allocates on the GC heap (see Section 1.9).

```
let v = new Vec2 { x: 1.0, y: 2.0 };
let origin = new Vec2 { x: 0.0, y: 0.0 };
```

The `new` keyword disambiguates construction from block expressions, making the syntax unambiguous for the parser.

For convenience factories, use static methods in `impl` blocks:

```
impl Vec2 {
    fn zero() -> Vec2 {
        new Vec2 { x: 0.0, y: 0.0 }
    }

    fn splat(v: float) -> Vec2 {
        new Vec2 { x: v, y: v }
    }
}

let origin = Vec2::zero();
```

### 1.8.2 Shallow Copy Semantics

Assignment copies all fields by value. Reference-typed fields (such as `string` or `Array<T>`) copy the pointer — both
copies share the same referenced object, but the struct values themselves are independent.

```
struct Vec2 { x: float, y: float }
struct Note { text: string, priority: int }  // string field is a reference

let a = new Vec2 { x: 1.0, y: 2.0 };
let b = a;           // b is a fresh copy: { x: 1.0, y: 2.0 }
// a and b are independent; mutation to b.x does NOT affect a.x

let c = new Note { text: "hello", priority: 1 };
let d = c;           // d.text and c.text point to the same string object
                     // but the Note struct itself is two independent copies
```

### 1.8.3 Structural Equality

Value-type structs auto-derive field-by-field equality. Two structs are equal if and only if all corresponding fields
compare equal using the standard equality rules for each field's type. Reference-typed fields use reference equality.

Classes require explicit `Eq` contract implementation — they have no auto-derived equality (see Section 1.9).

### 1.8.4 Passing Semantics

Value-type structs are always passed by copy. When a struct is passed as a function argument, the callee receives an
independent copy. `mut self` on a method mutates the local copy, not the caller's value. This is identical to how
`int` and `float` behave.

```
fn translate(pos: Vec2, delta: Vec2) -> Vec2 {
    new Vec2 { x: pos.x + delta.x, y: pos.y + delta.y }
}
// pos is a copy inside translate -- the caller's original value is unchanged
```

### 1.8.5 Recursive Structs

Recursive value-type structs are illegal. A value-type struct that directly or indirectly contains itself has infinite
size, which is a compile-time error. The compiler detects this by walking the type graph during type size computation
and reports the cycle by name.

```
struct Bad {
    child: Bad,       // ERROR: infinite size — value-type struct cannot contain itself
}

struct Node {
    data: int,
    next: Node?,      // Still illegal for value types — Option<Node> has infinite size too
}
```

Use a `class` for recursive data structures (see Section 1.9). Class instances are heap-allocated references, so a class
field of its own type holds a pointer, not an inline copy.

### 1.8.6 Construction Sequence (IL)

`new Vec2 { x: 1.0, y: 2.0 }` compiles to the following IL:

```
NEW           r0, Vec2_type        // initialize value inline (no heap allocation)
LOAD_FLOAT    r1, 1.0
SET_FIELD     r0, x_field, r1
LOAD_FLOAT    r1, 2.0
SET_FIELD     r0, y_field, r1
```

No `CALL __on_create` step — value-type structs have no lifecycle hooks.

The full sequence:

1. **NEW** — initialize the struct value in-place (no GC heap allocation).
2. **SET_FIELD** — apply default values for all fields that have them.
3. **SET_FIELD** — apply construction-site overrides (these overwrite defaults where specified).

---
