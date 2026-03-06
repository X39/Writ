# Phase 54: LSP Navigation and Completions - Research

**Researched:** 2026-03-14
**Domain:** Language Server Protocol — navigation features, completions, semantic highlighting
**Confidence:** HIGH

## Summary

Phase 54 wires the six remaining LSP capabilities (hover, go-to-definition, find-references, completions, signature help, semantic highlighting) into the existing `writ-lsp` tower-lsp backend. The Phase 53 skeleton already provides a working diagnostic pipeline, document store, and project-mode analysis via `AnalysisHost`. Phase 54 must extend that analysis layer to expose typed AST data — `TypedAst`, `TyInterner`, and `TypeEnv` — back to the LSP handlers, and then implement each LSP method as a position-to-query operation.

The Writ compiler already produces all the information required for these features. The `TypedAst` carries per-expression `Ty` tags and `SimpleSpan` positions. The `DefMap` maps every name to its declaration site with `FileId` and `SimpleSpan`. The `TypeEnv` holds fields, methods, entity components, and function signatures indexed by `DefId`. The main engineering challenge is (1) extending `AnalysisResult` to surface typed data alongside diagnostics, (2) writing a span-to-node query that converts a cursor `(line, character)` position back to a byte offset and walks the TypedAst to find the node at that offset, and (3) implementing each LSP handler using that query plus the existing maps.

`TyInterner::display` currently produces `"struct"` / `"entity"` / `"class"` for named types rather than their actual names. It must be upgraded to a `display_with_def_map(&self, ty: Ty, def_map: &DefMap) -> String` variant that looks up `DefEntry::name` for named kinds. This is required for meaningful hover text.

**Primary recommendation:** Extend `AnalysisResult` to carry `Option<TypedAst>` and `Option<TyInterner>`, add a `position_to_byte_offset` + typed-node-walker, then implement each LSP handler in `backend.rs` delegating to thin query helpers in a new `queries.rs` module.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LSP-02 | Keyword and type name completions when typing identifiers | Prelude constants, DefMap global names; `CompletionItem` list returned from `textDocument/completion` |
| LSP-03 | Dot-completions for struct/class fields, methods, and entity components | TypeEnv::struct_fields, impl_index, entity_components; triggered on `.` after typed receiver expr |
| LSP-04 | Hover any identifier to see type, signature, or definition info | TypedExpr::Var/Call/Field nodes carry Ty + span; TyInterner::display for rendering |
| LSP-05 | Go-to-definition on any identifier | DefMap::arena carries FileId + name_span per DefId; TypedExpr::Call::callee_def_id is already stored |
| LSP-06 | Find all references across all project files | Must walk entire TypedAst to collect all use-sites with matching DefId; cross-file scanning |
| LSP-07 | Signature help with active parameter highlighted | FnSig::params from TypeEnv; tower-lsp SignatureHelp + ParameterInformation |
| DIFF-01 | Semantic highlighting for entity names, component types, dialogue speakers, keywords | LSP semantic tokens push model; custom token types registered in server capabilities |
| DIFF-02 | Dot-completion on entity-typed expressions shows extern component types | TypeEnv::entity_components indexed by DefId; DefKind::Entity in TyKind to resolve DefId |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tower-lsp | 0.20 | LSP protocol server framework | Already in use (Phase 53) |
| lsp-types | 0.94.1 | LSP protocol type definitions | Already in use (Phase 53) |
| tokio | 1 | Async runtime | Already in use (Phase 53) |
| dashmap | 6 | Thread-safe concurrent maps | Already in use for document_map |

No new crate dependencies are required for Phase 54. All LSP types needed (CompletionItem, Hover, Location, SignatureHelp, SemanticTokens) are already in `lsp-types 0.94.1`.

**Installation:** No new deps — all required types are already present.

## Architecture Patterns

### Recommended Project Structure
```
writ-lsp/src/
├── lib.rs              # module declarations (add queries module)
├── main.rs             # binary entry point (unchanged)
├── backend.rs          # tower-lsp LanguageServer impl (extend ServerCapabilities + handlers)
├── analysis_host.rs    # extend AnalysisResult to carry TypedAst + TyInterner
├── convert.rs          # existing span/diagnostic conversion (add ty_display helper)
└── queries.rs          # NEW: position-to-node walker + per-feature query functions
```

