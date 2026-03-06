---
phase: 40-spec-cleanup
verified: 2026-03-06T00:00:00Z
status: passed
score: 9/9 must-haves verified
re_verification: false
---

# Phase 40: Spec Cleanup Verification Report

**Phase Goal:** The language spec is internally consistent — §1.2.8 is removed, inbuilt method call paths are clarified, and writ.toml field names in spec, config.rs, and writ new scaffold are aligned
**Verified:** 2026-03-06
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| #   | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1   | `37_2_8_serialization_critical_sections_removed.md` does not exist on disk | VERIFIED | `ls` returns no file; git commit 736a1e3 deleted it |
| 2   | TOC entry for §2.8 Serialization Critical Sections REMOVED is absent from `01_table_of_contents.md` | VERIFIED | TOC jumps directly from `2.7 Operator Dispatch` to `2.9 Memory Model` — no 2.8 entry present |
| 3   | No spec file contains a dangling reference to `2_8` or `2.8 Serialization` | VERIFIED | grep across all spec files returns zero matches |
| 4   | `27_26_standard_library_builtins.md` contains `### 26.4 Root-Namespace Inbuilt Calls` with a table of log/say/choice | VERIFIED | Section exists at line 36 with complete table and prose |
| 5   | `14_13_dialogue_blocks_dlg.md` §13.9 no longer contains "Runtime namespace" or "not callable directly" — references "root-namespace inbuilt calls (§26.4)" instead | VERIFIED | Line 254: "root-namespace inbuilt calls (§26.4) provided by the runtime" — no Runtime namespace wording |
| 6   | `29_28_lowering_reference.md` §28.5 preamble no longer contains "Runtime namespace" — references "root-namespace inbuilts (§26.4)" | VERIFIED | Line 114: "root-namespace inbuilts (§26.4)" — no Runtime namespace wording |
| 7   | `LocaleConfig` in `config.rs` uses `#[serde(rename = "default")]` and `#[serde(rename = "supported")]` with `#[serde(default)]` | VERIFIED | Lines 38–43 in `writ-compiler/src/config.rs` show exact attributes |
| 8   | All 6 config tests pass: `parse_basic_config`, `locale_without_supported`, `scaffold_toml_round_trips`, `default_sources_when_omitted`, `discover_writ_files`, `missing_toml_error` | VERIFIED | `cargo test -p writ-compiler config` reports `6 passed; 0 failed` |
| 9   | `writ-cli/src/main.rs` scaffold template contains uncommented `[compiler]` section with `sources = ["sources/"]` | VERIFIED | Lines 179–181 show active `[compiler]` header and `sources = ["sources/"]` |

**Score:** 9/9 truths verified

---

## Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `language-spec/spec/37_2_8_serialization_critical_sections_removed.md` | Does NOT exist | VERIFIED (absent) | File deleted; no remnant on disk |
| `language-spec/spec/01_table_of_contents.md` | Updated TOC without 2.8 entry | VERIFIED | 2.7 directly followed by 2.9; §26.3 is the last section 26 entry (see note below) |
| `language-spec/spec/27_26_standard_library_builtins.md` | Contains "### 26.4 Root-Namespace Inbuilt Calls" | VERIFIED | Section exists at line 36 with table and prose |
| `language-spec/spec/14_13_dialogue_blocks_dlg.md` | Contains "root-namespace inbuilt calls" in §13.9 | VERIFIED | Line 254 uses exact phrase; old wording absent |
| `language-spec/spec/29_28_lowering_reference.md` | Contains "root-namespace inbuilts" in §28.5 preamble | VERIFIED | Line 114 uses exact phrase; old wording absent |
| `writ-compiler/src/config.rs` | `LocaleConfig` with `serde(rename = "default")` | VERIFIED | Lines 38–43 contain both rename attributes plus `#[serde(default)]` |
| `writ-cli/src/main.rs` | Scaffold with `sources = ["sources/"]` uncommented | VERIFIED | Lines 179–181 show active `[compiler]` section |

---

## Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `01_table_of_contents.md` | `37_2_8_serialization_critical_sections_removed.md` | anchor link | VERIFIED (link removed) | No "2_8" pattern found anywhere in spec files — dangling anchor properly eliminated |
| `14_13_dialogue_blocks_dlg.md` §13.9 | `27_26_standard_library_builtins.md` §26.4 | cross-reference `(§26.4)` | VERIFIED | Lines 254 and 262 in §13.9 both reference §26.4 |
| `29_28_lowering_reference.md` §28.5 | `27_26_standard_library_builtins.md` §26.4 | cross-reference `(§26.4)` | VERIFIED | Line 114 in §28.5 references §26.4 |
| `writ-compiler/src/config.rs` `LocaleConfig` | `writ.toml` `[locale]` fields | serde deserialization | VERIFIED | `#[serde(rename = "default")]` and `#[serde(rename = "supported")]` present; `scaffold_toml_round_trips` test confirms end-to-end |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| SPEC-01 | 40-01-PLAN.md | §1.2.8 Serialization Critical Sections removed from spec | SATISFIED | File absent from disk; TOC entry removed; zero dangling references; commit 736a1e3 |
| SPEC-02 | 40-02-PLAN.md | log/say/choice clarified as root-namespace inbuilts with no qualifier needed | SATISFIED | §26.4 added; §13.9 and §28.5 rewritten; no "Runtime namespace" wording remains in scoped files; commit cd67679 |
| SPEC-03 | 40-03-PLAN.md | writ.toml field names aligned across spec, config.rs, and `writ new` scaffold | SATISFIED | serde rename attributes present; 6 config tests green; scaffold emits `sources = ["sources/"]`; commits d6d5b06, a21d0b9, b9f06ce |

No orphaned requirements for Phase 40 found in REQUIREMENTS.md. Traceability table confirms SPEC-01, SPEC-02, SPEC-03 are all mapped to Phase 40 and marked Complete.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `writ-cli/src/main.rs` | 268 | `// TODO: Add your code here` | INFO | This is intentional scaffold boilerplate written into the generated `sources/main.writ` template for end users — not a development placeholder. No action required. |

No blocker or warning anti-patterns found.

---

## Observations (Out-of-Scope Gaps)

These items were noticed but are outside the explicit scope of Phase 40's must_haves:

**§26.4 missing from `01_table_of_contents.md`:** The new section `26.4 Root-Namespace Inbuilt Calls` does not have a corresponding entry in the TOC. Plan 40-02 did not include `01_table_of_contents.md` in `files_modified` and made no TOC entry part of its must_haves. The section exists and is fully cross-referenced from §13.9 and §28.5. A future cleanup task could add the TOC entry.

**`Runtime.say` / `Runtime.choice` in IL spec §2.17.3:** `language-spec/spec/46_2_17_execution_model.md` line 86 still uses `Runtime.say` and `Runtime.choice` notation in a descriptive context (explaining that these are extern calls, not special IL instructions). This is in the IL spec's execution model section — not one of the three language-spec files targeted by SPEC-02. The phrase "Runtime namespace" does not appear there, so it passed the SPEC-02 consistency check. This may warrant a follow-up correction in a later phase.

---

## Human Verification Required

None — all checks are verifiable programmatically. The spec is text and code; no visual UI or runtime behavior is involved.

---

## Gaps Summary

No gaps. All 9 must-have truths verified, all 3 requirements satisfied, all 5 git commits confirmed present, all 6 config tests green.

---

_Verified: 2026-03-06_
_Verifier: Claude (gsd-verifier)_
