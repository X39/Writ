# Phase 89: Language Reference and IL Spec - Research

**Researched:** 2026-03-27
**Domain:** mdBook documentation wiring, cross-references, and table rendering
**Confidence:** HIGH

## Summary

Phase 87 created 68 wrapper files (`docs/src/language-ref/*.md` and `docs/src/il-spec/*.md`) and wired them into `SUMMARY.md` under two top-level sections: "Language Reference" and "IL Specification". Every wrapper is a single-line `{{#include}}` directive pointing to the real source in `language-spec/spec/`. Phase 88 added Writ syntax highlighting. The build currently succeeds (`mdbook build` completes with no errors, only a benign version warning from mdbook-admonish 1.20.0 built against 0.4.52 while being called from 0.4.51 — this has no functional impact).

The work remaining in Phase 89 is entirely content-level: (1) verify all 68 chapters appear in the SUMMARY.md navigation (they do — already wired), (2) confirm IL spec tables and code blocks render correctly (current spot-checks show well-formed Markdown tables throughout), (3) add 3-5 cross-references using mdBook relative links. There are no structural blockers; the phase is mostly verification plus small edits.

**Primary recommendation:** Run `mdbook build` and `mdbook serve` to visually confirm all chapters render. Then add the 5 natural cross-references identified below directly to the spec source files in `language-spec/spec/`. No SUMMARY.md changes are required — the navigation structure is already complete.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **Cross-Reference Strategy**: Use mdBook relative links (`[Section X](../language-ref/types.md#enums)`) — standard approach, no preprocessor needed
- **Cross-references**: Create 3-5 natural cross-references at linkage points: types ↔ structs/enums, entities ↔ components, dialogue ↔ concurrency, IL instruction set → execution model
- **Forward references only** — no bidirectional linking (marginal benefit, high edit count)
- **IL Spec Rendering**: Fix table rendering issues in-place in spec source files — fix alignment issues or missing pipe characters
- **IL spec code blocks do NOT get syntax highlighting** — IL assembly/pseudocode is a different language from Writ
- **Keep the existing SUMMARY.md navigation structure** from Phase 87 — Language Reference and IL Specification already in separate sections

### Claude's Discretion
- Specific cross-reference selection (which 3-5 to add)
- Table formatting fixes needed for IL spec
- Any chapter title adjustments for clarity

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LANG-01 | Language spec chapters (syntax, types, structs, classes, entities, dialogue, concurrency) wired into SUMMARY.md as browsable chapters | Already wired in SUMMARY.md from Phase 87; 29 chapters under "# Language Reference" section — verification only |
| LANG-03 | Cross-references between spec chapters work as mdBook internal links | Relative path format `[text](../section/file.md#heading-anchor)` verified; natural linkage points identified below |
| IL-01 | IL spec chapters (instruction set, execution model, module format, type system) wired into SUMMARY.md | Already wired; 39 chapters under "# IL Specification" section — verification only |
| IL-02 | IL spec chapters render correctly with tables and code blocks preserved | Tables verified well-formed; code blocks use bare fences (no IL language tag — correct per decision); spot-check of 10+ files shows no alignment breakage |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| mdBook | 0.4.51 | Static site generator for documentation | Pinned (0.5.x breaks mdbook-admonish) — already installed at `/c/Users/msili/.cargo/bin/mdbook` |
| mdbook-admonish | 1.20.0 | Callout boxes (info/warning/tip) | Already installed; benign version warning (built for 0.4.52, running on 0.4.51) does not affect output |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `{{#include}}` preprocessor | built-in | Source file inclusion in wrapper chapters | Used in all 68 wrapper files — single-line pattern already established |

**Installation:** No new installations required. All tools already present on this machine.

## Architecture Patterns

### Established Project Structure
```
docs/
├── book.toml                     # mdBook config (site-url="/Writ/", build-dir=target/book)
├── mdbook-admonish.css           # admonish styles
├── src/
│   ├── SUMMARY.md                # Navigation (68 chapters + introduction, complete)
│   ├── introduction.md           # Landing page
│   ├── language-ref/             # 29 wrapper files (01-29)
│   │   └── *.md                  # Each: {{#include ../../../language-spec/spec/NN_file.md}}
│   └── il-spec/                  # 39 wrapper files
│       └── *.md                  # Each: {{#include ../../../language-spec/spec/NN_file.md}}
└── theme/
    └── highlight.js              # Bundled hljs + Writ grammar appended
language-spec/spec/               # Source of truth (00_ through 69_)
    ├── 00_-29_ *.md              # Language reference (H1 already stripped in Phase 87)
    └── 30_-69_ *.md              # IL specification (H1 already stripped)
```

