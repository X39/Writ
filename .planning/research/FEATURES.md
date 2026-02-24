# Feature Research

**Domain:** CST-to-AST lowering pipeline for a game scripting compiler (Rust)
**Researched:** 2026-02-26
**Confidence:** HIGH — rustc and Roslyn are documented reference implementations; Writ spec is authoritative for construct-specific rules

---

## Context

The lowering pipeline sits between `writ-parser` (produces a full-fidelity CST with 40+ node types) and downstream semantic analysis (type checking, name resolution, codegen). Its job is to desugar Writ's higher-level constructs — dialogue blocks (`dlg`), entities, operator overloads, optional sugar, compound assignments, formattable strings — into a simplified AST of primitives (functions, structs, dispatch-table impls, standard calls).

The Writ spec (Section 28) defines all lowerings exactly. The pipeline must follow those definitions while preserving source spans for diagnostics and propagating errors without aborting on the first failure.

Reference implementations studied: `rustc`'s `rustc_ast_lowering` crate (AST→HIR), GHC's Core desugaring, the Braid shading language compiler, Inkle's Ink dialogue DSL.

---

## Feature Landscape

### Table Stakes (Pipeline Is Broken Without These)

These are the non-negotiable features. If any one is missing, the lowering pipeline either cannot produce a usable AST or downstream passes cannot trust their input.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **AST type hierarchy** | The pipeline has nothing to produce without a target representation. Every pass lowers into AST nodes. | HIGH | A separate, simplified node set from the CST — stripped of trivia, syntactic CST-specific variants, and optional combinator scaffolding. Rust enums without lifetime parameters where possible to simplify downstream use. |
| **Dialogue lowering (`dlg` → `fn`)** | `dlg` is the primary authoring construct. The runtime only knows `fn`, `say()`, `choice()`. Without this pass, dialogue is entirely unusable. | HIGH | Per spec §28.1–28.4: speaker lines → `say(speaker, text)` calls; `$ choice` → `choice([Option(...)])` with lambda bodies; `$ if`/`$ match` → standard if/match with `say()` in branches; `->` transitions → `return target(args)` tail calls; speaker resolution via `Entity.getOrCreate<T>()` for `[Singleton]` entities and direct references for parameters. Context tracking required: active speaker state, local bindings visible to `$` escapes. |
| **Entity lowering (`entity` → struct + impls)** | `entity` is the second primary construct. The runtime and type checker deal in structs. | HIGH | Per spec §28.3: entity → struct with component fields (`_health: Health`), constructor fn with defaults, `ComponentAccess<T>` impl per `use` clause, lifecycle hook registrations (`on create`, `on destroy`, `on interact`). Singleton attribute (`[Singleton]`) must be recognized and cause `Entity.getOrCreate<T>()` access pattern emission. |
| **Operator lowering (`operator` → contract impl)** | Operator overloads in `impl` blocks use `operator +` syntax. Type checker works with named method calls on contracts (`Add::add`), not operator AST nodes. | MEDIUM | Per spec §17.2: each `operator +` in an `impl` block becomes an `impl Add<Rhs, Output> for Type` with a method body. Symbol-to-contract mapping: `+` → `Add`, `-` → `Sub`/`Neg`, `*` → `Mul`, `/` → `Div`, `%` → `Mod`, `==` → `Eq`, `<` → `Ord`, `[]` → `Index`, `[]=` → `IndexSet`. |
| **Compound assignment desugaring** | `a += b` must become `a = a + b` before the type checker sees it, since compound assignment is not independently overloadable (spec §17.3). | LOW | Mechanical: `AddAssign` → `Assign(lhs, Binary(lhs_clone, Add, rhs))`. Requires cloning the LHS expression — must preserve span of original `+=`. |
| **Optional / nullable sugar lowering** | `T?` is sugar for `Option<T>`; `null` is sugar for `Option::None` (spec §19). Type checker only knows `Option<T>`. | LOW | `TypeExpr::Nullable(T)` → `TypeExpr::Generic("Option", [T])`. `Expr::NullLit` → `Expr::Path(["Option", "None"])`. Also `PostfixOp::NullPropagate` (`?`) and `PostfixOp::Unwrap` (`!`) need AST representations — `?` becomes early-return-on-None, `!` becomes unwrap-or-panic. |
| **Formattable string lowering** | `$"Hello {name}!"` and dialogue text interpolation `{expr}` must become string concatenation with `.into<string>()` calls (spec §28.1). `FormattableString` nodes cannot reach the type checker as-is. | MEDIUM | Each `StringSegment::Expr(e)` → `e.into<string>()`. Segments joined with `+` (using `Add` contract). Plain `StringSegment::Text(s)` → string literal. Dialogue text segments follow the same rule. Single-segment strings (no interpolation) are just string literals — no concatenation overhead. |
| **Localization key generation** | `say()` calls with automatic FNV-1a keys for each dialogue line (spec §28.4). Manual `#key` overrides. Without keys, the runtime cannot perform localized string lookup. | MEDIUM | FNV-1a hash of the default-locale text content (excluding `{expr}` interpolation slots). Manual `loc_key` from `DlgLine` overrides the computed hash. Output: `say_localized(speaker, "a3f7c012", text)` instead of `say(speaker, text)`. Key uniqueness within a `dlg` block is a compile error. |
| **Concurrency pass-through** | `spawn`, `join`, `cancel`, `defer` need AST-level representations — they are not desugared but must survive lowering intact for the coroutine-aware backend (spec §20). | LOW | The CST already has `Expr::Spawn`, `Expr::Join`, `Expr::Cancel`, `Expr::Defer`. The lowering pass must map these to corresponding AST nodes without transformation. `spawn detached` distinction must be preserved (`Expr::Detached`). |
| **Source span preservation** | Every AST node must carry the span of the CST node that produced it. Error messages after lowering must point at original source positions, not synthesized positions. This is the defining quality bar of a production-grade pipeline. | MEDIUM | Every synthetic node (e.g., a `say()` call generated from a `SpeakerLine`) carries the span of the originating `DlgLine`. Rustc's `LoweringContext` enforces: every created `HirId` must be used, and lowering happens in scope of the owning item. Writ's equivalent: every `AstNode` carries `SimpleSpan` from its CST origin. Synthetic nodes generated with no CST origin (e.g., `Entity.getOrCreate()` injected for speaker resolution) carry the span of the construct that triggered generation. |
| **Error accumulation with pass continuation** | A single bad speaker reference in one dialogue block must not abort lowering of all other blocks. The pipeline must collect errors and continue, producing as complete an AST as possible. | MEDIUM | The CST already uses `Expr::Error` / `Stmt::Error` sentinels from parse recovery. The lowering pipeline extends this: lower passes produce `AstNode::Error(span, message)` for unresolvable constructs, push the error to a diagnostic accumulator, and return the error node so the caller can continue. Downstream passes skip error nodes without cascading failures. This mirrors rustc's `Ty::Error` and the Braid compiler's `Hole` node pattern. |
| **Multi-pass pipeline structure** | Each construct's lowering (dialogue, entity, operator) must be independently testable and independently executable. A single monolithic visitor is an anti-pattern — adding `component` lowering later should not require restructuring dialogue lowering. | MEDIUM | Ordered pass sequence where each pass takes `&[Item<'src>]` (or the prior pass's output) and produces the next IR level. Passes are composed, not merged. Rustc organizes this as separate functions/modules within `rustc_ast_lowering`; Writ can follow the same pattern. Passes run sequentially; earlier passes enable later ones. |

### Differentiators (Competitive Advantage / Quality-of-Life)

These features are not required for a working pipeline but provide meaningful advantages — either for developer experience, future extensibility, or language-level correctness.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Speaker context tracking with scoped state** | The spec's `@speaker` standalone form sets an active speaker for subsequent text lines. A naive pass forgets this across lines. Proper context tracking produces correct `say()` calls for multi-line speaker blocks without requiring callers to re-specify the speaker. | MEDIUM | The lowering pass for `dlg` must maintain a `current_speaker: Option<SpeakerRef>` that is updated by `DlgLine::SpeakerTag` and consumed by `DlgLine::TextLine`. Speaker resolution (local var vs. singleton entity) happens at this point. Differentiator because naive implementations get multi-line attributions wrong, producing silent semantic bugs. |
| **Derived operator auto-generation** | Spec §17.4 says `!=` derives from `Eq`, `>` from `Ord`, `<=` from `Eq+Ord`, `>=` from `Eq+Ord`. If a type implements `==` (→ `Eq`), the lowering pass can auto-generate the derived contract impls rather than requiring the author to write them. | MEDIUM | Requires knowing which operators are explicitly implemented. After operator lowering, scan for `Eq` and `Ord` impls and emit derived `NotEq`, `Gt`, `LtEq`, `GtEq` impls with their spec-defined bodies. Differentiator: catches a class of "forgot to implement `!=`" bugs at compile time rather than runtime. |
| **Diagnostic span enrichment** | Rather than just preserving the original span, annotate spans with desugaring provenance: "this `say()` call was generated from dialogue line at X." Enables IDE tooling to navigate from a runtime error back to the original dialogue line. | HIGH | Rustc implements this via `DesugaringKind` on spans — spans can be marked as relative to a desugaring kind, enabling the diagnostic system to explain why a synthetic node exists. Writ could carry a `LoweringOrigin` enum on AST spans. Valuable for a language targeting game writers who will not understand why a `say()` call failed. |
| **Singleton entity auto-detection in dialogue** | When a speaker identifier appears in `@Speaker`, the lowering pass should resolve it to `Entity.getOrCreate<Speaker>()` automatically if the named entity has `[Singleton]` attribute, without requiring the `dlg` author to write the `$` escape. | MEDIUM | Requires a symbol table pass before dialogue lowering: scan all top-level `EntityDecl` items, collect those with `[Singleton]` attribute, build a lookup set. During dialogue lowering, check speaker name against the set. This is what the spec requires (§13.2) but it demands that entity declarations are lowered before dialogue declarations — or that a preliminary scan happens first. Dependency: entity attribute reading before dialogue lowering. |
| **Component field flattening with defaults** | When lowering `entity` declarations, component `use` clauses have per-field defaults (`use Health { current: 80, max: 80 }`). The lowered struct constructor should populate these defaults correctly, including falling back to the component's own field defaults for fields not overridden in the `use` clause. | MEDIUM | Requires the component definition (either script-defined `ComponentDecl` or `ExternDecl::Component`) to be available during entity lowering. Two-phase: (1) resolve component field defaults from component decl; (2) merge with `UseField` overrides from entity's `use` clause; (3) emit constructor with merged defaults. |
| **Transition tail-call validation** | The spec says `->` (dialogue transition) must be the last statement in its block (§13.6). The lowering pass can enforce this as a lowering error, catching misplaced `->` before type checking. | LOW | During dialogue lowering, when a `Transition` is encountered, verify it is the last element in its containing `Vec<DlgLine>`. If not, emit a lowering error with span pointing to the premature `->`. This is mechanical but meaningful — violations would produce confusing type errors downstream without this check. |
| **Localization key collision detection** | Spec §13.7 says duplicate `#key` values within a `dlg` block are a compile error. The localization lowering pass can detect collisions at the point of key assignment. | LOW | Track emitted keys in a `HashSet<&str>` per `DlgDecl`. On collision, emit a lowering error. Straightforward but only happens during the lowering pass — downstream passes have no context to detect this. |

