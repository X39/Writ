# Writ IL Specification
## 4.1 Instruction Shape Reference

| Shape  | Layout           | Size   | Byte Breakdown                                      |
|--------|------------------|--------|-----------------------------------------------------|
| `N`    | `op`             | 2B     | `u16(op)`                                           |
| `R`    | `op r`           | 4B     | `u16(op) u16(r)`                                    |
| `RR`   | `op r r`         | 6B     | `u16(op) u16(r) u16(r)`                             |
| `RRR`  | `op r r r`       | 8B     | `u16(op) u16(r) u16(r) u16(r)`                      |
| `RI32` | `op r i32`       | 8B     | `u16(op) u16(r) u32(imm)`                           |
| `RI64` | `op r i64`       | 12B    | `u16(op) u16(r) u64(imm)`                           |
| `I32`  | `op pad i32`     | 8B     | `u16(op) u16(pad) i32(imm)` — used by BR            |
| `CALL` | `op r i32 r u16` | 12B    | `u16(op) u16(r_dst) u32(idx) u16(r_base) u16(argc)` |
| `var`  | per-instruction  | varies | Documented per instruction in §2                    |

