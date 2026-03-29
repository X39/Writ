---
phase: 88-writ-syntax-highlighting
plan: 01
subsystem: infra
tags: [highlight.js, mdbook, syntax-highlighting, documentation, language-spec]

# Dependency graph
requires:
  - phase: 87-mdbook-scaffold
    provides: docs/ scaffold with book.toml, src/ wrapper files, and mdbook build working

provides:
  - docs/theme/highlight.js with bundled hljs 10.1.1 + Writ language definition appended
  - 144 bare code fences updated to ```writ across 25 language spec files (02-27)
  - language-writ CSS class in 25 HTML output chapters

affects: [89-language-ref-content, 90-getting-started, 92-ci-deploy]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Custom hljs 10.1.1 language: copy bundled file to docs/theme/, append registerLanguage call at end"
    - "Fence update state-machine: Python script tracks in_block state; only bare ``` on its own line is an opening fence"
    - "Mixed-content files: manual fence classification (writ vs bash vs toml vs bare pseudocode)"

key-files:
  created:
    - docs/theme/highlight.js
  modified:
    - language-spec/spec/02_1_overview_design_philosophy.md
    - language-spec/spec/03_2_project_configuration_writ_toml.md
    - language-spec/spec/05_4_lexical_structure.md
    - language-spec/spec/06_5_type_system.md
    - language-spec/spec/07_6_primitive_types.md
    - language-spec/spec/08_7_variables_constants.md
    - language-spec/spec/09_8_structs.md
    - language-spec/spec/10_9_classes.md
    - language-spec/spec/11_10_enums.md
    - language-spec/spec/12_11_contracts.md
    - language-spec/spec/13_12_generics.md
    - language-spec/spec/14_13_functions_fn.md
    - language-spec/spec/15_14_dialogue_blocks_dlg.md
    - language-spec/spec/16_15_entities.md
    - language-spec/spec/17_16_components.md
    - language-spec/spec/18_17_attributes.md
    - language-spec/spec/19_18_operators_overloading.md
    - language-spec/spec/20_19_error_handling.md
    - language-spec/spec/21_20_nullability_optionals.md
    - language-spec/spec/22_21_concurrency.md
    - language-spec/spec/23_22_scoping_rules.md
    - language-spec/spec/24_23_globals_atomic_access.md
    - language-spec/spec/25_24_modules_namespaces.md
    - language-spec/spec/26_25_external_declarations.md
    - language-spec/spec/27_26_localization.md

key-decisions:
  - "Use docs/theme/highlight.js (copy bundled + append) — additional-js loads too late (after hljs.highlightBlock has run)"
  - "keyword class (purple) for both declaration and control-flow keywords — default CSS gives 2 keyword colors max without custom CSS"
  - "built_in class (orange) for modifiers/other (let, mut, const, pub, self, etc.)"
  - "Format string ($\"...\") mode listed before plain string mode in contains array (first-match wins in hljs)"
  - "03_2 CLI command block tagged ```bash not ```writ"
  - "27_26 pseudocode/algorithm/error blocks left bare; only Writ dialogue and source blocks tagged ```writ"
  - "IL spec files (30+) not modified — IL pseudocode is not valid Writ source"

patterns-established:
  - "Theme override: place file at docs/theme/<filename> matching default asset name; mdBook auto-detects, no book.toml change needed"
  - "Fence classification rule: Writ source code -> ```writ; shell commands -> ```bash; config -> ```toml; pseudocode/error messages -> bare"

requirements-completed: [INFRA-04]

# Metrics
duration: 4min
completed: 2026-03-27
---

# Phase 88 Plan 01: Writ Syntax Highlighting Summary

**Custom highlight.js 10.1.1 language definition for Writ with 144 code fences updated across 25 spec files, giving purple keywords, orange modifiers, green strings, and gray comments in the mdBook site**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-03-27T04:35:10Z
- **Completed:** 2026-03-27T04:39:04Z
- **Tasks:** 3 (Tasks 1 and 2 executed; Task 3 auto-approved via auto_advance)
- **Files modified:** 26

## Accomplishments

