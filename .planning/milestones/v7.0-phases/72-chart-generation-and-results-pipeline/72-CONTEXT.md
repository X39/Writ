# Phase 72: Chart Generation and Results Pipeline - Context

**Gathered:** 2026-03-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Python script (`generate.py`) that reads `raw.json` from the benchmark harness and produces SVG bar charts (execution time per benchmark, memory usage, startup time) plus a markdown `RESULTS.md` table. All output lands in `benchmark/results/YYYY-MM-DD/`. Host runner scripts (`run.sh`/`run.ps1`) invoke `generate.py` automatically for a one-command workflow. Benchmark programs themselves are Phase 71/73. CI workflow is Phase 74.

</domain>

<decisions>
## Implementation Decisions

### Chart Library
- Use **pygal** for SVG chart generation (locked decision from STATE.md: "Host-side chart generation (pygal)")
- pygal produces clean, embeddable SVG with built-in tooltips and responsive sizing
- No additional charting dependencies — pygal is the sole chart library

### Chart Styling
- Language-branded color palette: Writ=purple, Rust=orange, Lua=blue, Squirrel=teal, Python=gold, Node.js=green
- Light background with clean lines — optimized for GitHub README embedding and light/dark mode compatibility
- Y-axis labels include units (ms, MB, etc.)
- Bar labels show exact values

### Chart Layout — Execution Time
- **Per-benchmark**: one all-languages log-scale SVG (shows full range including Rust baseline)
- **Per-benchmark**: one interpreted-only linear-scale SVG (excludes Rust; better visual comparison among scripting languages)
- Writ bar shows combined compile+run time; tooltip breaks down compile vs run
- Y-axis anchored at 0, log scale for all-languages chart

### Chart Layout — Memory and Startup
- One memory SVG chart: grouped bar chart across all suites, all languages
- One startup SVG chart: grouped bar chart showing startup time per language
- Both use linear scale

### Markdown Table (RESULTS.md)
- Columns: Language | Benchmark | Median (ms) | Compile (ms) | Memory (MB) | Ratio to Rust
- Compile column shows value for Writ only, dash for other languages
- Ratio-to-Rust expressed as "×N.Nx" (e.g., ×14.2x)
- Precision: 1 decimal place for ms, 1 decimal for MB
- Grouped by benchmark suite with section headers

### Script Invocation
- `benchmark/generate.py` — standalone Python script, runs on host (not in Docker)
- CLI: `python3 benchmark/generate.py <path-to-raw.json>` — output files written alongside raw.json in the same directory
- `run.sh` and `run.ps1` auto-invoke `generate.py` after container exits, completing the one-command workflow
- Python 3.10+ required (host machine); pygal installed via pip

### Determinism (Success Criterion #5)
- Output must be bit-identical when re-run against the same `raw.json`
- No timestamps, random IDs, or non-deterministic content in SVG or markdown output
- pygal's `disable_xml_declaration=True` and fixed style config ensure reproducibility

### Claude's Discretion
- Exact pygal Style subclass configuration (font sizes, margins, spacing)
- Whether to use pygal's built-in `LightenStyle` or a custom `Style`
- Error handling for missing/null language entries in raw.json (skip gracefully)
- Whether to generate an index HTML page linking all SVGs (nice-to-have, not required)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase requirements
- `.planning/REQUIREMENTS.md` — REPORT-01 through REPORT-05 define all reporting requirements for this phase
- `.planning/ROADMAP.md` §Phase 72 — Success criteria (5 items) defining what must be TRUE

### Input format (raw.json)
- `benchmark/runner/bench_runner.sh` — Produces raw.json; defines the JSON schema (benchmarks[], meta{}, per-language objects with median/mean/stddev/mad/memory_kb fields)
- `benchmark/results/2026-03-20/raw.json` — Example raw.json from Phase 71 benchmark run

### Host runner scripts (integration points)
- `benchmark/runner/run.sh` — POSIX sh host launcher; needs `generate.py` call appended
- `benchmark/runner/run.ps1` — PowerShell host launcher; needs `generate.py` call appended

### Prior phase context
- `.planning/phases/70-docker-environment-and-measurement-harness/70-CONTEXT.md` — Docker architecture, JSON schema decisions, measurement approach
- `.planning/phases/71-compute-benchmarks-mvp/71-CONTEXT.md` — Benchmark structure, algorithm specs, directory conventions

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `benchmark/results/2026-03-20/raw.json`: Real benchmark data from Phase 71 — can be used for development and testing of generate.py
- `benchmark/runner/bench_runner.sh`: Defines the exact JSON structure that generate.py must parse (lines 270-290 show the per-suite assembly)
- `benchmark/runner/run.sh` and `run.ps1`: Host scripts where generate.py invocation will be added

### Established Patterns
- All benchmark output goes to `benchmark/results/YYYY-MM-DD/` dated subdirectories
- raw.json top-level structure: `{ "benchmarks": [...], "meta": { "date", "runs", "warmup", "platform" } }`
- Per-benchmark entry: `{ "suite", "writ_compile": {...}, "writ_run": {...}, "lua": {...}, "squirrel": {...}, "python": {...}, "node": {...}, "rust": {...}, "startup": {...} }`
- Per-language timing object: hyperfine format with `median`, `mean`, `stddev`, `min`, `max`, `times[]`, `mad`, `memory_kb`
- Startup times in `startup` sub-object: `writ_ms`, `lua_ms`, `squirrel_ms`, `python_ms`, `node_ms`, `rust_ms`

### Integration Points
- `run.sh` line 306: After `raw.json` is written — add `python3 benchmark/generate.py "$RESULTS_DIR/raw.json"` call
- `run.ps1`: Equivalent PowerShell invocation after container exits
- `benchmark/generate.py`: New file — no conflicts with existing structure

</code_context>

<specifics>
## Specific Ideas

- pygal was explicitly chosen as the charting library (STATE.md decision) — do not use matplotlib, plotly, or other alternatives
- "One command from the repo root" is the key UX requirement — run.sh/run.ps1 must produce charts without manual steps
- Interpreted-only charts are important for Writ positioning — Rust is so fast it compresses the scripting language bars visually
- Bit-identical output is a hard requirement (success criterion #5) — avoid any sources of non-determinism in SVG generation

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 72-chart-generation-and-results-pipeline*
*Context gathered: 2026-03-20*
