# 3.9 Arrays

| Mnemonic           | Shape | Operands                                              | Description                                                                                                                                               |
|--------------------|-------|-------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|
| `NEW_ARRAY`        | RI32  | r_dst, elem_type:u32                                  | Create a new empty array (length 0) carrying the given default-kind descriptor.                                                                            |
| `ARRAY_INIT`       | var   | r_dst, elem_type:u32, count:u16, r_base               | Create an array pre-filled from consecutive registers r_base..r_base+count-1. `Unavailable` is inferred from the first value when possible. Encoding: `u16(op) u16(r_dst) u32(elem_type) u16(count) u16(r_base)` = 12B. |
| `ARRAY_LOAD`       | RRR   | r_dst, r_arr, r_idx                                   | Read element at index. Crash on out-of-bounds.                                                                                                            |
| `ARRAY_STORE`      | RRR   | r_arr, r_idx, r_val                                   | Write element at index. Crash on out-of-bounds.                                                                                                           |
| `ARRAY_LEN`        | RR    | r_dst, r_arr                                          | Get array length as int.                                                                                                                                  |
| `ARRAY_RESIZE`     | RR    | r_arr, r_new_len                                      | Resize array to r_new_len. If growing, new slots filled with type defaults. If shrinking, excess elements dropped. Negative length crashes.                |
| `ARRAY_COPY`       | var   | r_dst_arr, r_dst_idx, r_src_arr, r_src_idx, r_len     | Copy r_len elements from src[src_idx..] to dst[dst_idx..]. Handles same-array overlap (memmove). OOB crashes. Encoding: 5x u16 operands = 12B.            |
| `ARRAY_SLICE`      | var   | r_dst, r_arr, r_start, r_end                          | Create a sub-array copy from index r_start (inclusive) to r_end (exclusive), preserving the source default-kind descriptor even for an empty slice. Encoding: `u16(op) u16(r_dst) u16(r_arr) u16(r_start) u16(r_end)` = 10B. |
| `NEW_ARRAY_SIZED`  | var   | r_dst, elem_type:u32, r_len                           | Create array of r_len elements filled with type defaults. Encoding: `u16(op) u16(r_dst) u32(elem_type) u16(r_len)` = 10B.                                 |
| `NEW_ARRAY_FILLED` | var   | r_dst, elem_type:u32, r_len, r_fill                   | Create array of r_len elements filled with value in r_fill. `Unavailable` is inferred from r_fill when possible. Encoding: `u16(op) u16(r_dst) u32(elem_type) u16(r_len) u16(r_fill)` = 12B. |

## Array default-kind descriptor

The historical operand name `elem_type` is retained in the binary format, but
the `u32` value is a runtime default category, not a metadata token:

| Value          | Category        | Default used when growing |
|----------------|-----------------|---------------------------|
| `0`            | Int             | `0`                       |
| `1`            | Float           | `0.0`                     |
| `2`            | Bool            | `false`                   |
| `3`            | String          | `""`                      |
| `4`            | Null reference  | `null`                    |
| `0xFFFFFFFF`   | Unavailable     | none                      |

`Unavailable` represents an erased generic element or an element category for
which the VM cannot construct a valid default (for example, a value-type
struct). `ARRAY_INIT` and `NEW_ARRAY_FILLED` may refine it from an existing Int,
Float, Bool, String, or reference value. Growing an array while its category is
still unavailable crashes; the VM must not silently substitute an Int or void
value. Invalid descriptor values also crash. Array slicing preserves the
descriptor, which lets generic collection code create a typed empty result from
an existing array without inventing a default.
