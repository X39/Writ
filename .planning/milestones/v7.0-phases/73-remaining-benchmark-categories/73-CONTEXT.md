# Phase 73: Remaining Benchmark Categories - Context

**Gathered:** 2026-03-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Five additional benchmark suites (string concatenation, array sort, hash map, OOP/dispatch, object creation) across all 6 languages (Writ, Lua, Squirrel, Python, Node.js, Rust), with output-checksum parity verification. Extends the existing Docker benchmark pipeline from Phase 70-72. bench_runner.sh auto-discovers new suites from `/bench/cases/*/` — no harness changes needed. generate.py already handles N benchmark suites dynamically. CI workflow is Phase 74.

</domain>

<decisions>
## Implementation Decisions

### Algorithm Specifications

Each benchmark has a locked canonical algorithm with exact parameters and expected output:

- **string_concat** (BENCH-03): Loop 100,000 iterations appending `"hello"` to an accumulator string. Print final string length. Expected output: `500000`. Tests string allocation/copying overhead.
- **array_sort** (BENCH-04): Create an array of 100,000 integers in descending order (100000, 99999, ..., 1). Sort ascending using manual quicksort (same algorithm in all languages — NOT built-in sort). Print first and last elements: `1 100000`. Tests recursive call overhead, array indexing, comparison/swap.
- **hash_map** (BENCH-05): Insert 100,000 key-value pairs (string key `"key_N"`, integer value N) into a hash map. Look up all 100,000 keys and sum the values. Print the sum. Expected output: `4999950000`. Tests hash table allocation, string hashing, lookup.
- **oop_dispatch** (BENCH-06): Define a base type with a virtual `compute()` method. Define 4 subtypes overriding `compute()` with different arithmetic. Create N=100,000 objects cycling through the 4 subtypes, call `compute()` on each, sum results. Print the sum. Tests dynamic dispatch overhead, vtable/contract lookup.
- **object_create** (BENCH-07): Allocate 1,000,000 small objects (class/struct with 2-3 fields: int, int, string). Sum a field from each. Print the sum. Tests allocation throughput, GC pressure.

### Missing Writ Capabilities

- **Hash map**: Writ has no `Map<K,V>` type (deferred to writ-std, future milestone). **Writ is excluded from hash_map benchmark.** Results show "N/A" for Writ in hash_map charts/tables. bench_runner.sh skips missing language files gracefully (no `hash_map.writ` → no Writ entry in raw.json for that suite).
- **Built-in sort**: Writ has no Array.sort() method. Manual quicksort is implemented in all languages for fair comparison.

### OOP Dispatch Pattern

Each language uses its native polymorphism mechanism:
- **Writ**: `contract` with `impl` — `CALL_VIRT` dispatch (verify this works end-to-end in compiler+runtime before benchmark)
- **Lua**: Metatables with `__index` method lookup
- **Squirrel**: Class inheritance with method override
- **Python**: Class inheritance with method override
- **Node.js**: ES6 class inheritance with method override
- **Rust**: `dyn Trait` with `Box<dyn Trait>` (trait object dispatch)

All implementations must produce the same output sum, confirming algorithmic equivalence.

### Object Creation Pattern

All languages create a small class/struct with 2-3 fields:
- **Writ**: `class` with `new` constructor (`new Point { x: i, y: i, label: "..." }`)
- **Lua**: Table construction `{ x = i, y = i, label = "..." }`
- **Squirrel**: Class instance creation
- **Python**: `__init__` with 3 fields
- **Node.js**: ES6 class constructor
- **Rust**: `struct` literal (stack-allocated — note this is inherently different from GC-allocated languages; serves as ceiling reference)

### Output Verification

Same as Phase 71: each program prints a deterministic result to stdout, exact string match across all participating languages per suite. Writ uses `log::info()`, others use native print.

### Directory Structure

- `benchmark/cases/string_concat/` — `string_concat.{writ,lua,nut,py,js,rs}`
- `benchmark/cases/array_sort/` — `array_sort.{writ,lua,nut,py,js,rs}`
- `benchmark/cases/hash_map/` — `hash_map.{lua,nut,py,js,rs}` (NO .writ — see Missing Capabilities)
- `benchmark/cases/oop_dispatch/` — `oop_dispatch.{writ,lua,nut,py,js,rs}`
- `benchmark/cases/object_create/` — `object_create.{writ,lua,nut,py,js,rs}`

### Claude's Discretion

- Exact quicksort partition scheme (Lomuto vs Hoare — pick one, use consistently)
- Exact OOP class hierarchy names and compute() formulas (as long as all languages match)
- Whether object_create uses format string for label field or a constant string
- Error handling for missing language files in bench_runner.sh (skip gracefully)
- Whether to pre-verify Writ contract dispatch compiles correctly before writing benchmarks (recommended: yes)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase requirements
- `.planning/REQUIREMENTS.md` — BENCH-03 (string concat), BENCH-04 (array sort), BENCH-05 (hash map), BENCH-06 (OOP dispatch), BENCH-07 (object creation)
- `.planning/ROADMAP.md` §Phase 73 — Success criteria (4 items) defining what must be TRUE

