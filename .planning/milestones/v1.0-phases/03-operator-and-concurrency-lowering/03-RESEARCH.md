# Phase 3: Operator and Concurrency Lowering - Research

**Researched:** 2026-02-26
**Domain:** Rust compiler declaration-level desugaring — operator overload declarations to contract impl nodes; concurrency expression pass-through with snapshot testing
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| R6 | Operator Lowering — `operator +` inside `impl` → standalone `impl Add<T, T> for Self`; unary `-()` → `Neg`; `!()` → `Not`; index `[]` → `Index`; `[]=` → `IndexMut`; derived operators (`!=`, `>`, `<=`, `>=`) auto-generated | The CST `ImplDecl` carries `ImplMember::Op(OpDecl)` for operator bodies. The AST `AstImplDecl` has `contract: Option<AstType>` and `members: Vec<AstImplMember>`. The lowering must extract each `ImplMember::Op` and emit a new `AstDecl::Impl` with `contract: Some(Add<T, T>)` and `members: [AstImplMember::Fn(lowered_op_body_as_fn)]`. The existing `lower_impl` passes `Op` through unchanged — Phase 3 replaces that with the desugaring pass. |
| R7 | Concurrency Pass-Through — `spawn`, `join`, `cancel`, `defer`, `detached` each map 1:1 to their `AstExpr` or `AstStmt` variant with span preserved and no semantic transformation | Already implemented in `lower_expr` (Phase 2 covered all `Expr` variants). The remaining work is snapshot tests covering these five primitives to meet the success criteria. |
</phase_requirements>

---

## Summary

Phase 3 divides into two work items with very different character. R7 is essentially complete at the expression level from Phase 2 — `lower_expr` already handles `Expr::Spawn`, `Expr::Detached`, `Expr::Join`, `Expr::Cancel`, and `Expr::Defer` as 1:1 pass-throughs with span preservation. The only remaining work is snapshot tests that demonstrate and lock down this behavior.

R6 is the substantive new work. The current `lower_impl` in `lower/mod.rs` (line 447–465) passes `ImplMember::Op(OpDecl)` directly through to `AstImplMember::Op(AstOpDecl)` — the operator stays inside its parent impl block as an opaque node. The R6 requirement transforms this: each operator member must be extracted and re-emitted as a new standalone `AstDecl::Impl { contract: Some(Add<T, T>), target: Self, members: [Fn(lowered_body)] }`. The parent impl block keeps only its non-operator `Fn` members.

The operator-to-contract name mapping is fully specified: `+` → `Add`, `-` (binary) → `Sub`, `*` → `Mul`, `/` → `Div`, `%` → `Mod`, `==` → `Eq`, `<` → `Ord`, `-()` (unary) → `Neg`, `!()` → `Not`, `[]` → `Index`, `[]=` → `IndexMut`. Derived operator auto-generation (R6 §17.4) synthesizes three additional `AstImplDecl` nodes from an `Eq` impl (`!=` → `!(a == b)`) and two from `Eq` + `Ord` (`<=`, `>=`, `>`). These are synthetic and must carry the originating impl span.

**Primary recommendation:** Implement R6 as a new `lower/operator.rs` module containing a `lower_operator_impls` function that takes an `ImplDecl` and emits `Vec<AstDecl>` — one pass-through impl (for non-Op members) plus one new `AstDecl::Impl` per `Op` member. Wire it into `lower_impl` in `lower/mod.rs`. R7 snapshot tests go into the existing `writ-compiler/tests/lowering_tests.rs` alongside Phase 2 tests.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust 2024 edition | (workspace) | Implementation language | Already established in all crates |
| `writ-parser` (internal) | workspace path | CST types: `ImplDecl`, `ImplMember::Op`, `OpDecl`, `OpSymbol` — the input to operator lowering | Already a direct dep in `writ-compiler/Cargo.toml` |
| `chumsky` | `0.12.0` | `SimpleSpan` for every synthetic `AstDecl::Impl` node emitted by operator desugaring | Already a direct dep (required by Rust 2024 edition explicit-dep rule) |
| `thiserror` | `2.0` | `LoweringError` — no new variants expected for Phase 3 (operator lowering has no error conditions at this stage) | Already present |

### Supporting (dev-deps)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `insta` | `1` with `ron` feature | Snapshot testing of lowered AST output | All R6 and R7 tests — one snapshot per transformation case |

### No New Dependencies Needed

Phase 3 requires zero new `[dependencies]` or `[dev-dependencies]` entries. All needed types, traits, and test infrastructure are already present.

**Verification:**
```bash
cargo build -p writ-compiler   # confirms current baseline compiles
cargo test -p writ-compiler    # confirms 14 existing tests pass
```

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| New `lower/operator.rs` for R6 | Inline in `lower/mod.rs` `lower_impl` function | `lower/mod.rs` is already long; operator lowering involves synthesizing multiple new `AstDecl::Impl` nodes per impl block, which warrants its own module for clarity |
| Emitting derived operator impls at call-site (expression level) | Emitting them at declaration-level when the `impl` block is lowered | Spec §17.4 says derived operators are auto-generated from a base implementation — this is a declaration-level transformation, not expression-level |

