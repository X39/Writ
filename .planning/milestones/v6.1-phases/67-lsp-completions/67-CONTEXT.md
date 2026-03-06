# Phase 67: LSP Completions - Context

**Gathered:** 2026-03-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix LSP auto-completion so users get accurate completions for (1) method calls on typed expressions (dot-completions) and (2) built-in namespaces like `log::` and `Option::`. This is a bug-fix phase — the completion infrastructure exists from v5.0 Phase 54 but currently returns empty/incorrect results for these cases.

</domain>

<decisions>
## Implementation Decisions

### Namespace completion scope
- `::` completions should support ALL FQN-prefixed definitions, not just `log::` and `Option::`
- This includes: `log::` (5 levels: trace/debug/info/warn/error), `Option::` (Some/None), `Result::` (Ok/Err), and user-defined enum variants (e.g. `MyEnum::VariantA`)
- The backend currently falls through to `identifier_completion` when trigger_char is `":"` — need new handler path for `::` prefix completions
- Synthetic log entries (FileId(u32::MAX)) should continue to be excluded from general identifier completions — they belong only in namespace-qualified completions

### Dot-completion fix approach
- Dot-completions already have full infrastructure: `build_dot_completions` handles Struct, Class, Entity, Enum, Array, Option types
- The backend strips the trailing `.`, re-analyzes via `analyze_standalone`, finds receiver via `expr_at_offset`, then calls `build_dot_completions`
- Bug likely in: (a) `expr_at_offset` not finding the receiver at `dot_offset.saturating_sub(1)`, (b) re-analysis producing different FileIds than expected (hardcoded `FileId(0)` assumption), or (c) the modified source not compiling cleanly after dot removal
- Fix must ensure the resolved type from the type checker is used, not a fallback empty list (per SC4)

### Test coverage
- Unit tests in `completion.rs` for new namespace completion function
- Integration tests in `analysis_host.rs` matching existing patterns (build_typed_ast_full helper)
- Existing dot-completion tests should be verified and new ones added for the specific broken scenarios

### Claude's Discretion
- Text-based `::` prefix extraction implementation details
- Exact approach to diagnosing why `expr_at_offset` fails for dot-completion receivers
- Whether to use cached analysis or re-analysis for colon-triggered completions
- Error handling for malformed namespace prefixes

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### LSP completion infrastructure
- `writ-lsp/src/queries/completion.rs` — All completion query functions: `build_identifier_completions`, `build_dot_completions`, `build_signature_help`
- `writ-lsp/src/backend.rs` lines 461-551 — Completion handler: trigger character dispatch (`.` vs `:` vs none), dot-completion flow (strip dot, re-analyze, find receiver type)
- `writ-lsp/src/analysis_host.rs` — `AnalysisHost::analyze_standalone` and `analyze_project` methods used for re-analysis

### Type system / DefMap
- `writ-compiler/src/resolve/def_map.rs` — `DefMap` structure: `by_fqn` (FQN -> DefId), `file_private` (FileId -> name -> DefId)
- `writ-compiler/src/resolve/prelude.rs` — Prelude constants: `PRELUDE_TYPE_NAMES` (Option, Result, etc.), `SUB_PRELUDE_VARIANT_NAMES` (None, Some), `LOG_LEVEL_NAMES`
- `writ-compiler/src/resolve/mod.rs` lines 25-55 — `inject_log_namespace`: creates synthetic DefIds with FQN `"log::{level}"`, FileId(u32::MAX)

### Expression walking
- `writ-lsp/src/queries/walk.rs` — `expr_at_offset` function used to find receiver expression for dot-completions

### Requirements
- `.planning/REQUIREMENTS.md` — LSP-01 (dot-completions), LSP-02 (namespace completions)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `build_dot_completions()`: Already handles Struct/Class/Entity/Enum/Array/Option — just needs to be reached with correct receiver type
- `build_identifier_completions()`: Handles keywords, prelude, and DefMap entries — provides template for new namespace completion function
- `DefMap.by_fqn`: Contains entries like `"log::info"`, `"log::debug"` — can filter by prefix to build namespace completions
- `prelude::LOG_LEVEL_NAMES`: Static list of log level names
- `prelude::PRELUDE_TYPE_NAMES`: Static list including "Option", "Result" — useful for namespace prefix detection
- `type_env.enum_variants`: Map from DefId to variant list — needed for user-defined enum `::` completions

### Established Patterns
- Trigger character dispatch in `completion()` handler: check trigger_char, branch to specialized completion function
- Re-analysis pattern for dot-completion: strip trigger char, run `analyze_standalone`, query result
- Integration tests use `build_typed_ast_full()` helper for unit-level testing and `AnalysisHost::analyze_standalone()` for integration-level
- Synthetic entries use `FileId(u32::MAX)` sentinel to distinguish from user code

### Integration Points
- `backend.rs` completion handler — add new branch for `":"` trigger character
- `completion.rs` — add new `build_namespace_completions()` function
- `backend.rs` line 94 — trigger characters already include `":"` (registered but unused)

</code_context>

<specifics>
## Specific Ideas

No specific requirements — standard LSP completion behavior expected. Success criteria from roadmap are precise: dot-completion shows correct methods, `log::` shows 5 levels, `Option::` shows Some/None.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 67-lsp-completions*
*Context gathered: 2026-03-18*
