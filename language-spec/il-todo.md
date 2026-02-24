# Writ IL Specification — TODO

Tracking all design decisions, spec updates, and IL specification work needed.

---

## A. Language Spec Updates (prerequisites — changes to the existing spec)

### A1. `self` Parameter Semantics — RESOLVED (see spec §12.5, il-spec §1.10)
- [x] Specify that `self` is an explicit parameter in the calling convention (not magic) — §12.5
- [x] Define `self` vs `mut self` — mutability of the receiver — §12.5.1
- [x] Define static functions as the absence of `self` (no separate keyword) — §12.5.2
- [x] `self.entity` in components → hidden `__entity` field on component TypeDef, set during SPAWN_ENTITY — §14.6.4, §15.2
- [x] Update spec sections: 8 (Structs), 14 (Entities), 15 (Components), 12 (Functions) — done

### A2. Struct/Entity Construction Semantics — RESOLVED (see il-spec §1.11, spec §8, §14.2, §14.7)
- [x] `new` keyword for construction: `new Type { field: value }` — disambiguates from blocks — §8.1, §14.2
- [x] Construction order: allocate (zeroed) → SET_FIELD for all fields → on_create → (INIT_ENTITY for entities) — §1.11
- [x] Default field values are runtime expressions, inlined at construction site by compiler — §1.11
- [x] `spawn` keyword reserved for task concurrency only — §14.2
- [x] No user-defined constructors; construction is compiler-generated. Factory methods via static fns — §1.11
- [x] Lifecycle hooks on structs: `on create`, `on finalize`, `on serialize`, `on deserialize` — §8.2
- [x] Components are extern-only, data-only (no script-defined components) — §15

### A3. Closure Specification — RESOLVED (see il-spec §1.9.3, §1.12)
- [x] Specify that closures lower to compiler-generated types (not opaque runtime magic) — §1.9.3 + §1.12
- [x] Define the capture struct: compiler creates a hidden struct with captured variables as fields — §1.9.3
- [x] Specify capture modes: `let` → copy into struct, `let mut` → shared capture struct — §1.9.3
- [x] Define the callable interface: resolved as C# delegate model, no hidden Call contract — §1.12.3 (CALL_INDIRECT)
- [x] Closure lowering emission — compiler implementation (F1), not a spec concern
- [x] Lifetime semantics: capture struct is GC-managed, lives as long as anything references it — §1.9.3

### A4. Serialization Hooks — RESOLVED (see spec §8.2, §14.6)
- [x] Lifecycle hooks for serialization: `on serialize` / `on deserialize` on structs and entities — spec §8.2, §14.6
- [x] `on finalize` hook for GC cleanup (non-deterministic) — spec §8.2, §14.6
- [x] Resurrection during `on finalize` is undefined behavior
- [x] CRITICAL_BEGIN/CRITICAL_END removed — the suspend-and-confirm model (A7) makes explicit critical sections unnecessary; the runtime only serializes at well-defined transition points

### A5. Memory Model — RESOLVED (see il-spec-draft.md §1.9)
- [x] Define value types vs reference types at the language level
  - [x] Primitives (`int`, `float`, `bool`) — value types, register-resident
  - [x] `string` — immutable reference type (interned literals, GC-managed runtime strings)
  - [x] Structs — **reference types** (heap-allocated, GC-managed)
  - [x] Entities — reference (handle to runtime-managed object)
  - [x] Arrays — reference type (heap-allocated, GC-managed)
  - [x] Enums — **value types** (tag + inline payload, copied on assignment)
- [x] Define ownership: **GC** (generational assumed, algorithm not mandated)
- [x] Define what "captured by reference" means for `let mut` in closures: **MutCell<T>** (shared heap-allocated mutable cell)
- [x] Specify assignment semantics: value types copy bits, reference types copy reference (shared object)
- [x] `let`/`let mut` controls **binding mutability**, not object mutability

