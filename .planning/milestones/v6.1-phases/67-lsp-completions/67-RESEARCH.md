# Phase 67: LSP Completions - Research

**Researched:** 2026-03-18
**Domain:** tower-lsp completion handlers, DefMap namespace queries, TypedAST expression walking
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- `::` completions support ALL FQN-prefixed definitions: `log::` (trace/debug/info/warn/error), `Option::` (Some/None), `Result::` (Ok/Err), and user-defined enum variants (e.g. `MyEnum::VariantA`).
- The backend falls through to `identifier_completion` when trigger_char is `":"` — a new handler path is needed for `::` prefix completions.
- Synthetic log entries (FileId(u32::MAX)) are excluded from general identifier completions but belong in namespace-qualified completions.
- Dot-completion infrastructure exists (`build_dot_completions` handles Struct, Class, Entity, Enum, Array, Option types). Bug likely in: (a) `expr_at_offset` not finding the receiver at `dot_offset.saturating_sub(1)`, (b) re-analysis producing different FileIds than expected (hardcoded `FileId(0)` assumption), or (c) the modified source not compiling cleanly after dot removal.
- Fix must ensure the resolved type from the type checker is used, not a fallback empty list (per SC4).

### Claude's Discretion

- Text-based `::` prefix extraction implementation details
- Exact approach to diagnosing why `expr_at_offset` fails for dot-completion receivers
- Whether to use cached analysis or re-analysis for colon-triggered completions
- Error handling for malformed namespace prefixes

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LSP-01 | User gets auto-completion for methods on typed expressions (dot-completions) | Dot-completion infrastructure exists; research identifies the exact bug site and the `expr_at_offset` + `FileId(0)` issue |
| LSP-02 | User gets auto-completion for built-in namespaces (e.g. `log::info`, `Option::Some`) | DefMap `by_fqn` and `namespace_members` provide all data; `enum_variants` covers user-defined enums; research identifies the missing `::`-handler dispatch path |
</phase_requirements>

---

## Summary

Phase 67 is a targeted bug-fix phase. The LSP already registers `":"` as a trigger character (`backend.rs` line 94) but the completion handler falls through to `identifier_completion` when the trigger is `":"` — the `::`-completion path simply does not exist yet. The fix requires adding one new function `build_namespace_completions()` in `completion.rs` and adding the dispatch branch in `backend.rs`.

For dot-completions the infrastructure is complete (`build_dot_completions` covers all types). The bug is in the diagnostic path: `analyze_standalone` always assigns `FileId(0)` to the single file it analyses. The completion handler passes `FileId(0)` to `expr_at_offset`, which is correct — but `expr_at_offset` requires the offset to fall within the span of a declaration in that file AND within an expression node. The key risk is that the receiver expression ends exactly at `dot_offset - 1`, making the half-open range check `offset >= span.end` fail for a single-character receiver. This must be verified against the actual span semantics.

**Primary recommendation:** Fix the `":"` completion dispatch first (it is self-contained and verifiable with unit tests). Then diagnose the dot-completion receiver-lookup failure using a targeted unit test before writing the integration test.

---

## Standard Stack

### Core — already in use, no new dependencies required
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tower-lsp | (workspace) | LSP server framework | Established in project |
| lsp-types | (workspace) | `CompletionItem`, `CompletionItemKind` | Established in project |
| writ-compiler | (workspace) | `DefMap`, `TypeEnv`, `TyInterner`, `TypedAst` | Project's own compiler |

No new crate dependencies are needed for this phase.

---

## Architecture Patterns

### Recommended Project Structure

No new files needed. All changes are in existing files:

```
writ-lsp/src/
├── backend.rs          # Add "::" dispatch branch in completion()
└── queries/
    └── completion.rs   # Add build_namespace_completions() function
```

### Pattern 1: Trigger Character Dispatch (existing, extend it)

**What:** The `completion()` handler in `backend.rs` branches on `trigger_char`.
**Current code (lines 479–551):**
```rust
if trigger_char == Some(".") {
    // dot-completion path (re-analyze, find receiver, build_dot_completions)
    ...
    return Ok(Some(CompletionResponse::Array(items)));
}
// IDENTIFIER COMPLETION (falls through here for ":" and None)
self.identifier_completion(&uri_str).await
```
**Fix:** Add a new branch before the fall-through:
```rust
if trigger_char == Some(":") {
    return self.namespace_completion(&source, byte_offset, &uri_str).await;
}
```

