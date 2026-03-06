# Phase 71: Compute Benchmarks MVP - Context

**Gathered:** 2026-03-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Fibonacci recursive and prime sieve (Eratosthenes) benchmark programs across all 6 languages (Writ, Lua, Squirrel, Python, Node.js, Rust), with verified output confirming algorithmic equivalence and Writ compile/run separation. The Docker environment and measurement harness (Phase 70) are complete — this phase only adds benchmark source files and validates the pipeline produces correct `raw.json` entries.

</domain>

<decisions>
## Implementation Decisions

### Algorithm Specification
- **Fibonacci**: Naive recursive fib(40). Expected output: `102334155` (printed to stdout). No memoization — this is a pure compute stress test measuring function call overhead and integer arithmetic
- **Prime Sieve**: Standard Eratosthenes sieve to N=1,000,000. Expected output: `78498` (the count of primes up to 1M, printed to stdout). Classic boolean-array sieve — mark composites, count remaining
- Algorithm spec is locked BEFORE writing any implementations (per STATE.md decision: "Algorithm spec precedes code")

### Output Verification
- Each program prints a single integer to stdout (the result)
- Verification = exact stdout string match across all 6 languages for each benchmark
- Writ uses `log::info()` to produce output (matching the stub pattern)
- Other languages use their standard print functions (print, console.log, println!, io.write, etc.)

### Writ Implementation
- **Fibonacci**: Straightforward — Writ has recursion (`fn_recursion` golden test) and `int` arithmetic. Direct translation: `pub fn fib(n: int) -> int` with recursive calls
- **Prime Sieve**: Use `int[]` as boolean proxy (0=composite, 1=prime) since Writ has no `bool[]` type. `type_array_ops` golden test confirms array indexing and mutation work. Use `while` loop for iteration (no range-based `for i in 0..n` — use `while i < n` pattern per `ctrl_while_loop` golden test)
- Output via `log::info()` with the result value (consistent with stub benchmark pattern)
- Each `.writ` file needs a `fn main()` entry point

### Directory Structure
- `benchmark/cases/fib/` — contains `fib.writ`, `fib.lua`, `fib.nut`, `fib.py`, `fib.js`, `fib.rs`
- `benchmark/cases/sieve/` — contains `sieve.writ`, `sieve.lua`, `sieve.nut`, `sieve.py`, `sieve.js`, `sieve.rs`
- Matches existing `benchmark/cases/stub/` convention — bench_runner.sh auto-discovers via `for suite_dir in /bench/cases/*/`

### Claude's Discretion
- Exact Writ syntax for array initialization and size management (e.g., how to create a large int[] for sieve)
- Whether to use helper functions or inline logic within main()
- Error handling in benchmark scripts (programs should not fail, but graceful fallback is fine)
- Whether fib(40) produces sufficient runtime variance for meaningful MAD statistics (can adjust N if too fast)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase requirements
- `.planning/REQUIREMENTS.md` — BENCH-01 (Fibonacci), BENCH-02 (Prime sieve), BENCH-08 (Verifiable output)
- `.planning/ROADMAP.md` §Phase 71 — Success criteria (4 items) defining what must be TRUE

### Measurement infrastructure (Phase 70, complete)
- `benchmark/runner/bench_runner.sh` — Measurement harness that auto-discovers suites from `/bench/cases/*/`, runs hyperfine, measures memory, produces raw.json
- `benchmark/runner/Dockerfile` — Docker image build with all 6 runtimes
- `benchmark/runner/run.sh` — Host launcher script (Linux/macOS)
- `benchmark/runner/run.ps1` — Host launcher script (Windows)

### Existing benchmark pattern
- `benchmark/cases/stub/` — Reference implementation showing file naming convention and per-language program structure (one file per language per suite)

### Writ language reference (for writing .writ benchmarks)
- `writ-golden/tests/golden/fn_recursion.writ` — Recursion pattern (factorial)
- `writ-golden/tests/golden/ctrl_while_loop.writ` — While loop pattern
- `writ-golden/tests/golden/type_array_ops.writ` — Array creation, indexing, mutation
- `writ-golden/tests/golden/ctrl_for_array.writ` — For-each over arrays
- `writ-golden/tests/golden/quest_system.writ` — Complex example exercising enums, functions, match, arrays, globals, atomic, dialogue builtins

### Prior phase context
- `.planning/phases/70-docker-environment-and-measurement-harness/70-CONTEXT.md` — Docker architecture, JSON schema, measurement approach decisions

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `benchmark/runner/bench_runner.sh`: Auto-discovers suites via `/bench/cases/*/` glob — new fib/ and sieve/ directories will be picked up automatically with no harness changes
- `benchmark/cases/stub/`: Template for file naming — one file per language: `{suite}.writ`, `{suite}.lua`, `{suite}.nut`, `{suite}.py`, `{suite}.js`, `{suite}.rs`
- Writ golden tests: Confirmed working patterns for recursion, while loops, array ops, for-each, globals, log::info output

### Established Patterns
- Writ programs use `fn main()` entry point, `log::info()` for output, `let mut` for mutable bindings
- No `bool[]` type — use `int[]` with 0/1 values for sieve
- No range-based for loops — use `while i < n` with manual increment (`i = i + 1`)
- No `println` or `print` — all output via `log::info()`
- `writ compile foo.writ -o foo.writc` then `writ run foo.writc` (two-step execution)

### Integration Points
- bench_runner.sh line 108: `writ compile "${suite_dir}${suite}.writ" -o "/tmp/${suite}.writc"` — expects `.writ` file named after suite
- bench_runner.sh line 132: `writ run /tmp/${suite}.writc` — runs compiled module
- Dockerfile COPY includes `benchmark/` directory — new files in cases/ will be included in image automatically

</code_context>

<specifics>
## Specific Ideas

- Writ's `log::info()` output needs to match plain stdout from other languages — verify bench_runner.sh captures Writ output equivalently
- fib(40) naive recursive is ~1-2 seconds in interpreted languages — good sweet spot for benchmarking
- Sieve to 1M needs a 1,000,001-element int[] in Writ — verify runtime handles large array allocation
- Squirrel sieve needs care: `sq` uses 1-based arrays and has different boolean semantics
- Node.js fibonacci may need explicit `process.stdout.write()` instead of `console.log()` to avoid trailing newline differences

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 71-compute-benchmarks-mvp*
*Context gathered: 2026-03-20*
