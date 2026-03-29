# Phase 81: Profile-Guided Optimization Build - Research

**Researched:** 2026-03-22
**Domain:** Rust LLVM PGO — instrumented build, profraw collection, profdata merge, optimized rebuild
**Confidence:** HIGH

## Summary

Profile-guided optimization (PGO) is a three-pass compiler technique: compile with instrumentation, run the training workload to collect `.profraw` files, merge them to `.profdata`, then recompile using that data. LLVM uses the profile to improve branch prediction, code layout, inlining decisions, and register allocation based on observed hot paths rather than static heuristics.

For this phase the training workload is `fib.writ` / `fib.writc` (fib(40)), which is also the measurement workload. The binary that matters is `writ` (writ-cli), which links writ-runtime — PGO instrumentation must cover the entire workspace so the dispatch loop in writ-runtime is profiled. The Phase 80 baseline is 43.765s. The VERIFY-04 target is under 30s, a gap of 13.765s (31%). PGO for match-heavy dispatch loops in Rust typically yields 5-15% improvement; bridging the full gap from this phase alone is uncertain.

All required tooling is already present: `llvm-profdata.exe` ships with the `llvm-tools` rustup component (already installed at nightly), and the nightly toolchain is the default. No build.rs files exist in any crate, so RUSTFLAGS propagation to build scripts is a non-issue. The `--target x86_64-pc-windows-msvc` flag should still be passed to Cargo to be safe — it prevents host build scripts from being compiled with PGO flags and prevents profile data from being emitted to unexpected locations.

**Primary recommendation:** Use RUSTFLAGS with `-Cprofile-generate` / `-Cprofile-use` directly, without cargo-pgo (not installed, adds install time, no clear advantage over manual RUSTFLAGS on a single-binary workspace). Three-step pipeline: instrument build → training run → optimized rebuild. Document delta and compare against 30s target regardless of outcome.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
All implementation choices are at Claude's discretion — pure infrastructure phase.

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase.

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| VERIFY-04 | fib(40) completes in under 30 seconds (release build) | PGO is the primary remaining optimization lever; outcome must be documented regardless |
| VERIFY-01 | fib(40) produces correct output 102334155 | PGO does not change semantics; fib output verified at each measurement |
| VERIFY-02 | cargo test --release passes with zero failures | PGO binary uses same code paths; existing suite is sufficient |
| VERIFY-03 | cargo build --release produces no warnings after the phase | No source changes; warning count unchanged from Phase 80 |
</phase_requirements>

---

## Standard Stack

### Core

| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| rustc `-Cprofile-generate` | stable (nightly 1.93.0) | Instrument binary to emit `.profraw` | Built into rustc, no external dependency |
| rustc `-Cprofile-use` | stable (nightly 1.93.0) | Recompile using merged profile data | Built into rustc, same flag family |
| `llvm-profdata merge` | LLVM 21.1.5 (in rustup llvm-tools) | Merge `.profraw` files to `.profdata` | Required by rustc; ships with `llvm-tools` component |

### llvm-profdata location (confirmed on this machine)

```
C:\Users\msili\.rustup\toolchains\nightly-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\bin\llvm-profdata.exe
```

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Manual RUSTFLAGS | cargo-pgo | cargo-pgo automates directory management but is not installed; the manual path is 3 commands and well-understood |
| llvm-profdata from rustup | llvm-profdata from vcpkg (also present) | Both work; rustup version matches the exact LLVM version used by rustc — prefer it |

**Installation:** None required. `llvm-tools` component is already installed.

---

## Architecture Patterns

### PGO Pipeline Overview

```
Step 1: Instrument build
  RUSTFLAGS="-Cprofile-generate=<abs_path>/pgo-data" cargo build --release
      --target x86_64-pc-windows-msvc
  → Emits instrumented writ.exe to target/x86_64-pc-windows-msvc/release/writ.exe

Step 2: Training run (fib(40) as workload)
  ./target/x86_64-pc-windows-msvc/release/writ.exe run benchmark/cases/fib/fib.writc
  → Emits default_<hash>.profraw into pgo-data/

Step 3: Merge profiles
  llvm-profdata.exe merge -o pgo-data/merged.profdata pgo-data/

Step 4: Optimized rebuild
  RUSTFLAGS="-Cprofile-use=<abs_path>/pgo-data/merged.profdata
             -Cllvm-args=-pgo-warn-missing-function"
    cargo build --release --target x86_64-pc-windows-msvc
  → Emits PGO-optimized writ.exe

Step 5: Measure
  Median of 3 cold runs of fib(40) against the optimized binary
```

