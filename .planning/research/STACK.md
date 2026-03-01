# Stack Research

**Domain:** Compiler middle-end — name resolution, type checking, IL codegen for the Writ language (v3.0 milestone)
**Researched:** 2026-03-02
**Confidence:** HIGH (core crates), MEDIUM (type inference strategy), HIGH (what to avoid)

---

## Context

This is the v3.0 milestone: adding name resolution, type checking, and IL codegen to the existing Writ compiler toolchain.

**Already validated and NOT re-researched:**
- Rust 2024 edition, workspace resolver "3"
- `chumsky 0.12` + `logos 0.16` (parser)
- `thiserror 2.0` (errors)
- `insta 1` (snapshot testing)
- `byteorder 1.5` (binary format)
- `clap 4.5` (CLI)
- `slotmap 1.1.1`, `indexmap 2.13` (runtime)

**Scope of this document:** What NEW crates or patterns are needed in `writ-compiler` and `writ-module` for the middle-end pipeline?

---

## Recommended Stack

### Core Technologies (New Additions Only)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `id-arena` | `2.3.0` | Type node storage with stable IDs | Name resolution and type checking require a type representation that can be referenced from multiple places without lifetimes. `id-arena`'s `Arena<T>` + `Id<T>` pattern gives stable integer identifiers — type equality is then `Id<Ty> == Id<Ty>` (pointer equality by another name, but without unsafe). No deletion needed: the type arena lives for the whole compilation. Preferred over `typed-arena` (returns references requiring `'arena` lifetime pollution throughout the type checker) and over `slotmap` (slotmap supports deletion, which adds overhead not needed here). |
| `rustc-hash` | `2.1.1` | Fast HashMap/HashSet for symbol tables and scope environments | Name resolution creates hundreds of small scope `HashMap`s. The stdlib default `SipHash` is cryptographic and measurably slower for short string keys. `rustc-hash` provides `FxHashMap` and `FxHashSet` — the same hasher used inside rustc, tuned for integer and short-string keys. A direct type alias swap: no API changes, just performance. |
| `ena` | `0.14.4` | Union-find for type variable unification | The Writ type system has local variable inference (`let x = 42; // inferred as int`). The standard approach is constraint-based: fresh type variables are assigned, then constraints (from assignments, function calls) are solved via unification. `ena::unify::UnificationTable` provides union-find with snapshot/rollback (needed when type checking branches). Extracted from rustc, actively maintained by the Rust compiler team. Alternative (hand-rolled union-find via `Vec`) is feasible but `ena` also provides the snapshot mechanism needed for backtracking. |
| `ariadne` | `0.6.0` | Rich diagnostic output with labeled source spans | Already used in `writ-parser` dev-dependencies for error display. The type checker produces errors with multiple labeled spans (e.g., "type `int` declared here, but used as `string` here"). `ariadne` handles multi-span, multi-file diagnostics with correct rendering of variable-width characters. The library is a sibling project to `chumsky` (same author, designed to work together). Moving it from dev-dep to a production dep in `writ-compiler` is the right step for phase 3. |

### Supporting Libraries (Evaluate Per Phase)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `bitflags` | `2.11.0` | Compact type modifier flags | Use if `TypeKind` variants need flag sets (e.g., mutability, constness, `extern` marker on declarations). A `Flags(u32)` field is cleaner than six separate `bool` fields and enables set operations. Only add if the number of boolean type properties exceeds 3–4. |
| `petgraph` | `0.8.3` | Dependency graph for declaration ordering | Use for the name resolution pass that must determine declaration order (structs referencing other structs, contract implementations needing their contract declared first). `petgraph::algo::toposort` gives ordering; `is_cyclic_directed` detects illegal cycles. Only add if declaration ordering proves non-trivial — a hand-rolled adjacency list may suffice for a language without recursive types at the module level. |
| `indexmap` | `2.13.0` | Already in workspace (via writ-runtime) | Use `IndexMap<String, SymbolId>` for scope frames where declaration order matters (e.g., struct fields must iterate in declaration order for IL field slot assignment). Already present in workspace — zero new dependency. |

### No New Development Tools

The existing dev toolchain (insta snapshots, cargo test) is sufficient. No new test frameworks needed.

---

## Architecture Patterns

### 1. Type Representation: Arena + Id, Not Rust References

**The problem:** A type checker that uses Rust references (`&'tcx TyKind`) requires threading the arena lifetime `'tcx` through every function signature. This is what rustc does — and it's a documented source of complexity ("the `'tcx` lifetime is everywhere").

**The Writ solution:** Use `id-arena`'s `Arena<TyKind>` and carry `Id<TyKind>` values. Type equality is `id_a == id_b`. Interning (deduplication) is layered on top: before inserting a new `TyKind`, check a `HashMap<TyKind, Id<TyKind>>` intern table. If found, return the existing `Id`. This eliminates `'tcx` from all signatures.

