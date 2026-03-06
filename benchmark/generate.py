#!/usr/bin/env python3
"""Generate SVG benchmark charts and RESULTS.md from raw.json.

Requires: pip install pygal==3.1.0
Usage:    python3 benchmark/generate.py path/to/raw.json
"""
import json
import re
import sys
from pathlib import Path

import pygal
from pygal.style import Style

# ── Language canonical order (colors must match this order exactly) ───────────
ALL_LANGS = [
    ('Writ',     'writ'),      # special: uses writ_compile + writ_run
    ('Rust',     'rust'),
    ('Lua',      'lua'),
    ('Squirrel', 'squirrel'),
    ('Python',   'python'),
    ('Node.js',  'node'),
]
INTERP_LANGS = [lang for lang in ALL_LANGS if lang[0] != 'Rust']

# ── Color palette (matches ALL_LANGS order) ───────────────────────────────────
LANG_COLORS = (
    '#7c3aed',  # Writ     — purple
    '#ea580c',  # Rust     — orange
    '#2563eb',  # Lua      — blue
    '#0d9488',  # Squirrel — teal
    '#eab308',  # Python   — gold
    '#16a34a',  # Node.js  — green
)

CHART_STYLE = Style(
    background='white',
    plot_background='white',
    foreground='#333333',
    foreground_strong='#000000',
    foreground_subtle='#999999',
    colors=LANG_COLORS,
)

# Strip pygal's embedded date comment for bit-identical re-runs
_DATE_COMMENT_RE = re.compile(r'<!--Generated with pygal[^>]*-->')


# ── Chart factory ─────────────────────────────────────────────────────────────

def make_chart(title, y_title, logarithmic=False):
    """Return a configured pygal.Bar instance."""
    return pygal.Bar(
        style=CHART_STYLE,
        disable_xml_declaration=True,   # embed in HTML without XML header
        no_prefix=True,                  # eliminates UUID-based CSS selectors
        title=title,
        y_title=y_title,
        logarithmic=logarithmic,
        height=400,
        width=900,
        show_legend=True,
        print_values=False,
    )


def render_svg(chart, path):
    """Render chart to SVG with deterministic output.

    pygal 3.1.0 returns bytes from chart.render(); decode before regex sub.
    """
    raw = chart.render()
    if isinstance(raw, bytes):
        svg_text = raw.decode('utf-8')
    else:
        svg_text = raw
    svg_text = _DATE_COMMENT_RE.sub('<!--Generated with pygal-->', svg_text)
    Path(path).write_text(svg_text, encoding='utf-8')


# ── Data helpers ──────────────────────────────────────────────────────────────

def writ_total_ms(b):
    """Combined compile+run time in ms; returns None if Writ not available."""
    wc = b.get('writ_compile')
    wr = b.get('writ_run')
    if wc is None or wr is None:
        return None
    return (wc['median'] + wr['median']) * 1000


def writ_compile_ms(b):
    """Compiler time in ms; returns None if Writ not available for this suite."""
    entry = b.get('writ_compile')
    if entry is None:
        return None
    return entry['median'] * 1000


def writ_run_ms(b):
    """Runtime execution time in ms; returns None if Writ not available for this suite."""
    entry = b.get('writ_run')
    if entry is None:
        return None
    return entry['median'] * 1000


def lang_ms(b, key):
    """Execution time in ms for a non-Writ language; None if key absent."""
    entry = b.get(key)
    if entry is None:
        return None
    return entry['median'] * 1000


def lang_memory_mb(b, key):
    """Peak anonymous RSS in MB; returns 0.0 if not measured or key absent."""
    entry = b.get(key)
    if entry is None:
        return 0.0
    return entry.get('memory_kb', 0) / 1024.0


BENCHMARK_DESCRIPTIONS = {
    'stub': {
        'category': 'Startup',
        'what': 'Process startup overhead (hello world)',
        'why': 'Measures CLI + compiler load + VM initialization cost. Game engines call into scripts frequently; low startup overhead matters.',
    },
    'fib': {
        'category': 'Compute',
        'what': 'Recursive function call overhead via fib(40)',
        'why': 'Tests register-based VM call/return efficiency. Game scripts call many small functions per frame.',
    },
    'sieve': {
        'category': 'Compute',
        'what': 'Sieve of Eratosthenes with 1M elements',
        'why': 'Tests array allocation, indexed mutation, and GC pressure under sustained allocation.',
    },
    'string_concat': {
        'category': 'Data Structures',
        'what': 'String concatenation in a loop (100K iterations)',
        'why': 'Tests string heap management and immutable string copying cost.',
    },
    'array_sort': {
        'category': 'Data Structures',
        'what': 'Quicksort on 100K-element array',
        'why': 'Combined test of function calls, array access, and branching — common patterns in game logic.',
    },
    'hash_map': {
        'category': 'Data Structures',
        'what': 'Hash map insert + lookup (100K entries)',
        'why': 'Tests associative container performance. Note: Writ lacks a Map type, so no Writ entry for this benchmark.',
    },
    'object_create': {
        'category': 'Object System',
        'what': 'Class/struct instantiation in a loop (1M objects)',
        'why': 'Tests `new` allocation throughput and GC pressure — relevant for entity-heavy game scenes.',
    },
    'oop_dispatch': {
        'category': 'Object System',
        'what': 'Virtual/contract method dispatch (100K calls)',
        'why': 'Tests CALL_VIRT + dispatch table lookup cost — Writ uses HashMap-based contract dispatch.',
    },
}

