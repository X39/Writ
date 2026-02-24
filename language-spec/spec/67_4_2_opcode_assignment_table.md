# Writ IL Specification
## 4.2 Opcode Assignment Table

Opcodes are partitioned by category in the high byte (see §2.5 for the scheme).

### 0x00 — Meta

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0000` | NOP      | N     |
| `0x0001` | CRASH    | R     |

### 0x01 — Data Movement

| Opcode   | Mnemonic    | Shape |
|----------|-------------|-------|
| `0x0100` | MOV         | RR    |
| `0x0101` | LOAD_INT    | RI64  |
| `0x0102` | LOAD_FLOAT  | RI64  |
| `0x0103` | LOAD_TRUE   | R     |
| `0x0104` | LOAD_FALSE  | R     |
| `0x0105` | LOAD_STRING | RI32  |
| `0x0106` | LOAD_NULL   | R     |

### 0x02 — Integer Arithmetic

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0200` | ADD_I    | RRR   |
| `0x0201` | SUB_I    | RRR   |
| `0x0202` | MUL_I    | RRR   |
| `0x0203` | DIV_I    | RRR   |
| `0x0204` | MOD_I    | RRR   |
| `0x0205` | NEG_I    | RR    |

### 0x03 — Float Arithmetic

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0300` | ADD_F    | RRR   |
| `0x0301` | SUB_F    | RRR   |
| `0x0302` | MUL_F    | RRR   |
| `0x0303` | DIV_F    | RRR   |
| `0x0304` | MOD_F    | RRR   |
| `0x0305` | NEG_F    | RR    |

### 0x04 — Bitwise & Logical

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0400` | BIT_AND  | RRR   |
| `0x0401` | BIT_OR   | RRR   |
| `0x0402` | SHL      | RRR   |
| `0x0403` | SHR      | RRR   |
| `0x0404` | NOT      | RR    |

### 0x05 — Comparison

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0500` | CMP_EQ_I | RRR   |
| `0x0501` | CMP_EQ_F | RRR   |
| `0x0502` | CMP_EQ_B | RRR   |
| `0x0503` | CMP_EQ_S | RRR   |
| `0x0504` | CMP_LT_I | RRR   |
| `0x0505` | CMP_LT_F | RRR   |

### 0x06 — Control Flow

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0600` | BR       | I32   |
| `0x0601` | BR_TRUE  | RI32  |
| `0x0602` | BR_FALSE | RI32  |
| `0x0603` | SWITCH   | var   |
| `0x0604` | RET      | R     |
| `0x0605` | RET_VOID | N     |

### 0x07 — Calls & Delegates

| Opcode   | Mnemonic      | Shape |
|----------|---------------|-------|
| `0x0700` | CALL          | CALL  |
| `0x0701` | CALL_VIRT     | var   |
| `0x0702` | CALL_EXTERN   | CALL  |
| `0x0703` | NEW_DELEGATE  | var   |
| `0x0704` | CALL_INDIRECT | var   |
| `0x0705` | TAIL_CALL     | var   |

### 0x08 — Object Model

| Opcode   | Mnemonic        | Shape |
|----------|-----------------|-------|
| `0x0800` | NEW             | RI32  |
| `0x0801` | GET_FIELD       | var   |
| `0x0802` | SET_FIELD       | var   |
| `0x0803` | SPAWN_ENTITY    | RI32  |
| `0x0804` | INIT_ENTITY     | R     |
| `0x0805` | GET_COMPONENT   | var   |
| `0x0806` | GET_OR_CREATE   | RI32  |
| `0x0807` | FIND_ALL        | RI32  |
| `0x0808` | DESTROY_ENTITY  | R     |
| `0x0809` | ENTITY_IS_ALIVE | RR    |

### 0x09 — Arrays

| Opcode   | Mnemonic     | Shape |
|----------|--------------|-------|
| `0x0900` | NEW_ARRAY    | RI32  |
| `0x0901` | ARRAY_INIT   | var   |
| `0x0902` | ARRAY_LOAD   | RRR   |
| `0x0903` | ARRAY_STORE  | RRR   |
| `0x0904` | ARRAY_LEN    | RR    |
| `0x0905` | ARRAY_ADD    | RR    |
| `0x0906` | ARRAY_REMOVE | RR    |
| `0x0907` | ARRAY_INSERT | RRR   |
| `0x0908` | ARRAY_SLICE  | var   |

### 0x0A — Type Operations

**Option (0x0A00–0x0A0F):**

| Opcode   | Mnemonic  | Shape |
|----------|-----------|-------|
| `0x0A00` | WRAP_SOME | RR    |
| `0x0A01` | UNWRAP    | RR    |
| `0x0A02` | IS_SOME   | RR    |
| `0x0A03` | IS_NONE   | RR    |

**Result (0x0A10–0x0A1F):**

| Opcode   | Mnemonic    | Shape |
|----------|-------------|-------|
| `0x0A10` | WRAP_OK     | RR    |
| `0x0A11` | WRAP_ERR    | RR    |
| `0x0A12` | UNWRAP_OK   | RR    |
| `0x0A13` | IS_OK       | RR    |
| `0x0A14` | IS_ERR      | RR    |
| `0x0A15` | EXTRACT_ERR | RR    |

**Enum (0x0A20–0x0A2F):**

| Opcode   | Mnemonic      | Shape |
|----------|---------------|-------|
| `0x0A20` | NEW_ENUM      | var   |
| `0x0A21` | GET_TAG       | RR    |
| `0x0A22` | EXTRACT_FIELD | var   |

### 0x0B — Concurrency

| Opcode   | Mnemonic       | Shape |
|----------|----------------|-------|
| `0x0B00` | SPAWN_TASK     | CALL  |
| `0x0B01` | SPAWN_DETACHED | CALL  |
| `0x0B02` | JOIN           | RR    |
| `0x0B03` | CANCEL         | R     |
| `0x0B04` | DEFER_PUSH     | RI32  |
| `0x0B05` | DEFER_POP      | N     |
| `0x0B06` | DEFER_END      | N     |

### 0x0C — Globals & Atomics

| Opcode   | Mnemonic     | Shape |
|----------|--------------|-------|
| `0x0C00` | LOAD_GLOBAL  | RI32  |
| `0x0C01` | STORE_GLOBAL | var   |
| `0x0C02` | ATOMIC_BEGIN | N     |
| `0x0C03` | ATOMIC_END   | N     |

### 0x0D — Conversion

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0D00` | I2F      | RR    |
| `0x0D01` | F2I      | RR    |
| `0x0D02` | I2S      | RR    |
| `0x0D03` | F2S      | RR    |
| `0x0D04` | B2S      | RR    |
| `0x0D05` | CONVERT  | var   |

### 0x0E — Strings

| Opcode   | Mnemonic   | Shape |
|----------|------------|-------|
| `0x0E00` | STR_CONCAT | RRR   |
| `0x0E01` | STR_BUILD  | var   |
| `0x0E02` | STR_LEN    | RR    |

### 0x0F — Boxing

| Opcode   | Mnemonic | Shape |
|----------|----------|-------|
| `0x0F00` | BOX      | RR    |
| `0x0F01` | UNBOX    | RR    |

# Appendix

Supplementary material including open design questions and a consolidated log of IL design decisions.

---

