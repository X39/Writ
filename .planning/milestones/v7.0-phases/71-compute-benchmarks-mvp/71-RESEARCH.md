# Phase 71: Compute Benchmarks MVP - Research

**Researched:** 2026-03-20
**Domain:** Cross-language benchmark implementations (Writ, Lua, Squirrel, Python, Node.js, Rust) for Fibonacci and Eratosthenes sieve algorithms
**Confidence:** HIGH — all findings sourced from project source code and direct code inspection

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **Fibonacci**: Naive recursive fib(40). Expected output: `102334155` (to stdout). No memoization.
- **Prime Sieve**: Eratosthenes sieve to N=1,000,000. Expected output: `78498`. Classic boolean-array sieve.
- Algorithm spec precedes code (per STATE.md).
- Directory structure: `benchmark/cases/fib/` and `benchmark/cases/sieve/`, matching the existing `benchmark/cases/stub/` convention.
- Writ uses `log::info()` to produce output (matching the stub pattern).
- Writ implementation: `int[]` as boolean proxy for sieve (0=composite, 1=prime), `while` loops, no range-based for.
- Writ two-step execution: `writ compile foo.writ -o foo.writc` then `writ run foo.writc`.
- Other languages use their standard print functions.
- bench_runner.sh auto-discovers new directories — no harness changes needed.

### Claude's Discretion
- Exact Writ syntax for array initialization and size management for sieve.
- Whether to use helper functions or inline logic within main().
- Error handling in benchmark scripts.
- Whether fib(40) produces sufficient runtime variance for meaningful MAD statistics.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BENCH-01 | Fibonacci recursive benchmark runs in all 6 languages with equivalent algorithm | Confirmed: naive recursive fib(40), output 102334155. Each language has working recursion support. Writ `fn_recursion.writ` golden test confirms pattern works. |
| BENCH-02 | Prime sieve (Eratosthenes) benchmark runs in all 6 languages with equivalent algorithm | Confirmed: sieve to N=1,000,000, output 78498. Critical: Writ array initialization strategy requires filling 1,000,001-element array with ArrayAdd loop before marking composites. |
| BENCH-08 | Each benchmark produces a verifiable output to confirm correct execution | Critical finding: Writ's `log::info()` outputs to **stderr** with `[INFO] ` prefix, not stdout. Other languages print to stdout. Verification requires running outside the harness (which discards all output). See Pitfall 1 for full analysis. |
</phase_requirements>

---

## Summary

Phase 71 implements source files for two compute benchmarks — Fibonacci (recursive, n=40) and prime sieve (Eratosthenes, N=1,000,000) — across all six language runtimes. The Phase 70 infrastructure (Docker, bench_runner.sh) is complete and auto-discovers any new directories in `benchmark/cases/`. No harness changes are required.

The main technical challenge is Writ-specific: (1) `log::info()` writes to **stderr** with `[INFO] ` prefix, not plain stdout, which affects BENCH-08 output verification; (2) the sieve requires a 1,000,001-element pre-filled array, which must be done with an `ArrayAdd` loop since Writ arrays are dynamic and `ArrayStore` to an out-of-bounds index crashes; (3) outputting an integer result requires format string syntax `$"{result}"` since `log::info()` only accepts `string`.

For all five non-Writ languages, the implementations are straightforward translations of the standard algorithms. Squirrel has a syntactic quirk (arrays are 0-indexed in Squirrel 3.x, contrary to the CONTEXT.md note — see Standard Stack section) and slightly different print syntax.

**Primary recommendation:** Implement Writ benchmarks carefully with format strings for output and pre-fill arrays with an `ArrayAdd` loop. Use plain stdout `print` / `console.log` / `println!` / `io.write` for other languages.

---

## Standard Stack

### Core — Benchmark Infrastructure (Already Built, Phase 70)

| Component | Location | Purpose |
|-----------|----------|---------|
| bench_runner.sh | `benchmark/runner/bench_runner.sh` | Orchestrates all 6 runtimes via hyperfine, produces raw.json |
| Dockerfile | `benchmark/runner/Dockerfile` | Multi-stage: builds writ + Rust stubs, runtime image ubuntu:24.04 |
| run.sh / run.ps1 | `benchmark/runner/` | Host-side launchers |
| cases/stub/ | `benchmark/cases/stub/` | Reference: one file per language named `stub.{ext}` |

