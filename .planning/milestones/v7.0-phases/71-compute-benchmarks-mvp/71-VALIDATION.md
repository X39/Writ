---
phase: 71
slug: compute-benchmarks-mvp
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-20
---

# Phase 71 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Shell scripts + jq validation (no Rust test framework needed — benchmarks are standalone programs) |
| **Config file** | none — validation is inline shell commands |
| **Quick run command** | `writ compile benchmark/cases/fib/fib.writ -o /tmp/fib.writc && writ run /tmp/fib.writc` |
| **Full suite command** | `RUNS=3 ./benchmark/runner/run.sh` |
| **Estimated runtime** | ~120 seconds (Docker build + 3 benchmark runs) |

---

## Sampling Rate

- **After every task commit:** Run `writ compile` + `writ run` on each `.writ` file to confirm compilation succeeds
- **After every plan wave:** Run `RUNS=3 ./benchmark/runner/run.sh` (full Docker pipeline)
- **Before `/gsd:verify-work`:** Full suite must produce valid raw.json with both fib and sieve entries
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 71-01-01 | 01 | 1 | BENCH-01, BENCH-08 | integration | `writ compile benchmark/cases/fib/fib.writ -o /tmp/fib.writc && writ run /tmp/fib.writc 2>&1 \| grep 102334155 && python3 benchmark/cases/fib/fib.py \| grep 102334155 && node benchmark/cases/fib/fib.js \| grep 102334155` | ❌ W0 | ⬜ pending |
| 71-02-01 | 02 | 1 | BENCH-02 | unit+golden | `cargo test -p writ-golden -- type_array_ops && cargo test -p writ-compiler` | ❌ W0 | ⬜ pending |
| 71-02-02 | 02 | 1 | BENCH-02, BENCH-08 | integration | `writ compile benchmark/cases/sieve/sieve.writ -o /tmp/sieve.writc && writ run /tmp/sieve.writc 2>&1 \| grep 78498 && python3 benchmark/cases/sieve/sieve.py \| grep 78498 && node benchmark/cases/sieve/sieve.js \| grep 78498` | ❌ W0 | ⬜ pending |
| 71-e2e | both | post | BENCH-01,02,08 | e2e | `RUNS=3 ./benchmark/runner/run.sh && jq '.benchmarks[] \| select(.suite=="fib")' benchmark/results/*/raw.json` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `benchmark/cases/fib/fib.writ` — Writ fibonacci source file
- [ ] `benchmark/cases/sieve/sieve.writ` — Writ prime sieve source file
- [ ] All 12 source files (6 languages × 2 benchmarks) present and runnable

*Existing Docker infrastructure (Phase 70) covers runtime and harness requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Output equivalence across languages | BENCH-08 | Requires running 6 different interpreters/compilers (Lua, Squirrel may not be locally installed) | Run each language's fib program, confirm all print `102334155`. Run each sieve, confirm all print `78498`. |
| Docker pipeline E2E | BENCH-01,02 | Requires Docker Desktop running | Run `RUNS=3 ./benchmark/runner/run.sh`, verify raw.json contains fib and sieve entries |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
