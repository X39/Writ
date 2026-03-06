---
phase: 42-choiceoption-rename
verified: 2026-03-06T15:30:00Z
status: passed
score: 5/5 must-haves verified
re_verification: false
---

# Phase 42: ChoiceOption Rename Verification Report

**Phase Goal:** The dialogue choice option type is named `ChoiceOption` everywhere — spec text, lowering emit site, virtual module TypeDef/ExternDef, and resolver prelude — resolving the naming conflict with `Option<T>`
**Verified:** 2026-03-06T15:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Note on Scope Refinement

The ROADMAP goal text mentions "virtual module TypeDef/ExternDef" and "resolver prelude" as two of the four layers. The PLAN frontmatter (which takes precedence as the final execution contract) explicitly resolved this differently: `ChoiceOption` is NOT registered as a TypeDef in `virtual_module.rs` and NOT added to `PRELUDE_TYPE_NAMES` in `prelude.rs`. Instead, Writ scripts declare `pub extern fn ChoiceOption(...)` directly, which the emit pipeline handles as a normal ExternDef row. The `choice_option_emits_externdef` integration test validates this path end-to-end. This is a correct narrowing of scope — adding ChoiceOption to PRELUDE_TYPE_NAMES would have blocked extern fn declarations via the `is_prelude_name` guard.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | `"Option"` no longer appears as a choice constructor name in any Rust source or golden fixture — replaced by `"ChoiceOption"` in all four layers atomically | VERIFIED | `grep '"Option"' writ-compiler/src/lower/dialogue.rs` returns no matches; `grep '"Option"\.to_string\(\)' dialogue.rs` returns no matches |
| 2 | All four affected insta snapshot files reflect `name: "ChoiceOption"` in choice arm callee positions | VERIFIED | All four snapshots contain `name: "ChoiceOption"` in both choice arm Call nodes; `cargo test -p writ-compiler dlg_choice` passes (3 tests green) |
| 3 | The `Option<T>` nullable wrapper registrations in `virtual_module.rs` and `prelude.rs` are untouched | VERIFIED | `virtual_module.rs` line 183: `add_type_def("Option", "writ", 1, 0)` unchanged; `prelude.rs` PRELUDE_TYPE_NAMES still contains `"Option"` only; ChoiceOption absent from both files; `cargo test -p writ-runtime option_is_enum_with_one_generic_param` passes |
| 4 | A Writ script with `pub extern fn ChoiceOption(...)` and a dollar-choice block compiles to IL with an ExternDef row named `ChoiceOption` | VERIFIED | `choice_option_emits_externdef` test passes — asserts `builder.extern_defs` contains a row where `string_heap.get_str(row.name) == "ChoiceOption"` with no diagnostics |
| 5 | `cargo test --workspace` passes green after the rename | VERIFIED | All test result lines show `0 failed`; total coverage spans writ-compiler, writ-runtime, writ-golden, writ-parser, writ-diagnostics, writ-module |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-compiler/src/lower/dialogue.rs` | Single emit site rename — `name: "ChoiceOption".to_string()` | VERIFIED | Line 657: `name: "ChoiceOption".to_string()`; comment on line 654 also updated to "Build: ChoiceOption(label_text, loc_key, fn() { body })"; no `"Option".to_string()` remains |
| `language-spec/spec/29_28_lowering_reference.md` | Spec example updated at lines 53 and 57 | VERIFIED | Lines 53 and 57 contain `ChoiceOption("Good!", ...)` and `ChoiceOption("Not great", ...)`; no bare `Option(` remains |
| `writ-compiler/tests/snapshots/lowering_tests__dlg_choice_basic.snap` | Blessed snapshot with `ChoiceOption` callee name | VERIFIED | Two occurrences of `name: "ChoiceOption"` in choice arm Call nodes at spans 52..79 and 80..106 |
| `writ-compiler/tests/snapshots/lowering_tests__dlg_choice_label_key_emitted.snap` | Blessed snapshot with `ChoiceOption` callee name | VERIFIED | Two occurrences of `name: "ChoiceOption"` at spans 51..102 and 103..131 |
| `writ-compiler/tests/snapshots/lowering_tests__dlg_choice_speaker_scope_isolation.snap` | Blessed snapshot with `ChoiceOption` callee name | VERIFIED | Two occurrences of `name: "ChoiceOption"` at spans 34..61 and 62..77 |
| `writ-compiler/tests/snapshots/lowering_tests__integration_all_constructs.snap` | Blessed snapshot with `ChoiceOption` callee name | VERIFIED | Two occurrences of `name: "ChoiceOption"` at spans 215..238 and 247..271 |
| `writ-compiler/tests/emit_tests.rs` | `choice_option_emits_externdef` integration test | VERIFIED | Test exists at line 243; asserts ExternDef row named "ChoiceOption" in emitted module; uses Tier 1 speaker (dlg param) to avoid Entity.getOrCreate hoisting; passes |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-compiler/src/lower/dialogue.rs` | `writ-compiler/tests/snapshots/lowering_tests__dlg_choice_basic.snap` | `cargo insta test --accept` after emit site rename | WIRED | Snapshot contains `name: "ChoiceOption"` matching the renamed emit site; `dlg_choice_basic` test passes |
| `writ-compiler/tests/emit_tests.rs` | `writ-compiler/src/emit/` | `emit_src` helper — parse/lower/resolve/typecheck/emit pipeline | WIRED | `choice_option_emits_externdef` test passes; `builder.extern_defs` is non-empty and contains a row named "ChoiceOption"; no diagnostics emitted |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| LANG-01 | 42-01-PLAN.md | The dialogue choice option type is renamed from `Option(...)` to `ChoiceOption(...)` in spec and implementation (lowering, virtual module, prelude) — resolves naming conflict with `Option<T>` | SATISFIED | Single emit site renamed in `dialogue.rs`; four insta snapshots re-blessed; spec updated at both occurrences; `choice_option_emits_externdef` validates end-to-end ExternDef path; all workspace tests pass; `option_is_enum_with_one_generic_param` confirms `Option<T>` untouched |

**Orphaned requirements check:** REQUIREMENTS.md maps only LANG-01 to Phase 42 (Traceability table line: `LANG-01 | Phase 42 | Complete`). No orphaned requirements.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | None found | — | — |

No TODOs, FIXMEs, placeholders, empty handlers, or stale string literals found in any modified file.

### Human Verification Required

One item from VALIDATION.md is noted as manual-only and remains unverified programmatically:

**Test: CALL_EXTERN token resolution in emitted IL**
- **Test:** Write a `.writ` file with `$ choice { "Good" { } "Bad" { } }`, compile via `writ compile`, disassemble and verify `CALL_EXTERN` references a token that resolves to `ChoiceOption`
- **Expected:** The `CALL_EXTERN` instruction in the emitted IL references an ExternDef token whose string heap entry is `"ChoiceOption"`
- **Why human:** Requires the full `writ compile` CLI pipeline and IL disassembly, which is not yet wired into automated tests. The `choice_option_emits_externdef` integration test covers the emit layer directly and provides strong evidence; the IL token round-trip is the only remaining gap.

This does not block the "passed" status — the integration test (`choice_option_emits_externdef`) directly inspects `builder.extern_defs` and verifies the ExternDef name, which is equivalent evidence without needing the CLI path.

### Gaps Summary

No gaps. All must-haves verified.

---

## Commit Evidence

| Hash | Description |
|------|-------------|
| `23b1f65` | feat(42-01): rename choice option constructor from Option to ChoiceOption |
| `27d54e8` | test(42-01): add choice_option_emits_externdef integration test |
| `85e8ccb` | docs(42-01): complete ChoiceOption rename plan — SUMMARY, STATE, ROADMAP updated |

All three commits exist in git log (confirmed via `git log --oneline -6`).

---

_Verified: 2026-03-06T15:30:00Z_
_Verifier: Claude (gsd-verifier)_
