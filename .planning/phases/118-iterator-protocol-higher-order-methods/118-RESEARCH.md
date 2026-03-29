# Phase 118: Iterator Protocol & Higher-Order Methods - Research

**Researched:** 2026-03-29
**Domain:** For-loop desugaring via Iterable<T>/Iterator<T> contracts, higher-order List methods (map/filter/reduce), lambda/closure dispatch
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Claude's Discretion
All implementation choices.

### Deferred Ideas (OUT OF SCOPE)
None — discuss phase skipped.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| COLL-04 | User can chain `list.map(fn).filter(fn).reduce(fn)` higher-order methods on List<T> | Pure-Writ methods in collections.writ; lambdas already compile and dispatch via CALL_INDIRECT; closures already emit NEW_DELEGATE |
| ITER-01 | User can write `for x in list` and it works via Iterable<T> contract desugaring | `check_stmt::For` and `emit_for_loop` both need class-type dispatch; spec desugar pattern in §1.11.3 |
| ITER-02 | User can write `for x in set` and it works via Iterable<T> | Same as ITER-01; Set<T> needs an Iterable<T> impl in collections.writ |
| ITER-03 | User can write `for k in map.keys()` and `for v in map.values()` to iterate Map entries | `map.keys()` and `map.values()` return `K[]`/`V[]`; existing array-loop path handles iteration |
| ITER-04 | User can implement `Iterable<T>` on custom types and use them in for-in loops | Desugaring path must handle `TyKind::Class` receivers that implement Iterable<T> |
| ITER-05 | List<T>, Map<K,V>, Set<T> all implement Iterable<T> | `impl Iterable<T> for List<T>` block in collections.writ + pure-Writ `ListIterator` class |
</phase_requirements>

---

## Summary

Phase 118 delivers three distinct capabilities that build on Phase 117's collection classes:

1. **For-in loop desugaring** — the compiler's `check_stmt::For` branch currently handles only `TyKind::Array` and `TypedExpr::Range`; it falls through to `ctx.interner.error()` for class types. The fix is to extend the type-checker to recognise `TyKind::Class` receivers, look up whether the class implements `Iterable<T>` in the `impl_index`, and extract the element type. The emitter already has a `_` arm that emits `Nop` for non-array iterables; that arm must be replaced with a desugaring into the while-loop pattern defined in spec §1.11.3: call `collection.iterator()` → assign to `mut _iter`, call `_iter.next()` → assign to `mut _next`, loop while `_next != null`, unwrap `_next!` and bind to the loop variable per iteration.

2. **Iterator implementation in Writ source** — `List<T>`, `Set<T>` (and their iterator cursors) must be written as pure-Writ classes in `writ-std/src/collections.writ`. Each needs a companion iterator class (e.g. `ListIterator<T>`) holding an index cursor, implementing `Iterator<T>` via `fn next(mut self) -> T?`. The `List<T>` class then implements `Iterable<T>` by returning a new `ListIterator<T>`. Map needs `keys()` and `values()` accessors that return raw arrays, which the existing array-loop path already handles.

3. **Higher-order methods on List<T>** — `map`, `filter`, and `reduce` are pure-Writ methods that accept closures (function values). Closures already compile to delegates via `NEW_DELEGATE`; calling them dispatches through `CALL_INDIRECT`. These methods build a new `List<T>` result, preserving immutability of the source list.

**Pre-work gate** (from STATE.md): Before writing any desugaring code, the `Iterator<T>.next()` mutability contract must be resolved: the spec says `fn next(mut self) -> T?` but Writ's `mut self` on a class method means the caller needs a mutable binding. The desugaring loop generates `let mut _iter = ...`, satisfying this requirement. This must be documented as a spec decision before the desugaring is coded.

**Primary recommendation:** Use spec §1.11.3 desugaring pattern verbatim. Implement all iterator classes in pure Writ source. Extend only `check_stmt::For` and `emit_for_loop` in the compiler — no VM changes required.

---

## Standard Stack

