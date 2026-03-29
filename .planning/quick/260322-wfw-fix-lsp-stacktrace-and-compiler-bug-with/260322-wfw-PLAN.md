---
phase: quick-260322-wfw
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - writ-compiler/src/check/check_stmt.rs
  - writ-compiler/src/emit/body/stmt.rs
  - writ-lsp/src/analysis_host.rs
  - writ-golden/tests/golden/ctrl_for_range.writ
  - writ-golden/tests/golden/ctrl_for_range.writil
autonomous: true
requirements: [FOR-RANGE-LOOP, LSP-GRACEFUL-ERRORS]

must_haves:
  truths:
    - "for i in 2..n iterates i from 2 to n-1 (exclusive range)"
    - "for i in 1..=5 iterates i from 1 to 5 (inclusive range)"
    - "for i in [2..n] treats the array-wrapped range as a 1-element array (no crash)"
    - "The LSP shows no runtime stacktrace for for-range loops"
    - "Existing for-array loops continue to work"
  artifacts:
    - path: "writ-compiler/src/emit/body/stmt.rs"
      provides: "Range iteration emission in emit_for_loop"
    - path: "writ-compiler/src/check/check_stmt.rs"
      provides: "Type checker support for Range iterables in for loops"
    - path: "writ-golden/tests/golden/ctrl_for_range.writ"
      provides: "Golden test for for-range loop compilation"
  key_links:
    - from: "writ-compiler/src/check/check_stmt.rs"
      to: "writ-compiler/src/check/ir.rs"
      via: "TypedExpr::Range detection for binding type derivation"
      pattern: "TypedExpr::Range"
    - from: "writ-compiler/src/emit/body/stmt.rs"
      to: "writ-compiler/src/check/ir.rs"
      via: "Pattern match on TypedExpr::Range in emit_for_loop"
      pattern: "TypedExpr::Range.*start.*end.*inclusive"
---

<objective>
Fix for-range loop compilation so `for i in 2..n` and `for i in 1..=5` work correctly.

Purpose: The language spec (Section 1.6.8) defines range iteration in for loops, but
the compiler currently does not implement it. The type checker assigns error type to
range iterables (only Array is handled), and the emitter falls through to a Nop.
This causes the LSP to show a confusing runtime crash stacktrace because the Range
struct construction uses type_idx=0 (unregistered Range TypeRef) which crashes the VM.

Output:
1. Type checker recognizes Range expressions as valid for-loop iterables
2. Emitter generates counter-based loop code for range iteration (no Range struct needed)
3. Golden test for for-range loops
4. LSP no longer shows runtime crash for for-range scripts
</objective>

<execution_context>
@C:/Users/msili/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/msili/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

Key interfaces:

From writ-compiler/src/check/ir.rs:
```rust
pub enum TypedExpr {
    // ...
    Range {
        ty: Ty,           // Currently Ty(0) = int (simplification)
        span: SimpleSpan,
        start: Option<Box<TypedExpr>>,
        end: Option<Box<TypedExpr>>,
        inclusive: bool,
    },
    // ...
}

pub enum TypedStmt {
    // ...
    For {
        binding: String,
        binding_span: SimpleSpan,
        binding_ty: Ty,
        mutable: bool,
        iterable: TypedExpr,
        body: Vec<TypedStmt>,
        span: SimpleSpan,
    },
    // ...
}
```

From writ-compiler/src/check/ty.rs:
```rust
pub enum TyKind {
    Int, Float, Bool, String, Void,
    Struct(DefId), Class(DefId), Entity(DefId), Enum(DefId),
    Array(Ty), Func { params: Vec<Ty>, ret: Ty },
    Option(Ty), Result(Ty, Ty), TaskHandle(Ty),
    GenericParam(u32), Infer(InferVar), Error,
}
// NOTE: No TyKind::Range variant exists. Range expressions have ty=int.
```

From writ-compiler/src/check/check_stmt.rs (for-loop handling, lines 161-194):
```rust
AstStmt::For { binding, binding_span, iterable, body, span } => {
    let typed_iterable = check_expr(ctx, iterable);
    let elem_ty = match ctx.interner.kind(typed_iterable.ty()).clone() {
        TyKind::Array(elem) => elem,
        _ => ctx.interner.error(),  // <-- Range falls here, silent error
    };
    // ... creates TypedStmt::For with binding_ty: elem_ty
}
```

From writ-compiler/src/emit/body/stmt.rs (emit_for_loop, lines 176-247):
```rust
fn emit_for_loop(...) {
    match emitter.interner.kind(iter_ty).clone() {
        TyKind::Array(_elem_ty) => { /* array counter loop */ }
        _ => {
            // Non-array iterables: emit Nop (future: Range iteration)
            let _ = emit_expr(emitter, iterable);
            emitter.emit(Instruction::Nop);
        }
    }
}
```

