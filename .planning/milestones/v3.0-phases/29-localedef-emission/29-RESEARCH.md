# Phase 29: LocaleDef Emission - Research

**Researched:** 2026-03-03
**Domain:** Rust compiler internals — metadata table emission, lowering pipeline data flow
**Confidence:** HIGH

## Summary

Phase 29 closes the last open requirement in v3.0: EMIT-25 (LocaleDef table emission). The `LocaleDef` table (table 18) records structural locale overrides — mappings from a default `dlg` MethodDef to a locale-specific replacement MethodDef. The table has already been wired end-to-end in `module_builder.rs` (`locale_defs: Vec<LocaleDefRow>`, `add_locale_def()`), `serialize.rs` (table 18 serialization loop), and `metadata.rs` (`LocaleDefRow` struct). The only missing piece is the collection logic: `collect.rs` has a TODO stub at line 95-96 where `collect_locale_defs()` should be called.

The central design challenge is a **data flow gap**: locale key information is produced during lowering (in `lower/dialogue.rs`), but the current `lower()` return type is `(Ast, Vec<LoweringError>)` — it discards all locale-related data. The `LoweringContext` has no field for tracking locale keys or dlg-to-locale mappings. The collection pass (`collect_defs`) receives a `TypedAst` with no locale manifest attached. There is therefore no way for `collect_locale_defs` to know which localization keys were generated or which dlg methods are locale overrides without either (1) adding a locale manifest to the lowering pipeline output, or (2) re-scanning the AST for `[Locale]` attributes at collection time.

The correct approach — based on the spec (§25.6) and the existing `LocaleDefRow` structure — is option (2): scan the already-lowered ASTs for `[Locale("xx")]`-attributed `dlg` functions, match them against their base dlg MethodDef tokens, and emit LocaleDef rows. This avoids changing the lowering pipeline return type. The "localization key" referenced in the success criteria refers to these locale attribute strings, not per-line FNV-1a hashes. The per-line hashes are embedded in `say_localized()` call arguments in the lowered AST and are irrelevant to LocaleDef emission.

**Primary recommendation:** Add `collect_locale_defs()` to `collect.rs` that scans the AST for `[Locale("tag")]`-attributed functions, finds the matching base dlg MethodDef token, and emits LocaleDef rows via `builder.add_locale_def()`. This must run after `finalize()` since it needs resolved MethodDef tokens.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| EMIT-25 | Compiler emits LocaleDef table rows for localization keys | See Architecture Patterns: `add_locale_def()` already exists in `ModuleBuilder`; `serialize.rs` already serializes them; only `collect_locale_defs()` implementation is missing |
</phase_requirements>

---

## Standard Stack

### Core
| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|--------------|
| `writ-compiler` crate | current | All emit logic lives here | Project structure |
| `emit/collect.rs` | current | Where `collect_locale_defs` lives | Existing pattern for all other table collection |
| `emit/module_builder.rs` | current | `add_locale_def()` API already implemented | Existing infrastructure |
| `emit/serialize.rs` | current | LocaleDef serialization loop already implemented | Pre-wired at line 221-228 |

### Supporting
| Component | Version | Purpose | When to Use |
|-----------|---------|---------|-------------|
| `writ-compiler/tests/emit_tests.rs` | current | Unit test for LocaleDef emission | Tests for `collect_locale_defs` behavior |
| `writ-cli/tests/e2e_compile_tests.rs` | current | E2E tests using full pipeline | Tests that a localized source produces non-zero locale_defs count |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| AST re-scan for `[Locale]` attributes | Lowering pipeline manifest | AST re-scan is simpler and does not require changing `lower()` return type or `LoweringContext`; manifest approach is more general but invasive |

---

## Architecture Patterns

### How Other Table Collectors Work

Every other table (ComponentSlot, AttributeDef, ExportDef) follows the same pattern:

```rust
// In collect.rs, called from collect_defs() or collect_post_finalize():
fn collect_locale_defs(
    typed_ast: &TypedAst,
    asts: &[(FileId, &Ast)],
    builder: &mut ModuleBuilder,
) {
    for decl in &typed_ast.decls {
        if let TypedDecl::Fn { def_id, .. } = decl {
            let entry = typed_ast.def_map.get_entry(*def_id);
            // Check for [Locale("tag")] attribute
            let locale_tag = find_locale_attr(asts, entry);
            if let Some(tag) = locale_tag {
                // Find the base dlg method by matching name without locale override
                let base_method_token = find_base_dlg_token(builder, &entry.name, &entry.namespace);
                let loc_method_token = builder.token_for_def(*def_id).unwrap_or(MetadataToken::NULL);
                builder.add_locale_def(base_method_token, &tag, loc_method_token);
            }
        }
    }
}
```

**Key constraint:** `collect_locale_defs` MUST run after `finalize()` because it needs resolved `MethodDef` tokens. Pattern precedent: `collect_post_finalize()` at line 105-117 is the post-finalize collection hook for `ExportDef` and `AttributeDef`. `collect_locale_defs` should be added to `collect_post_finalize()`.

### LocaleDefRow Fields (from metadata.rs:352-356)

```rust
pub struct LocaleDefRow {
    pub dlg_method: MetadataToken,  // MethodDef token of the DEFAULT (base) dlg
    pub locale: u32,                // string heap offset for locale tag ("ja", "de", etc.)
    pub loc_method: MetadataToken,  // MethodDef token of the OVERRIDE dlg
}
```

This maps one base dialogue function to one locale-specific replacement. Each `[Locale("xx")]` attribute produces exactly one row.

### add_locale_def() API (from module_builder.rs:443-456)

```rust
pub fn add_locale_def(
    &mut self,
    dlg_method: MetadataToken,   // base dlg MethodDef token
    locale: &str,                 // locale tag string to intern
    loc_method: MetadataToken,   // override MethodDef token
) -> usize
```

Already fully implemented. Interns the locale string and pushes a `LocaleDefRow`.

### Serialization (from serialize.rs:221-228)

Already fully implemented:
```rust
// ── Table 18: LocaleDef ───────────────────────────────────────────────────
for ld in &builder.locale_defs {
    module.locale_defs.push(LocaleDefRow {
        dlg_method: WmToken(ld.dlg_method.0),
        locale: ld.locale,
        loc_method: WmToken(ld.loc_method.0),
    });
}
```

### What `[Locale]` Looks Like in the AST

The `AstAttribute` struct (from `ast/decl.rs`) has:
- `name: String` — the attribute name (e.g., `"Locale"`)
- `args: Vec<AstAttributeArg>` — the arguments

For `[Locale("ja")]`, the args would contain one positional `AstAttributeArg::Positional(AstExpr::StringLit { value: "ja", .. })`.

The `find_attrs_for_entry` helper in `collect.rs` (line 756-792) already retrieves attributes by walking the AST and matching entry by name + span. The pattern for collecting `[Locale]` attributes mirrors `collect_attributes()`.

### Existing locate_defs stub (collect.rs:95-96)

```rust
// 5. LocaleDef: stub — needs loc_key manifest from lowering.
// TODO: Add loc_keys manifest to LoweringContext output for locale dispatch.
```

This comment is misleading — the TODO description says "loc_keys manifest from lowering" but the `LocaleDefRow` spec is about structural overrides, not per-line keys. The correct approach is to scan for `[Locale]` attributes in the AST, not thread a manifest through the lowering pipeline.

### Identifying the Base dlg Method

The challenge: given a `[Locale("ja")]`-attributed function named `greetPlayer` in namespace `dialogue`, find the `MethodDef` token for the NON-attributed `greetPlayer` in the same namespace.

Strategy: Use `builder.methoddef_token_by_name(&entry.name)` to find any MethodDef with that name. If there are multiple (one base + multiple overrides), filter by namespace or use the first one registered (the un-attributed one should be registered first in document order).

