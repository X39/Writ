---
phase: 59-vsix-release-build
plan: "01"
subsystem: infra
tags: [vsix, vsce, vscode-extension, typescript, cargo, release-build]

# Dependency graph
requires:
  - phase: 57-vs-code-extension-integration
    provides: copy-bins.js, smoke-test.sh, .vscodeignore, npm package scripts
  - phase: 58-dialogue-speaker-semantic-tokens
    provides: writ-lsp release binary with dialogue speaker token support
provides:
  - writ-0.1.0.vsix distributable VS Code extension package (4.12 MB)
  - writ-vscode/bin/writ-lsp.exe staged release binary
  - writ-vscode/bin/writ-dap.exe staged release binary
  - writ-vscode/out/extension.js compiled TypeScript entry point
affects: [release distribution, v5.0 milestone completion]

# Tech tracking
tech-stack:
  added: ["@types/node ^25.5.0 (devDependency — fixes tsc fs/path/process imports)", "package-lock.json"]
  patterns: [".vscodeignore excludes scripts/, tests/, .gitignore, tsconfig.tsbuildinfo from VSIX", "smoke-test.sh uses cygpath -w for Windows POSIX-to-native path conversion before Node.js require()"]

key-files:
  created: []
  modified:
    - writ-vscode/package.json
    - writ-vscode/package-lock.json
    - writ-vscode/.vscodeignore
    - writ-vscode/scripts/smoke-test.sh

key-decisions:
  - "@types/node added as devDependency: tsc could not resolve fs/path/process without it — necessary for extension.ts to compile"
  - ".vscodeignore extended to exclude scripts/, tests/, .gitignore, tsconfig.tsbuildinfo — these are dev artifacts not needed by the runtime extension"
  - "smoke-test.sh uses cygpath -w conversion: mktemp -d returns POSIX /tmp/... on Windows (Git Bash) but Node.js require() needs native Windows path"

patterns-established:
  - "Windows shell script path portability: always convert POSIX mktemp paths via cygpath -w before passing to Node.js"

requirements-completed: [VSIX-BUILD]

# Metrics
duration: 25min
completed: 2026-03-16
---

# Phase 59 Plan 01: VSIX Release Build Summary

**writ-0.1.0.vsix (4.12 MB) produced with release-mode writ-lsp.exe (7.4 MB) and writ-dap.exe (5.5 MB) bundled; all 6 smoke test checks green**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-03-16T23:00:00Z
- **Completed:** 2026-03-16T23:25:00Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Ran full VSIX packaging pipeline: npm install, cargo build --release, copy-bins.js, vsce package
- Produced writ-0.1.0.vsix (4.12 MB) containing compiled extension.js and both release-mode binaries
- Fixed 3 auto-detected issues (missing @types/node, incomplete .vscodeignore, Windows path bug in smoke-test.sh)
- All 6 smoke test checks pass: extension.js, writ-lsp.exe, writ-dap.exe, package.json, writ.tmLanguage.json, debuggers contribution

## Task Commits

Each task was committed atomically:

1. **Task 1: Install npm dependencies and run full VSIX packaging pipeline** - `7669804` (chore)
2. **Task 2: Run smoke test and verify VSIX contents** - `fa30c9a` (fix)

## Files Created/Modified
- `writ-vscode/package.json` - Added @types/node ^25.5.0 devDependency (auto-installed by npm, now explicit)
- `writ-vscode/package-lock.json` - Created by npm install (307 packages)
- `writ-vscode/.vscodeignore` - Extended to exclude scripts/, tests/, .gitignore, tsconfig.tsbuildinfo
- `writ-vscode/scripts/smoke-test.sh` - Fixed Windows POSIX-to-native path conversion for Node.js require()