### A6. Save/Load Serialization — RESOLVED (see il-spec §1.13)
- [x] Define what the save file contains (call stacks, heap, globals, entity registry, task tree) — §1.13.1
- [x] Define module identity scheme (version identifier, format runtime-defined) — §1.13.2
- [x] IL in save: spec recommends embedding full IL, runtime loads old+new IL side-by-side on mismatch — §1.13.2
- [x] Version mismatch: runtime must report conflict, recommended strategy is IL coexistence with natural drain — §1.13.2
- [x] Save binary format is runtime-defined, not part of the spec
- [x] Save timing is host-decided, not a runtime concern; runtime must not serialize during in-flight extern calls — §1.13.3
- [x] Extern/host state excluded — host responsible for its own save/load — §1.13.1

### A7. Runtime-Host Interface — RESOLVED (see il-spec §1.14)
- [x] Components are always extern and data-only (no script-defined components) — spec §15
- [x] Runtime suspends on host operations until host confirms (suspend-and-confirm model) — il-spec §1.14.2
- [x] Runtime -> Host request contract (entity spawned/destroyed/unreferenced, component field access, extern calls, dialogue events) — §1.14.2
- [x] Host -> Runtime command contract (tick, fire event, start dialogue, save/load, hot reload) — §1.14.3
- [x] Entity ownership split: runtime owns script state, host owns native state — §1.14.4
- [x] Component proxying: GET_FIELD/SET_FIELD on component fields go through host API with suspension — §1.14.4
- [x] Singleton binding: host may pre-register native entities that bind to [Singleton] types — §1.14.5
- [x] Failure mode: missing component on entity spawn = unrecoverable crash — §1.14.6
- [x] Entity unreference notification: runtime tells host when GC has no more script refs, host decides native fate — §1.14.2
- [x] Runtime logging interface with log levels for error reporting to host — §1.14.7
- [x] Lifecycle hook failure semantics: crash unwinds task, runtime logs to host — spec §8.2, §14.6
- [x] Entity cleanup ordering: on destroy → on finalize → runtime cleanup — spec §14.6
- [x] Dialogue suspension: say/choice/wait are transition points, suspend until host responds — spec §13.9
- [x] Component back-reference: compiler-emitted hidden `@entity` field, not user-accessible — spec §15, il-spec summary
- [x] Implementation guidance for protocol, errors, hot reload, validation — §1.14.8

### A8. Atomic Warning Suppression Mechanism
- [ ] Define a language-level mechanism to suppress the compiler warning for transition points inside `atomic` blocks
- [ ] The compiler MUST warn on transition points inside atomic (§1.17.6); this mechanism allows the author to acknowledge the risk
- [ ] Possible approaches: attribute annotation, `unsafe` block, pragma — to be decided

---

## B. IL Module Format — RESOLVED (see il-spec §1.16)

### B1. Binary Container — RESOLVED (§1.16.1, §1.16.2, §1.16.3)
- [x] Magic bytes (`WRIT` / 0x57524954) and format version (u16, starts at 1) — §1.16.1
- [x] Header layout: 200-byte fixed header (module name/version, heap locations, 21-table directory) — §1.16.1
- [x] Heap formats — §1.16.1:
  - [x] String Heap (length-prefixed UTF-8, u32 length + bytes)
  - [x] Blob Heap (length-prefixed byte sequences, same encoding)
- [x] Alignment: little-endian throughout, table rows aligned to 4-byte boundaries — §1.16.1
- [x] Multi-module architecture with name-based cross-module resolution (DAG, no circular deps) — §1.16.2
- [x] Module versioning: Semantic Versioning 2.0.0 (MAJOR.MINOR.PATCH), min_version on ModuleRef — §1.16.3
- [x] ~~Well-known type table~~ — removed; core types live in `writ-runtime` module (§1.16.8), referenced via standard TypeRef