From writ-module/src/instruction.rs (relevant instructions):
```rust
LoadInt { r_dst: u16, value: i64 }
CmpLtI { r_dst: u16, r_a: u16, r_b: u16 }
CmpLeI { r_dst: u16, r_a: u16, r_b: u16 }  // May not exist -- check
BrFalse { r_cond: u16, offset: i32 }
Br { offset: i32 }
AddI { r_dst: u16, r_a: u16, r_b: u16 }
```

Spec reference -- Section 1.6.8 "Range in For Loops":
```
for i in 0..5 { }    // i = 0, 1, 2, 3, 4 (exclusive)
for i in 1..=5 { }   // i = 1, 2, 3, 4, 5 (inclusive)
```
</context>

<tasks>

<task type="auto">
  <name>Task 1: Fix type checker and emitter to support for-range iteration</name>
  <files>writ-compiler/src/check/check_stmt.rs, writ-compiler/src/emit/body/stmt.rs</files>
  <action>
**1a. Fix type checker in `check_stmt.rs` (for-loop handler, around line 168):**

The current code derives `elem_ty` only from `TyKind::Array(elem)`, falling through to
error type for everything else. Add detection for when the iterable expression is a
`TypedExpr::Range`: in that case, the binding type should be `int` (range elements are
always integers in Writ).

Change the for-loop handler (lines 168-172) from:
```rust
let elem_ty = match ctx.interner.kind(typed_iterable.ty()).clone() {
    TyKind::Array(elem) => elem,
    _ => ctx.interner.error(),
};
```

To:
```rust
let elem_ty = match ctx.interner.kind(typed_iterable.ty()).clone() {
    TyKind::Array(elem) => elem,
    _ => {
        // Check if the iterable is a Range expression -- ranges iterate as int
        if matches!(&typed_iterable, crate::check::ir::TypedExpr::Range { .. }) {
            ctx.interner.int()
        } else {
            ctx.interner.error()
        }
    }
};
```

This requires importing TypedExpr or using the full path. The module already imports
`super::ir::TypedStmt` -- add `TypedExpr` to the imports from `super::check_expr`
or reference it via `crate::check::ir::TypedExpr`.

**1b. Implement range iteration in the emitter `emit_for_loop` in `stmt.rs`:**

In `emit_for_loop`, the match on `emitter.interner.kind(iter_ty)` will not work for
ranges because Range has type `int` (same as TyKind::Int). Instead, check whether the
`iterable` expression is a `TypedExpr::Range` BEFORE the type-based match. This is the
correct approach because:
- TyKind has no Range variant
- Range expressions have ty=int
- We need the start/end/inclusive fields from the expression, not just the type

Restructure `emit_for_loop` to:

```rust
fn emit_for_loop(
    emitter: &mut super::BodyEmitter<'_>,
    binding: &str,
    binding_ty: crate::check::ty::Ty,
    iterable: &crate::check::ir::TypedExpr,
    body: &[crate::check::ir::TypedStmt],
) {
    use crate::check::ir::TypedExpr;

    // Check for Range iterable FIRST (before type-based dispatch)
    if let TypedExpr::Range { start, end, inclusive, .. } = iterable {
        emit_for_range(emitter, binding, binding_ty, start.as_deref(), end.as_deref(), *inclusive, body);
        return;
    }

    let iter_ty = iterable.ty();
    let int_ty = crate::check::ty::Ty(0);
    let bool_ty = crate::check::ty::Ty(2);

    match emitter.interner.kind(iter_ty).clone() {
        TyKind::Array(_elem_ty) => {
            // ... existing array iteration code (unchanged) ...
        }
        _ => {
            // Non-array, non-range iterables: emit Nop (future: iterator protocol)
            let _ = emit_expr(emitter, iterable);
            emitter.emit(Instruction::Nop);
        }
    }
}
```

**1c. Add `emit_for_range` helper in `stmt.rs`:**

