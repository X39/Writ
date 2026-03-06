#!/usr/bin/env bash
# Rebuild all Writ Rust crates (compiler, runtime, LSP, DAP, assembler, CLI)
set -euo pipefail

cd "$(dirname "$0")/.."

echo "=== Building Writ workspace (debug) ==="
cargo build --workspace

echo ""
echo "=== Build complete ==="
echo "Binaries:"
ls -lh target/debug/writ-cli.exe target/debug/writ-lsp.exe target/debug/writ-dap.exe 2>/dev/null || \
ls -lh target/debug/writ-cli target/debug/writ-lsp target/debug/writ-dap 2>/dev/null