### B2. Metadata Tables — RESOLVED (§1.16.4, §1.16.5)
- [x] Metadata tokens: u32, top 8 bits = table ID, bottom 24 bits = row index (1-based) — §1.16.4
- [x] 21 metadata tables with fixed-size rows and list ownership pattern — §1.16.5:
  - [x] `ModuleDef` — module identity (1 row)
  - [x] `ModuleRef` — dependencies on other modules (name + min_version)
  - [x] `TypeDef` — types defined in this module (struct/enum/entity/component via kind flag)
  - [x] `TypeRef` — types in other modules (resolved by name at load time)
  - [x] `TypeSpec` — instantiated generic types (TypeDef + type arguments)
  - [x] `FieldDef` — fields on types defined here
  - [x] `FieldRef` — fields in other modules (ABI-safe, name-based resolution)
  - [x] `MethodDef` — methods/functions defined here (includes intrinsic flag for writ-runtime)
  - [x] `MethodRef` — methods in other modules (resolved by name + signature)
  - [x] `ParamDef` — method parameters
  - [x] `ContractDef` — contract declarations
  - [x] `ContractMethod` — method slots within a contract
  - [x] `ImplDef` — (type, contract) → method table mapping
  - [x] `GenericParam` — type parameters on types and methods
  - [x] `GenericConstraint` — bounds (contract requirements) on generic params
  - [x] `GlobalDef` — constants and `global mut` variables
  - [x] `ExternDef` — extern function/type declarations
  - [x] `ComponentSlot` — entity → component bindings
  - [x] `LocaleDef` — dialogue locale dispatch
  - [x] `ExportDef` — convenience index of pub-visible items
  - [x] `AttributeDef` — metadata attributes ([Singleton], [Deprecated], etc.)

### B3. Method Body Layout — RESOLVED (§1.16.6)
- [x] Register count (u16, from MethodDef)
- [x] Register type table — u32 blob heap offsets per register, pointing to TypeRef encodings — §1.16.6
- [x] Code bytes (instruction stream)
- [x] No defer table needed — runtime manages defer stack via DEFER_PUSH/DEFER_POP
- [x] No exception table — Writ has no try/catch
- [x] Optional debug info: DebugLocal (register-to-name) and SourceSpan (PC-to-source) — conditional on module debug flag

---

## C. IL Type System — RESOLVED (see il-spec §1.15)

### C1. Primitive Type Tags — RESOLVED (§1.15.2)
- [x] Assign fixed tags: void=0x00, int=0x01, float=0x02, bool=0x03, string=0x04
- [x] Primitives stored directly in registers (value types); string is a GC reference

### C2. Composite Type References — RESOLVED (§1.15.3)
- [x] Single TypeDef table for structs, enums, entities, components (kind flag on TypeDef)
- [x] Array type encoding: kind 0x20 + recursive element TypeRef
- [x] Option/Result: regular generic enums in type system, specialness only at instruction level
- [x] Function types: kind 0x30 + blob offset to signature (param_count, param_types, return_type)
- [x] Closure types: compiler-generated TypeDef (per §1.12), referenced as kind 0x10

### C3. Generic Representation — RESOLVED (§1.15.4)
- [x] Open generic types: GenericParam rows on TypeDef/MethodDef with zero-based ordinals
- [x] Generic constraints: GenericConstraint rows binding GenericParam to contract TypeDefs
- [x] Closed/instantiated types: TypeSpec entries (TypeDef + concrete TypeRef arguments)
- [x] Runtime dispatch: contract dispatch table (concrete_type_tag, contract_id, method_slot) → entry point
- [x] Value types through generics: boxing (CLR model) — heap-allocate with type tag
- [x] Spec mandates correct behavior, does not mandate dispatch strategy (no required JIT/monomorphization)

### C4. Enum Representation — RESOLVED (§1.15.5)
- [x] Tag: u16 discriminant (up to 65535 variants)
- [x] Tag-only variants: just tag, no payload space
- [x] Payload variants: tag + inline fields per TypeDef
- [x] Size: sizeof(tag) + sizeof(largest variant payload), all variants padded to same size
- [x] Enum registers hold complete value as single abstract unit (runtime determines physical storage)
- [x] Option<T> null-pointer optimization: permitted, not mandated

---

## D. Instruction Set — RESOLVED (see il-spec §1.5, §2.x, summary)

### D1. Encoding Format — RESOLVED (see il-spec §1.5, §2.x, summary §Opcode Assignment Table)
- [x] Opcode width: u16 — §1.5
- [x] Register operands: u16 — §1.5
- [x] Table indices: u32 — §1.5
- [x] Instruction format: variable-width, opcode determines operand layout — §1.5
- [x] Canonical instruction shapes: N, R, RR, RRR, RI32, RI64, I32, CALL, var — §1.5
- [x] Per-instruction `var` layouts documented inline — §2.x
- [x] Register sizing: abstract typed slots (runtime determines physical storage from register type table) — §1.5, §1.16.6
- [x] Opcode numbering scheme: high byte = category, low byte = instruction — §1.5, summary
- [x] Full opcode assignment table (90 instructions, 16 categories) — summary