### Absolute Path Requirement

RUSTFLAGS `-Cprofile-generate` and `-Cprofile-use` paths **must be absolute**. Cargo invokes rustc from varying working directories per crate during a workspace build; a relative path would resolve differently for each crate, producing scattered `.profraw` files or failing to find the `.profdata` file.

Use `$PWD` (bash) or `%CD%` (cmd) when constructing the path, or hardcode the repo root.

### Why `--target x86_64-pc-windows-msvc` Matters

When `RUSTFLAGS` is set globally, it applies to all rustc invocations including any build scripts (`build.rs`). The `--target` flag makes Cargo use a separate compilation path for the host (build scripts) vs. target (the actual binary), so PGO flags only apply to the target binary. No build.rs files exist in this workspace, but the flag is cheap to pass and is the documented best practice.

### Pattern 1: Instrument Build (Windows bash)

```bash
# From repo root (D:/dev/git/Writ)
PGODIR="$(pwd)/pgo-data"
mkdir -p "$PGODIR"

RUSTFLAGS="-Cprofile-generate=$PGODIR" \
  cargo build --release --target x86_64-pc-windows-msvc
```

### Pattern 2: Training Run

The training binary lands at `target/x86_64-pc-windows-msvc/release/writ.exe`.
Run fib.writc (pre-compiled bytecode — already committed) to avoid compiler overhead:

```bash
./target/x86_64-pc-windows-msvc/release/writ.exe run benchmark/cases/fib/fib.writc
```

A single training run is sufficient for this workload. The fib dispatch loop executes ~300M recursive calls, producing a statistically dense profile. Running multiple times adds marginal benefit but increases Step 2 cost substantially.

### Pattern 3: Merge and Optimize

```bash
LLVM_PROFDATA="$HOME/.rustup/toolchains/nightly-x86_64-pc-windows-msvc/lib/rustlib/x86_64-pc-windows-msvc/bin/llvm-profdata.exe"
"$LLVM_PROFDATA" merge -o "$PGODIR/merged.profdata" "$PGODIR"

RUSTFLAGS="-Cprofile-use=$PGODIR/merged.profdata -Cllvm-args=-pgo-warn-missing-function" \
  cargo build --release --target x86_64-pc-windows-msvc
```

The `-pgo-warn-missing-function` flag surfaces any functions in the optimized build that had no coverage in the training run — useful for diagnosing incomplete profiles.

### Pattern 4: Measuring the PGO Binary

The optimized binary path changes to `target/x86_64-pc-windows-msvc/release/writ.exe` (same location, just rebuilt). Use the same timing method as prior phases:

```bash
# Three cold runs, record median
time ./target/x86_64-pc-windows-msvc/release/writ.exe run benchmark/cases/fib/fib.writc
```

### Recommended Project Structure for PGO Artifacts

```
pgo-data/                      # gitignored — large binary profdata
├── default_<hash>.profraw     # emitted during training run
└── merged.profdata            # after llvm-profdata merge
```

Add `pgo-data/` to `.gitignore`. The merged `.profdata` file should not be committed (can be 50-100 MB for a full workspace training run).

### Anti-Patterns to Avoid

