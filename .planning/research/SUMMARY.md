# Project Research Summary

**Project:** Writ v13.0 Standard Library & Language Ergonomics
**Domain:** Language toolchain feature extension — generic constraints, stdlib collections, iterator protocol, string utilities, diagnostics polish
**Researched:** 2026-03-29
**Confidence:** HIGH

## Executive Summary

Writ v13.0 is a capability-completion milestone on an existing, fully functional 10-crate Rust language toolchain. The codebase already contains the foundation for every v13.0 feature: generic bounds are parsed and type-checked but never emitted to IL; array mutation instructions exist in the VM but are not surfaced as compiler-resolvable dot-call methods; the `Iterable<T>` and `Iterator<T>` contracts are declared in the virtual module but the for-in emitter falls through to a `Nop` stub for non-array receivers; 11 string utility methods are absent from the `IntrinsicId` enum despite Rust `std::str` providing all implementations directly. The recommended approach is disciplined, additive surgery on existing subsystems — no new external dependencies, no new IL opcodes, no format version bump — filling the precise gaps that separate what the language can represent from what it can compile and run.

The correct build order is determined entirely by dependency chains: generic constraints must come first because `Map<K,V>`, `Set<T>`, and `List<T>.contains()` all require `<T: Eq>` or `<T: Hashable>` bound enforcement. Array primitives and string utilities can be developed in parallel as they are self-contained compiler-emitter and virtual-module extension work respectively. Collections are pure-Writ source files in a new `writ-std/` directory that compile exactly like user code — no compiler special-casing — and are loaded via the already-present `RuntimeBuilder.libraries` field. Iterator desugaring wires the existing `Iterable<T>` contract machinery into a new match arm in `check_stmt.rs` and `emit_for_loop`. Diagnostics polish is a capstone quality layer with no functional prerequisites.

The dominant risks are correctness traps at integration seams. Constraint enforcement that is syntax-only silently compiles broken IL. Iterator desugaring that generates an immutable binding for a `mut self` method produces compiler-internal mutability errors on every for-in loop. GC transitive tracing must be verified before collection classes depend on it. ariadne panics when a multi-span diagnostic's secondary label references a file absent from the renderer's sources slice. Every one of these risks has a concrete prevention strategy: write the failing test first, resolve the API contract design question before writing any code that depends on it, and audit integration points before enabling features that exercise them.

---

## Key Findings

### Recommended Stack

The entire v13.0 feature set is addressable within the existing locked dependency set. No new `Cargo.toml` entries are required or recommended. All Rust-side string implementations delegate to `std::str`. Array backing uses the existing `Vec`-backed `HeapObject::Array`. The IL binary format stays at `format_version 4` — table 14 (`GenericConstraintRow`) is already defined but was never written during emission. Adding 12 `IntrinsicId` variants is additive with no reader/writer changes. The `writ-std` crate holds pure-Writ source files compiled by the existing pipeline.

**Core technologies (all existing, all unchanged):**
- `writ-parser` / `chumsky 0.12` — generic bounds syntax already parsed; `GenericParam.bounds` populated in CST
- `writ-compiler` (lower / resolve / check / emit) — five-pass pipeline; all v13.0 changes are additive match arms or new field reads, never rewrites
- `writ-module` (IL binary, 21 tables) — `GenericConstraintRow` table 14 already defined; no format change needed
- `writ-runtime` (register VM, virtual module, GC) — `IntrinsicId` variants additive-only; `GcMode::Manual` must not change
- `ariadne 0.6.0` — no fix-suggestion API; fix hints rendered as `with_note` text; IDE fixes delivered via `lsp-types 0.94` `WorkspaceEdit`
- `tower-lsp 0.20.0` — `CodeAction` with `WorkspaceEdit` is the correct fix-delivery mechanism for the LSP

**What NOT to add:** No hash-map Rust intrinsic for `Map<K,V>` (breaks reflection/serialization inspection of map internals), no new opcodes for string methods (intrinsic dispatch path handles this without format changes), no `ariadne` version upgrade (0.7 is not stable), no automatic GC triggers alongside collection growth (unrooted HeapRef window during allocation).

