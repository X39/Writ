# Project Research Summary

**Project:** Writ Compiler — CST-to-AST Lowering Pipeline
**Domain:** Compiler IR lowering pipeline (Rust, game scripting language)
**Researched:** 2026-02-26
**Confidence:** HIGH

## Executive Summary

The Writ compiler's next milestone is building a CST-to-AST lowering pipeline in a new `writ-compiler` crate. The existing `writ-parser` crate produces a full-fidelity CST using chumsky 0.12 + logos 0.16, with `Spanned<T> = (T, SimpleSpan)` throughout. The lowering pipeline must desugar six major constructs — dialogue blocks (`dlg`), entities, operator overloads, optional/nullable sugar, formattable strings, and compound assignments — into a simplified AST of primitives that the downstream type checker can consume. All lowering rules are fully specified in Section 28 of the Writ language spec, making this a well-bounded, spec-driven implementation task rather than an exploratory design problem.

The recommended approach is a pass-based pipeline following the `rustc_ast_lowering` pattern: a shared `LoweringContext` struct carries mutable state (errors, speaker scope stack, localization key counter) through independent fold functions, one per desugaring construct. The AST must use owned types (`String`, `Box<T>`, `Vec<T>`) with no `'src` lifetime inherited from the CST — carrying the source buffer lifetime into the AST is the single most damaging architectural mistake possible at this stage, as it cascades into every downstream phase. Every AST node must carry a `SimpleSpan` field from day one; retrofitting span preservation after passes are written is a full-day rewrite.

The two highest risks are span tombstoning (generating synthetic AST nodes without source spans, causing downstream errors to point at byte 0) and speaker resolution incompleteness (implementing only the singleton entity tier of the three-tier speaker lookup, silently producing incorrect code for dialogue blocks that receive entity parameters). Both risks must be addressed in the AST type definitions and dialogue lowering phases respectively, and both require test cases on day one — not after the core pipeline ships.

## Key Findings

### Recommended Stack

See `.planning/research/STACK.md` for full details.

The stack is minimal by design. The implementation language is Rust 2024 edition (already established in the workspace). The lowering pipeline depends directly on `writ-parser` for CST types and adds three production dependencies: `thiserror 2.0` for structured `LoweringError` types, `const-fnv1a-hash 1.1` for compile-time FNV-1a localization key generation, and nothing else. Diagnostic rendering (`ariadne 0.6`) belongs in `writ-cli`, not `writ-compiler`, to keep the compiler crate CLI-agnostic. `insta 1.x` with the `ron` feature is the dev-dependency for snapshot testing lowered AST output.

No arenas, no proc-macro visitor frameworks, no Salsa query system. These add complexity with no payoff at this scale. The pattern is manual fold functions per CST node variant — zero dependencies, directly testable, and exhaustiveness-checked by the Rust compiler.

**Core technologies:**
- `Rust 2024 edition`: Implementation language — already in workspace, no change required
- `writ-parser` (internal): CST source types — lowering consumes `writ_parser::cst` directly
- `thiserror 2.0`: Error type definition — structured `LoweringError` with source spans, no terminal dependency
- `const-fnv1a-hash 1.1`: Localization key generation — `no_std`, stable Rust, computes FNV1A-32 at compile time per spec §28.4
- `insta 1.x` (dev): Snapshot testing — standard approach for compiler IR testing; already in `writ-parser` dev-deps

### Expected Features

See `.planning/research/FEATURES.md` for full details and dependency graph.

**Must have (P1 — pipeline incomplete without these):**
- AST type hierarchy — the output type; everything else depends on it
- Source span preservation — design constraint on AST types; cannot be retrofitted
- Error accumulation with pass continuation — design constraint on pipeline runner; cannot be retrofitted
- Multi-pass pipeline structure — defines extensibility; determines how future constructs are added
- Dialogue lowering (`dlg` → `fn` with `say()`, `choice()`, `->` → `return`) — primary language construct
- Speaker context tracking — required for correct multi-line speaker attribution
- Localization key generation (FNV-1a auto-keys + manual `#key` override) — runtime L10N requires keys
- Entity lowering (`entity` → struct + component fields + lifecycle hooks) — second primary construct
- Operator lowering (`operator +` → contract method call) — required for operator-overloaded code
- Optional sugar lowering (`T?` → `Option<T>`, `null` → `Option::None`) — foundational; affects all type-annotated nodes
- Compound assignment desugaring (`+=` → `= + `) — required before operator lowering is complete
- Formattable string lowering (`$"..."` → concat chain) — required by dialogue lowering
- Concurrency pass-through (`spawn`/`join`/`cancel`/`defer` → AST primitives) — must survive lowering intact

