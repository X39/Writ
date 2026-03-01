# Feature Research

**Domain:** Compiler middle-end and back-end — name resolution, type checking, and IL codegen for a game scripting language with Rust-like type system, entity-component architecture, cooperative tasks, dialogue, and closures
**Researched:** 2026-03-02
**Confidence:** HIGH (spec is authoritative and complete; compiler pipeline patterns are well-established)

---

## Overview

This is not a greenfield feature set — the Writ language spec fully defines what must be compiled,
the existing AST types define the input surface, and the existing IL binary format defines the
output target. The research question is: *which features in each of the three phases (name
resolution, type checking, IL codegen) are table stakes that block everything else, which are
differentiators worth investing in now, and which seem useful but should be deferred?*

This file covers v3.0 only: the three-phase pipeline that connects the existing lowered AST
output of `writ-compiler` to the IL binary format consumed by `writ-runtime`. Everything in
v2.0 (VM, module format, assembler, runtime) is already shipped.

**Input boundary:** `Vec<AstDecl>` from the existing lowering pipeline — dialogue lowered to
`AstDecl::Fn`, entities in `AstDecl::Entity`, optionals lowered to `Option<T>`, compound
assignments expanded, operators lowered to contract impls.

**Output boundary:** A `writ_module::Module` with all 21 metadata tables populated, string/blob
heaps filled, and method bodies containing valid `Instruction` sequences consumed by `writ-runtime`.

---

## Feature Landscape

### Phase 1: Name Resolution

**What it does:** Builds a symbol table mapping every identifier use-site to its declaration.
Resolves qualified paths (`a::b::c`), `using` imports, enum variants, type references, and
method names. Produces a resolved AST (or annotated AST) where every name points to its
definition.

#### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Multi-file symbol collection | Projects span many files; all `pub` declarations across namespaces must be visible | MEDIUM | First pass over all `AstDecl` trees: collect `fn`, `struct`, `entity`, `enum`, `contract`, `impl`, `component`, `extern`, `const`, `global` by their namespace-qualified name. Must handle declarative (`namespace x;`) and block (`namespace x { }`) forms. Files without `namespace` go in the root namespace. |
| `using` resolution | Unqualified names after `using survival;` must resolve to `survival::HealthPotion` etc. | MEDIUM | `using` is scoped: file-level `using` is visible for the entire file; block `namespace` scoped `using` is visible only within that block. `using` does not re-export. Alias form (`using items = survival::items;`) adds a namespace alias, not individual names. |
| Qualified path (`::`) resolution | `survival::HealthPotion`, `::audio::Mixer` (root-anchored), `QuestStatus::InProgress` | MEDIUM | Three contexts: root-anchored `::x`, namespace access `ns::Name`, enum variant `Enum::Variant`. Namespace and type name spaces are separate so there is no ambiguity between a namespace `Option` and an enum `Option`. |
| Visibility enforcement | Private declarations are file-local; `pub` declarations are cross-file; type-private members accessible only within the type | MEDIUM | `dlg` defaults to `pub`. Everything else defaults to private. Contract impl methods are always public (cannot be made private). Lifecycle hooks have no visibility modifier. |
| Same-namespace cross-file visibility | `pub` declarations in the same namespace are visible across files without `using` | LOW | Required by spec §23.5. Within a namespace, `pub` names are freely cross-accessible. |
| Ambiguity detection | Two `using` statements importing the same unqualified name → error at the usage site (not the `using` site) | LOW | Error at usage site, not declaration. Fully qualified names always resolve unambiguously. |
| Type name resolution | `AstType::Named`, `AstType::Generic`, `AstType::Array`, `AstType::Func` all contain type references that must resolve to `TypeDef` tokens | HIGH | `Option`, `Result`, `Range`, `Array<T>`, `Entity`, the 17 contracts — all live in the `writ-runtime` virtual module and must be looked up via cross-module name resolution using the existing `Domain` infrastructure. Every `AstType` must map to a `TypeRef` blob for later IL emit. |
| Impl–type association | `impl Contract for Type { ... }` must resolve both `Contract` and `Type` to their definitions | MEDIUM | Impl blocks attach method bodies to a type. The resolver must locate the `AstImplDecl` and associate it with the correct `TypeDef` and `ContractDef`. Orphan impl detection (impl for a type not defined in this module) should be an error. |
| Generic parameter scoping | `fn foo<T: Add>(a: T) -> T` — `T` is in scope for the parameter list and body; `T` shadows any outer `T` | MEDIUM | Generic params on `AstFnDecl`, `AstStructDecl`, `AstImplDecl`, etc. Each creates a local scope with the type parameter name bound. Multiple bounds (`T: Foo + Bar`) produce multiple constraint entries. |
| Forward references within a namespace | A struct can reference another struct declared later in the same file or namespace | MEDIUM | Requires two-pass collection: collect all declarations first, then resolve all bodies. Within a single file, top-level declarations are not order-sensitive. |
| `self` / `mut self` resolution | In method bodies, `self` refers to the receiver; type of `self` is the enclosing type | LOW | During method body resolution, push a synthetic `self` binding with the enclosing type. `mut self` marks the receiver as mutable — enforce no rebinding in the resolver (type checker validates mutation). |
| Singleton speaker validation | `@Speaker` in lowered dialogue resolves to an `Entity.getOrCreate<T>()` call — `T` must be a `[Singleton]`-attributed entity with a `Speaker` component | MEDIUM | Deferred from lowering (PROJECT.md §Key Decisions: "Singleton speaker assumption"). The resolver must now verify that every speaker name used in the lowered dialogue `getOrCreate` call resolves to a `[Singleton]` entity with a `Speaker` component. |

#### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Diagnostic-quality ambiguity errors | "Name `Item` is ambiguous — defined in both `ns_a` and `ns_b`. Use `ns_a::Item` or `ns_b::Item` to disambiguate." with exact spans | MEDIUM | The resolver already has the source spans on every AST node. Good error messages are the primary UX differentiator for a language compiler. Include both candidates in the error with their definition spans. |
| `[Conditional]` and `[Singleton]` attribute validation | `[Conditional]` requires a method with a `bool`-returning signature; `[Singleton]` requires entity kind | LOW | Straightforward attribute name lookup + kind check. Deferred from lowering per PROJECT.md. Validate in resolver once declaration kinds are known. |
| Namespace path convention warnings | Flag `survival/items.writ` declaring `namespace game;` as a style warning | LOW | Spec §23.11 says this is a "language server flag it as a warning." Implement as a warn-level diagnostic, not an error. |
| Unresolved name suggestions | "Cannot find `HealthPotion`; did you mean `survival::HealthPotion`?" | MEDIUM | After resolution failure, fuzzy-match against all known `pub` names in the symbol table. Levenshtein distance or prefix-based. Significantly reduces "why doesn't this resolve?" frustration for script authors. |

#### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Re-export via `using` | "Convenient re-exports" | Spec §23.4 is explicit: `using` does not re-export. Adding re-exports would break the spec and require tracking re-export chains during resolution. | Consumers must add their own `using` or use `::` qualification. This is by design — it prevents accidental API surface exposure. |
| Import-all glob (`using survival::*`) | "Convenience import" | Not in the Writ spec. Glob imports make it ambiguous which names a file depends on, complicate IDE tooling, and create invisible name conflicts. | `using survival;` already brings the entire namespace into scope with conflict detection. |
| Structural subtyping / duck typing | "Automatically implement contracts based on field names" | Breaks the explicit contract model. Rust/Writ use nominal typing — you must explicitly `impl Contract for Type`. | Explicit `impl` blocks are the correct mechanism. The compiler can suggest "did you mean to impl ContractX for TypeY?" when a method call is unresolved. |

---

### Phase 2: Type Checking

**What it does:** Walks the resolved AST and assigns a type to every expression. Verifies
assignment compatibility, function call argument types, contract bounds satisfaction, mutability
rules, return type consistency, and special rules for `?`, `!`, `try`, closures, and entity
access patterns.

#### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Primitive type propagation | `let x = 42` infers `int`; `let y = 3.14` infers `float`; `true`/`false` are `bool`; string literals are `string` | LOW | Foundation for all other inference. The four primitive type tags (0x01–0x04) map directly to these. |
| Type inference for `let` bindings | `let x = expr` infers the type of `x` from `expr`. Function signatures, struct fields, entity properties must be fully annotated. | MEDIUM | Spec §5.2. Bidirectional type inference (push type context down to sub-expressions) is the standard approach. Constraint-based HM inference is over-engineering for a language with explicit annotations on all declarations. |
| Function call type checking | Argument types must match parameter types; return type propagates to caller | MEDIUM | Includes: arity check, positional vs named argument matching, `self`/`mut self` receiver checking, return type propagation. |
| Struct/entity field type checking | Field access `obj.field` resolves to the field's declared type; assignment checks compatibility | MEDIUM | For entities, must distinguish script fields (direct GC field) from component fields (host-proxied, requires `GET_COMPONENT` path). |
| Contract bounds checking | At generic call sites, verify concrete type arguments satisfy `T: ContractName` bounds | HIGH | Walk the `ImplDef` table (built in resolution) for each concrete type. Multiple bounds (`T: Foo + Bar`) require checking each independently. Operator calls (`a + b`) lower to contract dispatch in the AST — the type checker must verify `Add<T,R>` is implemented. |
| Mutability enforcement | `let` bindings are immutable — neither reassignment nor field mutation allowed; `let mut` allows both | HIGH | Spec: "strict binding mutability — `let` prevents both reassignment AND mutation." This is stricter than Rust's `let` (which allows re-binding via shadowing, but Writ's also prevents this). Track mutability through field access chains. `self` vs `mut self` on methods is checked here. |
| Return type checking | Every code path in a function body must return a value of the declared return type; void functions must not return values | MEDIUM | Control flow coverage: if/else, match exhaustiveness (partial), loop break values. Missing return on a non-void function is an error. |
| `Option<T>` / `?` / `!` rules | `expr?` works only on `Option<T>`, enclosing function must return `Option<...>`; `expr!` works on Option and Result | MEDIUM | The `?` postfix operator propagates `None` — check that enclosing function return type is `Option<...>`. The `!` postfix operator always succeeds or crashes — no enclosing-function constraint, but the expression type is the inner `T`. |
| `try` / Result rules | `try expr` works only on `Result<T,E>`, enclosing function must return compatible `Result` | MEDIUM | Spec §18.3. Check that the `E` in the enclosing return type is compatible (same or wider) with the `E` in `Result<T,E>`. No implicit conversion — the error types must be compatible. |
| Pattern match exhaustiveness | `match` on an enum must cover all variants or include a wildcard; `if let` is inherently non-exhaustive (just like an `if` without `else`) | HIGH | For enums: collect all variants from the TypeDef, verify each variant is covered by at least one arm. Range patterns and or-patterns add complexity. Non-exhaustive match on a non-enum scrutinee (e.g., `int`) requires a wildcard. |
| Component access type rules | `guard[Health]` on a concrete entity type where `Health` is declared returns `Health` (not Optional); `target[Health]` on generic `Entity` returns `Option<Health>` | MEDIUM | The resolver knows which components an entity type declares. If the component is declared in the entity, the access is guaranteed and returns `Health` directly. Otherwise returns `Option<Health>`. |
| Closure capture and type inference | Lambda expressions (`fn(x: int) { ... }`) infer captured variable types from surrounding scope; check that captured `let` variables are not mutated inside the closure | HIGH | Closures capture `let` (immutable) by value and `let mut` by reference. The type checker must identify captures, verify immutability constraints, and determine the closure's function type for matching against `fn(...)` type annotations. |
| Generic type argument inference | `let item = first(inventory)` infers `T = Item` from `List<Item>`; `parse<int>("42")` is explicit | HIGH | Inference requires unification of type variables against concrete types. For Writ's simple case (no higher-kinded types, single-level generics), standard constraint accumulation + unification is sufficient. Explicit type args are used as ground truth. |
| Binary operator type checking | `a + b` dispatches through `Add<T,R>` contract — type checker must locate the impl and return type `R` | MEDIUM | All binary operators were already lowered in v1.x to explicit contract call forms in the AST (`BinaryOp::Add` etc.). The type checker resolves each to the appropriate impl and reads the return type from the contract method signature. |
| `spawn`/`join`/`cancel` type rules | `spawn expr` returns a task handle typed to the return type; `join handle` returns the result type; `cancel handle` is void | MEDIUM | `spawn expr` where `expr: fn() -> T` produces a `TaskHandle<T>`. `join handle` where `handle: TaskHandle<T>` returns `T`. `cancel` accepts any `TaskHandle`. Detached tasks have no handle. |
| `new Type { field: value }` construction type | The type of a `new` expression is the constructed type; field types must match declared field types; all required fields must be present | MEDIUM | For entities: only entity properties (not component slots) appear in the `new { }` literal. Component overrides live in the entity declaration, not the construction site. |
| `for` loop variable binding type | `for x in collection` — type of `x` is the element type of the `Iterable<T>` impl on `collection`'s type | MEDIUM | Look up `Iterable<T>` impl for the collection type, extract `T`. Bind `x` with type `T` in the loop body. Arrays, `Range<int>`, and user types with `Iterable` impls all work the same way. |

#### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Precise mutability error messages | "Cannot assign to `self.gold` because `self` is not `mut self`. Change the method signature to `fn applyDiscount(mut self, ...)` to allow mutation." | MEDIUM | The type checker already knows the source span and the binding site. Threading the "reason" through the error lets the error message point at both the invalid mutation site and the immutable binding declaration. |
| Contract satisfaction suggestion | "Type `Guard` does not implement `Iterable<T>`. To use `Guard` in a `for` loop, add `impl Iterable<Entity> for Guard { ... }`." | MEDIUM | After a failed contract check, compute which contract is missing and emit a diagnostic with a concrete suggestion. Especially valuable for newcomers who write `for x in guards` and get a cryptic error. |
| Closure capture inference | No explicit capture annotations required — the type checker infers which outer variables are captured and at what mutability | MEDIUM | This is the standard approach (Rust, Swift). The alternative (explicit capture lists) is fine for power users but burdensome for the game-scripting audience Writ targets. Infer captures by walking the closure body for unbound names. |
| Dead-code path type checking | Flag unreachable code after `return`, `->` (dialogue transition), or `crash` | LOW | After a definite return/crash, further statements are unreachable. Emit a warning-level diagnostic. The type checker's control flow graph already tracks this for exhaustiveness. |

#### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Implicit coercions | "Let me pass `int` where `float` is expected" | Spec §10.2 is explicit: no implicit conversion at assignment or argument boundaries — callers must use `.into<float>()`. Implicit coercions hide bugs and make type inference non-local. | Explicit `expr.into<T>()` is the Writ way. The only implicit coercion in the spec is formattable-string interpolation (automatically calls `.into<string>()`). |
| Full HM type inference for all declarations | "Infer field types, parameter types, return types automatically" | Spec §5.2 requires full annotation on all declarations (functions, fields, entity properties). HM inference for declaration-site types introduces complex error messages and makes the language harder to read without IDE support. | Inference only for `let` bindings and generic type arguments at call sites. All declarations are fully annotated. |
| Structural pattern exhaustiveness for non-enums | "Exhaustive match on `int` without wildcard" | Spec allows matches on non-enum types but a wildcard is required for non-enum scrutinees. Proving range coverage for `int` requires SMT-level reasoning. | Require wildcard (`_`) for non-enum match scrutinees. Error message: "match on `int` must include a wildcard arm." |
| Cross-function escape analysis for closures | "Statically prove closures don't escape their enclosing scope" | Writ closures are GC-managed delegates — they can escape freely (stored in fields, returned, passed to `spawn`). Escape analysis is a future optimization concern for the JIT, not a correctness requirement here. | GC handles closure lifetime. The type checker only checks that captured `let` variables are not mutated inside the closure, not escape lifetime. |

---

### Phase 3: IL Code Generation

**What it does:** Walks the type-annotated AST and emits IL `Instruction` sequences into
`writ-runtime`'s module format. Assigns virtual registers, populates all 21 metadata tables,
emits TypeRef blobs, resolves method/field/type tokens, and produces a `writ_module::Module`
ready for execution by the existing VM.

#### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| ModuleDef + ModuleRef population | Every valid module needs a `ModuleDef` row (name, version, flags) and `ModuleRef` rows for `writ-runtime` and any cross-module dependencies | LOW | `ModuleDef` is always 1 row. `ModuleRef` rows correspond to external namespaces/modules referenced. `writ-runtime` is always a dependency. |
| TypeDef emission for all named types | `struct`, `entity`, `enum`, `component`, `extern struct`, `extern component` → `TypeDefRow` with appropriate `kind` | MEDIUM | Includes: field list, method list, generic param list, component slot list (for entities). Entity kind = 2, Struct = 0, Enum = 1, Component = 3. |
| FieldDef emission | Every field on every TypeDef → `FieldDefRow` with name, type blob, flags (pub/priv, component field flag) | MEDIUM | Component fields need a flag distinguishing them from script fields (the VM uses this to route GET_FIELD/SET_FIELD to host proxy vs GC heap). TypeRef blobs for field types built from the type checker's resolved type information. |
| MethodDef + ParamDef emission | Every function, method, lifecycle hook → `MethodDefRow` with name, signature blob, flags; each parameter → `ParamDefRow` | MEDIUM | Signature blob: param_count u16, param_types as TypeRef blobs, return_type TypeRef blob. Lifecycle hooks get hook_kind flags (1=on_create, 2=on_destroy, 3=on_interact, 4=on_serialize, 5=on_deserialize, 6=on_finalize). |
| ContractDef + ContractMethod emission | Every `contract` declaration → `ContractDefRow` + one `ContractMethodRow` per member | MEDIUM | Includes the 17 built-in contracts from `writ-runtime` virtual module (these are referenced via TypeRef, not re-emitted). Only user-defined contracts are emitted into the module's ContractDef table. |
| ImplDef emission | Every `impl Contract for Type` block → `ImplDefRow` mapping (type_token, contract_token) + method list | MEDIUM | Operator impls (`operator +` etc.) were already lowered to contract impls in the AST. Each `AstImplDecl` → one ImplDef row. Method list points into the MethodDef table. |
| GenericParam + GenericConstraint emission | Type parameters (`<T>`) → `GenericParamRow`; bounds (`T: Add`) → `GenericConstraintRow` | LOW | Straightforward mapping from `AstGenericParam`. GenericParam ordinal = position in the parameter list. Each bound → one GenericConstraint row referencing the GenericParam and the contract. |
| GlobalDef + ExternDef emission | `global mut name: type` → `GlobalDefRow`; `extern fn` / `extern struct` / `extern component` → `ExternDefRow` | LOW | Globals are allocated by the VM; the GlobalDef row provides the type and name. ExternDef rows are the call targets for `CALL_EXTERN` instructions. |
| ComponentSlot emission | Each `use Component { overrides }` in an entity → `ComponentSlotRow` with entity type, component type, override blob | MEDIUM | Override blob: serialized list of (field_name_or_index, value) pairs. The VM uses these at `SPAWN_ENTITY` time to configure the host-allocated component instances. |
| Register allocation (linear) | Assign a virtual register index to each local variable, temporary expression result, and function parameter | MEDIUM | Linear allocation: each new SSA-like value gets the next register number. No reuse / lifetime-aware allocation needed for correctness — the VM uses the declared register count from `MethodDef` metadata. Register type declarations (one TypeRef blob per register) must be emitted in the method body header. |
| Basic IL instruction emission (arithmetic, control flow, data movement) | The core of the compiler — emit the right instructions for each AST node | HIGH | Covers: `LOAD_INT`, `LOAD_FLOAT`, `LOAD_TRUE/FALSE`, `LOAD_STRING`, `LOAD_NULL`, `ADD_I/F`, `SUB_I/F`, `MUL_I/F`, `DIV_I/F`, `MOD_I/F`, `NEG_I/F`, `BIT_AND`, `BIT_OR`, `SHL`, `SHR`, `NOT`, all 6 comparison instructions, `BR`, `BR_TRUE`, `BR_FALSE`, `SWITCH`, `RET`, `RET_VOID`, `MOV`. |
| `CALL` / `CALL_VIRT` emission | Direct calls (`fn foo()`) → `CALL`; contract dispatch calls (`value.contractMethod()`) → `CALL_VIRT` with (type_tag, contract_id, slot) | HIGH | `CALL` encodes a MethodRef token. `CALL_VIRT` encodes (TypeRef, ContractRef, slot_index). The type checker has already determined which calls are virtual (based on whether the receiver's static type is a concrete type or a type parameter). |
| `CALL_EXTERN` emission | `extern fn` calls suspend the task via the runtime-host protocol | MEDIUM | Each `CALL_EXTERN` encodes an ExternDef index. The type checker already marked these as transition points. The codegen looks up the ExternDef row for the extern function declaration. |
| Object model instruction emission | `NEW`, `GET_FIELD`, `SET_FIELD` for struct and entity script fields | MEDIUM | `new Type { }` → `NEW` + zero or more `SET_FIELD`. Component field accesses go through `GET_COMPONENT` (for the component handle) then `GET_FIELD`/`SET_FIELD` on the component result. Type checker already determined which fields are script vs component. |
| Entity instruction emission | `SPAWN_ENTITY`, `INIT_ENTITY`, `DESTROY_ENTITY`, `ENTITY_IS_ALIVE`, `GET_COMPONENT`, `GET_OR_CREATE`, `FIND_ALL` | HIGH | `new Guard { }` → `SPAWN_ENTITY` + `SET_FIELD` overrides + `INIT_ENTITY`. `Entity.destroy()` → `DESTROY_ENTITY`. `Entity.getOrCreate<T>()` → `GET_OR_CREATE`. The spec §14.7.5 defines the exact construction sequence that codegen must produce. |
| Array instruction emission | `NEW_ARRAY`, `ARRAY_INIT`, `ARRAY_LOAD`, `ARRAY_STORE`, `ARRAY_LEN`, `ARRAY_ADD`, `ARRAY_REMOVE`, `ARRAY_INSERT`, `ARRAY_SLICE` | MEDIUM | Array literals `[1, 2, 3]` → `NEW_ARRAY` + `ARRAY_INIT` + per-element `ARRAY_ADD`. Index access `arr[i]` → `ARRAY_LOAD` (get) or `ARRAY_STORE` (set). |
| Option/Result instruction emission | `WRAP_SOME`, `UNWRAP`, `IS_SOME`, `IS_NONE`, `WRAP_OK`, `WRAP_ERR`, `UNWRAP_OK`, `IS_OK`, `IS_ERR`, `EXTRACT_ERR` | MEDIUM | `!` postfix on `Option` → `UNWRAP`; on `Result` → `UNWRAP_OK`. `?` postfix on `Option` → `IS_NONE` + `BR_TRUE` to return `None`. `try` on `Result` → `IS_ERR` + `BR_TRUE` to propagate. |
| Closure / delegate emission | Lambda expressions → compiler-generated struct TypeDef (capture struct) + method + `NEW_DELEGATE` | HIGH | Each lambda with captures: emit a generated TypeDef for the capture struct, emit a MethodDef for the lambda body with the capture struct as the implicit first param (env), emit `NEW` + `SET_FIELD` for captures + `NEW_DELEGATE`. Capture-free lambdas: emit just the MethodDef + `NEW_DELEGATE` with null target. |
| `spawn`/`join`/`cancel`/`defer` instruction emission | `SPAWN_TASK`, `SPAWN_DETACHED`, `JOIN`, `CANCEL`, `DEFER_PUSH`, `DEFER_POP`, `DEFER_END` | HIGH | `spawn expr` → `SPAWN_TASK` with method token; `spawn detached expr` → `SPAWN_DETACHED`; `join handle` → `JOIN`; `cancel handle` → `CANCEL`; `defer { body }` → `DEFER_PUSH` + body + `DEFER_END`. |
| `atomic { }` block emission | `ATOMIC_BEGIN` + body + `ATOMIC_END` | LOW | Direct wrapping. |
| Conversion instruction emission | `I2F`, `F2I`, `I2S`, `F2S`, `B2S`, `CONVERT` | LOW | Primitive conversions use the specific instructions. `expr.into<T>()` on user types dispatches through the `Into` contract → `CALL_VIRT` with `CONVERT` as the backing intrinsic for primitives. |
| String instruction emission | `STR_CONCAT`, `STR_BUILD`, `STR_LEN` | LOW | `a + b` where both are `string` → `STR_CONCAT`. Formattable string interpolation chains (already expanded in v1.x) → `STR_BUILD`. |
| Boxing for value types in generic context | When a value type (`int`, `float`, `bool`, enum) is passed to a generic parameter, emit `BOX` | MEDIUM | The type checker knows which call sites pass value types to generic parameters. Codegen emits `BOX` before the call and `UNBOX` after if the result needs to be used as a value type. |
| Pattern match emission | `match` on enums → `GET_TAG` + `SWITCH` jump table or chain of `BR_TRUE`/`BR_FALSE`; destructuring → `EXTRACT_FIELD` | HIGH | Enum match: `GET_TAG` → `SWITCH` with one offset per variant → per-variant body + `EXTRACT_FIELD` for payload. `if let Option::Some(x) = expr` → `IS_SOME` + `BR_FALSE` to else + `UNWRAP` to bind `x`. Wildcard `_` → unconditional `BR` to end. |
| Enum construction instruction emission | `NEW_ENUM r_dst, variant_tag, payload_fields` | LOW | `QuestStatus::InProgress(step: 5)` → `LOAD_INT r1, 5` + `NEW_ENUM r0, tag=1, [r1]`. Tag-only variants: `NEW_ENUM r0, tag=0, []`. |
| Localization metadata emission | `LocaleDef` table rows for all auto-generated and manual localization keys | LOW | The FNV-1a keys were already computed in v1.x lowering. Codegen reads them from the lowered AST and emits them into the `LocaleDef` table. The `say_localized` call was already emitted by lowering. |
| Lifecycle hook method registration | `on_create`, `on_destroy`, `on_interact`, `on_finalize`, `on_serialize`, `on_deserialize` → MethodDef rows with correct `hook_kind` flags | MEDIUM | Each `AstEntityHook` in `AstEntityDecl.hooks` → one `MethodDefRow` with hook_kind = 1–6. The entity's TypeDef row references these hook methods. The VM uses these to fire hooks at the right time. |
| ExportDef emission | `pub` top-level declarations → `ExportDefRow` entries for cross-module accessibility | LOW | Every `pub` top-level fn, type, contract, global → one `ExportDef` row. Used by cross-module TypeRef resolution at domain load time. |

#### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Debug info emission (DebugLocal + SourceSpan) | Crash reports and disassembler output show original `.writ` file/line/col; essential for script authors debugging | MEDIUM | Every AST node carries a `SimpleSpan`. Codegen can emit `SourceSpan` entries mapping IL instruction offsets to source spans. `DebugLocal` entries map register indices to variable names. Module flag bit 0 signals debug info presence. The existing disassembler already reads this format. |
| Tail call for dialogue transitions | `-> otherDialog(args)` lowers to `return otherDialog()` in the AST — codegen must emit `TAIL_CALL` to prevent unbounded stack growth in long dialogue chains | MEDIUM | Spec says `->` is a tail call. Dialogue trees can be deep (50+ transitions). `TAIL_CALL` reuses the current frame. Codegen must recognize `return f(args)` in functions lowered from `dlg` and emit `TAIL_CALL` instead of `CALL` + `RET`. |
| `CALL_VIRT` specialization for known types | When the receiver's static type is concrete (not a generic param), the type checker knows the exact method — codegen can emit `CALL` instead of `CALL_VIRT`, skipping dispatch overhead | LOW | A simple optimization that falls naturally out of the type information already computed. Emit `CALL_VIRT` only when the receiver is a generic type parameter or a contract-typed parameter. |
| `?` propagation desugaring in codegen | `expr?` and `try expr` are still AST-level `UnaryPostfix::NullPropagate` and `AstExpr::Try` nodes — codegen must expand them to the correct instruction sequences with early return | MEDIUM | `?` on `Option<T>`: `IS_NONE` + `BR_FALSE` around `LOAD_NULL` + `RET` (return None), otherwise `UNWRAP`. `try` on `Result<T,E>`: `IS_ERR` + `BR_FALSE` around `EXTRACT_ERR` + `WRAP_ERR` + `RET`, otherwise `UNWRAP_OK`. |
| Constant folding for int/float literals | `const MAX_HP: int = 100 * 100` → emit `LOAD_INT 10000` directly | LOW | Fold constant arithmetic expressions during codegen. Avoids emitting pointless arithmetic instructions for compile-time-known values. Apply only to `const` declarations and literal-only expressions. |
| AttributeDef emission | `[Singleton]`, `[Conditional]`, and user attributes stored in the `AttributeDef` table for runtime introspection | LOW | The runtime uses `[Singleton]` at `GET_OR_CREATE` time. `[Conditional]` is evaluated by the host. Emitting `AttributeDef` rows preserves semantic metadata in the binary. |

#### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Register reuse / liveness analysis | "Smaller register count, faster VM" | The VM allocates registers abstractly — the physical size depends on the TypeRef blob, not a fixed slot size. The overhead of liveness-based allocation dwarfs its benefit for an interpreted VM. Premature optimization that increases codegen complexity significantly. | Linear allocation: next-register = ++counter. The VM handles the physical layout. Save register reuse for the JIT milestone. |
| Optimization passes (DCE, CSE, inlining) | "Faster IL output" | The reference VM is an interpreter for correctness validation, not a performance benchmark. Optimization passes increase codegen complexity without improving the correctness story. | Correctness first. JIT milestone adds optimization. Any IL optimization that matters (like tail-call) is spec-required, not a general optimization pass. |
| Stack machine output | "Simpler to implement than register assignment" | The IL spec is register-based (spec §2.1). Emitting stack-machine-style IL into a register machine (all temps go to fresh registers, then immediately MOVed to results) is technically correct but wasteful and harder to read in the disassembler output. | Linear register allocation (next-register strategy) is only slightly more complex than stack machine output and produces idiomatic IL that matches hand-written assembler tests. |
| Two-phase type inference (full HM) | "More powerful, eliminates annotation burden" | Spec requires explicit annotations on all declarations. Two-phase HM with constraint solving is significantly more complex to implement, debug, and produce good error messages for. The game scripting audience doesn't need inferred function signatures. | Bidirectional propagation: push known type context down into `let` bindings and generic call sites. Explicit annotations on everything else. Same power for Writ's type system, much simpler implementation. |

---

## Feature Dependencies

```
[Name Resolution: symbol table]
    └──required by──> [Type Checking: all name lookups]
    └──required by──> [Type Checking: contract bounds checking]
    └──required by──> [IL Codegen: all token resolution]

[Name Resolution: type name resolution to TypeRef]
    └──required by──> [Type Checking: all expression typing]
    └──required by──> [IL Codegen: TypeRef blob emission]

[Name Resolution: impl–type association]
    └──required by──> [Type Checking: contract satisfaction]
    └──required by──> [IL Codegen: ImplDef emission]

[Type Checking: expression types]
    └──required by──> [IL Codegen: correct instruction selection]
                          (ADD_I vs ADD_F, CALL vs CALL_VIRT, UNWRAP vs UNWRAP_OK)

[Type Checking: mutability]
    └──required by──> [IL Codegen: mut self vs self parameter encoding]

[Type Checking: closure captures]
    └──required by──> [IL Codegen: capture struct TypeDef + NEW_DELEGATE emission]

[Type Checking: generic type arguments at call sites]
    └──required by──> [IL Codegen: boxing for value-type generic args]
    └──required by──> [IL Codegen: CALL vs CALL_VIRT selection]

[Type Checking: component field vs script field distinction]
    └──required by──> [IL Codegen: GET_COMPONENT path vs direct GET_FIELD]

[IL Codegen: TypeDef emission]
    └──required by──> [IL Codegen: FieldDef, MethodDef, ComponentSlot emit]

[IL Codegen: MethodDef emission]
    └──required by──> [IL Codegen: method body (instruction sequence) emit]
    └──required by──> [IL Codegen: ImplDef emit (method list)]

[IL Codegen: register allocation]
    └──required by──> [IL Codegen: register type table in method body header]
    └──required by──> [IL Codegen: all instruction operand encoding]

[writ-runtime virtual module (already exists)]
    └──required by──> [Name Resolution: Option/Result/Range/17 contracts/Entity resolution]
    └──required by──> [Type Checking: built-in contract satisfaction]
    └──required by──> [IL Codegen: cross-module TypeRef to writ-runtime types]

[Existing AST lowering (already shipped v1.x)]
    └──provides──> [Name Resolution: input AstDecl trees]

[Existing writ-module crate (already shipped v2.0)]
    └──provides──> [IL Codegen: Module/ModuleBuilder/Instruction types as output targets]
```

### Dependency Notes

- **Name resolution is the absolute prerequisite for everything.** Neither type checking nor
  codegen can proceed without knowing what each identifier refers to. Build and test the symbol
  table first, with comprehensive test coverage across namespaces, `using`, and qualified paths.

- **Type checking depends on name resolution being complete, not incremental.** The type checker
  needs to look up any type, function, or contract by token. Building the full symbol table in
  a dedicated pass (separate from the type-checking pass) is cleaner than interleaving.

- **IL codegen depends on type annotations, not just types.** Instruction selection (e.g.,
  `ADD_I` vs `ADD_F`, `CMP_EQ_I` vs `CMP_EQ_S`) requires knowing the concrete type of each
  sub-expression at emit time. The type checker must annotate every expression node with its
  resolved type before codegen begins.

- **Closure codegen depends on capture analysis.** The codegen for a lambda must know its
  capture set to emit the correct capture struct TypeDef and `NEW` + `SET_FIELD` sequence.
  Capture analysis is part of type checking, not codegen.

- **`writ-runtime` virtual module must be loadable before name resolution can complete.** The
  17 contracts, `Option`, `Result`, `Range`, `Entity`, `Array<T>`, and primitive pseudo-TypeDefs
  are referenced in almost every Writ program. The resolver must be able to look up names in the
  virtual module's metadata. The existing `Domain` + `ModuleRef` resolution infrastructure
  (already in `writ-runtime`) provides this.

- **Entity construction sequence is spec-mandated.** `new Guard { name: "Steve" }` must emit
  exactly `SPAWN_ENTITY` → `SET_FIELD` overrides → `INIT_ENTITY` in that order (spec §14.7.5).
  This is not implementation-defined — the VM's entity registry enforces this ordering.

---

## MVP Definition

### Launch With (v3.0 — Full Compilation Pipeline)

The compilation pipeline must be able to: resolve all names in a multi-file Writ project,
type-check all expressions and declarations, and emit a spec-valid IL module that the existing
`writ-runtime` VM can execute correctly end-to-end.

**Name Resolution:**
- [ ] **Symbol collection (all declaration kinds)** — collect `fn`, `struct`, `entity`, `enum`, `contract`, `impl`, `component`, `extern`, `const`, `global` across all files into a symbol table keyed by fully-qualified namespace path
- [ ] **`using` resolution (plain and alias)** — scoped `using` declarations bringing names into scope with conflict detection
- [ ] **Qualified path `::` resolution** — namespace access, enum variant access, root-anchored `::Name` resolution
- [ ] **Visibility enforcement** — file-local private, cross-namespace pub, type-private members
- [ ] **Type name resolution** — every `AstType` mapped to a `TypeRef` blob or primitive tag; `writ-runtime` virtual module types resolved via cross-module lookup
- [ ] **Impl–type association** — impl blocks matched to their TypeDef and ContractDef
- [ ] **Generic parameter scoping** — type parameters scoped to their declaration and body
- [ ] **Singleton speaker validation** — `@Speaker` in lowered dialogue verified as `[Singleton]` entity with `Speaker` component
- [ ] **`[Singleton]` and `[Conditional]` attribute validation** — attribute targets checked against allowed kinds

**Type Checking:**
- [ ] **Primitive type propagation** — literals typed as `int`/`float`/`bool`/`string`
- [ ] **`let` inference and annotation** — infer from initializer; annotated `let` checks compatibility
- [ ] **Function call checking** — arity, argument types, return type propagation, `self` receiver
- [ ] **Field access checking** — struct/entity script fields vs component fields; visibility
- [ ] **Contract bounds checking** — generic call sites verify concrete types satisfy all bounds
- [ ] **Mutability enforcement** — `let` vs `let mut`, `self` vs `mut self`, no mutation through immutable binding
- [ ] **Return type checking** — all paths return correct type; void functions don't return values
- [ ] **`Option`/`?`/`!` rules** — `?` requires `Option` scrutinee and `Option` return context
- [ ] **`Result`/`try`/`!` rules** — `try` requires `Result` scrutinee and compatible `Result` return context
- [ ] **Pattern exhaustiveness for enums** — all variants covered or wildcard present
- [ ] **Component access type rules** — concrete entity type → guaranteed component; generic Entity → `Option<Component>`
- [ ] **Closure capture inference** — identify captures, check immutability constraints, determine closure's function type
- [ ] **Generic type argument inference** — unify type variables at call sites; explicit type args as ground truth
- [ ] **`spawn`/`join`/`cancel` type rules** — task handle types and result type propagation
- [ ] **`new` construction type checking** — field presence, field types, entity vs struct distinction
- [ ] **`for` loop element type binding** — `Iterable<T>` impl lookup for element type

**IL Code Generation:**
- [ ] **Module metadata emission** — ModuleDef, ModuleRef (including `writ-runtime`), ExportDef
- [ ] **TypeDef + FieldDef + MethodDef + ParamDef emission** — all named types with full metadata
- [ ] **ContractDef + ContractMethod + ImplDef emission** — user contracts and their implementations
- [ ] **GenericParam + GenericConstraint emission** — type parameters and their bounds
- [ ] **GlobalDef + ExternDef emission** — globals and extern function declarations
- [ ] **ComponentSlot emission** — entity component slots with override blobs
- [ ] **Register allocation (linear)** — assign register indices; emit per-register TypeRef table in method body header
- [ ] **All basic instruction emission** — arithmetic (I and F), logic, comparison, data movement, control flow
- [ ] **`CALL` / `CALL_VIRT` / `CALL_EXTERN` / `CALL_INDIRECT` emission** — correct dispatch selection from type checker info
- [ ] **Object model emission** — `NEW`, `GET_FIELD`, `SET_FIELD` for structs
- [ ] **Entity instruction emission** — `SPAWN_ENTITY`, `INIT_ENTITY`, `DESTROY_ENTITY`, `ENTITY_IS_ALIVE`, `GET_COMPONENT`, `GET_OR_CREATE`, `FIND_ALL`
- [ ] **Array instruction emission** — all 9 array instructions
- [ ] **Option/Result instruction emission** — all 10 Option/Result instructions
- [ ] **Closure/delegate emission** — capture struct TypeDef, method body, `NEW_DELEGATE` with correct target
- [ ] **Concurrency instruction emission** — `SPAWN_TASK`, `SPAWN_DETACHED`, `JOIN`, `CANCEL`, `DEFER_PUSH`, `DEFER_POP`, `DEFER_END`
- [ ] **`atomic { }` block emission** — `ATOMIC_BEGIN` + body + `ATOMIC_END`
- [ ] **Pattern match emission** — `GET_TAG` + `SWITCH` or branch chain + `EXTRACT_FIELD` for enum destructuring
- [ ] **Enum construction emission** — `NEW_ENUM` with tag and payload registers
- [ ] **Conversion instruction emission** — `I2F`, `F2I`, `I2S`, `F2S`, `B2S`, `CONVERT`
- [ ] **String instruction emission** — `STR_CONCAT`, `STR_BUILD`, `STR_LEN`
- [ ] **Boxing for generic value types** — `BOX`/`UNBOX` at generic call sites where value types are passed
- [ ] **Lifecycle hook registration** — hook MethodDef rows with correct hook_kind flags
- [ ] **`?` propagation and `try` desugaring** — expand to IS_NONE/IS_ERR + early return instruction sequences
- [ ] **Tail call for dialogue transitions** — `return dialogueFn(args)` in dlg-lowered functions → `TAIL_CALL`
- [ ] **Localization metadata emission** — `LocaleDef` table rows for auto and manual keys
- [ ] **Debug info emission** — SourceSpan entries mapping instruction offsets to source spans; DebugLocal entries mapping registers to names

### Add After Validation (v3.x)

- [ ] **Diagnostic-quality ambiguity errors with multiple candidates** — add when basic error messages are validated
- [ ] **Unresolved name suggestions (fuzzy match)** — add when the symbol table is complete and stable
- [ ] **Contract satisfaction suggestion in type errors** — add when contract checking is working
- [ ] **`CALL_VIRT` specialization to `CALL` for known concrete types** — add as a cleanup pass after correctness is validated
- [ ] **Constant folding for `const` expressions** — add when performance testing reveals unnecessary arithmetic in emitted IL
- [ ] **`AttributeDef` table emission** — add when host integration requires runtime attribute inspection

### Future Consideration (v4+)

- [ ] **`writ-std` module** — `List<T>`, `Map<K,V>`, common utilities — ordinary Writ code, separate milestone; requires v3.0 codegen to be validated first
- [ ] **Incremental compilation** — file-level change detection, partial re-type-check; requires stable module identity scheme
- [ ] **Language server (LSP)** — name resolution and type checking are the semantic foundation; add after v3.0 pipeline is stable
- [ ] **JIT compilation** — separate `writ-jit` crate; requires reference interpreter (v2.0) + type-annotated IR (v3.0)

---

## Feature Prioritization Matrix

| Feature Area | User Value | Implementation Cost | Priority |
|---|---|---|---|
| Symbol collection (all declaration kinds) | HIGH | MEDIUM | P1 |
| `using` + `::` resolution + visibility | HIGH | MEDIUM | P1 |
| Type name resolution to TypeRef | HIGH | HIGH | P1 |
| Impl–type association | HIGH | MEDIUM | P1 |
| Primitive type propagation + `let` inference | HIGH | LOW | P1 |
| Function call type checking | HIGH | MEDIUM | P1 |
| Field access + component field distinction | HIGH | MEDIUM | P1 |
| Contract bounds checking | HIGH | HIGH | P1 |
| Mutability enforcement | HIGH | HIGH | P1 |
| Return type checking | HIGH | MEDIUM | P1 |
| Option/Result/`?`/`!`/`try` rules | HIGH | MEDIUM | P1 |
| Pattern exhaustiveness for enums | HIGH | HIGH | P1 |
| Closure capture inference | HIGH | HIGH | P1 |
| Generic type argument inference | HIGH | HIGH | P1 |
| `for` loop element type + `spawn` types | HIGH | MEDIUM | P1 |
| Module metadata emission (ModuleDef/ModuleRef/Export) | HIGH | LOW | P1 |
| TypeDef + FieldDef + MethodDef emission | HIGH | MEDIUM | P1 |
| ImplDef + ContractDef emission | HIGH | MEDIUM | P1 |
| Register allocation + per-register type table | HIGH | MEDIUM | P1 |
| All basic instruction emission | HIGH | HIGH | P1 |
| `CALL`/`CALL_VIRT`/`CALL_EXTERN` emission | HIGH | HIGH | P1 |
| Object model emission (NEW/GET_FIELD/SET_FIELD) | HIGH | MEDIUM | P1 |
| Entity instruction emission | HIGH | HIGH | P1 |
| Array instruction emission | HIGH | MEDIUM | P1 |
| Option/Result instruction emission | HIGH | MEDIUM | P1 |
| Closure / delegate emission | HIGH | HIGH | P1 |
| Concurrency instruction emission | HIGH | HIGH | P1 |
| Pattern match emission | HIGH | HIGH | P1 |
| `?` propagation + `try` desugaring in codegen | HIGH | MEDIUM | P1 |
| Tail call for dialogue transitions | HIGH | MEDIUM | P1 |
| Lifecycle hook registration | HIGH | MEDIUM | P1 |
| Debug info emission (SourceSpan + DebugLocal) | HIGH | MEDIUM | P1 |
| `[Singleton]`/`[Conditional]` attribute validation | MEDIUM | LOW | P1 |
| Singleton speaker validation | MEDIUM | LOW | P1 |
| Localization metadata emission | MEDIUM | LOW | P1 |
| Diagnostic-quality ambiguity + suggestion errors | MEDIUM | MEDIUM | P2 |
| `CALL_VIRT` → `CALL` specialization for concrete types | MEDIUM | LOW | P2 |
| Constant folding for `const` expressions | LOW | LOW | P2 |
| `AttributeDef` table emission | LOW | LOW | P2 |
| Unresolved name fuzzy suggestions | MEDIUM | MEDIUM | P2 |
| `writ-std` module | MEDIUM | MEDIUM | P3 |
| Incremental compilation | HIGH | HIGH | P3 |
| Language server (LSP) | HIGH | HIGH | P3 |
| JIT compilation | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for v3.0 launch (complete, working compilation pipeline)
- P2: Should have; add after pipeline validated end-to-end
- P3: Future consideration; separate milestone

---

## Prior Art and Reference Implementations Analyzed

**Name resolution in multi-file projects:**
The approach of separating declaration collection (pass 1) from body resolution (pass 2) is
standard in compilers like Go (package-level declaration collection before type checking) and
Rust (where the resolver handles forward references within a module). For Writ's namespace model,
the closest analog is Go's package system: all files in the same namespace contribute to a shared
symbol space, with `using` as the equivalent of Go's named imports.

**Bidirectional type inference:**
Writ's type system (explicit annotations on declarations, inference only for `let` bindings and
generic call sites) matches the approach used in Swift and Kotlin rather than full Hindley-Milner.
Bidirectional propagation (push expected types down, pull inferred types up) is well-documented
in "Bidirectional Type Checking" (Dunfield & Krishnaswami 2021) and is simpler to implement with
high-quality error messages than constraint-based HM.

**Generic dispatch through contract tables:**
The `(concrete_type_tag, contract_id, slot) → method` lookup table that the type checker
validates is already built by the existing `writ-runtime` for `CALL_VIRT`. The codegen emits
the three-part key; the runtime resolves it. This mirrors the CLR's `callvirt` + virtual method
table pattern, simplified to a flat hash map.

**Closure representation:**
The C# delegate model (compiler-generated capture struct + method + delegate object) is exactly
what the IL spec mandates (§2.12). This is also how Swift closures work internally. The key
insight is that the closure body is emitted as a normal `MethodDef` whose first parameter is the
capture struct — making it callable both via `CALL` (if the capture struct is known) and
`CALL_INDIRECT` (through a delegate, when the function type is used abstractly).

**Register-based codegen:**
LLVM IR's "unlimited virtual registers + later allocation" model is the inspiration. For an
interpreted VM (not a JIT), no allocation is needed beyond linear numbering. Each expression
result gets the next register. The only constraint is that the register count is declared in
the method header upfront — so the codegen makes one pass to count registers before emitting
the register type table.

---

*Feature research for: Writ Compiler v3.0 — Name Resolution, Type Checking, IL Code Generation*
*Researched: 2026-03-02*
