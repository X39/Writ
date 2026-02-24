# Phase 1: AST Foundation - Research

**Researched:** 2026-02-26
**Domain:** Rust compiler AST type definitions + pipeline infrastructure (crate setup, LoweringContext, LoweringError)
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| R1 | AST Type Hierarchy — `AstExpr`, `AstStmt`, `AstDecl`, `AstType` enums; owned types only; no `'src` lifetime; no CST sugar variants; concurrency primitives as first-class AST nodes; `Expr::Error` / `Stmt::Error` variants | Owned enum pattern confirmed by rustc, swc, cranelift prior art; CST node inventory in `writ-parser/src/cst.rs` provides the complete shape to translate; span-per-node convention from chumsky `SimpleSpan` docs |
| R2 | Pipeline Infrastructure — `LoweringContext` with `errors`, `speaker_stack`, `loc_key_counter`; `lower(items: Vec<Spanned<Item<'_>>>) -> (Ast, Vec<LoweringError>)` stub; `LoweringError` with `SimpleSpan` + message; `thiserror` | `LoweringContext` pattern from rustc `rustc_ast_lowering`; `thiserror 2.0.18` confirmed on docs.rs; `Ast` as newtype `Vec<AstItem>` is standard |
| R14 | Span Preservation — every AST node carries `span: SimpleSpan`; no `SimpleSpan::new(0, 0)` tombstones; synthetic nodes carry span pointing to CST origin | `SimpleSpan` is `Copy` (two `usize` fields); chumsky docs confirmed; span-per-node is the established rustc HIR pattern |
</phase_requirements>

---

## Summary

Phase 1 establishes the two structural prerequisites for the entire lowering pipeline: (1) the AST type hierarchy that all later passes emit into, and (2) the pipeline infrastructure skeleton (`LoweringContext`, `LoweringError`, and the stub `lower()` entry point). Nothing is lowered in this phase. The goal is that every subsequent phase has a target type and a context to write into from day one.

The work is almost entirely type definition — no algorithmic logic. The most important decision, already locked in project state, is that the AST uses **owned types** (`String`, `Box<T>`, `Vec<T>`) with no `'src` lifetime from the CST. This is the single highest-leverage architectural decision: carrying `'src` into the AST would cascade through every downstream phase. The second locked decision is **span-per-node** as a hard invariant: every `AstExpr`, `AstStmt`, `AstDecl`, and `AstType` carries `span: SimpleSpan`, enforced at construction time.

The codebase starting point is minimal: `writ-compiler/src/main.rs` is a hello-world stub, and `writ-compiler/Cargo.toml` has no dependencies. The CST is fully defined in `writ-parser/src/cst.rs` and is the definitive inventory of what the AST must cover (after lowering away sugar). The crate needs to be restructured from a binary to a library-plus-binary, dependencies added, and the `ast/` and `lower/` module skeletons created.

**Primary recommendation:** Define AST types in `ast/` first (they have no dependencies), then add `lower/error.rs` (`LoweringError` via `thiserror`), then `lower/context.rs` (`LoweringContext`), then wire the stub `lower()` in `lower/mod.rs` and expose it from `lib.rs`. Each step compiles independently.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust 2024 edition | (workspace) | Implementation language | Already established in workspace `Cargo.toml` |
| `writ-parser` (internal) | workspace path | CST source types (`Item`, `Expr`, `Stmt`, `TypeExpr`, `Spanned<T>`, `SimpleSpan`) | Direct dep — lowering consumes `writ_parser::cst` directly |
| `thiserror` | `2.0` | Structured `LoweringError` type — implements `std::error::Error` with `#[error("...")]` macros | Standard Rust error type library; v2 confirmed; no terminal/rendering dependency |

### Supporting (Phase 1 — dev-deps only)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `insta` | `1` with `ron` feature | Snapshot testing for future passes | Add as dev-dep now (matches `writ-parser` pattern) so phase 2+ tests work without a Cargo.toml change |

### Not Needed in Phase 1