### D2. Data Movement — RESOLVED (see il-spec §2.1)
- [x] MOV, LOAD_INT, LOAD_FLOAT, LOAD_TRUE, LOAD_FALSE, LOAD_STRING, LOAD_NULL — §2.1
- [x] LOAD_CONST folded into LOAD_GLOBAL — GlobalDef covers both constants and `global mut`; runtime optimizes via mutability flag

### D3. Arithmetic — RESOLVED (see il-spec §2.2, §2.3, §2.4)
- [x] Integer: ADD_I, SUB_I, MUL_I, DIV_I, MOD_I, NEG_I — §2.2
- [x] Float: ADD_F, SUB_F, MUL_F, DIV_F, MOD_F, NEG_F — §2.3
- [x] Bitwise & logical: BIT_AND, BIT_OR, SHL, SHR, NOT — §2.4
- [x] String concatenation: dedicated STR_CONCAT + STR_BUILD — §2.14

### D4. Comparison — RESOLVED (see il-spec §2.5)
- [x] Primitive typed: CMP_EQ_I, CMP_EQ_F, CMP_EQ_B, CMP_EQ_S, CMP_LT_I, CMP_LT_F — §2.5
- [x] Derived ops (!=, >, <=, >=) are compiler desugaring, not IL instructions — §2.5
- [x] User-type comparison: CALL_VIRT through Eq/Ord contracts — §2.5

### D5. Control Flow — RESOLVED (see il-spec §2.6)
- [x] BR, BR_TRUE, BR_FALSE, SWITCH, RET, RET_VOID — §2.6

### D6. Calls — RESOLVED (see il-spec §2.7, §1.6)
- [x] CALL, CALL_VIRT, CALL_EXTERN, NEW_DELEGATE, CALL_INDIRECT, TAIL_CALL — §2.7
- [x] Calling convention: consecutive registers, return in r_dst — §1.6
- [x] No variadics in the language; argc on call instructions handles argument count — §2.7

### D7. Object Model — RESOLVED (see il-spec §2.8)
- [x] NEW, GET_FIELD, SET_FIELD, SPAWN_ENTITY, INIT_ENTITY, GET_COMPONENT, GET_OR_CREATE, FIND_ALL, DESTROY_ENTITY, ENTITY_IS_ALIVE — §2.8

### D8. Arrays — RESOLVED (see il-spec §2.9)
- [x] NEW_ARRAY, ARRAY_INIT, ARRAY_LOAD, ARRAY_STORE, ARRAY_LEN, ARRAY_ADD, ARRAY_REMOVE, ARRAY_INSERT, ARRAY_SLICE — §2.9
- [x] ARRAY_CONTAINS dropped — compiler lowers to loop with CALL_VIRT on Eq contract

### D9. Type Operations — RESOLVED (see il-spec §2.10)
- [x] Option: WRAP_SOME, UNWRAP, IS_SOME, IS_NONE — §2.10
- [x] Result: WRAP_OK, WRAP_ERR, UNWRAP_OK, IS_OK, IS_ERR, EXTRACT_ERR — §2.10
- [x] TYPE_CHECK dropped — Writ has no `is` operator, no inheritance, no `any` type; all cases covered by GET_TAG, IS_SOME, CALL_VIRT

### D10. Enum / Pattern Matching — RESOLVED (see il-spec §2.10)
- [x] NEW_ENUM, GET_TAG, EXTRACT_FIELD — §2.10

### D11. Concurrency — RESOLVED (see il-spec §2.11)
- [x] SPAWN_TASK, SPAWN_DETACHED, JOIN, CANCEL, DEFER_PUSH, DEFER_POP, DEFER_END — §2.11

### D12. Globals & Atomics — RESOLVED (see il-spec §2.12)
- [x] LOAD_GLOBAL, STORE_GLOBAL, ATOMIC_BEGIN, ATOMIC_END — §2.12

