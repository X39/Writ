# Writ IL Specification
## 3.12 Globals & Atomics

| Mnemonic       | Shape | Operands              | Description                                                                                                               |
|----------------|-------|-----------------------|---------------------------------------------------------------------------------------------------------------------------|
| `LOAD_GLOBAL`  | RI32  | r_dst, global_idx:u32 | Read a global variable. The runtime ensures atomic read semantics.                                                        |
| `STORE_GLOBAL` | var   | global_idx:u32, r_src | Write a global variable. The runtime ensures atomic write semantics. Encoding: `u16(op) u32(global_idx) u16(r_src)` = 8B. |
| `ATOMIC_BEGIN` | N     | —                     | Enter an atomic section. The runtime guarantees no other task reads or writes the involved globals until ATOMIC_END.      |
| `ATOMIC_END`   | N     | —                     | Exit the atomic section.                                                                                                  |

`ATOMIC_BEGIN` / `ATOMIC_END` must be properly nested. An ATOMIC_BEGIN without a matching ATOMIC_END before function
exit is a verification error. The runtime MAY detect this at load time or at runtime.

