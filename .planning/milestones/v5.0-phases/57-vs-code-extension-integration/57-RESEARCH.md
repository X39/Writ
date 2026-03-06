# Phase 57: VS Code Extension Integration - Research

**Researched:** 2026-03-16
**Domain:** VS Code extension packaging, DAP client integration, binary bundling
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Binary bundling strategy:**
- Binaries placed in `writ-vscode/bin/` directory (writ-lsp and writ-dap executables)
- Extension resolves bundled binaries via `context.asAbsolutePath(path.join('bin', 'writ-lsp'))` replacing the current dev path (`../target/debug/writ-lsp`)
- Platform-specific: build script compiles for the target platform and copies binaries to `bin/` before `vsce package`
- `.vscodeignore` ensures only `bin/`, `out/`, `syntaxes/`, and config files are included in the VSIX (no `src/`, `node_modules/`, etc.)
- `bin/` directory added to `.gitignore` — binaries are build artifacts, not checked in

**DAP client integration:**
- Register a `DebugAdapterDescriptorFactory` for debug type `"writ"` — standard VS Code pattern for stdio-based debug adapters
- Factory returns `DebugAdapterExecutable` pointing to the bundled `writ-dap` binary path
- Extension contributes `debuggers` in package.json with type `"writ"`, label `"Writ Debug"`, and `configurationAttributes` for launch requests
- DAP server started from the same bundled `bin/` directory as the LSP server

**launch.json snippet design:**
- Debug type: `"writ"`, request: `"launch"`
- Required field: `"program"` — path to the .writ entry file
- Snippet uses `"${workspaceFolder}/${file}"` as the default program value so the current file is pre-filled
- Snippet name: `"Writ: Launch Current File"`
- Configuration attributes define `program` as required string with description
- No other required fields — cwd defaults to workspace folder, no args needed for basic use

### Claude's Discretion
- Build/package script implementation details (shell script, npm script, or Makefile)
- `.vscodeignore` exact contents beyond the essentials listed above
- Whether to add a second snippet for "Launch with Arguments"
- End-to-end smoke test strategy and framework choice
- Error messages when bundled binary is not found (fallback behavior)
- Whether to support a `writ.serverPath` setting for dev override (replaces current hardcoded dev path)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| EXT-03 | Extension bundles and launches writ-lsp and writ-dap server binaries | Binary bundling via `bin/` dir, `.vscodeignore` exclusions, `context.asAbsolutePath` resolution, `DebugAdapterDescriptorFactory` registration pattern |
| EXT-04 | Extension provides default launch.json configuration snippet for Writ debugging | `contributes.debuggers.configurationSnippets` in package.json, static snippet body with tabstop syntax |
</phase_requirements>

---

## Summary

Phase 57 wires the two already-implemented server binaries (writ-lsp, writ-dap) into the VS Code extension for out-of-the-box packaging. The LSP side needs its binary path updated from the dev-relative `../target/debug/writ-lsp` to a bundled path via `context.asAbsolutePath('bin/writ-lsp')`. The DAP side is new: a `DebugAdapterDescriptorFactory` must be registered so VS Code can start writ-dap over stdio for debug sessions. Both are well-established, stable VS Code patterns with extensive official documentation.

The `package.json` gains a `contributes.debuggers` section that simultaneously (a) registers the `"writ"` debug type, (b) declares `configurationAttributes` for the launch config schema, and (c) provides `configurationSnippets` for IntelliSense in launch.json. Snippets are static — they cannot be generated dynamically from a `DebugConfigurationProvider`; they must be declared in package.json.

The main non-obvious concern is the Windows executable bit: building and packaging the VSIX on Windows strips POSIX executable bits from bundled binaries. Since this project targets Windows as the primary dev environment, the binary will work on Windows (no executable bit needed) but VSIX files built on Windows will not work on Linux/macOS. For a dev/local VSIX this is acceptable; a cross-platform CI build would need a Linux runner.

