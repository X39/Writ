#!/usr/bin/env bash
# Rebuild VS Code extension: compile TypeScript, build release binaries, copy to bin/, package VSIX
set -euo pipefail

cd "$(dirname "$0")/../writ-vscode"

echo "=== 1/4 Installing npm dependencies ==="
npm install

echo ""
echo "=== 2/4 Compiling TypeScript ==="
npm run compile

echo ""
echo "=== 3/4 Building Rust binaries (release) ==="
npm run build-binaries

echo ""
echo "=== 4/4 Copying binaries and packaging VSIX ==="
npm run copy-binaries
npx vsce package

echo ""
echo "=== Done ==="
VSIX=$(ls -t *.vsix 2>/dev/null | head -1)
if [ -n "$VSIX" ]; then
  echo "VSIX: writ-vscode/$VSIX ($(du -h "$VSIX" | cut -f1))"
  echo ""
  echo "Install with: code --install-extension writ-vscode/$VSIX"
fi
