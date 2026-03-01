# Project Research Summary

**Project:** Writ Compiler v3.0 — Middle-End Pipeline
**Domain:** Compiler middle-end — name resolution, type checking, and IL codegen for the Writ game scripting language
**Researched:** 2026-03-02
**Confidence:** HIGH

## Executive Summary

Writ v3.0 is a compiler middle-end milestone: connecting the existing lowered AST output (produced by the v1.x lowering passes in `writ-compiler`) to the existing binary IL runtime (shipped in v2.0 as `writ-runtime` + `writ-module`). The pipeline is well-scoped — three sequential phases (name resolution, type checking, IL codegen) that transform `Vec<AstDecl>` into a `writ_module::Module`. The target format is fully specified (90 instructions, 21 metadata tables, complete IL spec), and the output consumer (the VM) is already working. This is a correctness-first milestone, not a performance or incremental-compilation milestone.

The recommended approach is to implement all three phases as new modules inside `writ-compiler` (not new crates), following the existing `lower/` module as a precedent. The dependency graph is `Ast → NameResolved → Typed → Module`, with each phase producing a distinct IR and accumulating errors rather than halting immediately. Four new production dependencies are needed: `id-arena` (type storage without lifetime pollution), `rustc-hash` (fast HashMaps for symbol tables), `ena` (union-find for type variable unification), and a promotion of `ariadne` from dev-dep to production dep at the CLI boundary. All phases must be built sequentially — name resolution is the hard prerequisite for type checking, and type checking must be complete before codegen begins, because IL instruction selection depends on fully-resolved types at every expression node.

The primary risk category is correctness-before-completion: 15 specific pitfalls are documented, all of which produce either silent wrong behavior or deferred runtime crashes rather than immediate compile-time errors. The mitigation strategy is consistent: establish correct architectural shapes before writing any logic (two-pass collection in name resolution, a separate `TypedExpr` IR, contract-canonical slot ordering in codegen), write adversarial tests for each pitfall immediately after implementing the relevant feature, and defer nothing that is currently listed as deferred in PROJECT.md tech debt — specifically `?`/`!` desugaring, singleton speaker validation, and lifecycle hook TypeDef registration.

---

## Key Findings

### Recommended Stack

See `.planning/research/STACK.md` for full details.

The existing workspace (Rust 2024 edition, `chumsky 0.12`, `logos 0.16`, `thiserror 2.0`, `insta 1`, `slotmap 1.1.1`, `indexmap 2.13`) requires no changes. The middle-end adds exactly four new crates and one new cross-crate dependency link. `writ-compiler` gains `writ-module` as a dependency for the codegen phase — this is safe, as the direction is compiler → module with no cycle.

**Core technologies (new additions for v3.0):**
- `id-arena 2.3.0`: Type node storage — `Arena<T>` + `Id<T>` eliminates `'tcx` lifetime pollution from all type-checker function signatures; type equality becomes `id_a == id_b`; interning via a `FxHashMap<TyKind, Id<TyKind>>` deduplicates type nodes
- `rustc-hash 2.1.1`: Symbol table performance — `FxHashMap`/`FxHashSet` for the hundreds of small scope frames created during name resolution; the same hasher used inside `rustc`, tuned for short-string and integer keys
- `ena 0.14.4`: Type variable unification — `UnificationTable` with snapshot/rollback for `let` inference and generic call site unification; extracted from `rustc`, maintained by the Rust compiler team
- `ariadne 0.6.0`: Diagnostic rendering — already in `writ-parser` dev-deps; promote to production dep in `writ-cli` for multi-span, multi-file error display; designed explicitly to pair with `chumsky`

Two conditional additions: `petgraph 0.8.3` only if declaration-ordering cycles require graph toposort; `bitflags 2.11.0` only if type modifier flags exceed 3-4 boolean fields. Do not add `salsa` (incremental compilation is out of scope), `rayon` (parallel type checking is premature), or any native-code backend.

### Expected Features

See `.planning/research/FEATURES.md` for full details, feature dependency graph, and prioritization matrix.

This is a spec-driven milestone: the Writ language spec fully defines what must be compiled, and the IL spec fully defines what must be emitted. The feature landscape is divided across three phases with clear dependency ordering.