- **Relative paths in RUSTFLAGS**: Every crate in the workspace resolves differently. Always use absolute paths.
- **Training on fib.writ source**: Compiling `.writ` → `.writc` during the training run exercises the compiler, not just the VM. Use the pre-compiled `fib.writc` to keep the profile focused on the dispatch hot path.
- **LTO + PGO interaction (instrumentation build)**: The instrumentation build should use the same Cargo profile (`[profile.release]`) as the final build. The existing `lto = "fat"` and `codegen-units = 1` settings remain in `Cargo.toml` for both passes. Do not temporarily remove them — the profile must be collected from the same code shape that will be optimized.
- **Stale `.profraw` without cleanup**: If pgo-data/ contains old `.profraw` files from a prior attempt, `llvm-profdata merge` will include them. Always clean `pgo-data/` before Step 1 or pass only the new `.profraw` explicitly to merge.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Profile file discovery | Shell glob to find profraw files | `llvm-profdata merge pgo-data/` (directory argument) | llvm-profdata accepts a directory and finds all `.profraw` files automatically |
| PGO automation | Custom shell script wrapper | RUSTFLAGS env var (3 commands) | No abstraction needed at this scale |
| LLVM tool installation | Build LLVM from source | `llvm-tools` rustup component (already installed) | Exact version match with rustc's LLVM; already present |

---

## Common Pitfalls

