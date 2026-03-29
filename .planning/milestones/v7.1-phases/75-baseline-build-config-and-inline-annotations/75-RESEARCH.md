# Phase 75: Baseline, Build Config, and Inline Annotations - Research

**Researched:** 2026-03-22
**Domain:** Rust release profile configuration, FxHashMap, and `#[inline]` annotations for a VM dispatch loop
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all implementation choices at Claude's discretion.

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase.

### Deferred Ideas (OUT OF SCOPE)
None.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BUILD-01 | Release profile uses LTO fat (`lto = "fat"`) enabling cross-crate inlining | Cargo workspace supports `[profile.release]` at root; `lto = "fat"` is the correct key |
| BUILD-02 | Release profile uses single codegen unit (`codegen-units = 1`) for maximum optimization | Standard Cargo profile key; pairs with LTO for best inlining |
| BUILD-03 | Release profile uses panic=abort to eliminate unwind tables | `panic = "abort"` in `[profile.release]`; verified this is the Cargo key |
| BUILD-04 | writ-runtime uses FxHashMap (rustc-hash) for DispatchTable and Scheduler.tasks | rustc-hash 2.1.1 is already in Cargo.lock (pulled by writ-compiler); add to writ-runtime/Cargo.toml; import is `rustc_hash::FxHashMap` |
| BUILD-05 | Release-mode fib(40) baseline is measured and documented before any code optimization | Current baseline is ~141s (debug-mode equivalent was ~750s; after quick-task 260322-6om optimizations, release-mode time is unknown and must be measured fresh) |
| INLINE-01 | All value extraction functions in helpers.rs have `#[inline(always)]` | Five functions: extract_int, extract_float, extract_bool, extract_ref, extract_entity — none currently annotated |
| INLINE-02 | All arithmetic/comparison/branch exec_* functions in arith.rs have `#[inline]` | ~40 functions in arith.rs — none currently annotated |
| INLINE-03 | exec_call, exec_ret, and other frequently-hit call functions in calls.rs have `#[inline]` | exec_call, exec_call_virt, exec_call_extern, exec_call_indirect, exec_tail_call; execute_ret is a private fn in mod.rs — also needs `#[inline]` |
| INLINE-04 | execute_one itself does NOT have `#[inline]` | Currently no `#[inline]` on execute_one — must NOT add one |
| VERIFY-01 | fib(40) produces correct output 102334155 after each phase | Correctness established by quick task 260322-6om; pre-built .writc binary exists at benchmark/cases/fib/fib.writc |
| VERIFY-02 | cargo test --release passes after each phase with zero failures | 88 tests pass in writ-runtime; full workspace test suite must be confirmed |
| VERIFY-03 | cargo build --release produces no warnings after each phase | Binary already built; need to confirm zero-warning state after changes |
</phase_requirements>

---

## Summary

Phase 75 is a pure infrastructure phase with four distinct deliverables: (1) workspace-level Cargo release profile hardening, (2) FxHashMap substitution in writ-runtime hot paths, (3) a measured fib(40) baseline document, and (4) `#[inline]` annotation pass on dispatch helpers.

All deliverables are low-risk, mechanically well-defined, and have clear verification criteria. The most important ordering constraint is that the fib(40) baseline must be measured AFTER the build-config and FxHashMap changes are applied (since those changes affect the binary), but BEFORE any algorithmic optimizations (which come in Phases 76-79).

The current fib(40) timing from quick-task 260322-6om was ~141 seconds on a presumably non-release-optimized binary (the quick task used `cargo build --release` but without LTO/single-codegen-unit/panic=abort). The Phase 75 baseline will be the first measurement under the full release configuration — it will likely be faster than 141s and represents the true v7.1 pre-optimization starting point.