### Pattern 1: Extended AnalysisResult

**What:** Extend `AnalysisResult` to carry the typed analysis output alongside diagnostics.

**When to use:** All LSP features (hover, goto-def, find-refs, completions, sig help, sem tokens) need the typed data. The analysis must now preserve — not discard — `TypedAst` and `TyInterner`.

**Current analysis_host.rs (to change):**
```rust
// Phase 53: typecheck result is silently dropped
match typecheck_result {
    Ok((_typed, _interner, type_diags)) => {
        all_diags.extend(type_diags);
    }
    ...
}
```

**Phase 54 change:**
```rust
pub struct AnalysisResult {
    pub diagnostics: Vec<Diagnostic>,
    pub file_sources: Vec<(FileId, String, String)>,
    /// NEW: typed output when typecheck succeeds
    pub typed_ast: Option<writ_compiler::check::ir::TypedAst>,
    pub ty_interner: Option<writ_compiler::check::ty::TyInterner>,
}
```

Store `typed` and `interner` from `Ok((typed, interner, type_diags))` into the result.

### Pattern 2: Position to Byte Offset Conversion

**What:** Convert a cursor `(line, character)` LSP Position (0-based, UTF-16 column) back to a byte offset in the source text.

**When to use:** Every LSP handler (hover, goto-def, completion, sig-help) is triggered by a cursor position and must find which AST node sits at that position.

**Implementation in queries.rs:**
```rust
/// Convert an LSP Position to a byte offset in the given source text.
/// LSP Position.character is a UTF-16 code-unit count.
pub fn position_to_byte_offset(source: &str, pos: lsp_types::Position) -> Option<usize> {
    let mut current_line: u32 = 0;
    let mut iter = source.char_indices().peekable();
    // advance past (pos.line) newlines
    while current_line < pos.line {
        match iter.next() {
            Some((_, '\n')) => current_line += 1,
            None => return None,
            _ => {}
        }
    }
    // now advance pos.character UTF-16 code units
    let mut utf16_col: u32 = 0;
    while utf16_col < pos.character {
        match iter.next() {
            Some((_, '\n')) | None => return None,
            Some((_, ch)) => utf16_col += ch.len_utf16() as u32,
        }
    }
    iter.next().map(|(byte_idx, _)| byte_idx)
        .or_else(|| Some(source.len())) // end-of-file position
}
```

This mirrors the inverse of `offset_to_position` already in `convert.rs`.

### Pattern 3: TypedExpr Node at Offset

**What:** Walk a `TypedAst` to find the innermost `TypedExpr` or `TypedStmt` whose span contains a given byte offset.

**When to use:** Hover, goto-def, and signature-help all query: "what is at this cursor position?"

**Implementation approach:**
```rust
/// Walk a TypedAst to find the TypedExpr whose span most-narrowly contains `byte_offset`.
/// Returns the expression and its DefId (if the expression is a Call or Var with callee_def_id).
pub fn expr_at_offset(
    ast: &writ_compiler::check::ir::TypedAst,
    file_id: FileId,
    byte_offset: usize,
) -> Option<&writ_compiler::check::ir::TypedExpr> {
    // Walk all Fn and Impl method bodies in the TypedAst
    for decl in &ast.decls {
        match decl {
            TypedDecl::Fn { body, .. } => {
                if let Some(found) = find_in_expr(body, byte_offset) {
                    return Some(found);
                }
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    if let Some(found) = find_in_expr(body, byte_offset) {
                        return Some(found);
                    }
                }
            }
            TypedDecl::Const { value, .. } | TypedDecl::Global { value, .. } => {
                if let Some(found) = find_in_expr(value, byte_offset) {
                    return Some(found);
                }
            }
            _ => {} // Struct/Entity/Enum/etc. have no expr bodies
        }
    }
    None
}
```

The walker uses span containment: `span.start <= offset < span.end`. When multiple nodes contain the offset (nested expressions), the innermost (most-narrow span) wins.