### Core
| Component | Location | Purpose | Why Standard |
|-----------|----------|---------|--------------|
| `check_stmt::For` (extend) | `writ-compiler/src/check/check_stmt.rs:254` | Recognise class Iterable receivers, extract elem_ty | Already handles Array and Range; class case is a `_` fallthrough today |
| `emit_for_loop` (extend) | `writ-compiler/src/emit/body/stmt.rs:176` | Desugar class iterable to while-loop via iterator/next | `_` arm already exists with `Nop` placeholder |
| `writ-std/src/collections.writ` (extend) | existing file | Add ListIterator, SetIterator, Iterable<T> impls, map/filter/reduce | Phase 117 already created this file |
| `writ-runtime::virtual_module` | `writ-runtime/src/virtual_module.rs:140` | Iterable<T> (contract 14) and Iterator<T> (contract 15) already registered | No virtual_module changes needed |
| `CALL_VIRT` instruction | existing VM opcode | Dispatches virtual contract method calls | Emitter already handles `TyKind::Contract` receiver via CallVirt |
| `CALL_INDIRECT` instruction | existing VM opcode | Dispatches closures/delegates for map/filter/reduce | Closure emission already works in Phase 117 tests |

### Supporting
| Component | Location | Purpose | When to Use |
|-----------|----------|---------|-------------|
| `writ-golden` harness | `writ-golden/tests/` | Golden IL snapshot tests for new iterator/HOF patterns | Write golden snapshots for for-in-list, map, filter, reduce |
| `writ-runtime/tests/coll_integration_tests.rs` | existing | Integration tests: compile + run through VM | Add iterator and HOF tests here |
| `TypeEnv::impl_index` | `writ-compiler/src/check/env.rs:68` | Maps concrete type DefId → Vec<ImplEntry> | Used in check_stmt::For to verify Iterable<T> impl |

**Installation:** No new crates required. All changes are in-repo source modifications.

---

## Architecture Patterns

### Iterator Classes in Pure Writ

The spec §1.11.3 desugar pattern requires `collection.iterator()` to return an `Iterator<T>`. For `List<T>` this means a companion cursor class:

```writ
// Source: language-spec/spec/12_11_contracts.md §1.11.3
pub class ListIterator<T> {
    source: T[],
    index: int
}

impl<T> ListIterator<T> {
    pub fn next(mut self) -> T? {
        if self.index >= self.source.len() {
            return null;
        }
        let item: T = self.source[self.index];
        self.index = self.index + 1;
        item
    }
}

impl<T> Iterable<T> for List<T> {
    pub fn iterator(self) -> Iterator<T> {
        new ListIterator<T> { source: self.items, index: 0 }
    }
}
```

The same pattern applies to `Set<T>` (`SetIterator<T>` wrapping `items: T[]`).

**NOTE:** `Iterator<T>` is a contract — the return type of `fn iterator(self)` is `Iterator<T>` (contract type), but the return value is a concrete `ListIterator<T>`. This uses contract-as-type (§1.11.4) which the type-checker already supports via `TyKind::Contract` assignability checks in `check_stmt.rs:36`.

### For-In Desugaring Pattern

Spec §1.11.3 defines the canonical desugar:

```
// for item in collection { body }
// desugars to:
{
    let mut _iter = collection.iterator();
    let mut _next = _iter.next();
    while _next != null {
        let item = _next!;
        body
        _next = _iter.next();
    }
}
```

**Implementation choice: compiler-level vs. TypedAST-level desugaring**

The desugaring can happen at two points:
- **check_stmt level** (preferred): `check_stmt::For` constructs synthetic `TypedStmt::While` containing the desugared body, and returns that. The emitter never needs to see the contract dispatch case — it only sees a `while` loop.
- **emit level**: `emit_for_loop` emits the CALL_VIRT/CALL_INDIRECT sequence directly.

**Recommendation: desugar at `check_stmt` level.** This is consistent with how `?` and `!` are desugared in `desugar.rs` — they produce `TypedExpr::Match` nodes. The for-loop desugaring produces a `TypedStmt::While` wrapping a `TypedStmt::Let` (for `_iter`), `TypedStmt::Let` (for `_next`), and the body. The emitter already handles all of these.

