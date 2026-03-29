# Domain Pitfalls

**Domain:** Adding generic constraints, collections, iterator protocol, string utilities, and diagnostics polish to an existing Writ language toolchain (v13.0 Standard Library & Language Ergonomics).
**Researched:** 2026-03-29
**Confidence:** HIGH — derived from direct codebase inspection of all affected layers (writ-compiler/src/check/, writ-runtime/src/dispatch/, writ-runtime/src/gc.rs, writ-diagnostics/) and verified against known patterns in Rust generics, CLR, and game scripting language implementations.

---

## Critical Pitfalls

### Pitfall 1: Generic Constraints Added to ena Unification Without Propagating Into Substitution

**What goes wrong:**
The existing `instantiate_generic_fn` in `writ-compiler/src/check/infer.rs` creates fresh `InferVar` values for each generic parameter and substitutes them into param/return types. `FnSig.bounds` already exists as `Vec<Vec<DefId>>` (bounds[i] = required contracts for generics[i]). When generic constraints are added, the bounds field must be checked at each call site — but `instantiate_generic_fn` currently ignores bounds entirely. Adding constraint syntax without wiring up enforcement at call sites means constraint violations silently compile to broken IL, failing only at runtime when `CALL_VIRT` cannot find an impl for the wrong type.

**Why it happens:**
`substitute()` in `infer.rs` is a purely structural type substitution — it only replaces `GenericParam(i)` with the inferred `Ty`. Bounds enforcement is a separate check that must run after the inferred type is resolved. It is easy to add the `bounds` field to `FnSig`, add parsing support, and ship the feature without ever connecting the enforcement step.

**Consequences:**
A function `fn sort<T: Ordered>(items: T[])` called with `T = MyStruct` where `MyStruct` does not implement `Ordered` produces no compile error. The emitter emits a `CALL_VIRT` for `Ordered.compare()` that fails at runtime. The error is a runtime crash with no source location, not a compile-time diagnostic.

**Prevention:**
After `unify()` resolves all `InferVar` values for a generic call, iterate `fn_sig.bounds[i]` for each resolved `Ty`. For each required contract `DefId`, check that the resolved type's `impl_index` entry includes that contract. Emit `TypeError::ConstraintNotSatisfied { type_name, constraint_name, call_span }` if it does not. This check belongs in `check_call` in `check_expr/call.rs`, after the `instantiate_generic_fn` + unification loop.

**Detection:**
- `bounds` field on `FnSig` is populated but never read after `instantiate_generic_fn`
- No test: call `fn f<T: SomeContract>(x: T)` with a type that lacks the contract impl, verify a compile error is produced
- Runtime crash with `contract not found` or `dispatch table miss` on a generic call

**Phase to address:**
Generic constraints spec + type-checker enforcement phase. Must happen before any collection methods use constraints.

---

### Pitfall 2: Iterator Protocol `for x in collection` Desugaring Conflicts With Cooperative Yielding

**What goes wrong:**
The `for x in collection` desugaring produces a loop over `Iterable<T>.next()`. In a cooperative scheduler, a `for` loop that iterates a large collection (e.g., all entities, a 10,000-element list) runs as a single uninterruptible stretch of execution — no yield points inside the generated loop body. If the loop body contains only pure computation (no `say`, no `spawn`, no explicit `defer`), the scheduler never gets to run other tasks. A `for x in all_entities { x.update() }` in a game update hook can starve the dialogue system.

**Why it happens:**
The generated desugaring looks like:
```
let iter = collection.iter();
loop {
    match iter.next() {
        Some(x) => { /* user body */ }
        None => break
    }
}
```
None of these generated instructions are yield points. The user's body only yields if the user explicitly wrote yield-inducing constructs. The compiler cannot insert `yield` into user loop bodies without changing the language semantics.

**Consequences:**
Entity update scripts that iterate large collections block all other tasks. Dialogue timing breaks. The cooperative scheduler provides fairness guarantees that pure-Writ loops silently violate.

**Prevention:**
Document clearly in the spec and iterator protocol section: `for x in collection` generates a tight loop with no implicit yield points. Users who need to yield during iteration must write explicit yield points in the loop body (`defer`, `spawn`, etc.). For large-iteration scenarios, provide a pattern recommendation (e.g., break large iterations into chunks across ticks using a `TaskHandle` + index). Do NOT attempt to auto-insert yield points — this changes observable semantics for short iterations.

Separately, verify that the generated `Iterable.next()` intrinsic body does not accidentally introduce yield points by involving the scheduler. The iterator must be a pure pop-and-return with no side effects on the task state.

