# Phase 113: LSP Completions Refactor - Research

**Researched:** 2026-03-29
**Domain:** LSP completion pipeline — `writ-lsp/src/queries/completion.rs`
**Confidence:** HIGH

## Summary

The `build_namespace_completions` function in `completion.rs` (lines 715-801) has two hardcoded early-return branches for `"Option"` and `"Result"` namespaces that bypass the `type_env` lookup entirely. These branches exist because `Option` and `Result` are prelude types — they are NOT registered in `DefMap` as `DefKind::Enum` entries; they are structural `TyKind::Option(Ty)` and `TyKind::Result(Ty, Ty)` variants in the type checker, resolved via `LookupResult::PreludeType`. Since there is no `DefId` for them, `type_env.enum_variants` has no entries for `Option` or `Result`.

The fix requires injecting prelude enum variant information into `TypeEnv` during environment build so that `build_namespace_completions` can look up `Option` and `Result` variants the same way it looks up user-defined enum variants. The key design decision is whether to inject synthetic `DefId`s into `DefMap` for `Option`/`Result`, or to add a dedicated side-table in `TypeEnv` for prelude enum variants keyed by name.

The existing user-defined enum path (case 3 in `build_namespace_completions`) already works correctly and is the pattern to extend. The hardcoded branches need to be replaced entirely.

**Primary recommendation:** Add a `prelude_enum_variants: HashMap<&'static str, Vec<&'static str>>` field to `TypeEnv` (populated at build time with `Option → [Some, None]` and `Result → [Ok, Err]`), then consolidate all name-keyed enum completion into a unified lookup path in `build_namespace_completions`.

## Standard Stack

This is a pure refactor within the existing crate. No new dependencies required.

| Crate | Role | Note |
|-------|------|------|
| `writ-compiler` | `TypeEnv`, `TyKind`, prelude definitions | Already a dependency of `writ-lsp` |
| `writ-lsp` | Completion query functions | The change target |

**Installation:** No new packages needed.

## Architecture Patterns

### Current `build_namespace_completions` call site

`backend.rs` lines 562-582: namespace completion triggered by `:` keypress, passes `(namespace, &typed_ast.def_map, type_env)` to `build_namespace_completions`. The `type_env` reference is already available — this is the hook point for the new lookup.

### Current hardcoded branches (the code to replace)

```rust
// writ-lsp/src/queries/completion.rs line 721-748
if namespace == "Option" {
    return vec![
        CompletionItem { label: "Some".to_string(), kind: Some(CompletionItemKind::ENUM_MEMBER), ..Default::default() },
        CompletionItem { label: "None".to_string(), kind: Some(CompletionItemKind::ENUM_MEMBER), ..Default::default() },
    ];
}
if namespace == "Result" {
    return vec![
        CompletionItem { label: "Ok".to_string(), kind: Some(CompletionItemKind::ENUM_MEMBER), ..Default::default() },
        CompletionItem { label: "Err".to_string(), kind: Some(CompletionItemKind::ENUM_MEMBER), ..Default::default() },
    ];
}
```

### Existing user-defined enum path (case 3, keep and generalize)

```rust
// writ-lsp/src/queries/completion.rs lines 776-798
let enum_def_id = def_map.by_fqn.iter().find_map(|(_fqn, &def_id)| {
    let entry = def_map.get_entry(def_id);
    if entry.name == namespace && entry.kind == DefKind::Enum {
        Some(def_id)
    } else {
        None
    }
});
if let Some(def_id) = enum_def_id {
    if let Some(variants) = type_env.enum_variants.get(&def_id) {
        items = variants.iter().map(|v| CompletionItem {
            label: v.name.clone(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            ..Default::default()
        }).collect();
    }
}
```

### Approach A: `prelude_enum_variants` side-table in `TypeEnv`

Add a new field to `TypeEnv` in `writ-compiler/src/check/env.rs`:

```rust
pub prelude_enum_variants: FxHashMap<&'static str, Vec<&'static str>>,
```

Populate in `TypeEnv::build` in `writ-compiler/src/check/env.rs`:

```rust
let mut prelude_ev = FxHashMap::default();
prelude_ev.insert("Option", vec!["Some", "None"]);
prelude_ev.insert("Result", vec!["Ok", "Err"]);
env.prelude_enum_variants = prelude_ev;
```

Then in `build_namespace_completions`, replace the two hardcoded branches with:

```rust
// Check prelude enum variants first (Option, Result)
if let Some(variant_names) = type_env.prelude_enum_variants.get(namespace) {
    return variant_names.iter().map(|&name| CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::ENUM_MEMBER),
        ..Default::default()
    }).collect();
}
```

### Approach B: Extend user-defined path to also scan prelude (not recommended)

Requires injecting synthetic `DefId` entries into `DefMap` for `Option`/`Result` — more invasive, affects resolver invariants, higher regression risk.

