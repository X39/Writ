# Pitfalls Research

**Domain:** Cross-language benchmark suite — adding Docker-containerized, CI-integrated, chart-generating benchmarks comparing Writ against Lua, Squirrel, Python, Node.js, and native Rust to an existing language toolchain.
**Researched:** 2026-03-20
**Confidence:** HIGH — all critical pitfalls are grounded in direct inspection of the Writ PROJECT.md (v7.0 milestone targets), current Docker/cgroup documentation, V8/Node.js tier compilation documentation, CI benchmark noise research (2025-2026), Squirrel repository commit history, and statistical benchmarking literature. See Sources section.

---

## Critical Pitfalls

### Pitfall 1: Benchmark Workloads That Favor Different Paradigms Produce Meaningless Results

**What goes wrong:**
Each language in the comparison (Writ, Lua, Squirrel, Python, Node.js, Rust) has different idiomatic patterns. If the benchmark author writes OOP-centric workloads and implements them OOP-style in Python and Rust but uses flat procedural code in Lua and Squirrel (because that is more natural there), the comparison is not measuring language performance — it is measuring the author's familiarity with each language. Similarly, a string-processing benchmark that uses Python's C-extension `str.join()` against Writ's manually looped concatenation is comparing a C function to interpreted bytecode, not comparing language performance.

The four categories in v7.0 (compute-heavy, string processing, data structures, OOP/dispatch) are especially prone to this. OOP/dispatch benchmarks require object allocation and virtual call chains in every language. If Lua and Squirrel use metatables to approximate OOP and Python/Node.js use native classes, the overhead profiles are entirely different, not because Lua is faster or slower, but because the implementation strategies differ.

**Why it happens:**
Benchmark authors write fluent code in languages they know and "good-enough" code in languages they don't. The unfairness is invisible because each implementation runs correctly, just at different speeds for unrelated reasons.

**How to avoid:**
For each benchmark category, define a canonical algorithm description that is paradigm-neutral — specify the exact data structure, the exact sequence of operations, and the exact output expected. Then implement each language version by following the canonical algorithm literally, not the idiomatic pattern. Have someone review each non-primary language implementation for "this doesn't actually do the same thing." For OOP/dispatch specifically: count object allocations and virtual dispatch calls; they must be identical across languages. Use the "Are We Fast Yet?" benchmark design principle: the same algorithmic complexity, the same number of operations.

Additionally: string concatenation benchmarks must be structured so each language uses the same approach (e.g., building a result string via repeated append inside a loop), not the library shortcut available only in some languages. If `str.join` is used in Python, use the equivalent in every language; if no equivalent exists, use the loop form in all.

**Warning signs:**
- One language runs a "string processing" benchmark 50x faster than all others. Likely explanation: that language is using a C-extension for the inner operation, not its interpreter.
- Rust "native" finishes compute benchmarks in 1ms but interpreted languages take 500ms. If the Rust task is computing the exact same algorithm with the same number of arithmetic operations, this is expected and correct; if Rust uses SIMD auto-vectorization that the spec does not, it is not.
- Different languages allocate wildly different numbers of objects in the OOP benchmark (detectable by adding an allocation counter to the host).

**Phase to address:** Benchmark design phase (first phase) — workload definitions must be locked before any language implementation is written.

---

### Pitfall 2: Measuring Node.js Before V8 JIT Has Fully Warmed Up

**What goes wrong:**
Node.js/V8 uses a multi-tier JIT pipeline: Ignition (bytecode interpreter) → Sparkplug (fast baseline JIT) → Maglev (mid-tier optimizing JIT) → TurboFan (full optimizing JIT). A function that is called once runs in Ignition. A function called ~50 times gets Sparkplug-compiled. A function called ~200 times gets Maglev. TurboFan kicks in later for "hot" functions. If the benchmark measures the first N iterations before TurboFan optimization, the result reflects the interpreter tier, not the steady-state JIT performance.

For short benchmarks (e.g., run loop 1000 times), the JIT may never reach TurboFan. The measured performance is 5-10x slower than steady-state. Since the other comparison targets (Writ, Lua, Python) are interpreters that do not JIT, a "fair" short-run benchmark actually undercounts Node.js performance relative to what a real application would see.

V8 also inlines decisions and speculative type feedback at the point of JIT compilation. If the benchmark passes only `int` values during warmup and then passes a `float` during measurement, V8 deoptimizes and falls back to a slower path. The deoptimization spike appears as a massive outlier.

**Why it happens:**
Benchmark scripts written as `for (let i = 0; i < N; i++) { doWork(); }` start measuring from iteration 0, before any JIT warmup. The author sees a median time and does not realize they are measuring the interpreter tier.