```rust
pub struct TypeInterner {
    arena: Arena<TyKind>,
    map: FxHashMap<TyKind, Id<TyKind>>,
}

impl TypeInterner {
    pub fn intern(&mut self, ty: TyKind) -> Id<TyKind> {
        if let Some(&id) = self.map.get(&ty) {
            return id;
        }
        let id = self.arena.alloc(ty.clone());
        self.map.insert(ty, id);
        id
    }
}
```

`TyKind` must implement `Eq + Hash` for the intern table. This is a clean pattern used by rust-analyzer (pre-Salsa era) and is appropriate for a language without lifetime parameters in its type system.

### 2. Scope Stack: Vec of FxHashMap (Ribs Pattern)

Rustc uses "Ribs" — a stack of scopes. Each Rib is pushed on entry to a block, function, or loop and popped on exit. Name lookup traverses inward-to-outward. This is the right pattern for Writ's scoping rules (§21).

```rust
pub struct ScopeEnv {
    // Each frame: name -> declaration Id
    ribs: Vec<FxHashMap<String, DeclId>>,
}

impl ScopeEnv {
    pub fn push(&mut self) { self.ribs.push(FxHashMap::default()); }
    pub fn pop(&mut self)  { self.ribs.pop(); }
    pub fn define(&mut self, name: String, id: DeclId) { ... }
    pub fn lookup(&self, name: &str) -> Option<DeclId> {
        self.ribs.iter().rev().find_map(|rib| rib.get(name).copied())
    }
}
```

No external crate needed. `Vec<FxHashMap<String, DeclId>>` is the entire implementation.

### 3. Type Inference: Two-Phase Constraint Collection + Unification

The Writ type system has local type inference only (function signatures and field types are explicit). This means:

1. **Phase A (constraint collection):** Walk expressions, assign fresh type variables via `ena::UnificationTable::new_key()`, emit constraints (`TypeVar(x) == Int`).
2. **Phase B (unification):** Solve constraints. `ena::UnificationTable::unify_var_var` / `unify_var_value`.
3. **Zonk:** Walk the AST again, replace all `TypeVar(x)` with their resolved types. Unresolved variables after this pass are a "type annotation required" error.

The `ena` snapshot/rollback is needed for type checking `if` branches independently.

### 4. IL Codegen: Extend writ-compiler to Call writ-module

The `writ-module` crate already provides a `ModuleBuilder` with a complete fluent API for all 21 IL table types. IL codegen is a new pass in `writ-compiler` that:

1. Receives the type-checked, fully-resolved AST.
2. Instantiates a `ModuleBuilder` from `writ-module`.
3. Traverses the AST depth-first, emitting instructions via `ModuleBuilder`.
4. Returns the finished `Module`.

**No new crate needed for codegen.** The builder is already written. The codegen pass belongs in `writ-compiler/src/codegen/` and adds `writ-module` as a dependency to `writ-compiler`.

### 5. Diagnostic Accumulation: Keep LoweringContext Pattern

The existing `LoweringContext` accumulates errors as `Vec<LoweringError>`. Extend this pattern: the `ResolveContext` and `TypeCheckContext` each accumulate errors. Callers collect all errors before failing. This matches the existing design and enables reporting multiple errors per compilation.

For rendering errors to the terminal, `ariadne` provides the `Report` + `Label` API. The existing `LoweringError` variants already carry `SimpleSpan`; this is directly compatible with ariadne's `Span` trait.

---

## Installation

```toml
# writ-compiler/Cargo.toml additions for v3.0:

[dependencies]
writ-parser  = { path = "../writ-parser" }
writ-module  = { path = "../writ-module" }    # NEW: for IL codegen
chumsky      = { version = "0.12.0", features = ["pratt"] }
thiserror    = "2.0"
id-arena     = "2.3"                          # NEW: type node arena
rustc-hash   = "2.1"                          # NEW: fast HashMaps for scope/symbol tables
ena          = "0.14"                         # NEW: union-find for type inference

[dev-dependencies]
insta        = { version = "1", features = ["ron"] }
ariadne      = "0.6"                          # Move from dev-dep to dep if rendering in-crate
```

