---
phase: 122-cross-module-type-resolution
plan: "03"
subsystem: language-spec
tags: [xmod, spec, writ-toml, dependencies, cross-module]

requires:
  - phase: 122-01
    provides: [inject_module_types, inject_library_sigs, compile_with_libraries]
provides:
  - Language spec section 1.2.8 documenting [dependencies] table in writ.toml
  - Language spec section 1.2.9 documenting cross-module type resolution semantics
  - Language spec section 1.2.10 documenting built-in virtual module types
affects: [language-spec, user-documentation]

tech-stack:
  added: []
  patterns:
    - "Spec sections 1.2.8-1.2.10 document the public-facing contract for cross-module type resolution"

key-files:
  created: []
  modified:
    - language-spec/spec/03_2_project_configuration_writ_toml.md

key-decisions:
  - "New sections appended as 1.2.8, 1.2.9, 1.2.10 to the existing writ.toml spec file (not a new file) — keeps all project configuration in one place"
  - "using declarations documented as the standard mechanism for unqualified access to library namespaces — no special library-specific syntax"
  - "Duplicate definition errors identify the library by its writ.toml dependency name to aid user diagnosis"

patterns-established: []

requirements-completed: [XMOD-05]

duration: 1min
completed: "2026-03-29T22:38:19Z"
---

# Phase 122 Plan 03: Language Spec — Cross-Module Type Resolution Summary

**Added writ.toml [dependencies] documentation, cross-module type resolution semantics, and virtual module built-ins to the language spec (sections 1.2.8–1.2.10).**

## Performance

- **Duration:** 1 min
- **Started:** 2026-03-29T22:37:14Z
- **Completed:** 2026-03-29T22:38:19Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

### Task 1: Document cross-module type resolution in language spec

Added three new subsections to `language-spec/spec/03_2_project_configuration_writ_toml.md`:

**Section 1.2.8 — Dependencies**
- Describes the optional `[dependencies]` table in `writ.toml`
- Documents both simple-form (string path) and detailed-form (inline table with `path` key) syntax
- States that paths are relative to the project root, each dependency must be a valid `.writc` module, and the dependency name is used only in diagnostics

**Section 1.2.9 — Cross-Module Type Resolution**
- Explains that unrecognized type names are looked up from dependency modules
- Documents namespace visibility: fully-qualified names always work; `using` declarations bring a namespace into scope
- Lists resolution rules: `using` imports, type positions, method/field/contract resolution via binary signatures
- Documents the `E0001 duplicate definition` error with example output when a user type collides with a library type

**Section 1.2.10 — Virtual Module Types**
- Lists all 18 built-in contracts (Add, Sub, Mul, Div, Mod, Neg, Not, Eq, Ord, BitAnd, BitOr, Index, IndexSet, Iterable, Iterator, Into, Error, Hashable, Reflectable)
- Lists built-in generic types: `Option<T>`, `Result<T, E>`, `ChoiceOption`
- States these are available without any `[dependencies]` declaration
- Explains they use the same DefMap resolution mechanism as user library types

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None.

## Self-Check: PASSED
