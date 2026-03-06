# Phase 73: Remaining Benchmark Categories - Research

**Researched:** 2026-03-20
**Domain:** Cross-language benchmark implementations (Writ, Lua, Squirrel, Python, Node.js, Rust) + generate.py patch
**Confidence:** HIGH

## Summary

Phase 73 adds five benchmark suites (string_concat, array_sort, hash_map, oop_dispatch, object_create) to the existing Docker pipeline from Phases 70–72. All algorithms are fully specified in CONTEXT.md with exact parameters and expected outputs. The bench_runner.sh harness auto-discovers new suite directories with no changes required. The Dockerfile automatically picks up new `benchmark/cases/` files. generate.py handles N benchmark suites dynamically — but it currently crashes when `writ_compile` or `writ_run` is `null` (which WILL happen for hash_map since there is no `hash_map.writ`). This is a mandatory generate.py patch.

The most technically risky item is the oop_dispatch benchmark for Writ: there are NO end-to-end golden tests for the `contract`/`impl` feature. The emitter and runtime unit tests confirm CALL_VIRT is implemented, but an end-to-end smoke-test compiling and running a contract-dispatched Writ program must be performed before writing the benchmark. If it fails, the fallback is non-virtual method calls (with a documented limitation).

**Primary recommendation:** Write all five benchmark suites, patch generate.py for null Writ entries, and smoke-test Writ contract dispatch before committing oop_dispatch.writ.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Algorithm Specifications — exact params and expected output are locked:**

- **string_concat** (BENCH-03): Loop 100,000 iterations appending `"hello"` to an accumulator string. Print final string length. Expected output: `500000`.
- **array_sort** (BENCH-04): Create an array of 100,000 integers in descending order (100000, 99999, ..., 1). Sort ascending using manual quicksort (same algorithm in all languages — NOT built-in sort). Print first and last elements: `1 100000`.
- **hash_map** (BENCH-05): Insert 100,000 key-value pairs (string key `"key_N"`, integer value N) into a hash map. Look up all 100,000 keys and sum the values. Print the sum. Expected output: `4999950000`.
- **oop_dispatch** (BENCH-06): Define a base type with a virtual `compute()` method. Define 4 subtypes overriding `compute()` with different arithmetic. Create N=100,000 objects cycling through 4 subtypes, call `compute()` on each, sum results. Print the sum.
- **object_create** (BENCH-07): Allocate 1,000,000 small objects (class/struct with 2-3 fields: int, int, string). Sum a field from each. Print the sum.

**Missing Writ Capabilities:**

- Writ has no `Map<K,V>` — `hash_map.writ` is excluded. Results show "N/A" for Writ in hash_map charts/tables. bench_runner.sh skips missing language files gracefully.
- Writ has no `Array.sort()` — manual quicksort implemented in all languages.

**OOP Dispatch — per-language dispatch mechanism locked:**
- Writ: `contract` with `impl` — CALL_VIRT dispatch
- Lua: Metatables with `__index` method lookup
- Squirrel: Class inheritance with method override
- Python: Class inheritance with method override
- Node.js: ES6 class inheritance with method override
- Rust: `dyn Trait` with `Box<dyn Trait>` (trait object dispatch)

**Object Creation — per-language pattern locked:**
- Writ: `class` with `new` constructor (`new Point { x: i, y: i, label: "..." }`)
- Lua: Table construction `{ x = i, y = i, label = "..." }`
- Squirrel: Class instance creation
- Python: `__init__` with 3 fields
- Node.js: ES6 class constructor
- Rust: `struct` literal (stack-allocated — ceiling reference)

**Directory structure locked:**
- `benchmark/cases/string_concat/` — `string_concat.{writ,lua,nut,py,js,rs}`
- `benchmark/cases/array_sort/` — `array_sort.{writ,lua,nut,py,js,rs}`
- `benchmark/cases/hash_map/` — `hash_map.{lua,nut,py,js,rs}` (NO .writ)
- `benchmark/cases/oop_dispatch/` — `oop_dispatch.{writ,lua,nut,py,js,rs}`
- `benchmark/cases/object_create/` — `object_create.{writ,lua,nut,py,js,rs}`

**Output verification:** Same as Phase 71 — exact string match across all participating languages. Writ uses `log::info()`, others use native print.

### Claude's Discretion

