# Phase 115: Generic Constraints - Research

**Researched:** 2026-03-29
**Domain:** Writ compiler — type checker bounds enforcement, IL GenericConstraint table emission
**Confidence:** HIGH

## Summary

Phase 115 completes the generic constraint enforcement pipeline in the Writ compiler. The parsing infrastructure, FnSig.bounds population, and `check_contract_bounds` function are **already implemented** — the phase's work is fixing the gaps that prevent them from working end-to-end: primitive types never satisfy bounds, bound declaration spans are not threaded through the error type (so multi-span diagnostics can't point to both the call site and the bound declaration), and `add_generic_constraint` in `ModuleBuilder` has a TODO that discards the `constraint_def_id` (so table 14 is always empty).

The architecture decision in STATE.md is precise and correct: bounds enforcement belongs ONLY in `check_call`/`check_decl`, never in `env_build`. The existing `check_contract_bounds` is already called in both `check_call_with_sig` and the multi-overload path in `resolve_overloaded_call`. The phase therefore has three well-scoped sub-problems: (1) fix `check_contract_bounds` to handle primitives with explicit `impl Contract for PrimitiveType` impls, (2) add bound declaration span to `UnsatisfiedBound` so `with_secondary` can point to the bound, and (3) wire `add_generic_constraint` to actually store the contract token during finalize.

**Primary recommendation:** Write the failing test first (call a bound-constrained generic fn with a non-implementing type, expect E0103), then fix each of the three gaps, then add the passing tests. This matches the explicit pitfall note in STATE.md.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — discuss phase was skipped per user setting.

### Claude's Discretion
All implementation choices at Claude's discretion. Use ROADMAP phase goal, success criteria, and codebase conventions.

### Deferred Ideas (OUT OF SCOPE)
None — discuss phase skipped.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GEN-01 | User can declare single contract bounds on generic type params (`<T: Eq>`) | Parser already produces `AstGenericParam.bounds`; `build_generic_bounds` already populates `FnSig.bounds`; no new parsing needed |
| GEN-02 | User can declare multiple contract bounds on a single type param (`<T: Eq + Ord>`) | Parser already handles `+` separated bounds in `generic_params.rs`; `FnSig.bounds[i]` is `Vec<DefId>` — the multi-bound structure is already correct |
| GEN-03 | Compiler enforces bounds at call sites — error when passing a type that doesn't implement the required contract | `check_contract_bounds` already called at both call sites; fix: primitive handling (currently hardcoded `false`) and add infer-var-not-resolved handling |
| GEN-04 | Compiler emits generic constraints to IL GenericConstraint table rows | `add_generic_constraint` has a TODO that discards `constraint_def_id`; `finalize()` already has step 9 stub; fix: store constraint DefId in a side-table, resolve to MetadataToken during finalize |
| GEN-05 | Constraint violation errors show multi-span diagnostics (call site + constraint declaration) | `UnsatisfiedBound` currently has only `call_span` + `file`; add `bound_decl_span: SimpleSpan` and `bound_decl_file: FileId`; thread through from `check_contract_bounds` which has access to the FnSig and can look up the bound's AstGenericParam span |
| GEN-06 | Constraint violation errors include fix suggestion ("add `impl Eq for Foo`") | Already implemented in `UnsatisfiedBound` `From<TypeError>`: `with_help(format!("consider adding \`impl {} for {} {{ ... }}\`", ...))` — this is DONE |
</phase_requirements>

---

## Standard Stack

### Core (all project-internal)
| Component | Location | Purpose | Status |
|-----------|----------|---------|--------|
| Parser | `writ-parser/src/parser/generic_params.rs` | Parses `<T: Bound + Other>` into `Vec<AstGenericParam>` with bounds | DONE — no changes needed |
| `FnSig.bounds` | `writ-compiler/src/check/env.rs` | `Vec<Vec<DefId>>` — bounds per generic param | DONE — already populated by `build_generic_bounds` |
| `check_contract_bounds` | `writ-compiler/src/check/check_expr/call.rs:424` | Core enforcement logic | EXISTS — needs primitive fix + span plumbing |
| `UnsatisfiedBound` | `writ-compiler/src/check/error.rs:31` | E0103 error variant | EXISTS — needs `bound_decl_span` and `bound_decl_file` fields |
| `add_generic_constraint` | `writ-compiler/src/emit/module_builder.rs:347` | Adds GenericConstraint row | EXISTS — needs constraint DefId side-table + finalize resolution |
| `collect_fn` | `writ-compiler/src/emit/collect/functions.rs:58` | Emits GenericParam rows for functions | EXISTS — needs follow-up `add_generic_constraint` calls |