### Pattern 4: Hover Handler

**What:** tower-lsp `hover` method implementation — LSP request `textDocument/hover`.

**Registration in `initialize` (ServerCapabilities):**
```rust
hover_provider: Some(HoverProviderCapability::Simple(true)),
```

**Handler in backend.rs:**
```rust
async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
    let uri_str = params.text_document_position_params.text_document.uri.to_string();
    let pos = params.text_document_position_params.position;

    let source = match self.document_map.get(&uri_str) { ... };
    let analysis = self.run_analysis_for(&uri_str).await;

    let Some(typed_ast) = analysis.typed_ast else { return Ok(None) };
    let Some(interner) = analysis.ty_interner else { return Ok(None) };

    let byte_offset = queries::position_to_byte_offset(&source, pos)?;
    let expr = queries::expr_at_offset(&typed_ast, file_id, byte_offset)?;

    let hover_text = queries::hover_text_for_expr(expr, &typed_ast.def_map, &interner);
    Ok(Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: hover_text,
        }),
        range: Some(convert::span_to_range(&source, &expr.span())),
    }))
}
```

### Pattern 5: Go-to-Definition Handler

**What:** tower-lsp `goto_definition` — LSP request `textDocument/definition`.

**Registration:**
```rust
definition_provider: Some(OneOf::Left(true)),
```

**Logic:** Find the `TypedExpr` at the cursor. Extract its `DefId` (from `TypedExpr::Call::callee_def_id`, `TypedExpr::Var` via DefMap name lookup, or `TypedExpr::New::target_def_id`). Look up `DefMap::get_entry(def_id)` to get `FileId` and `name_span`. Convert to a `Location`.

**Key API:**
```rust
// DefEntry fields used for goto-def
let entry = typed_ast.def_map.get_entry(def_id);
let target_uri = file_id_to_url(entry.file_id, &file_sources);
let range = convert::span_to_range(source_for_file(entry.file_id), &entry.name_span);
Ok(Some(GotoDefinitionResponse::Scalar(Location { uri: target_uri, range })))
```

Synthetic definitions (`FileId(u32::MAX)` for log/dialogue builtins) have no meaningful location — return `None` for those.

### Pattern 6: Find References

**What:** tower-lsp `references` — LSP request `textDocument/references`.

**Registration:**
```rust
references_provider: Some(OneOf::Left(true)),
```

**Logic:** Find the `DefId` at the cursor (same as goto-def). Then walk the entire `TypedAst` collecting every `TypedExpr` node whose `DefId` matches. Build a `Vec<Location>`.

This is the most expensive handler — O(n * m) where n = all expressions, m = number of files. Acceptable for v5.0 since the full pipeline already runs on every keystroke.

### Pattern 7: Completions

**What:** tower-lsp `completion` — LSP request `textDocument/completion`.

**Registration:**
```rust
completion_provider: Some(CompletionOptions {
    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
    ..Default::default()
}),
```

**Two completion modes:**

**Mode A — identifier completions (LSP-02):** Triggered when no trigger character. Return all items from:
- Prelude names (`PRELUDE_PRIMITIVE_NAMES`, `PRELUDE_TYPE_NAMES`, `PRELUDE_CONTRACT_NAMES`)
- All public DefMap entries (functions, structs, entities, enums, consts)
- Keywords: `fn`, `struct`, `entity`, `enum`, `impl`, `contract`, `let`, `mut`, `if`, `else`, `for`, `while`, `return`, `spawn`, `yield`, `new`, `using`, `namespace`, `extern`, `global`, `const`, `pub`, `priv`

**Mode B — dot completions (LSP-03, DIFF-02):** Triggered by `.`. Determine the expression immediately left of the dot. Look up its `Ty`. Based on `TyKind`:
- `Struct(def_id)` or `Class(def_id)`: fields from `TypeEnv::struct_fields[def_id]` + methods from `TypeEnv::impl_index[def_id]`
- `Entity(def_id)`: entity properties from `TypeEnv::entity_fields[def_id]` + methods from impl_index + extern component names from `TypeEnv::entity_components[def_id]` (DIFF-02)
- `Array(_)`: built-in methods `push`, `pop`, `len`, `is_empty`
- `Option(_)`: `is_some`, `is_none`, `unwrap`