### Pattern 1: Wrapper Include
**What:** Each chapter file is a single `{{#include}}` directive pointing to the spec source.
**When to use:** All existing chapters follow this — do not add content to wrapper files.
**Example:**
```markdown
{{#include ../../../language-spec/spec/06_5_type_system.md}}
```
*Note: Cross-references and table fixes go in the spec source file, not the wrapper.*

### Pattern 2: mdBook Relative Cross-Reference
**What:** Links between chapters using relative paths from the wrapper file's location.
**When to use:** When one spec chapter references concepts defined in another.
**Example — from language-ref/concurrency.md to il-spec/execution.md:**
```markdown
[Execution Model](../il-spec/execution.md#217-execution-model)
```
**Example — from language-ref/entities.md to language-ref/components.md:**
```markdown
[Components](components.md#161-component-declarations)
```

**Anchor format:** mdBook generates GitHub-flavored anchors from headings. Rules:
- Lowercase all characters
- Replace spaces with `-`
- Strip all punctuation except `-`
- Dots in section numbers (e.g., `## 2.17.2 Task States`) → anchor `#2172-task-states`
- Em-dash (`—`) is stripped: `## 0x00 — Meta` → `#0x00--meta`

### Pattern 3: Table Fix in Spec Source
**What:** Edit `language-spec/spec/NN_file.md` directly for alignment or pipe issues.
**When to use:** Only when a table fails to render (missing pipe, misaligned separator row).
**Important:** The spec source is the only file to edit — wrapper files are untouched.