### D13. Serialization Control — REMOVED
- [x] `CRITICAL_BEGIN` / `CRITICAL_END` removed — unnecessary with suspend-and-confirm model

### D14. Boxing — RESOLVED (see il-spec §2.15)
- [x] BOX, UNBOX — RR shape, runtime reads register type table for type info — §2.15
- [x] Required for value types through generic parameters (CLR boxing model from C3)

---

## E. Execution Model — RESOLVED

### E1. Runtime Architecture — RESOLVED (see il-spec §1.17)
- [x] Define the managed call stack: stack of frames, each frame = (method, pc, registers[], defer_stack) — §1.17.1
- [x] Define the transition point model: exhaustive catalogue of suspending instructions — §1.17.3
- [x] Define task model: states (Ready/Running/Suspended/Completed/Cancelled), transitions, entry points — §1.17.2, §1.17.4
- [x] Define task tree: scoped tasks auto-cancel on parent exit, detached tasks independent — §1.17.8
- [x] Define scheduling model: cooperative run-to-suspension, execution limits recommended — §1.17.5
- [x] Define atomic section guarantees: no interleaving, no budget suspension, implementation guidance — §1.17.6
- [x] Define crash propagation: full task stack unwinding with defer handlers at each frame — §1.17.7

### E2. Serialization Model — see A6 (il-spec §1.13) for spec-level decisions
- [x] Define what "serialize the VM" means: all task stacks + heap + globals — §1.13.1
- [x] Serialization format is runtime-defined, not spec — §1.13
- [x] Save timing is host-decided; runtime must not serialize during in-flight extern calls — §1.13.3
- [x] Extern/native state excluded; host responsible — §1.13.1
- [x] Versioning: runtime reports conflicts, recommended IL coexistence strategy — §1.13.2

### E3. Contract Dispatch — RESOLVED (see il-spec §1.15.4)
- [x] Dispatch table structure: `(type_tag, contract_id, method_slot)` → entry point — §1.15.4
- [x] Runtime populates from ImplDef rows at module load time — §1.15.4
- [x] Performance model: flat table recommended, inline caching permitted — §1.15.4

### E4. Entity Runtime — RESOLVED (see spec §14.5, §1.9.5)
- [x] Entity storage model: generation-indexed registry with opaque handles — §1.9.5
- [x] Component access: type-tag lookup against ComponentSlot list, returns ref or None — §1.9.5
- [x] Singleton registry: per-type registry keyed by TypeDef token, getOrCreate checks first — §1.9.5
- [x] Define entity lifecycle: spawn → on_create → alive → on_destroy → dead — spec §14.5.1, §14.6, il-spec §1.9.5
- [x] Define entity handle model: opaque registry-based handles, dead access crashes, `Entity.isAlive()` to check — spec §14.5.1, §14.5.2
- [x] Define `Entity.destroy()` / `Entity.isAlive()` as static Entity namespace methods — spec §14.5.2, il-spec §2.8
- [x] Define `self.owner` in components → entity back-reference — spec §15, il-spec summary (`@entity` hidden field)

### E5. Garbage Collection / Memory Management — RESOLVED (see il-spec §1.9.7)
- [x] GC algorithm: spec mandates GC, does not prescribe algorithm; recommends generational/incremental, precise scanning — §1.9.7
- [x] GC roots: registers, globals, entity registry, task handles — §1.9.6
- [x] Define finalization: `on finalize` hook on structs and entities (non-deterministic, UB on resurrection) — spec §8.2, §14.6
- [x] Finalization ordering: implementation-defined; finalizers must not assume other objects are valid — §1.9.7
- [x] Finalization execution: queued during GC, executed as tasks in subsequent scheduling pass — §1.9.7

---

## G. `writ-runtime` Module — RESOLVED (G1, G2; see il-spec §1.18)

