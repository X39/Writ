# Phase 87: mdBook Scaffold - Research

**Researched:** 2026-03-27
**Domain:** mdBook static site scaffold — configuration, SUMMARY.md generation, H1 header stripping, mdbook-admonish setup
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all implementation choices are at Claude's discretion (auto-generated infrastructure phase).

### Claude's Discretion
All implementation choices: directory structure, SUMMARY.md organization, H1-stripping strategy, wrapper file approach, admonish test page content.

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| INFRA-01 | mdBook project scaffold with book.toml, SUMMARY.md, and directory structure under docs/ | Directory layout and book.toml schema documented below; exact file paths confirmed from existing project architecture research |
| INFRA-02 | book.toml correctly sets site-url = "/Writ/" and build-dir for gh-pages path routing | site-url and build-dir configuration verified against official mdBook renderer docs |
| INFRA-03 | mdbook-admonish 1.20.0 preprocessor configured for info/warning/tip callout boxes | Install + book.toml config verified from official mdbook-admonish docs; syntax examples confirmed |
| LANG-02 | Duplicate H1 headers stripped from spec files so each chapter renders with its own title | 69 spec files audited — 31 have "# 1. Writ Language Specification", 38 have "# Writ IL Specification", 2 have "# Appendix"; H1-strip + H2-promote strategy defined |
</phase_requirements>

---

## Summary

Phase 87 creates the foundational `docs/` directory that every subsequent v9.0 phase depends on. The work has three parts: (1) scaffold the `docs/` directory with `book.toml` and directory structure, (2) wire all 70 spec files into `SUMMARY.md` as chapters using `{{#include}}` wrapper files, and (3) strip the shared document-level H1 from all spec files so each chapter shows its own title in the sidebar.

The existing project-level research (`.planning/research/`) has already investigated the full v9.0 stack. This phase-level research synthesizes what specifically Phase 87 needs. All technical decisions are already locked in `STATE.md`: mdBook 0.4.51, `site-url = "/Writ/"`, `build-dir = "book"` inside `docs/`, `{{#include}}` wrappers with `language-spec/spec/` as the single source of truth.

The most mechanically intensive work is LANG-02: 70 spec files each have a shared document-level H1 (`# 1. Writ Language Specification` or `# Writ IL Specification` or `# Appendix`) followed by their real section H2. Stripping the duplicate H1 and promoting the H2 to H1 is a deterministic one-time transformation applied to each file. The wrapper file then provides its own H1 title, and the `{{#include}}` of the spec file (starting from what is now its H1) provides the chapter body.