**When to use:** Any time `trigger_character == ":"` is received from the client.

### Pattern 2: Namespace Prefix Extraction (text-based, new)

**What:** Extract the namespace prefix from source text using backward scan from cursor.
**Approach:** Walk backward from `byte_offset`, skip the second `:`  (trigger sends one `:` but the user typed `::` so the source contains two), then read the identifier preceding it.

```rust
// Source: derived from existing extract_callee_name() in completion.rs
fn extract_namespace_prefix(source: &str, cursor: usize) -> Option<String> {
    let bytes = source.as_bytes();
    let mut i = cursor;
    // cursor is positioned after the second ':'; skip any trailing ':' chars
    while i > 0 && bytes[i - 1] == b':' {
        i -= 1;
    }
    let end = i;
    // read alphanumeric + underscore backward
    while i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
        i -= 1;
    }
    if i == end { return None; }
    std::str::from_utf8(&bytes[i..end]).ok().map(|s| s.to_string())
}
```

**Confidence note:** The exact number of `:` bytes at cursor depends on how VS Code sends the trigger. VS Code sends one trigger character event per typed character. When the user types `log::`, the server receives two separate `":"` trigger events — once for the first `:` and once for the second `:`. At the time of the second trigger, the source already contains `log::`. So at `byte_offset`, the source has `...log::` and `cursor` points past the second colon. The backward scan must skip both colons then read the identifier. **Verify this assumption with a unit test using a source string that contains `log::` at various positions.**

### Pattern 3: DefMap Namespace Query (existing, use pub_members_of)

**What:** Query all definitions in a namespace using `DefMap.pub_members_of(namespace)`.
**Existing infrastructure:** `DefMap` has `namespace_members: FxHashMap<String, Vec<DefId>>`. The `pub_members_of()` method is already defined. Synthetic log entries are stored with `namespace: "log".to_string()` and ARE tracked in `namespace_members` (because `insert()` calls `self.namespace_members.entry(entry.namespace.clone()).or_default().push(id)` — but ONLY for `DefVis::Pub` entries).

**Critical check:** The synthetic log entries in `inject_log_namespace` use `def_map.arena.alloc(entry)` + `def_map.by_fqn.insert(fqn, id)` DIRECTLY — they bypass `def_map.insert()`. This means they are NOT added to `namespace_members`. **This is a key finding**: `pub_members_of("log")` returns empty. Instead, use `by_fqn` filtering:

```rust
// Source: direct inspection of def_map.rs and resolve/mod.rs inject_log_namespace
fn build_namespace_completions(
    namespace: &str,
    def_map: &DefMap,
    type_env: &TypeEnv,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let prefix = format!("{}::", namespace);
    for (fqn, &def_id) in &def_map.by_fqn {
        if fqn.starts_with(&prefix) {
            let entry = def_map.get_entry(def_id);
            let simple_name = fqn.strip_prefix(&prefix).unwrap_or(fqn);
            let kind = match entry.kind {
                DefKind::Fn | DefKind::ExternFn => CompletionItemKind::FUNCTION,
                DefKind::Enum => CompletionItemKind::ENUM,
                // ...
                _ => CompletionItemKind::VALUE,
            };
            items.push(CompletionItem {
                label: simple_name.to_string(),
                kind: Some(kind),
                ..Default::default()
            });
        }
    }
    // Also add enum variants for user-defined enum type prefixes
    // (e.g. "MyEnum" -> look up DefId, then type_env.enum_variants)
    if items.is_empty() {
        // Try as an enum type name
        if let Some(def_id) = def_map.by_fqn.values()
            .copied()
            .find(|&id| def_map.get_entry(id).name == namespace
                       && matches!(def_map.get_entry(id).kind, DefKind::Enum))
        {
            if let Some(variants) = type_env.enum_variants.get(&def_id) {
                for v in variants {
                    items.push(CompletionItem {
                        label: v.name.clone(),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        ..Default::default()
                    });
                }
            }
        }
    }
    items
}
```

**Note on Option:: and Result::** These are prelude types. Their variant constructors (`Some`, `None`, `Ok`, `Err`) are not in `by_fqn` as enum variants — they are in `PRELUDE_TYPE_NAMES` / `SUB_PRELUDE_VARIANT_NAMES`. Since `Option` and `Result` are not user-defined enums, they won't appear in `type_env.enum_variants`. The variants for `Option::` and `Result::` must be hardcoded (or discovered via a different path). The simplest correct approach: check for exact matches "Option", "Result" and return hardcoded variant lists, in addition to the `by_fqn` prefix scan.

