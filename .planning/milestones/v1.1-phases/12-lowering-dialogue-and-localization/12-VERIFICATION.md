---
phase: 12-lowering-dialogue-and-localization
status: passed
verified: 2026-03-01
verifier: orchestrator
score: 5/5
---

# Phase 12: Lowering — Dialogue and Localization — Verification

## Phase Goal

Dialogue lowering produces correct localization keys (with namespace prefix, preserved slot identifiers, emitted choice keys), uses the right call (`say` vs `say_localized`), and correctly isolates speaker scope across branching dialogue constructs.

## Success Criteria Verification

### 1. FNV-1a localization keys include the file namespace as a prefix (e.g., `my_module.fnv_key`) rather than `fnv_key` alone

**Status: PASS**

- `namespace_stack: Vec<String>` field added to `LoweringContext` in `context.rs`
- API added: `push_namespace`, `pop_namespace`, `set_namespace`, `current_namespace`
- `lower_namespace()` in `lower/mod.rs` threads namespace into `LoweringContext` via push/pop/set
- Dialogue lowering reads current namespace at init time and prefixes FNV-1a keys as `namespace.fnv_key`
- Namespace stored as joined `::` string in `DlgLowerState`, not as segments, since FNV input needs flat string
- Plan 12-01 evidence: LoweringContext namespace API in context.rs; namespace prefix in dialogue.rs
- Plan 12-02 tests: `dlg_namespace_in_loc_key` (single-segment), `dlg_namespace_multi_segment_in_loc_key` (multi-segment) — 2 snapshot tests pass

### 2. Interpolation slot names in localization content strings are preserved as-written (`{name}`) and are not replaced with a generic `{expr}` placeholder

**Status: PASS**

- `expr_to_slot_text()` helper added to `lower/dialogue.rs`, recursively reconstructing slot text from CST `Expr` nodes
- Handles `Expr::Ident` (produces `{name}`), `Expr::MemberAccess` (produces `{player.name}`), `Expr::Call` (produces `{fn(..)}`)
- CST tuple-style destructuring used: `Expr::MemberAccess(object, (field, _field_span))`, `Expr::Call(callee, _args)`
- Plan 12-01 evidence: `expr_to_slot_text()` in dialogue.rs; tuple-style CST destructuring
- Plan 12-02 tests: `dlg_interpolation_slot_preserved` (simple ident), `dlg_interpolation_member_access_preserved` (member access) — 2 snapshot tests pass

### 3. Choice label localization keys appear as named bindings in the lowered output rather than being discarded via `let _ = key`

**Status: PASS**

- Choice label loc key emitted as second arg to `Option(label, key, fn() { body })` instead of discarded with `let _ = key`
- Plan 12-01 evidence: second arg to Option() in choice label lowering in dialogue.rs
- Plan 12-02 test: `dlg_choice_label_key_emitted` — 1 snapshot test passes confirming key as second arg

### 4. `say()` is emitted for dialogue lines with no localization key; `say_localized()` is emitted only when a `#key` override or auto-generated key is present

**Status: PASS**

- `make_say()` and `make_say_localized()` helper functions added to `lower/dialogue.rs`
- `say()` emitted for unkeyed lines; `say_localized()` emitted only for lines with manual `#key`
- Auto FNV keys still computed for unkeyed lines (occurrence tracking + CSV tooling) but only `make_say()` emitted at runtime
- Plan 12-01 evidence: `make_say`/`make_say_localized` helpers in dialogue.rs; conditional dispatch
- Plan 12-02 tests: `dlg_say_without_key`, `dlg_say_localized_with_key`, `dlg_say_mixed_key_dispatch` — 3 snapshot tests pass

### 5. After a `$ if` or `$ match` dialogue branch, the speaker reverts to its pre-branch value — a speaker set inside one branch does not leak into sibling branches or subsequent lines

**Status: PASS**

- Speaker scope save/restore added to `lower_dlg_if`, `lower_dlg_else`, and `lower_dlg_match` in `lower/dialogue.rs`
- Uses same `speaker_stack_depth()` + pop-loop pattern as existing choice arm handling
- Tests use `@player $ if` pattern (SpeakerTag, pushes to stack) not `@player Hello.` (SpeakerLine, no push) to correctly exercise scope isolation
- Plan 12-01 evidence: save/restore in if/else/match branches in dialogue.rs
- Plan 12-02 tests: `dlg_speaker_scope_isolation_if`, `dlg_speaker_scope_isolation_if_else`, `dlg_speaker_scope_isolation_match` — 3 snapshot tests pass

## Requirement Coverage

All 5 phase requirements accounted for:

| Requirement | Plan    | Status   | Evidence |
|-------------|---------|----------|----------|
| DLG-01      | 12-01   | Verified | Namespace stack API in LoweringContext; namespace prefix in FNV keys; 2 snapshot tests pass |
| DLG-02      | 12-01   | Verified | `expr_to_slot_text()` preserves slot names; 2 snapshot tests pass |
| DLG-03      | 12-01   | Verified | Choice label key as second arg to Option(); 1 snapshot test passes |
| DLG-04      | 12-01   | Verified | `make_say`/`make_say_localized` helpers; conditional dispatch; 3 snapshot tests pass |
| DLG-05      | 12-01   | Verified | Speaker scope save/restore in if/else/match; 3 snapshot tests pass |

## Test Results

```
cargo test --workspace: 250 passed, 0 failed
  97 lowering tests (up from 86 pre-Phase 12)
  13 unit tests (string_utils)
  74 lexer tests
  239 parser tests (note: number corrected from 239 total at Phase 11 end)
  2 doc tests
```

New tests added: 11 lowering snapshot tests (Plan 12-02)
- DLG-01: 2 namespace tests
- DLG-02: 2 interpolation slot tests
- DLG-03: 1 choice label key test
- DLG-04: 3 say dispatch tests
- DLG-05: 3 speaker scope isolation tests

17 existing dialogue snapshots updated to reflect new lowering behavior.

## Gaps Found

None.

---
*Phase: 12-lowering-dialogue-and-localization*
*Verified: 2026-03-01*