### Anti-Features (Deliberately Not Built)

Features that seem useful but create concrete problems at this stage. Building these now would waste time, create coupling, or undermine the pipeline's core value.

| Anti-Feature | Why Requested | Why Problematic | Alternative |
|--------------|---------------|-----------------|-------------|
| **Type checking in the lowering pass** | Speaker resolution (`@Narrator`) feels like name resolution, which feels like type checking. It's tempting to fold it in. | Coupling name resolution and type checking into lowering makes the passes non-independent and non-testable in isolation. The lowering pass does not have enough information to fully type-check — component field types, contract bounds, etc. require a full symbol table that doesn't exist yet. Mixing phases is the root cause of most compiler architecture rewrites. | Lower speakers to `Entity.getOrCreate<T>()` calls with the identifier as a type argument — mark unresolved speakers as `LoweringError` nodes. Name resolution and type checking happen in the next phase. |
| **Single monolithic visitor** | A single `fn lower_item(item: &Item)` that matches on all 14 item types and dispatches inline seems simpler to write. | Monolithic visitors grow indefinitely and cannot be tested in isolation. Adding `component` lowering later requires modifying the same function that handles `dlg` lowering — merge conflicts, regression risk, tangled state. | Independent pass functions (`lower_dialogue`, `lower_entity`, `lower_operators`, etc.) composed in an ordered pipeline. Each can be tested with just the CST nodes it cares about. |
| **Incremental / query-based lowering** | Rustc's query system (`tcx.lower_to_hir(def_id)`) enables incremental compilation where only changed items are re-lowered. This sounds like the right architecture from the start. | Implementing a query system requires a dependency-tracking infrastructure (like Salsa) that doesn't exist yet and would consume the entire milestone just to set up. The Writ compiler at this stage has no existing consumers that need incremental compilation. | A simple ordered pipeline that re-lowers everything on each compilation run. Salsa or a custom query system can be retrofitted later once the pipeline is stable and the performance need is demonstrated. |
| **Macro/attribute expansion during lowering** | Attributes like `[Singleton]` look like they could be expanded by the lowering pass (like Rust proc-macros). | Writ has no macro system (out of scope per spec). Attributes are metadata for the lowering pass to read, not transform. Building expansion infrastructure is premature and would complicate the attribute reading needed for `[Singleton]` detection. | Read attributes as opaque metadata in the lowering pass. `[Singleton]` triggers specific lowering behavior directly — no expansion pipeline. |
| **Optimization in the lowering pass** | When a `$"text"` formattable string has no interpolation, it's just a string literal — optimizing this in the lowering pass seems trivial. | Optimizations in a lowering pass obscure what the pass is supposed to do (faithful representation) and create correctness surface area. Constant folding, dead branch elimination, and similar optimizations belong in a dedicated optimization pass that runs after the AST is stable. | Lower faithfully (no-interpolation formattable string still produces a single-segment concatenation). A later constant folding pass can collapse it to a literal. |
| **Runtime function implementation** | `say()`, `choice()`, `Entity.getOrCreate<T>()` are referenced by the lowered AST but not defined by the lowering pass. It's tempting to stub them in the AST. | The runtime is a separate crate (`writ-runtime`) and is explicitly out of scope for this milestone. Defining even stubs in the compiler creates a circular dependency concern and violates crate boundaries. | Emit runtime function calls as `Expr::Call` nodes referencing named functions. The linker/runtime crate provides the implementations. |
| **Full derive-operator implementation** | Auto-generating `!=`, `>`, `<=`, `>=` from `Eq`/`Ord` is in the Differentiators section. Going further and auto-deriving `Hash`, `Display`, `Clone`-equivalent contracts would seem natural. | The spec does not define these derivations. Adding them creates spec-divergent behavior. The spec's open questions section (§29) notes several TBD semantics — building on top of TBD spec is building on sand. | Implement only the four derived operators explicitly defined in §17.4. Any additional derives belong in a future spec revision. |