**Detection:**
- No test that verifies a `for` loop over 1000 elements runs in a single scheduler timeslice
- Iterator protocol doc does not mention cooperative scheduling implications
- Golden test for `for x in list` desugaring does not show the absence of YIELD instructions in generated IL

**Phase to address:**
Iterator protocol design phase. The cooperative scheduling note must be in the spec before the desugaring is implemented.

---

### Pitfall 3: Pure-Writ Collections (`List<T>`, `Map<K,V>`, `Set<T>`) as Class Instances Hit GC Root Collection Per-Iteration

**What goes wrong:**
`List<T>` written in pure Writ will be a `class` (heap-allocated reference type). Internally it holds an `Array<T>` (already GC-managed). Each `List.push(item)` call updates a field on the class instance. The existing `MarkSweepHeap::collect()` traces live objects starting from the roots slice passed by the runtime. If a `List<T>` is held only in a local register (not in an entity field or global), it is reachable as a root from the active frame's register file — which is correct. However, the GC root collector must trace through `HeapObject::Struct` fields to find the inner `Array<T>`. If the `MarkSweepHeap` traces fields shallowly (only one level), the inner `Array` HeapRef inside the `List` class fields may be collected while the `List` itself is alive.

**Why it happens:**
`collect_value_refs` in `gc.rs` collects `HeapRef` values from a `Value`. The MarkSweep GC then traces each live HeapObject's fields recursively. If the implementation only traces the `Struct.fields` Vec one level (immediate children) without recursing into nested HeapObjects, the inner `Array` is only reachable transitively. As long as the GC correctly recurses (which is the standard mark algorithm), this is not an issue — but any optimization that short-circuits tracing can break it.

**Consequences:**
A `List<MyClass>` that is alive loses its internal backing `Array`. All subsequent element accesses return garbage or crash. The bug manifests non-deterministically depending on when GC is triggered.

**Prevention:**
Before implementing pure-Writ collections, write a GC correctness test: allocate a `class` with a field of type `Array<T>`, populate the array, trigger a manual GC, verify all array elements survive. This tests the exact transitive tracing path that `List<T>` will depend on. If this test fails, fix the GC tracing depth before writing collection classes.

**Detection:**
- No GC correctness test for transitively-nested heap objects before collection work begins
- `MarkSweepHeap::trace` or equivalent does not recurse into `HeapObject::Struct` fields
- Collection element access returns `Value::Void` after a GC cycle

**Phase to address:**
Array primitives phase (which precedes collection classes). Validate transitive GC tracing with array-in-struct test before any pure-Writ collection class depends on it.

---

### Pitfall 4: String Utility Methods Implemented as Writ-Side Functions Cannot Handle Non-ASCII Without Byte-Aware Iteration

**What goes wrong:**
The existing string representation is a Rust `String` (UTF-8) stored as `HeapObject::String`. The existing `s.len()` returns byte length (from v12.0 fix). String utilities like `split`, `starts_with`, `ends_with`, `contains`, `replace` must be implemented as intrinsics (Rust-side functions in `dispatch/intrinsics.rs`) rather than pure-Writ because:
1. Pure-Writ cannot express byte-level access to a String — there is no character-at-index operator.
2. If string indexing is added naively as `string[i]` = byte at index `i`, multi-byte UTF-8 sequences are split, producing invalid UTF-8 in substrings.

If string utilities are written as pure-Writ functions using array-style indexing, they will produce corrupted output for any non-ASCII input (emoji, accented characters, CJK text). Game content almost always contains non-ASCII.

**Why it happens:**
`len()` returning byte length rather than code-point count is already established behavior. Developers adding `split` in Writ code will naturally write `for i in 0..s.len()` — which iterates bytes, not characters. When the split boundary falls mid-codepoint, the resulting substring is not valid UTF-8. Rust will panic when trying to construct the String.

**Consequences:**
String utilities appear to work for ASCII-only test cases and fail silently (producing wrong output) or crash (Rust panic at FFI boundary) on non-ASCII production content.

**Prevention:**
All string utility methods (`split`, `trim`, `starts_with`, `ends_with`, `contains`, `replace`, `to_upper`, `to_lower`) must be Rust intrinsics. They operate on the `String` value from `HeapObject::String` using Rust's Unicode-correct string methods. Do NOT implement them in pure Writ using byte indexing. Add `IntrinsicId::StringSplit`, `StringTrim`, etc. variants following the existing `StringLen`, `StringConcat` pattern in `dispatch/intrinsics.rs`.

