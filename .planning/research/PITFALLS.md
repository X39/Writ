# Pitfalls Research

**Domain:** Compiler middle-end — name resolution, type checking, and IL codegen for a language with Rust-like strictness, generics with boxing, contract/trait dispatch, closures, entities, dialogue, and cooperative tasks. Added on top of existing lowered AST and working IL runtime.
**Researched:** 2026-03-02
**Confidence:** HIGH — drawn from: rustc developer guide (name resolution, pattern exhaustiveness), Ezyang's "AST Typing Problem" analysis, Thunderseethe's typed IR lowering postmortem, Luau compiler bytecode generation (DeepWiki analysis), CLR virtual dispatch documentation, Writ IL and language spec (all sections), existing Writ codebase structure, and general compiler implementation literature. Claims specific to the Writ architecture are HIGH confidence; claims about general compiler patterns are MEDIUM-HIGH confidence from multiple corroborating sources.

---

## Critical Pitfalls

### Pitfall 1: Single-Pass Name Collection — Mutually Recursive Types and Functions Fail to Resolve

**What goes wrong:**
Name resolution walks the AST top-to-bottom and resolves each name as it is encountered. When `struct A { field: B }` appears before `struct B { ... }`, `B` is unknown at resolution time and the pass emits an "undefined type" error. The same problem occurs for mutually recursive functions (`fn foo() { bar(); }` and `fn bar() { foo(); }` in the same file) and for `impl` blocks that appear before the type they target. In Writ, this is particularly acute: entity declarations reference component types that may be declared in a different order, and `impl Contract for Type` blocks can appear far below the type they implement.

**Why it happens:**
Single-pass resolution is the natural first implementation: walk the tree, add names to scope, look them up. It works for local variables (which are correctly forward-reference-prohibited per §21.2 of the spec) but fails for top-level declarations where the spec explicitly allows any ordering.

**How to avoid:**
Implement a two-phase top-level collection pass before any name resolution. Phase 1 walks all top-level declarations and `impl` blocks, registering their names into the module's type/value namespace without resolving anything. Phase 2 resolves all references using the fully-populated namespaces. Only local variable bindings (inside function bodies) are resolved in a single pass — the spec prohibits local forward references, so single-pass is correct there. This is the standard approach: Rust's `rustc` explicitly separates "collect" (phase 1) from "resolve" (phase 2) for top-level items.

**Warning signs:**
- "Undefined type" errors for types that are defined later in the same file.
- `impl` blocks that appear after the `contract` they implement fail to register.
- Entity declarations that `use Component` where `Component` is defined later in the file error.
- Test files that pass only when types are declared in dependency order.

**Phase to address:** Name resolution — the two-phase architecture must be the first design decision, before writing any lookup logic.

---

### Pitfall 2: Forgetting That `let` Prevents Both Reassignment AND Mutation — Two Distinct Checks

**What goes wrong:**
The spec (§7.1 and memory decision in IL spec) is explicit: `let` binds immutably — the binding cannot be reassigned (`name = "Bob"` on an immutable `let name` is an error) AND the bound value cannot be mutated through that binding (calling `mut self` methods, passing to `mut` parameters, or taking a mutable borrow). Implementing only the reassignment check is a common partial implementation. The mutation check is silently skipped because it requires tracking whether a method call or parameter pass would modify the receiver.

**Why it happens:**
Reassignment checks are syntactically obvious: any `Assign` node with a `let`-bound target is an error. Mutation prevention requires the type checker to know, for every method call and argument pass, whether the callee takes `mut self` / `mut` parameter — which requires the method resolution to be complete before the mutability check runs. If method resolution and mutability checking are interleaved, the dependency is missed.

**How to avoid:**
Implement mutability checking as a separate pass that runs after method resolution is complete. For each call expression `obj.method(args)`, look up the resolved method's `is_mut_self` flag in the method metadata and check whether `obj` is in a mutable binding position. For argument passing, check whether the parameter is `mut` and whether the argument is a mutable binding. The AST already has `AstFnParam::SelfParam { mutable: bool }` and regular params can be marked mutable. Store resolved method references (not just names) in a typed annotation layer so the mutability check pass has direct access to the `is_mut_self` flag.

**Warning signs:**
- Type checker that only rejects `name = "Bob"` (reassignment) but allows `name.mutate()` on a `let` binding.
- No test cases for calling a `mut self` method through an immutable binding.
- Method resolution and mutability enforcement in the same function with no clear separation.
- The type checker passes `let x = new Guard {}; x.damage(10);` without error (damage takes `mut self`).

**Phase to address:** Type checking — specifically, a dedicated mutability analysis pass after method resolution is complete.

---

### Pitfall 3: Resolving `impl` Blocks Without Unifying the Contract Dispatch Table — CALL_VIRT Fails at Runtime

**What goes wrong:**
The type checker verifies that `impl Contract for Type` provides all required method signatures. The IL codegen emits `CALL_VIRT` for contract-dispatched calls. But the runtime's dispatch table maps `(type_tag, contract_id, method_slot) → method_entry`. If the codegen assigns method slots based on the order methods appear in the `impl` block rather than the canonical order defined in the `contract` declaration, the slot numbers diverge. The type checker passes, codegen succeeds, but `CALL_VIRT` calls the wrong method at runtime — the bug is silent (no crash unless the wrong method has incompatible types).