---

## Architecture Patterns

### Recommended Project Structure (no new files needed)

All changes are in existing files. The order matters:

```
writ-compiler/src/
├── check/
│   ├── error.rs              -- add bound_decl_span, bound_decl_file to UnsatisfiedBound
│   └── check_expr/
│       └── call.rs           -- fix check_contract_bounds: primitive impl lookup + span threading
├── emit/
│   ├── module_builder.rs     -- fix add_generic_constraint to store DefId, fix finalize step 9
│   └── collect/
│       └── functions.rs      -- add add_generic_constraint calls after GenericParam loop
writ-compiler/tests/
└── typecheck_tests.rs        -- new tests: failing test first, then passing cases
```

### Pattern 1: Primitive Bound Satisfaction

The current code returns `false` for all primitives. The correct approach is to check if there is an `impl ContractName for PrimitiveName` block in the user's source, OR to treat built-in contracts (e.g., `Eq`, `Ord`) as auto-satisfied for primitives.

**Decision to make:** Either (a) require the user to write `impl Eq for int {}` in a prelude file, or (b) auto-satisfy known structural contracts for primitives in `check_contract_bounds`.

For Phase 115 — before collections or writ-std exist — option (b) is the only workable approach. Auto-satisfy means: when the concrete type is a primitive (Int, Float, Bool, String) and the bound contract is one of the well-known structural contracts (Eq, Ord, Hash, etc.), treat it as satisfied. But since Phase 115 must be self-contained and not depend on writ-std, the pragmatic approach is:

**Primitives satisfy a bound if the user has written an `impl ContractName for PrimitiveName` in the current module — AND primitives also satisfy bounds if the bound contract is registered as a builtin contract in the virtual module.**

Actually the simpler and most correct approach: look up `impl_index` for primitives by checking if the bound contract's name matches a known auto-implemented contract for that primitive, OR (Phase 115's actual case) require that `impl Eq for int {}` is present in the test file. The test for GEN-01/GEN-03 success case only needs to call `foo<T: Eq>(a: T, b: T)` with a struct that has `impl Eq for MyStruct {}`. For the failure case, it's called with a struct that doesn't implement Eq.

The STATE.md note says: "Bounds enforcement belongs ONLY in `check_call`/`check_decl`, never in `env_build`." Primitives not satisfying any bound is a real bug for `fn identity<T: Eq>(a: T, b: T) -> bool` called with `identity(1, 2)`. The fix is: in `check_contract_bounds`, when `concrete_def_id` is None (primitive), check the `impl_index` via a synthetic lookup. But primitives do NOT have DefIds in the current type system.

**Actual fix path:** Add a fallback in `check_contract_bounds` that, for primitive types (Int, Float, Bool, String), searches the `impl_index` values (not by DefId key) for any `ImplEntry` whose impl covers that primitive type. However, the current `impl_index` is keyed by `DefId` of the implementing type — primitives have no DefId.

**Pragmatic resolution for Phase 115:** Primitives satisfy bounds if the user has explicitly registered a `impl ContractName for PrimitiveType {}` — but since primitives have no DefId, the compiler cannot currently store such impls. Therefore Phase 115's success criteria only requires that struct/class types (which DO have DefIds) satisfy bounds. The test cases for GEN-01/GEN-02 should use a user-defined struct with an impl block.

This matches the success criteria as written: "call it with a type that implements Eq without error" — a struct implementing Eq satisfies this without needing primitive support.

### Pattern 2: Bound Declaration Span Threading

The `FnSig` does not store spans. The span of the bound declaration (`<T: Eq>`) lives only in the AST `AstFnDecl.generics[i].span` and `.bounds[j]` (which has a span via `AstType`).

Two options for getting the bound span into `check_contract_bounds`:
- (a) Thread it through `FnSig`: add `bound_spans: Vec<Vec<SimpleSpan>>` to `FnSig` — spans parallel to `bounds`
- (b) Look up the AST at check time: pass the `asts` slice to `check_contract_bounds` and re-find the fn decl

