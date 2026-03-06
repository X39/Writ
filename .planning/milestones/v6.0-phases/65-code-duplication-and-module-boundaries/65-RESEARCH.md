# Phase 65: Code Duplication and Module Boundaries - Research

**Researched:** 2026-03-18
**Domain:** Rust crate structure — visibility, wildcard imports, code duplication
**Confidence:** HIGH

## Summary

Phase 65 is the final cleanup phase of v6.0. All file-splitting (Phases 63-64) is complete, leaving five pending requirements: DUP-01, DUP-02, MOD-01, MOD-02, MOD-03. The workspace compiles clean with zero clippy warnings (`cargo clippy --workspace` exits 0).

The primary DUP finding is `lower_dlg_text` in `dialogue.rs` vs `lower_fmt_string` in `fmt_string.rs` — two near-identical functions that build left-associative `BinaryOp::Add` chains from different segment types (`DlgTextSegment` vs `StringSegment`). These types are structurally identical (Text/Expr variants), but defined in two CST modules. Consolidation into a shared generic helper is **viable** and recommended.

Wildcard imports fall into two tiers: (a) internal module-IR wildcards (`use super::ir::*`) that import large enum sets from same-crate internal modules — these should be made explicit; (b) legitimate external prelude wildcards (`use lsp_types::*`, `use dap::prelude::*`, `use chumsky::prelude::*`) that follow idioms established by those libraries and carry high rewrite cost for minimal clarity gain — these should be **exempted**. The policy distinction between external-prelude wildcards and internal module wildcards must be documented.

Public API surface is substantially wider than necessary. Most items in the `emit/` and `check/` submodules are `pub` but only accessed from within `writ-compiler` itself (tests in the same crate do not require `pub`, only `pub(crate)` — but since the test suite in `writ-compiler/tests/` is an integration test accessing the crate as external, `pub` IS needed for test-accessed items). The module structure doc comments are absent from most `lib.rs` files.

**Primary recommendation:** Consolidate `lower_dlg_text`/`lower_fmt_string` via a shared generic helper in `lower/fmt_string.rs`. Replace internal `use super::ir::*`/`use crate::ast::decl::*` wildcards with explicit imports. Narrow `pub` to `pub(crate)` where items are not accessed by external crates or integration tests. Add module-level doc comments to all crate `lib.rs` files.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DUP-01 | Duplicate code patterns identified and consolidated across all crates | One confirmed high-value target: `lower_dlg_text`/`lower_fmt_string`. No other significant duplication found. |
| DUP-02 | Known duplication in `lower_dlg_text`/`lower_fmt_string` reviewed and consolidated | Both functions are ~50 lines each with identical logic structure — generic helper approach is feasible. |
| MOD-01 | Module structure reviewed across all 9 crates for clarity | All 8 library crates plus writ-cli assessed. Most `lib.rs` files lack doc comment structure headers. |
| MOD-02 | Wildcard imports reviewed and replaced with explicit imports where appropriate | 57 wildcard import sites found. External library preludes are legitimate exemptions; internal module wildcards should be explicit. |
| MOD-03 | Public API surface reviewed — unnecessary `pub` narrowed | 846 `pub` declarations vs 28 `pub(crate)`. Integration tests in `writ-compiler/tests/` drive genuine `pub` needs; items only used within a crate can be narrowed. |
</phase_requirements>

## Standard Stack

This phase involves no new library dependencies — it is pure Rust refactoring. The relevant Rust visibility system:

### Core Visibility Rules (HIGH confidence)
| Visibility | Meaning | Use When |
|------------|---------|----------|
| `pub` | Visible to any crate | Item exported in lib.rs re-exports OR accessed by integration tests |
| `pub(crate)` | Visible within same crate only | Item used across modules within one crate but not by external crates |
| `pub(super)` | Visible to parent module | Item used by sibling module files (submodule pattern) |
| private | Module-only | Item used only within one file |

**Key insight for this codebase:** `writ-compiler/tests/` is an integration test directory — tests there access `writ-compiler` as an external crate, so any item accessed from there MUST remain `pub`. Items only used in `writ-compiler/src/` can be `pub(crate)`.

### Installation
No new dependencies. All work is refactoring existing Rust code.

## Architecture Patterns

### Recommended Project Structure (post-phase)
```
writ-compiler/src/
├── lib.rs            # //! Module structure doc header + pub re-exports
├── lower/
│   ├── fmt_string.rs # Shared helper for both fmt strings AND dlg text segments
│   └── dialogue.rs   # Calls shared helper, dlg-specific logic stays here
└── (all other modules unchanged)
```