### Expected Features

Full detail in `.planning/research/FEATURES.md`.

**Must have (table stakes — P1):**
- `<T: Contract>` single and multiple bounds (`T: A + B`), enforced at call sites with `ConstraintNotSatisfied` error
- Array primitives: surface all existing IL instructions as compiler-resolvable dot-call methods on `T[]` receivers
- `List<T>` — push, pop, get, set, len, contains (with `T: Eq` bound), written in pure Writ
- `List<T>` implements `Iterable<T>` — `for x in list` works end-to-end
- `Map<K: Hashable, V>` — get, set, contains_key, remove, keys — primitive keys only in v13.0
- `Set<T: Eq>` — add, remove, contains — primitives and structs with structural equality only
- String utilities: `split`, `trim`, `trim_start`, `trim_end`, `starts_with`, `ends_with`, `contains`, `replace`, `to_upper`, `to_lower` — all as Rust intrinsics
- Multi-span diagnostics used consistently at all new constraint-violation and non-iterable for-in error sites

**Should have (competitive differentiators — P2):**
- `List<T>.map<U>()`, `.filter()`, `.reduce<U>()` — pure Writ higher-order methods
- LSP completions and hover type info for `List`, `Map`, `Set` variables
- Diagnostics: `for x in expr` where expr is non-iterable shows secondary label pointing to the non-implementing type
- `--deny-warnings` CLI flag for CI pipelines

**Defer (v14+):**
- `List<T>.sort()` — requires `Ord` bound enforcement and a sort algorithm written in Writ
- Lazy iterator chains (Sequence/coroutine model)
- Warning suppression pragmas
- `EntityList<T>` with component query integration
- Partial generic type-arg LSP completion

**Confirmed anti-features (do not implement):**
- HashMap backed by Rust FFI — breaks reflection, serialization, stdlib introspection
- `any`-typed unparameterized `List` — destroys type safety; use `any[]` at reflection boundaries instead
- Auto-GC triggers alongside collection growth — unrooted HeapRef allocation window

### Architecture Approach

All v13.0 changes follow the existing five-layer pipeline (parser → lower → resolve → typecheck → emit → runtime) by adding narrow, non-breaking match arms to existing dispatch points. Collections are pure-Writ source files that compile like any user class — the compiler never special-cases `List`, `Map`, or `Set`. String utilities extend the virtual module via the established `add_intrinsic_method` / `IntrinsicId` / `execute_intrinsic` pattern. Generic constraint enforcement runs in `check_expr/call.rs` after `instantiate_generic_fn` + unification resolves `InferVar` values — critically, never during `TypeEnv::build` which sees a partially-populated `impl_index`. Iterator desugaring emits `CallVirt` for `.iterator()` and `.next()` in a new arm of `emit_for_loop`, leaving the existing Array and Range arms entirely untouched.

**Major components and their v13.0 delta:**
1. `writ-parser` — add `bounds: Vec<Spanned<TypeExpr>>` to `GenericParam`; parse `: Contract + Contract` syntax
2. `writ-compiler/lower` — propagate `AstGenericParam.bounds: Vec<AstType>` through the lowering pass
3. `writ-compiler/check/env_build` — resolve bound names to `Vec<DefId>`, populate `FnSig.bounds`
4. `writ-compiler/check/check_expr/call` — add bound-check loop after generic instantiation; emit `BoundNotSatisfied`
5. `writ-compiler/check/check_stmt` — add `TyKind::Class/Struct` arm: check `impl_index` for `Iterable<T>` contract
6. `writ-compiler/emit/body/stmt` — add `CallVirt` desugaring arm in `emit_for_loop` for contract-iterable types
7. `writ-runtime/virtual_module` — register 11 string intrinsic methods + `ArrayIterator` class + `Hashable` builtin contract
8. `writ-runtime/dispatch/intrinsics` — implement all 11 string + `ArrayIteratorNext` intrinsics using `std::str`
9. `writ-diagnostics/diagnostic` — add `FixSuggestion { file_id, span, replacement }` struct + `with_fix()` builder method
10. `writ-lsp/backend` — guard type-env access against partial-parse `None`; wire `CodeAction` for fix suggestions
11. `writ-std/` (new crate or directory) — pure-Writ `List<T>`, `Map<K,V>`, `Set<T>`, iterator impls; loaded as library module

