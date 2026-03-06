---
phase: 54-lsp-navigation-and-completions
verified: 2026-03-14T00:00:00Z
status: passed
score: 13/13 must-haves verified
re_verification: null
gaps: []
human_verification:
  - test: "Open a .writ file in VS Code with writ-lsp running; hover a variable identifier"
    expected: "A tooltip appears showing `varname: TypeName` in a writ code block"
    why_human: "Visual tooltip rendering and editor integration cannot be verified programmatically"
  - test: "Type a struct-typed variable name followed by '.' in the editor"
    expected: "Completion popup shows the struct's fields and methods with type details"
    why_human: "LSP completion UI rendering in VS Code editor requires live session"
  - test: "Place cursor inside a function call argument list and trigger signature help"
    expected: "Signature overlay shows parameter names/types; active parameter highlights as comma is typed"
    why_human: "Active parameter highlighting is a VS Code UI behavior"
  - test: "Right-click an entity type reference and select 'Find All References'"
    expected: "References panel shows all use-sites across the project"
    why_human: "Multi-file reference collection across real project files requires manual validation"
  - test: "Open a file with an entity definition; observe syntax coloring vs a struct definition"
    expected: "Entity name has a different color from struct name (semantic highlighting active)"
    why_human: "TextMate scope colorization in VS Code is visual and theme-dependent"
---

# Phase 54: LSP Navigation and Completions Verification Report

**Phase Goal:** Users can navigate their Writ codebase by hovering, jumping to definitions, finding references, and getting completions — all backed by the full type-checked AST.
**Verified:** 2026-03-14
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | AnalysisResult carries typed AST, type interner, and TypeEnv after analysis | VERIFIED | `analysis_host.rs:10-21` — three Optional fields: `typed_ast`, `ty_interner`, `type_env`; both `analyze_standalone` and `analyze_project` populate them on Ok from the 4-tuple typecheck return |
| 2 | TyInterner can display named types using their actual names | VERIFIED | `ty.rs:178` — `pub fn display_named` resolves Struct/Class/Entity/Enum DefIds to `def_map.get_entry(def_id).name`; falls back to `display()` for primitives |
| 3 | A cursor position can be mapped to the innermost TypedExpr at that offset | VERIFIED | `queries.rs:58-91` — `expr_at_offset` walks all Fn/Impl/Const/Global bodies, uses `find_in_expr` with `update_best` for narrowest-span selection |
| 4 | Hover and goto-def handlers read from analysis cache without re-running the pipeline | VERIFIED | `backend.rs:162-205,207-275` — both `hover` and `goto_definition` read `self.analysis_cache.get(&uri_str)` before any query |
| 5 | Hovering an identifier shows its type name or function signature in a tooltip | VERIFIED | `queries.rs:250-298` — `hover_text_for_expr` handles Var (name:type), Call (fn sig), Field, ComponentAccess, New, SelfRef, Path, Error; wired at `backend.rs:192-204` |
| 6 | Go-to-definition on an identifier jumps to its declaration location | VERIFIED | `backend.rs:207-275` — resolves DefId, guards `FileId(u32::MAX)` builtins, returns `GotoDefinitionResponse::Scalar(Location)` with `entry.name_span` |
| 7 | Go-to-definition on builtins returns None instead of crashing | VERIFIED | `backend.rs:247-250` — `if entry.file_id == FileId(u32::MAX) { return Ok(None); }` |
| 8 | Find all references returns every use-site across project files | VERIFIED | `queries.rs:326-349` — `collect_references` walks all Fn/Impl/Const/Global bodies; `backend.rs:277-361` — maps spans to file sources by containment, falls back to trigger URI |
| 9 | Typing an identifier prefix shows keyword, type name, and definition completions | VERIFIED | `queries.rs:532-614` — `build_identifier_completions` emits 34 keywords + 5 primitives + 5 prelude types + 17 contracts + all non-synthetic DefMap entries; wired at `backend.rs:449-451,718-750` |
| 10 | Typing . after a struct-typed expression shows its fields and methods | VERIFIED | `queries.rs:629-654` — `build_dot_completions` handles `TyKind::Struct`/`Class` with `struct_fields` + `impl_index`; wired at `backend.rs:381-447` (dot trigger detection + re-analysis) |
| 11 | Typing . after an entity-typed expression shows properties, methods, and component names | VERIFIED | `queries.rs:655-691` — Entity branch reads `entity_fields`, `impl_index`, and `entity_components` (DIFF-02: `CompletionItemKind::MODULE` with detail "component") |
| 12 | Calling a function shows signature help with the active parameter highlighted | VERIFIED | `queries.rs:760-836` — `build_signature_help` backward-scans for `(`, comma-counts for `active_parameter`, looks up `fn_sigs`; wired at `backend.rs:453-495` |
| 13 | Semantic highlighting visually distinguishes entity/struct/component/function tokens | VERIFIED | `queries.rs:1005-1075` — `collect_semantic_tokens` emits entity(2), type(1), function(5), component(3), variable(6) tokens; `backend.rs:497-562` delta-encodes; `package.json:31-40` maps custom types to TextMate scopes |

