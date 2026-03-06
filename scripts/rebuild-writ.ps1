# Rebuild all Writ Rust crates (compiler, runtime, LSP, DAP, assembler, CLI)
$ErrorActionPreference = "Stop"

Push-Location "$PSScriptRoot\.."
try {
    Write-Host "=== Building Writ workspace (debug) ===" -ForegroundColor Cyan
    cargo build -r --workspace
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    Write-Host ""
    Write-Host "=== Build complete ===" -ForegroundColor Green
    Write-Host "Binaries:"
    foreach ($bin in @("writ-cli", "writ-lsp", "writ-dap")) {
        $path = "target\debug\$bin.exe"
        if (Test-Path $path) {
            $size = [math]::Round((Get-Item $path).Length / 1MB, 1)
            Write-Host "  $path (${size} MB)"
        }
    }
} finally {
    Pop-Location
}