**Primary recommendation:** Follow the `DebugAdapterExecutable` factory pattern exactly as documented. Keep the extension simple: update the LSP binary path, add the DAP factory, update package.json. Use an npm script for the build/copy step.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `vscode` (types) | `^1.74.0` | Extension host API | Already declared; activationEvents: [] requires 1.74+ |
| `vscode-languageclient` | `^9.0.0` | LSP client | Already declared; no changes needed |
| `@types/vscode` | `^1.74.0` | TypeScript type defs | Already declared |
| `vsce` (CLI) | latest | Package extension as .vsix | Official Microsoft packaging tool |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@vscode/test-cli` | latest | Run integration tests | If smoke tests run inside VS Code process |
| `@vscode/test-electron` | latest | VS Code download for tests | Required by @vscode/test-cli for desktop tests |

**Note:** `@vscode/test-cli` and `@vscode/test-electron` are only needed if the end-to-end smoke test activates a real VS Code instance. A simpler smoke test (shell script: build VSIX, verify binary exists, check package.json fields) avoids the dependency entirely.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Shell/npm build script | Makefile | npm scripts are simpler and don't require Make on Windows |
| `@vscode/test-cli` integration test | Shell smoke test | Shell test is faster, no VS Code download needed, sufficient for CI |

**Installation (no new runtime deps):**
```bash
# Dev tooling only — install globally or as devDependency
npm install -D @vscode/vsce
```

---

## Architecture Patterns

### Recommended Project Structure

```
writ-vscode/
├── bin/                     # Bundled binaries (gitignored, built by build script)
│   ├── writ-lsp             # LSP server binary (writ-lsp.exe on Windows)
│   └── writ-dap             # DAP server binary (writ-dap.exe on Windows)
├── out/                     # Compiled TypeScript (gitignored)
│   └── extension.js
├── src/
│   └── extension.ts         # Extension entry point — MODIFIED this phase
├── syntaxes/
│   └── writ.tmLanguage.json # TextMate grammar — no changes
├── .vscodeignore            # MODIFIED this phase
├── .gitignore               # MODIFIED this phase (add bin/)
├── language-configuration.json
├── package.json             # MODIFIED this phase
└── tsconfig.json
```

### Pattern 1: DebugAdapterDescriptorFactory (DAP binary launch)

**What:** Factory registered with `vscode.debug.registerDebugAdapterDescriptorFactory` tells VS Code how to start the debug adapter process when F5 is pressed.

**When to use:** Any extension with a stdio-based debug adapter binary.

**Example:**
```typescript
// Source: https://code.visualstudio.com/api/extension-guides/debugger-extension
import * as path from 'path';
import * as vscode from 'vscode';

class WritDebugAdapterDescriptorFactory
    implements vscode.DebugAdapterDescriptorFactory {

    constructor(private context: vscode.ExtensionContext) {}

    createDebugAdapterDescriptor(
        _session: vscode.DebugSession
    ): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        let binaryName = 'writ-dap';
        if (process.platform === 'win32') {
            binaryName += '.exe';
        }
        const binaryPath = this.context.asAbsolutePath(
            path.join('bin', binaryName)
        );
        return new vscode.DebugAdapterExecutable(binaryPath, []);
    }
}

// In activate():
const factory = new WritDebugAdapterDescriptorFactory(context);
context.subscriptions.push(
    vscode.debug.registerDebugAdapterDescriptorFactory('writ', factory)
);
```

### Pattern 2: Bundled binary path resolution (LSP update)

**What:** Replace the dev-time relative path with `context.asAbsolutePath` pointing into `bin/`.

**Example:**
```typescript
// Before (Phase 53 dev path — REPLACE):
// context.asAbsolutePath(path.join('..', 'target', 'debug', 'writ-lsp'))

// After (Phase 57 bundled path):
let binaryName = 'writ-lsp';
if (process.platform === 'win32') {
    binaryName += '.exe';
}
const serverCommand = context.asAbsolutePath(path.join('bin', binaryName));
```

### Pattern 3: Optional writ.serverPath dev override

**What:** Read a user-overridable configuration setting before falling back to the bundled path. Useful for extension developers who want to test with a locally built binary.

**When to use:** When `writ.serverPath` is non-empty in workspace settings, use that path; otherwise use bundled binary.

**Example:**
```typescript
// Source: vscode.workspace.getConfiguration pattern
const config = vscode.workspace.getConfiguration('writ');
const serverPathOverride = config.get<string>('serverPath', '');
const serverCommand = serverPathOverride
    ? serverPathOverride
    : context.asAbsolutePath(path.join('bin', binaryName));
