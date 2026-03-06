---
phase: 73
slug: remaining-benchmark-categories
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-20
---

# Phase 73 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (writ-compiler, writ-runtime), golden test runner (writ-golden), output-checksum smoke tests |
| **Config file** | `Cargo.toml` workspace |
| **Quick run command** | `cargo test -p writ-golden 2>&1 \| tail -5` |
| **Full suite command** | `cargo test --workspace 2>&1 \| tail -20` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run local smoke test (compile+run .writ file, verify output)
- **After every plan wave:** Run `python3 benchmark/generate.py` against test raw.json
- **Before `/gsd:verify-work`:** Full Docker run producing updated raw.json with all 7 benchmarks
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 73-01-01 | 01 | 1 | BENCH-03 | smoke | `writ compile benchmark/cases/string_concat/string_concat.writ -o /tmp/sc.writc && writ run /tmp/sc.writc` | ❌ W0 | ⬜ pending |
| 73-01-02 | 01 | 1 | BENCH-03 | smoke | `lua5.4 benchmark/cases/string_concat/string_concat.lua` (+ nut/py/js/rs) | ❌ W0 | ⬜ pending |
| 73-02-01 | 02 | 1 | BENCH-04 | smoke | `writ compile benchmark/cases/array_sort/array_sort.writ -o /tmp/as.writc && writ run /tmp/as.writc` | ❌ W0 | ⬜ pending |
| 73-02-02 | 02 | 1 | BENCH-04 | smoke | `lua5.4 benchmark/cases/array_sort/array_sort.lua` (+ nut/py/js/rs) | ❌ W0 | ⬜ pending |
| 73-03-01 | 03 | 1 | BENCH-05 | smoke | `lua5.4 benchmark/cases/hash_map/hash_map.lua` (+ nut/py/js/rs) | ❌ W0 | ⬜ pending |
| 73-03-01 | 03 | 1 | BENCH-06 | smoke | `writ compile benchmark/cases/oop_dispatch/oop_dispatch.writ -o /tmp/od.writc && writ run /tmp/od.writc` | ❌ W0 | ⬜ pending |
| 73-03-02 | 03 | 1 | BENCH-06 | smoke | `lua5.4 benchmark/cases/oop_dispatch/oop_dispatch.lua` (+ nut/py/js/rs) | ❌ W0 | ⬜ pending |
| 73-03-03 | 03 | 1 | BENCH-07 | smoke | `writ compile benchmark/cases/object_create/object_create.writ -o /tmp/oc.writc && writ run /tmp/oc.writc` | ❌ W0 | ⬜ pending |
| 73-03-04 | 03 | 1 | BENCH-07 | smoke | `lua5.4 benchmark/cases/object_create/object_create.lua` (+ nut/py/js/rs) | ❌ W0 | ⬜ pending |
| 73-01-03 | 01 | 1 | BENCH-03-07 | integration | `python3 benchmark/generate.py benchmark/results/YYYY-MM-DD/raw.json` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `benchmark/cases/string_concat/string_concat.{writ,lua,nut,py,js,rs}` — BENCH-03 smoke tests
- [ ] `benchmark/cases/array_sort/array_sort.{writ,lua,nut,py,js,rs}` — BENCH-04 smoke tests
- [ ] `benchmark/cases/hash_map/hash_map.{lua,nut,py,js,rs}` — BENCH-05 smoke tests (no .writ)
- [ ] `benchmark/cases/oop_dispatch/oop_dispatch.{writ,lua,nut,py,js,rs}` — BENCH-06 smoke tests
- [ ] `benchmark/cases/object_create/object_create.{writ,lua,nut,py,js,rs}` — BENCH-07 smoke tests
- [ ] Writ contract dispatch smoke-test — prerequisite for BENCH-06
- [ ] generate.py null writ guard patch — prerequisite for BENCH-03-07

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Full Docker pipeline produces raw.json with 7 benchmarks | BENCH-03-07 | Requires Docker environment | Run `run.sh` or `run.ps1`, verify raw.json contains all 7 suites |
| Charts and RESULTS.md include all 7 benchmarks | BENCH-03-07 | Visual verification | Check SVG files and RESULTS.md in dated output directory |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
