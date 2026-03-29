# Architecture Patterns

**Domain:** Writ language toolchain — v13.0 Standard Library & Language Ergonomics
**Researched:** 2026-03-29
**Confidence:** HIGH (direct codebase inspection, all source files read)

## Context: Existing Architecture

The five-layer pipeline every v13.0 feature must integrate with:

```
writ-parser  →  writ-compiler  →  writ-module  →  writ-runtime
  (CST)        lower+resolve       (IL binary)      (register VM)
               +typecheck
               +codegen
```

**Key data types at each boundary:**

| Boundary | Type | Notes |
|----------|------|-------|
| parser → compiler | `cst::Program<'src>` | CST with spans, lifetime-tied to source |
| lower → resolve | `ast::Ast` (owned) | No lifetime. AstType, AstStmt, AstExpr, AstDecl |
| resolve → typecheck | `NameResolvedAst` + `DefMap` | DefId arena, by_fqn map |
| typecheck → codegen | `TypedAst` + `TyInterner` + `TypeEnv` | TypedDecl/TypedExpr/TypedStmt |
| codegen → runtime | `writ_module::Module` (binary) | 21 metadata tables, method bodies |

## Feature Integration Map

### Feature 1: Generic Constraints (`<T: Contract>`)

**Current state:** Generic params exist at all layers. `DefEntry.generics: Vec<String>` in DefMap. `TyKind::GenericParam(u32)` in type checker. `FnSig.bounds: Vec<Vec<DefId>>` field exists but is always empty (populated from AST but never enforced). `FnSig.generics: Vec<String>` populated. Instantiation via `InferVar` substitution in `infer.rs` works. Contract identity check exists in `unify.rs`. The `PreludeContract(String)` variant in `ResolvedType` already handles named contracts.

**What is missing:**
1. **Parser (writ-parser):** `GenericParam` in CST has no bound syntax. Need `: Contract + Contract` after the param name. `cst::GenericParam` must gain an optional `bounds: Vec<TypeExpr>` field.
2. **Lowering (writ-compiler/lower):** `AstGenericParam` must carry `bounds: Vec<AstType>`. Lowering from `cst::GenericParam` must populate bounds.
3. **Resolver (writ-compiler/resolve/resolver.rs):** `build_fn_sig` in `env_build.rs` must resolve bound `AstType` names to `DefId` and populate `FnSig.bounds[i]`.
4. **Type checker (writ-compiler/check):** At call sites where a generic is instantiated, after unification resolves `Infer(var)` to a concrete type, check that the concrete type has an `impl ContractBound for ConcreteType` entry in `TypeEnv.impl_index`. Emit `E00XX: type T does not satisfy bound Contract` diagnostic.
5. **Codegen (writ-compiler/emit):** No change needed — constraints are erased after type-check. IL already uses contract dispatch (`CallVirt`) for generic calls.

**Touch points:**

| File | Change |
|------|--------|
| `writ-parser/src/cst.rs` `GenericParam` | Add `bounds: Vec<Spanned<TypeExpr>>` |
| `writ-parser/src/parser/` | Parse `: T + U` bound syntax in generic param position |
| `writ-compiler/src/lower/` | Lower `GenericParam.bounds` to `AstGenericParam.bounds: Vec<AstType>` |
| `writ-compiler/src/ast/decl.rs` `AstGenericParam` | Add `bounds: Vec<AstType>` |
| `writ-compiler/src/check/env_build.rs` `build_fn_sig` | Resolve bounds to `Vec<DefId>`, populate `FnSig.bounds` |
| `writ-compiler/src/check/check_expr/call.rs` | After unification of generic call, enforce bounds |
| `writ-compiler/src/check/error.rs` | New `TypeError::BoundNotSatisfied` variant |
| `writ-diagnostics/src/code.rs` | New error code E00XX |

**Data flow:**

```
CST: GenericParam { name, bounds: [TypeExpr] }
  → AST: AstGenericParam { name, bounds: [AstType] }
    → DefEntry.generics (names only, unchanged)
    → FnSig.bounds: Vec<Vec<DefId>> (resolved contract DefIds per param)
      → call-site check: for each resolved InferVar, verify impl_index[concrete_ty] contains bound contract
```