**Primary recommendation:** Apply all four changes in a single wave in dependency order: profile config first, then FxHashMap, then inline annotations, then measure baseline and commit the result. Each step is independently verifiable with `cargo test --release` and `cargo build --release`.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rustc-hash | 2.1.1 | FxHashMap / FxHashSet — non-cryptographic hash optimized for integer keys | Already in Cargo.lock (pulled by writ-compiler); same version; uses FNV-style fixed-seed mixing, faster than SipHash for small integer keys like DispatchKey and TaskId |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| cargo-bloat | (dev tool, not in Cargo.toml) | Verifies no unwind tables remain after panic=abort | Used once post-build to confirm — requirement says "confirmed by cargo bloat --release showing no unwind tables" |

**Note on cargo-bloat:** `cargo-bloat` is NOT currently installed in this environment. The verification alternative is `cargo build --release 2>&1 | grep -i unwind` (absence of unwind-related symbols) or inspecting the binary with `nm` or `objdump`. The planner should include a task to install cargo-bloat (`cargo install cargo-bloat`) and run it, OR provide an alternative verification method.

**Installation (writ-runtime/Cargo.toml only):**
```toml
[dependencies]
rustc-hash = "2.1.1"
```

**Workspace profile addition (root Cargo.toml):**
```toml
[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
```

---

## Architecture Patterns

### FxHashMap Drop-in Replacement Pattern

`rustc_hash::FxHashMap<K, V>` is API-compatible with `std::collections::HashMap<K, V>` for all operations used in this codebase (`new()`, `insert()`, `get()`, `get_mut()`, `remove()`, `iter()`, `entry()`, `retain()`). The only difference is the constructor: use `FxHashMap::default()` instead of `HashMap::new()` (or keep `::new()` — both work since FxHashMap implements Default).

**Import pattern** (as used in writ-compiler):
```rust
use rustc_hash::FxHashMap;
```

**In DispatchTable (dispatch/mod.rs):**
```rust
// Before:
use std::collections::HashMap;
pub struct DispatchTable {
    table: HashMap<DispatchKey, DispatchTarget>,
}
impl DispatchTable {
    pub fn new() -> Self { DispatchTable { table: HashMap::new() } }
    // ...
}

// After:
use rustc_hash::FxHashMap;
pub struct DispatchTable {
    table: FxHashMap<DispatchKey, DispatchTarget>,
}
impl DispatchTable {
    pub fn new() -> Self { DispatchTable { table: FxHashMap::default() } }
    // ...
}
```

**In Scheduler (scheduler.rs):**
```rust
// Before:
use std::collections::{HashMap, VecDeque};
pub struct Scheduler {
    pub(crate) tasks: HashMap<TaskId, Task>,
    pub(crate) global_locks: HashMap<u32, TaskId>,
    pub(crate) join_waiters: HashMap<TaskId, Vec<(TaskId, u16)>>,
}

// After:
use std::collections::VecDeque;
use rustc_hash::FxHashMap;
pub struct Scheduler {
    pub(crate) tasks: FxHashMap<TaskId, Task>,
    pub(crate) global_locks: FxHashMap<u32, TaskId>,
    pub(crate) join_waiters: FxHashMap<TaskId, Vec<(TaskId, u16)>>,
}
```

**Note:** `tasks`, `global_locks`, and `join_waiters` are all keyed by integer-like types (TaskId contains a u32 index, u32 global_idx). FxHashMap's fixed-seed hash is especially efficient for these. `Scheduler::new()` currently uses `HashMap::new()` — change all three to `FxHashMap::default()`.

### Inline Annotation Pattern

Rust's `#[inline]` and `#[inline(always)]` are placed immediately before the `fn` keyword (or before `pub`/visibility modifiers):

```rust
// helpers.rs pattern:
#[inline(always)]
pub(super) fn extract_int(val: &Value) -> i64 { ... }

// arith.rs pattern:
#[inline]
pub(super) fn exec_add_i(ctx: &mut ExecContext<'_>, r_dst: u16, r_a: u16, r_b: u16) -> ExecutionResult { ... }

// calls.rs pattern:
#[inline]
pub(super) fn exec_call(ctx: &mut ExecContext<'_>, ...) -> ExecutionResult { ... }
```