### Critical Pitfalls

Full detail with detection checklists and phase assignments in `.planning/research/PITFALLS.md`.

1. **Bounds not enforced after unification (PITFALL 1 — CRITICAL)** — Parsing bounds and populating `FnSig.bounds` without connecting the enforcement step in `check_call` means constraint violations compile silently to runtime crashes with no source location. Prevention: the bounds check loop in `check_call` after `instantiate_generic_fn` + unify is mandatory; write a failing test (call generic fn with non-implementing type, expect `ConstraintNotSatisfied`) before writing any passing tests.

2. **Constraint check during env_build sees partial impl_index (PITFALL 5 — CRITICAL)** — If bounds are validated during `TypeEnv::build`, impls declared after the constrained function produce order-dependent false errors. Prevention: all bound enforcement belongs in `check_call`/`check_decl`, never in `env_build`. Test: declare `impl MyContract for MyType` after the function requiring `<T: MyContract>` and verify no false error.

3. **Iterator desugaring generates immutable binding for a `mut self` method (PITFALL 6 — CRITICAL)** — The compiler's own for-in desugaring will fail the compiler's own mutability checker if `let iter` is generated but `Iterator<T>.next()` requires `mut self`. Prevention: resolve the `next()` contract signature before writing any desugaring code — either use value-returning `next()` semantics or generate `mut iter` in the desugaring lowering pass.

4. **GC transitive tracing not verified before collections (PITFALL 3 — CRITICAL)** — A `List<T>` wrapping an inner `Array<T>` as a field depends on the GC tracing transitively through `HeapObject::Struct` fields. A shallow-tracing GC frees the inner array while the list is alive, causing non-deterministic corruption. Prevention: write and pass a `class-containing-array` GC correctness test before any collection class code exists.

5. **ariadne panics on secondary label with missing FileId in sources slice (PITFALL 7 — MODERATE)** — Multi-span errors pointing to a definition in a different file crash the ariadne renderer and drop the LSP connection. Prevention: audit `render_diagnostics` sources slice construction before enabling any cross-file secondary labels; add graceful fallback (demote missing-file labels to notes).

6. **String utilities via byte indexing corrupt non-ASCII (PITFALL 4 — CRITICAL)** — All 11 string utilities must be Rust intrinsics using Unicode-correct `str` methods. `to_upper`/`to_lower` must use `to_ascii_uppercase`/`to_ascii_lowercase` for locale-independence. Pure-Writ implementations using byte indexing produce corrupted output for any non-ASCII game content.

---

## Implications for Roadmap

Based on the dependency chain established in the architecture research, five phases are recommended. Phases 1-3 form the P1 core. Phases 4-5 are P2 completions.

### Phase 1: Generic Constraints Foundation
**Rationale:** Every downstream feature has a hard dependency here. `Map<K,V>` requires `K: Hashable`, `Set<T>` requires `T: Eq`, `List<T>.contains()` requires `T: Eq`. This phase touches the most layers (parser through type-checker through IL emitter) and sets the correctness baseline. The critical pitfalls (bounds-not-enforced after unification, premature env_build checking) are most safely addressed when this is the only active front with a focused test suite.
**Delivers:** `<T: Contract>` and `<T: A + B>` enforced at call sites; `BoundNotSatisfied` error with multi-span pointing to definition site and call site; IL emission for `GenericConstraintRow` table 14 (closes the existing latent bug); fix suggestions on bound errors ("consider implementing ContractName for Type").
**Addresses:** FEATURES P1 — generic bounds; STACK area 1 — constraint emission gap.
**Avoids:** PITFALL 1 (bounds-not-enforced — test-first approach mandatory), PITFALL 5 (env_build premature checking — bounds enforcement only in `check_call`/`check_decl`).
**Research flag:** Standard patterns. No per-phase research needed.

