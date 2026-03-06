#!/bin/bash
set -euo pipefail

# =============================================================================
# bench_runner.sh — Writ Benchmark Orchestration (runs inside Docker container)
# =============================================================================
# Measures execution time, memory, and startup for 6 language runtimes.
# Outputs: $RESULTS_DIR/raw.json
#
# Environment variables:
#   RESULTS_DIR  (default: /results)  — where raw.json is written
#   RUNS         (default: 15)        — hyperfine repetitions per measurement
#   WARMUP       (default: 5)         — hyperfine warmup iterations
# =============================================================================

RESULTS_DIR="${RESULTS_DIR:-/results}"
RUNS="${RUNS:-15}"
WARMUP="${WARMUP:-5}"

# =============================================================================
# Section 1: Version emission (INFRA-01, success criterion 5)
# =============================================================================

echo "=== Runtime Versions ==="
lua5.4 -v 2>&1 | head -1
sq --version 2>&1 | head -1 || echo "sq (version info unavailable)"
python3 --version
node --version
writ --help 2>&1 | head -1 || echo "writ (present)"
hyperfine --version
echo "========================="

# =============================================================================
# Section 2: Helper functions
# =============================================================================

# measure_anon_rss: Measures peak anonymous RSS in KB for a given command.
# Runs the command in the background, polls /proc/<pid>/status until exit.
# For very short-lived processes (stub benchmarks), may return 0 (documented).
# Redirects stdout/stderr of the benchmarked command to /dev/null.
measure_anon_rss() {
    local peak_kb=0
    "$@" > /dev/null 2>&1 &
    local pid=$!
    while kill -0 "$pid" 2>/dev/null; do
        local rss
        rss=$(awk '/^RssAnon:/{print $2}' "/proc/$pid/status" 2>/dev/null || echo 0)
        if [ "$rss" -gt "$peak_kb" ] 2>/dev/null; then
            peak_kb=$rss
        fi
    done
    wait "$pid" || true
    echo "$peak_kb"
}

# add_mad: jq filter that computes MAD from times[] array and adds it to the
# results[0] object. Takes a hyperfine JSON file on stdin, outputs the result
# object with an added `mad` field.
# MAD = median(|Xi - median(X)|)
# Note: jq `fabs` may not be available in all versions; use inline abs pattern.
add_mad() {
    jq '.results[0] + {
        mad: (
            .results[0].times as $times |
            ($times | length) as $n |
            (.results[0].median) as $med |
            ($times | map((. - $med) | if . < 0 then -. else . end) | sort) as $devs |
            $devs[($n / 2 | floor)]
        )
    }'
}

# run_hyperfine: Wrapper that runs hyperfine and pipes through add_mad.
# Returns the results[0] object with mad field added.
run_hyperfine() {
    local outfile="$1"; shift
    hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --ignore-failure \
        --export-json "$outfile" \
        "$@" 2>/dev/null
    add_mad < "$outfile"
}

# =============================================================================
# Section 3: Pre-compile stub for startup measurement
# Pre-compile once before the benchmark loop so /tmp/stub.writc exists.
# =============================================================================

echo "Pre-compiling stub for startup measurement..."
writ compile /bench/cases/stub/stub.writ -o /tmp/stub.writc 2>/dev/null || true

# =============================================================================
# Section 3: Main benchmark loop
# =============================================================================

mkdir -p "$RESULTS_DIR"

results='{"benchmarks":[],"meta":{}}'