### Pattern 4: Dot-Completion Receiver Bug Diagnosis

**What:** The completion handler calls `expr_at_offset(typed_ast, dot_offset.saturating_sub(1), FileId(0))`. This fails for one specific reason that can be diagnosed:

`find_in_expr` uses `offset >= span.end` (half-open) as the out-of-range check (see `walk.rs` line 137). For a receiver expression like `p` (a single character at byte offset `N`), the span is `{start: N, end: N+1}`. When `dot_offset = N+1` (the cursor is on the dot), `dot_offset.saturating_sub(1) = N`, which is exactly `span.start`. This IS within `[N, N+1)` so it should be found.

The more likely failure: the MODIFIED source (with the dot stripped) changes byte offsets of tokens that come AFTER the dot. But the receiver is BEFORE the dot, so its span is unchanged. The receiver expr's span in the original AST should match the modified source.

**Most likely actual bug:** The `identifier_completion` fallback at the end of the dot branch (`if items.is_empty() { return Ok(None); }`) returns `None` (no completions), not an empty list. VS Code would show nothing. The actual problem may be that `expr_at_offset` returns `None` because:

1. The source WITH the dot present is what the user has typed. The MODIFIED source strips the dot. But the analysis result is from the MODIFIED source. In Writ, `p` by itself is valid as a standalone expression. If `p` is a local binding not at the top level but inside a function body, `expr_at_offset` in `walk.rs` only visits `TypedDecl::Fn { body, .. }`, `TypedDecl::Impl { methods, .. }`, `TypedDecl::Const { value, .. }`, `TypedDecl::Global { value, .. }`. For a `let` statement inside a function, `p` would be the tail expression of the function body block. This should be found.

2. The real failure point: `p` in `fn main() { let p: Point = ...; p. }` after stripping the dot becomes `fn main() { let p: Point = ...; p }`. The `p` expression is the tail expression of the block. Its span in the re-analyzed AST is correct. `expr_at_offset` should find it.

**Recommended diagnostic approach:** Write a unit test in `completion.rs` using the `build_typed_ast_full` helper:
```rust
#[test]
fn test_dot_completion_receiver_found() {
    let src = "pub struct Point { x: int, y: int }
fn main() { let p: Point = new Point { x: 1, y: 2 }; p }";
    let (ast, mut interner, type_env) = build_typed_ast_full(src);
    // p is at src.rfind("p }").unwrap()
    let p_offset = src.rfind("; p }").unwrap() + 2; // byte offset of 'p'
    let expr = crate::queries::expr_at_offset(&ast, p_offset, FileId(0));
    assert!(expr.is_some(), "should find p expression at offset {}", p_offset);
}
```

If this test passes, the issue is in how `dot_offset` is computed (off-by-one), not in `expr_at_offset` itself. If it fails, `expr_at_offset` has a genuine span bug.

### Pattern 5: Cached Analysis for `::` Completions (discretion area)

**Recommendation:** Use cached analysis (`analysis_cache`) rather than re-analysis for `::` completions. The `::` trigger does not invalidate typing — the source text before the `::` is valid. The cached TypedAst + DefMap contains all the enum variant data and all FQN entries. This avoids the performance cost of a full re-analysis for every colon keypress.

```rust
// In backend.rs, the namespace_completion helper:
async fn namespace_completion(
    &self,
    source: &str,
    byte_offset: usize,
    uri_str: &str,
) -> jsonrpc::Result<Option<CompletionResponse>> {
    let namespace = extract_namespace_prefix(source, byte_offset)?;
    // Use cached analysis — no re-analysis needed
    let cache_entry = match self.analysis_cache.get(uri_str) {
        Some(e) => e.clone(),
        None => return Ok(None),
    };
    let (typed_ast, _, type_env) = match (
        &cache_entry.typed_ast, &cache_entry.ty_interner, &cache_entry.type_env
    ) {
        (Some(t), Some(i), Some(e)) => (t, i, e),
        _ => return Ok(None),
    };
    let items = crate::queries::build_namespace_completions(&namespace, &typed_ast.def_map, type_env);
    if items.is_empty() { return Ok(None); }
    Ok(Some(CompletionResponse::Array(items)))
}
```

