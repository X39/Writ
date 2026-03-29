# Phase 23: Type Checking - Research

**Researched:** 2026-03-02
**Domain:** Static type inference and checking for a custom scripting language in Rust
**Confidence:** HIGH

## Summary

Phase 23 implements a complete type checker for the Writ language. Input is `NameResolvedAst` (produced by Phase 22), output is `TypedAst` with no `Option<Ty>` fields on expression nodes. The phase does not emit IL — that is Phases 24-25.

The implementation follows an established Rust compiler pattern: an interned `Ty` type backed by a `TyInterner`, a local inference pass that propagates types bottom-up through expressions, and a unification step (either `ena` union-find or a substitution map) for generic type argument inference. All errors are collected without early exit, using a `Ty::Error` poison type to suppress cascading. The existing `writ-diagnostics` crate provides the `Diagnostic` builder API already used by name resolution — type errors follow identical patterns with multi-span context.

The phase is split across three plans. Plan 23-01 establishes the typed IR foundation, handles literal/primitive types, `let` binding inference, and function call checking. Plan 23-02 adds field access, component access, contract bounds, mutability enforcement, and return-type checking. Plan 23-03 covers the more complex features: `?`/`!`/`try` desugaring, enum exhaustiveness, closure capture inference, generic unification, spawn/join/cancel types, `new`-construction checking, and `for`-loop element types.

**Primary recommendation:** Build the `TyInterner` and `TyKind` in Plan 23-01 first — everything else in the phase depends on `Ty` being a cheap `Copy` interned ID. Use `ena` union-find for inference variables; it integrates cleanly with the existing `id-arena` + `rustc-hash` codebase and handles forward references without a second pass.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Typed IR shape**
- New IR types — fresh `TypedExpr`, `TypedStmt`, `TypedDecl` enums, completely separate from the existing AST and resolve IR. Clean `NameResolvedAst -> TypedAst` pipeline. Codegen only sees typed nodes.
- Inline type field — every `TypedExpr` variant carries `ty: Ty, span: Span` directly (no wrapper struct, no side-table)
- Interned `Ty` via arena — `Ty` is a `Copy` ID (`Ty(u32)`) into a `TyInterner` with `TyKind` enum. Enables cheap passing, structural equality, deduplication. `TyKind` variants: `Int`, `Float`, `Bool`, `String`, `Void`, `Struct(DefId)`, `Entity(DefId)`, `Enum(DefId)`, `Array(Ty)`, `Func{params, ret}`, `Option(Ty)`, `Result(Ty, Ty)`, `TaskHandle(Ty)`, `GenericParam(u32)`, `Infer(InferVar)`, `Error` (poison)
- `?` and `!` desugared — `UnaryPostfix` nodes are desugared to typed `Match` nodes in the Typed IR. No raw `?`/`!` operator nodes survive into the output. Codegen only sees match patterns.

**Inference & unification**
- Local inference + bidirectional lambda context — local variable types inferred from initializer (spec §5.2). Lambda parameter/return types inferred from expected-type context (function parameter, typed variable, contract method) per spec §12.4.2. No cross-function inference. No implicit conversion at assignment/argument boundaries (spec §10.2: `Into<T>` requires explicit `.into<T>()` call).
- Generic type argument inference — at call sites, type args omitted when inferable from arguments (spec §11.2: `first(inventory)` infers `T` from `List<Item>`). Inference approach (ena union-find vs substitution map) at Claude's discretion.
- Contract bound checking — approach (eager at call site vs deferred) at Claude's discretion. Must report unsatisfied bounds with the bound named (success criterion 2).
- Closure capture classification during type checking — `let` bindings captured by value, `let mut` bindings captured by reference (spec §12.4.4). Captures annotated in `TypedExpr::Lambda` with `Capture { name, ty, mode: ByValue|ByRef }`. Codegen reads annotations directly.

**Error strategy**
- Collect all errors, poison on error — continue checking after errors using `Ty::Error` as poison type. Errors involving poison types are suppressed (no cascading). Report all independent type errors at once.
- Multi-span with context — errors point to BOTH the error site AND the relevant declaration (e.g., "expected int (from function signature at line 5), got string (at line 12)"). Matches success criteria for precise, actionable errors.
- Actionable suggestions — include `help:` hints where possible: missing contract impl suggestions (TYPE-19), `Into<T>` conversion hints, missing `mut` suggestions, similar name suggestions. Follows the pattern established by name resolution's "did you mean?" suggestions.
- Error code numbering — at Claude's discretion (separate range vs continuing sequence)

