---
phase: quick-260323-vkg
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - writ-diagnostics/src/code.rs
  - writ-compiler/src/check/error.rs
  - writ-compiler/src/check/env_build.rs
  - writ-compiler/src/check/env.rs
  - writ-compiler/src/check/check_decl.rs
  - writ-compiler/src/emit/body/call.rs
  - writ-compiler/tests/typecheck_tests.rs
autonomous: true
requirements: [BUG-CONTRACT-INCOMPLETE-IMPL, BUG-CONTRACT-AS-TYPE, BUG-CLASS-DISPATCH]

must_haves:
  truths:
    - "Incomplete contract impl produces a compile-time error listing missing methods"
    - "Using a contract name as a type annotation produces a clear compile-time error"
    - "Method calls on class-typed receivers dispatch correctly without fallthrough"
    - "The original 5-bug repro script produces exactly 2 compile errors and zero runtime crashes"
  artifacts:
    - path: "writ-compiler/src/check/error.rs"
      provides: "IncompleteContractImpl and ContractAsType error variants"
      contains: "IncompleteContractImpl"
    - path: "writ-diagnostics/src/code.rs"
      provides: "E0122 and E0123 error codes"
      contains: "E0122"
    - path: "writ-compiler/src/check/env.rs"
      provides: "validate_contract_impls method on TypeEnv"
      contains: "validate_contract_impls"
    - path: "writ-compiler/src/emit/body/call.rs"
      provides: "TyKind::Class in analyze_callee match"
      contains: "TyKind::Class"
    - path: "writ-compiler/tests/typecheck_tests.rs"
      provides: "Tests for incomplete impl and contract-as-type errors"
      contains: "incomplete_contract_impl"
  key_links:
    - from: "writ-compiler/src/check/env.rs"
      to: "writ-compiler/src/check/error.rs"
      via: "validate_contract_impls emits IncompleteContractImpl errors"
      pattern: "IncompleteContractImpl"
    - from: "writ-compiler/src/check/env_build.rs"
      to: "writ-compiler/src/check/error.rs"
      via: "def_id_to_ty emits ContractAsType diagnostic"
      pattern: "ContractAsType"
    - from: "writ-compiler/src/emit/body/call.rs"
      to: "CallKind::Direct"
      via: "TyKind::Class arm returns Direct"
      pattern: "TyKind::Class.*Direct"
---

<objective>
Fix 3 compiler bugs that cause silent failures and runtime crashes when using contracts:
1. No compile error for incomplete contract implementations (missing methods silently ignored)
2. Using a contract name as a type annotation silently poisons to Error type (crashes at runtime)
3. TyKind::Class missing from analyze_callee dispatch (method calls on class receivers fall through)

Purpose: These bugs make contracts unusable in practice -- the compiler silently accepts invalid code that crashes at runtime.
Output: 2 new error types, 1 validation pass, 1 dispatch fix, integration tests confirming all 3 bugs are caught at compile time.
</objective>

<execution_context>
@.claude/get-shit-done/workflows/execute-plan.md
@.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@writ-compiler/src/check/error.rs
@writ-compiler/src/check/env.rs
@writ-compiler/src/check/env_build.rs
@writ-compiler/src/check/check_decl.rs
@writ-compiler/src/emit/body/call.rs
@writ-diagnostics/src/code.rs
@writ-compiler/tests/typecheck_tests.rs

<interfaces>
<!-- Key types and contracts the executor needs -->

From writ-compiler/src/check/env.rs:
```rust
pub struct TypeEnv {
    pub contract_methods: FxHashMap<DefId, Vec<FnSig>>,
    pub impl_index: FxHashMap<DefId, Vec<ImplEntry>>,
    // ... other fields
}
```

From writ-compiler/src/check/env.rs (ImplEntry):
```rust
pub struct ImplEntry {
    pub impl_def_id: DefId,
    pub contract_def_id: Option<DefId>,
    pub methods: Vec<(String, FnSig)>,
}
```

From writ-compiler/src/check/env_build.rs:
```rust
fn def_id_to_ty(def_id: DefId, def_map: &DefMap, interner: &mut TyInterner) -> Ty {
    let entry = def_map.get_entry(def_id);
    match entry.kind {
        DefKind::Struct | DefKind::ExternStruct => interner.intern(TyKind::Struct(def_id)),
        DefKind::Class | DefKind::ExternClass => interner.intern(TyKind::Class(def_id)),
        DefKind::Entity => interner.intern(TyKind::Entity(def_id)),
        DefKind::Enum => interner.intern(TyKind::Enum(def_id)),
        _ => interner.error(),  // <-- BUG: DefKind::Contract falls here
    }
}
```

From writ-compiler/src/emit/body/call.rs (analyze_callee, line ~246):
```rust
TyKind::Struct(_) | TyKind::Entity(_) => {
    // Concrete receiver -> EMIT-27: specialize to CALL
    return CallKind::Direct;
}
// NOTE: TyKind::Class(_) is MISSING here
```

From writ-diagnostics/src/code.rs (last assigned codes):
```rust
pub const E0121: &str = "E0121"; // recursive struct has infinite size
// E0122 and E0123 are available
```

