# Architecture Research

**Domain:** Compiler frontend — name resolution, type checking, IL codegen for the Writ language
**Researched:** 2026-03-02
**Confidence:** HIGH

## Standard Architecture

### System Overview

```
+============================================================================+
|  Existing Crates (mostly unchanged)                                        |
|                                                                            |
|  writ-parser           writ-compiler                                       |
|  logos lexer           LoweringContext, lower()                            |
|  chumsky CST           multi-pass CST → AST                                |
+====================================+=======================================+
                                     |
                              Ast { items: Vec<AstDecl> }
                                     |
                                     v
+====================================+=======================================+
|  NEW phases inside writ-compiler   (same crate, new modules)              |
|                                                                            |
|  +--------------------+   +--------------------+   +------------------+  |
|  |  resolve/          |   |  typecheck/         |   |  codegen/        |  |
|  |  name resolution   | → |  type inference +   | → |  IL codegen      |  |
|  |  NameResolved (IR) |   |  checking           |   |  → ModuleBuilder |  |
|  |                    |   |  Typed (IR)         |   |                  |  |
|  +--------------------+   +--------------------+   +--------+---------+  |
+================================================================|===========+
                                                                 |
                                                          writ_module::Module
                                                                 |
+================================================================|===========+
|  Existing Crates (unchanged)                                   |           |
|                                                                v           |
|  writ-module                    writ-runtime                              |
|  Module / ModuleBuilder         VM + scheduler + entities + GC            |
|                                                                            |
|  writ-assembler                 writ-cli                                   |
|  text IL → Module               `writ` binary (gains `compile` subcommand)|
+============================================================================+
```

**Decision: All new phases live inside `writ-compiler` as additional modules.**

Rationale: Name resolution and type checking have a tight feedback loop with each other and with the AST types that already live in `writ-compiler`. Introducing a new crate (`writ-resolver` or `writ-typeck`) creates a circular dependency risk: the resolver needs AST types from `writ-compiler`, but `writ-compiler` could also want resolver output. Keeping all three phases inside `writ-compiler` keeps the dependency graph acyclic and the `Ast` → `NameResolved` → `Typed` → `Module` pipeline linear. This matches how Rust's own `rustc_resolve` lives in the same logical compilation unit as the AST lowering passes. The assembler precedent (all assembler phases in one crate) also validates this approach.

`writ-module` gains no new code — it is already the correct interface for codegen output.
`writ-cli` gains a `compile` subcommand that calls `writ-compiler`'s new public `compile()` function.

### Component Responsibilities

| Component | Responsibility | Communicates With |
|-----------|----------------|-------------------|
| `writ-compiler::lower` | Existing: CST → `Ast` (desugaring, lowering) | Consumes `writ-parser::cst`; produces `writ-compiler::ast::Ast` |
| `writ-compiler::resolve` | NEW: `Ast` → `NameResolved` — binds all identifier/path occurrences to their declaring `DefId`, builds per-scope `SymbolTable`, handles `using` imports, detects undefined names and ambiguities | Consumes `Ast`; produces `NameResolved` IR + `DefMap` |
| `writ-compiler::typecheck` | NEW: `NameResolved` → `Typed` — infers and checks all expression types, validates contract impls, checks mutability, resolves overloaded operators | Consumes `NameResolved` + `DefMap`; produces `Typed` IR |
| `writ-compiler::codegen` | NEW: `Typed` → `Module` — walks typed IR, emits typed IL instructions via `ModuleBuilder` | Consumes `Typed` IR + `DefMap`; calls `writ_module::ModuleBuilder`; returns `Module` |
| `writ-module::ModuleBuilder` | Existing: programmatic IL module construction API | Used exclusively by codegen phase |
| `writ-cli` | Gains `compile` subcommand | Calls `writ_compiler::compile()`; writes output `.writil` |

## Recommended Project Structure

