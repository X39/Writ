---
phase: 07-dialogue-stack-fix-and-dead-code-cleanup
verified: 2026-02-27T12:00:00Z
status: passed
score: 4/4 must-haves verified
re_verification: false
---

# Phase 7: Dialogue Stack Fix and Dead Code Cleanup — Verification Report

**Phase Goal:** Close three integration gaps: (1) fix speaker stack leak in lower_dialogue(), (2) remove dead loc_key_counter/next_loc_key() code, (3) fix stale doc comment in stmt.rs
**Verified:** 2026-02-27
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `lower_dialogue()` drains the speaker stack to its pre-call depth before returning — no leaked SpeakerTag scopes persist across sequential dlg items | VERIFIED | Lines 111 and 118-120 of `dialogue.rs`: `let speaker_depth = ctx.speaker_stack_depth();` before `lower_dlg_lines`, followed by `while ctx.speaker_stack_depth() > speaker_depth { ctx.pop_speaker(); }` |
| 2 | `LoweringContext` no longer contains `loc_key_counter` field or `next_loc_key()` method | VERIFIED | `context.rs` has no `loc_key_counter` field, no `loc_key_counter: 0` initializer, and no `next_loc_key()` method. `grep` across all of `writ-compiler/src/` returns zero matches |
| 3 | Doc comment in `stmt.rs` accurately describes DlgDecl and Transition handling (no stale `todo!()` references) | VERIFIED | Lines 13-14 of `stmt.rs` now read: "`Stmt::DlgDecl` lowers to a `let` binding wrapping the dialogue body as a lambda. `Stmt::Transition` lowers to `AstStmt::Return` with the target as a `Call` expression." — no `todo!()` present |
| 4 | All 69 existing tests pass with zero snapshot changes | VERIFIED | `cargo test -p writ-compiler` output: `test result: ok. 69 passed; 0 failed; 0 ignored` |

**Score:** 4/4 truths verified

---

## Required Artifacts

| Artifact | Provides | Exists | Substantive | Wired | Status |
|----------|----------|--------|-------------|-------|--------|
| `writ-compiler/src/lower/dialogue.rs` | Speaker stack save/restore around `lower_dlg_lines` in `lower_dialogue()` | Yes | Yes — 117 lines of substantive dialogue lowering; save/restore at lines 111-120 | Yes — called from `mod.rs` pipeline | VERIFIED |
| `writ-compiler/src/lower/context.rs` | `LoweringContext` without dead `loc_key_counter` field and `next_loc_key()` method | Yes | Yes — 67 lines; struct has only `errors` and `speaker_stack`; `new()` has no `loc_key_counter: 0` init | Yes — used by all lowering passes | VERIFIED |
| `writ-compiler/src/lower/stmt.rs` | Accurate doc comment for `lower_stmt` describing DlgDecl→let+lambda and Transition→Return | Yes | Yes — doc comment at lines 8-14 is accurate; `DlgDecl` arm at lines 72-96 implements exactly the documented behavior; `Transition` arm at lines 98-117 implements exactly the documented behavior | Yes — called from `lower_dlg_lines` and `lower_fn` | VERIFIED |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-compiler/src/lower/dialogue.rs` | `writ-compiler/src/lower/context.rs` | `speaker_stack_depth()` + `pop_speaker()` drain loop | WIRED | `dialogue.rs` line 111: `ctx.speaker_stack_depth()` save; lines 118-120: drain loop calling `ctx.pop_speaker()`. Pattern also present in `lower_choice` at lines 526/545. `speaker_stack_depth()` defined in `context.rs` line 64. |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| R8 | `07-01-PLAN.md` | Dialogue Lowering — `lower_dialogue()` correctly handles all dialogue constructs including speaker scoping | SATISFIED | Speaker stack save/restore added to `lower_dialogue()`. Active speaker scoped correctly across `dlg` body boundaries — no leak across sequential dlg items. All R8 acceptance criteria in REQUIREMENTS.md are marked satisfied and remain so after this phase. |
| R11 | `07-01-PLAN.md` | Dialogue Transition Validation — transition semantics correct | SATISFIED | `stmt.rs` doc comment now accurately describes `Stmt::Transition` lowering. `Stmt::Transition` arm at `stmt.rs` lines 98-117 confirms the actual implementation matches the corrected doc comment. R11 acceptance criteria (NonTerminalTransition error, target call lowering) unaffected by this surgical change. |

**Orphaned requirements check:** No additional requirements mapped to Phase 7 in REQUIREMENTS.md beyond R8 and R11.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `dialogue.rs` | 415 | `// ... as literal placeholders.` (doc string for `raw_text_content`) | Info | Not an anti-pattern — the word "placeholder" appears in a function doc comment accurately describing behavior, not as a stub indicator. No action needed. |

No blockers. No stubs. No `TODO`/`FIXME`/`HACK` comments in any of the three modified files. No `allow(dead_code)` annotations in `context.rs`.

---

## Commit Verification

Both task commits are present in git history and correctly describe their contents:

- `27b2884` — `fix(07-01): fix speaker stack leak in lower_dialogue()`
- `32f43d7` — `chore(07-01): remove dead loc_key_counter code and fix stale doc comments`

---

## Human Verification Required

None. All three changes are verifiable from source inspection and test execution:

- Stack fix: structurally verified via source read, pattern confirmed against the existing `lower_choice` pattern
- Dead code removal: confirmed by zero `grep` matches across all of `writ-compiler/src/`
- Doc comment accuracy: confirmed by reading both the comment and the implementing arm in `lower_stmt`
- Test continuity: confirmed by `cargo test -p writ-compiler` — 69/69 pass, zero failures, zero snapshot changes

---

## Summary

Phase 7 achieves its goal completely. All three integration gaps identified in the v1.0 milestone audit are closed:

**Gap 1 (Speaker stack leak):** `lower_dialogue()` at lines 110-120 of `dialogue.rs` now saves `ctx.speaker_stack_depth()` before calling `lower_dlg_lines` and drains the stack back to that saved depth afterward. This is the identical save/restore pattern already used in `lower_choice` (lines 526/545), confirming pattern consistency. The fix is a pure state operation — it emits no AST nodes — which is why all 69 snapshot tests pass unchanged.

**Gap 2 (Dead code):** `LoweringContext` in `context.rs` has been cleaned to contain only `errors: Vec<LoweringError>` and `speaker_stack: Vec<SpeakerScope>`. The dead `loc_key_counter: u32` field, its `loc_key_counter: 0` initializer, and the `next_loc_key()` method are entirely absent. The struct-level doc comment no longer references `next_loc_key()`. Zero call sites for these constructs exist anywhere in the codebase.

**Gap 3 (Stale doc comment):** The doc comment on `lower_stmt` in `stmt.rs` at lines 13-14 accurately describes both `Stmt::DlgDecl` (lowers to a `let` binding wrapping the dialogue body as a lambda) and `Stmt::Transition` (lowers to `AstStmt::Return` with the target as a `Call` expression). The comment matches the actual implementation at lines 72-117 exactly.

Build quality: `cargo build -p writ-compiler` produces zero warnings. All 69 tests pass. Requirements R8 and R11 remain satisfied.

---

_Verified: 2026-02-27T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
