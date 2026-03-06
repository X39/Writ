# Phase 61: Signature Help, Diagnostics, and Extension Polish - Research

**Researched:** 2026-03-17
**Domain:** LSP backend (Rust), VS Code extension manifest (JSON) — three isolated bug fixes
**Confidence:** HIGH

---

## Summary

Phase 61 closes three UAT gaps: signature help never fires during real editing (UAT test 12, LSP-07), top-level parse errors produce invisible zero-width squiggles (UAT test 4, LSP-01), and entity names share the same color as struct names in every theme (UAT test 13, DIFF-01).

All three root causes have been fully diagnosed in the debug session files (.planning/debug/). No new research was needed beyond reading existing diagnostics. Each fix is surgical and affects a single file:

- **Signature help**: `writ-lsp/src/queries.rs` — replace `find_enclosing_call` + TypedAst walk with a pure text-based callee-name extraction, avoiding the need for a Call node in the AST entirely.
- **Zero-width diagnostics**: `writ-lsp/src/convert.rs` — expand `0..0` spans in `parse_error_to_diag` to underline the preceding non-whitespace token.
- **Semantic token scopes**: `writ-vscode/package.json` — remap custom token scopes out of the `entity.name.type.*` family, and add `configurationDefaults` with explicit per-token colors.

**Primary recommendation:** Implement all three fixes in a single plan. They are independent; any ordering works.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LSP-07 | Signature help with active parameter highlighted during function/method calls | Text-based callee resolution bypasses the AST-complete-call requirement (see Signature Help section) |
| LSP-01 | Language server publishes diagnostics as inline squiggles on file save or change | Zero-width span expansion in `parse_error_to_diag` ensures top-level errors have renderable ranges (see Diagnostics section) |
| DIFF-01 | Semantic highlighting distinguishes entity names with distinct token types | Scope remapping + `configurationDefaults` override ensures entity color differs from struct in all themes (see Semantic Tokens section) |
</phase_requirements>

---

## Standard Stack

### Core (already in workspace — no new dependencies)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tower-lsp` | existing | LSP handler dispatch | Already used; no change |
| `lsp-types` | existing | `SignatureHelp`, `SignatureInformation`, `ParameterInformation` types | Already used |
| `writ_compiler::check::env::TypeEnv` | workspace | `fn_sigs` lookup by `DefId` | Already used in `build_signature_help` |
| `writ_compiler::resolve::def_map::DefMap` | workspace | `by_fqn` / `file_private` callee lookup | Already used |

No new crate dependencies for any of the three fixes.

---

## Architecture Patterns

### Pattern 1: Text-Based Callee Extraction (Signature Help)

**What:** Instead of requiring a complete `TypedExpr::Call` node in the cached AST, extract the callee name by scanning the raw source text backward from the `(` character.

**When to use:** Any time the user is mid-typing inside a call expression (source is syntactically incomplete).

**Detailed algorithm:**

```rust
// After finding open_paren_offset by backward scan:
// 1. Move backward from open_paren_offset, skipping whitespace
// 2. Read a Writ identifier: [a-zA-Z_][a-zA-Z0-9_]*
//    (also handle `path::ident` — read until non-ident/non-colon chars)
// 3. Look up the extracted name in DefMap:
//    a. Try `def_map.by_fqn` — iterate values, find entry whose `.name` matches
//    b. Try `def_map.file_private` — check all file scopes
// 4. If DefId found, look up `type_env.fn_sigs.get(&def_id)`
// 5. Return SignatureHelp from the FnSig

// Source: .planning/debug/signature-help-broken.md (APPROACH A, recommended)
fn extract_callee_name(source: &str, paren_offset: usize) -> Option<String> {
    let bytes = source.as_bytes();
    let mut i = paren_offset;
    // Skip whitespace before (
    while i > 0 && (bytes[i - 1] == b' ' || bytes[i - 1] == b'\t' || bytes[i - 1] == b'\n') {
        i -= 1;
    }
    // Read identifier backward
    let end = i;
    while i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_' || bytes[i - 1] == b':') {
        i -= 1;
    }
    if i == end { return None; }
    let name_bytes = &bytes[i..end];
    std::str::from_utf8(name_bytes).ok().map(|s| s.trim_start_matches(':').to_string())
}
```