**How to avoid:**
For Node.js specifically: run a warmup loop of at least 200-500 iterations (enough to reach TurboFan for the inner function) before starting the timed measurement. Use the `--allow-natives-syntax` flag in development to call `%OptimizeFunctionOnNextCall(fn)` and verify the function is actually TurboFan-compiled before timing. Alternatively, use a framework like `bench-node` (the official Node.js benchmarking library by Rafael Gonzaga) which handles warmup automatically.

Structure the benchmark as: warmup phase (not timed) → measurement phase (timed). Report only the measurement phase.

Note: Node.js v22.9.0 to current disables Maglev by default in some configurations; verify which JIT tier is active for the Node.js version used in the Docker container.

**Warning signs:**
- Node.js benchmark times drop significantly between the first 100 and next 100 iterations.
- Adding `--jitless` flag changes Node.js performance by less than 10% (indicates TurboFan was not active without it either).
- The Node.js result has much higher variance than Lua results for the same workload.

**Phase to address:** Benchmark harness phase — the warmup protocol must be specified per language before any measurements are taken.

---

### Pitfall 3: LuaJIT and PUC Lua Are Not Interchangeable — Picking One Without Disclosing Which

**What goes wrong:**
LuaJIT can be 5-30x faster than PUC Lua 5.4 for numeric-heavy workloads but can be slower for some string operations and modern Lua constructs. LuaJIT does not support Lua 5.4 semantics (it implements Lua 5.1 with some 5.2 extensions), so integer division `//`, bitwise operators in Lua 5.3+ style, and `<close>` variables are not available in LuaJIT. If benchmark implementations use PUC Lua 5.4 syntax but are labeled "Lua (LuaJIT)" in the chart, the benchmark is invalid — the code will not even run under LuaJIT without modifications.