**Must have — Phase 1 (Name Resolution):**
- Two-pass symbol collection: all declaration kinds collected across all files before any body resolution
- `using` resolution (plain and alias), qualified path `::` resolution, visibility enforcement, same-namespace cross-file visibility per §23
- Type name resolution: every `AstType` mapped to a `TypeRef` blob or primitive tag, including cross-module lookup of `writ-runtime` virtual module types
- Impl-type association, generic parameter scoping, forward reference handling
- Singleton speaker validation and `[Singleton]`/`[Conditional]` attribute validation (explicitly deferred from lowering; must not be deferred again)

**Must have — Phase 2 (Type Checking):**
- Primitive type propagation, `let` inference via bidirectional propagation, function call checking, field access and component field distinction
- Contract bounds checking at generic call sites, strict mutability enforcement (`let` blocks both reassignment AND mutation through `mut self` methods — two separate checks)
- Return type checking, `Option`/`?`/`!` rules, `Result`/`try` rules, pattern match exhaustiveness for enums
- Closure capture inference (classify `let` as by-value, `let mut` as by-reference), generic type argument inference via unification
- `spawn`/`join`/`cancel` type rules: `spawn expr` always produces `TaskHandle`, never the callee's return type
- `?` and `!` desugaring to typed match nodes in the typed IR (deferred from lowering; must not be deferred again)

**Must have — Phase 3 (IL Codegen):**
- All 21 metadata tables populated with correct token assignment
- Linear register allocation with LIFO high-watermark tracking (not per-expression reset)
- All 90 IL instructions emitted with correct selection driven by type annotations
- Entity construction sequence exactly as spec §14.7.5: `SPAWN_ENTITY → SET_FIELD for overrides only → INIT_ENTITY`
- Lifecycle hook TypeDef registration (emit MethodDef AND register token in TypeDef hook slot)
- Closure/delegate emission: compiler-generated capture struct TypeDef + method + `NEW_DELEGATE`
- `TAIL_CALL` for dialogue transitions (`return dialogueFn(args)` in dlg-lowered functions)
- Debug info emission (SourceSpan + DebugLocal) for all method bodies
- CALL_VIRT slot numbers derived from contract declaration order, never from impl block traversal order

**Should have (P2, after pipeline validated end-to-end):**
- Diagnostic-quality ambiguity errors with multiple candidates and definition spans
- Unresolved name fuzzy suggestions ("did you mean `survival::HealthPotion`?")
- Contract satisfaction suggestion in type errors
- `CALL_VIRT` → `CALL` specialization when receiver's static type is known concrete
- Constant folding for `const` expressions

**Defer to v4+:**
- `writ-std` module (requires v3.0 to be validated first)
- Incremental compilation (requires stable module identity scheme)
- Language server / LSP (requires stable type-checking API)
- JIT compilation (requires validated reference interpreter + type-annotated IR)

### Architecture Approach

See `.planning/research/ARCHITECTURE.md` for full details, data flow diagrams, build order, and anti-patterns.

All three new phases live inside `writ-compiler` as additional Rust modules (`resolve/`, `typecheck/`, `codegen/`), not as new crates. This matches the `lower/` module precedent, avoids circular dependency risk, and keeps intra-phase imports as natural `mod` imports rather than cross-crate public APIs. The pipeline is strictly linear: each phase produces a distinct IR type (no in-place AST mutation) and accumulates errors before the next phase boundary. Phase boundaries are hard stops: errors in name resolution prevent type checking from running; errors in type checking prevent codegen from running.

**Major components:**
1. `resolve/` — Two-pass name resolution: pass 1 collects all top-level declarations into `DefMap + NamespaceMap`; pass 2 resolves all references in bodies using the fully-populated map; produces `NameResolved` IR where every `Ident`/`Path` is replaced by a `DefId`
2. `typecheck/` — Constraint-based type checking: assigns explicit types from annotations, infers `let` bindings via `ena` union-find, validates contract impls (signature match AND completeness), produces `Typed` IR where every expression node carries a non-optional `Ty`; includes `?`/`!` desugaring and closure capture classification
3. `codegen/` — Two-sub-pass IL emission: skeleton pass emits all `TypeDef`/`FieldDef`/`MethodDef` rows and assigns `MetadataToken`s; body pass emits instruction sequences using the token map; drives `writ_module::ModuleBuilder`; produces `writ_module::Module`

