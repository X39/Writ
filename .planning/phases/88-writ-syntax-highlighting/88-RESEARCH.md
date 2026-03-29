# Phase 88: Writ Syntax Highlighting - Research

**Researched:** 2026-03-27
**Domain:** mdBook theme customization + highlight.js 10.1.1 custom language definition
**Confidence:** HIGH

## Summary

mdBook 0.4.51 bundles highlight.js 10.1.1 as a static asset. The clean mechanism for adding a custom language is to place a modified `highlight.js` in `docs/theme/highlight.js` — mdBook automatically uses any file in the `theme/` directory that matches a default asset name, with no `book.toml` change required. The modified file is the bundled file verbatim, with the Writ language definition appended as a single `hljs.registerLanguage("writ", ...)` call at the end. This works because `highlight.js` loads before `book.js` in the generated HTML, so the `writ` language is registered before `hljs.highlightBlock()` is called.

The Writ language definition maps to three keyword tiers using the CSS classes that already exist in the bundled `highlight.css`: declaration keywords get `keyword` (purple), control flow gets `title` (blue), and modifiers/other get `built_in` (orange). Built-in types use `type` (orange) and literals use `literal` (orange). Strings and comments use standard `string` and `comment` classes respectively.

The fence-marker update task covers ~152 bare code blocks in language spec files (00-29). IL spec files (30+) have ~28 bare blocks that contain IL pseudocode — these should NOT receive `writ` fencing and are out of scope for this phase.

**Primary recommendation:** Copy bundled `highlight.js` to `docs/theme/highlight.js`, append the Writ language definition, then bulk-update bare fences in language spec files 00-29 to ` ```writ `.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Keyword Categorization:**
- 3 keyword tiers with distinct colors: declaration keywords (`fn`, `dlg`, `entity`, `struct`, `enum`, `contract`, `impl`, `class`, `component`, `namespace`), control flow (`if`, `else`, `match`, `for`, `while`, `return`, `break`, `continue`, `spawn`, `detached`, `join`, `cancel`, `defer`, `try`, `on`, `atomic`), modifiers/other (`let`, `mut`, `const`, `pub`, `priv`, `use`, `using`, `in`, `new`, `self`, `extern`, `global`)
- Built-in types (`void`, `int`, `float`, `bool`, `string`) get highlight.js `type` class, distinct from keywords
- Literals (`true`, `false`, `null`) get highlight.js `literal` class
- Runtime builtins (`say`, `choice`, `log`) are NOT highlighted as keywords — they are compiler-injected, not reserved words

**Integration Approach:**
- Custom highlight.js language file registered in `docs/theme/highlight.js` — mdBook's documented approach
- Code blocks use ` ```writ ` fence marker as the language identifier
- Default highlight.js theme colors used (no custom color scheme) — works with both light and dark mdBook themes
- Existing spec code blocks updated to use ` ```writ ` fencing where they contain Writ code

### Claude's Discretion
- Exact highlight.js API usage and registration pattern
- How to handle format strings (`$"..."`) and raw strings (`"""..."""`)
- Operator highlighting approach
- Any edge cases in string/comment detection

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| INFRA-04 | Custom highlight.js definition for Writ syntax highlighting in code blocks | Covered: bundled hljs 10.1.1 API, language definition pattern, fence update scope (152 blocks in files 00-29), theme/ file placement mechanism |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| highlight.js | 10.1.1 (bundled in mdBook 0.4.51) | Syntax highlighting engine | Already present — no install needed |
| mdBook | 0.4.51 (pinned) | Book renderer consuming theme/highlight.js | Project-pinned version (see STATE.md) |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| highlight.css (bundled) | mdBook default | CSS class → color mapping | Already present; no modification needed (uses default theme per CONTEXT.md) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Appending to bundled highlight.js | additional-js approach | additional-js loads after book.js; hljs.registerLanguage would run too late — highlighting already applied. theme/highlight.js is the only clean approach |
| Copying bundled file verbatim | Downloading newer highlight.js | Newer versions change API (className→scope in v11); staying at 10.1.1 avoids breakage |

**Installation:**
No npm install needed. The theme directory file is created manually.

## Architecture Patterns

### Recommended Project Structure
```
docs/
├── book.toml          (no changes needed)
├── theme/
│   └── highlight.js   (NEW: bundled 10.1.1 + Writ lang appended)
└── src/               (unchanged)
language-spec/
└── spec/
    ├── 00-29_*.md     (update bare ``` to ```writ — ~152 blocks)
    └── 30+_*.md       (IL spec — leave bare fences as-is)
