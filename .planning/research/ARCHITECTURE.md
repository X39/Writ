# Architecture Research

**Domain:** Compiler CST-to-AST lowering pipeline (Rust, game scripting language)
**Researched:** 2026-02-26
**Confidence:** HIGH

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         writ-parser (existing)                           │
│                                                                          │
│  Source String → lex() → Vec<(Token, Span)> → parse() → CST + Errors   │
│                                                                          │
│  Output: (Option<Vec<Spanned<Item<'src>>>>, Vec<RichError>)              │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ CST (Item<'src>, Expr<'src>, ...)
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        writ-compiler (new)                               │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                       Lowering Pipeline                           │   │
│  │                                                                   │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │   │
│  │  │  Pass 1  │→ │  Pass 2  │→ │  Pass 3  │→ │  Pass N  │        │   │
│  │  │ Optional │  │Fmt String│  │ Operator │  │ Dialogue │        │   │
│  │  │ Lowering │  │ Lowering │  │ Lowering │  │ Lowering │        │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └────┬─────┘        │   │
│  │                                                   │              │   │
│  │  ┌──────────────────────────────────────────┐    │              │   │
│  │  │         Entity Lowering                   │    │              │   │
│  │  └──────────────────────────────────────────┘    │              │   │
│  │                                                   │              │   │
│  │  ┌──────────────────────────────────────────┐    │              │   │
│  │  │         Concurrency Pass-Through          │    │              │   │
│  │  └──────────────────────────────────────────┘    │              │   │
│  │                                                   │              │   │
│  │  ┌────────────────────────────────────────── ┐   │              │   │
│  │  │    LoweringContext (shared state carrier)  │   │              │   │
│  │  │  - span map: CST span → AST node          │   │              │   │
│  │  │  - errors: Vec<LoweringError>             │   │              │   │
│  │  │  - speaker scope stack                    │   │              │   │
│  │  │  - localization key counter               │   │              │   │
│  │  └────────────────────────────────────────── ┘   │              │   │
│  └──────────────────────────────────────────────────┘              │   │
│                                                                          │
│  Output: (Ast, Vec<LoweringError>)                                       │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ AST (simplified, span-carrying)
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              Downstream (future phases — type checking, codegen)          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Communicates With |
|-----------|----------------|-------------------|
| `writ-parser` (existing) | Produces full-fidelity CST from source strings | Provides `Item<'src>`, `Expr<'src>`, spans to lowering pipeline |
| `ast` module (new) | Defines the simplified AST node type hierarchy (no trivia, no CST sugar) | Consumed by all passes and by downstream type checker |
| `LoweringContext` (new) | Carries shared mutable state during lowering: errors, span map, speaker scope, loc key generator | Threaded through every pass |
| `lower_optional` pass | Rewrites `T?` → `AstType::Option(T)` and `null` → `AstExpr::OptionNone` | Consumes CST `TypeExpr` and `Expr::Null`; produces AST nodes |
| `lower_fmt_string` pass | Rewrites `FormattableString` interpolation segments → `AstExpr::BinOp(Concat, ...)` with `.into<string>()` calls | Consumes `Expr::FormattableString`; produces AST `Call` + concat chain |
| `lower_operator` pass | Rewrites operator expressions → contract impl method calls (Add, Sub, etc.) | Consumes `Expr::Binary` with operator token; produces `AstExpr::MethodCall` |
| `lower_dialogue` pass | Rewrites `DlgDecl` → `AstFnDecl` with `say()`, `choice()`, speaker resolution, `-> transition` → `return` | Most complex pass; consumes `Item::Dlg` and all `DlgLine` variants |
| `lower_entity` pass | Rewrites `EntityDecl` → struct + component fields + `ComponentAccess` impls + lifecycle hook registrations | Consumes `Item::Entity`; produces multiple AST `StructDecl` + `ImplDecl` nodes |
| `lower_localization` pass | Adds FNV-1a key generation to `say()` calls; handles `#key` overrides | Runs after dialogue lowering; consumes intermediate `AstExpr::Call("say", ...)` |
| Concurrency pass-through | `spawn`, `join`, `cancel`, `defer` are valid AST primitives — no desugaring, just CST→AST mapping | Consumes `Expr::Spawn`, `Expr::Join`, etc.; produces equivalent AST nodes |
| Pipeline orchestrator | Sequences passes, threads `LoweringContext`, collects errors, returns final `(Ast, Vec<LoweringError>)` | Entry point for the compiler crate |

## Recommended Project Structure

```
writ-compiler/src/
├── lib.rs                  # Public API: lower(cst: Vec<Spanned<Item>>) -> (Ast, Vec<LoweringError>)
├── main.rs                 # CLI entry point (thin wrapper around lib)
├── ast/
│   ├── mod.rs              # AST type definitions — simplified node set
│   ├── expr.rs             # AstExpr variants (no FormattableString, no T?, etc.)
│   ├── stmt.rs             # AstStmt variants
│   ├── decl.rs             # AstDecl variants (AstFnDecl, AstStructDecl, etc.)
│   └── types.rs            # AstType variants (Option<T> unwrapped, no ? sugar)
├── lower/
│   ├── mod.rs              # Pipeline orchestrator: sequences all passes
│   ├── context.rs          # LoweringContext: shared state, error collection, span map
│   ├── error.rs            # LoweringError type with source spans
│   ├── optional.rs         # Pass: T? → Option<T>, null → Option::None
│   ├── fmt_string.rs       # Pass: FormattableString → concat + .into<string>()
│   ├── operator.rs         # Pass: operator overloads → contract method calls
│   ├── dialogue.rs         # Pass: dlg → fn with say/choice/speaker resolution
│   ├── entity.rs           # Pass: entity → struct + impls + lifecycle hooks
│   ├── localization.rs     # Pass: say() → say_localized() with FNV-1a keys
│   └── concurrency.rs      # Pass-through: spawn/join/cancel/defer → AST primitives
└── util/
    ├── fnv.rs              # FNV-1a hash computation for localization keys
    └── span.rs             # Span utilities: CST span → AST span mapping
```

### Structure Rationale

- **`ast/`:** The AST is a first-class module, not embedded in lowering. Downstream consumers (type checker, codegen) import from `ast` directly without depending on lowering internals.
- **`lower/`:** Each pass is its own file. Adding a new desugaring = adding one file and registering it in `mod.rs`. No existing file changes.
- **`lower/context.rs`:** All shared mutable state lives in one struct. Passes receive `&mut LoweringContext` — no hidden global state, fully testable in isolation.
- **`util/`:** Stateless helpers (FNV hash, span utilities) kept separate from pass logic to keep passes focused.

## Architectural Patterns

### Pattern 1: Pass-Based Pipeline with Shared Context

**What:** Each desugaring is a function with the signature `fn lower_X(ctx: &mut LoweringContext, node: CstNode) -> AstNode`. Passes are composed by the orchestrator in `lower/mod.rs`, which calls them in dependency order. Each pass is independently testable.

**When to use:** When desugarings are semantically independent (optional lowering does not depend on dialogue lowering). This is the pattern used by `rustc_ast_lowering` with its `LoweringContext` struct.

**Trade-offs:** Linear pass ordering means passes cannot see each other's output (no lookahead between passes). This is acceptable here because all Writ desugarings are defined to be independent and sequential (spec Section 28 does not describe any interaction between lowerings).

**Example (Rust):**
```rust
// lower/mod.rs — pipeline orchestrator
pub fn lower(items: Vec<Spanned<Item<'_>>>) -> (Ast, Vec<LoweringError>) {
    let mut ctx = LoweringContext::new();
    let ast_items: Vec<AstItem> = items
        .into_iter()
        .flat_map(|(item, span)| lower_item(&mut ctx, item, span))
        .collect();
    (Ast { items: ast_items }, ctx.take_errors())
}

fn lower_item(ctx: &mut LoweringContext, item: Item<'_>, span: SimpleSpan) -> Vec<AstItem> {
    match item {
        Item::Dlg(decl)    => vec![lower_dialogue(ctx, decl)],
        Item::Entity(decl) => lower_entity(ctx, decl),   // one entity → multiple AST items
        Item::Fn(decl)     => vec![lower_fn(ctx, decl)],
        // ... other variants pass through with structural lowering only
    }
}
```

### Pattern 2: Fold-Based Node Transformation (not Visitor-Based)

**What:** Each pass is a *fold* — it consumes CST nodes and produces new AST nodes. This is distinct from a *visitor* (which traverses in-place). In Rust terms, the function takes ownership or a reference and returns a new owned value.

**When to use:** When the output type differs from the input type (CST → AST). Visitor works when mutating in-place; fold works when building a new tree. Since the CST and AST are separate type hierarchies, fold is the correct choice.

**Trade-offs:** Fold requires materializing the entire AST as a new allocation. For a game scripting compiler (not a megaproject like LLVM), this is fine — the allocation cost is negligible. The benefit is type safety: the compiler enforces you cannot accidentally leave CST nodes in the AST output.

**Example (Rust):**
```rust
// lower/optional.rs — fold pattern: CST TypeExpr → AST AstType
pub fn lower_type(ctx: &mut LoweringContext, ty: Spanned<TypeExpr<'_>>) -> AstType {
    let (type_expr, span) = ty;
    match type_expr {
        TypeExpr::Optional(inner) => {
            // T? → Option<T>
            AstType::Generic {
                name: "Option",
                args: vec![lower_type(ctx, *inner)],
                span: ctx.map_span(span),
            }
        }
        TypeExpr::Named(name) => AstType::Named { name: name.to_string(), span: ctx.map_span(span) },
        // ... other variants
    }
}
```

### Pattern 3: LoweringContext as State Carrier

**What:** A single struct (`LoweringContext`) holds all mutable cross-cutting state: the accumulated error list, the span mapping table (CST offset → AST span), and any pass-local stacks (speaker scope for dialogue, localization key counter). Every pass receives `&mut LoweringContext`.

**When to use:** Always — this is the pattern used by both `rustc` (`LoweringContext` in `rustc_ast_lowering`) and Swift's SIL generation. It prevents hidden global state and makes passes unit-testable by constructing a fresh `LoweringContext` per test.

**Trade-offs:** None meaningful at this scale. At LLVM/rustc scale, a shared context becomes a bottleneck for parallelism; for a scripting language compiler this is not a concern.

**Example (Rust):**
```rust
// lower/context.rs
pub struct LoweringContext {
    /// Accumulated errors with source spans.
    errors: Vec<LoweringError>,
    /// Stack of currently-in-scope default speakers (for dlg blocks).
    speaker_stack: Vec<SpeakerScope>,
    /// Counter for auto-generated localization keys.
    loc_key_counter: u32,
}

impl LoweringContext {
    pub fn emit_error(&mut self, err: LoweringError) { self.errors.push(err); }
    pub fn take_errors(self) -> Vec<LoweringError> { self.errors }
    pub fn push_speaker(&mut self, s: SpeakerScope) { self.speaker_stack.push(s); }
    pub fn pop_speaker(&mut self) { self.speaker_stack.pop(); }
    pub fn current_speaker(&self) -> Option<&SpeakerScope> { self.speaker_stack.last() }
    pub fn next_loc_key(&mut self) -> u32 { let k = self.loc_key_counter; self.loc_key_counter += 1; k }
}
```

### Pattern 4: Pass Registration via `match` Dispatch (not a Registry)

**What:** The orchestrator uses a `match` over `Item` variants to dispatch to specific passes. There is no dynamic pass registry, no trait objects, no plugin system. Adding a new pass means adding a new match arm and a new file.

**When to use:** When the set of passes is known at compile time (which it always is for a language-defined lowering). Dynamic registries add complexity with no benefit here.

**Trade-offs:** Less flexible than a dynamic registry but safer and simpler. The `match` is exhaustive, so forgetting to handle a new CST variant is a compile error.

## Data Flow

### Lowering Pass Flow

```
CST (Vec<Spanned<Item<'src>>>)          [from writ-parser::parse()]
    │
    ▼
lower/mod.rs: lower()
    │
    ├─→ lower_item() matches on Item variant
    │       │
    │       ├─→ Item::Fn   → lower_fn() → lower types and exprs recursively
    │       │       └─→ lower_type() [optional.rs]  ← T? / null
    │       │       └─→ lower_expr() [fmt_string.rs] ← FormattableString
    │       │       └─→ lower_expr() [operator.rs]   ← Binary ops
    │       │       └─→ lower_expr() [concurrency.rs] ← spawn/join/cancel/defer
    │       │
    │       ├─→ Item::Dlg  → dialogue.rs:lower_dialogue()
    │       │       └─→ Speaker resolution → Entity.getOrCreate<T>() calls
    │       │       └─→ SpeakerLine/TextLine → say() calls
    │       │       └─→ Choice → choice([...]) call
    │       │       └─→ Transition → return call
    │       │       └─→ CodeEscape → passthrough to lower_stmt()
    │       │       └─→ localization.rs: decorate say() → say_localized()
    │       │
    │       └─→ Item::Entity → entity.rs:lower_entity()
    │               └─→ Component fields → struct fields
    │               └─→ ComponentAccess impls (one per component)
    │               └─→ Lifecycle hook registrations
    │
    ├─→ LoweringContext threaded through all passes (errors, speaker stack, loc keys)
    │
    └─→ (Ast { items: Vec<AstItem> }, Vec<LoweringError>)
```

### Span Preservation Flow

```
CST node carries SimpleSpan (byte offsets into source string)
    │
    ▼ ctx.map_span(cst_span)
    │
AST node carries AstSpan (same byte offsets, no re-computation)
    │
    ▼
LoweringError references AstSpan → points back to original source
    │
    ▼
Downstream type checker / diagnostics can display source lines
```

Spans are carried through as-is. The `LoweringContext` does not remap them — CST spans are byte offsets into the original source string and remain valid in the AST. No span arithmetic required.

### Key Data Flows

1. **Optional lowering:** `TypeExpr::Optional(Box<TypeExpr>)` in CST → `AstType::Generic { name: "Option", args: [...] }` in AST. Happens inside `lower_type()`, called recursively from every position that lowers a type.
2. **Formattable string lowering:** `Expr::FormattableString(Vec<FmtSegment>)` → chain of `AstExpr::BinOp(Concat, str_lit, call(".into<string>()"))`. Happens inside `lower_expr()`, called from expression positions.
3. **Dialogue lowering:** `Item::Dlg(DlgDecl)` → `AstItem::Fn(AstFnDecl)`. The most structurally complex transformation: `DlgDecl.body: Vec<DlgLine>` → `Vec<AstStmt>`. The speaker stack in `LoweringContext` tracks the active `@speaker` across lines.
4. **Entity lowering:** `Item::Entity(EntityDecl)` → `Vec<AstItem>` (one `AstItem::Struct` + N `AstItem::Impl` for each `use Component` clause). The `lower_entity` function is the only pass that expands one CST item into multiple AST items.
5. **Concurrency pass-through:** `Expr::Spawn`, `Expr::Join`, `Expr::Cancel`, `Expr::Defer` → equivalent `AstExpr` primitives with no semantic transformation. These are first-class AST nodes; the runtime handles their semantics.

## Pass Ordering Strategy

The six substantive passes must be ordered by two constraints:

**Constraint 1 — Localization runs after dialogue.** `lower_localization` adds FNV-1a keys to `say()` calls produced by `lower_dialogue`. It must see `say()` AST nodes, not raw `DlgLine` CST nodes.

**Constraint 2 — Type and expression lowering runs inside structural passes.** Optional (`T?`) and formattable string lowering are not standalone top-level passes — they are invoked from within `lower_fn`, `lower_dialogue`, and `lower_entity` as those passes encounter type positions and expression positions. This avoids a redundant full-tree traversal.

```
Recommended pass order:

1. lower_fn        — Processes Fn items; internally calls lower_type, lower_expr
   lower_expr calls:
   ├─ lower_optional    (TypeExpr::Optional / Expr::Null → Option variants)
   ├─ lower_fmt_string  (Expr::FormattableString → concat chain)
   ├─ lower_operator    (Expr::Binary with overloadable op → method call)
   └─ lower_concurrency (Expr::Spawn/Join/Cancel/Defer → AstExpr primitives)

2. lower_dialogue   — Processes Dlg items; internally calls lower_expr (same sub-passes)
   Then calls lower_localization as a sub-pass on the produced say() calls

3. lower_entity     — Processes Entity items; internally calls lower_type for component fields

4. lower_localization — Called from within lower_dialogue, NOT as a top-level pass
   (this is a detail of implementation; conceptually it is a post-dialogue sub-pass)
```

Adding a new desugaring (e.g., formattable string format specifiers once they are spec-complete) means:
1. Add a new file `lower/fmt_specifiers.rs`
2. Call it from `lower_expr()` at the `Expr::FormattableString` match arm
3. No other file changes required

## Anti-Patterns

### Anti-Pattern 1: Single-Pass Monolithic Lowering

**What people do:** Write one giant `match` over all CST node types in a single function or file.
**Why it's wrong:** Dialogue lowering alone has ~10 cases. Entity lowering generates multiple output items. Operator lowering requires knowing which operators are overloadable. Combining these in one function produces an unmaintainable 500+ line match block where all state is local.
**Do this instead:** One file per logical desugaring. Shared state in `LoweringContext`. Orchestrator in `lower/mod.rs` dispatches at the top level only.

### Anti-Pattern 2: Using the Visitor Pattern for CST→AST Lowering

**What people do:** Implement a `Visitor` trait over CST nodes, mutating a mutable output builder inside the visitor. (This is how TypeScript's binder works for in-place annotation, but not for lowering.)
**Why it's wrong:** When input and output types differ (CST vs. AST), visitor cannot return the new type from each `visit_*` method without awkward side-channel state. The fold pattern (function returns new AST node) is cleaner.
**Do this instead:** Write `fn lower_expr(ctx: &mut LoweringContext, expr: Expr<'_>) -> AstExpr`. The return type makes the transformation contract explicit.

### Anti-Pattern 3: Query-Based Architecture (Salsa) at This Stage

**What people do:** Reach for incremental compilation infrastructure (Salsa, rustc's query system) for all compiler phases.
**Why it's wrong:** Query systems pay for themselves when incremental recompilation matters — rustc uses queries because users recompile the same codebase repeatedly. A game scripting compiler's lowering pipeline runs once per script load. The overhead of a query system (memoization, dependency tracking, cache invalidation) is not justified, and it would make the code significantly harder to understand and test.
**Do this instead:** Sequential pass pipeline returning `(Ast, Vec<LoweringError>)`. If incremental compilation becomes needed later, the clean pass separation makes introducing queries straightforward.

### Anti-Pattern 4: Discarding Spans During Lowering

**What people do:** Build AST nodes without threading span information through from the CST.
**Why it's wrong:** Every lowering error (unknown speaker, invalid transition target) must point back to original source. If spans are dropped, errors say "somewhere in this file" instead of "line 42, column 7." `rustc`, Swift, and TypeScript all preserve spans through every IR stage — this is non-negotiable for a usable compiler.
**Do this instead:** Every `AstExpr`, `AstStmt`, `AstDecl`, and `AstType` carries `span: SimpleSpan`. `lower_expr` always propagates the CST node's span to the produced AST node, even when the shape changes dramatically (e.g., a `DlgLine::SpeakerLine` produces an `AstExpr::Call` whose span points to the original speaker line).

## Integration Points

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `writ-parser` → `writ-compiler` | `writ-parser::parse()` returns `(Option<Vec<Spanned<Item<'src>>>>, Vec<RichError>)`. The compiler calls `parse()`, handles parse errors, then calls `lower()`. | CST types cross the boundary; the compiler crate takes a direct dependency on `writ-parser`. |
| `lower/mod.rs` → each pass | Each pass is a free function in its own module; `lower/mod.rs` imports and calls them. No traits, no dynamic dispatch. | Passes do not depend on each other — only on `LoweringContext` and the CST/AST types. |
| `lower/*` passes → `ast/` | Each pass returns `Ast*` types imported from `ast/`. Passes import from `ast` but do not import from other passes. | This creates a clear dependency graph: passes depend on `ast`, not on each other. |
| `writ-compiler` → downstream | Downstream phases (type checker, codegen) depend on `ast/` types and receive `(Ast, Vec<LoweringError>)`. | The AST module is the sole interface contract. Downstream never sees CST types. |

### External Services

None. The lowering pipeline is pure computation (no I/O, no network, no file system during lowering). The only external dependency is `writ-parser` (CST types) and the Rust standard library.

## Build Order (Component Dependencies)

```
1. ast/              — defines AST types, no dependencies on other compiler modules
2. lower/context.rs  — defines LoweringContext, depends on ast/ error types
3. lower/error.rs    — defines LoweringError, depends on ast/ span types
4. util/fnv.rs       — FNV-1a hash, pure computation, no dependencies
5. lower/optional.rs — depends on ast/, lower/context.rs
6. lower/fmt_string.rs — depends on ast/, lower/context.rs
7. lower/operator.rs — depends on ast/, lower/context.rs
8. lower/concurrency.rs — depends on ast/, lower/context.rs
9. lower/dialogue.rs — depends on ast/, lower/context.rs, lower/localization.rs
10. lower/localization.rs — depends on ast/, lower/context.rs, util/fnv.rs
11. lower/entity.rs  — depends on ast/, lower/context.rs
12. lower/mod.rs     — depends on all passes above; is the public entry point
13. lib.rs           — re-exports lower/mod.rs public API
```

This order maps directly to development phases: AST types first (they are the interface contract), then context/error infrastructure, then simple stateless passes (optional, fmt_string), then the complex stateful passes (dialogue, entity), then the orchestrator.

## Comparison: Relevant Patterns from Real Compilers

| Compiler | Pattern Used | Applicable Lesson for Writ |
|----------|--------------|---------------------------|
| rustc `rustc_ast_lowering` | `LoweringContext` struct threaded through fold functions; each desugaring is a method on `LoweringContext` | Use the same pattern: `LoweringContext` as state carrier, fold functions returning owned AST nodes |
| rustc HIR `intravisit` | Walk functions have exhaustive `match` so adding a field to a CST struct causes a compile error | Use exhaustive `match` in pass functions; adding a new `Item` variant will force all passes to handle it |
| Swift SILGen | Mandatory passes (correctness) run before optimization passes; they are distinct in code | Separate correctness-preserving lowering (desugar) from optimization (not applicable yet) |
| Kotlin K2 FIR | Separate desugaring stage before type checking; one unified IR after desugaring | Keep lowering completely separate from type checking; lowering produces AST, type checker consumes AST |
| TypeScript transformer | Composable visitor passes using `visitNode` / `visitEachChild`; each transform is a separate file | Use separate files per transform; orchestrator composes them — same structure, fold instead of visitor |

## Scalability Considerations

This is a scripting language compiler, not a production Rust-scale compiler. Scalability concerns are ordered accordingly:

| Concern | Current Scale (game scripts) | If language grows significantly |
|---------|------------------------------|--------------------------------|
| Compile time | Not a concern; scripts are small | Add parallelism per top-level item (Rayon); pass structure already supports it |
| New desugarings | Add one file, one match arm | No structural change required; pipeline is designed for extension |
| TBD spec features (tuples, destructuring) | Defer until spec is resolved | Add new pass files; existing passes unchanged |
| Error quality | Span preservation guarantees this | No change needed; spans flow through already |

## Sources

- [Rust Compiler Development Guide — Overview](https://rustc-dev-guide.rust-lang.org/overview.html) — MEDIUM confidence (official, current)
- [Rust Compiler Development Guide — HIR](https://rustc-dev-guide.rust-lang.org/hir.html) — MEDIUM confidence (official, current)
- [rustc_ast_lowering crate docs — LoweringContext](https://doc.rust-lang.org/stable/nightly-rustc/rustc_ast_lowering/struct.LoweringContext.html) — HIGH confidence (official rustc docs)
- [Swift SIL documentation](https://github.com/swiftlang/swift/blob/main/docs/SIL/SIL.md) — HIGH confidence (official Swift source)
- [Swift Compiler Architecture](https://www.swift.org/documentation/swift-compiler/) — HIGH confidence (official Swift docs)
- [Kotlin K2 Compiler Migration Guide](https://kotlinlang.org/docs/k2-compiler-migration-guide.html) — HIGH confidence (official JetBrains docs)
- [Crash Course on the Kotlin Compiler — K1/K2](https://medium.com/google-developer-experts/crash-course-on-the-kotlin-compiler-k1-k2-frontends-backends-fe2238790bd8) — MEDIUM confidence (Google Developer Expert, technical depth)
- [TypeScript Deep Dive — Binder](https://basarat.gitbook.io/typescript/overview/binder) — MEDIUM confidence (community reference, widely cited)
- [Post-Modern Compiler Design Vol 1 — Lowering](https://www.cs.purdue.edu/homes/rompf/pmca/vol1/lowering.html) — MEDIUM confidence (Purdue academic, conceptual)
- [SWC visitor pattern discussion](https://github.com/swc-project/swc/discussions/3044) — LOW confidence (community discussion, illustrative)
- [Rust Design Patterns — Visitor](https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html) — HIGH confidence (official Rust community patterns book)
- [MIR RFC 1211](https://rust-lang.github.io/rfcs/1211-mir.html) — HIGH confidence (official Rust RFC)

---
*Architecture research for: Writ compiler CST-to-AST lowering pipeline*
*Researched: 2026-02-26*
