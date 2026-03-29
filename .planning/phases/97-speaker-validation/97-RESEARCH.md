# Phase 97: Speaker Validation - Research

**Researched:** 2026-03-27
**Domain:** Writ compiler — resolver validate pass, dialogue lowering, attribute system
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — auto-generated infrastructure phase; no user discussion occurred.

### Claude's Discretion
All implementation choices at Claude's discretion. Use ROADMAP phase goal, success criteria, and codebase conventions.

Key design notes:
- E0007 for non-[Singleton] entity speakers
- Distinct error for non-existent entity speakers
- Contract-typed speakers must be suppressed (no false E0007)

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SPKR-01 | `@speaker` reference targeting a non-`[Singleton]` entity produces E0007 | validate_speakers() in validate.rs reads entity attrs from asts via find_attrs_for_entry pattern; emits ResolutionError::InvalidSpeaker |
| SPKR-02 | `@speaker` reference targeting a non-existent entity produces an error | validate_speakers() calls def_map.get() for the speaker name; if not found and not a contract, emits ResolutionError::UnresolvedName (or a new variant) |
</phase_requirements>

## Summary

Phase 97 implements the body of `validate_speakers()` in `writ-compiler/src/resolve/validate.rs`. The function stub already exists (lines 94-107), is already wired into the resolve pipeline in `mod.rs` (line 114), and the error type `ResolutionError::InvalidSpeaker` (E0007) already exists in `error.rs`. The infrastructure is complete; only the validation logic is missing.

The core challenge is that the lowered AST has already erased dialogue speaker names from the `DlgDecl` CST — dialogue declarations are lowered to `AstFnDecl` at the `lower()` stage, and their singleton speakers become `let _name = Entity.getOrCreate<Name>()` hoisted statements, while param-based speakers become plain `AstExpr::Ident` references. The validate pass operates on the lowered AST, not the CST, so it cannot re-scan `DlgLine::SpeakerLine` nodes.

The standard approach in this codebase for similar semantic passes is:
1. Walk all `AstDecl::Fn` nodes to find dialogue-lowered functions (identifiable because they contain `say()`/`say_localized()` calls with speaker idents)
2. Alternatively — and more precisely — scan `AstDecl::Entity` entries in the DefMap to build a `singleton_entities: HashSet<String>` from entities whose `attrs` contain `[Singleton]`
3. Walk all `AstFnDecl` bodies to find `Entity.getOrCreate<Name>()` patterns (the hoisted let-bindings from singleton speaker collection in lowering) and validate each `Name` against the DefMap+singleton set

However, the cleanest approach given the existing CST/lowering design is to collect speaker names in the `LoweringContext` during `lower_dialogue()` and pass them forward to the resolver. But this would require threading speaker state through the entire pipeline, which is an architectural change.

**Primary recommendation:** Implement `validate_speakers()` by walking the lowered AST's `AstFnDecl` bodies to find `Entity.getOrCreate<TypeArg>()` GenericCall patterns (the hoisted singleton let-bindings from `lower_dialogue`), extract the entity name from the type argument, look it up in the DefMap, and check for `[Singleton]` attribute presence. Also scan `say()`/`say_localized()` call arguments for the speaker ident to get the correct span. For param-based speakers (Tier 1), the type is `Entity` which is a contract — suppress validation for those (success criterion 4).

## Standard Stack

### Core (all existing project infrastructure)
| Component | Location | Purpose | Status |
|-----------|----------|---------|--------|
| `validate.rs` | `writ-compiler/src/resolve/validate.rs` | Post-resolution validation pass file | Exists, stub to fill in |
| `ResolutionError::InvalidSpeaker` | `writ-compiler/src/resolve/error.rs:60-64` | E0007 error type | Exists |
| `ResolutionError::UnresolvedName` | `writ-compiler/src/resolve/error.rs:24-30` | E0003 for non-existent entity | Exists |
| `DefMap` + `DefKind::Entity` | `writ-compiler/src/resolve/def_map.rs` | Entity lookup by name | Exists |
| `AstEntityDecl.attrs` | `writ-compiler/src/ast/decl.rs:268` | Entity attribute access | Exists |
| `find_attrs_for_entry` pattern | `writ-compiler/src/check/env_build.rs:756` | Extract attrs from DefEntry+AST | Pattern to reuse |
| `code::E0007` | `writ-diagnostics/src/code.rs:13` | Diagnostic code constant | Exists |
| `resolve_src` test helper | `writ-compiler/tests/resolve_tests.rs:410-419` | Full pipeline test helper | Exists |