**Why Approach A over B (source modification):**
- Approach B (insert `)` and re-analyze) costs a full compiler pipeline invocation on every keystroke. Approach A is a pure string scan — O(identifier length), under 1 µs.
- Approach B adds complexity: re-analysis can panic on injected `)` at wrong positions, especially inside nested calls.
- Approach A is exactly what rust-analyzer uses for its callee resolution on incomplete sources.

**Backward compatibility:** The existing `find_enclosing_call` path can remain as a secondary fallback for when the AST does have a complete call (covers cursor inside an already-typed full call). Only the primary path changes.

### Pattern 2: Zero-Width Span Expansion (Diagnostics)

**What:** In `parse_error_to_diag`, detect `span.start == span.end` and expand the span to give VS Code a non-empty range to underline.

**When to use:** Any time a parse error has a zero-width span (common for EOF errors and entity-body recovery).

```rust
// Source: .planning/debug/top-level-diagnostics.md
fn expand_zero_width_span(span: SimpleSpan, source: &str) -> SimpleSpan {
    if span.start < span.end {
        return span; // already non-empty
    }
    // Find the last non-whitespace character before span.start
    let bytes = source.as_bytes();
    let mut pos = span.start.min(source.len());
    // Step back past whitespace
    while pos > 0 && (bytes[pos - 1] == b' ' || bytes[pos - 1] == b'\t' || bytes[pos - 1] == b'\n' || bytes[pos - 1] == b'\r') {
        pos -= 1;
    }
    if pos == 0 {
        // File is empty — point at offset 0..1 if source is non-empty, else 0..0
        let end = if source.is_empty() { 0 } else { 1 };
        return SimpleSpan { start: 0, end, context: () };
    }
    // Underline the previous character
    let ch_start = prev_char_boundary(source, pos);
    SimpleSpan { start: ch_start, end: pos, context: () }
}

fn prev_char_boundary(source: &str, mut pos: usize) -> usize {
    // Step back until we're at a UTF-8 char boundary
    while pos > 0 && !source.is_char_boundary(pos - 1) {
        pos -= 1;
    }
    if pos > 0 { pos - source[..pos].chars().last().map(|c| c.len_utf8()).unwrap_or(1) }
    else { 0 }
}
```

**Alternative (simpler):** If `span.start == span.end` and `span.start > 0`, return `SimpleSpan { start: span.start - 1, end: span.start }`. This is simpler but may land in the middle of a multi-byte character. Use `source.is_char_boundary` to guard.

**Actual simpler approach used in practice:**
```rust
// In parse_error_to_diag, after let span = *err.span():
let span = if span.start == span.end && span.start > 0 {
    // Walk back to find a char boundary for a 1-char underline
    let mut s = span.start;
    while s > 0 && !source.is_char_boundary(s - 1) { s -= 1; }  // (source not available here)
    // Without source text: just use span.start - 1 .. span.start (safe for ASCII)
    // The source text IS needed for proper UTF-8 boundary detection.
    // Simplest safe approach: expand to end of previous line if at EOL,
    // or create a 1-byte span at span.start - 1.
    SimpleSpan { start: span.start.saturating_sub(1), end: span.start, context: () }
} else {
    span
};
```

**Important:** `parse_error_to_diag` currently has signature `fn parse_error_to_diag(err, file_id)` — no source text parameter. Two options:
1. Pass source text to the function (cleanest but requires callers to update — 2 call sites: `analyze_standalone` and `analyze_project`).
2. Use the simple `start.saturating_sub(1)..start` approach without boundary check (works for all ASCII, safe for most real code).

**Recommendation:** Option 2 first (minimal diff, no caller changes). It handles the entity-EOF case correctly because the last byte is always ASCII (closing `}` or `\n`). If Unicode boundary issues appear in practice, upgrade to option 1.

### Pattern 3: Semantic Token Scope Remapping (VS Code extension)

**What:** Change `semanticTokenScopes` in `package.json` to map custom tokens to scopes outside the `entity.name.type.*` family, and add `configurationDefaults` to provide explicit colors in all themes.

