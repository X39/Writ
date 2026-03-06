---
phase: 72-chart-generation-and-results-pipeline
verified: 2026-03-20T18:00:00Z
status: passed
score: 11/11 must-haves verified
re_verification: false
---

# Phase 72: Chart Generation and Results Pipeline Verification Report

**Phase Goal:** Users can run one command from the repo root and get dated SVG bar charts and a markdown results table committed under `benchmark/results/YYYY-MM-DD/`, regenerable from any historical `raw.json`
**Verified:** 2026-03-20
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `generate.py` reads `raw.json` and produces per-benchmark SVG bar charts for execution time (log scale, all languages; linear scale, interpreted only) | VERIFIED | `generate_exec_charts()` exists at line 123; `exec_stub_all.svg` and `exec_stub_interp.svg` present in results dir; SVGs contain `<svg` and `log scale`/`interpreted` in titles |
| 2 | `generate.py` produces memory usage and startup time SVG charts | VERIFIED | `generate_memory_chart()` at line 162, `generate_startup_chart()` at line 185; `memory.svg` and `startup.svg` present in `benchmark/results/2026-03-20/` |
| 3 | `generate.py` produces `RESULTS.md` with language, median_ms, compile_ms, memory_mb, ratio-to-Rust columns | VERIFIED | `generate_results_md()` at line 209; `RESULTS.md` contains `| Language | Benchmark | Median (ms) | Compile (ms) | Memory (MB) | Ratio to Rust |`; Writ row shows `1.0` in compile column; Rust row shows `x1.0x` |
| 4 | All output files land in `benchmark/results/YYYY-MM-DD/` | VERIFIED | `out_dir = raw_path.parent` in `main()` (line 286); all 5 files confirmed present in `benchmark/results/2026-03-20/` |
| 5 | Deleting and re-running `generate.py` produces bit-identical SVG and markdown output | VERIFIED | `no_prefix=True` + `disable_xml_declaration=True` + `_DATE_COMMENT_RE` date comment stripping; no UUID patterns found in SVGs (0 matches for UUID regex); date comment normalized to `<!--Generated with pygal-->` |
| 6 | `run.sh` auto-invokes `generate.py` after container exits; missing python3 shows warning | VERIFIED | Lines 57-67 of `run.sh`: `command -v python3` guard, `python3 "$REPO_ROOT/benchmark/generate.py" "$RESULTS_DIR/raw.json"`, warning with pip hint |
| 7 | `run.ps1` auto-invokes `generate.py` after container exits; missing python3 shows warning | VERIFIED | Lines 47-57 of `run.ps1`: `Get-Command python3 -ErrorAction SilentlyContinue` guard, `& python3` invocation, `Write-Warning` with pip hint |
| 8 | Writ bar in execution charts shows combined compile+run time with compile/run breakdown in tooltip | VERIFIED | Lines 138-144 of `generate.py`: Writ value is `{'value': round(wc + wr, 3), 'label': f'compile: {wc:.2f}ms, run: {wr:.2f}ms'}` |
| 9 | Interpreted-only chart excludes Rust | VERIFIED | `INTERP_LANGS = [lang for lang in ALL_LANGS if lang[0] != 'Rust']` (line 24); 0 occurrences of `"Rust"` in `exec_stub_interp.svg` |
| 10 | `generate.py` is substantive (not a stub) | VERIFIED | 311 lines; all 9 required functions present: `make_chart`, `render_svg`, `writ_total_ms`, `writ_compile_ms`, `writ_run_ms`, `lang_ms`, `lang_memory_mb`, `ratio_str`, `generate_exec_charts`, `generate_memory_chart`, `generate_startup_chart`, `generate_results_md`, `main` |
| 11 | Commits are real (documented in summaries) | VERIFIED | `f9ab5fd`, `53d59b9`, `83c7d5f` all confirmed in git log |