### No new dependencies required
This phase adds no new crates, libraries, or dependencies. It only fills in the existing stub.

## Architecture Patterns

### Recommended Project Structure
No new files. All changes in:
```
writ-compiler/src/resolve/validate.rs    — implement validate_speakers() body
writ-compiler/tests/resolve_tests.rs     — add speaker validation test cases
```

### Pattern 1: Attribute-checked validation (matches existing validate_attributes style)

The existing `validate_attributes()` function walks the AST and checks attrs inline. The speaker validation needs DefMap access in addition.

```rust
// Source: writ-compiler/src/resolve/validate.rs (existing validate_speakers stub)
pub fn validate_speakers(
    asts: &[(FileId, &Ast)],
    def_map: &DefMap,
    diags: &mut Vec<Diagnostic>,
) {
    // Build singleton entity set from DefMap + AST attribute scan
    let singleton_entities = collect_singleton_entity_names(asts, def_map);
    // Walk all Fn decls for hoisted singleton let-bindings
    for &(file_id, ast) in asts {
        validate_speakers_in_items(&ast.items, file_id, def_map, &singleton_entities, diags);
    }
}
```

### Pattern 2: Hoisted let-binding detection

After lowering, singleton speakers appear as:
```
AstStmt::Let {
    name: "_merchant",          // "_" + speaker_name.to_lowercase()
    value: AstExpr::GenericCall {
        callee: AstExpr::MemberAccess { object: Ident("Entity"), field: "getOrCreate" },
        type_args: [AstType::Named { name: "Merchant" }],
        args: [],
    }
}
```
The entity name is in `type_args[0]` as `AstType::Named { name }`.

Detecting this pattern is the reliable way to find singleton speaker references in the lowered AST. The function name (before lowering) is irrelevant — what matters is the `Entity.getOrCreate<Name>` call pattern which is uniquely emitted by `lower_dialogue`.

### Pattern 3: Contract-typed speaker suppression

Tier 1 speakers (from dlg params typed as `Entity`) are NOT hoisted as `Entity.getOrCreate<Name>()` calls. They remain as plain `AstExpr::Ident` references. The validate pass only needs to check Tier 2 (singleton) speakers, which are always identified by the `Entity.getOrCreate<Name>()` pattern. Tier 1 speakers are already suppressed by design — they never generate this pattern.

### Pattern 4: [Singleton] attribute check

```rust
// Reuse the pattern from check/env_build.rs:find_attrs_for_entry
// For each Entity DefEntry in the DefMap, check if attrs contain "Singleton"
fn entity_has_singleton_attr(asts: &[(FileId, &Ast)], entry: &DefEntry) -> bool {
    // Walk asts to find AstDecl::Entity matching entry.name + entry.name_span
    // Check if entity.attrs contains AstAttribute { name: "Singleton", .. }
}
```

This pattern is already demonstrated at `check/env_build.rs:756-801` and `check/env.rs:225-231` for `[Deprecated]` and `[Conditional]` detection. The same pattern applied to `[Singleton]` on `AstDecl::Entity`.

### Pattern 5: Non-existent entity speaker error

When `Entity.getOrCreate<Name>()` is found and `def_map.get("Name")` returns `None`, emit `ResolutionError::UnresolvedName`. This is the `ResolutionError::InvalidSpeaker` for the non-existent case — but looking at the existing error definition at `error.rs:202-207`, the current `InvalidSpeaker` message says "speaker not found" which covers both cases. The success criteria says "distinct error" — this could mean a distinct `E0007` message distinguishing "entity exists but not Singleton" vs "entity doesn't exist at all", using the same `InvalidSpeaker` variant but with different message text, or a new error variant.

**Decision: use a single `InvalidSpeaker` variant with a `reason: SpeakerErrorKind` field, or emit two different error messages from the same variant.** The current variant has `name` and `file`/`span` — the message format can encode the reason. The existing `From<ResolutionError>` for `InvalidSpeaker` already says "speaker not found" — update the message to include the entity name and reason. Alternatively, keep E0007 for non-[Singleton] and use E0003 (UnresolvedName) for non-existent. That matches the spirit of "distinct error" without a new variant.

**Recommended:** Use `ResolutionError::UnresolvedName` (E0003) for non-existent speaker and `ResolutionError::InvalidSpeaker` (E0007) for non-[Singleton] entity. This gives maximally distinct errors.

### Anti-Patterns to Avoid