```

### Pattern 4: configurationSnippets in package.json

**What:** Static JSON object in `contributes.debuggers` that appears in IntelliSense when editing launch.json. The `body` uses VS Code snippet syntax (tabstops `${1:...}`, escaped variable references `^"\${workspaceFolder}/..."` ).

**Key constraint:** Snippets MUST be declared statically in package.json. A `DebugConfigurationProvider` cannot provide snippets dynamically.

**Note on snippet body escaping:** In the `body` object, variable references like `${workspaceFolder}` must be escaped as `^"\${workspaceFolder}"` to prevent JSON expansion and produce a literal `${workspaceFolder}` in the generated snippet. The `^"` prefix is a vsce/VS Code snippet body escaping convention for string values.

**Example:**
```json
// Source: https://code.visualstudio.com/api/extension-guides/debugger-extension
"contributes": {
  "debuggers": [
    {
      "type": "writ",
      "label": "Writ Debug",
      "languages": ["writ"],
      "configurationAttributes": {
        "launch": {
          "required": ["program"],
          "properties": {
            "program": {
              "type": "string",
              "description": "Absolute path to the .writ entry file to debug."
            }
          }
        }
      },
      "initialConfigurations": [
        {
          "type": "writ",
          "request": "launch",
          "name": "Launch Writ Program",
          "program": "${workspaceFolder}/${file}"
        }
      ],
      "configurationSnippets": [
        {
          "label": "Writ: Launch Current File",
          "description": "Launch the current .writ file in the Writ debugger.",
          "body": {
            "type": "writ",
            "request": "launch",
            "name": "${1:Launch Writ Program}",
            "program": "^\"\\${workspaceFolder}/\\${file}\""
          }
        }
      ]
    }
  ]
}
```

### Pattern 5: .vscodeignore for binary inclusion

**What:** `.vscodeignore` specifies what to EXCLUDE from the VSIX. Files not mentioned are included. The `bin/` directory must NOT be in `.vscodeignore` so binaries are bundled.

**Current .vscodeignore (from code):**
```
**/*.ts
**/tsconfig.json
node_modules/
src/
.vscode/
```

**Updated .vscodeignore for Phase 57 — no bin/ exclusion:**
```
**/*.ts
**/tsconfig.json
node_modules/
src/
.vscode/
*.vsix
```

The `bin/` directory is intentionally absent from `.vscodeignore` so its contents are included in the VSIX. The `out/` directory should also NOT be excluded (compiled JS must be in VSIX).

### Pattern 6: Binary not found — error handling

**What:** Check that the binary exists before starting; show a user-visible error if not.

**Example:**
```typescript
import * as fs from 'fs';

// In activate(), before starting client:
if (!fs.existsSync(serverCommand)) {
    vscode.window.showErrorMessage(
        `Writ: bundled language server not found at ${serverCommand}. ` +
        `Rebuild the extension with the build script.`
    );
    return;
}
```

### Anti-Patterns to Avoid