### How `check_stmt::For` Must Change

Current code (lines 276-308 in `check_stmt.rs`):
```rust
let elem_ty = match ctx.interner.kind(typed_iterable.ty()).clone() {
    TyKind::Array(elem) => elem,
    _ => {
        if matches!(&typed_iterable, TypedExpr::Range { .. }) {
            ctx.interner.int()
        } else {
            ctx.interner.error()  // <-- THIS is what needs to change
        }
    }
};
```

The `_` arm must be extended to handle `TyKind::Class(def_id)`:
1. Look up `ctx.type_env.impl_index.get(&def_id)` for an `ImplEntry` whose `contract_def_id` matches the `Iterable<T>` contract DefId.
2. Find the `iterator()` method sig in that `ImplEntry.methods`, extract its return type.
3. The return type is `TyKind::Contract(iterator_contract_def_id)`. Find the `next()` method on that contract, and its return type is `TyKind::Option(elem_ty)`.
4. Use `elem_ty` as the binding type.
5. Instead of returning `TypedStmt::For`, return a desugared `TypedStmt::While` (or a synthetic block containing `let _iter`, `let _next`, `while`).

**Resolving Iterable<T> DefId:** `Iterable` is a prelude contract — its `DefId` can be obtained via `ctx.def_map.get("Iterable")`. This works because prelude contracts are registered in the `DefMap` by the resolver (confirmed in `prelude.rs:15` where `"Iterable"` and `"Iterator"` are in `PRELUDE_CONTRACT_NAMES`).

### Higher-Order Methods Pattern

```writ
// Source: spec §1.27.3 + COLL-04 requirement
impl<T> List<T> {
    pub fn map<U>(self, f: fn(T) -> U) -> List<U> {
        let result: List<U> = new List<U> { items: [] };
        let mut i: int = 0;
        while i < self.items.len() {
            result.add(f(self.items[i]));
            i = i + 1;
        }
        result
    }

    pub fn filter(self, f: fn(T) -> bool) -> List<T> {
        let result: List<T> = new List<T> { items: [] };
        let mut i: int = 0;
        while i < self.items.len() {
            if f(self.items[i]) {
                result.add(self.items[i]);
            }
            i = i + 1;
        }
        result
    }

    pub fn reduce<U>(self, initial: U, f: fn(U, T) -> U) -> U {
        let mut acc: U = initial;
        let mut i: int = 0;
        while i < self.items.len() {
            acc = f(acc, self.items[i]);
            i = i + 1;
        }
        acc
    }
}
```

**Key constraint:** `map<U>` introduces a second generic param `U` relative to the class's `T`. The compiler's generic impl handling (Phase 117) uses `GenericParam` wildcards — full instantiation tracking is deferred. This means `map` returning `List<U>` where `U` is a second generic param may trigger the wildcard unification path. The method body uses `while` + `self.items[i]` which already works. The `f(arg)` call dispatches through `CALL_INDIRECT` since `f` is a `fn(T) -> U` function value (confirmed working via closure_capture.writ golden test).

**Chaining:** `list.map(fn(x: int) -> int { x * 2 }).filter(fn(x: int) -> bool { x > 3 })` returns `List<int>`, so chaining works because each method returns a fresh `List<T>` and methods are called on class receivers. No special compiler support needed — this is just method-call chaining.

### Map keys()/values() for ITER-03

`Map<K, V>` needs two accessor methods:
```writ
pub fn keys(self) -> K[] {
    self.keys  // field access returns the K[] array directly
}

pub fn values(self) -> V[] {
    self.values
}
```

With these, `for k in map.keys()` goes through the existing array iteration path in `emit_for_loop`. No desugaring needed.