**Primary recommendation:** Create `docs/` scaffold first, verify `mdbook build` succeeds with an empty SUMMARY.md, then add the H1-stripped spec wrappers one section at a time, confirming the sidebar title is correct for each. Add mdbook-admonish last and verify with one test page.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| mdBook | 0.4.51 | Static site generator | 0.4.x is the last series compatible with mdbook-admonish; 0.5.x broke the preprocessor API (issue #233) |
| mdbook-admonish | 1.20.0 | Styled callout blocks (note/warning/tip/danger) | Latest 1.x release; requires `mdbook ^0.4.40`, compatible with 0.4.51 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cargo (Rust toolchain) | stable (1.87+) | Install mdBook and mdbook-admonish binaries | Already present; needed to `cargo install` the tools |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| mdBook 0.4.51 | mdBook 0.5.x | 0.5.x adds native admonition syntax (no plugin), but mdbook-admonish is incompatible with 0.5.x — stay on 0.4.51 |
| `{{#include}}` wrappers | Copy spec files into docs/src/ | Copies break sync with spec source of truth; includes maintain sync automatically |
| H1-strip in spec files | `{{#include file:2:}}` line-range skip | Line-range skips are fragile if files gain a blank line at top; direct H1 strip is cleaner and fixes the files for standalone reading too |

**Installation:**
```bash
# Local development (on this machine — mdbook not yet installed)
cargo install mdbook --version 0.4.51
cargo install mdbook-admonish --version 1.20.0

# Initialize admonish assets (run once from docs/ directory)
mdbook-admonish install docs/
```

**Version verification (confirmed 2026-03-27):**
- mdbook 0.4.51 — last stable 0.4.x release
- mdbook-admonish 1.20.0 — latest release (June 2025)

---

## Architecture Patterns

### Recommended Project Structure

```
docs/
├── book.toml                         # mdBook configuration
├── mdbook-admonish.css               # injected by `mdbook-admonish install`
└── src/
    ├── SUMMARY.md                    # master chapter list (all 70 spec files + intro)
    ├── introduction.md               # landing page
    ├── language-ref/
    │   ├── README.md                 # section intro
    │   ├── overview.md               # {{#include ../../../language-spec/spec/02_...md}}
    │   ├── project-config.md
    │   ├── naming-conventions.md
    │   ├── lexical.md
    │   ├── type-system.md
    │   ├── primitives.md
    │   ├── variables.md
    │   ├── structs.md
    │   ├── classes.md
    │   ├── enums.md
    │   ├── contracts.md
    │   ├── generics.md
    │   ├── functions.md
    │   ├── dialogue.md
    │   ├── entities.md
    │   ├── components.md
    │   ├── attributes.md
    │   ├── operators.md
    │   ├── error-handling.md
    │   ├── nullability.md
    │   ├── concurrency.md
    │   ├── scoping.md
    │   ├── globals.md
    │   ├── modules.md
    │   ├── extern.md
    │   ├── localization.md
    │   ├── stdlib.md
    │   ├── grammar.md
    │   └── lowering.md
    └── il-spec/
        ├── README.md                 # section intro
        ├── vm.md                     # {{#include ../../../language-spec/spec/30_...md}}
        ├── typed-il.md
        ├── execution-model.md
        ├── binary-format.md
        ├── instruction-encoding.md
        ├── calling-convention.md
        ├── operator-dispatch.md
        ├── memory-model.md
        ├── self-parameter.md
        ├── construction.md
        ├── delegates.md
        ├── serialization.md
        ├── runtime-host.md
        ├── type-system.md
        ├── module-format.md
        ├── execution.md
        ├── writ-runtime-module.md
        ├── opcodes-meta.md           # {{#include}} files 48_–67_
        ├── ... (one file per opcode category)
        └── appendix.md               # {{#include}} 68_ and 69_
```

**Note on build-dir:** The ROADMAP success criterion says output goes to `docs/target/book/`. This means `build-dir = "target/book"` (not `"book"`). Pitfall 9 in the project research warns that the default `"book"` output is untracked by `.gitignore` (which covers `target/` but not `book/`). Using `target/book` keeps output inside the already-gitignored `target/` directory.

### Pattern 1: `{{#include}}` Wrapper Files

**What:** Each chapter in `docs/src/language-ref/` or `docs/src/il-spec/` is a thin file with an H1 title and a single `{{#include}}` directive pointing to the spec file.

**When to use:** All spec-backed chapters. Never copy content — include it.

**Example:**
```markdown
<!-- docs/src/language-ref/dialogue.md -->
# Dialogue Blocks

{{#include ../../../language-spec/spec/15_14_dialogue_blocks_dlg.md}}
```

After H1 stripping, the spec file (`15_14_dialogue_blocks_dlg.md`) starts with `## 1.14 Dialogue Blocks` (promoted to `# 1.14 Dialogue Blocks`). The wrapper file provides the clean chapter title `# Dialogue Blocks` that appears at the top. Wait — this would result in TWO H1s in the rendered output. The correct pattern is:

**Correct approach:** The wrapper file provides NO H1 of its own. The spec file's promoted H1 IS the chapter heading. The wrapper is truly minimal:

```markdown
{{#include ../../../language-spec/spec/15_14_dialogue_blocks_dlg.md}}
```

The SUMMARY.md entry provides the sidebar title. The spec file's promoted H1 renders as the chapter heading. This avoids double H1 while keeping wrapper files clean.

**SUMMARY.md entry:**
```markdown
- [Dialogue Blocks](language-ref/dialogue.md)
```

### Pattern 2: H1 Strip and H2 Promote in Spec Files

**What:** Each spec file currently opens with a document-level H1 (`# 1. Writ Language Specification` for lang spec, `# Writ IL Specification` for IL spec, or `# Appendix`). The real section content starts at the H2. Strip the H1 line and promote all headings one level (H2 → H1, H3 → H2, etc.).

**Spec files audited:**
- 31 lang spec files: first line is `# 1. Writ Language Specification`, second line is `## 1.N Section Name`
- 38 IL spec files: first line is `# Writ IL Specification`, second line is `## 2.N Section Name` or `## 3.N Opcode Name`
- 2 appendix files (`68_a_open_questions.md`, `69_b_il_decision_log.md`): first line is `# Appendix`, second line is a real section H2

**Special cases:**
- `00_preamble.md`: first line is `# 1. Writ Language Specification` but the content that follows is preamble text (no H2 section). Strip H1, keep the preamble body as intro content. The wrapper's SUMMARY.md entry provides the sidebar title.
- `01_table_of_contents.md`: contains `<!-- TOC -->` marker and a Markdown TOC list with dead cross-chapter anchors. Replace with a clean language reference landing page (as documented in PITFALLS.md Pitfall 7). Do NOT include this file via `{{#include}}` — it contains the HTML comment TOC that renders as a visible broken list.

**Heading promotion rule:**
```
Before: # Writ IL Specification\n## 2.1 Register-Based Virtual Machine\n### ...
After:  # 2.1 Register-Based Virtual Machine\n## ...
```

Strip line 1 (the shared H1). If line 2 is blank, also strip that blank line. The former H2 becomes H1, H3 becomes H2, etc.

**Implementation approach:**
Use a script (Python or shell) to process all 70 spec files. Script steps:
1. Read the file
2. Confirm first line matches the expected shared H1 pattern
3. Remove first line (and any following blank line before the H2)
4. Demote all heading levels by one (replace leading `##` with `#`, `###` with `##`, etc.)
5. Write back

This is deterministic and reversible (tracked in git). Run once, commit the result.

### Pattern 3: SUMMARY.md Structure

**What:** mdBook requires SUMMARY.md to list every page. Uses `-` list items with `[Title](path.md)` links. Part titles use `# Heading` (H1 only).

**SUMMARY.md skeleton:**
```markdown
# Summary

[Introduction](introduction.md)

# Language Reference

- [Overview](language-ref/overview.md)
- [Project Configuration](language-ref/project-config.md)
- [Naming Conventions](language-ref/naming-conventions.md)
- [Lexical Structure](language-ref/lexical.md)
- [Type System](language-ref/type-system.md)
- [Primitive Types](language-ref/primitives.md)
- [Variables & Constants](language-ref/variables.md)
- [Structs](language-ref/structs.md)
- [Classes](language-ref/classes.md)
- [Enums](language-ref/enums.md)
- [Contracts](language-ref/contracts.md)
- [Generics](language-ref/generics.md)
- [Functions (fn)](language-ref/functions.md)
- [Dialogue Blocks (dlg)](language-ref/dialogue.md)
- [Entities](language-ref/entities.md)
- [Components](language-ref/components.md)
- [Attributes](language-ref/attributes.md)
- [Operators & Overloading](language-ref/operators.md)
- [Error Handling](language-ref/error-handling.md)
- [Nullability & Optionals](language-ref/nullability.md)
- [Concurrency](language-ref/concurrency.md)
- [Scoping Rules](language-ref/scoping.md)
- [Globals & Atomic Access](language-ref/globals.md)
- [Modules & Namespaces](language-ref/modules.md)
- [External Declarations](language-ref/extern.md)
- [Localization](language-ref/localization.md)
- [Standard Library](language-ref/stdlib.md)
- [Grammar Summary (EBNF)](language-ref/grammar.md)
- [Lowering Reference](language-ref/lowering.md)

# IL Specification

- [Register-Based VM](il-spec/vm.md)
- [Typed IL](il-spec/typed-il.md)
- [Execution Model](il-spec/execution-model.md)
- [Binary Format](il-spec/binary-format.md)
- [Instruction Encoding](il-spec/instruction-encoding.md)
- [Calling Convention](il-spec/calling-convention.md)
- [Operator Dispatch](il-spec/operator-dispatch.md)
- [Memory Model](il-spec/memory-model.md)
- [Self Parameter](il-spec/self-parameter.md)
- [Construction Model](il-spec/construction.md)
- [Delegate Model](il-spec/delegates.md)
- [Save/Load Serialization](il-spec/serialization.md)
- [Runtime-Host Interface](il-spec/runtime-host.md)
- [IL Type System](il-spec/type-system.md)
- [IL Module Format](il-spec/module-format.md)
- [Execution Model (Detail)](il-spec/execution.md)
- [writ-runtime Module](il-spec/writ-runtime-module.md)
- [Opcodes: Meta](il-spec/opcodes-meta.md)
- [Opcodes: Data Movement](il-spec/opcodes-data.md)
- [Opcodes: Integer Arithmetic](il-spec/opcodes-int.md)
- [Opcodes: Float Arithmetic](il-spec/opcodes-float.md)
- [Opcodes: Bitwise/Logical](il-spec/opcodes-bitwise.md)
- [Opcodes: Comparison](il-spec/opcodes-comparison.md)
- [Opcodes: Control Flow](il-spec/opcodes-control.md)
- [Opcodes: Calls](il-spec/opcodes-calls.md)
- [Opcodes: Object Model](il-spec/opcodes-object.md)
- [Opcodes: Arrays](il-spec/opcodes-arrays.md)
- [Opcodes: Type Operations](il-spec/opcodes-types.md)
- [Opcodes: Concurrency](il-spec/opcodes-concurrency.md)
- [Opcodes: Globals/Atomics](il-spec/opcodes-globals.md)
- [Opcodes: Conversion](il-spec/opcodes-conversion.md)
- [Opcodes: Strings](il-spec/opcodes-strings.md)
- [Opcodes: Boxing](il-spec/opcodes-boxing.md)
- [Opcodes: Serialization (Removed)](il-spec/opcodes-serialization.md)
- [Instruction Count by Category](il-spec/instruction-count.md)
- [Instruction Shape Reference](il-spec/instruction-shape.md)
- [Opcode Assignment Table](il-spec/opcode-table.md)
- [Open Questions](il-spec/open-questions.md)
- [IL Decision Log](il-spec/decision-log.md)
```

**Key rules (from official docs, HIGH confidence):**
- Use `-` consistently (never mix `-` and `*`)
- Part titles must use `#` (H1 only — H2+ are silently ignored)
- Prefix chapters (unnumbered) must come before any `-` list items
- `[Introduction](introduction.md)` with no leading `-` is a prefix chapter (appears unnumbered in sidebar)

### Anti-Patterns to Avoid

- **Placing book.toml at repo root:** mdBook copies everything in `src` into the build output. At repo root, `src = "."` includes all Rust files. Use `docs/` subdirectory with `src = "src"`.
- **Using build-dir = "book":** The default `book/` directory is not covered by `.gitignore` (which covers `target/`). Use `build-dir = "target/book"` to keep generated files inside the already-ignored `target/` directory.
- **Modifying 01_table_of_contents.md with {{#include}}:** This file contains a `<!-- TOC -->` HTML comment and a broken anchor-link TOC. Replace with a clean landing page; do not include the raw file.
- **Mixing `-` and `*` in SUMMARY.md:** mdBook silently drops entries after the first delimiter inconsistency.
- **Committing docs/target/book/ to git:** Always add `docs/target/` to `.gitignore`. Generated output does not belong in source control.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Callout/admonition boxes | Custom HTML/CSS | mdbook-admonish 1.20.0 | Material Design styling, maintained, fenced code block syntax |
| Static site generator | Custom Markdown-to-HTML | mdBook 0.4.51 | Built-in search, chapter navigation, sidebar, print view, ToC |
| H1 stripping script | Complex parser | Simple Python/sed one-liner per file | The pattern is deterministic: line 1 is always the shared H1 |

**Key insight:** The H1 stripping is the only custom code needed in this phase. Everything else uses existing tools exactly as designed.

---

## Common Pitfalls

### Pitfall 1: Duplicate H1 Creates Broken Sidebar Titles
**What goes wrong:** All 70 chapters show "Writ Language Specification" or "Writ IL Specification" in the sidebar instead of section-specific titles. Anchor IDs collide across all chapter pages.
**Why it happens:** All spec files open with the same document-level H1 from the original monolithic spec format.
**How to avoid:** Strip the shared H1 and promote section H2 to H1 in all 70 spec files before creating SUMMARY.md entries. Verify `mdbook serve` shows unique sidebar titles.
**Warning signs:** Sidebar shows the same title for every chapter.

### Pitfall 2: build-dir Default Pollutes git Status
**What goes wrong:** After `mdbook build`, `git status` shows hundreds of untracked `.html` files.
**Why it happens:** Default `build-dir = "book"` puts output at `docs/book/`, not covered by `.gitignore`.
**How to avoid:** Set `build-dir = "target/book"` in `book.toml`. The `target/` directory is already gitignored.
**Warning signs:** HTML files appear in `git status` after a build.

### Pitfall 3: site-url Missing Causes Unstyled Deployed Site
**What goes wrong:** `mdbook serve` looks correct locally but the deployed GitHub Pages site has no CSS, no search, and broken navigation.
**Why it happens:** Without `site-url = "/Writ/"`, mdBook generates root-relative asset paths (`/book.css`) that 404 on GitHub Pages where the book is served from `/Writ/`.
**How to avoid:** Set `site-url = "/Writ/"` in `book.toml` before any testing. Verify by inspecting the generated `target/book/index.html` for `href="/Writ/..."` asset references.
**Warning signs:** Deployed site is unstyled; DevTools shows 404 for `book.css`.

### Pitfall 4: 01_table_of_contents.md HTML Comment Renders Visibly
**What goes wrong:** The TOC chapter shows a giant visible Markdown list of dead anchor links (all pointing to `#1-writ-language-specification`).
**Why it happens:** mdBook passes HTML comments through to the Markdown renderer; the `<!-- TOC -->` marker is invisible but the Markdown list below it renders as clickable links that navigate nowhere useful.
**How to avoid:** Replace `01_table_of_contents.md` with a clean landing page. Do not `{{#include}}` the raw TOC file.
**Warning signs:** TOC chapter displays a list of links that all 404 or navigate within the same page.

### Pitfall 5: SUMMARY.md Syntax Errors Drop Chapters Silently
**What goes wrong:** Some chapters disappear from the sidebar with no error message.
**Why it happens:** mdBook's SUMMARY.md parser is strict — mixed list delimiters, part titles not using H1, or nested prefix chapters silently break the entry.
**How to avoid:** Use `-` consistently throughout. Only use `#` (H1) for part titles. Run `mdbook build` after each section is added to catch errors early.
**Warning signs:** Fewer chapters appear than expected; no build error is reported.

### Pitfall 6: mdbook-admonish Version Mismatch Breaks Build
**What goes wrong:** `mdbook build` fails with a version mismatch error after running `mdbook-admonish install`.
**Why it happens:** The `assets_version` field in `book.toml` must match the installed binary version. If the binary is upgraded without rerunning `mdbook-admonish install`, the CSS file and the version in book.toml diverge.
**How to avoid:** Always run `mdbook-admonish install docs/` after installing or upgrading the binary. The install command updates both the CSS file and the `assets_version` in `book.toml`.
**Warning signs:** Build error mentioning `assets_version` or CSS version mismatch.

---

## Code Examples

Verified patterns from official sources:

### book.toml (Complete Configuration for Phase 87)
```toml
# Source: mdBook official docs (general.html, renderers.html)
[book]
title = "Writ Language"
authors = ["Writ Contributors"]
description = "A game scripting language with first-class dialogue support"
language = "en"
src = "src"

[build]
build-dir = "target/book"

[output.html]
site-url = "/Writ/"
git-repository-url = "https://github.com/X39/Writ"
git-repository-icon = "fa-github"
edit-url-template = "https://github.com/X39/Writ/edit/master/{path}"

[output.html.search]
enable = true

[preprocessor.admonish]
command = "mdbook-admonish"
assets_version = "3.0.3"   # injected/updated by `mdbook-admonish install docs/`
```

### mdbook-admonish Callout Box Syntax
```markdown
<!-- Source: tommilligan.github.io/mdbook-admonish/ -->

```admonish note
This is a note callout.
```

```admonish warning
Be careful with this pattern.
```

```admonish tip
A helpful suggestion.
```

```admonish info title="Custom Title"
Admonish supports custom titles via TOML parameters.
```
```

### H1 Strip Script (Shell)
```bash
# Strip shared H1 and promote all headings by one level
# Run from repo root: bash strip_h1.sh language-spec/spec/
# Processes all 70 spec files in-place
for f in language-spec/spec/*.md; do
  first_line=$(head -1 "$f")
  # Only process files with the known shared H1 patterns
  if [[ "$first_line" == "# 1. Writ Language Specification" || \
        "$first_line" == "# Writ IL Specification" || \
        "$first_line" == "# Appendix" ]]; then
    # Remove first line (and following blank line if present), then promote headings
    python3 - "$f" << 'PYEOF'
import sys, re
path = sys.argv[1]
with open(path, 'r', encoding='utf-8') as fh:
    lines = fh.readlines()
# Drop first line (shared H1)
lines = lines[1:]
# Drop leading blank lines
while lines and lines[0].strip() == '':
    lines = lines[1:]
# Promote heading levels: ## -> #, ### -> ##, etc.
result = []
for line in lines:
    m = re.match(r'^(#{2,})(.*)', line)
    if m:
        result.append('#' * (len(m.group(1)) - 1) + m.group(2) + '\n')
    else:
        result.append(line)
with open(path, 'w', encoding='utf-8') as fh:
    fh.writelines(result)
PYEOF
  fi
done
```

### Minimal Wrapper File Pattern
```markdown
<!-- docs/src/language-ref/dialogue.md -->
<!-- No H1 here — the spec file's promoted H1 is the chapter heading -->
{{#include ../../../language-spec/spec/15_14_dialogue_blocks_dlg.md}}
```

### Test Page for mdbook-admonish Verification (introduction.md)
```markdown
# Writ Language

Welcome to the Writ language documentation.

```admonish note
Writ is a statically-typed game scripting language. This documentation covers the
language reference, IL specification, and architecture overview.
```

```admonish warning
Writ is pre-1.0. Breaking changes may occur.
```

```admonish tip
Start with the [Language Reference](language-ref/overview.md) section.
```
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| mdBook admonitions via plugin | mdBook 0.5.x has built-in `> [!NOTE]` syntax | mdBook 0.5.0 (2024) | Irrelevant for now — 0.5.x broke mdbook-admonish; stay on 0.4.51 |
| `actions/upload-pages-artifact@v1/v2` | v3 required | January 2025 | Phase 92 (CI) must use v3 |
| Branch-push to gh-pages | GitHub Actions artifact deployment | GitHub Actions GA | Settings must be switched to "GitHub Actions" source — Phase 92 concern |

**Deprecated/outdated:**
- mdbook-linkcheck: Abandoned since ~2022; breaks with edition 2024 in book.toml. Do not install.
- `peaceiris/actions-mdbook@v1`: Old wrapper action; prefer direct `cargo install` in CI.

---

## Open Questions

1. **`00_preamble.md` inclusion strategy**
   - What we know: After H1 strip, the preamble contains intro prose with no section H2. It's a short preamble about the language and file extension.
   - What's unclear: Should it be wired into SUMMARY.md at all, or merged into `introduction.md`?
   - Recommendation: Wire it as the first entry under "Language Reference" or merge its content into `docs/src/introduction.md`. Given its brevity (~10 lines), merging into `introduction.md` is cleaner than a near-empty dedicated chapter.

2. **Heading promotion depth**
   - What we know: The heading promotion (H2 → H1) is correct for files where the real content starts at H2.
   - What's unclear: Do any spec files have H4+ headings that would need H4 → H3 promotion?
   - Recommendation: Apply promotion uniformly to all heading levels (reduce by one). This is safe regardless of depth.

3. **`docs/target/book/` vs `docs/book/` in Success Criterion 1**
   - What we know: The ROADMAP success criterion says output in `docs/target/book/`. This requires `build-dir = "target/book"` in book.toml.
   - What's unclear: None — this is clear. Use `target/book` as build-dir.
   - Recommendation: Set `build-dir = "target/book"` in book.toml. Add `docs/target/` to `.gitignore` if it is not already covered (the root `.gitignore` covers `target/` at repo root, but may not cover `docs/target/`).

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | Install mdBook + mdbook-admonish | Yes | cargo 1.93.0-nightly, rustc 1.93.0-nightly | — |
| mdBook | Build the docs site | No (not installed) | — | `cargo install mdbook --version 0.4.51` |
| mdbook-admonish | INFRA-03 admonish callouts | No (not installed) | — | `cargo install mdbook-admonish --version 1.20.0` |
| Python 3 | H1 strip script | Assumed available (Windows 11 + typical dev env) | — | Port to shell/sed if absent |

**Missing dependencies with no fallback:**
- mdBook 0.4.51 — must be installed before any plan tasks can be executed

**Missing dependencies with fallback:**
- Python 3 for H1 strip script — can be replaced with a sed/awk one-liner if Python is unavailable; or the H1 strip can be done directly as part of the Rust build script

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | mdBook build (CLI validation — no unit test framework for this phase) |
| Config file | `docs/book.toml` (created in Wave 1) |
| Quick run command | `mdbook build docs/` |
| Full suite command | `mdbook build docs/ && mdbook test docs/` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INFRA-01 | `docs/` directory with book.toml and SUMMARY.md exists | smoke | `test -f docs/book.toml && test -f docs/src/SUMMARY.md` | Wave 0 creates these |
| INFRA-02 | site-url = "/Writ/" in generated HTML | smoke | `grep -r 'href="/Writ/' docs/target/book/index.html` | Wave 0 creates book.toml |
| INFRA-03 | admonish CSS present and callout renders | smoke | `grep -r 'admonish' docs/target/book/index.html` | Wave 0 creates book.toml |
| LANG-02 | No chapter shows "Writ Language Specification" in sidebar | smoke | `grep -r "Writ Language Specification" docs/target/book/toc.js 2>/dev/null \|\| echo "PASS"` | Wave 1 strips H1s |

**Note:** All validation is `mdbook build` success plus HTML inspection. There are no unit tests because the output is HTML, not code.

### Sampling Rate
- **Per task commit:** `mdbook build docs/` — must exit 0
- **Per wave merge:** `mdbook build docs/ && mdbook test docs/`
- **Phase gate:** Full build green + manual `mdbook serve` verification of sidebar titles

### Wave 0 Gaps
- [ ] `docs/book.toml` — must exist before any mdbook command runs
- [ ] `docs/src/SUMMARY.md` — required by mdBook (build fails without it)
- [ ] `cargo install mdbook --version 0.4.51` — binary must be installed

*(All gaps are resolved in Wave 1 of the plan — this phase is creating the infrastructure from scratch)*

---

## Sources

### Primary (HIGH confidence)
- [mdBook official docs — Configuration](https://rust-lang.github.io/mdBook/format/configuration/general.html) — `src`, `build-dir`, `language` options confirmed
- [mdBook official docs — HTML renderer config](https://rust-lang.github.io/mdBook/format/configuration/renderers.html) — `site-url` option confirmed with GitHub Pages use case
- [mdBook official docs — SUMMARY.md format](https://rust-lang.github.io/mdBook/format/summary.html) — part titles, chapter syntax, prefix/suffix chapters
- [mdBook official docs — Include directive](https://rust-lang.github.io/mdBook/format/mdbook.html) — line range and anchor includes confirmed
- [mdbook-admonish official docs](https://tommilligan.github.io/mdbook-admonish/) — install command, book.toml config, fenced code block syntax
- Direct inspection of `D:/dev/git/Writ/language-spec/spec/` — 70 spec files, H1 patterns confirmed by grep

### Secondary (MEDIUM confidence)
- [mdbook-admonish GitHub releases](https://github.com/tommilligan/mdbook-admonish/releases/latest) — 1.20.0 is the latest release (June 2025)
- [mdbook-admonish crates.io](https://crates.io/crates/mdbook-admonish) — `mdbook ^0.4.40` dependency confirmed
- [mdBook issue #233 — mdBook 0.5 incompatibility](https://github.com/tommilligan/mdbook-admonish/issues/233) — 0.5.x incompatibility confirmed

### Tertiary (LOW confidence)
- None for this phase — all critical claims verified from official sources

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions verified from crates.io and official releases
- Architecture: HIGH — patterns verified from official mdBook docs and existing project research
- Pitfalls: HIGH — confirmed by direct inspection of the 70 spec files and official doc sources

**Research date:** 2026-03-27
**Valid until:** 2026-04-27 (mdBook 0.4.x stable; admonish 1.x API stable)

---

## Project Constraints (from CLAUDE.md)

No CLAUDE.md found in the project root. No project-specific coding constraints to enforce.

**From STATE.md (treated as locked decisions):**
- mdBook 0.4.51 pinned — do not use 0.5.x
- `site-url = "/Writ/"` must be set before any local testing
- Spec files use `{{#include}}` from wrapper files — `language-spec/spec/` stays as single source of truth
- `build-dir = "target/book"` implied by success criterion: "produces output in docs/target/book/"
- `docs/target/book/` must not be committed to git (add to `.gitignore`)