- **Walking CST instead of lowered AST:** The validate pass runs after lowering; CST is no longer available. Don't try to reach back to `DlgDecl`.
- **Threading speaker state through lower() to resolve():** Would require a major pipeline API change. Not needed — the hoisted let pattern is detectable in the lowered AST.
- **Scanning say()/say_localized() call args for spans:** The span on the hoisted `let` binding comes from `speaker_span` in lowering (the CST speaker span). This is the correct span to use for the diagnostic — it points at the `@Speaker` token. Use `let_stmt.name_span` (which is set to `speaker_span` in `lower_dialogue.rs:117-140`).
- **Checking namespace qualification:** Entity names in dialogue are looked up as simple names (the lowering code uses `AstType::Named { name: speaker_name.clone() }` at line 133). The validate pass should do the same unqualified lookup.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Attribute extraction | Custom attr scanner | `find_attrs_for_entry` pattern from `env_build.rs:756` | Already tested and handles namespace/file boundary correctly |
| Error formatting | Custom diagnostic builder | `ResolutionError::InvalidSpeaker` / `UnresolvedName` via `.into()` | Consistent with all other diagnostics in the resolver |
| Test infrastructure | Custom parse/lower/resolve chain | `resolve_src()` helper from `resolve_tests.rs:410-419` | Identical pattern used throughout the test suite |

## Common Pitfalls

### Pitfall 1: Namespace mismatch on entity lookup
**What goes wrong:** Dialogue speaker `@Merchant` is looked up as `"Merchant"` but the entity is declared in namespace `"game"` as `"game::Merchant"`. `def_map.get("Merchant")` returns None, yielding a false "not found" error.
**Why it happens:** The lowering code stores the simple speaker name in `AstType::Named { name: speaker_name.clone() }` without namespace qualification. When the dlg and entity are in the same file/namespace, this works because the type checker resolves names in context. But validate_speakers operates outside any scope chain.
**How to avoid:** Use the DefMap's `by_fqn` table (exact FQN) AND `file_private` table (simple name within file). If the entity is private/local, it may be in `file_private[file_id][name]`. Also check if `def_map.get(name)` works for root-level entities. For namespace-qualified entities, the name in the hoisted let will still be the simple name (e.g., `"Merchant"`, not `"game::Merchant"`) because CST captures only the identifier after `@`.
**Warning signs:** Test case with namespaced entity emitting false E0007/E0003.
**Resolution:** Mirror the scope resolution that `collect_singleton_speakers` does — it uses the raw CST name. The validate pass should check simple name in `def_map.by_fqn` as well as all namespace-prefixed variants. The simplest safe approach: iterate all DefIds of kind Entity and check `entry.name == speaker_name`. This avoids namespace ordering assumptions.

### Pitfall 2: False positive for param-typed speakers
**What goes wrong:** A dlg with `dlg greet(npc: Entity)` passes `npc` as a speaker. In the body, `@npc Hello.` becomes `say(_npc, ...)` where `_npc` is a simple ident, NOT an `Entity.getOrCreate<Name>()`. If the validate pass mistakenly flags plain idents, it would be a false positive.
**Why it happens:** Tier 1 speakers are already excluded from `collect_singleton_speakers` in lowering, so they don't generate the hoisted let pattern. The validate pass only needs to handle the hoisted let pattern.
**How to avoid:** The validation logic should only trigger on `AstStmt::Let` nodes whose `value` is `AstExpr::GenericCall` with callee `Entity.getOrCreate`. Plain idents are ignored. This is the natural outcome if detection is based on the hoisted-let pattern.

### Pitfall 3: Missing match on nested block namespaces
**What goes wrong:** Entity declared inside `namespace game { [Singleton] entity Merchant { ... } }` is in a block namespace. If `validate_speakers_in_items` doesn't recurse into `AstDecl::Namespace(Block { .. })`, it won't build the singleton set for that entity.
**Why it happens:** Namespace blocks are common. The `validate_attributes` function already handles this recursion (line 35-37 of validate.rs).
**How to avoid:** Follow the same recursion pattern as `validate_attrs_in_items` — always match `AstDecl::Namespace(Block)` and recurse.

### Pitfall 4: Using `InvalidSpeaker` span incorrectly
**What goes wrong:** Emitting the diagnostic with the `let` statement span (the entire `let _merchant = Entity.getOrCreate<Merchant>()`) instead of the speaker identifier span.
**Why it happens:** The `let` statement span and the name_span are different. The name_span (set to `speaker_span` in lower_dialogue.rs:117) points at `@Merchant` in the source — the ideal diagnostic location.
**How to avoid:** Use `let_stmt.name_span` not `let_stmt.span`. In `lower_dialogue.rs:117`, `name_span: span` where `span = *speaker_span`. So the hoisted let's `name_span` IS the CST speaker span.