CATEGORY_ORDER = ['Startup', 'Compute', 'Data Structures', 'Object System']


def ratio_str(ms, rust_ms):
    """Format ratio as 'xN.Nx'; returns 'N/A' if rust_ms is zero."""
    if not rust_ms:
        return 'N/A'
    return f'x{ms / rust_ms:.1f}x'


def ratio_to_lua_str(ms, lua_ms, is_lua=False):
    """Format ratio to Lua as 'xN.Nx'; '-' for Lua's own row; 'N/A' if lua_ms absent."""
    if is_lua:
        return '-'
    if not lua_ms:
        return 'N/A'
    return f'x{ms / lua_ms:.1f}x'


# ── Chart generators ──────────────────────────────────────────────────────────

def generate_exec_charts(b, out_dir):
    """Generate two execution-time SVGs for one benchmark suite:
    1. All-languages, log scale  →  exec_{suite}_all.svg
    2. Interpreted-only, linear  →  exec_{suite}_interp.svg
    """
    suite = b['suite']

    def build_chart(langs, logarithmic, title_suffix):
        chart = make_chart(
            f'Execution Time -- {suite} ({title_suffix})',
            'Time (ms)',
            logarithmic=logarithmic,
        )
        names = []
        for name, key in langs:
            if key == 'writ':
                wc = writ_compile_ms(b)
                wr = writ_run_ms(b)
                if wc is None or wr is None:
                    continue
                value = {
                    'value': round(wc + wr, 3),
                    'label': f'compile: {wc:.2f}ms, run: {wr:.2f}ms',
                }
            else:
                ms = lang_ms(b, key)
                if ms is None:
                    continue
                value = round(ms, 3)
            chart.add(name, [value])
            names.append(name)
        chart.x_labels = names
        return chart

    all_chart = build_chart(ALL_LANGS, logarithmic=True, title_suffix='log scale, all languages')
    render_svg(all_chart, out_dir / f'exec_{suite}_all.svg')

    interp_chart = build_chart(INTERP_LANGS, logarithmic=False, title_suffix='interpreted languages')
    render_svg(interp_chart, out_dir / f'exec_{suite}_interp.svg')


def generate_memory_chart(benchmarks, out_dir):
    """Grouped bar chart of memory usage across all suites.

    Output: memory.svg
    """
    chart = make_chart('Memory Usage by Benchmark', 'Memory (MB)')
    chart.x_labels = [b['suite'] for b in benchmarks]

    for name, key in ALL_LANGS:
        values = []
        for b in benchmarks:
            # Use writ_run for Writ (runtime process, not compiler)
            actual_key = 'writ_run' if key == 'writ' else key
            mb = lang_memory_mb(b, actual_key)
            if mb > 0:
                values.append(round(mb, 2))
            else:
                values.append({'value': 0, 'label': 'not measured (process too fast)'})
        chart.add(name, values)

    render_svg(chart, out_dir / 'memory.svg')


def generate_startup_chart(benchmarks, out_dir):
    """Bar chart of startup time per language.

    Uses benchmarks[0]['startup'] (startup is language-level, not suite-level).
    Output: startup.svg
    """
    chart = make_chart('Startup Time by Language', 'Time (ms)')

    startup = benchmarks[0]['startup']
    startup_keys = [
        ('Writ',     'writ_ms'),
        ('Rust',     'rust_ms'),
        ('Lua',      'lua_ms'),
        ('Squirrel', 'squirrel_ms'),
        ('Python',   'python_ms'),
        ('Node.js',  'node_ms'),
    ]
    chart.x_labels = [name for name, _ in startup_keys]
    for name, key in startup_keys:
        chart.add(name, [round(startup.get(key, 0), 3)])

    render_svg(chart, out_dir / 'startup.svg')


