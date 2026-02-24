# Writ IL Specification
## 3.2 Integer Arithmetic

All operands are `int` (64-bit signed). Using these instructions on non-int registers is undefined behavior in the IL (
the compiler must not emit this).

| Mnemonic | Shape | Operands        | Description                          |
|----------|-------|-----------------|--------------------------------------|
| `ADD_I`  | RRR   | r_dst, r_a, r_b | Addition.                            |
| `SUB_I`  | RRR   | r_dst, r_a, r_b | Subtraction.                         |
| `MUL_I`  | RRR   | r_dst, r_a, r_b | Multiplication.                      |
| `DIV_I`  | RRR   | r_dst, r_a, r_b | Division. Crash on division by zero. |
| `MOD_I`  | RRR   | r_dst, r_a, r_b | Modulo. Crash on division by zero.   |
| `NEG_I`  | RR    | r_dst, r_src    | Negation (`-x`).                     |