**Mutability model**
- Method-signature based detection — methods with `mut self` are mutations. Calling a `mut self` method on a `let` binding is an error. Covers: field assignment, `mut self` method calls, passing as `mut` parameter.
- Root-binding propagation — `let mut x` makes ALL field chains through `x` mutable; `let x` makes them ALL immutable. No per-field mutability. Applies uniformly to struct fields AND component fields (`guard.Health.current`).
- Enforced through function value aliases — storing a `mut self` method as a function value preserves the mutability requirement. Calling it on an immutable binding is still an error. Keeps mutability sound.
- Arrays follow the same rule — mutating methods (`push`, `pop`, `insert`, `remove`, index assignment) require `let mut` binding. Read operations (`length`, indexing, iteration) are fine on `let`.
- For-loop variables immutable by default — `for item in items` makes `item` immutable. `for mut item in items` is required to mutate. Source collection must also be `mut` for `mut item` iteration.
- Dual-span error presentation — mutability errors show BOTH the binding declaration site and the violation site (success criterion 3)

### Claude's Discretion
- Unification algorithm choice (ena union-find vs substitution map)
- Contract bound checking timing (eager vs deferred)
- Error code numbering scheme
- Exact `TyInterner` implementation details
- Performance optimizations in the type checking pass

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TYPE-01 | Compiler infers primitive types for literals (int, float, bool, string) | TyKind::Int/Float/Bool/String; literal nodes in AST carry the tag |
| TYPE-02 | Compiler infers `let` binding types from initializer; checks annotated bindings for compatibility | Bottom-up expr typing; annotated bindings check inferred type == declared type |
| TYPE-03 | Compiler checks function call arity, argument types, and propagates return types | DefMap has fn signature; call site checks arg count and each arg type against param type |
| TYPE-04 | Compiler checks field access types, distinguishing struct/entity script fields from component fields | DefMap knows struct/entity/component DefKind; field lookup returns `Ty` |
| TYPE-05 | Compiler verifies concrete type arguments satisfy contract bounds at generic call sites | ImplDef lookup by (type, contract); report bound name on failure (TYPE-19 overlap) |
| TYPE-06 | Compiler enforces strict mutability (`let` prevents reassignment AND mutation; `mut self` vs `self`) | Root-binding flag tracked per local; dual-span errors (CONTEXT decision) |
| TYPE-07 | Compiler verifies all code paths return the declared type; void functions don't return values | Control-flow return-type check; if/match arm unification |
| TYPE-08 | Compiler checks `?` requires Option scrutinee and Option return context; `!` works on Option and Result | Desugar to typed match; context is `Option<T>` or `Result<T, E>` |
| TYPE-09 | Compiler checks `try` requires Result scrutinee and compatible Result return context | Desugar to typed match; enclosing function return type must be Result |
| TYPE-10 | Compiler checks enum `match` exhaustiveness (all variants covered or wildcard present) | Enum variant list from DefMap; missing variants named in error |
| TYPE-11 | Compiler types component access as guaranteed (concrete entity) or Optional (generic Entity) | entity[Component] → Ty::Concrete if entity has `use Component`; else Option<Ty> |
| TYPE-12 | Compiler infers closure captures, classifies by-value vs by-reference, and checks immutability constraints | Capture walk: `let` = by-value, `let mut` = by-ref; immutability errors |
| TYPE-13 | Compiler infers generic type arguments at call sites via unification | ena InferVar per type param; unify argument types; report unsolved vars |
| TYPE-14 | Compiler checks `spawn` returns TaskHandle, `join` returns result type, `cancel` is void | TyKind::TaskHandle(inner_ty); spawn fn return type wraps in TaskHandle |
| TYPE-15 | Compiler checks `new Type {}` for field presence, field types, and entity vs struct distinction | Field set from DefMap; missing required fields; extra fields; entity vs struct distinction |
| TYPE-16 | Compiler binds `for` loop variable type via `Iterable<T>` impl lookup | Iterable<T> contract lookup; element type T; loop var binding |
| TYPE-17 | Compiler desugars `?` and `!` operators to typed match nodes in the typed IR | Overlap with TYPE-08/09; no raw UnaryPostfix in TypedExpr |
| TYPE-18 | Compiler produces precise mutability errors pointing to both mutation site and immutable binding | Dual-span: primary = mutation site, secondary = binding declaration |
| TYPE-19 | Compiler suggests missing contract implementations in type errors | help: text naming the missing impl; e.g., "consider `impl Add<int, int> for MyType`" |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `ena` | 0.14.4 | Union-find for type inference variables (InferVar unification) | Used by rustc itself for the same problem; integrates with Rust ownership model cleanly |
| `id-arena` | 2.3.0 | Already in deps; `TyInterner` will use `Vec<TyKind>` (not arena, but same pattern) | Project already uses it for DefId; Ty(u32) follows same pattern |
| `rustc-hash` | 2.1.1 | `FxHashMap` for type interning dedup table | Already in deps; used throughout resolve |
| `writ-diagnostics` | workspace | Diagnostic builder for type errors | Already used in name resolution; same API |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `insta` | 1.x | Snapshot tests for typed IR output | Already in dev-deps; same test pattern as Phase 22 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `ena` union-find | Simple substitution map | Substitution map is simpler but requires re-running resolution on every lookup; ena handles cycles and forward refs naturally. For Writ's limited inference scope (no cross-function), either works — but ena is the safer choice |
| `Vec<TyKind>` with `Ty(u32)` index | `id-arena::Arena<TyKind>` | Both work; Vec is simpler, arena gives type safety. Recommendation: use Vec+index (same pattern as TyKind interning in rustc) |

