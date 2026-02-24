# Pitfalls Research

**Domain:** Compiler CST-to-AST lowering pipeline (Rust, Writ language)
**Researched:** 2026-02-26
**Confidence:** MEDIUM — core pitfalls drawn from compiler literature, Rust compiler design (rustc HIR lowering), and Writ's own spec semantics. Domain is well-understood; Writ-specific gotchas are inferred from spec rules.

---

## Critical Pitfalls

### Pitfall 1: Span Tombstoning — Generated Nodes With No Source Origin

**What goes wrong:**
Desugared nodes (the `say()` calls, `Entity.getOrCreate<T>()` calls, `Option::None` substitutions) are synthetic — they did not exist in the original source. If these nodes carry no span, or carry a fabricated dummy span, then any downstream error that touches them produces a diagnostic with no location, or a nonsensical location (e.g., pointing at byte 0). This is especially severe for dialogue lowering: a single `dlg` block with 20 `say()` calls generates 20 synthetic call nodes, all of which need accurate origin tracking.

**Why it happens:**
Lowering passes add structure that the user never wrote. It is tempting to `todo!()` or use `SimpleSpan::new(0, 0)` for generated node spans and defer the problem. The problem does not surface until a downstream pass reports an error, by which point the span infrastructure is locked in.

**How to avoid:**
Every generated AST node must carry a `lowered_from: SimpleSpan` field pointing to the CST node that caused its generation. For `say(_narrator, "text")` calls generated from `@Narrator text.`, this is the span of the `@` attribution line in the source. Establish this convention in the `AstNode` base type before writing any lowering pass — not after. Require it in code review.

**Warning signs:**
- Any lowering pass that constructs an AST node without referencing a CST span.
- Diagnostics downstream that say "error at 0:0" or point to the start of a file.
- Synthetic node constructors without a `span:` argument.

**Phase to address:** AST type definitions phase (the first phase). The span-carrying convention must be baked into the AST before any lowering pass is written.

---

### Pitfall 2: Speaker Resolution Short-Circuiting — Missing the Three-Tier Lookup

**What goes wrong:**
Dialogue lowering must resolve `@Speaker` through three tiers: (1) local variables/parameters, (2) `[Singleton]` entities with a `Speaker` component via `Entity.getOrCreate<T>()`, (3) compile error. If the lowering pass only implements tier 2 (the common case — `Narrator`, `OldTim`), then dialogue blocks that pass an entity as a parameter (`dlg shopScene(guard: Guard)`) silently produce incorrect lowered output: a `Entity.getOrCreate<Guard>()` call instead of a direct reference to the `guard` parameter. This is a semantic bug, not a type error — it compiles and runs but creates a new singleton instead of using the passed guard instance.

**Why it happens:**
The spec's example (Section 13.2) shows singleton speakers prominently. Singleton resolution is straightforward. Parameter resolution requires the lowering pass to carry a symbol table of in-scope names, which is more infrastructure. Teams often prototype tier 2 first and never come back to tier 1.

**How to avoid:**
Implement all three tiers in the same pass with explicit branching. The lowering pass for `dlg` must accept a scope context (a map of parameter names to their types) and check it before attempting singleton resolution. Write a test case for `dlg scene(guard: Guard) { @guard Halt! }` on day one of the dialogue lowering phase — this is the tier-1 case and must be a first-class test.

**Warning signs:**
- Dialogue lowering tests only cover `@Narrator`, `@OldTim`, and other global singletons.
- No test for `@paramName` where the parameter is typed as an entity.
- The lowering pass does not accept or maintain a parameter scope map.

**Phase to address:** Dialogue lowering phase. Must be addressed before the first version of dialogue lowering ships.

---

### Pitfall 3: Pass Ordering Inversion — Running a Pass Before Its Dependencies Are Lowered

**What goes wrong:**
If the operator lowering pass runs before the formattable string lowering pass, then formattable strings inside operator implementations (`$"value is {x}"`) are still CST-level `FmtString` nodes when the operator pass tries to process them. The operator pass either panics, ignores them, or generates malformed AST. More generally, any pass that expects "clean" sub-expressions will fail if those sub-expressions still contain constructs that another pass is responsible for lowering.

