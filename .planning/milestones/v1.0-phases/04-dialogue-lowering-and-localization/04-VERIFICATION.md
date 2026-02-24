---
phase: 04-dialogue-lowering-and-localization
verified: 2026-02-26T20:30:00Z
status: passed
score: 5/5 must-haves verified
gaps: []
human_verification: []
---

# Phase 4: Dialogue Lowering and Localization Verification Report

**Phase Goal:** `dlg` blocks are fully lowered to `fn` declarations with correct speaker resolution, say/choice/transition semantics, localization keys, and all dialogue-specific validation errors
**Verified:** 2026-02-26T20:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (from ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `@Speaker text` lowers to `say_localized(speaker_ref, key, "text")`; standalone `@Speaker` sets active speaker without leaking into sibling `$ choice` branches | VERIFIED | `dlg_speaker_singleton_tier2.snap` shows `say_localized(_narrator, "c63a5e3a", "Welcome...")`. `dlg_choice_speaker_scope_isolation.snap` shows arm B uses `_narrator`, not `_player` — no cross-arm leak. |
| 2 | Speaker resolution handles all three tiers: params direct, singletons via `Entity.getOrCreate<T>()`, unknown TextLine emits `LoweringError` | VERIFIED | Tier 1: `dlg_speaker_param_tier1.snap` — `player` param used directly, no hoisting. Tier 2: `dlg_speaker_singleton_tier2.snap` — `let _narrator = Entity.getOrCreate<Narrator>()` hoisted at fn top. Tier 3: `dlg_text_without_speaker_error.snap` — `UnknownSpeaker { name: "", span }` error emitted, `AstExpr::Error` used as placeholder. |
| 3 | `->` and `-> target(args)` lower to `return target()` / `return target(args)`; `->` before end of block produces `LoweringError` | VERIFIED | `dlg_transition_at_end.snap` — `AstStmt::Return { value: Call { callee: Ident("farewell") } }`. `dlg_non_terminal_transition_error.snap` — `NonTerminalTransition { span }` emitted and lowering continues. |
| 4 | FNV-1a 8-char hex auto-keys; identical text gets distinct keys (occurrence_index); manual `#key` overrides auto-keys; duplicate `#key` produces `LoweringError` | VERIFIED | `dlg_loc_key_is_8char_hex.snap` — keys `"ae3cbcf6"` and `"5adf80cb"` (8-char hex). `dlg_loc_key_distinct_for_duplicate_text.snap` — identical "Move along." lines get `"5268ffd5"` and `"5168fe42"` (distinct, occurrence_index differs). `dlg_loc_key_manual_override.snap` — key is `"greeting"` literal. `dlg_loc_key_duplicate_collision.snap` — `DuplicateLocKey { key: "greet", first_span, second_span }`. |
| 5 | Snapshot tests cover three-tier speaker resolution, `$ choice` scoping, `->` transitions, `#key` override and collision, `{expr}` interpolation | VERIFIED | 17 accepted insta snapshot files in `writ-compiler/tests/snapshots/`, all 46 tests pass (0 failures). |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/lower/dialogue.rs` | `lower_dialogue` entry point with all 8 DlgLine variants, FNV-1a keys, speaker resolution, choice scoping, transition validation | VERIFIED | 708 lines. All 8 variants handled in `lower_dlg_lines`: `SpeakerLine`, `SpeakerTag`, `TextLine`, `CodeEscape`, `Choice`, `If`, `Match`, `Transition`. All helpers private (`fn`, not `pub fn`). |
| `writ-compiler/src/lower/mod.rs` | Updated wiring — `pub mod dialogue;`, `use lower_dialogue`, both `Item::Dlg` call sites | VERIFIED | Line 8: `pub mod dialogue;`. Line 28: `use crate::lower::dialogue::lower_dialogue;`. Line 109 (top-level): `decls.push(AstDecl::Fn(lower_dialogue(dlg_decl, dlg_span, &mut ctx)));`. Line 329 (namespace block): `decls.push(AstDecl::Fn(lower_dialogue(dlg_decl, dlg_span, ctx)));`. |
| `writ-compiler/src/lower/stmt.rs` | `Stmt::DlgDecl` lowers to `AstStmt::Let` wrapping `Lambda`; `Stmt::Transition` lowers to `AstStmt::Return` | VERIFIED | Lines 72–96: `Stmt::DlgDecl` calls `lower_dialogue` then wraps result in `AstStmt::Let { value: AstExpr::Lambda }`. Lines 98–117: `Stmt::Transition` returns `AstStmt::Return { value: Call { target } }`. Zero `todo!("Phase 4")` stubs remain. |
| `writ-compiler/src/lower/context.rs` | `speaker_stack_depth()` method added | VERIFIED | Lines 74–77: `pub fn speaker_stack_depth(&self) -> usize { self.speaker_stack.len() }`. Used in `lower_choice` for save/restore. |
| `writ-compiler/tests/lowering_tests.rs` | `lower_src_with_errors` helper + 17 new R8-R11 snapshot tests | VERIFIED | Lines 17–23: `lower_src_with_errors` returns `(Ast, Vec<LoweringError>)` without asserting errors empty. 17 R8-R11 tests present (lines 285–436). |
| `writ-compiler/tests/snapshots/` | 17 accepted insta snapshot files for all dialogue tests | VERIFIED | All 17 files present: `lowering_tests__dlg_choice_basic.snap`, `dlg_choice_speaker_scope_isolation.snap`, `dlg_code_escape_statement.snap`, `dlg_conditional_if.snap`, `dlg_loc_key_distinct_for_duplicate_text.snap`, `dlg_loc_key_duplicate_collision.snap`, `dlg_loc_key_is_8char_hex.snap`, `dlg_loc_key_manual_override.snap`, `dlg_multiple_speakers_hoisting.snap`, `dlg_non_terminal_transition_error.snap`, `dlg_speaker_param_tier1.snap`, `dlg_speaker_singleton_tier2.snap`, `dlg_speaker_tag_sets_active.snap`, `dlg_text_interpolation.snap`, `dlg_text_without_speaker_error.snap`, `dlg_transition_at_end.snap`, `dlg_transition_with_args.snap`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `dialogue.rs` | `mod.rs` | `lower_dialogue` called from both `Item::Dlg` arms | WIRED | `use crate::lower::dialogue::lower_dialogue` at line 28; called at lines 109 and 329 |
| `dialogue.rs` | `context.rs` | `push_speaker`, `pop_speaker`, `current_speaker`, `speaker_stack_depth` | WIRED | All four methods called in `dialogue.rs`: `push_speaker` in `SpeakerTag` arm; `pop_speaker` and `speaker_stack_depth` in `lower_choice`; `current_speaker` in `TextLine` arm |
| `dialogue.rs` | `error.rs` | `LoweringError::UnknownSpeaker`, `NonTerminalTransition`, `DuplicateLocKey` via `ctx.emit_error` | WIRED | All three error variants emitted: `UnknownSpeaker` at line 257, `NonTerminalTransition` at line 307, `DuplicateLocKey` at line 356 |
| `dialogue.rs` | `expr.rs` | `lower_expr` called for code escapes and interpolated expressions | WIRED | `lower_expr` called in `lower_dlg_text` (line 444) and `lower_dlg_if`/`lower_dlg_match`/`lower_transition` for expression positions |
| `lowering_tests.rs` | `dialogue.rs` | `lower_src`/`lower_src_with_errors` dispatches `Item::Dlg` to `lower_dialogue` | WIRED | `dlg_speaker_singleton_tier2.snap` shows `Entity.getOrCreate<Narrator>()` hoisting — proof the full chain executes |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| R8 | 04-01, 04-02 | Dialogue Lowering — `dlg` to `fn` with speaker resolution, all DlgLine variants, `$ choice`/`if`/`match`, `->` transition | SATISFIED | `dialogue.rs` handles all 8 variants. 10 R8 happy-path snapshot tests pass: `dlg_speaker_param_tier1`, `dlg_speaker_singleton_tier2`, `dlg_speaker_tag_sets_active`, `dlg_text_interpolation`, `dlg_code_escape_statement`, `dlg_choice_basic`, `dlg_conditional_if`, `dlg_transition_at_end`, `dlg_transition_with_args`, `dlg_multiple_speakers_hoisting`. |
| R9 | 04-01, 04-02 | Localization Key Generation — FNV-1a 8-char hex, manual `#key` overrides, deterministic | SATISFIED | `fnv1a_32()` in `dialogue.rs` lines 391–400 — exact spec algorithm. Snapshots show 8-char hex keys (e.g., `"c63a5e3a"`, `"ae3cbcf6"`). `dlg_loc_key_manual_override.snap` shows `"greeting"` literal as key. `dlg_loc_key_distinct_for_duplicate_text.snap` shows `"5268ffd5"` vs `"5168fe42"` for identical text lines (occurrence_index differs). |
| R10 | 04-01, 04-02 | Localization Key Collision Detection — duplicate `#key` within a `dlg` block | SATISFIED | `compute_or_use_loc_key` in `dialogue.rs` checks `state.manual_keys` HashMap. `dlg_loc_key_duplicate_collision.snap` shows `DuplicateLocKey { key: "greet", first_span: 32..37, second_span: 54..59 }`. |
| R11 | 04-01, 04-02 | Dialogue Transition Validation — `->` must be last in block | SATISFIED | Non-terminal check at `dialogue.rs` line 306: `if i < len - 1 { ctx.emit_error(NonTerminalTransition) }`. `dlg_non_terminal_transition_error.snap` shows error emitted AND lowering continues (post-transition line is still lowered). |

**Orphaned requirements check:** No additional requirements mapped to Phase 4 in REQUIREMENTS.md beyond R8, R9, R10, R11. No orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `writ-compiler/src/lower/stmt.rs` | 13 | Doc comment says `Stmt::DlgDecl` and `Stmt::Transition` use `todo!()` — outdated comment | Info | Comment describes old state; actual code at lines 72–117 is fully implemented. No behavioral impact. |
| `D:/dev/git/Writ/.planning/ROADMAP.md` | 76 | `04-02-PLAN.md` marked `[ ]` (not checked) despite being complete | Info | Documentation inconsistency only. All code and tests are implemented and passing. |

No blocker or warning anti-patterns found in code. The `todo!("Phase 5: entity lowering")` at mod.rs line 111 is correct and expected — Phase 5 has not started.

### Human Verification Required

None. All success criteria are verifiable programmatically via snapshot tests and cargo build output.

### Test Count Verification

- Pre-Phase 4 tests: 29 (R3/R4/R5/R6/R7 snapshot tests)
- Phase 4 tests: 17 (R8-R11 dialogue snapshot tests)
- Total: 46 tests, 0 failures (confirmed by `cargo test -p writ-compiler`)

### FNV-1a Algorithm Verification

The implementation at `dialogue.rs` lines 391–400 matches spec section 25.2.2 exactly:
- `OFFSET_BASIS = 0x811c9dc5`
- `PRIME = 0x01000193`
- XOR-then-multiply per byte
- `format!("{:08x}", hash)` — 8-char lowercase hex output
- Input format: `namespace\0method\0speaker\0content\0occurrence_index`
- No external crate used

The two identical "Move along." lines produce `"5268ffd5"` (occurrence_index=0) and `"5168fe42"` (occurrence_index=1) — confirming the occurrence_index disambiguator is working.

### Speaker Scope Isolation Verification

The `dlg_choice_speaker_scope_isolation.snap` snapshot proves the choice arm scope save/restore works correctly:
- Arm "A": uses `_player` (from `@Player I choose A.` SpeakerLine in that arm)
- Arm "B": uses `_narrator` (span `14..22`) — the outer stack-pushed Narrator, proving no Player leakage from arm A

The `speaker_stack_depth()` + pop-back-to-depth pattern in `lower_choice` correctly isolates each arm's speaker state.

---

*Verified: 2026-02-26T20:30:00Z*
*Verifier: Claude (gsd-verifier)*