- Exact quicksort partition scheme (Lomuto vs Hoare — pick one, use consistently)
- Exact OOP class hierarchy names and `compute()` formulas (as long as all languages match)
- Whether object_create uses format string for label field or a constant string
- Error handling for missing language files in bench_runner.sh (skip gracefully — already implemented)
- Whether to pre-verify Writ contract dispatch compiles correctly before writing benchmarks (recommended: yes)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BENCH-03 | String concatenation benchmark runs in all 6 languages with equivalent algorithm | string_concat.{writ,lua,nut,py,js,rs} in `benchmark/cases/string_concat/`; Writ uses `+` string operator (confirmed in expr_string_concat golden test) |
| BENCH-04 | Array sort benchmark runs in all 6 languages with equivalent algorithm | array_sort.{writ,lua,nut,py,js,rs} in `benchmark/cases/array_sort/`; manual quicksort; Writ uses `int[]` with while-loop index access |
| BENCH-05 | Hash map insert/lookup benchmark runs in all 6 languages with equivalent algorithm | hash_map.{lua,nut,py,js,rs} (NO .writ); generate.py must be patched to handle null writ_compile/writ_run |
| BENCH-06 | OOP virtual dispatch benchmark runs in all 6 languages with equivalent algorithm | oop_dispatch.{writ,lua,nut,py,js,rs}; Writ contract/impl pattern must be smoke-tested first; CALL_VIRT emitter confirmed present in unit tests |
| BENCH-07 | Object creation benchmark runs in all 6 languages with equivalent algorithm | object_create.{writ,lua,nut,py,js,rs}; Writ `pub class` with `new` constructor confirmed in type_class_new golden test |
</phase_requirements>

---

## Standard Stack

### Core — All already present in Docker image (no new dependencies)

| Runtime | Version | Purpose | Status |
|---------|---------|---------|--------|
| Writ CLI | project binary | Compile + run .writ files | In Dockerfile Stage 1 |
| Lua 5.4 | `lua5.4` | Run .lua files | In Dockerfile Stage 3 |
| Squirrel 3.2 | `sq` | Run .nut files | In Dockerfile Stage 3 |
| Python 3 | `python3` | Run .py files | In Dockerfile Stage 3 |
| Node.js 22 LTS | `node` | Run .js files | In Dockerfile Stage 3 |
| Rust (rustc -O) | 1.88-slim | Pre-compile .rs binaries | In Dockerfile Stage 2 |
| hyperfine | 1.20.0 | Time measurement | In Dockerfile Stage 3 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| pygal | 3.1.0 | SVG chart generation | generate.py — already installed |
| jq | system | JSON assembly in bench_runner.sh | Already in Dockerfile |

**Installation:** None required — all runtimes and tools are already in the Docker image.

---

## Architecture Patterns

### Established File Structure (from Phases 70–72)

```
benchmark/
├── cases/
│   ├── stub/                    # startup measurement (already exists)
│   ├── fib/                     # Phase 71 reference (already exists)
│   ├── sieve/                   # Phase 71 reference (already exists)
│   ├── string_concat/           # NEW: Phase 73
│   │   ├── string_concat.writ
│   │   ├── string_concat.lua
│   │   ├── string_concat.nut
│   │   ├── string_concat.py
│   │   ├── string_concat.js
│   │   └── string_concat.rs
│   ├── array_sort/              # NEW: Phase 73
│   │   └── array_sort.{writ,lua,nut,py,js,rs}
│   ├── hash_map/                # NEW: Phase 73 (NO .writ file)
│   │   └── hash_map.{lua,nut,py,js,rs}
│   ├── oop_dispatch/            # NEW: Phase 73
│   │   └── oop_dispatch.{writ,lua,nut,py,js,rs}
│   └── object_create/           # NEW: Phase 73
│       └── object_create.{writ,lua,nut,py,js,rs}
├── runner/
│   ├── bench_runner.sh          # No changes needed
│   ├── Dockerfile               # No changes needed
│   ├── run.sh                   # No changes needed
│   └── run.ps1                  # No changes needed
└── generate.py                  # PATCH REQUIRED: null writ guard
```

### Pattern 1: Writ Benchmark File

All Writ benchmarks follow this exact pattern (verified from fib.writ, sieve.writ):