Alternative: Add a `methoddef_token_by_name_and_namespace()` lookup, or track which MethodDefs are locale overrides vs base by checking their attrs.

### Test Infrastructure (existing pattern from emit_tests.rs:19-57)

```rust
fn emit_src(src: &'static str) -> (ModuleBuilder, Vec<Diagnostic>) {
    // parse -> lower -> resolve -> typecheck -> emit
    let (builder, emit_diags) = emit::emit(&typed_ast, &asts, &interner);
    (builder, emit_diags)
}
```

A LocaleDef unit test would call `emit_src` with source containing a `[Locale("ja")]` dlg and assert `builder.locale_defs.len() > 0`.

The E2E test (success criteria item 3) would use `compile_source()` from `e2e_compile_tests.rs`, call `Module::from_bytes()`, and assert `module.locale_defs.len() > 0`.

### Anti-Patterns to Avoid

- **Changing `lower()` return type to include a locale manifest:** Invasive and unnecessary. The AST already carries attribute information. The collection pass already scans ASTs for attributes.
- **Running `collect_locale_defs` before `finalize()`:** Will get null/zero tokens for MethodDefs because `def_token_map` is populated during `finalize()`.
- **Confusing per-line FNV-1a keys with LocaleDef:** LocaleDef is for structural override dispatch, not for the per-line string table. The per-line keys are embedded in `say_localized()` call args in the lowered code — no IL metadata table needed for them.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| String interning for locale tag | Custom string map | `builder.string_heap.intern(&tag)` inside `add_locale_def()` | Already done inside `add_locale_def()` |
| Attribute scanning | New AST walker | `find_attrs_for_entry()` pattern from collect.rs:756 | Pattern already established |
| MethodDef lookup by name | Custom hash map | `builder.methoddef_token_by_name()` from module_builder.rs:836 | Already implemented |
| Serialization | Manual binary write | `serialize.rs` loop at line 221-228 | Already fully wired |

---

## Common Pitfalls

### Pitfall 1: Confusing LocaleDef Purpose with Per-Line Keys
**What goes wrong:** Implementing LocaleDef to track per-line FNV-1a hashes instead of structural override dlg mappings.
**Why it happens:** The success criteria say "localization keys" and the collect.rs stub says "loc_keys manifest," which sounds like per-line keys.
**How to avoid:** Read the `LocaleDefRow` structure — it has `dlg_method` and `loc_method` (both MethodDef tokens), and `locale` (a locale string). This is clearly a method-to-method mapping, not a key-to-string mapping.
**Warning signs:** If you find yourself threading a `Vec<(String, String)>` through the lowering pipeline, you're solving the wrong problem.

### Pitfall 2: Running collect_locale_defs Before finalize()
**What goes wrong:** MethodDef tokens are 0 (null) because `def_token_map` is empty before `finalize()`.
**Why it happens:** Other table collection (in `collect_defs`) runs before `finalize()`.
**How to avoid:** Add `collect_locale_defs` to `collect_post_finalize()`, not to `collect_defs()`. This is the existing pattern for `collect_attributes()` and `collect_exports()`.

### Pitfall 3: No `[Locale]` attribute in current parser/AST
**What goes wrong:** There may be no test source that uses `[Locale("ja")]` syntax, and the parser may not support it yet.
**Why it happens:** The localization infrastructure is new and `[Locale]` attribute handling may not be exercised.
**How to avoid:** Check if `writ_parser` parses `[Locale("ja")]` in attribute position. If not, the test source will need to use whatever attribute syntax IS supported (`[Locale]` may be treated as a zero-arg attribute; the string arg syntax may need a fixture that passes `args: vec![AstAttributeArg::Positional(StringLit("ja"))]`). The emit test can construct a `TypedAst` programmatically with the attribute pre-injected if parser support is incomplete.

