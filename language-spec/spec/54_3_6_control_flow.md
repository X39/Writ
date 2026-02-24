# Writ IL Specification
## 3.6 Control Flow

| Mnemonic   | Shape | Operands                     | Description                                                                                                                                                                              |
|------------|-------|------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `BR`       | I32   | offset:i32                   | Unconditional relative branch. Offset from the start of this instruction.                                                                                                                |
| `BR_TRUE`  | RI32  | r_cond, offset:i32           | Branch if r_cond is `true`.                                                                                                                                                              |
| `BR_FALSE` | RI32  | r_cond, offset:i32           | Branch if r_cond is `false`.                                                                                                                                                             |
| `SWITCH`   | var   | r_tag, n:u16, offsets:i32[n] | Jump table. r_tag indexes into the offset array. If r_tag < 0 or r_tag >= n, falls through to the next instruction. Encoding: `u16(op) u16(r_tag) u16(n) i32[n]`. Total: `6 + 4n` bytes. |
| `RET`      | R     | r_src                        | Return value from current method. Triggers defer handlers.                                                                                                                               |
| `RET_VOID` | N     | —                            | Return void. Triggers defer handlers.                                                                                                                                                    |

`BR` uses a minimal `I32` shape: `u16(op) + padding:u16 + i32(offset)` = 8 bytes. The padding keeps alignment uniform
with RI32.