```writ
// Source: benchmark/cases/fib/fib.writ, benchmark/cases/sieve/sieve.writ
fn main() {
    // setup
    let mut accumulator: int = 0;
    let mut i: int = 0;
    while i < N {
        // work
        i = i + 1;
    }
    log::info($"{result}");   // ONLY output mechanism in Writ
}
```

Key Writ constraints (HIGH confidence — from golden tests):
- Entry point: `fn main()`
- Output: `log::info($"{value}")` — the ONLY way to print in Writ benchmarks
- Mutable bindings: `let mut`
- Arrays: `int[]` initialized as `[]`, elements added with `.push()`; accessed with `arr[i]`
- Loops: `while` only — Writ has no C-style `for` or range for-loop over integers
- Array length: `arr.len()`
- String concatenation: `a + b` (verified in `expr_string_concat.writ`)
- Class construction: `new ClassName { field: value }` (verified in `type_class_new.writ`)
- Contracts: `contract Name { fn method(self) -> T; }` + `impl Name for Type { fn method(self) -> T { ... } }`

### Pattern 2: bench_runner.sh Auto-Discovery

bench_runner.sh line 108 attempts:
```bash
writ compile "${suite_dir}${suite}.writ" -o "/tmp/${suite}.writc"
```

If the file is absent, `writ compile` fails, hyperfine returns non-zero, and `writ_compile_json="null"`. Same for `writ_run_json`. The assembled JSON for hash_map will have `"writ_compile": null, "writ_run": null`. **No changes needed to bench_runner.sh.**

### Pattern 3: Rust Benchmark Auto-Compilation

Dockerfile Stage 2 runs:
```bash
for rs in cases/*/*.rs; do
    name=$(basename "$(dirname "$rs")");
    rustc -O -o "/bench/bin/${name}" "$rs";
done
```

New `.rs` files are auto-compiled. No Dockerfile changes needed.

### Pattern 4: generate.py Null Writ Guard (MANDATORY PATCH)

Current code crashes on `null` writ entries:
```python
# CURRENT (crashes when writ_compile is null):
def writ_compile_ms(b):
    return b['writ_compile']['median'] * 1000  # TypeError if null
```

Required patch:
```python
# Source: analysis of generate.py + raw.json structure
def writ_compile_ms(b):
    """Compiler time in ms; returns None if not available."""
    entry = b.get('writ_compile')
    if entry is None:
        return None
    return entry['median'] * 1000

def writ_run_ms(b):
    """Runtime time in ms; returns None if not available."""
    entry = b.get('writ_run')
    if entry is None:
        return None
    return entry['median'] * 1000
```

All call sites that consume these functions must also guard for `None`:
- `generate_exec_charts()`: skip Writ series when `writ_compile_ms(b)` returns `None`
- `generate_memory_chart()`: `lang_memory_mb(b, 'writ_run')` already handles missing keys via `.get()` — returns 0.0
- `generate_results_md()`: skip Writ row when `writ_compile_ms(b)` or `writ_run_ms(b)` returns `None`

### Anti-Patterns to Avoid

- **Using `for i in range(N)` in Writ:** Writ has no range-based for. Use `while i < N { ... i = i + 1; }`.
- **Using built-in sort in any language:** All languages must use the same manual quicksort algorithm for BENCH-04.
- **Using print() or console.log() in Writ:** Writ benchmarks use `log::info()` exclusively.
- **Assuming Writ has Map<K,V>:** It does not — hash_map.writ must not be created.
- **Not guarding generate.py for null writ:** Without the null guard, `python3 generate.py raw.json` crashes on hash_map suite.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| New Docker infrastructure | Custom harness scripts | Existing bench_runner.sh | Auto-discovers new suite dirs; no harness changes needed |
| Custom chart generation code | New chart renderer | Existing generate.py (patched) | Already handles N benchmarks dynamically |
| Custom quicksort per-language from scratch | Novel implementations | Lomuto or Hoare — pick one, translate to all 6 languages | Consistency is the goal; the partition scheme doesn't matter as long as all 6 match |
| Map<K,V> workaround for Writ | Array linear scan | Simply omit hash_map.writ | Linear scan measures something different; N/A is honest |

**Key insight:** This phase is almost entirely file creation. The infrastructure is complete. The only code change is a small guard patch in generate.py.

---

## Common Pitfalls