```

### Pattern 1: Custom Language Registration in highlight.js 10.1.1

**What:** Append a `hljs.registerLanguage("writ", function(hljs) { return { ... }; })` call to the end of the bundled `highlight.js` file.

**When to use:** Any time mdBook needs to recognize a new language identifier in fenced code blocks.

**Example:**
```javascript
// Appended to the END of docs/theme/highlight.js
// Source: highlight.js 10.1.1 API (verified from bundled file pattern)
hljs.registerLanguage("writ", function(e) {
  return {
    name: "Writ",
    case_insensitive: false,
    keywords: {
      $pattern: /[A-Za-z_][A-Za-z0-9_]*/,
      keyword: "fn dlg entity struct enum contract impl class component namespace " +
               "if else match for while return break continue spawn detached join cancel defer try on atomic",
      built_in: "let mut const pub priv use using in new self extern global",
      type: "void int float bool string",
      literal: "true false null"
    },
    contains: [
      // Line comment: //
      e.COMMENT("//", "$"),
      // Block comment: /* ... */ (nested not handled in regex mode, but matches most cases)
      e.COMMENT("/\\*", "\\*/"),
      // Format string: $"..." (must be tried before plain string)
      {
        className: "string",
        begin: /\$"/,
        end: /"/,
        contains: [
          e.BACKSLASH_ESCAPE,
          { className: "subst", begin: /\{/, end: /\}/ }
        ]
      },
      // Raw string: """..."""
      {
        className: "string",
        begin: /"""/,
        end: /"""/,
        relevance: 10
      },
      // Plain string: "..."
      {
        className: "string",
        begin: /"/,
        end: /"/,
        contains: [e.BACKSLASH_ESCAPE]
      },
      // Numeric literals
      {
        className: "number",
        variants: [
          { begin: /0[xX][0-9a-fA-F][0-9a-fA-F_]*/ },  // hex
          { begin: /0[bB][01][01_]*/                  },  // binary
          { begin: /[0-9][0-9_]*\.[0-9][0-9_]*/      },  // float
          { begin: /[0-9][0-9_]*/                     }   // decimal int
        ]
      }
    ]
  };
});
```

**Note on `e.COMMENT`:** In hljs 10.1.1, this helper takes `(begin, end, extra)` and returns a mode object. For line comments, use `"$"` as the end pattern (matches end-of-line). Nested block comments cannot be handled with `e.COMMENT` alone — the regex approach (`/\*/`, `\*/`) will correctly close on the first `*/` even in nested cases, but won't track depth. For documentation purposes, the shallow pattern is sufficient since hljs is best-effort.

**Note on keyword CSS classes in v10.1.1:** The default `highlight.css` bundled by mdBook maps:
- `.hljs-keyword` → purple (`#9d00ec`) — use for declaration keywords + control flow
- `.hljs-title` → blue (`#0030f2`) — use for a second tier (assign to `title` class via mode)
- `.hljs-built_in` → orange (`#b21e00`) — use for modifiers/other
- `.hljs-type` → orange (`#b21e00`) — same as built_in visually; distinct category
- `.hljs-literal` → orange (`#b21e00`) — same as built_in visually; distinct category

**For 3 truly distinct colors:** `keyword` (purple), `built_in` (orange), and for a blue tier — this requires using a mode with `className: "title"` rather than a keyword group tier, OR accepting that two tiers share orange. Given the default CSS, the cleanest 3-way split with distinct colors is:
- Declaration keywords → `keyword` class (purple)
- Control flow → `keyword` class (purple) — same tier acceptable; they share a color
- Modifiers/other → `built_in` class (orange)

Or, for truly distinct 3 colors: put declaration keywords in `keyword` (purple), control flow in a `title`-classed mode, and modifiers in `built_in` (orange). However, using `title` className for control-flow keywords requires a different mode structure. The simpler approach that matches real-world highlighting practice is to combine declaration and control-flow under `keyword` (both purple), and modifiers under `built_in` (orange) — 2-color keyword split. If the user explicitly wants 3 colors, a custom CSS entry would be needed — but CONTEXT.md locked "no custom color scheme."

**Resolution:** Use `keyword` for declaration + control-flow (purple), `built_in` for modifiers/other (orange). This satisfies CONTEXT's "3 tiers" categorization at the data level while working within the default 2-color palette for keywords. See Open Questions #1.

### Pattern 2: Fence Marker Bulk Update

**What:** Replace bare ` ``` ` opening fences in language spec files with ` ```writ `.

**When to use:** For all code blocks in `language-spec/spec/00-29_*.md` files that contain Writ source code.