### Pattern 1: Generic Segment Lowering
**What:** Replace `lower_dlg_text` with a call to a generalized `lower_segment_chain` that handles any `(Text, Expr)` segment type via a closure or trait.
**When to use:** Both `DlgTextSegment` and `StringSegment` have identical structure — `Text(&str)` and `Expr(inner_expr)` — so a generic function parameterized on the segment type works cleanly.

**Example — shared helper approach:**
```rust
// Source: writ-compiler/src/lower/fmt_string.rs

/// Lowers any segment list into a left-associative Add chain.
/// `extract_text` maps a Text segment to its string content.
/// `extract_expr` maps an Expr segment to its inner CST expression.
pub(crate) fn lower_segment_chain<S, FT, FE>(
    segments: Vec<Spanned<S>>,
    outer_span: SimpleSpan,
    ctx: &mut LoweringContext,
    extract_text: FT,
    extract_expr: FE,
) -> AstExpr
where
    FT: Fn(&S) -> Option<&str>,
    FE: Fn(S) -> Box<Spanned<writ_parser::cst::Expr<'_>>>,
{ ... }
```

Alternative: keep `lower_fmt_string` as-is, and have `lower_dlg_text` call it after mapping `DlgTextSegment` to `StringSegment`. The mapping is trivial since both have identical variants.

**Preferred approach:** Document the identity and call `lower_fmt_string` from `lower_dlg_text` after converting `DlgTextSegment` → `StringSegment`. This eliminates the body duplication while keeping the call site in `dialogue.rs`.

**Concrete conversion:**
```rust
// In dialogue.rs — lower_dlg_text delegates to lower_fmt_string
fn lower_dlg_text(
    segments: Vec<Spanned<DlgTextSegment<'_>>>,
    outer_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstExpr {
    // DlgTextSegment and StringSegment are structurally identical.
    // Convert and delegate to the shared implementation.
    let string_segments = segments.into_iter().map(|(seg, span)| {
        let converted = match seg {
            DlgTextSegment::Text(s) => StringSegment::Text(s),
            DlgTextSegment::Expr(e) => StringSegment::Expr(e),
        };
        (converted, span)
    }).collect();
    lower_fmt_string(string_segments, outer_span, ctx)
}
```
This requires verifying that `DlgTextSegment::Expr` and `StringSegment::Expr` carry the same inner type (`Box<Spanned<Expr<'_>>>`). Given that both are in `writ_parser::cst`, this is HIGH confidence true.

### Pattern 2: Explicit Import Lists
**What:** Replace `use super::ir::*` with an explicit list of the types actually used in each file.
**When to use:** All non-test internal wildcards.

**Example — check_stmt.rs before/after:**
```rust
// BEFORE
use super::ir::*;

// AFTER — explicit list derived from the TypedStmt/TypedExpr variants actually used
use super::ir::{
    TypedStmt, TypedExpr, TypedLiteral, TypedAst,
    // ... only what check_stmt.rs actually references
};
```

**How to find used items:** Run `cargo check` after removing the wildcard — the compiler will list every undefined name. This is the safest procedure.

### Pattern 3: lib.rs Module Doc Headers
**What:** Each crate's `lib.rs` gets a `//!` doc header explaining the module hierarchy.
**When to use:** All 8 lib crates (writ-compiler, writ-runtime, writ-parser, writ-module, writ-assembler, writ-diagnostics, writ-lsp, writ-dap). writ-cli uses `main.rs` which already has a doc comment header.

**Example — writ-compiler/src/lib.rs:**
```rust
//! Writ compiler: source-to-IL compilation pipeline.
//!
//! ## Module structure
//!
//! - `ast`     — Simplified AST produced by lowering (CST → AST)
//! - `lower`   — CST lowering: desugars and normalises the CST into AST
//! - `resolve` — Name resolution: builds DefMap, resolves all names to DefIds
//! - `check`   — Type checking: produces TypedAst from resolved AST
//! - `emit`    — IL emission: TypedAst → binary .writc module bytes
//! - `config`  — writ.toml parsing and project configuration

pub mod ast;
// ...
```