### Anti-Patterns to Avoid
- **Editing wrapper files for cross-references:** Add links to the spec source files, not to wrapper files. Wrapper content appears verbatim in the rendered page via include.
- **Adding language tag to IL code blocks:** IL assembly is not a known highlight.js language; leaving fences bare (` ``` `) is correct and already the pattern throughout.
- **Bidirectional cross-references:** Per locked decision, forward-only. Do not add back-links.
- **Modifying SUMMARY.md:** The navigation is complete from Phase 87 — no additions needed.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-references | Custom preprocessor or link-rewriting script | mdBook native relative links | mdBook resolves relative paths at build time; preprocessors add complexity |
| Link validation | Custom script | Manual verification with `mdbook build` (errors on broken links) | mdBook reports broken `{{#include}}` paths as build errors |
| Table formatting | Reformatting tool | Direct text edit | Tables are already well-formed; only edge cases need fixing |

## Current State Assessment

### Build Status
- **mdBook version:** 0.4.51 (pinned) — verified installed
- **Build result:** Success (no errors, one benign version warning)
- **Warning:** `mdbook-admonish preprocessor was built against version 0.4.52 ... being called from version 0.4.51` — this is cosmetic only; admonish syntax in spec files renders correctly

### SUMMARY.md Completeness
All 68 chapters are wired. Verified counts:
- Language Reference section: 29 chapters (overview through lowering-reference)
- IL Specification section: 39 chapters (vm through decision-log)
- No missing entries; no gaps in navigation

### Wrapper File Integrity
- All 68 wrapper files contain exactly 1 line
- All `{{#include}}` targets verified to exist on disk
- No broken include paths

### Table Rendering Health
Spot-checked 12 IL spec files. Findings:
- Tables use standard GFM alignment (pipes, separator row with dashes) — all well-formed
- The large Metadata Tables table in `45_2_16_il_module_format.md` has very wide "Key Fields" and "Purpose" columns but valid pipe syntax — renders correctly in browsers (horizontal scroll)
- `56_3_8_object_model.md` (Calls section): wide "Description" cells — valid syntax, wide columns are normal for opcode reference tables
- `65_4_0_instruction_count_by_category.md`: well-formed with bold total row
- `66_4_1_instruction_shape_reference.md`: clean, 5 columns, well-formed
- **No alignment fixes identified as required** in spot-check

### Code Block Health
- Language spec code blocks use ` ```writ ` — correct (Writ highlighting active from Phase 88)
- IL spec code blocks use bare ` ``` ` — correct per locked decision (no IL syntax highlighting)
- `ebnf` language tag used in grammar summary — not defined in hljs; renders as plain text, which is acceptable

## Cross-Reference Selection (Claude's Discretion)

Five natural forward cross-references to add. All go into the spec source file (`language-spec/spec/`), not wrappers.

### Cross-reference 1: Types → Structs
**Source file:** `language-spec/spec/06_5_type_system.md`
**Location:** After the Type Categories table (end of §1.5.1)
**Link target:** `language-ref/structs.md` (which includes `09_8_structs.md`)
**Text to add:**
```markdown
See [Structs](../language-ref/structs.md) for struct declaration syntax and semantics.
```
**Rationale:** Type table lists structs as value types — direct reader to the full struct reference.

### Cross-reference 2: Entities → Components
**Source file:** `language-spec/spec/16_15_entities.md`
**Location:** §1.15.1 Entity Declaration (after the `use` keyword is introduced in the code example)
**Link target:** `language-ref/components.md`
**Text to add:**
```markdown
Components are always extern and data-only. See [Components](components.md) for the full component declaration syntax.
```
**Rationale:** Entities reference components via `use` — readers need to understand component declarations.

### Cross-reference 3: Dialogue → Concurrency
**Source file:** `language-spec/spec/15_14_dialogue_blocks_dlg.md`
**Location:** §1.14.9 Dialogue Suspension
**Link target:** `language-ref/concurrency.md`
**Text to add:**
```markdown
Dialogue suspension uses the same cooperative task model described in [Concurrency](concurrency.md).
```
**Rationale:** Dialogue suspension text references the task scheduler — direct link to the concurrency chapter.

### Cross-reference 4: Concurrency → IL Execution Model
**Source file:** `language-spec/spec/22_21_concurrency.md`
**Location:** End of §1.21.1 Execution Model
**Link target:** `../il-spec/execution.md`
**Text to add:**
```markdown
The full task state machine and scheduling semantics are specified in the [IL Execution Model](../il-spec/execution.md).
```
**Rationale:** Language-level concurrency chapter mentions yielding — links to the authoritative IL specification.

### Cross-reference 5: IL Instruction Set (Calls) → IL Execution Model
**Source file:** `language-spec/spec/55_3_7_calls.md`
**Location:** After the calls table (end of file)
**Link target:** `execution.md` (same il-spec directory)
**Text to add:**
```markdown
Call semantics, including transition-point suspension for `CALL_EXTERN`, are detailed in [Execution Model](execution.md#2173-transition-points).
```
**Rationale:** The calls table references suspend behavior — forward link to the execution model section that defines it.

## Common Pitfalls

### Pitfall 1: Anchor Generation for Numbered Headings
**What goes wrong:** mdBook anchor for `## 2.17.3 Transition Points` is `#2173-transition-points` (dots stripped), not `#2.17.3-transition-points`.
**Why it happens:** GFM anchor rules strip punctuation including dots; the section number dots are not preserved.
**How to avoid:** Verify each anchor by building the book and inspecting the rendered HTML, or apply the rule: lowercase, spaces-to-dashes, all non-alphanumeric-non-dash stripped.
**Warning signs:** A cross-reference link renders but clicking it scrolls to top of page (anchor not found).

### Pitfall 2: Relative Path Direction
**What goes wrong:** A cross-reference from a `language-ref/` wrapper to an `il-spec/` wrapper uses the wrong relative path (e.g., `il-spec/execution.md` instead of `../il-spec/execution.md`).
**Why it happens:** The include is served from the wrapper file's location. Paths must be relative to the wrapper, not the spec source.
**How to avoid:** Cross-references in spec source files must account for the wrapper location. From `language-spec/spec/`, the rendered path is `docs/src/language-ref/` — so going to `il-spec/` requires `../il-spec/`.
**Warning signs:** `mdbook build` prints a warning about unresolvable link.

### Pitfall 3: Cross-References in Included Files vs. Wrapper Files
**What goes wrong:** A link added to a spec source file uses a path relative to `language-spec/spec/` (the actual file location) rather than relative to `docs/src/language-ref/` (where it will be rendered).
**Why it happens:** The `{{#include}}` preprocessor copies content verbatim; relative links in included content are resolved relative to the including file's location, not the included file's location.
**How to avoid:** Write links as if they are in `docs/src/language-ref/` or `docs/src/il-spec/`, not in `language-spec/spec/`. Example: to link from a language-ref chapter to an il-spec chapter, use `../il-spec/execution.md`, not `../../docs/src/il-spec/execution.md`.

### Pitfall 4: Wide Table Overflow
**What goes wrong:** Very wide tables (e.g., the 21-row Metadata Tables table in `45_2_16_il_module_format.md`) may appear cut off on narrow screens.
**Why it happens:** mdBook's default CSS allows tables to overflow with horizontal scroll, but very long cell content may wrap awkwardly.
**How to avoid:** No fix needed — table overflow is standard behavior. The tables are valid Markdown and render correctly.

## Code Examples

### Verified mdBook Relative Link Patterns

Cross-reference from language-ref chapter to another language-ref chapter (same directory):
```markdown
See [Components](components.md) for component declaration syntax.
```

Cross-reference from language-ref chapter to il-spec chapter (cross-directory):
```markdown
See [Execution Model](../il-spec/execution.md) for task scheduling semantics.
```

Cross-reference to a specific section anchor:
```markdown
See [Transition Points](../il-spec/execution.md#2173-transition-points) for suspension rules.
```

mdBook anchor derivation rule (applied to `## 2.17.3 Transition Points`):
1. Lowercase: `2.17.3 transition points`
2. Replace spaces with `-`: `2.17.3-transition-points`
3. Strip non-alphanumeric, non-dash: `2173-transition-points`
Result: `#2173-transition-points`

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| mdbook | Build verification | ✓ | 0.4.51 | — |
| mdbook-admonish | Callout rendering | ✓ | 1.20.0 | — |

**Missing dependencies with no fallback:** None.

**Note:** The mdbook-admonish version warning (built for 0.4.52, running on 0.4.51) is cosmetic — admonish directives are not used in the spec files being wired in this phase.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | mdBook build (CLI tool, not a test framework) |
| Config file | `docs/book.toml` |
| Quick run command | `cd docs && mdbook build 2>&1` |
| Full suite command | `cd docs && mdbook build 2>&1 && mdbook serve --open 2>&1` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LANG-01 | All 29 language-ref chapters browsable in navigation | smoke | `cd docs && mdbook build 2>&1 \| grep -c "ERROR"` (expect 0) | ✅ SUMMARY.md already wired |
| LANG-03 | Cross-references resolve as working mdBook internal links | manual | `cd docs && mdbook build 2>&1` (broken links print warnings) | ❌ Wave 0: add 3-5 links first |
| IL-01 | All 39 IL spec chapters browsable in navigation | smoke | `cd docs && mdbook build 2>&1 \| grep -c "ERROR"` (expect 0) | ✅ SUMMARY.md already wired |
| IL-02 | Tables and code blocks render correctly | manual | Visual inspection via `mdbook serve` | ✅ Tables verified well-formed |

### Sampling Rate
- **Per task commit:** `cd docs && mdbook build 2>&1`
- **Per wave merge:** `cd docs && mdbook build 2>&1` (build must be clean)
- **Phase gate:** Clean build + visual inspection of at least one language-ref chapter and one IL spec chapter before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] Cross-reference links do not yet exist — must be added before LANG-03 can be validated (no test file creation needed; the links are edits to existing spec source files)

*(Existing mdBook build infrastructure covers all other requirements — no new test files needed)*

## Sources

### Primary (HIGH confidence)
- Direct file inspection — `docs/src/SUMMARY.md`, all 68 wrapper files, `docs/book.toml`
- Live build verification — `cd docs && mdbook build` exits successfully (2026-03-27)
- Spot-check of 12+ spec source files in `language-spec/spec/`

### Secondary (MEDIUM confidence)
- mdBook relative link behavior: standard documentation feature, verified in current mdBook 0.4.51 build
- GFM anchor derivation rules: standard behavior confirmed by mdBook documentation convention

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — tools verified installed and functional on this machine
- Architecture: HIGH — wrapper pattern inspected directly; build verified
- Pitfalls: HIGH — anchor format and relative-path rules are deterministic and verified
- Cross-references: HIGH — linkage points chosen from confirmed section headings in inspected source files

**Research date:** 2026-03-27
**Valid until:** 2026-06-01 (mdBook 0.4.51 is pinned; spec files are stable)
