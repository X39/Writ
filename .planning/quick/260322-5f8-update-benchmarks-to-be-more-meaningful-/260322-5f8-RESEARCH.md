# Quick Task: Update Benchmarks — Research

**Researched:** 2026-03-22
**Domain:** Benchmark methodology, warmup, expressiveness
**Confidence:** HIGH

## Summary

The current benchmark suite has three key problems: (1) only 3 runs with 2 warmup iterations, producing statistically weak results; (2) the raw.json only contains the `stub` benchmark (the other 6 cases appear to fail silently inside Docker, so RESULTS.md only shows stub data); and (3) the results presentation lacks narrative — it shows numbers but does not explain what the benchmarks are designed to reveal about Writ's VM characteristics.

**Primary recommendation:** Increase warmup to 5 and runs to 15-20, fix the missing benchmark results (likely runtime failures for fib/sieve/string_concat/array_sort/object_create/oop_dispatch), and add a "What This Measures" section to RESULTS.md for each benchmark.

## Current State Analysis

### Benchmark Configuration
| Parameter | Current | Problem |
|-----------|---------|---------|
| `WARMUP` | 2 | Too low — filesystem cache, CPU frequency scaling, and branch predictor warmup are not saturated in 2 iterations |
| `RUNS` | 10 (default), but ran with 3 | 3 runs is statistically meaningless — cannot compute reliable confidence intervals |
| Suites measured | 7 defined, 1 reported | Only `stub` appears in raw.json; the 6 real benchmarks produced no results |

### Seven Benchmark Cases
| Case | What It Measures | Iterations | Writ-Specific Purpose |
|------|-----------------|------------|----------------------|
| `stub` | Startup overhead only (hello world) | 1 | CLI + compiler load + VM init cost |
| `fib` | Recursive function call overhead | fib(40) ~2^40 calls | Register-based VM call/return efficiency |
| `sieve` | Array allocation + indexed mutation | 1M elements | Array heap allocation, GC pressure |
| `string_concat` | String concatenation in loop | 100K concats | String heap management, immutable string copying |
| `array_sort` | Quicksort with swaps | 100K elements | Function calls + array access + branching |
| `object_create` | Class instantiation in loop | 1M objects | `new` + GC allocation throughput |
| `oop_dispatch` | Contract (virtual) dispatch | 100K dispatches | CALL_VIRT + HashMap dispatch table cost |
| `hash_map` | Map insert + lookup | 100K entries | **No Writ implementation** (only .js/.lua/.nut/.py/.rs) |

### Critical Finding: Missing Results
The raw.json only contains the `stub` suite. This means either:
1. The Writ benchmarks crash at runtime (most likely — known bugs like StrLen, closure capture)
2. The other language runtimes fail too and hyperfine reports failure

The `hash_map` case has no `.writ` file at all — Writ lacks a Map/HashMap type.

## Warmup Best Practices

**Confidence: HIGH** — based on hyperfine documentation and benchmark methodology standards.

### What Warmup Does
Hyperfine's `--warmup N` runs the command N times before measuring. This addresses:
1. **Filesystem cache**: First run reads from disk; subsequent runs hit page cache
2. **CPU frequency scaling**: Modern CPUs ramp up frequency under load (turbo boost)
3. **DNS/network**: Not relevant here (all local)
4. **Branch predictor training**: CPU learns branch patterns after first few runs

### Recommended Values
| Parameter | Recommended | Rationale |
|-----------|-------------|-----------|
| `--warmup` | 5 | Saturates filesystem cache and CPU frequency scaling; diminishing returns after 5 |
| `--runs` | 15-20 | Produces stable median and meaningful MAD; hyperfine defaults to 10 which is a bare minimum |
| `--min-runs` | 10 | Use `--min-runs` instead of `--runs` to let hyperfine auto-detect when results stabilize |

### Additional hyperfine Flags to Consider
| Flag | Purpose | When to Use |
|------|---------|-------------|
| `--min-runs 10` | Run at least 10, more if variance is high | Replace fixed `--runs` |
| `--show-output` | Show benchmark stdout (useful for debugging failures) | Debug mode |
| `--prepare 'sync; echo 3 > /proc/sys/vm/drop_caches'` | Drop caches before each run | Cold-cache measurement (probably not wanted) |
| `--ignore-failure` | Continue even if command returns non-zero | Prevents silent benchmark dropout |

## Expressiveness Improvements