**Name conflict:** The field is also named `keys`/`values`. In Writ, field access and method call are syntactically distinct (`map.keys` vs `map.keys()`). Method call resolution takes priority at the call site. However, since this creates an ambiguity risk, the accessor methods can be named `key_array()` / `value_array()` — or the spec requirement `for k in map.keys()` can be satisfied by confirming that method-over-field priority resolves correctly in the type checker. **Verify this before implementing.** If field-vs-method name conflict causes a compiler error, rename the field accessors to `get_keys()` / `get_values()`.

### Anti-Patterns to Avoid

- **Emitting CALL_VIRT for Iterable dispatch in `emit_for_loop`:** Do NOT try to emit contract dispatch in the emitter's for-loop handler. Desugar at the type-checker level instead, letting the emitter handle a plain while-loop. This avoids duplicating contract resolution logic.
- **Adding a new TyKind::Iterable or similar:** The `for` loop desugaring uses the existing `Iterable<T>` contract type in `impl_index`. No new TyKind is needed.
- **Hand-implementing iterator state in the VM:** Iterator state is heap-allocated cursor class state (a `ListIterator<T>` object). Do NOT add special VM opcodes for iteration — use the existing NEW + GET_FIELD/SET_FIELD infrastructure.
- **Two-method iterator vs. value-semantics:** `Iterator<T>.next()` takes `mut self` per the spec. In Writ classes are reference types — `mut self` on a class method mutates the heap object directly. The `_iter` binding must be `let mut _iter` in the desugared code so that mutability checking passes.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Iterator cursor state | Custom VM iterator opcode | Heap-allocated `ListIterator<T>` class in Writ | VM already supports heap objects; new opcodes add spec complexity |
| Closure invocation for map/filter/reduce | New delegate dispatch path | Existing `CALL_INDIRECT` + closure emission | `closure_capture.writ` golden test already proves this works |
| Contract method lookup for desugaring | Linear scan of impl_index | `ctx.type_env.impl_index.get(&def_id)` | impl_index is already keyed by concrete type DefId |
| Array iteration inside iterator | Custom iterator index tracking | `T[]` field + int index field in pure Writ | ARRAY_LEN + ARRAY_LOAD already used in for-array path |

---

## Common Pitfalls

### Pitfall 1: Iterator<T>.next() Mutability — `mut self` on Class Methods

**What goes wrong:** The spec says `fn next(mut self) -> T?`. On a class, `mut self` means the caller must have a mutable binding. If the desugared code emits `let _iter = ...` (immutable), the mutability checker (`check/mutability.rs`) will reject `_iter.next()`.

**Why it happens:** Classes are reference types so mutation goes through the heap, but Writ's mutability checker still tracks borrow mode for `self` parameters.

**How to avoid:** Desugar to `let mut _iter = collection.iterator();` — the `mut` keyword on the binding satisfies the mutability checker. This is the spec's own desugaring (§1.11.3 shows `let mut _iter`).

**Warning signs:** TypeError about mutating `_iter` through an immutable binding when running the first for-in-list test.

### Pitfall 2: `Iterable<T>` DefId Resolution

