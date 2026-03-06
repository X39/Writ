---
phase: quick
plan: 260322-5zi
subsystem: documentation
tags: [readme, docs, v7.0]
dependency_graph:
  requires: []
  provides: [up-to-date project README, up-to-date VS Code extension README]
  affects: []
tech_stack:
  added: []
  patterns: []
key_files:
  created: []
  modified:
    - README.MD
    - writ-vscode/README.md
decisions:
  - "Kept License/Contributing/CLA sections verbatim from original README"
  - "Added Developer Tooling and Benchmark Suite as top-level feature sections"
  - "Replaced v0.5 version history with v1.0-v7.0 milestone summary"
metrics:
  duration: "~2 minutes"
  completed_date: "2026-03-22"
  tasks_completed: 2
  files_modified: 2
---

# Quick Task 260322-5zi: Update README.md and VS Code Extension README Summary

Updated both README files to accurately reflect the v7.0 project state: 10-crate workspace, full compiler pipeline, LSP/DAP developer tooling, and benchmark suite.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Rewrite project README.md | 7e5029a | README.MD |
| 2 | Update VS Code extension README | df100f1 | writ-vscode/README.md |

## Changes Made

### README.MD (root)

- Updated project description to reflect production-quality toolchain (79,005 LOC)
- Added **Developer Tooling** feature section (LSP/DAP/VS Code extension, crash stacktraces)
- Added **Benchmark Suite** feature section (Docker, 7 case categories, SVG charts, CI)
- Expanded Project Structure from 8 to 13 entries (added writ-lsp, writ-dap, writ-vscode/, benchmark/, and updated writ-cli description)
- Added `writ build [path]` to CLI commands table
- Replaced stale v0.5 single-line status with v1.0–v7.0 milestone history (7 bullet points)
- Updated spec section range from 30-69 to 30-66
- Preserved logo header, License, Contributing, Code of Conduct, and CLA verbatim

### writ-vscode/README.md

- Added Namespace completions to Language Intelligence feature list
- Added New-keyword completions to Language Intelligence feature list
- Added Crash stacktraces to Language Intelligence feature list
- Added Multi-file project debugging and Crash halt entries to Debugging (DAP) section
- Added 0.1.1 release notes section documenting all v6.1+ improvements

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None.

## Self-Check: PASSED

- README.MD exists and contains: writ-lsp, writ-dap, writ-vscode, benchmark, writ build, v7.0, LGPL-3.0-only
- writ-vscode/README.md exists and contains: Namespace completions, stacktrace, new keyword, 0.1.1
- Commits 7e5029a and df100f1 verified in git log