### Phase 2: Array Primitives + String Utilities
**Rationale:** Array primitive surfacing — compiler dot-call resolution for the existing `ArrayAdd`/`Remove`/`Insert`/`Contains`/`Slice` VM instructions — is the prerequisite for writing `List<T>` in pure Writ. String utilities are independent, high user-value, and validate the `IntrinsicId` extension pattern at low risk before the larger virtual-module iterator work. These two tracks share no code and can be developed in parallel within the phase.
**Delivers:** `.add(x)`, `.remove_at(i)`, `.insert(i, x)`, `.contains(x)`, `.slice(r)` usable on `T[]` receivers in the type-checker and emitter; 11 new `string` methods as Rust intrinsics; `ArrayIterator` class registered in virtual module; `Hashable` builtin contract auto-implemented for primitives.
**Addresses:** FEATURES P1 — array primitives, string utilities; STACK areas 2, 4.
**Avoids:** PITFALL 4 (all string utilities as intrinsics from day one; non-ASCII test required for each), PITFALL 10 (`to_ascii_*` variants for case methods, not Unicode-aware `to_uppercase`/`to_lowercase`).
**Research flag:** Standard patterns. Virtual module extension and IntrinsicId patterns are fully established.

### Phase 3: Collections (List, Map, Set)
**Rationale:** Depends on Phase 1 (bound enforcement for `Eq`/`Hashable`) and Phase 2 (array dot-call methods confirmed stable). Pure-Writ implementation exercises the "stdlib as library module" integration path end-to-end for the first time. `Map` and `Set` are scoped conservatively — primitive-only keys for `Map`, primitives + structs with structural equality for `Set` — to avoid the reference-equality and hash correctness traps that arise from using class instances as keys.
**Delivers:** `List<T>` (push, pop, get, set, len, contains), `Map<K: Hashable, V>` (get, set, contains_key, remove, keys), `Set<T: Eq>` (add, remove, contains) — all written in pure Writ, compiled as a library module, loaded before user code. GC correctness test for nested heap objects passes before any collection class is written.
**Addresses:** FEATURES P1 — collections; STACK area 6; ARCHITECTURE — stdlib as library module.
**Avoids:** PITFALL 3 (GC correctness test for class-containing-array passes before collection work begins), PITFALL 8 (primitive-only Map keys with `Hashable` constraint), PITFALL 11 (GcMode::Manual confirmed before growth path written), PITFALL 13 (Set restricted to Equatable types).
**Research flag:** One pre-work spike needed: compile a stub `.writ` file as a library module and verify `writ-cli` loads it before user code. This is a one-day validation, not a research gap.

### Phase 4: Iterator Protocol + Higher-Order Collection Methods
**Rationale:** Depends on Phase 3 (`List<T>` must exist to validate `for x in list`). The `for-in` desugaring change in `check_stmt.rs` and `emit_for_loop` is narrow but requires concrete `Iterable<T>` implementors to test against. `map`/`filter`/`reduce` are pure-Writ stdlib additions that complete the collection ergonomics story.
**Delivers:** `for x in collection` works for any type implementing `Iterable<T>`; `List<T>.map<U>()`, `.filter()`, `.reduce<U>()` as stdlib methods; LSP completions for collection types with inferred element type hover.
**Addresses:** FEATURES P1 — for-in Iterable desugaring; FEATURES P2 — map/filter/reduce, LSP completions.
**Avoids:** PITFALL 2 (document no-implicit-yield semantics in spec before implementation; do not auto-insert yield points), PITFALL 6 (`next()` mutability contract design must be resolved as a spec decision before any desugaring code is written), PITFALL 15 (`Option<T>` boxing overhead — use value/sentinel design for tight iteration loops).
**Research flag:** Design decision required before implementation begins: `Iterator<T>.next()` contract signature (value-returning vs. `mut self`). The cooperative-scheduler no-yield note must be in the spec. Neither is a research gap — both are deliberate choices that must be made and documented first.