| Library | Reason to Defer |
|---------|----------------|
| `const-fnv1a-hash` | Only needed in Phase 4 (localization key generation inside dialogue lowering) |
| `ariadne` | Diagnostic rendering belongs in `writ-cli`, not `writ-compiler`; not needed until CLI wires up |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `thiserror` | `miette` | `miette` bundles error definition + rendering; prefer `thiserror` to keep `writ-compiler` CLI-agnostic |
| `String` in AST nodes | `&'src str` | Never for AST — carries source buffer lifetime into every downstream phase; confirmed anti-pattern |
| `String` in AST nodes | `Arc<str>` | Only warranted if AST is shared across threads or cloned at high volume; neither applies here |
| Owned enum tree | `typed-arena` / `la-arena` | Arenas pay off when IR is a graph (cyclic type references); the lowering-stage AST is a tree; defer to type-checker phase |

**Installation:**

```toml
# writ-compiler/Cargo.toml

[dependencies]
writ-parser = { path = "../writ-parser" }
thiserror = "2.0"
const-fnv1a-hash = "1.1"   # add now; used in Phase 4

[dev-dependencies]
insta = { version = "1", features = ["ron"] }
```

> Note: `const-fnv1a-hash` can be added in Phase 4 when first used, but adding it in Phase 1 is low-cost and prevents a Cargo.toml change mid-pipeline build. The research summary recommends it for Phase 1 crate setup.

---

## Architecture Patterns

### Recommended Project Structure

The `writ-compiler` crate is currently a binary. It needs to become a library-plus-binary.

```
writ-compiler/src/
├── lib.rs                  # pub use lower::lower; re-exports public API
├── main.rs                 # CLI entry point (thin stub — not part of this phase's goal)
├── ast/
│   ├── mod.rs              # pub mod expr; pub mod stmt; pub mod decl; pub mod types; + pub use
│   ├── expr.rs             # AstExpr enum
│   ├── stmt.rs             # AstStmt enum
│   ├── decl.rs             # AstDecl enum (AstFnDecl, AstStructDecl, AstImplDecl, AstEnumDecl, ...)
│   └── types.rs            # AstType enum (no Nullable sugar — Option<T> directly)
└── lower/
    ├── mod.rs              # pub fn lower(...) -> (Ast, Vec<LoweringError>) stub
    ├── context.rs          # LoweringContext struct
    └── error.rs            # LoweringError type via thiserror
```

Later phases add files to `lower/` (`optional.rs`, `fmt_string.rs`, `dialogue.rs`, etc.) without changing the above.

### Pattern 1: Span-Per-Node as a Hard Invariant

**What:** Every AST node struct/enum carries a `span: SimpleSpan` field. For enums, it goes either in a wrapper struct or in every variant. The `SimpleSpan` is `Copy`, so it costs nothing to thread through.

**When to use:** Always. No exceptions. This is what prevents span tombstoning (Pitfall 1 from project research).

**Example:**

```rust
// ast/expr.rs
use chumsky::span::SimpleSpan;

#[derive(Debug, Clone, PartialEq)]
pub enum AstExpr {
    IntLit    { value: i64,    span: SimpleSpan },
    FloatLit  { value: f64,    span: SimpleSpan },
    StringLit { value: String, span: SimpleSpan },
    BoolLit   { value: bool,   span: SimpleSpan },
    Ident     { name: String,  span: SimpleSpan },
    // ... binary ops, calls, control flow, etc.
    Spawn     { expr: Box<AstExpr>, span: SimpleSpan },  // concurrency pass-through
    Join      { expr: Box<AstExpr>, span: SimpleSpan },
    Cancel    { expr: Box<AstExpr>, span: SimpleSpan },
    Defer     { expr: Box<AstExpr>, span: SimpleSpan },
    Detached  { expr: Box<AstExpr>, span: SimpleSpan },
    Error     { span: SimpleSpan },  // error recovery sentinel
}
```

### Pattern 2: LoweringContext as State Carrier

**What:** A single `LoweringContext` struct owns all mutable cross-cutting state. Passes receive `&mut LoweringContext`. All state is visible, testable, and thread-safe by construction.