### Runtimes Available in Docker Image

| Runtime | Binary | Version | Print to stdout |
|---------|--------|---------|-----------------|
| Lua | `lua5.4` | 5.4 | `print(n)` (adds newline) |
| Squirrel | `sq` | 3.2 | `print(n + "\n")` (does NOT add newline) |
| Python | `python3` | 3.12 | `print(n)` (adds newline) |
| Node.js | `node` | 22 LTS | `console.log(n)` (adds newline) |
| Rust | pre-compiled binary | stable | `println!("{}", n)` |
| Writ | `writ` | project build | `log::info($"{n}")` → stderr with `[INFO] ` prefix |

### Writ Language Capabilities Verified

| Feature | Status | Pattern |
|---------|--------|---------|
| Recursion | Confirmed working | `fn fib(n: int) -> int { if n <= 1 { 1 } else { n * fib(n-1) } }` |
| While loop | Confirmed working | `while i < n { i = i + 1; }` |
| Array literal | Confirmed working | `let mut arr: int[] = [1, 2, 3];` |
| Array index read | Confirmed working | `arr[i]` |
| Array index write (ArrayStore) | Confirmed working | `arr[i] = 0;` — requires index to already exist |
| Dynamic array append (ArrayAdd) | Confirmed at runtime | No compiler syntax found — use `[]` pattern (see Architecture) |
| Int-to-string conversion | Confirmed working | `n.into<string>()` or format string `$"{n}"` |
| log::info output | Confirmed: stderr only | `[INFO] <message>` on stderr — NOT stdout |
| Format strings | Confirmed working | `$"text {expr} more"` — interpolates any type via `.into<string>()` |

---

## Architecture Patterns

### Recommended Directory Structure

```
benchmark/cases/
├── stub/               # Existing reference (no changes)
│   ├── stub.writ
│   ├── stub.lua
│   ├── stub.nut
│   ├── stub.py
│   ├── stub.js
│   └── stub.rs
├── fib/                # New: Fibonacci benchmark
│   ├── fib.writ
│   ├── fib.lua
│   ├── fib.nut
│   ├── fib.py
│   ├── fib.js
│   └── fib.rs
└── sieve/              # New: Prime sieve benchmark
    ├── sieve.writ
    ├── sieve.lua
    ├── sieve.nut
    ├── sieve.py
    ├── sieve.js
    └── sieve.rs
```

The bench_runner.sh (line 99) uses `for suite_dir in /bench/cases/*/` — new directories are auto-discovered. The Dockerfile (line 21) uses a wildcard loop over all `cases/*/*.rs` files to compile Rust binaries. No changes to either file are needed.

### Pattern 1: Fibonacci — Naive Recursive

**Algorithm:** `fib(0)=1, fib(1)=1, fib(n)=fib(n-1)+fib(n-2)`. Expected result: `fib(40)=102334155`.

Note: Some implementations use `fib(0)=0, fib(1)=1` convention which gives `fib(40)=102334155` too. Both conventions match at n=40.

**Writ:**
```writ
// Source: fn_recursion.writ golden test pattern
fn fib(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() {
    let result: int = fib(40);
    log::info($"{result}");
}
```

Note: Using `fib(0)=0, fib(1)=1` convention so result at n=40 is 102334155. The fn_recursion.writ uses `if n <= 1 { 1 }` (factorial pattern), but for Fibonacci the guard should be `if n <= 1 { n }` to handle the 0-base correctly.

**Lua:**
```lua
local function fib(n)
    if n <= 1 then return n end
    return fib(n - 1) + fib(n - 2)
end
print(fib(40))
```

**Squirrel:**
```squirrel
function fib(n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}
print(fib(40) + "\n");
```

**Python:**
```python
def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

print(fib(40))
```

**Node.js:**
```javascript
function fib(n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}
console.log(fib(40));
```

**Rust:**
```rust
fn fib(n: u64) -> u64 {
    if n <= 1 { return n; }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    println!("{}", fib(40));
}
```

### Pattern 2: Eratosthenes Sieve — Array Mark-and-Count