Conversely, installing LuaJIT in the Docker container and calling the binary `lua` makes it appear as if "Lua" is faster than it actually is for the target game engine use case (Writ's primary comparator), since most game scripting hosts embed PUC Lua, not LuaJIT.

**Why it happens:**
On Ubuntu/Debian, `apt install lua5.4` installs PUC Lua and `apt install luajit` installs LuaJIT. Both produce a `lua`-like binary (`lua5.4` and `luajit`). Scripts that accidentally invoke the wrong one produce valid output but at a different performance tier.

**How to avoid:**
Include both LuaJIT and PUC Lua 5.4 as separate benchmark targets, or pick exactly one and label it explicitly in every chart and table. Recommend: use PUC Lua 5.4 as the primary target (most representative of embedded game scripting use), with LuaJIT as an optional second series. Write all Lua benchmark implementations in Lua 5.4 syntax only. In the Docker container, pin versions: `lua5.4` and optionally `luajit`. Verify the binary invoked in the runner script matches the intended implementation.

Write a build-time check that prints the Lua version string at benchmark start and fails if the version does not match the expected one.

**Warning signs:**
- Lua benchmark code uses `//` (integer division) but the LuaJIT binary is invoked — this will produce a syntax error, and if errors are silently swallowed, the benchmark will report a zero or fallback value.
- The reported Lua performance changes dramatically between CI runs because the PATH lookup finds different `lua` binaries depending on container layer ordering.

**Phase to address:** Docker environment phase — language version pinning must be locked before any cross-language measurements are taken.

---

### Pitfall 4: Squirrel Has No Package Manager and Must Be Built from Source in Docker

**What goes wrong:**
Squirrel (`sq`) is not available in the standard Ubuntu `apt` repositories for current Ubuntu LTS versions in a ready-to-use form. The `squirrel3` package exists in Debian testing/unstable, and as of Debian Bookworm (2022) it is in Debian stable, but Ubuntu LTS images (22.04, 24.04) may not have a packaged `sq` binary that matches the version expected by the benchmarks. The official Squirrel repository (`albertodemichelis/squirrel`) builds from source using CMake with no release artifacts published to a package registry.

Building Squirrel from source inside a Docker container adds 2-5 minutes to the image build time (CMake, C++ compiler, full build), and the build may fail on arm64 Docker images (GitHub Actions arm64 runners) if arm64 cross-compilation is not configured.

The Squirrel project is low-activity: the most recent commit on the official repository was February 2026, with months between commits. This means the last tagged release may be years old and the HEAD may have unfixed build issues.

**Why it happens:**
Game scripting language benchmarks traditionally include Squirrel due to its historical use in games (Left 4 Dead 2, Portal 2, Unreal Engine). Benchmark authors include it without verifying it can actually be installed reproducibly.

**How to avoid:**
Check the Debian/Ubuntu package availability for `squirrel3` in the target base image before committing to it as a benchmark target. If the package is unavailable, either: (a) pin a known-working source commit and build in a Docker stage with CMake, caching the build layer, or (b) use a pre-built binary published in the benchmark repository itself. Add a version-check assertion at container startup: `sq -v` must return the expected version string. Include a `Dockerfile.squirrel` build stage that caches the Squirrel build separately from the main benchmark image to avoid rebuilding on every benchmark run.

If Squirrel cannot be built reproducibly, drop it from the Docker runner and document it as "Squirrel: not available in CI — run locally." This is better than a broken or silent result.

**Warning signs:**
- Docker build succeeds but `sq` binary is missing from the final image (CMake build step silently exited non-zero but the Dockerfile's `RUN` chain continued due to `||` or missing `set -e`).
- The benchmark runs but Squirrel results are all 0ms (the runner invoked a missing binary without checking the exit code).
- ARM64 CI runners fail to build Squirrel because the CMakeLists.txt does not handle 64-bit pointer assumptions.

**Phase to address:** Docker environment phase — Squirrel build must be validated before any benchmarks that include it.

---

### Pitfall 5: Docker Memory Measurement Reports Resident Set Size Including OS Page Cache

**What goes wrong:**
When measuring "memory usage" inside a Docker container, the most accessible metric is `docker stats` or reading `/proc/self/status`. Both report the Resident Set Size (RSS), which includes file system page cache on Linux. For languages that load their standard library from disk at startup (Python, Node.js), the first run fills the page cache, and RSS includes those cached pages. Subsequent runs may show lower RSS if the cache is warm. The difference can be 10-50MB, making it appear that Python uses wildly different memory across runs.

Additionally, on cgroup v2 (default on Ubuntu 22.04+ and recent Debian), `memory.current` reports total memory including page cache. The correct metric for "heap memory used by the program" is `memory.stat` → `anon` (anonymous memory: heap + stack). Reporting `memory.current` as "memory usage" overstates all interpreted language memory usage by the size of their loaded standard library.

A separate issue: Docker GPU benchmarks (not applicable here) double-count GPU allocations, but the analogous problem for this project is that the Writ runtime's `MarkSweepHeap` GC reports heap size at the moment of measurement, which may be after the last GC sweep (low) or just before a GC collection (high). Measuring "peak heap" vs "live heap at end" gives different numbers.

**Why it happens:**
`/proc/self/status VmRSS` is the most visible and commonly cited memory metric, and documentation rarely clarifies the page-cache inclusion problem. cgroup v2 changed how metrics are exposed and the old documentation examples all use cgroup v1 paths.

**How to avoid:**
For all process-level memory measurements, read `VmRSS - RssFile - RssShmem` from `/proc/self/status`, which gives anonymous RSS only (heap + stack, excludes page cache). On cgroup v2, read `/sys/fs/cgroup/memory.stat` → `anon` field. Write a shared shell function `measure_anon_rss()` used by all language runner scripts so the measurement method is identical across languages.

For the Writ runtime specifically: expose a `Domain::heap_stats()` method that reports total allocated bytes as tracked by `MarkSweepHeap`, and report this alongside the OS-level memory metric as a separate "writ heap" column.

Document explicitly in the benchmark results README which memory metric is reported and what it excludes.

**Warning signs:**
- Python memory usage varies by 30-40MB between the first and second run of the same benchmark (page cache effect).
- Writ memory use reported by the OS is higher than Rust native by an unexpected amount, possibly because the Writ binary itself is larger and its code segment contributes to RSS.
- Memory values on CI are different from local Docker runs because the CI runner has less available memory, causing the kernel to evict more cached pages.

**Phase to address:** Benchmark harness phase — the measurement method for memory must be standardized before any language comparison is published.

---

### Pitfall 6: Python Startup Overhead Dominates Short-Running Benchmarks

**What goes wrong:**
Python's interpreter startup time is 50-150ms on a cold start (importing standard modules, initializing the bytecode VM, loading `sys.path` entries). For a benchmark that runs 1000 iterations of a tight loop in 20ms, the startup overhead is 7x the actual work. If the runner script measures wall-clock time from `subprocess.run(["python3", "bench.py"])` start to exit, the reported time is dominated by interpreter startup, not the benchmark computation.

This makes Python appear much slower than it is for the specific workload. Conversely, Node.js has even higher startup overhead (~200-400ms for a minimal script) but its JIT makes long-running workloads faster. Measuring startup-dominated time inverts the expected ordering for short workloads.

Rust native has essentially no startup overhead but does load its runtime libraries (dynamically linked) which contributes a few milliseconds.

**Why it happens:**
Shell-level timing (`time python3 bench.py`) naturally includes startup. For batch scripts where the benchmark runs in a subprocess, the harness measures the outer wall clock time.

**How to avoid:**
Measure startup time separately as an explicit metric (as the v7.0 spec already plans — "3 metrics: execution time, memory usage, startup time"). For the "execution time" metric, separate it from startup time by: (a) having each language's benchmark script self-time internally and print a structured result (e.g., `RESULT: 42.3ms`), then the harness reads that value rather than measuring the subprocess wall clock; or (b) running the benchmark workload multiple iterations within a single process invocation, so startup is amortized. Report startup time as its own chart column.

For Python specifically: using `python3 -S` (no site-packages import) reduces startup from ~100ms to ~30ms; consider using this for the pure computation benchmarks where standard library access is not needed.

**Warning signs:**
- Python results for "compute-heavy" benchmarks are slower than Lua by 5-10x, but Lua is known to have a slower VM. Likely cause: startup overhead dominates.
- Adding `time.sleep(0)` to the Python benchmark changes the wall-clock result by 0% (indicating the measured time is mostly startup, not computation).
- The "startup time" and "execution time" results are identical for Python (the harness is not separating them).

**Phase to address:** Benchmark harness phase — self-timing within each language's script must be designed before any language implementations are written.

---

### Pitfall 7: GitHub Actions Shared Runners Have 10-30% Timing Variance

**What goes wrong:**
GitHub Actions standard (free) runners are Azure VMs shared with other users. Hypervisor scheduling, noisy neighbor I/O activity, and CPU frequency scaling can cause the same benchmark to report times that differ by 10-30% between runs on different runners, or even different jobs on the same run. A benchmark that runs in 200ms in one CI job may run in 260ms in the next job triggered by a git push, appearing as a 30% regression when no code changed.

For a benchmark suite whose results are committed to `benchmark/results/`, this variance means the committed results will be meaningless for longitudinal comparison — every re-run will overwrite the previous results with values that differ by more than most real regressions.

Published data (RunsOn, 2025) shows that standard GitHub-hosted runners can report CPU performance differences of 20%+ between identical jobs. Azure also occasionally provisions faster SKUs (newer CPU generations), making it appear that performance improved when it was just a hardware assignment change.

**Why it happens:**
Virtual machines on shared hypervisors do not have dedicated CPU cores. The CPUID reported by Azure Hyper-V reflects an older generation than the actual hardware in some cases, making performance appear inconsistent.

**How to avoid:**
Do not commit benchmark numbers from standard GitHub Actions runners as authoritative results. Use CI benchmarks only for regression detection (comparing current run against previous run's median), not for absolute performance reporting.

For publishable absolute numbers: run on a self-hosted runner with pinned CPU frequency (`cpufreq-set -g performance`) or on a dedicated VM. Alternatively, disable the CI-commit step and document that authoritative results must be generated locally in the Docker container.

If CI must produce results: run each benchmark 5-10 times within the same job (same runner), discard the top and bottom values, and report the trimmed mean. Use a regression threshold of at least 15% before flagging a regression (CodSpeed's published guidance for standard runners is 15% threshold for sub-1% false positive rate).

**Warning signs:**
- The CI benchmark job reports times that vary by >15% between consecutive runs with identical code.
- Committed `benchmark/results/` files show irregular spikes in results history that correlate with nothing in the code history.
- The "OOP/dispatch" benchmark (which is more sensitive to cache behavior) shows 2x variance while the compute benchmark shows only 5% variance.

**Phase to address:** CI workflow phase — the variance threshold and result-commit policy must be defined before the GitHub Actions workflow is written.

---

### Pitfall 8: Writ Compile Time Is Included in "Execution Time" for Interpreted Languages, Making the Comparison Unfair

**What goes wrong:**
The v7.0 spec correctly plans to "report Writ compile time and runtime separately." However, this is harder to implement correctly than it appears. When measuring Lua, Python, Node.js, and Squirrel, the runner invokes the interpreter directly on source files — there is no separate compilation step. Writ requires `writ compile` (producing a `.writc` file) followed by `writ run`. If the benchmark measures `writ compile + run`, it is comparing Writ's compiler + VM against Python's VM alone.

The comparison becomes: Writ (compile + execute) vs Lua (execute). This makes Writ look 3-10x slower for any workload where compilation overhead exceeds execution time, which is likely for short benchmarks.

Conversely, Python, Lua, and Squirrel do perform "compilation" (to bytecode) on first load, but this is milliseconds. If these startup bytecode compilation costs are not counted but Writ's full AOT compilation is counted, the comparison is again unfair — but in the opposite direction when compilation produces better-optimized code.

**Why it happens:**
The simplest implementation measures subprocess wall clock time for all languages, which inherently includes all startup work. Separating Writ compile time requires two separate subprocess invocations (or a single invocation that prints both timings).

**How to avoid:**
Define three explicitly-named metrics in the benchmark output schema:
- `startup_ms`: time from process start to first instruction of benchmark code (includes interpreter init, source loading, bytecode compilation for all languages; includes `writ compile` for Writ).
- `execution_ms`: time for the benchmark loop/workload to complete, self-reported by the benchmark script internally.
- `total_ms`: startup + execution.

For Writ specifically: run `writ compile` once as a pre-measurement step, report its time as `compile_ms`, then run `writ run` N times and report only the run time as `execution_ms`. This way Writ `execution_ms` is comparable to other languages' `execution_ms`.

The "Writ compile time" column is then a separate publishable metric showing compilation overhead, which is informative and honest.

**Warning signs:**
- Writ "execution time" bars in the chart are consistently 3-5x taller than Lua bars for the same benchmark, but Writ VM instruction throughput (opcodes per second) tests show it is within 2x of Lua. The discrepancy is explained by compilation overhead being included.
- The benchmark harness has a single `run_benchmark(language, script)` function that does not have a Writ-specific code path.

**Phase to address:** Benchmark harness phase — the two-step Writ measurement protocol must be designed at the same time as the harness, not added later.

---

### Pitfall 9: SVG Chart Y-Axis Truncation Makes Small Differences Look Large

**What goes wrong:**
Benchmark charts with a truncated Y-axis (not starting at zero) make a 5% performance difference between Writ and Lua look like Writ is 3x slower, because the bar for the faster language appears 3x shorter when the axis starts at 190ms instead of 0ms. This pattern is common in marketing charts and misleads readers about the practical significance of performance differences.

For a benchmark suite intended to be "publishable" and included in the README, a truncated-axis chart damages credibility when readers notice it. It is also common to choose a log scale for charts where values span multiple orders of magnitude (e.g., Rust: 1ms, Writ: 40ms, Python: 800ms) — log scale is correct here, but must be clearly labeled, because readers often interpret a log-scale bar chart as linear.

A related issue: including Rust native in the same bar chart as interpreted languages on a linear scale makes all interpreted language bars appear nearly identical in height (all 400-800ms) with Rust as a tiny sliver at the bottom (1-2ms). The information content of the chart is near zero for the interpreted language comparison.

**Why it happens:**
Charting libraries default to fitting the data range, not starting at zero. Matplotlib, Chart.js, and most SVG charting tools will auto-scale the Y-axis to the data range unless explicitly told not to. Developers generate the chart without inspecting the axis bounds.

**How to avoid:**
Always set Y-axis minimum to 0 for bar charts. For charts that include Rust native alongside interpreted languages, generate two charts: one showing all languages with a log scale (clearly labeled "log scale"), and one showing only the interpreted/VM-based languages on a linear scale. Add axis labels and units to every chart. Include error bars (or min/max whiskers) showing measurement variance so readers can judge whether differences are meaningful.

Generate charts programmatically from structured JSON/TOML result files — never manually. This ensures the axis configuration is in version-controlled code, not an ephemeral matplotlib session.

**Warning signs:**
- The generated SVG chart shows bar heights that are visually 3-5x different, but the actual values differ by less than 20%.
- Rust is invisible in the chart because all other bars are 400x taller.
- No units are labeled on the Y-axis (the reader cannot tell if the numbers are ms, us, or ns).

**Phase to address:** Chart generation phase — chart configuration must encode explicit axis-zero policy and log-scale labeling policy before any charts are generated.

---

### Pitfall 10: Statistical Invalidity from Insufficient Iterations and No Outlier Handling

**What goes wrong:**
A benchmark that runs each workload only 5 times and reports the mean cannot distinguish signal from noise, especially in a Docker container on shared CI infrastructure. A single GC pause, OS context switch, or JIT deoptimization event in one of those 5 runs will inflate the mean by 20-50%. The result looks authoritative (a single number in a chart) but is statistically meaningless.

Conversely, running 10,000 iterations of a "compute-heavy" benchmark takes longer than the CI job timeout. For a benchmark that takes 500ms per iteration, 10,000 iterations is 83 minutes.

The v7.0 spec plans 3 metrics but does not specify iteration counts or statistical treatment. Without specifying these, different benchmark categories (which have wildly different per-iteration times) will use the same iteration count, producing either meaningless or impossibly slow results.

**Why it happens:**
Benchmark authors choose iteration counts by feel ("1000 seems like a lot") rather than by desired measurement precision.

**How to avoid:**
Use adaptive iteration count: run the benchmark in a tight loop until 2 seconds of wall clock time have elapsed, counting iterations. Report median and inter-quartile range (IQR) over those iterations, not mean. Discard the highest and lowest 10% of samples as outliers. This approach (used by Criterion.rs and bench-node) automatically adjusts for different workload durations.

Minimum floor: at least 30 iterations for statistical validity (Central Limit Theorem applies above ~30 samples). Maximum ceiling: 10 seconds of total measurement time per benchmark per language on CI.

Report: median, 5th percentile (best case), 95th percentile (worst case). Do not report mean alone.

For the Writ runtime specifically: ensure GC does not run during the timed measurement window. Pre-run the benchmark once to populate all objects, then force a GC cycle, then start timing.

**Warning signs:**
- The 5-run standard deviation is more than 15% of the mean for any benchmark.
- Removing one outlier from a 5-run set changes the mean by more than 20%.
- The "compute-heavy" benchmark runs 3x slower on the second CI job compared to the first (GC timing non-determinism).

**Phase to address:** Benchmark harness phase — the statistical protocol (iterations, outlier treatment, reported statistic) must be specified before any measurements are taken.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Measure wall-clock time for all languages uniformly | Single runner script, no per-language special cases | Includes startup overhead in "execution time" for all languages; Writ compile time included; makes Writ look slower than it is | Never for publishable results — self-timing within each benchmark script is required |
| Use a single iteration count (e.g., 1000) for all benchmarks | Simple harness code | Short workloads are dominated by timing overhead; long workloads time out CI jobs | Only for an initial sanity-check run, never for committed results |
| Commit CI runner results directly to `benchmark/results/` | Fully automated pipeline | Results have 15-30% variance from run to run; history is noise | Only if the commit message documents "CI result — high variance" and results are not used for trend analysis |
| Include Rust native in every chart | Shows maximum possible performance ceiling | Rust is 50-500x faster, making all interpreted languages look identical in a linear-scale bar chart | Only on log-scale charts with explicit labeling |
| Hardcode Squirrel version as "latest" in Dockerfile | No version pinning work required | Build breaks when Squirrel master has a compilation error; results change silently when upstream changes behavior | Never — pin to a specific git commit hash |
| Skip warmup for Node.js | Simpler benchmark scripts | Measures Ignition interpreter performance, not V8's actual steady-state JIT performance; Node.js appears 5-10x slower than realistic | Never for any benchmark comparing interpreted languages |
| Use the same memory measurement command for all languages | Consistent-looking harness code | `/proc/self/status VmRSS` includes page cache on cgroup v2; memory numbers are overstated for all languages but by different amounts | Never for publishable results — read anonymous RSS specifically |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Docker + `writ` CLI | Invoking `writ run` as the benchmark measurement (includes compile time) | Run `writ compile` once outside the timed window, then benchmark `writ run` on the precompiled `.writc` file separately |
| GitHub Actions + Docker | Building the benchmark Docker image inside the CI job on every commit | Cache the Docker image layer containing compiled/installed language runtimes using `docker/build-push-action` with `cache-from: type=gha`; the Squirrel build layer in particular is expensive |
| SVG chart generation | Generating SVGs from raw result arrays in the chart script | Write raw results to a structured JSON file first, then generate charts from JSON; this allows re-generating charts without re-running benchmarks |
| Lua version detection | Calling `lua --version` and parsing the output string | PUC Lua prints version to stderr, not stdout; `luajit --version` prints to stdout. Parse both to handle both variants |
| Python `subprocess` timing | Using `time` module from inside Python to measure a `subprocess.run()` call | This measures Python overhead too; use `os.times()` or the host shell's `time` command, or have the child script self-report its timing via stdout |
| Node.js `--jitless` flag | Adding `--jitless` to "disable JIT for fairness" with other interpreters | This makes Node.js 5-10x slower than it would be in any real application; do not disable JIT; instead ensure proper warmup |
| Squirrel `sq` exit codes | Assuming `sq script.nut` returns non-zero on runtime error | Squirrel's `sq` binary may return 0 even on script errors; check stdout/stderr for error messages, do not rely solely on exit code for benchmark validity checks |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Docker OverlayFS write amplification | Benchmark that writes temporary files inside the container shows extreme outliers; median is not representative | Use a tmpfs mount (`--tmpfs /tmp`) for any benchmark I/O rather than the OverlayFS layer | Any benchmark that writes > 1MB of temporary data |
| Page cache cold-start for first-run measurements | First benchmark run is 2-3x slower than subsequent runs for all languages | Run each benchmark twice; discard the first run (cold cache); report from second run onward | Consistently on the first benchmark invocation in any fresh container |
| Python `import` at module level vs inside the benchmark loop | Importing `math` or `re` inside the timing loop adds 0.5-2ms per import | Move all imports to module level before the timing start | Any benchmark that imports standard library modules |
| Node.js `require()` inside the timed loop | Similar to Python import: require() is cached after first call but the first call is slow | Move all require/import statements outside the timed section | First iteration of any Node.js benchmark that requires modules inside the loop |
| Writ GC sweep during timing window | One in N Writ benchmark iterations takes 10x longer than average, inflating variance | Pre-allocate all objects before timing starts; force a GC cycle immediately before starting the timer | Any Writ benchmark that allocates objects in the loop |
| cgroup memory limit on Docker runner | Container is killed mid-benchmark with OOM; results are missing or zero | Set `--memory` Docker limit generously (at least 4GB) or use `--memory-swap -1` for unlimited swap | Any benchmark running inside a container with default memory limits on resource-constrained CI runners |

---

## "Looks Done But Isn't" Checklist

- [ ] **Workload parity**: verify the OOP benchmark allocates the same number of objects in each language — instrument each language to count allocations and assert they match within 5%.
- [ ] **Writ compile/run split**: verify that `writ compile` time is reported as a separate column and that the "execution time" chart does NOT include compilation time for Writ.
- [ ] **Node.js warmup**: verify that Node.js results for a long-running compute benchmark do not change by more than 10% when doubling the warmup iteration count.
- [ ] **Lua version pin**: verify the Docker container's `lua` binary reports the expected version string at container startup — add an assertion that fails the build if the version does not match.
- [ ] **Squirrel binary present**: verify `sq --version` exits with code 0 in the Docker container before any benchmark that includes Squirrel.
- [ ] **Memory metric**: verify that the reported memory numbers use anonymous RSS (not total RSS) by comparing the value against known allocation sizes in a minimal test script.
- [ ] **Chart Y-axis starts at zero**: verify in the generated SVG that the `viewBox` and axis elements encode `y=0` as the baseline; automated check: parse the SVG and assert the Y-axis minimum is 0.
- [ ] **Statistical sufficiency**: verify that each benchmark runs at least 30 iterations (not 5) by checking the benchmark harness with a fast benchmark (1ms per iteration) that should accumulate many samples.
- [ ] **CI variance check**: run the full benchmark suite twice in succession on GitHub Actions without changing any code; verify results differ by less than 15% (if they differ by more, document this in the results README and do not use CI numbers for trend tracking).
- [ ] **Squirrel exit code check**: verify that the runner script detects a Squirrel runtime error and marks the benchmark as failed (not reports 0ms success).

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Workload parity violations discovered after charts are published | HIGH | Rewrite the affected language implementations to match canonical algorithm; rerun all benchmarks; regenerate all charts; update README; add automated parity check to prevent recurrence |
| Node.js warmup missing from committed results | MEDIUM | Add warmup to Node.js benchmark scripts; rerun Node.js benchmarks; update charts and tables for Node.js series only |
| Squirrel not buildable in Docker CI | LOW-MEDIUM | Drop Squirrel from CI; document as "local-only"; or pin a working source commit and fix the Dockerfile build stage |
| Memory metrics reporting total RSS instead of anon RSS | MEDIUM | Update measurement shell function; rerun memory benchmarks for all languages; note discrepancy in changelog |
| CI results committed with 30% variance | MEDIUM | Delete committed CI results; add variance disclaimer to results README; change CI policy to not auto-commit results; generate authoritative results locally |
| Chart Y-axis truncated in published README | LOW | Regenerate chart with fixed axis; update README; 1-2 hours |
| Writ compile time included in execution time results | MEDIUM | Separate measurement script into compile-phase and run-phase; rerun all Writ benchmarks; update charts and tables |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Non-equivalent workloads across languages | Phase 1: Benchmark design | Allocations per language match; output values match; algorithm description exists as a spec document |
| Node.js JIT warmup missing | Phase 2: Benchmark harness | Node.js result changes <10% when warmup iterations are doubled |
| LuaJIT vs PUC Lua conflation | Phase 2: Docker environment | `lua --version` assertion in container startup; benchmark label in chart matches actual binary |
| Squirrel build complexity | Phase 2: Docker environment | `sq --version` succeeds in Docker CI before any benchmark runs |
| Memory metric includes page cache | Phase 2: Benchmark harness | Anonymous RSS measurement function verified against known allocation size |
| Python startup inflates execution time | Phase 2: Benchmark harness | Python benchmark self-reports execution time via stdout; startup time is a separate measured metric |
| CI runner timing variance | Phase 3: CI workflow | Two successive identical CI runs differ by <15%; regression threshold set to 15% minimum |
| Writ compile time mixed with execution time | Phase 2: Benchmark harness | `compile_ms` and `execution_ms` are separate columns in the raw results JSON |
| Chart Y-axis truncation | Phase 4: Chart generation | SVG axis minimum is 0; automated assertion in chart generation script |
| Insufficient iterations / no outlier handling | Phase 2: Benchmark harness | Minimum 30 iterations enforced by harness; results include median and IQR, not just mean |

---

## Sources

- Docker runtime metrics documentation — cgroup v1/v2 memory accounting, `total_inactive_file` vs `inactive_file` for cache subtraction: [Docker Docs: Runtime Metrics](https://docs.docker.com/engine/containers/runmetrics/) (HIGH confidence, official docs)
- cgroup v2 memory reporting discrepancy with JVM, RSS vs anon: [ibmruntimes/ci.docker issue #124](https://github.com/ibmruntimes/ci.docker/issues/124), [hashicorp/nomad issue #16230](https://github.com/hashicorp/nomad/issues/16230) (HIGH confidence, official issue trackers)
- V8 JIT tiers (Ignition → Sparkplug → Maglev → TurboFan): [V8 Maglev blog](https://v8.dev/blog/maglev), [bench-node](https://github.com/RafaelGSS/bench-node) (HIGH confidence, official V8 blog)
- Node.js v22.9.0 Maglev disable: [Node.js performance state 2024](https://nodesource.com/blog/State-of-Nodejs-Performance-2024) (MEDIUM confidence, authoritative source)
- LuaJIT does not implement Lua 5.3+ integer division `//` and bitwise operators: [Hacker News discussion](https://news.ycombinator.com/item?id=18889788), [LuaJIT issue #728](https://github.com/LuaJIT/LuaJIT/issues/728) (HIGH confidence, multiple sources)
- Squirrel official repository last commit February 2026: [albertodemichelis/squirrel GitHub](https://github.com/albertodemichelis/squirrel/commits/master) (HIGH confidence, direct inspection)
- Squirrel Debian package status: [squirrel3 Debian manpages](https://manpages.debian.org/testing/squirrel3/squirrel.1.en.html) (MEDIUM confidence — Debian testing, Ubuntu LTS availability not confirmed)
- GitHub Actions CPU variance (20%+ on shared runners), Azure CPUID misreporting: [RunsOn CPU benchmarks](https://runs-on.com/benchmarks/github-actions-cpu-performance/) (HIGH confidence, direct measurement data)
- CI benchmark false positive rates: CodSpeed research — 2% gate gives 0.04% false alarm rate: [CodSpeed CI benchmarks](https://codspeed.io/blog/benchmarks-in-ci-without-noise) (HIGH confidence, published measurement data)
- Process isolation reduces false positives except with Docker: [arxiv:2511.03533](https://arxiv.org/html/2511.03533) — IEEE/ACM 2025 (HIGH confidence, peer-reviewed 2025)
- Microbenchmark statistical methods — median over mean, bootstrapping, RMAD for Go, RCIW for Java: [µOpTime: arxiv:2501.12878](https://arxiv.org/html/2501.12878) (HIGH confidence, peer-reviewed 2025)
- Adaptive iteration benchmarking: [Statistical Methods for Reliable Benchmarks](https://modulovalue.com/blog/statistical-methods-for-reliable-benchmarks/) (MEDIUM confidence, technical blog)
- Cross-language benchmark fairness — "Are We Fast Yet?" paradigm-neutral algorithm design: [smarr/are-we-fast-yet GitHub](https://github.com/smarr/are-we-fast-yet), [ACM DLS paper](https://dl.acm.org/doi/10.1145/2989225.2989232) (HIGH confidence, peer-reviewed, established benchmark framework)
- Truncated Y-axis misleading effects: [ACM CHI 2024: Y-Axis Truncation](https://dl.acm.org/doi/fullHtml/10.1145/3613904.3642102) (HIGH confidence, peer-reviewed)
- Python startup overhead 50-150ms: [pyperformance documentation](https://pyperformance.readthedocs.io/usage.html) (HIGH confidence, official Python benchmarking suite)
- Docker startup OverlayFS right-skew on Azure: [arxiv:2602.15214](https://arxiv.org/html/2602.15214) (HIGH confidence, 2026 measurement study)

---
*Pitfalls research for: Writ v7.0 — adding cross-language benchmark suite (Writ vs Lua, Squirrel, Python, Node.js, Rust) with Docker containerization, CI integration, and SVG chart generation*
*Researched: 2026-03-20*
