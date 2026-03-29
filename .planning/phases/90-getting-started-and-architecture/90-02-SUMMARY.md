---
phase: 90-getting-started-and-architecture
plan: "02"
subsystem: docs
tags: [documentation, mdbook, architecture, compiler-pipeline, crate-map]
dependency_graph:
  requires: [Phase 87 (mdBook scaffold), Phase 88 (syntax highlighting), Phase 90-01 (Getting Started)]
  provides: [Architecture section in mdBook site]
  affects: [docs/src/SUMMARY.md, docs/src/architecture/, docs/src/introduction.md]
tech_stack:
  added: []
  patterns: [Standalone prose chapters, mdbook-admonish callouts, ASCII dependency table]
key_files:
  created:
    - docs/src/architecture/compiler-pipeline.md
    - docs/src/architecture/crate-map.md
  modified:
    - docs/src/SUMMARY.md
    - docs/src/introduction.md
decisions:
  - "Architecture page uses prose + table only, no code snippets — per CONTEXT.md locked decision; code belongs in cargo doc"
  - "All 10 crates documented (not 9 as stated in ARCH-02) — writ-golden is test-only; admonish note clarifies its test-only status"
  - "Dependency layers prose section added — explains foundation/compiler/runtime/tooling layering in plain text"
metrics:
  duration: "5m"
  completed: "2026-03-27"
  tasks_completed: 2
  files_created: 2
  files_modified: 2
---

# Phase 90 Plan 02: Architecture Documentation Summary

Architecture documentation section with Compiler Pipeline overview (5-stage pipeline table + prose) and Crate Map (10-crate dependency table + Contributing section), wired into mdBook navigation between Getting Started and Language Reference.

## Tasks Completed

### Task 1: Create Architecture chapter files

Created `docs/src/architecture/` directory with two standalone prose Markdown files.

- **compiler-pipeline.md** — `# Compiler Pipeline` H1, 5-stage pipeline table (Parse/Lower/Resolve/Typecheck/Codegen with Input/Output/Crate columns), one prose paragraph per stage describing what it does, Pipeline Integration section explaining writ-cli orchestration.
- **crate-map.md** — `# Crate Map` H1, 10-crate Markdown table (Crate/Binary/Purpose/Workspace Dependencies columns), admonish note about writ-golden test-only status, Dependency Layers prose section, Contributing section with Building/Testing/Pull Requests subsections.

**Commit:** `d73097c`

### Task 2: Wire Architecture section into SUMMARY.md, update introduction, and verify build

Inserted `# Architecture` section with 2 chapter links between `# Getting Started` and `# Language Reference` in `docs/src/SUMMARY.md`. Updated `docs/src/introduction.md` with 4-section documentation list (Getting Started, Architecture, Language Reference, IL Specification), updated admonish note to mention getting started guides, and updated admonish tip to point new users to `getting-started/installation.md` instead of the old Overview link. Verified `mdbook build` exits 0.

**Commit:** `4c69b36`

## Verification Results

- `docs/src/architecture/compiler-pipeline.md` exists, contains `# Compiler Pipeline`, all 5 stage names (Parse, Lower, Resolve, Typecheck, Codegen), owning crate for each stage, no code snippets
- `docs/src/architecture/crate-map.md` exists, contains `# Crate Map`, all 10 crate names, `## Contributing` section
- `docs/src/SUMMARY.md` has `# Architecture` at line 11, between `# Getting Started` (line 5) and `# Language Reference` (line 16)
- `docs/src/SUMMARY.md` contains `- [Compiler Pipeline](architecture/compiler-pipeline.md)` and `- [Crate Map](architecture/crate-map.md)`
- `docs/src/introduction.md` contains Getting Started and Architecture in the documentation list
- `docs/src/introduction.md` tip callout points to `getting-started/installation.md`
- `mdbook build` exits 0 (INFO: Book building has started; INFO: Running the html backend)

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — all content is fully written and wired into site navigation.

## Self-Check: PASSED

- `docs/src/architecture/compiler-pipeline.md` — FOUND
- `docs/src/architecture/crate-map.md` — FOUND
- Commit `d73097c` — FOUND
- Commit `4c69b36` — FOUND