---

## Feature Dependencies

```
[AST type hierarchy]
    └──required by──> ALL other passes (nothing to emit without AST types)

[Entity lowering]
    └──required by──> [Dialogue lowering]
                          (speaker resolution needs to know which entities are [Singleton])
    └──required by──> [Component field flattening with defaults]

[Operator lowering]
    └──required by──> [Derived operator auto-generation]
                          (must know which operators are explicitly implemented first)

[Optional sugar lowering]
    └──required by──> [Entity lowering]
                          (entity fields use T? types; must be lowered before struct emission)
    └──required by──> [Formattable string lowering]
                          (interpolated expressions may use ? propagation)

[Formattable string lowering]
    └──required by──> [Dialogue lowering]
                          (dialogue text segments follow the same interpolation rules)

[Localization key generation]
    └──required by──> [Dialogue lowering]
                          (keys are assigned during dialogue lowering, not a separate pass)
    └──enhances──> [Diagnostic span enrichment]
                          (key collision errors benefit from enriched spans)

[Source span preservation]
    └──required by──> ALL passes (every pass must carry spans forward)

[Error accumulation with pass continuation]
    └──required by──> ALL passes (each pass produces errors that must not halt others)

[Multi-pass pipeline structure]
    └──required by──> [Singleton entity auto-detection in dialogue]
                          (needs a preliminary scan pass before dialogue lowering)

[Transition tail-call validation] ──enhances──> [Dialogue lowering]
[Localization key collision detection] ──enhances──> [Localization key generation]
[Speaker context tracking] ──required by──> [Dialogue lowering]
```

