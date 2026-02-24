# Writ IL Specification
## 3.4 Bitwise & Logical

| Mnemonic  | Shape | Operands        | Description                                 |
|-----------|-------|-----------------|---------------------------------------------|
| `BIT_AND` | RRR   | r_dst, r_a, r_b | Bitwise AND on int.                         |
| `BIT_OR`  | RRR   | r_dst, r_a, r_b | Bitwise OR on int.                          |
| `SHL`     | RRR   | r_dst, r_a, r_b | Shift left. r_b is shift count.             |
| `SHR`     | RRR   | r_dst, r_a, r_b | Arithmetic shift right. r_b is shift count. |
| `NOT`     | RR    | r_dst, r_src    | Logical NOT. Operand must be bool.          |

