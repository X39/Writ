# Phase 59: VSIX Release Build - Research

**Researched:** 2026-03-16
**Domain:** VS Code extension packaging (vsce / npm pipeline)
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Build target:**
- Build for current platform only (Windows x86_64) — this is v0.1.0, single-platform distribution
- Cross-compilation for other platforms is a future concern

**Build pipeline:**
- Use existing `npm run package` in writ-vscode/ which chains: compile → build-binaries → copy-binaries → vsce package
- `cargo build --release -p writ-lsp -p writ-dap` produces optimized binaries in target/release/
- `copy-bins.js` copies release binaries to writ-vscode/bin/
- `vsce package` bundles everything into a .vsix

**Artifact handling:**
- .vsix file lands in writ-vscode/ directory (standard vsce behavior)
- No need to copy it elsewhere — this is the canonical location

**Post-build verification:**
- Run smoke-test.sh as a separate step after packaging (not chained into npm run package)
- Smoke test validates: extension.js, writ-lsp binary, writ-dap binary, package.json, TextMate grammar, debuggers contribution
- Clearer diagnostics on failure when run separately

### Claude's Discretion
- Whether to fix any build warnings or errors encountered during cargo build --release
- Exact npm/node version requirements (use whatever is currently installed)
- Whether to add the .vsix path to .gitignore if not already excluded

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

---

## Summary

Phase 59 is an operational execution phase. All tooling was built in Phase 57: the npm scripts, copy-bins.js, smoke-test.sh, and .vscodeignore are complete and in-tree. This phase simply runs the pipeline and verifies the output produces a valid .vsix.

The current repository state is highly favorable: both `writ-lsp.exe` and `writ-dap.exe` are already present in `target/release/` (built 2026-03-16). The only prerequisite gap is that `node_modules/` does not exist in `writ-vscode/` — `npm install` must be run before `npm run package` will succeed. The `writ-vscode/out/` directory also does not exist yet (TypeScript has not been compiled), which is expected and handled by `npm run compile` inside the package script.

The .vsix exclusion is fully handled: `writ-vscode/.gitignore` already contains `*.vsix`, so the artifact will not be committed. The root `.gitignore` does not contain a `*.vsix` entry, but this is not a gap — the writ-vscode-scoped gitignore is sufficient.

**Primary recommendation:** Run `npm install` in `writ-vscode/`, then `npm run package`, then `bash scripts/smoke-test.sh`. Fix any issues encountered before declaring the phase complete.

---

## Standard Stack

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| `@vscode/vsce` | ^3.0.0 | Packages extension into .vsix | Official Microsoft VSIX packaging tool |
| `typescript` | ^5.0.0 | Compiles extension.ts to out/extension.js | Project TypeScript version |
| `cargo` | 1.93.0-nightly | Compiles Rust crates to release binaries | Workspace-level Rust toolchain |
| `node` | 25.5.0 (installed) | Runs npm scripts and copy-bins.js | Project Node.js version |

### Supporting
| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `vscode-languageclient` | ^9.0.0 | Runtime LSP client in extension | Already in dependencies |
| `copy-bins.js` | in-tree | Copies release binaries to bin/ | Part of `npm run copy-binaries` |
| `smoke-test.sh` | in-tree | Validates VSIX contents | Run after `npm run package` |

**Installation:**
```bash
cd writ-vscode && npm install
```

---

## Architecture Patterns

### Pipeline: npm run package
```
npm run compile          →  tsc -b  (src/ → out/extension.js)
npm run build-binaries   →  cargo build --release -p writ-lsp -p writ-dap
npm run copy-binaries    →  node scripts/copy-bins.js  (target/release/ → bin/)
npx vsce package         →  writ-0.1.0.vsix (in writ-vscode/)
```

### VSIX Contents (controlled by .vscodeignore)
```
extension/
├── out/extension.js          # compiled TypeScript
├── bin/writ-lsp.exe          # bundled LSP binary (Windows)
├── bin/writ-dap.exe          # bundled DAP binary (Windows)
├── package.json              # extension manifest
├── language-configuration.json
└── syntaxes/writ.tmLanguage.json
```

`.vscodeignore` excludes: `**/*.ts`, `**/tsconfig.json`, `node_modules/`, `src/`, `.vscode/`, `*.vsix`

### Post-build Smoke Test Pattern
```bash
bash writ-vscode/scripts/smoke-test.sh
```
The script:
1. Finds the .vsix via `ls -1 *.vsix | head -1` (run from `writ-vscode/`)
2. Checks each required path with `unzip -l "$VSIX" | grep -q "$f"`
3. Extracts `extension/package.json` to a tmpdir and validates `contributes.debuggers[0].type === 'writ'` via Node.js
4. Accumulates all failures before exiting — reports all missing items at once

