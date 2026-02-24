---
phase: 02-foundational-expression-lowering
plan: 01
subsystem: compiler
tags: [rust, lowering, ast, cst, optional, fmt-string, compound-assign, expression-fold]

requires:
  - phase: 01-ast-foundation
    provides: AstExpr, AstStmt, AstDecl, AstType, LoweringContext, LoweringError, lower() stub

provides:
  - lower_type() in optional.rs — folds all TypeExpr variants; T? → Generic { "Option", [T] }
  - lower_fmt_string() in fmt_string.rs — folds FormattableString segments to Binary Add chain with .into<string>() calls
  - lower_expr() in expr.rs — exhaustive fold over all CST Expr variants with R3/R4/R5 desugarings
  - lower_stmt() in stmt.rs — folds all CST Stmt variants to AstStmt
  - lower_fn() in mod.rs — lowers FnDecl to AstFnDecl with all sub-nodes recursively lowered
  - All structural pass-through lowering (Struct, Enum, Contract, Impl, Component, Extern, Const, Global, Namespace, Using)
  - Real Item dispatch in lower() replacing Phase 1 stub

affects:
  - 02-foundational-expression-lowering (plans 02+)
  - 03-dialogue-block-parser
  - 04-top-level-declarations
  - 05-error-recovery-and-integration

tech-stack:
  added: []
  patterns:
    - "Consuming fold functions (lower_expr, lower_type, lower_stmt) — each takes Spanned<CstNode> and returns owned AstNode"
    - "Span threading — every synthetic AST node (Binary Add chains, compound assign expansion) carries the originating CST expression's outer span"
    - "Cross-module circular import resolution — lower_stmt and lower_expr mutually reference each other via top-level use imports rather than inline use in match arms"
    - "Attrs flattening — Vec<Spanned<Vec<Attribute>>> flattened to Vec<AstAttribute> via for loops (not closures) to avoid &mut LoweringContext capture issues"

key-files:
  created:
    - writ-compiler/src/lower/optional.rs
    - writ-compiler/src/lower/fmt_string.rs
    - writ-compiler/src/lower/expr.rs
    - writ-compiler/src/lower/stmt.rs
  modified:
    - writ-compiler/src/lower/mod.rs

key-decisions:
  - "Inline use for lower_stmt inside expr.rs match arms replaced with top-level import — Rust 2024 edition warns on inline use in match arms when top-level import is available"
  - "lower_attrs uses explicit for loops not functional combinators — flat_map with move closure captures &mut LoweringContext incorrectly (FnMut vs FnOnce); for loop avoids the issue"
  - "lower_extern _e_span parameter unused — extern decl span tracked by inner variant spans (Fn/Struct/Component each carry their own span); outer ExternDecl has no span field in AstExternDecl"

patterns-established:
  - "Pattern 1: Exhaustive match — No _ => wildcards on Expr or TypeExpr matches; Rust's exhaustive match enforces completeness"
  - "Pattern 2: Compound assignment — lower_expr(lhs) once, clone AstExpr result for second use as target; avoids double-lowering or CST cloning"
  - "Pattern 3: Formattable string — segments.is_empty() → empty StringLit; non-empty → left-fold into Binary Add chain via Iterator::fold"

requirements-completed: [R3, R4, R5]

duration: 6min
completed: 2026-02-26
---

# Phase 02 Plan 01: Foundational Expression Lowering Summary