### Anti-Patterns to Avoid
- **Splitting `use super::ir::*` without running `cargo check`:** The IR modules export many types — always use the compiler's undefined-name output to get the exact list.
- **Narrowing `pub` without checking integration tests:** `writ-compiler/tests/*.rs` files access the crate externally. Any item they use must stay `pub`.
- **Converting `use lsp_types::*` to explicit imports:** The tower-lsp ecosystem uses `lsp_types::*` as a near-universal pattern; `backend.rs` uses ~40+ lsp types. The conversion cost exceeds the clarity gain. Exempt this.
- **Converting `use chumsky::prelude::*`:** Chumsky parsers are written assuming prelude import; `parser/program.rs` is 3,014 lines in one scope (documented exception) — don't touch chumsky wildcards.
- **Converting `use dap::prelude::*`:** The `dap` crate (debug adapter protocol) is designed with a prelude pattern identical to chumsky. Keep.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Finding which names a wildcard imports | Custom grep | `cargo check` after removing wildcard — compiler lists every undefined name |
| Finding items used only within a crate | Custom analysis | `cargo check` after narrowing `pub` to `pub(crate)` — compiler reports external access violations |
| Segment type conversion | Generic trait abstraction | Simple `match` conversion from DlgTextSegment to StringSegment in `lower_dlg_text` |

## Common Pitfalls

### Pitfall 1: Integration Tests Require `pub`
**What goes wrong:** Narrowing `pub` to `pub(crate)` on items that integration tests in `writ-compiler/tests/` access.
**Why it happens:** `tests/emit_body_tests.rs`, `tests/emit_serialize_tests.rs`, etc. import from `writ_compiler::emit::body::*`, `writ_compiler::check::ir::*`, etc. These are external accesses — they need `pub`.
**How to avoid:** For each `pub` item being considered for narrowing, grep `writ-compiler/tests/` for its name. If found, keep `pub`.
**Warning signs:** `cargo check` produces "method/struct/fn is private" errors in test files.

### Pitfall 2: `use super::ir::*` Imports Many Types
**What goes wrong:** Making explicit the wildcard `use super::ir::*` in `check_decl.rs`, `check_expr/mod.rs`, `check_stmt.rs`, `desugar.rs` requires listing every `TypedDecl`, `TypedExpr`, `TypedStmt`, `TypedLiteral`, `TypedPattern`, `TypedArm`, `Capture`, `CaptureMode`, `TypedAst` variant used in each file.
**Why it happens:** The IR modules export 9 large enums/structs — the explicit lists will be long.
**How to avoid:** Remove the wildcard, run `cargo check`, copy the "not found" names into the explicit import list.
**Warning signs:** Very long explicit import lines. Consider using `ir::{self, ...}` or grouping items.

### Pitfall 3: `lower_dlg_text` Segment Conversion Lifetime
**What goes wrong:** `DlgTextSegment<'src>` and `StringSegment<'src>` both carry `'src` lifetimes from the parser input. The conversion must preserve the lifetime.
**Why it happens:** CST types are lifetime-parameterized by the source `&'src str` input.
**How to avoid:** The `lower_dlg_text` function already takes ownership (`Vec<Spanned<DlgTextSegment<'_>>>`), so the conversion `match seg { DlgTextSegment::Text(s) => StringSegment::Text(s), ... }` preserves the lifetime naturally.
**Warning signs:** Lifetime error on the converted `Vec<Spanned<StringSegment<'_>>>`.

### Pitfall 4: `use crate::ast::decl::*` — Wide but Legitimate
**What goes wrong:** `check_decl.rs`, `resolver.rs`, `env_build.rs` import `crate::ast::decl::*` which exports 32 public items (AstFnDecl, AstStructDecl, AstEnumDecl, ...). Making this explicit creates a 32-item import list.
**Why it happens:** The decl module IS a large set of AST node types, all of which are used throughout the checker and resolver.
**How to avoid:** Evaluate whether explicitness here adds clarity or just noise. If most files use 20+ of the 32 items, a wildcard may be the right call with a comment explaining the intentionality.
**Warning signs:** Import list approaching 25+ items — at that point document the wildcard instead of replacing it.

### Pitfall 5: writ-module `use crate::tables::*` Is Internal Prelude
**What goes wrong:** `builder.rs`, `module.rs`, `reader.rs`, `writer.rs` all `use crate::tables::*` — the tables module exports 23 row structs that are all used in every file.
**Why it happens:** The module format tables module is a "domain vocabulary" for the entire crate. Every file works with the same 23 row types.
**How to avoid:** Document this as an intentional internal prelude pattern rather than converting to 23-item explicit imports.

## Code Examples

### DlgTextSegment vs StringSegment types
```rust
// Source: writ-parser/src/cst.rs

pub enum StringSegment<'src> {
    Text(&'src str),
    Expr(Box<Spanned<Expr<'src>>>),
}

pub enum DlgTextSegment<'src> {
    Text(&'src str),
    Expr(Box<Spanned<Expr<'src>>>),
}
```
These are structurally identical. The conversion between them is a trivial match with no data transformation.

