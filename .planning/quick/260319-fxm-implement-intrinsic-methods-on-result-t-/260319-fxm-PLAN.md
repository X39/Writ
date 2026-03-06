---
phase: quick-260319-fxm
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - writ-compiler/src/check/check_expr/access.rs
  - writ-compiler/src/emit/body/expr/builtins.rs
  - writ-lsp/src/queries/completion.rs
  - writ-golden/tests/golden_tests.rs
  - writ-golden/tests/golden/result_methods.writ
  - writ-golden/tests/golden/result_methods.writil
  - writ-golden/tests/golden/option_methods.writ
  - writ-golden/tests/golden/option_methods.writil
autonomous: true
requirements: [RESULT-METHODS]

must_haves:
  truths:
    - "Result<T,E>.is_ok() type-checks and returns bool"
    - "Result<T,E>.is_err() type-checks and returns bool"
    - "Result<T,E>.unwrap() type-checks and returns T"
    - "Result<T,E>.unwrap_err() type-checks and returns E"
    - "Option<T>.is_some() type-checks and returns bool"
    - "Option<T>.is_none() type-checks and returns bool"
    - "Option<T>.unwrap() type-checks and returns T"
    - "LSP completions for Result<T,E> include all four methods"
    - "Golden tests compile and produce expected IL for all methods"
  artifacts:
    - path: "writ-compiler/src/check/check_expr/access.rs"
      provides: "Type checker intrinsic method resolution for Option and Result"
      contains: "TyKind::Result"
    - path: "writ-lsp/src/queries/completion.rs"
      provides: "LSP completions for Result<T,E> methods"
      contains: "is_ok"
    - path: "writ-golden/tests/golden/result_methods.writ"
      provides: "Golden test source for Result intrinsic methods"
    - path: "writ-golden/tests/golden/option_methods.writ"
      provides: "Golden test source for Option intrinsic methods"
  key_links:
    - from: "writ-compiler/src/check/check_expr/access.rs"
      to: "writ-compiler/src/emit/body/expr/builtins.rs"
      via: "TypedExpr::Field type used by emitter to select instruction"
      pattern: "TyKind::Result.*is_ok|is_err|unwrap|unwrap_err"
---

<objective>
Implement intrinsic method resolution for Result<T, E> (and fix missing Option method resolution) in the type checker, add LSP completions for Result, fix a spec-compliance bug in the emitter, and add golden tests for all methods.

Purpose: Result<T,E> has IL instructions (IS_OK, IS_ERR, UNWRAP_OK, EXTRACT_ERR) and emitter support, but the type checker rejects method calls with UnknownField errors. Option methods have the same type checker gap. This blocks any Writ code from calling these intrinsic methods.

Output: Working end-to-end pipeline for all Option and Result intrinsic methods, verified by golden tests.
</objective>

<execution_context>
@C:/Users/msili/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/msili/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

Key existing code patterns:

From writ-compiler/src/check/check_expr/access.rs — check_member_access dispatches on TyKind:
- TyKind::Struct/Class/Entity — looks up struct_fields, then impl_index for methods
- TyKind::Enum — looks up impl_index for methods
- `_ =>` catch-all — emits UnknownField error (THIS is where Option/Result methods fail)

From writ-compiler/src/emit/body/expr/builtins.rs — try_emit_builtin_method:
- TyKind::Option: "is_none" -> IsNone, "is_some" -> IsSome, "unwrap" -> Unwrap
- TyKind::Result: "is_err" -> IsErr, "is_ok" -> IsOk, "unwrap_ok" -> UnwrapOk, "unwrap_err"|"extract_err" -> ExtractErr
  NOTE: Emitter uses "unwrap_ok" but spec says method name is "unwrap". Must add "unwrap" alias.

From writ-lsp/src/queries/completion.rs lines 206-219:
- TyKind::Option: completions for is_some, is_none, unwrap
- No TyKind::Result arm exists (falls into `_ => {}`)

From language-spec/spec/47_2_18_writ_runtime_module_contents.md:
- Option<T>: is_some(self)->bool, is_none(self)->bool, unwrap(self)->T
- Result<T,E>: is_ok(self)->bool, is_err(self)->bool, unwrap(self)->T, unwrap_err(self)->E

From writ-module/src/instruction.rs:
- Instruction::IsOk { r_dst: u16, r_result: u16 }
- Instruction::IsErr { r_dst: u16, r_result: u16 }
- Instruction::UnwrapOk { r_dst: u16, r_result: u16 }
- Instruction::ExtractErr { r_dst: u16, r_result: u16 }

From writ-golden/tests/golden_tests.rs:
- Pattern: `#[test] fn test_X() { run_golden_test("X"); }`
- BLESS=1 env var writes expected .writil files
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add intrinsic method resolution in type checker + fix emitter spec compliance</name>
  <files>writ-compiler/src/check/check_expr/access.rs, writ-compiler/src/emit/body/expr/builtins.rs</files>
  <action>
In writ-compiler/src/check/check_expr/access.rs, function `check_member_access`, add two new arms to the `match kind` block BEFORE the `_ =>` catch-all:

1. TyKind::Option(inner_ty) arm:
   Match on `field`:
   - "is_some" | "is_none" -> return TypedExpr::Field with ty = Func { params: [], ret: bool_ty }
   - "unwrap" -> return TypedExpr::Field with ty = Func { params: [], ret: inner_ty }
   - _ -> fall through to UnknownField error (emit error like the catch-all does)