### Anti-Patterns to Avoid

- **Re-analyzing for `::` completion:** `::` doesn't break syntax, so re-analysis is wasteful. Use cache.
- **Using `namespace_members` for log namespace:** `inject_log_namespace` bypasses `def_map.insert()` and goes directly to `arena.alloc` + `by_fqn.insert`. Therefore `pub_members_of("log")` returns empty. Always use `by_fqn` prefix scan.
- **Assuming Option/Result variants are in `type_env.enum_variants`:** They are prelude types, not user-defined enums. Their variants must be hardcoded.
- **Stripping only one `:` before the namespace name:** VS Code sends one `":"` trigger per colon, so at trigger time the source already contains `namespace::`. Scan must skip both colons.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Enum variant lookup for user types | Custom enum scanner | `type_env.enum_variants.get(&def_id)` | Already populated during typecheck |
| FQN prefix matching | Trie or custom index | `def_map.by_fqn.iter().filter(starts_with prefix)` | Sufficient for expected namespace sizes |
| Type walking for receiver type | Custom type walker | `receiver_expr.ty()` (TypedExpr method) | Already returns the resolved Ty |

---

## Common Pitfalls

### Pitfall 1: `inject_log_namespace` Bypasses `namespace_members`
**What goes wrong:** `pub_members_of("log")` returns an empty slice even though `log::trace` etc. exist in `by_fqn`.
**Why it happens:** `inject_log_namespace` calls `def_map.arena.alloc()` + `def_map.by_fqn.insert()` directly, skipping the `def_map.insert()` method that populates `namespace_members`.
**How to avoid:** Use `by_fqn` prefix scanning (`starts_with("log::")`) to find log entries.
**Warning signs:** Unit test for `log::` completions returns empty list despite correct trigger path.

### Pitfall 2: Option/Result Variants Not in DefMap
**What goes wrong:** `Option::` completion returns empty because there's no "Option" enum in `type_env.enum_variants`.
**Why it happens:** `Option` and `Result` are prelude types from `writ-runtime`. They don't appear as user-defined Enum DefKinds in the DefMap. `PRELUDE_TYPE_NAMES` lists them but doesn't register enum variants.
**How to avoid:** Hardcode Option variants (Some, None) and Result variants (Ok, Err) with ENUM_MEMBER kind in `build_namespace_completions` for these exact namespace names.
**Warning signs:** `Option::` and `Result::` completions return empty list.

### Pitfall 3: FileId Mismatch in Dot-Completion Re-Analysis
**What goes wrong:** `expr_at_offset` gets a wrong `FileId`, skips all declarations, returns `None`.
**Why it happens:** `analyze_standalone` always assigns `FileId(0)`. The completion handler hardcodes `FileId(0)`. This is correct as long as dot-completion always uses standalone analysis (not project analysis). If the workspace root contains a `writ.toml`, the production `publish_diagnostics_for` uses project analysis with multi-file FileIds — but the dot-completion always re-runs `analyze_standalone`, so `FileId(0)` is always correct in that path.
**How to avoid:** This hardcoded `FileId(0)` is intentional and correct for the dot-completion re-analysis path. Do not change it.
**Warning signs:** No completions appear even though the re-analysis succeeds and returns a valid TypedAst.

### Pitfall 4: Off-by-One in `dot_offset.saturating_sub(1)`
**What goes wrong:** `expr_at_offset` receives offset `N-1` but the receiver expression's span is `[N, N+1)`. The expression is missed.
**Why it happens:** If the receiver is a multi-character identifier `foo`, its span is `[foo_start, foo_end)` where `foo_end = dot_offset`. `dot_offset - 1 = foo_end - 1`, which IS within `[foo_start, foo_end)`. So for multi-char receivers this works. For single-char receivers (e.g. `p`): span is `[N, N+1)`, `dot_offset = N+1`, `dot_offset - 1 = N = span.start` — also within range.
**How to avoid:** The existing `dot_offset.saturating_sub(1)` logic is correct. Verify with the diagnostic unit test pattern above.
**Warning signs:** Only single-character receivers fail, multi-character receivers work.

