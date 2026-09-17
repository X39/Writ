# 3.9 Arrays

| Mnemonic           | Shape | Operands                                              | Description                                                                                                                                               |
|--------------------|-------|-------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|
| `NEW_ARRAY`        | RI32  | r_dst, default_kind:u32                               | Create a new empty array (length 0) carrying the given growth-default recipe.                                                                               |
| `ARRAY_INIT`       | var   | r_dst, default_kind:u32, count:u16, r_base            | Create an array pre-filled from consecutive registers r_base..r_base+count-1. `Unavailable` is refined from the first value when possible. Encoding: `u16(op) u16(r_dst) u32(default_kind) u16(count) u16(r_base)` = 12B. |
| `ARRAY_LOAD`       | RRR   | r_dst, r_arr, r_idx                                   | Read element at index. Crash on out-of-bounds.                                                                                                            |
| `ARRAY_STORE`      | RRR   | r_arr, r_idx, r_val                                   | Write element at index. Crash on out-of-bounds.                                                                                                           |
| `ARRAY_LEN`        | RR    | r_dst, r_arr                                          | Get array length as int.                                                                                                                                  |
| `ARRAY_RESIZE`     | RR    | r_arr, r_new_len                                      | Resize array to r_new_len. If growing, new slots filled with type defaults. If shrinking, excess elements dropped. Negative length crashes.                |
| `ARRAY_COPY`       | var   | r_dst_arr, r_dst_idx, r_src_arr, r_src_idx, r_len     | Copy r_len elements from src[src_idx..] to dst[dst_idx..]. Handles same-array overlap (memmove). OOB crashes. Encoding: 5x u16 operands = 12B.            |
| `ARRAY_SLICE`      | var   | r_dst, r_arr, r_start, r_end                          | Create a sub-array copy from index r_start (inclusive) to r_end (exclusive), preserving the source default-kind descriptor even for an empty slice. Encoding: `u16(op) u16(r_dst) u16(r_arr) u16(r_start) u16(r_end)` = 10B. |
| `NEW_ARRAY_SIZED`  | var   | r_dst, default_kind:u32, r_len                        | Create an array of r_len elements using the growth-default recipe. Encoding: `u16(op) u16(r_dst) u32(default_kind) u16(r_len)` = 10B.                      |
| `NEW_ARRAY_FILLED` | var   | r_dst, default_kind:u32, r_len, r_fill                | Create an array of r_len copies of r_fill. `Unavailable` is refined from r_fill when possible. Encoding: `u16(op) u16(r_dst) u32(default_kind) u16(r_len) u16(r_fill)` = 12B. |

## Array growth-default kind

The `default_kind` operand answers one narrow runtime question: if an operation must create a new array element without
being given a value, which value can it synthesize? It is **not** the array's element type, a TypeDef/TypeRef token, or
a substitute for static type checking. The compiler and verifier enforce `T[]` element homogeneity separately.
The binary contains only the `u32` value, not an operand name. Text IL and implementation APIs call it
`default_kind`; there is no alternate or legacy interpretation in which the value is an element-type token.

The operand is a fixed `u32` discriminant:

| Value        | Name            | Value synthesized for a new slot |
|--------------|-----------------|----------------------------------|
| `0`          | Int             | `0`                              |
| `1`          | Float           | `0.0`                            |
| `2`          | Bool            | `false`                          |
| `3`          | String          | `""`                             |
| `4`          | NullReference   | `null`                           |
| `0xFFFFFFFF` | Unavailable     | no value can be synthesized      |

`NullReference` covers every reference-typed element; the descriptor does not identify the referenced class.
`Unavailable` is used when the element type is erased or has no universally valid synthesized value, for example a
value-type struct. `ARRAY_INIT` and `NEW_ARRAY_FILLED` may replace `Unavailable` with a category inferred from an actual
Int, Float, Bool, String, or reference value. An empty initialization has no value to inspect, so it remains
`Unavailable`.

An operation that needs to synthesize a slot—currently a growing `ARRAY_RESIZE` or `NEW_ARRAY_SIZED` with nonzero
length—crashes if the stored kind is `Unavailable`. It must not silently use Int zero, `Void`, or another placeholder.
Any value outside the six rows above is malformed and also crashes. `ARRAY_SLICE` copies the source array's stored
kind, including for an empty slice, so later growth has the same behavior as growth of the source.