```
writ-compiler/
├── Cargo.toml                     (add writ-module dependency)
└── src/
    ├── lib.rs                     (add pub mod resolve, typecheck, codegen; pub fn compile())
    ├── ast/                       (existing — unchanged)
    │   ├── mod.rs
    │   ├── decl.rs
    │   ├── expr.rs
    │   ├── stmt.rs
    │   └── types.rs
    ├── lower/                     (existing — unchanged)
    │   ├── mod.rs
    │   ├── context.rs
    │   ├── error.rs
    │   └── ...
    ├── resolve/                   (NEW)
    │   ├── mod.rs                 (pub fn resolve(ast: Ast) -> (NameResolved, Vec<ResolveError>))
    │   ├── def.rs                 (DefId, DefKind, DefMap: DefId → AstDecl location)
    │   ├── scope.rs               (Scope, SymbolTable: name → DefId, rib stack for block scoping)
    │   ├── namespace.rs           (NamespaceMap: namespace path → Set<DefId>; using resolution)
    │   ├── ir.rs                  (NameResolved IR: mirrors Ast but Ident/Path replaced by DefId refs)
    │   └── error.rs               (ResolveError: UndefinedName, AmbiguousName, PrivateName, ...)
    ├── typecheck/                 (NEW)
    │   ├── mod.rs                 (pub fn typecheck(ir: NameResolved, defs: &DefMap) -> (Typed, Vec<TypeError>))
    │   ├── ty.rs                  (Ty enum: Int, Float, Bool, String, Named(DefId), Generic(DefId, Vec<Ty>), ...)
    │   ├── infer.rs               (InferCtx: type variable unification, constraint solving)
    │   ├── env.rs                 (TypeEnv: local variable → Ty map, threaded through function bodies)
    │   ├── check.rs               (check_expr, check_stmt, check_decl — top-level type walk)
    │   ├── coerce.rs              (implicit coercions: int literal → float, Option wrapping)
    │   ├── contract.rs            (contract impl validation: all methods present, types match)
    │   ├── ir.rs                  (Typed IR: mirrors NameResolved but every expr/stmt carries Ty)
    │   └── error.rs               (TypeError: TypeMismatch, UnresolvedType, MissingImpl, ...)
    └── codegen/                   (NEW)
        ├── mod.rs                 (pub fn codegen(ir: Typed, defs: &DefMap) -> Result<Module, CodegenError>)
        ├── context.rs             (CodegenCtx: ModuleBuilder + token maps + register allocator)
        ├── type_sig.rs            (Ty → TypeRef blob encoding for ModuleBuilder)
        ├── decl.rs                (emit_fn, emit_struct, emit_enum, emit_entity, emit_impl, ...)
        ├── expr.rs                (emit_expr → register allocation + instruction emission)
        ├── stmt.rs                (emit_stmt: let, for, while, return, atomic, ...)
        ├── register.rs            (LinearRegisterAllocator: assigns u16 register indices)
        ├── label.rs               (LabelMap: label name → byte offset; forward ref patching)
        └── error.rs               (CodegenError: UnsupportedFeature, RegisterOverflow, ...)
```

### Structure Rationale

- **`resolve/`, `typecheck/`, `codegen/` as modules not crates:** All three need access to the same `ast::*` type hierarchy. Keeping them inside `writ-compiler` avoids exporting AST types as a public API for cross-crate consumption. The existing `lower/` module set the precedent.
- **`writ-module` as a dependency of `writ-compiler`:** Codegen needs `ModuleBuilder`. This is the only new dependency — it is already the right direction (compiler produces modules, does not depend on the VM).
- **`DefId` as the resolution currency:** All name-to-declaration bindings pass through an opaque `DefId` (e.g., `u32` index into a flat `DefMap`). This decouples the IR from the raw `String` names in the AST and lets type checking skip all name lookups.
- **Separate `ir.rs` per phase:** Each phase produces its own IR type rather than mutating the `Ast` in place. The `NameResolved` IR mirrors the `Ast` structure but replaces `Ident { name }` and `Path { segments }` with `DefId`-resolved references. The `Typed` IR adds a `Ty` annotation on every expression node. This makes each phase testable in isolation without needing the subsequent phase.
- **`InferCtx` inside `typecheck/infer.rs`:** Holds the union-find structure for type variable unification. When type inference resolves all type variables (or reports errors for unresolved ones), the `Typed` IR is emitted with all variables substituted. This is the standard Hindley-Milner constraint + unification approach, appropriate for Writ's "local variables inferred, signatures explicit" model.
- **`LinearRegisterAllocator` in `codegen/register.rs`:** Assigns a fresh `u16` register per SSA-like value. For the reference implementation, simple linear allocation (no reuse) is correct and sufficient. The IL spec's abstract typed registers match this directly — the VM allocates storage from type metadata, not register indices.

