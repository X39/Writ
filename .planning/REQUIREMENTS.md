# Requirements: Writ Compiler v3.1

**Defined:** 2026-03-03
**Core Value:** Correct, spec-compliant implementation at every layer — lowering matches Section 28 exactly, runtime matches the IL spec exactly

## v3.1 Requirements

Requirements for compiler bug fixes and golden file test coverage. Each golden file test compiles a .writ source, disassembles the IL, hand-validates correctness against the spec, fixes any bugs discovered, and locks the validated IL as a regression golden file.

### Compiler Bugs

- [x] **BUG-01**: Compiler does not stack overflow when compiling valid Writ source files (unbounded recursion eliminated)
- [x] **BUG-02**: Register type blobs are correctly interned into the blob heap (registers carry their actual type, not all `int`)
- [x] **BUG-03**: CALL instructions emit the correct method metadata token for the callee (not token=0)
- [x] **BUG-04**: RET instructions reference the register containing the computed return value (not an uninitialized register)
- [x] **BUG-05**: Extern function calls (`::log`, `::print`, etc.) emit correct CALL_EXTERN instructions in the IL
- [x] **BUG-06**: Call argument setup does not emit phantom MOV from uninitialized registers
- [x] **BUG-07**: Direct function calls emit `CALL r_dst, method_idx, r_base, argc` (not `CALL_INDIRECT`); `CALL_INDIRECT` is reserved for delegate/closure dispatch
- [x] **BUG-08**: Method signature blobs encode parameter types in declaration order and return type last — no type swapping in disassembly output
- [x] **BUG-09**: `RET` in every non-void code path references the actual computed result register, not a newly-allocated void register
- [x] **BUG-10**: `MOV` in if/else branch merging uses the actual branch-value register as source, not a void register
- [x] **BUG-11**: Branch instructions (`BR`, `BR_FALSE`, `BR_TRUE`) emit correct relative byte offsets — `apply_fixups()` is called after all instructions are emitted for a method body
- [x] **BUG-12**: Function parameters occupy registers r0..r(param_count-1) at method entry — `emit_all_bodies` pre-allocates parameter registers and populates `emitter.locals` so every variable reference to a parameter returns the same register (eliminates uninitialized registers in else-branches and across any multi-reference scopes)
- [x] **BUG-13**: `DebugLocal.name` contains the correct string heap offset for the variable's source name — variable names from `let` bindings and function parameters are interned into the string heap and recorded in debug info
- [x] **BUG-14**: `SourceSpan` entries are emitted for each instruction — the body emitter records `(instruction_index, span)` pairs and the serializer converts them to byte-offset-keyed SourceSpan entries with 1-based line/column coordinates
- [x] **BUG-15**: The parser accepts top-level function declarations without an explicit visibility modifier (`fn main() {}`) — bare `fn` is valid syntax; `pub fn` is public, bare `fn` is private
- [x] **BUG-16**: Empty void function bodies emit zero registers — no spurious `.reg r0 void` declaration for functions like `fn main() {}` that produce no values
- [x] **BUG-17**: `CALL` instruction method token (`0x07xxxxxx`) is resolved to the correct in-module method index at runtime — the VM does not crash with "call to invalid method index" when executing any compiled .writil file that contains function calls

### Tech Debt

- [ ] **DEBT-01**: Closure capture list is populated with actual captured variables (TYPE-12 fix — closures with outer variables execute correctly)
- [ ] **DEBT-02**: GC finalization hooks invoke on_finalize method via scheduler (runtime.rs finalization queue)
- [ ] **DEBT-03**: Generic constraint DefId resolved to token during finalize (module_builder.rs:328)
- [ ] **DEBT-04**: Dead code removed (extract_callee_def_id_opt and other unreachable code)

### Metadata Completeness

- [x] **META-01**: MethodDef table rows include param_count (u16) encoding the number of parameter registers r0..r(param_count-1); ParamDef rows are emitted with correct name and type_sig for all function and method parameters; disassembler displays parameter names alongside types in method signatures

### Golden File Tests — Core Language

- [x] **GOLD-01**: Golden file test harness exists (compile .writ → disassemble → compare against .expected IL text file; failures show diff)
- [ ] **GOLD-02**: Functions golden test — function declarations, parameters, return types, local variables, function calls between methods
- [ ] **GOLD-03**: Structs golden test — struct definition, `new` construction, field access, method calls, GET_FIELD/SET_FIELD instructions
- [ ] **GOLD-04**: Enums golden test — enum definition with variants (unit + payload), pattern matching (GET_TAG/SWITCH), exhaustiveness
- [ ] **GOLD-05**: Entities golden test — entity definition, SPAWN_ENTITY/INIT_ENTITY/DESTROY_ENTITY, component slots, lifecycle hooks
- [ ] **GOLD-06**: Dialogue golden test — dlg blocks, say/say_localized, choice branches, speaker resolution, string interpolation
- [ ] **GOLD-07**: Contracts golden test — contract definition, impl blocks, CALL_VIRT dispatch, operator overloading

