# Phase 88: Writ Syntax Highlighting - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Writ code blocks across all mdBook chapters render with syntax highlighting — keywords, strings, comments, types, and dialogue/entity/spawn constructs each styled distinctly. Custom highlight.js language definition injected via docs/theme/ without patching mdBook internals.

</domain>

<decisions>
## Implementation Decisions

### Keyword Categorization
- 3 keyword tiers with distinct colors: declaration keywords (`fn`, `dlg`, `entity`, `struct`, `enum`, `contract`, `impl`, `class`, `component`, `namespace`), control flow (`if`, `else`, `match`, `for`, `while`, `return`, `break`, `continue`, `spawn`, `detached`, `join`, `cancel`, `defer`, `try`, `on`, `atomic`), modifiers/other (`let`, `mut`, `const`, `pub`, `priv`, `use`, `using`, `in`, `new`, `self`, `extern`, `global`)
- Built-in types (`void`, `int`, `float`, `bool`, `string`) get highlight.js `type` class, distinct from keywords
- Literals (`true`, `false`, `null`) get highlight.js `literal` class
- Runtime builtins (`say`, `choice`, `log`) are NOT highlighted as keywords — they are compiler-injected, not reserved words

### Integration Approach
- Custom highlight.js language file registered in `docs/theme/highlight.js` — mdBook's documented approach
- Code blocks use ` ```writ ` fence marker as the language identifier
- Default highlight.js theme colors used (no custom color scheme) — works with both light and dark mdBook themes
- Existing spec code blocks updated to use ` ```writ ` fencing where they contain Writ code

### Claude's Discretion
- Exact highlight.js API usage and registration pattern
- How to handle format strings (`$"..."`) and raw strings (`"""..."""`)
- Operator highlighting approach
- Any edge cases in string/comment detection

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-parser/src/lexer.rs` — complete token/keyword definitions (Logos lexer with all Writ tokens)
- `docs/book.toml` — already configured with mdbook-admonish, ready for theme additions
- `docs/src/` — 68 chapter wrapper files with spec content

### Established Patterns
- mdBook theme customization via `docs/theme/` directory
- Spec files in `language-spec/spec/` contain code blocks that need ` ```writ ` fencing

### Integration Points
- `docs/book.toml` — may need `[output.html]` theme configuration
- `docs/theme/highlight.js` — new file for custom language registration
- `language-spec/spec/*.md` — code blocks need fence marker updates

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard highlight.js language registration approaches.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