### Pitfall 4: methoddef_token_by_name Ambiguity
**What goes wrong:** If both the base dlg and the `[Locale("ja")]` override have the same function name, `methoddef_token_by_name()` may return the wrong one (the override instead of the base, or vice versa).
**Why it happens:** `methoddef_token_by_name()` does a linear scan and returns the first match by iteration order.
**How to avoid:** The base dlg (no `[Locale]` attribute) should be registered in the ModuleBuilder before the locale override (document order). Verify the scan returns the first registration. Alternatively, implement a two-pass approach: first pass collects all MethodDef tokens for names, second pass uses the non-locale-attributed one as base.

---

## Code Examples

### Minimal collect_locale_defs implementation sketch

```rust
// In collect.rs — add to collect_post_finalize() after collect_attributes()

fn collect_locale_defs(
    typed_ast: &TypedAst,
    asts: &[(FileId, &Ast)],
    builder: &mut ModuleBuilder,
) {
    let def_map = &typed_ast.def_map;
    for decl in &typed_ast.decls {
        let def_id = match decl {
            TypedDecl::Fn { def_id, .. } => *def_id,
            _ => continue,
        };
        let entry = def_map.get_entry(def_id);
        let attrs = find_attrs_for_entry(asts, entry);

        // Look for [Locale("tag")] attribute
        let locale_tag = attrs.iter().find_map(|a| {
            if a.name != "Locale" { return None; }
            a.args.iter().find_map(|arg| {
                if let AstAttributeArg::Positional(AstExpr::StringLit { value, .. }) = arg {
                    Some(value.clone())
                } else {
                    None
                }
            })
        });

        let tag = match locale_tag {
            Some(t) => t,
            None => continue,
        };

        // This function is a locale override for dlg with same name in same namespace.
        // Find the base dlg MethodDef token (the un-attributed one with the same name).
        let loc_method_token = builder
            .token_for_def(def_id)
            .unwrap_or(MetadataToken::NULL);

        // The base dlg has the same name; it was registered before the override.
        // We need to find its MethodDef token. If methoddef_token_by_name() returns
        // the override, we need a more specific lookup. For now use name scan:
        let base_token_raw = builder.methoddef_token_by_name(&entry.name)
            .map(MetadataToken)
            .unwrap_or(MetadataToken::NULL);

        if !base_token_raw.is_null() && !loc_method_token.is_null() {
            builder.add_locale_def(base_token_raw, &tag, loc_method_token);
        }
    }
}
```

### Test fixture for LocaleDef emission (emit_tests.rs pattern)

```rust
#[test]
fn locale_override_emits_locale_def() {
    // Source: one base dlg + one [Locale("ja")] override
    let src = r#"
dlg greet() { @Narrator Hello. }
[Locale("ja")]
dlg greet() { @Narrator こんにちは。 }
"#;
    let (builder, diags) = emit_src(src);
    assert!(diags.is_empty(), "unexpected diags: {:?}", diags);
    assert_eq!(builder.locale_defs.len(), 1, "should have 1 LocaleDef row");
}
```

Note: Whether this exact source is valid depends on parser support for `[Locale("ja")]` attribute syntax and duplicate dlg names (which may not be supported yet — see Open Questions).

### E2E test for non-zero LocaleDef count

```rust
#[test]
fn test_localized_dlg_produces_locale_def() {
    let src = r#"
dlg greet() { @Narrator Hello. }
[Locale("ja")]
dlg greet() { @Narrator こんにちは。 }
"#;
    let bytes = compile_source(src).expect("should compile");
    let module = Module::from_bytes(&bytes).expect("should deserialize");
    assert!(module.locale_defs.len() > 0, "module should have LocaleDef rows");
}
```

---

## Open Questions