**Critical consideration:** At the point of typing `expr.`, the source is syntactically incomplete. The parser will fail or produce an error AST. Completions must therefore:
1. Use the last-successful full parse (cache it alongside the document map), OR
2. Strip the trailing `.` from the current text and re-run analysis on the modified source.

The recommended approach for v5.0 is Option 2: strip the trailing dot, run analysis, find the type of the expression immediately before the cursor. This avoids needing a separate completion cache.

### Pattern 8: Signature Help

**What:** tower-lsp `signature_help` — LSP request `textDocument/signatureHelp`.

**Registration:**
```rust
signature_help_provider: Some(SignatureHelpOptions {
    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
    ..Default::default()
}),
```

**Logic:** Walk back from cursor to find the innermost active function call. Count commas between the opening `(` and the cursor to determine the active parameter index. Look up the callee's `FnSig` from `TypeEnv::fn_sigs`.

```rust
// Source: lsp-types 0.94 SignatureHelp struct
SignatureHelp {
    signatures: vec![SignatureInformation {
        label: format_fn_sig(&sig, &interner, &def_map),
        documentation: None,
        parameters: Some(sig.params.iter().map(|(name, ty)| {
            ParameterInformation {
                label: ParameterLabel::Simple(
                    format!("{}: {}", name, interner.display(*ty))
                ),
                documentation: None,
            }
        }).collect()),
        active_parameter: Some(active_param_idx),
    }],
    active_signature: Some(0),
    active_parameter: Some(active_param_idx),
}
```

### Pattern 9: Semantic Tokens (DIFF-01)

**What:** tower-lsp semantic tokens full request — LSP request `textDocument/semanticTokens/full`.

**Registration:**
```rust
semantic_tokens_provider: Some(
    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
        SemanticTokensRegistrationOptions {
            text_document_registration_options: Default::default(),
            semantic_tokens_options: SemanticTokensOptions {
                work_done_progress_options: Default::default(),
                legend: SemanticTokensLegend {
                    token_types: vec![
                        SemanticTokenType::KEYWORD,
                        SemanticTokenType::TYPE,          // struct/class/enum names
                        SemanticTokenType::new("entity"), // Writ entity declarations
                        SemanticTokenType::new("component"), // extern component types
                        SemanticTokenType::new("dialogueSpeaker"), // entity speakers in dialogue
                        SemanticTokenType::FUNCTION,
                        SemanticTokenType::VARIABLE,
                        SemanticTokenType::PARAMETER,
                    ],
                    token_modifiers: vec![],
                },
                range: None,
                full: Some(SemanticTokensFullOptions::Bool(true)),
            },
            static_registration_options: Default::default(),
        }
    )
),
```

**Encoding:** LSP semantic tokens use a delta-encoded relative format. Each token is `(deltaLine, deltaStartChar, length, tokenType, tokenModifiers)`. The builder pattern from `lsp-types` handles delta computation.

**Token sources:** Walk the `TypedAst`. For each expression/declaration:
- `TypedDecl::Entity` name span → `entity` token type
- `TypedDecl::Struct`/`Class` name span → `type` token type
- `TypedDecl::Fn` name span → `function` token type
- `TypedExpr::Var` where DefKind is Entity → `entity` token modifier
- Component names in `AstEntityDecl::component_slots` → `component` token type
- Dialogue speaker labels in `AstEntityHook` → `dialogueSpeaker` token type

The token type must also be registered in the VS Code extension's `semanticTokenScopes` (package.json contributes section) for full colorization support. However, the server can emit the tokens without client-side configuration — they just won't display with custom colors until the extension is updated.

### Pattern 10: TyInterner::display Upgrade

**What:** Named types (`TyKind::Struct(DefId)`, `TyKind::Class(DefId)`, `TyKind::Entity(DefId)`, `TyKind::Enum(DefId)`) currently display as `"struct"`, `"class"`, `"entity"`, `"enum"`. For hover text to be useful, they must display as the actual name.

