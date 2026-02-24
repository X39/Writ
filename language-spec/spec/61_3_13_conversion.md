# Writ IL Specification
## 3.13 Conversion

**Primitive-to-primitive (specialized, no dispatch):**

| Mnemonic | Shape | Operands     | Description                                                 |
|----------|-------|--------------|-------------------------------------------------------------|
| `I2F`    | RR    | r_dst, r_src | int -> float. Exact when possible, nearest float otherwise. |
| `F2I`    | RR    | r_dst, r_src | float -> int. Truncation toward zero.                       |
| `I2S`    | RR    | r_dst, r_src | int -> string. Decimal representation.                      |
| `F2S`    | RR    | r_dst, r_src | float -> string. Runtime-defined precision.                 |
| `B2S`    | RR    | r_dst, r_src | bool -> string. `"true"` or `"false"`.                      |

**General conversion (user types):**

| Mnemonic  | Shape | Operands                      | Description                                                                                                                                                                |
|-----------|-------|-------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `CONVERT` | var   | r_dst, r_src, target_type:u32 | Invoke `Into<T>` for the value in r_src, where T is target_type. Dispatches through the contract system. Encoding: `u16(op) u16(r_dst) u16(r_src) u32(target_type)` = 10B. |

