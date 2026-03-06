# Rebuild VS Code extension: compile TypeScript, build release binaries, copy to bin/, package VSIX
$ErrorActionPreference = "Stop"

Push-Location "$PSScriptRoot\..\writ-vscode"
try {
    Write-Host "=== 1/4 Installing npm dependencies ===" -ForegroundColor Cyan
    npm install
    if ($LASTEXITCODE -ne 0) { throw "npm install failed" }

    Write-Host ""
    Write-Host "=== 2/4 Compiling TypeScript ===" -ForegroundColor Cyan
    npm run compile
    if ($LASTEXITCODE -ne 0) { throw "tsc compile failed" }

    Write-Host ""
    Write-Host "=== 3/4 Building Rust binaries (release) ===" -ForegroundColor Cyan
    npm run build-binaries
    if ($LASTEXITCODE -ne 0) { throw "cargo release build failed" }

    Write-Host ""
    Write-Host "=== 4/4 Copying binaries and packaging VSIX ===" -ForegroundColor Cyan
    npm run copy-binaries
    if ($LASTEXITCODE -ne 0) { throw "copy-binaries failed" }
    npx vsce package
    if ($LASTEXITCODE -ne 0) { throw "vsce package failed" }

    Write-Host ""
    Write-Host "=== Done ===" -ForegroundColor Green
    $vsix = Get-ChildItem -Filter "*.vsix" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($vsix) {
        $size = [math]::Round($vsix.Length / 1MB, 2)
        Write-Host "VSIX: writ-vscode\$($vsix.Name) (${size} MB)"
        Write-Host ""
        Write-Host "Install with: code --install-extension writ-vscode\$($vsix.Name)"
    }
} finally {
    Pop-Location
    pause
}