Key cross-cutting patterns: `DefId` as the resolution currency (no string lookups after pass 1), `FxHashMap` rib stack for O(1) scope lookup, `LinearRegisterAllocator` with LIFO high-watermark for correct temporary management, `CodegenCtx` as the stateful thread through all emission functions.

### Critical Pitfalls

See `.planning/research/PITFALLS.md` for all 15 pitfalls with full prevention strategies, warning signs, and recovery costs.

1. **Single-pass name resolution** — forward references between top-level declarations fail. Prevention: implement two-phase collection before writing any lookup logic; this is the first architectural decision in Phase 1. Recovery cost if discovered late: MEDIUM.

2. **`let` mutation vs. reassignment — two distinct checks** — only checking `x = y` but missing `x.mutMethod()` through an immutable binding. Prevention: separate mutability analysis pass that runs after method resolution; store `is_mut_self` flag in method metadata. Recovery cost if discovered late: MEDIUM.

3. **CALL_VIRT slot ordering: impl-order vs. contract-order** — methods in the `impl` block listed in a different order than the contract declaration produce incorrect runtime dispatch (silent wrong behavior). Prevention: always look up canonical slot from the contract declaration during impl codegen, never from the impl block's traversal order. Recovery cost if discovered late: MEDIUM.

4. **Type-annotated AST: in-place mutation vs. separate typed IR** — `Option<ResolvedType>` fields on `AstExpr` create `None` traps throughout codegen. Prevention: define `TypedExpr`/`TypedStmt` IR before writing any type-checking logic; all type fields are non-optional. Recovery cost if discovered late: HIGH.

5. **`?` and `!` desugaring deferred again** — `UnaryPostfix` nodes left in the typed IR have no corresponding IL instruction. Prevention: desugar in the type checker's expression lowering pass; verify the typed IR contains no raw `UnaryPostfix` nodes. Recovery cost if discovered late: MEDIUM.

---

## Implications for Roadmap

The strict dependency ordering established in research dictates the phase structure. Name resolution must be fully working and tested before type checking begins; type checking must be fully working before codegen begins. Within codegen, the metadata skeleton pass must be complete before any method body can reference tokens.

### Phase 1: Name Resolution

**Rationale:** Name resolution is the absolute prerequisite for all downstream work. Neither the type checker nor codegen can proceed without knowing what each identifier refers to. The two-pass architecture (collection before body resolution) must be the first design decision — retrofitting it is expensive.

**Delivers:** `NameResolved` IR with every identifier bound to a `DefId`; `DefMap` mapping every `DefId` to its declaration location; `NamespaceMap` resolving `using` imports and cross-namespace visibility; type name resolution to TypeRef blobs including `writ-runtime` virtual module types; singleton speaker validation; attribute validation.

**Addresses:** All Phase 1 table-stakes features from FEATURES.md — symbol collection, `using` + `::` resolution, visibility enforcement, type name resolution, impl-type association, generic parameter scoping, singleton speaker validation, `[Singleton]`/`[Conditional]` attribute validation.

**Avoids:** Pitfall 1 (single-pass resolution), Pitfall 6 (speaker validation ordering — post-collection check), Pitfall 3 (namespace-as-string-table anti-pattern).

**Research flag:** Standard patterns — two-pass collection + rib stack is well-documented (rustc dev guide). No phase research needed.

### Phase 2: Type Checking

**Rationale:** Requires complete `DefMap` from Phase 1. Cannot be started until Phase 1 is validated. The typed IR structure produced here determines whether codegen can be implemented cleanly — defining the IR correctly up front is more important than any individual type rule.

**Delivers:** `Typed` IR where every expression node carries a non-optional `Ty`; closure capture classifications (`CaptureByValue` / `CaptureByRef`); boxing annotations at generic call sites; `?`/`!` desugaring to typed match nodes; fully validated contract impls (signature match AND completeness); mutability violations as compile errors; `spawn` expressions typed as `TaskHandle`.

**Addresses:** All Phase 2 table-stakes features from FEATURES.md — primitive type propagation, `let` inference, function call checking, field access + component field distinction, contract bounds, mutability enforcement, return type checking, Option/Result/`?`/`!`/`try` rules, pattern exhaustiveness, closure capture inference, generic type argument inference, `spawn`/`join`/`cancel` type rules, `new` construction type checking, `for` loop element type binding.