### Pitfall 1: Relative Path in RUSTFLAGS
**What goes wrong:** `RUSTFLAGS="-Cprofile-generate=pgo-data"` — rustc is invoked with different CWDs per crate during a workspace build. Each crate writes profraw to a different `pgo-data/` subdirectory (e.g., inside each crate's source directory), so the merge step either finds no files or merges incomplete data.
**How to avoid:** Always expand to absolute path before setting RUSTFLAGS: `PGODIR="$(pwd)/pgo-data"`.
**Warning signs:** Only some `.profraw` files appear in the expected `pgo-data/` directory after the training run.

### Pitfall 2: PGO Binary Ends Up in Different Path
**What goes wrong:** Expecting the binary at `target/release/writ.exe` but it was built with `--target x86_64-pc-windows-msvc`, so it lands at `target/x86_64-pc-windows-msvc/release/writ.exe`.
**How to avoid:** When `--target` is specified, Cargo places the binary under `target/<triple>/release/`. Use that path explicitly in the training and measurement steps.
**Warning signs:** Running `./target/release/writ.exe` runs the old non-PGO binary.

### Pitfall 3: Instrumented Build Runs Slowly — Don't Benchmark It
**What goes wrong:** Timing the training run and assuming that's the PGO performance. The instrumented binary is 2-3x slower due to profiling overhead. Only the Step 4 optimized rebuild should be benchmarked.
**How to avoid:** Training run is for profile collection only; do not record its time in BASELINE.md.

### Pitfall 4: Old .profraw Files Polluting the Merge
**What goes wrong:** A prior failed or partial instrumentation run left `.profraw` files in `pgo-data/`. llvm-profdata merges all of them, diluting the profile with stale data.
**How to avoid:** `rm -rf pgo-data/` before Step 1.

### Pitfall 5: Missing Function Warnings Without `-pgo-warn-missing-function`
**What goes wrong:** Some code paths in the optimized binary had zero coverage during training. LLVM falls back to static heuristics for those functions — not an error, but may indicate the training was incomplete.
**How to avoid:** Add `-Cllvm-args=-pgo-warn-missing-function` in Step 4. Review warnings; for fib(40) training, expect warnings for error/crash paths that fib never executes.

### Pitfall 6: PGO Does Not Close the Full Gap
**What goes wrong:** Expecting PGO to reduce 43.765s to under 30s. Research indicates 5-15% gains for match-heavy dispatch; 15% of 43.765s is ~6.5s, yielding ~37s — still above 30s.
**How to avoid:** Document the result regardless. VERIFY-04 success requires the result to be measured and documented; success is not required by Phase 81 planning — Phase 82 handles further optimization if VERIFY-04 is still open.
**Warning signs:** If the result is still above 30s, this is expected; document and proceed.

### Pitfall 7: cargo test --release With --target Flag
**What goes wrong:** After building with `--target x86_64-pc-windows-msvc`, running `cargo test --release` without the same `--target` flag compiles a separate non-PGO test binary. The RUSTFLAGS from the instrumentation pass should be cleared before running tests.
**How to avoid:** Run `cargo test --release` (no target flag, no RUSTFLAGS) separately from the PGO build steps. The test suite uses the standard release build, not the PGO binary.

---

## Code Examples

### Complete PGO Pipeline (bash on Windows)

```bash
# From D:/dev/git/Writ repo root

# Step 0: Clean any prior profiling data
PGODIR="$(pwd)/pgo-data"
rm -rf "$PGODIR"
mkdir -p "$PGODIR"

# Step 1: Instrumented build
RUSTFLAGS="-Cprofile-generate=$PGODIR" \
  cargo build --release --target x86_64-pc-windows-msvc

# Step 2: Training run (use pre-compiled bytecode, NOT .writ source)
./target/x86_64-pc-windows-msvc/release/writ.exe run benchmark/cases/fib/fib.writc

# Step 3: Merge profiles
LLVM_PROFDATA="$HOME/.rustup/toolchains/nightly-x86_64-pc-windows-msvc/lib/rustlib/x86_64-pc-windows-msvc/bin/llvm-profdata.exe"
"$LLVM_PROFDATA" merge -o "$PGODIR/merged.profdata" "$PGODIR"

# Step 4: Optimized rebuild
RUSTFLAGS="-Cprofile-use=$PGODIR/merged.profdata -Cllvm-args=-pgo-warn-missing-function" \
  cargo build --release --target x86_64-pc-windows-msvc

# Step 5: Measure (3 cold runs, record median)
time ./target/x86_64-pc-windows-msvc/release/writ.exe run benchmark/cases/fib/fib.writc
time ./target/x86_64-pc-windows-msvc/release/writ.exe run benchmark/cases/fib/fib.writc
time ./target/x86_64-pc-windows-msvc/release/writ.exe run benchmark/cases/fib/fib.writc

# Step 6: Verify correctness
./target/x86_64-pc-windows-msvc/release/writ.exe run benchmark/cases/fib/fib.writc
# Expected output: 102334155

# Step 7: Test suite (no --target, no RUSTFLAGS — standard release build)
cargo test --release
```

### Verifying profraw Files Were Created

```bash
ls pgo-data/*.profraw
# Expected: at least one file like default_<hash>.profraw
```

### .gitignore Entry

```
# PGO profiling artifacts
pgo-data/
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual llvm-profdata invocation | cargo-pgo automates the pipeline | 2023 (cargo-pgo released) | Optional; manual RUSTFLAGS still fully supported |
| codegen-units > 1 for PGO gains | codegen-units = 1 + PGO together | 2019 research finding | With fat LTO the gain from more codegen units is already captured; keep CGU=1 |

**Note on PGO + LTO + CGU=1 interaction (MEDIUM confidence):**
The 2019 LLVM investigation found PGO gains are reduced with CGU=1 because the profile has fewer distinct compilation units to optimize independently. The Writ release profile already has `codegen-units = 1` + `lto = "fat"`. This means LTO has already absorbed much of the code layout and inlining benefit that PGO would normally provide. PGO will still help with branch prediction and machine code ordering, but gains may be on the lower end (5-8% rather than 10-15%).

---

## Validation Architecture

`workflow.nyquist_validation` is not set in `.planning/config.json` — treated as enabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`#[test]`) |
| Config file | `Cargo.toml` workspace (test integration under `tests/`) |
| Quick run command | `cargo test -p writ-runtime --release 2>&1 \| tail -5` |
| Full suite command | `cargo test --release 2>&1 \| tail -20` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VERIFY-01 | fib(40) output == 102334155 | smoke/manual | run PGO binary against fib.writc, check stdout | manual |
| VERIFY-02 | cargo test --release zero failures | integration | `cargo test --release` | existing suite |
| VERIFY-03 | cargo build --release zero warnings | build check | `cargo build --release 2>&1 \| grep "^warning"` | N/A |
| VERIFY-04 | fib(40) < 30 s release mode | benchmark | `time ./target/x86_64-pc-windows-msvc/release/writ.exe run benchmark/cases/fib/fib.writc` | manual |

### Sampling Rate

- **Per task commit:** `cargo test -p writ-runtime --release 2>&1 | tail -5`
- **Per wave merge:** `cargo test --release 2>&1 | tail -20`
- **Phase gate:** Full suite green + fib(40) median measured and documented before `/gsd:verify-work`

### Wave 0 Gaps

None — existing test infrastructure covers all phase requirements. This phase adds no new code paths; PGO is a build-system change only. The existing `vm_tests.rs`, `pool_tests.rs`, `gc_tests.rs`, and `task_tests.rs` suites provide correctness coverage.

---

## Open Questions

1. **Will PGO close the VERIFY-04 gap?**
   - What we know: Phase 80 baseline is 43.765s. Target is 30s — a 31% reduction needed. PGO for Rust with `codegen-units=1` + `lto=fat` typically yields 5-10% improvement on dispatch-heavy workloads. That projects to ~39-41s, not 30s.
   - What's unclear: Whether the fib(40) hot path (match-heavy recursion) has unusual PGO responsiveness, or whether the CGU=1 + LTO combination pre-absorbs most of PGO's potential benefit.
   - Recommendation: Execute the pipeline, record the result, document honestly. Phase 82 is the fallback if VERIFY-04 remains open.

2. **Should the benchmark crates (writ-cli dependencies) all be instrumented, or only writ-runtime?**
   - What we know: `RUSTFLAGS` with `-Cprofile-generate` applies to every crate in the workspace. The hot path is entirely in writ-runtime and writ-module; the parser and compiler are not exercised by `writ.exe run fib.writc`.
   - What's unclear: Whether profiling unused crates adds meaningful noise.
   - Recommendation: Instrument the full workspace (via RUSTFLAGS on `cargo build --release`). LLVM handles "no profile data for function X" gracefully (falls back to static heuristics). This is simpler and matches the documented approach.

3. **Should pgo-data/ be gitignored proactively before the task or as part of the task?**
   - Recommendation: Add the `.gitignore` entry in the first task commit so the profraw files are never staged accidentally.

---

## Sources

### Primary (HIGH confidence)

- [rustc PGO documentation](https://doc.rust-lang.org/beta/rustc/profile-guided-optimization.html) — complete 4-step workflow, RUSTFLAGS usage, absolute path requirement, `--target` recommendation
- Direct filesystem verification: `llvm-profdata.exe` present at `~/.rustup/toolchains/nightly-x86_64-pc-windows-msvc/lib/rustlib/x86_64-pc-windows-msvc/bin/llvm-profdata.exe`
- `rustup component list --installed` — `llvm-tools-x86_64-pc-windows-msvc` confirmed installed
- `rustc --version --verbose` — nightly 1.93.0, LLVM 21.1.5
- `Cargo.toml` — workspace members, no build.rs files found in any crate

### Secondary (MEDIUM confidence)

- [Kobzol's cargo-pgo blog post (2023)](https://kobzol.github.io/rust/cargo/2023/07/28/rust-cargo-pgo.html) — cargo-pgo workflow and llvm-tools-preview component name
- [LLVM dev list: PGO effectiveness in Rust (2019)](https://lists.llvm.org/pipermail/llvm-dev/2019-December/137331.html) — CGU=1 reduces PGO gain from ~4% to ~0.3%; primary cause is reduced compilation unit granularity
- [Rust Performance Book PGO section](https://nnethercote.github.io/perf-book/build-configuration.html) — "can improve runtime speed by 10% or more" (general claim, not CGU=1 specific)
- [PGO in Rust practical guide — OxidizeConf 2024](https://www.datocms-assets.com/98516/1734435430-zaitsau_2024.pdf) — PGO particularly helps match-heavy dispatch loops and interpreter workloads

### Tertiary (LOW confidence)

- General ecosystem consensus (multiple sources): 5-15% range is common for interpreter-style dispatch, but highly workload-dependent. This project's CGU=1+LTO combination may reduce the ceiling.

---

## Metadata

**Confidence breakdown:**
- PGO pipeline commands: HIGH — directly from official rustc docs + confirmed tooling present
- llvm-profdata path: HIGH — filesystem verified
- Windows/MSVC compatibility: HIGH — nightly MSVC toolchain confirmed; LLVM 21.1.5 present
- Expected speedup range: MEDIUM — 5-15% general; may be lower due to CGU=1+LTO already in place
- Gap closure (VERIFY-04): LOW — depends on actual measurement; research suggests partial but not full gap closure

**Research date:** 2026-03-22
**Valid until:** 2026-04-22 (no external dependencies; toolchain is pinned nightly)