### Golden File Tests — Advanced Features

- [ ] **GOLD-08**: Generics golden test — generic functions, generic structs, type parameters, constraint bounds, BOX/UNBOX at generic call sites
- [ ] **GOLD-09**: Closures golden test — lambda expressions, capture struct synthesis, NEW_DELEGATE, CALL_INDIRECT, function value passing
- [ ] **GOLD-10**: Error handling golden test — Result type, ? operator (IS_ERR + early return), try blocks, WRAP_OK/WRAP_ERR
- [ ] **GOLD-11**: Concurrency golden test — spawn (SPAWN_TASK), join (JOIN), cancel (CANCEL), defer (DEFER_PUSH/DEFER_POP), detached spawn
- [ ] **GOLD-12**: Globals and atomics golden test — global constants, global mut, atomic sections (ATOMIC_BEGIN/ATOMIC_END)
- [ ] **GOLD-13**: Arrays and strings golden test — array creation (NEW_ARRAY), array ops (ARRAY_LOAD/STORE/LEN), formatted strings (STR_CONCAT/STR_BUILD), type conversions (I2S/F2S)
- [ ] **GOLD-14**: Option type golden test — Option<T> construction (WRAP_SOME/LOAD_NULL), IS_NONE/IS_SOME, unwrap (! operator), ? propagation
- [ ] **GOLD-15**: Control flow golden test — if/else, while loops, for loops (array iteration), match expressions, break/continue/return

## Future Requirements

### Assembler Parity
- **ASM-01**: Assembler supports .export/.extern_fn/.component/.locale/.attribute directives
- **ASM-02**: Register type blob offsets correctly interned in assembler round-trip

### Additional Validation
- **VAL-01**: Compiled .writil modules load and execute correctly on the VM end-to-end
- **VAL-02**: Compiler warns on transition points inside atomic blocks (A8 spec item)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Standard library (writ-std) | Separate milestone; no runtime dependency |
| JIT compilation | Correctness must be established first |
| Optimization passes | Premature before correctness is proven |
| New language features | Bug fixing only; no spec additions |
| LSP / IDE support | Separate milestone |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| BUG-01 | Phase 30 | Complete |
| BUG-02 | Phase 30 | Complete |
| BUG-03 | Phase 30 | Complete |
| BUG-04 | Phase 30 | Complete |
| BUG-05 | Phase 30 | Complete |
| BUG-06 | Phase 30 | Complete |
| BUG-07 | Phase 31.1 | Complete |
| BUG-08 | Phase 31.1 | Complete |
| BUG-09 | Phase 31.1 | Complete |
| BUG-10 | Phase 31.1 | Complete |
| BUG-11 | Phase 31.1 | Complete |
| BUG-12 | Phase 31.2 | Complete |
| BUG-13 | Phase 31.2 | Complete |
| BUG-14 | Phase 31.2 | Complete |
| BUG-15 | Phase 31.2 | Complete |
| GOLD-01 | Phase 31 | Complete |
| GOLD-02 | Phase 31 | Pending |
| GOLD-03 | Phase 32 | Pending |
| GOLD-04 | Phase 32 | Pending |
| GOLD-15 | Phase 32 | Pending |
| GOLD-05 | Phase 33 | Pending |
| GOLD-06 | Phase 33 | Pending |
| GOLD-07 | Phase 33 | Pending |
| GOLD-08 | Phase 34 | Pending |
| GOLD-10 | Phase 34 | Pending |
| GOLD-14 | Phase 34 | Pending |
| GOLD-09 | Phase 35 | Pending |
| GOLD-11 | Phase 35 | Pending |
| GOLD-12 | Phase 35 | Pending |
| GOLD-13 | Phase 36 | Pending |
| DEBT-01 | Phase 36 | Pending |
| DEBT-02 | Phase 36 | Pending |
| DEBT-03 | Phase 36 | Pending |
| DEBT-04 | Phase 36 | Pending |
| META-01 | Phase 39 | Complete |

**Coverage:**
- v3.1 requirements: 31 total
- Mapped to phases: 31
- Unmapped: 0

---
*Requirements defined: 2026-03-03*
*Last updated: 2026-03-03 — traceability mapped during roadmap creation*
