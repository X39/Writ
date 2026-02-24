# Stack Research

**Domain:** CST-to-AST lowering pipeline for a statically-typed game scripting language compiler (Rust)
**Researched:** 2026-02-26
**Confidence:** HIGH (core choices), MEDIUM (supporting libraries)

---

## Context

This is a **subsequent milestone** adding a lowering pipeline to an existing project. The constraints are hard:

- Existing chumsky `0.12` + logos `0.16` parser in `writ-parser` produces a full-fidelity CST with `Spanned<T> = (T, SimpleSpan)` throughout.
- The `writ-compiler` crate is a stub. The lowering pipeline is its first real code.
- The AST must own its data (no `'src` lifetime from the borrowed source string — the parser uses `&'src str`, which is only valid while the source buffer lives).
- Downstream of lowering: type checker (not in scope for this milestone). The AST is the hand-off artifact.
- All lowering rules are spec-defined in Section 28 of the language spec.

The primary question is: which Rust crates and patterns support building a **multi-pass, span-preserving CST-to-AST lowering pipeline** in 2025?

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Rust | 2024 edition | Implementation language | Already established; workspace uses it; 2024 edition is current stable. |
| `writ-parser` (internal) | workspace | CST source types | The lowering pass consumes `writ_parser::cst` directly — no intermediate format. |

### AST Storage Strategy: Owned Enums, No Arena

The CST uses `&'src str` slices tied to the source buffer lifetime. The AST must **not** carry this lifetime — it is the output artifact, stored and passed between phases. The standard Rust answer for compiler IRs at this scale (pre-type-checker, single-crate compilation) is **owned enums with `String`/`Box`** for string data, not arenas.

**Why not arenas (`typed-arena`, `la-arena`) at this stage?**