### Recommended approach

Use Approach A. It is the minimal targeted change: one new field on `TypeEnv`, populated at build time, consumed by the completion query function. The user-defined enum path (case 3) continues to work for user-defined enums without modification. Adding a user-defined enum to a file already works via case 3 — this refactor only concerns the prelude types.

### Project Structure (files to modify)

```
writ-compiler/src/check/env.rs          # Add prelude_enum_variants field; populate in TypeEnv::build
writ-lsp/src/queries/completion.rs      # Replace two hardcoded branches with prelude_enum_variants lookup
                                        # Update unit tests: test_namespace_completions_option,
                                        # test_namespace_completions_result (remove empty TypeEnv pattern)
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Injecting `Option`/`Result` variant info | Custom DefMap entries, ad-hoc resolver changes | New `prelude_enum_variants` field on `TypeEnv` |
| Mapping prelude type name to variants | String matching at completion time | Pre-computed map in `TypeEnv::build` |

## Common Pitfalls

### Pitfall 1: Existing unit tests use empty TypeEnv literals

**What goes wrong:** `test_namespace_completions_option` and `test_namespace_completions_result` construct a `TypeEnv` by listing all fields explicitly with `Default::default()` values. After adding `prelude_enum_variants`, these struct literal constructors will fail to compile unless the new field is included.

**How to avoid:** Update both tests to either populate the new field, or use a `TypeEnv::default()` / `TypeEnv::empty()` constructor if one is added.

**Warning signs:** `error[E0063]: missing field prelude_enum_variants in initializer of TypeEnv`

### Pitfall 2: Test assertions break after removing hardcoded path

**What goes wrong:** The existing tests `test_namespace_completions_option` and `test_namespace_completions_result` pass with a fresh empty `TypeEnv`. After the refactor, they need a populated `type_env` (with `prelude_enum_variants` set) to pass.

**How to avoid:** Either (a) populate `prelude_enum_variants` directly in the test, or (b) add a `TypeEnv::with_prelude()` constructor, or (c) run the full pipeline (`build_typed_ast_full`) and use the resulting `type_env`. Option (a) is simplest.

### Pitfall 3: Case 2 (DefMap prefix scan) shadows user-defined enums named `Option` or `Result`

**What goes wrong:** Case 2 in `build_namespace_completions` does a `by_fqn` prefix scan for `"{namespace}::"`. If a user defines `pub enum Option { ... }`, it would have an FQN like `"Option"` (no namespace prefix), so case 2 wouldn't fire. But be aware of ordering: the prelude lookup (new case 1) runs before case 3 (user enum lookup), so a user-defined `Option` enum would be shadowed by the prelude path.

**How to avoid:** Check the prelude path AFTER the user-defined enum lookup (case 3 first), or document that the prelude takes priority. Given that `Option` cannot legally be shadowed (the resolver rejects this), the ordering doesn't matter in practice.

### Pitfall 4: `TypeEnv` field initialization in tests scattered across the codebase

**What goes wrong:** Multiple test helpers construct `TypeEnv` structs directly. Search for ALL such sites before changing the struct definition.

**How to avoid:** Run `grep -r "TypeEnv {" --include="*.rs"` to find all direct struct literal sites before adding the field. There are at least 4 in `completion.rs` tests.

## Code Examples

### Verified current TypeEnv struct definition

```rust
// writ-compiler/src/check/env.rs lines 55-64
pub struct TypeEnv {
    pub fn_sigs: FxHashMap<DefId, FnSig>,
    pub struct_fields: FxHashMap<DefId, Vec<(String, Ty, SimpleSpan)>>,
    pub entity_fields: FxHashMap<DefId, Vec<(String, Ty, SimpleSpan)>>,
    pub entity_components: FxHashMap<DefId, Vec<String>>,
    pub enum_variants: FxHashMap<DefId, Vec<EnumVariantSig>>,
    pub contract_methods: FxHashMap<DefId, Vec<FnSig>>,
    pub impl_index: FxHashMap<DefId, Vec<ImplEntry>>,
    pub const_types: FxHashMap<DefId, Ty>,
    pub global_types: FxHashMap<DefId, (Ty, bool)>,
    pub component_fields: FxHashMap<DefId, Vec<(String, Ty, SimpleSpan)>>,
    // (+ deprecated_items, conditional_fns, fallback_for_conditional from tests)
}
```

### Prelude type definitions (source of truth for variant names)

```rust
// writ-compiler/src/resolve/prelude.rs line 10
pub const PRELUDE_TYPE_NAMES: &[&str] = &["Option", "Result", "Range", "Array", "Entity"];