**Should have (P2 — add once core pipeline tests pass):**
- Localization key collision detection — spec §13.7 requires duplicate `#key` to be a compile error
- Transition tail-call validation — `->` must be terminal; enforce at lowering time
- Singleton entity auto-detection in dialogue — spec §13.2 requires `[Singleton]` speaker resolution
- Component field flattening with defaults — entity `use` clause field defaults
- Derived operator auto-generation (`!=`, `>`, `<=`, `>=` from `Eq`/`Ord`) — spec §17.4

**Defer (v2+):**
- Diagnostic span enrichment (`LoweringOrigin` on spans) — requires IDE consumer infrastructure
- Incremental/query-based lowering — defer until build performance is a demonstrated bottleneck

### Architecture Approach

See `.planning/research/ARCHITECTURE.md` for full details, data flow diagrams, and code examples.

The `writ-compiler` crate is structured as two top-level modules: `ast/` (AST type definitions — the interface contract for all downstream phases) and `lower/` (the pipeline, one file per pass). A `LoweringContext` struct in `lower/context.rs` carries all cross-cutting mutable state through every pass. The public API is a single function: `lower(items: Vec<Spanned<Item<'_>>>) -> (Ast, Vec<LoweringError>)`. Passes are composed by `lower/mod.rs` (the orchestrator) using `match` dispatch — no dynamic pass registry. Localization key generation runs as a sub-pass inside dialogue lowering, not as a separate top-level pass.

**Major components:**
1. `ast/` module — Simplified AST node type hierarchy: `AstExpr`, `AstStmt`, `AstDecl`, `AstType`. No CST sugar, no lifetime parameters, every node carries `span: SimpleSpan`. This is the sole interface to downstream phases.
2. `LoweringContext` — Shared mutable state: `errors: Vec<LoweringError>`, `speaker_stack: Vec<SpeakerScope>`, `loc_key_counter: u32`. Threaded through every pass as `&mut LoweringContext`.
3. `lower/mod.rs` (pipeline orchestrator) — Sequences all passes in dependency order; assembles `(Ast, Vec<LoweringError>)` from pass outputs.
4. Individual pass files — `optional.rs`, `fmt_string.rs`, `operator.rs`, `dialogue.rs`, `entity.rs`, `localization.rs`, `concurrency.rs`. Each is independently testable with a fresh `LoweringContext`.
5. `lower/error.rs` — `LoweringError` type with span references, powered by `thiserror`.

**Pass ordering (dependency-constrained):**
1. `lower_fn` — processes `Fn` items; internally invokes `lower_type` (optional), `lower_expr` (fmt_string, operator, concurrency)
2. `lower_dialogue` — processes `Dlg` items; internally calls `lower_expr` sub-passes, then `lower_localization` on produced `say()` calls
3. `lower_entity` — processes `Entity` items; internally calls `lower_type` for component fields

### Critical Pitfalls

See `.planning/research/PITFALLS.md` for all 8 pitfalls with full prevention strategies, warning signs, and recovery costs.

1. **Span tombstoning on generated nodes** — Every synthetic AST node (the `say()` calls, `Entity.getOrCreate<T>()` calls generated from dialogue lowering) must carry a `lowered_from: SimpleSpan` pointing to its CST origin. Establish this as a hard constraint in AST type definitions before writing any pass. Dummy spans (`SimpleSpan::new(0, 0)`) are never acceptable on nodes that can produce errors. Recovery cost if discovered late: HIGH (full-day audit and rewrite of all AST constructors and lowering passes).

2. **Speaker resolution missing tier-1 (parameter) lookup** — The `@Speaker` lookup has three tiers: (1) local parameters/variables, (2) `[Singleton]` entity via `Entity.getOrCreate<T>()`, (3) compile error. Implementing only tier 2 silently produces incorrect code when a `dlg` block receives an entity as a parameter. Write `dlg scene(guard: Guard) { @guard Halt! }` as a test on day one of dialogue lowering.