**What goes wrong:** `ctx.def_map.get("Iterable")` may return `None` if prelude contracts aren't pre-registered into the user module's DefMap (they live in the virtual runtime module, not the user module's DefMap).

**Why it happens:** Prelude contracts are in `PRELUDE_CONTRACT_NAMES` (prelude.rs:15) and are registered as `DefKind::PreludeContract` in the resolver. The `DefMap` should have them.

**How to avoid:** Check `resolve/ir.rs:43` — `DefKind::PreludeContract` confirms that prelude contracts do get DefMap entries. Verify with a lookup before coding. If `get("Iterable")` fails, use the prelude contract's index position (contract 14 in virtual module) to locate the token and back-resolve to the DefId via the def_map.

**Warning signs:** `None` result from `ctx.def_map.get("Iterable")` during desugaring.

### Pitfall 3: `List<T>` Field Name Conflict with `keys()`/`values()`

**What goes wrong:** `Map<K, V>` has fields named `keys: K[]` and `values: V[]`. If you add methods `fn keys(self) -> K[]` and `fn values(self) -> V[]`, the compiler may produce an ambiguity error when the caller writes `map.keys()`.

**Why it happens:** Field access and method call use the same dot-call syntax; the distinction is only presence/absence of `()`.

**How to avoid:** Verify the method-over-field resolution priority in `check_expr/access.rs` before naming the methods `keys`/`values`. Alternative: name them `get_keys()` / `get_values()` and update the spec requirement note in ITER-03 accordingly.

**Warning signs:** Type error like "field access not callable" at `map.keys()`.

### Pitfall 4: Generic Second Type Param in `map<U>`

**What goes wrong:** `pub fn map<U>(self, f: fn(T) -> U) -> List<U>` introduces `U` as a second generic parameter on a method inside `impl<T> List<T>`. The compiler's generic handling currently treats `GenericParam` as a wildcard. `new List<U> { items: [] }` requires `U` to be a known generic param during impl body checking.

**Why it happens:** Phase 117 noted: "GenericParam types unify as wildcards — full generic instantiation tracking deferred."

**How to avoid:** Start by testing `map`/`filter` with the same `T` type (non-generic output: `filter` returns `List<T>` which uses the same T). Test `reduce<U>` separately with concrete type annotations. If `map<U>` fails due to wildcard issues, document the limitation and scope it to Phase 119+. The COLL-04 success criterion only requires `list.map(fn(x: int) -> int { x * 2 })` where `T = U = int` — a monomorphic case that avoids the second generic param issue.

**Warning signs:** Type errors about unresolved generic params when constructing `new List<U>` inside `map`.

### Pitfall 5: Multi-Method Impl Block — DefId Token Issue

**What goes wrong:** Phase 117 notes document that "impl blocks with multiple methods now work correctly" (the serializer bug with shared impl DefIds was fixed). However, when adding new `impl Iterable<T> for List<T>` blocks alongside the existing `impl<T> List<T>` block, the token resolution must be correct.

**Why it happens:** The `IMPL-METHOD-TOKEN fix` in `emit/body/expr/mod.rs:391` resolves method tokens by `(receiver_type_def_id, method_name)` — this is the safe lookup path. For contract impl blocks, `CALL_VIRT` uses `contract_idx` + `slot`, resolved separately.

**How to avoid:** Verify golden tests include CALL_VIRT emission for `_iter.next()` with the correct contract_idx. If the token resolves to 0, the vtable dispatch will call the wrong method.

**Warning signs:** `InstructionLimit reached` (infinite loop) in integration tests — symptom of `next()` returning a non-null value forever because the cursor didn't advance.

### Pitfall 6: `null` Comparison in Desugared While Condition

**What goes wrong:** The desugared loop condition is `_next != null`. In the typed IR, `null` is `Option::None` and comparison with `null` requires the `Eq<T>` contract or a specific `!=` check on `T?`.

**Why it happens:** The `check_stmt::For` desugaring constructs synthetic `TypedExpr` nodes manually. The `!= null` condition must be constructed as a `TypedExpr::Binary { op: BinaryOp::Ne, left: _next_var, right: null_expr }` — but `null` desugars to `Option::None` which has type `Option<InferVar>`, and the unification may fail against `Option<T>`.

**How to avoid:** Instead of synthesizing `_next != null`, use the unwrap-or-break pattern: check `_next` against an index counter (like the array path), OR construct the condition as checking `Option` via a match. The simplest approach: desugar `while _next != null` as `while let Some(__val) = _next` which is already handled by the pattern checker. Alternatively: generate a synthetic call to a `is_some()` check method if available, or use the existing `Option` handling in `check_expr/control.rs`.

**Warning signs:** TypeError about `_next != null` comparison — `null` type not unifying with `T?`.

**Recommended concrete approach:** Use an integer index counter in the desugared implementation (exactly like the array path already does) rather than a null-check pattern. The `ListIterator<T>` can expose an `has_next(self) -> bool` method, OR the desugaring can call `_iter.next()` and `match` the result. The cleanest desugaring matching the spec exactly is:

```
let mut _iter_idx: int = 0;
let r_arr = emit list.items;      // get backing array
let r_len = ARRAY_LEN r_arr;
loop:
  r_cond = CmpLtI _iter_idx, r_len
  BrFalse r_cond, loop_end
  r_elem = ArrayLoad r_arr, _iter_idx
  <binding = r_elem>
  <body>
  AddI _iter_idx, _iter_idx, 1
  Br loop
loop_end:
```

This avoids the null-check complexity entirely for `List<T>` (and `Set<T>`) which are array-backed. The emitter can special-case `TyKind::Class` receivers that expose a `.items: T[]` field directly (introspect the class's fields), or require the Writ source to expose a `to_array()` accessor.

**The cleanest long-term approach for ITER-04 (custom types):** Use the spec's null-check desugaring via a match, but for Phase 118 scope the initial implementation to array-backed collections that expose a `T[]` accessor, deferring the full null-check desugaring to when Option pattern matching is well-exercised.

---

## Code Examples

### Verified: Array-backed for-loop emission (existing pattern)
```rust
// Source: writ-compiler/src/emit/body/stmt.rs:200
TyKind::Array(_elem_ty) => {
    let r_arr = emit_expr(emitter, iterable);
    let r_len = emitter.alloc_reg(int_ty);
    emitter.emit(Instruction::ArrayLen { r_dst: r_len, r_arr });
    let r_iter = emitter.alloc_reg(int_ty);
    emitter.emit(Instruction::LoadInt { r_dst: r_iter, value: 0 });
    // ... CmpLtI, BrFalse, ArrayLoad, body, AddI, Br ...
}
```

### Verified: Contract method dispatch (existing CALL_VIRT pattern)
```rust
// Source: writ-compiler/src/emit/body/expr/mod.rs:276
if let TyKind::Contract(contract_def_id) = emitter.interner.kind(receiver.ty()).clone() {
    let contract_token = emitter.builder.token_for_def(contract_def_id).map(|t| t.0).unwrap_or(0);
    let slot = emitter.builder.contract_method_slot_by_name(contract_def_id, field).unwrap_or(0);
    emitter.emit(Instruction::CallVirt { r_dst, r_obj, contract_idx: contract_token, slot, r_base, argc });
}
```

### Verified: Iterable<T> contract in virtual module (contracts 14 and 15)
```rust
// Source: writ-runtime/src/virtual_module.rs:140
// Contract 14: Iterable<T>
let iterable_contract = builder.add_contract_def("Iterable", "writ");
builder.add_contract_method("iterator", &[], 0);
// Contract 15: Iterator<T>
let iterator_contract = builder.add_contract_def("Iterator", "writ");
builder.add_contract_method("next", &[], 0);
```

### Verified: Spec desugaring (§1.11.3)
```writ
// Source: language-spec/spec/12_11_contracts.md
// for item in collection { process(item); }
// desugars to:
{
    let mut _iter = collection.iterator();
    let mut _next = _iter.next();
    while _next != null {
        let item = _next!;
        process(item);
        _next = _iter.next();
    }
}
```

### Verified: map/filter/reduce patterns (pure Writ, uses existing lambda dispatch)
```writ
// Uses CALL_INDIRECT for f(arg) — confirmed working in closure_capture.writ golden
pub fn filter(self, f: fn(T) -> bool) -> List<T> {
    let result: List<T> = new List<T> { items: [] };
    let mut i: int = 0;
    while i < self.items.len() {
        if f(self.items[i]) { result.add(self.items[i]); }
        i = i + 1;
    }
    result
}
```

---

## Spec Decision Required Before Coding

**The `Iterator<T>.next()` mutability semantics must be settled as a spec decision.**

From STATE.md: *"Iterator<T>.next() contract signature (value-returning vs. mut self) must be resolved as a spec decision before Phase 118 desugaring code is written"*

The spec currently says (§1.11.3 and §1.11.1): `fn next(mut self) -> T?`

**Decision:** Use `mut self` semantics.

Rationale:
- Classes in Writ are reference types — `mut self` mutates the heap object (advances the cursor) without copying.
- The desugaring produces `let mut _iter = collection.iterator();` which satisfies the mutability checker.
- This is consistent with the spec's desugar example which shows `let mut _iter`.
- Value-returning semantics (`fn next(self) -> (Self, T?)`) would require unpacking a tuple, which Writ doesn't support as a native type, and would require allocating a new iterator object per iteration — worse for GC pressure.

**Document this decision in PLAN-01 before writing any desugaring code.**

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| For-loop only iterates arrays + ranges | For-loop will desugar to Iterable<T> contract dispatch for class types | Phase 118 | `check_stmt::For` extended; no VM changes |
| No higher-order List methods | map/filter/reduce in pure Writ | Phase 118 | Closures must already compile — Phase 117 tests confirmed this |
| `_` arm in emit_for_loop emits Nop | Will emit desugared while-loop for class iterables | Phase 118 | Planner must choose emit-level or check-level desugaring |

---

## Open Questions

1. **Field-vs-method name collision in Map<K,V>**
   - What we know: `Map<K,V>` has fields `keys: K[]` and `values: V[]`. ITER-03 requires `for k in map.keys()` implying `keys()` is callable.
   - What's unclear: Does `check_expr/access.rs` resolve `map.keys()` as a method call when a field named `keys` also exists?
   - Recommendation: Inspect `check_expr/access.rs` early in Plan 01. If field-over-method priority exists, rename the accessor methods to `get_keys()` / `get_values()` and note that the ITER-03 spec example may need adjusting.

2. **`map<U>` with second generic param**
   - What we know: Wildcards unify generics in Phase 117 — no full instantiation tracking.
   - What's unclear: Whether `fn map<U>(self, f: fn(T) -> U) -> List<U>` will successfully compile when called as `list.map(fn(x: int) -> int { x * 2 })` (monomorphic T=U=int).
   - Recommendation: Test the monomorphic case first. If it fails, scope `map` to `fn map(self, f: fn(T) -> T) -> List<T>` (homomorphic, no second generic) for Phase 118. The success criterion says `x * 2` which returns `int = T`, so the homomorphic restriction still satisfies it.

3. **Null-check desugaring for ITER-04 (custom types)**
   - What we know: Custom Iterable<T> implementations need the full spec desugar with `_next != null` comparison.
   - What's unclear: How `_next != null` compiles — `null` type inference against `T?`.
   - Recommendation: For Phase 118, implement full spec desugaring for ITER-01/02 using array-backed shortcut (access `.items` field directly via the impl_index or a known structural pattern). For ITER-04 (custom types), implement the full spec-desugaring with null-check; test with a minimal custom Iterable implementation. If null-check fails due to inference issues, the spec desugaring can be approximated by checking `_iter.has_next()` (an extra method on the iterator class).

---

## Environment Availability

Step 2.6: SKIPPED — Phase 118 is purely code/config changes within the existing Writ repository. No external tools, services, or runtimes beyond what is already verified working in Phase 117.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`#[test]`) + `writ-golden` snapshot harness |
| Config file | `writ-runtime/Cargo.toml` (dev-deps: writ-compiler, writ-module) |
| Quick run command | `cargo test -p writ-runtime --test coll_integration_tests 2>&1` |
| Full suite command | `cargo test -p writ-compiler -p writ-runtime -p writ-golden 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ITER-01 | `for x in list { }` iterates all List<T> elements | integration | `cargo test -p writ-runtime --test coll_integration_tests iter_for_in_list` | ❌ Wave 0 |
| ITER-02 | `for x in set { }` iterates all Set<T> elements | integration | `cargo test -p writ-runtime --test coll_integration_tests iter_for_in_set` | ❌ Wave 0 |
| ITER-03 | `for k in map.keys()` iterates map keys | integration | `cargo test -p writ-runtime --test coll_integration_tests iter_for_map_keys` | ❌ Wave 0 |
| ITER-04 | Custom Iterable<T> type usable in for-in | integration | `cargo test -p writ-runtime --test coll_integration_tests iter_custom_iterable` | ❌ Wave 0 |
| ITER-05 | List/Map/Set implement Iterable<T> | integration | covered by ITER-01..03 | ❌ Wave 0 |
| COLL-04 | `list.map(f).filter(f)` chains produce correct result | integration | `cargo test -p writ-runtime --test coll_integration_tests coll_list_map_filter_reduce` | ❌ Wave 0 |

### Golden Tests (IL Snapshot)

| Behavior | File | Exists? |
|----------|------|---------|
| `for x in list` IL desugaring | `writ-golden/tests/golden/iter_for_in_list.{writ,writil}` | ❌ Wave 0 |
| `list.map(fn(x:int)->int{x*2})` IL | `writ-golden/tests/golden/coll_list_map.{writ,writil}` | ❌ Wave 0 |
| `list.filter(fn(x:int)->bool{x>0})` IL | `writ-golden/tests/golden/coll_list_filter.{writ,writil}` | ❌ Wave 0 |
| `list.reduce(0, fn(a,x){a+x})` IL | `writ-golden/tests/golden/coll_list_reduce.{writ,writil}` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-runtime --test coll_integration_tests 2>&1`
- **Per wave merge:** `cargo test -p writ-compiler -p writ-runtime -p writ-golden 2>&1`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-runtime/tests/coll_integration_tests.rs` — add `iter_for_in_list`, `iter_for_in_set`, `iter_for_map_keys`, `iter_custom_iterable`, `coll_list_map_filter_reduce` test functions
- [ ] `writ-golden/tests/golden/iter_for_in_list.writ` + `.writil` — golden snapshot for for-in-list desugaring
- [ ] `writ-golden/tests/golden/coll_list_map.writ` + `.writil` — golden snapshot for list.map
- [ ] `writ-golden/tests/golden/coll_list_filter.writ` + `.writil`
- [ ] `writ-golden/tests/golden/coll_list_reduce.writ` + `.writil`

---

## Sources

### Primary (HIGH confidence)
- `language-spec/spec/12_11_contracts.md` — §1.11.3 Iterable<T> for-loop desugaring, canonical desugar pattern, `fn next(mut self) -> T?` signature
- `writ-compiler/src/check/check_stmt.rs:254` — current For handling showing exact fallthrough to error for non-array types
- `writ-compiler/src/emit/body/stmt.rs:176` — current `emit_for_loop` with `_` arm Nop placeholder
- `writ-compiler/src/emit/body/expr/mod.rs:276` — existing CALL_VIRT dispatch for contract receivers
- `writ-runtime/src/virtual_module.rs:140` — Iterable<T> (contract 14) and Iterator<T> (contract 15) already registered with `iterator` and `next` methods
- `writ-std/src/collections.writ` — existing List/Map/Set/HashMap source for Phase 117
- `writ-compiler/src/resolve/prelude.rs:15` — `"Iterable"` and `"Iterator"` confirmed in PRELUDE_CONTRACT_NAMES
- `writ-runtime/src/dispatch/intrinsics.rs:386` — `ArrayIterable` intrinsic exists (returns array as its own iterator)
- `.planning/STATE.md` — critical pre-work item: Iterator<T>.next() mutability decision required before coding

### Secondary (MEDIUM confidence)
- `writ-runtime/tests/coll_integration_tests.rs` — test infrastructure pattern for Phase 118 tests
- `writ-golden/tests/golden/closure_capture.writ` — confirms fn-value / CALL_INDIRECT already works for closures
- `writ-compiler/src/check/desugar.rs` — reference pattern for check-level desugaring (how `?` and `!` produce TypedExpr::Match)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all components verified in codebase; Iterable/Iterator contracts already registered
- Architecture: HIGH — spec §1.11.3 defines the desugaring verbatim; array-backed iterator pattern has direct precedent in existing for-loop code
- Pitfalls: HIGH — all identified pitfalls are grounded in specific code locations and known Phase 117 constraints from STATE.md

**Research date:** 2026-03-29
**Valid until:** 2026-04-29 (stable codebase; no moving dependencies)