**Detection:**
- String utility test suite uses only ASCII input
- `split` or `trim` implemented as a loop over `s.len()` in Writ source
- No test: `" Héllo ".trim()` returns `"Héllo"` (tests the 2-byte é codepoint boundary)

**Phase to address:**
String utilities phase. All methods must be intrinsics from the start. Add a UTF-8 non-ASCII test for each utility.

---

### Pitfall 5: Generic Constraint Checking During ImplDef Emission Tries to Resolve Bounds Before `impl_index` Is Fully Populated

**What goes wrong:**
Generic constraint checking in the type checker needs to look up whether a concrete type `T` has an `impl` for a required contract. The lookup uses `type_env.impl_index: FxHashMap<DefId, Vec<ImplEntry>>`. This map is built during the `TypeEnv::build` pass, which processes declarations in the order they appear in the AST.

If a `class List<T: Iterable>` is declared before the `impl Iterable for List<int>` is processed, and the constraint check is triggered during `TypeEnv::build`, the `impl_index` for `List` is empty at check time — even though the impl exists in the source. The constraint check emits a false "constraint not satisfied" error.

**Why it happens:**
`TypeEnv::build` is a single forward pass over declarations. Impl entries are inserted as they are encountered. Constraints on generic type declarations that are validated during `TypeEnv::build` see a partially-populated `impl_index`.

**Consequences:**
Valid programs produce spurious `ConstraintNotSatisfied` errors depending on declaration order. Users reorder declarations to suppress errors (a hack), then discover reordering breaks other things.

**Prevention:**
Generic constraint checking must run after `TypeEnv::build` completes — not during it. The `typecheck()` function in `check/mod.rs` already has a two-phase structure: `TypeEnv::build` first, then `check_decl` on each declaration. Constraint checking belongs in the `check_decl` phase (specifically in `check_call` at each call site), not in the `env_build` phase. Never validate bounds during environment construction.

**Detection:**
- Test: declare `impl MyContract for MyType` after the function that uses `<T: MyContract>` — verify no false error
- `env_build.rs` contains constraint validation logic (it should not)
- Constraint errors depend on source file declaration order

**Phase to address:**
Generic constraints spec + type-checker enforcement phase. The two-phase separation (build env first, then check with complete env) is the primary correctness requirement.

---

### Pitfall 6: `for x in collection` Desugaring Generates a Method Call on a Temporary Iterator — Borrow Conflict With Existing Mutability Rules

**What goes wrong:**
The `for x in collection` desugaring calls `collection.iter()` to obtain an iterator object, then calls `.next()` on that iterator in a loop. In Writ's mutability model, `next()` on a mutable iterator advances internal state. If the iterator is a pure-Writ `class` with a mutable cursor field, calling `.next()` requires `mut self`. This means the `let iter = collection.iter()` binding must be declared `mut iter`. The desugaring must emit `mut iter`, not `let iter`.

If the desugaring generates `let iter = collection.iter()` (immutable), calling `iter.next()` where `next(self: mut Self)` is declared will fail the mutability check.

**Why it happens:**
`for` desugaring is mechanical. The temptation is to generate the simplest possible code:
```
let iter = collection.iter();
```
But `iter` must be mutable if the iterator modifies its own state. The desugaring is compiler-generated, so the compiler's own mutability checker will reject what the compiler generates — a self-inflicted error.

**Consequences:**
Every `for x in collection` produces `E0XXX: cannot call mut method on immutable binding` from the compiler's own mutability checker. The user has no way to fix this (they didn't write the `let iter` binding).

**Prevention:**
The `for` desugaring lowering pass (wherever it lives — likely a new case in `lower/stmt.rs` or the type checker) must generate `mut iter` for the iterator binding. Alternatively, if the iterator protocol uses shared-ownership semantics (the iterator is returned by value and calls are by value), the `next()` signature can take `self` by value. Design the `Iterable<T>` contract's `next()` signature to return `(Option<T>, Self)` (value-returning, immutable) or require `mut self` and generate `mut iter` in the desugaring. Choose one approach and be consistent.

**Detection:**
- First attempt to run a `for x in list` desugaring produces mutability errors from the compiler's own output
- `Iterable<T>` contract spec does not specify whether `next` takes `self` or `mut self`
- No golden test for `for` desugaring output verifying the generated IL's register assignment

**Phase to address:**
Iterator protocol contract design phase — resolve `next()` mutability before any desugaring implementation.

---

## Moderate Pitfalls

### Pitfall 7: Multi-Span Diagnostics in ariadne Require Both Files to Be in the `sources` Slice

