param(
    [int]$Runs = 15,
    [int]$Warmup = 5,
    [string]$ContainerCmd = ""
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$Date = Get-Date -Format "yyyy-MM-dd"
$ResultsDir = Join-Path $RepoRoot "benchmark\results\$Date"

New-Item -ItemType Directory -Force -Path $ResultsDir | Out-Null

# Detect Docker or Podman
if ($ContainerCmd -eq "") {
    if (Get-Command docker -ErrorAction SilentlyContinue) {
        $ContainerCmd = "docker"
    }
    elseif (Get-Command podman -ErrorAction SilentlyContinue) {
        $ContainerCmd = "podman"
    }
    else {
        Write-Error "Neither docker nor podman found in PATH. Install Docker: https://docs.docker.com/get-docker/"
        exit 1
    }
}

Write-Host "Building benchmark image..."
& $ContainerCmd build -t writ-bench -f "$ScriptDir\Dockerfile" $RepoRoot

# Docker Desktop on Windows handles Windows paths natively in volume mounts.
# Convert backslashes to forward slashes for consistency, but keep the drive letter.
$ResultsFwd = $ResultsDir -replace '\\', '/'

Write-Host "Running benchmarks (Runs=$Runs, Warmup=$Warmup)..."
& $ContainerCmd run --rm `
    -v "${ResultsFwd}:/results" `
    -e "RESULTS_DIR=/results" `
    -e "RUNS=$Runs" `
    -e "WARMUP=$Warmup" `
    writ-bench

Write-Host ""

# Generate charts and markdown table
if (Get-Command python3 -ErrorAction SilentlyContinue) {
    Write-Host "Generating charts..."
    & python3 "$RepoRoot\benchmark\generate.py" "$ResultsDir\raw.json"
    Write-Host "Done. Results: $ResultsDir\"
} else {
    Write-Warning "python3 not found - charts not generated"
    Write-Warning "  Run manually: python3 benchmark\generate.py $ResultsDir\raw.json"
    Write-Warning "  Requires: pip install pygal==3.1.0"
    Write-Host "Done. Results: $ResultsDir\raw.json"
}