**Fix:** Add an overloaded display method that accepts the DefMap:

```rust
// In ty.rs
pub fn display_named(&self, ty: Ty, def_map: &crate::resolve::def_map::DefMap) -> String {
    match self.kind(ty) {
        TyKind::Struct(def_id) | TyKind::Class(def_id) |
        TyKind::Entity(def_id) | TyKind::Enum(def_id) => {
            def_map.get_entry(*def_id).name.clone()
        }
        TyKind::Array(elem) => format!("{}[]", self.display_named(*elem, def_map)),
        TyKind::Option(inner) => format!("Option<{}>", self.display_named(*inner, def_map)),
        TyKind::Result(ok, err) => format!(
            "Result<{}, {}>",
            self.display_named(*ok, def_map),
            self.display_named(*err, def_map)
        ),
        TyKind::Func { params, ret } => {
            let ps: Vec<String> = params.iter()
                .map(|p| self.display_named(*p, def_map)).collect();
            format!("fn({}) -> {}", ps.join(", "), self.display_named(*ret, def_map))
        }
        _ => self.display(ty), // primitives, TaskHandle, GenericParam, etc.
    }
}
```

This should be added to `writ-compiler/src/check/ty.rs` and used by the LSP hover and completion rendering code. It does not break any existing callers of `display`.

### Anti-Patterns to Avoid

- **Running full pipeline per handler:** The current `publish_diagnostics_for` reruns the full compiler pipeline on every save/change. Do NOT add a second pipeline call per LSP request. Instead, cache the last `AnalysisResult` in `Backend` (in a `DashMap<String, Arc<AnalysisResult>>`) so hover/goto-def/completion can reuse the already-computed result from the most recent analysis run.

- **Parallel AnalysisResult + document_map updates without coordination:** The document map is updated atomically via DashMap. Cache invalidation must happen in `publish_diagnostics_for` — when a new analysis completes, atomically replace the cached result for that URI.

- **Re-running analysis synchronously in async handlers:** Hover/goto-def/completion should read from the analysis cache, not call `spawn_blocking` again. Only `publish_diagnostics_for` invokes the full pipeline.

- **Leaking strings for every analysis run:** The current `Box::leak` pattern is intentional for the `'static str` contract. However, Phase 54 must not double-leak: if the cached `AnalysisResult` is replaced, the old leaked strings remain. This is acceptable for v5.0 (a running LSP server holds on the order of O(files) leaks, which is negligible), but should be documented as known memory overhead.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| LSP semantic token delta encoding | Manual delta computation | `lsp-types::SemanticTokens` + builder | Fiddly off-by-one errors; tokens must be sorted by position |
| Position ↔ offset conversion | Ad-hoc char iteration | `convert::offset_to_position` (already exists) + new inverse | Established pattern; UTF-16 handling is subtle |
| Completion item kind constants | Custom u32 tags | `lsp_types::CompletionItemKind::*` | Spec-defined integer values |
| Hover markdown formatting | Custom renderer | `lsp_types::MarkupContent` with `MarkupKind::Markdown` | Standard LSP pattern |
| Finding definition spans | Walking AST from scratch | `DefMap::get_entry(def_id).name_span` + `FileId` | DefEntry already has the answer |

**Key insight:** The compiler already computed everything needed. Phase 54 is primarily a query layer on top of already-computed data, not new compilation logic.

## Common Pitfalls

### Pitfall 1: Incomplete TypedDecl body coverage
**What goes wrong:** Only `TypedDecl::Fn` bodies are walked. Methods in `TypedDecl::Impl`, constant values in `TypedDecl::Const`, and global values in `TypedDecl::Global` are skipped. Hover and goto-def silently fail inside those contexts.
**Why it happens:** The `Fn` case is the most obvious. Impl methods and const bodies are easily missed.
**How to avoid:** The node walker must cover all four variants: `Fn { body }`, `Impl { methods }` (each method body), `Const { value }`, `Global { value }`.
**Warning signs:** Hover works inside free functions but fails inside `impl` blocks.