**What goes wrong:**
The existing `render_diagnostics` in `writ-diagnostics/src/render.rs` adds `secondary_labels` to ariadne's `Report`. Each secondary label carries a `(FileId, span_range)`. ariadne requires that every `FileId` referenced by a label is present in the `sources: &[(FileId, &str, &str)]` slice provided to the renderer. If a secondary label points to a definition in a different file than the primary error, and that file is not included in the `sources` slice, ariadne panics (index out of bounds or silent wrong output).

**Why it happens:**
Single-file programs have worked fine: all spans are in the same file. Multi-span errors for "type parameter defined here" / "constraint violated here" will reference two files for the first time. The LSP's `publish_diagnostics_for` builds its `sources` slice from the current file's text. It does not include the definition file for the second label.

**Consequences:**
Multi-span generic constraint errors pointing to a definition in a dependency file crash the ariadne renderer. The crash happens in the CLI (`writ compile`). In the LSP, the analysis thread panics and the connection drops.

**Prevention:**
Before adding any multi-span errors, audit `render_diagnostics` and the LSP's diagnostic publishing to ensure that all `FileId` values in `secondary_labels` are included in the `sources` slice. The `run_pipeline` helper in `writ-cli` must collect source text for all files that contribute secondary spans. A safe default: when rendering, filter out secondary labels whose `FileId` is not in the sources slice and add them as notes instead. This degrades gracefully rather than panicking.

**Detection:**
- ariadne panics during `writ compile` when a type error spans two files
- LSP connection drops when a cross-file secondary label is emitted
- No test that renders a diagnostic with a secondary label in a different file

**Phase to address:**
Diagnostics polish phase (multi-span errors). Must validate source slice completeness before enabling multi-file secondary labels.

---

### Pitfall 8: `Map<K,V>` Key Hashing Requires a `Hashable` Contract — Cannot Use `FxHashMap` Without Providing a Writ-Side Hash Implementation

**What goes wrong:**
`Map<K,V>` requires that `K` supports equality and hashing. In Rust, the underlying storage uses `FxHashMap<Value, Value>` (since values need to be Writ `Value` types). But `Value` (or the struct/class it wraps) does not currently implement any Writ-level hashing contract. A user who creates `Map<MyStruct, int>` has no way to tell the runtime how to hash `MyStruct` without a `Hashable` contract or a built-in mechanism.

**Why it happens:**
Building a hash map in Writ without a `Hashable` contract means the runtime must use Rust's default hashing on `Value`, which hashes `HeapRef` by address. Two logically-equal `MyStruct` instances at different heap addresses will produce different hash keys — breaking map correctness.

**Consequences:**
`Map<MyStruct, int>` silently inserts duplicate keys. Lookup always fails to find entries that were inserted by a different heap allocation of the same logical key.

**Prevention:**
Option A: Restrict `Map<K,V>` to primitive key types (`int`, `string`, `bool`, `float`) where Writ-side equality and hashing are well-defined as intrinsics. Document this restriction explicitly. Add a compile-time constraint check: `<K: Hashable>` where `Hashable` is a builtin contract automatically implemented for primitives only.

Option B: Implement a `Hashable` contract with `fn hash(self) -> int` that users can implement for their types, and use it to build a Writ-side hash function that the VM's `Map` implementation calls via `CALL_VIRT`.

Option A is strongly preferred for v13.0. Option B is a future milestone addition.

**Detection:**
- `Map<MyStruct, int>` compiles without error
- Map operations with struct keys produce incorrect results
- No test: `Map<string, int>` insert + lookup succeeds; `Map<string, int>` with two equal keys produces one entry

**Phase to address:**
Collections design phase. Must define key type restrictions before implementing `Map`.

---

### Pitfall 9: LSP Partial Parsing With chumsky — Error Recovery Nodes Must Not Produce Orphaned `InferVar` Values

**What goes wrong:**
LSP partial parsing uses chumsky's error recovery to produce a partial AST when the user is mid-edit. The type checker runs on this partial AST to provide completions and hover info even during editing. When a partial expression contains a type-inference gap (e.g., the user has typed `let x =` with no RHS), the type checker creates an `InferVar` for the missing expression's type. If the `UnifyCtx` is not reset between LSP analysis runs, orphaned `InferVar` values from previous edits accumulate in the `InPlaceUnificationTable`. On the second analysis run, old `InferVar` values may unify with new ones via stale union-find entries, producing spurious type errors for valid code.

**Why it happens:**
The LSP analysis cache in `backend.rs` stores `Arc<AnalysisResult>` per URI. When the document changes, a fresh type-check is triggered. If the `UnifyCtx` is constructed fresh each time (as it currently is in `typecheck()`), this is not a problem. But if anyone reuses the `CheckCtx` across analysis runs (e.g., to cache partial results), the `UnifyCtx.table` will carry stale state.