```rust
/// Emit a for loop over a range expression.
///
/// Pattern for exclusive range (start..end):
/// ```text
/// r_iter  = emit start (or LOAD_INT 0 if None)
/// r_end   = emit end
/// loop_start:
///   r_cond = CMP_LT_I r_iter, r_end     // exclusive: i < end
///   BR_FALSE r_cond, loop_end
///   ... body (binding = r_iter) ...
///   r_one  = LOAD_INT 1
///   r_iter = ADD_I r_iter, r_one
///   BR loop_start
/// loop_end:
/// ```
///
/// For inclusive range (start..=end), use CMP_LE_I instead of CMP_LT_I.
/// If CMP_LE_I does not exist in the instruction set, emulate with:
///   r_end_plus = ADD_I r_end, 1; CMP_LT_I r_iter, r_end_plus
fn emit_for_range(
    emitter: &mut super::BodyEmitter<'_>,
    binding: &str,
    binding_ty: crate::check::ty::Ty,
    start: Option<&crate::check::ir::TypedExpr>,
    end: Option<&crate::check::ir::TypedExpr>,
    inclusive: bool,
    body: &[crate::check::ir::TypedStmt],
) {
    let int_ty = crate::check::ty::Ty(0);
    let bool_ty = crate::check::ty::Ty(2);

    // Emit start value (default to 0 if not specified)
    let r_iter = if let Some(s) = start {
        emit_expr(emitter, s)
    } else {
        let r = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::LoadInt { r_dst: r, value: 0 });
        r
    };

    // Emit end value (if no end, this is an open range -- emit Nop and return)
    let r_end = if let Some(e) = end {
        emit_expr(emitter, e)
    } else {
        // Open-ended range in for loop -- not meaningful, emit Nop
        emitter.emit(Instruction::Nop);
        return;
    };

    // For inclusive ranges, add 1 to end and use CmpLtI (since CmpLeI may not exist)
    // Check if Instruction::CmpLeI exists. Per Phase 78 notes, there is NO CmpLeI in the VM.
    // So for inclusive: r_limit = r_end + 1, then CmpLtI r_iter, r_limit
    let r_limit = if inclusive {
        let r_one = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::LoadInt { r_dst: r_one, value: 1 });
        let r_lim = emitter.alloc_reg(int_ty);
        emitter.emit(Instruction::AddI { r_dst: r_lim, r_a: r_end, r_b: r_one });
        r_lim
    } else {
        r_end
    };

    // Labels
    let loop_start = emitter.new_label();
    let loop_end = emitter.new_label();
    emitter.push_loop(loop_end, loop_start);

    emitter.mark_label_here(loop_start);

    // Condition: r_iter < r_limit
    let r_cond = emitter.alloc_reg(bool_ty);
    emitter.emit(Instruction::CmpLtI { r_dst: r_cond, r_a: r_iter, r_b: r_limit });

    // BrFalse to loop_end
    let brf_idx = emitter.instructions.len();
    emitter.emit(Instruction::BrFalse { r_cond, offset: 0 });
    emitter.add_fixup(brf_idx, loop_end);

    // Bind the iterator register as the loop variable
    emitter.locals.insert(binding.to_string(), r_iter);

    // Emit body
    for stmt in body {
        emit_stmt(emitter, stmt);
    }

    // Increment: r_iter = r_iter + 1
    let r_one = emitter.alloc_reg(int_ty);
    emitter.emit(Instruction::LoadInt { r_dst: r_one, value: 1 });
    emitter.emit(Instruction::AddI { r_dst: r_iter, r_a: r_iter, r_b: r_one });

    // Branch back to loop_start
    let br_idx = emitter.instructions.len();
    emitter.emit(Instruction::Br { offset: 0 });
    emitter.add_fixup(br_idx, loop_start);

    emitter.mark_label_here(loop_end);
    emitter.pop_loop();
}
```

IMPORTANT: Verify that `Instruction::CmpLtI` is the correct variant name by checking
`writ-module/src/instruction.rs`. The disassembly output shows `CMP_LT_I` which maps to
`CmpLtI` in Rust. Per the Phase 78 decision, there is NO `CmpLeI` in the VM, so inclusive
ranges must use the `r_end + 1` with `CmpLtI` approach.

Also verify the `inclusive` field in the checker: currently `check_expr` for Range always
sets `inclusive: false` (line 349 of check_expr/mod.rs). This is a bug -- it should
propagate the `kind` from the AST. Fix it:

In `writ-compiler/src/check/check_expr/mod.rs` line 328-351, change:
```rust
inclusive: false,
```
to:
```rust
inclusive: matches!(kind, crate::ast::expr::RangeKind::Inclusive),
```

This is needed so `..=` ranges are properly recognized as inclusive.
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo build -p writ-compiler 2>&1 | tail -5</automated>
  </verify>
  <done>Type checker recognizes Range expressions in for-loops and assigns int binding type. Emitter generates counter-based loop for range iteration. Inclusive ranges use end+1 with CmpLtI. The inclusive flag is correctly propagated from the AST RangeKind.</done>
</task>

<task type="auto">
  <name>Task 2: Add golden test and verify LSP behavior</name>
  <files>writ-golden/tests/golden/ctrl_for_range.writ, writ-golden/tests/golden/ctrl_for_range.writil, writ-compiler/src/check/check_expr/mod.rs, writ-lsp/src/analysis_host.rs</files>
  <action>
**2a. Fix the inclusive flag in check_expr (if not already done in Task 1):**

In `writ-compiler/src/check/check_expr/mod.rs`, the Range handler at line 349 hardcodes
`inclusive: false`. Change to:
```rust
inclusive: matches!(kind, crate::ast::expr::RangeKind::Inclusive),
```

Note: `kind` is `_` in the pattern -- change the pattern from `kind: _` to just `kind`
so it's captured. The full match arm should be:
```rust
AstExpr::Range { start, kind, end, span } => {
```
(remove the `_` on kind). Then use `kind` at line 349.

**2b. Create golden test `ctrl_for_range.writ`:**

```writ
fn main() {
    let mut sum = 0;
    for i in 0..5 {
        sum = sum + i;
    }
}
```

**2c. Generate the expected `.writil` output:**

Compile the golden test and capture the disassembly:
```bash
cargo run -p writ-cli -- compile writ-golden/tests/golden/ctrl_for_range.writ -o /tmp/ctrl_for_range.writc
cargo run -p writ-cli -- disasm /tmp/ctrl_for_range.writc
```

Save the disassembly output as `ctrl_for_range.writil`.

**2d. Run the full test suite to verify no regressions:**

```bash
cargo test -p writ-compiler
cargo test -p writ-golden
cargo test -p writ-lsp
cargo test -p writ-runtime
```

**2e. Verify the original user script works:**

Test both variants:
```bash
# Without brackets
echo 'pub fn main() { let mut a = 0; let mut b = 0; let n = 5; for i in 2..n { let temp = a + b; a = b; b = temp; } }' > /tmp/test_user.writ
cargo run -p writ-cli -- compile /tmp/test_user.writ
cargo run -p writ-cli -- run /tmp/test_user.writc
# Should NOT crash

# With brackets (array of one range)
echo 'pub fn main() { let mut a = 0; let mut b = 0; let n = 5; for i in [2..n] { let temp = a + b; a = b; b = temp; } }' > /tmp/test_user2.writ
cargo run -p writ-cli -- compile /tmp/test_user2.writ
cargo run -p writ-cli -- run /tmp/test_user2.writc
# Array-of-range: still iterates (array has 1 element, the range object)
# This may still crash due to Range struct construction (type_idx=0).
# That is acceptable as a known limitation -- the correct syntax is without brackets.
```

**2f. Add LSP analysis test for for-range (no runtime crash):**

In `writ-lsp/src/analysis_host.rs` tests section, add:

```rust
#[test]
fn test_for_range_no_runtime_crash() {
    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let src = "pub fn main() {\n    let mut sum = 0;\n    for i in 0..5 {\n        sum = sum + i;\n    }\n}";
            let result = AnalysisHost::analyze_standalone(src.to_string(), "test.writ".to_string());
            let errors: Vec<_> = result.diagnostics.iter()
                .filter(|d| d.severity == Severity::Error)
                .collect();
            assert!(errors.is_empty(), "for-range should compile and run cleanly, got: {:?}",
                errors.iter().map(|d| &d.message).collect::<Vec<_>>());
        })
        .unwrap();
    handle.join().unwrap();
}
```
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test -p writ-compiler 2>&1 | tail -3 && cargo test -p writ-golden 2>&1 | tail -3 && cargo test -p writ-lsp -- test_for_range 2>&1 | tail -5</automated>
  </verify>
  <done>Golden test ctrl_for_range passes. The user's original for-range script compiles and runs without crash. LSP shows no runtime error diagnostic for for-range loops. All existing tests pass.</done>
</task>

</tasks>

<verification>
1. `cargo build -p writ-compiler` -- compiles with range iteration support
2. `cargo test -p writ-compiler` -- all compiler tests pass
3. `cargo test -p writ-golden` -- golden tests pass including new ctrl_for_range
4. `cargo test -p writ-lsp` -- LSP tests pass including for-range no-crash test
5. `cargo run -p writ-cli -- compile /tmp/test_for_range.writ && cargo run -p writ-cli -- run /tmp/test_for_range.writc` -- no crash
6. `cargo test -p writ-runtime` -- no regressions
</verification>

<success_criteria>
- `for i in 0..5 { }` iterates i = 0, 1, 2, 3, 4
- `for i in 1..=5 { }` iterates i = 1, 2, 3, 4, 5
- `for i in start..end { }` works with variable bounds
- The LSP does not show a runtime crash stacktrace for for-range loops
- The inclusive flag (`..=` vs `..`) is correctly propagated from parser through checker to emitter
- All existing tests pass (no regressions in array for-loops, golden tests, LSP tests)
</success_criteria>

<output>
After completion, create `.planning/quick/260322-wfw-fix-lsp-stacktrace-and-compiler-bug-with/260322-wfw-SUMMARY.md`
</output>