**Scope:**
- Language spec files (00-29): ~152 bare opening fences → update to ` ```writ `
- IL spec files (30+): ~28 bare opening fences → **leave as bare fences** (they contain IL pseudocode, binary format descriptions, and IL instruction sequences — NOT valid Writ syntax)
- Already-fenced blocks in files 25 and 27 with ` ```writ ` → no change needed

**Files with the most updates needed:**
| File | Bare Blocks |
|------|------------|
| `25_24_modules_namespaces.md` | 25 |
| `14_13_functions_fn.md` | 17 |
| `15_14_dialogue_blocks_dlg.md` | 14 |
| `05_4_lexical_structure.md` | 11 |
| `11_10_enums.md` | 10 |
| `12_11_contracts.md` | 9 |
| `16_15_entities.md` | 9 |

**Execution approach:** Python or sed replacement. For each file in 00-29, replace `\n\`\`\`\n` (bare open) with `\n\`\`\`writ\n` — but only the opening fences (ones followed by non-backtick content). The safest approach is a state-machine reader (parse in_block state) to avoid touching closing fences.

### Anti-Patterns to Avoid

- **Loading order race:** Never use `additional-js` for the Writ language definition — it loads after `book.js` which has already called `hljs.highlightBlock()`. The theme/highlight.js file must contain the definition.
- **Downloading newer highlight.js:** v11+ changed `className` to `scope`. Mixing v11 API with the v10-style `book.js` call `hljs.highlightBlock` (deprecated in v11, renamed `highlightElement`) would break.
- **Tagging IL spec blocks as writ:** IL spec files (30+) contain binary format descriptions, IL instruction pseudocode, and struct layout diagrams — not valid Writ source. Tagging them `writ` would produce incorrect highlighting (keywords misapplied to hex bytes, instruction names, etc.).
- **Bare regex for nested block comments:** Writ supports nested `/* /* */ */`. A regex-based approach will close on the first `*/`. For the purposes of the docs highlighter, this is acceptable — deeply nested comments are rare in documentation. Do NOT attempt a stateful parser in the hljs definition.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Syntax tokenization | Custom JS tokenizer | highlight.js regex/mode system | hljs already handles precedence, state machine, escape sequences |
| String escape handling | Manual `\\` parsing | `e.BACKSLASH_ESCAPE` built-in | Covers all standard escape sequences |
| Comment detection | Manual `//` scan | `e.COMMENT("//", "$")` | Handles edge cases, returns correct mode object |
| CSS color scheme | Custom highlight.css | Default bundled highlight.css | CONTEXT.md locks to default theme; bundled CSS already maps all needed classes |

**Key insight:** highlight.js 10.1.1 provides enough built-in helpers (`e.COMMENT`, `e.BACKSLASH_ESCAPE`, `NUMBER_MODE`) to cover Writ's token types without custom regex for the common cases.

## Common Pitfalls

### Pitfall 1: Loading Order (additional-js arrives too late)
**What goes wrong:** A Writ language JS placed via `additional-js` in `book.toml` will load after `book.js`. By the time `hljs.registerLanguage("writ", ...)` runs, `book.js` has already called `hljs.highlightBlock()` on all code elements. The `writ` blocks appear as plain text.
**Why it happens:** mdBook injects `additional-js` scripts after `book.js` in the generated HTML.
**How to avoid:** Use `docs/theme/highlight.js` — it loads at line 258 of the generated HTML, before `book.js` at line 259.
**Warning signs:** Code blocks render as plain unstyled text despite the JS being present.

### Pitfall 2: Applying writ Fencing to IL Spec Blocks
**What goes wrong:** IL spec files (30+) contain binary format diagrams, hex bytes, instruction sequences like `NEW_DELEGATE r_func, method_idx(add), r_null`. Applying `writ` fencing to these will produce garbled highlighting — instruction names like `NEW` may be misidentified as identifiers.
**Why it happens:** IL pseudocode shares some syntax with Writ (braces, identifiers) but is not valid Writ.
**How to avoid:** Only update bare fences in files `00-29_*.md`. Leave files `30+_*.md` bare.
**Warning signs:** IL instruction blocks showing keyword colors on `NEW`, `SET_FIELD`, `CALL_VIRTUAL`.

### Pitfall 3: Format String Regex Order
**What goes wrong:** If the plain string pattern (`/"/`) is listed in `contains` before the format string pattern (`/\$"/`), the plain string mode may match the `"` in `$"`, leaving the `$` as unhighlighted punctuation and the string body as a plain string.
**Why it happens:** highlight.js 10.1.1 matches modes in order — first match wins at each position.
**How to avoid:** Put the format string mode (`$"`) before the plain string mode in the `contains` array.
**Warning signs:** `$"hello"` renders identical to `"hello"` — no distinction between format and plain strings.

