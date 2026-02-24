# Writ IL Specification
## 4.0 Instruction Count by Category

| Category           | Count  | Instruction Mnemonics                                                                                                         |
|--------------------|--------|-------------------------------------------------------------------------------------------------------------------------------|
| Meta               | 2      | NOP, CRASH                                                                                                                    |
| Data Movement      | 7      | MOV, LOAD_INT, LOAD_FLOAT, LOAD_TRUE, LOAD_FALSE, LOAD_STRING, LOAD_NULL                                                      |
| Integer Arithmetic | 6      | ADD_I, SUB_I, MUL_I, DIV_I, MOD_I, NEG_I                                                                                      |
| Float Arithmetic   | 6      | ADD_F, SUB_F, MUL_F, DIV_F, MOD_F, NEG_F                                                                                      |
| Bitwise & Logical  | 5      | BIT_AND, BIT_OR, SHL, SHR, NOT                                                                                                |
| Comparison         | 6      | CMP_EQ_I, CMP_EQ_F, CMP_EQ_B, CMP_EQ_S, CMP_LT_I, CMP_LT_F                                                                    |
| Control Flow       | 6      | BR, BR_TRUE, BR_FALSE, SWITCH, RET, RET_VOID                                                                                  |
| Calls & Delegates  | 6      | CALL, CALL_VIRT, CALL_EXTERN, NEW_DELEGATE, CALL_INDIRECT, TAIL_CALL                                                          |
| Object Model       | 10     | NEW, GET_FIELD, SET_FIELD, SPAWN_ENTITY, INIT_ENTITY, GET_COMPONENT, GET_OR_CREATE, FIND_ALL, DESTROY_ENTITY, ENTITY_IS_ALIVE |
| Arrays             | 9      | NEW_ARRAY, ARRAY_INIT, ARRAY_LOAD, ARRAY_STORE, ARRAY_LEN, ARRAY_ADD, ARRAY_REMOVE, ARRAY_INSERT, ARRAY_SLICE                 |
| Option             | 4      | WRAP_SOME, UNWRAP, IS_SOME, IS_NONE                                                                                           |
| Result             | 6      | WRAP_OK, WRAP_ERR, UNWRAP_OK, IS_OK, IS_ERR, EXTRACT_ERR                                                                      |
| Enum               | 3      | NEW_ENUM, GET_TAG, EXTRACT_FIELD                                                                                              |
| Concurrency        | 7      | SPAWN_TASK, SPAWN_DETACHED, JOIN, CANCEL, DEFER_PUSH, DEFER_POP, DEFER_END                                                    |
| Globals & Atomics  | 4      | LOAD_GLOBAL, STORE_GLOBAL, ATOMIC_BEGIN, ATOMIC_END                                                                           |
| Conversion         | 6      | I2F, F2I, I2S, F2S, B2S, CONVERT                                                                                              |
| Strings            | 3      | STR_CONCAT, STR_BUILD, STR_LEN                                                                                                |
| Boxing             | 2      | BOX, UNBOX                                                                                                                    |
| **Total**          | **91** |                                                                                                                               |

