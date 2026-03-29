# Phase 86: LSP Support - Research

**Researched:** 2026-03-24
**Domain:** writ-lsp — dot-completions, hover, and diagnostics for `TyKind::Contract`
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all implementation choices are at Claude's discretion.

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LSP-01 | Dot-completion on contract-typed variables shows contract methods | `build_dot_completions` in `writ-lsp/src/queries/completion.rs` has no `TyKind::Contract` arm — adding it using `type_env.contract_methods` is the fix |
| LSP-02 | Hover on contract-typed variables shows the contract type name | `display_named` already handles `TyKind::Contract(def_id)` correctly; `hover_text_for_def` missing `DefKind::Contract` arm needs adding |
| LSP-03 | Diagnostics for contract-typed code produce no false positives and no missing errors | Diagnostics flow through the compiler pipeline unchanged; the only false-positives came from E0122 (now deleted in Phase 84); no structural changes needed — verified by tests |
</phase_requirements>

---

## Summary

Phase 86 is a targeted LSP fix for the three places where `TyKind::Contract` was left out when the type was introduced in Phase 84.

**Gap 1 (LSP-01 — dot-completion):** `build_dot_completions` in `writ-lsp/src/queries/completion.rs` has a `match interner.kind(receiver_ty)` with arms for `TyKind::Struct`, `TyKind::Class`, `TyKind::Entity`, `TyKind::Enum`, `TyKind::Array`, `TyKind::Option`, `TyKind::Result`, and a catch-all `_ => {}`. There is no `TyKind::Contract` arm. When the user types `myContractVar.`, the receiver type is `TyKind::Contract(def_id)` and the function returns an empty list.

**Gap 2 (LSP-02 — hover on declaration site):** `hover_text_for_def` in `writ-lsp/src/queries/hover.rs` has match arms for `DefKind::Fn`, `DefKind::Enum`, `DefKind::Const`, `DefKind::Struct`, `DefKind::Class`, `DefKind::Entity`, `DefKind::Global`, and `_ => String::new()`. `DefKind::Contract` falls into the default — hovering over a `contract` declaration produces no tooltip. Hovering over a *variable* whose type is a contract works correctly because the `binding_at_offset` priority-1 path in `backend.rs` calls `display_named(binding.ty, ...)`, and `display_named` already handles `TyKind::Contract(def_id)` by looking up the contract name in the def-map (confirmed in `writ-compiler/src/check/ty.rs` line 203).

**Gap 3 (LSP-03 — diagnostics):** No structural gap. E0122 (`ContractAsType` error) was deleted in Phase 84. The compiler's type-checker now accepts contract-typed variables and emits CALL_VIRT correctly (Phase 85). Diagnostics for contract code will work correctly as-is; the requirement is satisfied by verifying that (a) valid contract code compiles clean, and (b) invalid contract code (e.g., assigning a type that does not implement the contract) still produces E0123. This can be covered by a diagnostics regression test in the protocol test suite.

**Primary recommendation:** Three small, well-scoped changes — one in `completion.rs`, one in `hover.rs`, and one test file — are sufficient to satisfy all three requirements.

---

## Standard Stack

No new dependencies required. The phase uses the existing stack.

### Core (no changes)
| Crate | Role |
|-------|------|
| `writ-lsp` | LSP server — the only crate modified |
| `writ-compiler` | Provides `TyKind`, `TypeEnv`, `FnSig`, `DefMap` — read-only from LSP |
| `tower-lsp` | LSP protocol layer |
| `lsp-types` | `CompletionItem`, `CompletionItemKind`, `Hover`, etc. |

---

## Architecture Patterns

### Pattern 1: Adding a `TyKind::Contract` arm to `build_dot_completions`

**What:** Mirror the pattern used for `TyKind::Struct` methods: look up `type_env.contract_methods.get(&def_id)`, then emit one `CompletionItem` per `FnSig` with `kind = METHOD`.

**Where:** `writ-lsp/src/queries/completion.rs`, inside the `match interner.kind(receiver_ty)` block, between the `TyKind::Enum` arm and the `TyKind::Array` arm.

**Data shape (confirmed from `writ-compiler/src/check/env.rs`):**
```
type_env.contract_methods: FxHashMap<DefId, Vec<FnSig>>
```
Each `FnSig` carries `name: String`, `params: Vec<(String, Ty)>`, `ret: Ty`, and `self_param: Option<bool>`.

**Example arm (mirrors Struct method completions):**
```rust
TyKind::Contract(def_id) => {
    if let Some(methods) = type_env.contract_methods.get(def_id) {
        for sig in methods {
            let detail = format_fn_sig_oneliner(sig, interner, def_map);
            items.push(CompletionItem {
                label: sig.name.clone(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(detail),
                ..Default::default()
            });
        }
    }
}
```

The private helper `format_fn_sig_oneliner` is already defined in the same file (line 302).

### Pattern 2: Adding `DefKind::Contract` to `hover_text_for_def`

**What:** Return a tooltip for hovering over the `contract Foo { ... }` declaration name.

