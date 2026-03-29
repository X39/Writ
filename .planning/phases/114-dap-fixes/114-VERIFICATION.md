---
phase: 114-dap-fixes
verified: 2026-03-29T00:00:00Z
status: passed
score: 4/4 must-haves verified
gaps: []
human_verification:
  - test: "Step into a function defined in a different file in VS Code"
    expected: "The editor opens the correct source file for that function, not the entry file"
    why_human: "Multi-file DAP session requires a running VS Code + writ-dap process to observe"
---

# Phase 114: DAP Fixes Verification Report

**Phase Goal:** The debug adapter attributes source locations correctly per frame and handles dialogue string interpolation
**Verified:** 2026-03-29
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A stack frame's source file is determined by the frame's method_idx FileId, not source_paths.first() | VERIFIED | `inspection.rs:526-532` uses `self.method_file_ids.get(method_idx)` with `.or_else(|| self.source_paths.first())` as fallback only |
| 2 | Stepping into a function defined in a different file shows that file's path | VERIFIED (automated) | `method_file_ids` built per-file in `compile_and_load_project` via name->FileId lookup; wired through `handlers.rs:224`; used in `build_stack_frames`. Human verification noted below for live session |
| 3 | A dialogue text line containing `{name}` interpolation compiles and produces correct lowered IL | VERIFIED | `dlg_interp.writ` fixture exists with `{name}` and `{count}`; blessed `.writil` snapshot shows `STR_BUILD`, `I2S`, and `MOV` instructions; `cargo test -p writ-golden -- dlg_interp` passes |
| 4 | Existing DAP integration tests pass after both fixes | VERIFIED | `cargo test -p writ-dap` passes all tests (102 across all test binaries, 0 failures) |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `writ-dap/src/server/mod.rs` | `method_file_ids` field on `DapServer` | VERIFIED | Line 50: `pub(super) method_file_ids: Vec<Option<writ_diagnostics::FileId>>`, initialized `Vec::new()` at line 66 |
| `writ-dap/src/launch.rs` | `method_file_ids` construction from `per_file_asts` | VERIFIED | `compile_and_load` returns `vec![Some(FileId(0)); n]` (lines 42-45); `compile_and_load_project` builds via name lookup (lines 93-106); `collect_decl_names` helper collects Fn/Entity/Impl/Namespace decls (lines 235-277) |
| `writ-dap/src/server/inspection.rs` | Per-frame source path lookup using `method_file_ids` | VERIFIED | Lines 526-532: `self.method_file_ids.get(method_idx).and_then(...)` with fallback; `source_paths.first()` only appears once as `.or_else()` fallback |
| `writ-golden/tests/golden/dlg_interp.writ` | Golden fixture with dialogue text interpolation | VERIFIED | File exists; contains `@npc Hello {name}!`, `@npc Visit number {count} for you.`, and `@npc Welcome back, {name}, on visit {count}.`; registered in `golden_tests.rs:801-803`; blessed snapshot `dlg_interp.writil` present |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `writ-dap/src/launch.rs` | `writ-dap/src/server/handlers.rs` | `compile_and_load` / `compile_and_load_project` return `method_file_ids` | WIRED | `handlers.rs:132` destructures `(module, source_paths, method_file_ids)`; both compile function call sites destructure the 3-tuple correctly |
| `writ-dap/src/server/handlers.rs` | `writ-dap/src/server/mod.rs` | stores `method_file_ids` on `DapServer` | WIRED | `handlers.rs:224`: `self.method_file_ids = method_file_ids;` |
| `writ-dap/src/server/mod.rs` | `writ-dap/src/server/inspection.rs` | `build_stack_frames` reads `method_file_ids` | WIRED | `inspection.rs:526-529`: `self.method_file_ids.get(method_idx).and_then(|opt| *opt).and_then(|fid| self.source_paths.iter().find(|(id, _)| *id == fid))` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `inspection.rs` `build_stack_frames` | `frame_source` | `self.method_file_ids` populated from `run_pipeline` name->FileId walk of `per_file_asts` | Yes — real `FileId` per declaration collected from AST, not hardcoded | FLOWING |
| `dlg_interp.writ` IL snapshot | `STR_BUILD` instruction operands | `{name}` (string, `MOV` no-op path) and `{count}` (int, `I2S` conversion) in `greet` method body | Yes — snapshot shows 3 `STR_BUILD` calls with correct segment counts (3, 3, 5) | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All DAP tests pass | `cargo test -p writ-dap` | 0 failures across all test binaries | PASS |
| `dlg_interp` golden test passes | `cargo test -p writ-golden -- dlg_interp` | `test test_dlg_interp ... ok` | PASS |
| Compiler tests show no regressions | `cargo test -p writ-compiler` | 95 passed, 0 failed | PASS |
| `source_paths.first()` is fallback only | `grep source_paths.first inspection.rs` | Appears once as `.or_else()` fallback; primary path uses `method_file_ids.get(method_idx)` | PASS |
| Commits exist in git history | `git show --stat ecdaf99 e629afc` | Both commits present with expected file changes | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DAP-01 | 114-01-PLAN.md | Per-frame source file attribution uses correct FileId, not `source_paths.first()` fallback | SATISFIED | `inspection.rs` uses `method_file_ids[method_idx]` lookup; `source_paths.first()` is fallback only; wired through the full launch pipeline |
| DAP-02 | 114-01-PLAN.md | String interpolation in dialogue text lines works through `lower_fmt_string` pipeline | SATISFIED | `dlg_interp.writ` fixture exercises both `string` and `int` interpolation; snapshot blessed; `cargo test -p writ-golden -- dlg_interp` passes |

No orphaned requirements: both DAP-01 and DAP-02 are claimed by the plan and both are satisfied.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `handlers.rs:231` | 231 | `source_paths.first()` used for breakpoint-update events (not for stack frames) | Info | This is the breakpoint-resolved event path, not the stack-trace path. The per-frame fix in `build_stack_frames` is the bug site; this usage is for a separate event type and is acceptable as-is. |

No blockers or warnings. The single `source_paths.first()` in `handlers.rs` is in the breakpoint-change event path (separate from stack frame source attribution) and is not the bug site fixed by DAP-01.

### Human Verification Required

#### 1. Multi-file per-frame source attribution in live VS Code session

**Test:** Launch a two-file writ project in VS Code with the writ-dap adapter. Set breakpoints in both files. Step into a function defined in a file other than the entry file.
**Expected:** The editor navigates to the correct source file for each frame in the call stack. Frames from file A show file A's path; frames from file B show file B's path.
**Why human:** The `method_file_ids` lookup is verified to be wired correctly in code, but confirming the VS Code UI actually opens the right file requires a running DAP session with a real multi-file project.

### Gaps Summary

No gaps. All four observable truths are verified. All three key links are wired. Both required artifacts are substantive and flowing. Requirements DAP-01 and DAP-02 are fully satisfied. One human verification item is noted for live multi-file DAP session behavior, but all automated evidence supports correctness.

---

_Verified: 2026-03-29_
_Verifier: Claude (gsd-verifier)_