**Uses:** `ena 0.14.4` (union-find), `id-arena 2.3.0` (type interner), `rustc-hash 2.1.1` (TypeEnv scope maps).

**Implements:** `typecheck/` component — `InferCtx`, `TypeEnv`, `check.rs`, `contract.rs`, separate `TypedExpr`/`TypedStmt` IR defined before any logic is written.

**Avoids:** Pitfall 2 (`let` mutation — separate mutability pass after method resolution), Pitfall 4 (boxing annotations on call nodes), Pitfall 5 (closure capture classification in type checker, not codegen), Pitfall 7 (contract completeness check), Pitfall 8 (component access type ambiguity — concrete entity vs. generic Entity), Pitfall 11 (in-place AST mutation), Pitfall 12 (`?`/`!` desugaring), Pitfall 13 (`spawn` task handle type).

**Research flag:** Standard patterns — bidirectional type checking with constraint unification is well-documented. Component access type distinction and closure capture classification are Writ-specific but fully specified. No phase research needed.

### Phase 3: IL Codegen — Metadata Skeleton

**Rationale:** Codegen has a mandatory internal sub-ordering: all TypeDef/FieldDef/MethodDef/ContractDef rows must be emitted and assigned `MetadataToken`s before any method body can reference them (forward references require the token to exist first). The skeleton pass also establishes the CALL_VIRT slot ordering from the contract declaration — this decision cannot be changed after method bodies start emitting.

**Delivers:** All 21 metadata tables populated in the `ModuleBuilder`; `DefId → MetadataToken` mapping complete; `Ty → TypeRef blob` encoding working; lifecycle hook TypeDef registration (each hook's method token registered in the entity's TypeDef hook slot); CALL_VIRT slot numbers assigned from contract declaration order.

**Addresses:** Module metadata emission (ModuleDef, ModuleRef, ExportDef), TypeDef + FieldDef + MethodDef + ParamDef, ContractDef + ContractMethod + ImplDef, GenericParam + GenericConstraint, GlobalDef + ExternDef, ComponentSlot, lifecycle hook registration, localization key registry initialization.

**Uses:** `writ-module::ModuleBuilder` (new dependency on `writ-compiler`), `indexmap 2.13` (declaration-order iteration for field slots).

**Avoids:** Pitfall 3 (CALL_VIRT slot ordering established here from contract declaration), Pitfall 14 (lifecycle hooks registered in TypeDef metadata in this pass, before method bodies are emitted).

**Research flag:** Standard patterns — the skeleton-pass-then-body-pass approach matches the assembler's existing two-pass design. No phase research needed.

### Phase 4: IL Codegen — Method Bodies

**Rationale:** Requires all metadata tokens from Phase 3. This is the highest-complexity implementation phase — 90 instructions across 16 categories, entity construction sequences, closure emit, concurrency, dialogue tail calls, pattern match, boxing, `?`/`try` desugaring in instruction sequences. The register allocator design must be established first; retrofitting it requires rewriting all expression codegen.

**Delivers:** Complete instruction sequences for all method bodies; correct LIFO high-watermark register allocation with no register clobber on simultaneous live values; entity construction sequences exactly per spec §14.7.5 (`SPAWN_ENTITY → overrides only → INIT_ENTITY`); closure/delegate emission with compiler-generated capture struct TypeDef; `SPAWN_TASK`/`JOIN`/`CANCEL`/`DEFER_*` emission; `TAIL_CALL` for dialogue transitions; `BOX`/`UNBOX` from type-checker boxing annotations; debug info (SourceSpan + DebugLocal); cross-file localization key collision detection.

**Addresses:** All Phase 3 table-stakes features from FEATURES.md — all basic instruction emission, CALL/CALL_VIRT/CALL_EXTERN/CALL_INDIRECT, object model, entity instructions, array instructions, Option/Result instructions, closure/delegate emission, concurrency, pattern match, enum construction, conversion, string, boxing, `?`/`try` desugaring in codegen, tail call, localization, debug info.

