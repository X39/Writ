# Phase 2: Foundational Expression Lowering - Research

**Researched:** 2026-02-26
**Domain:** Rust compiler expression desugaring (optional sugar, formattable strings, compound assignments) + insta snapshot testing
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| R3 | Optional Sugar Lowering — `T?` → `Option<T>` in all type positions; `null` → `Option::None` | CST has `TypeExpr::Nullable(Box<Spanned<TypeExpr>>)` and `Expr::NullLit`; AST already defines `AstType::Generic { name: "Option", ... }` and `AstExpr::Path` as the targets. Pure structural rewrite — no context state needed. |
| R4 | Formattable String Lowering — `$"Hello {name}!"` → concat chain with `.into<string>()`; `{{`/`}}` → literal braces | CST has `Expr::FormattableString(Vec<Spanned<StringSegment>>)` where `StringSegment` is `Text(&str)` or `Expr(Box<Spanned<Expr>>)`. AST target is nested `AstExpr::Binary { op: BinaryOp::Add, ... }` with `AstExpr::GenericCall` for `.into<string>()`. |
| R5 | Compound Assignment Desugaring — `a += b` → `a = a + b` for all five operators | CST has `Expr::Assign(lhs, AssignOp, rhs)` where `AssignOp` has `AddAssign`/`SubAssign`/`MulAssign`/`DivAssign`/`ModAssign`. AST target is `AstExpr::Assign { target, value: AstExpr::Binary { ... } }`. Plain `AssignOp::Assign` passes through. |
</phase_requirements>

---

## Summary

Phase 2 implements three expression-level desugaring passes that serve as shared helpers invoked from inside every subsequent structural pass. They are not top-level pipeline passes — they are utility functions (`lower_type`, `lower_expr`) called per-node from inside `lower_fn`, `lower_dialogue`, and `lower_entity`. Completing them before any structural pass is written ensures those passes always work on clean, sugar-free sub-expressions.

All three desugarings are fully specified by the Writ language spec and the CST/AST types already defined in Phase 1. The implementations are mechanical rewrites — no context state is required for optional or compound-assignment lowering, and formattable string lowering only needs the outer expression-lowering function for recursive descent. No new dependencies are needed: `insta 1.46.3` (already in `[dev-dependencies]`) handles snapshot testing for all three passes.

The file structure for Phase 2 is: two new files (`lower/optional.rs` for R3, `lower/fmt_string.rs` for R4), compound assignment handling wired into the expression-lowering function in `lower/expr.rs`, and a `tests/` directory under `writ-compiler/` with one integration test file per pass. The `lower/mod.rs` orchestrator gains real dispatch (iterate items, dispatch to `lower_fn` for `Item::Fn`), with the expression helpers invoked from inside `lower_fn`.

**Primary recommendation:** Create `lower/expr.rs` first (it houses `lower_expr`, the shared entry point for expression lowering that routes to all sub-desugarings). Add `lower/optional.rs` (type lowering) and `lower/fmt_string.rs` (formattable string lowering) as separate files. Write snapshot tests for each immediately after implementation — `insta` snapshots are the phase success gate.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust 2024 edition | (workspace) | Implementation language | Already established |
| `writ-parser` (internal) | workspace path | CST types consumed by lowering: `TypeExpr::Nullable`, `Expr::FormattableString`, `Expr::Assign` with `AssignOp` | Direct dependency already in `writ-compiler/Cargo.toml` |
| `chumsky` | `0.12.0` | `SimpleSpan` for span preservation on all emitted AST nodes | Already a direct dep in `writ-compiler/Cargo.toml` (required by Rust 2024 edition explicit dep rule) |
| `thiserror` | `2.0.18` | `LoweringError` (already implemented in Phase 1) | No new variants needed for Phase 2 passes |

### Supporting (dev-deps)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `insta` | `1.46.3` with `ron` feature | Snapshot testing of lowered AST output | All phase tests — one snapshot per transformation case |

### No New Dependencies Needed

Phase 2 requires no new `[dependencies]` entries. `insta` is already in `[dev-dependencies]`. The `const-fnv1a-hash` crate needed for Phase 4 localization keys is not required here.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `insta` snapshots | Manual `assert_eq!` on full AST struct | Hand-written expected AST structs for lowering output are impractical (deep nesting, many span fields); insta + `cargo insta review` is the standard compiler testing approach |
| Separate `lower_optional.rs` and `lower_fmt_string.rs` files | One omnibus `lower_expr.rs` | Separate files keep each lowering concern independently testable and independently reviewable |

**Installation:**

```bash
# No changes to Cargo.toml needed — all dependencies already present.
# Confirm with:
cargo build -p writ-compiler
```

---

## Architecture Patterns

### Recommended Project Structure After Phase 2

