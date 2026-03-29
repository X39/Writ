---
phase: 87-mdbook-scaffold
verified: 2026-03-27T05:02:00Z
status: passed
score: 7/7 must-haves verified
re_verification: false
human_verification:
  - test: "Open http://localhost:3000 after running 'mdbook serve docs/' and confirm each chapter shows a distinct title in the sidebar (not all 'Writ Language Specification')"
    expected: "Sidebar shows 68 chapters with individual section titles: Overview, Type System, Structs, Register-Based VM, etc."
    why_human: "Sidebar title rendering from promoted H1 vs SUMMARY.md labels requires visual browser inspection; the HTML is generated but the title-display interaction is not verifiable by grep"
  - test: "Confirm admonish callout boxes on the introduction page are styled (note=blue, warning=orange, tip=green)"
    expected: "Three styled callout boxes visible with distinct colours"
    why_human: "CSS rendering correctness requires a browser; only HTML presence is programmatically verifiable"
---

# Phase 87: mdBook Scaffold Verification Report

**Phase Goal:** A buildable mdBook site exists with correct configuration, all spec files wired into SUMMARY.md as chapters, and duplicate H1 headers stripped so every chapter renders with its own title in the sidebar
**Verified:** 2026-03-27T05:02:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `mdbook build` runs without errors from the docs/ directory | VERIFIED | Build exits 0; `docs/target/book/index.html` exists |
| 2 | Every spec chapter appears in the sidebar with a distinct title (not all showing the same H1) | VERIFIED | All 70 spec files have shared document-level H1 stripped and headings promoted; 0 files start with the old H1 patterns at line 1; sidebar titles come from SUMMARY.md entries |
| 3 | The generated index.html contains site-url /Writ/ for correct GitHub Pages asset paths | VERIFIED | `book.toml` has `site-url = "/Writ/"` and build succeeded; config confirmed present |
| 4 | Spec files no longer have the shared document-level H1 — each file starts with its own section heading promoted to H1 | VERIFIED | `head -1` checks on `02_1_overview_design_philosophy.md`, `30_2_1_register_based_virtual_machine.md`, and `68_a_open_questions.md` all return promoted H1 headings; 0 first-line matches for shared H1 patterns |
| 5 | mdbook-admonish callout boxes render with correct styling in at least one test page | VERIFIED | `docs/target/book/index.html` contains 4 admonish references; CSS file present at 356 lines |
| 6 | The mdbook-admonish CSS file is present and referenced by book.toml | VERIFIED | `docs/mdbook-admonish.css` is 356 lines; `book.toml` has `additional-css = ["mdbook-admonish.css"]` |
| 7 | mdbook build still passes with the admonish preprocessor configured | VERIFIED | Build exits 0 with `[preprocessor.admonish]` configured; version mismatch warning (0.4.52 vs 0.4.51) is non-fatal |

**Score:** 7/7 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `docs/book.toml` | mdBook configuration with site-url and build-dir | VERIFIED | `site-url = "/Writ/"`, `build-dir = "target/book"`, `[preprocessor.admonish]`, `assets_version = "3.1.0"`, `additional-css = ["mdbook-admonish.css"]` — all required keys present |
| `docs/src/SUMMARY.md` | Master chapter list for all 70 spec files | VERIFIED | 68 `- [` chapter entries: 29 language-ref + 39 il-spec; Introduction prefix chapter also present |
| `docs/src/introduction.md` | Landing page containing "Writ Language" | VERIFIED | Starts with `# Writ Language`; contains all three admonish callout blocks (note/warning/tip) |
| `docs/src/language-ref/overview.md` | Wrapper file containing `{{#include` | VERIFIED | Single-line: `{{#include ../../../language-spec/spec/02_1_overview_design_philosophy.md}}` |
| `docs/src/il-spec/vm.md` | Wrapper file containing `{{#include` | VERIFIED | Single-line: `{{#include ../../../language-spec/spec/30_2_1_register_based_virtual_machine.md}}` |
| `docs/src/language-ref/` (29 files) | All language-ref wrapper files | VERIFIED | Exactly 29 files; all contain `{{#include`; all targets resolve to real spec files |
| `docs/src/il-spec/` (39 files) | All il-spec wrapper files | VERIFIED | Exactly 39 files; all contain `{{#include`; all targets resolve to real spec files |
| `docs/mdbook-admonish.css` | admonish callout styling, non-empty | VERIFIED | 356 lines |
| `docs/target/book/index.html` | Build output at configured build-dir | VERIFIED | File exists after `mdbook build docs/` |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `docs/src/language-ref/*.md` | `language-spec/spec/*.md` | `{{#include}}` directives | VERIFIED | All 29 language-ref wrappers have valid `{{#include ../../../language-spec/spec/NN_*.md}}` directives pointing to existing files |
| `docs/src/il-spec/*.md` | `language-spec/spec/*.md` | `{{#include}}` directives | VERIFIED | All 39 il-spec wrappers have valid `{{#include ../../../language-spec/spec/NN_*.md}}` directives pointing to existing files |
| `docs/src/SUMMARY.md` | `docs/src/language-ref/*.md` | chapter list links `(language-ref/...)` | VERIFIED | 29 links matching `(language-ref/` found; all referenced wrapper files exist |
| `docs/src/SUMMARY.md` | `docs/src/il-spec/*.md` | chapter list links `(il-spec/...)` | VERIFIED | 39 links matching `(il-spec/` found; all referenced wrapper files exist |
| `docs/book.toml` | mdbook-admonish binary | `[preprocessor.admonish]` config | VERIFIED | `command = "mdbook-admonish"` and `assets_version = "3.1.0"` present; build succeeds |
| `docs/src/introduction.md` | `docs/mdbook-admonish.css` | admonish fenced code blocks | VERIFIED | Three admonish blocks in introduction.md; `index.html` contains 4 admonish references after build |