---

## Architecture Patterns

### Recommended Project Structure After Phase 3

```
writ-compiler/src/
├── lib.rs
├── ast/                      (unchanged from Phase 2)
└── lower/
    ├── mod.rs                # lower() — lower_impl updated to call lower_operator_impls
    ├── context.rs            # LoweringContext (no changes needed)
    ├── error.rs              # LoweringError (no new variants for Phase 3)
    ├── expr.rs               # lower_expr() (unchanged — concurrency already done)
    ├── stmt.rs               # lower_stmt() (unchanged)
    ├── optional.rs           # lower_type() (unchanged)
    ├── fmt_string.rs         # lower_fmt_string() (unchanged)
    └── operator.rs           # NEW: lower_operator_impls() — R6 desugaring

writ-compiler/tests/
└── lowering_tests.rs         # Add R6 + R7 snapshot tests to existing file
```

### Pattern 1: Operator Lowering — Impl Member Extraction

**What:** `lower_impl` currently uses `lower_op_decl` to pass `ImplMember::Op` through unchanged. Phase 3 replaces this: `lower_impl` calls `lower_operator_impls(i, i_span, ctx)` which returns a `Vec<AstDecl>`. The caller site in `lower()` must accept this Vec (one decl per non-Op-member group, plus one per Op). Both top-level item dispatch and namespace block dispatch need updating.

**The key structural change:** Instead of `lower_impl` returning a single `AstImplDecl`, it needs to return `Vec<AstDecl>` — one for the non-operator members (possibly empty if the impl only has operators), plus one `AstDecl::Impl` per `operator` member with the correct contract set. The `lower()` loop changes from `decls.push(AstDecl::Impl(lower_impl(...)))` to `decls.extend(lower_operator_impls(...))`.

**Example — operator lowering function:**

```rust
// lower/operator.rs

use chumsky::span::SimpleSpan;
use writ_parser::cst::{ImplDecl, ImplMember, OpDecl, OpSymbol, Spanned};
use crate::ast::{AstDecl, AstType};
use crate::ast::decl::{AstFnDecl, AstImplDecl, AstImplMember, AstParam};
use crate::lower::context::LoweringContext;
use crate::lower::optional::lower_type;
use crate::lower::stmt::lower_stmt;

/// Lowers an `impl` block, extracting operator declarations into
/// standalone contract impl nodes and leaving function members in place.
///
/// Returns a Vec because one input ImplDecl may produce N+1 output AstDecls:
/// - One for the remaining Fn members (omitted if empty)
/// - One per Op member (each becomes impl <Contract> for <Target>)
pub fn lower_operator_impls(
    i: ImplDecl<'_>,
    i_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> Vec<AstDecl> {
    let target_type = lower_type(i.target.clone());
    let contract_type = i.contract.as_ref().map(|c| lower_type(c.clone()));

    let mut fn_members = Vec::new();
    let mut operator_decls: Vec<AstDecl> = Vec::new();

    for (member, member_span) in i.members {
        match member {
            ImplMember::Fn((fn_decl, fn_span)) => {
                fn_members.push(AstImplMember::Fn(lower_fn(fn_decl, fn_span, ctx)));
            }
            ImplMember::Op((op_decl, op_span)) => {
                // Desugar operator + (other: T) -> T { body }
                // into: impl Add<T, T> for Self { fn add(other: T) -> T { body } }
                let contract = op_symbol_to_contract(&op_decl.symbol.0, &op_decl, op_span, &target_type);
                let method_name = op_symbol_to_method_name(&op_decl.symbol.0);
                let lowered_fn = op_decl_to_fn(op_decl, op_span, method_name, ctx);

                operator_decls.push(AstDecl::Impl(AstImplDecl {
                    contract: Some(contract),
                    target: target_type.clone(),
                    members: vec![AstImplMember::Fn(lowered_fn)],
                    span: op_span,
                }));

                // Also auto-generate derived operator impls (spec §17.4)
                // From Eq (==): generate != impl
                // From Ord (<): generate > impl
                // From Eq + Ord: generate <= and >= impls
                // (Done in a second pass after collecting all Op members)
            }
        }
    }

    let mut result = Vec::new();

    // Emit the base impl (Fn members only) if there are any
    if !fn_members.is_empty() || contract_type.is_some() {
        result.push(AstDecl::Impl(AstImplDecl {
            contract: contract_type,
            target: target_type.clone(),
            members: fn_members,
            span: i_span,
        }));
    }

    result.extend(operator_decls);
    result
}
```

**Why the Fn impl emits even with no Fn members when there is a contract:** An `impl MyContract for MyType { operator +(...) { ... } }` should produce the contract impl for the remaining methods (none) plus the operator impl. This needs care — if `fn_members` is empty AND there is no `contract_type`, skip the base impl.

### Pattern 2: Operator Symbol to Contract Name Mapping

**What:** Each `OpSymbol` maps to a contract name and a type signature for the `impl Contract<Args, Ret> for Target` form.

**The complete mapping (from spec §17.2):**