### Pitfall 2: TyInterner::display shows "struct" not the struct name
**What goes wrong:** Hover over a variable of type `Potion` shows `"struct"` instead of `"Potion"`.
**Why it happens:** `TyInterner::display` does not have access to the `DefMap`, so it cannot look up the name.
**How to avoid:** Add `display_named(&self, ty: Ty, def_map: &DefMap) -> String` as described above.
**Warning signs:** All hover tooltips for user-defined types show their kind word, not their name.

### Pitfall 3: Completion triggered on incomplete source
**What goes wrong:** When the user types `player.` the source has a trailing `.` which causes a parse error. The analysis pipeline produces no typed AST, so dot-completions return nothing.
**Why it happens:** Dot-completions are triggered precisely when the source is syntactically invalid.
**How to avoid:** For completion requests, re-run analysis on source text with the trigger character removed (strip the `.` at the cursor position), then query the expression immediately before the removed character.
**Warning signs:** Dot-completions always return empty lists.

### Pitfall 4: Cached AnalysisResult races with live document updates
**What goes wrong:** A document update races with a hover request. The hover handler reads a stale cache from a previous version.
**Why it happens:** DashMap provides atomic per-entry operations but not cross-map atomicity.
**How to avoid:** Stale cache responses are acceptable for hover/goto-def (return slightly out-of-date data). The LSP spec does not require hover to reflect the current in-flight edit. The existing `publish_diagnostics_for` path updates the cache after each analysis completes.
**Warning signs:** Hard to detect — data is correct but from a previous version. Non-critical for v5.0.

### Pitfall 5: Semantic tokens out of order
**What goes wrong:** `SemanticTokens` encoding is undefined if tokens are not sorted by position (line, then character). The editor silently ignores or misrenders them.
**Why it happens:** Walking the TypedAst does not guarantee position order — impl methods may appear in source before or after standalone functions.
**How to avoid:** Collect all `(byte_offset, length, token_type)` tuples first, then sort by byte offset before encoding the delta sequence.
**Warning signs:** Some tokens highlight correctly; others do not highlight at all or are off by one token.

### Pitfall 6: Find-references walking only the triggering file
**What goes wrong:** Find-references only returns uses in the current file, not across all project files.
**Why it happens:** `TypedAst` from standalone analysis covers only one file. Multi-file references need project-mode analysis.
**How to avoid:** Find-references must always use `AnalysisHost::analyze_project` (or the cached project-mode result), never standalone analysis.
**Warning signs:** Find-references misses uses in other files.

### Pitfall 7: Synthetic DefEntry (log::*, dialogue builtins) in goto-def
**What goes wrong:** Go-to-definition on `log::info(...)` or `say(...)` tries to navigate to `FileId(u32::MAX)`. The file URL cannot be constructed, crashing or returning a nonsense location.
**Why it happens:** Synthetic entries have `file_id = FileId(u32::MAX)` and zero spans as sentinels.
**How to avoid:** Before building a `Location` for goto-def, check `entry.file_id == FileId(u32::MAX)` and return `None` instead.
**Warning signs:** Go-to-definition on builtin calls panics or navigates to a garbage position.

## Code Examples

Verified patterns from existing codebase:

### Analysis result cache in Backend
```rust
// In backend.rs — add to Backend struct
pub(crate) analysis_cache: DashMap<String, Arc<crate::analysis_host::AnalysisResult>>,
```

### Hover implementation skeleton
```rust
// Source: tower-lsp 0.20 LanguageServer trait
async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
    let uri_str = params.text_document_position_params
        .text_document.uri.to_string();
    let pos = params.text_document_position_params.position;

    let source = match self.document_map.get(&uri_str) {
        Some(s) => s.clone(),
        None => return Ok(None),
    };
    let cache_entry = match self.analysis_cache.get(&uri_str) {
        Some(e) => e.clone(),
        None => return Ok(None),
    };
    let (typed_ast, interner) = match (&cache_entry.typed_ast, &cache_entry.ty_interner) {
        (Some(t), Some(i)) => (t, i),
        _ => return Ok(None),
    };

    let byte_offset = match crate::queries::position_to_byte_offset(&source, pos) {
        Some(o) => o,
        None => return Ok(None),
    };
    // ... find expr, build hover text
    Ok(None) // replaced with real impl
}
```