Option (a) is the right pattern for this codebase (avoid repeated AST lookups in hot paths, keep data close to where it's needed). Add `bound_spans: Vec<Vec<(SimpleSpan, FileId)>>` to `FnSig` and populate it in `build_fn_sig`.

Actually, looking at the existing code, `FnSig` already has all the necessary info except spans. The `DefEntry` has `file_id` and `name_span`. Adding `bound_spans: Vec<Vec<SimpleSpan>>` to `FnSig` (file is already in `env.build` context) is minimal. Or simply add a `bound_decl_span: SimpleSpan` to `FnSig` at the generic level (the overall `<T: Bound>` span, not per-bound).

The simplest correct approach: add `bound_decl_spans: Vec<SimpleSpan>` to `FnSig`, parallel to `bounds`, each being the span of `AstGenericParam[i].span`. Add `fn_file: FileId` to `FnSig` for cross-file secondary labels. Then update `UnsatisfiedBound` to hold `bound_span: SimpleSpan` and `bound_file: FileId`.

### Pattern 3: GenericConstraint IL Emission

Current `add_generic_constraint` discards the `constraint_def_id`. Fix requires a side-table:

```rust
// In ModuleBuilder (new field):
generic_constraint_contract_ids: Vec<DefId>,  // parallel to generic_constraints

// In add_generic_constraint:
self.generic_constraint_contract_ids.push(constraint_def_id);

// In finalize() step 9:
for (i, row) in self.generic_constraints.iter_mut().enumerate() {
    let contract_def_id = self.generic_constraint_contract_ids[i];
    row.constraint = self.def_token_map
        .get(&contract_def_id)
        .copied()
        .unwrap_or(MetadataToken::NULL);
    // param_row: remap from provisional generic_param index to final 1-based row
    row.param_row = (row.param_row + 1); // already 0-based provisional index
}
```

Also needs: `collect_fn` must call `add_generic_constraint` after `add_generic_param`, iterating `entry`'s bounds. But `collect_fn` doesn't have access to `FnSig.bounds` or the AST bounds — it has `entry.generics` (names only). Therefore `collect_fn` needs the bounds data. Options:
- Pass the `FnSig` or `bounds: &[Vec<DefId>]` as a parameter to `collect_fn`
- Look up the AST `AstFnDecl.generics[i].bounds` in `collect_fn`

The existing pattern in `collect_fn` uses `entry.generics` (names only). The `build_generic_bounds` function in `env_build.rs` shows how to resolve bound names to DefIds from the AST. Since `collect_fn` already has `asts` and `def_map`, it can call the same `build_generic_bounds` logic inline, or a shared helper can be extracted.

**Best approach:** In `collect_fn`, find the `AstFnDecl` (already done via `find_fn_decl`) and iterate `fn_decl.generics[i].bounds` to resolve each bound name to a `DefId` via `def_map.get(name)`, then call `add_generic_constraint(param_idx, contract_def_id)`. This is a self-contained 10-line addition.

### Anti-Patterns to Avoid

- **Checking bounds in `env_build`:** Will cause false errors due to partial `impl_index` during build. Bounds check must run after unification resolves InferVars to concrete types.
- **Checking bounds before `instantiate_generic_fn` and unification:** The InferVars won't be resolved yet. `check_contract_bounds` correctly runs after argument type-checking.
- **Returning early from `check_contract_bounds` when `resolved_ty` is None:** When an InferVar is unresolved (e.g., unbound generic that never gets inferred), silently skipping is correct to avoid false positives. But add a comment explaining this.
- **Assuming `infer_vars.len() == sig.bounds.len()`:** The guard `if i < infer_vars.len()` is already present. Keep it.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Multi-span diagnostics | Custom error formatting | Existing `Diagnostic::with_secondary()` | Already works for E0101, E0107, E0108 |
| Contract impl lookup | Custom contract-check logic | Existing `impl_index` in `TypeEnv` | Already used in `check_contract_bounds` |
| Bound name resolution in emit | Custom name-to-DefId | Existing `def_map.get(name)` | Same pattern as `build_generic_bounds` in env_build |
| GenericParam row lookup | Custom index scan | `generic_params.len() - 1` after `add_generic_param` returns the provisional index | Provisional index is the 0-based Vec index |

---

## Common Pitfalls

### Pitfall 1: Checking bounds during TypeEnv::build (order-dependent false errors)
**What goes wrong:** `impl_index` is only fully populated after all `DefKind::Impl` entries are processed. Checking bounds at `build_fn_sig` time means the impl for a later-declared type may not be in `impl_index` yet.
**Why it happens:** TypeEnv::build walks declarations in source order.
**How to avoid:** STATE.md locks this: bounds enforcement is ONLY in `check_call`/`check_decl`, never in `env_build`.
**Warning signs:** Tests pass when impl is declared before the fn, fail when declared after.

### Pitfall 2: Bounds not checked for the overload-resolution "no match found" fallthrough
**What goes wrong:** In `resolve_overloaded_call`, when no overload matches (falls to index 0 as a "show this error" path), `check_call_with_sig` is called which includes `check_contract_bounds`. This is correct — the bounds check should be included.
**How to avoid:** Already handled — both `check_call_with_sig` and the exact-one-match path in `resolve_overloaded_call` call `check_contract_bounds`.

### Pitfall 3: `add_generic_constraint` called before the corresponding `add_generic_param` returns its index
**What goes wrong:** The provisional `param_index` for the constraint must be the index returned by `add_generic_param`, not assumed from position.
**How to avoid:** Capture the return value of `add_generic_param` and pass it to `add_generic_constraint`.

```rust
// CORRECT:
let param_idx = builder.add_generic_param(TableId::MethodDef, method_handle.0, i as u16, g);
// then for each bound on this param:
for bound_def_id in &bounds[i] {
    builder.add_generic_constraint(param_idx, *bound_def_id);
}
```

### Pitfall 4: finalize() step 9 param_row is 0-based but spec requires 1-based
**What goes wrong:** `GenericConstraintRow.param_row` is a 1-based row index per spec section 2.16.5.
**How to avoid:** During finalize, remap provisional 0-based param indices to 1-based. The finalize step already does this for other tables. Note: the provisional index stored in `generic_constraints[i].param_row` is the 0-based Vec index of `generic_params` — convert to `param_row + 1` (the 1-based index in the final table).

### Pitfall 5: `UnsatisfiedBound` secondary label uses different FileId than primary
**What goes wrong:** STATE.md pitfall for Phase 119 notes "ariadne panics if secondary label references a FileId absent from the renderer's sources slice". Phase 115 adds a secondary label to `UnsatisfiedBound`. If the bound declaration is in a different file from the call site, the renderer must be given both files' sources.
**How to avoid:** For Phase 115, test only same-file bounds. The secondary label must use the correct `bound_file` (from `FnSig` / `DefEntry.file_id`). The existing `render_diagnostics` function must include both files.

---

## Code Examples

### Example 1: Fixing `check_contract_bounds` to thread bound declaration spans

```rust
// In FnSig (env.rs) — add new fields:
pub struct FnSig {
    // ... existing fields ...
    pub bounds: Vec<Vec<DefId>>,
    /// Spans of the generic param declarations, parallel to `bounds`.
    /// Used for secondary labels in UnsatisfiedBound diagnostics.
    pub bound_decl_spans: Vec<SimpleSpan>,
    /// File in which this function is declared (for cross-file secondary labels).
    pub fn_file: FileId,
}
```

```rust
// In build_fn_sig (env_build.rs):
let bound_decl_spans: Vec<SimpleSpan> = fn_decl.generics.iter().map(|gp| gp.span).collect();
FnSig {
    // ... existing fields ...
    bounds,
    bound_decl_spans,
    fn_file: entry.file_id,
}
```

```rust
// In UnsatisfiedBound (error.rs) — add span fields:
UnsatisfiedBound {
    ty_name: String,
    bound_name: String,
    call_span: SimpleSpan,
    file: FileId,
    bound_decl_span: SimpleSpan,   // NEW
    bound_decl_file: FileId,       // NEW
},
```

```rust
// In From<TypeError> for Diagnostic, UnsatisfiedBound arm — add secondary label:
TypeError::UnsatisfiedBound { ty_name, bound_name, call_span, file,
                               bound_decl_span, bound_decl_file } =>
    Diagnostic::error(code::E0103, format!("..."))
        .with_primary(file, call_span, "unsatisfied bound here")
        .with_secondary(bound_decl_file, bound_decl_span, "bound declared here")
        .with_help(format!("consider adding `impl {} for {} {{ ... }}`", bound_name, ty_name))
        .build(),
```

### Example 2: Fixing `add_generic_constraint` in ModuleBuilder

```rust
// New side-table field in ModuleBuilder:
generic_constraint_contract_ids: Vec<DefId>,

// Updated add_generic_constraint:
pub fn add_generic_constraint(&mut self, param_index: usize, constraint_def_id: DefId) -> usize {
    self.generic_constraints.push(GenericConstraintRow {
        param_row: param_index as u32,  // provisional 0-based; remapped in finalize
        constraint: MetadataToken::NULL, // resolved in finalize
    });
    self.generic_constraint_contract_ids.push(constraint_def_id);
    self.generic_constraints.len() - 1
}

// In finalize(), step 9 (replace the stub):
for (i, row) in self.generic_constraints.iter_mut().enumerate() {
    let contract_def_id = self.generic_constraint_contract_ids[i];
    // Remap provisional 0-based param index to 1-based row index
    row.param_row = row.param_row + 1;
    // Resolve contract DefId to MetadataToken
    row.constraint = self.def_token_map
        .get(&contract_def_id)
        .copied()
        .unwrap_or(MetadataToken::NULL);
}
self.final_generic_constraint_count = self.generic_constraints.len() as u32;
```

### Example 3: Emitting GenericConstraint rows in collect_fn

```rust
// In collect_fn (functions.rs), replace the existing GenericParam loop:
if let Some(fn_decl) = find_fn_decl(asts, entry) {
    // ... existing fn sig / param logic ...

    // GenericParam + GenericConstraint
    for (i, (g, ast_gp)) in entry.generics.iter().zip(fn_decl.generics.iter()).enumerate() {
        let param_idx = builder.add_generic_param(TableId::MethodDef, method_handle.0, i as u16, g);
        // Emit GenericConstraint rows for each bound on this param
        for bound_ast_ty in &ast_gp.bounds {
            if let crate::ast::types::AstType::Named { name, .. } = bound_ast_ty {
                if let Some(contract_def_id) = def_map.get(name) {
                    builder.add_generic_constraint(param_idx, contract_def_id);
                }
            }
        }
    }
}
```

### Example 4: Failing test first (required by STATE.md pitfall note)

```rust
// typecheck_tests.rs — write this BEFORE passing tests:
#[test]
fn generic_bound_not_satisfied_emits_e0103() {
    let (_ast, diags) = typecheck_src(
        r#"pub contract Eq { fn eq(other: self) -> bool; }
           pub struct Foo { x: int }
           // NOTE: no impl Eq for Foo
           pub fn check_eq<T: Eq>(a: T, b: T) -> bool { true }
           pub fn test() { check_eq(Foo(x: 1), Foo(x: 2)); }"#,
    );
    assert!(
        has_error(&diags, "E0103"),
        "expected E0103 for unsatisfied Eq bound, got: {:?}", diags
    );
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Bounds parsed but silently ignored | `check_contract_bounds` called at call sites | Already in codebase | Now fails appropriately for named types — primitives still wrongly fail |
| `add_generic_constraint` discards DefId | Fix: side-table stores DefId, finalize resolves | Phase 115 | Table 14 will be populated in emitted binaries |
| `UnsatisfiedBound` single-span | Fix: add bound_decl_span secondary label | Phase 115 | Multi-span error pointing to both call site and bound declaration |

**Already done (no work needed):**
- Parser produces correct `AstGenericParam.bounds` (verified in `generic_params.rs`)
- `build_generic_bounds` correctly maps bound names to `DefId` via `def_map.get()`
- `FnSig.bounds` is correctly populated for all user-defined functions
- `check_contract_bounds` is called in both `check_call_with_sig` and the single-overload path
- `UnsatisfiedBound` already has `with_help` fix suggestion (GEN-06 is essentially done)
- `GenericConstraintRow` struct exists in `metadata.rs` and is serialized in `serialize.rs`

---

## Open Questions

1. **Primitive type bound satisfaction**
   - What we know: primitives have no `DefId`, so `impl_index` lookup by `DefId` key won't find them
   - What's unclear: does Phase 115 need `fn identity<T: Eq>(a: T, b: T)` to work with `identity(1, 2)` (primitive args)?
   - Recommendation: Success criterion says "call it with a type that implements Eq" — a struct with an impl satisfies this. Write tests using structs only. Leave primitive bound satisfaction for Phase 116/117 when writ-std introduces `impl Eq for int {}` in a virtual module. Document this as a known limitation in the phase plan.

2. **`fn_file` field in FnSig or reuse existing info**
   - What we know: `TypeEnv.build` has `entry.file_id` when building each `FnSig`
   - What's unclear: Is it cheaper to add `fn_file: FileId` to `FnSig` or to pass `DefId` through and look it up at check time?
   - Recommendation: Add `fn_file: FileId` to `FnSig` — consistent with how other error types carry `file: FileId` directly. The look-up-at-call-time approach requires passing `def_map` through `check_contract_bounds`.

3. **`entry.generics` vs `fn_decl.generics` in collect_fn**
   - What we know: `entry.generics` is `Vec<String>` (names only); `fn_decl.generics` is `Vec<AstGenericParam>` (names + bound AST types + spans)
   - What's unclear: Does the length always match? Yes — `entry.generics` is built from `fn_decl.generics` during lowering.
   - Recommendation: Zip `entry.generics.iter()` with `fn_decl.generics.iter()` to get both the name and the bound types.

---

## Environment Availability

Step 2.6: SKIPPED — no external tool dependencies. All changes are to existing Rust source files in the Writ compiler workspace. `cargo test` is the only tooling required, and Rust/Cargo are confirmed available from prior phases.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `#[test]` with `cargo test` (no external test framework) |
| Config file | `Cargo.toml` workspace |
| Quick run command | `cargo test -p writ-compiler generic 2>&1` |
| Full suite command | `cargo test -p writ-compiler 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GEN-01 | `fn foo<T: Eq>(a: T, b: T)` accepts type with `impl Eq` — no error | unit | `cargo test -p writ-compiler generic_single_bound_satisfied` | ❌ Wave 0 |
| GEN-02 | `fn foo<T: Eq + Ord>(a: T)` enforces both constraints independently | unit | `cargo test -p writ-compiler generic_multi_bound_both_satisfied` | ❌ Wave 0 |
| GEN-03 | `fn foo<T: Eq>(a: T, b: T)` with non-implementing type emits E0103 | unit | `cargo test -p writ-compiler generic_bound_not_satisfied_emits_e0103` | ❌ Wave 0 |
| GEN-03 | Multi-bound: missing one of two contracts emits E0103 | unit | `cargo test -p writ-compiler generic_multi_bound_missing_one_emits_e0103` | ❌ Wave 0 |
| GEN-04 | Emitted IL binary GenericConstraint table has rows for declared bounds | unit | `cargo test -p writ-compiler emit_generic_constraint_table` | ❌ Wave 0 |
| GEN-05 | E0103 diagnostic has secondary label pointing to bound declaration span | unit | `cargo test -p writ-compiler generic_bound_error_has_secondary_label` | ❌ Wave 0 |
| GEN-06 | E0103 help text says "consider adding `impl Eq for Foo`" | unit | covered by GEN-03 test (assert on diag.help) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-compiler generic`
- **Per wave merge:** `cargo test -p writ-compiler`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-compiler/tests/typecheck_tests.rs` — new test functions for GEN-01..03, GEN-05, GEN-06 (file exists; add test functions)
- [ ] `writ-compiler/tests/emit_tests.rs` — new test for GEN-04 GenericConstraint table rows (file exists; add test function)

*(No new test files needed — all go into existing test files following established patterns)*

---

## Sources

### Primary (HIGH confidence)
- Direct codebase analysis — all findings verified by reading source files
  - `writ-compiler/src/check/check_expr/call.rs` — `check_contract_bounds` implementation
  - `writ-compiler/src/check/env.rs` — `FnSig.bounds` structure
  - `writ-compiler/src/check/env_build.rs` — `build_generic_bounds` implementation
  - `writ-compiler/src/check/error.rs` — `UnsatisfiedBound` variant and conversion
  - `writ-compiler/src/emit/module_builder.rs` — `add_generic_constraint` TODO state
  - `writ-compiler/src/emit/collect/functions.rs` — `collect_fn` GenericParam loop
  - `writ-compiler/src/emit/metadata.rs` — `GenericConstraintRow` structure (table 14)
  - `writ-compiler/src/emit/serialize.rs` — GenericConstraint serialization
  - `writ-diagnostics/src/diagnostic.rs` — `with_secondary` multi-span support
  - `.planning/STATE.md` — architecture decisions and Phase 115 pitfall notes

### Secondary (MEDIUM confidence)
- N/A — all findings from primary source code analysis

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all components verified by direct source reading
- Architecture: HIGH — three clearly scoped fixes, all backed by existing code patterns
- Pitfalls: HIGH — documented in STATE.md and confirmed by reading the actual TODO in `add_generic_constraint`

**Research date:** 2026-03-29
**Valid until:** Stable — this is an internal compiler codebase with no external dependencies