```rust
// Source: language-spec/spec/18_17_operators_overloading.md §17.2 + §17.4
fn op_symbol_to_contract(
    sym: &OpSymbol,
    op: &OpDecl,
    span: SimpleSpan,
    target: &AstType,
) -> AstType {
    // For binary ops: impl Add<OtherType, ReturnType> for Self
    // For unary ops: impl Neg<ReturnType> for Self
    // For index ops: impl Index<IndexType, ReturnType> for Self
    match sym {
        OpSymbol::Add => make_contract("Add", op, span),    // + → Add<Other, Ret>
        OpSymbol::Sub => make_contract("Sub", op, span),    // - (binary) → Sub<Other, Ret>
        OpSymbol::Mul => make_contract("Mul", op, span),    // * → Mul<Other, Ret>
        OpSymbol::Div => make_contract("Div", op, span),    // / → Div<Other, Ret>
        OpSymbol::Mod => make_contract("Mod", op, span),    // % → Mod<Other, Ret>
        OpSymbol::Eq  => make_contract("Eq", op, span),     // == → Eq<Other> (ret bool implied)
        OpSymbol::Lt  => make_contract("Ord", op, span),    // < → Ord<Other> (ret bool implied)
        OpSymbol::Not => make_contract("Not", op, span),    // !() → Not<Ret>
        // Unary - (zero params): → Neg<Ret>
        OpSymbol::Sub if op.params.is_empty() => make_contract("Neg", op, span),
        OpSymbol::Index    => make_contract("Index", op, span),    // [] → Index<Idx, Ret>
        OpSymbol::IndexSet => make_contract("IndexMut", op, span), // []= → IndexMut<Idx, Val>
    }
}
```

**Note on Sub disambiguation:** The CST uses `OpSymbol::Sub` for BOTH binary subtraction and unary negation. Disambiguation is by parameter count: zero params = unary `Neg`, one param = binary `Sub`. This is the same disambiguation that the spec implies.

**Contract type args construction:**
- Binary operators (`Add`, `Sub`, `Mul`, `Div`, `Mod`): `Contract<OtherType, ReturnType>` — type args are `[param[0].ty, return_type]`
- Comparison operators (`Eq`, `Ord`): `Contract<OtherType>` — one type arg (return is always `bool`)
- Unary operators (`Neg`, `Not`): `Contract<ReturnType>` — one type arg
- Index operators (`Index`, `IndexMut`): `Contract<IndexType, ReturnType>` / `Contract<IndexType, ValueType>`

**Method name mapping (the fn name inside the generated impl):**

| OpSymbol | Params | Contract | Method Name |
|----------|--------|----------|-------------|
| Add | 1 | Add | `add` |
| Sub | 1 | Sub | `sub` |
| Sub | 0 | Neg | `neg` |
| Mul | 1 | Mul | `mul` |
| Div | 1 | Div | `div` |
| Mod | 1 | Mod | `mod_` (or `mod`) |
| Eq | 1 | Eq | `eq` |
| Lt | 1 | Ord | `lt` |
| Not | 0 | Not | `not` |
| Index | 1 | Index | `index` |
| IndexSet | 2 | IndexMut | `index_set` |

### Pattern 3: Derived Operator Auto-Generation (spec §17.4)

**What:** When an `impl` block defines `operator ==` (Eq) or `operator <` (Ord), derived operators are auto-generated as additional `AstDecl::Impl` nodes.

**The four derived rules:**

```
Eq impl present → generate:
  impl Ne for Self {
      fn ne(other: Self) -> bool { !(self == other) }
  }

Ord impl present → generate:
  impl Gt for Self {
      fn gt(other: Self) -> bool { other < self }
  }

Both Eq AND Ord present → generate:
  impl LtEq for Self {
      fn lt_eq(other: Self) -> bool { self < other || self == other }
  }
  impl GtEq for Self {
      fn gt_eq(other: Self) -> bool { !(self < other) }
  }
```

**Implementation approach:** After processing all `ImplMember::Op` members in an impl block, check if any produced an `Eq` contract impl and any produced an `Ord` contract impl. If yes, emit the corresponding derived impls with synthesized `AstFnDecl` bodies. All synthetic nodes carry the originating impl block's span (`i_span`).

**Synthesizing the body of `ne`:**
```rust
// Synthesize: fn ne(other: SelfType) -> bool { !(self == other) }
// AstStmt::Return { value: AstExpr::UnaryPrefix { op: Not, expr: Binary { op: Eq, ... } } }
```

This requires constructing `AstExpr` nodes from scratch — no CST input. Use `i_span` (the originating impl span) for all synthetic node spans.

### Pattern 4: Concurrency Snapshot Tests (R7 verification)

**What:** `lower_expr` already handles all five concurrency CST variants as 1:1 pass-throughs (Phase 2). Phase 3 adds snapshot tests that lock down this behavior. Tests go into the existing `writ-compiler/tests/lowering_tests.rs`.

**Note on `defer` in CST:** `Expr::Defer` wraps an inner `Expr`. The spec §20.2 shows `defer { ... }` as a block syntax, but in the CST it is represented as `Expr::Defer(Box<Spanned<Expr>>)` where the inner `Expr` is a `Block(Vec<Spanned<Stmt>>)`. The test should verify the block form.