### Pitfall 5: Dot-Completion Source Modification Changes Subsequent Offsets
**What goes wrong:** Stripping the `.` from the source shifts byte offsets of all tokens after the dot. The `dot_offset` in the ORIGINAL source equals the byte offset of the receiver's end in the MODIFIED source.
**Why it happens:** The format! removes exactly one byte (the `.`), so `modified_source[..dot_offset]` is identical to `original_source[..dot_offset]`. Receiver expressions are always BEFORE `dot_offset`, so their spans are unchanged.
**How to avoid:** No action needed — this is not actually a problem. Receiver spans are in the prefix of the source that was not modified.

### Pitfall 6: `":"` Trigger Arrives TWICE Per `::` Sequence
**What goes wrong:** Namespace completion fires on the FIRST `:` keypress, when the source has `foo:` (incomplete). `extract_namespace_prefix` finds no valid namespace and returns empty completions. This is expected and harmless — VS Code will suppress the empty list. On the SECOND `:` keypress, the source has `foo::` and the prefix extraction finds `foo`.
**How to avoid:** Make `extract_namespace_prefix` return `None` if it doesn't find a clean `<ident>::` pattern. The `None` return causes the handler to fall through or return `Ok(None)`. This is correct.

---

## Code Examples

### Example 1: Existing `build_dot_completions` call pattern
```rust
// Source: backend.rs lines 529-546 (existing dot-completion handler)
let receiver_expr =
    match crate::queries::expr_at_offset(typed_ast, dot_offset.saturating_sub(1), FileId(0)) {
        Some(e) => e,
        None => return Ok(None),
    };
let receiver_ty = receiver_expr.ty();
let items = crate::queries::build_dot_completions(
    receiver_ty,
    interner,
    &typed_ast.def_map,
    type_env,
);
if items.is_empty() {
    return Ok(None);
}
return Ok(Some(CompletionResponse::Array(items)));
```

### Example 2: `by_fqn` prefix scan for namespace completions
```rust
// Source: direct inspection of def_map.rs (DefMap.by_fqn structure)
// Scanning for all FQNs that start with "log::"
let prefix = "log::";
for (fqn, &def_id) in &def_map.by_fqn {
    if let Some(simple) = fqn.strip_prefix(prefix) {
        // simple = "trace", "debug", etc.
    }
}
```

### Example 3: Integration test pattern (matches existing `build_typed_ast_full` pattern)
```rust
// Source: completion.rs tests module (lines 539-571)
#[test]
fn test_namespace_completions_log() {
    let src = "fn main() { log::info(\"hello\"); }";
    let (ast, _interner, type_env) = build_typed_ast_full(src);
    let items = build_namespace_completions("log", &ast.def_map, &type_env);
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"info"));
    assert!(labels.contains(&"trace"));
    assert!(labels.contains(&"warn"));
}
```

### Example 4: How `analyze_standalone` assigns FileId(0)
```rust
// Source: analysis_host.rs lines 49-50
pub fn analyze_standalone(source: String, display_path: String) -> AnalysisResult {
    let file_id = FileId(0);  // <-- always 0 for standalone
    // ...
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No `":"` handler — falls to identifier_completion | Need new `::` namespace handler | Phase 67 (this phase) | Enables `log::`, `Option::`, enum namespace completions |
| Dot-completion infrastructure present but broken in practice | Fix receiver lookup and verify | Phase 67 (this phase) | Typed dot-completions work correctly |

---

## Open Questions

1. **Does `Option::` need `type_env.enum_variants` or hardcoded variants?**
   - What we know: `Option` is a prelude type; not registered as a user enum; `type_env.enum_variants` does not contain it.
   - What's unclear: Whether `writ-runtime` exposes its Option enum through the normal DefMap enum path.
   - Recommendation: Hardcode `Some` and `None` for `"Option"` and `Ok`/`Err` for `"Result"` in `build_namespace_completions`. This is the correct design boundary — prelude builtins have known static variant sets.

2. **Does `by_fqn` prefix scan need to exclude file_private entries?**
   - What we know: `file_private` entries are NOT in `by_fqn`. `by_fqn` contains only public entries.
   - What's unclear: Nothing — this is settled. `by_fqn` is already the right map to scan.
   - Recommendation: Use `by_fqn` directly.

3. **Is the dot-completion failure caused by `expr_at_offset` returning None or by `build_dot_completions` returning empty?**
   - What we know: `build_dot_completions` has a final `_ => {}` arm that returns empty for unrecognized types, including `TyKind::Error`.
   - What's unclear: Whether the re-analysis of the modified source (with dot stripped) introduces a type error that changes the receiver's type to `TyKind::Error`.
   - Recommendation: Add the diagnostic unit test (Pattern 4 above) first; if `expr_at_offset` succeeds but returns a `TyKind::Error` receiver, the fix is to ensure the modified source compiles cleanly.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`#[test]`) + `#[tokio::test]` for async protocol tests |
