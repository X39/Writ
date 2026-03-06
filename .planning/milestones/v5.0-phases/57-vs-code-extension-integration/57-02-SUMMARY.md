---
phase: 57-vs-code-extension-integration
plan: "02"
subsystem: infra
tags: [vscode, vsix, build-script, smoke-test, node-js, bash, binary-bundling]

# Dependency graph
requires:
  - phase: 57-01-vs-code-extension-integration
    provides: "package.json with copy-binaries/package scripts, .gitignore with bin/, .vscodeignore without bin/"
provides:
  - "scripts/copy-bins.js: copies writ-lsp and writ-dap from target/release to bin/ with platform .exe suffix and exit-1 on missing binary"
  - "scripts/smoke-test.sh: verifies VSIX contains extension.js, both server binaries, package.json, TextMate grammar, and debuggers contribution"
affects: [phase-57-packaging, CI-pipeline]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Node.js build script using fs.copyFileSync for cargo release binary bundling"
    - "Bash VSIX smoke test using unzip -l to inspect zip contents without VS Code"
    - "Platform detection in both scripts (process.platform === win32 / $OSTYPE)"

key-files:
  created:
    - writ-vscode/scripts/copy-bins.js
    - writ-vscode/scripts/smoke-test.sh
  modified: []

key-decisions:
  - "copy-bins.js exits with code 1 and names the missing binary — fail fast with actionable error rather than silently producing an incomplete bin/"
  - "smoke-test.sh accumulates all FAIL flags before exiting — reports all missing files at once for better diagnostics (not fail-fast per file)"
  - "smoke-test.sh extracts package.json from VSIX to a temp dir and validates debuggers contribution via Node.js — no external JSON tool dependency"

patterns-established:
  - "Pattern: Node.js build script with __dirname-relative paths for workspace-root-agnostic binary copy"
  - "Pattern: Bash VSIX inspection via unzip -l + grep for content verification without unpacking the full archive"

requirements-completed: [EXT-03]

# Metrics
duration: 3min
completed: 2026-03-16
---

# Phase 57 Plan 02: Build Tooling and Smoke Test Summary

**Node.js binary copy script and Bash VSIX smoke test completing the writ-vscode build pipeline: copy-bins.js stages cargo release binaries into bin/ and smoke-test.sh verifies the packaged VSIX contains all 5 required files plus a valid debuggers contribution.**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-16T15:00:00Z
- **Completed:** 2026-03-16T15:03:00Z
- **Tasks:** 1
- **Files modified:** 2 (created)

## Accomplishments
- Created `writ-vscode/scripts/copy-bins.js`: copies `writ-lsp` and `writ-dap` from `target/release/` to `writ-vscode/bin/`, handles Windows `.exe` suffix, exits with code 1 and a named error message if the source binary is missing
- Created `writ-vscode/scripts/smoke-test.sh`: checks 5 required VSIX entries (`extension/out/extension.js`, both server binaries, `extension/package.json`, `extension/syntaxes/writ.tmLanguage.json`) and validates `debuggers[0].type === 'writ'` via Node.js

## Task Commits

Each task was committed atomically:

1. **Task 1: Create binary copy script and smoke test** - `b77ce4a` (feat)

**Plan metadata:** (pending final docs commit)

## Files Created/Modified
- `writ-vscode/scripts/copy-bins.js` - Node.js script called by `npm run copy-binaries`; copies cargo release binaries to `bin/` with platform-appropriate `.exe` suffix; exits 1 if source not found
- `writ-vscode/scripts/smoke-test.sh` - Bash script that finds the `.vsix` in `writ-vscode/`, verifies 5 required file paths via `unzip -l`, extracts `package.json` to a temp dir, and validates the debuggers contribution type via Node.js

## Decisions Made
- `copy-bins.js` uses `process.exit(1)` with an explicit message naming the binary and the cargo command to run — actionable error, no silent partial copy
- `smoke-test.sh` accumulates a `FAIL` flag rather than calling `exit 1` per missing file — all missing entries are reported before the script exits, improving CI diagnostics
- `smoke-test.sh` validates `debuggers` contribution by extracting `package.json` from the VSIX and parsing it with Node.js inline — avoids `jq` dependency which may not be present on all CI environments

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- The full build pipeline is now complete: `npm run package` compiles TypeScript, builds Rust binaries, copies them to `bin/`, and packages the VSIX
- `bash writ-vscode/scripts/smoke-test.sh` validates the VSIX after `npm run package` (requires `unzip` on PATH)
- Phase 57 is the final phase of the v5.0 milestone; no further phases depend on this

---
*Phase: 57-vs-code-extension-integration*
*Completed: 2026-03-16*