### The lower_fmt_string function signature
```rust
// Source: writ-compiler/src/lower/fmt_string.rs
pub fn lower_fmt_string(
    segments: Vec<Spanned<StringSegment<'_>>>,
    outer_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstExpr
```

### Wildcards by site — categorized
**External library preludes (EXEMPT — keep as wildcards):**
```
writ-dap/src/main.rs:            use dap::prelude::*;
writ-dap/src/server/handlers.rs: use dap::prelude::*;
writ-dap/src/server/helpers.rs:  use dap::prelude::*;
writ-dap/src/server/inspection.rs: use dap::prelude::*;
writ-dap/src/server/mod.rs:      use dap::prelude::*;
writ-lsp/src/backend.rs:         use lsp_types::*;
writ-parser/src/parser/...rs:    use chumsky::prelude::*;  (4 files)
writ-parser/src/parser/program.rs: use chumsky::pratt::*;
```

**Internal module wildcards (ACTION: make explicit):**
```
writ-assembler/src/assembler.rs: use crate::ast::*;
writ-assembler/src/parser.rs:    use crate::ast::*;
writ-compiler/src/check/check_decl.rs:    use crate::ast::decl::*;
writ-compiler/src/check/check_decl.rs:    use super::ir::*;
writ-compiler/src/check/check_expr/mod.rs: use crate::ast::expr::*;
writ-compiler/src/check/check_expr/mod.rs: use super::ir::*;
writ-compiler/src/check/check_stmt.rs:    use super::ir::*;
writ-compiler/src/check/desugar.rs:       use super::ir::*;
writ-compiler/src/check/env_build.rs:     use crate::ast::decl::*;
writ-compiler/src/emit/module_builder.rs: use super::metadata::*;
writ-compiler/src/emit/serialize.rs:      use writ_module::tables::*;
writ-compiler/src/resolve/resolver.rs:    use crate::ast::decl::*;
writ-compiler/src/resolve/resolver.rs:    use crate::resolve::ir::*;
writ-module/src/builder.rs:    use crate::tables::*;
writ-module/src/module.rs:     use crate::tables::*;
writ-module/src/reader.rs:     use crate::tables::*;
writ-module/src/writer.rs:     use crate::tables::*;
writ-parser/src/lib.rs:        pub use cst::*;
```

**Test-file wildcards (EXEMPT — standard test pattern):**
```
use super::*;  (in #[cfg(test)] blocks — 20+ files)
writ-assembler/tests/parse_tests.rs: use writ_assembler::ast::*;
writ-compiler/tests/resolve_tests.rs: use writ_compiler::resolve::prelude::*;
writ-dap/tests/test_initialize_sequence.rs: use dap::prelude::*;
writ-module/tests/round_trip.rs: use writ_module::tables::*;
writ-parser/tests/parser_tests.rs: use writ_parser::cst::*;
```

**Domain-vocabulary internal wildcards (DOCUMENT INTENT, keep as wildcards):**
```
writ-module tables pattern: use crate::tables::*  in 4 files
  → 23 row struct types, all used in all files. Document as internal prelude.
writ-compiler/emit/module_builder.rs: use super::metadata::*
  → 21 table row structs, all used. Document as internal prelude.
writ-compiler/emit/serialize.rs: use writ_module::tables::*
  → 23 row struct types. Document.
```