## Architectural Patterns

### Pattern 1: Two-Pass Name Resolution (Decls Before Bodies)

**What:** Name resolution runs in two passes over the AST. Pass 1 collects all top-level declaration names into the `DefMap` and `NamespaceMap` without resolving bodies. Pass 2 resolves all name references (in function bodies, type positions, impl targets) against the fully-populated `DefMap`. This handles forward references between top-level declarations.

**When to use:** Mandatory — Writ allows `fn a() { b() }` and `fn b() { ... }` in any order. Single-pass resolution would fail on forward references.

**Trade-offs:** Two passes means the AST is traversed twice. For this project size this is negligible. The alternative (building a dependency graph and topological ordering) is far more complex and still needs two conceptual phases.

**Example:**
```rust
// resolve/mod.rs
pub fn resolve(ast: Ast) -> (NameResolved, Vec<ResolveError>) {
    let mut ctx = ResolveCtx::new();

    // Pass 1: collect all top-level def names
    for decl in &ast.items {
        ctx.register_decl(decl);
    }

    // Pass 2: resolve all references in bodies
    let mut ir_items = Vec::new();
    for decl in ast.items {
        ir_items.push(ctx.resolve_decl(decl));
    }

    (NameResolved { items: ir_items }, ctx.take_errors())
}
```

### Pattern 2: Rib Stack for Block-Scoped Name Resolution

**What:** A "rib" is an abstraction of a scope. The resolver maintains a `Vec<Rib>` (a stack). When entering a function body, a new rib is pushed. When entering a `{ }` block, another rib is pushed. Name resolution walks the rib stack from innermost to outermost, returning the first match (shadowing). Leaving a scope pops the rib.

**When to use:** All block-scoped name lookup in Writ: `let` bindings, function parameters, `for` loop bindings, match arm variables. This is the pattern used by `rustc_resolve` and mirrors how `LoweringContext` uses its namespace stack.

**Trade-offs:** Simple and correct for Writ's lexical scoping rules. The `LoweringContext::push_namespace / pop_namespace` pattern in `lower/context.rs` is structurally identical — the resolver extends the same idea to all name kinds.

**Example:**
```rust
// resolve/scope.rs
pub struct Rib {
    pub bindings: HashMap<String, DefId>,
    pub kind: RibKind,  // FunctionBody | Block | ForLoop | MatchArm
}

pub struct SymbolTable {
    ribs: Vec<Rib>,
}

impl SymbolTable {
    pub fn push_rib(&mut self, kind: RibKind) { self.ribs.push(Rib::new(kind)); }
    pub fn pop_rib(&mut self) { self.ribs.pop(); }

    pub fn bind(&mut self, name: String, def: DefId) {
        self.ribs.last_mut().unwrap().bindings.insert(name, def);
    }

    pub fn lookup(&self, name: &str) -> Option<DefId> {
        // Innermost wins (shadowing)
        for rib in self.ribs.iter().rev() {
            if let Some(&def) = rib.bindings.get(name) {
                return Some(def);
            }
        }
        None
    }
}
```

### Pattern 3: Hindley-Milner Constraint Unification for Local Type Inference

**What:** Writ's type system requires explicit annotations on function parameters and return types but infers local variable types. The type checker uses a constraint-based approach: fresh type variables (`TyVar`) are introduced for unannotated locals; equality constraints are generated as expressions are checked; a union-find structure resolves constraints by unification.

**When to use:** Any `let x = expr;` with no annotation. Also for resolving which concrete type `Option::Some(x)` wraps when the outer context specifies an `Option<int>`. Full Hindley-Milner (polymorphic let) is not required — Writ does not have implicit polymorphism. The inference is local and rank-1.

**Trade-offs:** A full union-find implementation is ~50 lines. The alternative (bidirectional propagation without unification) breaks on cases like `let x = f(); let y: int = x;` where x's type is determined by downstream use. Unification handles this correctly.