**CST-to-AST expression lowering: T? → Option<T>, $"..." → Binary Add chain, +=/-=/*=/... → Assign{Binary}, plus complete Item dispatch replacing Phase 1 stub**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-26T15:42:11Z
- **Completed:** 2026-02-26T15:47:37Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- `lower_type` in `optional.rs` handles all 6 `TypeExpr` variants exhaustively; `Nullable(T)` → `Generic { name: "Option", args: [lower_type(T)] }` (R3)
- `lower_fmt_string` in `fmt_string.rs` converts `FormattableString`/`FormattableRawString` segments to a left-associative `Binary { op: Add }` chain with `.into<string>()` `GenericCall` wrappers on interpolated expressions (R4)
- `lower_expr` in `expr.rs` exhaustively matches all 27+ CST `Expr` variants; `NullLit` → `Path["Option","None"]`, compound assignments → expanded `Assign { Binary { ... } }`, both fmtstring variants dispatched to `lower_fmt_string` (R3/R4/R5)
- `lower_stmt` in `stmt.rs` handles all 10 `Stmt` variants; `DlgDecl`/`Transition` use `todo!()` for Phase 4
- `lower()` in `mod.rs` replaces Phase 1 stub with real `Item` dispatch: `Fn` through `lower_fn`, all structural items through dedicated lowering, `Dlg`/`Entity` with `todo!()` for later phases
- `lower_fn` recursively lowers attrs (flattened), vis, generics, params, return type, and body
- All structural pass-throughs implemented: Namespace (Declarative and Block), Using, Struct, Enum, Contract, Impl, Component, Extern, Const, Global

## Task Commits

Each task was committed atomically:

1. **Task 1: Create lower_type (optional.rs) and lower_expr (expr.rs) with lower_fmt_string (fmt_string.rs)** - `56d0401` (feat)
2. **Task 2: Create lower_stmt (stmt.rs), lower_fn helpers, and wire lower/mod.rs** - `9bc575b` (feat)

## Files Created/Modified

- `writ-compiler/src/lower/optional.rs` — `lower_type()`: folds all TypeExpr variants; T? → Generic { "Option", [T] }
- `writ-compiler/src/lower/fmt_string.rs` — `lower_fmt_string()`: FormattableString/FormattableRawString → left-associative Binary Add chain
- `writ-compiler/src/lower/expr.rs` — `lower_expr()`: central recursive fold over all 27+ CST Expr variants; also houses `lower_pattern`, `lower_arg`, `lower_lambda_param`, `lower_match_arm` helpers
- `writ-compiler/src/lower/stmt.rs` — `lower_stmt()`: folds all 10 Stmt variants; DlgDecl/Transition todo!() for Phase 4
- `writ-compiler/src/lower/mod.rs` — Real Item dispatch replacing Phase 1 stub; all structural pass-through lowering; lower_fn, lower_fn_sig, lower_op_sig, lower_op_decl, lower_struct, lower_enum, lower_contract, lower_impl, lower_component, lower_extern, lower_const, lower_global, lower_namespace, lower_using

## Decisions Made

- **Inline use replaced with top-level import in expr.rs**: `expr.rs` and `stmt.rs` mutually reference each other. Inline `use crate::lower::stmt::lower_stmt;` inside match arms generated "unused import" warnings in Rust 2024 edition. Resolved by adding `lower_stmt` as a top-level import at the file level.
- **For-loop for lower_attrs**: The initial `flat_map` + `move` closure implementation for `lower_attrs` failed to compile because `ctx: &mut LoweringContext` cannot be moved into a `FnMut` closure that iterates multiple times. Rewritten as explicit `for` loop which correctly borrows `ctx` mutably per attribute.
- **lower_extern _e_span unused**: `ExternDecl` outer span intentionally discarded because `AstExternDecl` is an enum whose each variant (Fn, Struct, Component) carries its own span via the inner struct; there is no outer span field to populate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed Rust 2024 inline use warning causing compilation ambiguity**
- **Found during:** Task 1 (expr.rs implementation)
- **Issue:** `use crate::lower::stmt::lower_stmt;` inside match arms in `lower_expr` generated "unused import" warnings because Rust 2024 edition treats these as separate scopes; the imports were actually used but triggered the lint
- **Fix:** Moved `lower_stmt` import to top of `expr.rs` file and removed all inline `use` statements from match arm bodies
- **Files modified:** `writ-compiler/src/lower/expr.rs`
- **Verification:** `cargo build -p writ-compiler` zero warnings
- **Committed in:** 56d0401 (Task 1 commit)

**2. [Rule 3 - Blocking] Fixed &mut LoweringContext capture error in lower_attrs closure**
- **Found during:** Task 2 (mod.rs implementation)
- **Issue:** `lower_attrs` used `flat_map(|block| block.into_iter().map(move |attr| ...ctx...))` — `move` closure captured `ctx: &mut LoweringContext` by value, but `FnMut` requires multiple calls which would require the mutable ref to be used across closure boundaries (E0507)
- **Fix:** Rewrote `lower_attrs` as an explicit `for` loop with `result.push()`, avoiding the closure capture entirely
- **Files modified:** `writ-compiler/src/lower/mod.rs`
- **Verification:** `cargo build -p writ-compiler` zero errors, zero warnings
- **Committed in:** 9bc575b (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both auto-fixes were Rust borrow/lint issues with the closure implementation approach. No scope creep — plan was executed exactly as specified, just with for-loops instead of functional combinators in one place.

## Issues Encountered

None beyond the auto-fixed blocking issues above.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- All five foundational lowering functions are implemented and wired into the `lower()` pipeline
- `lower_expr`, `lower_type`, `lower_stmt`, and `lower_fn` are available for Phase 3+ structural passes (dialogue, entity) to call
- Phase 3 (dialogue block parser) and Phase 4 (top-level declarations) can immediately use these helpers
- Remaining `todo!()` markers: `Item::Dlg` (Phase 4), `Item::Entity` (Phase 5), `Stmt::DlgDecl` (Phase 4), `Stmt::Transition` (Phase 4)

---
*Phase: 02-foundational-expression-lowering*
*Completed: 2026-02-26*