**Installation:**
```bash
# Add to writ-compiler/Cargo.toml [dependencies]:
ena = "0.14.4"
```
(All other dependencies already present)

---

## Architecture Patterns

### Recommended Project Structure
```
writ-compiler/src/
├── check/
│   ├── mod.rs          # Entry point: typecheck(NameResolvedAst) -> (TypedAst, Vec<Diagnostic>)
│   ├── ir.rs           # TypedExpr, TypedStmt, TypedDecl, TypedAst definitions
│   ├── ty.rs           # Ty(u32), TyKind, TyInterner, InferVar
│   ├── infer.rs        # Inference context, bidirectional typing, let binding
│   ├── unify.rs        # ena union-find wrapper, InferVar resolution
│   ├── check_expr.rs   # Expression type checking (field access, calls, operators)
│   ├── check_stmt.rs   # Statement checking (let, for, while, return)
│   ├── check_decl.rs   # Declaration checking (fn, struct, entity, enum, impl)
│   ├── mutability.rs   # Mutability tracking, root-binding propagation
│   ├── pattern.rs      # Pattern checking and exhaustiveness
│   ├── desugar.rs      # ? / ! / try desugaring to typed match nodes
│   └── error.rs        # TypeError enum, impl From<TypeError> for Diagnostic
```

`lib.rs` addition:
```rust
pub mod check;
pub use check::typecheck;
```

### Pattern 1: TyInterner with structural deduplication

**What:** A flat `Vec<TyKind>` where `Ty(u32)` is an index. A `FxHashMap<TyKind, Ty>` ensures structural deduplication — calling `intern(TyKind::Array(Ty(3)))` twice returns the same `Ty`.

**When to use:** Every time a `Ty` is needed for a newly constructed type.

**Example:**
```rust
// Source: project pattern (matches id-arena DefId pattern from Phase 22)
pub struct TyInterner {
    kinds: Vec<TyKind>,
    map: FxHashMap<TyKind, Ty>,
}

impl TyInterner {
    pub fn intern(&mut self, kind: TyKind) -> Ty {
        if let Some(&ty) = self.map.get(&kind) {
            return ty;
        }
        let id = Ty(self.kinds.len() as u32);
        self.kinds.push(kind.clone());
        self.map.insert(kind, id);
        id
    }

    pub fn kind(&self, ty: Ty) -> &TyKind {
        &self.kinds[ty.0 as usize]
    }

    // Convenience constructors
    pub fn int(&mut self) -> Ty { self.intern(TyKind::Int) }
    pub fn error(&mut self) -> Ty { self.intern(TyKind::Error) }
    pub fn option(&mut self, inner: Ty) -> Ty { self.intern(TyKind::Option(inner)) }
}
```