**When to use:** Always — this is the `rustc_ast_lowering` pattern.

**Example:**

```rust
// lower/context.rs
use chumsky::span::SimpleSpan;
use crate::lower::error::LoweringError;

/// A speaker scope entry for active-speaker tracking in dialogue lowering.
pub struct SpeakerScope {
    pub name: String,
    pub span: SimpleSpan,
}

/// Shared mutable state threaded through every lowering pass.
pub struct LoweringContext {
    /// Accumulated errors — all passes append here; pipeline never halts.
    pub errors: Vec<LoweringError>,
    /// Stack of currently-active speakers (push on dlg entry / branch entry, pop on exit).
    pub speaker_stack: Vec<SpeakerScope>,
    /// Counter for auto-generating deterministic localization keys.
    pub loc_key_counter: u32,
}

impl LoweringContext {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            speaker_stack: Vec::new(),
            loc_key_counter: 0,
        }
    }

    pub fn emit_error(&mut self, err: LoweringError) {
        self.errors.push(err);
    }

    pub fn take_errors(self) -> Vec<LoweringError> {
        self.errors
    }

    pub fn push_speaker(&mut self, scope: SpeakerScope) {
        self.speaker_stack.push(scope);
    }

    pub fn pop_speaker(&mut self) {
        self.speaker_stack.pop();
    }

    pub fn current_speaker(&self) -> Option<&SpeakerScope> {
        self.speaker_stack.last()
    }

    pub fn next_loc_key(&mut self) -> u32 {
        let k = self.loc_key_counter;
        self.loc_key_counter += 1;
        k
    }
}
```

### Pattern 3: LoweringError via thiserror

**What:** `LoweringError` is a proper `std::error::Error` implementor with structured variants. Each variant carries a `SimpleSpan` for source location. `thiserror` handles the `Display` and `Error` impls via derive macros.

**Example:**

```rust
// lower/error.rs
use chumsky::span::SimpleSpan;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoweringError {
    #[error("unknown speaker `{name}` at {span:?}")]
    UnknownSpeaker { name: String, span: SimpleSpan },

    #[error("dialogue transition `->` must be the last statement in its block (at {span:?})")]
    NonTerminalTransition { span: SimpleSpan },

    #[error("duplicate localization key `{key}` (first at {first_span:?}, again at {second_span:?})")]
    DuplicateLocKey {
        key: String,
        first_span: SimpleSpan,
        second_span: SimpleSpan,
    },

    #[error("conflicting component method `{method}` (from {first_component} and {second_component}) at {span:?}")]
    ConflictingComponentMethod {
        method: String,
        first_component: String,
        second_component: String,
        span: SimpleSpan,
    },
}
```

### Pattern 4: Public API as Stub

**What:** `lower/mod.rs` exports the public `lower()` function which in Phase 1 is a stub returning an empty `Ast`. Later phases replace the stub body.

**Example:**

```rust
// lower/mod.rs
use writ_parser::cst::{Item, Spanned};
use crate::ast::Ast;
use crate::lower::{context::LoweringContext, error::LoweringError};

/// Lowers a CST item list to a simplified AST.
///
/// # Pass Ordering
///
/// Passes execute in this order (rationale: each pass's output is required
/// by subsequent passes):
/// 1. `lower_fn`      — Fn items; calls lower_type (optional), lower_expr (fmt_string, operator, concurrency)
/// 2. `lower_dialogue`— Dlg items; calls lower_expr sub-passes, then lower_localization as sub-pass
/// 3. `lower_entity`  — Entity items; calls lower_type for component fields
///
/// Expression-level passes (optional, fmt_string, operator, concurrency) are NOT top-level
/// passes — they are invoked from inside structural passes as helpers.
pub fn lower(items: Vec<Spanned<Item<'_>>>) -> (Ast, Vec<LoweringError>) {
    let mut ctx = LoweringContext::new();
    // Phase 1 stub: returns empty AST.
    // Phases 2-6 will replace this body with actual pass dispatch.
    let _ = items;
    let _ = &mut ctx;
    (Ast::empty(), ctx.take_errors())
}
```