### Anti-Patterns to Avoid
- **Running `cargo build --release` from `writ-vscode/`:** The npm script uses `--manifest-path ../Cargo.toml` so it works from within `writ-vscode/`. If running cargo manually, run from the workspace root.
- **Running smoke-test.sh from the wrong directory:** The script does `cd "$(dirname "$0")/.."` which normalizes to `writ-vscode/`. It must find `*.vsix` there.
- **Skipping `npm install`:** `node_modules/` is absent; `vsce` is a devDependency and will not be available without `npm install`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Binary path detection | Custom platform logic | `copy-bins.js` (already written) | Handles `.exe` suffix on Windows |
| VSIX content validation | Manual unzip inspection | `smoke-test.sh` (already written) | Checks all required paths + debuggers contribution |
| TypeScript compilation | Manual tsc invocation | `npm run compile` | Already wired into `npm run package` chain |

**Key insight:** All tooling is already implemented. This phase executes, not builds.

---

## Common Pitfalls

### Pitfall 1: node_modules Missing
**What goes wrong:** `npm run package` fails immediately because `vsce` is not found (`npx vsce package` falls back to remote download, which may fail or use wrong version).
**Why it happens:** `node_modules/` is in `.gitignore`; the directory is absent in a fresh checkout.
**How to avoid:** Run `npm install` from `writ-vscode/` before any other npm command.
**Warning signs:** `sh: vsce: command not found` or `Cannot find module '@vscode/vsce'`

### Pitfall 2: Stale Release Binaries
**What goes wrong:** `copy-bins.js` copies old binaries that don't include Phase 58 (dialogue speaker tokens) changes.
**Why it happens:** `target/release/` currently has binaries from 2026-03-16 which are up to date, but if cargo build is skipped the binaries won't reflect latest changes.
**How to avoid:** Use `npm run package` which always runs `npm run build-binaries` first. Do not run `npm run copy-binaries` in isolation unless cargo has just been run.
**Warning signs:** Binary timestamps older than latest Rust source changes.

### Pitfall 3: smoke-test.sh on Windows (OSTYPE detection)
**What goes wrong:** The script checks `$OSTYPE == "msys"*` or `$OSTYPE == "cygwin"*` or `$OS == "Windows_NT"` to decide whether to append `.exe`. Under Git Bash on Windows, `$OSTYPE` is `msys` and `$OS` is `Windows_NT`, so detection works. Under WSL2, neither matches — but this is a non-issue for the current build (Windows x86_64 target).
**How to avoid:** Run smoke-test.sh from Git Bash (not PowerShell, not CMD). The shebang `#!/usr/bin/env bash` requires bash.
**Warning signs:** Smoke test reports `MISSING: extension/bin/writ-lsp` but the file exists without `.exe` extension.

### Pitfall 4: .vsix publisher field
**What goes wrong:** `vsce package` may warn about `publisher` field if the publisher is not registered on the VS Code Marketplace. For local distribution this is fine.
**Why it happens:** `package.json` has `"publisher": "writ-lang"` which is not a registered Marketplace publisher.
**How to avoid:** For a local .vsix (v0.1.0, no Marketplace publish), pass `--no-rewrite-relative-links` or simply accept the warning. The .vsix is still produced.
**Warning signs:** Warning message from vsce about publisher; does NOT block .vsix production.

### Pitfall 5: vsce package.json validation
**What goes wrong:** `vsce package` validates that `package.json` has required fields: `name`, `version`, `publisher`, `engines.vscode`, `main`, `description`. Missing or invalid fields abort packaging.
**Why it happens:** vsce enforces a schema for marketplace-compliant packages.
**How to avoid:** All required fields are present in the current `package.json` — no action needed.
**Warning signs:** Error message from vsce citing a specific missing field.

---

## Code Examples

### Correct invocation sequence (from workspace root)
```bash
cd writ-vscode
npm install
npm run package
bash scripts/smoke-test.sh
```

### Manual steps if npm run package needs debugging
```bash
# Step 1: Compile TypeScript
cd writ-vscode && npx tsc -b

# Step 2: Build Rust release binaries (from workspace root)
cargo build --release -p writ-lsp -p writ-dap

# Step 3: Copy binaries to bin/
cd writ-vscode && node scripts/copy-bins.js

# Step 4: Package with vsce
cd writ-vscode && npx vsce package
```

### Expected .vsix filename
```
writ-0.1.0.vsix
```
Derived from `"name": "writ"` and `"version": "0.1.0"` in package.json.

### Verifying binary presence inside .vsix without smoke-test.sh
```bash
unzip -l writ-vscode/writ-0.1.0.vsix | grep "bin/"
# Expected:
#   extension/bin/writ-lsp.exe
#   extension/bin/writ-dap.exe
```

---

## Current State Assessment