**Note on `spawn detached`:** The spec says `spawn detached expr` but in the CST this is `Expr::Spawn(Box<Spanned<Expr::Detached(...)>>)` — i.e., the parser disambiguates by making `detached` a separate `Expr::Detached` variant that wraps the inner expression. The actual `spawn detached moveBoulder(...)` would parse as `Spawn(Detached(Call(...)))`. Snapshot tests should cover both `spawn expr` and `spawn detached expr`.

**Example test structure:**
```rust
// R7 concurrency pass-through tests

#[test]
fn concurrency_spawn_passthrough() {
    let ast = lower_src("fn f() { let h = spawn doWork(); }");
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn concurrency_join_passthrough() {
    let ast = lower_src("fn f(h: Handle) { join h; }");
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn concurrency_cancel_passthrough() {
    let ast = lower_src("fn f(h: Handle) { cancel h; }");
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn concurrency_defer_passthrough() {
    let ast = lower_src("fn f() { defer { cleanup(); } }");
    insta::assert_debug_snapshot!(ast);
}

#[test]
fn concurrency_detached_passthrough() {
    let ast = lower_src("fn f() { let h = spawn detached doWork(); }");
    insta::assert_debug_snapshot!(ast);
}
```

### Pattern 5: Wiring Into `lower/mod.rs` and Namespace Blocks

**What:** The `lower_impl` function in `lower/mod.rs` currently returns a single `AstImplDecl`. After Phase 3, it must call `lower_operator_impls` and extend the `decls` Vec with multiple results. Both the top-level dispatch loop and the `lower_namespace` block-form loop need updating.

**Current call sites that need updating:**

1. Top-level `lower()` loop (line 84–86 in `lower/mod.rs`):
   ```rust
   Item::Impl((i, i_span)) => {
       decls.push(AstDecl::Impl(lower_impl(i, i_span, &mut ctx)));
   }
   ```
   Changes to:
   ```rust
   Item::Impl((i, i_span)) => {
       decls.extend(lower_operator_impls(i, i_span, &mut ctx));
   }
   ```

2. `lower_namespace` Block arm (lines 319–321 in `lower/mod.rs`):
   ```rust
   Item::Impl((i, i_span)) => {
       decls.push(AstDecl::Impl(lower_impl(i, i_span, ctx)));
   }
   ```
   Changes identically.

3. The existing `lower_impl` private function in `lower/mod.rs` can be replaced entirely by `lower_operator_impls` or kept as a helper (for the no-operator case).

### Anti-Patterns to Avoid

- **Keeping `AstImplMember::Op` in the final AST:** After Phase 3, no `AstImplDecl` should contain `AstImplMember::Op` nodes. The whole point of R6 is to eliminate operator decls as impl members. If any `AstImplMember::Op` remains, downstream phases will be confused.
- **Synthesizing derived operators with tombstone spans:** All synthetic `AstExpr::Binary`, `AstExpr::UnaryPrefix`, and `AstExpr::Return` nodes in derived operator bodies must carry `i_span` (the originating impl block span). Never use `SimpleSpan::new(0, 0)`.
- **Forgetting the namespace block dispatch:** There are TWO call sites for `lower_impl` in `lower/mod.rs` — one in the top-level `lower()` loop and one in `lower_namespace`. Both must be updated to call `lower_operator_impls` and `extend` rather than `push`.
- **Using `_ =>` wildcard on `OpSymbol`:** The match on `OpSymbol` in `op_symbol_to_contract` must be exhaustive. All 10 variants (`Add`, `Sub`, `Mul`, `Div`, `Mod`, `Eq`, `Lt`, `Not`, `Index`, `IndexSet`) must be handled explicitly.
- **Wrong parameter count for unary `Sub` disambiguation:** `OpSymbol::Sub` with zero params is unary negation (`Neg`); with one param is binary subtraction (`Sub`). A guard on `op.params.is_empty()` is the correct disambiguation.
- **Emitting an empty base impl when there are no Fn members and no contract:** If an `impl` block contains only operator declarations with no base contract and no Fn members, the "base impl" node would be `impl Self { members: [] }` — emit nothing in this case.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Snapshot testing of lowered impl AST | Manual `assert_eq!` with hand-written `AstImplDecl` structs | `insta` with `assert_debug_snapshot!` | The desugared output has multiple levels of nesting (impl → fn → stmt → expr) with span fields; hand-written expectations are impractical |
| Contract name to contract type mapping | Ad-hoc string table | Explicit `match OpSymbol` | The mapping is a small, bounded enum — an explicit match is cleaner and Rust will catch missing branches |
| Derived operator body synthesis | `eval`-style dynamic construction | Direct `AstExpr` construction | The bodies are simple known forms (`!(a == b)`, `b < a`, etc.) — construct them directly from `AstExpr` variants |

**Key insight:** Phase 3's complexity is in correctness of the operator mapping (all 10 symbols, the binary/unary disambiguation, the derived generation rules), not in algorithmic complexity. Explicit exhaustive matches are the correct tool.

---

## Common Pitfalls

### Pitfall 1: `OpSymbol::Sub` Binary/Unary Disambiguation