### Dependency Notes

- **AST type hierarchy requires nothing but enables everything:** It must be the first deliverable. Without it, there is no target for lowering passes to emit.
- **Entity lowering before dialogue lowering:** The spec's `@Speaker` inline-form resolution requires knowing which entities are `[Singleton]`. Either entity declarations are scanned before dialogue lowering begins, or a preliminary "collect singletons" pass runs first. The multi-pass pipeline structure is what makes this ordering explicit and enforceable.
- **Optional sugar lowering is foundational:** It affects TypeExpr nodes throughout — entity field types, function signatures, component fields all use `T?`. This pass must run before any pass that emits typed AST nodes.
- **Formattable string lowering before dialogue lowering:** Dialogue text interpolation (`{expr}`) follows the same rules as `FormattableString`. The dialogue lowering pass should reuse the formattable-string lowering logic rather than duplicating it.
- **Span preservation is a cross-cutting constraint, not a pass:** Every pass must thread spans through its output. It is not something that can be added retroactively without rewriting all passes. It must be a design constraint on the AST type definitions from day one.
- **Error accumulation must be designed into the pipeline runner:** A shared `Vec<LoweringError>` or equivalent accumulator must be threaded through all passes. Passes take `&mut Vec<LoweringError>` (or equivalent) and push errors rather than returning `Result`. This prevents any single error from aborting the pipeline.