**Avoids:** Pitfall 4 (boxing emission from type-checker annotations), Pitfall 5 (closure emit from capture classifications), Pitfall 9 (register clobber — LIFO allocator with high-watermark), Pitfall 10 (entity construction SET_FIELD only for explicit overrides, not defaults), Pitfall 15 (localization key cross-file collision — module-level registry).

**Research flag:** The entity construction sequence, closure delegate emission, and CALL_VIRT slot resolution are Writ-specific; the IL spec is authoritative and complete. The register allocator design is the highest-risk decision — establish it before writing any expression codegen. No external research needed.

### Phase 5: CLI Integration and End-to-End Validation

**Rationale:** Wire all phases into the `writ-cli compile` subcommand; validate the full source → .writil → VM execution pipeline with representative Writ programs covering all language features.

**Delivers:** `writ-cli compile` subcommand; end-to-end test suite covering entities, dialogue, closures, generics, concurrency, Option/Result, pattern match; the "looks done but isn't" checklist from PITFALLS.md fully green.

**Avoids:** All 15 pitfalls — final integration tests catch anything missed in earlier phases.

**Research flag:** Standard CLI wiring (clap subcommand). No phase research needed.

### Phase Ordering Rationale

- **Strict sequential ordering** is imposed by data dependencies: type checking cannot begin without `DefMap`; codegen cannot begin without `Typed` IR; method bodies cannot be emitted without metadata tokens.
- **Sub-phase ordering within codegen** (metadata before bodies) mirrors the two-pass pattern used successfully in name resolution and is required by forward-reference structure in the IL module format.
- **Deferred P2 features** (fuzzy name suggestions, diagnostic polish, `CALL_VIRT` specialization, constant folding) are intentionally placed after Phase 5 validation — they add user experience value but cannot be correctly implemented until the pipeline is proven correct.
- **The 15 pitfalls from research directly informed phase sequencing.** Pitfalls 1 and 6 (name resolution architecture), pitfalls 2, 4, 5, 7, 8, 11, 12, and 13 (type checking), and pitfalls 3, 9, 10, 14, and 15 (codegen) are each addressed in the phase where they are introduced, not deferred.

### Research Flags

Phases needing deeper research during planning:
- None identified. The Writ language spec and IL spec are both complete and authoritative. All compiler patterns (two-pass resolution, HM unification, linear register allocation, metadata-before-bodies codegen) are standard and well-documented.

Phases with standard patterns (no research-phase needed):
- **Phase 1:** Two-pass name resolution with rib stack — established pattern (rustc, Go)
- **Phase 2:** Bidirectional type checking with constraint unification — established pattern (Swift, Kotlin, rustc)
- **Phase 3:** Metadata skeleton pass — mirrors the assembler's existing two-pass approach
- **Phase 4:** Register-based codegen — matches the IL spec's design intent; existing assembler tests serve as reference
- **Phase 5:** CLI wiring — straightforward `clap` subcommand addition

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All crates verified against docs.rs with confirmed versions; version compatibility confirmed for Rust 2024 edition; `ena` and `id-arena` are rustc-extracted libraries with authoritative lineage |
| Features | HIGH | The Writ language spec and IL spec are the authoritative and complete source; feature set is determined by the spec, not market research; dependency ordering is unambiguous |
| Architecture | HIGH | All patterns (two-pass resolver, typed IR, skeleton+body codegen) are verified against rustc dev guide and the existing `writ-compiler` pipeline structure; anti-patterns are documented with concrete examples |
| Pitfalls | HIGH | 15 pitfalls documented with specific warning signs, verification tests, and recovery costs; majority are corroborated by rustc dev guide, compiler literature, and Writ's own PROJECT.md tech debt log |

**Overall confidence:** HIGH

### Gaps to Address

- **`ena` snapshot/rollback necessity:** Research recommends `ena` for union-find with snapshot/rollback for type-checking `if` branches independently. If type checking proves to be fully forward-only (no speculative paths), a hand-rolled union-find without rollback would suffice, saving a dependency. Validate after implementing basic type checking — starting with `ena` is safe and can be simplified if rollback is never triggered.

- **`petgraph` necessity:** Only needed if declaration ordering requires cycle detection (e.g., mutually recursive type aliases). The current spec does not have type aliases. Skip `petgraph` initially; add only if declaration ordering proves non-trivial in practice.