**Consequences:**
Correct Writ code under active editing produces spurious "type mismatch" errors that disappear when the file is saved (full analysis). Users see incorrect red squiggles during editing.

**Prevention:**
Ensure `CheckCtx` — including `UnifyCtx` — is constructed fresh on every LSP analysis invocation. Never cache or reuse `CheckCtx` across document edits. Document this as an invariant in `check/mod.rs`. Add a comment: "A new `CheckCtx` must be created for every `typecheck()` call. The `UnifyCtx` state is not safe to reuse."

**Detection:**
- LSP shows spurious errors that disappear on save/reparse
- `UnifyCtx` constructed once and shared across multiple `typecheck()` calls
- No test: run two sequential analysis passes on the same file; verify error set is identical

**Phase to address:**
Diagnostics polish / LSP partial parse phase. Verify `CheckCtx` lifetime before adding partial-parse analysis.

---

### Pitfall 10: `to_upper` / `to_lower` on Strings — Locale-Dependent Behavior if Delegated to Rust's `to_uppercase`

**What goes wrong:**
Rust's `str::to_uppercase()` and `str::to_lowercase()` perform Unicode case mapping, which is locale-dependent for certain characters (Turkish dotless-i is the canonical example). Game scripts that use `to_lower()` to normalize user input for comparison (e.g., matching command strings) may produce different results depending on the host OS locale settings.

**Why it happens:**
The straightforward intrinsic implementation calls `s.to_uppercase()` on the Rust `String`. This is correct for most Latin text but produces unexpected results for Turkish locale (where `"I".to_lowercase()` is `"ı"` not `"i"`).

**Consequences:**
Command string matching fails in localized builds. The bug is locale-dependent and extremely hard to reproduce from a developer's machine.

**Prevention:**
Use `s.to_ascii_uppercase()` / `s.to_ascii_lowercase()` for the `to_upper` / `to_lower` intrinsics. These are locale-independent and match the behavior game scripts actually need (ASCII-range case normalization). Document this design decision: Writ's `to_upper`/`to_lower` are ASCII-range only. For full Unicode case mapping, a future `writ-std` function can use the Unicode-aware variants.

**Detection:**
- `to_lower` implemented via `str::to_lowercase()` (Unicode-aware, locale-sensitive)
- No test on non-ASCII input for `to_upper`/`to_lower`

**Phase to address:**
String utilities phase. The ASCII vs. Unicode-aware decision must be made before implementation.

---

### Pitfall 11: `List<T>` Backed by `Array<T>` — Growth Allocation Triggers GC, Which Can Free the Old Array Before the New Array Is Rooted

**What goes wrong:**
When `List<T>` grows beyond its capacity, the Writ-side implementation must allocate a new, larger `Array<T>` and copy elements. In pure Writ code, the new array allocation may trigger a GC cycle if the heap is near capacity. At the point of allocation, the new array is not yet stored in the `List` class's field — it is in a local register on the call stack. The old array is still in the `List`'s field. If the GC runs during the allocation, it should trace the old array through the `List` field (keeping it alive) and keep the new array through the local register (also alive). This should be correct if both paths are traced.

However, if the GC is triggered by the `alloc_array` call before the result is stored in a register (between the heap allocation and the assignment to the register in the VM dispatch loop), the new `HeapRef` is temporarily not rooted. This is a window of potential collection.

**Why it happens:**
The VM's GC integration point is the host calling `Runtime::collect()` externally. The existing `GcMode::Manual` design means GC never runs mid-instruction. This window is not an issue for `GcMode::Manual`. However, if v13.0 adds any automatic GC trigger (e.g., collecting after every N allocations), the window opens.

**Prevention:**
Do not add automatic GC triggers in v13.0. Keep `GcMode::Manual`. Document this constraint in the collection growth implementation. Any future automatic GC feature must first solve the between-instruction rooting gap for freshly-allocated objects.

**Detection:**
- Any automatic-GC trigger (byte-threshold or allocation-count-threshold) added alongside collection classes
- No test: allocate a large `List<T>`, force growth, trigger manual GC, verify all elements survive

**Phase to address:**
Collections implementation phase. Verify GC mode is still `Manual` before writing the growth path.

---

## Minor Pitfalls

### Pitfall 12: Fix Suggestions in ariadne Require a Specific `Span` That Covers the Exact Replacement Range