| Item | Status | Notes |
|------|--------|-------|
| `target/release/writ-lsp.exe` | PRESENT (5.8 MB, 2026-03-16) | Ready for copy-bins.js |
| `target/release/writ-dap.exe` | PRESENT (7.6 MB, 2026-03-16) | Ready for copy-bins.js |
| `writ-vscode/node_modules/` | ABSENT | `npm install` required |
| `writ-vscode/out/` | ABSENT | Created by `npm run compile` |
| `writ-vscode/bin/` | EXISTS (empty) | Created by copy-bins.js |
| `writ-vscode/*.vsix` | ABSENT | Goal of this phase |
| `writ-vscode/.gitignore` contains `*.vsix` | YES | Git exclusion handled |

---

## Validation Architecture

> workflow.nyquist_validation is absent from config.json — treated as enabled.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | smoke-test.sh (bash script, in-tree) |
| Config file | none — standalone script |
| Quick run command | `bash writ-vscode/scripts/smoke-test.sh` |
| Full suite command | `bash writ-vscode/scripts/smoke-test.sh` |

### Phase Requirements → Test Map

This phase is operational (no formal REQ-IDs). The success criteria map to smoke test assertions:

| Success Criterion | Test Type | Automated Command | Notes |
|-------------------|-----------|-------------------|-------|
| `writ-lsp.exe` present in .vsix | smoke | `bash writ-vscode/scripts/smoke-test.sh` | Checks `extension/bin/writ-lsp.exe` |
| `writ-dap.exe` present in .vsix | smoke | `bash writ-vscode/scripts/smoke-test.sh` | Checks `extension/bin/writ-dap.exe` |
| `extension.js` present in .vsix | smoke | `bash writ-vscode/scripts/smoke-test.sh` | Checks `extension/out/extension.js` |
| `package.json` present in .vsix | smoke | `bash writ-vscode/scripts/smoke-test.sh` | Checks `extension/package.json` |
| TextMate grammar present | smoke | `bash writ-vscode/scripts/smoke-test.sh` | Checks `extension/syntaxes/writ.tmLanguage.json` |
| debuggers contribution type=writ | smoke | `bash writ-vscode/scripts/smoke-test.sh` | Extracts + validates via Node.js |

### Sampling Rate
- **Per task commit:** `bash writ-vscode/scripts/smoke-test.sh`
- **Per wave merge:** `bash writ-vscode/scripts/smoke-test.sh`
- **Phase gate:** Smoke test green before `/gsd:verify-work`

### Wave 0 Gaps
None — `smoke-test.sh` is already in-tree and covers all phase success criteria.

---

## Open Questions

1. **Cargo build warnings**
   - What we know: `cargo build --release` may emit warnings (clippy lints, deprecations, unused imports)
   - What's unclear: Whether any warnings will be treated as errors (`RUSTFLAGS=-D warnings` or `[profile.release] deny-warnings`)
   - Recommendation: Claude's discretion — fix warnings that are blockers; note non-blocking warnings in commit message

2. **npm install peer dependency warnings**
   - What we know: `vscode-languageclient ^9.0.0` with `@types/vscode ^1.74.0` may emit peer dependency warnings
   - What's unclear: Whether `npm install` will fail vs. warn
   - Recommendation: Use `npm install --legacy-peer-deps` if peer dependency errors block installation (LOW likelihood)

---

## Sources

### Primary (HIGH confidence)
- Direct file inspection: `writ-vscode/package.json` — npm scripts, dependencies, vsce version
- Direct file inspection: `writ-vscode/scripts/copy-bins.js` — binary copy logic
- Direct file inspection: `writ-vscode/scripts/smoke-test.sh` — validation logic
- Direct file inspection: `writ-vscode/.vscodeignore` — VSIX inclusion rules
- Direct file inspection: `writ-vscode/src/extension.ts` — extension activation code
- Direct file inspection: `writ-vscode/.gitignore` — git exclusion of *.vsix
- Direct file inspection: `D:/dev/git/Writ/.gitignore` — root git exclusions (no vsix entry)
- Bash inspection: `target/release/` — both binaries present and current
- Bash inspection: `writ-vscode/node_modules/` — absent, npm install required

### Secondary (MEDIUM confidence)
- `59-CONTEXT.md` — locked decisions from /gsd:discuss-phase

### Tertiary (LOW confidence)
- vsce publisher warning behavior — based on prior knowledge of vsce tooling; not verified against vsce 3.x changelog

---

## Metadata

**Confidence breakdown:**
- Pipeline mechanics: HIGH — all scripts read directly, behavior verified
- Current state: HIGH — filesystem inspected directly
- Pitfalls: HIGH (npm_modules/smoke-test), MEDIUM (vsce warnings)
- Validation: HIGH — smoke-test.sh is the test

**Research date:** 2026-03-16
**Valid until:** 2026-04-16 (stable tooling; vsce 3.x is not fast-moving)