### Measurement infrastructure (Phase 70, complete)
- `benchmark/runner/bench_runner.sh` — Measurement harness; auto-discovers suites from `/bench/cases/*/`, runs hyperfine, measures memory, produces raw.json. Skips missing language files gracefully.
- `benchmark/runner/Dockerfile` — Docker image with all 6 runtimes
- `benchmark/runner/run.sh` — Host launcher (Linux/macOS)
- `benchmark/runner/run.ps1` — Host launcher (Windows)

### Chart generation (Phase 72, complete)
- `benchmark/generate.py` — Reads raw.json, produces SVG charts + RESULTS.md. Handles N benchmark suites dynamically.

### Existing benchmark implementations (Phase 71, pattern reference)
- `benchmark/cases/fib/` — Reference for file naming, algorithm structure, per-language idioms
- `benchmark/cases/sieve/` — Reference for array-heavy Writ benchmark (int[] pattern, while-loop iteration)

### Writ language reference (for writing .writ benchmarks)
- `writ-golden/tests/golden/type_class_new.writ` — Class construction with `new` keyword
- `writ-golden/tests/golden/type_array_ops.writ` — Array creation, indexing, mutation, push, len
- `writ-golden/tests/golden/fn_recursion.writ` — Recursive function pattern (needed for quicksort)
- `writ-golden/tests/golden/ctrl_while_loop.writ` — While loop (Writ has no range-based for)
- `language-spec/spec/12_11_contracts.md` — Contract definition and implementation syntax

### Prior phase context
- `.planning/phases/70-docker-environment-and-measurement-harness/70-CONTEXT.md` — Docker architecture, JSON schema, measurement approach
- `.planning/phases/71-compute-benchmarks-mvp/71-CONTEXT.md` — Algorithm-spec-first approach, output verification, Writ patterns
- `.planning/phases/72-chart-generation-and-results-pipeline/72-CONTEXT.md` — Chart generation, pygal styling, RESULTS.md format

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `benchmark/runner/bench_runner.sh`: Auto-discovers suites via `/bench/cases/*/` glob — new directories are picked up automatically with no harness changes
- `benchmark/cases/fib/` and `benchmark/cases/sieve/`: Template implementations for all 6 languages — copy and adapt
- `benchmark/generate.py`: Handles N benchmark suites dynamically — new suites appear in charts automatically
- Writ golden tests confirm: class construction (`type_class_new`), array ops (`type_array_ops`), recursion (`fn_recursion`), while loops (`ctrl_while_loop`), string formatting (`$"..."`)

### Established Patterns
- Writ programs: `fn main()` entry point, `log::info()` for output, `let mut` for mutable bindings, `while` loops (no range for), `int[]` for arrays
- Writ classes: `pub class Name { field: type }` + `new Name { field: value }` construction
- Writ contracts: `contract Name { fn method(self) -> T; }` + `impl Name for Type { fn method(self) -> T { ... } }`
- No `Map<K,V>` in Writ (writ-std deferred) — hash_map benchmark excludes Writ
- No `Array.sort()` in Writ — manual sort needed (same in all languages for fairness)
- bench_runner.sh line 108: expects `{suite_dir}{suite}.writ` — if file missing, language is skipped

### Integration Points
- New directories under `benchmark/cases/` auto-discovered by bench_runner.sh
- Dockerfile COPY includes `benchmark/` — new files included in image automatically
- generate.py reads all benchmarks from raw.json dynamically — no code changes needed for new suites
- Writ contract dispatch (CALL_VIRT) needs verification before oop_dispatch benchmark — check with `writ compile` + `writ run`

</code_context>

<specifics>
## Specific Ideas

- STATE.md blocker: "OOP/dispatch canonical algorithm across Squirrel metatables, Lua metatables, Python classes, Writ structs, Rust traits is not defined" — this context defines the canonical algorithm and per-language dispatch mechanism
- Manual quicksort chosen over built-in sort to measure raw language speed, consistent with the compute-benchmark philosophy (fib = call overhead, sieve = array/loop overhead)
- Writ's contract dispatch (CALL_VIRT) should be smoke-tested before writing the oop_dispatch benchmark — if it doesn't work end-to-end, fall back to non-virtual method calls and document the limitation
- Hash map exclusion for Writ is honest — better to show N/A than fake a workaround with array linear scan (which would measure something completely different)
- Object creation benchmark measures GC pressure in interpreted languages vs stack allocation in Rust — the comparison is inherently apples-to-oranges for Rust but useful as a ceiling reference

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 73-remaining-benchmark-categories*
*Context gathered: 2026-03-20*