### Pattern 2: TypedExpr with inline Ty and Span

**What:** Each `TypedExpr` variant carries `ty: Ty` and `span: Span` directly. No wrapper struct needed.

**When to use:** Every expression node in the typed IR.

**Example:**
```rust
pub enum TypedExpr {
    Literal { ty: Ty, span: Span, value: LiteralValue },
    Var     { ty: Ty, span: Span, name: String, def_source: VarSource },
    Call    { ty: Ty, span: Span, callee: Box<TypedExpr>, args: Vec<TypedExpr> },
    Field   { ty: Ty, span: Span, receiver: Box<TypedExpr>, field: String },
    Match   { ty: Ty, span: Span, scrutinee: Box<TypedExpr>, arms: Vec<TypedArm> },
    // ... etc
    // NOTE: No UnaryPostfix variant -- ? and ! desugar to Match
}

impl TypedExpr {
    pub fn ty(&self) -> Ty {
        match self {
            TypedExpr::Literal { ty, .. } => *ty,
            TypedExpr::Var { ty, .. } => *ty,
            // ... all variants carry ty
        }
    }
}
```

### Pattern 3: Error-collecting inference context

**What:** A `CheckCtx` struct carrying `TyInterner`, `Vec<Diagnostic>`, and the `DefMap`. Errors are pushed and `Ty::Error` (poison) is returned. Any subsequent operation on a poison type is silently skipped.

**When to use:** Throughout all checking functions.

**Example:**
```rust
pub struct CheckCtx<'def> {
    pub interner: TyInterner,
    pub diags: Vec<Diagnostic>,
    pub def_map: &'def DefMap,
    pub infer: UnifyCtx,  // ena wrapper
}

impl CheckCtx<'_> {
    pub fn is_error(&self, ty: Ty) -> bool {
        matches!(self.interner.kind(ty), TyKind::Error)
    }

    pub fn error(&mut self, err: impl Into<Diagnostic>) -> Ty {
        self.diags.push(err.into());
        self.interner.error()
    }
}
```

### Pattern 4: Mutability scope tracking

**What:** A `LocalEnv` maps variable names to `(Ty, Mutability, binding_span)`. Mutability propagates from root binding: `let x` = immutable all the way down.

**When to use:** All field accesses and method calls must check receiver mutability.

**Example:**
```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mutability { Immutable, Mutable }

pub struct LocalEnv {
    scopes: Vec<Vec<(String, Ty, Mutability, Span)>>,
}

impl LocalEnv {
    pub fn push_scope(&mut self) { self.scopes.push(vec![]); }
    pub fn pop_scope(&mut self) { self.scopes.pop(); }

    pub fn define(&mut self, name: String, ty: Ty, mut_: Mutability, span: Span) {
        self.scopes.last_mut().unwrap().push((name, ty, mut_, span));
    }

    pub fn lookup(&self, name: &str) -> Option<(Ty, Mutability, Span)> {
        for scope in self.scopes.iter().rev() {
            for (n, ty, m, sp) in scope.iter().rev() {
                if n == name { return Some((*ty, *m, *sp)); }
            }
        }
        None
    }
}
```

### Pattern 5: ? / ! desugaring

**What:** `?` on `Option<T>` desugars to a match that returns `None` from the enclosing function if the scrutinee is `None`, otherwise unwraps to `T`. `!` desugars to a match that crashes on `None`/`Err` and unwraps the value.

**When to use:** During expression checking, when `UnaryPostfix(?, expr)` or `UnaryPostfix(!, expr)` is encountered.

**Example (? on Option<T>):**
```rust
// Source: spec §18.1 + CONTEXT.md decision
// Input:   expr?          (where expr: Option<T>, enclosing fn returns Option<U>)
// Output:  TypedExpr::Match {
//     ty: T,   // unwrapped type
//     scrutinee: expr,
//     arms: [
//         (Pattern::EnumVariant(Option::Some, [bind("__tmp")]), TypedExpr::Var(__tmp, T)),
//         (Pattern::EnumVariant(Option::None, []),              TypedExpr::Return(None)),
//     ]
// }
```

