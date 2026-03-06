---
phase: quick-260319-gjo
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - writ-compiler/src/check/ir.rs
  - writ-compiler/src/check/desugar.rs
  - writ-compiler/src/emit/body/mod.rs
  - writ-compiler/src/emit/body/expr/mod.rs
  - writ-compiler/src/emit/body/closure.rs
  - writ-compiler/src/emit/collect/walker.rs
autonomous: true
requirements: [FIX-UNWRAP-CRASH]

must_haves:
  truths:
    - "Force-unwrap operator (n!) compiles without E9001 errors"
    - "Force-unwrap emits Crash instruction at IL level for the None/Err arm"
    - "Existing tests continue to pass (no regressions)"
  artifacts:
    - path: "writ-compiler/src/check/ir.rs"
      provides: "TypedExpr::Crash variant"
      contains: "Crash"
    - path: "writ-compiler/src/check/desugar.rs"
      provides: "Unwrap desugaring using Crash instead of Error"
      contains: "TypedExpr::Crash"
    - path: "writ-compiler/src/emit/body/expr/mod.rs"
      provides: "Crash emission as LoadString + Instruction::Crash"
      contains: "TypedExpr::Crash"
  key_links:
    - from: "writ-compiler/src/check/desugar.rs"
      to: "writ-compiler/src/check/ir.rs"
      via: "TypedExpr::Crash variant construction"
      pattern: "TypedExpr::Crash"
    - from: "writ-compiler/src/emit/body/expr/mod.rs"
      to: "writ-module Instruction::Crash"
      via: "emit_expr match arm emitting Crash instruction"
      pattern: "Instruction::Crash"
---

<objective>
Fix force-unwrap operator (n!) compilation bug where TypedExpr::Error used as runtime crash placeholder causes expr_has_error to reject the entire function with E9001.

Purpose: The `!` (force-unwrap) operator on Option/Result types currently fails to compile because the desugared match's crash arm uses `TypedExpr::Error` (a compilation-error marker) instead of a dedicated runtime-crash node. The emitter's error-detection pre-pass sees it and skips the function.

Output: Working `n!` operator that compiles to IL Crash instruction.
</objective>

<execution_context>
@C:/Users/msili/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/msili/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@writ-compiler/src/check/ir.rs (TypedExpr enum — add Crash variant)
@writ-compiler/src/check/desugar.rs (build_unwrap_match — replace Error with Crash)
@writ-compiler/src/emit/body/mod.rs (expr_has_error + collect_lambda_bodies_from_expr — add Crash as leaf)
@writ-compiler/src/emit/body/expr/mod.rs (emit_expr — add Crash emission)
@writ-compiler/src/emit/body/closure.rs (scan_expr_for_lambdas — add Crash to leaf match arm)
@writ-compiler/src/emit/collect/walker.rs (walk_expr — add Crash to leaf match arm)

<interfaces>
<!-- Instruction::Crash already exists in writ-module -->
From writ-module/src/instruction.rs:
```rust
Crash { r_msg: u16 },  // opcode 0x0001, Shape R (4B)
```