- **Multi-file compilation scope:** The architecture research notes that Phases 1-4 assume a pre-merged AST or single-file compilation. A module driver that parses all `.writ` files and merges `DefMap`s across files is needed for real projects. The exact mechanism (merge before resolve, or per-file resolve with cross-file DefMap joining) should be decided before Phase 1 implementation begins to avoid redesigning the `DefMap` structure mid-phase.

- **`ariadne` placement:** Should stay in `writ-cli` dev-deps if diagnostic rendering lives only at the CLI boundary, or move to `writ-compiler` production dep if the compiler exposes a `render_diagnostics` API. Decide at the start of Phase 5 CLI integration.

---

## Sources

### Primary (HIGH confidence)
- Writ Language Specification §5, §7, §11, §13, §14, §21, §23 — type system, variables, generics, dialogue, entities, scoping, modules (authoritative)
- Writ IL Specification §2.1–§2.16 — typed IL, calling convention, boxing, entity construction protocol, register model (authoritative)
- Writ PROJECT.md — known tech debt: lifecycle hook dispatch, singleton speaker assumption, `?`/`!` desugaring deferred (authoritative)
- Existing `writ-compiler` source (`ast/`, `lower/`, `lower/context.rs`) — pipeline shape and conventions (codebase)
- Existing `writ-module` source (`builder.rs`, `tables.rs`, `instruction.rs`) — codegen output API (codebase)
- [rustc Dev Guide: Name Resolution](https://rustc-dev-guide.rust-lang.org/name-resolution.html) — two-phase collection, rib stack, forward references
- [rustc Dev Guide: ty module](https://rustc-dev-guide.rust-lang.org/ty.html) — TyKind interning, arena allocation, type equality via IDs
- [rustc Dev Guide: Type Inference](https://rustc-dev-guide.rust-lang.org/type-inference.html) — constraint-based inference with union-find
- [rustc Dev Guide: Two-Phase Borrows](https://rustc-dev-guide.rust-lang.org/borrow_check/two_phase_borrows.html) — mutability analysis phases
- [id-arena docs.rs 2.3.0](https://docs.rs/id-arena/2.3.0/id_arena/) — Arena<T> + Id<T> API, no-lifetime-in-callers pattern
- [rustc-hash docs.rs 2.1.1](https://docs.rs/rustc-hash/latest/rustc_hash/) — FxHashMap/FxHashSet design rationale for compiler use
- [ena docs.rs 0.14.4](https://docs.rs/ena/latest/ena/) — UnificationTable, snapshot/rollback, union-find for type inference
- [ariadne docs.rs 0.6.0](https://docs.rs/ariadne/latest/ariadne/) — multi-span, multi-file diagnostic rendering

### Secondary (MEDIUM confidence)
- [The AST Typing Problem — Edward Z. Yang (2013)](https://blog.ezyang.com/2013/05/the-ast-typing-problem/) — explicitly-typed IR advantages vs. optional-field decoration on existing AST nodes
- [Lowering AST to Escape the Typechecker — Thunderseethe's Devlog](https://thunderseethe.dev/posts/lowering-base-ir/) — typed IR practical tradeoffs and postmortem
- [Luau Bytecode Generation — DeepWiki](https://deepwiki.com/luau-lang/luau/4.1-bytecode-generation) — LIFO register allocation (RegScope), three-way closure capture classification
- [Interface Dispatch — Lukas Atkinson (2018)](https://lukasatkinson.de/2018/interface-dispatch/) — slot-based dispatch table ordering, contract-canonical slot assignment
- [Lowering Rust Traits to Logic — Nicholas Matsakis (2017)](https://smallcultfollowing.com/babysteps/blog/2017/01/26/lowering-rust-traits-to-logic/) — contract completeness checking, solver design
- [Implementing a typechecker in Rust (RCL)](https://ruudvanasseldonk.com/2024/implementing-a-typechecker-for-rcl-in-rust) — practical single-pass typechecking, Env struct pattern

### Tertiary (LOW confidence)
- [How the CLR Dispatches Virtual Method Calls](https://www.codestudy.net/blog/clr-implementation-of-virtual-method-calls-to-interface-members/) — method slot ordering in vtables; Writ's dispatch model differs from CLR but the slot-canonical-ordering principle is directly applicable

---
*Research completed: 2026-03-02*
*Ready for roadmap: yes*