### Pattern 6: enum exhaustiveness check

**What:** For a `match` on an enum type, collect all variant names from the DefMap, subtract variants covered by arms (including wildcard `_`), report missing variants by name.

**When to use:** Every `match` expression on an `Enum(DefId)` type.

**Example:**
```rust
fn check_exhaustiveness(
    ctx: &mut CheckCtx,
    enum_id: DefId,
    arms: &[AstMatchArm],
    span: Span,
) {
    let enum_variants = ctx.def_map.enum_variants(enum_id);  // Vec<String>
    let mut covered: FxHashSet<String> = FxHashSet::default();
    let mut has_wildcard = false;

    for arm in arms {
        match &arm.pattern {
            Pattern::Wildcard | Pattern::Binding(_) => { has_wildcard = true; }
            Pattern::EnumVariant { name, .. } => { covered.insert(name.clone()); }
            // ...
        }
    }

    if !has_wildcard {
        let missing: Vec<_> = enum_variants.iter()
            .filter(|v| !covered.contains(*v))
            .cloned()
            .collect();
        if !missing.is_empty() {
            ctx.diags.push(TypeError::NonExhaustiveMatch {
                missing,
                // span points to the match expression
            }.into());
        }
    }
}
```

### Anti-Patterns to Avoid

- **Option<Ty> on expression nodes:** Every `TypedExpr` variant MUST carry a concrete `Ty`, even if it is `TyKind::Error`. Never store `Option<Ty>` in the typed IR.
- **Early return on first error:** Push to `ctx.diags` and return `Ty::Error`; never abort the whole check. Cascading errors on poison types are suppressed by `is_error()` guards.
- **Mutating the AST in-place:** The pipeline is strict: `NameResolvedAst -> TypedAst`. Never modify the resolved IR. Always produce fresh typed nodes.
- **Inlining enum variant data into DefEntry before Phase 23:** The DefMap currently stores `DefKind::Enum` but does not have a `variants` method. Plan 23-01 must add this to the DefMap API (or a parallel table) before the exhaustiveness checker can use it.
- **Using `resolve_value` from scope.rs for type checking:** The scope chain in `resolve/scope.rs` is for name resolution only. The type checker has its own `LocalEnv` for tracking local variable types and mutability. Do not reuse the resolve scope for type state.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Inference variable unification | Custom substitution map with occurs-check | `ena` 0.14.4 `UnificationTable` | ena handles snapshot/rollback, cycles, and occurs-check; hand-rolled unification is a common source of infinite loops |
| Type interning dedup | Per-call linear search | `FxHashMap<TyKind, Ty>` as interning map | Hash-based dedup is O(1) amortized; linear search is O(n) and causes quadratic slowdown on large programs |
| Error code allocation | Magic number literals | Constants in `writ-diagnostics/src/code.rs` | Matches the established pattern from Phase 22 (E0001–E0007, W0001–W0004); type errors start at E0100 (recommendation) |

**Key insight:** The unification problem for local generic inference is well-understood. `ena` is rustc's own union-find crate, stable and minimal. The only risk is over-engineering: Writ's generics are shallow (no higher-kinded types, no associated types), so the full power of ena isn't needed — but using it avoids an entire class of bugs.

---

## Common Pitfalls

### Pitfall 1: Enum variant list not in DefMap
**What goes wrong:** `DefEntry` for an enum only stores `DefKind::Enum` and the enum's name. There is no `variants: Vec<String>` field. Exhaustiveness checking (TYPE-10) and pattern type checking require knowing the variant names and their payload types.
**Why it happens:** Phase 22 (collector) only stored enough information for name resolution; variant payloads were not needed then.
**How to avoid:** Plan 23-01 must define a parallel `EnumDef` table or extend `DefEntry` to carry `variants: Vec<EnumVariant>` where `EnumVariant { name, fields: Vec<(String, Ty)> }`. This is a foundational piece — everything in Plan 23-03 depends on it.
**Warning signs:** If the exhaustiveness check has to re-parse the original AST to find variant names, something went wrong in the data model.