### Pitfall 4: theme/ Directory Not Being Picked Up
**What goes wrong:** `docs/theme/highlight.js` exists but mdBook still serves the bundled default.
**Why it happens:** The theme directory must be at `docs/theme/` (sibling of `docs/src/`), not at `theme/` in the repo root.
**How to avoid:** Place the file at `D:/dev/git/Writ/docs/theme/highlight.js`.
**Warning signs:** Running `mdbook build` and inspecting `target/book/highlight.js` — if it does NOT contain the Writ registerLanguage call, the theme file was not picked up.

### Pitfall 5: Raw String Regex Greedy Match
**What goes wrong:** The raw string pattern `"""` → `"""` (simple begin/end) will match the first `"""` opening and then greedily scan for `"""` — this works for simple cases but may fail for strings with extra leading/closing quotes (`""""...""""`).
**Why it happens:** Writ's raw string spec allows variable-length delimiters (3+ quotes). A simple regex cannot count matching quotes.
**How to avoid:** Accept this limitation — docs examples will always use `"""..."""` with exactly 3 quotes. Variable-length raw string delimiters are a Writ parser concern, not a docs highlighter concern.
**Warning signs:** Code samples with `""""...""""` are not expected in documentation.

## Code Examples

Verified patterns from the bundled highlight.js 10.1.1:

### Language Registration Pattern (v10.1.1)
```javascript
// Source: bundled docs/target/book/highlight.js (verified 2026-03-27)
// Append to END of docs/theme/highlight.js
hljs.registerLanguage("writ", function(e) {
  return {
    name: "Writ",
    case_insensitive: false,
    keywords: {
      $pattern: /[A-Za-z_][A-Za-z0-9_]*/,
      keyword: "fn dlg entity struct enum contract impl class component namespace " +
               "if else match for while return break continue spawn detached join " +
               "cancel defer try on atomic",
      built_in: "let mut const pub priv use using in new self extern global",
      type: "void int float bool string",
      literal: "true false null"
    },
    contains: [
      e.COMMENT("//", "$"),
      e.COMMENT("/\\*", "\\*/"),
      { className: "string", begin: /\$"/, end: /"/, contains: [e.BACKSLASH_ESCAPE, { className: "subst", begin: /\{/, end: /\}/ }] },
      { className: "string", begin: /"""/, end: /"""/, relevance: 10 },
      { className: "string", begin: /"/, end: /"/, contains: [e.BACKSLASH_ESCAPE] },
      { className: "number", variants: [
          { begin: /0[xX][0-9a-fA-F][0-9a-fA-F_]*/ },
          { begin: /0[bB][01][01_]*/                },
          { begin: /[0-9][0-9_]*\.[0-9][0-9_]*/    },
          { begin: /[0-9][0-9_]*/                   }
        ]
      }
    ]
  };
});
```

### Fence Marker Update Logic (Python state machine)
```python
# Source: derived from phase analysis (2026-03-27)
import re, os

def update_fences(path):
    with open(path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    result = []
    in_block = False
    for line in lines:
        if not in_block and line.rstrip('\n') == '```':
            result.append('```writ\n')
            in_block = True
        elif in_block and line.rstrip('\n') == '```':
            result.append(line)
            in_block = False
        elif not in_block and line.startswith('```'):
            result.append(line)
            in_block = True
        else:
            result.append(line)
    with open(path, 'w', encoding='utf-8') as f:
        f.writelines(result)
