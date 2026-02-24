---
phase: 01-ast-foundation
plan: 01
subsystem: compiler
tags: [rust, ast, chumsky, SimpleSpan, owned-types, thiserror, insta]

# Dependency graph
requires: []
provides:
  - "AstExpr enum with all expression variants including concurrency pass-through (Spawn, Join, Cancel, Defer, Detached)"
  - "AstStmt enum with all statement variants including Error recovery sentinel"
  - "AstDecl enum with all declaration variants (no Dlg/Entity — lowered away)"
  - "AstType enum with no Nullable sugar variant"
  - "Ast container struct as flat Vec<AstDecl>"
  - "writ-compiler restructured as library + binary crate"
affects: [02-ast-foundation, all subsequent phases]

# Tech tracking
tech-stack:
  added:
    - "writ-parser (workspace path dep) — CST source types"
    - "chumsky 0.12.0 (direct dep) — SimpleSpan for span-per-node"
    - "thiserror 2.0 — LoweringError in Plan 02"
    - "insta 1 with ron feature (dev-dep) — snapshot testing in Phase 2+"
  patterns:
    - "Span-per-node invariant: every AstExpr/AstStmt/AstDecl/AstType variant carries span: SimpleSpan"
    - "Owned-data invariant: all AST nodes use String, Box<T>, Vec<T> — no 'src lifetime"
    - "Sugar-free AST: no Nullable, FormattableString, CompoundAssign, Dlg, or Entity variants"
    - "Concurrency pass-through: Spawn, Join, Cancel, Defer, Detached as first-class AstExpr variants"
    - "Error recovery sentinel: AstExpr::Error and AstStmt::Error variants with span"

key-files:
  created:
    - "writ-compiler/src/lib.rs — library crate root with pub mod ast"
    - "writ-compiler/src/ast/mod.rs — Ast container + module re-exports"
    - "writ-compiler/src/ast/expr.rs — AstExpr enum + supporting types (BinaryOp, PrefixOp, PostfixOp, RangeKind, AstArg, AstLambdaParam, AstMatchArm, AstPattern)"
    - "writ-compiler/src/ast/stmt.rs — AstStmt enum"
    - "writ-compiler/src/ast/decl.rs — AstDecl enum + all supporting structs (AstFnDecl, AstStructDecl, AstEnumDecl, AstContractDecl, AstImplDecl, AstComponentDecl, AstExternDecl, AstConstDecl, AstGlobalDecl, AstParam, AstGenericParam, AstOpSymbol, AstOpSig, AstOpDecl, AstFnSig, etc.)"
    - "writ-compiler/src/ast/types.rs — AstType enum"
  modified:
    - "writ-compiler/Cargo.toml — added writ-parser, chumsky, thiserror deps; insta dev-dep"

key-decisions:
  - "Added chumsky as direct dependency (not transitive via writ-parser) — Rust 2024 edition requires explicit direct deps; transitive imports resolve to 'unresolved crate' errors"
  - "Nullable lowers to Generic { name: 'Option', args: [T] } — no AstType::Nullable variant; enforced at type definition level"
  - "NullLit lowers to Path expression Option::None — no AstExpr::NullLit variant"
  - "Compound assignments (+=, -=, etc.) lower to a = a op b — no compound Assign variant in AstExpr"
  - "Dlg lowers to Fn before reaching AST — no AstDecl::Dlg variant"
  - "Entity lowers to Struct + Impl + lifecycle registrations — no AstDecl::Entity variant"
  - "AstDecl has no span field — each variant struct carries its own span (mirrors CST Item enum pattern)"

patterns-established:
  - "Span-per-node: every enum variant carries span: SimpleSpan as the last named field"
  - "No Default derive on AST types — prevents span tombstoning accidents"
  - "No SimpleSpan::new(0, 0) allowed — real span always required at construction"

requirements-completed: [R1, R14]

# Metrics
duration: 6min
completed: 2026-02-26
---

# Phase 01 Plan 01: AST Foundation Summary

**AstExpr, AstStmt, AstDecl, AstType enums with owned data, span-per-node, no CST sugar, and concurrency pass-through — writ-compiler restructured as library + binary**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-26T15:08:14Z
- **Completed:** 2026-02-26T15:14:44Z
- **Tasks:** 1
- **Files modified:** 8

## Accomplishments