### Pitfall 1: generate.py Crash on hash_map Suite
**What goes wrong:** `python3 generate.py raw.json` raises `TypeError: 'NoneType' object is not subscriptable` at `b['writ_compile']['median']` when processing the hash_map suite (where `writ_compile` is `null`).
**Why it happens:** generate.py functions `writ_compile_ms()` and `writ_run_ms()` assume the fields always exist. bench_runner.sh correctly emits `null` for missing language files.
**How to avoid:** Patch both functions to return `None` when the entry is absent. Guard all call sites.
**Warning signs:** Any suite missing a language file will trigger this if generate.py is not patched first.

### Pitfall 2: Writ Contract Dispatch Not Tested End-to-End
**What goes wrong:** The oop_dispatch.writ file compiles but produces an incorrect result (or crashes at runtime) because `contract`/`impl`/CALL_VIRT has never been exercised by a complete writ compile + writ run pipeline.
**Why it happens:** There are zero golden tests with `contract` or `impl ... for` syntax. Unit tests confirm emitter and runtime dispatch table logic separately, but no integration path has been validated.
**How to avoid:** Before writing oop_dispatch.writ, write a minimal smoke-test (`contract Computable { fn compute(self) -> int; } pub class Foo {} impl Computable for Foo { fn compute(self) -> int { 42 } } fn main() { ... }`), run `writ compile && writ run`, verify output. If it fails, fall back to plain class methods.
**Warning signs:** Compiler emits no diagnostic but runtime crashes with dispatch error.

### Pitfall 3: Quicksort Implementation Diverges Across Languages
**What goes wrong:** One language's manual quicksort produces correct sorted output but a different first/last element pair, failing the output parity check.
**Why it happens:** Edge cases in partition (especially when all elements are equal, or array of size 1) can make different implementations diverge on boundary handling. Also: off-by-one in the 100000-to-1 descending array construction.
**How to avoid:** Use Lomuto partition scheme consistently. Verify each language outputs `1 100000` with N=100000 before finalizing. Test locally before Docker run.
**Warning signs:** Output is sorted but printed differently (e.g., Python prints `"1 100000"` but Lua prints `"1\t100000"`).

### Pitfall 4: hash_map Expected Sum Overflow in Some Languages
**What goes wrong:** The hash_map sum `4999950000` exceeds 32-bit integer max (2,147,483,647). Languages using 32-bit int by default will overflow silently.
**Why it happens:** Sum = N*(N-1)/2 for N=100000 = 4,999,950,000 > 2^31-1.
**How to avoid:**
- Rust: use `i64` or `u64` for the accumulator
- Node.js: JavaScript numbers are 64-bit floats — safe up to 2^53, no issue
- Python: arbitrary precision integers, no issue
- Lua: numbers are 64-bit floats by default in Lua 5.4, no issue (but verify: 4999950000 < 2^53)
- Squirrel: uses 64-bit integers internally, no issue
**Warning signs:** sum prints as a negative number or truncated value.

### Pitfall 5: Writ String Concatenation in Loop Performance Trap
**What goes wrong:** The string_concat benchmark produces correct output but takes extremely long to run if Writ's string concatenation creates intermediate copies on each iteration.
**Why it happens:** This is expected — string_concat benchmarks string allocation overhead. It's the point of the test. The risk is the benchmark taking so long it causes hyperfine to timeout.
**How to avoid:** Test locally with `writ compile && writ run` before Docker run. If 100,000 iterations is too slow for timed measurement, reduce to 10,000 and adjust expected output to `50000`. But the CONTEXT.md locks it at 100,000 — so proceed as specified.

### Pitfall 6: Writ for-loop vs while-loop
**What goes wrong:** `for i in 0..N` is NOT valid Writ syntax. Using it produces a parse error.
**Why it happens:** Writ only has `for item in collection` (array iteration), not range-for.
**How to avoid:** All Writ loops must use: `let mut i: int = 0; while i < N { ... i = i + 1; }`.

### Pitfall 7: Object Count in oop_dispatch
**What goes wrong:** 100,000 objects cycling through 4 types means the output sum depends on which exact arithmetic each subtype uses. If the formula is wrong (or not identical across languages), the output check fails.
**How to avoid:** Lock the compute() formulas for the 4 subtypes explicitly in the plan and use them identically in all 6 languages. Recommended: type 0 returns `1`, type 1 returns `2`, type 2 returns `3`, type 3 returns `4`. Cycling 100,000 objects: 25,000 of each. Sum = 25000*(1+2+3+4) = 250,000.

---