**Score:** 11/11 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `benchmark/generate.py` | SVG chart generation and RESULTS.md table production | VERIFIED | 311 lines; `#!/usr/bin/env python3`; imports `pygal`, `Style`; all 6 chart/table functions implemented |
| `benchmark/results/2026-03-20/exec_stub_all.svg` | All-languages log-scale execution time chart | VERIFIED | Present; contains `<svg`; title includes `log scale, all languages`; 6 language series |
| `benchmark/results/2026-03-20/exec_stub_interp.svg` | Interpreted-only linear-scale execution time chart | VERIFIED | Present; contains `<svg`; title includes `interpreted languages`; Rust absent from series |
| `benchmark/results/2026-03-20/memory.svg` | Memory usage comparison chart | VERIFIED | Present; contains `<svg`; title `Memory Usage by Benchmark` |
| `benchmark/results/2026-03-20/startup.svg` | Startup time comparison chart | VERIFIED | Present; contains `<svg`; title `Startup Time by Language` |
| `benchmark/results/2026-03-20/RESULTS.md` | Markdown results table | VERIFIED | Present; correct table header; Writ shows compile_ms; Rust shows `x1.0x`; `not measured` note present |
| `benchmark/runner/run.sh` | POSIX sh launcher with generate.py integration | VERIFIED | `#!/bin/sh`, `set -eu` preserved; `command -v python3` guard; generate.py invoked; warning if absent |
| `benchmark/runner/run.ps1` | PowerShell launcher with generate.py integration | VERIFIED | `Get-Command python3 -ErrorAction SilentlyContinue`; `& python3` invocation; `Write-Warning` if absent |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `benchmark/generate.py` | `benchmark/results/2026-03-20/raw.json` | `json.loads(raw_path.read_text())` in main() | WIRED | Line 281 reads raw.json; `out_dir = raw_path.parent` ensures output co-located |
| `benchmark/generate.py` | `benchmark/results/2026-03-20/*.svg` | `render_svg(chart, out_dir / 'exec_{suite}_all.svg')` | WIRED | `render_svg()` called 4+ times with `out_dir` path; files confirmed present |
| `benchmark/generate.py` | `benchmark/results/2026-03-20/RESULTS.md` | `out_path.write_text(...)` in generate_results_md() | WIRED | Line 264-265; `RESULTS.md` written to `out_dir / 'RESULTS.md'` |
| `benchmark/runner/run.sh` | `benchmark/generate.py` | `python3 "$REPO_ROOT/benchmark/generate.py" "$RESULTS_DIR/raw.json"` | WIRED | Line 60 of run.sh; guarded by `command -v python3` |
| `benchmark/runner/run.ps1` | `benchmark/generate.py` | `& python3 "$RepoRoot\benchmark\generate.py" "$ResultsDir\raw.json"` | WIRED | Line 50 of run.ps1; guarded by `Get-Command python3` |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| REPORT-01 | 72-01 | SVG bar charts generated for each benchmark category (execution time) | SATISFIED | `generate_exec_charts()` produces `exec_stub_all.svg` (log scale, all languages) + `exec_stub_interp.svg` (linear, interpreted only) |
| REPORT-02 | 72-01 | SVG bar chart for memory usage comparison | SATISFIED | `generate_memory_chart()` produces `memory.svg`; confirmed present with valid SVG |
| REPORT-03 | 72-01 | SVG bar chart for startup time comparison | SATISFIED | `generate_startup_chart()` produces `startup.svg`; confirmed present with valid SVG |
| REPORT-04 | 72-01 | Markdown table generated with all metrics for README embedding | SATISFIED | `generate_results_md()` produces `RESULTS.md` with Language/Benchmark/Median/Compile/Memory/Ratio-to-Rust columns; all 6 languages present |
| REPORT-05 | 72-02 | Charts and tables committed to `benchmark/results/` | SATISFIED | Output files in `benchmark/results/2026-03-20/`; run.sh and run.ps1 auto-invoke generate.py after container exits; commits `53d59b9` and `83c7d5f` confirmed |

**Orphaned requirements check:** REPORT-06 (historical trend charts) is mapped in REQUIREMENTS.md but listed as a future/deferred requirement, not assigned to Phase 72. No orphan gap.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | No anti-patterns found |

Scanned: `benchmark/generate.py`, `benchmark/runner/run.sh`, `benchmark/runner/run.ps1`
Checked for: TODO/FIXME/XXX/HACK/PLACEHOLDER, empty return bodies, console.log-only stubs. None found.

---

### Human Verification Required

#### 1. Visual SVG rendering quality

**Test:** Open `benchmark/results/2026-03-20/exec_stub_all.svg` in a browser, `memory.svg`, `startup.svg`, and `exec_stub_interp.svg`.
**Expected:** Bars render with correct colors (Writ=purple, Rust=orange, Lua=blue, Squirrel=teal, Python=gold, Node.js=green), readable axis labels, legend visible, title legible.
**Why human:** SVG pixel rendering, font fallback, legend layout correctness cannot be verified programmatically.

#### 2. Tooltip content on Writ bars

**Test:** Hover over the Writ bar in `exec_stub_all.svg` in a browser (requires pygal-tooltips.min.js CDN to load).
**Expected:** Tooltip shows `compile: 1.02ms, run: 0.67ms` (or current raw.json values).
**Why human:** JavaScript-rendered tooltip behavior requires browser interaction.

#### 3. One-command end-to-end (Docker required)

**Test:** Run `sh benchmark/runner/run.sh` from the repo root on Linux/macOS with Docker installed.
**Expected:** Docker image builds, container runs all benchmarks, container exits, python3 automatically invoked, charts and RESULTS.md appear in `benchmark/results/YYYY-MM-DD/`.
**Why human:** Requires Docker daemon; cannot run in static analysis.

---

### Gaps Summary

No gaps. All automated checks passed. The phase goal is fully achieved:

- `benchmark/generate.py` is substantive (311 lines), reads `raw.json`, and produces all 4 SVG charts plus `RESULTS.md` in the correct dated directory
- All 5 REPORT requirements (REPORT-01 through REPORT-05) are satisfied
- Determinism mechanisms are in place (`no_prefix=True`, `disable_xml_declaration=True`, `_DATE_COMMENT_RE` stripping); no UUID patterns found in output SVGs
- Both runner scripts (`run.sh` and `run.ps1`) are wired to auto-invoke `generate.py` with graceful python3 availability guard
- All 3 documented commits (`f9ab5fd`, `53d59b9`, `83c7d5f`) confirmed in git log

The only items flagged for human review are visual/interactive concerns that cannot be verified statically (SVG rendering quality, tooltip behavior, full Docker run).

---

_Verified: 2026-03-20T18:00:00Z_
_Verifier: Claude (gsd-verifier)_