3. **Pass ordering inversion** — Optional sugar and formattable string lowering are invoked from inside structural passes (`lower_fn`, `lower_dialogue`, `lower_entity`) as those passes encounter type and expression positions — not as separate top-level passes. If a pass that expects "clean" sub-expressions runs before the sub-expression lowering, it encounters unexpected CST node types and panics. Document pass order explicitly in `lower/mod.rs` rustdoc.

4. **Active speaker state not scoped across `$ choice` branches** — Using a single mutable `current_speaker` variable causes speaker changes inside a `$ choice` branch to leak into sibling branches and post-branch continuation lines. The speaker state must be a stack: push on branch entry, pop on exit. This is straightforward but must be the first implementation, not a retrofit.

5. **Localization key collision from text-only hash input** — Computing FNV-1a keys from dialogue text content alone produces collisions for identical text in different `dlg` blocks. Keys must hash `blockName + lineIndex + text`. Keys must be deterministic across compiler runs (no pointer addresses, timestamps, or allocation-order-derived values). A CI test compiling the same source twice and diffing key output is the verification.

## Implications for Roadmap

Based on combined research findings, the natural phase structure follows the build-order dependency chain from ARCHITECTURE.md. The key constraint is: **AST types must be the first deliverable** because they are the interface contract that all passes emit into. The second constraint is that span preservation and error accumulation are design-time decisions that cannot be retrofitted — they must be decided before the first pass is written.

### Phase 1: AST Type Hierarchy and Pipeline Infrastructure

**Rationale:** The AST type hierarchy is the prerequisite for everything else. Without it, passes have no target type to emit. Span preservation and error accumulation are structural decisions baked into the AST types and pipeline runner — retrofitting them costs more than getting them right first. This phase has no dependencies and unblocks all subsequent phases.
**Delivers:** `ast/` module with complete `AstExpr`/`AstStmt`/`AstDecl`/`AstType` definitions; `LoweringContext`; `LoweringError`; public `lower()` stub that compiles; `writ-compiler/Cargo.toml` with `thiserror` and `const-fnv1a-hash`.
**Addresses:** AST type hierarchy, source span preservation (as design constraint), error accumulation (as pipeline constraint), multi-pass pipeline structure.
**Avoids:** Span tombstoning pitfall — every AST node constructor requires a `span` argument from the start.
**Research flag:** Standard patterns (rustc HIR, Kotlin FIR). Skip research-phase; use `rustc_ast_lowering` as direct reference.

### Phase 2: Foundational Expression Lowering (Optional Sugar, Formattable Strings, Compound Assignments)

**Rationale:** These three desugarings are foundational: optional sugar affects type positions throughout the AST (entity fields, function signatures), formattable string lowering is required by dialogue lowering, and compound assignment is mechanical. All three are low-complexity but high-dependency. Completing them before the stateful passes (dialogue, entity) ensures those passes work on clean sub-expressions.
**Delivers:** `lower/optional.rs` (`T?` → `Option<T>`, `null` → `Option::None`); `lower/fmt_string.rs` (`$"..."` → concat chain); compound assignment desugaring in `lower_expr`.
**Uses:** `insta` snapshots for each pass; `LoweringContext` from Phase 1.
**Implements:** `lower_type()` and `lower_expr()` shared helpers called from all structural passes.
**Avoids:** Pass ordering inversion pitfall — these are implemented as helpers before structural passes are written.
**Research flag:** Well-documented patterns. Skip research-phase.

### Phase 3: Operator Lowering and Concurrency Pass-Through

**Rationale:** Operator lowering (`operator +` → contract method call) is a moderately complex but self-contained pass with no dialogue or entity dependencies. Concurrency pass-through is trivial (CST node → equivalent AST node, no transformation). Both are required for complete expression lowering coverage before the complex passes.
**Delivers:** `lower/operator.rs` (symbol → `Add`/`Sub`/`Mul`/etc. contract impl method calls); `lower/concurrency.rs` (`spawn`/`join`/`cancel`/`defer` → AST primitives).
**Uses:** `writ-parser::cst` operator variants; `LoweringContext`.
**Avoids:** `spawn` keyword ambiguity — verify CST distinguishes `SpawnEntity` from `SpawnTask`; file parser issue if not.
**Research flag:** Standard patterns for operator desugaring. Skip research-phase; verify CST spawn disambiguation with parser team.

### Phase 4: Dialogue Lowering and Localization