### Pitfall 5: find_attrs_for_entry doesn't recurse into namespaces
**What goes wrong:** `find_attrs_for_entry` in `env_build.rs:764` iterates `&ast.items` but doesn't recurse into block namespace items. An entity inside a block namespace wouldn't be found.
**Why it happens:** The env_build version is non-recursive (it walks top-level items only). However, this isn't a problem because the DefEntry records `file_id` and `name_span`, and the entity AST lookup for attrs should match exactly — the name_span from the DefEntry is absolute in the source and will match even inside a namespace block... but only if the loop recurses.
**How to avoid:** When writing the new `entity_has_singleton_attr` helper for validate.rs, add namespace block recursion. Don't copy env_build's version blindly — add the missing recursion.

## Code Examples

### Entity attribute check pattern (from env_build.rs:756-801)
```rust
// Source: writ-compiler/src/check/env_build.rs:775-777
AstDecl::Entity(e) if e.name == entry.name && e.name_span == entry.name_span => {
    return e.attrs.clone();
}
```

### Singleton detection from attrs
```rust
// Check if attrs slice contains [Singleton]
fn has_singleton_attr(attrs: &[AstAttribute]) -> bool {
    attrs.iter().any(|a| a.name == "Singleton")
}
```

### Hoisted let-binding pattern detection
```rust
// Source: writ-compiler/src/lower/dialogue.rs:117-140
// The hoisted let looks like:
// AstStmt::Let {
//     name: format!("_{}", speaker_name.to_lowercase()),
//     name_span: speaker_span,          // <- use this for diagnostic span
//     value: AstExpr::GenericCall {
//         callee: AstExpr::MemberAccess {
//             object: AstExpr::Ident { name: "Entity" },
//             field: "getOrCreate",
//         },
//         type_args: [AstType::Named { name: <EntityTypeName> }],
//         args: [],
//     }
// }
fn extract_singleton_speaker_name(stmt: &AstStmt) -> Option<(&str, SimpleSpan)> {
    if let AstStmt::Let { name, name_span, value, .. } = stmt
        && name.starts_with('_')
    {
        if let AstExpr::GenericCall { callee, type_args, args, .. } = value.as_ref()
            && args.is_empty()
            && type_args.len() == 1
        {
            if let AstExpr::MemberAccess { object, field, .. } = callee.as_ref()
                && field == "getOrCreate"
            {
                if let AstExpr::Ident { name: obj_name, .. } = object.as_ref()
                    && obj_name == "Entity"
                {
                    if let AstType::Named { name: entity_name, .. } = &type_args[0] {
                        return Some((entity_name, *name_span));
                    }
                }
            }
        }
    }
    None
}
```

### Test pattern (from resolve_tests.rs + deprecated_tests.rs pattern)
```rust
// Source: writ-compiler/tests/resolve_tests.rs:410-419
fn resolve_src(src: &'static str) -> (NameResolvedAst, Vec<Diagnostic>) {
    let ast = parse_and_lower(src);
    let file_id = FileId(0);
    let asts = vec![(file_id, &ast)];
    let file_paths = vec![(file_id, "src/test.writ")];
    resolve::resolve(&asts, &file_paths)
}

// Speaker validation test example
#[test]
fn singleton_speaker_valid() {
    let (_, diags) = resolve_src(r#"
[Singleton]
pub entity Merchant {}
dlg greet() {
    @Merchant Hello.
}
"#);
    assert!(!diags.iter().any(|d| d.code == "E0007"), "no speaker error expected");
}

#[test]
fn non_singleton_speaker_emits_e0007() {
    let (_, diags) = resolve_src(r#"
pub entity Merchant {}
dlg greet() {
    @Merchant Hello.
}
"#);
    assert!(diags.iter().any(|d| d.code == "E0007"), "E0007 expected for non-Singleton entity");
}

#[test]
fn nonexistent_speaker_emits_error() {
    let (_, diags) = resolve_src(r#"
dlg greet() {
    @Ghost Hello.
}
"#);
    // Should emit a resolution error, not a panic or silent pass
    assert!(!diags.is_empty(), "error expected for non-existent speaker");
}
```

## State of the Art