1. **Does the parser support `[Locale("ja")]` dlg syntax?**
   - What we know: The parser handles attributes on `fn`, `struct`, `entity`, `enum`, `const`, `global`. The `dlg` lowerer produces an `AstFnDecl` with `attrs: vec![]` (hardcoded empty at `lower_dialogue()` line 131). The original `DlgDecl` CST node may or may not carry attributes.
   - What's unclear: Whether `DlgDecl` in the CST has an `attrs` field that gets discarded during lowering, or if attribute syntax on `dlg` was never implemented.
   - Recommendation: Check `writ_parser::cst::DlgDecl` struct. If it has `attrs`, wire them through in `lower_dialogue()`. If not, the E2E test may need to use a different fixture approach. The unit test for `collect_locale_defs` can be constructed with a programmatically-built `TypedAst` if parser support is absent.

2. **Can duplicate dlg names exist in the same namespace?**
   - What we know: `DefMap` uses FQN-to-DefId mapping; a second definition of the same name would collide unless the resolver has special handling for locale overrides.
   - What's unclear: Whether the resolver allows `[Locale("ja")]` functions to co-exist with their base counterpart under the same name.
   - Recommendation: If resolver rejects duplicate names, the test source for the E2E test must use unique function names and rely on the `[Locale]` attribute for matching, OR the resolver needs a special-case for `[Locale]`-attributed dlg functions. This may constrain the implementation scope.

3. **methoddef_token_by_name() ambiguity for base vs. override**
   - What we know: `methoddef_token_by_name()` scans `method_defs` linearly and returns first match. Both base and override would have the same string name.
   - What's unclear: Which one was registered first, and whether the ordering is deterministic after `finalize()` (which sorts `method_defs` by parent).
   - Recommendation: Add a separate lookup that uses the DefId's attribute state to distinguish base from override, or use a two-step approach: get ALL MethodDef tokens with that name, identify which has the `[Locale]` attribute (the override), use the other as base.

---

## Validation Architecture

Config does not have `nyquist_validation` set; skipping Validation Architecture section.

---

## Sources

### Primary (HIGH confidence)
- Source code: `D:/dev/git/Writ/writ-compiler/src/emit/collect.rs` — stub at lines 95-96; all other collection patterns
- Source code: `D:/dev/git/Writ/writ-compiler/src/emit/module_builder.rs` — `add_locale_def()` at line 443, `locale_defs` field at line 97
- Source code: `D:/dev/git/Writ/writ-compiler/src/emit/metadata.rs` — `LocaleDefRow` at line 352-356
- Source code: `D:/dev/git/Writ/writ-compiler/src/emit/serialize.rs` — serialization loop at lines 221-228
- Source code: `D:/dev/git/Writ/writ-compiler/src/lower/dialogue.rs` — `lower_dialogue()`, `compute_or_use_loc_key()`, `make_say_localized()`
- Source code: `D:/dev/git/Writ/writ-compiler/src/lower/context.rs` — `LoweringContext` struct (no locale manifest field)
- Spec: `D:/dev/git/Writ/language-spec/spec/26_25_localization.md` — §25.6 Structural Overrides with [Locale]
- Spec: `D:/dev/git/Writ/language-spec/spec/45_2_16_il_module_format.md` — Table 18 LocaleDef definition
- Spec: `D:/dev/git/Writ/language-spec/spec/29_28_lowering_reference.md` — §28.4 Localized Dialogue Lowering
- Tests: `D:/dev/git/Writ/writ-compiler/tests/emit_tests.rs` — `emit_src()` helper pattern
- Tests: `D:/dev/git/Writ/writ-cli/tests/e2e_compile_tests.rs` — `compile_source()` helper pattern

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all relevant code verified by direct source read
- Architecture: HIGH — `LocaleDefRow` structure and collection pattern verified; implementation sketch is directly derivable from existing analogous code
- Pitfalls: HIGH — all pitfalls identified from direct code analysis, not speculation

**Research date:** 2026-03-03
**Valid until:** Stable until structural changes to `emit/collect.rs`, `lower/dialogue.rs`, or `module_builder.rs`