**Why it happens:**
The contract declaration defines the canonical slot ordering. The `impl` block may list methods in a different order, or use a different subset (if the contract has optional methods — though Writ doesn't currently). If the codegen walks the `impl` block's method list to assign slots, the slot numbers are impl-order rather than contract-order. The disconnect only surfaces at runtime when CALL_VIRT uses slot 0 to mean "first method in the contract declaration" but the impl emitted slot 0 for "first method in the impl block."

**How to avoid:**
During codegen of an `impl` block, always look up the method's slot number from the **contract declaration**, not the `impl` declaration. The flow is: (1) the name resolver builds a `ContractDef { methods: IndexMap<name, slot> }` from each contract declaration, with slots assigned in declaration order; (2) during `impl` codegen, for each method, look up `contract.methods[method_name]` to get its canonical slot number; (3) emit the MethodDef with that slot number. If a method in the impl block is not in the contract, it is a type error (caught earlier). This guarantees the slot assignment is always contract-canonical, regardless of impl ordering.

**Warning signs:**
- CALL_VIRT integration test that passes but calls the first method in the contract rather than the semantically correct method.
- Codegen that assigns method slots by iterating `impl_block.members.iter().enumerate()` rather than by contract lookup.
- No test that implements the same contract with methods in different orders and checks CALL_VIRT correctness.
- Any test that verifies `say()` or `choice()` work but never tests that CALL_VIRT dispatches to the semantically-correct method on a non-trivial contract.

**Phase to address:** IL codegen — specifically the `impl` block emission phase, which must reference the contract declaration for slot ordering.

---

### Pitfall 4: Boxing Generic Value Types Without Tracking the Box in Type Annotations — Type Erasure at CALL Boundaries

**What goes wrong:**
The IL spec (§2.2) requires generic value-type arguments to be boxed. When the type checker sees `fn sum<T: Add<T,T>>(a: T, b: T)` called with `int` arguments, it must emit `BOX` instructions to wrap the ints before passing them to the call, and emit `UNBOX` to recover the concrete type on the other side. If the codegen emits `CALL sum` without boxing the `int` arguments, the VM sees a raw int register where it expects a boxed value — the callee reads garbage or crashes. The inverse is also a bug: if the codegen unconditionally boxes even when the callee is monomorphic (no generic parameters), every numeric operation pays a heap allocation cost per call.

**Why it happens:**
The boxing requirement is easy to state but requires the type checker to annotate each call site with "which arguments need boxing" — information that must be threaded from the type checker to the codegen. If the type checker and codegen are implemented as a single pass without an explicit typed annotation layer, the boxing decision is made inconsistently or forgotten entirely. The "no monomorphization" decision in the spec makes this a runtime concern, but the boxed/unboxed distinction must be correct at every call boundary.

**How to avoid:**
Build a typed annotation layer between the type checker and codegen. For each `Call` and `GenericCall` node, the type annotation records: (1) the resolved method, (2) which argument positions require boxing (because the parameter is generic `T`), (3) whether the return value needs unboxing (because the return type is generic `T` but the callee uses it in a non-generic context). The codegen consumes these annotations and emits `BOX`/`UNBOX` instructions at the marked positions. This separates the "where to box" decision (type checker responsibility) from the "how to emit box instructions" decision (codegen responsibility). Implement a test: `fn identity<T>(x: T) -> T` called with an `int` literal — verify the emitted IL has `BOX_INT` before the call and `UNBOX_INT` after.

**Warning signs:**
- Generic function calls that emit `CALL` without any preceding `BOX_*` instruction in the IL output.
- Type checker that knows a parameter is generic `T` but does not mark it for boxing in the call annotation.
- No test for a generic function called with a primitive type that verifies the IL contains box/unbox instructions.
- Tests that call generic functions only with struct arguments (which are already reference types and don't need boxing).

**Phase to address:** Type checking + IL codegen interface — establish the typed annotation layer before writing any codegen for generic function calls.

---

### Pitfall 5: Closure Capture Capture-Mode Mismatch — `let` vs `let mut` Capture Produces Wrong IL

**What goes wrong:**
The spec (§21.2) states: lambdas capture `let` variables by value and `let mut` variables by reference. If the codegen captures all closed-over variables by reference (the easier implementation), `let` bindings appear mutable inside the closure when they should be frozen copies. The inverse (always capture by value) means mutations inside a closure to `let mut` outer variables are silently dropped — the outer variable is unchanged after the closure runs, but no error is reported.

**Why it happens:**
Closure capture analysis is implemented late, after other expression codegen is working. The shortcut of "capture everything by reference (upvalue)" works for all correctness tests that only use `let mut` outer variables in closures. The `let` capture-by-value case is only tested when a test specifically verifies that the outer immutable binding is not changed by the closure — which is rarely the first test written.

**How to avoid:**
During closure analysis in the type checker, classify each captured variable as either `CaptureByValue` (the binding was `let`) or `CaptureByRef` (the binding was `let mut`). Store this classification in the `Lambda` node's annotation. During codegen, emit the appropriate capture instruction per variable: `LOAD_REG` + `CREATE_CLOSURE` with a value slot for by-value captures, and `CAPTURE_REF` for by-reference captures. Refer to Luau's three-way classification (`LCT_VAL`, `LCT_REF`, `LCT_UPVAL`) as a reference implementation. Write two tests: (1) a closure that tries to modify a captured `let` variable — verify this is a type error; (2) a closure that modifies a captured `let mut` variable — verify the outer variable reflects the change after the closure runs.

**Warning signs:**
- Closure implementation that uses a single `Vec<(name, register)>` for all captures without tracking mutability.
- Closures where the by-value vs by-reference distinction is left as a TODO.
- No test that verifies a `let` outer variable cannot be mutated from inside a closure.
- Integration test for closures that only uses `let mut` outer variables.

**Phase to address:** Type checking (capture analysis) and IL codegen (closure emission). Capture classification must be done during type checking, before codegen.

---

### Pitfall 6: Speaker Resolution Ordering — Dialogue Singleton Speakers Looked Up Before Entity Types Are Resolved

**What goes wrong:**
Dialogue lowering (already done in `writ-compiler`) deferred speaker resolution: the lowering pass notes `@OldTim` as a singleton lookup but does not validate that `OldTim` is a real entity type with `[Singleton]` and a `Speaker` component. This validation was explicitly deferred to the name resolution phase (see PROJECT.md: "Singleton speaker assumption for non-param names — Defers entity validation to name resolution phase"). If the name resolution phase does not implement this check, invalid speaker names silently produce runtime crashes when `Entity.getOrCreate<OldTim>()` is called with a type that does not exist or is not a singleton. Worse, if the check is added to name resolution but runs before entity types are collected, every singleton speaker reference is flagged as "unknown entity type."

**Why it happens:**
Speaker resolution validation must run after entity types, component declarations, and `[Singleton]` attributes are all collected. If it runs during the same pass that collects entity types (rather than after), it sees an incomplete namespace. The interleaving of "collect" and "resolve" phases for entity-specific constructs is easy to get wrong.

**How to avoid:**
Speaker resolution validation is a post-collection check, not a collection-time check. After the two-phase declaration collection pass completes and all entity types are registered, run a dialogue-specific validation pass that: (1) checks each singleton speaker reference against the collected entity type namespace, (2) verifies the entity has the `[Singleton]` attribute, (3) verifies the entity has a `Speaker` component slot. This pass runs after the collection phase and before the full type-checking pass. Emit precise errors: "Speaker `OldTim` used in dialogue but no entity named `OldTim` is declared" is better than a generic "undefined identifier."

**Warning signs:**
- No test that uses an `@Speaker` where the entity type is misspelled or non-existent — if no such test exists, the validation was not implemented.
- Speaker validation that runs during the collection pass rather than after it.
- Any dialogue test that hardcodes a valid speaker entity in the same file — the test does not catch cross-file or late-declaration failures.
- Missing check for `[Singleton]` attribute presence — valid non-singleton entity used as a speaker should be a type error.

**Phase to address:** Name resolution — speaker validation is a post-collection semantic check that runs after all entity types are registered.

---

### Pitfall 7: Missing `impl` Completeness Check — Contract with Unimplemented Methods Passes Type Checking

**What goes wrong:**
The type checker verifies that a method in an `impl` block has the correct signature for its contract. But it does not check that all required methods in the contract are present in the `impl` block. A `struct` that implements only half of a contract passes the type checker. At runtime, `CALL_VIRT` for the unimplemented method slot produces a missing-method dispatch error — or worse, dispatches to the wrong method if the slot numbering is dense and another method happens to fill the slot.

**Why it happens:**
The per-method signature check is the natural first implementation: for each method in the `impl`, look up the contract and verify the signature. The inverse check — for each method in the contract, verify the `impl` provides it — requires iterating the contract definition and matching against the `impl`, which is a second query direction that is easy to forget.

**How to avoid:**
After verifying each method's signature, run a completeness check: collect all method names from the contract definition, collect all method names from the `impl` block, and assert the `impl` set is a superset of the contract set. Missing methods are reported as errors with the message "impl of `Contract` for `Type` is missing methods: [list]". This check is the same pattern as Rust's "not all trait items implemented" error. Write a test: a contract with two methods, an `impl` that implements only one — verify the type checker emits an error naming the missing method.

**Warning signs:**
- Type checker that validates `impl` methods one-by-one but has no "contract completeness" check.
- No test for a partial `impl` block missing one required method.
- A test suite that only tests `impl` blocks with all methods present — happy-path only.
- Runtime dispatch errors that say "method not found" on a type that claims to implement a contract.

**Phase to address:** Type checking — contract completeness check is part of `impl` block validation.

---

### Pitfall 8: Component Access Type Ambiguity — Known vs. Generic Entity Reference Produces Different Types

**What goes wrong:**
The spec (§14.3 and §14.7.4) defines two distinct behaviors for `entity[Component]`: when called on a known entity type (component declared in the entity), the result type is `Component` (not optional); when called on a generic `Entity` reference, the result is `Option<Component>`. If the type checker treats both as returning `Option<Component>`, then accessing `guard[Sprite].visible` on a `Guard` entity (which declares `use Sprite`) incorrectly requires an unwrap. If it treats both as `Component`, then accessing `target[Health]` on a generic `Entity` fails to enforce null safety. The wrong treatment in either direction causes spurious type errors or missed null-safety errors.

**Why it happens:**
The distinction requires the type checker to know whether the receiver type is a concrete entity type with a known component set, or a generic `Entity` handle. This requires the type checker to carry entity-type metadata (which components are declared) during component access resolution. The metadata query is non-trivial and easy to skip by defaulting to one of the two cases.

**How to avoid:**
During type checking of `BracketAccess { object, index }` where `index` resolves to a component type: (1) determine the static type of `object`; (2) if `object`'s type is a named entity type and the component is in that entity's declared component set, the result type is `Component` directly; (3) if `object`'s type is the base `Entity` type or a type parameter, the result type is `Option<Component>`. The entity metadata needed for step (2) is available from the declaration collection pass: `EntityDef { component_slots: Vec<ComponentType> }`. Build this metadata structure during the collection pass and query it during component access type-checking.

**Warning signs:**
- Type checker that always returns `Option<Component>` for bracket access on entity types.
- No test that accesses a declared component on a concrete entity type without unwrapping.
- No test that accesses a component on a generic `Entity` and verifies the result requires handling `None`.
- A test that uses `guard[Sprite]!.visible = false` (forced unwrap) on a `Guard` entity — the unwrap should not be necessary.

**Phase to address:** Type checking — component access type resolution, after the entity metadata collection pass.

---

### Pitfall 9: Register Numbering in IL Codegen — Temporaries Clobber Live Values

**What goes wrong:**
The IL calling convention (§2.6) places arguments in consecutive registers starting at a base register in the caller's frame. If the codegen for expression evaluation allocates temporary registers starting from register 0 each time, a temporary used to compute argument N can overwrite a still-live value computed for argument N-1. Example: `foo(bar(), baz())` — if `bar()` result is placed in `r0` and `baz()` evaluation internally allocates a temporary also in `r0`, the result of `bar()` is clobbered before `foo` is called. The IL has no implicit liveness — every register lives until explicitly overwritten.

**Why it happens:**
A naive "always allocate from register 0" scheme works for sequential code where each value is used immediately. It fails for expression trees where multiple values must be live simultaneously. The LIFO register allocation strategy used by Luau (and appropriate for Writ) requires tracking the current "high water mark" register and allocating new temporaries only above it — but this tracking is easy to omit on the first implementation.

**How to avoid:**
Implement a `RegisterAllocator` with `alloc() -> RegId` and `free(RegId)` that maintains a stack-based (LIFO) free list. The "high water mark" is the largest register number ever allocated in the current method body. Temporaries must be freed in LIFO order — if register 5 is allocated, it must be freed before registers 3 and 4 are freed. For expression trees, evaluate sub-expressions from innermost to outermost, holding each result in its allocated register until the parent expression is evaluated. The register allocator's state is reset at the start of each method body. Write a test: a deeply nested binary expression `a + b + c + d` — verify no two live values share a register in the emitted IL.

**Warning signs:**
- Codegen that tracks "current temp register" as a single `u32` counter that resets to 0 for each expression.
- `CALL` instructions where argument registers overlap with temporaries used to compute later arguments.
- Nested expression codegen that passes register 0 to all recursive codegen calls as the output register.
- No test for expressions where multiple intermediate values must be live simultaneously.

**Phase to address:** IL codegen — the register allocator design is the first decision in expression codegen; retrofitting it is a near-complete rewrite of the codegen pass.

---

### Pitfall 10: Entity Construction Codegen — Generating SET_FIELD for Default-Value Fields Unnecessarily

**What goes wrong:**
`new Guard { name: "Steve" }` compiles to: `SPAWN_ENTITY`, `LOAD_STRING "Steve"`, `SET_FIELD name`, `INIT_ENTITY` (per §14.7.5). But for fields that use the entity's default value (`health: int = 80` when construction doesn't override health), the codegen should NOT emit a `SET_FIELD` — the entity's TypeDef metadata already stores the default value, and `SPAWN_ENTITY` applies it. If the codegen walks all entity fields and emits `SET_FIELD` for every field (including those with defaults that were not overridden), the construction sequence is bloated with redundant writes. More critically, if a default value is a non-trivial expression evaluated at construction time (e.g., `patrolRoute: List<vec2> = List::new()`), emitting this as a `SET_FIELD` in every constructor is semantically wrong — the default should be initialized once from the TypeDef metadata, not re-evaluated at every construction site.

**Why it happens:**
Walking all fields and emitting SET_FIELD is the straightforward implementation. Distinguishing "this field was overridden in this `new` expression" from "this field uses the TypeDef default" requires tracking which fields the `AstNewField` list covers — the complement set should produce no IL, not default-initialization IL.

**How to avoid:**
The codegen for `New { ty, fields }` must: (1) emit `SPAWN_ENTITY` with the TypeDef token; (2) for each field in `fields` (the explicit overrides only), emit `LOAD_*` + `SET_FIELD`; (3) emit `INIT_ENTITY`. The TypeDef metadata (stored in the IL module during type declaration emission) carries the default values for fields not listed in the `new` expression. The runtime applies these defaults in `SPAWN_ENTITY`. Never emit `SET_FIELD` for fields absent from the `AstNewField` list.

**Warning signs:**
- Entity construction codegen that iterates the full entity field list rather than the `AstNewField` list.
- `SET_FIELD` instructions in the emitted IL for fields not mentioned in the `new` expression.
- Default expressions (e.g., `List::new()`) being evaluated at every construction site rather than once in the TypeDef metadata.
- No test that constructs an entity with default fields and verifies no spurious `SET_FIELD` instructions appear in the IL.

**Phase to address:** IL codegen — entity construction emission, after the entity metadata pass.

---

### Pitfall 11: Type-Annotated AST Design — Mutating the Existing AST vs. Producing a New Typed IR

**What goes wrong:**
The type checker needs to attach resolved types to every expression node (to inform codegen). Two common approaches: (a) add an `Option<ResolvedType>` field to every `AstExpr` variant and mutate it in-place during type checking; (b) produce a new `TypedExpr` IR that structurally mirrors `AstExpr` but carries non-optional type information. Approach (a) seems cheaper but has a critical flaw: `Option<ResolvedType>` means the type field is nullable, so codegen can reach an untyped node through normal program flow (e.g., error recovery leaves a node untyped). If codegen panics on `None`, the error messages are cryptic. If codegen silently skips, the output IL is incomplete.

**Why it happens:**
Approach (a) is lower-effort initially — no new IR to define, just one field per variant. The problem becomes apparent only when error recovery is implemented: a node that failed type-checking has `None` in the type field, and every subsequent pass must guard against `None`. The resulting code becomes `if let Some(ty) = node.ty { ... } else { /* what? */ }` throughout codegen.

**How to avoid:**
Produce a separate `TypedExpr` / `TypedStmt` IR after type checking completes. Each node in the typed IR carries `ResolvedType` (non-optional) and the resolved metadata (method reference for calls, field index for member access, slot for CALL_VIRT). The typed IR is only produced for declarations that passed type checking — declarations with errors do not produce typed IR, and codegen is skipped for them. This is the approach documented in Ezyang's "AST Typing Problem": "explicitly typed IR doesn't decorate each node with a type, but arranges that the type can be quickly computed using only local information." The fixed cost of defining two data structures is worth the elimination of `Option<Type>` throughout codegen.

**Warning signs:**
- `AstExpr` variants with an `Option<ResolvedType>` field.
- Codegen that uses `.unwrap()` on optional type fields — a future panic site.
- Codegen and type checker in the same pass, updating mutable AST fields.
- Error recovery that sets type fields to `None` and relies on codegen to skip those nodes gracefully.

**Phase to address:** Type checking architecture — define the typed IR before writing any type-checking logic, so the output type is clear from the start.

---

### Pitfall 12: `?` and `!` Postfix Operators — Desugaring Not Wired to Type-Checker Context

**What goes wrong:**
`expr?` (null propagation) and `expr!` (unwrap) are syntactically present in the AST as `UnaryPostfix { op: NullPropagate | Unwrap }`. These are not desugared in the lowering pass (per PROJECT.md: "`?` propagation / `!` unwrap desugaring — spec §18, deferred to type-checker phase"). If the type checker does not implement these desugarings, the codegen sees raw `UnaryPostfix` nodes and has no IL instruction to emit for them. The spec does not define `?` or `!` as primitive IL operations — they must be lowered to `match`/`if let` patterns by the type checker before reaching codegen.

**Why it happens:**
The lowering pass explicitly deferred these to the type-checker phase — the TODO is documented. But when implementing the type checker, the implementer may focus on common expressions and defer postfix operators to "later." Later arrives when a test that uses `entity[Health]!.current` reaches codegen and produces no IL for the `!` operation, silently emitting incorrect code.

**How to avoid:**
In the type checker / typed IR lowering phase, immediately after resolving the type of a `UnaryPostfix` expression, desugar it: `expr!` on `Option<T>` lowers to a `match` with `Some(v) => v, None => crash("unwrap of None at {span}")`. `expr?` on `Option<T>` inside a function returning `Option<R>` lowers to a `match` with `Some(v) => v, None => return None`. These lowerings produce typed IR that codegen knows how to handle. Add both desugarings to the type checker in the first pass — do not defer again.

**Warning signs:**
- Typed IR that still contains `UnaryPostfix { op: NullPropagate }` nodes without desugaring.
- Codegen that has a TODO comment or panics for `UnaryPostfix::Unwrap` or `UnaryPostfix::NullPropagate`.
- Tests that avoid `?` and `!` operators entirely — the omission masks the missing desugaring.
- A test suite that passes but all test expressions use `if let Option::Some(x) = ...` instead of `x!` or `x?`.

**Phase to address:** Type checking — `?` and `!` desugaring must be implemented as part of the type checker's expression lowering, not deferred further.

---

### Pitfall 13: `spawn` Codegen — Task Register Type Is Unresolved at the Spawn Site

**What goes wrong:**
`let task = spawn moveBoulder(vec2 { x: 10.0, y: 5.0 })` in a dialogue block must lower to a `SPAWN_TASK` IL instruction that produces a task handle in a register. The type of `task` is a task handle — not the return type of `moveBoulder`. If the type checker infers the type of `spawn expr` as the return type of the spawned function (e.g., `void` for `moveBoulder`), then `join task` and `cancel task` see a `void`-typed register as their argument, not a task handle, and the codegen emits incorrect or missing instructions. The same applies to `spawn detached`, where the handle may be discarded.

**Why it happens:**
`spawn expr` evaluates `expr` as a function call and launches it as a task — the "type" of the `spawn` expression is not the function's return type but a `TaskHandle` (an opaque runtime type). This is a semantic distinction that the type checker must encode explicitly. Naive type inference would propagate the return type of the called function as the type of the `spawn` expression, which is wrong.

**How to avoid:**
In the type system, define `TaskHandle` as a distinct opaque type (mapping to the runtime's task handle representation). The type checker rule for `Spawn { expr }` is: resolve the type of `expr` as a function call (for arity checking), but the type of the `Spawn` expression itself is always `TaskHandle`. `SpawnDetached { expr }` returns `TaskHandle` as well. `Join { expr }` requires `expr: TaskHandle` and returns `void`. `Cancel { expr }` requires `expr: TaskHandle` and returns `void`. These rules are a short, enumerable list — implement them completely in one pass.

**Warning signs:**
- The type of `spawn f()` inferred as the return type of `f` instead of `TaskHandle`.
- `join` or `cancel` expressions that accept any type for their argument without checking `TaskHandle`.
- No integration test for `spawn` + `join` + `cancel` that verifies the emitted IL includes `SPAWN_TASK`, `TASK_JOIN`, `TASK_CANCEL` instructions.
- Codegen for `spawn` that emits `CALL` instead of `SPAWN_TASK`.

**Phase to address:** Type checking — `spawn`/`join`/`cancel` type rules, implemented as part of the concurrency expression type-checking pass.

---

### Pitfall 14: Lifecycle Hook Codegen — Hooks Not Registered as TypeDef Metadata, Only Emitted as Methods

**What goes wrong:**
Entity lifecycle hooks (`on create`, `on destroy`, `on interact`, `on finalize`, `on serialize`, `on deserialize`) lower to methods named `__on_create`, etc. (per §14.7.3). But for the runtime to fire them correctly, the TypeDef metadata must register each hook by its hook type (e.g., `lifecycle_hooks: { OnCreate => method_token }` in the TypeDef). If the codegen emits the hook methods as regular MethodDefs but does not register them in the TypeDef's hook metadata, the runtime never knows `__on_create` should be called by `INIT_ENTITY`. The hook method body exists in the module but is dead code.

**Why it happens:**
Emitting the method body is the visible step — it produces IL instructions. Registering the method token in the TypeDef metadata is a separate, invisible step that is easy to forget. The existing runtime code (PROJECT.MD known debt: "Lifecycle hook dispatch — infrastructure ready but method name lookup not wired") already documents this gap.

**How to avoid:**
Entity codegen must have two distinct outputs for each lifecycle hook: (1) emit the method body as a `MethodDef` in the module; (2) update the entity's `TypeDef` metadata to register the method token under the appropriate hook slot. The `ModuleBuilder` API (already built in v2.0) must expose a way to set lifecycle hook method tokens on a TypeDef. Verify with a test: compile an entity with `on create { }`, run the IL through the runtime, and assert `INIT_ENTITY` fires the `__on_create` method body.

**Warning signs:**
- Entity hook methods present in the module's MethodDef table but not referenced in the TypeDef's hook metadata.
- INIT_ENTITY that does nothing (no `on_create` body fires) despite the entity having an `on create` hook.
- Codegen that iterates `entity.hooks` and emits method bodies but does not call any `set_lifecycle_hook` on the ModuleBuilder.
- The known tech debt item "lifecycle hook dispatch not wired" — this is the codegen side that makes the wiring possible.

**Phase to address:** IL codegen — entity TypeDef emission, which must happen before any lifecycle hook method emission so the TypeDef token is available for hook registration.

---

### Pitfall 15: Localization Key Collision in Codegen — FNV-1a Keys Not Verified for Uniqueness Across the Full Compile Unit

**What goes wrong:**
The lowering pass generates FNV-1a localization keys per dialogue line and detects collisions within a single `dlg` block. But when multiple `dlg` blocks across multiple files are compiled together into one IL module, the same FNV-1a hash might appear in two different dialogue functions — a cross-file collision. The runtime string table uses these keys as indices; a collision means one line's localized text is returned for a different line's key. The failure mode is a displayed dialogue line showing the wrong translated text — a correctness bug that is invisible in English (default locale) but breaks all localized builds.

**Why it happens:**
The lowering pass's collision detection is file-scoped or `dlg`-block-scoped (the detection already exists per PROJECT.md). Cross-file collision detection requires a global accumulator across all files in the compile unit, which is a different data structure than the per-block or per-file accumulator.

**How to avoid:**
During IL module emission, maintain a module-wide localization key registry: `HashMap<key_string, (dlg_function_name, line_span)>`. For each `say_localized` or `say` emission, check the key against this registry. If a collision is found, use the `#key` override mechanism to emit a disambiguation prefix (or report a compiler error requiring the author to add a `#key`). The manual `#key` mechanism exists precisely to prevent fragile auto-generated hash instability — prefer reporting an error over silent disambiguation.

**Warning signs:**
- No cross-file localization key deduplication in the module builder.
- Localization test suite that only tests single-file compilations.
- A compile-unit-level collision that silently uses whichever key was registered last.
- No test for two `dlg` functions in different files with identical default-locale text strings.

**Phase to address:** IL codegen — the module-level locale table emission pass.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Single-pass name resolution (no pre-collection) | Simpler initial implementation | Mutually recursive types, cross-file `impl` blocks, and forward entity references all fail with false errors | Never — pre-collection adds one tree walk and enables all subsequent passes to work correctly |
| Optional type fields on existing `AstExpr` (`Option<ResolvedType>`) | No new IR to define | Codegen must guard every type access; error recovery leaves `None` traps; panics or silent wrong code in downstream passes | Never in production; acceptable for a one-file prototype only |
| Deriving CALL_VIRT slot from `impl` method order instead of contract declaration order | No need to look up contract during codegen | Wrong method dispatched at runtime; bug is silent until a test covers the out-of-order case | Never |
| Skipping `let`-mutation check (only checking reassignment) | Faster type checker implementation | `let`-bound objects mutated through method calls; spec compliance broken; diagnostic quality degrades | Never |
| Boxing generics at all call sites regardless of monomorphism | Simpler boxing logic (always box `T` positions) | Unnecessary heap allocations for concrete-type calls to generic functions; performance issue in hot paths | Acceptable for initial implementation; optimize after correctness is established |
| Emitting lifecycle hook methods without registering in TypeDef metadata | Codegen is simpler (emit methods only) | Hooks are dead code; runtime never fires them; all entity lifecycle tests pass but test the wrong behavior | Never |
| Skipping completeness check for `impl` blocks (only checking signature of methods present) | Faster type checker | Partial `impl` compiles; runtime dispatch for missing methods crashes or misbehaves | Never |
| Not implementing `?` / `!` desugaring in type checker, leaving `UnaryPostfix` nodes in typed IR | Deferred work | Codegen has no IL for null propagation / unwrap; any use of these operators produces incomplete or crashing IL | Never — these are core language features used in entities, error handling, and optional component access |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `writ-compiler` AST → typed IR | Building the typed IR in the same `writ-compiler` crate by mutating `AstExpr` nodes | Define a `TypedExpr` / `TypedDecl` IR in a new module within `writ-compiler`, separate from the `ast` module. The typed IR borrows no data from the `AstExpr` — it is a fresh tree. |
| `writ-compiler` typed IR → `writ-module` `ModuleBuilder` | Using `ModuleBuilder` directly in the type checker to emit metadata as types are resolved | Separate the type analysis pass (produces typed IR + metadata tables) from the codegen pass (consumes typed IR + metadata tables, drives `ModuleBuilder`). Interleaving analysis and emission makes testing either in isolation impossible. |
| `impl` contract method slot ordering vs. runtime dispatch table | Codegen assigns slots in `impl` traversal order; runtime dispatch table built from contract declaration order | The contract declaration is the source of truth for slot numbers. Every `impl` codegen operation must look up the slot from the contract declaration, never from the `impl` block's own traversal order. |
| Entity `AstEntityDecl` with hooks vs. IL TypeDef metadata | Emitting hooks as MethodDefs but forgetting to wire them to the TypeDef via `set_lifecycle_hook` | Entity codegen must proceed in two phases: (1) emit the TypeDef shell (registers the entity type); (2) emit hook MethodDefs and update the TypeDef's hook slots. The TypeDef token must exist before any hook method can reference it. |
| Localization key FNV-1a collisions across compile units | Per-file or per-dlg collision detection that misses cross-file collisions | The module-level locale key registry must be a single accumulator shared across all files in the compile unit. It must be populated and checked during the module emission pass, not during per-file lowering. |
| Snapshot tests for typed IR / codegen | Snapshotting the raw IL binary output — any register renaming causes all snapshots to fail | Snapshot a human-readable structured dump: typed IR tree with resolved types, plus the disassembled IL text output. The disassembler (already built in v2.0) should be used for IL snapshots. |
| `spawn` task handle types | Codegen emits `CALL` for `spawn expr` because the type checker propagated the callee's return type | `spawn expr` always produces a `TaskHandle` register regardless of the callee's return type. The type checker must special-case `Spawn` and `SpawnDetached` nodes before they reach the general call resolution logic. |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Quadratic symbol lookup in nested scopes | Name resolution slows dramatically for files with deeply nested scopes or many `using` imports | Use a `HashMap`-based scope chain, not a `Vec` that is linearly scanned | Visible at ~50 nested scopes or ~500 imports in a single file |
| Re-resolving type references on every type-check call | Type checking slows linearly with the number of type annotations in a large program | Memoize type resolution: resolve each `AstType` once, store the resolved `TypeId`, reuse it | Large entity declarations with many fields — common in game projects |
| Building a fresh `HashMap` for the contract dispatch table for every `impl` block | Each `impl` build is fast; many `impl` blocks per entity cause quadratic table construction | Build the contract dispatch table once per type after all `impl` blocks are collected, not incrementally per `impl` | Programs with entities implementing 5+ contracts each |
| Walking the full entity field list for every `new` expression to determine defaults | Linear in the number of fields for each construction expression | Cache the entity's default field set in the entity metadata during collection; look up in O(1) during codegen | Entities with 10+ fields constructed many times |
| Cloning the entire scope chain on every block entry for shadowing analysis | Memory pressure in functions with many nested blocks | Use a persistent/functional scope structure or a stack of `HashMap`s with push/pop — do not clone | Functions with deeply nested `if`/`match`/`while` blocks (common in complex dialogue) |

---

## "Looks Done But Isn't" Checklist

- [ ] **Name resolution:** A type referenced before its declaration in the same file resolves correctly — verify with a test where `struct B { field: A }` appears before `struct A { ... }`.
- [ ] **Name resolution:** `impl Contract for Type` blocks register in the dispatch table even when the `impl` appears after the `struct` declaration — verify with a reversed-order test file.
- [ ] **Name resolution:** Speaker `@Name` in a dialogue that references an entity not marked `[Singleton]` is a compile error — verify the error is emitted.
- [ ] **Mutability check:** Calling a `mut self` method on a `let`-bound variable is a compile error — verify `let x = new Guard {}; x.damage(10);` fails type checking.
- [ ] **Mutability check:** Calling an immutable `self` method on a `let`-bound variable succeeds — verify `let x = new Guard {}; x.greet();` passes.
- [ ] **Contract impl:** A partial `impl` block missing one required method is a compile error — verify with a two-method contract where only one is implemented.
- [ ] **CALL_VIRT slot ordering:** `impl` methods in reverse declaration order dispatch correctly — verify a contract with three methods implemented in reverse order.
- [ ] **Boxing:** A generic function called with `int` arguments emits `BOX_INT` before the call — verify in the IL output.
- [ ] **Closure capture:** A `let` outer variable captured in a lambda cannot be mutated inside the lambda — verify this is a type error.
- [ ] **Closure capture:** A `let mut` outer variable captured in a lambda reflects mutations in the outer scope after the lambda runs — verify by running the lambda and checking the outer binding.
- [ ] **`?` / `!` desugaring:** `expr!` on `Option<T>` lowers to a match; codegen does not see raw `UnaryPostfix::Unwrap` nodes — verify the typed IR contains no `UnaryPostfix` nodes.
- [ ] **`spawn`:** The type of `spawn f()` is `TaskHandle`, not the return type of `f` — verify the type checker annotates `Spawn` nodes with `TaskHandle`.
- [ ] **Entity construction:** `new Guard {}` with all-default fields emits exactly `SPAWN_ENTITY, INIT_ENTITY` with no `SET_FIELD` instructions — verify the IL output.
- [ ] **Entity construction:** `new Guard { name: "Steve" }` emits exactly `SPAWN_ENTITY, LOAD_STRING, SET_FIELD name, INIT_ENTITY` — verify the field override is the only SET_FIELD.
- [ ] **Lifecycle hooks:** `on create` body is wired to the TypeDef's hook metadata — verify INIT_ENTITY fires the hook body by running the IL through the runtime.
- [ ] **Component access:** `guard[Sprite].visible` on a concrete `Guard` entity (Sprite declared) type-checks without requiring unwrap — verify this passes without `!`.
- [ ] **Component access:** `target[Health]` on a generic `Entity` reference type-checks as `Option<Health>` — verify this requires handling `None`.
- [ ] **Localization:** Two `dlg` blocks in different files with identical text produce different localization keys or a compile error — verify no silent collision.
- [ ] **Register allocation:** Expression `a + (b * (c + d))` allocates no register twice while all operands are live — verify by inspecting the IL register assignments.

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Single-pass name resolution discovered after type checker is written | MEDIUM — add pre-collection pass without rewriting lookup | Add a `collect_declarations(decls) -> NamespaceMap` function that runs before the existing resolve pass; thread the map into all lookup calls; re-run tests |
| `Option<ResolvedType>` fields already added to `AstExpr` | HIGH — requires new typed IR definition | Define `TypedExpr` IR; rewrite type checker to produce it rather than mutate `AstExpr`; rewrite codegen to consume it; the AST remains unchanged |
| Wrong CALL_VIRT slot ordering discovered at runtime | MEDIUM — fix slot assignment in `impl` codegen | Add `contract_slot_for(method_name)` lookup to all `impl` emission; re-run all CALL_VIRT integration tests |
| Boxing missing at generic call boundaries | MEDIUM — add boxing pass or annotation layer | Add a "boxing analysis" step to the type checker that annotates each call's generic argument positions; add BOX/UNBOX emission to codegen |
| Lifecycle hook methods emitted but not registered | LOW — one additional `set_lifecycle_hook` call per hook | Find all entity hook method emission sites; add the TypeDef metadata update after each one; add an integration test |
| `?` / `!` desugaring missing from type checker | MEDIUM — desugar in the typed IR production pass | Add `UnaryPostfix::Unwrap → TypedExpr::Match { ... crash arm }` and `UnaryPostfix::NullPropagate → TypedExpr::Match { ... early return arm }` rules to the type checker; re-run all tests using `!` and `?` |
| Speaker resolution not validating entity types | LOW — add post-collection validation pass | Add `validate_dialogue_speakers(typed_decls, entity_map)` pass after collection; emit `UnknownSpeaker` errors for unresolved names |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Single-pass name resolution | Name resolution — declaration collection sub-pass | Test: `struct B { field: A }` before `struct A` in same file resolves correctly |
| `let` mutation not checked | Type checking — mutability analysis pass | Test: `let x = new Guard {}; x.damage(10)` fails with "cannot call `mut self` through immutable binding" |
| CALL_VIRT slot ordering mismatch | IL codegen — `impl` block method slot assignment | Test: contract with 3 methods, `impl` with methods in reverse order — CALL_VIRT dispatches correctly |
| Boxing absent at generic boundaries | Type checking — generic call annotation; IL codegen — BOX emission | Test: generic function called with `int` — emitted IL contains `BOX_INT` |
| Closure capture mode mismatch | Type checking — capture analysis; IL codegen — closure emission | Test: closure captures `let` var — mutation inside lambda is a type error |
| Speaker resolution validation missing | Name resolution — post-collection dialogue check | Test: `@NonExistentEntity` in dialogue block — compiler emits `UnknownSpeaker` error |
| Contract impl completeness | Type checking — `impl` block validation | Test: partial `impl` (one method missing) — compiler emits "missing method" error |
| Component access type ambiguity | Type checking — `BracketAccess` expression resolution | Test: concrete entity `guard[Sprite]` → `Sprite` (no optional); generic `entity[Health]` → `Option<Health>` |
| Register clobber in codegen | IL codegen — register allocator design | Test: nested binary expression — no register assigned to two live values simultaneously |
| Entity construction default fields | IL codegen — `New` expression emission | Test: `new Guard {}` → emits `SPAWN_ENTITY, INIT_ENTITY` only; no `SET_FIELD` for default-value fields |
| Typed AST design (`Option<Type>`) | Type checking architecture — typed IR decision | Code review: no `Option<ResolvedType>` in typed IR; all type fields are non-optional |
| `?` / `!` not desugared | Type checking — `UnaryPostfix` handling | Test: `expr!` on `Option<T>` → typed IR contains `Match` node, not `UnaryPostfix` |
| `spawn` produces wrong type | Type checking — concurrency expression rules | Test: `let h = spawn f()` → type of `h` is `TaskHandle`; `join h` and `cancel h` type-check |
| Lifecycle hooks not registered in TypeDef | IL codegen — entity TypeDef emission | Integration test: entity with `on create` — INIT_ENTITY fires hook body in runtime |
| Localization key cross-file collision | IL codegen — module-level locale key registry | Test: two `dlg` blocks in different files with identical text — no silent collision |

---

## Sources

- [Name resolution — Rust Compiler Development Guide](https://rustc-dev-guide.rust-lang.org/name-resolution.html) — two-phase collection + resolution, rib-based scoping, forward reference handling (HIGH confidence — authoritative rustc source)
- [Pattern and Exhaustiveness Checking — Rust Compiler Development Guide](https://rustc-dev-guide.rust-lang.org/pat-exhaustive-checking.html) — match arm reachability, redundant arm detection (HIGH confidence — authoritative rustc source)
- [The AST Typing Problem — Edward Z. Yang (2013)](https://blog.ezyang.com/2013/05/the-ast-typing-problem/) — approaches to decorating AST with type information, optional-field pitfalls, explicitly-typed IR advantages (MEDIUM confidence — widely referenced in PL community)
- [Lowering Our AST to Escape the Typechecker — Thunderseethe's Devlog](https://thunderseethe.dev/posts/lowering-base-ir/) — alpha equivalence, typed IR benefits, typed IR vs. decorated AST tradeoffs (MEDIUM confidence — practical implementation postmortem)
- [Luau Bytecode Generation — DeepWiki](https://deepwiki.com/luau-lang/luau/4.1-bytecode-generation) — LIFO register allocation (RegScope), three-way closure capture classification (LCT_VAL/LCT_REF/LCT_UPVAL), closure sharing optimization (MEDIUM confidence — derived analysis of production code)
- [How the CLR Dispatches Virtual Method Calls — codestudy.net](https://www.codestudy.net/blog/clr-implementation-of-virtual-method-calls-to-interface-members/) — method slot ordering, vtable layout, dispatch correctness (MEDIUM confidence — CLR implementation reference)
- [Interface Dispatch — Lukas Atkinson (2018)](https://lukasatkinson.de/2018/interface-dispatch/) — interface dispatch mechanisms, slot-based dispatch table, hash-based fallback (MEDIUM confidence — well-cited reference)
- [Lowering Rust Traits to Logic — Nicholas Matsakis (2017)](https://smallcultfollowing.com/babysteps/blog/2017/01/26/lowering-rust-traits-to-logic/) — trait coherence, contract completeness checking, solver design principles (MEDIUM confidence — rustc core contributor postmortem)
- [Two-Phase Borrows — Rust Compiler Development Guide](https://rustc-dev-guide.rust-lang.org/borrow_check/two_phase_borrows.html) — mutability analysis, borrow checking phases, separate analysis from enforcement (HIGH confidence — authoritative rustc source)
- Writ Language Specification §5 (type system), §7 (variables / `let` strictness), §11 (generics), §13 (dialogue / speaker resolution), §14 (entities / lifecycle hooks), §21 (scoping rules) — (HIGH confidence — authoritative spec)
- Writ IL Specification §2.2 (typed IL, generic boxing), §2.6 (calling convention, register layout), §2.10 (self parameter), §2.16.7 (entity construction protocol) — (HIGH confidence — authoritative spec)
- Writ PROJECT.md — known tech debt: lifecycle hook dispatch not wired, singleton speaker assumption deferred, `?`/`!` desugaring deferred — (HIGH confidence — project source of truth)

---
*Pitfalls research for: Writ compiler middle-end — name resolution, type checking, IL codegen (v3.0 milestone)*
*Researched: 2026-03-02*