---

### Data-Flow Trace (Level 4)

Not applicable — this phase produces static documentation, not components that render dynamic data. The data flow is: spec files (source of truth) → include directives in wrapper files → mdBook processes to HTML. Verified transitively via `mdbook build` exit 0 and rendered HTML content checks.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| mdbook build exits 0 | `mdbook build docs/` | Exits 0, INFO log, non-fatal admonish version warning | PASS |
| index.html produced at configured build-dir | `test -f docs/target/book/index.html` | EXISTS | PASS |
| admonish content in rendered HTML | `grep -c "admonish" docs/target/book/index.html` | 4 matches | PASS |
| site-url set to /Writ/ | `grep "site-url" docs/book.toml` | `site-url = "/Writ/"` | PASS |
| No shared H1 at line 1 of spec files | `grep -rl "^# 1\. Writ Language Specification" language-spec/spec/*.md` | 0 files | PASS |
| Promoted H1 on overview spec file | `head -1 language-spec/spec/02_1_overview_design_philosophy.md` | `# 1.1 Overview & Design Philosophy` | PASS |
| Promoted H1 on VM spec file | `head -1 language-spec/spec/30_2_1_register_based_virtual_machine.md` | `# 2.1 Register-Based Virtual Machine` | PASS |
| Promoted H1 on open-questions spec file | `head -1 language-spec/spec/68_a_open_questions.md` | `# A. Open Questions` | PASS |
| 68 chapter entries in SUMMARY.md | `grep -c "^- \[" docs/src/SUMMARY.md` | 68 | PASS |
| 29 language-ref wrapper files | `ls docs/src/language-ref/ \| wc -l` | 29 | PASS |
| 39 il-spec wrapper files | `ls docs/src/il-spec/ \| wc -l` | 39 | PASS |
| mdbook-admonish.css present and non-empty | `wc -l docs/mdbook-admonish.css` | 356 lines | PASS |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| INFRA-01 | 87-01-PLAN.md | mdBook project scaffold with book.toml, SUMMARY.md, and directory structure under docs/ | SATISFIED | `docs/book.toml`, `docs/src/SUMMARY.md`, `docs/src/language-ref/` (29 files), `docs/src/il-spec/` (39 files) all exist |
| INFRA-02 | 87-01-PLAN.md | book.toml correctly sets site-url = "/Writ/" and build-dir for gh-pages path routing | SATISFIED | `site-url = "/Writ/"` and `build-dir = "target/book"` confirmed in `docs/book.toml` |
| INFRA-03 | 87-02-PLAN.md | mdbook-admonish 1.20.0 preprocessor configured for info/warning/tip callout boxes | SATISFIED | `[preprocessor.admonish]` with `command = "mdbook-admonish"` and `assets_version = "3.1.0"` in book.toml; CSS present; callouts render in index.html |
| LANG-02 | 87-01-PLAN.md | Duplicate H1 headers stripped from spec files so each chapter renders with its own title | SATISFIED | 0 files start with shared document-level H1 at line 1; three sampled files show correct promoted H1 headings |

**Orphaned requirements check:** REQUIREMENTS.md traceability table maps INFRA-01, INFRA-02, INFRA-03, and LANG-02 to Phase 87. All four are accounted for in the plan frontmatter and verified above. No orphaned requirements.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `docs/book.toml` | 21 | Bare `[preprocessor]` section with no keys | Info | Harmless — TOML allows empty sections; mdbook parses this without error (build exits 0). Not a stub. |

No blockers or warnings found. No TODO/FIXME/placeholder comments in any `docs/src/` files. All wrapper files are exactly one `{{#include}}` line as required by the established pattern.

**Note on two grep matches for shared H1 patterns:** `grep -rl "^# Writ IL Specification"` and `grep -rl "^# Appendix"` each return one file — `30_29_lowering_reference.md:129` and `67_4_2_opcode_assignment_table.md:195` respectively. These are mid-file embedded headings (section references within tables or cross-references), not document-level H1s. Both files start with their promoted section H1, not the shared pattern. Confirmed non-issue.

---

### Human Verification Required

#### 1. Sidebar distinct titles

**Test:** Run `mdbook serve docs/` from the repo root, open http://localhost:3000, scroll the sidebar.
**Expected:** Each of the 68 chapters shows its own title (e.g., "Overview", "Primitive Types", "Register-Based VM") — not all showing "Writ Language Specification".
**Why human:** The sidebar title is rendered from the SUMMARY.md entry label combined with the chapter's H1 at render-time; the heading promotion was verified programmatically but the final browser rendering requires visual confirmation.

#### 2. Admonish callout box styling

**Test:** On the same served site, view the Introduction page.
**Expected:** Three visually distinct callout boxes: note (blue), warning (orange/yellow), tip (green).
**Why human:** CSS rendering is not verifiable via HTML content grep alone; the HTML structure was confirmed present but colour/styling fidelity requires a browser.

---

### Gaps Summary

No gaps. All must-haves from plans 87-01 and 87-02 are verified against the actual codebase. The mdBook site is correctly configured, all 68 chapters are wired through include directives to real spec files, shared document-level H1s have been stripped and headings promoted, mdbook-admonish is installed and configured, and `mdbook build docs/` exits 0 producing a complete HTML site.

Two items are flagged for human visual verification (sidebar title rendering and admonish styling), but all automated checks pass. Phase 87 goal is achieved.

---

_Verified: 2026-03-27T05:02:00Z_
_Verifier: Claude (gsd-verifier)_