**Where:** `writ-lsp/src/queries/hover.rs`, inside the `match entry.kind` block in `hover_text_for_def`, after the `DefKind::Entity` arm and before `DefKind::Global`.

**Example arm:**
```rust
DefKind::Contract => {
    format!("```writ\ncontract {}\n```", entry.name)
}
```

Optionally, list method names for a richer tooltip — but the pattern established by `DefKind::Struct` is to show only `struct Name`, so `contract Name` is the correct minimal form.

### Pattern 3: Diagnostics regression tests in `test_protocol.rs`

**What:** Two new `#[tokio::test]` functions in `writ-lsp/tests/test_protocol.rs`, following the established wire-protocol test pattern.

Test A (LSP-03, no false positives): open a source with valid contract code → assert zero error diagnostics.

Test B (LSP-03, no missing errors): open a source where a concrete type that does NOT implement the contract is assigned to a contract-typed variable → assert at least one error diagnostic.

**Pattern (mirrors existing `test_diagnostics_invalid_source` at line 395):**
```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_diagnostics_contract_valid_no_errors() { ... }

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_diagnostics_contract_invalid_produces_error() { ... }
```

### Pattern 4: Unit tests in `completion.rs` (dot-completion for contract)

Following the `test_dot_completion_integration_struct` pattern (line 1315): use inline source that defines a contract, then exercise the full pipeline:
1. Embed a trailing `.` in the source.
2. Strip it, call `AnalysisHost::analyze_standalone`.
3. `expr_at_offset` → `build_dot_completions`.
4. Assert method names appear in the labels.

### Anti-Patterns to Avoid

- **Modifying the compiler:** All three gaps are in `writ-lsp`. The compiler already provides the right data (`contract_methods`, `TyKind::Contract` in `display_named`). Do not add anything to `writ-compiler`.
- **Hardcoding method names:** The contract's methods live in `type_env.contract_methods` — always read from there, never hardcode.
- **Changing the diagnostics pipeline:** Diagnostics already flow through the existing compiler pipeline. LSP-03 is satisfied by tests, not code changes.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Format method signature for completion detail | Custom formatter | `format_fn_sig_oneliner` (already in `completion.rs`) |
| Resolve contract type name for hover | Custom name lookup | `display_named` in `TyInterner` (already handles `TyKind::Contract`) |
| Collect contract methods | Direct AST traversal | `type_env.contract_methods.get(&def_id)` |

---

## Common Pitfalls

### Pitfall 1: The `def_id` in `TyKind::Contract` is borrowed, not owned

**What goes wrong:** The match arm receives `TyKind::Contract(def_id)` where `def_id` is a `&DefId` reference (the interner returns references). Passing `def_id` directly to `HashMap::get` may require an explicit dereference.

**How to avoid:** Use `type_env.contract_methods.get(def_id)` — Rust's `HashMap::get` accepts `&DefId` via the `Borrow` impl. No dereference needed. Confirm the borrow compiles without error.

**Warning signs:** Compiler error `mismatched types: expected DefId, found &DefId`.

### Pitfall 2: `hover_text_for_def` is only called for declaration sites

**What goes wrong:** The developer adds the `DefKind::Contract` arm but does not verify that hovering over a *variable use* site also works. Variable-use hover goes through `binding_at_offset` (Priority 1 in `backend.rs`) or `hover_text_for_expr` (Priority 3), not through `hover_text_for_def`.

**How to avoid:** Verify `display_named` for `TyKind::Contract` by reading `writ-compiler/src/check/ty.rs` — it is already correct (line 203). Write a unit test in `hover.rs` (or the completion tests module) that checks hover text for a contract-typed binding.

**Warning signs:** The test for LSP-02 passes for declaration-site hover but fails for variable-use hover.

### Pitfall 3: Dot-completion for contracts goes through the stripped-source path

**What goes wrong:** Dot-completion in `backend.rs` strips the `.` from the source, runs a fresh analysis, then calls `expr_at_offset` at `dot_offset - 1`. The receiver expression at that offset must have type `TyKind::Contract`. If the contract variable is declared as `let x: MyContract = ...;` but the type annotation spans are not correct in the stripped source, `expr_at_offset` may return `None` or return the wrong expression.

**How to avoid:** The existing `Var` expression node carries the inferred `ty` field, which is set during type-checking. After stripping the dot and re-analyzing, the `Var` node for `x` will have `ty = TyKind::Contract(def_id)` as long as type-checking ran successfully. Confirm with a full-pipeline integration test (Pattern 4 above).

### Pitfall 4: No `contract_methods` entry for a contract with no methods

**What goes wrong:** If a contract is declared but has no methods, `type_env.contract_methods.get(&def_id)` returns `None`. The arm silently returns zero items, which is correct behavior — but it must not panic.

**How to avoid:** The `if let Some(methods) = ...` guard already handles this correctly. No special casing needed.

---

## Code Examples

### Confirmed: `type_env.contract_methods` type signature
```rust
// Source: writ-compiler/src/check/env.rs line 61
pub contract_methods: FxHashMap<DefId, Vec<FnSig>>,
```