```
writ-compiler/src/
├── lib.rs                    # unchanged
├── main.rs                   # unchanged
├── ast/                      # unchanged (Phase 1 output)
│   ├── mod.rs
│   ├── expr.rs               # AstExpr (target for all 3 desugarings)
│   ├── stmt.rs               # AstStmt (target for lower_stmt)
│   ├── decl.rs               # AstDecl (target for lower_fn)
│   └── types.rs              # AstType::Generic is the T? target
└── lower/
    ├── mod.rs                # lower() — gains real Item dispatch in Phase 2
    ├── context.rs            # LoweringContext (unchanged)
    ├── error.rs              # LoweringError (unchanged)
    ├── expr.rs               # lower_expr() — main expression fold; handles all Expr variants
    ├── stmt.rs               # lower_stmt() — folds Stmt, calls lower_expr / lower_type
    ├── optional.rs           # lower_type() — folds TypeExpr; T? → Option<T>
    └── fmt_string.rs         # lower_fmt_string() — $"..." → concat chain

writ-compiler/tests/
└── lowering_tests.rs         # insta snapshot tests for all Phase 2 passes
```

### Pattern 1: The Expression Fold Function (`lower_expr`)

**What:** A consuming function `lower_expr(expr: Spanned<Expr<'_>>, ctx: &mut LoweringContext) -> AstExpr` that pattern-matches every CST `Expr` variant and returns the corresponding `AstExpr`. For most variants, this is a structural translation. For sugar variants (`FormattableString`, compound `Assign`), it performs the desugaring. This is the central recursive entry point — every branch that contains sub-expressions must call `lower_expr` recursively on them.

**When to use:** Called from `lower_stmt`, `lower_fn` body lowering, `lower_dialogue` (Phase 4), `lower_entity` field defaults (Phase 5).

**Critical invariant:** The outer span of the original CST expression must be preserved and assigned to the outermost AST node produced by any desugaring.

**Example — compound assignment branch:**

```rust
// lower/expr.rs
use writ_parser::cst::{Expr, AssignOp, Spanned};
use crate::ast::expr::{AstExpr, BinaryOp};
use crate::lower::context::LoweringContext;
use chumsky::span::SimpleSpan;

pub fn lower_expr(spanned: Spanned<Expr<'_>>, ctx: &mut LoweringContext) -> AstExpr {
    let (expr, span) = spanned;
    match expr {
        // --- Compound assignment: a += b → a = a + b ---
        Expr::Assign(lhs, op, rhs) => {
            let lhs_span = lhs.1;
            let lowered_lhs = lower_expr(*lhs, ctx);
            let lowered_rhs = lower_expr(*rhs, ctx);

            match op {
                AssignOp::Assign => AstExpr::Assign {
                    target: Box::new(lowered_lhs),
                    value: Box::new(lowered_rhs),
                    span,
                },
                AssignOp::AddAssign => AstExpr::Assign {
                    target: Box::new(lowered_lhs.clone()),
                    value: Box::new(AstExpr::Binary {
                        left: Box::new(lowered_lhs),
                        op: BinaryOp::Add,
                        right: Box::new(lowered_rhs),
                        span, // use outer span for synthetic binary node
                    }),
                    span,
                },
                // ... SubAssign, MulAssign, DivAssign, ModAssign follow same pattern
                _ => todo!()
            }
        }

        // --- All other variants are structural translations ---
        Expr::IntLit(s) => AstExpr::IntLit {
            value: s.parse::<i64>().unwrap_or(0),
            span,
        },
        Expr::NullLit => AstExpr::Path {
            segments: vec!["Option".to_string(), "None".to_string()],
            span,
        },
        Expr::FormattableString(segments) => lower_fmt_string(segments, span, ctx),
        // ...
        Expr::Error => AstExpr::Error { span },
    }
}
```

**Note on cloning for compound assignment:** The `lhs` expression is used twice in `a = a + b`. The `lhs` CST node is consumed by the first `lower_expr` call. The resulting `AstExpr` must be cloned for use as both `target` and `left`. Since `AstExpr` already derives `Clone`, this is valid. Consider whether cloning is acceptable here vs. an alternative representation — for Phase 2 it is correct and simple.

### Pattern 2: The Type Fold Function (`lower_type`)

**What:** A consuming function `lower_type(ty: Spanned<TypeExpr<'_>>, span: SimpleSpan) -> AstType` that handles all type positions. The critical case is `TypeExpr::Nullable(inner)` → `AstType::Generic { name: "Option", args: [lower_type(inner)], span }`.

**When to use:** Called from `lower_fn` parameter types, return types, struct field types, `let` annotation types. Also from `lower_entity` (Phase 5) for component field types.

