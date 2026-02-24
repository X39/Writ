# Writ IL Specification
## 3.3 Float Arithmetic

All operands are `float` (64-bit IEEE 754). IEEE 754 semantics apply throughout (inf, NaN propagation, no crash on /0).

| Mnemonic | Shape | Operands        | Description                                 |
|----------|-------|-----------------|---------------------------------------------|
| `ADD_F`  | RRR   | r_dst, r_a, r_b | Addition.                                   |
| `SUB_F`  | RRR   | r_dst, r_a, r_b | Subtraction.                                |
| `MUL_F`  | RRR   | r_dst, r_a, r_b | Multiplication.                             |
| `DIV_F`  | RRR   | r_dst, r_a, r_b | Division. Returns ±inf or NaN per IEEE 754. |
| `MOD_F`  | RRR   | r_dst, r_a, r_b | Modulo (IEEE 754 remainder).                |
| `NEG_F`  | RR    | r_dst, r_src    | Negation.                                   |