### Pitfall 2: Missing DefMap AST data for method signatures
**What goes wrong:** `DefEntry` carries only `kind`, `vis`, `generics`, `name`, `name_span`, `file_id`, `namespace`, `span`. It does NOT carry function parameter types, return types, struct field types, or impl block target/contract associations. These are in the original AST but were not persisted into the DefMap in Phase 22.
**Why it happens:** Phase 22's collector only needed enough to resolve names; it did not materialize full type signatures.
**How to avoid:** Plan 23-01 must define a `TypeEnv` table (indexed by DefId) that stores elaborated type signatures for every definition: `FnSig { params: Vec<(String, ResolvedType)>, ret: ResolvedType }`, `StructFields { fields: Vec<(String, ResolvedType)> }`, etc. This table is built by walking the `NameResolvedAst` decls at the start of type checking. Alternatively, the type checker can walk the original AST directly, using the DefMap for name lookups. **Recommendation:** Walk the original AST (available from `NameResolvedAst`), build a `TypeEnv` keyed by DefId early in Plan 23-01, then use that throughout.
**Warning signs:** If the type checker is accessing `ast.items` directly inside expression-checking functions (not just at startup), the architectural boundary is broken.

### Pitfall 3: Impl block association not materialized
**What goes wrong:** `def_map.impl_blocks` is a `Vec<DefId>` but each impl's target type and implemented contract are not stored in the DefMap. Type checking needs to answer "does type X implement contract Y?" for contract bound checking (TYPE-05) and for-loop element type resolution (TYPE-16).
**Why it happens:** Phase 22 tracked impl blocks by DefId but left association to type checking.
**How to avoid:** Early in Plan 23-01 (or at start of type check pass), build an `ImplIndex`: a `FxHashMap<(DefId, Option<DefId>), Vec<DefId>>` mapping `(target_type_def_id, Some(contract_def_id))` to the impl block DefIds for that combination. This enables O(1) contract satisfaction lookup.
**Warning signs:** Searching `impl_blocks` linearly for each contract check.

### Pitfall 4: Shadowing in LocalEnv breaks mutability tracking
**What goes wrong:** Writ allows variable shadowing (`let x = 10; let x = x * 2;`). If the LocalEnv uses a plain `FxHashMap<String, (Ty, Mutability, Span)>`, shadowing clobbers the previous binding. A shadowed variable's later reference should find the newer binding, but error spans must point to the correct declaration.
**Why it happens:** Stack-based shadowing is easy to miss when using flat maps.
**How to avoid:** Use a `Vec<Vec<(String, Ty, Mutability, Span)>>` (stack of scopes, each scope a list). Lookup iterates from the end of the innermost scope outward. New `let` always pushes to the current scope's list without removing previous entries. See Pattern 4 above.
**Warning signs:** `"expected int, got string"` errors pointing to the wrong `let` binding when shadowing is in play.

### Pitfall 5: ? / try context not threaded through recursive calls
**What goes wrong:** `?` requires the enclosing function return type to be `Option<T>`. `try` requires `Result<T, E>`. If the return type context is not passed as a parameter to expression-checking functions, the desugar step cannot verify the context or compute the propagation type.
**Why it happens:** It is easy to check the scrutinee type but forget to verify that the function return type is compatible.
**How to avoid:** The `CheckCtx` (or inference context) must carry the current function's declared return type and file_id at all times. When `?` is encountered, verify: (a) scrutinee is `Option<T>`, (b) enclosing return type is `Option<U>`. When `try` is encountered, verify: (a) scrutinee is `Result<T, E>`, (b) enclosing return type is `Result<U, E2>`.
**Warning signs:** `?` on an `Option<T>` inside a `Result<U, E>`-returning function silently passes without an error.

### Pitfall 6: Generic params interned with wrong index
**What goes wrong:** `TyKind::GenericParam(u32)` uses a numeric index, not a name string. If the index is not stable (e.g., it changes between the declaration site and the call site), unification breaks silently.
**Why it happens:** Generic parameter names (like `T`, `U`) are strings in the AST but must be index-based in the type system to enable substitution.
**How to avoid:** When entering a generic function's body for checking, create a mapping `{"T" -> 0, "U" -> 1}` from the function's `generics: Vec<String>` in its `DefEntry`. Use this mapping to convert `GenericParam("T")` (from `ResolvedType`) to `TyKind::GenericParam(0)` consistently throughout. At call sites, create fresh `InferVar` for each generic param slot and unify.
**Warning signs:** "type mismatch: GenericParam(0) vs GenericParam(1)" errors that occur because two instantiations used different index assignments.