**What goes wrong:** `operator -(other: T) -> T { ... }` (binary Sub) and `operator -() -> T { ... }` (unary Neg) both use `OpSymbol::Sub` in the CST. A naive match on `OpSymbol::Sub => "Sub"` generates a wrong `impl Sub<T, T> for Self` for the unary case.

**Why it happens:** The CST uses the same `OpSymbol::Sub` for both forms; the disambiguation is only possible by looking at `op.params.len()`.

**How to avoid:** In `op_symbol_to_contract`, add a guard:
```rust
OpSymbol::Sub if op.params.is_empty() => make_contract("Neg", op, span),
OpSymbol::Sub => make_contract("Sub", op, span),
```
Or equivalently, check param count before branching on the symbol.

**Warning signs:** A snapshot test for `operator -() -> vec2 { ... }` shows `contract: "Sub"` instead of `contract: "Neg"`.

### Pitfall 2: Two Call Sites for `lower_impl` in `lower/mod.rs`

**What goes wrong:** The developer updates the top-level `lower()` dispatch loop to use `lower_operator_impls` but forgets the `lower_namespace` Block arm, which also calls `lower_impl`. Tests that exercise top-level `impl` blocks pass; tests with `impl` inside a `namespace {}` block still emit `AstImplMember::Op` nodes.

**Why it happens:** `lower/mod.rs` duplicates the Item dispatch logic in two places (top-level and namespace block). The duplication is a structural artifact.

**How to avoid:** After implementing `lower_operator_impls`, search `lower/mod.rs` for all occurrences of `lower_impl` and update every call site:
```bash
grep -n "lower_impl" writ-compiler/src/lower/mod.rs
```
Both must use `decls.extend(lower_operator_impls(...))`.

**Warning signs:** A namespace snapshot test shows `AstImplMember::Op` in the AST output.

### Pitfall 3: Derived Operators Synthesized Even Without the Base Impl

**What goes wrong:** The derived-operator generation code runs for every impl block regardless of whether that block defines an `Eq` or `Ord` operator. The developer writes a loop that emits derived impls based on a flag that isn't correctly scoped per-impl-block.

**Why it happens:** The derivation logic is run after the main processing loop, and the flag (`has_eq`, `has_ord`) must be local to each call of `lower_operator_impls` — not a global or module-level state.

**How to avoid:** Track `has_eq` and `has_ord` as `bool` locals inside `lower_operator_impls`. Reset them for every call. Do not use `LoweringContext` fields for this.

**Warning signs:** Two separate `impl` blocks produce derived operators from one block's `Eq` impl crossing into the other's scope.

### Pitfall 4: Synthetic Derived Operator Bodies Using Wrong `self`/`other` Reference

**What goes wrong:** The derived `ne` body should be `!(self == other)`. Building this as `AstExpr` requires `AstExpr::SelfLit` for `self` and `AstExpr::Ident { name: "other" }` for the parameter. A developer uses `AstExpr::Ident { name: "self" }` (string ident) instead of `AstExpr::SelfLit`, which may not work in downstream phases that distinguish these forms.

**Why it happens:** `self` looks like any other identifier.

**How to avoid:** Use `AstExpr::SelfLit { span }` for the `self` expression in all synthetic derived operator bodies.

### Pitfall 5: The Base Impl Emitted When It Should Be Empty

**What goes wrong:** An impl block `impl vec2 { operator +(other: vec2) -> vec2 { ... } }` (operators only, no methods) emits a spurious `impl vec2 { members: [] }` alongside the `impl Add<vec2, vec2> for vec2` node.

**Why it happens:** The base impl is emitted unconditionally.

**How to avoid:** Only emit the base impl if `fn_members` is non-empty OR if there is a `contract_type` (meaning the original impl had a `for` clause indicating it was implementing a named contract that may have type-system significance beyond just members). If the impl is a bare `impl vec2 { }` with no contract and no fn members, skip it.

### Pitfall 6: R7 Tests Failing Because `spawn detached` Parses as Nested Expressions

**What goes wrong:** The test `let h = spawn detached doWork();` is written expecting `AstExpr::Spawn` with a direct `AstExpr::Call` inside. But the CST parses `spawn detached expr` as `Expr::Spawn(Box<Expr::Detached(Box<Expr::Call(...)>)>)`, so the lowered AST is `AstExpr::Spawn { expr: AstExpr::Detached { expr: AstExpr::Call { ... } } }`.

**Why it happens:** The CST represents `spawn detached` as nested — `detached` is a separate wrapping expression. The `spawn` sees `detached expr` as its inner expression.

**How to avoid:** Write the snapshot test and run it once to see the actual structure before writing any expectation. The snapshot framework will capture the actual output; accept it if correct.

---

## Code Examples

Verified patterns from the existing codebase:

### Current `lower_impl` in `lower/mod.rs` (to be replaced)