**What goes wrong:**
ariadne's `Report::with_fix()` API (if used) requires the replacement span to exactly cover the text to replace. If the diagnostic emitter stores `primary_span` as a span covering the entire expression but the fix should replace only a sub-token, the editor will replace too much text with the suggestion. For example, a "did you mean `mut x`?" suggestion on a mutability error should cover only the `let` keyword, not the entire binding declaration.

**Prevention:**
When adding fix suggestions, emit a dedicated span for the fix target (not the primary span). Store this as a separate field on the `Diagnostic` struct, or use ariadne's label-replacement API. Test each fix suggestion in the golden test suite to verify the replacement range is correct.

**Phase to address:**
Diagnostics polish phase (fix suggestions).

---

### Pitfall 13: `Set<T>` Deduplication Requires Value Equality — `Value::Ref` Equality Is Pointer Equality

**What goes wrong:**
`Set<T>` deduplicates elements using equality. Two `class` instances that are logically equal (same field values) may have different `HeapRef` addresses. The existing structural equality for `struct` types (v4.0) uses field-by-field comparison. Classes do not have auto-generated structural equality — they use reference identity by default. `Set<MyClass>` will store duplicates because `myClass1 == myClass2` tests reference identity.

**Prevention:**
For v13.0, restrict `Set<T>` to primitive element types (`int`, `string`, `bool`) where equality is unambiguous. Classes are reference-identity compared; using them as set elements is semantically ambiguous. Document: `Set<T>` requires `T` to be a primitive. Add a constraint: `<T: Equatable>` where `Equatable` is auto-implemented only for primitives and structs with structural equality.

**Phase to address:**
Collections design phase.

---

### Pitfall 14: Diagnostics Warning Levels — Adding `Severity::Note` Alongside Warnings Confuses `--deny-warnings` Behavior

**What goes wrong:**
The existing `Severity` enum has `Error`, `Warning`, and `Note`. A `--deny-warnings` CLI flag (if added in v13.0 for CI use) should promote `Warning` to `Error`. It must NOT promote `Note` to `Error`. If the implementation treats all non-Error severities as warnings for the deny flag, notes become hard errors — preventing compilation for informational messages.

**Prevention:**
When implementing `--deny-warnings`, explicitly check `severity == Severity::Warning` (not `severity != Severity::Error`). Add a test: `Note`-severity diagnostics are not affected by `--deny-warnings`.