- `typed-arena` and `la-arena` shine when nodes need to reference each other via stable pointers or indices across a large typed graph (e.g., the HIR's `DefId` system in rustc). The Writ AST at lowering time is a tree, not a graph — items do not need to reference sibling items by index.
- Arenas add complexity (arena lifetimes or `Id<T>` indirection) without payoff for a tree-shaped IR.
- The decision to add arenas belongs to the type-checker phase, where cyclic type references make arenas worthwhile. That is explicitly out of scope.

**Recommended pattern:** Plain owned `enum`/`struct` AST nodes. `String` for identifiers (converted from `&'src str`). `Box<Ast*>` for recursive children. `Vec<AstItem>` for sequences. This is zero-dependency, directly serializable, and the natural Rust idiom.

**Confidence:** HIGH — this is what every Rust compiler at pre-typechecking scale uses (see: rustc's own AST before it reaches the arena-backed HIR, swc's AST, cranelift's CLIF IR at construction time).

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thiserror` | `2.0.18` | Lowering error types | Define `LowerError` as a proper `std::error::Error` implementor with structured variants (`UnknownSpeaker`, `InvalidTransition`, etc.) and source spans. Required from day one. |
| `ariadne` | `0.6.0` | Diagnostic rendering | Render lowering errors to the terminal. Already a dev-dependency in `writ-parser` (same ecosystem as chumsky — sister project). Use for CLI output in `writ-cli`. |
| `const-fnv1a-hash` | `1.1.0` | Compile-time FNV-1a for localization keys | The spec (Section 28.4) requires pre-computed FNV-1a keys for `say_localized()`. This `no_std` crate computes FNV1A-32/64 at compile time on stable Rust. Use `fnv1a_hash_str_32` for dialogue text → 8-hex-char key generation. |
| `insta` | `1.46.3` | Snapshot testing of AST output | Test that CST inputs lower to the expected AST. Snapshot testing is the standard approach for compiler IR testing — hand-writing expected AST structs is impractical. Already used in `writ-parser` dev-dependencies. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo-insta` | Interactive snapshot review | Run `cargo insta review` after adding new lowering tests to approve golden AST outputs. |
| `rustfmt` | Formatting | Standard; no config needed beyond existing workspace settings. |
| `clippy` | Lint | Especially useful for catching `match` arms that miss CST variants during exhaustive lowering. |

---

## Installation

```toml
# writ-compiler/Cargo.toml

[dependencies]
writ-parser = { path = "../writ-parser" }
thiserror = "2.0"
const-fnv1a-hash = "1.1"

[dev-dependencies]
ariadne = "0.6"
insta = { version = "1", features = ["ron"] }
```

```toml
# writ-cli/Cargo.toml (for diagnostic rendering at the CLI layer)
[dependencies]
ariadne = "0.6"
```

Note: `ariadne` belongs in `writ-cli` for rendering, not in `writ-compiler`. The compiler crate should return structured `LowerError` values; the CLI converts them to `ariadne` reports. This keeps `writ-compiler` CLI-agnostic.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Owned `String` in AST nodes | `&'src str` slices (carry CST lifetime into AST) | Never for the AST. The AST must outlive the source buffer. Carrying `'src` into AST types forces every consumer to also carry that lifetime, cascading through the type checker and beyond. |
| Owned `String` in AST nodes | `Arc<str>` or `Rc<str>` | Only if the AST needs to be shared across threads or cloned cheaply at high volume. At this stage neither applies. |
| Plain owned enum tree | `typed-arena` / `la-arena` arenas | When the IR becomes a graph (e.g., type nodes referencing each other). Defer to the type-checker phase. |
| Plain owned enum tree | `id-arena` (index-based) | Same as above — premature for a tree-shaped IR. |
| `thiserror` for error types | `miette` for error types | `miette 7.x` combines error definition + diagnostic rendering in one crate. Good choice if you want a single-crate solution. The recommended split (`thiserror` for types, `ariadne` for rendering) keeps `writ-compiler` free of terminal/ANSI dependencies, which matters for library use. Use `miette` if you decide `writ-compiler` should own the rendering concern. |
| `const-fnv1a-hash` | Runtime `fnv` crate | The spec says keys are "pre-computed." Computing them at compile time is correct and matches the spec. `const-fnv1a-hash` is `no_std` stable. Use runtime `fnv` only if keys need to be computed from user-provided strings at runtime (not the case here). |
| `ariadne` | `codespan-reporting` | Both are solid. `ariadne` is the chumsky sister project, already in the dev-deps of `writ-parser`, and has more advanced overlap heuristics. Prefer `ariadne` to minimize ecosystem divergence. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `proc-macro`-based visitor generation (e.g., `swc_visit`) | The CST is ~40 node types — manageable without a macro-generated visitor framework. Generated visitors obscure control flow, complicate debugging, and add a heavy proc-macro compilation step. The Fold pattern implemented manually is cleaner at this scale. | Manual `lower_*` functions per CST node variant. |
| `salsa` (incremental query framework) | Salsa is the right tool for demand-driven, incremental compilation (used in rust-analyzer). It requires restructuring the entire compiler around a query system, which is premature. This milestone is a batch-mode pipeline. | Sequential `lower()` call chain. Add salsa later if incremental compilation becomes a requirement. |
| `LLVM` / `inkwell` | Code generation is explicitly out of scope for this milestone. | Reserved for a future codegen phase. |
| `logos` directly in `writ-compiler` | The lexer lives in `writ-parser`. The compiler consumes CST types, not tokens. | Depend on `writ-parser` types only. |
| Cloning HIR/AST nodes across passes | Cloning is a sign that nodes are being referenced in multiple places — an indicator the data model should use indices instead. At the tree level, prefer consuming transforms (`fn lower_item(item: cst::Item) -> Result<ast::Item, LowerError>`). | Owned, consuming fold functions. |

---

## Stack Patterns by Variant

**If a lowering pass needs to carry context (speaker state, localization table):**
- Use a `struct LowerCtx` that passes mutable state through recursive calls.
- Do not use global state or thread-locals.
- Example: dialogue speaker tracking needs to know the "current default speaker" across consecutive `@speaker` / `TextLine` pairs.

**If the AST needs to reference source spans for error messages:**
- Carry `chumsky::span::SimpleSpan` (re-export from `writ-parser`) directly in each AST node.
- `SimpleSpan` is `Copy` and tiny (two `usize` fields). There is no cost to storing it in every node.
- Pattern: `pub struct AstFnDecl { pub span: SimpleSpan, pub name: String, ... }`

**If a pass produces multiple errors (not just the first):**
- Return `(Option<AstNode>, Vec<LowerError>)` rather than `Result<AstNode, LowerError>`.
- Collect errors and continue lowering. Propagate `None` for fatally-errored nodes so downstream passes can detect and skip them (same pattern as `Expr::Error` in the CST).

**If localization key generation is needed:**
- Use `const-fnv1a-hash::fnv1a_hash_str_32(text.as_bytes(), None)` and format as 8 lowercase hex chars.
- For manual `#key` overrides, pass the key through verbatim.

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `chumsky 0.12` | `ariadne 0.6` | Sister projects, developed together. `ariadne 0.4+` works with chumsky 0.12 spans. Confirmed via existing `writ-parser` dev-dep on `ariadne = "0.6"`. |
| `thiserror 2.0` | Rust 2024 edition | thiserror 2.x dropped MSRV to 1.61; no conflicts with Rust 2024. |
| `insta 1.x` | `ron` feature | `writ-parser` already uses `insta = { version = "1", features = ["ron"] }`. Use the same config for consistency. |
| `const-fnv1a-hash 1.1` | `no_std`, stable Rust | No conflicts. Works in any Rust 2021/2024 crate. |

---

## Key Design Decisions (Not Crate Choices)

These are architecture decisions the stack research surfaced, not library choices, but they belong here because they constrain what goes in `Cargo.toml`.

### 1. Fold Pattern, Not Visitor

The lowering transform takes `cst::Item` and produces `ast::Item`. This is a **consuming fold** — the old tree is consumed, the new tree is produced. It is not a visitor (which observes without producing a new structure). Rustc calls this pattern a "folder" and uses it for AST → HIR lowering.

Each pass is a function or method: `fn lower_dlg(dlg: cst::DlgDecl, ctx: &mut LowerCtx) -> Result<ast::FnDecl, Vec<LowerError>>`. No trait-object dispatch, no macro-generated boilerplate.

**Why:** The Writ CST has ~40 node types. Manual fold functions are readable, compile fast, and are easy to extend. When a new construct is added, `rustc`'s `match` exhaustiveness checking will flag every lowering function that doesn't handle it.

### 2. Span Preservation via Direct Copy

`SimpleSpan` is `Copy`. Every AST node stores its origin span directly. No indirection, no span table, no offset math.

**Why:** The spec requires "lowering errors must reference original source spans." Storing spans in every node is the simplest implementation. The cost is ~16 bytes per node (two `usize`s), negligible for the node counts a game script compiler handles.

### 3. Multi-Pass Architecture via Sequential Function Calls

Passes are ordered functions called in sequence, not a pluggable pass manager. The pipeline is: `lower_items → lower_dlg_blocks → lower_entities → lower_operators → lower_optionals → lower_interpolated_strings → generate_loc_keys`.

**Why:** Salsa/query-based pipelines are the correct long-term architecture for incremental compilation, but they require restructuring the entire compiler. This milestone is batch-mode. Sequential calls are simpler, testable, and fast to implement. Each pass is independently unit-testable with `insta` snapshots.

### 4. Owned `String` Over `&'src str` in AST

All identifier strings are converted from `&'src str` to owned `String` during lowering. The source buffer is not accessible to the compiler crate.

**Why:** The alternative — threading `'src` through the AST — cascades the source-buffer lifetime into every type that touches the AST, including the type checker, symbol table, and error messages. The cost of this design mistake is measured in hours of fighting the borrow checker. Owned strings eliminate the problem. A `String` allocation per identifier is not a performance concern for a game scripting compiler.

---

## Sources

- [rustc-dev-guide: The HIR](https://rustc-dev-guide.rust-lang.org/hir.html) — ID-based out-of-band storage, span preservation patterns — HIGH confidence
- [rustc-dev-guide: Overview of the Compiler](https://rustc-dev-guide.rust-lang.org/overview.html) — multi-pass IR pipeline rationale — HIGH confidence
- [rustc_ast_lowering docs](https://doc.rust-lang.org/stable/nightly-rustc/rustc_ast_lowering/index.html) — fold pattern, unique IDs, span usage — HIGH confidence
- [ariadne docs (docs.rs)](https://docs.rs/ariadne/latest/ariadne/) — version 0.6.0 confirmed — HIGH confidence
- [thiserror docs (docs.rs)](https://docs.rs/thiserror/latest/thiserror/) — version 2.0.18 confirmed — HIGH confidence
- [const-fnv1a-hash docs (docs.rs)](https://docs.rs/const-fnv1a-hash/latest/const_fnv1a_hash/) — version 1.1.0, `fnv1a_hash_str_32` confirmed — HIGH confidence
- [la-arena docs (docs.rs)](https://docs.rs/la-arena/latest/la_arena/) — version 0.3.1, index-based arena from rust-analyzer — HIGH confidence
- [typed-arena docs (docs.rs)](https://docs.rs/typed-arena/latest/typed_arena/) — version 2.0.2 — HIGH confidence
- [index_vec docs (docs.rs)](https://docs.rs/index_vec/latest/index_vec/) — version 0.1.4, typed newtype-index Vec — HIGH confidence
- [insta docs (docs.rs)](https://docs.rs/insta/latest/insta/) — version 1.46.3 confirmed — HIGH confidence
- [miette docs (docs.rs)](https://docs.rs/miette/latest/miette/) — version 7.6.0 — HIGH confidence
- [chumsky SimpleSpan docs](https://docs.rs/chumsky/0.12.0/chumsky/span/struct.SimpleSpan.html) — Copy span, downstream usage patterns — HIGH confidence
- [chumsky Parser trait docs](https://docs.rs/chumsky/latest/chumsky/trait.Parser.html) — `Spanned<T>` pattern, `SimpleSpan<usize>` — HIGH confidence
- [Thunderseethe: Lowering AST to IR](https://thunderseethe.dev/posts/lowering-base-ir/) — consuming fold pattern, panic vs. Result for post-typecheck lowering — MEDIUM confidence (independent blog, consistent with rustc patterns)
- [Rust Design Patterns: Fold](https://rust-unofficial.github.io/patterns/patterns/creational/fold.html) — fold vs. visitor distinction — MEDIUM confidence (community resource, well-maintained)
- [Rust Design Patterns: Visitor](https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html) — walk_* functions, stateful visitors — MEDIUM confidence
- [Arenas in Rust — LogRocket](https://blog.logrocket.com/guide-using-arenas-rust/) — arena tradeoffs summary — MEDIUM confidence
- [Rust Performance Book: Hashing](https://nnethercote.github.io/perf-book/hashing.html) — FNV vs fxhash vs ahash tradeoffs — MEDIUM confidence
- [miette: Compiler Crates page](https://sdiehl.github.io/compiler-crates/miette.html) — miette vs. ariadne positioning — MEDIUM confidence (third-party survey)
- [WebSearch: multi-pass lowering pipeline patterns] — aggregate from multiple sources above — see individual citations

---

*Stack research for: Writ compiler CST-to-AST lowering pipeline*
*Researched: 2026-02-26*