### Goto-definition location building
```rust
// Construct a Location from a DefEntry
fn def_entry_to_location(
    entry: &writ_compiler::resolve::def_map::DefEntry,
    file_sources: &[(FileId, String, String)],
    trigger_uri: &lsp_types::Url,
) -> Option<lsp_types::Location> {
    if entry.file_id == writ_diagnostics::FileId(u32::MAX) {
        return None; // synthetic builtin
    }
    let (_, display_path, source) = file_sources.iter()
        .find(|(fid, _, _)| *fid == entry.file_id)?;
    let uri = display_path_to_url(display_path, trigger_uri);
    let range = crate::convert::span_to_range(source, &entry.name_span);
    Some(lsp_types::Location { uri, range })
}
```

### Semantic tokens builder
```rust
// Source: lsp-types 0.94 SemanticTokensBuilder pattern
let mut builder = lsp_types::SemanticTokensBuilder::new();
// tokens must be added in ascending line/character order
for (line, char, len, token_type) in sorted_tokens {
    builder.push(line, char, len, token_type, 0);
}
let data = builder.build();
lsp_types::SemanticTokens { result_id: None, data }
```

Note: `lsp-types 0.94` does not expose a `SemanticTokensBuilder` struct. The delta encoding must be implemented manually or via the `SemanticTokensBuilder` from `lsp-types`. Verify whether `SemanticTokensBuilder` is exported in 0.94.1; if not, hand-roll the delta encoding (it is 10-15 lines and is the one acceptable custom implementation in this phase).

### Delta encoding for semantic tokens (if builder not available)
```rust
// Manual delta encoding — acceptable since the algorithm is trivial
let mut prev_line = 0u32;
let mut prev_start = 0u32;
let mut data = Vec::new();
for (abs_line, abs_start, length, token_type) in sorted_tokens {
    let delta_line = abs_line - prev_line;
    let delta_start = if delta_line == 0 { abs_start - prev_start } else { abs_start };
    data.push(SemanticToken {
        delta_line,
        delta_start,
        length,
        token_type,
        token_modifiers_bitset: 0,
    });
    prev_line = abs_line;
    prev_start = abs_start;
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Discard TypedAst after typecheck | Carry TypedAst in AnalysisResult | Phase 54 | Enables all navigation features |
| No analysis cache | DashMap cache in Backend | Phase 54 | Decouples analysis from per-request latency |
| `TyInterner::display` returns kind word | `display_named` with DefMap lookup | Phase 54 | Hover shows actual names |

**Deprecated/outdated:**
- `ServerCapabilities { ..Default::default() }` in `initialize`: Phase 53 left all optional capabilities as None. Phase 54 must populate them explicitly.

## Open Questions

1. **SemanticTokensBuilder availability in lsp-types 0.94.1**
   - What we know: lsp-types 0.94 includes `SemanticToken` struct and `SemanticTokens`
   - What's unclear: Whether a builder/helper for delta encoding ships with the crate
   - Recommendation: Check at plan time with `cargo doc --open` on lsp-types. If absent, hand-roll the 15-line delta encoder shown above — it is trivial and well-understood.

2. **Hover on tokens in incomplete expressions**
   - What we know: TypedExpr::Error nodes carry a span and `Ty::Error`
   - What's unclear: Whether hover on syntactically invalid code should show something or nothing
   - Recommendation: Return `None` (no hover) when the expression at the cursor is `TypedExpr::Error`. This is the standard LSP behavior.

3. **Dialogue speaker highlight sources**
   - What we know: `AstEntityHook::contract` carries the hook name (e.g., "OnInteract"). Speaker identifiers in dialogue are embedded in `AstFnDecl` bodies as calls.
   - What's unclear: How speaker labels surface in the TypedAst — they may appear as `TypedExpr::Var` with a matching `DefKind::Entity` DefId in the DefMap, or they may not be stored as separate nodes.
   - Recommendation: For DIFF-01, emit semantic tokens for Entity declarations and entity-typed variables. Dialogue speaker resolution can be approximated by flagging any `TypedExpr::Var` whose resolved `Ty` is `TyKind::Entity(_)`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`#[test]`) |