Notes:
- `ariadne` stays in dev-dependencies if diagnostic rendering lives in `writ-cli`. Move it to a production dependency only if `writ-compiler` exposes a `render_diagnostics` API.
- `petgraph` is conditional on whether declaration ordering needs a full graph (see "Alternatives Considered").
- `bitflags` is conditional on type modifier complexity.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `id-arena 2.3` for type storage | `typed-arena 2.0.2` (references, `'arena` lifetime) | Use typed-arena if you specifically want to navigate the type graph with references rather than IDs, and are comfortable threading `'arena` through all type-checker functions. This is the rustc approach. For a smaller language without lifetime parameters in its type system, the `'arena` burden is not worth it. |
| `id-arena 2.3` for type storage | `salsa` (query-based compilation) | Use salsa if you need incremental compilation (IDE re-check on keypress). Salsa adds significant design complexity: all computation must be expressed as memoized queries, the database object is pervasive, and the learning curve is steep. Writ v3.0 is a batch compiler; incremental compilation is not required. Salsa is the right choice for a language server (future milestone), not for a correctness-first batch compiler. |
| `rustc-hash 2.1` for HashMaps | stdlib `HashMap` with `SipHash` | Use stdlib HashMap when security matters (e.g., accepting user-controlled keys in a web service). The symbol table in a compiler is not user-facing in a security sense; deterministic, fast hashing is preferred. |
| `ena 0.14` for unification | Hand-rolled union-find (`Vec<Option<TypeVar>>`) | Hand-rolling union-find is feasible for simple languages without the snapshot/rollback requirement. Add snapshots only when the type checker needs to explore speculative paths. If Writ's inference proves to be fully forward-only (no backtracking needed), a hand-rolled approach saves a dependency. Verify after implementing the basic type checker. |
| `Vec<FxHashMap>` scope stack | Persistent (immutable) scope maps | Persistent maps (e.g., `im` crate) allow cheap snapshot of the current scope by structural sharing. Useful when you need to freeze scope state for closures. Writ closures capture by value at declaration time; the type checker can record the scope snapshot when a closure node is encountered. Only needed if closure capture analysis becomes complex. |
| Extend `writ-compiler` for codegen | New `writ-codegen` crate | Create a separate `writ-codegen` crate if the codegen pass grows large enough to have its own tests and release cycle, or if multiple backends are planned. For the initial IL codegen pass targeting a single IL format, keeping it in `writ-compiler` is simpler. Split later if needed. |
| `ariadne 0.6` for diagnostics | `miette` | `miette` is a comprehensive diagnostic framework that also replaces `thiserror` and adds procedural macros. It is better suited for applications that want opinionated error printing out of the box. Writ already uses `thiserror` for error types and manages rendering separately; migrating to `miette` would require touching all existing error types. Not worth the migration cost. Use `miette` for a greenfield project. |
| `ariadne 0.6` for diagnostics | `codespan-reporting` | `codespan-reporting` is a lower-level library focused purely on rendering. `ariadne` is the more actively maintained successor and is explicitly designed to pair with chumsky (same author). Since writ-parser already uses ariadne in dev-deps, ariadne is the correct continuation. |

---

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `salsa` | Salsa's query system requires restructuring the entire compilation pipeline around its database object. This is a correct architectural choice for IDE-oriented incremental compilation, but v3.0 is a batch compiler where correctness is the goal, not incremental re-checking. Adding salsa to a non-incremental pipeline adds ~20 minutes of design work per feature with no benefit. | Flat, imperative passes in `writ-compiler/src/resolve/`, `writ-compiler/src/typecheck/`, `writ-compiler/src/codegen/`. |
| `lalrpop` / `pest` / second parser | The middle-end does not parse new syntax. The CST is already produced by `writ-parser`. Name resolution and type checking operate on `writ-compiler::ast` types, not text. | No new parser. |
| `rayon` for parallel type checking | Type checking is single-file for v3.0. Parallel type checking requires split compilation units and a more complex name resolution model (well-defined module interfaces before type checking). Premature for v3.0. | Sequential single-threaded passes. |
| `cranelift` / `inkwell` / `wasm-bindgen` | Code generation targets other than Writ IL are explicitly out of scope. The IL spec already defines the target format; `writ-module::ModuleBuilder` is the codegen backend. | `writ-module::ModuleBuilder` for IL emission. |
| `serde` on the type IR | The internal type representation (`TyKind`, `DeclId`, resolved AST) does not need serialization. The serialized artifact is the binary IL module (handled by `writ-module`). Adding serde to internal type IR adds derive macros and feature gating with zero benefit. | Binary module serialization via `writ-module::ModuleWriter`. |
| Separate diagnostic crate | Creating a `writ-diagnostics` crate for the diagnostic types adds cross-crate dependency complexity. The existing `LoweringError` pattern — error types in the crate that produces them, rendering at the boundary (CLI) — is the correct separation. | Keep error types in their producing crate (`writ-compiler`), render in `writ-cli`. |

---

## Crate Placement for v3.0 Features

