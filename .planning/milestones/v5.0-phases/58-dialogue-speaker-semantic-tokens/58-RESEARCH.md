# Phase 58: Dialogue Speaker Semantic Tokens - Research

**Researched:** 2026-03-16
**Domain:** Writ LSP semantic tokens — dialogue `@Speaker` construct highlighting
**Confidence:** HIGH

## Summary

Phase 58 is a surgical gap-closure. The v5.0 milestone audit (`v5.0-MILESTONE-AUDIT.md`) identified that `TOKEN_TYPE_DIALOGUE_SPEAKER` (type index 4) is registered in the `SemanticTokensLegend` in `backend.rs` and mapped in `package.json` `semanticTokenScopes`, but is annotated `#[allow(dead_code)]` in `queries.rs:987` because `collect_semantic_tokens` never emits it.

The root cause is a fundamental architectural fact: dialogue is fully lowered to regular `Fn` declarations before the TypedAst is constructed. The CST `DlgDecl` with its `SpeakerLine` and `SpeakerTag` variants (which carry `@Speaker` source spans) is consumed during lowering in `writ-compiler/src/lower/dialogue.rs`. By the time `collect_semantic_tokens` walks the `TypedAst`, the original `@SpeakerName` tokens exist only as either entity-typed variable references (`TypedExpr::Var`) or as generic Ident expressions — neither of which is distinguishable as a dialogue speaker at the typed IR level.

The fix requires working one level lower: the CST. Speaker spans exist in `DlgLine::SpeakerLine { speaker: Spanned<&str> }` and `DlgLine::SpeakerTag(Spanned<&str>)`. The spans cover the identifier name only — the parser uses `just(Token::At).ignore_then(select! { Token::Ident(name) => name }.map_with(|n, e| (n, e.span())))`, so the `@` sigil is excluded from the speaker span. These spans must be collected by re-parsing the source text, before lowering discards them. An alternative — threading speaker spans through lowering into the DefMap or TypedAst — would require invasive compiler changes and is out of scope for a single gap-closure phase.

**Primary recommendation:** Add a `collect_dialogue_speaker_tokens(source: &str) -> Vec<RawSemanticToken>` function in `queries.rs` that re-parses the source using `writ_parser::parse` and walks the resulting `Vec<Spanned<Item>>` to collect `(span, TOKEN_TYPE_DIALOGUE_SPEAKER)` entries for all `@SpeakerName` identifiers. Merge these tokens with the existing TypedAst-derived tokens in `collect_semantic_tokens`, sort, and return. Remove the `#[allow(dead_code)]` annotation from `TOKEN_TYPE_DIALOGUE_SPEAKER`.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DIFF-01 | Semantic highlighting distinguishes entity names, component types, dialogue speakers, and keywords with distinct token types | Dialogue speaker spans exist in the CST `DlgLine::SpeakerLine` and `DlgLine::SpeakerTag` variants (span excludes `@`); `TOKEN_TYPE_DIALOGUE_SPEAKER` (4) is already registered in the legend and package.json; only emission is missing |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| writ-parser | workspace | Re-parse source to access CST `DlgDecl` | Already a dependency of writ-lsp (Cargo.toml line 22); parse is cheap — no name resolution or type checking |
| writ-lsp queries.rs | workspace | Extend `collect_semantic_tokens` | All semantic token logic lives in this single file |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| chumsky SimpleSpan | 0.12.0 | Byte-offset spans from CST nodes | Used throughout writ-lsp; `push_token_for_span` already accepts it |

No new crate dependencies required. `writ-parser` is already in `writ-lsp/Cargo.toml`.

**Installation:** No new deps.

## Architecture Patterns

### Recommended Project Structure
```
writ-lsp/src/
├── queries.rs          # Add: collect_dialogue_speaker_tokens(source) -> Vec<RawSemanticToken>
│                       #      Extend: collect_semantic_tokens() calls collect_dialogue_speaker_tokens
└── (no other files change)
```

### Pattern 1: CST Re-parse for Semantic Tokens

