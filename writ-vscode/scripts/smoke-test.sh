#!/usr/bin/env bash
# smoke-test.sh — verify VSIX contains required files without launching VS Code.
# Usage: bash writ-vscode/scripts/smoke-test.sh
set -euo pipefail

cd "$(dirname "$0")/.."

# Detect platform-specific binary extension
EXT=""
if [[ "$OSTYPE" == "msys"* ]] || [[ "$OSTYPE" == "cygwin"* ]] || [[ "${OS:-}" == "Windows_NT" ]]; then
    EXT=".exe"
fi

# Find the .vsix file (produced by npm run package)
VSIX=$(ls -1 *.vsix 2>/dev/null | head -1)
if [[ -z "$VSIX" ]]; then
    echo "ERROR: No .vsix file found. Run 'npm run package' first."
    exit 1
fi

echo "Testing: $VSIX"

FAIL=0
for f in "extension/out/extension.js" "extension/bin/writ-lsp${EXT}" "extension/bin/writ-dap${EXT}" "extension/package.json" "extension/syntaxes/writ.tmLanguage.json"; do
    if unzip -l "$VSIX" 2>/dev/null | grep -q "$f"; then
        echo "  OK: $f"
    else
        echo "  MISSING: $f"
        FAIL=1
    fi
done

# Verify debuggers contribution in package.json inside VSIX
TMPDIR=$(mktemp -d)
unzip -q -o "$VSIX" "extension/package.json" -d "$TMPDIR" 2>/dev/null
# On Windows (Git Bash / MSYS2), mktemp gives a POSIX path (/tmp/...) but Node.js
# needs the native Windows path. Use cygpath if available, otherwise fall back to
# the POSIX path (works on Linux/macOS directly).
if command -v cygpath >/dev/null 2>&1; then
    NODE_TMPDIR=$(cygpath -w "$TMPDIR")
else
    NODE_TMPDIR="$TMPDIR"
fi
# Forward-slash the path for Node.js require() safety
NODE_TMPDIR_FWD="${NODE_TMPDIR//\\//}"
if node -e "const p=require('${NODE_TMPDIR_FWD}/extension/package.json'); process.exit(p.contributes.debuggers && p.contributes.debuggers[0].type==='writ' ? 0 : 1)" 2>/dev/null; then
    echo "  OK: debuggers contribution type=writ"
else
    echo "  MISSING: debuggers contribution"
    FAIL=1
fi
rm -rf "$TMPDIR"

if [[ $FAIL -ne 0 ]]; then
    echo "SMOKE TEST FAILED"
    exit 1
fi

echo "Smoke test passed."