<!-- String constant emission pattern (from literal.rs) -->
From writ-compiler/src/emit/body/expr/literal.rs:
```rust
let r_dst = emitter.alloc_reg(ty);
let instr_idx = emitter.instructions.len();
emitter.emit(Instruction::LoadString { r_dst, string_idx: 0 }); // placeholder
emitter.pending_strings.push((instr_idx, s.clone()));
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add TypedExpr::Crash variant and update desugar.rs</name>
  <files>writ-compiler/src/check/ir.rs, writ-compiler/src/check/desugar.rs</files>
  <action>
1. In `writ-compiler/src/check/ir.rs`, add a new `Crash` variant to the `TypedExpr` enum (place it right before `Error`):
```rust
Crash {
    ty: Ty,
    span: SimpleSpan,
    message: String,
},
```
This represents an intentional runtime crash (unwrap failure), NOT a compilation error.

2. Add `TypedExpr::Crash { ty, .. }` to the `ty()` match arm in `impl TypedExpr` (line ~199, alongside the other `| TypedExpr::X { ty, .. }` arms).

3. Add `TypedExpr::Crash { span, .. }` to the `span()` match arm in `impl TypedExpr` (line ~229, alongside the other `| TypedExpr::X { span, .. }` arms).

4. In `writ-compiler/src/check/desugar.rs`, update the import on line 15 to include the `Crash` variant awareness (no explicit import needed since it's the same enum, but ensure TypedExpr is imported).

5. In `build_unwrap_match` (line 241-248), replace:
```rust
body: TypedExpr::Error {
    ty: value_ty,
    span,
},
```
with:
```rust
body: TypedExpr::Crash {
    ty: value_ty,
    span,
    message: "unwrap failed: value is None/Err".into(),
},
```
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo check -p writ-compiler 2>&1 | head -30</automated>
  </verify>
  <done>TypedExpr::Crash variant exists with ty/span/message fields. build_unwrap_match uses Crash instead of Error. Compiler check passes (no missing match arms yet expected from exhaustiveness — that drives Task 2).</done>
</task>

<task type="auto">
  <name>Task 2: Handle TypedExpr::Crash in all emitter pattern matches</name>
  <files>writ-compiler/src/emit/body/mod.rs, writ-compiler/src/emit/body/expr/mod.rs, writ-compiler/src/emit/body/closure.rs, writ-compiler/src/emit/collect/walker.rs</files>
  <action>
**ALL changes are driven by Rust exhaustiveness — every `match expr` on TypedExpr needs a Crash arm. The compiler will tell you if you miss any.**

1. **`writ-compiler/src/emit/body/mod.rs` — `expr_has_error` function (line ~328-331):**
   Add `TypedExpr::Crash { .. }` to the leaf-node match arm that returns false (NOT an error):
   ```rust
   TypedExpr::Literal { .. }
   | TypedExpr::Var { .. }
   | TypedExpr::SelfRef { .. }
   | TypedExpr::Path { .. }
   | TypedExpr::Crash { .. } => {}  // Crash is intentional, not an error
   ```
   This is THE critical fix — it stops expr_has_error from treating Crash as an error.

2. **`writ-compiler/src/emit/body/mod.rs` — `collect_lambda_bodies_from_expr` function (line ~724-728):**
   Add `TypedExpr::Crash { .. }` to the leaf-node match arm:
   ```rust
   TypedExpr::Literal { .. }
   | TypedExpr::Var { .. }
   | TypedExpr::SelfRef { .. }
   | TypedExpr::Path { .. }
   | TypedExpr::Error { .. }
   | TypedExpr::Crash { .. } => {}
   ```

3. **`writ-compiler/src/emit/body/expr/mod.rs` — `emit_expr` function:**
   Add a new match arm for `TypedExpr::Crash` BEFORE the `TypedExpr::Error` arm (around line 221). Emit a LoadString with the crash message, then emit `Instruction::Crash`:
   ```rust
   // ── Crash (intentional runtime panic from unwrap) ─────────────────
   TypedExpr::Crash { ty, message, .. } => {
       // Load crash message as a string constant
       let r_msg = emitter.alloc_reg(Ty(3)); // String type
       let instr_idx = emitter.instructions.len();
       emitter.emit(Instruction::LoadString { r_dst: r_msg, string_idx: 0 }); // placeholder
       emitter.pending_strings.push((instr_idx, message.clone()));
       // Emit crash instruction
       emitter.emit(Instruction::Crash { r_msg });
       // Allocate result register for type continuity (unreachable at runtime)
       emitter.alloc_reg(*ty)
   }
   ```

4. **`writ-compiler/src/emit/body/closure.rs` — `scan_expr_for_lambdas` function (line ~190-194):**
   Add `TypedExpr::Crash { .. }` to the leaf-node match arm:
   ```rust
   TypedExpr::Literal { .. }
   | TypedExpr::Var { .. }
   | TypedExpr::SelfRef { .. }
   | TypedExpr::Path { .. }
   | TypedExpr::Error { .. }
   | TypedExpr::Crash { .. } => {}
   ```

5. **`writ-compiler/src/emit/collect/walker.rs` — `walk_expr` function (line ~106-110):**
   Add `TypedExpr::Crash { .. }` to the leaf-node match arm:
   ```rust
   TypedExpr::Literal { .. }
   | TypedExpr::Var { .. }
   | TypedExpr::SelfRef { .. }
   | TypedExpr::Path { .. }
   | TypedExpr::Error { .. }
   | TypedExpr::Crash { .. } => {}
   ```

6. **Run `cargo check -p writ-compiler` and fix any remaining exhaustiveness errors.** The compiler will flag any other match on TypedExpr that needs updating. Treat Crash as a leaf node (no children to recurse into) in all walkers.
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test 2>&1 | tail -20</automated>
  </verify>
  <done>All exhaustiveness errors resolved. `cargo test` passes. Force-unwrap operator `n!` compiles without E9001 — the Crash node in desugared match is not treated as a compilation error. The emitted IL contains LoadString + Crash instructions for the unwrap-failure path.</done>
</task>

</tasks>

<verification>
1. `cargo check -p writ-compiler` — zero errors (exhaustiveness satisfied)
2. `cargo test` — all existing tests pass
3. Create a test file with the following Writ code and verify it compiles without E9001:
```writ
pub fn main() {
    let n: int? = Some(1);
    let a = n.unwrap();
    let b = n!;
    let c = n.is_none();
    let d = n.is_some();
}
```
</verification>

<success_criteria>
- TypedExpr::Crash variant exists and is used by build_unwrap_match
- expr_has_error does NOT flag Crash as an error
- emit_expr emits LoadString + Instruction::Crash for Crash nodes
- All pattern matches on TypedExpr handle the Crash variant
- cargo test passes with zero failures
- The test Writ code with n! compiles successfully
</success_criteria>

<output>
After completion, create `.planning/quick/260319-gjo-fix-compilation-bug-with-option-intrinsi/260319-gjo-SUMMARY.md`
</output>