### Phase 5: Diagnostics Polish + LSP Hardening
**Rationale:** No functional dependencies — benefits from all previous phases being complete because all new error types are now present. The fix-suggestion infrastructure and LSP partial-parse guard are pure quality improvements that make the entire v13.0 feature set more ergonomic.
**Delivers:** Structured `FixSuggestion { file_id, span, replacement }` field on `Diagnostic`; `DiagnosticBuilder::with_fix()` method; LSP `CodeAction` with `WorkspaceEdit` for applicable fix suggestions; LSP partial-parse hardening (last-good AST preserved for completions/hover during editing); consistent multi-span secondary labels on all v13.0 error types; `--deny-warnings` CLI flag.
**Addresses:** FEATURES P2 — diagnostics polish; STACK area 5; ARCHITECTURE — diagnostics data model gaps.
**Avoids:** PITFALL 7 (ariadne sources slice completeness audit before enabling cross-file secondary labels), PITFALL 9 (fresh `CheckCtx` per analysis invocation; invariant documented in `check/mod.rs`), PITFALL 12 (dedicated fix-target span separate from primary span; golden test for each suggestion), PITFALL 14 (explicit `== Warning` check for `--deny-warnings`; Note severity unaffected).
**Research flag:** Standard patterns. ariadne 0.6.0 fix API gap resolved by text-note approach + LSP WorkspaceEdit.

### Phase Ordering Rationale

- **Constraints before collections:** `Map`, `Set`, and `List.contains()` all require bound enforcement. Shipping collections without working bounds means they cannot correctly use their own contracts at call sites.
- **Primitives before stdlib:** The array dot-call emitter gap must be closed before pure-Writ collection code can use `array.add()` and `array.remove_at()`. String utilities exercise the IntrinsicId extension pattern at low cost, validating the pattern before iterator intrinsics depend on it.
- **Collections before iterator desugaring:** The type-checker needs concrete `Iterable<T>` implementors to validate the for-in extension path. Writing desugaring without a real collection to iterate produces incomplete integration coverage.
- **Diagnostics last:** The quality capstone is independent of all functional features and benefits from having all new error sites in place before the polish pass begins.

### Research Flags

Phases requiring a design decision or one-day spike before planning:
- **Phase 3 (Collections):** Write and pass the `class-containing-array` GC correctness test before any collection class code. Validate `writ-cli` library module pre-loading with a stub `.writ` file. Neither is a research gap — both are pre-work that blocks implementation.
- **Phase 4 (Iterator Protocol):** `Iterator<T>.next()` contract signature must be resolved as a spec decision before any desugaring code is written. The cooperative-scheduler no-yield note must be in the spec first.

Phases with standard, established patterns (no research needed):
- **Phase 1 (Generic Constraints):** Bound enforcement at call sites is a well-documented pattern; all infrastructure exists; Pitfalls 1 and 5 are known and preventable.
- **Phase 2 (String Utilities):** IntrinsicId extension pattern is fully established with zero ambiguity.
- **Phase 5 (Diagnostics):** ariadne fix API gap is resolved; LSP WorkspaceEdit patterns are standard.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All findings from direct codebase inspection; all Cargo.lock versions confirmed; no external research required |
| Features | HIGH | Primary sources: codebase + official C#/Kotlin/Swift/GDScript docs; feature gaps verified against actual missing IntrinsicId variants and absent match arms in source |
| Architecture | HIGH | All integration points identified from direct source inspection; data flow diagrams derived from actual types at each pipeline boundary; no assumptions about unread code |
| Pitfalls | HIGH | Derived from codebase inspection of actual partial implementations and GC design; cross-checked against CLR and Kotlin generics patterns; 14 pitfalls with specific file/function context |

**Overall confidence:** HIGH

### Gaps to Address

- **`Iterator<T>.next()` contract signature:** Research identified the mutability conflict (Pitfall 6) but prescribes two valid solutions — value-returning `(Option<T>, Self)` semantics vs. `mut self` with generated `mut iter`. This must be resolved as a spec/design decision at the start of Phase 4, not left to the implementing developer.