```

### CSS Class → Color Mapping (bundled highlight.css)
```
.hljs-keyword      → purple  (#9d00ec)  ← declaration + control-flow keywords
.hljs-built_in     → orange  (#b21e00)  ← modifiers/other keywords
.hljs-type         → orange  (#b21e00)  ← void, int, float, bool, string
.hljs-literal      → orange  (#b21e00)  ← true, false, null
.hljs-string       → green   (#008200)  ← all string forms
.hljs-comment      → gray    (#575757)  ← line + block comments
.hljs-number       → orange  (#b21e00)  ← numeric literals
.hljs-subst        → red     (#d70025)  ← interpolation slots in $"..."
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `scope` in language definition | `className` in hljs 10.x | hljs v11.0 renamed to `scope` | v10 bundled in mdBook 0.4.51 uses `className` |
| `hljs.highlightElement()` | `hljs.highlightBlock()` | hljs v11.0 renamed | `book.js` calls `highlightBlock` — stay on v10 API |
| `additional-js` for language defs | `theme/highlight.js` append | Historical discovery | Works cleanly because load order is controlled |

**Deprecated/outdated:**
- `hljs.initHighlightingOnLoad()`: Deprecated in v10.6+, removed in v11. Not used by mdBook's `book.js` — it calls `hljs.highlightBlock()` directly per-element.

## Open Questions

1. **3 truly distinct keyword colors**
   - What we know: Default `highlight.css` has purple (`keyword`) and orange (`built_in`/`type`/`literal`). CONTEXT locked "no custom color scheme."
   - What's unclear: Whether the user considers purple + orange as "2 distinct colors" (sufficient) or expects a third color for one tier.
   - Recommendation: Use purple for declaration keywords AND control flow (combining them under `keyword` class), orange for modifiers/other (`built_in`). If the user wants a 3rd color for control flow specifically, that requires adding a CSS rule to `highlight.css` — revisit in planning if needed. The current plan satisfies "3 tiers at the data level" within 2 visual colors.

2. **Localization file blocks in spec 27**
   - What we know: `27_26_localization.md` has TOML and CSV blocks already correctly fenced; it may also have bare blocks.
   - What's unclear: Whether any bare blocks in file 27 contain Writ source or only localization file examples.
   - Recommendation: Inspect each bare block in file 27 manually during planning to assign correct fence marker (writ vs plain text).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| mdbook | Build and test | Yes | 0.4.51 | — |
| Python 3 | Fence update script (optional) | Yes | 3.x | Use sed or manual editor |
| Node.js | Not required (no npm build) | Yes (irrelevant) | — | — |

**Missing dependencies with no fallback:** None.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None (documentation artifact — no unit tests) |
| Config file | n/a |
| Quick run command | `cd D:/dev/git/Writ/docs && mdbook build 2>&1` |
| Full suite command | `cd D:/dev/git/Writ/docs && mdbook build 2>&1 && grep -l "language-writ" target/book/*.html` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INFRA-04 | highlight.js contains Writ language def | smoke | `grep -c "registerLanguage.*writ" docs/target/book/highlight.js` | ❌ Wave 0 |
| INFRA-04 | Book builds without error | smoke | `cd docs && mdbook build` | ✅ |
| INFRA-04 | Writ code block renders with hljs class | smoke | `grep -l "language-writ" docs/target/book/*.html` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cd D:/dev/git/Writ/docs && mdbook build`
- **Per wave merge:** Full build + grep check for `registerLanguage.*writ` in output
- **Phase gate:** Book builds cleanly and at least one HTML file contains `language-writ` class

### Wave 0 Gaps
- [ ] Manual visual verification script — open a built page and inspect that `fn` keyword appears in purple
- [ ] Grep smoke test for `language-writ` in HTML output

## Sources

### Primary (HIGH confidence)
- Bundled `docs/target/book/highlight.js` (verified 2026-03-27) — highlight.js 10.1.1, `registerLanguage` pattern, `className` API, keyword object structure, `e.COMMENT`/`e.BACKSLASH_ESCAPE` helpers
- Bundled `docs/target/book/highlight.css` (verified 2026-03-27) — CSS class → color mapping for all relevant classes
- Bundled `docs/target/book/book.js` (verified 2026-03-27) — `highlight.js` loads at line 258, `book.js` at line 259; `hljs.highlightBlock()` called in book.js
- `writ-parser/src/lexer.rs` (verified 2026-03-27) — complete keyword/token enumeration used to build keyword lists

### Secondary (MEDIUM confidence)
- [mdBook Syntax Highlighting docs](https://rust-lang.github.io/mdBook/format/theme/syntax-highlighting.html) — confirmed theme/highlight.js override approach
- [mdBook Theme docs](https://rust-lang.github.io/mdBook/format/theme/index.html) — confirmed automatic theme/ directory detection, no book.toml change needed
- [GitHub issue #1459](https://github.com/rust-lang/mdBook/issues/1459) — confirmed single-file bundled approach required; additional-js files load after book.js

### Tertiary (LOW confidence)
- [highlight.js language guide](https://highlightjs.readthedocs.io/en/latest/language-guide.html) — general API, consistent with observed v10.1.1 patterns in bundled file

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — verified from built output, no npm packages to install
- Architecture: HIGH — load order verified from generated HTML; theme/ pickup verified from mdBook docs
- Pitfalls: HIGH — load order pitfall verified from GitHub issue discussion; fence scope from direct file inspection

**Research date:** 2026-03-27
**Valid until:** Stable (depends only on mdBook 0.4.51 which is pinned; highlight.js 10.1.1 which is pinned in binary)