From writ-compiler/tests/typecheck_tests.rs:
```rust
fn typecheck_src(src: &'static str) -> (TypedAst, Vec<Diagnostic>) { ... }
fn has_error(diags: &[Diagnostic], code: &str) -> bool { ... }
fn has_no_errors(diags: &[Diagnostic]) -> bool { ... }
fn count_errors(diags: &[Diagnostic], code: &str) -> usize { ... }
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add error types, error codes, and contract-as-type diagnostic</name>
  <files>
    writ-diagnostics/src/code.rs,
    writ-compiler/src/check/error.rs,
    writ-compiler/src/check/env_build.rs
  </files>
  <action>
1. In `writ-diagnostics/src/code.rs`, add two new error codes after E0121:
   - `pub const E0122: &str = "E0122"; // contract used as type annotation`
   - `pub const E0123: &str = "E0123"; // incomplete contract implementation`

2. In `writ-compiler/src/check/error.rs`, add two new variants to `TypeError`:
   ```rust
   ContractAsType {
       contract_name: String,
       span: SimpleSpan,
       file: FileId,
   },
   IncompleteContractImpl {
       ty_name: String,
       contract_name: String,
       missing_methods: Vec<String>,
       span: SimpleSpan,
       file: FileId,
   },
   ```

3. In the `impl From<TypeError> for Diagnostic` block in error.rs, add match arms:
   - `ContractAsType` -> E0122 error: "contract `{contract_name}` cannot be used as a type" with primary span and help text: "contracts are bounds, not types; use as a generic bound: `fn foo<T: {contract_name}>(x: T)`"
   - `IncompleteContractImpl` -> E0123 error: "incomplete implementation of contract `{contract_name}` for `{ty_name}`" with primary span and help text listing the missing method names: "missing methods: {missing_methods.join(", ")}"

4. In `writ-compiler/src/check/env_build.rs`, modify `def_id_to_ty()` to handle `DefKind::Contract` explicitly. Instead of falling through to `interner.error()`, it should STILL return `interner.error()` (to prevent cascading type errors), but this function is not the right place to emit diagnostics since it has no access to diagnostics or spans. Instead, the diagnostic will be emitted in `resolve_ast_type_inner` (which calls `def_id_to_ty` and does have span access). Read `resolve_ast_type_inner` to find where `def_id_to_ty` is called, and BEFORE that call, check if the resolved DefId has `DefKind::Contract`. If so, push a `ContractAsType` diagnostic and return `interner.error()`. This requires passing a `&mut Vec<Diagnostic>` or similar through the resolution chain. ALTERNATIVE simpler approach: In `def_id_to_ty`, add `DefKind::Contract` as an explicit match arm that returns `interner.error()` (document that diagnostics are emitted elsewhere). Then in the type checker's `check_let` or wherever type annotations are resolved, detect the error type + contract name and emit `ContractAsType`. The SIMPLEST approach: change `resolve_ast_type_inner` to check for `DefKind::Contract` before calling `def_id_to_ty`, and return an error. Since `resolve_ast_type_inner` already has access to the def_map but not diagnostics, instead make `def_id_to_ty` return a `Result<Ty, DefKind>` so the caller can detect the Contract case and handle it. Actually, look at how the caller chain works — `resolve_ast_type_inner` -> `def_id_to_ty`. The simplest fix: In `def_id_to_ty`, add `DefKind::Contract => interner.error()` as an explicit arm. Then in `env_build.rs`'s `resolve_ast_type_inner`, after calling `def_id_to_ty`, check if the def_id resolved to a Contract and if so, collect a diagnostic. Since `resolve_ast_type_inner` returns only `Ty`, the diagnostic collection needs to happen at the call site. Read the full `resolve_ast_type_inner` function and its callers to find the best injection point. The goal: when a user writes `let c: MyContract = ...`, the compiler emits E0122 instead of silently poisoning.
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo build -p writ-compiler 2>&1 | tail -5</automated>
  </verify>
  <done>E0122 and E0123 error codes exist, ContractAsType and IncompleteContractImpl variants compile, def_id_to_ty handles DefKind::Contract explicitly, and contract-as-type usage emits E0122</done>
</task>

<task type="auto">
  <name>Task 2: Incomplete contract impl validation, class dispatch fix, and tests</name>
  <files>
    writ-compiler/src/check/env.rs,
    writ-compiler/src/check/check_decl.rs,
    writ-compiler/src/emit/body/call.rs,
    writ-compiler/tests/typecheck_tests.rs
  </files>
  <action>
1. **Incomplete contract impl validation** — Add a `validate_contract_impls` method to `TypeEnv` in `env.rs` (or as a standalone function). This function:
   - Iterates over `self.impl_index` entries
   - For each `ImplEntry` that has a `contract_def_id` (i.e., `impl Contract for Type`):
     - Looks up the contract's required methods from `self.contract_methods[contract_def_id]`
     - Compares method names: for each required contract method, checks if a matching method name exists in `impl_entry.methods`
     - Collects missing method names
     - If any are missing, produces a `TypeError::IncompleteContractImpl` with the type name, contract name, and missing methods list
   - Returns `Vec<TypeError>` (or `Vec<Diagnostic>`)
   - Needs access to `DefMap` to get type/contract names from DefIds (pass as parameter)

   Call this validation at the end of `TypeEnv::build()` in env.rs (after all impls are registered). Collect the returned diagnostics into the `diags` vec that `build()` already returns.

   Note: The span should be the impl block's span (available from the impl's DefEntry via `def_map.get_entry(impl_entry.impl_def_id).span`), and the file should be `def_map.get_entry(impl_entry.impl_def_id).file_id`.