**Rationale:** Dialogue lowering is the most complex pass and depends on Phase 2 (formattable string lowering for `{expr}` interpolation in dialogue text) and implicitly on entity scan results for singleton speaker resolution. It is isolated in its own phase because it introduces stateful context (speaker scope stack) and a sub-pass (localization key generation). This is where the highest-risk pitfalls live.
**Delivers:** `lower/dialogue.rs` (complete `dlg` → `fn` with `say()`, `choice()`, `->` → `return`, three-tier speaker resolution); `lower/localization.rs` (FNV-1a key auto-generation + `#key` override); speaker scope stack in `LoweringContext`.
**Uses:** `const-fnv1a-hash` for key generation; `lower_fmt_string` helper from Phase 2.
**Avoids:** Speaker resolution tier-1 pitfall (parameter lookup before singleton lookup); active speaker scope leak across `$ choice` branches; `->` emitted without `return` wrapper; localization key collision from text-only hash.
**Research flag:** Needs careful spec-checking against §13, §28.1–28.4. Consider a brief research-phase focused on the three-tier speaker resolution rule and the `$ choice` scoping semantics.

### Phase 5: Entity Lowering

**Rationale:** Entity lowering (`entity` → struct + component impls + lifecycle hooks) is architecturally independent of dialogue lowering but depends on optional sugar lowering (Phase 2) for `T?` field types. It is the second-most complex pass, involving partitioning entity members into properties, components, hooks, and methods before generating multiple AST items from one CST item.
**Delivers:** `lower/entity.rs` (entity → `AstStructDecl` + N `AstImplDecl` for `ComponentAccess<T>` + lifecycle hook registrations); correct handling of `[Singleton]` attribute.
**Avoids:** Entity component/property conflation pitfall — member partitioning is an explicit pre-lowering step, not inline branching during code generation.
**Research flag:** Standard struct-lowering patterns. Skip research-phase; focus on spec §14.3 for component field conflict rules.

### Phase 6: Pipeline Integration and P2 Quality Features

**Rationale:** With all core passes complete, the pipeline orchestrator wires them in correct order and the full test suite validates the integrated output. P2 quality features (collision detection, transition validation, singleton auto-detection, derived operators) are added here once the core pipeline is stable.
**Delivers:** Complete `lower/mod.rs` orchestrator; integration tests using `insta` snapshots; localization key collision detection; transition tail-call validation; `[Singleton]` speaker auto-detection; derived operator auto-generation (`!=`, `>`, `<=`, `>=` from `Eq`/`Ord`); component field default flattening.
**Avoids:** Hash instability pitfall — CI test compares key output across two identical compilations.
**Research flag:** Standard integration work. Skip research-phase.

### Phase Ordering Rationale

- Phases 1–2 are prerequisites for everything: without AST types and core expression lowering, structural passes cannot be written or tested.
- Phase 3 (operators, concurrency) is ordered before the stateful passes because `lower_expr` is called from inside `lower_dialogue` and `lower_entity` — the sub-expression helpers must exist first.
- Phase 4 (dialogue) comes before Phase 5 (entity) because dialogue lowering is the harder implementation with the most pitfall risk; completing it first de-risks the milestone before entity work begins.
- Phase 6 is explicitly last: P2 features require a stable, tested core pipeline as their foundation. Adding them before integration testing would obscure which failures are from core lowering vs. quality features.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 4 (Dialogue Lowering):** The three-tier speaker resolution rule (spec §13.2) and the `$ choice` branch scoping semantics (spec §13.4) should be re-read carefully before implementation. A brief research-phase is warranted to clarify ambiguities in the spec before writing the most complex pass.

Phases with standard patterns (skip research-phase):
- **Phase 1:** Direct analogue to `rustc_ast_lowering` infrastructure. rustc dev guide is the reference.
- **Phase 2:** Optional/nullable lowering and string interpolation lowering are textbook desugarings.
- **Phase 3:** Operator-to-contract mapping is fully specified in §17.2; concurrency is pass-through.
- **Phase 5:** Entity-to-struct lowering follows standard struct generation patterns; spec §14 is authoritative.
- **Phase 6:** Integration and quality features; no novel patterns.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Core dependencies (thiserror, const-fnv1a-hash, insta) verified on docs.rs. Fold-over-visitor recommendation backed by rustc, swc, and Rust Design Patterns book. Version compatibility confirmed. |
| Features | HIGH | Writ spec (§13–§20, §28) is authoritative and fully defines all lowering rules. rustc AST→HIR is a direct reference implementation. Feature dependency graph is derived from spec dependencies, not speculation. |
| Architecture | HIGH | `LoweringContext` + fold pattern is documented by rustc dev guide, Swift SIL generation, and Kotlin K2. Pass ordering constraints are derived from spec-defined lowering dependencies. File structure follows established Rust compiler conventions. |
| Pitfalls | MEDIUM | Core pitfalls (span tombstoning, pass ordering, error accumulation) are well-documented in compiler literature. Writ-specific pitfalls (speaker resolution tiers, speaker scope leakage, localization key stability) are inferred from spec rules — no external validation possible since Writ is a novel language. |