// writ-compiler/src/resolve/prelude.rs line 20
pub const SUB_PRELUDE_VARIANT_NAMES: &[&str] = &["None", "Some"];
```

Note: `Ok` and `Err` are NOT in `SUB_PRELUDE_VARIANT_NAMES`. They come from `Result` usage conventions.

### Existing direct TypeEnv literal construction in completion.rs tests (must update)

Lines 1199-1213 and 1225-1239 and 1262-1276 and 1277: four sites in `completion.rs` test module construct `TypeEnv` with all fields listed. All four need the `prelude_enum_variants` field added after the struct change.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) |
| Config file | none |
| Quick run command | `cargo test -p writ-lsp -- completion 2>&1` |
| Full suite command | `cargo test -p writ-lsp 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LSP-01 | Option:: completions sourced from type_env | unit | `cargo test -p writ-lsp -- test_namespace_completions_option` | Yes (modify) |
| LSP-01 | Result:: completions sourced from type_env | unit | `cargo test -p writ-lsp -- test_namespace_completions_result` | Yes (modify) |
| LSP-01 | User-defined enum variants appear in :: completions | unit | `cargo test -p writ-lsp -- test_namespace_completions_user_enum` | Yes (keep) |
| LSP-01 | Existing completions pass after refactor | integration | `cargo test -p writ-lsp` | Yes |

### Sampling Rate

- **Per task commit:** `cargo test -p writ-lsp -- completion 2>&1`
- **Per wave merge:** `cargo test -p writ-lsp 2>&1`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

None — existing test infrastructure covers all phase requirements. Tests `test_namespace_completions_option` and `test_namespace_completions_result` need modification (not creation) to verify the new type_env-driven path.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LSP-01 | Option/Result namespace completions driven by type_env, not hardcoded | Approach A: add `prelude_enum_variants` field to TypeEnv; replace 2 hardcoded branches in `build_namespace_completions` |

</phase_requirements>

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

None — all implementation choices are at Claude's discretion.

### Claude's Discretion

All implementation choices — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Deferred Ideas (OUT OF SCOPE)

None.
</user_constraints>

## Open Questions

1. **Additional fields on TypeEnv beyond the compiler-visible ones**
   - What we know: The test helpers construct `TypeEnv` with fields including `deprecated_items`, `conditional_fns`, `fallback_for_conditional` (visible at lines 1210-1212 in completion.rs tests). These are not shown in the first 65 lines of `env.rs` read above.
   - What's unclear: Whether there are additional fields that would also need updating when adding `prelude_enum_variants`.
   - Recommendation: Before writing the plan, confirm all `TypeEnv` fields by reading `env.rs` lines 55-90 fully (already done — fields visible at lines 55-64 plus implicit fields inferred from test literals). The plan task for `env.rs` should explicitly list all fields including `deprecated_items`, `conditional_fns`, and `fallback_for_conditional` to ensure the new field is added in the right place.

2. **Whether `Ok`/`Err` are truly not in any prelude constant**
   - What we know: `SUB_PRELUDE_VARIANT_NAMES` only contains `["None", "Some"]`. The resolver comment at line 87-88 says prelude types are "valid but vacuous" for `using` imports.
   - What's unclear: Whether there's a separate constant for `Result` variants.
   - Recommendation: The implementation should use inline string literals `"Ok"` and `"Err"` populated from domain knowledge, or introduce a new `SUB_PRELUDE_RESULT_VARIANT_NAMES` constant in `prelude.rs`. Either is acceptable; using a constant improves traceability.

## Environment Availability

Step 2.6: SKIPPED — phase is purely code/config changes within the existing Rust workspace. No external tool dependencies.

## Sources

### Primary (HIGH confidence)

- Direct code reading: `writ-lsp/src/queries/completion.rs` — full `build_namespace_completions` function (lines 709-801), all existing tests (lines 1178-1279)
- Direct code reading: `writ-compiler/src/check/env.rs` — `TypeEnv` struct definition and `enum_variants` population
- Direct code reading: `writ-compiler/src/resolve/prelude.rs` — `PRELUDE_TYPE_NAMES`, `SUB_PRELUDE_VARIANT_NAMES`
- Direct code reading: `writ-lsp/src/backend.rs` — namespace completion call site (lines 550-584)
- Direct code reading: `writ-compiler/src/resolve/resolver.rs` — confirms `Option`/`Result` resolve to `LookupResult::PreludeType`, not `DefId`

### Secondary (MEDIUM confidence)

- `.planning/MILESTONES.md` line 126: "Hardcoded Option/Result variants in namespace completions (not in type_env.enum_variants)" — confirms the debt entry description

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — single crate, no external dependencies, all findings from direct source reading
- Architecture: HIGH — full function bodies read, test patterns confirmed
- Pitfalls: HIGH — TypeEnv literal tests identified by direct count (4 sites)

**Research date:** 2026-03-29
**Valid until:** 2026-04-28 (stable code, no external dependencies)