| Config file | none — workspace-level `cargo test` |
| Quick run command | `cargo test -p writ-lsp` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LSP-02 | Keyword completions returned for identifier context | unit | `cargo test -p writ-lsp test_keyword_completions` | ❌ Wave 0 |
| LSP-03 | Dot completions for struct fields returned | unit | `cargo test -p writ-lsp test_dot_completions_struct` | ❌ Wave 0 |
| LSP-04 | Hover over variable returns type string | unit | `cargo test -p writ-lsp test_hover_var_type` | ❌ Wave 0 |
| LSP-04 | Hover over function call returns signature | unit | `cargo test -p writ-lsp test_hover_call_sig` | ❌ Wave 0 |
| LSP-05 | Goto-def on function call returns declaration location | unit | `cargo test -p writ-lsp test_goto_def_fn` | ❌ Wave 0 |
| LSP-05 | Goto-def on builtin (log::info) returns None | unit | `cargo test -p writ-lsp test_goto_def_builtin_none` | ❌ Wave 0 |
| LSP-06 | Find-refs collects all use-sites of a definition | unit | `cargo test -p writ-lsp test_find_references` | ❌ Wave 0 |
| LSP-07 | Signature help returns correct parameter at active index | unit | `cargo test -p writ-lsp test_signature_help_param` | ❌ Wave 0 |
| DIFF-01 | Semantic tokens emitted for entity declarations | unit | `cargo test -p writ-lsp test_semantic_tokens_entity` | ❌ Wave 0 |
| DIFF-02 | Dot completions on entity show component names | unit | `cargo test -p writ-lsp test_dot_completions_entity_components` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-lsp`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-lsp/src/queries.rs` — the new query module (position_to_byte_offset, expr_at_offset, hover_text_for_expr)
- [ ] Unit tests for all LSP-0N and DIFF-0N requirements listed above — each test exercises the relevant query function on hand-crafted source strings without spawning a live LSP server

*(All gaps are in new files; existing test infrastructure in `analysis_host.rs` and `convert.rs` is unchanged.)*

## Sources

### Primary (HIGH confidence)
- Writ codebase direct inspection — `writ-lsp/src/`, `writ-compiler/src/check/`, `writ-compiler/src/resolve/` — all findings from direct code reading
- `writ-compiler/src/check/ty.rs` — `TyInterner::display` limitation confirmed by reading source (line 147–172)
- `writ-compiler/src/check/env.rs` — `TypeEnv` fields confirmed (fn_sigs, struct_fields, entity_components, impl_index, entity_fields, component_fields)
- `writ-compiler/src/check/ir.rs` — `TypedAst`, `TypedExpr` span/ty fields confirmed
- `writ-compiler/src/resolve/def_map.rs` — `DefEntry` fields (file_id, name_span, kind) confirmed
- `writ-lsp/Cargo.toml` — tower-lsp 0.20, lsp-types 0.94.1 confirmed

### Secondary (MEDIUM confidence)
- tower-lsp 0.20 `LanguageServer` trait — method signatures for hover, goto_definition, references, completion, signature_help, semantic_tokens_full are standard; verified against documentation pattern
- lsp-types 0.94 struct fields for `Hover`, `Location`, `CompletionItem`, `SignatureHelp`, `SemanticTokens` — standard LSP types; HIGH confidence on existence, MEDIUM on exact field names without live compilation check

### Tertiary (LOW confidence)
- `SemanticTokensBuilder` existence in lsp-types 0.94.1 — not confirmed; flagged as open question

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all deps already in Cargo.toml, confirmed versions
- Architecture: HIGH — based on direct reading of existing codebase patterns
- Pitfalls: HIGH — derived from actual code (TyInterner::display, synthetic FileId, Box::leak pattern)
- LSP protocol details: MEDIUM — tower-lsp 0.20 trait signatures are standard; specific struct fields not verified against compiled output

**Research date:** 2026-03-14
**Valid until:** 2026-04-14 (deps are pinned; LSP spec is stable)