**Why it happens:**
Each pass is written and tested in isolation against the CST. When the full pipeline is assembled, ordering is assumed to be obvious. But the actual ordering constraints are not documented and become implicit knowledge.

**How to avoid:**
Define the pipeline's pass order explicitly as a data structure (a `Vec<Box<dyn LoweringPass>>`) with comments on each entry explaining what invariant it assumes and what invariant it produces. For Writ, the natural order is: (1) optional sugar `T?` → `Option<T>`, (2) formattable strings `{expr}` → concatenation, (3) operator lowering, (4) dialogue lowering (depends on formattable strings being resolved for text interpolation), (5) entity lowering. Document this order in the pipeline module's rustdoc.

**Warning signs:**
- Integration tests failing with "unexpected node type" panics at a specific pass.
- Lowering passes that contain `match` arms handling both lowered and un-lowered node variants as a workaround.
- No documented ordering rationale in the pipeline assembler.

**Phase to address:** Multi-pass pipeline architecture phase. The order must be established and documented before individual passes are written, not after they are assembled.

---

### Pitfall 4: Dialogue Transition (`->`) Emitted as a Non-Terminal Statement

**What goes wrong:**
The spec defines `->` as always terminal — a tail call that does not return. The lowering rule is `-> otherDialog` → `return otherDialog()`. If the lowering pass emits `otherDialog()` (a bare call without `return`), execution falls through to the next statement in the lowered `fn` body. In a dialogue block where `->` appears inside a `$ choice` branch, this means execution continues into subsequent `say()` calls after the choice branch finishes, producing dialogue that plays content it should not.

**Why it happens:**
The `->` operator in CST is a statement, and the lowering rule produces a call expression. It is easy to emit the call and forget to wrap it in a `return` statement. The spec's note ("-> is always terminal") is easy to overlook when focused on argument passing.

**How to avoid:**
The AST node for a transition must be `AstReturn(AstCall(...))` — not a bare `AstCall`. Write a lowering helper `lower_transition(target, args)` that always produces a `Return`-wrapped call and is the only way to emit a transition. Make it impossible to emit a bare transition call without going through this helper.

**Warning signs:**
- Dialogue that continues speaking after a `->` in test output.
- The lowering pass emitting `AstCall` for transitions rather than `AstReturn(AstCall(...))`.
- Absence of tests that verify the terminal semantics (i.e., statements after `->` must be unreachable).

**Phase to address:** Dialogue lowering phase.

---

### Pitfall 5: Localization Key Collision — Hash Instability Across Lowering Runs

**What goes wrong:**
The spec (Section 28.4) requires auto-generated FNV-1a keys for dialogue text. If the key is computed from the text content alone, then two dialogue lines with identical text in different `dlg` blocks produce the same key. This is a localization collision: one string table entry maps to two dialogue lines that may require different translations. The runtime will display the same translation for both, regardless of context.

A second variant: if the key computation is not stable across compiler runs (e.g., it incorporates a node ID that changes when unrelated code is added), then every compile breaks the localization string table, invalidating all existing translations.

**Why it happens:**
FNV-1a on raw text is the obvious first implementation. Collisions are not caught until a localization team loads the string table and notices duplicates. Key instability is not caught until CI re-runs the compiler and compares outputs.

**How to avoid:**
The key must incorporate both the text content AND the fully-qualified dialogue block name (e.g., `FNV1a("greetPlayer::line3::Hey, " + name_placeholder)`). The block name ensures two identical strings in different contexts get different keys. The line index provides further disambiguation within a block. The hash input must be deterministic and source-derived, not runtime-node-derived. Write a test that compiles the same source twice (with an unrelated change between runs) and asserts key stability.

**Warning signs:**
- Two dialogue lines with identical text producing identical keys in test output.
- Key values changing between compiler runs with no source change.
- Hash computation that incorporates any non-source-derived value (pointers, allocation order, timestamps).

**Phase to address:** Localization key generation phase.

---

### Pitfall 6: Entity Component Default Resolution — Property vs. Component Field Confusion

**What goes wrong:**
The entity lowering rule (Section 28.3, Section 14.6) must generate both a struct with component fields AND a constructor that applies defaults. The defaults have two sources: entity-level property defaults (`name: string = "Guard"`) and component-level field defaults (`use Health { current: 80, max: 80 }`). If the lowering pass conflates these — treating both as flat struct field defaults — then component fields are initialized at the wrong level, breaking the `ComponentAccess<T>` contract implementation.