**What:** Re-parse the source text using `writ_parser::parse` in a new helper function, walk `Item::Dlg` nodes in the returned `Vec<Spanned<Item>>`, and emit `TOKEN_TYPE_DIALOGUE_SPEAKER` tokens for each `@SpeakerName` occurrence.

**When to use:** Any time `collect_semantic_tokens` is called for a file containing `dlg` declarations. The re-parse is fast (single-file, no name resolution or type checking) and acceptable for the semantic token refresh frequency.

**Why CST, not TypedAst:** By the time the TypedAst is built, dialogue lowering has transformed `dlg` into `fn`. The `@SpeakerName` identifiers become either:
- Tier 1 (param speakers): `AstExpr::Ident { name: speaker_name }` — indistinguishable from any other variable reference
- Tier 2 (singleton speakers): `AstExpr::Ident { name: format!("_{}", speaker.to_lowercase()) }` — a transformed mangled name

Neither carries the original `@SpeakerName` token type. The only location where speaker spans are preserved as-is is the CST `DlgDecl::body`, before `lower_dialogue` is called.

**Verified API facts:**
- `writ_parser::parse(source: &str) -> (Option<Vec<Spanned<Item>>>, Vec<Error>)` — returns a flat item list, NOT a struct
- `Item::Dlg(Spanned<DlgDecl>)` — the Dlg variant wraps `(DlgDecl, SimpleSpan)`
- `DlgDecl::body: Vec<Spanned<DlgLine>>` — the dialogue body lines
- `DlgLine::SpeakerLine { speaker: Spanned<&'src str>, .. }` — `speaker` is `(name, span)` where span excludes `@`
- `DlgLine::SpeakerTag(Spanned<&'src str>)` — `(name, span)` where span excludes `@`
- `Spanned<T>` is `type Spanned<T> = (T, SimpleSpan)` (confirmed in `cst.rs:11`)

**Implementation in queries.rs:**
```rust
// Source: writ-parser/src/cst.rs — DlgDecl, DlgLine::SpeakerLine, DlgLine::SpeakerTag
use writ_parser::cst::{Item, DlgLine};

/// Collect TOKEN_TYPE_DIALOGUE_SPEAKER tokens for all @Speaker names in the source.
///
/// Re-parses `source` via writ_parser::parse and walks Item::Dlg entries to find
/// SpeakerLine and SpeakerTag lines. Each @SpeakerName occurrence gets a
/// RawSemanticToken with token_type = TOKEN_TYPE_DIALOGUE_SPEAKER (4).
///
/// The speaker span covers the identifier name only — the @ sigil is excluded
/// by the parser (just(Token::At).ignore_then(ident_with_span)).
///
/// Returns tokens in source order (not sorted — caller merges and sorts).
pub fn collect_dialogue_speaker_tokens(source: &str) -> Vec<RawSemanticToken> {
    let mut tokens = Vec::new();

    // Re-parse source; gracefully handle errors — partial CSTs may still yield items.
    let (items_opt, _parse_errs) = writ_parser::parse(source);
    let Some(items) = items_opt else { return tokens };

    // Walk top-level items looking for dlg declarations
    for (item, _item_span) in &items {
        if let Item::Dlg((dlg_decl, _dlg_span)) = item {
            collect_speaker_tokens_in_dlg_body(&dlg_decl.body, source, &mut tokens);
        }
    }

    tokens
}

fn collect_speaker_tokens_in_dlg_body(
    lines: &[writ_parser::cst::Spanned<DlgLine<'_>>],
    source: &str,
    tokens: &mut Vec<RawSemanticToken>,
) {
    for (line, _line_span) in lines {
        match line {
            DlgLine::SpeakerLine { speaker: (_, span), .. } => {
                push_token_for_span(tokens, source, span, TOKEN_TYPE_DIALOGUE_SPEAKER);
            }
            DlgLine::SpeakerTag((_, span)) => {
                push_token_for_span(tokens, source, span, TOKEN_TYPE_DIALOGUE_SPEAKER);
            }
            DlgLine::Choice((choice, _)) => {
                for (arm, _) in &choice.arms {
                    collect_speaker_tokens_in_dlg_body(&arm.body, source, tokens);
                }
            }
            DlgLine::If((dlg_if, _)) => {
                collect_speaker_tokens_in_dlg_body(&dlg_if.then_block, source, tokens);
                collect_dlg_if_else_speakers(&dlg_if.else_block, source, tokens);
            }
            DlgLine::Match((dlg_match, _)) => {
                for (arm, _) in &dlg_match.arms {
                    collect_speaker_tokens_in_dlg_body(&arm.body, source, tokens);
                }
            }
            DlgLine::TextLine { .. }
            | DlgLine::CodeEscape(_)
            | DlgLine::Transition(_) => {}
        }
    }
}

fn collect_dlg_if_else_speakers(
    else_block: &Option<Box<writ_parser::cst::Spanned<writ_parser::cst::DlgElse<'_>>>>,
    source: &str,
    tokens: &mut Vec<RawSemanticToken>,
) {
    if let Some(boxed) = else_block {
        let (dlg_else, _) = boxed.as_ref();
        match dlg_else {
            writ_parser::cst::DlgElse::ElseIf(elif) => {
                collect_speaker_tokens_in_dlg_body(&elif.then_block, source, tokens);
                collect_dlg_if_else_speakers(&elif.else_block, source, tokens);
            }
            writ_parser::cst::DlgElse::Else(lines) => {
                collect_speaker_tokens_in_dlg_body(lines, source, tokens);
            }
        }
    }
}
```