## Code Examples

Verified patterns from existing source files:

### Writ: String Concatenation Loop Pattern
```writ
// Source: writ-golden/tests/golden/expr_string_concat.writ (+ sieve.writ loop pattern)
fn main() {
    let mut s: string = "";
    let mut i: int = 0;
    while i < 100000 {
        s = s + "hello";
        i = i + 1;
    }
    let len: int = s.len();   // NOTE: verify .len() works on string (see Open Questions)
    log::info($"{len}");
}
```

### Writ: Manual Quicksort Pattern (Lomuto)
```writ
// Source: pattern derived from fn_recursion.writ (recursive fn) + sieve.writ (array indexing)
fn partition(arr: int[], lo: int, hi: int) -> int {
    let pivot: int = arr[hi];
    let mut i: int = lo - 1;
    let mut j: int = lo;
    while j < hi {
        if arr[j] <= pivot {
            i = i + 1;
            let tmp: int = arr[i];
            arr[i] = arr[j];
            arr[j] = tmp;
        }
        j = j + 1;
    }
    let tmp: int = arr[i + 1];
    arr[i + 1] = arr[hi];
    arr[hi] = tmp;
    i + 1
}

fn quicksort(arr: int[], lo: int, hi: int) {
    if lo < hi {
        let p: int = partition(arr, lo, hi);
        quicksort(arr, lo, p - 1);
        quicksort(arr, p + 1, hi);
    }
}
```

### Writ: Contract + Impl Pattern
```writ
// Source: language-spec/spec/12_11_contracts.md + emit_tests.rs
contract Computable {
    fn compute(self) -> int;
}

pub class TypeA {}
pub class TypeB {}

impl Computable for TypeA {
    fn compute(self) -> int { 1 }
}

impl Computable for TypeB {
    fn compute(self) -> int { 2 }
}
```

### Writ: Class Construction
```writ
// Source: writ-golden/tests/golden/type_class_new.writ
pub class Point {
    x: int,
    y: int,
    label: string
}

fn main() {
    let p: Point = new Point { x: 1, y: 2, label: "item" };
    let val: int = p.x;
}
```

### Lua: OOP with Metatables
```lua
-- Standard Lua OOP pattern for virtual dispatch
local Base = {}
Base.__index = Base

function Base:new(type_id)
    return setmetatable({type_id = type_id}, self)
end

local TypeA = setmetatable({}, {__index = Base})
TypeA.__index = TypeA

function TypeA:compute() return 1 end
```

### Squirrel: OOP with Class Inheritance
```squirrel
// Squirrel native class/method override (confirmed from sieve.nut pattern)
class Base {
    constructor() {}
    function compute() { return 0; }
}

class TypeA extends Base {
    constructor() { Base.constructor(); }
    function compute() { return 1; }
}
```

### Rust: Trait Object Dispatch
```rust
// Source: fib.rs pattern + standard Rust trait pattern
trait Computable {
    fn compute(&self) -> i64;
}

struct TypeA;
impl Computable for TypeA {
    fn compute(&self) -> i64 { 1 }
}

fn main() {
    let objects: Vec<Box<dyn Computable>> = /* ... */;
    let sum: i64 = objects.iter().map(|o| o.compute()).sum();
    println!("{}", sum);
}
```