**Constraint enforcement location:** `check_expr/call.rs` after `instantiate_generic_fn` + unification loop. Walk `fn_sig.bounds[i]`, resolve `InferVar[i]` to concrete `Ty`, verify `type_env.impl_index[concrete_def_id]` contains an `ImplEntry` with `contract_def_id == bound_def_id`.

---

### Feature 2: Array Primitives (alloc/copy/shrink on contiguous backing)

**Current state:** The IL already has a rich array instruction set: `NewArray`, `ArrayInit`, `ArrayLoad`, `ArrayStore`, `ArrayLen`, `ArrayAdd`, `ArrayRemove`, `ArrayInsert`, `ArraySlice` (0x0900–0x0908). The runtime handles these in `dispatch/objects.rs`. There is no `ArrayCopy` or `ArrayShrink` instruction today.

**What is needed:**
- Assess whether `ArraySlice` (0x0908) + `ArrayInit` cover the collection needs or if new instructions are required.
- For `List<T>`, `Map<K,V>`, `Set<T>` written in pure Writ, the backing array operations (`add`, `remove`, `len`, `[]`) already exist as IL instructions. Collections can call these directly via operator overloading.
- String utility methods (split, trim, etc.) need new string intrinsics in the runtime virtual module (writ-runtime/src/virtual_module.rs) and new `IntrinsicId` entries (writ-runtime/src/dispatch/mod.rs).

**Likely no new IL instructions needed** for collections — `ArrayAdd`/`ArrayRemove`/`ArrayLen`/`ArrayLoad`/`ArrayStore` are sufficient as a backing store for `List<T>`. String utilities need new intrinsic IDs and runtime dispatch cases but not new opcodes (they can go through `CallVirt` → intrinsic dispatch path already used for `int.add`, `string.len`, etc.).

**Touch points for string utilities:**

| File | Change |
|------|--------|
| `writ-runtime/src/virtual_module.rs` | Add `split`, `trim`, `starts_with`, `ends_with`, `contains`, `replace`, `to_upper`, `to_lower` intrinsic method entries on the String pseudo-TypeDef |
| `writ-runtime/src/dispatch/mod.rs` | Add `IntrinsicId` variants for each string method |
| `writ-runtime/src/dispatch/intrinsics.rs` | Implement each string intrinsic using Rust `str` methods |

---

### Feature 3: Collections (List<T>, Map<K,V>, Set<T> in pure Writ)

**Architecture decision: pure Writ source files, not Rust intrinsics.**

Collections are declared as standard `.writ` files in a new `writ-std/` crate (or a `stdlib/` directory compiled before user code). They implement contracts (`Iterable<T>`, `Index<K,V>`, `IndexSet<K,V>`, `Eq`, etc.) using existing operator overloading infrastructure.

**Key design:**
- `List<T>` wraps a raw `T[]` array. Methods: `add(v)`, `remove(i)`, `get(i)`, `len()`, `operator[](i)`, `operator[]=(i, v)`. Implements `Iterable<T>`.
- `Map<K,V>` wraps parallel `K[]` and `V[]` arrays with linear-scan lookup (simple for MVP). Implements `Index<K,V>` and `IndexSet<K,V>`.
- `Set<T>` wraps a `T[]` with uniqueness enforcement. Implements `Iterable<T>` and `Index<int,T>`.

**Integration approach:**
- These compile to normal IL like any user class.
- The compiler needs no special knowledge of `List<T>` — it's just a generic class with contracts.
- The stdlib module is loaded as a library module (the `RuntimeBuilder.libraries` vector already supports this).
- The compiler pipeline treats stdlib sources the same as user sources during multi-file compilation.
- `writ-cli` passes stdlib modules to the runtime at build/run time.

**New component:** `writ-std/` crate (or embedded `stdlib/` sources). No changes to core compiler layers.

**Touch points:**

| File/Component | Change |
|----------------|--------|
| `writ-std/` (new) | Pure-Writ source files for List, Map, Set |
| `writ-cli/src/` | Pre-compile and load stdlib module(s) alongside user module |
| `writ-runtime/src/runtime.rs` `RuntimeBuilder` | Already supports `libraries: Vec<Module>` — no change needed |