**Integration into collect_semantic_tokens:**
```rust
pub fn collect_semantic_tokens(
    ast: &TypedAst,
    interner: &TyInterner,
    source: &str,
    file_id: writ_diagnostics::FileId,
) -> Vec<RawSemanticToken> {
    let mut tokens = Vec::new();

    // ... existing TypedDecl walk unchanged ...

    // PHASE 58: Emit dialogue speaker tokens by re-parsing the source.
    // The TypedAst has no dialogue-specific nodes (dlg is lowered to fn).
    let speaker_tokens = collect_dialogue_speaker_tokens(source);
    tokens.extend(speaker_tokens);

    // Sort by position (line, then start_char)
    tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.start_char.cmp(&b.start_char)));
    tokens
}
```

### Pattern 2: CST Structure Reference (Verified)

**What:** Exact CST type hierarchy for dialogue access.

**Verified facts:**
- `writ_parser::parse(src)` returns `(Option<Vec<Spanned<Item<'_>>>>, Vec<Rich<...>>)`
- `Spanned<T> = (T, SimpleSpan)` — a type alias, not a struct
- `Item::Dlg(Spanned<DlgDecl<'src>>)` — inner tuple is `(DlgDecl, SimpleSpan)`, destructured as `(dlg_decl, _dlg_span)`
- `DlgDecl::body: Vec<Spanned<DlgLine<'src>>>` — confirmed in `cst.rs:847`
- `DlgLine::SpeakerLine { speaker: Spanned<&'src str>, text: ..., loc_key: ... }` — `speaker` is `(name, span)`
- `DlgLine::SpeakerTag(Spanned<&'src str>)` — tuple variant, destructured as `(name, span)`

**Speaker span excludes `@`:** Confirmed in `parser.rs:2051-2054`:
```
just(Token::At).ignore_then(
    select! { Token::Ident(name) => name }
        .map_with(|n, e| (n, e.span())),
)
```
The `Token::At` is consumed with `ignore_then`, so `e.span()` captures only the `Ident` token, not the `@`. The speaker span starts at the first character of the name.

### Pattern 3: Removing the Dead Code Annotation

Once `TOKEN_TYPE_DIALOGUE_SPEAKER` is used by `collect_dialogue_speaker_tokens`, remove the `#[allow(dead_code)]` annotation from that constant only:

```rust
// Before (Phase 54 state in queries.rs):
#[allow(dead_code)]
const TOKEN_TYPE_KEYWORD: u32 = 0;
const TOKEN_TYPE_TYPE: u32 = 1;
const TOKEN_TYPE_ENTITY: u32 = 2;
const TOKEN_TYPE_COMPONENT: u32 = 3;
#[allow(dead_code)]
const TOKEN_TYPE_DIALOGUE_SPEAKER: u32 = 4;  // <-- remove this allow
const TOKEN_TYPE_FUNCTION: u32 = 5;
const TOKEN_TYPE_VARIABLE: u32 = 6;
#[allow(dead_code)]
const TOKEN_TYPE_PARAMETER: u32 = 7;

// After (Phase 58):
// Remove #[allow(dead_code)] from TOKEN_TYPE_DIALOGUE_SPEAKER only.
// TOKEN_TYPE_KEYWORD and TOKEN_TYPE_PARAMETER remain annotated (reserved, unused).
```

### Anti-Patterns to Avoid

- **Threading speaker spans through the compiler:** Do not add speaker span tracking to `LoweringContext`, `AstFnDecl`, or `DefEntry`. This would require changes to the resolver, type checker, and IR — invasive for a one-token gap closure. The re-parse approach is simpler and correct.

- **Walking `TypedExpr::Var` nodes and guessing speaker identity:** Any `TypedExpr::Var` whose `Ty` is `TyKind::Entity(_)` could be a speaker reference, but it could equally be any entity-typed variable. There is no reliable way to distinguish `@Speaker` variable references from other entity-typed variables in the TypedAst.

- **Attempting to parse only `dlg` blocks via string scanning:** The parser is designed to parse whole files. Do not scan for `dlg` with string operations and parse sub-sections.

- **Worrying about duplicate tokens:** The CST `SpeakerTag` lines do not produce any statement in the TypedAst (they are state-only in the lowering: `ctx.push_speaker(...)` with no statement emitted). `SpeakerLine` produces `AstStmt::Expr { expr: make_say(speaker_ref, ...) }` — the `speaker_ref` expression is an `AstExpr::Ident` that resolves to the mangled `_speakername` variable, not the original `@SpeakerName` span. Therefore, the CST speaker spans do NOT appear as `TypedExpr` nodes in the TypedAst at all. No duplication with `TOKEN_TYPE_ENTITY` tokens occurs in practice.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CST access for speaker spans | Manual `@`-prefix text scanning with regex | `writ_parser::parse` + `Item::Dlg` walk | Parser handles all syntax edge cases; text scanning misses nested speakers in Choice/If/Match arms |
| Span-to-position conversion | Custom byte-to-line/char calculation | Existing `push_token_for_span` in `queries.rs` | Already converts `SimpleSpan` to UTF-16 positions; reuse trivially correct |
| Token sorting | Manual merge | `tokens.sort_by(...)` already in `collect_semantic_tokens` | Existing sort handles all tokens uniformly |
| Else branch recursion | Inline match | `collect_dlg_if_else_speakers` helper | Mirrors pattern from `lower/dialogue.rs:227-244` |

**Key insight:** Phase 54 built all the infrastructure. Phase 58 is one new ~60-line function that feeds into the already-working pipeline. The entire change is contained in `queries.rs`.

## Common Pitfalls

### Pitfall 1: Wrong parse API shape — accessing `.items` on a Vec
**What goes wrong:** Code writes `cst.items` (treating the parse result as a `Program` struct) instead of iterating directly over the `Vec<Spanned<Item>>`.
**Why it happens:** `cst.rs` defines a `Program` struct with `.items`, but `writ_parser::parse` returns `Option<Vec<Spanned<Item>>>` directly, not `Option<Program>`.
**How to avoid:** Use `let Some(items) = items_opt else { return tokens }` and iterate `&items` directly. Do not write `items.items`.
**Warning signs:** Compilation error `no field items on Vec<...>`.