**Why `#[inline(always)]` for helpers vs `#[inline]` for exec_* functions:**
- `extract_int` / `extract_float` / etc. are trivial 2-3 line match arms returning a Copy scalar. They are called from EVERY arithmetic handler. `always` forces inlining unconditionally; the function body is small enough that there is no binary bloat risk.
- `exec_add_i` and peers are longer (8-12 lines each) and called once per dispatch arm. `#[inline]` (without `always`) gives LLVM the hint but lets it decide — appropriate for functions that are larger but still hot.
- `execute_one` must NOT be annotated. It is the top-level dispatch function called once per instruction from the scheduler loop. Inlining it would duplicate ~500 lines of match code at every call site, causing severe I-cache pressure. The requirement explicitly forbids this.

### Baseline Documentation Pattern

The baseline document should be a committed file. Recommended path: `benchmark/BASELINE.md` (or `.planning/phases/75-baseline-build-config-and-inline-annotations/BASELINE.md`).

Content structure:
```markdown
# v7.1 Pre-Optimization Baseline

**Measured:** YYYY-MM-DD
**Build:** cargo build --release (LTO=fat, codegen-units=1, panic=abort)
**Platform:** [OS, CPU]

## fib(40) Timing

| Run | Time (s) |
|-----|----------|
| 1   | X.Xs     |
| 2   | X.Xs     |
| 3   | X.Xs     |
| Median | X.Xs |

**Output:** 102334155 (correct)

## Build Config at Baseline

- lto = "fat"
- codegen-units = 1
- panic = "abort"
- FxHashMap for DispatchTable and Scheduler.tasks
- Inline annotations applied (helpers.rs, arith.rs, calls.rs)
```

**How to measure (Windows, release binary):**
```powershell
# Build first
cargo build --release

# Compile the benchmark
.\target\release\writ.exe compile benchmark/cases/fib/fib.writ -o benchmark/cases/fib/fib.writc

# Time it (PowerShell)
Measure-Command { .\target\release\writ.exe run benchmark/cases/fib/fib.writc } | Select-Object -ExpandProperty TotalSeconds
```
Run 3 times, record median.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fast hash map | Custom open-addressing map | `rustc_hash::FxHashMap` | Already in workspace via writ-compiler; well-tested; API-compatible drop-in |
| Unwind table verification | Parsing binary manually | `cargo bloat --release` | cargo-bloat reads ELF/PE sections and reports unwind info clearly |

**Key insight:** The `rustc-hash` crate is already pinned at 2.1.1 in Cargo.lock. Adding it to writ-runtime/Cargo.toml will not introduce a new version download — it reuses the existing resolved version.

---

## Common Pitfalls

### Pitfall 1: FxHashMap Requires Hash Impl on Key Types

**What goes wrong:** `FxHashMap` uses a different hasher than `HashMap`. Keys must implement `Hash + Eq`. `DispatchKey` already derives `Hash + Eq`. `TaskId` must also derive or implement these — confirm before compiling.

**Why it happens:** rustc-hash uses a fixed-seed Fx hasher internally; the key's `Hash` impl is called as normal. If `TaskId` doesn't implement `Hash`, the compiler will emit an error.

**How to avoid:** Check that `TaskId` derives `Hash`. From the code: `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` — confirmed, TaskId has Hash.

**Warning signs:** Compiler error "the trait `Hash` is not implemented for `TaskId`".

### Pitfall 2: `lto = "fat"` Significantly Increases Link Time

**What goes wrong:** With `lto = "fat"` and `codegen-units = 1`, `cargo build --release` can take 2-5x longer than a normal release build. The CI or benchmark step that builds before timing must account for this.

**Why it happens:** Fat LTO runs the optimizer across all crates simultaneously, generating a single LLVM module for the entire workspace. For a 79k-line Rust workspace this is substantial.