**Example:**
```rust
// typecheck/infer.rs
pub struct InferCtx {
    next_var: u32,
    subst: Vec<Option<Ty>>,  // union-find: TyVar(i) → Ty or None
}

impl InferCtx {
    pub fn fresh_var(&mut self) -> Ty {
        let v = self.next_var;
        self.next_var += 1;
        self.subst.push(None);
        Ty::Var(v)
    }

    pub fn unify(&mut self, a: Ty, b: Ty) -> Result<(), TypeError> {
        let a = self.resolve(a);
        let b = self.resolve(b);
        match (a, b) {
            (Ty::Var(i), t) | (t, Ty::Var(i)) => { self.subst[i as usize] = Some(t); Ok(()) }
            (Ty::Int, Ty::Int) => Ok(()),
            (Ty::Named(da), Ty::Named(db)) if da == db => Ok(()),
            (a, b) => Err(TypeError::TypeMismatch { expected: a, found: b, span: ... }),
        }
    }
}
```

### Pattern 4: `CodegenCtx` as the Stateful Codegen Thread

**What:** The codegen phase threads a `CodegenCtx` through all emission functions. It holds the `ModuleBuilder`, a `DefId → MetadataToken` map (so that references to already-emitted declarations resolve to their token), a per-function `LinearRegisterAllocator`, and a `LabelMap` for forward branch patching.

**When to use:** All codegen functions receive `&mut CodegenCtx`. The `ModuleBuilder` is never passed directly — all builder calls go through `CodegenCtx` methods that also update the token maps.

**Trade-offs:** Matches the pattern used by `LoweringContext` in the lowering phase. Centralizing state in a context struct prevents codegen functions from needing 6 parameters. The risk is that `CodegenCtx` grows too large; subdivide into `FnCodegenCtx` (per-function state: register allocator, label map) nested inside `ModuleCodegenCtx` (cross-function state: builder, token maps).

**Example:**
```rust
// codegen/context.rs
pub struct ModuleCodegenCtx {
    pub builder: ModuleBuilder,
    pub def_tokens: HashMap<DefId, MetadataToken>,  // resolved def → IL token
    pub string_pool: HashMap<String, u32>,           // deduplicated string index
}

pub struct FnCodegenCtx<'m> {
    pub module: &'m mut ModuleCodegenCtx,
    pub regs: LinearRegisterAllocator,
    pub labels: LabelMap,
    pub instructions: Vec<Instruction>,
}

impl FnCodegenCtx<'_> {
    pub fn alloc_reg(&mut self) -> u16 { self.regs.alloc() }
    pub fn emit(&mut self, instr: Instruction) { self.instructions.push(instr); }
    pub fn define_label(&mut self, name: &str) { self.labels.define(name, self.instructions.len()); }
    pub fn finish_method(self, name: &str, sig: &[u8], flags: u16) -> MetadataToken {
        let body = MethodBody { code: encode_instructions(&self.instructions), ... };
        self.module.builder.add_method(name, sig, flags, self.regs.count(), body)
    }
}
```

## Data Flow

### Full Pipeline Flow

```
Source text (.writ files)
    |
    v
writ-parser::parse()
    |
    v
Vec<Spanned<Item>>  (CST — preserves all syntax, has &str borrows)
    |
    v
writ-compiler::lower()         [existing, unchanged]
    |-- desugar T? → Option<T>
    |-- lower dlg → fn
    |-- lower entity → struct + impl + hooks
    |-- lower operators → contract impls
    |
    v
writ-compiler::ast::Ast { items: Vec<AstDecl> }   (owned, no lifetimes)
    |
    v
writ-compiler::resolve::resolve()                  [NEW]
    |-- pass 1: collect top-level def names → DefMap
    |-- pass 2: bind all Ident/Path refs → DefId
    |-- resolve using-imports → namespace visibility sets
    |-- detect undefined names, privacy violations, ambiguities
    |
    v
NameResolved IR + DefMap + Vec<ResolveError>
    |
    (if errors: report and stop; otherwise continue)
    |
    v
writ-compiler::typecheck::typecheck()              [NEW]
    |-- assign explicit types from signatures and field decls
    |-- infer local variable types (unification)
    |-- check operand types, return types, contract impls
    |-- resolve operator overloads via contract dispatch
    |
    v
Typed IR + Vec<TypeError>
    |
    (if errors: report and stop; otherwise continue)
    |
    v
writ-compiler::codegen::codegen()                  [NEW]
    |-- emit TypeDef rows for structs, enums, entities, components
    |-- emit FieldDef rows for all fields
    |-- emit MethodDef + bodies for all functions
    |-- emit ImplDef rows for contract implementations
    |-- encode TypeRef blobs for all types
    |-- allocate registers (linear, SSA-style)
    |-- patch forward branch labels
    |-- call ModuleBuilder::build() → Module
    |
    v
writ_module::Module
    |
    v
writ-cli `compile` subcommand writes .writil binary
    |
    v
writ-runtime Domain::load(module) → runs in VM
```