### Pitfall 2: Wrong `Item::Dlg` destructuring pattern
**What goes wrong:** Code writes `Item::Dlg(dlg_decl)` but `Item::Dlg` wraps `Spanned<DlgDecl>` = `(DlgDecl, SimpleSpan)`, so the inner value is a tuple. Pattern match must be `Item::Dlg((dlg_decl, _))`.
**Why it happens:** Single-element tuple wrapper looks like a struct wrapper.
**How to avoid:** Use `if let Item::Dlg((dlg_decl, _dlg_span)) = item` pattern.
**Warning signs:** Compilation error with type mismatch on the Dlg variant.

### Pitfall 3: Missing recursive descent into nested dialogue structures
**What goes wrong:** Only top-level `DlgLine::SpeakerLine` and `SpeakerTag` are collected, but speakers inside `Choice` arms, `If` branches, and `Match` arms are skipped.
**Why it happens:** The naive implementation only iterates the top-level body without recursing.
**How to avoid:** `collect_speaker_tokens_in_dlg_body` must handle `Choice`, `If`, `Match` by recursing. Use the pattern from `lower/dialogue.rs:184-245` as reference.
**Warning signs:** Speakers at the top level of a dlg are highlighted; speakers inside `$ choice { }` or `$ if { }` blocks are not.

### Pitfall 4: Parse failure hides all speakers on files with syntax errors
**What goes wrong:** If the source has a syntax error, `writ_parser::parse` may return `None`, causing `collect_dialogue_speaker_tokens` to return empty.
**Why it happens:** Parser error recovery may return `None` for severely malformed files.
**How to avoid:** The guard `let Some(items) = items_opt else { return tokens }` is correct — graceful degradation. Dialogue speakers not highlighted during active syntax errors is acceptable behavior.
**Warning signs:** Speakers disappear when an unrelated syntax error exists. Expected, not a bug.

### Pitfall 5: `DlgLine::If` else branch not recursed
**What goes wrong:** `DlgLine::If` correctly recurses into `then_block` but not into `else_block`. Speakers in `else { }` or `else if { }` sub-branches are missed.
**Why it happens:** `else_block` is `Option<Box<Spanned<DlgElse>>>` — a nested structure that requires a separate helper.
**How to avoid:** Implement `collect_dlg_if_else_speakers` following the same pattern as `lower/dialogue.rs:227-244`.
**Warning signs:** Speakers inside `else` blocks of conditional dialogue are not highlighted.

## Code Examples

### Verified pattern: parse returns flat item list

```rust
// Source: writ-lsp/src/analysis_host.rs:46
// Existing usage of writ_parser::parse in the codebase:
let (cst_opt, parse_errs) = writ_parser::parse(src);
// cst_opt is Option<Vec<Spanned<Item<'_>>>>
// cst is Vec<(Item, SimpleSpan)> — iterate directly
for (item, _span) in &cst {
    // ...
}
```

### Verified pattern: DlgLine variants from lower/dialogue.rs

```rust
// Source: writ-compiler/src/lower/dialogue.rs:192,298
// Confirmed destructuring pattern used by lowering:
DlgLine::SpeakerLine { speaker: (name, span), text, loc_key } => {
    // name: &str, span: SimpleSpan — span excludes '@'
    // span is the span of the identifier token only
}
DlgLine::SpeakerTag((name, span)) => {
    // name: &str, span: SimpleSpan — span excludes '@'
}
```

### Verified pattern: push_token_for_span (already in queries.rs)

```rust
// Source: writ-lsp/src/queries.rs:1239-1262
// Existing helper — accepts SimpleSpan, converts to UTF-16 for SemanticToken:
fn push_token_for_span(
    tokens: &mut Vec<RawSemanticToken>,
    source: &str,
    span: &SimpleSpan,   // byte-offset range
    token_type: u32,
) { ... }
// Reuse this directly — no changes needed to this function.
```

### Verified pattern: parser.rs speaker span excludes `@`

```rust
// Source: writ-parser/src/parser.rs:2051-2054
let speaker_line = just(Token::At)
    .ignore_then(                                       // @ consumed, span not captured
        select! { Token::Ident(name) => name }
            .map_with(|n, e| (n, e.span())),           // span = Ident span only
    )
    // ...
```

### Test pattern: dialogue semantic token test

