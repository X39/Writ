# Writ IL Specification
## 3.1 Data Movement

| Mnemonic      | Shape | Operands           | Description                                                                                                    |
|---------------|-------|--------------------|----------------------------------------------------------------------------------------------------------------|
| `MOV`         | RR    | r_dst, r_src       | Copy register to register. Semantics depend on type: value copy for primitives, reference copy for heap types. |
| `LOAD_INT`    | RI64  | r_dst, value:i64   | Load 64-bit signed integer literal.                                                                            |
| `LOAD_FLOAT`  | RI64  | r_dst, value:f64   | Load 64-bit IEEE 754 float literal. Same encoding width as LOAD_INT, different interpretation.                 |
| `LOAD_TRUE`   | R     | r_dst              | Load boolean `true`.                                                                                           |
| `LOAD_FALSE`  | R     | r_dst              | Load boolean `false`.                                                                                          |
| `LOAD_STRING` | RI32  | r_dst, str_idx:u32 | Load string reference from the string heap by index.                                                           |
| `LOAD_NULL`   | R     | r_dst              | Load `Option::None` / null.                                                                                    |

