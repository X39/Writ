#!/bin/sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DATE=$(date +%Y-%m-%d)
RESULTS_DIR="$REPO_ROOT/benchmark/results/$DATE"
RUNS="${RUNS:-15}"
WARMUP="${WARMUP:-5}"

mkdir -p "$RESULTS_DIR"

# Detect Docker or Podman (prefer whichever is actually responsive)
CONTAINER_CMD=""
if command -v docker > /dev/null 2>&1 && docker version > /dev/null 2>&1; then
    CONTAINER_CMD=docker
elif command -v podman > /dev/null 2>&1; then
    CONTAINER_CMD=podman
elif command -v docker > /dev/null 2>&1; then
    CONTAINER_CMD=docker
else
    echo "error: neither docker nor podman found in PATH" >&2
    echo "Install Docker: https://docs.docker.com/get-docker/" >&2
    exit 1
fi

echo "Building benchmark image..."
"$CONTAINER_CMD" build -t writ-bench -f "$SCRIPT_DIR/Dockerfile" "$REPO_ROOT"

echo "Running benchmarks (RUNS=$RUNS, WARMUP=$WARMUP)..."
# On Windows (MINGW/MSYS), MSYS auto-converts Unix paths in args to Windows paths.
# This breaks volume mounts: '/results' becomes 'C:\Program Files\Git\results'.
# Use MSYS_NO_PATHCONV=1 for the run command only, and convert the host path
# from /d/path style to /mnt/d/path style for Podman's WSL backend.
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        # MSYS_NO_PATHCONV=1 prevents MSYS from converting /results to a Windows path.
        # Docker Desktop uses /d/path style (MINGW default); Podman WSL needs /mnt/d/path.
        if [ "$CONTAINER_CMD" = "podman" ]; then
            drive=$(printf '%s' "$RESULTS_DIR" | cut -c2 | tr '[:upper:]' '[:lower:]')
            rest=$(printf '%s' "$RESULTS_DIR" | cut -c3-)
            MOUNT_DIR="/mnt/${drive}${rest}"
        else
            MOUNT_DIR="$RESULTS_DIR"
        fi
        MSYS_NO_PATHCONV=1 "$CONTAINER_CMD" run --rm \
            -v "${MOUNT_DIR}://results" \
            -e "RESULTS_DIR=//results" \
            -e "RUNS=$RUNS" \
            -e "WARMUP=$WARMUP" \
            writ-bench
        ;;
    *)
        "$CONTAINER_CMD" run --rm \
            -v "$RESULTS_DIR:/results" \
            -e "RESULTS_DIR=/results" \
            -e "RUNS=$RUNS" \
            -e "WARMUP=$WARMUP" \
            writ-bench
        ;;
esac

echo ""

# Generate charts and markdown table
if command -v python3 > /dev/null 2>&1; then
    echo "Generating charts..."
    python3 "$REPO_ROOT/benchmark/generate.py" "$RESULTS_DIR/raw.json"
    echo "Done. Results: $RESULTS_DIR/"
else
    echo "Warning: python3 not found — charts not generated"
    echo "  Run manually: python3 benchmark/generate.py $RESULTS_DIR/raw.json"
    echo "  Requires: pip install pygal==3.1.0"
    echo "Done. Results: $RESULTS_DIR/raw.json"
fi