### Pattern 5: Ast as a Newtype Container

**What:** `Ast` is a thin container over `Vec<AstItem>`. It provides a stable public type boundary between the lowering pipeline and downstream consumers.

**Example:**

```rust
// ast/mod.rs
use crate::ast::decl::AstDecl;

/// The lowered AST: a flat list of top-level declarations.
/// No CST sugar variants. No `'src` lifetime.
#[derive(Debug, Clone, PartialEq)]
pub struct Ast {
    pub items: Vec<AstDecl>,
}

impl Ast {
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }
}
```

### Anti-Patterns to Avoid

- **Lifetime on AST nodes:** `pub enum AstExpr<'src>` — do not do this. Convert `&'src str` to `String` at lowering time.
- **Missing span field on any AST node:** Every variant must carry `span: SimpleSpan`. If a variant looks like it doesn't need one (e.g., `Error`), add it anyway — future diagnostic passes will need it.
- **Using `SimpleSpan::new(0, 0)` as a placeholder:** This is a tombstone. It causes all downstream diagnostics to point at byte 0. If a real span is not available during construction, that is a design error — find the right span.
- **One giant AST enum:** Do not define a single `AstNode` enum that combines expressions, statements, and declarations. Keep them separate (`AstExpr`, `AstStmt`, `AstDecl`, `AstType`) for type safety.
- **CST sugar in AST types:** No `Nullable(Box<AstType>)`, no `FormattableString`, no `CompoundAssign` variants. These are lowered away before reaching the AST. If they appear in the AST, the pipeline's purpose is defeated.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Error type with `Display` + `Error` impl | Manual `impl Display` + `impl Error` boilerplate | `thiserror` | One `#[derive(Error)]` replaces ~30 lines; variants with `#[error("...")]` are self-documenting |
| FNV-1a hash for localization keys | Custom hash function | `const-fnv1a-hash` | `no_std`, stable Rust, compile-time computation; hand-rolled FNV is error-prone for the 2a vs 1a distinction |
| Span type | Custom `(usize, usize)` span | `chumsky::span::SimpleSpan` | Already in `writ-parser`'s public API; `Copy`, zero-cost; downstream tools (ariadne) expect it |
| Snapshot testing of AST output | Manual `assert_eq!` of every field | `insta` | Writing expected AST structs by hand for lowering tests is impractical; snapshot review via `cargo insta review` is the standard pattern |

**Key insight:** The entire Phase 1 stack is about choosing the right thin wrappers — `thiserror` for errors, `SimpleSpan` for spans, `insta` for tests. There is no novel algorithmic work. The risk is architectural (lifetime decisions, span discipline), not technical.

---

## Common Pitfalls

### Pitfall 1: Carrying `'src` Lifetime Into the AST

**What goes wrong:** A field like `name: &'src str` in an AST node compiles fine at first. But it forces every type that holds the AST to also carry `'src`. This cascades to `LoweringContext`, to `Ast`, to the type checker struct, to every test. Removing it later costs hours of borrow checker fights.

**Why it happens:** It's the path of least resistance — `&'src str` requires no allocation, while `String` does. The cost is deferred.

**How to avoid:** Every `&'src str` in a CST node becomes a `String` in the corresponding AST node. The conversion happens in the lowering pass at the `to_string()` call site. This is the only acceptable pattern.

**Warning signs:** Any `<'src>` or `<'_>` lifetime parameter appearing on an `Ast*` type.

### Pitfall 2: Span Tombstoning (Highest-Risk Phase 1 Error)

**What goes wrong:** A developer writes `AstExpr::IntLit { value: 42, span: SimpleSpan::new(0, 0) }` to get the type to compile. This placeholder is never replaced. All downstream errors on this node point at source position 0.

**Why it happens:** The stub `lower()` in Phase 1 doesn't construct any real AST nodes — so the temptation is to not worry about span discipline until lowering starts. But the discipline must be enforced in the *type definitions*, not at call sites.

