---
phase: 53-lsp-server-skeleton-and-diagnostics
plan: "02"
subsystem: tooling
tags: [vscode, textmate, typescript, vscode-languageclient, syntax-highlighting, lsp]

# Dependency graph
requires: []
provides:
  - writ-vscode/ directory with complete VS Code extension skeleton
  - TextMate grammar (source.writ) for Writ syntax highlighting
  - package.json registering .writ file association and language server connection
  - extension.ts launching writ-lsp binary over stdio via LanguageClient
  - language-configuration.json for bracket matching and comment toggling
affects:
  - 53-03 (writ-lsp Rust crate — the binary this extension launches)
  - 57-packaging (vsix bundling will embed the writ-lsp binary into the extension)

# Tech tracking
tech-stack:
  added:
    - vscode-languageclient ^9.0.0 (Node.js LSP client)
    - "@types/vscode ^1.74.0 (TypeScript VS Code API definitions)"
    - "typescript ^5.0.0 (extension compilation)"
  patterns:
    - TextMate grammar with repository-based pattern composition
    - Stdio LanguageClient (command-based ServerOptions) for native Rust binary
    - activationEvents: [] (VS Code 1.74+ auto-activates from language contribution)

key-files:
  created:
    - writ-vscode/package.json
    - writ-vscode/tsconfig.json
    - writ-vscode/src/extension.ts
    - writ-vscode/language-configuration.json
    - writ-vscode/.vscodeignore
    - writ-vscode/syntaxes/writ.tmLanguage.json

key-decisions:
  - "activationEvents: [] sufficient for VS Code 1.74+ — no manual onLanguage event needed"
  - "Dev path: context.asAbsolutePath('../target/debug/writ-lsp') — Phase 57 bundles binary"
  - "Triple-quoted strings pattern ordered before double-quoted to prevent greedy matching"
  - "9 grammar repository entries: comments, attributes, strings, keywords, numbers, dialogue-blocks, function-calls, type-names, operators"
  - "String ordering in grammar: triple -> formattable ($\") -> regular double-quoted"

patterns-established:
  - "TextMate pattern: triple-quoted before double-quoted to avoid greedy terminal match"
  - "Grammar repository composition: strings, keywords, operators included by top-level patterns array"
  - "Extension stdio launch: { command: serverCommand, args: [], options: { shell: false } }"

requirements-completed: [EXT-01, EXT-02]

# Metrics
duration: 2min
completed: "2026-03-14"
---

# Phase 53 Plan 02: VS Code Extension Skeleton Summary

**TextMate grammar (source.writ) with 9 pattern groups and TypeScript extension launching writ-lsp over stdio via vscode-languageclient**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-14T00:25:26Z
- **Completed:** 2026-03-14T00:27:34Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- VS Code extension skeleton with package.json registering .writ file association (EXT-02) and TextMate grammar reference (EXT-01)
- TextMate grammar covers all major Writ constructs: comments, strings (3 forms), keywords (declaration/control/other/primitive), numbers (decimal/hex/binary), dialogue sigils (@SpeakerName, $ escape), attributes, operators, function calls, type names
- extension.ts launches writ-lsp Rust binary over stdio using LanguageClient with file system watcher for .writ files
- language-configuration.json provides bracket matching, auto-closing pairs, comment toggling for // and /* */

## Task Commits

Each task was committed atomically:

1. **Task 1: Create VS Code extension skeleton with language configuration** - `e46e4d9` (feat)
2. **Task 2: Create TextMate grammar for Writ syntax highlighting** - `86b5ea9` (feat)

## Files Created/Modified

- `writ-vscode/package.json` - Extension manifest: language contribution (.writ), grammar reference, vscode-languageclient dep, build scripts
- `writ-vscode/tsconfig.json` - TypeScript compilation to ES2020/commonjs, strict mode, outputs to out/
- `writ-vscode/src/extension.ts` - Activate: launches writ-lsp via stdio LanguageClient; deactivate: stops client
- `writ-vscode/language-configuration.json` - Bracket pairs, auto-close, surrounding pairs, comment markers, folding markers
- `writ-vscode/.vscodeignore` - Excludes .ts, tsconfig, node_modules, src from vsix package
- `writ-vscode/syntaxes/writ.tmLanguage.json` - TextMate grammar with 9 repository entries

## Decisions Made

- `activationEvents: []` is correct for VS Code 1.74+ — language contributions auto-activate the extension, no manual event registration needed
- Extension uses dev path to writ-lsp (`../target/debug/writ-lsp`) with `.exe` suffix on Windows; Phase 57 will bundle the binary
- Triple-quoted strings (`"""..."""`) are ordered first in the strings repository to prevent the regular double-quote pattern from greedily consuming the opening `"` of a triple-quote sequence
- Grammar includes `function-calls` (identifier before `(`) and `type-names` (PascalCase) as named patterns; semantic highlighting (Phase 54 DIFF-01) will refine further
- Dialogue `$` escape matched only before `{`, `if`, `match`, `choice` — not before `"` since `$"` is handled by the formattable string pattern

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required. Users will need to run `npm install` in `writ-vscode/` before building the extension, but this is standard development setup.

## Next Phase Readiness

- writ-vscode extension skeleton is complete and independent of the writ-lsp Rust crate
- Plan 53-03 (writ-lsp Rust crate) can now proceed — this extension will launch the binary it produces
- Grammar provides immediate syntax highlighting; semantic highlighting (Phase 54) layers on top

## Self-Check: PASSED

All files exist on disk. Commits e46e4d9 and 86b5ea9 confirmed in git log.

---
*Phase: 53-lsp-server-skeleton-and-diagnostics*
*Completed: 2026-03-14*