### Key Data Flows

1. **`DefId` as the cross-phase currency.** After name resolution, all `String` names are replaced by `DefId` indices into the `DefMap`. Type checking and codegen never do string-based lookup — they use `DefId` to retrieve type information and emit `MetadataToken` references.

2. **`DefMap → MetadataToken` mapping in codegen.** Before emitting any method bodies, codegen does a first pass over all declarations to emit `TypeDef`, `FieldDef`, `MethodDef`, `ContractDef` rows (skeleton pass). This assigns `MetadataToken`s to every definition. Body emission then cross-references these tokens. This two-sub-pass structure within codegen mirrors the two-pass pattern of name resolution.

3. **`Ty → TypeRef blob` encoding.** The `codegen/type_sig.rs` module converts the type checker's `Ty` enum into the binary TypeRef blob format required by `writ-module`. Primitive types become their `0x00`–`0x04` tags; named types emit `0x10` + `TypeDef` index; generic instantiations emit `0x11` + `TypeSpec` index; arrays emit `0x20` + element TypeRef. This is a pure function: `fn encode_ty(ty: &Ty, ctx: &mut ModuleCodegenCtx) -> Vec<u8>`.

4. **Error accumulation without halting.** Each phase follows the existing `LoweringContext` error accumulation pattern: errors are collected into a `Vec<PhaseError>` and the phase continues on best-effort. The phase boundary (resolve → typecheck → codegen) does halt if the preceding phase produced errors, since a downstream phase cannot meaningfully operate on an IR with unresolved or ill-typed nodes.

5. **`using` imports resolved before body passes.** The namespace resolution pass (part of resolve pass 1) builds a `NamespaceMap` that flattens all `using` declarations into a set of visible `pub` `DefId`s per scope. Body resolution (pass 2) queries this map for unqualified names, which is O(1) per lookup after the map is built.

## Integration Points

### Existing Crate Changes

| Crate | Change | Rationale |
|-------|--------|-----------|
| `writ-compiler/Cargo.toml` | Add `writ-module = { path = "../writ-module" }` dependency | Codegen phase uses `ModuleBuilder` |
| `writ-compiler/src/lib.rs` | Add `pub mod resolve`, `pub mod typecheck`, `pub mod codegen`; add `pub fn compile(ast: Ast) -> Result<Module, CompileErrors>` | New phases are new pub modules; `compile()` is the entry point |
| `writ-cli/Cargo.toml` | Add `writ-compiler = { path = "../writ-compiler" }` dependency | CLI needs to call `writ-compiler::compile()` |
| `writ-cli/src/main.rs` | Add `compile` subcommand | Drives source → parse → lower → compile pipeline |
| `writ-parser` | None | No changes needed |
| `writ-module` | None | Consumed as-is |
| `writ-runtime` | None | Out of scope for this milestone |
| `writ-assembler` | None | Out of scope |

### New Module Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `lower` → `resolve` | `lower` produces `Ast`; `resolve` consumes it | `Ast` is already the boundary type; no change |
| `resolve` → `typecheck` | `resolve` produces `NameResolved + DefMap`; `typecheck` consumes both | `NameResolved` and `DefMap` are new types defined in `resolve/` |
| `typecheck` → `codegen` | `typecheck` produces `Typed + DefMap`; `codegen` consumes both | `Typed` is a new type defined in `typecheck/`; `DefMap` passes through unchanged |
| `codegen` → `writ-module` | `codegen` calls `writ_module::ModuleBuilder` methods; returns `writ_module::Module` | One-way: codegen depends on `writ-module`, not vice versa |

### Build Order

The dependency graph within the milestone forces this implementation order:

