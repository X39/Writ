# Writ IL Specification
## 2.5 Instruction Encoding

**Decision:**

- Opcodes: **u16** (65536 slots — future-proof).
- Register operands: **u16** (up to 65535 registers per function).
- Table indices: **u32** metadata tokens (§2.16.4). Heap references are raw u32 offsets.
- Byte order: **little-endian** throughout.
- Instructions are **variable-width**, with the opcode determining the operand layout.

**Instruction shapes:**

| Shape  | Layout                    | Size | Used By                                        |
|--------|---------------------------|------|------------------------------------------------|
| `N`    | `op`                      | 2B   | `NOP`, `RET_VOID`, `ATOMIC_BEGIN`, etc.        |
| `R`    | `op r`                    | 4B   | `LOAD_TRUE r`, `CRASH r`, etc.                 |
| `RR`   | `op r r`                  | 6B   | `MOV r r`, `NEG_I r r`, etc.                   |
| `RRR`  | `op r r r`                | 8B   | `ADD_I r r r`, `CMP_EQ_I r r r`, etc.          |
| `RI32` | `op r i32`                | 8B   | `LOAD_STRING r idx`, `LOAD_GLOBAL r idx`, etc. |
| `RI64` | `op r i64`                | 12B  | `LOAD_INT r val`, `LOAD_FLOAT r val`           |
| `CALL` | `op r_dst i32 r_base u16` | 12B  | `CALL`, `CALL_EXTERN`, `SPAWN_TASK`, etc.      |

Instructions that don't fit these shapes use documented per-instruction layouts (e.g., `SWITCH`, `CALL_VIRT`,
`CALL_INDIRECT`).

**Opcode numbering scheme:**

The u16 opcode space is partitioned by category in the high byte. The low byte identifies the instruction within
its category. Each category has 256 slots, providing room for future expansion without renumbering existing
instructions.

| High Byte | Category                                 |
|-----------|------------------------------------------|
| `0x00`    | Meta                                     |
| `0x01`    | Data Movement                            |
| `0x02`    | Integer Arithmetic                       |
| `0x03`    | Float Arithmetic                         |
| `0x04`    | Bitwise & Logical                        |
| `0x05`    | Comparison                               |
| `0x06`    | Control Flow                             |
| `0x07`    | Calls & Delegates                        |
| `0x08`    | Object Model                             |
| `0x09`    | Arrays                                   |
| `0x0A`    | Type Operations (Option / Result / Enum) |
| `0x0B`    | Concurrency                              |
| `0x0C`    | Globals & Atomics                        |
| `0x0D`    | Conversion                               |
| `0x0E`    | Strings                                  |
| `0x0F`    | Boxing                                   |

Within `0x0A`, sub-ranges separate the three groups: Option at `0x0A00`, Result at `0x0A10`, Enum at `0x0A20`.

The full opcode assignment table is in the summary (03-summary.md §Opcode Assignment Table).