---

### Feature 4: Iterator Protocol (Iterable<T>, for-in desugaring)

**Current state:** `for x in expr` is already parsed (CST `Stmt::For`), lowered (AST `AstStmt::For`), type-checked, and emitted. However, it only works for `Array(T)` and `Range` — anything else falls through to `ctx.interner.error()` in `check_stmt.rs`. The `Iterable` and `Iterator` contract names exist in `prelude.rs` and `virtual_module.rs` but are not connected to `for` loop type-checking.

**What is missing:**
1. **Type checker (`check_stmt.rs`, `for` arm):** When iterable type is neither `Array` nor `Range`, look up `Iterable<T>` contract in the virtual module, check whether the iterable type implements it via `impl_index`, and if so extract the element type `T` from the type argument. Emit `TypeError::NotIterable` if not.
2. **Codegen (`emit/body/stmt.rs`, `emit_for_loop`):** Add a new dispatch arm for `TyKind::Class(def_id)` or `TyKind::Struct(def_id)` when the type implements `Iterable<T>`. Desugar to: call `.iter()` → get an `Iterator<T>` object → emit a while loop calling `.has_next()` and `.next()` on each iteration. This is standard `CallVirt` — no new instructions.
3. **Iterator contract implementations:** `List<T>.iter()` returns a `ListIterator<T>` class. These are implemented in pure Writ in `writ-std/`.
4. **Chainable adapters (map/filter/reduce):** Also pure Writ classes in `writ-std/`, no compiler changes needed.

**Desugaring pattern for contract-based iterables:**

```
for x in collection { body }
  →
let __iter = collection.iter();
while __iter.has_next() {
    let x = __iter.next();
    body
}
```

This desugaring happens in `check_stmt.rs` for the typed IR, then `emit/body/stmt.rs` emits `CallVirt` for `.has_next()` and `.next()` using the standard contract dispatch mechanism.

**Touch points:**

| File | Change |
|------|--------|
| `writ-compiler/src/check/check_stmt.rs` (for arm) | Add contract-based iterable path: check `impl_index` for `Iterable<T>`, extract element type |
| `writ-compiler/src/emit/body/stmt.rs` `emit_for_loop` | Add `TyKind::Class/Struct` arm: emit `CallVirt` desugaring pattern |
| `writ-std/` | `Iterator<T>` contract impls for List, Set, Range |

---

### Feature 5: String Utilities

**Architecture:** String utilities are a hybrid: the low-level operations (`trim`, `split` returning arrays, etc.) are Rust-side intrinsics (because Writ has no raw byte manipulation). Higher-level methods can call these intrinsics.

**Intrinsic dispatch path (existing):** `CallVirt` on a string value → `DispatchTable` finds it is a virtual module intrinsic → `execute_intrinsic(id, ...)` is called. This path already works for `int.add`, `string.len`, `string.concat`, etc.

**New intrinsic IDs needed:**

| Method | Signature | Notes |
|--------|-----------|-------|
| `split(sep: string) -> string[]` | Returns new array | Array allocation via runtime heap |
| `trim() -> string` | Strip whitespace | Pure string op |
| `trim_start() -> string` | Strip leading | Pure string op |
| `trim_end() -> string` | Strip trailing | Pure string op |
| `starts_with(prefix: string) -> bool` | | |
| `ends_with(suffix: string) -> bool` | | |
| `contains(substr: string) -> bool` | | |
| `replace(from: string, to: string) -> string` | | |
| `to_upper() -> string` | | |
| `to_lower() -> string` | | |
| `index_of(substr: string) -> int` | -1 if not found | |
| `substring(start: int, len: int) -> string` | | |

**These are additions to the virtual module only — no IL format change, no format_version bump.**

---

### Feature 6: Diagnostics Polish

**Current state:** `writ-diagnostics::Diagnostic` already supports:
- Primary span + label (`primary_file`, `primary_span`, `primary_label`)
- Multiple secondary labels (`secondary_labels: Vec<SecondaryLabel>`)
- Help text (`help: String`)
- Notes (`notes: Vec<String>`)

The data model is complete. The gaps are:

