# 1. Writ Language Specification
## 17. Operators & Overloading

### 17.1 Operator Precedence (highest to lowest)

| Prec | Operators                    | Assoc | Notes                                        |
|------|------------------------------|-------|----------------------------------------------|
| 1    | `.` `[]` `!` `?`             | Left  | Member access, index, unwrap, null-propagate |
| 2    | `- (unary)` `! (unary)`      | Right | Negation, logical NOT                        |
| 3    | `*` `/` `%`                  | Left  | Multiplication, division, modulo             |
| 4    | `+` `-`                      | Left  | Addition, subtraction                        |
| 5    | `<<` `>>`                    | Left  | Bit shifts                                   |
| 6    | `&`                          | Left  | Bitwise AND                                  |
| 7    | `\|`                         | Left  | Bitwise OR                                   |
| 8    | `<` `>` `<=` `>=`            | Left  | Comparison                                   |
| 9    | `==` `!=`                    | Left  | Equality                                     |
| 10   | `&&`                         | Left  | Logical AND (short-circuit)                  |
| 11   | `\|\|`                       | Left  | Logical OR (short-circuit)                   |
| 12   | `..` `..=`                   | Left  | Range (exclusive, inclusive)                 |
| 13   | `=` `+=` `-=` `*=` `/=` `%=` | Right | Assignment                                   |

### 17.2 Overloading Syntax

Operators are overloaded inside `impl` blocks using the `operator` keyword. The compiler automatically maps these to the
corresponding builtin contract.

```
impl vec2 {
    // Binary operators
    operator +(other: vec2) -> vec2 {
        vec2 { x: self.x + other.x, y: self.y + other.y }
    }

    operator *(scalar: float) -> vec2 {
        vec2 { x: self.x * scalar, y: self.y * scalar }
    }

    operator %(other: vec2) -> vec2 {
        vec2 { x: self.x % other.x, y: self.y % other.y }
    }

    // Unary operators
    operator -() -> vec2 {
        vec2 { x: -self.x, y: -self.y }
    }

    // Comparison
    operator ==(other: vec2) -> bool {
        self.x == other.x && self.y == other.y
    }

    // Index read
    operator [](index: int) -> float {
        match index {
            0 => { self.x }
            1 => { self.y }
        }
    }

    // Index write
    operator []=(index: int, value: float) {
        match index {
            0 => { self.x = value; }
            1 => { self.y = value; }
        }
    }
}

// Implicitly registers: impl Add<vec2, vec2> for vec2,
//   impl Neg<vec2> for vec2, impl Index<int, float> for vec2, etc.
```

### 17.3 Compound Assignment

Compound assignment operators (`+=`, `-=`, `*=`, `/=`, `%=`) are syntactic sugar. They are not independently
overloadable.

```
a += b;     // desugars to: a = a + b;   (uses Add)
a -= b;     // desugars to: a = a - b;   (uses Sub)
a *= b;     // desugars to: a = a * b;   (uses Mul)
a /= b;     // desugars to: a = a / b;   (uses Div)
a %= b;     // desugars to: a = a % b;   (uses Mod)
```

### 17.4 Derived Operators

Some operators are auto-derived from a base implementation and cannot be overridden individually:

| Derived  | Base         | Rule                |
|----------|--------------|---------------------|
| `a != b` | `Eq`         | `!(a == b)`         |
| `a > b`  | `Ord`        | `b < a`             |
| `a <= b` | `Eq` + `Ord` | `a < b \|\| a == b` |
| `a >= b` | `Eq` + `Ord` | `!(a < b)`          |

---