- **`writ-std` crate location and build integration:** Research recommends `writ-std/` as a new top-level crate or a `stdlib/` directory inside `writ-runtime`. The choice affects `writ-cli` pre-compile coordination. Validate with a stub `.writ` library file at the start of Phase 3 before writing any collection code.

- **`Hashable` contract scope:** The `Hashable` builtin contract does not yet exist in the virtual module. Phase 2 must define it as a builtin contract auto-implemented for `int`, `string`, `bool`, `float` only. This is a one-day virtual module addition, not a research gap, but it must be in Phase 2's scope definition.

- **`Option<T>` allocation in iterator tight loops (Pitfall 15):** `Iterator<T>.next()` returning `Option<T>` allocates a heap object on each call. For game-loop iteration over large collections this is a per-frame allocation budget concern. The Phase 4 spec decision on `next()` semantics must address this — prefer a value-sentinel design or an intrinsic-backed next that avoids boxing.

---

## Sources

### Primary (HIGH confidence — direct codebase inspection)
- `A:/dev/git/Writ/writ-compiler/src/check/env_build.rs` — `build_generic_bounds`, `FnSig.bounds` populated but unused downstream
- `A:/dev/git/Writ/writ-compiler/src/check/check_expr/call.rs` — `check_contract_bounds` + `instantiate_generic_fn` confirmed; `add_generic_constraint()` exists with no callsite
- `A:/dev/git/Writ/writ-compiler/src/emit/body/stmt.rs` — `emit_for_loop` Array + Range handled; `_ => Nop` stub confirmed
- `A:/dev/git/Writ/writ-runtime/src/virtual_module.rs` — `Iterable<T>` contract 14, `Iterator<T>` contract 15 confirmed; no `ArrayIterator` class present
- `A:/dev/git/Writ/writ-runtime/src/dispatch/mod.rs` — full `IntrinsicId` enum; no split/trim/starts_with/etc. variants present
- `A:/dev/git/Writ/writ-module/src/instruction.rs` — array opcodes 0x0900–0x0908 sufficient; no new array opcodes needed
- `A:/dev/git/Writ/writ-diagnostics/src/diagnostic.rs` — `DiagnosticBuilder`: `with_secondary`, `with_help`, `with_note` present; no `with_fix`
- `A:/dev/git/Writ/writ-diagnostics/src/render.rs` — ariadne rendering confirmed; no fix suggestion output
- `A:/dev/git/Writ/Cargo.lock` — all locked versions confirmed
- `A:/dev/git/Writ/.planning/PROJECT.md` — v13.0 scope and out-of-scope boundaries

### Secondary (HIGH confidence — official external docs)
- [Microsoft Docs: Constraints on type parameters (C#)](https://learn.microsoft.com/en-us/dotnet/csharp/programming-guide/generics/constraints-on-type-parameters) — generic bounds patterns and multi-constraint syntax
- [Kotlin Docs: Iterators](https://kotlinlang.org/docs/iterators.html) — `Iterable`/`Iterator` two-method protocol, `hasNext()` + `next()` design
- [Swift: IteratorProtocol (Apple Developer)](https://developer.apple.com/documentation/swift/iteratorprotocol) — value-returning `next() -> T?` design; confirms Option-sentinel approach
- [GDScript reference (Godot 4)](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_basics.html) — confirms for-in works only on built-in Array/Dictionary; user-defined iteration not supported
- [ariadne crate docs (0.6.0)](https://docs.rs/ariadne/latest/ariadne/) — confirmed no fix-suggestion API in 0.6.0; secondary label and multi-file rendering confirmed
- [Microsoft Docs: String.Split](https://learn.microsoft.com/en-us/dotnet/api/system.string.split) — split return type conventions

### Tertiary (context cross-checks)
- CLR generics implementation patterns — constraint erasure after type-check; no IL-level enforcement needed at runtime
- Rust `str` method docs — `to_uppercase` is Unicode-aware and locale-sensitive; `to_ascii_uppercase` is stable and locale-independent; game scripts need the latter

---
*Research completed: 2026-03-29*
*Ready for roadmap: yes*
