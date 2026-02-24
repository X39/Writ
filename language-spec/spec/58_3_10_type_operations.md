# Writ IL Specification
## 3.10 Type Operations

**Option (specialized):**

| Mnemonic    | Shape | Operands     | Description                                       |
|-------------|-------|--------------|---------------------------------------------------|
| `WRAP_SOME` | RR    | r_dst, r_val | Construct `Option::Some(val)`.                    |
| `UNWRAP`    | RR    | r_dst, r_opt | Extract value from `Option::Some`. Crash on None. |
| `IS_SOME`   | RR    | r_dst, r_opt | Test if Option is Some -> bool.                   |
| `IS_NONE`   | RR    | r_dst, r_opt | Test if Option is None -> bool.                   |

**Result (specialized):**

| Mnemonic      | Shape | Operands        | Description                                       |
|---------------|-------|-----------------|---------------------------------------------------|
| `WRAP_OK`     | RR    | r_dst, r_val    | Construct `Result::Ok(val)`.                      |
| `WRAP_ERR`    | RR    | r_dst, r_err    | Construct `Result::Err(err)`.                     |
| `UNWRAP_OK`   | RR    | r_dst, r_result | Extract Ok value. Crash on Err.                   |
| `IS_OK`       | RR    | r_dst, r_result | Test if Result is Ok -> bool.                     |
| `IS_ERR`      | RR    | r_dst, r_result | Test if Result is Err -> bool.                    |
| `EXTRACT_ERR` | RR    | r_dst, r_result | Extract the Err value. Undefined if Result is Ok. |

**General enum operations:**

| Mnemonic        | Shape | Operands                                              | Description                                                                                                                                                                                                                                                                              |
|-----------------|-------|-------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `NEW_ENUM`      | var   | r_dst, type_idx:u32, tag:u16, field_count:u16, r_base | Construct an enum value. tag selects the variant, fields are read from consecutive registers r_base..+field_count. For tag-only variants (no payload), field_count is 0 and r_base is ignored. Encoding: `u16(op) u16(r_dst) u32(type_idx) u16(tag) u16(field_count) u16(r_base)` = 14B. |
| `GET_TAG`       | RR    | r_dst, r_enum                                         | Extract the tag as int.                                                                                                                                                                                                                                                                  |
| `EXTRACT_FIELD` | var   | r_dst, r_enum, field_idx:u16                          | Extract a payload field from the current variant. The caller must have verified the tag first. field_idx is the zero-based index within the variant's payload fields. Encoding: `u16(op) u16(r_dst) u16(r_enum) u16(field_idx)` = 8B.                                                    |