- **Committing binaries to git:** `bin/` must be in `.gitignore` — binaries are build artifacts, not source.
- **Using `program` field in `contributes.debuggers`:** The `program` field (path to a JS debug adapter) is for Node.js-based adapters. For native binary adapters, use `DebugAdapterDescriptorFactory` instead — leave `program` out of the `debuggers` entry.
- **Calling `vsce package` before compiling TypeScript:** Always run `tsc -b` then copy binaries to `bin/` then run `vsce package`. Order matters.
- **Dynamic configurationSnippets:** Attempting to provide snippets via `DebugConfigurationProvider.provideDebugConfigurations` — these populate the launch.json picker for "Add Configuration" but are NOT the same as IntelliSense snippets. IntelliSense snippets are package.json only.
- **Packaging from Windows for cross-platform:** VSIX built on Windows strips POSIX executable bits from bundled binaries. For a local dev VSIX targeting Windows only, this is acceptable. For marketplace publishing across platforms, use a Linux/macOS CI runner or platform-specific VSIX targets (`vsce package --target win32-x64`).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Launch.json IntelliSense snippets | Custom DebugConfigurationProvider | `configurationSnippets` in package.json | Only static declaration in package.json produces IntelliSense snippets; provider only populates "Add Configuration" picker |
| VSIX packaging | Custom zip/archive | `vsce package` | vsce handles VSIX manifest (extension.vsixmanifest), content-types, publisher validation, and all packaging conventions |
| Binary path resolution | Manual `__dirname` + relative paths | `context.asAbsolutePath()` | `context.asAbsolutePath` is the correct API for resolving paths relative to the extension's installation directory, which differs from `__dirname` in production |

**Key insight:** VS Code extension packaging is opinionated. The `vsce` tool, `context.asAbsolutePath`, and the static `configurationSnippets` all have specific behaviors that custom solutions break. Use the standard APIs.

---

## Common Pitfalls

### Pitfall 1: Incorrect snippet body escaping
**What goes wrong:** `${workspaceFolder}` in the snippet body expands at JSON parse time instead of being passed as a literal VS Code variable reference, producing `undefined` or empty string in the generated launch.json.
**Why it happens:** VS Code snippet bodies in package.json use a special escaping convention: string values that should contain literal `${...}` must be prefixed with `^"` and the `$` must be escaped as `\$`.
**How to avoid:** Use `"^\"\\${workspaceFolder}/\\${file}\""` in the JSON body value, OR test the snippet by opening a launch.json and invoking IntelliSense to verify the generated configuration.
**Warning signs:** The generated launch.json has `"program": ""` or `"program": "undefined"` instead of `"program": "${workspaceFolder}/${file}"`.

### Pitfall 2: Binary not executable on Unix
**What goes wrong:** Extension installs fine on Linux/macOS but the language server fails to start with "permission denied".
**Why it happens:** Building the VSIX on Windows strips the Unix executable bit from `bin/writ-lsp` and `bin/writ-dap`. The VSIX format (a ZIP variant) does not preserve POSIX file attributes when created on Windows.
**How to avoid:** For local Windows-only dev use, this is not a concern. If targeting Linux/macOS or the marketplace, build the VSIX on Linux (via CI) or use `vsce package --target win32-x64` to publish platform-specific VSIXs.
**Warning signs:** `EACCES: permission denied` in VS Code Output panel for the Writ extension.

### Pitfall 3: Using `program` field in debuggers contribution for native binary
**What goes wrong:** Setting `"program": "./bin/writ-dap"` in `contributes.debuggers` causes VS Code to attempt to run the binary through Node.js (`node ./bin/writ-dap`), not as a native executable.
**Why it happens:** The `program` field in `contributes.debuggers` is designed for Node.js-based debug adapters. VS Code runs it with `node <program>`.
**How to avoid:** Omit `program` from the `debuggers` contribution. Use `DebugAdapterDescriptorFactory` returning a `DebugAdapterExecutable` with the full binary path. This is the correct pattern for native binaries.
**Warning signs:** DAP server crashes immediately on launch; error logs mention "node" trying to execute a binary.

### Pitfall 4: out/ not in VSIX
**What goes wrong:** Compiled `out/extension.js` is missing from the VSIX, causing the extension to fail with "Cannot find module './out/extension.js'".
**Why it happens:** If `out/` is accidentally added to `.vscodeignore`, the compiled JavaScript is excluded.
**How to avoid:** Do not add `out/` to `.vscodeignore`. Verify with `vsce ls` (lists all files that will be included in the VSIX) before packaging.
**Warning signs:** Extension fails to activate; VS Code shows "Extension host terminated unexpectedly".