| Old Approach | Current Approach | Status |
|--------------|-----------------|--------|
| `validate_speakers()` was a no-op stub | Fill in the stub body | This phase |
| Speaker validation planned for "deeper implementation" | Now directly implementable via hoisted-let detection | Resolved |

## Open Questions

1. **Namespace-qualified entity speakers**
   - What we know: `collect_singleton_speakers` in lowering uses the raw CST identifier string (e.g., `"Merchant"` not `"game::Merchant"`) as the speaker name. The hoisted let's type arg is `AstType::Named { name: "Merchant" }`.
   - What's unclear: If an entity is in a namespace, should the validate pass do a full name search across all entities in the DefMap, or should it look up by simple name first (matching what lowering produces)?
   - Recommendation: Scan all DefEntries of kind Entity and match by `entry.name == speaker_name`. This is O(n) but n is small for typical programs. This avoids namespace bugs entirely.

2. **New `SpeakerErrorKind` variant vs. two existing error types**
   - What we know: Success criterion 2 says "distinct error" for non-existent vs. non-Singleton. The current `InvalidSpeaker` variant uses one format string.
   - What's unclear: Whether "distinct" means distinct error code or distinct message.
   - Recommendation: Use E0007 (`InvalidSpeaker`) for the non-[Singleton] case (entity exists, wrong kind) and E0003 (`UnresolvedName`) for the non-existent case. This gives distinct codes, matches the existing error semantics, and requires no new error variants.

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — pure Rust compiler code changes only)

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) |
| Config file | none — standard cargo test discovery |
| Quick run command | `cargo test -p writ-compiler speaker` |
| Full suite command | `cargo test -p writ-compiler` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SPKR-01 | `@speaker` targeting non-[Singleton] entity produces E0007 | unit | `cargo test -p writ-compiler non_singleton_speaker` | ❌ Wave 0 |
| SPKR-01 | `@speaker` targeting [Singleton] entity compiles clean | unit | `cargo test -p writ-compiler singleton_speaker_valid` | ❌ Wave 0 |
| SPKR-02 | `@speaker` targeting non-existent entity produces error | unit | `cargo test -p writ-compiler nonexistent_speaker` | ❌ Wave 0 |
| SPKR-01 | Contract-typed param speakers produce no false E0007 | unit | `cargo test -p writ-compiler contract_speaker_suppressed` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-compiler speaker`
- **Per wave merge:** `cargo test -p writ-compiler`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-compiler/tests/speaker_validation_tests.rs` — covers SPKR-01, SPKR-02

*(No framework or fixture gaps — `resolve_src` test helper already exists in `resolve_tests.rs` and can be copy-pasted or the test file can use the same pattern directly.)*

## Sources

### Primary (HIGH confidence)
- `writ-compiler/src/resolve/validate.rs` — existing stub with TODO comment, exact function to implement
- `writ-compiler/src/resolve/error.rs` — `InvalidSpeaker` variant, E0007 wiring
- `writ-compiler/src/lower/dialogue.rs` — exact AST shape of hoisted singleton let-bindings (lines 112-142)
- `writ-compiler/src/lower/context.rs` — LoweringContext, SpeakerScope — confirms no speaker data flows forward to resolver
- `writ-compiler/src/resolve/mod.rs:114` — confirms `validate_speakers()` is already called in the pipeline
- `writ-compiler/src/check/env_build.rs:756-801` — `find_attrs_for_entry` pattern for attribute extraction
- `writ-diagnostics/src/code.rs:13` — E0007 constant already defined
- `writ-compiler/src/ast/decl.rs:267-281` — `AstEntityDecl` shape with `attrs` field
- `writ-compiler/src/resolve/def_map.rs` — DefMap, DefKind::Entity
- `writ-compiler/tests/resolve_tests.rs:410-419` — `resolve_src` test helper

### Secondary (MEDIUM confidence)
- `writ-golden/tests/golden/dlg_quest_pattern.writ` — confirms real dialogue uses param-based speakers (Tier 1, e.g., `merchant: Entity`), which must NOT trigger E0007

## Metadata

**Confidence breakdown:**
- Implementation approach: HIGH — stub is in place, error types exist, pipeline wiring exists, only function body missing
- Hoisted-let detection pattern: HIGH — directly read from lower/dialogue.rs source; exact AST shape is known
- Namespace entity lookup: MEDIUM — the simple-name scan approach is safe but may be slightly inefficient for large programs
- Test pattern: HIGH — identical pattern already used in deprecated_tests.rs and resolve_tests.rs

**Research date:** 2026-03-27
**Valid until:** Stable (no external dependencies; pure internal code)