for suite_dir in /bench/cases/*/; do
    suite=$(basename "$suite_dir")
    echo "--- Benchmarking: $suite ---"

    # -------------------------------------------------------------------------
    # 3a. Writ compile (INFRA-07)
    # Pre-compile once outside the timed loop so the .writc file exists for run
    # -------------------------------------------------------------------------
    echo "  [writ compile]"
    writ compile "${suite_dir}${suite}.writ" -o "/tmp/${suite}.writc" 2>/dev/null || true
    if [ ! -f "/tmp/${suite}.writc" ]; then
        echo "ERROR: writ compile failed for $suite — .writc not produced" >&2
    fi

    writ_compile_json=""
    if hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --ignore-failure \
        --export-json "/tmp/writ_compile_raw.json" \
        "writ compile ${suite_dir}${suite}.writ -o /tmp/${suite}.writc" \
        2>/dev/null; then
        writ_compile_json=$(add_mad < /tmp/writ_compile_raw.json)
    else
        echo "  WARNING: writ compile benchmark failed for $suite, using null" >&2
        writ_compile_json="null"
    fi
    if [ "$writ_compile_json" != "null" ]; then
        writ_compile_mem_kb=$(measure_anon_rss writ compile "${suite_dir}${suite}.writ" -o "/tmp/${suite}_mem.writc" 2>/dev/null || echo 0)
        writ_compile_json=$(printf '%s' "$writ_compile_json" | jq --argjson mem "$writ_compile_mem_kb" '. + {memory_kb: $mem}')
    fi

    # -------------------------------------------------------------------------
    # 3b. Writ run (INFRA-07)
    # -------------------------------------------------------------------------
    echo "  [writ run]"
    writ_run_json=""
    if hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --ignore-failure \
        --export-json "/tmp/writ_run_raw.json" \
        "writ run /tmp/${suite}.writc" \
        2>/dev/null; then
        writ_run_json=$(add_mad < /tmp/writ_run_raw.json)
    else
        echo "  WARNING: writ run benchmark failed for $suite, using null" >&2
        writ_run_json="null"
    fi
    if [ "$writ_run_json" != "null" ]; then
        writ_run_mem_kb=$(measure_anon_rss writ run "/tmp/${suite}.writc" || echo 0)
        writ_run_json=$(printf '%s' "$writ_run_json" | jq --argjson mem "$writ_run_mem_kb" '. + {memory_kb: $mem}')
    fi

    # -------------------------------------------------------------------------
    # 3c. Lua
    # -------------------------------------------------------------------------
    echo "  [lua5.4]"
    lua_json=""
    if hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --ignore-failure \
        --export-json "/tmp/lua_raw.json" \
        "lua5.4 ${suite_dir}${suite}.lua" \
        2>/dev/null; then
        lua_json=$(add_mad < /tmp/lua_raw.json)
    else
        echo "  WARNING: lua benchmark failed for $suite, using null" >&2
        lua_json="null"
    fi
    if [ "$lua_json" != "null" ]; then
        lua_mem_kb=$(measure_anon_rss lua5.4 "${suite_dir}${suite}.lua" || echo 0)
        lua_json=$(printf '%s' "$lua_json" | jq --argjson mem "$lua_mem_kb" '. + {memory_kb: $mem}')
    fi

    # -------------------------------------------------------------------------
    # 3d. Squirrel
    # -------------------------------------------------------------------------
    echo "  [sq (squirrel)]"
    squirrel_json=""
    if hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --ignore-failure \
        --export-json "/tmp/squirrel_raw.json" \
        "sq ${suite_dir}${suite}.nut" \
        2>/dev/null; then
        squirrel_json=$(add_mad < /tmp/squirrel_raw.json)
    else
        echo "  WARNING: squirrel benchmark failed for $suite, using null" >&2
        squirrel_json="null"
    fi
    if [ "$squirrel_json" != "null" ]; then
        squirrel_mem_kb=$(measure_anon_rss sq "${suite_dir}${suite}.nut" || echo 0)
        squirrel_json=$(printf '%s' "$squirrel_json" | jq --argjson mem "$squirrel_mem_kb" '. + {memory_kb: $mem}')
    fi

    # -------------------------------------------------------------------------
    # 3e. Python
    # -------------------------------------------------------------------------
    echo "  [python3]"
    python_json=""
    if hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --ignore-failure \
        --export-json "/tmp/python_raw.json" \
        "python3 ${suite_dir}${suite}.py" \
        2>/dev/null; then
        python_json=$(add_mad < /tmp/python_raw.json)
    else
        echo "  WARNING: python benchmark failed for $suite, using null" >&2
        python_json="null"
    fi
    if [ "$python_json" != "null" ]; then
        python_mem_kb=$(measure_anon_rss python3 "${suite_dir}${suite}.py" || echo 0)
        python_json=$(printf '%s' "$python_json" | jq --argjson mem "$python_mem_kb" '. + {memory_kb: $mem}')
    fi

    # -------------------------------------------------------------------------
    # 3f. Node.js
    # -------------------------------------------------------------------------
    echo "  [node]"
    node_json=""
    if hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --ignore-failure \
        --export-json "/tmp/node_raw.json" \
        "node ${suite_dir}${suite}.js" \
        2>/dev/null; then
        node_json=$(add_mad < /tmp/node_raw.json)
    else
        echo "  WARNING: node benchmark failed for $suite, using null" >&2
        node_json="null"
    fi
    if [ "$node_json" != "null" ]; then
        node_mem_kb=$(measure_anon_rss node "${suite_dir}${suite}.js" || echo 0)
        node_json=$(printf '%s' "$node_json" | jq --argjson mem "$node_mem_kb" '. + {memory_kb: $mem}')
    fi

    # -------------------------------------------------------------------------
    # 3g. Rust (pre-compiled binary)
    # -------------------------------------------------------------------------
    echo "  [rust (pre-compiled)]"
    rust_json=""
    if hyperfine --runs "$RUNS" --warmup "$WARMUP" \
        --ignore-failure \
        --export-json "/tmp/rust_raw.json" \
        "/bench/bin/${suite}" \
        2>/dev/null; then
        rust_json=$(add_mad < /tmp/rust_raw.json)
    else
        echo "  WARNING: rust benchmark failed for $suite, using null" >&2
        rust_json="null"
    fi
    if [ "$rust_json" != "null" ]; then
        rust_mem_kb=$(measure_anon_rss "/bench/bin/${suite}" || echo 0)
        rust_json=$(printf '%s' "$rust_json" | jq --argjson mem "$rust_mem_kb" '. + {memory_kb: $mem}')
    fi

    # -------------------------------------------------------------------------
    # Section 4: Startup time measurement (INFRA-06)
    # Measured using the same stub files (hello-world programs).
    # Writ startup uses the pre-compiled /tmp/stub.writc.
    # -------------------------------------------------------------------------
    echo "  [startup times]"
    startup_json='{}'
    for lang_entry in \
        "writ:writ run /tmp/stub.writc" \
        "lua:lua5.4 /bench/cases/stub/stub.lua" \
        "squirrel:sq /bench/cases/stub/stub.nut" \
        "python:python3 /bench/cases/stub/stub.py" \
        "node:node /bench/cases/stub/stub.js" \
        "rust:/bench/bin/stub"; do
        lang_name="${lang_entry%%:*}"
        lang_exec="${lang_entry#*:}"
        if hyperfine --runs "$RUNS" --warmup "$WARMUP" \
            --ignore-failure \
            --export-json "/tmp/startup_${lang_name}.json" \
            "$lang_exec" 2>/dev/null; then
            ms_val=$(jq '.results[0].median * 1000' "/tmp/startup_${lang_name}.json")
            startup_json=$(printf '%s' "$startup_json" | jq --arg k "${lang_name}_ms" --argjson v "$ms_val" '. + {($k): $v}')
        else
            echo "  WARNING: startup measurement failed for $lang_name, using 0" >&2
            startup_json=$(printf '%s' "$startup_json" | jq --arg k "${lang_name}_ms" '. + {($k): 0}')
        fi
    done

    # -------------------------------------------------------------------------
    # Section 5: Assemble per-suite JSON with jq (INFRA-08)
    # -------------------------------------------------------------------------
    echo "  [assembling suite result]"
    results=$(printf '%s' "$results" | jq \
        --arg suite "$suite" \
        --argjson writ_compile "${writ_compile_json:-null}" \
        --argjson writ_run "${writ_run_json:-null}" \
        --argjson lua "${lua_json:-null}" \
        --argjson squirrel "${squirrel_json:-null}" \
        --argjson python "${python_json:-null}" \
        --argjson node "${node_json:-null}" \
        --argjson rust "${rust_json:-null}" \
        --argjson startup "$startup_json" \
        '.benchmarks += [{
            suite: $suite,
            writ_compile: $writ_compile,
            writ_run: $writ_run,
            lua: $lua,
            squirrel: $squirrel,
            python: $python,
            node: $node,
            rust: $rust,
            startup: $startup
        }]')

    echo "  Done: $suite"
done

# =============================================================================
# Section 6: Add meta and write raw.json (INFRA-08)
# =============================================================================

DATE=$(date +%Y-%m-%d)
PLATFORM=$(uname -m)
results=$(printf '%s' "$results" | jq \
    --arg date "$DATE" --arg runs "$RUNS" --arg warmup "$WARMUP" --arg platform "$PLATFORM" \
    '.meta = {date: $date, runs: ($runs | tonumber), warmup: ($warmup | tonumber), platform: $platform}')

mkdir -p "$RESULTS_DIR"
printf '%s' "$results" | jq '.' > "$RESULTS_DIR/raw.json"
echo "Results written to $RESULTS_DIR/raw.json"