**Why it happens:**
In the CST, `EntityDecl` mixes property declarations and `use` (component) declarations in the same `members` list. A naive lowering pass that flattens all defaults into a single struct initializer misses the distinction between "this entity's own fields" and "this component's internal fields."

**How to avoid:**
During entity lowering, partition the members into properties, component `use` declarations, lifecycle hooks, and methods before generating any output. Each partition drives a different piece of lowered output: properties → struct fields, components → nested component struct fields + `ComponentAccess` impls, hooks → registered callbacks, methods → `impl` methods. Write this partitioning as an explicit pre-lowering step with clear data structures, not as inline branching during code generation.

**Warning signs:**
- Entity lowering producing a flat struct where component fields are at the same level as entity properties.
- No distinct `ComponentAccess<T>` impl generated per `use` declaration.
- Test entities with two components that have a field of the same name not producing a compile error (they should conflict without explicit access).

**Phase to address:** Entity lowering phase.

---

### Pitfall 7: Concurrency Primitive Pass-Through — `spawn` Keyword Ambiguity

**What goes wrong:**
The Writ spec uses `spawn` for two distinct things: entity instantiation (`spawn Guard { ... }`) and background task creation (`spawn moveBoulder(vec2(10, 5))`). The entity lowering pass and the concurrency pass-through must not interfere. If the entity lowering pass eagerly transforms all `spawn` expressions before the concurrency pass runs, it will attempt to lower task spawns as entity instantiations, producing malformed output. Conversely, if the concurrency pass processes entity spawns, it will emit wrong AST.

**Why it happens:**
Both forms use the `spawn` keyword and appear in similar statement positions. The CST may or may not distinguish them syntactically depending on how the parser disambiguates. If the CST uses a single `Spawn` node with a sub-expression, each pass must inspect the sub-expression to determine which kind of spawn it is — creating coupling between passes.

**How to avoid:**
Verify the CST's representation of `spawn`. If the parser already distinguishes `SpawnEntity` from `SpawnTask`, lowering passes can filter by node type with no risk of collision. If not, file a parser issue and add disambiguation during the first lowering pass before any other pass runs. Either way, write a test with a `dlg` that contains both kinds of `spawn` in the same block (entity spawn in a code block, task spawn for a background animation) and assert correct lowering of both.

**Warning signs:**
- The CST uses a single `Spawn` enum variant for both entity and task spawn.
- Lowering passes that handle `spawn` without checking the sub-expression type.
- No tests containing both spawn forms in the same context.

**Phase to address:** Multi-pass pipeline architecture phase (disambiguation), then entity and concurrency phases (consumers).

---

### Pitfall 8: Active Speaker State Not Scoped Across `$ choice` Branches

**What goes wrong:**
The spec (Section 13.2) defines `@speaker` on its own line as setting the active speaker for all subsequent lines "until the next `@` or end of block." Inside a `$ choice` branch, the active speaker at the start of the branch may be inherited from the enclosing context. If the lowering pass implements active speaker as a single mutable variable threaded through the dialogue lowering, a branch that changes the active speaker (`@Player Things are rough.`) can leak that speaker into sibling branches or into continuation lines after the choice block.

**Why it happens:**
Mutable state is the simplest implementation. The leakage is subtle: in linear dialogue it works correctly. It only fails in branching constructs (`$ choice`, `$ if`) where each branch has an independent speaker context that must not cross-contaminate.

**How to avoid:**
Implement the active speaker as a scoped stack: push a copy of the current state when entering any branching construct, pop on exit. Each branch of a `$ choice` or `$ if/else` operates on its own speaker context copy. The post-branch speaker state reverts to whatever it was before the branch.

**Warning signs:**
- A `$ choice` branch with `@Player` affecting speaker attribution in a sibling branch.
- Test dialogue where a choice branch changes speaker and subsequent non-branch lines use the wrong speaker.
- The lowering pass using a single `current_speaker: Option<&str>` variable instead of a stack.

