# Writ IL Specification
## 3.9 Arrays

| Mnemonic       | Shape | Operands                                | Description                                                                                                                                               |
|----------------|-------|-----------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|
| `NEW_ARRAY`    | RI32  | r_dst, elem_type:u32                    | Create a new empty array with the given element type.                                                                                                     |
| `ARRAY_INIT`   | var   | r_dst, elem_type:u32, count:u16, r_base | Create an array pre-filled from consecutive registers r_base..r_base+count-1. Encoding: `u16(op) u16(r_dst) u32(elem_type) u16(count) u16(r_base)` = 12B. |
| `ARRAY_LOAD`   | RRR   | r_dst, r_arr, r_idx                     | Read element at index. Crash on out-of-bounds.                                                                                                            |
| `ARRAY_STORE`  | RRR   | r_arr, r_idx, r_val                     | Write element at index. Crash on out-of-bounds.                                                                                                           |
| `ARRAY_LEN`    | RR    | r_dst, r_arr                            | Get array length as int.                                                                                                                                  |
| `ARRAY_ADD`    | RR    | r_arr, r_val                            | Append element to end.                                                                                                                                    |
| `ARRAY_REMOVE` | RR    | r_arr, r_idx                            | Remove element at index. Crash on out-of-bounds.                                                                                                          |
| `ARRAY_INSERT` | RRR   | r_arr, r_idx, r_val                     | Insert element at index, shifting subsequent elements.                                                                                                    |
| `ARRAY_SLICE`  | var   | r_dst, r_arr, r_start, r_end            | Create a sub-array copy from index r_start (inclusive) to r_end (exclusive). Encoding: `u16(op) u16(r_dst) u16(r_arr) u16(r_start) u16(r_end)` = 10B.     |