### Wildcard actionability tiers
```
TIER 1 — Replace with explicit imports (small export set, narrow usage):
  writ-compiler/check/check_stmt.rs:  use super::ir::*   (9 items, ~5 used)
  writ-compiler/check/desugar.rs:     use super::ir::*   (9 items, ~6 used)
  writ-compiler/resolve/resolver.rs:  use crate::resolve::ir::*  (4 items)

TIER 2 — Replace or document (large export set, most items used):
  writ-compiler/check/check_decl.rs:  use super::ir::*   (9 items, ~8 used)
  writ-compiler/check/check_expr/mod.rs: use super::ir::*  (9 items, most used)
  writ-compiler/check/check_decl.rs:  use crate::ast::decl::*  (32 items)
  writ-compiler/check/env_build.rs:   use crate::ast::decl::*  (32 items)
  writ-compiler/resolve/resolver.rs:  use crate::ast::decl::*  (32 items)
  writ-compiler/check/check_expr/mod.rs: use crate::ast::expr::*  (10 items)

TIER 3 — Document as intentional internal prelude (keep wildcard):
  writ-module/src/{builder,module,reader,writer}.rs: use crate::tables::*
  writ-compiler/emit/module_builder.rs: use super::metadata::*
  writ-compiler/emit/serialize.rs: use writ_module::tables::*
  writ-assembler/{assembler,parser}.rs: use crate::ast::*   (AST vocabulary)
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| All files in single monolithic files | Files split by sub-concern | Phases 63-64 | Completed |
| No clippy warnings | Clean clippy baseline | Phase 62 | Completed |
| Wildcard imports throughout | Mixed (phase 65 target) | This phase | Clarity |

## Open Questions

1. **DlgTextSegment → StringSegment conversion feasibility**
   - What we know: Both types have identical variant structure (`Text(&'src str)`, `Expr(Box<Spanned<Expr<'src>>>)`)
   - What's unclear: Whether `DlgTextSegment::Expr` inner type matches `StringSegment::Expr` inner type exactly (both should be `Box<Spanned<Expr<'src>>>` based on cst.rs reading)
   - Recommendation: Verify in cst.rs before writing the conversion. If they differ, use the generic-closure approach instead.

2. **Which `pub` items in `writ-compiler/emit/` are truly needed externally**
   - What we know: `writ-compiler/tests/` is an integration test crate that imports many emit:: items as `pub`
   - What's unclear: Which specific items in `emit/body/*.rs` are never accessed by tests or external crates
   - Recommendation: The planner should prescribe running `cargo check` after targeted narrowing rather than attempting to enumerate all candidates analytically.

3. **writ-assembler `use crate::ast::*` scope**
   - What we know: `ast.rs` exports ~15 AST node structs for the assembler's own IR
   - What's unclear: Whether assembler tests import from `writ_assembler::ast::*` externally (parse_tests.rs does)
   - Recommendation: Keep assembler ast::* explicit because it's only 15 items and is an internal IR not designed as a public API.

## Validation Architecture

> `workflow.nyquist_validation` is absent from `.planning/config.json` — treating as enabled.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) |
| Config file | Cargo.toml (workspace) |
| Quick run command | `cargo test --workspace --no-fail-fast 2>&1 \| tail -20` |
| Full suite command | `cargo test --workspace 2>&1 \| tail -30` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DUP-01 | No duplicate logic for string-building chains | unit | `cargo test -p writ-compiler --test lowering_tests 2>&1 \| tail -5` | Yes |
| DUP-02 | `lower_dlg_text` delegates to `lower_fmt_string` | unit | `cargo test -p writ-compiler --test lowering_tests 2>&1 \| tail -5` | Yes |
| MOD-01 | All lib.rs have doc headers | manual review | `cargo doc --workspace --no-deps 2>&1 \| grep "warning"` | Yes |
| MOD-02 | No internal wildcard imports (except documented) | static | `cargo check --workspace 2>&1 \| tail -5` (no errors = correct) | Yes |
| MOD-03 | `pub` narrowed where possible | static | `cargo check --workspace 2>&1 \| tail -5` | Yes |

### Sampling Rate
- **Per task commit:** `cargo check --workspace`
- **Per wave merge:** `cargo test --workspace --no-fail-fast`
- **Phase gate:** Full test suite green before verification

### Wave 0 Gaps
None — existing test infrastructure covers all phase requirements. The lowering tests are comprehensive; pub narrowing is validated by `cargo check`.

## Sources

### Primary (HIGH confidence)
- Direct source code inspection of all 10 workspace crates (`writ-compiler/src/lower/dialogue.rs`, `writ-compiler/src/lower/fmt_string.rs`, all `lib.rs` files, all wildcard import sites)
- `cargo check --workspace` — confirmed clean compile
- `cargo clippy --workspace` — confirmed zero warnings

### Secondary (MEDIUM confidence)
- Rust Reference on visibility modifiers: `pub(crate)` vs `pub(super)` vs `pub`
- Prior phase decisions in STATE.md (Phase 63 and 64 decisions on file splitting rationale)

### Tertiary (LOW confidence)
- None required — all findings are from direct source inspection

## Metadata

**Confidence breakdown:**
- DUP analysis: HIGH — both functions read, segment types confirmed in cst.rs
- Wildcard classification: HIGH — all 57 sites catalogued with tier assignment
- pub narrowing scope: MEDIUM — integration test usage requires `cargo check` verification per item
- Architecture patterns: HIGH — standard Rust idioms, no external dependencies

**Research date:** 2026-03-18
**Valid until:** 2026-04-18 (stable Rust idioms; code structure stable after phase 64)
