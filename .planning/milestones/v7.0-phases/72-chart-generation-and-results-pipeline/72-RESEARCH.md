# Phase 72: Chart Generation and Results Pipeline - Research

**Researched:** 2026-03-20
**Domain:** Python SVG chart generation (pygal), data pipeline, markdown generation
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Use **pygal** for SVG chart generation — locked in STATE.md ("Host-side chart generation (pygal)")
- No additional charting dependencies — pygal is the sole chart library
- Language-branded color palette: Writ=purple (#7c3aed), Rust=orange (#ea580c), Lua=blue (#2563eb), Squirrel=teal (#0d9488), Python=gold (#eab308), Node.js=green (#16a34a)
- Light background optimized for GitHub README embedding and light/dark mode compatibility
- Y-axis labels include units (ms, MB, etc.)
- Bar labels show exact values
- **Per-benchmark exec time**: one all-languages log-scale SVG + one interpreted-only linear-scale SVG (excludes Rust)
- Writ bar shows combined compile+run time; tooltip breaks down compile vs run
- One memory SVG: grouped bar chart, all suites, all languages; linear scale
- One startup SVG: grouped bar chart showing startup time per language; linear scale
- RESULTS.md columns: Language | Benchmark | Median (ms) | Compile (ms) | Memory (MB) | Ratio to Rust
- Compile column shows value for Writ only, dash for other languages
- Ratio format: "xN.Nx" (e.g., x14.2x)
- Precision: 1 decimal for ms, 1 decimal for MB
- Grouped by benchmark suite with section headers in RESULTS.md
- `benchmark/generate.py` — standalone Python script, runs on host (not Docker)
- CLI: `python3 benchmark/generate.py <path-to-raw.json>`
- Output files written alongside raw.json in the same directory
- `run.sh` and `run.ps1` auto-invoke `generate.py` after container exits
- Python 3.10+ required (host machine); pygal installed via pip
- Output must be bit-identical when re-run against the same `raw.json`
- `disable_xml_declaration=True` and fixed style config ensure reproducibility

### Claude's Discretion
- Exact pygal Style subclass configuration (font sizes, margins, spacing)
- Whether to use pygal's built-in `LightenStyle` or a custom `Style`
- Error handling for missing/null language entries in raw.json (skip gracefully)
- Whether to generate an index HTML page linking all SVGs (nice-to-have, not required)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| REPORT-01 | SVG bar charts generated for each benchmark category (execution time) | pygal Bar with logarithmic=True for all-languages; second Bar with logarithmic=False for interpreted-only; one chart per benchmark suite |
| REPORT-02 | SVG bar chart for memory usage comparison | pygal Bar, linear scale, one series per language, x_labels=suites, memory_kb/1024 for MB conversion |
| REPORT-03 | SVG bar chart for startup time comparison | pygal Bar, linear scale, one series per language, startup sub-object from raw.json (already in ms) |
| REPORT-04 | Markdown table generated with all metrics for README embedding | Python f-string table generation; Writ ratio = (compile_ms + run_ms) / rust_ms |
| REPORT-05 | Charts and tables committed to `benchmark/results/` | generate.py writes files to same dir as raw.json; run.sh/run.ps1 call generate.py after container exits |
</phase_requirements>

---

## Summary

Phase 72 implements `benchmark/generate.py`, a standalone Python script that reads `raw.json` produced by the Docker benchmark harness and generates SVG bar charts plus a markdown results table. The script uses the **pygal 3.1.0** library (the project's locked choice), runs on the host machine (not in Docker), and writes all output files alongside `raw.json` in `benchmark/results/YYYY-MM-DD/`.

The central technical challenge is **deterministic SVG output**: pygal embeds `date.today().isoformat()` in an SVG comment and generates a random `uuid4()` per chart instance. Both are non-deterministic sources that would break bit-identical re-generation. The fix is: use `no_prefix=True` (disables UUID-based CSS selectors) and strip the date comment via regex after rendering. These two measures, combined with a fixed `Style` configuration, produce fully deterministic output — verified by running the generation twice against the same data and confirming byte-identical SVGs.

A secondary issue is the "Y-axis anchored at 0, log scale" requirement from CONTEXT.md. Log scale cannot mathematically include 0 (log(0) is undefined). Pygal's `logarithmic=True` sets the y-axis minimum to the smallest data value, not 0. The planner should resolve this as: the all-languages chart uses log scale starting at the minimum data value (not 0), which is the standard for log scale bar charts. The "anchored at 0" language in CONTEXT.md should be interpreted as "bars extend from the chart baseline" which pygal satisfies automatically.

**Primary recommendation:** One Python file (`benchmark/generate.py`), ~150–200 lines, using pygal 3.1.0 with `no_prefix=True`, `disable_xml_declaration=True`, a date-comment strip, and deterministic chart IDs derived from suite name + chart type.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pygal | 3.1.0 | SVG chart generation | Project-locked decision; produces embeddable SVG with tooltips, supports log scale, pure Python |
| json | stdlib | Parse raw.json | No dep needed |
| re | stdlib | Strip non-deterministic SVG comment | Minimal targeted fix |
| os / pathlib | stdlib | File path construction | Standard file I/O |
| sys | stdlib | argv parsing | No dep needed for single-arg CLI |
| datetime | stdlib | Derive output date from meta.date | raw.json contains `meta.date` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| argparse | stdlib | CLI argument parsing | If CLI grows beyond single positional arg |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| pygal | matplotlib | matplotlib produces raster PNG by default; SVG requires extra work; not project-locked |
| pygal | plotly | plotly produces interactive HTML; heavier; not project-locked |
| regex strip | monkey-patch date.today | Both work; regex strip is simpler and doesn't affect other code |
| no_prefix=True | chart.uuid = fixed_str | Both work; no_prefix=True is cleaner (eliminates UUID from CSS selectors entirely) |

**Installation:**
```bash
python3 -m pip install pygal==3.1.0
```

**Version verification:** Verified 2026-03-20 — pygal 3.1.0 released 2025-12-09, available on PyPI. Python >=3.8 required.

---

## Architecture Patterns

### Recommended Project Structure
```
benchmark/
├── generate.py              # New file — the entire pipeline in one script
├── runner/
│   ├── run.sh               # Add generate.py invocation at end (line 56)
│   ├── run.ps1              # Add generate.py invocation at end (line 46)
│   └── bench_runner.sh      # Unchanged — produces raw.json inside Docker
└── results/
    └── YYYY-MM-DD/
        ├── raw.json         # Input (produced by Docker)
        ├── exec_<suite>_all.svg        # All-languages log-scale per benchmark
        ├── exec_<suite>_interp.svg     # Interpreted-only linear-scale per benchmark
        ├── memory.svg                  # Memory comparison across all suites
        ├── startup.svg                 # Startup time per language
        └── RESULTS.md                  # Markdown table
```

### Pattern 1: Chart Generation with Deterministic Output

**What:** Create pygal charts, strip non-deterministic content, write to file.
**When to use:** Every chart output in generate.py.
**Example:**
```python
# Source: verified against pygal 3.1.0 source (github.com/Kozea/pygal)
import pygal
import re
from pygal.style import Style

LANG_COLORS = (
    '#7c3aed',  # Writ — purple
    '#ea580c',  # Rust — orange
    '#2563eb',  # Lua — blue
    '#0d9488',  # Squirrel — teal
    '#eab308',  # Python — gold
    '#16a34a',  # Node.js — green
)

CHART_STYLE = Style(
    background='white',
    plot_background='white',
    foreground='#333333',
    foreground_strong='#000000',
    foreground_subtle='#999999',
    colors=LANG_COLORS,
)

def make_chart(title, y_title, logarithmic=False):
    return pygal.Bar(
        style=CHART_STYLE,
        disable_xml_declaration=True,  # embed in HTML without XML header
        no_prefix=True,                # disables UUID-based CSS selectors
        title=title,
        y_title=y_title,
        logarithmic=logarithmic,
        height=400,
        width=900,
        show_legend=True,
    )

_DATE_COMMENT_RE = re.compile(r'<!--Generated with pygal[^>]*-->')

def render_svg(chart, path):
    """Render chart to SVG file with deterministic output."""
    svg = chart.render()
    # pygal embeds date.today() in a comment — strip for determinism
    svg = _DATE_COMMENT_RE.sub('<!--Generated with pygal-->', svg)
    with open(path, 'w', encoding='utf-8') as f:
        f.write(svg)
```

### Pattern 2: Data Extraction from raw.json

**What:** Parse raw.json fields into ms/MB units for chart consumption.
**When to use:** At the top of generate.py, before any chart generation.

```python
# Source: verified against benchmark/results/2026-03-20/raw.json
import json, sys

def load_raw(path):
    with open(path, encoding='utf-8') as f:
        return json.load(f)

# raw.json field units:
#   median        → seconds (multiply by 1000 for ms)
#   memory_kb     → kilobytes (divide by 1024 for MB)
#   startup.*_ms  → milliseconds (already in ms)

def writ_total_ms(b):
    """Combined compile+run time in ms."""
    return (b['writ_compile']['median'] + b['writ_run']['median']) * 1000

def writ_compile_ms(b):
    return b['writ_compile']['median'] * 1000

def writ_run_ms(b):
    return b['writ_run']['median'] * 1000

def lang_ms(b, key):
    """Execution time in ms for non-Writ language."""
    entry = b.get(key)
    if entry is None:
        return None
    return entry['median'] * 1000

def lang_memory_mb(b, key):
    """Peak anonymous RSS in MB; returns 0.0 if not measured."""
    entry = b.get(key)
    if entry is None:
        return 0.0
    kb = entry.get('memory_kb', 0)
    return kb / 1024.0
```

### Pattern 3: Per-Benchmark Execution Time Charts

**What:** Generate two SVGs per benchmark suite — all-languages log scale, interpreted-only linear scale.
**When to use:** REPORT-01 requirement.

```python
# Source: verified live against pygal 3.1.0
ALL_LANGS = [
    # (display_name, raw.json_key_or_special)
    ('Writ',     'writ'),    # special: writ_compile + writ_run
    ('Rust',     'rust'),
    ('Lua',      'lua'),
    ('Squirrel', 'squirrel'),
    ('Python',   'python'),
    ('Node.js',  'node'),
]
INTERP_LANGS = [l for l in ALL_LANGS if l[0] != 'Rust']

def exec_chart_data(b, langs):
    """Build (name, value_or_dict) pairs for an exec time chart."""
    rows = []
    for name, key in langs:
        if key == 'writ':
            wc = writ_compile_ms(b)
            wr = writ_run_ms(b)
            rows.append((name, {
                'value': round(wc + wr, 3),
                'label': f'compile: {wc:.2f}ms, run: {wr:.2f}ms'
            }))
        else:
            ms = lang_ms(b, key)
            if ms is None:
                continue
            rows.append((name, round(ms, 3)))
    return rows

def generate_exec_charts(b, out_dir):
    suite = b['suite']

    # All-languages, log scale
    chart = make_chart(
        f'Execution Time — {suite} (log scale, all languages)',
        'Time (ms)',
        logarithmic=True,
    )
    data = exec_chart_data(b, ALL_LANGS)
    chart.x_labels = [name for name, _ in data]
    for name, val in data:
        chart.add(name, [val])
    render_svg(chart, out_dir / f'exec_{suite}_all.svg')

    # Interpreted-only, linear scale
    chart2 = make_chart(
        f'Execution Time — {suite} (interpreted languages)',
        'Time (ms)',
        logarithmic=False,
    )
    data2 = exec_chart_data(b, INTERP_LANGS)
    chart2.x_labels = [name for name, _ in data2]
    for name, val in data2:
        chart2.add(name, [val])
    render_svg(chart2, out_dir / f'exec_{suite}_interp.svg')
```

### Pattern 4: Memory and Startup Charts (Grouped, Multi-Suite)

**What:** One memory chart and one startup chart across all suites.
**When to use:** REPORT-02 and REPORT-03.

```python
# Source: verified live against pygal 3.1.0
def generate_memory_chart(benchmarks, out_dir):
    """One series per language, x_labels = suite names."""
    chart = make_chart('Memory Usage by Benchmark', 'Memory (MB)')
    suites = [b['suite'] for b in benchmarks]
    chart.x_labels = suites

    for name, key in ALL_LANGS:
        values = []
        for b in benchmarks:
            if key == 'writ':
                mb = lang_memory_mb(b, 'writ_run')  # runtime memory
            else:
                mb = lang_memory_mb(b, key)
            values.append(round(mb, 2) if mb > 0 else {'value': 0, 'label': 'not measured'})
        chart.add(name, values)
    render_svg(chart, out_dir / 'memory.svg')


def generate_startup_chart(benchmarks, out_dir):
    """Startup time from b['startup'] sub-object (already in ms)."""
    chart = make_chart('Startup Time by Language', 'Time (ms)')

    # startup is measured per-suite but uses the same stub — take the first benchmark
    # or average across suites if multiple benchmarks exist
    b = benchmarks[0]
    startup = b['startup']

    STARTUP_KEYS = [
        ('Writ',     'writ_ms'),
        ('Rust',     'rust_ms'),
        ('Lua',      'lua_ms'),
        ('Squirrel', 'squirrel_ms'),
        ('Python',   'python_ms'),
        ('Node.js',  'node_ms'),
    ]
    chart.x_labels = [name for name, _ in STARTUP_KEYS]
    for name, key in STARTUP_KEYS:
        chart.add(name, [round(startup.get(key, 0), 3)])
    render_svg(chart, out_dir / 'startup.svg')
```

### Pattern 5: RESULTS.md Markdown Table

**What:** Generate a markdown table with one row per language per benchmark suite.
**When to use:** REPORT-04.

```python
# Source: CONTEXT.md table specification
def ratio_str(numerator_ms, rust_ms):
    if rust_ms == 0:
        return 'N/A'
    return f'x{numerator_ms / rust_ms:.1f}x'

def generate_results_md(data, out_dir):
    benchmarks = data['benchmarks']
    meta = data['meta']
    lines = []
    lines.append(f'# Benchmark Results — {meta["date"]}')
    lines.append(f'')
    lines.append(f'Platform: {meta["platform"]} | Runs: {meta["runs"]} | Warmup: {meta["warmup"]}')
    lines.append(f'')

    header = '| Language | Benchmark | Median (ms) | Compile (ms) | Memory (MB) | Ratio to Rust |'
    sep    = '|----------|-----------|-------------|--------------|-------------|---------------|'

    for b in benchmarks:
        suite = b['suite']
        lines.append(f'## {suite}')
        lines.append(f'')
        lines.append(header)
        lines.append(sep)

        rust_ms = lang_ms(b, 'rust') or 1.0  # guard division by zero

        # Writ row
        wc = writ_compile_ms(b)
        wr = writ_run_ms(b)
        wt = wc + wr
        wm = lang_memory_mb(b, 'writ_run')
        lines.append(
            f'| Writ | {suite} | {wt:.1f} | {wc:.1f} | {wm:.1f} | {ratio_str(wt, rust_ms)} |'
        )

        # Other languages
        for name, key in [('Rust','rust'),('Lua','lua'),('Squirrel','squirrel'),('Python','python'),('Node.js','node')]:
            ms = lang_ms(b, key)
            if ms is None:
                continue
            mb = lang_memory_mb(b, key)
            compile_col = '-'
            lines.append(
                f'| {name} | {suite} | {ms:.1f} | {compile_col} | {mb:.1f} | {ratio_str(ms, rust_ms)} |'
            )
        lines.append(f'')

    out_path = out_dir / 'RESULTS.md'
    with open(out_path, 'w', encoding='utf-8') as f:
        f.write('\n'.join(lines))
```

### Pattern 6: run.sh / run.ps1 Integration

**What:** Append `python3 benchmark/generate.py` invocation to both host runner scripts.
**When to use:** REPORT-05 — one-command workflow.

For `run.sh` (append after line 56, the `echo "Done."` line):
```sh
# Generate charts and markdown table
if command -v python3 > /dev/null 2>&1; then
    echo "Generating charts..."
    python3 "$REPO_ROOT/benchmark/generate.py" "$RESULTS_DIR/raw.json"
    echo "Charts: $RESULTS_DIR/"
else
    echo "Warning: python3 not found — run 'python3 benchmark/generate.py $RESULTS_DIR/raw.json' manually"
fi
```

For `run.ps1` (append after line 46, the `Write-Host "Done."` line):
```powershell
# Generate charts and markdown table
if (Get-Command python3 -ErrorAction SilentlyContinue) {
    Write-Host "Generating charts..."
    & python3 "$RepoRoot\benchmark\generate.py" "$ResultsDir\raw.json"
    Write-Host "Charts: $ResultsDir\"
} else {
    Write-Warning "python3 not found — run 'python3 benchmark\generate.py $ResultsDir\raw.json' manually"
}
```

### Anti-Patterns to Avoid

- **Embedding date.today() in output:** pygal does this automatically in a comment — must be stripped for determinism. Do not add any other timestamp sources.
- **Using chart.uuid without overriding:** pygal generates `uuid4()` per chart instance; `no_prefix=True` eliminates UUID from CSS selectors completely.
- **Opening raw.json from a hardcoded path:** generate.py must take `<path-to-raw.json>` as a positional CLI argument so it works for any dated results directory.
- **Using writ_compile.memory_kb for the memory chart:** writ_compile memory measures the compiler process; the meaningful runtime comparison uses writ_run.memory_kb (currently 0 for the stub — this is expected and documented).
- **Using `logarithmic=True` with zero-valued data:** pygal silently skips log(0). Memory values of 0 should be charted as-is on linear scale with a tooltip noting "not measured."
- **Leaving memory chart broken for short-lived processes:** Many languages report 0 KB for quick benchmarks (polling misses the process). This is a documented limitation from the bench_runner.sh design — display 0 with tooltip "not measured."

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SVG bar chart generation | Custom SVG string builder | pygal Bar | Handles viewport, CSS, tooltips, log scale, responsive sizing |
| Log scale axis | Manual axis calculation | pygal `logarithmic=True` | Handles tick placement, label formatting, visual proportions |
| Color palette per series | Manual SVG color attrs | `Style(colors=tuple)` | pygal applies colors in series-add order automatically |
| Markdown table formatting | Complex string formatting | Python f-strings with fixed column format | Simple enough to do inline; no library needed |

**Key insight:** pygal handles all SVG complexity (CSS, viewBox, axis ticks, tooltips, animation). The only non-trivial custom work is the determinism fix (date comment strip).

---

## Common Pitfalls

### Pitfall 1: Non-Deterministic SVG from pygal's date comment
**What goes wrong:** Re-running generate.py on a different calendar day against the same raw.json produces different SVG files (different date in `<!--Generated with pygal 3.1.0 (etree) ... on 2026-03-20-->` comment). Bit-identical check fails.
**Why it happens:** pygal calls `date.today().isoformat()` when building the SVG root. This is hardcoded in `pygal/svg.py`.
**How to avoid:** Strip the comment with regex after `chart.render()`:
```python
import re
_DATE_RE = re.compile(r'<!--Generated with pygal[^>]*-->')
svg = _DATE_RE.sub('<!--Generated with pygal-->', chart.render())
```
**Warning signs:** SVGs differ when diff'd on the first comment line only.

### Pitfall 2: Non-Deterministic SVG from UUID prefix (without no_prefix)
**What goes wrong:** Each `pygal.Bar()` instance generates a new `uuid4()` and embeds it in CSS selectors (`#chart-{uuid} ...`). Two renders of identical data produce different SVGs.
**Why it happens:** pygal calls `str(uuid4())` in `BaseGraph.__init__`.
**How to avoid:** Pass `no_prefix=True` to the chart constructor. This sets the CSS ID selector to `''` instead of `#chart-{uuid}`.
**Warning signs:** Multiple occurrences of a UUID pattern in SVG diff output.

### Pitfall 3: Log Scale Cannot Include Zero
**What goes wrong:** The CONTEXT.md says "Y-axis anchored at 0, log scale." Log scale cannot render 0 on the y-axis (log(0) = -infinity). Attempting to add 0-valued data points to a log scale chart silently skips them.
**Why it happens:** Mathematical constraint — log scale is only defined for positive reals.
**How to avoid:** Interpret "anchored at 0" as "bars extend from the chart baseline" (which pygal handles). The y-axis minimum will be slightly below the smallest data value. This is standard for log scale bar charts and is clearly labeled with `(log scale)` in the chart title.
**Warning signs:** Missing bars for languages with very fast execution times near 0.

### Pitfall 4: median Values in raw.json are Seconds, Not Milliseconds
**What goes wrong:** raw.json `median` fields are in seconds (hyperfine output format). Displaying them directly on the chart gives values like 0.0005 instead of 0.5.
**Why it happens:** hyperfine reports in seconds; the bench_runner.sh does not convert before writing raw.json. Exception: `startup.*_ms` fields are already in milliseconds.
**How to avoid:** Always multiply `median` by 1000 for ms. The `startup` sub-object fields are named `*_ms` explicitly — use them as-is.
**Warning signs:** Chart values 1000x too small; RESULTS.md shows sub-millisecond values.

### Pitfall 5: memory_kb=0 for Short-Lived Processes
**What goes wrong:** Many languages show `memory_kb: 0` in raw.json (Writ run, Lua, Rust) because the RSC poll misses the process before it exits. Displaying 0.0 MB with no context is confusing.
**Why it happens:** `measure_anon_rss()` in bench_runner.sh polls `/proc/<pid>/status` in a loop; fast processes exit before the first poll.
**How to avoid:** Display 0.0 as a bar (it is valid data) but add a tooltip `{'value': 0.0, 'label': 'not measured (process too fast)'}`. Document this in RESULTS.md header.
**Warning signs:** All fast languages showing 0.0 MB while slower ones show measured values.

### Pitfall 6: STYLE Object Shared Across Charts Causes Color Mis-assignment
**What goes wrong:** If series are added in different orders across charts (e.g., memory chart vs. exec chart), the color palette order no longer matches the language assignment.
**Why it happens:** pygal assigns colors in the order `chart.add()` is called. If you add languages in different order for different charts, the colors rotate incorrectly.
**How to avoid:** Always add series in the same canonical order for all charts: Writ, Rust, Lua, Squirrel, Python, Node.js. Define this as a top-level constant.

---

## Code Examples

Verified patterns from direct testing against pygal 3.1.0 on Python 3.11.9:

### Full Minimal Working generate.py Skeleton
```python
#!/usr/bin/env python3
"""benchmark/generate.py — Generate SVG charts and RESULTS.md from raw.json."""
import json
import re
import sys
from pathlib import Path

import pygal
from pygal.style import Style

# ── Color palette (canonical order) ──────────────────────────────────────────
LANG_COLORS = (
    '#7c3aed',  # Writ — purple
    '#ea580c',  # Rust — orange
    '#2563eb',  # Lua — blue
    '#0d9488',  # Squirrel — teal
    '#eab308',  # Python — gold
    '#16a34a',  # Node.js — green
)

CHART_STYLE = Style(
    background='white',
    plot_background='white',
    foreground='#333333',
    foreground_strong='#000000',
    foreground_subtle='#999999',
    colors=LANG_COLORS,
)

_DATE_COMMENT_RE = re.compile(r'<!--Generated with pygal[^>]*-->')


def make_chart(title, y_title, logarithmic=False):
    return pygal.Bar(
        style=CHART_STYLE,
        disable_xml_declaration=True,
        no_prefix=True,
        title=title,
        y_title=y_title,
        logarithmic=logarithmic,
        height=400,
        width=900,
        show_legend=True,
        print_values=False,
    )


def render_svg(chart, path):
    svg = chart.render()
    svg = _DATE_COMMENT_RE.sub('<!--Generated with pygal-->', svg)
    Path(path).write_text(svg, encoding='utf-8')


def main():
    if len(sys.argv) != 2:
        print(f'Usage: python3 {sys.argv[0]} <path-to-raw.json>', file=sys.stderr)
        sys.exit(1)

    raw_path = Path(sys.argv[1])
    out_dir = raw_path.parent

    with open(raw_path, encoding='utf-8') as f:
        data = json.load(f)

    benchmarks = data['benchmarks']

    # 1. Per-benchmark exec time charts (REPORT-01)
    for b in benchmarks:
        # ... exec chart generation
        pass

    # 2. Memory chart (REPORT-02)
    # ... memory chart generation

    # 3. Startup chart (REPORT-03)
    # ... startup chart generation

    # 4. RESULTS.md (REPORT-04)
    # ... markdown table generation

    print(f'Output written to: {out_dir}')


if __name__ == '__main__':
    main()
```

### Writ Tooltip with Compile/Run Breakdown
```python
# Source: verified against pygal 3.1.0 value config docs
wc = b['writ_compile']['median'] * 1000  # seconds -> ms
wr = b['writ_run']['median'] * 1000
writ_value = {
    'value': round(wc + wr, 3),
    'label': f'compile: {wc:.2f}ms, run: {wr:.2f}ms'
}
chart.add('Writ', [writ_value])
```

### Startup Chart (startup sub-object is already in ms)
```python
# Source: verified against raw.json schema (bench_runner.sh lines 258-264)
startup = b['startup']  # keys: writ_ms, lua_ms, squirrel_ms, python_ms, node_ms, rust_ms
chart.add('Writ',     [round(startup.get('writ_ms', 0), 3)])
chart.add('Rust',     [round(startup.get('rust_ms', 0), 3)])
chart.add('Lua',      [round(startup.get('lua_ms', 0), 3)])
chart.add('Squirrel', [round(startup.get('squirrel_ms', 0), 3)])
chart.add('Python',   [round(startup.get('python_ms', 0), 3)])
chart.add('Node.js',  [round(startup.get('node_ms', 0), 3)])
```

### Ratio-to-Rust Formatting
```python
# x14.2x format (CONTEXT.md spec)
def ratio_str(ms, rust_ms):
    if not rust_ms:
        return 'N/A'
    return f'x{ms / rust_ms:.1f}x'
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| matplotlib for benchmark charts | pygal (SVG, embeddable) | Project decision | SVG works in GitHub README without hosting |
| Raster PNG charts | SVG charts | Project decision | Scalable, tooltips, no resolution issues |
| Manual SVG string generation | pygal library | - | Handles all SVG complexity |

**Deprecated/outdated:**
- pygal 2.x: API is compatible with 3.x for this use case; 3.1.0 is current as of December 2025

---

## Open Questions

1. **Startup chart across multiple suites**
   - What we know: startup times are measured per-suite but always use the same stub files (`/bench/cases/stub/stub.*`)
   - What's unclear: should the startup chart show one set of bars (from bench[0].startup) or average across all suites?
   - Recommendation: Use bench[0].startup (or average if multiple benchmarks exist). The stub is the same regardless of suite, so results are equivalent. Simplest correct choice is bench[0].startup.

2. **Memory chart for Writ: compile vs. run**
   - What we know: `writ_compile.memory_kb=200` (compiler), `writ_run.memory_kb=0` (runtime, too fast to measure)
   - What's unclear: should the memory chart show Writ compile memory, run memory, or combined?
   - Recommendation: Show `writ_run.memory_kb` (the runtime process) for consistency with other languages. The compiler is a separate process. Add a note in RESULTS.md that Writ runtime memory shows 0 for short benchmarks (polling limitation).

3. **Output file naming when multiple suites exist**
   - What we know: currently one suite ("stub") in raw.json; Phase 73 adds more
   - What's unclear: `exec_stub_all.svg`, `exec_fib_all.svg` etc. — is this the right naming?
   - Recommendation: `exec_{suite}_all.svg` and `exec_{suite}_interp.svg` per CONTEXT.md pattern. This is forward-compatible.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None detected — benchmark/generate.py is a new standalone script |
| Config file | None — no existing Python test infrastructure |
| Quick run command | `python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json && ls benchmark/results/2026-03-20/*.svg benchmark/results/2026-03-20/RESULTS.md` |
| Full suite command | Same — single integration test against real raw.json |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REPORT-01 | SVG exec time charts generated | smoke | `python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json && test -f benchmark/results/2026-03-20/exec_stub_all.svg && test -f benchmark/results/2026-03-20/exec_stub_interp.svg` | ❌ Wave 0 |
| REPORT-02 | SVG memory chart generated | smoke | `test -f benchmark/results/2026-03-20/memory.svg` | ❌ Wave 0 |
| REPORT-03 | SVG startup chart generated | smoke | `test -f benchmark/results/2026-03-20/startup.svg` | ❌ Wave 0 |
| REPORT-04 | RESULTS.md generated with correct columns | smoke | `test -f benchmark/results/2026-03-20/RESULTS.md && grep -q 'Ratio to Rust' benchmark/results/2026-03-20/RESULTS.md` | ❌ Wave 0 |
| REPORT-05 | Output files in correct directory | smoke | `ls benchmark/results/2026-03-20/*.svg benchmark/results/2026-03-20/RESULTS.md` | ❌ Wave 0 |
| REPORT-01/05 | Bit-identical re-run determinism | smoke | `python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json && cp benchmark/results/2026-03-20/exec_stub_all.svg /tmp/first.svg && python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json && diff /tmp/first.svg benchmark/results/2026-03-20/exec_stub_all.svg` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `python3 benchmark/generate.py benchmark/results/2026-03-20/raw.json` (visual inspection of output)
- **Per wave merge:** Determinism check (run twice, diff output)
- **Phase gate:** All SVG files present + RESULTS.md contains all required columns + determinism verified

### Wave 0 Gaps
- [ ] `benchmark/generate.py` — the entire new file
- [ ] `benchmark/runner/run.sh` — append generate.py invocation (2-line addition)
- [ ] `benchmark/runner/run.ps1` — append generate.py invocation (4-line addition)
- [ ] `python3 -m pip install pygal==3.1.0` — host prerequisite (document in README or generate.py header)

---

## Sources

### Primary (HIGH confidence)
- pygal 3.1.0 source code (`github.com/Kozea/pygal/blob/master/pygal/graph/base.py`) — uuid4() generation confirmed
- pygal 3.1.0 source code (`pygal/svg.py`) — `date.today().isoformat()` in SVG comment confirmed
- pygal 3.1.0 live testing on Python 3.11.9 — determinism verified (both fixes confirmed bit-identical output)
- `pygal.org/en/stable/documentation/custom_styles.html` — Style(colors=tuple) constructor verified
- `pygal.org/en/stable/api/pygal.config.html` — no_prefix, logarithmic, disable_xml_declaration options confirmed
- `benchmark/results/2026-03-20/raw.json` — actual data schema verified (seconds for median, already-ms for startup)
- `benchmark/runner/bench_runner.sh` — data assembly and schema confirmed (lines 270-307)
- `benchmark/runner/run.sh` / `run.ps1` — integration point identified (last echo line in each)
- PyPI `pypi.org/project/pygal/` — version 3.1.0, released 2025-12-09, Python >=3.8

### Secondary (MEDIUM confidence)
- `pygal.org/en/stable/documentation/configuration/chart.html` — chart configuration options (page structure confirmed; individual option pages were 404)
- `pygal.org/en/stable/documentation/first_steps.html` — basic Bar chart API confirmed

### Tertiary (LOW confidence)
- WebSearch results on pygal log scale behavior — multiple sources confirm log scale bars start from data minimum (not 0); consistent with mathematical constraint

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — pygal 3.1.0 installed and tested live; all patterns verified against real raw.json
- Architecture: HIGH — patterns proven with live code execution; determinism confirmed
- Pitfalls: HIGH — uuid4() and date.today() sources confirmed from pygal source code; all fixes tested

**Research date:** 2026-03-20
**Valid until:** 2026-06-20 (pygal is stable; stdlib patterns do not expire)