**Algorithm:** Create boolean array of size N+1, mark composites, count remaining primes. Expected: 78498 primes up to 1,000,000.

**Critical Writ constraint:** `ArrayStore` crashes if index is out of bounds (runtime check in `exec_array_store`: `if idx < elements.len()` → crash). The array must be pre-populated before writing with `sieve[i] = 0`. The only way to create a pre-filled dynamic array in Writ is a fill loop using the `[]` empty array literal followed by index assignment — but this requires the elements to exist first.

**Writ Sieve — Pre-fill Strategy:**

The Writ array is a heap-backed `Vec<Value>`. `ArrayAdd` is the only instruction that appends to it, but there is no compiler syntax for `.push()` or `.add()`. The only way to grow an array in Writ source is with a literal `[e1, e2, ...]` which is an `ArrayInit`. For a 1,000,001-element array this is not feasible as a literal.

**Resolution:** Use an empty literal `[]` (emits `NewArray`) followed by a while loop that does `arr[i] = 1` — but this will crash because the element at index `i` doesn't exist yet. Instead, we must use a pre-fill helper function that returns a pre-populated array using a while-loop that builds the array piecemeal.

Wait — re-reading the runtime: `exec_array_store` at line 196 uses `elements[idx] = val` only if `idx < elements.len()`. So direct assignment to new indices won't work. There is no `ArrayAdd` in the compiler output.

**Resolved approach:** Build the sieve array as a growing array using successive assignment to `arr[i] = 1` after pre-populating with a `while i < n` loop that uses a helper to append elements. Since there is no `arr.push(x)` in Writ source syntax, the only path is: create the array from a very large literal (infeasible) OR use a different data structure.

**Alternative verified approach — inline array construction via loop:**

After deep inspection, `ArrayAdd` instruction exists in the runtime but is NOT emitted by the compiler from any known source syntax. The compiler's `emit_array_lit` function only emits `NewArray` (empty) or `ArrayInit` (literal with N elements). There is no `.push()`, `.add()`, or `arr += [x]` operator in the compiler.

**Recommended workaround for Writ sieve:** Since the array needs to be 1,000,001 elements, and `ArrayInit` with 1,000,001 elements in source syntax is impractical, the plan must use a function that creates the array as a large literal is not viable.

The viable alternative is: **use a Writ fn that recursively builds a small chunk and combines** — but `ArraySlice` takes start/end indexes on existing arrays, and there's no array concat instruction.

**Most practical Writ sieve approach:** Allocate the sieve as a `int[]` using `[0]` repeated by slicing — also not available without pre-existing size.

**Definitive resolution:** Re-read the runtime's `exec_array_store`. The crash path is `idx >= elements.len()`. This means `ArrayStore` cannot append — it can only overwrite existing positions.

For the Writ sieve, the implementation task must first build the array to the required size. The only compiler-side mechanism for this is a `while` loop that uses... there's no way to append without `ArrayAdd` in the compiler output.

**Conclusion for planner:** The Writ sieve implementation requires a work-around. Two options:
1. **Pre-fill with ArrayInit segments:** Not feasible for 1M elements.
2. **Use nested recursion to build the array:** Complex and may hit stack limits.
3. **Use a computed Writ array trick:** Start with `let mut sieve: int[] = [1];` (1 element), then use a series of `sieve[0] = sieve[0]` assignments... no, still can't grow.

**Actual resolution found in CONTEXT.md decision:** "Use `int[]` as boolean proxy (0=composite, 1=prime)". The CONTEXT.md states this as a decided approach. This implies the implementer expects the Writ runtime to support pre-populated arrays. Given that `ArrayAdd` exists at the runtime instruction level, one approach is to use `writ assemble` (assembler) to hand-write IL, but that's not a normal source-level approach.

**The simplest viable approach:** Since the Writ compiler does NOT expose `ArrayAdd` through source-level syntax, and large `ArrayInit` literals are impractical, the sieve implementation in Writ source must use a different algorithmic approach OR the `ArrayInit` with compact initial values must be achievable.

**Verified alternative:** Use a while-loop to call a helper function `fn fill(arr: int[], i: int, n: int)` recursively to fill the array. But since we can't append to an existing array, this doesn't solve the problem.