### G1. Core Type Definitions — RESOLVED (§1.18.1–§1.18.5)
- [x] `Option<T>` enum layout: None=0, Some(T)=1 — §1.18.1
- [x] `Result<T, E>` enum layout: Ok(T)=0, Err(E)=1 — §1.18.1
- [x] `Range<T>` struct: start, end, start_inclusive, end_inclusive — §1.18.2
- [x] Core contracts: all 17 (Add, Sub, Mul, Div, Mod, Neg, Not, Eq, Ord, Index, IndexSet, BitAnd, BitOr, Iterable, Iterator, Into, Error) — §1.18.3
- [x] Primitive pseudo-TypeDefs (Int, Float, Bool, String) for contract impl anchoring — §1.18.4
- [x] Primitive contract implementations via intrinsic methods — §1.18.5

### G2. Module Specification — RESOLVED (§1.18.6–§1.18.8)
- [x] Array<T> TypeDef with methods (add, removeAt, insert, contains, slice, iterator) and contract impls — §1.18.6
- [x] Entity base handle type with static methods (destroy, isAlive, getOrCreate, findAll) — §1.18.7
- [x] Versioning: tracks IL spec version, reported via logging interface on mismatch — §1.18.8

### G3. `writ-std` Scope (future, not part of core spec)
- [ ] Define utility types (`List<T>`, `Map<K, V>`, etc.)
- [ ] Define common helper functions
- [ ] This is ordinary Writ code, not part of the core spec — can be implemented incrementally

---

## F. Compiler Work (implementation tasks after spec is done)

### F1. IL Emitter
- [ ] New compiler phase: AST → IL bytecode emission
- [ ] Register allocation (trivial: linear scan of locals → virtual registers)
- [ ] Constant folding (optional, can defer)
- [ ] Closure lowering to compiler-generated types (see A3)

### F2. Binary Writer
- [ ] Serialize metadata tables to binary format
- [ ] Serialize instruction streams
- [ ] Write complete module binary

### F3. Disassembler
- [ ] Text representation of IL for debugging
- [ ] `writ disasm` command to dump human-readable IL

---

## Decision Log