```rust
// Source: writ-compiler/src/lower/mod.rs lines 447–465
fn lower_impl(i: ImplDecl<'_>, i_span: SimpleSpan, ctx: &mut LoweringContext) -> AstImplDecl {
    AstImplDecl {
        contract: i.contract.map(lower_type),
        target: lower_type(i.target),
        members: i
            .members
            .into_iter()
            .map(|(member, _member_span)| match member {
                ImplMember::Fn((fn_decl, fn_span)) => {
                    AstImplMember::Fn(lower_fn(fn_decl, fn_span, ctx))
                }
                ImplMember::Op((op_decl, op_span)) => {
                    AstImplMember::Op(lower_op_decl(op_decl, op_span, ctx))  // <-- Phase 3 changes this
                }
            })
            .collect(),
        span: i_span,
    }
}
```

### CST Types for Operator Lowering (input)

```rust
// Source: writ-parser/src/cst.rs
pub enum ImplMember<'src> {
    Fn(Spanned<FnDecl<'src>>),
    Op(Spanned<OpDecl<'src>>),           // ← Phase 3 processes this
}

pub struct OpDecl<'src> {
    pub vis: Option<Visibility>,
    pub symbol: Spanned<OpSymbol>,       // which operator
    pub params: Vec<Spanned<Param<'src>>>,  // 0 params = unary, 1 = binary
    pub return_type: Option<Spanned<TypeExpr<'src>>>,
    pub body: Vec<Spanned<Stmt<'src>>>,
}

pub enum OpSymbol {
    Add, Sub, Mul, Div, Mod,
    Eq, Lt,
    Not,
    Index, IndexSet,
}
```

### AST Types for Operator Lowering (output)

```rust
// Source: writ-compiler/src/ast/decl.rs
pub struct AstImplDecl {
    pub contract: Option<AstType>,   // ← Set to "Add<T, T>" etc. after Phase 3
    pub target: AstType,
    pub members: Vec<AstImplMember>,
    pub span: SimpleSpan,
}

pub enum AstImplMember {
    Fn(AstFnDecl),
    Op(AstOpDecl),                   // ← Should be empty after Phase 3 passes through
}
```

### Concurrency CST → AST (already implemented in `lower_expr`)

```rust
// Source: writ-compiler/src/lower/expr.rs lines 206–234
// These are ALREADY IMPLEMENTED — only tests are needed for R7

Expr::Spawn(e) => AstExpr::Spawn { expr: Box::new(lower_expr(*e, ctx)), span },
Expr::Detached(e) => AstExpr::Detached { expr: Box::new(lower_expr(*e, ctx)), span },
Expr::Join(e) => AstExpr::Join { expr: Box::new(lower_expr(*e, ctx)), span },
Expr::Cancel(e) => AstExpr::Cancel { expr: Box::new(lower_expr(*e, ctx)), span },
Expr::Defer(e) => AstExpr::Defer { expr: Box::new(lower_expr(*e, ctx)), span },
```

### Synthetic Derived Operator Body Construction

```rust
// Constructing `!(self == other)` for Ne derived impl
// All spans use `impl_span` (the originating impl block's span)
let ne_body = AstExpr::UnaryPrefix {
    op: PrefixOp::Not,
    expr: Box::new(AstExpr::Binary {
        left: Box::new(AstExpr::SelfLit { span: impl_span }),
        op: BinaryOp::Eq,
        right: Box::new(AstExpr::Ident { name: "other".to_string(), span: impl_span }),
        span: impl_span,
    }),
    span: impl_span,
};

// Constructing `b < a` for Gt derived impl (note: other is left, self is right)
let gt_body = AstExpr::Binary {
    left: Box::new(AstExpr::Ident { name: "other".to_string(), span: impl_span }),
    op: BinaryOp::Lt,
    right: Box::new(AstExpr::SelfLit { span: impl_span }),
    span: impl_span,
};
```

### `insta` Test Pattern (established by Phase 2)