1. **Multi-span errors are not consistently used.** Many errors emit only a primary span and discard secondary context (e.g., type mismatch errors don't point to both the expected and actual type definition sites).
2. **Fix suggestions:** The `help` field exists but is often empty. Structured fix suggestions (not just text) would require a new `suggestions: Vec<FixSuggestion>` field in `Diagnostic` with `(file_id, span, replacement_text)` — this enables the LSP to offer code actions.
3. **Warning levels:** All warnings are undifferentiated. A `WarningLevel` enum on `Diagnostic` (or a filter in the CLI renderer) would allow `--deny warnings` style flag.
4. **LSP partial parse:** `writ-lsp/src/backend.rs` re-parses on every keystroke. For incomplete input (e.g., mid-type), the parser returns errors but provides partial CST. The LSP must tolerate `None` TypeEnv or partial TypedAst without crashing — this requires checking for `None` before accessing type-checking results.

**Touch points:**

| File | Change |
|------|--------|
| `writ-diagnostics/src/diagnostic.rs` | Add `suggestions: Vec<FixSuggestion>` to `Diagnostic`, add `FixSuggestion { file_id, span, replacement }` struct |
| `writ-diagnostics/src/code.rs` | Add missing error codes (E00XX for bound violations, etc.) |
| `writ-compiler/src/check/error.rs` | Add `BoundNotSatisfied`, populate secondary labels on existing errors |
| `writ-lsp/src/backend.rs` | Guard type-check-dependent handlers against partial-parse failure |
| `writ-cli/src/` `render.rs` | Render `suggestions` as annotated help lines |

---

## Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `writ-parser` | Lex + parse source → CST with spans | Produces `Program<'src>` for compiler |
| `writ-compiler/lower` | CST → owned AST, sugar lowering | Consumes `Program<'src>`, produces `Ast` |
| `writ-compiler/resolve` | Two-pass symbol collection → `DefMap`, `NameResolvedAst` | Consumes `Ast`, produces `NameResolvedAst + DefMap` |
| `writ-compiler/check` | Type inference + constraint enforcement → `TypedAst` | Consumes `NameResolvedAst + DefMap`, produces `TypedAst + TyInterner + TypeEnv` |
| `writ-compiler/emit` | `TypedAst` → IL binary via `ModuleBuilder` | Consumes all check outputs, produces `writ_module::Module` |
| `writ-module` | IL binary format, 21 tables, reader/writer | Shared by compiler and runtime |
| `writ-runtime` | Register VM, entity system, GC, scheduler | Loads `Module`, executes IL |
| `writ-runtime/virtual_module` | Programmatic stdlib metadata (contracts, intrinsics) | Built at startup, loaded as implicit library |
| `writ-std` (new) | Pure-Writ collection sources (List, Map, Set, iterators) | Compiled to `Module`, loaded as library |

## Data Flow Changes for v13.0

### Generic Constraints Flow

```
cst::GenericParam.bounds (new field)
  → AstGenericParam.bounds: Vec<AstType>  [lower/]
    → env_build::build_fn_sig → FnSig.bounds: Vec<Vec<DefId>>  [check/env_build.rs]
      → call-site bound check in check_expr/call.rs  [check/]
        (no downstream change — constraints erased before codegen)
```

### Iterator Protocol Flow

```
AstStmt::For { iterable, binding, body }
  → check_stmt.rs: type = Array? → elem_ty  [EXISTING]
                   type = Range? → int  [EXISTING]
                   type implements Iterable<T>? → T  [NEW PATH]
                     → TypedStmt::For { binding_ty: T, iterable, body }
                       → emit/body/stmt.rs: Array → index loop  [EXISTING]
                                            Iterable → CallVirt desugaring  [NEW PATH]
```

### String Utilities Flow

```
method call on string receiver
  → check_expr/call.rs resolves to virtual module method
    → codegen emits CallVirt with contract_idx=String, slot=method_slot
      → runtime DispatchTable routes to IntrinsicId::StringXxx
        → execute_intrinsic handles the Rust-side implementation
```

### Collections Flow

```
writ-std/*.writ compiled to Module(s)
  → loaded as library before user code runs
    → runtime Domain resolves List<T>, Map<K,V>, Set<T> type names
      → user code instantiates via `new List<T>()` → normal NEW + contract dispatch
```

## Suggested Build Order

The dependencies between features determine the correct build order:

**Phase 1: Generic Constraints**
Build this first because:
- Collections need `<T: Eq>` bounds on `Set<T>` and `Map<K,V>`
- Iterator protocol needs `Iterable<T>` bounds on `for-in`
- It touches the most layers (parser → lowering → resolver → typechecker)
- Provides the constraint-checking foundation everything else builds on

**Phase 2: Array Primitives + String Utilities**
Build before collections because:
- Collections are backed by arrays — string utilities exercise the same intrinsic dispatch path
- Both are self-contained runtime additions (new IntrinsicIds, new virtual module methods)
- No new IL instructions needed — validates that assumption early
- String utilities provide immediate user-visible value independent of collections

**Phase 3: Collections (List, Map, Set)**
Build after constraints and array primitives:
- Depends on `<T: Eq>` bound enforcement from Phase 1
- Depends on array backing store confirmed stable from Phase 2
- Pure Writ — tests the "stdlib as library module" integration end-to-end

**Phase 4: Iterator Protocol**
Build after collections:
- Collections implement `Iterable<T>` — need them to test `for x in list`
- The `for-in` desugaring change in `check_stmt.rs`/`emit/body/stmt.rs` is narrow but needs collections to validate
- `map`/`filter`/`reduce` adapters go here too (pure Writ in writ-std)

**Phase 5: Diagnostics Polish**
Build last because:
- No functional dependencies — pure quality improvement
- Benefits from all previous phases: better errors for bound violations, collection type mismatches, iteration errors
- LSP partial-parse guard is independent but benefits from having all new types registered

## Patterns to Follow

### Pattern: Narrow Type-Check Integration

Add new type paths to `check_stmt.rs` / `check_expr/call.rs` as additional match arms, not rewrites of existing arms. The existing `Array(elem)` and `Range` arms in the `for` handler are the model.

```rust
// In check_stmt.rs, for-loop arm
let elem_ty = match ctx.interner.kind(typed_iterable.ty()).clone() {
    TyKind::Array(elem) => elem,           // existing
    _ if is_range(...) => int_ty,           // existing
    TyKind::Class(def_id) | TyKind::Struct(def_id) => {
        // NEW: check impl_index for Iterable<T>
        find_iterable_elem_type(ctx, def_id, span)
            .unwrap_or_else(|| { emit_not_iterable_error(...); ctx.interner.error() })
    }
    _ => { emit_not_iterable_error(...); ctx.interner.error() }
};
```

### Pattern: Virtual Module Extension

Add new string methods following the exact pattern used for int/string arithmetic in `virtual_module.rs`: `add_intrinsic_method` returns a `MetadataToken`, which is then passed to `add_impl_method` on the type's implementation entry. New `IntrinsicId` variants go in `dispatch/mod.rs`, dispatch cases go in `dispatch/intrinsics.rs`.

### Pattern: Stdlib as Library Module

The `RuntimeBuilder.libraries: Vec<Module>` field and `Domain::load_library` already exist. The writ-cli compile+run path needs to pre-compile stdlib sources once and pass the resulting `Module` as a library. No runtime changes needed — only `writ-cli` coordination.

### Pattern: Bound Enforcement at Call Site

Check bounds after `instantiate_generic_fn` resolves `InferVar`s. At that point all type arguments are known (or `Infer(var)` if still unresolved — emit an ambiguity error). For each `(generic_idx, bound_def_ids)` in `fn_sig.bounds`, resolve `infer_vars[generic_idx]` to a concrete `Ty`, walk `type_env.impl_index[concrete_def_id]`, and verify at least one `ImplEntry.contract_def_id == bound_def_id` exists.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Hardcoding Collection Types in the Compiler

`List<T>`, `Map<K,V>`, `Set<T>` must NOT be special-cased in `check/` or `emit/`. They are user-visible class types defined in writ-std. The compiler knows them only through normal DefMap resolution. If collections are hardcoded in the compiler, stdlib becomes non-replaceable and the design violates the existing "pure-data `writ-module`" principle.

**Instead:** Collections compile like any generic class. The compiler only needs to know about `Iterable<T>` and `Iterator<T>` contracts (already in the virtual module).

### Anti-Pattern 2: Skipping GenericParam.bounds in Lowering

It is tempting to parse bounds and immediately resolve them in the resolver, skipping AST representation. This breaks the multi-pass pipeline — the lowering pass must produce `AstGenericParam.bounds: Vec<AstType>` first, then the resolver resolves AstType names to DefIds. Resolving names during lowering would couple the passes.

### Anti-Pattern 3: New IL Instructions for String Methods

Adding new opcodes (`StringSplit`, `StringTrim`, etc.) would require format_version bumps, assembler/disassembler updates, and reader/writer changes in writ-module. The intrinsic dispatch path via `CallVirt` handles this without touching the binary format.

### Anti-Pattern 4: Mutating check_stmt.rs For Loop for Non-Iterable Types

The current `for-in` emitter in `stmt.rs` has two specializations (Array and Range). Adding a third "Iterable contract" path must not change the behavior of the Array or Range paths — those are performance-critical hot paths that emit direct index loops, not indirected `CallVirt` chains.

## Scalability Considerations

| Concern | Now (MVP) | Later |
|---------|-----------|-------|
| Map<K,V> backing | Parallel arrays, O(n) lookup | Hash-array or sorted array in later stdlib iteration |
| Generic bound checking | Linear scan over impl_index per call site | Cache (type, contract) → bool after first check |
| Stdlib compilation | Pre-compile to .writil, ship as artifact | No change |
| Iterator protocol overhead | Two CallVirt per iteration (has_next + next) | Inline caching or specialization in later VM pass |

## New vs Modified Summary

### New Components

| Component | Location | Purpose |
|-----------|----------|---------|
| `writ-std/` crate | New top-level crate | Pure-Writ List, Map, Set, Iterator impls |
| `FixSuggestion` struct | `writ-diagnostics/src/diagnostic.rs` | Structured fix hint for LSP code actions |
| `BoundNotSatisfied` error | `writ-compiler/src/check/error.rs` | Generic constraint violation error |
| String intrinsic IDs | `writ-runtime/src/dispatch/mod.rs` | 12 new IntrinsicId variants |

### Modified Components (Minimal Diffs)

| Component | What Changes |
|-----------|-------------|
| `writ-parser/src/cst.rs` `GenericParam` | Add `bounds: Vec<Spanned<TypeExpr>>` field |
| `writ-compiler/src/ast/decl.rs` `AstGenericParam` | Add `bounds: Vec<AstType>` field |
| `writ-compiler/src/check/env_build.rs` | Populate `FnSig.bounds` from resolved bound types |
| `writ-compiler/src/check/check_expr/call.rs` | Add bound-check after generic instantiation |
| `writ-compiler/src/check/check_stmt.rs` | Add contract-iterable path in for-loop arm |
| `writ-compiler/src/emit/body/stmt.rs` | Add `CallVirt` desugaring arm in `emit_for_loop` |
| `writ-runtime/src/virtual_module.rs` | Add string methods + Iterable/Iterator impls for stdlib types |
| `writ-runtime/src/dispatch/intrinsics.rs` | Add string intrinsic implementations |
| `writ-lsp/src/backend.rs` | Guard type-env access against partial-parse `None` |

## Sources

- Direct inspection of `writ-compiler/src/check/ty.rs` — TyKind enum, TyInterner
- Direct inspection of `writ-compiler/src/check/env.rs` — FnSig.bounds field (exists, unused)
- Direct inspection of `writ-compiler/src/check/check_stmt.rs` — for-loop arm (Array + Range only)
- Direct inspection of `writ-compiler/src/emit/body/stmt.rs` — emit_for_loop (Array + Range only)
- Direct inspection of `writ-runtime/src/virtual_module.rs` — Iterable/Iterator contracts present
- Direct inspection of `writ-module/src/instruction.rs` — array opcodes 0x0900–0x0908 sufficient
- Direct inspection of `writ-diagnostics/src/diagnostic.rs` — multi-span structure already exists
- Direct inspection of `language-spec/spec/13_12_generics.md` — bound syntax `<T: Contract>`
- Direct inspection of `language-spec/spec/28_27_standard_library_builtins.md` — Iterable/Iterator contracts