---

## MVP Definition

### Launch With (v1) — Required for the milestone to be "complete"

These are the features without which the pipeline cannot be considered functional. A downstream type-checker cannot be written without them.

- [ ] **AST type hierarchy** — without this, there is no output type
- [ ] **Source span preservation** (as a design constraint on AST types) — retrofitting is a rewrite
- [ ] **Error accumulation with pass continuation** (as a pipeline design constraint) — retrofitting is a rewrite
- [ ] **Multi-pass pipeline structure** — defines how passes compose; determines extensibility
- [ ] **Optional sugar lowering** (`T?` → `Option<T>`, `null` → `Option::None`) — foundational, affects all type-annotated nodes
- [ ] **Compound assignment desugaring** (`+=`, `-=`, etc.) — mechanical, no dependencies, blocks operator lowering completeness
- [ ] **Formattable string lowering** (`$"..."` and dialogue `{expr}`) — required by dialogue lowering
- [ ] **Dialogue lowering** (`dlg` → `fn` with `say()`, `choice()`, `->` → `return`) — the showcase construct; primary language feature
- [ ] **Speaker context tracking** — part of dialogue lowering correctness; multi-line attributions require it
- [ ] **Localization key generation** (FNV-1a auto-keys + manual `#key` override) — runtime cannot do L10N without keys
- [ ] **Entity lowering** (`entity` → struct + component fields + lifecycle hooks) — second primary construct
- [ ] **Operator lowering** (`operator +` → `impl Add for T`) — required for any code using overloaded operators

### Add After Core Pipeline Validates (v1.x)

- [ ] **Localization key collision detection** — trigger: first test case with a duplicate `#key`
- [ ] **Transition tail-call validation** — trigger: first test case where `->` appears mid-block
- [ ] **Singleton entity auto-detection in dialogue** — trigger: first dialogue test that uses a `[Singleton]` speaker
- [ ] **Component field flattening with defaults** — trigger: first entity test with partially-overridden `use` clause
- [ ] **Derived operator auto-generation** (`!=`, `>`, `<=`, `>=` from `Eq`/`Ord`) — trigger: first test that expects derived operators

### Future Consideration (v2+)

- [ ] **Diagnostic span enrichment** (`LoweringOrigin` metadata on spans) — defer until IDE tooling work begins; requires consumer infrastructure to be useful
- [ ] **Incremental / query-based lowering** — defer until build performance becomes a demonstrated bottleneck
- [ ] **Concurrency semantic validation** (e.g., `join` on a cancelled handle) — defer to runtime or a dedicated semantic pass; spec §20 does not define compile-time checks for this

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| AST type hierarchy | HIGH | HIGH | P1 |
| Source span preservation | HIGH | LOW (design constraint) | P1 |
| Error accumulation | HIGH | LOW (design constraint) | P1 |
| Multi-pass pipeline | HIGH | MEDIUM | P1 |
| Dialogue lowering | HIGH | HIGH | P1 |
| Speaker context tracking | HIGH | MEDIUM | P1 |
| Localization key generation | HIGH | MEDIUM | P1 |
| Entity lowering | HIGH | HIGH | P1 |
| Operator lowering | HIGH | MEDIUM | P1 |
| Optional sugar lowering | HIGH | LOW | P1 |
| Compound assignment desugaring | HIGH | LOW | P1 |
| Formattable string lowering | HIGH | LOW | P1 |
| Concurrency pass-through | MEDIUM | LOW | P1 |
| Localization key collision detection | MEDIUM | LOW | P2 |
| Transition tail-call validation | MEDIUM | LOW | P2 |
| Singleton entity auto-detection | HIGH | MEDIUM | P2 |
| Component field flattening | MEDIUM | MEDIUM | P2 |
| Derived operator auto-generation | MEDIUM | MEDIUM | P2 |
| Diagnostic span enrichment | LOW | HIGH | P3 |
| Incremental lowering | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for milestone completion
- P2: Should have; add once core pipeline tests pass
- P3: Nice to have; future milestone