- Restructured `writ-compiler` from binary-only stub to library + binary crate
- Defined complete AST type hierarchy: `AstExpr` (30 variants), `AstStmt` (9 variants), `AstDecl` (11 variants + Stmt), `AstType` (5 variants)
- All concurrency primitives present as first-class `AstExpr` variants: Spawn, Detached, Join, Cancel, Defer
- All error recovery sentinels present: `AstExpr::Error`, `AstStmt::Error`
- All CST sugar lowered away at type-definition level: no Nullable, FormattableString, compound Assign, Dlg, or Entity
- All types use owned data (String, Box<T>, Vec<T>) — zero `'src` lifetime parameters on any `Ast*` type
- Every AST variant carries `span: SimpleSpan` — span-per-node invariant enforced structurally

## Task Commits

1. **Task 1: Restructure crate and define AST type enums** - `77a118f` (feat)

## Files Created/Modified

- `writ-compiler/Cargo.toml` — Added writ-parser, chumsky 0.12, thiserror 2.0 deps; insta dev-dep
- `writ-compiler/src/lib.rs` — Library crate root: `pub mod ast`
- `writ-compiler/src/ast/mod.rs` — `Ast` container struct + submodule re-exports
- `writ-compiler/src/ast/types.rs` — `AstType` enum (Named, Generic, Array, Func, Void — no Nullable)
- `writ-compiler/src/ast/expr.rs` — `AstExpr` enum + BinaryOp, PrefixOp, PostfixOp, RangeKind, AstArg, AstLambdaParam, AstMatchArm, AstPattern
- `writ-compiler/src/ast/stmt.rs` — `AstStmt` enum (no DlgDecl, no Transition)
- `writ-compiler/src/ast/decl.rs` — `AstDecl` enum + AstFnDecl, AstStructDecl, AstEnumDecl, AstContractDecl, AstImplDecl, AstComponentDecl, AstExternDecl, AstConstDecl, AstGlobalDecl, AstParam, AstGenericParam, AstOpSymbol, AstOpSig, AstOpDecl, AstFnSig, AstVisibility, AstAttribute, AstNamespaceDecl, AstUsingDecl, AstStructField, AstEnumVariant, AstImplMember, AstContractMember, AstComponentMember

## Decisions Made

- **chumsky as direct dependency:** Rust 2024 edition does not allow importing from transitive dependencies. The research noted either approach (direct dep or re-export from writ-parser) works. Direct dep was chosen for explicitness and to avoid relying on transitive re-export stability.
- **AstDecl has no top-level span field:** Each variant (AstFnDecl, AstStructDecl, etc.) carries its own `span: SimpleSpan`. This mirrors the CST `Item` enum pattern and avoids span redundancy.
- **Comprehensive AstDecl defined upfront:** All variants that the pipeline will eventually produce are defined now. This prevents breaking changes in Phase 2+ from adding new variants to exhaustive matches.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added chumsky as direct dependency to writ-compiler**
- **Found during:** Task 1 (first cargo build attempt)
- **Issue:** All four AST files used `use chumsky::span::SimpleSpan;` which resolved to `E0433: failed to resolve: use of unresolved module or unlinked crate 'chumsky'`. Rust 2024 edition requires explicit direct dependencies; transitive access is not permitted.
- **Fix:** Added `chumsky = { version = "0.12.0", features = ["pratt"] }` to `writ-compiler/Cargo.toml` dependencies. Version matches `writ-parser`'s chumsky dependency exactly.
- **Files modified:** `writ-compiler/Cargo.toml`
- **Verification:** `cargo build -p writ-compiler` succeeded after adding the dep
- **Committed in:** `77a118f` (part of task commit)

---

**Total deviations:** 1 auto-fixed (1 blocking dependency)
**Impact on plan:** Required for compilation. Research had flagged this as an open question with two valid approaches; direct dep was the right choice. No scope creep.

## Issues Encountered

None beyond the chumsky transitive dependency issue (documented above as deviation).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- All AST types are defined and public — Plan 02 (LoweringError, LoweringContext, `lower()` stub) can proceed immediately
- `writ-compiler` compiles as both library and binary — `lib.rs` exports `pub mod ast`
- No blocking concerns for Phase 2+

---
*Phase: 01-ast-foundation*
*Completed: 2026-02-26*