## Decisions Made
- Added `@types/node` as an explicit devDependency. The extension source (`src/extension.ts`) uses `fs`, `path`, and `process` from Node.js. Without `@types/node`, tsc fails with "Cannot find module 'fs'" errors. This is a missing type declaration, not a runtime issue.
- Extended `.vscodeignore` to exclude `scripts/`, `tests/`, `.gitignore`, and `tsconfig.tsbuildinfo`. The initial ignore list only excluded `*.ts`, `tsconfig.json`, `node_modules/`, `src/`, `.vscode/`, and `*.vsix`. First packaging run included those dev artifacts in the VSIX.
- Fixed `smoke-test.sh` Windows path conversion. On Windows (Git Bash / MSYS2), `mktemp -d` returns a POSIX path (`/tmp/tmp.xxxxx`) that is invisible to Node.js, which requires the native Windows path (`C:\Users\...\AppData\Local\Temp\...`). Used `cygpath -w` with a Linux fallback.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Missing @types/node caused TypeScript compilation failure**
- **Found during:** Task 1 (npm run compile step)
- **Issue:** `src/extension.ts` imports `fs` and `path` and uses `process`, but `@types/node` was absent from devDependencies. tsc errors: "Cannot find module 'fs'", "Cannot find module 'path'", "Cannot find name 'process'."
- **Fix:** Ran `npm install --save-dev @types/node`; @types/node ^25.5.0 added to package.json devDependencies
- **Files modified:** `writ-vscode/package.json`, `writ-vscode/package-lock.json`
- **Verification:** `npm run compile` completed with exit code 0, `out/extension.js` produced
- **Committed in:** `7669804` (Task 1 commit)

**2. [Rule 2 - Missing Critical] .vscodeignore missing exclusions for dev artifacts**
- **Found during:** Task 1 (vsce package step — reviewed file list output)
- **Issue:** First `vsce package` run included `.gitignore`, `tsconfig.tsbuildinfo`, `scripts/copy-bins.js`, `scripts/smoke-test.sh`, `tests/structural.test.js` in the VSIX — development artifacts that should not be distributed
- **Fix:** Added `scripts/`, `tests/`, `.gitignore`, `**/tsconfig.tsbuildinfo` to `.vscodeignore`; reran `vsce package`
- **Files modified:** `writ-vscode/.vscodeignore`
- **Verification:** Second `vsce package` run shows exactly 8 files: `[Content_Types].xml`, `extension.vsixmanifest`, `extension/package.json`, `extension/language-configuration.json`, `extension/syntaxes/writ.tmLanguage.json`, `extension/bin/writ-lsp.exe`, `extension/bin/writ-dap.exe`, `extension/out/extension.js`
- **Committed in:** `7669804` (Task 1 commit)

**3. [Rule 1 - Bug] smoke-test.sh Windows path conversion failed for Node.js require()**
- **Found during:** Task 2 (smoke-test.sh run — 5/6 checks passed, debuggers contribution check failed)
- **Issue:** `mktemp -d` in Git Bash on Windows returns `/tmp/tmp.xxxxxx` (POSIX path), but Node.js on Windows resolves paths natively. `require('/tmp/tmp.xxxxxx/extension/package.json')` fails with `MODULE_NOT_FOUND`. The actual Windows path is `C:\Users\...\AppData\Local\Temp\tmp.xxxxxx`.
- **Fix:** Added `cygpath -w "$TMPDIR"` call to convert POSIX temp path to native Windows path, then forward-slash-normalized the result for `require()` safety. Added Linux/macOS fallback (skip cygpath if not available).
- **Files modified:** `writ-vscode/scripts/smoke-test.sh`
- **Verification:** `bash writ-vscode/scripts/smoke-test.sh` exits 0 with all 6 "OK:" lines including "OK: debuggers contribution type=writ"
- **Committed in:** `fa30c9a` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 blocking, 1 missing critical, 1 bug)
**Impact on plan:** All 3 fixes necessary for the pipeline to succeed and the smoke test to pass on Windows. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `writ-0.1.0.vsix` is ready for local installation via `code --install-extension writ-0.1.0.vsix`
- v5.0 milestone (LSP and DAP) is complete — all phases 52-59 delivered
- No blockers

---
*Phase: 59-vsix-release-build*
*Completed: 2026-03-16*

## Self-Check: PASSED

- FOUND: writ-vscode/writ-0.1.0.vsix
- FOUND: writ-vscode/out/extension.js
- FOUND: writ-vscode/bin/writ-lsp.exe
- FOUND: writ-vscode/bin/writ-dap.exe
- FOUND: .planning/phases/59-vsix-release-build/59-01-SUMMARY.md
- FOUND commit: 7669804 (Task 1)
- FOUND commit: fa30c9a (Task 2)
