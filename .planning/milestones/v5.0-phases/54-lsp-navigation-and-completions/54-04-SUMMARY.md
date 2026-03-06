---
phase: 54-lsp-navigation-and-completions
plan: 04
subsystem: writ-lsp
tags: [semantic-tokens, lsp, vscode, highlighting]
dependency_graph:
  requires: ["54-01", "54-02", "54-03"]
  provides: ["semantic_tokens_full handler", "collect_semantic_tokens", "semanticTokenScopes"]
  affects: ["writ-vscode extension", "writ-lsp backend"]
tech_stack:
  added: []
  patterns: ["TypedAst walk for semantic tokens", "LSP delta-encoding", "TextMate scope mapping"]
key_files:
  created: []
  modified:
    - writ-lsp/src/queries.rs
    - writ-lsp/src/backend.rs
    - writ-vscode/package.json
decisions:
  - "TOKEN_TYPE_KEYWORD and TOKEN_TYPE_PARAMETER are defined but unused — annotated with #[allow(dead_code)] for future use"
  - "ComponentAccess component name token uses span.end - component.len() offset; spans the identifier portion only"
  - "SimpleSpan constructed as struct literal { start, end, context: () } — SimpleSpan::new does not exist"
  - "FileId for semantic_tokens_full resolved by URI-to-path matching against file_sources; falls back to FileId(0)"
metrics:
  duration: 10min
  completed_date: "2026-03-14"
  tasks_completed: 2
  files_modified: 3
---

# Phase 54 Plan 04: Semantic Token Highlighting Summary

**One-liner:** Semantic highlighting with TypedAst walk emitting entity/type/function/component/variable tokens, delta-encoded to VS Code via tower-lsp, mapped to TextMate scopes in package.json.

## What Was Built

### Task 1: collect_semantic_tokens + semantic_tokens_full handler

Added to `writ-lsp/src/queries.rs`:

- `RawSemanticToken` struct with absolute position fields (line, start_char, length, token_type)
- Token type constants (TOKEN_TYPE_ENTITY=2, TOKEN_TYPE_COMPONENT=3, etc.) matching the legend registered in ServerCapabilities
- `collect_semantic_tokens(ast, interner, source, file_id) -> Vec<RawSemanticToken>` — walks all TypedDecl variants, emitting declaration name tokens, then recurses into bodies via `collect_tokens_in_expr`
- `collect_tokens_in_expr` — handles entity-typed Var references (TOKEN_TYPE_ENTITY) and ComponentAccess component name tokens (TOKEN_TYPE_COMPONENT); recurses into all sub-expressions
- `collect_tokens_in_stmts/stmt` — statement-level recursion
- `push_token_for_span` — converts SimpleSpan byte offsets to UTF-16 positions via `offset_to_position`

Added to `writ-lsp/src/backend.rs`:

- `semantic_tokens_full` handler in the `LanguageServer` impl block
- Retrieves source + analysis cache, resolves FileId from URI by matching file_sources paths
- Calls `collect_semantic_tokens`, delta-encodes the sorted raw tokens into `Vec<SemanticToken>`
- Returns `SemanticTokensResult::Tokens`

### Task 2: VS Code semanticTokenScopes

Updated `writ-vscode/package.json` contributes section with:

```json
"semanticTokenScopes": [{
  "language": "writ",
  "scopes": {
    "entity": ["entity.name.type.writ"],
    "component": ["entity.name.type.component.writ"],
    "dialogueSpeaker": ["entity.name.type.speaker.writ"]
  }
}]
```

Standard token types (KEYWORD, TYPE, FUNCTION, VARIABLE, PARAMETER) are handled automatically by VS Code.

## Token Coverage

| Declaration type | Token type emitted |
|------------------|--------------------|
| entity Foo { }   | entity (2)         |
| struct Foo { }   | type (1)           |
| class Foo { }    | type (1)           |
| enum Foo { }     | type (1)           |
| contract Foo { } | type (1)           |
| fn foo() { }     | function (5)       |
| component Foo {} | component (3)      |
| extern component | component (3)      |
| const FOO = ...  | variable (6)       |
| global FOO = ... | variable (6)       |
| entity-typed var | entity (2)         |
| entity.Component | component (3)      |

## Tests Added

- `test_semantic_tokens_entity_decl` — entity declaration name gets entity token type
- `test_semantic_tokens_struct_decl` — struct declaration name gets type token type
- `test_semantic_tokens_entity_var_ref` — variable with entity type gets entity token type
- `test_semantic_tokens_sorted` — output is sorted by (line, start_char); at least 3 tokens for Foo/Bar/baz

All 42 writ-lsp tests pass.

## Commits

| Hash    | Description                                                          |
|---------|----------------------------------------------------------------------|
| b18d51a | feat(54-04): implement semantic tokens - collect_semantic_tokens and handler |
| 3894060 | feat(54-04): add semanticTokenScopes to VS Code extension package.json |

## Deviations from Plan

None — plan executed exactly as written, with one minor fix: `SimpleSpan` has no `::new()` constructor; used struct literal `SimpleSpan { start, end, context: () }` instead.

## Self-Check: PASSED