Decisions already made during design discussion:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Stack vs Register VM | **Register-based** | Explicit state aids serialization; virtual registers avoid alloc complexity |
| Type preservation | **Typed IL (CLR-style)** | Generics preserved, reflection support, JIT specialization possible |
| Execution model | **Cooperative yielding, preemptive serialization** | Functions are normal code; runtime manages suspension at transition points |
| Binary format | **Binary from day one** | Text format can be added later; binary is the primary artifact |
| Instruction scope | **Full instruction set** | Complete spec before implementation |
| Opcode width | **u16** | Future-proof against running out of opcode space |
| Register addressing | **u16** | Up to 65535 registers per function |
| Table indices | **u32** | Up to ~4 billion entries per table |
| Enum no-payload variants | **Tag-only, no payload space** | Decided by implementer; saves space, tag is sufficient |
| Operator overloading in IL | **Compiler concern** | Primitives use typed instructions (ADD_I, etc.); user-type ops dispatch through CALL_VIRT on contracts |
| BR padding | **Pad to RI32 shape** | Parse simplicity over 2-byte savings |
| Bulk string concat | **STR_BUILD instruction** | Common operation in game scripting (formattable strings) |
| Option/Result specialization | **Both specialized + general enum instructions** | Common types get fast path, general enum instructions for user types |
| DEFER_POP | **Keep it** | Optimization for compiler to discard irrelevant defers early |
| Memory model | **GC-assumed** | Language semantics designed for GC; runtime implementors choose algorithm |
| Structs | **Reference types** | Heap-allocated, GC-managed. Assignment copies reference (shared object). |
| Enums | **Value types** | Tag + inline payload. Copied on assignment. Reference payloads are GC-traced. |
| Binding mutability | **Binding-only** | `let`/`let mut` controls the binding, not the object. Standard GC-language model. |
| Closure mut captures | **Shared capture struct** | Compiler generates a struct for mutable captures; both outer scope and closure reference it |
| Dead entity access | **Crash** | Accessing a destroyed entity handle crashes the task (same as unwrap None) |
| Entity ownership | **Runtime owns script state, host owns native state** | Extern component fields proxied through host API |
| Save/load IL | **Include original IL in save** | PCs become invalid if scripts are recompiled; save must reference or embed the IL |
| Self parameter | **Explicit `self`/`mut self`** | Methods take explicit receiver; `self` = immutable, `mut self` = mutable; operators/hooks have implicit self |
| Binding mutability enforcement | **Strict (Rust-like)** | `let` prevents mutation AND reassignment; `let mut` allows both; applies to method calls via `self`/`mut self` |
| Construction syntax | **`new Type { ... }`** | `new` keyword disambiguates from blocks; same for structs and entities |
| Components | **Extern-only, data-only** | No script-defined components; host provides storage and behavior |
| Lifecycle hooks | **Universal `on` hooks** | `on create/finalize/serialize/deserialize` for structs+entities; `on destroy/interact` entity-only |
| Runtime-host model | **Suspend-and-confirm** | Runtime suspends on host operations until host confirms change |
| Serialization critical sections | **Removed** | CRITICAL_BEGIN/CRITICAL_END unnecessary — suspend-and-confirm model ensures saves only at transition points |
| Save/load scope | **Spec defines what, runtime defines how** | Save contents specified; format, timing, migration policy are runtime concerns |
| IL version mismatch | **Report + recommended coexistence** | Runtime must report; spec recommends loading old IL alongside new, draining old stacks naturally |
| Lifecycle hook crash | **Unwind + log** | Hook crash terminates the calling task; runtime logs to host via §1.14.7 |
| Entity cleanup order | **destroy → finalize → cleanup** | `on destroy` (deterministic), then `on finalize` (GC), then runtime removes entity |
| Dialogue suspension | **Transition points** | say/choice/wait suspend execution; host decides when to resume |
| Runtime logging | **Required interface** | Runtime must provide error/warn/info/debug logging to host |
| Component back-ref | **Compiler-emitted `@entity`** | Hidden field, unreachable from script, used for component access lowering |
| Registers | **Abstract typed slots** | Each register holds one value of its declared type; runtime determines physical storage |
| Primitive type tags | **Fixed u8 tags** | void=0x00, int=0x01, float=0x02, bool=0x03, string=0x04 |
| TypeRef encoding | **Variable-length blob** | Kind byte + payload; 6 kinds covering primitives, named types, generics, arrays, function types |
| TypeDef table | **Single table, kind flag** | Structs, enums, entities, components share one table; kind field distinguishes |
| Option/Result in types | **Regular generic enums** | No special type encoding; specialness is instruction-level only |
| Generic dispatch | **Boxing + contract table** | Value types boxed through generics; dispatch via (type_tag, contract, slot) → entry point |
| Enum tag | **u16 discriminant** | Up to 65535 variants; total size = tag + largest payload |
| Option null-ptr opt | **Permitted, not mandated** | Runtime may use null pointer for Option<ref> — IL code unchanged |
| Module format | **Binary, 200-byte fixed header** | Magic `WRIT`, format version u16, string/blob heaps, 21-table directory |
| Multi-module linking | **Name-based resolution** | Modules form a DAG; TypeRef/MethodRef/FieldRef resolved by name at load time |
| Metadata tokens | **u32: top 8 bits table, bottom 24 row** | 16M rows per table; uniform encoding for all table references |
| Module versioning | **Semver 2.0.0** | MAJOR.MINOR.PATCH; same-major compatibility; ModuleRef carries min_version |
| Heap encoding | **Length-prefixed** | u32(length) + bytes for both string heap and blob heap |
| Entity construction buffering | **INIT_ENTITY flushes batch** | Component writes buffered during SPAWN_ENTITY→INIT_ENTITY window |
| Well-known type table | **Removed** | Core types in `writ-runtime` module; no special header table needed |
| `writ-runtime` | **Runtime-provided module** | Spec mandates contents; intrinsic flag on MethodDefs for native implementations |
| `writ-std` | **Optional Writ library** | Utility types (List, Map); ordinary module, not part of core spec |
| Circular module deps | **Forbidden** | Modules must form a DAG |
| Cross-module fields | **FieldRef (ABI-safe)** | Name-based resolution at load time; field reordering doesn't break dependents |
| Boxing instructions | **BOX/UNBOX (RR shape)** | Runtime reads register type table — no redundant type token |
| TYPE_CHECK | **Dropped** | No `is` operator, no inheritance, no `any` type; GET_TAG/IS_SOME/CALL_VIRT cover all cases |
| LOAD_CONST | **Folded into LOAD_GLOBAL** | GlobalDef has mutability flag; runtime optimizes constant reads internally |
| ARRAY_CONTAINS | **Dropped** | Compiler lowers to loop with CALL_VIRT on Eq — no dedicated instruction |
| Opcode numbering | **High byte = category, low byte = instruction** | 16 categories × 256 slots each; 91 instructions assigned |
| Entity handles | **Opaque registry-based handles** | Not direct GC pointers; runtime resolves against entity registry; dead handles crash on access |
| Entity destroy/isAlive | **Static Entity namespace methods** | `Entity.destroy(e)` / `Entity.isAlive(e)` — lower to DESTROY_ENTITY / ENTITY_IS_ALIVE |
| Tick execution model | **Run-to-suspension** | Cooperative; tasks run until they suspend, complete, or crash; execution limits recommended but not mandated |
| Task state machine | **5 core states** | Ready/Running/Suspended/Completed/Cancelled; runtimes may add states (e.g., Draining for atomics) |
| Threading model | **Recommended, not required** | Runtime may dispatch tasks across threads; must guarantee atomic section exclusion |
| Execution limits | **Recommended** | Instruction budget or time limit; runtime pauses tasks at instruction boundary; not during atomic sections |
| Atomic in execution model | **Hard guarantees** | No interleaving, no budget suspension; implementation strategy is runtime choice |
| Transition points in atomic | **Compiler MUST warn** | Can cause deadlock; future mechanism to suppress warning (TODO A8) |
| Crash unwinding | **Full task stack** | All frames unwound with defer handlers at each level; secondary crashes logged and skipped |
| JOIN on cancelled task | **Crash** | No return value to deliver; joining task crashes |
| Contract dispatch | **Runtime-built from ImplDef** | Flat table by (type_tag, contract_id, slot) recommended; inline caching permitted |
| Entity storage | **Generation-indexed registry** | Opaque handles, generation counters for stale detection; ComponentSlot list for component lookup |
| Singleton storage | **Per-type registry** | Keyed by TypeDef token; getOrCreate checks registry first |
| GC algorithm | **Not prescribed** | Precise scanning recommended (enabled by typed registers); generational/incremental for game workloads |
| Finalization ordering | **Implementation-defined** | Finalizers must not assume other objects are valid; deterministic cleanup belongs in on_destroy |
| Finalization execution | **Queued, executed as tasks** | Not during GC pause; allows normal execution including transition points |
| Option tag order | **None=0, Some=1** | Zero-init = None; matches specialized IL instructions |
| Result tag order | **Ok=0, Err=1** | Matches specialized IL instructions |
| Range type | **Single generic struct** | `Range<T>` with start, end, start_inclusive, end_inclusive; supports all inclusivity combinations |
| Primitive contract dispatch | **Pseudo-TypeDefs in writ-runtime** | Int/Float/Bool/String pseudo-types anchor ImplDef entries for generic dispatch |
| Array methods | **writ-runtime intrinsics** | Methods on Array<T> TypeDef; runtime emits corresponding IL instructions; optimizer can lower |
| writ-runtime nature | **Virtual/pseudo module** | Spec-defined, runtime-provided; need not exist as separate binary file |
| writ-runtime versioning | **Tracks IL spec version** | Major version bump in IL spec = major bump in writ-runtime |

---

## Working Order

Suggested sequence for tackling these items:

1. ~~**A1–A7** — Language spec prerequisites~~ — DONE
2. ~~**C1–C4** — IL type system~~ — DONE
3. ~~**B1–B3** — Module format~~ — DONE
4. ~~**D1** — Instruction encoding (includes register sizing strategy)~~ — DONE
5. ~~**D2–D14** — Full instruction set (90 instructions, opcode table assigned)~~ — DONE
6. ~~**E1–E5** — Execution model~~ — DONE
7. ~~**G1–G2** — `writ-runtime` module contents~~ — DONE
8. **F1–F3** — Compiler implementation
9. **G3** — `writ-std` library (future, post-spec)