**Phase to address:**
Diagnostics polish phase.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Generic constraints spec | Bounds checked during env_build (too early) | Validate bounds only in check_call/check_decl, never in env_build |
| Generic constraints type-checker | Missing enforcement after infer_vars resolve | Add bounds check loop in check_call after unify loop; write failing test first |
| Array primitives (alloc/copy/shrink) | GC tracing depth for nested heap objects | Write GC correctness test for array-in-struct before any collection code |
| `List<T>` class | Growth allocation window under auto-GC | Confirm GcMode::Manual; no automatic triggers; document growth as GC-safe only under manual mode |
| `Map<K,V>` | Key hashing using address for class keys | Restrict K to primitives + Hashable contract (ASCII-only in v13.0); design Hashable constraint |
| `Set<T>` | Reference equality for class elements | Restrict T to primitives + Equatable constraint |
| Iterator protocol contract design | `next()` mutability conflict with desugaring | Decide `next` signature (value-returning or `mut self`) before writing any desugaring |
| `for x in collection` desugaring | Tight loop starving cooperative scheduler | Document no-yield semantics; no auto-yield insertion; spec note required |
| String utilities | Non-ASCII corruption from byte-based indexing | All utilities as Rust intrinsics; non-ASCII test for each; `to_upper/lower` via `to_ascii_*` |
| Diagnostics polish (multi-span) | ariadne panics on missing FileId in sources slice | Audit sources slice construction in CLI and LSP before enabling cross-file secondary labels |
| Diagnostics polish (fix suggestions) | Replacement span covers wrong range | Dedicated fix-target span field; golden test for each suggestion |
| LSP partial parse | Stale InferVar from reused CheckCtx | CheckCtx created fresh per analysis invocation; invariant documented in check/mod.rs |
| Warning levels | `--deny-warnings` promoting Notes to errors | Explicit `== Warning` check; test Note severity is unaffected |

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Skip bounds enforcement after infer_vars resolve | Faster to ship constraint syntax without full enforcement | Constraint violations are runtime crashes with no source location | Never — enforcement and syntax must ship together |
| Implement `to_upper`/`to_lower` via Rust's Unicode-aware methods | Correct for all Unicode | Locale-dependent behavior breaks command normalization in localized builds | Never — use ASCII-range variants for game script string normalization |
| Allow `Map<MyClass, V>` with reference-identity key hashing | Simpler collection design | Logically-equal instances never find each other in the map; silent correctness bugs | Never — restrict key types to Hashable and document it |
| Generate `let iter` (not `mut iter`) in for-loop desugaring | Simpler codegen | Every `for` loop produces a mutability error from the compiler's own generated code | Never — must generate `mut iter` or use value-semantics iterator design |
| Add multi-span secondary labels without updating sources slice in CLI/LSP | Richer error messages faster | ariadne panics on any cross-file secondary label; LSP connection drops | Never — validate sources slice completeness before enabling |
| Implement string utilities in pure Writ using byte indexing | No new IntrinsicId variants needed | Corrupted output or Rust panic on any non-ASCII input | Never — string utilities must be intrinsics |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Generic bounds + `FnSig.bounds` | Bounds populated but never consumed after `instantiate_generic_fn` | Add bounds check loop in `check_call` after infer_vars are resolved via unify |
| `for` desugaring + mutability checker | Desugaring generates `let iter`; `.next(mut self)` fails mutability check | Design `next()` to be value-returning or generate `mut iter` in the desugaring lowering pass |
| `for` desugaring + cooperative scheduler | Iterator loop has no yield points | Document in spec; do not auto-insert; recommend explicit yields for long-running iterations |
| Pure-Writ `List<T>` + `MarkSweepHeap` | Transitive GC tracing not tested for class-containing-array | Write GC correctness test for nested heap structures before writing any collection class |
| `Map<K,V>` + `Value` hashing | HashMap keyed by `Value` uses address-based hash for HeapRefs | Restrict K to Hashable (primitives only in v13.0); document restriction |
| ariadne multi-span + multiple source files | Secondary label FileId not in sources slice causes panic | Always include all referenced FileIds in sources slice; add graceful fallback in render_diagnostics |
| `to_upper`/`to_lower` + non-ASCII | Rust `to_uppercase` is locale-sensitive | Use `to_ascii_uppercase`/`to_ascii_lowercase`; document as ASCII-range behavior |
| LSP partial parse + `UnifyCtx` state | Reusing `CheckCtx` across edits accumulates stale InferVars | Fresh `CheckCtx` per `typecheck()` call; invariant documented |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| `List.contains()` implemented as O(n) linear scan in pure Writ | Acceptable for small lists; game queries over entity lists are O(n) | Document complexity; for frequently-queried membership, recommend `Set<T>` | Any hot-path entity iteration that calls `list.contains()` per tick |
| `Map.get()` in pure Writ calls `Hashable.hash()` via CALL_VIRT each lookup | CALL_VIRT overhead on every map access | Implement `Map` using Rust intrinsics where possible; keep the hot path in Rust | Maps used in tight game loops (per-tick entity queries) |
| `Iterable.next()` returning `Option<T>` allocates an `Option` heap object on each call | Iterator loop over 1000 elements allocates 1000 Option objects | Design `next()` to return a sentinel value (null/void for exhaustion) instead of `Option<T>`; or use an intrinsic that avoids boxing | Any iterator-heavy computation |
| Generic constraint check at every call site via `CALL_VIRT` into `impl_index` | Compile time grows with number of generic call sites | Constraint checking is compile-time only; no runtime cost; this is not a runtime performance trap but a compile-time trap for programs with many generic calls | Programs with hundreds of generic function instantiations |

---

## "Looks Done But Isn't" Checklist