**Phase to address:** Dialogue lowering phase.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `todo!()` for `T?` → `Option<T>` in the dialogue pass | Dialogue lowering ships without optional sugar support | Every downstream pass that encounters `T?` in a dialogue context panics | Never — optional sugar must be lowered before dialogue |
| Dummy spans (`SimpleSpan::new(0, 0)`) on generated nodes | Avoids span infrastructure work | All lowering-stage errors point at byte 0; IDE integration breaks | Never for generated nodes that can produce errors |
| Hardcoding `Narrator` and `Player` as known singletons | Simplifies prototype speaker resolution | Breaks for user-defined singleton speakers; becomes a permanent exception list | Never — use the three-tier lookup from day one |
| Flat struct for entity lowering (skip `ComponentAccess` impls) | Entity struct generation works for simple tests | `guard[Health]` component access syntax cannot be implemented without the impls | MVP only, with explicit TODO and failing test |
| Computing FNV-1a keys from text content alone | Simple one-liner key generation | Key collisions for identical text in different dialogue blocks | Never — include block name in hash input |
| Single mutable `current_speaker` in dialogue lowering | Easy to implement | Speaker leaks across `$ choice` branches silently | Never — use a stack from the first implementation |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| CST `Spanned<T>` → AST span field | Dropping the `SimpleSpan` during pattern matching on the inner `T` | Always destructure as `(node, span)` and thread `span` into every generated AST node |
| `writ-parser::cst` types consumed directly | Assuming all CST nodes are well-formed (parser has error recovery) | Handle `Error` recovery nodes gracefully — they may appear anywhere in a dialogue body |
| `Entity.getOrCreate<T>()` call generation | Generating the call with a string name instead of a type parameter | The type parameter must be the resolved entity type name, not the speaker string as used in source |
| FNV-1a hash computation | Using Rust's default `std::hash` (which is randomized by default via `RandomState`) | Use the `fnv` crate or a hand-rolled FNV-1a with a fixed seed; never use `std::collections::HashMap` for key generation |
| `-> transition` inside `$ if` / `$ match` | Validating that `->` is the last statement only at the `dlg` body level | `->` must be the terminal statement within its immediate block — validate at each block boundary |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Re-scanning the entire CST to find all speaker references before lowering each `dlg` | Slow lowering for files with many `dlg` blocks | Collect all singleton entity declarations in a pre-pass, build a lookup table, reuse it | When a file has 50+ `dlg` blocks — O(n²) speaker lookup |
| Cloning the entire CST before each lowering pass | Memory usage spikes; slowness on large files | Lower in a single traversal where possible; use references into the CST | When CST size exceeds a few MB |
| Generating localization keys synchronously during lowering with I/O | Slows the lowering pipeline | Key generation must be pure (hash-only); I/O (writing to string tables) is a post-lowering step | Any file with 100+ dialogue lines |

---

## "Looks Done But Isn't" Checklist

