---
phase: 64-cross-crate-file-splits
plan: "02"
subsystem: writ-lsp
tags: [file-split, lsp, queries, refactor]
dependency_graph:
  requires: []
  provides: [writ-lsp/src/queries/ folder module]
  affects: [writ-lsp/src/backend.rs (unchanged, all crate::queries::* paths stable)]
tech_stack:
  added: []
  patterns: [pub(super) visibility for shared helpers across sibling submodules]
key_files:
  created:
    - writ-lsp/src/queries/mod.rs
    - writ-lsp/src/queries/walk.rs
    - writ-lsp/src/queries/hover.rs
    - writ-lsp/src/queries/references.rs
    - writ-lsp/src/queries/completion.rs
    - writ-lsp/src/queries/semantic.rs
  modified: []
  deleted:
    - writ-lsp/src/queries.rs
decisions:
  - "collect_references placed in references.rs (not hover.rs) per artifact spec, despite being in the Hover+References section of the original file — matches the logical ownership boundary"
  - "decl_file_id placed in walk.rs as pub(super) — used by hover.rs and references.rs via super::walk::decl_file_id()"
  - "Tests distributed to submodule files with correct writ_compiler API (lower+resolve+typecheck pipeline, not simplified API)"
  - "completion.rs and semantic.rs exceed 600-line limit only because of distributed inline tests; production code sections are 531 and 395 lines respectively"
metrics:
  duration: "~20 minutes"
  completed: "2026-03-18"
  tasks: 1
  files: 7
---

# Phase 64 Plan 02: writ-lsp queries.rs Split Summary

Split `writ-lsp/src/queries.rs` (2,768 lines) into a `queries/` folder module with 6 files organized by LSP query category. All 86 writ-lsp tests pass, zero clippy warnings, and all `crate::queries::*` call sites in backend.rs remain unchanged via mod.rs re-exports.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Split queries.rs into queries/ folder module | df4cf7c | queries/mod.rs, walk.rs, hover.rs, references.rs, completion.rs, semantic.rs |

## Output Files

| File | Lines | Contents |
|------|-------|----------|
| `writ-lsp/src/queries/mod.rs` | 43 | Module declarations + 20 individual pub use re-exports |
| `writ-lsp/src/queries/walk.rs` | 382 | position_to_byte_offset, expr_at_offset, find_def_id_at_offset, pub(super) decl_file_id, walk helpers |
| `writ-lsp/src/queries/hover.rs` | 476 | hover_text_for_expr, hover_text_for_def, extract_doc_comment, pattern_at_offset, PatternHoverInfo, format_fn_sig_hover |
| `writ-lsp/src/queries/references.rs` | 562 | collect_references, collect_refs_in_*, BindingInfo, binding_at_offset, def_at_offset, type_ann_def_id_at_offset |
| `writ-lsp/src/queries/completion.rs` | 720 | build_identifier_completions, build_dot_completions, build_signature_help, format_fn_sig_oneliner, find_enclosing_call |
| `writ-lsp/src/queries/semantic.rs` | 622 | RawSemanticToken, collect_semantic_tokens, collect_dialogue_speaker_tokens, push_token_for_span |

## Key Design Decisions

**collect_references placement:** The original file had `collect_references` at line 672 within the "Hover and References queries" section (lines 273-866). Per the plan's artifact spec, `collect_references` goes in `references.rs`. The plan's line ranges were slightly off — the actual split puts collect_references in references.rs alongside BindingInfo, binding_at_offset, def_at_offset, type_ann_def_id_at_offset. This matches the logical ownership boundary.

**decl_file_id visibility:** Made `pub(super)` in walk.rs. Used by hover.rs via `super::walk::decl_file_id` and references.rs via `super::walk::decl_file_id`. This follows the same pattern used in Phase 63 for shared helpers.

**Test distribution:** Tests from the original monolithic `#[cfg(test)] mod tests { ... }` block were distributed to their respective submodule files. Each test helper (`build_typed_ast`, `build_typed_ast_full`) is duplicated in each module that needs it, using the full `writ_compiler::lower + resolve + typecheck` pipeline.

## Deviations from Plan

**File size limit:** completion.rs (720 lines) and semantic.rs (622 lines) exceed the 600-line guideline. Production code only is 531 and 395 lines respectively — both well under 600 lines. The excess comes from distributed inline tests, which were required by the plan's Step 8. This is an internal conflict in the plan spec.

**hover.rs does not contain collect_references:** The plan's action text said hover.rs covers "Lines 273-866" which includes collect_references (line 672). However, the plan's artifact spec says references.rs "contains: pub fn collect_references". The artifact spec was followed.

## Verification Results

- `cargo test -p writ-lsp`: 86 passed, 0 failed
- `cargo clippy -p writ-lsp -- -D warnings`: 0 warnings, 0 errors
- All `crate::queries::*` call sites in backend.rs resolve via mod.rs re-exports
- No glob re-exports (`pub use *`) in mod.rs — all re-exports are explicit

## Self-Check: PASSED

- queries/mod.rs: FOUND
- queries/walk.rs: FOUND
- queries/hover.rs: FOUND
- queries/references.rs: FOUND
- queries/completion.rs: FOUND
- queries/semantic.rs: FOUND
- queries.rs deleted: CONFIRMED
- commit df4cf7c: FOUND