2. TyKind::Result(ok_ty, err_ty) arm:
   Match on `field`:
   - "is_ok" | "is_err" -> return TypedExpr::Field with ty = Func { params: [], ret: bool_ty }
   - "unwrap" -> return TypedExpr::Field with ty = Func { params: [], ret: ok_ty }
   - "unwrap_err" -> return TypedExpr::Field with ty = Func { params: [], ret: err_ty }
   - _ -> fall through to UnknownField error

Use `ctx.interner.bool()` to get the bool type. Use `ctx.interner.func(vec![], ret_ty)` to create function types. Mirror the pattern used for struct method access where TypedExpr::Field is returned with a fn_ty.

Additionally, in writ-compiler/src/emit/body/expr/builtins.rs, add "unwrap" as an alias alongside "unwrap_ok" in the TyKind::Result match arm (line 67-71 area). The spec says the user-facing method name is `unwrap`, not `unwrap_ok`. Change the match arm to:
```
"unwrap" | "unwrap_ok" => {
    let r_dst = emitter.alloc_reg(ty);
    emitter.emit(Instruction::UnwrapOk { r_dst, r_result });
    return Some(r_dst);
}
```

Do NOT modify any existing Option handling in the emitter — it already correctly uses "unwrap".
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo build -p writ-compiler 2>&1 | tail -5</automated>
  </verify>
  <done>Type checker resolves Option and Result intrinsic methods to correct function types. Emitter accepts "unwrap" as method name on Result (spec-compliant). `cargo build -p writ-compiler` succeeds.</done>
</task>

<task type="auto">
  <name>Task 2: Add LSP completions for Result + golden tests for Option and Result methods</name>
  <files>writ-lsp/src/queries/completion.rs, writ-golden/tests/golden_tests.rs, writ-golden/tests/golden/result_methods.writ, writ-golden/tests/golden/result_methods.writil, writ-golden/tests/golden/option_methods.writ, writ-golden/tests/golden/option_methods.writil</files>
  <action>
1. In writ-lsp/src/queries/completion.rs, add a TyKind::Result(_, _) arm immediately after the existing TyKind::Option(_) arm (after line 219, before the `_ => {}` arm):

```rust
TyKind::Result(_, _) => {
    for (name, detail) in [
        ("is_ok", "fn is_ok() -> bool"),
        ("is_err", "fn is_err() -> bool"),
        ("unwrap", "fn unwrap() -> T"),
        ("unwrap_err", "fn unwrap_err() -> E"),
    ] {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }
}
```

2. Create writ-golden/tests/golden/option_methods.writ:
```writ
fn main() {
    let opt: int? = Option::Some(42);
    let a: bool = opt.is_some();
    let b: bool = opt.is_none();
    let c: int = opt.unwrap();
}
```

3. Create writ-golden/tests/golden/result_methods.writ:
```writ
fn main() {
    let res: Result<int, string> = Ok(42);
    let a: bool = res.is_ok();
    let b: bool = res.is_err();
    let c: int = res.unwrap();
    let res2: Result<int, string> = Err("bad");
    let d: string = res2.unwrap_err();
}
```

4. Add test functions to writ-golden/tests/golden_tests.rs. Add these right after the `test_adv_option_match` function (before Section K):

```rust
/// Golden test: Option<T> intrinsic methods (is_some, is_none, unwrap).
///
/// Locks that Option intrinsic method calls type-check correctly and emit
/// the expected IS_SOME, IS_NONE, UNWRAP instructions.
#[test]
fn test_option_methods() {
    run_golden_test("option_methods");
}

/// Golden test: Result<T, E> intrinsic methods (is_ok, is_err, unwrap, unwrap_err).
///
/// Locks that Result intrinsic method calls type-check correctly and emit
/// the expected IS_OK, IS_ERR, UNWRAP_OK, EXTRACT_ERR instructions.
#[test]
fn test_result_methods() {
    run_golden_test("result_methods");
}
```

5. Bless the golden files by running with BLESS=1:
```
BLESS=1 cargo test -p writ-golden -- test_option_methods test_result_methods
```
This will create the .writil files. Verify the generated IL contains the expected instructions:
- option_methods.writil should contain IS_SOME, IS_NONE, UNWRAP
- result_methods.writil should contain IS_OK, IS_ERR, UNWRAP_OK, EXTRACT_ERR, WRAP_OK, WRAP_ERR

6. Run the full test suite to confirm nothing is broken:
```
cargo test -p writ-golden
cargo test -p writ-lsp
```
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test -p writ-golden -- test_option_methods test_result_methods 2>&1 | tail -10 && cargo test -p writ-lsp 2>&1 | tail -5</automated>
  </verify>
  <done>LSP completions include all four Result methods. Golden tests for option_methods and result_methods pass. Full `cargo test -p writ-golden` and `cargo test -p writ-lsp` pass with no regressions.</done>
</task>

</tasks>

<verification>
Run full test suite to catch regressions:
```
cargo test -p writ-compiler -p writ-golden -p writ-lsp
```

Verify IL output manually: the blessed .writil files should contain:
- option_methods.writil: IS_SOME, IS_NONE, UNWRAP instructions
- result_methods.writil: IS_OK, IS_ERR, UNWRAP_OK, EXTRACT_ERR instructions
</verification>

<success_criteria>
- `cargo test -p writ-compiler` passes (type checker changes compile)
- `cargo test -p writ-golden` passes (all golden tests including new ones)
- `cargo test -p writ-lsp` passes (completions work)
- result_methods.writil contains IS_OK, IS_ERR, UNWRAP_OK, EXTRACT_ERR
- option_methods.writil contains IS_SOME, IS_NONE, UNWRAP
- No regressions in existing tests
</success_criteria>

<output>
After completion, create `.planning/quick/260319-fxm-implement-intrinsic-methods-on-result-t-/260319-fxm-SUMMARY.md`
</output>