| Config file | `Cargo.toml` workspace |
| Quick run command | `cargo test -p writ-lsp -- completion` |
| Full suite command | `cargo test -p writ-lsp` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LSP-01 | `p.` shows struct fields | unit | `cargo test -p writ-lsp -- test_dot_completion` | ❌ Wave 0 |
| LSP-01 | `arr.` shows Array methods | unit | `cargo test -p writ-lsp -- test_dot_completion_array` | ❌ Wave 0 |
| LSP-02 | `log::` shows 5 levels | unit | `cargo test -p writ-lsp -- test_namespace_completions_log` | ❌ Wave 0 |
| LSP-02 | `Option::` shows Some/None | unit | `cargo test -p writ-lsp -- test_namespace_completions_option` | ❌ Wave 0 |
| LSP-02 | `MyEnum::` shows variants | unit | `cargo test -p writ-lsp -- test_namespace_completions_enum` | ❌ Wave 0 |

**Note:** Tests live in `writ-lsp/src/queries/completion.rs` `#[cfg(test)]` block and `writ-lsp/src/analysis_host.rs` `#[cfg(test)]` block, matching existing project patterns. The existing `test_dot_completions_struct_fields` test passes — it calls `build_dot_completions` directly with a manually constructed Ty. New tests for the INTEGRATION path (via `AnalysisHost::analyze_standalone` + `expr_at_offset`) are the Wave 0 gap.

### Sampling Rate
- **Per task commit:** `cargo test -p writ-lsp -- completion`
- **Per wave merge:** `cargo test -p writ-lsp`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-lsp/src/queries/completion.rs` — add `test_dot_completion_receiver_found` (diagnostic unit test for `expr_at_offset` with receiver)
- [ ] `writ-lsp/src/queries/completion.rs` — add `test_namespace_completions_log`, `test_namespace_completions_option`, `test_namespace_completions_enum` (after `build_namespace_completions` is written)
- [ ] `writ-lsp/src/analysis_host.rs` — add integration test for dot-completion via `analyze_standalone` + `expr_at_offset` chain

---

## Sources

### Primary (HIGH confidence)
- Direct source inspection: `writ-lsp/src/backend.rs` — completion handler, trigger character dispatch, dot-completion flow
- Direct source inspection: `writ-lsp/src/queries/completion.rs` — `build_dot_completions`, `build_identifier_completions`, existing test patterns
- Direct source inspection: `writ-lsp/src/queries/walk.rs` — `expr_at_offset`, span semantics (half-open `offset >= span.end`)
- Direct source inspection: `writ-lsp/src/analysis_host.rs` — `analyze_standalone` always assigns `FileId(0)`
- Direct source inspection: `writ-compiler/src/resolve/def_map.rs` — `DefMap` structure, `by_fqn`, `namespace_members`, `pub_members_of`
- Direct source inspection: `writ-compiler/src/resolve/mod.rs` — `inject_log_namespace` bypasses `def_map.insert()`, uses `arena.alloc` directly
- Direct source inspection: `writ-compiler/src/resolve/prelude.rs` — `PRELUDE_TYPE_NAMES`, `LOG_NAMESPACE_LEVELS`, `SUB_PRELUDE_VARIANT_NAMES`
- Direct source inspection: `writ-compiler/src/check/env.rs` — `TypeEnv.enum_variants` structure and population

### Secondary (MEDIUM confidence)
- Protocol knowledge: LSP specification — trigger character events fire once per typed character; both `:` keypresses in `::` each fire a separate trigger event. Standard LSP client behavior confirmed by tower-lsp usage in `test_hover_protocol.rs`.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all from existing project code
- Architecture: HIGH — all key data structures directly inspected from source
- Pitfalls: HIGH — pitfalls derived from direct code inspection of inject_log_namespace bypass and span semantics
- `Option::`/`Result::` hardcoding requirement: HIGH — confirmed prelude types not in type_env.enum_variants

**Research date:** 2026-03-18
**Valid until:** Stable (until compiler internals change) — 90 days