### Confirmed: `FnSig` fields available for display
```rust
// Source: writ-compiler/src/check/env.rs (FnSig struct)
// name: String, params: Vec<(String, Ty)>, ret: Ty, self_param: Option<bool>, generics: Vec<String>
```

### Confirmed: `display_named` already handles `TyKind::Contract`
```rust
// Source: writ-compiler/src/check/ty.rs lines 199-204
TyKind::Struct(def_id)
| TyKind::Class(def_id)
| TyKind::Entity(def_id)
| TyKind::Enum(def_id)
| TyKind::Contract(def_id) => {
    def_map.get_entry(*def_id).name.clone()
}
```

### Confirmed: `build_dot_completions` match exhausts at `_ => {}`
```rust
// Source: writ-compiler/src/queries/completion.rs line 295
_ => {} // No dot completions for primitives, void, func, etc.
```
Add the `TyKind::Contract` arm immediately before this catch-all.

### Confirmed: `hover_text_for_def` default case
```rust
// Source: writ-lsp/src/queries/hover.rs line 280
_ => String::new(),
```
Add the `DefKind::Contract` arm immediately before this catch-all.

### Existing integration test wiring pattern (for new tests)
```rust
// Source: writ-lsp/tests/test_protocol.rs ~line 358
// 1. client.open_document_and_collect_diagnostics(uri, source).await
// 2. Filter notifications by URI
// 3. Assert diagnostics array is empty (or non-empty)
```

---

## State of the Art

| Old State | Current State | Changed In | Impact |
|-----------|---------------|-----------|--------|
| `TyKind::Contract` did not exist | `TyKind::Contract(DefId)` variant added to TyInterner | Phase 84 | LSP must now handle it |
| E0122 blocked contract-typed variables from compiling | E0122 removed; contract types compile correctly | Phase 84 | LSP-03 false-positive source is gone |
| Contract method calls used CALL | Contract method calls emit CALL_VIRT | Phase 85 | IL codegen correct; LSP unaffected |

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in + `tokio::test` for async LSP tests |
| Config file | `writ-lsp/Cargo.toml` — `[[test]] name = "test_protocol"` |
| Quick run | `cargo test -p writ-lsp` |
| Full suite | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | Location |
|--------|----------|-----------|-------------------|---------|
| LSP-01 | Dot-completion on contract-typed var returns contract methods | unit + integration | `cargo test -p writ-lsp test_dot_completions_contract` | `completion.rs` tests module |
| LSP-02 | Hover on contract-typed variable shows contract name | unit | `cargo test -p writ-lsp test_hover_contract` | `hover.rs` tests module |
| LSP-03 | Valid contract code: zero diagnostics | integration | `cargo test -p writ-lsp test_diagnostics_contract_valid` | `test_protocol.rs` |
| LSP-03 | Invalid contract code: produces error diagnostic | integration | `cargo test -p writ-lsp test_diagnostics_contract_invalid` | `test_protocol.rs` |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-lsp`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full workspace green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-lsp/src/queries/completion.rs` — add `test_dot_completions_contract` and `test_dot_completion_integration_contract` to the existing `#[cfg(test)]` module
- [ ] `writ-lsp/src/queries/hover.rs` — add `test_hover_contract_def` to the existing `#[cfg(test)]` module
- [ ] `writ-lsp/tests/test_protocol.rs` — add `test_diagnostics_contract_valid_no_errors` and `test_diagnostics_contract_invalid_produces_error`

All four gap items are new test functions in existing files — no new files or config needed.

---

## Open Questions

None — all relevant code has been directly inspected and the gaps are unambiguous.

---

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — pure in-process Rust code changes with no new tools, services, or runtimes).

---

## Sources

### Primary (HIGH confidence)
- Direct source read: `writ-lsp/src/queries/completion.rs` — confirmed `TyKind::Contract` arm is missing from `build_dot_completions`
- Direct source read: `writ-lsp/src/queries/hover.rs` — confirmed `DefKind::Contract` arm is missing from `hover_text_for_def`; confirmed `display_named` correctly handles `TyKind::Contract`
- Direct source read: `writ-compiler/src/check/env.rs` — confirmed `TypeEnv.contract_methods: FxHashMap<DefId, Vec<FnSig>>`
- Direct source read: `writ-compiler/src/check/ty.rs` — confirmed `display_named` handles `TyKind::Contract`
- Direct source read: `writ-lsp/src/backend.rs` — confirmed hover priority chain; `binding_at_offset` → `hover_text_for_def` → `hover_text_for_expr`
- Direct source read: `writ-lsp/tests/test_protocol.rs` — confirmed test patterns for diagnostics and hover

### Secondary (MEDIUM confidence)
None needed — all claims are from direct code inspection.

---

## Metadata

**Confidence breakdown:**
- LSP-01 gap: HIGH — code directly inspected, missing arm is unambiguous
- LSP-02 gap: HIGH — both missing arm in `hover_text_for_def` and working path via `display_named` confirmed
- LSP-03: HIGH — no code changes needed, only regression tests; E0122 removal confirmed in Phase 84

**Research date:** 2026-03-24
**Valid until:** 2026-04-24 (stable codebase; only invalidated if `TyKind::Contract` or `TypeEnv` are refactored)
