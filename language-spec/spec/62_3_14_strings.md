# Writ IL Specification
## 3.14 Strings

| Mnemonic     | Shape | Operands                 | Description                                                                                                                                                                                                 |
|--------------|-------|--------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `STR_CONCAT` | RRR   | r_dst, r_a, r_b          | Concatenate two strings. Returns a new string.                                                                                                                                                              |
| `STR_BUILD`  | var   | r_dst, count:u16, r_base | Concatenate count consecutive string registers r_base..+count into one string. Optimized for formattable string lowering (`$"HP: {hp}/{max}"`). Encoding: `u16(op) u16(r_dst) u16(count) u16(r_base)` = 8B. |
| `STR_LEN`    | RR    | r_dst, r_str             | String length in characters (not bytes).                                                                                                                                                                    |