### Pitfall 5: build order — compile before copy, copy before package
**What goes wrong:** Old or missing `out/extension.js` in VSIX because packaging ran before TypeScript compilation.
**Why it happens:** The build script must: (1) `tsc -b`, (2) `cargo build --release`, (3) copy binaries to `bin/`, (4) `vsce package`. Any reordering breaks the VSIX.
**How to avoid:** Encode the build order in a single npm script or shell script that runs all steps in sequence.

---

## Code Examples

Verified patterns from official sources:

### Full updated extension.ts structure

```typescript
// Source: pattern from https://code.visualstudio.com/api/extension-guides/debugger-extension
import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient;

function getBinaryPath(
    context: vscode.ExtensionContext,
    name: string
): string {
    const binaryName = process.platform === 'win32' ? `${name}.exe` : name;
    // Check for dev override via writ.serverPath (Claude's discretion)
    const config = vscode.workspace.getConfiguration('writ');
    const override = config.get<string>('serverPath', '');
    if (override) {
        return path.join(override, binaryName);
    }
    return context.asAbsolutePath(path.join('bin', binaryName));
}

class WritDebugAdapterDescriptorFactory
    implements vscode.DebugAdapterDescriptorFactory {

    constructor(private context: vscode.ExtensionContext) {}

    createDebugAdapterDescriptor(
        _session: vscode.DebugSession
    ): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        const binaryPath = getBinaryPath(this.context, 'writ-dap');
        return new vscode.DebugAdapterExecutable(binaryPath, []);
    }
}

export function activate(context: vscode.ExtensionContext): void {
    // --- LSP: updated to bundled binary path ---
    const serverCommand = getBinaryPath(context, 'writ-lsp');

    if (!fs.existsSync(serverCommand)) {
        vscode.window.showErrorMessage(
            `Writ: language server binary not found at "${serverCommand}". ` +
            `Run the build script to bundle the binaries.`
        );
        return;
    }

    const serverOptions: ServerOptions = {
        command: serverCommand,
        args: [],
        options: { shell: false },
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'writ' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.writ'),
        },
    };

    client = new LanguageClient(
        'writ',
        'Writ Language Server',
        serverOptions,
        clientOptions
    );
    client.start();

    // --- DAP: register debug adapter descriptor factory ---
    const factory = new WritDebugAdapterDescriptorFactory(context);
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterDescriptorFactory('writ', factory)
    );
}

export function deactivate(): Thenable<void> | undefined {
    return client?.stop();
}
```

### contributes.debuggers in package.json

```json
"contributes": {
  "languages": [ /* existing, no change */ ],
  "grammars": [ /* existing, no change */ ],
  "semanticTokenScopes": [ /* existing, no change */ ],
  "debuggers": [
    {
      "type": "writ",
      "label": "Writ Debug",
      "languages": ["writ"],
      "configurationAttributes": {
        "launch": {
          "required": ["program"],
          "properties": {
            "program": {
              "type": "string",
              "description": "Absolute path to the .writ file to debug."
            }
          }
        }
      },
      "initialConfigurations": [
        {
          "type": "writ",
          "request": "launch",
          "name": "Launch Writ Program",
          "program": "${workspaceFolder}/${file}"
        }
      ],
      "configurationSnippets": [
        {
          "label": "Writ: Launch Current File",
          "description": "Launch the current .writ file in the Writ debugger.",
          "body": {
            "type": "writ",
            "request": "launch",
            "name": "${1:Launch Writ Program}",
            "program": "^\"\\${workspaceFolder}/\\${file}\""
          }
        }
      ]
    }
  ]
}
```

### Build script (npm script in package.json)

```json
"scripts": {
  "compile": "tsc -b",
  "watch": "tsc -b -w",
  "build-binaries": "cargo build --release --manifest-path ../Cargo.toml -p writ-lsp -p writ-dap",
  "copy-binaries": "node scripts/copy-bins.js",
  "package": "npm run compile && npm run build-binaries && npm run copy-binaries && vsce package"
}
```

### copy-bins.js (Node.js build script)