**Score:** 13/13 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-lsp/src/queries.rs` | Position-to-node query functions for all LSP handlers | VERIFIED | 1,240+ lines; exports `position_to_byte_offset`, `expr_at_offset`, `find_def_id_at_offset`, `hover_text_for_expr`, `collect_references`, `build_identifier_completions`, `build_dot_completions`, `build_signature_help`, `collect_semantic_tokens`, `RawSemanticToken` |
| `writ-compiler/src/check/ty.rs` | `display_named` method on TyInterner | VERIFIED | Line 178: `pub fn display_named(&self, ty: Ty, def_map: &DefMap) -> String` |
| `writ-lsp/src/analysis_host.rs` | Extended AnalysisResult with typed_ast, ty_interner, type_env | VERIFIED | Lines 10-21: all three Optional fields declared; both analyze functions capture the 4-tuple |
| `writ-lsp/src/backend.rs` | All LSP handlers + analysis_cache | VERIFIED | `analysis_cache: DashMap<String, Arc<AnalysisResult>>` at line 32; hover(162), goto_definition(207), references(277), completion(363), signature_help(453), semantic_tokens_full(497) |
| `writ-vscode/package.json` | semanticTokenScopes contribution | VERIFIED | Lines 31-40: entity, component, dialogueSpeaker mapped to TextMate scopes |
| `writ-lsp/src/lib.rs` | `pub mod queries` declaration | VERIFIED | Line 4: `pub mod queries;` |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `analysis_host.rs` (typecheck call) | `writ_compiler::check::typecheck` | captures 4-tuple `(typed, interner, type_env, type_diags)` | WIRED | `analysis_host.rs:106` — `Ok((typed, interner, type_env, type_diags)) =>` stores all three in Option locals |
| `backend.rs` | `analysis_host.rs` | `analysis_cache` stores `Arc<AnalysisResult>` per URI | WIRED | `backend.rs:32` field + `backend.rs:602-603` — `Arc::new(analysis_result)` inserted on successful analysis |
| `queries.rs` | `writ-compiler/check/ir.rs` | walks TypedAst to find node at offset | WIRED | `queries.rs:58-91` — walks `TypedDecl::Fn/Impl/Const/Global` variants, recurses via `find_in_expr` |
| `backend.rs (hover)` | `queries.rs (expr_at_offset + hover_text_for_expr)` | finds TypedExpr at cursor, calls hover_text_for_expr | WIRED | `backend.rs:183-196` — `expr_at_offset` then `hover_text_for_expr` called sequentially |
| `backend.rs (goto_definition)` | `def_map.rs` | `find_def_id_at_offset -> DefMap::get_entry -> Location` | WIRED | `backend.rs:240-274` — `find_def_id_at_offset` → `get_entry(def_id)` → `span_to_range` → `Location` |
| `backend.rs (references)` | `queries.rs (collect_references)` | walks TypedAst collecting matching DefIds | WIRED | `backend.rs:315-316` — `collect_references(typed_ast, def_id, &typed_ast.def_map)` |
| `backend.rs (completion)` | `queries.rs (build_completions/build_dot_completions)` | trigger character detection routes to identifier or dot | WIRED | `backend.rs:381-450` — `trigger_char == Some(".")` → `build_dot_completions`; else → `identifier_completion` |
| `queries.rs (build_dot_completions)` | `check/env.rs (TypeEnv)` | struct_fields, entity_fields, entity_components, impl_index lookups | WIRED | `queries.rs:631-690` — direct lookups on `type_env.struct_fields`, `entity_fields`, `entity_components`, `impl_index` |
| `backend.rs (signature_help)` | `check/env.rs (TypeEnv::fn_sigs)` | looks up FnSig by callee DefId | WIRED | `backend.rs:486-492` → `build_signature_help` → `queries.rs:801` — `type_env.fn_sigs.get(&def_id)` |
| `backend.rs (semantic_tokens_full)` | `queries.rs (collect_semantic_tokens)` | collects tokens, delta-encodes, returns SemanticTokens | WIRED | `backend.rs:534` — `collect_semantic_tokens(typed_ast, interner, &source, file_id)` → delta loop at 537-556 |
| `package.json` | `backend.rs (SemanticTokensLegend)` | semanticTokenScopes maps custom types to TextMate scopes | WIRED | `package.json:31-40` matches the legend order: entity(idx 2), component(idx 3), dialogueSpeaker(idx 4) registered in `backend.rs:97-99` |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| LSP-02 | 54-01, 54-03 | User gets keyword and type name completions when typing identifiers | SATISFIED | `build_identifier_completions` returns 34 keywords + prelude names + all DefMap entries; confirmed by `test_identifier_completions_has_keywords` and `test_identifier_completions_has_prelude` |
| LSP-03 | 54-03 | User gets dot-completions for struct/class fields, methods, and entity components | SATISFIED | `build_dot_completions` handles Struct/Class fields + methods, Entity fields/methods/components; test `test_dot_completions_struct_fields` passes |
| LSP-04 | 54-01, 54-02 | User can hover any identifier to see its type, signature, or definition info | SATISFIED | `hover_text_for_expr` + `hover` handler fully wired; `test_hover_text_var` and `test_hover_text_fn_call` pass |
| LSP-05 | 54-01, 54-02 | User can go-to-definition on any identifier to jump to its declaration | SATISFIED | `goto_definition` handler resolves DefId → name_span → Location; builtin sentinel guard at `FileId(u32::MAX)` |
| LSP-06 | 54-02 | User can find all references of a definition across all files | SATISFIED | `collect_references` + `references` handler with file-source containment matching; `test_collect_references_finds_uses` passes |
| LSP-07 | 54-03 | User sees signature help with active parameter highlighted during function calls | SATISFIED | `build_signature_help` backward-scans for `(`, comma-counts for `active_parameter`; `test_signature_help_finds_param` passes |
| DIFF-01 | 54-01, 54-04 | Semantic highlighting distinguishes entity names, component types, dialogue speakers, keywords | SATISFIED | `collect_semantic_tokens` emits entity(2)/component(3)/type(1)/function(5) tokens; `semanticTokenScopes` in package.json maps to TextMate scopes; 4 semantic token tests pass |
| DIFF-02 | 54-03 | Dot-completion on entity-typed expressions shows available extern component types | SATISFIED | `build_dot_completions` Entity branch reads `type_env.entity_components`, emits `CompletionItemKind::MODULE` items; `test_dot_completions_entity_components` passes |

**All 8 phase requirement IDs satisfied. No orphaned requirements.**

---

### Anti-Patterns Found

No anti-patterns found in phase-modified files:
- No TODO/FIXME/HACK/PLACEHOLDER comments in queries.rs or backend.rs
- No stub implementations (`return null`, empty responses, `Not implemented`)
- No console.log-only handlers
- All handlers contain real logic (cache reads, query delegation, LSP response construction)

---

### Human Verification Required

The following items require manual verification in a live VS Code session:

#### 1. Hover Tooltip Display

**Test:** Open a `.writ` file, hover over a variable name (e.g., `x` in `let x: int = 5`)
**Expected:** Tooltip shows `` ```writ\nx: int\n``` `` rendered as a code block
**Why human:** Visual rendering of `HoverContents::Markup` in the editor UI cannot be checked programmatically

#### 2. Dot-Completion Popup

**Test:** Type a struct-typed variable followed by `.` (e.g., `my_point.`)
**Expected:** Completion popup lists the struct's fields and methods with type annotations
**Why human:** LSP completion list rendering and popup interaction require a live editor session

#### 3. Signature Help Active Parameter

**Test:** Type `foo(` for a two-parameter function, then type `,`
**Expected:** Signature overlay shows both parameters; second parameter highlights after the comma
**Why human:** Active parameter visual highlighting in VS Code signature help UI

#### 4. Cross-File Find References

**Test:** In a project with `writ.toml`, call find-all-references on a function defined in one file and called in another
**Expected:** References panel shows use-sites from both files
**Why human:** Cross-file span attribution in project mode requires real file system and multiple editor windows

#### 5. Entity vs Struct Visual Distinction (DIFF-01)

**Test:** Open a file with both `entity Player {}` and `struct Item {}` declarations; observe token coloring
**Expected:** `Player` and `Item` receive visually distinct colors based on semantic token type assignments
**Why human:** TextMate scope colorization depends on the active VS Code theme and cannot be verified without rendering

---

### Gaps Summary

No gaps. All automated checks passed. Phase goal is fully achieved at the code level.

The five human verification items above are expected for LSP features — they cannot be verified programmatically without a running editor session. All underlying implementation is in place and tested by the 42-test suite.

---

_Verified: 2026-03-14_
_Verifier: Claude (gsd-verifier)_