**How to avoid:** Make `SimpleSpan::new(0, 0)` impossible to use accidentally by naming the pattern explicitly in code review. The `span` field in every AST node constructor is a required argument — it cannot be defaulted. Do not add `Default` derives to AST node types.

**Warning signs:** Any `SimpleSpan::new(0, 0)` in `lower/` or `ast/` source files.

### Pitfall 3: Defining AST with CST Sugar Variants

**What goes wrong:** A developer mirrors the CST variants directly into the AST for convenience: `AstType::Nullable(Box<AstType>)`. This defeats the purpose of lowering. When the type checker sees `AstType::Nullable`, it has to handle sugar — but the contract says the AST is sugar-free.

**Why it happens:** Starting from the CST as a template produces "AST" types that are just renamed CST types.

**How to avoid:** Define the AST from spec primitives, not from CST variants. The spec defines what the AST is the *target* of; the CST is the *source*. For types: `Option<T>` in the AST is `AstType::Generic { name: "Option", args: [T], span }`, never `AstType::Nullable(T)`.

**Warning signs:** AST type definitions that contain `Nullable`, `FormattableString`, `CompoundAssign`, `Dlg`, or `Entity` variants.

### Pitfall 4: Forgetting Concurrency Primitives

**What goes wrong:** Phase 1 defines `AstExpr` but omits `Spawn`, `Join`, `Cancel`, `Defer`, `Detached` variants because "those are handled in Phase 3." Phase 3 then has to modify `AstExpr` — breaking any code that had exhaustive matches on it.

**Why it happens:** The phase scope says "AST Foundation" and concurrency pass-through is Phase 3's concern. But the *types* must exist in Phase 1 — Phase 3 only writes the lowering logic.

**How to avoid:** All AST variants must be defined in Phase 1, even if no lowering pass populates them yet. R1 explicitly lists concurrency primitives as required in the AST type hierarchy.

**Warning signs:** `AstExpr` without `Spawn`, `Join`, `Cancel`, `Defer`, `Detached` variants.

### Pitfall 5: writ-compiler Staying a Binary

**What goes wrong:** The crate only has `main.rs`. Adding `lib.rs` exposes the public API that tests and downstream crates consume. If Phase 2 adds tests before `lib.rs` exists, Rust will not find the `writ_compiler` module path.

**Why it happens:** The starter crate is a binary stub. Converting to library-plus-binary requires creating `lib.rs` and updating `Cargo.toml` if needed (though the workspace edition handles most of this).

**How to avoid:** Create `src/lib.rs` as the first task in Phase 1. It can be minimal (just `pub mod ast; pub mod lower;`) but it must exist before any other work happens.

---

## Code Examples

Verified patterns from official sources and project research:

### SimpleSpan Usage (from chumsky docs)

```rust
// SimpleSpan is defined in chumsky::span
// Already used throughout writ-parser as: pub type Spanned<T> = (T, SimpleSpan);
// It is Copy: two usize fields (start, end) representing byte offsets.

use chumsky::span::SimpleSpan;

let span: SimpleSpan = SimpleSpan::new(10, 25); // bytes 10..25 in source
// span.start() -> 10
// span.end()   -> 25
```