```javascript
// scripts/copy-bins.js — run after cargo build --release
const fs = require('fs');
const path = require('path');

const isWin = process.platform === 'win32';
const ext = isWin ? '.exe' : '';
const targetDir = path.join(__dirname, '..', '..', 'target', 'release');
const binDir = path.join(__dirname, '..', 'bin');

fs.mkdirSync(binDir, { recursive: true });

for (const name of ['writ-lsp', 'writ-dap']) {
    const src = path.join(targetDir, name + ext);
    const dst = path.join(binDir, name + ext);
    fs.copyFileSync(src, dst);
    console.log(`Copied ${src} -> ${dst}`);
}
```

### writ.serverPath configuration contribution

```json
// In package.json contributes.configuration (if dev override is added)
"configuration": {
  "title": "Writ",
  "properties": {
    "writ.serverPath": {
      "type": "string",
      "default": "",
      "description": "Path to directory containing writ-lsp and writ-dap binaries. Leave empty to use bundled binaries."
    }
  }
}
```

### Smoke test — shell script approach

```bash
#!/usr/bin/env bash
# smoke-test.sh — verify VSIX contents without launching VS Code
set -euo pipefail

# 1. Run the full build
cd "$(dirname "$0")/.."
npm run package

# 2. Find the produced .vsix
VSIX=$(ls *.vsix | head -1)
echo "Testing: $VSIX"

# 3. Verify required files exist in VSIX (it's a zip)
EXT=".exe"
if [[ "$OSTYPE" != "msys"* ]] && [[ "$OSTYPE" != "cygwin"* ]] && [[ "$(uname)" != *MINGW* ]]; then
    EXT=""
fi

for f in "extension/out/extension.js" "extension/bin/writ-lsp${EXT}" "extension/bin/writ-dap${EXT}"; do
    if unzip -l "$VSIX" | grep -q "$f"; then
        echo "OK: $f"
    else
        echo "MISSING: $f"
        exit 1
    fi
done

echo "Smoke test passed."
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `adapterExecutableCommand` contribution point | `DebugAdapterDescriptorFactory` API | VS Code 1.37 (2019) | Factory is stable and the only supported way for native binaries; old command approach is removed |
| `activationEvents: ["onDebug"]` | `activationEvents: []` + language contributions | VS Code 1.74 (2022) | Already in use in this extension; no change needed |
| `program` field in `contributes.debuggers` | Omit `program` + use factory | Recommended since 1.37 | `program` field is for Node.js adapters only; native binaries use factory |

**Deprecated/outdated:**
- `adapterExecutableCommand`: Removed from VS Code. Use `DebugAdapterDescriptorFactory`.
- `vscode-debugadapter` npm package: Superseded by the `@vscode/debugadapter` package for implementing adapters in TypeScript. Not relevant here since writ-dap is Rust.

---

## Open Questions

1. **Snippet body variable escaping — exact production behavior**
   - What we know: The `^"` prefix and `\$` escaping convention is documented in the Mock Debug sample and the debugger extension guide.
   - What's unclear: Whether VS Code 1.74+ changed the escaping rules in any way.
   - Recommendation: After implementing, manually invoke the snippet in a launch.json file and verify `"program"` is set to `"${workspaceFolder}/${file}"` literally. If the output is wrong, adjust escaping.

2. **writ.serverPath as Claude's discretion**
   - What we know: `workspace.getConfiguration` pattern is standard; the decision of whether to add it is discretionary.
   - What's unclear: Whether the current extension architecture (Phase 53's dev path hardcoded) already causes friction for contributors.
   - Recommendation: Add `writ.serverPath` as a configuration property. It's a small addition that makes the extension useful for both bundled (production) and local build (dev) workflows, and removes the currently hardcoded `../target/debug` path permanently.

3. **Smoke test scope**
   - What we know: `@vscode/test-cli` + `@vscode/test-electron` can run real VS Code integration tests; shell-based VSIX inspection is simpler.
   - What's unclear: Whether the phase requires actually launching VS Code or just verifying VSIX contents.
   - Recommendation: Use the shell-based smoke test (verify VSIX contains required files). This is fast, has no network dependency, and satisfies "validates the integration" for the purpose of this phase. Real VS Code launch tests are overkill for a packaging phase.