2. **Class dispatch fix** — In `writ-compiler/src/emit/body/call.rs`, function `analyze_callee`, at the match on receiver type (around line 246), add `TyKind::Class(_)` to the existing arm:
   ```rust
   TyKind::Struct(_) | TyKind::Entity(_) | TyKind::Class(_) => {
       return CallKind::Direct;
   }
   ```

3. **Integration tests** — Add to `writ-compiler/tests/typecheck_tests.rs`:

   a. `test_incomplete_contract_impl_error`:
   ```rust
   #[test]
   fn test_incomplete_contract_impl_error() {
       let src = r#"
           pub contract MyContract {
               fn requiredA(self);
               fn requiredB(self);
           }
           pub class MyClass {}
           impl MyContract for MyClass {
               fn requiredA(self) {}
           }
           pub fn main() {}
       "#;
       let (_ast, diags) = typecheck_src(src);
       assert!(has_error(&diags, "E0123"), "expected E0123 for incomplete impl, got: {:?}", diags);
   }
   ```

   b. `test_complete_contract_impl_no_error`:
   ```rust
   #[test]
   fn test_complete_contract_impl_no_error() {
       let src = r#"
           pub contract MyContract {
               fn requiredA(self);
           }
           pub class MyClass {}
           impl MyContract for MyClass {
               fn requiredA(self) {}
           }
           pub fn main() {}
       "#;
       let (_ast, diags) = typecheck_src(src);
       assert!(has_no_errors(&diags), "unexpected errors: {:?}", diags);
   }
   ```

   c. `test_contract_as_type_error`:
   ```rust
   #[test]
   fn test_contract_as_type_error() {
       let src = r#"
           pub contract MyContract {
               fn doThing(self);
           }
           pub class MyClass {}
           impl MyContract for MyClass {
               fn doThing(self) {}
           }
           pub fn main() {
               let c: MyContract = new MyClass{};
           }
       "#;
       let (_ast, diags) = typecheck_src(src);
       assert!(has_error(&diags, "E0122"), "expected E0122 for contract-as-type, got: {:?}", diags);
   }
   ```

   d. `test_class_method_call_no_error` (verifies class dispatch works):
   ```rust
   #[test]
   fn test_class_method_call_no_error() {
       let src = r#"
           pub contract MyContract {
               fn doThing(self);
           }
           pub class MyClass {}
           impl MyContract for MyClass {
               fn doThing(self) {}
           }
           pub fn main() {
               let c = new MyClass{};
               c.doThing();
           }
       "#;
       let (_ast, diags) = typecheck_src(src);
       assert!(has_no_errors(&diags), "unexpected errors: {:?}", diags);
   }
   ```
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test -p writ-compiler --test typecheck_tests -- incomplete_contract_impl contract_as_type class_method_call 2>&1 | tail -20</automated>
  </verify>
  <done>
    - `test_incomplete_contract_impl_error` passes (E0123 emitted for missing methods)
    - `test_complete_contract_impl_no_error` passes (no false positives)
    - `test_contract_as_type_error` passes (E0122 emitted for contract used as type)
    - `test_class_method_call_no_error` passes (class dispatch works correctly)
    - `cargo test -p writ-compiler` has no regressions
  </done>
</task>

</tasks>

<verification>
Run full compiler test suite to confirm no regressions:
```bash
cargo test -p writ-compiler 2>&1 | tail -5
```

Run the original repro script through the compiler and verify it produces compile errors (not runtime crashes):
```bash
# Create temp file with repro, compile, verify errors in output
echo 'pub contract MyContract { fn implementedFunc(self); fn notImplementedFunc(self); } pub class MyClass {} impl MyContract for MyClass { fn implementedFunc(self){} } pub fn main() { let c: MyContract = new MyClass{}; c.implementedFunc(); c.notImplementedFunc(); }' | cargo run -p writ-compiler -- --check /dev/stdin 2>&1
```
The output should contain E0122 (contract as type) and E0123 (incomplete impl), with no runtime crash.
</verification>

<success_criteria>
- E0122 emitted when contract name used as type annotation (`let c: MyContract`)
- E0123 emitted when impl block is missing required contract methods
- TyKind::Class included in analyze_callee dispatch (CallKind::Direct)
- All 4 new tests pass
- Full `cargo test -p writ-compiler` passes with no regressions
- The original 5-bug repro produces compile-time errors instead of runtime crashes
</success_criteria>

<output>
After completion, create `.planning/quick/260323-vkg-fix-contract-typing-dispatch-and-incompl/260323-vkg-SUMMARY.md`
</output>