- [ ] **Dialogue lowering:** `-> name(args)` with arguments — verify args are lowered through the transition, not dropped.
- [ ] **Dialogue lowering:** `@speaker` on its own line followed by multiple continuation lines — verify all continuations use that speaker, not just the first.
- [ ] **Dialogue lowering:** `{expr}` interpolation in `$ choice` option labels — verify labels go through formattable string lowering, not just body lines.
- [ ] **Dialogue lowering:** `#key` on a `$ choice` option label — verify the manual key is preserved on the `Option(...)` AST node, not discarded.
- [ ] **Entity lowering:** `on interact(who: Entity) { -> guardDialog(self, who) }` — verify `->` inside lifecycle hooks lowers correctly (the hook body is dialogue-like but inside an entity, not a `dlg`).
- [ ] **Entity lowering:** Component fields with the same name across two components — verify the lowering produces a compile error (Section 14.3), not silent shadowing.
- [ ] **Span preservation:** Every lowered `fn` that originated from a `dlg` — verify the function's span points at the `dlg` keyword, not the first `say()` call.
- [ ] **Localization:** Two `dlg` blocks with a line `"Hello."` in each — verify the generated keys are distinct.
- [ ] **Concurrency:** `defer` block inside a `dlg` scope — verify it is preserved as an AST node and not discarded as "not dialogue-related."
- [ ] **Error accumulation:** A `dlg` with an unknown speaker — verify the error is collected and lowering continues for the rest of the block, producing at most one error per unknown speaker.

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Span tombstoning discovered after multiple passes are written | HIGH | Audit every AST node type; add `span: SimpleSpan` field; update all constructors; update all lowering passes — likely a full-day refactor |
| Pass ordering inversion causing cascading failures | MEDIUM | Reorder passes in the pipeline assembler; add integration tests at each ordering boundary; run full test suite to verify |
| Speaker resolution missing tier-1 (parameter) lookup | MEDIUM | Extend the lowering pass to accept a scope context; add parameter-name-to-type map; add tests for `@param` forms; no AST type changes required |
| Localization key collision discovered by localization team | HIGH | Recompute all keys with block-qualified hash inputs; invalidate existing string tables; coordinate with localization team to re-export |
| `spawn` ambiguity causing entity-as-task lowering | MEDIUM | Add disambiguation either in parser (preferred) or as a pre-pass; update affected lowering tests |
| Active speaker leaking across `$ choice` branches | LOW | Replace mutable variable with stack; all dialogue lowering tests should catch this immediately after the fix |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Span tombstoning on generated nodes | Phase 1: AST type definitions | Every AST node constructor requires a `span` argument; CI rejects any `SimpleSpan::new(0, 0)` in lowering code |
| Speaker resolution tier-1 missing | Phase: Dialogue lowering | Test `dlg scene(guard: Guard) { @guard Halt! }` → `guard` direct reference (not `getOrCreate`) |
| Pass ordering inversion | Phase: Multi-pass pipeline architecture | Document pass order as ordered list; integration test that lowers formattable strings in an operator body |
| `->` emitted without `return` | Phase: Dialogue lowering | Test that statements after `->` in a branch are unreachable in lowered output |
| Localization key collision | Phase: Localization key generation | Test two `dlg` blocks with identical text → distinct keys |
| Entity component/property conflation | Phase: Entity lowering | Test entity with two components having a same-name field → produces compile error |
| `spawn` keyword ambiguity | Phase: Multi-pass pipeline architecture | Test `dlg` with both `spawn Guard {}` and `spawn task()` in same block → correct distinct lowering |
| Active speaker leaking across branches | Phase: Dialogue lowering | Test `$ choice` branch that changes speaker → sibling branch uses original speaker |
| Hash instability across compiler runs | Phase: Localization key generation | CI test: compile same source twice, diff localization key output — must be identical |
| Error cascading from unknown speaker | Phase: Dialogue lowering | Test `dlg` with one unknown speaker + valid subsequent lines → exactly one error, valid lowering for valid lines |

---

## Sources

- [Desugaring the Relationship Between Concrete and Abstract Syntax — Thunderseethe's Devlog](https://thunderseethe.dev/posts/desugar-base/) — span/CST mapping, desugaring infrastructure patterns (MEDIUM confidence)
- [Make AST→HIR lowering incremental — rust-lang/compiler-team #452](https://github.com/rust-lang/compiler-team/issues/452) — span preservation at scale (MEDIUM confidence)
- [What is the correct desugaring for index expressions? — rust-lang/reference #651](https://github.com/rust-lang/reference/issues/651) — desugaring complexity, lang item paths vs regular paths (MEDIUM confidence)
- [Post-Modern Compiler Design Vol. 1: Error Handling — Purdue](https://www.cs.purdue.edu/homes/rompf/pmca/vol1/error-handling-1.html) — accumulation vs fail-fast strategies (MEDIUM confidence)
- [The difference between compiling and lowering — Hacker News](https://news.ycombinator.com/item?id=14425039) — pass ordering, idempotency principle (LOW confidence, community discussion)
- [CMU 15411 Compiler Pipeline — tianboh/compiler](https://github.com/tianboh/compiler) — elaboration pass ordering reference (MEDIUM confidence)
- Writ language spec: `language-spec/spec/29_28_lowering_reference.md` — canonical lowering rules (HIGH confidence)
- Writ language spec: `language-spec/spec/14_13_dialogue_blocks_dlg.md` — dialogue construct semantics (HIGH confidence)
- Writ language spec: `language-spec/spec/15_14_entities.md` — entity and component semantics (HIGH confidence)
- Writ language spec: `language-spec/spec/21_20_concurrency.md` — concurrency primitive semantics (HIGH confidence)

---
*Pitfalls research for: Writ compiler CST-to-AST lowering pipeline*
*Researched: 2026-02-26*