```rust
#[test]
fn test_semantic_tokens_dialogue_speaker() {
    // dlg with two speaker lines — no entity declarations needed for CST-only test
    let src = "dlg intro {\n    @Alice Hello there.\n    @Bob Greetings!\n}\n";

    let tokens = collect_dialogue_speaker_tokens(src);

    // Verify Alice gets a dialogue speaker token
    let alice_offset = src.find("Alice Hello").unwrap();
    let alice_pos = crate::convert::offset_to_position(src, alice_offset);
    let alice_token = tokens
        .iter()
        .find(|t| t.line == alice_pos.line && t.start_char == alice_pos.character);
    assert!(
        alice_token.is_some(),
        "expected dialogue speaker token for Alice, got: {:?}",
        tokens.iter().map(|t| (t.line, t.start_char, t.token_type)).collect::<Vec<_>>()
    );
    assert_eq!(alice_token.unwrap().token_type, TOKEN_TYPE_DIALOGUE_SPEAKER);

    // Verify Bob also gets a token
    let bob_offset = src.find("Bob Greetings").unwrap();
    let bob_pos = crate::convert::offset_to_position(src, bob_offset);
    let bob_token = tokens
        .iter()
        .find(|t| t.line == bob_pos.line && t.start_char == bob_pos.character);
    assert!(bob_token.is_some(), "expected dialogue speaker token for Bob");
    assert_eq!(bob_token.unwrap().token_type, TOKEN_TYPE_DIALOGUE_SPEAKER);
}

#[test]
fn test_semantic_tokens_includes_dialogue_speaker() {
    // Full collect_semantic_tokens returns dialogue tokens merged with decl tokens
    // Use a full pipeline test (build_typed_ast_full for entities + CST for dlg)
    let src = "pub entity Alice {}\ndlg intro {\n    @Alice Hello.\n}\n";
    let (ast, interner, _type_env) = build_typed_ast_full(src);
    let tokens = collect_semantic_tokens(&ast, &interner, src, FileId(0));

    // Should include TOKEN_TYPE_ENTITY for Alice declaration AND
    // TOKEN_TYPE_DIALOGUE_SPEAKER for @Alice in dialogue
    let has_entity = tokens.iter().any(|t| t.token_type == TOKEN_TYPE_ENTITY);
    let has_speaker = tokens.iter().any(|t| t.token_type == TOKEN_TYPE_DIALOGUE_SPEAKER);
    assert!(has_entity, "expected entity token for Alice declaration");
    assert!(has_speaker, "expected dialogue speaker token for @Alice in dlg");
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| TOKEN_TYPE_DIALOGUE_SPEAKER defined, never emitted (`#[allow(dead_code)]`) | TOKEN_TYPE_DIALOGUE_SPEAKER emitted for CST-level `@Speaker` spans | Phase 58 | DIFF-01 fully satisfied |
| Phase 54 open question: "dialogue speaker resolution approximate via entity-typed Var" | Definitive: CST re-parse extracts exact `@Speaker` spans | Phase 58 | Accurate — no false positives from non-speaker entity vars |

**Deprecated/outdated:**
- `#[allow(dead_code)]` on `TOKEN_TYPE_DIALOGUE_SPEAKER` in `queries.rs:987` — remove after Phase 58.

## Open Questions

None. All previously open questions are resolved by direct code inspection:

1. **Speaker span includes `@` or not:** RESOLVED — parser uses `ignore_then` to consume `@`; speaker span covers identifier only (`parser.rs:2051-2054`).

2. **`Item` enum variant name for Dlg:** RESOLVED — confirmed `Item::Dlg(Spanned<DlgDecl>)` at `cst.rs:67`.

3. **`parse` return type shape:** RESOLVED — returns `Option<Vec<Spanned<Item>>>` (flat list, not `Program` struct); confirmed in `parser.rs:3310` and `analysis_host.rs:46`.