### generate.py: Null Writ Guard Patch
```python
# Patch for generate.py — guards against null writ_compile/writ_run in raw.json
# Required for hash_map suite where no hash_map.writ exists

def writ_compile_ms(b):
    """Compiler time in ms; returns None if writ not available for this suite."""
    entry = b.get('writ_compile')
    if entry is None:
        return None
    return entry['median'] * 1000

def writ_run_ms(b):
    """Runtime time in ms; returns None if writ not available for this suite."""
    entry = b.get('writ_run')
    if entry is None:
        return None
    return entry['median'] * 1000

# In build_chart() (inside generate_exec_charts()):
if key == 'writ':
    wc = writ_compile_ms(b)
    wr = writ_run_ms(b)
    if wc is None or wr is None:
        continue  # skip Writ bar entirely for this suite
    value = { 'value': round(wc + wr, 3), 'label': f'compile: {wc:.2f}ms, run: {wr:.2f}ms' }

# In generate_results_md():
wc = writ_compile_ms(b)
wr = writ_run_ms(b)
if wc is not None and wr is not None:
    wt = wc + wr
    wm = lang_memory_mb(b, 'writ_run')
    lines.append(f'| Writ | {suite} | {wt:.1f} | {wc:.1f} | {wm:.1f} | {ratio_str(wt, rust_ms_val)} |')
# (else: omit Writ row — shows as N/A by absence)
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manually update harness for each suite | Auto-discovery via `/bench/cases/*/` glob | Phase 70 | No bench_runner.sh changes needed for new suites |
| Host-side raw.json | Container produces raw.json, host runs generate.py | Phase 72 | generate.py is host-side Python, easy to patch |
| Built-in sort for comparison | Manual quicksort in all languages | Phase 73 CONTEXT | Measures raw language speed consistently |

**N/A handling for missing languages:** The existing `lang_ms(b, key)` function in generate.py already handles missing non-Writ languages gracefully (returns `None` via `.get()`). Only the Writ-specific functions need patching.

---

## Open Questions

1. **Does `string.len()` exist in Writ?**
   - What we know: `arr.len()` works for `int[]` (confirmed in type_array_ops.writ). The `+` operator works for strings (confirmed in expr_string_concat.writ). The spec's section on contracts shows `Into<string>` but no direct `.len()` on strings.
   - What's unclear: Whether Writ exposes a `.len()` method on `string` type in the current compiler.
   - Recommendation: **Verify first.** If `.len()` is not available on strings, compute length by counting iterations (the benchmark loop count IS the length after appending "hello" 100,000 times — result is always 500,000). Alternative: `let len: int = 500000;` is cheating. Better: confirm string.len() exists or open a quick-task to add it. If unavailable, restructure to print the count variable directly.

2. **Does Writ contract/impl dispatch work end-to-end?**
   - What we know: CALL_VIRT emitter is implemented (call.rs). Runtime dispatch table is populated (vm_tests.rs confirms 36 intrinsic + 1 user entry). No golden tests exercise contract/impl.
   - What's unclear: Whether the full pipeline (parser → resolver → type-checker → emitter → assembler → runtime) correctly handles `contract C { fn m(self) -> int; } pub class T {} impl C for T { fn m(self) -> int { 42 } } fn main() { ... }`.
   - Recommendation: **Smoke-test before writing oop_dispatch.writ.** If it fails, use non-virtual method calls on concrete types and document the limitation. The OOP dispatch test still measures *some* dispatch overhead.

3. **Exact compute() formula sum for oop_dispatch**
   - What we know: 100,000 objects cycling through 4 subtypes.
   - What's unclear: The exact formulas are at the plan's discretion — the planner must lock them.
   - Recommendation: Use `TypeA.compute()=1, TypeB.compute()=2, TypeC.compute()=3, TypeD.compute()=4`. 100,000 / 4 = 25,000 of each. Sum = 25,000 * (1+2+3+4) = 250,000. All 6 languages must output `250000`.

4. **object_create label field: format string or constant?**
   - What we know: CONTEXT.md leaves this to discretion.
   - What's unclear: Format string `$"item_{i}"` is prettier but may slow Writ down. A constant `"item"` is simpler.
   - Recommendation: Use constant `"item"` for all languages to avoid format-string overhead confounding the allocation measurement.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (writ-compiler, writ-runtime), golden test runner (writ-golden) |
| Config file | `Cargo.toml` workspace |
| Quick run command | `cargo test -p writ-golden 2>&1 \| tail -5` |
| Full suite command | `cargo test --workspace 2>&1 \| tail -20` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BENCH-03 | string_concat.writ compiles and outputs `500000` | smoke (writ compile+run) | `writ compile benchmark/cases/string_concat/string_concat.writ -o /tmp/sc.writc && writ run /tmp/sc.writc` | ❌ Wave 0 |
| BENCH-03 | string_concat.lua/nut/py/js/rs each output `500000` | smoke (direct run) | `lua5.4 benchmark/cases/string_concat/string_concat.lua` | ❌ Wave 0 |
| BENCH-04 | array_sort.writ outputs `1 100000` | smoke | `writ compile ... && writ run ...` | ❌ Wave 0 |
| BENCH-04 | array_sort.{lua,nut,py,js,rs} each output `1 100000` | smoke | direct invocation | ❌ Wave 0 |
| BENCH-05 | hash_map.{lua,nut,py,js,rs} each output `4999950000` | smoke | direct invocation | ❌ Wave 0 |
| BENCH-06 | oop_dispatch.writ compiles and outputs correct sum | smoke | `writ compile ... && writ run ...` | ❌ Wave 0 |
| BENCH-06 | oop_dispatch.{lua,nut,py,js,rs} output matching sum | smoke | direct invocation | ❌ Wave 0 |
| BENCH-07 | object_create.writ compiles and outputs correct sum | smoke | `writ compile ... && writ run ...` | ❌ Wave 0 |
| BENCH-07 | object_create.{lua,nut,py,js,rs} output matching sum | smoke | direct invocation | ❌ Wave 0 |
| BENCH-03-07 | generate.py handles null writ entries without crash | unit | `python3 benchmark/generate.py benchmark/results/YYYY-MM-DD/raw.json` | ❌ Wave 0 |

**Note:** These are all output-checksum smoke tests run locally (or in Docker), not unit tests. The "automated command" is the manual verification step, not a CI command.

### Sampling Rate
- **Per benchmark file:** Run locally and verify output matches expected value before committing
- **Per suite wave:** Run `python3 benchmark/generate.py` against a test raw.json containing a null writ entry to verify the patch
- **Phase gate:** Full Docker run producing updated raw.json with all 7 benchmarks before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `benchmark/cases/string_concat/string_concat.{writ,lua,nut,py,js,rs}` — BENCH-03
- [ ] `benchmark/cases/array_sort/array_sort.{writ,lua,nut,py,js,rs}` — BENCH-04
- [ ] `benchmark/cases/hash_map/hash_map.{lua,nut,py,js,rs}` — BENCH-05 (no .writ)
- [ ] `benchmark/cases/oop_dispatch/oop_dispatch.{writ,lua,nut,py,js,rs}` — BENCH-06
- [ ] `benchmark/cases/object_create/object_create.{writ,lua,nut,py,js,rs}` — BENCH-07
- [ ] `benchmark/generate.py` — null writ guard patch
- [ ] Writ contract dispatch smoke-test (prerequisite for oop_dispatch.writ)

---

## Sources

### Primary (HIGH confidence)
- `benchmark/runner/bench_runner.sh` — verified auto-discovery, null handling for missing language files
- `benchmark/generate.py` — verified writ_compile_ms/writ_run_ms call pattern, lang_ms null handling
- `benchmark/results/2026-03-20/raw.json` — verified JSON schema with null/non-null entries
- `benchmark/runner/Dockerfile` — verified Stage 2 auto-compilation loop for .rs files
- `writ-golden/tests/golden/fib.writ`, `sieve.writ` — verified Writ benchmark entry point, log::info, while-loop
- `writ-golden/tests/golden/expr_string_concat.writ` — verified `+` operator on strings
- `writ-golden/tests/golden/type_class_new.writ` — verified `new Class { field: value }` syntax
- `writ-golden/tests/golden/type_array_ops.writ` — verified `arr[i]`, `arr.push()`, `arr.len()`
- `writ-golden/tests/golden/fn_recursion.writ` — verified recursive function pattern
- `language-spec/spec/12_11_contracts.md` — verified contract/impl syntax
- `writ-compiler/src/emit/body/call.rs` — verified CALL_VIRT emitter exists for CallKind::Virtual
- `writ-compiler/tests/emit_body_tests.rs` — verified CALL_VIRT emitter unit tests pass
- `writ-runtime/tests/vm_tests.rs` — verified dispatch table populated with user contracts

### Secondary (MEDIUM confidence)
- `writ-compiler/tests/emit_tests.rs` — confirms `contract ... {}` and `impl ... {}` parse and emit ContractDef/ImplDef
- Inference from bench_runner.sh null handling — when `.writ` file is absent, `writ_compile` and `writ_run` become JSON `null`

### Tertiary (LOW confidence)
- `string.len()` availability in Writ — not directly observed in golden tests; `arr.len()` confirmed but string length method is unverified

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all infrastructure verified from existing files
- Architecture: HIGH — patterns directly from Phase 71 reference implementations
- generate.py null bug: HIGH — code path verified by reading source
- Writ contract dispatch e2e: LOW — unit tests exist but no golden test validates full pipeline
- Pitfalls: HIGH for most; MEDIUM for Writ string.len() availability

**Research date:** 2026-03-20
**Valid until:** 2026-04-20 (stable language + toolchain; no fast-moving dependencies)
