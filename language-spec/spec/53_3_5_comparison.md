# Writ IL Specification
## 3.5 Comparison

All comparison instructions produce a `bool` result in r_dst.

Primitive comparisons — no dispatch, direct evaluation:

| Mnemonic   | Shape | Operands        | Description                                        |
|------------|-------|-----------------|----------------------------------------------------|
| `CMP_EQ_I` | RRR   | r_dst, r_a, r_b | Integer equality.                                  |
| `CMP_EQ_F` | RRR   | r_dst, r_a, r_b | Float equality. NaN != NaN per IEEE 754.           |
| `CMP_EQ_B` | RRR   | r_dst, r_a, r_b | Bool equality.                                     |
| `CMP_EQ_S` | RRR   | r_dst, r_a, r_b | String equality (value comparison, not reference). |
| `CMP_LT_I` | RRR   | r_dst, r_a, r_b | Integer less-than.                                 |
| `CMP_LT_F` | RRR   | r_dst, r_a, r_b | Float less-than.                                   |

User-type comparisons go through `CALL_VIRT` on `Eq` / `Ord` contracts.

Derived operators (compiler desugars, not in the IL):

- `a != b` → `CMP_EQ` + `NOT`
- `a > b`  → `CMP_LT` with swapped operands (`CMP_LT r, r_b, r_a`)
- `a <= b` → `CMP_LT(a,b) || CMP_EQ(a,b)` or `NOT(CMP_LT(b,a))`
- `a >= b` → `NOT(CMP_LT(a,b))`