```
Phase 1 — resolve/ (no new crate dependency)
  resolve/def.rs         (DefId, DefKind, DefMap — no deps other than ast/)
  resolve/scope.rs       (Rib, SymbolTable — uses def.rs)
  resolve/namespace.rs   (NamespaceMap — uses def.rs + ast/ namespaces)
  resolve/ir.rs          (NameResolved IR — uses def.rs + ast/ for structure)
  resolve/error.rs       (ResolveError — thiserror, SimpleSpan)
  resolve/mod.rs         (resolve() — orchestrates passes using all above)

Phase 2 — typecheck/ (depends on resolve/)
  typecheck/ty.rs        (Ty enum — uses DefId from resolve/def.rs)
  typecheck/infer.rs     (InferCtx, union-find — uses ty.rs)
  typecheck/ir.rs        (Typed IR — adds Ty to NameResolved nodes)
  typecheck/env.rs       (TypeEnv — local variable → Ty map)
  typecheck/error.rs     (TypeError — thiserror, Ty, SimpleSpan)
  typecheck/contract.rs  (contract impl check — uses DefMap + Ty)
  typecheck/coerce.rs    (implicit coercions — uses InferCtx)
  typecheck/check.rs     (check_expr, check_stmt — uses all above)
  typecheck/mod.rs       (typecheck() — orchestrates using all above)

Phase 3 — codegen/ (depends on typecheck/ and writ-module)
  codegen/register.rs    (LinearRegisterAllocator — no external deps)
  codegen/label.rs       (LabelMap — no external deps)
  codegen/type_sig.rs    (Ty → TypeRef blob — uses ty.rs + writ-module table ids)
  codegen/error.rs       (CodegenError — thiserror)
  codegen/context.rs     (ModuleCodegenCtx, FnCodegenCtx — uses ModuleBuilder)
  codegen/expr.rs        (emit_expr — uses context + type_sig + register)
  codegen/stmt.rs        (emit_stmt — uses expr + context)
  codegen/decl.rs        (emit_fn, emit_struct, emit_enum, emit_entity — uses stmt + context)
  codegen/mod.rs         (codegen() — skeleton pass then body pass)

Phase 4 — wire up writ-cli
  writ-cli/src/main.rs   (compile subcommand: parse → lower → resolve → typecheck → codegen → write)
```

**Rationale for keeping in one crate:** The resolver needs to import `ast::AstDecl`, `ast::AstExpr`, etc. The type checker needs to import `resolve::DefId` and `resolve::NameResolved`. The codegen needs to import `typecheck::Typed` and `typecheck::Ty`. These are all natural intra-crate module imports. Splitting into three crates would make `writ-compiler::ast` a public dependency of two downstream crates, which means `ast` itself would need to live in its own crate — creating at minimum a `writ-ast`, `writ-resolver`, `writ-typeck`, `writ-codegen` split that adds complexity with no benefit at this project scale.

## Anti-Patterns

### Anti-Pattern 1: Mutating the Ast In-Place Across Phases

**What people do:** Instead of defining `NameResolved` and `Typed` as separate IR types, annotate `AstExpr` nodes with `Option<DefId>` and `Option<Ty>` fields and fill them in during the respective passes.

**Why it's wrong:** The AST nodes' `Option<DefId>` fields are `None` during lowering, making it impossible to distinguish "not yet resolved" from "genuinely missing." After resolution, `None` could mean either "optional in the language" or "resolution failed." This ambiguity leaks into type checking. It also breaks the error-accumulation model: if resolution stops partway, partially-resolved nodes have `Some` and `None` fields mixed, which confuses downstream passes.

**Do this instead:** Define `NameResolved` and `Typed` as distinct IR types where every identifier slot is a `DefId` (not `Option<DefId>`) and every expression carries a `Ty`. Conversion is explicit and testable in isolation.

### Anti-Pattern 2: Calling `ModuleBuilder` Directly from `typecheck/`

**What people do:** Start emitting IL during type checking to avoid a separate codegen pass. "Why not emit instructions as soon as we know the type of each expression?"

**Why it's wrong:** Type checking is a constraint-solving process. A type variable introduced at expression X may only be resolved after seeing expression Y, which is downstream. If codegen is interleaved, X's instructions must be emitted before X's type is known — requiring placeholder registers or backpatching at the type level, not just the label level. The separate `Typed` IR guarantees all types are known before any instruction is emitted.

**Do this instead:** Type checking produces a fully-annotated `Typed` IR where every expression node carries its resolved `Ty`. Codegen then operates on this IR without needing to perform any type inference.