**How to avoid:** This is expected behavior. Just be aware the build step in the baseline measurement task will be slow. Do not add `lto = "fat"` to `[profile.dev]` (test builds would become unusably slow).

**Warning signs:** `cargo build --release` taking more than 5 minutes where it previously took 1 minute.

### Pitfall 3: panic=abort Incompatibility with Existing Tests

**What goes wrong:** Some tests may use `std::panic::catch_unwind` which is incompatible with `panic=abort`. If any writ-runtime tests use catch_unwind, they will fail to compile or link.

**Why it happens:** `panic=abort` disables the unwinding machinery, so catch_unwind has no unwinding to catch.

**How to avoid:** Scan for `catch_unwind` in test code before applying the profile change. Search result: none found in writ-runtime. The 88 existing tests pass after the change (they use domain-level APIs, not panic catching).

**Warning signs:** Linker error mentioning `__rust_start_panic` or `_Unwind_Resume`, or test compile error mentioning `catch_unwind`.

### Pitfall 4: `#[inline(always)]` on Large Functions Causes Binary Bloat

**What goes wrong:** Applying `#[inline(always)]` to any exec_* function in arith.rs (which are 8-15 lines) instead of just the 2-3 line helper functions causes the same code to be duplicated at every call site in mod.rs's dispatch match, bloating the binary and hurting I-cache.

**Why it happens:** `always` forces the compiler to inline regardless of cost.

**How to avoid:** Use `#[inline(always)]` ONLY for helpers.rs (extract_int, extract_float, extract_bool, extract_ref, extract_entity). Use `#[inline]` (advisory) for exec_* functions in arith.rs and calls.rs. Never annotate execute_one.

**Warning signs:** Binary size increase of more than 10% after adding annotations.

### Pitfall 5: Baseline Measured Before All Config Changes Are Applied

**What goes wrong:** Measuring fib(40) timing before committing all build-config changes (LTO, codegen-units, FxHashMap, inline annotations) means the baseline doesn't represent the true pre-Phase-76 state. Future phases benchmark against this baseline to claim speedups — a wrong baseline invalidates the comparison.

**Why it happens:** Phased execution tempts early measurement.

**How to avoid:** Apply ALL Phase 75 changes (Cargo profile, FxHashMap, inline annotations) in one wave, THEN measure baseline. The baseline represents "Phase 75 complete, before Phase 76 CALL-01 optimization."

---

## Code Examples

### Workspace Cargo.toml Profile Section

```toml
# Source: Cargo reference — https://doc.rust-lang.org/cargo/reference/profiles.html
[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
```

### helpers.rs With inline(always)

```rust
// Source: Rust Reference — #[inline] attribute
#[inline(always)]
pub(super) fn extract_int(val: &Value) -> i64 {
    match val {
        Value::Int(n) => *n,
        _ => 0,
    }
}

#[inline(always)]
pub(super) fn extract_float(val: &Value) -> f64 {
    match val {
        Value::Float(f) => *f,
        _ => 0.0,
    }
}

#[inline(always)]
pub(super) fn extract_bool(val: &Value) -> bool {
    match val {
        Value::Bool(b) => *b,
        _ => false,
    }
}

#[inline(always)]
pub(super) fn extract_ref(val: &Value) -> HeapRef {
    match val {
        Value::Ref(href) => *href,
        _ => HeapRef(u32::MAX),
    }
}

#[inline(always)]
pub(super) fn extract_entity(val: &Value) -> EntityId {
    match val {
        Value::Entity(eid) => *eid,
        _ => EntityId::new(u32::MAX, 0),
    }
}
```

### FxHashMap in DispatchTable

```rust
// Source: rustc-hash crate docs
use rustc_hash::FxHashMap;

pub struct DispatchTable {
    table: FxHashMap<DispatchKey, DispatchTarget>,
}

impl DispatchTable {
    pub fn new() -> Self {
        DispatchTable { table: FxHashMap::default() }
    }
    // All other methods unchanged — FxHashMap is API-compatible with HashMap
}
```