---

## Validation Architecture

> `workflow.nyquist_validation` is absent from `.planning/config.json` — treated as enabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Shell script + `unzip` (smoke test) / manual VS Code integration |
| Config file | `writ-vscode/scripts/smoke-test.sh` (Wave 0 gap — does not exist yet) |
| Quick run command | `bash writ-vscode/scripts/smoke-test.sh` |
| Full suite command | `bash writ-vscode/scripts/smoke-test.sh` |

**Note:** The Rust crates (writ-lsp, writ-dap) already have their own test suites runnable with `cargo test`. This phase's tests are about packaging and wiring, not logic. Shell-based VSIX inspection is the appropriate test medium.

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| EXT-03 | writ-lsp binary bundled in VSIX | smoke | `bash writ-vscode/scripts/smoke-test.sh` | ❌ Wave 0 |
| EXT-03 | writ-dap binary bundled in VSIX | smoke | `bash writ-vscode/scripts/smoke-test.sh` | ❌ Wave 0 |
| EXT-03 | Extension activates without error on .writ file | manual | Open .writ file in VS Code, check Output panel | N/A |
| EXT-04 | launch.json snippet appears in IntelliSense | manual | Edit launch.json in VS Code, trigger IntelliSense | N/A |
| EXT-04 | Snippet inserts valid configuration | manual | Verify generated launch.json `program` field | N/A |

### Sampling Rate

- **Per task commit:** `bash writ-vscode/scripts/smoke-test.sh`
- **Per wave merge:** `bash writ-vscode/scripts/smoke-test.sh`
- **Phase gate:** Smoke test green + manual VS Code activation verification before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `writ-vscode/scripts/smoke-test.sh` — covers EXT-03 (binary bundling verification)
- [ ] `writ-vscode/scripts/copy-bins.js` — build artifact copy script (needed before smoke test can run)

---

## Sources

### Primary (HIGH confidence)
- [VS Code Debugger Extension API](https://code.visualstudio.com/api/extension-guides/debugger-extension) — DebugAdapterDescriptorFactory pattern, configurationSnippets structure, DebugAdapterExecutable usage
- [VS Code Contribution Points — contributes.debuggers](https://code.visualstudio.com/api/references/contribution-points) — Full schema for debuggers section including configurationAttributes, initialConfigurations, configurationSnippets
- [VS Code Publishing Extension](https://code.visualstudio.com/api/working-with-extensions/publishing-extension) — .vscodeignore syntax, binary inclusion rules, Windows executable bit warning
- Existing `writ-vscode/src/extension.ts` — established platform detection pattern (`process.platform === 'win32'`), `context.asAbsolutePath` pattern referenced in Phase 53 decisions

### Secondary (MEDIUM confidence)
- [vscode-docs/debugger-extension.md on GitHub](https://github.com/Microsoft/vscode-docs/blob/main/api/extension-guides/debugger-extension.md) — configurationSnippets body escaping convention verified in mock debug sample reference
- [VS Code platform-specific extension sample](https://github.com/microsoft/vscode-platform-specific-sample) — binary bundling in VSIX for platform-specific builds
- [VS Code Testing Extensions](https://code.visualstudio.com/api/working-with-extensions/testing-extension) — @vscode/test-cli alternative; shell smoke test approach

### Tertiary (LOW confidence)
- Web search results on DebugAdapterExecutable constructor signature — verified against official API docs; HIGH confidence on the three parameters (command, args, env)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, existing packages well-understood
- Architecture: HIGH — DebugAdapterDescriptorFactory is the stable, documented API since VS Code 1.37; verified against official docs
- Pitfalls: HIGH — Windows executable bit and snippet body escaping are documented in official sources; DAP `program` field misuse is documented anti-pattern
- Smoke test approach: MEDIUM — shell VSIX inspection is pragmatic but manual VS Code activation test is ultimately required for full confidence

**Research date:** 2026-03-16
**Valid until:** 2026-09-16 (stable APIs; VS Code extension API for DAP has been stable since 1.37)