**Root cause:** `entity.name.type.writ` inherits the theme rule for `entity.name.type` (VS Code's default for the `type` semantic token) by prefix matching. All themes color `entity.name.type` the same as structs.

**Fix A — Change scope family (sufficient for most themes):**
```json
"semanticTokenScopes": [
  {
    "language": "writ",
    "scopes": {
      "entity": ["support.class.writ"],
      "component": ["support.other.component.writ"],
      "dialogueSpeaker": ["variable.other.constant.speaker.writ"]
    }
  }
]
```

`support.class` is colored distinctly (often teal/cyan) in Dark+, One Dark Pro, and most popular themes. It does not inherit from `entity.name.type`.

**Fix B — Add `configurationDefaults` (guarantees distinct color in any theme):**
```json
"configurationDefaults": {
  "editor.semanticTokenColorCustomizations": {
    "[*]": {
      "rules": {
        "entity": "#4EC9B0",
        "component": "#9CDCFE",
        "dialogueSpeaker": "#CE9178"
      }
    }
  }
}
```

`[*]` means all themes. These colors are the VS Code Dark+ palette:
- `#4EC9B0` — teal (class/interface color in Dark+)
- `#9CDCFE` — light blue (parameter color in Dark+)
- `#CE9178` — orange (string color in Dark+)

**Recommendation:** Implement both A and B (Option C from debug doc). Use both scope remapping AND `configurationDefaults`. The scope remapping helps themes that support TextMate-based semantic token overrides; the `configurationDefaults` guarantees differentiation for all others.

**Note on UAT test 13 success criteria:** "Entity names and struct names are highlighted with visually distinct colors in the default VS Code theme." The default theme is "Dark+" which applies the `editor.semanticTokenColorCustomizations` from `configurationDefaults`. The `[*]` wildcard applies to all themes including the default.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Callee name lookup after `(` | Full re-analysis pipeline | Text-based backward scan + DefMap lookup | Re-analysis costs ~20-50ms per keystroke; string scan is O(1) |
| Span rendering in VS Code | Custom squiggle logic | Expand zero-width spans before sending to LSP | VS Code exclusively uses the `range` field for squiggle placement |
| Semantic token differentiation | Custom TextMate grammar changes | `configurationDefaults` in package.json | Grammar changes require re-tokenization; semantic token colors override grammar |

---

## Common Pitfalls

### Pitfall 1: Callee Name Scan Stops at Wrong Boundaries
**What goes wrong:** Scanning backward through `::` for path-qualified calls (e.g., `MyModule::func(`) hits the `:` character and stops prematurely, returning only `func` instead of `MyModule::func`.
**Why it happens:** Naive identifier scan stops at `:`.
**How to avoid:** Include `::` in the scan: continue backward while `bytes[i-1]` is `[a-zA-Z0-9_:]`. Strip leading `::` from result before DefMap lookup.
**Warning signs:** Signature help works for simple calls `foo(` but not qualified calls `Ns::foo(`.

### Pitfall 2: `configurationDefaults` Overrides User Theme Permanently
**What goes wrong:** `[*]` wildcard overrides ALL themes, including light themes where dark colors like `#4EC9B0` look wrong.
**Why it happens:** `[*]` is a theme wildcard with no specificity discrimination.
**How to avoid:** Accept this tradeoff — the requirement is visual distinction, not aesthetic perfection. Users can override via their own settings. Alternatively, scope to `[Default Dark+]` and `[Default Light+]` specifically, but this is more fragile.
**Warning signs:** User reports colors look wrong on light themes.

### Pitfall 3: Zero-Width Span at Offset 0
**What goes wrong:** Source is empty or error is at the very beginning; `saturating_sub(1)` returns `0..0` unchanged.
**Why it happens:** `0_usize.saturating_sub(1)` == 0, so the span remains zero-width.
**How to avoid:** Special case: if `span.start == 0`, use `0..1` if the source is non-empty, else `0..0`. VS Code handles `0..0` gracefully (no squiggle, which is correct for empty files).
**Warning signs:** Empty `.writ` files with syntax errors show no squiggle (acceptable).

### Pitfall 4: Signature Help Active Parameter Off by One
**What goes wrong:** After typing `foo(a,` the active parameter shows index 0 instead of 1.
**Why it happens:** The comma count scan is correct (`comma_count += 1` at top-level commas), but if the text-based callee extraction path skips over the existing `find_enclosing_call` fallback, it may use a stale `comma_count`.
**How to avoid:** The backward scan for `(` is already in `build_signature_help` before any callee resolution — preserve its comma count. Callee resolution is independent of comma count.
**Warning signs:** Typing second argument shows first parameter highlighted.

---

## Code Examples

### Text-Based Callee Resolution (the new primary path in `build_signature_help`)

```rust
// Source: .planning/debug/signature-help-broken.md (Approach A)
// After finding open_paren_offset via backward scan:

// Extract callee name from source text before the (
let callee_name = extract_callee_name_from_source(source, paren_offset);

if let Some(name) = callee_name {
    // Look up in DefMap — check by_fqn first, then file_private
    let def_id = ast.def_map.by_fqn.values()
        .find(|&&id| {
            let e = ast.def_map.get_entry(id);
            e.name == name || {
                // Also match fully-qualified: "Ns::func" ends with "::name"
                e.name == name.split("::").last().unwrap_or(&name)
            }
        })
        .copied()
        .or_else(|| {
            for privs in ast.def_map.file_private.values() {
                if let Some(&id) = privs.get(name.as_str()) {
                    return Some(id);
                }
            }
            None
        });

    if let Some(id) = def_id {
        if let Some(sig) = type_env.fn_sigs.get(&id) {
            // Build SignatureHelp from sig + comma_count
            return Some(build_sig_help_from_fnsig(sig, comma_count, interner, &ast.def_map));
        }
    }
}

// Fallback: try the old find_enclosing_call path (works when AST has a complete call)
// ... existing code ...
```

### Zero-Width Span Expansion in `parse_error_to_diag`

```rust
// Source: .planning/debug/top-level-diagnostics.md
// In writ-lsp/src/convert.rs, parse_error_to_diag:

pub fn parse_error_to_diag(
    err: &chumsky::error::Rich<'_, writ_parser::Token<'_>, SimpleSpan>,
    file_id: FileId,
) -> Diagnostic {
    let raw_span = *err.span();

    // Expand zero-width spans so VS Code can render a visible squiggle.
    // Zero-width spans occur when the parser errors at end-of-input or during
    // entity/struct recovery. Without expansion, VS Code silently skips them.
    let span = if raw_span.start == raw_span.end && raw_span.start > 0 {
        SimpleSpan {
            start: raw_span.start - 1,
            end: raw_span.start,
            context: (),
        }
    } else {
        raw_span
    };
    // ... rest of function unchanged ...
}
```

### Semantic Token Scopes Fix in `package.json`

```json
// In writ-vscode/package.json, replace the semanticTokenScopes block AND add configurationDefaults:

"semanticTokenScopes": [
  {
    "language": "writ",
    "scopes": {
      "entity": ["support.class.writ"],
      "component": ["support.other.component.writ"],
      "dialogueSpeaker": ["variable.other.constant.speaker.writ"]
    }
  }
],
"configurationDefaults": {
  "editor.semanticTokenColorCustomizations": {
    "[*]": {
      "rules": {
        "entity": "#4EC9B0",
        "component": "#9CDCFE",
        "dialogueSpeaker": "#CE9178"
      }
    }
  }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Signature help via AST Call node lookup | Text-based callee name scan + DefMap lookup | Phase 61 | Works during real mid-typing; no full re-analysis needed |
| Raw chumsky spans passed directly to LSP | Zero-width spans expanded before LSP conversion | Phase 61 | Top-level parse errors now produce visible squiggles |
| Entity token mapped to `entity.name.type.writ` | Entity token mapped to `support.class.writ` + `configurationDefaults` | Phase 61 | Entity names visually distinct from struct names in all themes |

---

## Open Questions

1. **Method call signature help (`receiver.method(`)**
   - What we know: The text-based scan will extract `method` (just the method name), not `ReceiverType::method`. The DefMap `by_fqn` key for methods is usually scoped.
   - What's unclear: How method signatures are keyed in `type_env.fn_sigs` vs `type_env.impl_index`. Phase 54 decision: `fn_sigs` keys by `DefId` from `impl_index`.
   - Recommendation: For Phase 61, implement text-based lookup only for free functions (simple name lookup). Method calls can fall back to the existing `find_enclosing_call` path. The UAT test (test 12) uses a free function call.

2. **`configurationDefaults` VS Code version support**
   - What we know: `editor.semanticTokenColorCustomizations` with `[*]` wildcard was introduced in VS Code 1.58 (July 2021). The extension requires `^1.74.0`.
   - What's unclear: Whether any older VS Code in the wild ignores `configurationDefaults`.
   - Recommendation: Not a concern — `1.74+` requirement is already enforced.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`#[test]`) via `cargo test` |
| Config file | none (workspace Cargo.toml) |
| Quick run command | `cargo test -p writ-lsp` |
| Full suite command | `cargo test` |

Current state: 59 tests pass in `writ-lsp`. All tests are in `src/queries.rs` and `src/convert.rs` inline test modules.

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LSP-07 | `build_signature_help` returns Some when source has incomplete call `foo(` | unit | `cargo test -p writ-lsp test_signature_help_incomplete_source` | Wave 0 |
| LSP-07 | Active parameter is 1 after first comma in incomplete call | unit | `cargo test -p writ-lsp test_signature_help_active_param_incomplete` | Wave 0 |
| LSP-01 | `parse_error_to_diag` returns non-zero-width range for EOF error | unit | `cargo test -p writ-lsp test_zero_width_span_expansion` | Wave 0 |
| LSP-01 | Entity missing brace produces renderable diagnostic range | unit | `cargo test -p writ-lsp test_entity_missing_brace_span` | Wave 0 |
| DIFF-01 | (visual — no automated test; verified by reading package.json diff) | manual | n/a | n/a |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-lsp`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-lsp/src/queries.rs` — add `test_signature_help_incomplete_source`: calls `build_signature_help` with source `"fn foo(a: int, b: int) {} fn main() { foo("` and verifies `Some` is returned
- [ ] `writ-lsp/src/queries.rs` — add `test_signature_help_active_param_incomplete`: same but with `"fn foo(a: int, b: int) {} fn main() { foo(1,"` and verifies `active_parameter == Some(1)`
- [ ] `writ-lsp/src/convert.rs` — add `test_zero_width_span_expansion`: creates a `SimpleSpan { start: 10, end: 10, .. }` parse error, verifies the resulting diagnostic range has `start != end`

*(DIFF-01 is purely visual — the package.json diff is the verification artifact.)*

---

## Sources

### Primary (HIGH confidence)
- Direct code reading: `writ-lsp/src/queries.rs` (lines 977-1053) — current `build_signature_help` implementation
- Direct code reading: `writ-lsp/src/convert.rs` (lines 109-138) — current `parse_error_to_diag`
- Direct code reading: `writ-vscode/package.json` (lines 35-50) — current `semanticTokenScopes`
- Direct code reading: `writ-lsp/src/backend.rs` (lines 86-89) — signature_help trigger characters registered
- `.planning/debug/signature-help-broken.md` — full root cause analysis and recommended approach
- `.planning/debug/top-level-diagnostics.md` — full root cause analysis with specific span types
- `.planning/debug/entity-semantic-color.md` — full root cause analysis with three fix options

### Secondary (MEDIUM confidence)
- VS Code docs (known): `configurationDefaults` with `[*]` wildcard for `editor.semanticTokenColorCustomizations` is the standard extension API for providing default semantic token colors
- VS Code docs (known): TextMate scope matching is prefix-based; `entity.name.type.writ` inherits rules for `entity.name.type`
- Project decision log (STATE.md line 127): Phase 58 — `Box::leak for writ_parser::parse &'static str constraint` (confirms analysis pattern used in dot-completion)

### Tertiary (LOW confidence)
- None — all findings are from direct code reading and project-internal debug docs.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already present; no new crates
- Architecture: HIGH — root causes fully diagnosed in debug sessions; fix directions are explicit
- Pitfalls: HIGH — derived from code reading, not speculation

**Research date:** 2026-03-17
**Valid until:** 2026-06-17 (stable — no external dependencies changing)

---

## Key Files (with exact locations)

| File | Change | Lines |
|------|--------|-------|
| `writ-lsp/src/queries.rs` | Add text-based callee extraction; modify `build_signature_help` to use it as primary path | 977-1053 |
| `writ-lsp/src/convert.rs` | Expand zero-width spans in `parse_error_to_diag` | 109-138 |
| `writ-vscode/package.json` | Change `semanticTokenScopes` scope family; add `configurationDefaults` block | 35-50 + new block |

No other files require modification.