---

## Code Examples

Verified patterns from existing codebase and spec:

### Existing error diagnostic builder pattern (from writ-diagnostics)
```rust
// Source: writ-diagnostics/src/diagnostic.rs
// Used identically in writ-compiler/src/resolve/error.rs
Diagnostic::error(code::E0001, "type mismatch: expected int, got string")
    .with_primary(file_id, error_span, "got string here")
    .with_secondary(decl_file_id, decl_span, "expected int, from signature here")
    .with_help("consider converting: value.into<int>()")
    .build()
```

### ena union-find usage pattern
```rust
// Source: project decision (STATE.md) + ena 0.14.4 API
use ena::unify::{InPlaceUnificationTable, UnifyKey, NoError};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct InferVar(u32);

impl UnifyKey for InferVar {
    type Value = Option<Ty>;
    fn index(&self) -> u32 { self.0 }
    fn from_index(u: u32) -> Self { InferVar(u) }
    fn tag() -> &'static str { "InferVar" }
}

pub struct UnifyCtx {
    table: InPlaceUnificationTable<InferVar>,
}

impl UnifyCtx {
    pub fn new_var(&mut self) -> InferVar { self.table.new_key(None) }
    pub fn unify(&mut self, a: Ty, b: Ty) -> Result<(), ()> { /* ... */ }
    pub fn resolve(&mut self, var: InferVar) -> Option<Ty> {
        self.table.probe_value(var)
    }
}
```

### Capture inference for closures
```rust
// Source: CONTEXT.md decision (§12.4.4)
pub struct Capture {
    pub name: String,
    pub ty: Ty,
    pub mode: CaptureMode,
    pub binding_span: Span,  // span of the let/let mut that introduced the binding
}

pub enum CaptureMode {
    ByValue,   // let binding — copy the value into closure env
    ByRef,     // let mut binding — reference; closure can mutate through it
}

// TypedExpr::Lambda carries:
pub struct TypedLambda {
    pub ty: Ty,              // Func { params, ret }
    pub span: Span,
    pub params: Vec<(String, Ty)>,
    pub ret_ty: Ty,
    pub captures: Vec<Capture>,   // classified during checking
    pub body: Box<TypedExpr>,
}
```