4. **Whether parse errors cause full `None` return:** LOW risk — the guard `let Some(items) = items_opt else { return tokens }` handles it gracefully regardless.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`#[test]`) |
| Config file | none — workspace-level `cargo test` |
| Quick run command | `cargo test -p writ-lsp --lib` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DIFF-01 | `@Speaker` names in dlg blocks produce `TOKEN_TYPE_DIALOGUE_SPEAKER` tokens | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_semantic_tokens_dialogue_speaker` | ❌ Wave 0 |
| DIFF-01 | `collect_semantic_tokens` output includes dialogue speaker tokens alongside entity tokens | unit | `cargo test -p writ-lsp --lib -- queries::tests::test_semantic_tokens_includes_dialogue_speaker` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-lsp --lib`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] New function `collect_dialogue_speaker_tokens` in `writ-lsp/src/queries.rs`
- [ ] New helper functions `collect_speaker_tokens_in_dlg_body` and `collect_dlg_if_else_speakers` in `queries.rs`
- [ ] Extend `collect_semantic_tokens` to call and merge speaker tokens
- [ ] Remove `#[allow(dead_code)]` from `TOKEN_TYPE_DIALOGUE_SPEAKER`
- [ ] Two new tests added to the existing `#[cfg(test)]` block in `queries.rs`

*(No new files required — all changes are to the existing `writ-lsp/src/queries.rs`.)*

## Sources

### Primary (HIGH confidence)
- Direct codebase inspection — `writ-lsp/src/queries.rs` lines 979-992: token type constants; `TOKEN_TYPE_DIALOGUE_SPEAKER = 4` is `#[allow(dead_code)]`
- Direct codebase inspection — `writ-lsp/src/backend.rs` lines 90-111: `SemanticTokensLegend` registers `dialogueSpeaker` at index 4; confirmed legend order
- Direct codebase inspection — `writ-parser/src/cst.rs` lines 11, 59-67, 870-895: `Spanned<T> = (T, SimpleSpan)`; `Item::Dlg(Spanned<DlgDecl>)` at line 67; `DlgLine::SpeakerLine`/`SpeakerTag` confirmed
- Direct codebase inspection — `writ-parser/src/parser.rs` lines 3307-3311: `parse` returns `Option<Vec<Spanned<Item>>>`, not `Option<Program>`; lines 2051-2054: `ignore_then` confirms `@` excluded from speaker span
- Direct codebase inspection — `writ-compiler/src/lower/dialogue.rs` lines 35-166: `lower_dialogue` shows dlg becomes `AstFnDecl`; speaker spans consumed and not forwarded to TypedAst
- Direct codebase inspection — `writ-compiler/src/ast/stmt.rs` lines 6-9: NO `DlgDecl` variant in AST; dialogue lowered to `Fn`
- Direct codebase inspection — `writ-compiler/src/check/ir.rs` lines 285-334: `TypedDecl` has no Dialogue variant
- Direct codebase inspection — `.planning/v5.0-MILESTONE-AUDIT.md` lines 140-150: exact gap description, evidence, and fix scope
- Direct codebase inspection — `writ-lsp/Cargo.toml` line 22: `writ-parser` is already a dep of `writ-lsp`
- Direct codebase inspection — `writ-lsp/src/analysis_host.rs` lines 46, 208: confirmed `writ_parser::parse` call pattern produces `Vec<Spanned<Item>>`

### Secondary (MEDIUM confidence)
- `writ-compiler/src/lower/dialogue.rs:192,298`: Destructuring patterns `DlgLine::SpeakerLine { speaker: (name, span), .. }` and `DlgLine::SpeakerTag((name, span))` — confirms span available at CST and how it's accessed

### Tertiary (LOW confidence)
None — all claims verified from primary sources.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new deps, same codebase, same patterns as existing code
- Architecture: HIGH — direct reading confirms CST has spans, TypedAst does not; re-parse approach is clean, precedented, and used by existing test infrastructure
- Pitfalls: HIGH — derived from reading parser and lowering code; all edge cases identified from actual code
- Implementation specifics (span boundaries, API shapes): HIGH — all verified by reading parser.rs and analysis_host.rs

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (all deps pinned; LSP spec is stable; internal APIs are stable)
