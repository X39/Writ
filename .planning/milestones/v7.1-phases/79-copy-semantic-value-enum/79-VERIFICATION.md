---
phase: 79-copy-semantic-value-enum
verified: 2026-03-22T15:16:39Z
status: gaps_found
score: 9/10 must-haves verified
gaps:
  - truth: "fib(40) completes in under 30 seconds in release mode"
    status: failed
    reason: "Median of three measured runs was 44.873s — 14.873s above the 30s target. VERIFY-04 is marked Pending in REQUIREMENTS.md."
    artifacts:
      - path: "benchmark/BASELINE.md"
        issue: "Documents 44.873s median for Phase 79; target not met"
    missing:
      - "Additional optimization phases beyond Phase 79 scope are required to close the 14.873s gap (Phase 80+)"
---

# Phase 79: Copy-Semantic Value Enum — Verification Report

**Phase Goal:** Value derives Copy by storing InlineStruct fields on the GC heap, making register-to-register moves bitwise copies throughout all dispatch handlers
**Verified:** 2026-03-22T15:16:39Z
**Status:** gaps_found — phase goal achieved; one performance requirement (VERIFY-04) not met
**Re-verification:** No — initial verification

---

## Goal Achievement

The structural goal of the phase is fully achieved: `Value` derives `Copy`, `InlineStruct` is gone, and all dispatch handlers use `Value::Struct { type_idx, href }` backed by a GC heap allocation. The single gap is the quantitative fib(40) performance target, which is correctly open in REQUIREMENTS.md (VERIFY-04 marked `[ ]`).

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Value enum derives Copy — compiler accepts `#[derive(Copy)]` on Value | VERIFIED | `writ-runtime/src/value.rs` line 59: `#[derive(Debug, Clone, Copy, Default)]` on `enum Value` |
| 2 | Value::InlineStruct variant does not exist — no code references it | VERIFIED | `grep -r "InlineStruct" --include="*.rs" .` returns zero matches workspace-wide |
| 3 | Value::Struct { type_idx, href } variant exists and stores struct fields on GC heap | VERIFIED | `value.rs` line 68: `Struct { type_idx: u32, href: HeapRef }` |
| 4 | collect_value_refs pushes href for Value::Struct, enabling GC to trace struct registers | VERIFIED | `gc.rs` lines 61-62: `Value::Struct { href, .. } => refs.push(*href)` |
| 5 | GC regression test proves struct-in-register survives garbage collection | VERIFIED | `vm_tests.rs` lines 2279-2307: `test_gc_traces_struct_href_in_register` passes, asserts `objects_freed == 0` with struct root and `== 2` without |
| 6 | exec_new for Struct kind allocates on heap instead of inline | VERIFIED | `dispatch/objects.rs` lines 21-23: `ctx.heap.alloc_struct(field_count)` then `Value::Struct { type_idx, href }` |
| 7 | exec_get_field and exec_set_field route Struct variant through heap | VERIFIED | `dispatch/objects.rs` lines 53-63 (GetField) and 95-101 (SetField): both match `Value::Struct { href, .. }` and call `ctx.heap.get_field`/`ctx.heap.set_field` |
| 8 | cargo test --release passes with zero failures and zero warnings | VERIFIED | SUMMARY 79-01 and 79-02 both report zero failures, zero warnings; commits `ffaa866`, `01ce560`, `4e53552` |
| 9 | fib(40) produces correct output 102334155 | VERIFIED | SUMMARY 79-02 documents run output "102334155"; `benchmark/BASELINE.md` records correct result |
| 10 | fib(40) completes in under 30 seconds in release mode | FAILED | Median of three measured runs: 44.569s, 44.873s, 45.151s — median 44.873s. Target is 30s. Miss of 14.873s. |

**Score:** 9/10 truths verified

---

## Required Artifacts

### Plan 79-01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-runtime/src/value.rs` | Copy-derivable Value enum with `Struct { type_idx, href }` | VERIFIED | Line 59: `#[derive(Debug, Clone, Copy, Default)]`; line 68: `Struct { type_idx: u32, href: HeapRef }`; no InlineStruct |
| `writ-runtime/src/gc.rs` | Updated collect_value_refs tracing `Struct(HeapRef)` | VERIFIED | Lines 59-65: `Value::Struct { href, .. } => refs.push(*href)` replaces old recursive InlineStruct traversal |
| `writ-runtime/src/dispatch/objects.rs` | Heap-allocating exec_new for Struct, heap-routing get/set_field | VERIFIED | Lines 19-24: `ctx.heap.alloc_struct`, `Value::Struct { type_idx, href }`; lines 53-63 and 95-101: heap-routed get/set |
| `writ-runtime/tests/vm_tests.rs` | GC regression test for struct-in-register | VERIFIED | Lines 2279-2307: `test_gc_traces_struct_href_in_register` exists and is substantive |

### Plan 79-02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-runtime/tests/vm_tests.rs` | Fully migrated test suite with no InlineStruct references | VERIFIED | Zero InlineStruct references remain; stale comment updated in commit `4e53552` |