### Test helper pattern (mirrors Phase 22 resolve_tests.rs)
```rust
// Source: writ-compiler/tests/resolve_tests.rs (established pattern)
fn typecheck_src(src: &'static str) -> (TypedAst, Vec<Diagnostic>) {
    let ast = parse_and_lower(src);
    let file_id = FileId(0);
    let asts = vec![(file_id, &ast)];
    let file_paths = vec![(file_id, "src/test.writ")];
    let (resolved, resolve_diags) = writ_compiler::resolve::resolve(&asts, &file_paths);
    assert!(resolve_diags.iter().all(|d| d.severity != Severity::Error),
            "resolve errors: {:?}", resolve_diags);
    writ_compiler::check::typecheck(resolved)
}

fn has_type_error(diags: &[Diagnostic], code: &str) -> bool {
    diags.iter().any(|d| d.code == code && d.severity == Severity::Error)
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Monolithic type checker in single file | Modular: `ty.rs`, `infer.rs`, `unify.rs`, `check_expr.rs` etc. | Phase 22 established the module-per-concern pattern | Easier to test and extend independently |
| Type as `String` name | Interned `Ty(u32)` into `TyInterner` | Phase 23 decision | O(1) equality, cheap `Copy`, no string comparison |
| Mutable AST annotation | Separate `TypedAst` IR | Phase 23 decision | Clean pipeline boundary; codegen sees only typed nodes |

**Deprecated/outdated:**
- `ResolvedType::GenericParam(String)`: The name-based generic param representation from Phase 22's IR is used as input to Phase 23 but converted to index-based `TyKind::GenericParam(u32)` during type checking. The name-based form does not survive into `TypedAst`.
- `ResolvedType::PreludeType(String)` / `ResolvedType::PreludeContract(String)`: These string-tagged forms from the resolve IR get fully materialized into `TyKind::Option(Ty)`, `TyKind::Result(Ty, Ty)`, etc. during type checking.

---

## Open Questions

1. **Enum variant data not stored in DefMap**
   - What we know: `DefEntry` has `DefKind::Enum` but no variant names or payload types. Phase 22 collector didn't need them.
   - What's unclear: Should we extend `DefEntry`, add a parallel sidecar table, or re-traverse the original AST decls at type check startup?
   - Recommendation: Re-traverse the `NameResolvedAst.decls` at the start of type checking to build a `TypeEnv` table mapping `DefId -> TypeSig` (function signatures, struct fields, enum variants). This is Plan 23-01's first task.

2. **Impl block association not materialized**
   - What we know: `def_map.impl_blocks` is a flat `Vec<DefId>`. The impl's target type and optional contract are in the original AST but not in the DefMap.
   - What's unclear: Best indexing structure.
   - Recommendation: Build `ImplIndex: FxHashMap<DefId, Vec<ImplEntry>>` where `ImplEntry { impl_id, contract_def_id: Option<DefId>, method_sigs }` at type check startup. Keyed by target type's DefId.

3. **ena snapshot/rollback necessity**
   - What we know: STATE.md has a pending todo: "Validate whether ena snapshot/rollback is needed or if forward-only type checking suffices."
   - What's unclear: Writ has no backtracking inference (no overloaded `+` that needs to try multiple impls). Generic call sites unify from left to right.
   - Recommendation: Start without snapshot/rollback. ena supports it but Writ's inference is simple enough that forward-only should work. Revisit only if inference tests reveal backtracking needs.

4. **Error code range for type errors**
   - What we know: Phase 22 used E0001–E0007, W0001–W0004.
   - What's unclear: Continue from E0008 or start a new decade/century?
   - Recommendation: Start type errors at E0100 (readable signal that "this is a type error"), warnings at W0100. Leaves room in E0008-E0099 for future resolve errors.

---

## Sources

### Primary (HIGH confidence)
- Writ codebase (`writ-compiler/src/resolve/`, `writ-diagnostics/src/`) — direct inspection of existing API, patterns, and conventions
- `language-spec/spec/06_5_type_system.md` — Writ type system rules
- `language-spec/spec/08_7_variables_constants.md` — let/let mut mutability rules
- `language-spec/spec/09_8_structs.md` — struct construction, lifecycle hooks
- `language-spec/spec/10_9_enums.md` — enum variants, exhaustive matching, Option/Result
- `language-spec/spec/11_10_contracts.md` — contract definitions, builtin contracts, Into<T>
- `language-spec/spec/12_11_generics.md` — generic call sites, type arg inference
- `language-spec/spec/13_12_functions_fn.md` — return semantics, block values, for/while
- `language-spec/spec/14_13_dialogue_blocks_dlg.md` — (not examined; dialogue not in type checking scope)
- `language-spec/spec/15_14_entities.md` — entity construction, component access typing
- `language-spec/spec/19_18_error_handling.md` — ? / ! / try rules and domains
- `language-spec/spec/20_19_nullability_optionals.md` — T? = Option<T>, null keyword
- `language-spec/spec/40_2_11_construction_model.md` — new Type {} IL sequence
- `language-spec/spec/41_2_12_delegate_model_closures_function_values.md` — closure captures, delegate model
- `.planning/phases/23-type-checking/23-CONTEXT.md` — locked implementation decisions

### Secondary (MEDIUM confidence)
- `ena` 0.14.4 — union-find crate used by rustc; API known from training data, consistent with project decisions in STATE.md
- Project pattern: `FxHashMap` + `id-arena` convention from Phase 22 codebase

### Tertiary (LOW confidence)
- None — all key claims verified against codebase or spec.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already in workspace or are direct follow-ons to existing deps
- Architecture: HIGH — mirrors Phase 22's module structure, verified against existing code
- Pitfalls: HIGH — derived from direct inspection of what the DefMap does and does not store after Phase 22
- Code examples: HIGH — derived from existing diagnostic/scope/def_map code in the repository

**Research date:** 2026-03-02
**Valid until:** 2026-04-02 (stable domain — spec and codebase do not change rapidly)