**Does NOT need `&mut LoweringContext`:** Type lowering for Phase 2 is stateless — no errors to emit for optional sugar (it's always valid), no context state needed. Pass `ctx` only if you anticipate needing to emit errors in future passes (conservative: add it anyway for API consistency).

**Example:**

```rust
// lower/optional.rs
use writ_parser::cst::{TypeExpr, Spanned};
use crate::ast::types::AstType;
use chumsky::span::SimpleSpan;

pub fn lower_type(spanned: Spanned<TypeExpr<'_>>) -> AstType {
    let (ty, span) = spanned;
    match ty {
        TypeExpr::Named(name) => AstType::Named {
            name: name.to_string(),
            span,
        },
        TypeExpr::Generic(base, args) => AstType::Generic {
            name: match base.0 {
                TypeExpr::Named(n) => n.to_string(),
                _ => todo!("non-named generic base"),
            },
            args: args.into_iter().map(lower_type).collect(),
            span,
        },
        TypeExpr::Array(elem) => AstType::Array {
            elem: Box::new(lower_type(*elem)),
            span,
        },
        // KEY: T? → Option<T>
        TypeExpr::Nullable(inner) => AstType::Generic {
            name: "Option".to_string(),
            args: vec![lower_type(*inner)],
            span,
        },
        TypeExpr::Func(params, ret) => AstType::Func {
            params: params.into_iter().map(lower_type).collect(),
            ret: ret.map(|r| Box::new(lower_type(*r))),
            span,
        },
        TypeExpr::Void => AstType::Void { span },
    }
}
```

### Pattern 3: Formattable String Lowering (`lower_fmt_string`)

**What:** Folds a `Vec<Spanned<StringSegment>>` into a left-associative chain of `AstExpr::Binary { op: BinaryOp::Add, ... }` nodes, where each `StringSegment::Text` becomes an `AstExpr::StringLit` and each `StringSegment::Expr` becomes `AstExpr::GenericCall { callee: <lowered_expr>.field("into"), type_args: [AstType::Named("string")], args: [] }`.

**Empty string handling:** An empty `FormattableString` (no segments) should lower to `AstExpr::StringLit { value: "".to_string(), span }`.

**Single segment shortcut:** A single `Text` segment with no interpolations can lower directly to `AstExpr::StringLit`. A single `Expr` segment still needs the `.into<string>()` call.

**Concatenation direction:** The spec example `$"Hello {name}!"` lowers to `"Hello " + name.into<string>() + "!"`. This is left-associative: `("Hello " + name.into<string>()) + "!"`. Build using fold/reduce: start with the first segment's AstExpr, fold remaining segments with `Binary { op: Add, left: acc, right: next_segment, span }`.

**`{{` and `}}` handling:** The CST `StringSegment::Text` values already contain de-escaped text from the parser (the lexer resolves `{{` → `{` and `}}` → `}` during tokenization). Verify this assumption by checking the parser/lexer. If the CST preserves raw escape sequences, the lowering pass must decode them.

**`FormattableRawString` handling:** The CST also has `Expr::FormattableRawString(Vec<Spanned<StringSegment>>)`. This lowers identically to `FormattableString` — the raw vs. non-raw distinction (escape processing) is already resolved by the lexer before the CST is constructed.

**Example:**

```rust
// lower/fmt_string.rs
use writ_parser::cst::{StringSegment, Spanned};
use crate::ast::expr::{AstExpr, AstType, BinaryOp, AstArg};
use crate::lower::{context::LoweringContext, expr::lower_expr};
use chumsky::span::SimpleSpan;

pub fn lower_fmt_string(
    segments: Vec<Spanned<StringSegment<'_>>>,
    outer_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstExpr {
    if segments.is_empty() {
        return AstExpr::StringLit { value: String::new(), span: outer_span };
    }

    let mut parts: Vec<AstExpr> = segments
        .into_iter()
        .map(|(seg, seg_span)| match seg {
            StringSegment::Text(s) => AstExpr::StringLit {
                value: s.to_string(),
                span: seg_span,
            },
            StringSegment::Expr(inner_expr) => {
                // lower the interpolated expression, then wrap with .into<string>()
                let lowered = lower_expr(*inner_expr, ctx);
                // Emit: lowered.into<string>()
                AstExpr::GenericCall {
                    callee: Box::new(AstExpr::MemberAccess {
                        object: Box::new(lowered),
                        field: "into".to_string(),
                        field_span: seg_span,
                        span: seg_span,
                    }),
                    type_args: vec![AstType::Named {
                        name: "string".to_string(),
                        span: seg_span,
                    }],
                    args: vec![],
                    span: seg_span,
                }
            }
        })
        .collect();

    // Left-associative fold into Binary Add chain
    let first = parts.remove(0);
    parts.into_iter().fold(first, |acc, next| {
        AstExpr::Binary {
            left: Box::new(acc),
            op: BinaryOp::Add,
            right: Box::new(next),
            span: outer_span,
        }
    })
}
```

### Pattern 4: Statement Fold Function (`lower_stmt`)

**What:** A consuming function `lower_stmt(stmt: Spanned<Stmt<'_>>, ctx: &mut LoweringContext) -> AstStmt` that translates CST statements to AST statements, calling `lower_expr` and `lower_type` on sub-nodes.

**Phase 2 scope:** `lower_stmt` covers `Stmt::Let`, `Stmt::Expr`, `Stmt::Return`, `Stmt::For`, `Stmt::While`, `Stmt::Break`, `Stmt::Continue`, `Stmt::Atomic`. It must use `todo!()` for `Stmt::DlgDecl` and `Stmt::Transition` (Phase 4).

**Example — Let statement:**

```rust
Stmt::Let { mutable, name, ty, value } => {
    let (name_str, name_span) = name;
    let (val_expr, val_span) = value;
    AstStmt::Let {
        mutable,
        name: name_str.to_string(),
        name_span,
        ty: ty.map(lower_type),
        value: lower_expr((val_expr, val_span), ctx),
        span,
    }
}
```

### Pattern 5: Wiring Into `lower/mod.rs`

**What:** The Phase 1 `lower()` stub ignores all items. Phase 2 replaces the stub body with real dispatch: iterate items, match `Item::Fn` → call `lower_fn`, pass through all other structural items. `lower_fn` calls `lower_stmt` on its body, which recursively calls `lower_expr` and `lower_type`.

**Phase 2 scope in `lower/mod.rs`:** Handle `Item::Fn` only. Use `todo!()` for `Item::Dlg` and `Item::Entity`. Pass through `Item::Struct`, `Item::Enum`, `Item::Contract`, `Item::Impl`, `Item::Component`, `Item::Extern`, `Item::Const`, `Item::Global`, `Item::Namespace`, `Item::Using` as structural pass-throughs.

**Example — `lower_fn` sketch:**

```rust
// lower/fn.rs (or inline in lower/mod.rs for Phase 2)
pub fn lower_fn(
    fn_decl: writ_parser::cst::FnDecl<'_>,
    fn_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstDecl {
    let params: Vec<AstParam> = fn_decl.params.into_iter().map(|(param, param_span)| {
        AstParam {
            name: param.name.0.to_string(),
            name_span: param.name.1,
            ty: lower_type(param.ty),
            span: param_span,
        }
    }).collect();

    let return_type = fn_decl.return_type.map(lower_type);
    let body: Vec<AstStmt> = fn_decl.body.into_iter()
        .map(|stmt| lower_stmt(stmt, ctx))
        .collect();

    AstDecl::Fn(AstFnDecl {
        attrs: lower_attrs(fn_decl.attrs),
        vis: fn_decl.vis.map(lower_vis),
        name: fn_decl.name.0.to_string(),
        name_span: fn_decl.name.1,
        generics: fn_decl.generics.unwrap_or_default().into_iter()
            .map(|(gp, gp_span)| lower_generic_param(gp, gp_span))
            .collect(),
        params,
        return_type,
        body,
        span: fn_span,
    })
}
```

### Pattern 6: `insta` Snapshot Test Structure

**What:** Each pass has snapshot tests in `writ-compiler/tests/lowering_tests.rs`. Tests parse a small Writ source string (using `writ-parser`), call `lower()`, and snapshot the output `AstExpr`/`AstType`/`AstStmt`.

**Snapshot format:** `ron` (already configured in `Cargo.toml` dev-deps: `insta = { version = "1", features = ["ron"] }`). RON is human-readable and diff-friendly.

**Test workflow:**
1. Run `cargo test -p writ-compiler` on first pass — tests fail with "snapshot not found".
2. Run `cargo insta review` — inspect generated snapshots, accept correct ones.
3. Subsequent `cargo test` runs compare against accepted snapshots.
4. Snapshots stored in `writ-compiler/tests/snapshots/` (auto-created by insta).

**Note:** `insta` snapshots work with `assert_debug_snapshot!`, `assert_ron_snapshot!`, or `assert_yaml_snapshot!`. Use `assert_ron_snapshot!` for consistency with the RON feature flag.

**Example test:**

```rust
// writ-compiler/tests/lowering_tests.rs

use writ_compiler::lower;
use writ_parser::parser::parse;

fn lower_src(src: &str) -> writ_compiler::Ast {
    let (items, errors) = parse(src);
    assert!(errors.is_empty(), "parse errors: {:?}", errors);
    let (ast, lower_errors) = lower(items);
    assert!(lower_errors.is_empty(), "lowering errors: {:?}", lower_errors);
    ast
}

// ============================================================
// R3: Optional Sugar Lowering
// ============================================================

#[test]
fn optional_type_in_param() {
    let ast = lower_src("fn greet(name: string?) {}");
    insta::assert_ron_snapshot!(ast);
}

#[test]
fn null_literal_lowers_to_option_none() {
    let ast = lower_src("fn f() { let x: string? = null; }");
    insta::assert_ron_snapshot!(ast);
}

#[test]
fn nested_optional_type() {
    let ast = lower_src("fn f(x: List<string?>?) {}");
    insta::assert_ron_snapshot!(ast);
}

// ============================================================
// R4: Formattable String Lowering
// ============================================================

#[test]
fn fmt_string_simple_interpolation() {
    let ast = lower_src(r#"fn f(name: string) { let x = $"Hello {name}!"; }"#);
    insta::assert_ron_snapshot!(ast);
}

#[test]
fn fmt_string_escaped_braces() {
    let ast = lower_src(r#"fn f() { let x = $"{{literal}}"; }"#);
    insta::assert_ron_snapshot!(ast);
}

#[test]
fn fmt_string_multiple_segments() {
    let ast = lower_src(r#"fn f(a: int, b: int) { let x = $"a={a} b={b}"; }"#);
    insta::assert_ron_snapshot!(ast);
}

#[test]
fn fmt_string_no_interpolation() {
    let ast = lower_src(r#"fn f() { let x = $"plain string"; }"#);
    insta::assert_ron_snapshot!(ast);
}

// ============================================================
// R5: Compound Assignment Desugaring
// ============================================================

#[test]
fn compound_add_assign() {
    let ast = lower_src("fn f(mut x: int) { x += 1; }");
    insta::assert_ron_snapshot!(ast);
}

#[test]
fn compound_sub_assign() {
    let ast = lower_src("fn f(mut x: int) { x -= 2; }");
    insta::assert_ron_snapshot!(ast);
}

#[test]
fn compound_mul_assign() {
    let ast = lower_src("fn f(mut x: int) { x *= 3; }");
    insta::assert_ron_snapshot!(ast);
}

#[test]
fn compound_div_assign() {
    let ast = lower_src("fn f(mut x: int) { x /= 4; }");
    insta::assert_ron_snapshot!(ast);
}

#[test]
fn compound_mod_assign() {
    let ast = lower_src("fn f(mut x: int) { x %= 5; }");
    insta::assert_ron_snapshot!(ast);
}

#[test]
fn plain_assign_passes_through() {
    let ast = lower_src("fn f(mut x: int) { x = 0; }");
    insta::assert_ron_snapshot!(ast);
}
```

### Anti-Patterns to Avoid

- **Returning `Option<AstExpr>` from `lower_expr`:** Return `AstExpr::Error { span }` on error recovery, not `None`. This preserves the pipeline-never-halts invariant and makes the type signature consistent across all call sites.
- **Emitting `SimpleSpan::new(0, 0)` for synthetic nodes:** The synthetic `AstExpr::Binary` nodes generated by formattable string lowering and compound assignment expansion must carry the outer span of their originating CST expression. Never use a tombstone span.
- **Forgetting `FormattableRawString`:** `Expr::FormattableRawString` is a separate CST variant that lowers identically to `FormattableString`. Match both in `lower_expr` or it will be a compile error on exhaustive match.
- **Not calling `lower_expr` recursively inside `StringSegment::Expr`:** The interpolated expression `{expr}` may itself contain a `FormattableString`, compound assignment, or `NullLit`. Always recursively call `lower_expr` on segment expressions.
- **Creating a top-level pass for expression helpers:** These are helper functions, not pipeline passes. They are not called from `lower()` directly — they are called from inside `lower_fn`, `lower_dialogue`, `lower_entity`. The `lower()` orchestrator dispatches on `Item` variants, not on expression kinds.
- **Implementing `lower_stmt` as a match on top-level `Item::Stmt`:** In the CST, `Item::Stmt` represents a bare top-level statement. Most statements appear inside function bodies as `Stmt` inside `FnDecl::body`. Both must call the same `lower_stmt` function.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Snapshot testing of lowered AST output | Manual `assert_eq!` with hand-written expected `AstExpr` structs | `insta` with `assert_ron_snapshot!` | Deep AST trees with spans make manual assertions impractical; insta handles first-run generation and diff-friendly updates |
| String escape decoding in formattable strings | Custom `{{`/`}}` escape decoder in `lower_fmt_string` | Verify lexer already handles this | The `writ-parser` lexer almost certainly resolves `{{` → `{` during tokenization; adding a second decode in lowering would double-decode |
| FNV-1a hash for test key generation | Custom hash | `const-fnv1a-hash` (Phase 4) | Not needed for Phase 2; placeholder |

**Key insight:** Phase 2 is pure structural translation with no external algorithm complexity. The only "don't hand-roll" that matters here is snapshot testing — the effort of writing expected AST values manually for even a simple `$"Hello {name}!"` case (several levels of nested `AstExpr::Binary` + `AstExpr::GenericCall` + `AstExpr::MemberAccess` + spans) is prohibitive.

---

## Common Pitfalls

### Pitfall 1: Synthetic Node Span Tombstoning

**What goes wrong:** The `AstExpr::Binary` node generated for formattable string concatenation and compound assignment expansion is a synthetic node — it does not map 1:1 to a single CST node. A developer writes `span: SimpleSpan::new(0, 0)` as a placeholder.

**Why it happens:** The outer span of `$"Hello {name}!"` covers the whole string, but the individual `+` operators have no single CST span. It's unclear which span to use.

**How to avoid:** Use the span of the originating CST expression (the `FormattableString`'s outer span, or the compound assignment's outer span) for all synthetic nodes generated from that expression. This is the correct "lowered from" span — it points at the source construct that produced the synthetic node, which is exactly what downstream diagnostics need.

**Warning signs:** Any `SimpleSpan::new(0, 0)` in `lower/fmt_string.rs` or `lower/expr.rs`.

### Pitfall 2: Missing `FormattableRawString` Branch

**What goes wrong:** `lower_expr` handles `Expr::FormattableString` but omits `Expr::FormattableRawString`. This is a compile error on exhaustive match (Rust will catch it), but may be initially papered over with a `_ => todo!()` wildcard.

**Why it happens:** The developer focuses on the common case and defers the raw variant.

**How to avoid:** Remove any `_ => todo!()` wildcard from `lower_expr` before shipping Phase 2. Match all CST `Expr` variants explicitly — Rust's exhaustive match will enforce completeness.

**Warning signs:** A `_ =>` arm in `lower_expr` that covers more than `Expr::Error`.

### Pitfall 3: Not Recursively Lowering Interpolated Expressions

**What goes wrong:** In `lower_fmt_string`, a `StringSegment::Expr(inner)` is lowered by some shortcut that doesn't call the full `lower_expr`. The inner expression may contain compound assignments, null literals, or nested formattable strings that then survive into the AST as CST sugar.

**Why it happens:** The inner expression seems "already lowered" or the developer doesn't realize expressions can nest.

**How to avoid:** Always call `lower_expr(*inner, ctx)` on `StringSegment::Expr` contents — no exceptions. Write a test case with a nested formattable string: `$"outer {$"inner {x}"}"`  and verify the snapshot shows fully lowered output.

**Warning signs:** Snapshot for `fmt_string_nested` shows a `FormattableString` node inside the output.

### Pitfall 4: Cloning Issues in Compound Assignment Lowering

**What goes wrong:** `a += b` must lower to `a = a + b`, which uses `a` twice. The first `a` goes to the assignment `target`, the second to the `Binary` node's `left`. The CST `lhs` is a `Spanned<Expr<'_>>` (not `Copy`), so it cannot be used twice after being moved into `lower_expr`. A naive implementation will either fail to compile or produce code that clones CST nodes instead of lowered AST nodes.

**Why it happens:** The developer calls `lower_expr(lhs, ctx)` once for `target`, then tries to use `lhs` again for `left` — but `lhs` was moved.

**How to avoid:** There are two correct approaches:
1. Call `lower_expr` on `lhs` once, then clone the resulting `AstExpr` for the second use. `AstExpr` derives `Clone`, so `lowered_lhs.clone()` is valid.
2. Lower `lhs` to `AstExpr`, clone it, use the clone as `target` and the original as `left`.

Either is correct. Option 1 is preferred: lower first, then clone the cheap `AstExpr` rather than cloning the potentially larger CST node.

**Warning signs:** Borrow checker errors like "use of moved value: `lhs`" or `E0505` in `lower_expr`.

### Pitfall 5: Testing Only the Happy Path

**What goes wrong:** Tests only cover `$"plain string"`, `$"Hello {name}!"`, `x += 1`. The snapshot for `$"{{literal}}"` is never written. The `{{`/`}}` escape handling (whether done by the lexer or by lowering) is never tested. A bug exists that only affects escaped braces.

**Why it happens:** Escaped braces are easy to skip in test planning.

**How to avoid:** Success criterion 4 for Phase 2 requires snapshot tests for each lowering's "core transformations." Include at minimum:
- R4: `{{` and `}}` escape test; multi-segment test; no-interpolation test; single-expression test.
- R3: nested `T??` (double nullable — if valid in CST); field type test; return type test; local type test.
- R5: all five compound operators; plain `=` passthrough.

### Pitfall 6: `lower_type` Not Handling All `TypeExpr` Variants

**What goes wrong:** `lower_type` is a match on `TypeExpr`. A `_ => todo!()` arm causes a panic when a test uses a function type `fn(int) -> string?` — because the `Nullable` inside the `Func` return type hits the `Named` branch instead of the `Nullable` branch, or vice versa.

**Why it happens:** `TypeExpr` has 6 variants. Forgetting `Func` or `Array` is easy.

**How to avoid:** Remove wildcards from `lower_type` match. Implement all 6 variants: `Named`, `Generic`, `Array`, `Nullable`, `Func`, `Void`.

---

## Code Examples

Verified patterns from CST types and AST types already in the codebase:

### CST TypeExpr Variants (input to `lower_type`)

```rust
// Source: writ-parser/src/cst.rs
pub enum TypeExpr<'src> {
    Named(&'src str),
    Generic(Box<Spanned<TypeExpr<'src>>>, Vec<Spanned<TypeExpr<'src>>>),
    Array(Box<Spanned<TypeExpr<'src>>>),
    Nullable(Box<Spanned<TypeExpr<'src>>>),   // T? — the target of R3
    Func(Vec<Spanned<TypeExpr<'src>>>, Option<Box<Spanned<TypeExpr<'src>>>>),
    Void,
}
```

### CST Expr Variants Relevant to Phase 2 (input to `lower_expr`)

```rust
// Source: writ-parser/src/cst.rs (relevant excerpts)
pub enum Expr<'src> {
    NullLit,                                            // R3: → Option::None path
    FormattableString(Vec<Spanned<StringSegment<'src>>>), // R4: → concat chain
    FormattableRawString(Vec<Spanned<StringSegment<'src>>>), // R4: same lowering
    Assign(Box<Spanned<Expr<'src>>>, AssignOp, Box<Spanned<Expr<'src>>>), // R5
    // ... all other variants pass through via recursive lower_expr
}

pub enum AssignOp { Assign, AddAssign, SubAssign, MulAssign, DivAssign, ModAssign }

pub enum StringSegment<'src> {
    Text(&'src str),
    Expr(Box<Spanned<Expr<'src>>>),
}
```

### AST Target Types (output of Phase 2 lowering)

```rust
// Source: writ-compiler/src/ast/types.rs
pub enum AstType {
    Named { name: String, span: SimpleSpan },
    // T? lowers to this:
    Generic { name: String, args: Vec<AstType>, span: SimpleSpan },
    Array { elem: Box<AstType>, span: SimpleSpan },
    Func { params: Vec<AstType>, ret: Option<Box<AstType>>, span: SimpleSpan },
    Void { span: SimpleSpan },
}

// Source: writ-compiler/src/ast/expr.rs (relevant for R4 and R5 targets)
pub enum AstExpr {
    StringLit { value: String, span: SimpleSpan },   // text segments
    Path { segments: Vec<String>, span: SimpleSpan },  // null → Option::None
    Binary { left: Box<AstExpr>, op: BinaryOp, right: Box<AstExpr>, span: SimpleSpan }, // concat chain
    MemberAccess { object: Box<AstExpr>, field: String, field_span: SimpleSpan, span: SimpleSpan }, // .into
    GenericCall { callee: Box<AstExpr>, type_args: Vec<AstType>, args: Vec<AstArg>, span: SimpleSpan }, // .into<string>()
    Assign { target: Box<AstExpr>, value: Box<AstExpr>, span: SimpleSpan }, // a = ...
}

pub enum BinaryOp { Add, Sub, Mul, Div, Mod, /* ... */ }
```

### `insta` Snapshot Test Setup (verified: v1.46.3)

```toml
# writ-compiler/Cargo.toml [dev-dependencies] — already present
insta = { version = "1", features = ["ron"] }
```

```rust
// Usage pattern (insta 1.46.3)
#[test]
fn my_test() {
    let value = compute_something();
    // First run: creates .snap file in tests/snapshots/
    // Subsequent runs: compares against accepted snapshot
    insta::assert_ron_snapshot!(value);
    // Named snapshots (preferred for clarity):
    insta::assert_ron_snapshot!("name_of_snapshot", value);
}
```

```bash
# First run (creates pending snapshots):
cargo test -p writ-compiler

# Review and accept snapshots:
cargo insta review

# Or accept all (use carefully — review first):
cargo insta accept
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Ad-hoc `match` in a single pass over all constructs | Separate helper functions per lowering kind (`lower_type`, `lower_expr`, `lower_fmt_string`) | Established in rustc ~2015 | Each helper is independently testable; structural passes call helpers as needed |
| Mutable visitor with `visit_expr_mut` | Consuming fold (`lower_expr(expr: Expr) -> AstExpr`) | rustc HIR lowering pattern | Return type enforces the output is always an `AstExpr` — impossible to accidentally mutate the input |

**Deprecated/outdated:**
- Formattable string lowering via regex replacement on source strings: wrong level — lowering must operate on CST nodes, not source text.

---

## Open Questions

1. **Does the `writ-parser` lexer resolve `{{` → `{` and `}}` → `}` before constructing `StringSegment::Text`?**
   - What we know: `StringSegment::Text(&'src str)` holds a `&'src str` slice from the source buffer. If it holds the raw source bytes (including `{{`), then `lower_fmt_string` must decode them. If the lexer already resolves them, `lower_fmt_string` can use the text verbatim.
   - What's unclear: The `writ-parser` source for the formattable string lexer rule is not in scope for this research; it would need to be read from `writ-parser/src/lexer.rs` or `parser.rs`.
   - Recommendation: Before implementing `lower_fmt_string`, check `writ-parser/src/lexer.rs` for the `FormattableString` token rule. Look for whether it strips `{{` or passes raw bytes. Write a test that includes `{{` in a formattable string and inspect the CST `StringSegment::Text` value — if it contains `{` the lexer resolves it; if it contains `{{` the lowering pass must.
   - Impact: Low — at most 3 lines of string replacement in `lower_fmt_string` if the lexer does not resolve it.

2. **How should `lower_fn` handle the `attrs` field (flat attribute list vs. stacked `Vec<Vec<Attribute>>`)?**
   - What we know: CST `FnDecl.attrs` is `Vec<Spanned<Vec<Attribute<'src>>>>` (stacked attribute blocks). AST `AstFnDecl.attrs` is `Vec<AstAttribute>` (flat list). A `lower_attrs` function must flatten the outer `Vec` and convert each `Attribute<'src>` → `AstAttribute`.
   - What's unclear: The attribute body conversion (specifically `AttrArg` → `AstAttributeArg`) requires calling `lower_expr` on attribute argument expressions. This is straightforward but must be accounted for in the implementation scope.
   - Recommendation: Implement `lower_attrs` as a helper that flattens the stacked blocks and recursively lowers attribute argument expressions via `lower_expr`. Include this in Phase 2's `lower_fn` implementation.

3. **Should `lower_stmt` handle `Stmt::DlgDecl` and `Stmt::Transition` with `todo!()` or `unreachable!()`?**
   - What we know: `Stmt::DlgDecl` and `Stmt::Transition` are dialogue-specific statement forms that appear inside dialogue bodies. In Phase 2, only `Item::Fn` items are lowered. `Fn` bodies should not contain `DlgDecl` or `Transition` statements (those are only valid inside `dlg` blocks).
   - Recommendation: Use `todo!("Phase 4: dialogue statement forms not yet implemented")` rather than `unreachable!()`. It's theoretically possible for a parser to produce these in unexpected positions. `todo!()` produces a better panic message during development.

---

## Validation Architecture

Phase 2 success gate: all snapshot tests green, no `todo!()` panics on valid Writ input containing `T?`, `$"..."`, and compound assignments.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` + `insta 1.46.3` |
| Config file | None — `insta` detects RON feature from `Cargo.toml` |
| Quick run command | `cargo test -p writ-compiler` |
| Full suite command | `cargo test -p writ-compiler` |
| Snapshot review | `cargo insta review` (after first run) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File |
|--------|----------|-----------|-------------------|------|
| R3 | `T?` param → `Option<T>` | snapshot | `cargo test -p writ-compiler optional_type_in_param` | Wave 0 gap |
| R3 | `T?` return type → `Option<T>` | snapshot | `cargo test -p writ-compiler optional_type_in_return` | Wave 0 gap |
| R3 | `T?` field type → `Option<T>` | snapshot | `cargo test -p writ-compiler optional_type_in_field` | Wave 0 gap |
| R3 | `null` → `Option::None` path | snapshot | `cargo test -p writ-compiler null_literal_lowers_to_option_none` | Wave 0 gap |
| R4 | `$"Hello {name}!"` → concat chain | snapshot | `cargo test -p writ-compiler fmt_string_simple_interpolation` | Wave 0 gap |
| R4 | `{{`/`}}` → literal braces | snapshot | `cargo test -p writ-compiler fmt_string_escaped_braces` | Wave 0 gap |
| R4 | Multi-segment concat | snapshot | `cargo test -p writ-compiler fmt_string_multiple_segments` | Wave 0 gap |
| R5 | `a += b` → `a = a + b` | snapshot | `cargo test -p writ-compiler compound_add_assign` | Wave 0 gap |
| R5 | `a -= b` → `a = a - b` | snapshot | `cargo test -p writ-compiler compound_sub_assign` | Wave 0 gap |
| R5 | `a *= b` → `a = a * b` | snapshot | `cargo test -p writ-compiler compound_mul_assign` | Wave 0 gap |
| R5 | `a /= b` → `a = a / b` | snapshot | `cargo test -p writ-compiler compound_div_assign` | Wave 0 gap |
| R5 | `a %= b` → `a = a % b` | snapshot | `cargo test -p writ-compiler compound_mod_assign` | Wave 0 gap |
| R5 | Plain `a = b` passes through | snapshot | `cargo test -p writ-compiler plain_assign_passes_through` | Wave 0 gap |

### Wave 0 Gaps

All test infrastructure is new — no existing test files in `writ-compiler/tests/`.

- [ ] `writ-compiler/tests/lowering_tests.rs` — covers R3, R4, R5 snapshot tests
- [ ] `writ-compiler/tests/snapshots/` — directory created automatically by `insta` on first run

Framework is already installed (`insta = { version = "1", features = ["ron"] }` in `[dev-dependencies]`). No framework install needed.

---

## Sources

### Primary (HIGH confidence)

- `D:/dev/git/Writ/writ-parser/src/cst.rs` — definitive CST inventory; `TypeExpr::Nullable`, `Expr::FormattableString`, `Expr::Assign`, `AssignOp`, `StringSegment` types confirmed
- `D:/dev/git/Writ/writ-compiler/src/ast/expr.rs` — confirmed `AstExpr::Binary`, `AstExpr::Path`, `AstExpr::GenericCall`, `AstExpr::MemberAccess`, `AstExpr::Assign`, `BinaryOp` targets for all three lowerings
- `D:/dev/git/Writ/writ-compiler/src/ast/types.rs` — confirmed `AstType::Generic { name: "Option", ... }` as target for `T?`
- `D:/dev/git/Writ/writ-compiler/Cargo.toml` — confirmed `insta = { version = "1", features = ["ron"] }` already in dev-deps; `chumsky = "0.12.0"` as direct dep
- `D:/dev/git/Writ/language-spec/spec/20_19_nullability_optionals.md` — spec §19: `T?` = `Option<T>`, `null` = `Option::None`
- `D:/dev/git/Writ/language-spec/spec/18_17_operators_overloading.md` — spec §17.3: compound assignment lowering rules for all 5 operators
- `D:/dev/git/Writ/language-spec/spec/05_4_lexical_structure.md` — spec §4.4.2–4.4.4: formattable string semantics, `{{`/`}}` escape, `{expr}` → `.into<string>()`, `FormattableRawString` identical lowering
- `D:/dev/git/Writ/language-spec/spec/29_28_lowering_reference.md` — spec §28: lowering reference table and dialogue example using formattable string pattern
- `D:/dev/git/Writ/.planning/research/SUMMARY.md` — project research synthesis confirming pass ordering, fold pattern, insta for snapshot testing
- `D:/dev/git/Writ/.planning/STATE.md` — confirmed key decisions: fold pattern, LoweringContext, pass ordering rationale
- `cargo metadata` output — confirmed `insta 1.46.3`, `thiserror 2.0.18`, `chumsky 0.12.0` exact versions

### Secondary (MEDIUM confidence)

- `D:/dev/git/Writ/.planning/codebase/TESTING.md` — confirmed project uses `tests/` directory, `cargo test`, insta RON pattern, descriptive snake_case test function names
- `D:/dev/git/Writ/.planning/codebase/CONVENTIONS.md` — confirmed module naming (snake_case files), function naming, import order conventions
- `D:/dev/git/Writ/.planning/phases/01-ast-foundation/01-RESEARCH.md` — Phase 1 research: fold pattern rationale, span-per-node invariant, anti-patterns confirmed

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all existing versions confirmed via `cargo metadata`
- Architecture: HIGH — CST and AST types read directly from source; all inputs and outputs verified; lowering rules from spec; fold pattern from Phase 1 research
- Pitfalls: HIGH — tombstoning and recursive-lowering pitfalls are direct derivations from known rules; clone-in-compound-assign is a concrete Rust borrow checker constraint

**Research date:** 2026-02-26
**Valid until:** 2026-09-01 (stable Rust ecosystem; all dependencies are stable crates with no time-sensitive concerns)