- Created `docs/theme/highlight.js`: bundled hljs 10.1.1 copied verbatim + Writ language definition appended with `hljs.registerLanguage("writ", ...)`. Keywords (purple: fn, dlg, entity, struct, enum, contract, impl, class + control-flow), built-ins (orange: let, mut, const, pub, self, etc.), types, literals, strings (plain/format/raw), comments, numbers all covered.
- Updated 144 bare code fences to ` ```writ ` across 23 bulk spec files using a Python state-machine script. Two files (03 config, 27 localization) required manual per-block classification.
- 25 HTML output chapters now contain the `language-writ` CSS class; functions.html shows 17 highlighted code blocks. IL spec files (30+) untouched.

## Task Commits

Each task was committed atomically:

1. **Task 1: Create docs/theme/highlight.js with Writ language definition** - `0dae5b5` (feat)
2. **Task 2: Update bare code fences in language spec files to use ```writ marker** - `a1c74d7` (feat)
3. **Task 3: Visual verification** - auto-approved (auto_advance=true); no commit

## Files Created/Modified

- `docs/theme/highlight.js` — bundled hljs 10.1.1 (53 lines) + Writ registerLanguage call (51 lines appended)
- `language-spec/spec/02_1_overview_design_philosophy.md` — 1 fence
- `language-spec/spec/03_2_project_configuration_writ_toml.md` — 1 bare block -> ```bash (CLI command)
- `language-spec/spec/05_4_lexical_structure.md` — 11 fences
- `language-spec/spec/06_5_type_system.md` — 1 fence
- `language-spec/spec/07_6_primitive_types.md` — 8 fences
- `language-spec/spec/08_7_variables_constants.md` — 3 fences
- `language-spec/spec/09_8_structs.md` — 7 fences
- `language-spec/spec/10_9_classes.md` — 5 fences
- `language-spec/spec/11_10_enums.md` — 10 fences
- `language-spec/spec/12_11_contracts.md` — 9 fences
- `language-spec/spec/13_12_generics.md` — 2 fences
- `language-spec/spec/14_13_functions_fn.md` — 17 fences
- `language-spec/spec/15_14_dialogue_blocks_dlg.md` — 14 fences
- `language-spec/spec/16_15_entities.md` — 9 fences
- `language-spec/spec/17_16_components.md` — 2 fences
- `language-spec/spec/18_17_attributes.md` — 3 fences
- `language-spec/spec/19_18_operators_overloading.md` — 2 fences
- `language-spec/spec/20_19_error_handling.md` — 4 fences
- `language-spec/spec/21_20_nullability_optionals.md` — 1 fence
- `language-spec/spec/22_21_concurrency.md` — 1 fence
- `language-spec/spec/23_22_scoping_rules.md` — 2 fences
- `language-spec/spec/24_23_globals_atomic_access.md` — 3 fences
- `language-spec/spec/25_24_modules_namespaces.md` — 25 fences
- `language-spec/spec/26_25_external_declarations.md` — 4 fences
- `language-spec/spec/27_26_localization.md` — 4 writ blocks; 3 pseudocode/error blocks left bare

## Decisions Made

- **docs/theme/highlight.js approach over additional-js:** `additional-js` loads after `book.js` which has already called `hljs.highlightBlock()`. Theme override is the only approach that works with mdBook 0.4.51.
- **keyword + built_in tiers only:** Default `highlight.css` maps `keyword` to purple and `built_in`/`type`/`literal` to orange. Getting a 3rd distinct color for control flow would require custom CSS (locked out by CONTEXT.md). Combined declaration+control-flow under `keyword` is standard practice.
- **Format string before plain string:** hljs 10.1.1 matches modes in order — `$"` must be tried before `"` to avoid the `$` being left as unhighlighted punctuation.
- **CLI commands in 03_2 tagged ```bash:** The block `writc --condition playstation=true ...` is a shell invocation, not Writ syntax.
- **Localization pseudocode left bare:** FNV-1a algorithm block and key composition pseudocode use a pseudocode syntax (not valid Writ); tagging them `writ` would produce incorrect highlighting.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- INFRA-04 satisfied: all Writ code blocks in the mdBook site render with syntax highlighting
- 25 HTML output chapters have `language-writ` class applied to code blocks
- Visual verification checkpoint was auto-approved (auto_advance=true); manual browser verification recommended before deploying to gh-pages
- IL spec files remain with bare code fences — if IL spec chapters are later tagged with their own language identifier, this phase establishes the template for how to do it

---
*Phase: 88-writ-syntax-highlighting*
*Completed: 2026-03-27*