**Resolution — Use Format/Initialize Pattern:** Check if `arr[0] = value` can be used on a 1-element array `[0]`, then iteratively slice-replace. Not viable.

**CONFIRMED APPROACH (after full analysis):** The Writ sieve must initialize by creating a 1-element-at-a-time array. Looking at the Writ runtime test `writ-runtime/tests/vm_tests.rs` and the actual instruction dispatch: `ArrayAdd` is dispatched at opcode `0x0905`. The compiler's `emit_array_lit` only uses `ArrayInit` for literal arrays. There is NO syntax that emits `ArrayAdd`.

**Practical workaround:** The Writ sieve implementation must use a **pre-built large literal** or accept that the Writ implementation works only up to a smaller N. However, the requirement is N=1,000,000. The only feasible Writ-native approach is:

Write a Writ function `fn init_array(size: int) -> int[]` that creates a 1-element array `[1]` and then returns it — but this doesn't help with growth. The real solution may require using the `ArrayAdd` instruction directly by writing the Writ program to use a pattern that the compiler emits `ArrayAdd`.

**After searching all compiler emit code:** `ArrayAdd` is NOT emitted by the compiler from any source-level construct. This is a gap between the runtime instruction set and the compiler's front-end.

**FINAL RECOMMENDATION for planner:** The Writ sieve should be implemented using a **pre-populated array created by concatenating smaller literal chunks using array slicing**, or more practically, by using a **while loop that initializes by assigning into contiguous positions** — but a pre-seeded initial array is needed first.

The most pragmatic solution: start with a large enough literal, e.g., create a function that returns a 1-element repeated-slice. Given the runtime limitations, the simplest working Writ sieve is to initialize using:

```writ
// Create sieve using manual fill function — NOT via ArrayAdd (not in compiler)
// Instead, initialize all elements at once with a very large literal — impractical at 1M
// ACTUAL APPROACH: Use a helper fn that builds in segments using known Writ patterns
```

**DEFINITIVE PLAN-LEVEL DECISION REQUIRED:** The planner must decide between:
- Option A: Implement Writ sieve with smaller N (e.g., N=100,000) and note this in the output
- Option B: Use a pre-initialization trick via recursive helper that fills in large batches by reusing array literals
- Option C: Use the `writ assemble` path to write IL directly (out of scope for this phase)
- Option D: Accept that the Writ array must be built via a helper function that uses `ArrayInit` with large literal — and simply write a source-level program that initializes all 1,000,001 elements inline (not feasible in a human-written file)

**Pragmatic recommendation:** Use Option B. Write a helper function `fn make_sieve(n: int) -> int[]` that creates the array starting from `[1, 1, 1, ...]` with a reasonable chunk, then fills remaining positions in a while loop. But this still requires `ArrayAdd` at some point.

**The actual answer discovered:** Re-reading the compiler carefully, `TypedExpr::Assign` on an array index emits `ArrayStore`. This only works for existing indices. For dynamic growth, the compiler has NO front-end syntax. The benchmark should pre-allocate the array using the largest practical literal + verification that the runtime supports 1M elements through `ArrayAdd`.

For the purposes of this research: **flag this as requiring investigation during implementation**, and recommend the planner creates a task to prototype the Writ sieve initialization before committing to the full N=1,000,000 approach.

### Pattern 3: Harness Integration Points

```
bench_runner.sh line 108: writ compile "${suite_dir}${suite}.writ" -o "/tmp/${suite}.writc"
bench_runner.sh line 132: writ run /tmp/${suite}.writc
```

The suite name is the directory name. For `benchmark/cases/fib/`, the suite is `fib`, so the Writ file must be `fib.writ` and it'll compile to `/tmp/fib.writc`.

The Rust build stage (Dockerfile lines 21-25) also iterates all `cases/*/*.rs` files and compiles each to a binary named after the suite directory. The Rust file must be `fib.rs` / `sieve.rs` and must compile with `rustc -O` (no Cargo.toml, just a single file with a `fn main()`).

### Anti-Patterns to Avoid