def generate_results_md(data, out_dir):
    """Produce narrative RESULTS.md with categories, descriptions, and Lua ratio column.

    Output: RESULTS.md
    """
    benchmarks = data['benchmarks']
    meta = data['meta']

    lines = []
    lines.append(f'# Writ Benchmark Results -- {meta["date"]}')
    lines.append('')

    # ── Methodology section ───────────────────────────────────────────────────
    lines.append('## Methodology')
    lines.append('')
    lines.append(
        f'Platform: `{meta["platform"]}` | '
        f'Runs: {meta["runs"]} | '
        f'Warmup: {meta["warmup"]}'
    )
    lines.append('')
    lines.append('All benchmarks run inside a Docker container for reproducibility.')
    lines.append('')
    lines.append(
        'Writ timings include separate compile and run phases; other interpreted languages '
        'run source directly; Rust is pre-compiled native code.'
    )
    lines.append('')
    lines.append(
        'Ratio to Lua is the primary comparison — Lua is the closest competitor in the game '
        'scripting space. Ratio to Rust is shown for absolute reference.'
    )
    lines.append('')

    # ── Group benchmarks by category ──────────────────────────────────────────
    # Build a map: category -> list of benchmark objects, preserving order within category
    categorized: dict[str, list] = {cat: [] for cat in CATEGORY_ORDER}
    uncategorized: list = []

    for b in benchmarks:
        desc = BENCHMARK_DESCRIPTIONS.get(b['suite'])
        if desc:
            cat = desc['category']
            categorized.setdefault(cat, []).append(b)
        else:
            uncategorized.append(b)

    header = '| Language | Benchmark | Median (ms) | Compile (ms) | Memory (MB) | Ratio to Rust | Ratio to Lua |'
    sep    = '|----------|-----------|-------------|--------------|-------------|---------------|--------------|'

    def emit_benchmark(b):
        suite = b['suite']
        desc = BENCHMARK_DESCRIPTIONS.get(suite)

        lines.append(f'### {suite}')
        lines.append('')
        if desc:
            lines.append(f'**What this measures:** {desc["what"]}')
            lines.append('')
            lines.append(f'**Why it matters for Writ:** {desc["why"]}')
            lines.append('')

        lines.append(header)
        lines.append(sep)

        rust_ms_val = lang_ms(b, 'rust') or 1.0   # guard against zero
        lua_ms_val = lang_ms(b, 'lua')             # may be None

        # Writ row (combined compile+run; compile shown in Compile column)
        wc = writ_compile_ms(b)
        wr = writ_run_ms(b)
        if wc is not None and wr is not None:
            wt = wc + wr
            wm = lang_memory_mb(b, 'writ_run')
            lines.append(
                f'| Writ | {suite} | {wt:.1f} | {wc:.1f} | {wm:.1f} '
                f'| {ratio_str(wt, rust_ms_val)} '
                f'| {ratio_to_lua_str(wt, lua_ms_val)} |'
            )

        # All other languages
        for name, key in [
            ('Rust',     'rust'),
            ('Lua',      'lua'),
            ('Squirrel', 'squirrel'),
            ('Python',   'python'),
            ('Node.js',  'node'),
        ]:
            ms = lang_ms(b, key)
            if ms is None:
                continue
            mb = lang_memory_mb(b, key)
            is_lua = (key == 'lua')
            lines.append(
                f'| {name} | {suite} | {ms:.1f} | - | {mb:.1f} '
                f'| {ratio_str(ms, rust_ms_val)} '
                f'| {ratio_to_lua_str(ms, lua_ms_val, is_lua=is_lua)} |'
            )

        lines.append('')
        lines.append('*Memory values of 0.0 MB indicate the process exited before RSS polling could sample.*')
        lines.append('')

    # Emit benchmarks by category order
    for cat in CATEGORY_ORDER:
        cat_benchmarks = categorized.get(cat, [])
        if not cat_benchmarks:
            continue
        lines.append(f'## {cat}')
        lines.append('')
        for b in cat_benchmarks:
            emit_benchmark(b)

    # Emit uncategorized benchmarks
    if uncategorized:
        lines.append('## Uncategorized')
        lines.append('')
        for b in uncategorized:
            emit_benchmark(b)

    out_path = out_dir / 'RESULTS.md'
    out_path.write_text('\n'.join(lines) + '\n', encoding='utf-8')


# ── Entry point ───────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) != 2:
        print(f'Usage: python3 {sys.argv[0]} <path-to-raw.json>', file=sys.stderr)
        sys.exit(1)

    raw_path = Path(sys.argv[1]).resolve()
    if not raw_path.exists():
        print(f'Error: {raw_path} does not exist', file=sys.stderr)
        sys.exit(1)

    try:
        data = json.loads(raw_path.read_text(encoding='utf-8'))
    except json.JSONDecodeError as exc:
        print(f'Error: {raw_path} is not valid JSON: {exc}', file=sys.stderr)
        sys.exit(1)

    out_dir = raw_path.parent

    benchmarks = data['benchmarks']
    svg_count = 0

    # Per-benchmark execution time charts (REPORT-01)
    for b in benchmarks:
        generate_exec_charts(b, out_dir)
        svg_count += 2

    # Memory usage chart (REPORT-02)
    generate_memory_chart(benchmarks, out_dir)
    svg_count += 1

    # Startup time chart (REPORT-03)
    generate_startup_chart(benchmarks, out_dir)
    svg_count += 1

    # Markdown results table (REPORT-04)
    generate_results_md(data, out_dir)

    print(f'Generated {svg_count} SVG charts + RESULTS.md in {out_dir}')


if __name__ == '__main__':
    main()