### Problem: "What story do the numbers tell?"
Current RESULTS.md is a raw data dump. A reader sees "Writ fib: 250ms, Lua fib: 800ms" but cannot answer:
- Is Writ fast or slow *for an interpreted language*?
- Which VM subsystem is the bottleneck?
- What do the ratios mean for game scripting use cases?

### Recommended Structure for RESULTS.md

1. **Executive Summary** at top: "Writ is Nx Lua, Nx Python for compute-bound tasks. Object creation is the bottleneck due to GC pressure."

2. **Per-benchmark narrative**: Each section should state:
   - **What this measures**: "Tests function call overhead via recursive Fibonacci"
   - **Why it matters for Writ**: "Game scripts call many small functions per frame"
   - **Key finding**: "Writ is 2x slower than Lua here due to register save/restore overhead"

3. **Grouped categories** instead of flat list:
   - **Startup**: stub
   - **Compute**: fib, sieve
   - **Data structures**: array_sort, hash_map, string_concat
   - **Object system**: object_create, oop_dispatch

4. **Relative performance chart**: Show ratios normalized to Lua (the most comparable competitor) rather than Rust (which is unfairly fast as a compiled language).

### Chart Improvements
- Add a **radar/spider chart** showing Writ's relative performance across categories
- The "ratio to Rust" column is misleading — Rust is compiled native code. Use "ratio to Lua" as the primary comparison since Lua is the closest competitor in the game scripting space
- Add a **compile vs. run breakdown** bar (stacked) so readers see the compilation tax

## Common Pitfalls

### Pitfall 1: Silent Benchmark Failures
**What goes wrong:** Benchmarks fail but `|| true` swallows errors, producing empty results
**Current evidence:** Only `stub` in raw.json despite 7 defined cases
**Fix:** Add `--show-output` during development; use `set -x` to trace failures; check writ compile and writ run exit codes explicitly before benchmarking

### Pitfall 2: Measuring Process Startup Instead of Algorithm
**What goes wrong:** For short benchmarks, process fork/exec dominates the measurement
**Current evidence:** Stub benchmark shows Writ at 1.7ms — the actual work is near zero
**Fix:** For the real benchmarks this is fine (fib(40) takes seconds), but ensure workloads are large enough that startup is <1% of total time

### Pitfall 3: Memory Polling Race
**What goes wrong:** `measure_anon_rss` polls /proc/pid/status but fast processes exit before first sample
**Current evidence:** Multiple 0 KB memory readings in raw.json
**Fix:** Increase iteration counts so processes live longer, or use `/usr/bin/time -v` which captures peak RSS via wait4()

### Pitfall 4: Comparing Apples to Oranges
**What goes wrong:** Writ measures compile+run combined, but Rust is pre-compiled. Node.js JIT-compiles internally.
**Fix:** Already partially addressed by showing compile/run separately. Make this distinction prominent in RESULTS.md narrative.

## Actionable Changes

### Runner Changes (bench_runner.sh)
1. Change defaults: `WARMUP="${WARMUP:-5}"`, `RUNS="${RUNS:-15}"`
2. Add `--ignore-failure` to hyperfine invocations to prevent suite dropout
3. Replace `measure_anon_rss` polling with `/usr/bin/time -v` and parse "Maximum resident set size" (more reliable, captures peak even for short processes)
4. Add validation after each suite: if writ_run_json is null, log the actual error

### Generator Changes (generate.py)
1. Add executive summary paragraph at top of RESULTS.md
2. Add per-benchmark "What This Measures" descriptions
3. Add "ratio to Lua" column alongside "ratio to Rust"
4. Group benchmarks by category (startup / compute / data / objects)
5. Add a stacked bar chart showing compile vs. run time for Writ

### Case Changes
1. Fix whatever is causing the 6 non-stub benchmarks to fail (investigate in Docker)
2. Remove or stub `hash_map` since Writ has no Map type yet
3. Add descriptions as comments at top of each .writ file

### Presentation
1. Rename RESULTS.md sections from raw suite names to descriptive titles
2. Add a "Methodology" section explaining warmup, runs, Docker environment
3. Add a "Key Takeaways" section with 3-5 bullet points

## Sources

### Primary (HIGH confidence)
- Existing benchmark code in `benchmark/` directory — full read of all files
- raw.json results from 2026-03-20 run — only stub suite populated
- hyperfine documentation (built-in `--help` flags) for warmup/runs semantics

### Secondary (MEDIUM confidence)
- Standard benchmarking methodology (warmup saturation at 5 iterations, minimum 10 runs for statistical significance) — well-established in performance engineering literature