```rust
// Source: writ-compiler/tests/lowering_tests.rs
fn lower_src(src: &'static str) -> Ast {
    let (items, parse_errors) = writ_parser::parse(src);
    let items = items.expect("parse returned None");
    let error_msgs: Vec<String> = parse_errors.iter().map(|e| format!("{e:?}")).collect();
    assert!(error_msgs.is_empty(), "parse errors: {:?}", error_msgs);
    let (ast, lower_errors) = lower(items);
    assert!(lower_errors.is_empty(), "lowering errors: {:?}", lower_errors);
    ast
}

// Each test: parse → lower → snapshot
#[test]
fn operator_binary_add_desugars_to_add_contract() {
    let ast = lower_src("impl vec2 { operator +(other: vec2) -> vec2 { vec2(0, 0) } }");
    insta::assert_debug_snapshot!(ast);
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Operators stay as opaque `ImplMember::Op` nodes | Operators desugared to contract impls during lowering (Phase 3) | Phase 3 (now) | Downstream semantic phases only see contract impls — no operator-specific code |
| All impl members in one flat block | Operator members become separate `AstImplDecl` nodes per operator | Phase 3 (now) | One `impl` block with 3 operators emits 3 new `AstDecl::Impl` nodes |

**Deprecated/outdated:**
- `AstImplMember::Op` in the final AST: After Phase 3, this variant should never appear in the output of `lower()`. It may still exist as a CST-side type but should be a compiler error (from exhaustive downstream match) if it survives into the lowered AST.

---

## Open Questions

1. **Should the base impl (Fn-only members) be emitted when it has zero members?**
   - What we know: An `impl vec2 { operator +(...) { ... } }` with no Fn members would produce a `AstDecl::Impl { contract: None, target: vec2, members: [] }` as the "base" alongside the `Add` impl.
   - What's unclear: Whether downstream phases care about empty impl blocks.
   - Recommendation: Skip the empty base impl (no contract, no fn members) — emit nothing for it. This matches the spec semantics where `impl vec2 { operator +(...) }` means only the `Add` contract is being implemented.

2. **What parameter-shape should the generated contract type have for `Eq` and `Ord`?**
   - What we know: Spec §17.2 says `Eq` and `Ord` have one "other" parameter. The contract type must be `Eq<OtherType>` (single type arg) or `Eq<OtherType, bool>` (with explicit bool return)?
   - What's unclear: The exact generic signature of the built-in `Eq` and `Ord` contracts — the spec shows `impl Eq for Self` with an `operator ==` but doesn't specify the generic parameters.
   - Recommendation: Use the single-type-arg form `Eq<OtherType>` (return type `bool` is implied). Mirror however the standard library contracts are defined. This can be revisited in Phase 6 integration if downstream type-checking requires a different form. For Phase 3, the planner should decide the canonical form and document it.

3. **Does the Writ parser support `spawn detached expr` directly, or is `detached` a keyword that only appears after `spawn`?**
   - What we know: The CST has `Expr::Detached(Box<Spanned<Expr>>)` as a standalone variant, not nested inside `Expr::Spawn`. This means the parser creates `Spawn(Detached(inner_expr))` for `spawn detached expr`.
   - What's unclear: Whether `detached expr` (without `spawn`) is valid syntax.
   - Recommendation: The R7 snapshot test should use `spawn detached expr` and verify the nested structure `AstExpr::Spawn { expr: AstExpr::Detached { ... } }`. The planner should confirm whether a standalone `detached` test case is needed.

4. **Are `Shl` (`<<`) and `Shr` (`>>`) operators in scope for Phase 3?**
   - What we know: R6 in `REQUIREMENTS.md` lists `<<` and `>>` in the binary operator list. However, the CST `BinaryOp` enum does not contain `Shl`/`Shr` variants — only `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Eq`, `NotEq`, `Lt`, `Gt`, `LtEq`, `GtEq`, `And`, `Or`, `BitAnd`, `BitOr`. The CST `OpSymbol` also has no `Shl`/`Shr`.
   - What's unclear: Whether `<<` and `>>` are planned but not yet in the parser, or if R6's mention is a spec reference that hasn't been implemented in the CST yet.
   - Recommendation: Do NOT include `Shl`/`Shr` in Phase 3. Match the 10 `OpSymbol` variants that exist in the CST. Flag this discrepancy in the plan's open questions. No `Shl`/`Shr` `OpSymbol` exists to match on.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` + `insta 1` with `ron` feature |