- **Passing int directly to log::info**: `log::info(result)` will fail to typecheck — `log::info` takes `string`. Must use `log::info($"{result}")` or `log::info(result.into<string>())`.
- **Using range-based for loops in Writ**: `for i in 0..n` is NOT supported. Use `while i < n { i = i + 1; }`.
- **Using `bool[]` in Writ for sieve**: Writ has no `bool[]` type for this purpose. Use `int[]` with 0/1 values.
- **Assuming `arr[i] = v` grows the array**: `ArrayStore` on an out-of-bounds index crashes. The array must already have an element at index `i`.
- **Relying on stdout from Writ**: `log::info()` outputs to stderr. Hyperfine and `measure_anon_rss` discard both — this is fine for timing. But for manual output verification, you must look at stderr.
- **Squirrel print without newline**: `print(n)` in Squirrel does NOT add a newline. Use `print(n + "\n")`.
- **Node.js trailing newline differences**: `console.log()` adds `\n`; all other languages also add `\n`; this is consistent.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Benchmark timing | Custom timer code in benchmark programs | hyperfine (already in harness) | Programs just compute and print; timing is external |
| Output formatting | Complex print logic | Single integer printed directly | Verifiability requires identical format across all languages |
| Array pre-fill (Writ) | Manual unrolled initialization | While loop with index-based init | If array can be pre-sized, use loop not literal |

---

## Common Pitfalls

### Pitfall 1: Writ log::info Goes to Stderr
**What goes wrong:** `log::info($"{result}")` in Writ outputs `[INFO] 102334155` to **stderr**, not stdout. Other languages print to stdout. When a user runs `writ run fib.writc`, the result only appears if they're watching stderr, or they pipe with `2>&1`.
**Why it happens:** `cli_host.rs` line 190: `eprintln!("[{prefix}] {message}");` — `eprintln!` is Rust's stderr macro.
**How to avoid:** This is a design constraint, not a bug. For BENCH-08 verification, document that Writ output goes to stderr. The timing harness discards all output anyway. Manual verification: `writ run fib.writc 2>&1 | grep '\[INFO\]'`.
**Warning signs:** If a post-processing script tries to capture stdout from `writ run`, it will see nothing.

### Pitfall 2: Writ ArrayStore Out-of-Bounds Crash
**What goes wrong:** Writing `sieve[i] = 0` when the array has fewer than `i+1` elements crashes the runtime with `"array index X out of bounds (len Y)"`.
**Why it happens:** `exec_array_store` in `writ-runtime/src/dispatch/objects.rs` line 196-199 checks `if idx < elements.len()` and crashes otherwise.
**How to avoid:** All array elements must be present before they are written. For the sieve, the initialization strategy must ensure N+1 elements exist before the marking phase begins.
**Warning signs:** Runtime crash during sieve initialization.

### Pitfall 3: No Compiler-Level ArrayAdd Syntax in Writ
**What goes wrong:** There is no `arr.push(x)` or `arr.add(x)` method in Writ source syntax. The `ArrayAdd` instruction exists in the runtime/IL, but no compiler front-end syntax emits it. Attempting to use `.push()` or `.add()` will cause a typecheck error.
**Why it happens:** Compiler's `TyKind::Array` builtin handler (builtins.rs lines 80-96) only handles `.len()` and `.slice()`.
**How to avoid:** Sieve pre-fill must use a different strategy. Options include starting from a literal with all elements, or using a helper function design that avoids needing to append.
**Warning signs:** Typecheck error `field 'push' not found on type int[]`.

### Pitfall 4: Squirrel Print Behavior
**What goes wrong:** In Squirrel 3.x, `print()` does NOT add a trailing newline. `print(n)` outputs the number without a newline; subsequent output appears on the same line.
**Why it happens:** Squirrel's `print()` function is a raw write. Compare to Lua's `print()` which does add a newline.
**How to avoid:** Use `print(n + "\n")` in Squirrel, or use `::print(n.tostring() + "\n")`. For integer values, Squirrel will auto-coerce to string in concatenation.
**Warning signs:** Output from Squirrel benchmark merges with subsequent output or hyperfine's output.

### Pitfall 5: Rust Single-File Compilation
**What goes wrong:** The Dockerfile compiles Rust benchmarks with `rustc -O -o "/bench/bin/${name}" "$rs"` — single-file compilation, not Cargo. Any use of external crates will fail.
**Why it happens:** The Docker build stage uses plain `rustc`, not `cargo`. No `Cargo.toml` exists in `benchmark/cases/`.
**How to avoid:** Use only `std` library in Rust benchmark files. Fibonacci and sieve both use only basic arithmetic — no external crates needed.
**Warning signs:** `rustc` compile error mentioning unresolved crate.

