# Writ IL Specification
## 3.15 Boxing

Boxing is required when value types (`int`, `float`, `bool`, enums) pass through generic parameters. The compiler emits
`BOX` before passing a value type to a generic parameter and `UNBOX` when extracting a concrete value type from a
generic return or field.

| Mnemonic | Shape | Operands     | Description                                                                                                                                                                                                       |
|----------|-------|--------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `BOX`    | RR    | r_dst, r_src | Heap-allocate a boxed object containing the value in r_src. r_dst receives a reference to the box. The runtime reads r_src's declared type from the register type table to determine the box layout and type tag. |
| `UNBOX`  | RR    | r_dst, r_src | Extract the value from a boxed reference in r_src into r_dst. The runtime reads r_dst's declared type to verify the box contents match. Crash on type mismatch.                                                   |

No explicit type token is needed — registers are abstract typed slots (§2.5), so the runtime already knows every
register's type from the method body's register type table (§2.16.6).

**When the compiler emits boxing:**

```
// Source: fn identity<T>(val: T) -> T { val }
// Call site: let x = identity(42);

// Caller:
LOAD_INT    r0, 42
BOX         r1, r0              // box int for generic param
CALL        r2, identity, r1, 1
UNBOX       r3, r2              // unbox return value back to int
```

Reference types (`string`, structs, arrays, entities, delegates) are already references and pass through generics
without boxing.