| Config file | None — `insta` detects RON feature from `Cargo.toml` |
| Quick run command | `cargo test -p writ-compiler` |
| Full suite command | `cargo test -p writ-compiler` |
| Snapshot review | `cargo insta review` (after first run) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| R6 | Binary `operator +` → `impl Add<T, T> for Self` | snapshot | `cargo test -p writ-compiler operator_binary_add_desugars_to_add_contract` | ❌ Wave 0 |
| R6 | Binary `operator ==` → `impl Eq<T> for Self` | snapshot | `cargo test -p writ-compiler operator_eq_desugars_to_eq_contract` | ❌ Wave 0 |
| R6 | Binary `operator <` → `impl Ord<T> for Self` + derived `>`, `<=`, `>=` | snapshot | `cargo test -p writ-compiler operator_ord_desugars_with_derived` | ❌ Wave 0 |
| R6 | Unary `operator -()` → `impl Neg<T> for Self` | snapshot | `cargo test -p writ-compiler operator_unary_neg_desugars` | ❌ Wave 0 |
| R6 | Unary `operator !()` → `impl Not<T> for Self` | snapshot | `cargo test -p writ-compiler operator_not_desugars` | ❌ Wave 0 |
| R6 | `operator []` → `impl Index<Idx, Ret> for Self` | snapshot | `cargo test -p writ-compiler operator_index_desugars` | ❌ Wave 0 |
| R6 | `operator []=` → `impl IndexMut<Idx, Val> for Self` | snapshot | `cargo test -p writ-compiler operator_index_mut_desugars` | ❌ Wave 0 |
| R6 | Derived `!=` from `Eq` | snapshot | (covered by `operator_eq_desugars_to_eq_contract` or separate test) | ❌ Wave 0 |
| R6 | Derived `>`, `<=`, `>=` from `Eq`+`Ord` | snapshot | `cargo test -p writ-compiler operator_eq_and_ord_derives_all_four` | ❌ Wave 0 |
| R6 | Impl with mixed Fn + Op members | snapshot | `cargo test -p writ-compiler impl_mixed_fn_and_op_members` | ❌ Wave 0 |
| R7 | `spawn expr` → `AstExpr::Spawn` with span preserved | snapshot | `cargo test -p writ-compiler concurrency_spawn_passthrough` | ❌ Wave 0 |
| R7 | `join handle` → `AstExpr::Join` with span preserved | snapshot | `cargo test -p writ-compiler concurrency_join_passthrough` | ❌ Wave 0 |
| R7 | `cancel handle` → `AstExpr::Cancel` with span preserved | snapshot | `cargo test -p writ-compiler concurrency_cancel_passthrough` | ❌ Wave 0 |
| R7 | `defer { ... }` → `AstExpr::Defer` with block body, span preserved | snapshot | `cargo test -p writ-compiler concurrency_defer_passthrough` | ❌ Wave 0 |
| R7 | `spawn detached expr` → `AstExpr::Spawn { Detached { ... } }` | snapshot | `cargo test -p writ-compiler concurrency_detached_passthrough` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p writ-compiler`
- **Per wave merge:** `cargo test -p writ-compiler`
- **Phase gate:** Full suite green (all 14 existing + new Phase 3 tests) before moving to Phase 4

### Wave 0 Gaps

- [ ] `writ-compiler/src/lower/operator.rs` — new module for `lower_operator_impls`; covers R6
- [ ] New test functions in `writ-compiler/tests/lowering_tests.rs` — covers R6 and R7 snapshots
- [ ] Snapshots in `writ-compiler/tests/snapshots/` — created automatically by `insta` on first run; must be accepted via `cargo insta review`

*(Existing test infrastructure `writ-compiler/tests/lowering_tests.rs` and `cargo test` setup covers everything else — no new framework install needed)*

---

## Sources

### Primary (HIGH confidence)

- `D:/dev/git/Writ/writ-parser/src/cst.rs` — Confirmed `OpSymbol` enum (10 variants: Add, Sub, Mul, Div, Mod, Eq, Lt, Not, Index, IndexSet); `ImplMember::Op(Spanned<OpDecl>)`; `OpDecl` fields; `Expr` concurrency variants (Spawn, Detached, Join, Cancel, Defer) — all confirmed present
- `D:/dev/git/Writ/writ-compiler/src/lower/mod.rs` — Confirmed current `lower_impl` implementation (lines 447–465) passes `ImplMember::Op` through as `AstImplMember::Op` unchanged; confirmed two call sites (top-level loop and `lower_namespace`)
- `D:/dev/git/Writ/writ-compiler/src/lower/expr.rs` — Confirmed concurrency pass-through already implemented (lines 206–234); R7 is snapshot-only work
- `D:/dev/git/Writ/writ-compiler/src/ast/decl.rs` — Confirmed `AstImplDecl.contract: Option<AstType>` and `AstImplMember::Op(AstOpDecl)` exist; `AstOpSymbol` (10 variants)
- `D:/dev/git/Writ/writ-compiler/src/ast/expr.rs` — Confirmed `AstExpr::Spawn`, `Detached`, `Join`, `Cancel`, `Defer` variants; `BinaryOp::Eq`, `BinaryOp::Lt` for derived operator body synthesis; `AstExpr::SelfLit`, `AstExpr::UnaryPrefix { op: PrefixOp::Not }`
- `D:/dev/git/Writ/language-spec/spec/18_17_operators_overloading.md` — §17.2 operator overloading syntax; §17.4 derived operator rules (`!=` from Eq, `>` from Ord, `<=`/`>=` from Eq+Ord) — confirmed verbatim
- `D:/dev/git/Writ/language-spec/spec/21_20_concurrency.md` — §20.2 concurrency primitive table; spawn/join/cancel/defer/detached semantics confirmed
- `D:/dev/git/Writ/.planning/REQUIREMENTS.md` — R6 and R7 acceptance criteria confirmed; note R6 mentions `<<`/`>>` but CST has no `Shl`/`Shr` OpSymbol
- `D:/dev/git/Writ/.planning/STATE.md` — Key decisions confirmed: fold pattern, no wildcards on exhaustive matches, `assert_debug_snapshot` over RON (SimpleSpan lacks Serialize without serde feature)
- `D:/dev/git/Writ/writ-compiler/tests/lowering_tests.rs` — Confirmed `lower_src(&'static str)` helper pattern; `insta::assert_debug_snapshot!` is the snapshot macro in use
- `D:/dev/git/Writ/writ-compiler/Cargo.toml` — Confirmed `insta = { version = "1", features = ["ron"] }` in dev-deps; no new deps needed

### Secondary (MEDIUM confidence)

- `D:/dev/git/Writ/.planning/phases/02-foundational-expression-lowering/02-RESEARCH.md` — Architecture patterns for fold functions, anti-patterns, test structure — all apply unchanged to Phase 3

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all types verified from source files; all relevant code paths read and confirmed
- Architecture: HIGH — CST and AST types read directly; both call sites for `lower_impl` confirmed; concurrency pass-through confirmed already done; operator mapping verified from spec
- Pitfalls: HIGH — binary/unary Sub disambiguation is a concrete discoverable issue from CST inspection; two-call-sites pitfall confirmed from direct code reading; synthetic span rules are established project convention

**Research date:** 2026-02-26
**Valid until:** 2026-08-26 (stable Rust ecosystem; CST/AST types are internal and will only change with intentional refactoring)