**Overall confidence:** HIGH

### Gaps to Address

- **`spawn` keyword disambiguation:** The CST representation of `spawn` (entity instantiation vs. task creation) must be verified before Phase 3. If the parser does not distinguish them, a pre-pass or parser fix is needed. This is a dependency on `writ-parser` that the lowering pipeline cannot resolve unilaterally.
- **Spec §29 open questions:** The spec's open questions section (§29) notes several TBD semantics. Any TBD features that appear in the CST (tuples, destructuring) must not be lowered in this milestone. Passes must explicitly handle `todo!()` or `unreachable!()` for those constructs rather than silently dropping them.
- **`writ-runtime` API contract:** The lowered AST references `say()`, `choice()`, `Entity.getOrCreate<T>()` as named function calls. These must agree with the runtime crate's exported API. If the runtime API is not finalized, lowering must emit calls by a convention that the runtime will match — this should be confirmed before Phase 4 begins.

## Sources

### Primary (HIGH confidence)
- Writ language spec §13–§20, §28 — authoritative lowering rules for all constructs
- `writ-parser/src/cst.rs` — definitive CST node inventory
- [rustc-dev-guide: HIR](https://rustc-dev-guide.rust-lang.org/hir.html) — LoweringContext pattern, span preservation, fold pattern
- [rustc-dev-guide: Overview](https://rustc-dev-guide.rust-lang.org/overview.html) — multi-pass pipeline rationale
- [rustc_ast_lowering crate docs](https://doc.rust-lang.org/stable/nightly-rustc/rustc_ast_lowering/struct.LoweringContext.html) — direct implementation reference
- [thiserror docs.rs](https://docs.rs/thiserror/latest/thiserror/) — version 2.0.18 confirmed
- [const-fnv1a-hash docs.rs](https://docs.rs/const-fnv1a-hash/latest/const_fnv1a_hash/) — version 1.1.0, `fnv1a_hash_str_32` confirmed
- [insta docs.rs](https://docs.rs/insta/latest/insta/) — version 1.46.3 confirmed
- [ariadne docs.rs](https://docs.rs/ariadne/latest/ariadne/) — version 0.6.0, chumsky compatibility confirmed
- [Swift Compiler Architecture](https://www.swift.org/documentation/swift-compiler/) — SILGen pattern reference
- [Kotlin K2 Migration Guide](https://kotlinlang.org/docs/k2-compiler-migration-guide.html) — FIR desugaring reference
- [Rust Design Patterns: Fold](https://rust-unofficial.github.io/patterns/patterns/creational/fold.html) — fold vs. visitor distinction

### Secondary (MEDIUM confidence)
- [Thunderseethe: Lowering AST to IR](https://thunderseethe.dev/posts/lowering-base-ir/) — consuming fold pattern, span preservation
- [Thunderseethe: Desugaring Concrete/Abstract Syntax](https://thunderseethe.dev/posts/desugar-base/) — CST-to-AST specifics
- [Post-Modern Compiler Design Vol 1 — Lowering](https://www.cs.purdue.edu/homes/rompf/pmca/vol1/lowering.html) — pass ordering, elaboration
- [Braid Compiler Architecture](https://capra.cs.cornell.edu/braid/docs/hacking.html) — game-adjacent compiler reference
- [Rust Performance Book: Hashing](https://nnethercote.github.io/perf-book/hashing.html) — FNV vs. alternatives

### Tertiary (LOW confidence)
- [Writing my own dialogue scripting language — dansalva.to](https://dansalva.to/writing-my-own-dialogue-scripting-language/) — dialogue-specific lowering patterns
- [HN: The difference between compiling and lowering](https://news.ycombinator.com/item?id=14425039) — pass ordering, idempotency

---
*Research completed: 2026-02-26*
*Ready for roadmap: yes*