- [ ] **Constraint enforcement after resolution:** Call a generic function with a type that lacks the required contract. Verify `TypeError::ConstraintNotSatisfied` is emitted with the correct source span. If no error appears, bounds enforcement is missing from `check_call`.
- [ ] **Constraint ordering independence:** Declare a type's contract impl after the function that requires it. Verify no false "constraint not satisfied" error appears. If the error appears, env_build is doing premature constraint checking.
- [ ] **Iterator mutability desugaring:** Write `for x in myList` in a test program. Verify the compiler does not produce a mutability error on the generated iterator binding.
- [ ] **Iterator cooperative scheduling:** Write a `for` loop over 1000 elements with a pure arithmetic body. Verify the IL contains no YIELD instructions inside the loop. If YIELD appears, the desugaring is incorrectly inserting yield points.
- [ ] **GC transitive tracing:** Allocate a `List<SomeClass>`, add 10 elements, trigger manual GC. Verify all 10 elements survive. If any element is `Value::Void` post-GC, transitive tracing is broken.
- [ ] **Map key correctness:** Insert the same string key twice into a `Map<string, int>`. Verify the map has one entry, not two. If it has two, key hashing/equality is broken.
- [ ] **Non-ASCII string utilities:** Call `trim()` on `"  Héllo  "`. Verify result is `"Héllo"`. Call `to_lower()` on `"ABC"`. Verify `"abc"`. Call `to_lower()` on `"ABC"` with a Turkish locale environment variable — verify result is still `"abc"` (ASCII-range behavior).
- [ ] **Multi-span ariadne rendering:** Emit a diagnostic with a secondary label in a different file. Verify the CLI renders it without panicking. Verify the LSP does not drop the connection.
- [ ] **LSP partial parse stability:** Edit a file to have an incomplete expression (`let x =`). Verify the LSP shows exactly one error (incomplete expression), not spurious cascading type errors from stale InferVars.
- [ ] **`Set<T>` deduplication:** Insert the same integer value twice into a `Set<int>`. Verify the set has one element. Insert two class instances with identical field values. Verify the behavior is documented (two elements for reference types, one for primitives).

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Constraint enforcement missing from `check_call` | LOW | Add bounds check loop after unify in check_call; add test; no format or spec change needed |
| Constraint check fires during env_build (ordering-dependent false errors) | MEDIUM | Move constraint check to check_decl phase; add ordering-independence test; no format change |
| `for` desugaring generates immutable iter binding | LOW | Change generated binding to `mut iter`; or redesign Iterable.next() as value-returning; one-line fix if caught early |
| GC transitive tracing broken for nested heap objects | HIGH | Fix MarkSweepHeap::trace to recurse correctly; all collection tests will catch this; architecture change if tracing was never recursive |
| Map key identity bug with class keys | LOW | Restrict K to primitives with Hashable constraint; document; no runtime change needed if restriction is enforced at compile time |
| ariadne panic on missing FileId in sources slice | MEDIUM | Add fallback in render_diagnostics; audit all secondary label construction sites; add sources-completeness assertion |
| String utilities byte-corrupting non-ASCII | MEDIUM | Move implementations to Rust intrinsics; add non-ASCII tests; behavior change for existing callers if any used the broken Writ-side version |
| Stale InferVars across LSP analysis runs | LOW | Verify CheckCtx is constructed fresh per call; if not, add fresh construction; add test |

---

## Sources

- Direct codebase inspection: `writ-compiler/src/check/infer.rs` (instantiate_generic_fn, substitute — bounds not consumed), `writ-compiler/src/check/env.rs` (FnSig.bounds field present, env build order), `writ-compiler/src/check/unify.rs` (UnifyCtx single-pass, no bounds enforcement), `writ-compiler/src/check/mod.rs` (two-phase structure: TypeEnv::build then check_decl — correct place for bounds check), `writ-runtime/src/gc.rs` (GcMode::Manual, GcHeap::collect roots parameter, trace pattern), `writ-runtime/src/heap.rs` (HeapObject::Struct/Array nesting, BumpHeap alloc), `writ-runtime/src/dispatch/intrinsics.rs` (IntrinsicId pattern for string methods), `writ-diagnostics/src/render.rs` (ariadne sources slice, secondary_labels), `writ-diagnostics/src/diagnostic.rs` (Severity enum, SecondaryLabel), `writ-lsp/src/backend.rs` (analysis_cache per-URI, CheckCtx lifetime), `.planning/PROJECT.md` (v13.0 milestone scope, existing constraints)
- Rust string handling: [Rust std docs — str::to_uppercase](https://doc.rust-lang.org/std/primitive.str.html#method.to_uppercase) — HIGH confidence; documents locale-dependent behavior and the distinction from `to_ascii_uppercase`
- Cooperative scheduling interactions: Writ spec cooperative task design (PROJECT.md v2.0 task execution model), verified against existing `dispatch/concurrency.rs` yield-point implementation — HIGH confidence
- Generic constraint ordering in two-phase type checkers: [Rust compiler design book — name resolution ordering](https://rustc-dev-guide.rust-lang.org/name-resolution.html) — MEDIUM confidence; pattern reference for why constraint checking must happen after full environment construction
- ariadne multi-source rendering: [ariadne crate docs — Cache trait and multi-source reports](https://docs.rs/ariadne/latest/ariadne/) — MEDIUM confidence; requirement that all FileIds referenced by labels must be in the source cache

---
*Pitfalls research for: Writ v13.0 Standard Library & Language Ergonomics — adding generic constraints, collections, iterator protocol, string utilities, diagnostics polish to existing toolchain*
*Researched: 2026-03-29*