Source: [chumsky 0.12 SimpleSpan docs](https://docs.rs/chumsky/0.12.0/chumsky/span/struct.SimpleSpan.html)

### thiserror 2.0 Error Definition

```rust
// Source: https://docs.rs/thiserror/2.0.18/thiserror/
use thiserror::Error;
use chumsky::span::SimpleSpan;

#[derive(Debug, Error)]
pub enum LoweringError {
    #[error("unknown speaker `{name}`")]
    UnknownSpeaker {
        name: String,
        span: SimpleSpan,
    },

    #[error("lowering failed: {message}")]
    Generic {
        message: String,
        span: SimpleSpan,
    },
}
```

### Consuming Spanned Items from writ-parser

```rust
// writ_parser::cst::Spanned<T> = (T, SimpleSpan)
// The pattern for consuming in a lowering pass:

use writ_parser::cst::{Item, Spanned};

fn lower_item(ctx: &mut LoweringContext, spanned: Spanned<Item<'_>>) -> Option<AstDecl> {
    let (item, span) = spanned;  // always destructure to preserve span
    match item {
        Item::Fn(fn_spanned) => {
            let (fn_decl, fn_span) = fn_spanned;
            // Use fn_span for the AstFnDecl's span field
            todo!("Phase 2+")
        }
        Item::Dlg(_) => todo!("Phase 4"),
        Item::Entity(_) => todo!("Phase 5"),
        // Passthrough items (Struct, Enum, Contract, Impl, etc.)
        _ => todo!("Phase 2+"),
    }
}
```

### Crate Structure: lib.rs + main.rs

```rust
// src/lib.rs — library crate root
pub mod ast;
pub mod lower;

pub use lower::lower;
pub use lower::error::LoweringError;
pub use ast::Ast;
```

```rust
// src/main.rs — thin binary stub (unchanged for now, can be extended later)
fn main() {
    println!("writ-compiler");
}
```

### AST Type: AstType Without Nullable Sugar

```rust
// ast/types.rs — correct: no Nullable variant
use chumsky::span::SimpleSpan;

#[derive(Debug, Clone, PartialEq)]
pub enum AstType {
    /// Named type: `int`, `string`, `Guard`
    Named { name: String, span: SimpleSpan },

    /// Generic type: `Option<T>`, `List<T>`, `Result<A, B>`
    /// NOTE: T? lowers to Generic { name: "Option", args: [T], ... }
    /// There is no Nullable variant — that is CST sugar.
    Generic { name: String, args: Vec<AstType>, span: SimpleSpan },

    /// Array type: `T[]`
    Array { elem: Box<AstType>, span: SimpleSpan },

    /// Function type: `fn(int, string) -> bool`
    Func { params: Vec<AstType>, ret: Option<Box<AstType>>, span: SimpleSpan },

    /// Void
    Void { span: SimpleSpan },
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Visitor pattern for CST→AST transform | Fold pattern (consuming function per node variant) | Standard in rustc 2018+ | Type safety: return type enforces output is AST, not mutated CST |
| Global mutable lowering state | `LoweringContext` struct threaded through passes | rustc pattern, stable | Testable in isolation; no hidden state; thread-safe |
| Arenas for all IR | Owned enums for tree-shaped IR (arenas only for graph-shaped IR) | Separated in practice by ~2020 | Simpler type signatures; no arena lifetime parameters at lowering stage |
| `'src` lifetime on AST | Owned `String` in AST | Established Rust compiler pattern | Eliminates lifetime cascade into type checker and all consumers |

**Deprecated/outdated:**
- `syn`/`quote` proc-macro visitor generation for lowering: overkill at ~40 CST node types; adds compile time with no benefit. Do not use.
- `salsa` query system: correct for incremental compilation, wrong for batch-mode pipeline at this scale.

---

## Open Questions

1. **Does `writ-compiler` need a `[[bin]]` section in `Cargo.toml` when `lib.rs` is added?**
   - What we know: Rust allows both `lib.rs` and `main.rs` in the same crate without explicit Cargo.toml configuration when the workspace edition is 2024. The defaults are `src/lib.rs` (library) and `src/main.rs` (binary named after the package).
   - What's unclear: Whether the existing `writ-compiler/Cargo.toml` (which has `edition = "2024"` but no `[lib]` or `[[bin]]`) requires any change.
   - Recommendation: Add `lib.rs`, attempt `cargo build -p writ-compiler`, and check for errors. If Cargo complains, add explicit `[lib]` and `[[bin]]` sections. This is a 1-minute verification task.

2. **What is the `SimpleSpan` import path in `writ-compiler`?**
   - What we know: `writ-parser` uses `chumsky::span::SimpleSpan` directly. The `writ-parser` crate's `cst.rs` uses `use chumsky::span::SimpleSpan;`. The lowering pipeline in `writ-compiler` depends on `writ-parser`.
   - What's unclear: Should `writ-compiler` import `SimpleSpan` via `chumsky` (adding a direct `chumsky` dependency) or re-export it from `writ-parser`?
   - Recommendation: Re-export `SimpleSpan` from `writ-parser::cst` or use it via `writ_parser::cst::SimpleSpan` (since `cst.rs` imports and uses it). Alternatively, add `chumsky = "0.12"` as a direct dependency of `writ-compiler` for clarity. Either works; the explicit direct dep avoids relying on transitive re-export.

3. **Should `AstDecl` include all CST `Item` variants (Struct, Enum, Contract, Impl, etc.) or only the ones with active lowering (Fn, Dlg, Entity)?**
   - What we know: Phase 1's R1 requirement defines `AstExpr`, `AstStmt`, `AstDecl`, `AstType`. The pipeline handles `Fn`, `Dlg`, and `Entity` as the primary lowering targets. Other items (`Struct`, `Enum`, `Contract`, `Impl`, `Const`, `Global`, `Namespace`, `Using`) are likely structural pass-throughs.
   - What's unclear: How much of the CST pass-through structure needs to be defined in Phase 1 vs. Phase 2+. Defining a comprehensive `AstDecl` now prevents adding variants later (which would require updating all existing exhaustive matches).
   - Recommendation: Define `AstDecl` comprehensively in Phase 1 with all variants that the pipeline will eventually produce, even if most are stubs. Add `todo!()` / `unreachable!()` in the stub `lower()`. This avoids breaking changes in Phase 2+.

---

## Sources

### Primary (HIGH confidence)

- `D:/dev/git/Writ/writ-parser/src/cst.rs` — definitive CST node inventory; direct reference for what the AST must cover
- `D:/dev/git/Writ/.planning/research/SUMMARY.md` — project research synthesis with stack, architecture, and pitfall findings
- `D:/dev/git/Writ/.planning/research/STACK.md` — detailed stack research with version numbers and rationale
- `D:/dev/git/Writ/.planning/research/ARCHITECTURE.md` — LoweringContext pattern, pass ordering, data flow
- `D:/dev/git/Writ/.planning/research/PITFALLS.md` — all 8 pitfalls with prevention strategies and recovery costs
- `D:/dev/git/Writ/.planning/REQUIREMENTS.md` — R1, R2, R14 acceptance criteria
- [thiserror 2.0.18 docs.rs](https://docs.rs/thiserror/2.0.18/thiserror/) — `#[derive(Error)]` macro API confirmed
- [chumsky 0.12 SimpleSpan docs](https://docs.rs/chumsky/0.12.0/chumsky/span/struct.SimpleSpan.html) — `Copy` span, `SimpleSpan::new(start, end)` API confirmed
- [rustc_ast_lowering LoweringContext](https://doc.rust-lang.org/stable/nightly-rustc/rustc_ast_lowering/struct.LoweringContext.html) — direct reference implementation for the context pattern

### Secondary (MEDIUM confidence)

- [rustc-dev-guide: HIR](https://rustc-dev-guide.rust-lang.org/hir.html) — span preservation per-node convention, owned types in HIR
- [Rust Design Patterns: Fold](https://rust-unofficial.github.io/patterns/patterns/creational/fold.html) — fold vs. visitor distinction
- [Thunderseethe: Lowering AST to IR](https://thunderseethe.dev/posts/lowering-base-ir/) — consuming fold pattern, span preservation in practice

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — `thiserror`, `SimpleSpan`, owned enums are all confirmed against official docs; `insta` version matched to existing `writ-parser` dev-dep
- Architecture: HIGH — `LoweringContext` pattern directly from `rustc_ast_lowering`; module structure from project ARCHITECTURE.md research; CST inventory from `cst.rs`
- Pitfalls: HIGH — span tombstoning and lifetime cascade are verified Rust compiler design patterns; concurrency variant omission and CST sugar in AST are derived directly from R1 requirements and project research

**Research date:** 2026-02-26
**Valid until:** 2026-09-01 (stable Rust ecosystem; thiserror and chumsky APIs are stable; no time-sensitive concerns)