### Anti-Pattern 3: Namespace Resolution as a String Table

**What people do:** Build a `HashMap<String, DefId>` at the global level and resolve all names by direct string lookup.

**Why it's wrong:** Writ has block namespaces, `using`-scoped imports, file-local visibility, and shadow bindings in local scopes. A flat global map cannot represent any of these. A name `Guard` resolves to different `DefId`s in different scopes depending on `using` declarations and local bindings.

**Do this instead:** The `SymbolTable` rib stack handles local scoping; the `NamespaceMap` handles `pub` cross-namespace visibility; the two are queried in order (rib stack first, then namespace map for unqualified names, then error on not found). The `DefMap` is the flat `DefId → declaration info` table, not a name lookup table.

### Anti-Pattern 4: One Register Per Writ Variable

**What people do:** Assign one register to each `let` binding, allocating a fixed register at the `Let` statement point and reusing it throughout the function.

**Why it's wrong:** The IL spec's registers are abstract — there is no spilling, no register pressure, no calling convention register constraints. SSA-style "one fresh register per value" is simpler and correct. It produces more registers (all within the `u16` range; 65535 max), but the runtime allocates storage from type metadata regardless. The assembler already uses this model implicitly (each `.reg` directive is a flat list).

**Do this instead:** `LinearRegisterAllocator` simply increments a counter on each `alloc()` call. A `let x = expr` allocates a register for `expr`, then that register IS `x`'s register for the rest of the function. No reuse tracking needed for the reference implementation.

### Anti-Pattern 5: Adding `writ-module` as a Dependency of `writ-parser` or `writ-runtime` Dependency on `writ-compiler`

**What people do:** "The CLI needs to parse and compile, so let's put everything in `writ-cli`'s deps." This leads to `writ-cli` → `writ-compiler` → `writ-module` → `writ-runtime` forming a chain where runtime is pulled into the compiler.

**Why it's wrong:** `writ-compiler` must never depend on `writ-runtime`. The compiler backend emits IL; it does not execute it. If `writ-runtime` is in the compiler's dependency tree, a change to the VM forces recompilation of the compiler even when the language semantics haven't changed.

**Do this instead:** `writ-compiler` depends on `writ-module` only (the pure data layer). `writ-runtime` depends on `writ-module` only. `writ-cli` depends on all three: parser, compiler, module, runtime, assembler. The diamond dependency on `writ-module` is fine in Cargo — it is a pure data crate with no global state.

## Scalability Considerations

| Concern | Reference Impl (v3.0) | If it becomes a bottleneck |
|---------|----------------------|---------------------------|
| Name resolution performance | Two-pass traversal, O(n) in declaration count | Add fingerprinting to skip unchanged files (incremental) |
| Type inference scope | Local, rank-1, no implicit polymorphism — deliberately limited | Full HM polymorphism if the spec adds it later |
| Register count | Linear allocation, no reuse — may produce many registers per function | Add liveness-based register coalescing in a future pass |
| Codegen correctness | No optimization, direct emission | Add a peephole optimizer phase between `Typed` and `Module` |
| Multi-file compilation | Not scoped for v3.0; single-file or pre-merged AST assumed | Add a module driver that parses all .writ files, merges DefMaps across files |

## Sources

- Writ Language Specification §5 (type system), §21 (scoping rules), §23 (modules/namespaces), §28 (lowering reference) — HIGH confidence (authoritative)
- Writ IL Specification §2.15 (IL type system: TypeRef blob encoding) — HIGH confidence (authoritative)
- Existing `writ-compiler` source (`ast/`, `lower/`, `lower/context.rs`) — HIGH confidence (codebase)
- Existing `writ-module` source (`builder.rs`, `tables.rs`, `instruction.rs`) — HIGH confidence (codebase)
- rustc-dev-guide: Overview, HIR, Name Resolution, Type Inference — MEDIUM confidence (pattern reference; rustc is far larger but the rib-stack and HM patterns are standard)
- rustc-dev-guide: THIR (Typed HIR) — MEDIUM confidence (confirms the "produce fully-typed IR before codegen" pattern)
- Standard compiler literature: Hindley-Milner two-phase (constraint gen + unification) — HIGH confidence (textbook)

---
*Architecture research for: Writ v3.0 — name resolution, type checking, IL codegen*
*Researched: 2026-03-02*