---

## Competitor / Reference Implementation Analysis

This is a compiler for a domain-specific language, not a product competing in a market. "Competitors" here are reference implementations that inform what table stakes and differentiators look like.

| Feature | rustc (AST→HIR) | GHC (Core desugaring) | Ink (dialogue DSL) | Our Approach |
|---------|-----------------|----------------------|-------------------|--------------|
| Separate AST from CST | YES — HIR is distinct from AST | YES — Core is distinct from Haskell AST | Partial — compiles to bytecode, no explicit IR | Separate AST type hierarchy in `writ-compiler` |
| Span preservation | YES — every HIR node has Span; `LoweringContext` enforces usage | YES — every Core expr has SrcSpan | NO — dialogue compilers typically lose spans | AstNode carries SimpleSpan from CST origin |
| Node identity | YES — `HirId` (owner + local_id) | YES — Unique per Core binding | NO | AstNode carries stable NodeId derived from CST span |
| Error nodes / continuation | YES — `Ty::Error`, continues through type checking | YES — continues after desugaring errors | N/A (small scripts, abort-on-error) | `AstNode::Error(span, message)`, pipeline continues |
| Multi-pass | YES — separate functions per construct in `rustc_ast_lowering` | YES — distinct phases | NO — single pass | Ordered independent passes composed in pipeline runner |
| Derived operators | NO — user must impl all | YES — `deriving` mechanism in GHC | N/A | Auto-generate `!=`, `>`, `<=`, `>=` from §17.4 |
| Localization keys | N/A | N/A | Partial — Ink has L10N concept | FNV-1a auto-keys per spec §28.4 |
| Dialogue-specific lowering | N/A | N/A | YES — central feature | `dlg` → `fn` with runtime call emission |

---

## Sources

- [Lowering AST to HIR — Rust Compiler Dev Guide](https://rustc-dev-guide.rust-lang.org/hir/lowering.html) — HIGH confidence (official docs)
- [Overview of the Compiler — Rust Compiler Dev Guide](https://rustc-dev-guide.rust-lang.org/overview.html) — HIGH confidence (official docs)
- [rustc_ast_lowering — nightly rustc docs](https://doc.rust-lang.org/stable/nightly-rustc/rustc_ast_lowering/index.html) — HIGH confidence (official docs)
- [Desugaring the Relationship Between Concrete and Abstract Syntax — Thunderseethe's Devlog](https://thunderseethe.dev/posts/desugar-base/) — MEDIUM confidence (single author, well-reasoned, CST-to-AST specific)
- [Multi-pass compiler — Wikipedia](https://en.wikipedia.org/wiki/Multi-pass_compiler) — MEDIUM confidence (general reference)
- [Compiler Architecture — Braid](https://capra.cs.cornell.edu/braid/docs/hacking.html) — MEDIUM confidence (academic project, game-adjacent)
- [Writing my own dialogue scripting language — dansalva.to](https://dansalva.to/writing-my-own-dialogue-scripting-language/) — LOW confidence (single practitioner, useful for dialogue-specific patterns)
- Writ language spec §13 (Dialogue Blocks), §14 (Entities), §15 (Components), §17 (Operators), §19 (Optionals), §20 (Concurrency), §28 (Lowering Reference) — HIGH confidence (authoritative spec for this project)
- `writ-parser/src/cst.rs` — HIGH confidence (existing implementation, definitive CST node inventory)

---

*Feature research for: CST-to-AST lowering pipeline (Writ compiler)*
*Researched: 2026-02-26*
