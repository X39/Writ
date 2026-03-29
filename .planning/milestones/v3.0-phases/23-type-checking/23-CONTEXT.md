# Phase 23: Type Checking - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Every expression node in the program carries a fully resolved, non-optional type. All type rules from the Writ spec are enforced with precise, actionable errors. Input: `NameResolvedAst` (DefMap + Vec<ResolvedDecl>). Output: `TypedAst` where no expression has `Option<Ty>`. This phase does NOT emit IL — that's Phases 24-25.

</domain>

<decisions>
## Implementation Decisions

### Typed IR shape
- **New IR types** — fresh `TypedExpr`, `TypedStmt`, `TypedDecl` enums, completely separate from the existing AST and resolve IR. Clean `NameResolvedAst -> TypedAst` pipeline. Codegen only sees typed nodes.
- **Inline type field** — every `TypedExpr` variant carries `ty: Ty, span: Span` directly (no wrapper struct, no side-table)
- **Interned `Ty` via arena** — `Ty` is a `Copy` ID (`Ty(u32)`) into a `TyInterner` with `TyKind` enum. Enables cheap passing, structural equality, deduplication. `TyKind` variants: `Int`, `Float`, `Bool`, `String`, `Void`, `Struct(DefId)`, `Entity(DefId)`, `Enum(DefId)`, `Array(Ty)`, `Func{params, ret}`, `Option(Ty)`, `Result(Ty, Ty)`, `TaskHandle(Ty)`, `GenericParam(u32)`, `Infer(InferVar)`, `Error` (poison)
- **`?` and `!` desugared** — `UnaryPostfix` nodes are desugared to typed `Match` nodes in the Typed IR. No raw `?`/`!` operator nodes survive into the output. Codegen only sees match patterns.

### Inference & unification
- **Local inference + bidirectional lambda context** — local variable types inferred from initializer (spec §5.2). Lambda parameter/return types inferred from expected-type context (function parameter, typed variable, contract method) per spec §12.4.2. No cross-function inference. No implicit conversion at assignment/argument boundaries (spec §10.2: `Into<T>` requires explicit `.into<T>()` call).
- **Generic type argument inference** — at call sites, type args omitted when inferable from arguments (spec §11.2: `first(inventory)` infers `T` from `List<Item>`). Inference approach (ena union-find vs substitution map) at Claude's discretion.
- **Contract bound checking** — approach (eager at call site vs deferred) at Claude's discretion. Must report unsatisfied bounds with the bound named (success criterion 2).
- **Closure capture classification during type checking** — `let` bindings captured by value, `let mut` bindings captured by reference (spec §12.4.4). Captures annotated in `TypedExpr::Lambda` with `Capture { name, ty, mode: ByValue|ByRef }`. Codegen reads annotations directly.

### Error strategy
- **Collect all errors, poison on error** — continue checking after errors using `Ty::Error` as poison type. Errors involving poison types are suppressed (no cascading). Report all independent type errors at once.
- **Multi-span with context** — errors point to BOTH the error site AND the relevant declaration (e.g., "expected int (from function signature at line 5), got string (at line 12)"). Matches success criteria for precise, actionable errors.
- **Actionable suggestions** — include `help:` hints where possible: missing contract impl suggestions (TYPE-19), `Into<T>` conversion hints, missing `mut` suggestions, similar name suggestions. Follows the pattern established by name resolution's "did you mean?" suggestions.
- **Error code numbering** — at Claude's discretion (separate range vs continuing sequence)

### Mutability model
- **Method-signature based detection** — methods with `mut self` are mutations. Calling a `mut self` method on a `let` binding is an error. Covers: field assignment, `mut self` method calls, passing as `mut` parameter.
- **Root-binding propagation** — `let mut x` makes ALL field chains through `x` mutable; `let x` makes them ALL immutable. No per-field mutability. Applies uniformly to struct fields AND component fields (`guard.Health.current`).
- **Enforced through function value aliases** — storing a `mut self` method as a function value preserves the mutability requirement. Calling it on an immutable binding is still an error. Keeps mutability sound.
- **Arrays follow the same rule** — mutating methods (`push`, `pop`, `insert`, `remove`, index assignment) require `let mut` binding. Read operations (`length`, indexing, iteration) are fine on `let`.
- **For-loop variables immutable by default** — `for item in items` makes `item` immutable. `for mut item in items` is required to mutate. Source collection must also be `mut` for `mut item` iteration.
- **Dual-span error presentation** — mutability errors show BOTH the binding declaration site and the violation site (success criterion 3)

### Claude's Discretion
- Unification algorithm choice (ena union-find vs substitution map)
- Contract bound checking timing (eager vs deferred)
- Error code numbering scheme
- Exact `TyInterner` implementation details
- Performance optimizations in the type checking pass

</decisions>

<specifics>
## Specific Ideas

- Spec needs clarification: mutability enforcement through function value aliases (method references carry `mut self` requirement)
- Spec needs clarification: `for mut item in items` semantics — `mut` required to modify loop variable, and `items` must be `mut` too
- `Into<T>` always explicit with `<T>` type parameter at call site (spec §10.2) — only exception is string interpolation (implicit `.into<string>()`)
- Error presentation modeled after Rust's compiler diagnostics style (multi-span, help hints, error codes)

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `NameResolvedAst` + `DefMap` (`writ-compiler/src/resolve/ir.rs`): arena-backed symbol table with `DefId`, FQN lookup, generics — direct input to type checker
- `ResolvedType` enum: already has `Primitive`, `Named{def_id, type_args}`, `Array`, `Func`, `GenericParam`, `PreludeType`, `PreludeContract`, `Error` — maps cleanly to `TyKind`
- `DefEntry` + `DefKind` (`writ-compiler/src/resolve/def_map.rs`): carries `kind`, `vis`, `generics`, `name_span`, `file_id` — needed for method resolution and error reporting
- `writ-diagnostics` crate (`writ-diagnostics/src/`): `Diagnostic`, `FileId`, error rendering infrastructure — reuse for type errors
- `AstType` enum (`writ-compiler/src/ast/types.rs`): lowered type representation with spans — bridge between AST and resolve types

### Established Patterns
- Arena-based ID allocation (`id_arena::Arena`) for `DefId` — same pattern for `Ty` interning
- `FxHashMap` for fast lookups throughout resolve module
- Error collection pattern: `Vec<Diagnostic>` passed through all passes, accumulated, never early-exit
- Module structure: `writ-compiler/src/resolve/` has `mod.rs`, `def_map.rs`, `collector.rs`, `resolver.rs`, `scope.rs`, `ir.rs`, `error.rs`, `validate.rs`, `suggest.rs` — type checker should follow similar organization (`check/mod.rs`, `check/infer.rs`, `check/unify.rs`, etc.)

### Integration Points
- Type checker module: `writ-compiler/src/check/` (new module, add to `lib.rs`)
- Entry point: consumes `NameResolvedAst`, produces `TypedAst`
- Downstream: Phase 24 (metadata emission) and Phase 25 (method body codegen) consume `TypedAst`
- Spec reference files: `language-spec/spec/06_5_type_system.md`, `12_11_generics.md`, `08_7_variables_constants.md`, `11_10_contracts.md`

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 23-type-checking*
*Context gathered: 2026-03-02*