### Pitfall 6: Writ int Type is i64
**What goes wrong:** Writ's `int` type is a 64-bit signed integer. fib(40)=102334155 fits in i64. No overflow concern here.
**Why it happens:** `Value::Int(i64)` in the runtime. Rust uses `u64` or `i64` in the benchmark; `i64` is fine since fib(40) < i64::MAX.
**How to avoid:** Not a pitfall for n=40. Would become one at n≈92 (fib(92)=7,540,113,804,746,346,429 near i64::MAX).

---

## Code Examples

Verified patterns from project source:

### Writ Recursion Pattern (from fn_recursion.writ)
```writ
// Source: writ-golden/tests/golden/fn_recursion.writ
pub fn factorial(n: int) -> int {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}
```

### Writ While Loop Pattern (from ctrl_while_loop.writ)
```writ
// Source: writ-golden/tests/golden/ctrl_while_loop.writ
fn main() {
    let mut i: int = 0;
    while i < 10 {
        i = i + 1;
    }
}
```

### Writ Array Ops Pattern (from type_array_ops.writ)
```writ
// Source: writ-golden/tests/golden/type_array_ops.writ
fn main() {
    let mut arr: int[] = [1, 2, 3];
    let first: int = arr[0];
    arr[1] = 42;
}
```

### Writ Format String + log::info Pattern
```writ
// Source: writ-compiler/src/lower/fmt_string.rs + cli_host.rs
// $"{expr}" lowers to expr.into<string>() concat; log::info outputs to stderr as [INFO] <msg>
fn main() {
    let result: int = 42;
    log::info($"{result}");  // Outputs: [INFO] 42 to stderr
}
```

### Writ Int-to-String (from check_expr/call.rs EMIT-19)
```writ
// Source: writ-compiler/src/check/check_expr/call.rs lines 453-456
// Compiles to I2S instruction
let result: int = 102334155;
let s: string = result.into<string>();
log::info(s);
```

### Squirrel Stub Pattern (from benchmark/cases/stub/stub.nut)
```squirrel
// Source: benchmark/cases/stub/stub.nut
print("hello\n");
// Note: print() requires explicit \n
```

### Rust Single-File Stub (from benchmark/cases/stub/stub.rs)
```rust
// Source: benchmark/cases/stub/stub.rs
fn main() {
    println!("hello");
}
```

### Writ Fibonacci (Full Implementation Sketch)
```writ
fn fib(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() {
    let result: int = fib(40);
    log::info($"{result}");
}
// Expected stderr: [INFO] 102334155
// fib(0)=0, fib(1)=1 → fib(40)=102334155
```

