# Appendix
## B. IL Decision Log

| Decision                     | Choice                                             | Rationale                                                                                                         |
|------------------------------|----------------------------------------------------|-------------------------------------------------------------------------------------------------------------------|
| Stack vs Register VM         | **Register-based**                                 | Explicit state aids serialization; virtual registers avoid alloc complexity                                       |
| Type preservation            | **Typed IL (CLR-style)**                           | Generics preserved, reflection support, JIT specialization possible                                               |
| Execution model              | **Cooperative yielding, preemptive serialization** | Functions are normal code; runtime manages suspension at transition points                                        |
| Binary format                | **Binary from day one**                            | Text format can be added later; binary is the primary artifact                                                    |
| Opcode width                 | **u16**                                            | Future-proof against running out of opcode space                                                                  |
| Register addressing          | **u16**                                            | Up to 65535 registers per function                                                                                |
| Table indices                | **u32**                                            | Up to ~4 billion entries per table                                                                                |
| Enum no-payload variants     | **Tag-only, no payload space**                     | Saves space, tag is sufficient                                                                                    |
| Operator overloading in IL   | **Compiler concern**                               | Primitives use typed instructions; user-type ops dispatch through CALL_VIRT                                       |
| BR padding                   | **Pad to 8B**                                      | Parse simplicity over 2-byte savings                                                                              |
| Bulk string concat           | **STR_BUILD**                                      | Common operation in game scripting (formattable strings)                                                          |
| Option/Result specialization | **Both specialized + general enum**                | Common types get fast path, general instructions for user types                                                   |
| DEFER_POP                    | **Keep it**                                        | Optimization for compiler to discard irrelevant defers early                                                      |
| Memory model                 | **GC-assumed**                                     | Language semantics designed for GC; runtime implementors choose algorithm                                         |
| Structs                      | **Reference types**                                | Heap-allocated, GC-managed. Assignment copies reference (shared object).                                          |
| Enums                        | **Value types**                                    | Tag + inline payload. Copied on assignment. Reference payloads are GC-traced.                                     |
| Binding mutability           | **Binding-only**                                   | `let`/`let mut` controls the binding, not the object. Standard GC-language model.                                 |
| Closure/function values      | **Delegate model (C# style)**                      | Unified: closures, function refs, and bound methods are all delegates                                             |
| Closure mut captures         | **Shared capture struct**                          | Compiler generates a struct; both outer scope and closure reference it                                            |
| Empty closures               | **Null target optimization**                       | No capture struct allocated if nothing is captured                                                                |
| Dead entity access           | **Crash**                                          | Accessing a destroyed entity handle crashes the task                                                              |
| Entity ownership             | **Runtime owns script state, host owns native**    | Extern component fields proxied through host API                                                                  |
| Save/load IL                 | **Include original IL in save**                    | PCs become invalid if scripts are recompiled                                                                      |
| Self parameter               | **Explicit `self`/`mut self`**                     | Methods take explicit receiver; operators and lifecycle hooks have implicit self                                  |
| Binding mutability           | **Strict (prevents mutation)**                     | `let` prevents both reassignment and mutation through the binding                                                 |
| Component back-ref           | **Hidden `@entity` field**                         | Compiler-emitted, unreachable from script; set during SPAWN_ENTITY, used internally for component access lowering |
| Construction syntax          | **`new Type { ... }`**                             | `new` keyword disambiguates construction from blocks; same syntax for structs and entities                        |
| Default field values         | **Runtime expressions, inlined**                   | Compiler emits default expression code at each construction site; `NEW` allocates zeroed                          |
| Components                   | **Extern-only, data-only**                         | No script-defined components; components are host-provided data schemas, no methods                               |
| Lifecycle hooks              | **Universal `on` hooks**                           | `on create/finalize/serialize/deserialize` on structs and entities; `on destroy/interact` entity-only             |
| Host communication           | **Suspend-and-confirm**                            | Runtime suspends on host operations until host confirms; aligns with game engine logic loop                       |
| Registers                    | **Abstract typed slots**                           | Each register holds one value of declared type; runtime determines physical storage                               |
| Primitive type tags          | **Fixed u8 tags (0x00–0x04)**                      | void, int, float, bool, string — self-describing, no payload                                                      |
| TypeRef encoding             | **Variable-length blob**                           | Kind byte + payload; covers primitives, TypeDef, TypeSpec, GenericParam, Array, function types                    |
| TypeDef table                | **Single table**                                   | Structs, enums, entities, components share one table; `kind` flag distinguishes                                   |
| Option/Result types          | **Regular generic enums**                          | No special type encoding; specialness at instruction level only                                                   |
| Generic dispatch             | **Boxing (CLR model)**                             | Value types boxed when passed through generic params; dispatch via contract table                                 |
| Enum tag                     | **u16 discriminant**                               | Up to 65535 variants; total size = tag + largest payload, all variants padded                                     |
| Option null-ptr opt          | **Permitted, not mandated**                        | Runtime may optimize Option<ref> to bare nullable pointer                                                         |
| Module format                | **Binary, 200-byte fixed header**                  | Magic `WRIT`, format version u16, heaps + 21-table directory                                                      |
| Multi-module linking         | **Name-based resolution at load time**             | DAG of modules; TypeRef/MethodRef/FieldRef resolved by name                                                       |
| Metadata tokens              | **u32: top 8 = table ID, bottom 24 = row**         | Uniform encoding for all cross-table references                                                                   |
| Module versioning            | **Semver 3.0.0 (MAJOR.MINOR.PATCH)**               | ModuleRef carries min_version; same-major compatibility rule                                                      |
| String/blob heaps            | **Length-prefixed**                                | u32(length) + bytes; offset 0 = empty/null                                                                        |
| Entity construction          | **INIT_ENTITY commits buffered writes**            | Component writes buffered during construction, flushed as batch                                                   |
| Well-known type table        | **Removed**                                        | Core types in `writ-runtime` module; referenced via standard TypeRef                                              |
| `writ-runtime` module        | **Runtime-provided, spec-mandated**                | Intrinsic flag on MethodDefs; runtime implements natively                                                         |
| `writ-std` module            | **Optional, written in Writ**                      | Utility types (List, Map, etc.); imports from writ-runtime                                                        |
| Circular module deps         | **Forbidden (DAG)**                                | Load order follows dependency graph                                                                               |
| Cross-module fields          | **FieldRef (name-based, ABI-safe)**                | Field reordering in dependency doesn't break dependents                                                           |
| Boxing instructions          | **BOX/UNBOX (RR shape)**                           | Runtime reads register type table — no redundant type token needed                                                |
| TYPE_CHECK instruction       | **Dropped**                                        | No `is` operator, no inheritance, no `any` type — all cases covered by GET_TAG, IS_SOME, CALL_VIRT                |
| LOAD_CONST instruction       | **Folded into LOAD_GLOBAL**                        | GlobalDef covers both; runtime optimizes constant reads via mutability flag                                       |
| Opcode numbering             | **High byte = category, low byte = instruction**   | 16 categories × 256 slots; sub-ranges within 0x0A for Option/Result/Enum                                          |
| Entity handles               | **Opaque registry-based handles**                  | Not direct GC pointers; runtime resolves against entity registry; dead handles crash on access                    |
| Entity destroy/isAlive       | **Static methods in Entity namespace**             | `Entity.destroy(e)` / `Entity.isAlive(e)` — not instance methods; lower to DESTROY_ENTITY / ENTITY_IS_ALIVE       |
| Tick execution               | **Run-to-suspension, cooperative**                 | Tasks run until suspend/complete/crash; execution limits recommended, not mandated                                |
| Task states                  | **Ready/Running/Suspended/Completed/Cancelled**    | Minimum viable set; runtimes may extend (e.g., Draining for atomics)                                              |
| Threading                    | **Recommended, not required**                      | Multi-thread dispatch supported; atomic exclusion must be guaranteed                                              |
| Atomic sections              | **Hard guarantees, implementation free**           | No interleaving, no budget pause; drain-and-run / locking / single-thread all valid                               |
| Transition in atomic         | **Compiler MUST warn**                             | Deadlock risk; future suppression mechanism (TODO A8)                                                             |
| Crash unwinding              | **Full stack, all defers**                         | Every frame unwound; secondary defer crashes logged and skipped                                                   |
| JOIN cancelled task          | **Crash**                                          | No return value exists; joining task crashes                                                                      |