| Feature | Crate | Rationale |
|---------|-------|-----------|
| Name resolution pass (`resolve/`) | `writ-compiler` | Operates on `ast::AstDecl`; produces `resolved::ResolvedAst` with `DeclId` annotations. Natural extension of the lowering pipeline. |
| Type checking pass (`typecheck/`) | `writ-compiler` | Operates on `resolved::ResolvedAst`; produces `typed::TypedAst`. Uses `ena` for unification. |
| Type representation (`ty/`) | `writ-compiler` | `TyKind`, `Id<TyKind>`, `TypeInterner`. Internal to the compiler. |
| IL codegen pass (`codegen/`) | `writ-compiler` | Operates on `typed::TypedAst`; emits via `writ-module::ModuleBuilder`. Adds `writ-module` as a dependency. |
| Diagnostic rendering | `writ-cli` | `writ-cli` imports `ariadne` and renders `Vec<CompilerError>` to stderr. Keeps rendering out of the library crate. |

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `id-arena 2.3.0` | Rust stable, no_std optional | No conflicts with Rust 2024 edition. MIT/Apache-2.0. |
| `rustc-hash 2.1.1` | Rust 1.64+ | Well under the Rust 2024 edition requirement (1.85+). Zero unsafe in user code. |
| `ena 0.14.4` | Rust stable | Maintained by the Rust compiler team. No known compatibility issues. |
| `ariadne 0.6.0` | `chumsky 0.12`, Rust stable | Same author, designed to interoperate. Already used as dev-dep in `writ-parser`. |
| `petgraph 0.8.3` | Rust 1.64+ | No conflict with workspace. Conditional addition. |
| `writ-module` (existing) | `writ-compiler` (new dep) | Currently `writ-module` has no dependency on `writ-compiler`. Adding the reverse edge (`writ-compiler` → `writ-module`) is safe — no cycle. |

---

## Sources

- [id-arena docs.rs 2.3.0](https://docs.rs/id-arena/2.3.0/id_arena/) — Arena<T> and Id<T> API, no-lifetime-in-callers pattern — HIGH confidence
- [typed-arena docs.rs 2.0.2](https://docs.rs/typed-arena/latest/typed_arena/) — Reference-returning alternative, lifetime tradeoff — HIGH confidence
- [rustc-hash docs.rs 2.1.1](https://docs.rs/rustc-hash/latest/rustc_hash/) — FxHashMap/FxHashSet, design rationale for compiler use — HIGH confidence
- [ena docs.rs 0.14.4](https://docs.rs/ena/latest/ena/) — UnificationTable, snapshot/rollback, union-find for type inference — HIGH confidence
- [ariadne docs.rs 0.6.0](https://docs.rs/ariadne/latest/ariadne/) — Report, Label, multi-span diagnostics — HIGH confidence
- [petgraph docs.rs 0.8.3](https://docs.rs/petgraph/latest/petgraph/) — toposort, is_cyclic_directed — HIGH confidence (version 0.8.3 confirmed)
- [indexmap docs.rs 2.13.0](https://docs.rs/indexmap/latest/indexmap/) — Already in workspace — HIGH confidence
- [bitflags docs.rs 2.11.0](https://docs.rs/bitflags/latest/bitflags/) — Type modifier flags — HIGH confidence
- [Rustc Dev Guide: Name Resolution](https://rustc-dev-guide.rust-lang.org/name-resolution.html) — Ribs/scope-stack pattern, namespace separation — HIGH confidence (authoritative)
- [Rustc Dev Guide: ty module](https://rustc-dev-guide.rust-lang.org/ty.html) — TyKind interning, arena allocation, pointer equality — HIGH confidence (authoritative)
- [Rustc Dev Guide: Type Inference](https://rustc-dev-guide.rust-lang.org/type-inference.html) — Constraint-based inference with union-find — HIGH confidence (authoritative)
- [Implementing a typechecker in Rust (RCL)](https://ruudvanasseldonk.com/2024/implementing-a-typechecker-for-rcl-in-rust) — Single-pass typechecking, Env struct, no-framework approach — MEDIUM confidence (single external article, consistent with rustc patterns)
- [Writ Language Spec §5](../../../language-spec/spec/06_5_type_system.md) — Type categories: primitives, arrays, structs, entities, enums, optionals, generics — authoritative
- [Writ Language Spec §11](../../../language-spec/spec/12_11_generics.md) — Generics with contract bounds, boxing for value types, dispatch table — authoritative
- [Writ Language Spec §21](../../../language-spec/spec/22_21_scoping_rules.md) — Scoping rules that constrain the resolver — authoritative

---

*Stack research for: Writ compiler middle-end (name resolution, type checking, IL codegen)*
*Researched: 2026-03-02*