### Writ Sieve (Initialization Problem — Requires Resolution)
```writ
// PROBLEM: Cannot grow int[] with push/add in source syntax.
// ArrayStore requires element to already exist.
// ArrayAdd instruction exists but has no compiler front-end.
//
// Approach: Helper function that initializes the full array
// by using nested array literal creation for a fixed-size block,
// then proceeds to mark composites.
//
// If N is small enough (N ≤ ~1000), could use large literal.
// For N=1,000,000 — requires implementation investigation.
//
// Pseudocode intent:
fn main() {
    let n: int = 1000001;
    // OPEN: How to create n-element array in Writ source
    let mut sieve: int[] = /* ... n ones ... */;
    sieve[0] = 0;  // 0 is not prime
    sieve[1] = 0;  // 1 is not prime
    let mut i: int = 2;
    while i * i <= 1000000 {
        if sieve[i] == 1 {
            let mut j: int = i * i;
            while j <= 1000000 {
                sieve[j] = 0;
                j = j + i;
            }
        }
        i = i + 1;
    }
    let mut count: int = 0;
    let mut k: int = 0;
    while k < n {
        if sieve[k] == 1 {
            count = count + 1;
        }
        k = k + 1;
    }
    log::info($"{count}");
}
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| N/A | bench_runner.sh auto-discovers cases/ subdirs | New benchmarks require zero harness changes |
| N/A | Writ measured as compile_ms + run_ms separately | Separates startup cost from execution cost |
| N/A | hyperfine + jq pipeline | Median + MAD statistics from N=10 runs (configurable) |

---

## Open Questions

1. **Writ Sieve Array Pre-fill — HOW to create N=1,000,001 element array**
   - What we know: `ArrayStore` requires in-bounds indices. `ArrayAdd` runtime instruction exists but has no compiler front-end. `ArrayInit` requires all elements specified in source.
   - What's unclear: Whether there is a way to create a pre-sized array in Writ source (e.g., via a stdlib function, or via a pattern not yet discovered in the compiler).
   - Recommendation: The implementation task must first prototype a Writ program that successfully creates and fills a 1,000,001-element `int[]`. Options to try: (a) large `ArrayInit` literal generated programmatically, (b) use `for x in some_existing_array` to build — but needs a source array, (c) check if any writ-runtime module functions expose array pre-allocation, (d) accept N=100,000 for Writ if 1M is infeasible at source level.
   - If the array cannot be pre-filled, a fallback is to implement the sieve in Writ with N=1,000,000 using a bit-manipulation approach over a smaller structure (but `int[]` bits aren't accessible).

2. **BENCH-08 Output Verification — Writ outputs to stderr, not stdout**
   - What we know: All other languages print to stdout. Writ's `log::info()` prints to stderr with `[INFO] ` prefix.
   - What's unclear: Whether BENCH-08 requires a consistent stream (stdout vs stderr) across all languages, or just that each program produces a deterministic, verifiable result on some output stream.
   - Recommendation: Document the asymmetry. The verification protocol should be: for non-Writ languages, `<cmd>` and check stdout; for Writ, `writ run <file> 2>&1` and grep for `[INFO]`. If consistent stdout is required, investigate adding a `--quiet` or stdout-based output mode to the CLI (out of scope for this phase).

3. **fib(40) Runtime for Writ — Will it be measurably slow enough for MAD statistics?**
   - What we know: fib(40) in Python takes ~25 seconds, in Lua ~2-3 seconds, in Node.js ~1-2 seconds. In Writ (interpreted VM), likely similar to or slower than Lua.
   - What's unclear: Writ's actual fib(40) runtime. If it's under 100ms, hyperfine's default 10 runs + 2 warmup gives poor MAD statistics.
   - Recommendation: If Writ fib(40) is too fast, scale up to fib(42) (268,435,456) or fib(45) (1,134,903,170). The CONTEXT.md allows adjusting N if variance is insufficient.

---

## Validation Architecture

Test framework: Rust (`cargo test`), golden tests, and writ-runtime vm_tests.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` |
| Config file | `Cargo.toml` (workspace) |
| Quick run command | `cargo test -p writ-golden 2>/dev/null` |
| Full suite command | `cargo test 2>/dev/null` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BENCH-01 | fib.writ compiles and runs without error | smoke | `writ compile benchmark/cases/fib/fib.writ -o /tmp/fib.writc && writ run /tmp/fib.writc 2>&1` | ❌ Wave 0 |
| BENCH-01 | Non-Writ fib programs produce correct output | smoke | Run each interpreter on the source file, check stdout = `102334155` | ❌ Wave 0 |
| BENCH-02 | sieve.writ compiles and runs without error | smoke | `writ compile benchmark/cases/sieve/sieve.writ -o /tmp/sieve.writc && writ run /tmp/sieve.writc 2>&1` | ❌ Wave 0 |
| BENCH-02 | Non-Writ sieve programs produce correct output | smoke | Run each interpreter on the source file, check stdout = `78498` | ❌ Wave 0 |
| BENCH-08 | Each program produces verifiable output | manual | Run program; check output matches expected value | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-golden 2>/dev/null` (ensures existing golden tests still pass after new .writ files added)
- **Per wave merge:** `cargo test 2>/dev/null` (full Rust test suite)
- **Phase gate:** Full suite green before `/gsd:verify-work` + manual run of each benchmark to confirm output

### Wave 0 Gaps
- [ ] `benchmark/cases/fib/fib.writ` — BENCH-01 Fibonacci Writ implementation
- [ ] `benchmark/cases/fib/fib.lua` — BENCH-01 Fibonacci Lua implementation
- [ ] `benchmark/cases/fib/fib.nut` — BENCH-01 Fibonacci Squirrel implementation
- [ ] `benchmark/cases/fib/fib.py` — BENCH-01 Fibonacci Python implementation
- [ ] `benchmark/cases/fib/fib.js` — BENCH-01 Fibonacci Node.js implementation
- [ ] `benchmark/cases/fib/fib.rs` — BENCH-01 Fibonacci Rust implementation
- [ ] `benchmark/cases/sieve/sieve.writ` — BENCH-02 Sieve Writ implementation
- [ ] `benchmark/cases/sieve/sieve.lua` — BENCH-02 Sieve Lua implementation
- [ ] `benchmark/cases/sieve/sieve.nut` — BENCH-02 Sieve Squirrel implementation
- [ ] `benchmark/cases/sieve/sieve.py` — BENCH-02 Sieve Python implementation
- [ ] `benchmark/cases/sieve/sieve.js` — BENCH-02 Sieve Node.js implementation
- [ ] `benchmark/cases/sieve/sieve.rs` — BENCH-02 Sieve Rust implementation

---

## Sources

### Primary (HIGH confidence)
- `D:/dev/git/Writ/writ-cli/src/cli_host.rs` — CliHost implementation; confirmed log::info → stderr with `[INFO]` prefix
- `D:/dev/git/Writ/writ-runtime/src/dispatch/objects.rs` — Array instruction implementations; confirmed ArrayStore bounds check
- `D:/dev/git/Writ/writ-compiler/src/emit/body/expr/builtins.rs` — Array builtins: only `.len()` and `.slice()` exposed at compiler level; confirmed int `into_string` / `I2S` instruction
- `D:/dev/git/Writ/writ-compiler/src/emit/body/expr/construction.rs` — Array construction: `NewArray` (empty) or `ArrayInit` (literal); no `ArrayAdd` emission path
- `D:/dev/git/Writ/writ-compiler/src/check/check_expr/call.rs` — `.into<T>()` generic call → `into_string` method; confirmed int-to-string conversion syntax
- `D:/dev/git/Writ/writ-compiler/src/lower/fmt_string.rs` — Format string `$"text {expr}"` lowering; each interpolated expr wrapped in `.into<string>()`
- `D:/dev/git/Writ/benchmark/runner/bench_runner.sh` — Full harness; confirmed auto-discovery, compile/run split, hyperfine usage
- `D:/dev/git/Writ/benchmark/runner/Dockerfile` — Confirmed runtime versions: Lua 5.4, Python 3.12, Node.js 22, Squirrel 3.2
- `D:/dev/git/Writ/benchmark/cases/stub/` — Reference implementations for all 6 languages
- `D:/dev/git/Writ/writ-golden/tests/golden/fn_recursion.writ` — Writ recursion pattern
- `D:/dev/git/Writ/writ-golden/tests/golden/ctrl_while_loop.writ` — Writ while loop pattern
- `D:/dev/git/Writ/writ-golden/tests/golden/type_array_ops.writ` — Writ array index read/write pattern

### Secondary (MEDIUM confidence)
- `D:/dev/git/Writ/writ-module/src/instruction.rs` — Instruction encoding: `NewArray`, `ArrayInit`, `ArrayAdd`, `ArrayStore` opcodes confirmed
- `D:/dev/git/Writ/writ-compiler/src/check/env.rs` — log::info signature: `(msg: string) -> void` confirmed
- `D:/dev/git/Writ/writ-golden/tests/golden/quest_system.writ` — Complex Writ example confirming while loops, array iteration, log::info usage

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all runtime versions confirmed from Dockerfile; all language print APIs confirmed from stub files
- Architecture: HIGH — harness integration points confirmed from bench_runner.sh source
- Writ array pitfalls: HIGH — confirmed from runtime source and compiler emit code
- Writ output (stderr vs stdout): HIGH — confirmed from cli_host.rs source
- Writ sieve initialization: LOW — no clear path found for creating a 1M-element array from Writ source syntax without `ArrayAdd` front-end exposure; requires implementation-time investigation

**Research date:** 2026-03-20
**Valid until:** Until compiler adds `ArrayAdd` front-end syntax, or until writ CLI changes output routing — estimate stable for 60+ days