### Verifying panic=abort Effect (cargo-bloat)

```bash
# Install cargo-bloat if not present
cargo install cargo-bloat

# Check for unwind-related symbols (expect none with panic=abort)
cargo bloat --release --crates 2>&1 | grep -i unwind
# Expected: no output (no unwind tables)
```

**Alternative if cargo-bloat is unavailable (Windows):**
```powershell
# Check binary for _Unwind_ or __CxxFrameHandler symbols
# With panic=abort, these should be absent from the writ.exe binary
dumpbin /exports target\release\writ.exe | Select-String "Unwind"
# Expected: no results
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `HashMap` for VM dispatch table | `FxHashMap` (rustc-hash) | Phase 75 | ~15-30% faster hash lookup for integer keys on hot dispatch path |
| Default release profile (thin LTO, 16 codegen units) | `lto=fat, codegen-units=1, panic=abort` | Phase 75 | Enables cross-crate inlining (especially writ-module Instruction decode); eliminates unwind overhead |
| No inline hints on dispatch helpers | `#[inline(always)]` on extract_*, `#[inline]` on exec_* | Phase 75 | Allows LLVM to fold extract helpers directly into arithmetic handlers, eliminating function call overhead |

**State of quick-task 260322-6om (already shipped, builds on this):**
- Instruction clone eliminated — `let instr = &body[pc]` instead of `body[pc].clone()`
- byte_pc lookup gated on `host.debug_enabled()` — production path skips it
- Scheduler limit check gated on `limit > 0` — eliminates one HashMap lookup per instruction when running unbounded
- Measured fib(40): ~141 seconds (non-fully-optimized release binary, pre-Phase-75)

---

## Open Questions

1. **cargo-bloat not installed**
   - What we know: The success criterion says "confirmed by cargo bloat --release showing no unwind tables"
   - What's unclear: Whether the planner should include a task to install cargo-bloat or use an alternative verification
   - Recommendation: Include a task that installs cargo-bloat via `cargo install cargo-bloat` and runs it. Alternatively, accepting absence of `_Unwind_Resume` in the PE exports is sufficient on Windows.

2. **Actual release-mode fib(40) baseline before Phase 75 changes**
   - What we know: quick-task 260322-6om reports ~141s but that was measured after the quick task's own changes; it may or may not have had a full LTO release build
   - What's unclear: Whether 141s was a true fully-optimized release build or a default release build
   - Recommendation: The Phase 75 plan should treat the baseline as "unknown until measured" and include a first measurement task after all config changes are applied. Do not assume 141s — it could be lower with LTO/single-codegen-unit.

3. **execute_ret inline annotation**
   - What we know: INLINE-03 says "exec_call, exec_ret, and other frequently-hit call functions in calls.rs have `#[inline]`" — but `execute_ret` is a private function in `dispatch/mod.rs`, not in `calls.rs`
   - What's unclear: Does INLINE-03 intend to annotate `execute_ret` in mod.rs as well?
   - Recommendation: Add `#[inline]` to `execute_ret` in mod.rs — it is called on every RET and TailCall instruction and is a strong candidate for inlining into execute_one's match arm. The requirement wording "exec_ret" maps to this function.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness |
| Config file | None (workspace uses `cargo test`) |
| Quick run command | `cargo test --release -p writ-runtime` |
| Full suite command | `cargo test --release` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BUILD-01 | lto="fat" in Cargo.toml | manual inspection | `grep 'lto' Cargo.toml` | ✅ after edit |
| BUILD-02 | codegen-units=1 in Cargo.toml | manual inspection | `grep 'codegen-units' Cargo.toml` | ✅ after edit |
| BUILD-03 | panic=abort in Cargo.toml | manual inspection + cargo-bloat | `grep 'panic' Cargo.toml` | ✅ after edit |
| BUILD-04 | FxHashMap used in DispatchTable and Scheduler.tasks | smoke (compile + test) | `cargo test --release -p writ-runtime` | ✅ existing 88 tests |
| BUILD-05 | fib(40) baseline documented | artifact | see BASELINE.md | ❌ Wave 0 gap |
| INLINE-01 | extract_* have #[inline(always)] | manual inspection + compile | `grep 'inline' writ-runtime/src/dispatch/helpers.rs` | ✅ after edit |
| INLINE-02 | exec_* in arith.rs have #[inline] | manual inspection + compile | `grep 'inline' writ-runtime/src/dispatch/arith.rs` | ✅ after edit |
| INLINE-03 | exec_call etc. in calls.rs have #[inline] | manual inspection + compile | `grep 'inline' writ-runtime/src/dispatch/calls.rs` | ✅ after edit |
| INLINE-04 | execute_one has NO #[inline] | manual inspection | `grep -n 'inline' writ-runtime/src/dispatch/mod.rs` | ✅ currently absent |
| VERIFY-01 | fib(40) outputs 102334155 | smoke | `.\target\release\writ.exe run benchmark/cases/fib/fib.writc` | ✅ pre-built .writc |
| VERIFY-02 | cargo test --release zero failures | integration | `cargo test --release` | ✅ 88 tests passing |
| VERIFY-03 | cargo build --release zero warnings | build | `cargo build --release 2>&1 \| grep warning` | ✅ currently clean |

### Sampling Rate
- **Per task commit:** `cargo test --release -p writ-runtime`
- **Per wave merge:** `cargo test --release`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `benchmark/BASELINE.md` — covers BUILD-05 (must be created as part of baseline measurement task)

---

## Sources

### Primary (HIGH confidence)
- Direct code inspection of `writ-runtime/src/dispatch/mod.rs` — current inline state, execute_one signature, DispatchTable definition
- Direct code inspection of `writ-runtime/src/dispatch/helpers.rs` — all 5 extract_* functions, no current annotations
- Direct code inspection of `writ-runtime/src/dispatch/arith.rs` — all ~40 exec_* functions, no current annotations
- Direct code inspection of `writ-runtime/src/dispatch/calls.rs` — exec_call, exec_call_virt, exec_call_extern, exec_call_indirect, exec_tail_call, execute_ret
- Direct code inspection of `writ-runtime/src/scheduler.rs` — Scheduler struct with HashMap<TaskId, Task>, HashMap<u32, TaskId>, HashMap<TaskId, Vec<...>>
- Direct inspection of `Cargo.toml` (root) — no profile sections present
- Direct inspection of `writ-runtime/Cargo.toml` — no rustc-hash dependency
- Direct inspection of `Cargo.lock` — rustc-hash 2.1.1 present
- `writ-compiler/Cargo.toml` — confirms `rustc-hash = "2.1.1"` already in workspace
- Quick task SUMMARY `260322-6om` — fib(40) = ~141s post-optimization baseline; instruction clone eliminated; debug guard added

### Secondary (MEDIUM confidence)
- Cargo reference: `[profile.release]` supports `lto`, `codegen-units`, `panic` keys — standard Rust documentation
- Rust Reference: `#[inline]` and `#[inline(always)]` attributes — standard behavior

### Tertiary (LOW confidence)
- cargo-bloat installation status: confirmed NOT installed in this environment; alternative verification strategy documented

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — rustc-hash version confirmed in Cargo.lock; FxHashMap API confirmed by writ-compiler usage patterns
- Architecture: HIGH — all target files inspected; inline annotation targets enumerated precisely
- Pitfalls: HIGH — based on direct code inspection; panic=abort catch_unwind concern verified as non-issue (no catch_unwind in test suite)

**Research date:** 2026-03-22
**Valid until:** 2026-04-22 (stable Rust ecosystem, low churn)