---

## Key Link Verification

### Plan 79-01 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-runtime/src/value.rs` | `writ-runtime/src/gc.rs` | `Value::Struct { href, .. }` arm in `collect_value_refs` | WIRED | `gc.rs` line 62: `Value::Struct { href, .. } => refs.push(*href)` — struct refs flow directly into GC root set |
| `writ-runtime/src/dispatch/objects.rs` | `writ-runtime/src/gc.rs` | `exec_new` allocates on heap via `ctx.heap.alloc_struct` | WIRED | `objects.rs` line 21: `let href = ctx.heap.alloc_struct(field_count)` — struct data lives on heap, GC can trace it |

### Plan 79-02 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-runtime/src/value.rs` | `benchmark/cases/fib/fib.writ` | `cargo run -p writ-cli --release` | WIRED (correctness only) | Output "102334155" confirmed; performance link fails (44.873s vs 30s target) |

---

## Requirements Coverage

All ten requirement IDs declared across both plans (VALUE-01 through VALUE-06, VERIFY-01 through VERIFY-04) are accounted for.

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| VALUE-01 | 79-01 | InlineStruct fields stored on GC heap via HeapRef | SATISFIED | `exec_new` allocates via `alloc_struct`; field access goes through heap |
| VALUE-02 | 79-01 | Value enum derives Copy | SATISFIED | `#[derive(Debug, Clone, Copy, Default)]` on `enum Value` in value.rs line 59 |
| VALUE-03 | 79-01 | GC collect_value_refs traces Value::Struct(HeapRef) as heap root | SATISFIED | `gc.rs` line 62: `Value::Struct { href, .. } => refs.push(*href)` |
| VALUE-04 | 79-01 | GC regression test confirms struct-in-register survives GC | SATISFIED | `test_gc_traces_struct_href_in_register` in vm_tests.rs lines 2279-2307 |
| VALUE-05 | 79-01 | Field access reads/writes through HeapRef | SATISFIED | `exec_get_field` and `exec_set_field` both route `Value::Struct` through `ctx.heap` |
| VALUE-06 | 79-02 | All existing tests pass after Value migration | SATISFIED | Zero test failures reported; commits `ffaa866`, `01ce560`, `4e53552` |
| VERIFY-01 | 79-02 | fib(40) produces correct output 102334155 | SATISFIED | Documented in SUMMARY 79-02 and benchmark/BASELINE.md |
| VERIFY-02 | 79-02 | cargo test --release passes with zero failures | SATISFIED | Documented in both summaries; REQUIREMENTS.md marks as Complete |
| VERIFY-03 | 79-02 | cargo build --release produces no warnings | SATISFIED | REQUIREMENTS.md marks as Complete; summaries report zero warnings |
| VERIFY-04 | 79-02 | fib(40) completes in under 30 seconds | NOT MET | Measured 44.873s median — 14.873s above target. REQUIREMENTS.md leaves this `[ ]` (Pending). |

### Orphaned Requirements

None. All ten IDs mapped to this phase in REQUIREMENTS.md are covered by the two plans. No additional Phase 79 IDs appear in the traceability table that were not claimed by a plan.

---

## Anti-Patterns Found

No blockers or stubs detected.

| File | Pattern | Severity | Notes |
|------|---------|----------|-------|
| `dispatch/objects.rs` line 91 | `.clone()` on Value (now Copy) | Info | `frame.registers[r_val as usize].clone()` in `exec_set_field` is a redundant clone since Value is Copy. Compiler elides the distinction; functionally correct. Not a stub — data flows correctly to `set_field`. |

The redundant `.clone()` calls on `Value` throughout `dispatch/objects.rs` (e.g., array operations) are style noise after the Copy migration, not correctness issues. The plan acknowledged these as "can be left as-is" and they do not affect the goal.

---

## Human Verification Required

None required for automated checks. The performance miss (VERIFY-04) is a quantitative measurement already documented with three timed runs — no further human testing needed for this verification.

---

## Gaps Summary

One gap blocks full phase completion: **VERIFY-04 (fib(40) under 30 seconds)** was not met. The median of three measured release-mode runs was 44.873s — a 15.5% improvement over Phase 78 (53.134s) and a 46.1% cumulative improvement over the Phase 75 baseline (83.297s), but 14.873s above the v7.1 milestone target.

This gap is structural: all five planned optimization phases (75-79) are complete and the remaining 14.873s gap requires techniques beyond the current v7.1 scope (JIT, NaN-boxing, superinstructions, etc.), as listed in REQUIREMENTS.md under Future Requirements.

The phase goal — `Value derives Copy, InlineStruct removed, register moves are bitwise copies` — is fully and verifiably achieved. The gap is a milestone target miss that requires follow-up planning, not a defect in what Phase 79 set out to do.

---

_Verified: 2026-03-22T15:16:39Z_
_Verifier: Claude (gsd-verifier)_
