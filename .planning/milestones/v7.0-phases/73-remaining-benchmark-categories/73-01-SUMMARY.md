---
phase: 73-remaining-benchmark-categories
plan: 01
subsystem: benchmark
tags: [benchmark, string-concat, generate-py, null-guard, writ-compiler, writ-runtime]

requires:
  - phase: 72-chart-generation
    provides: generate.py chart/results pipeline that processes raw.json

provides:
  - string_concat benchmark suite (6 languages, 100000 iterations, output 500000)
  - Null-safe generate.py handling for suites without .writ files (hash_map support)

affects: [73-02, benchmark runner, generate.py, Docker benchmark pipeline]

tech-stack:
  added: []
  patterns:
    - "String concat benchmark: 100000 iterations of += 'hello', print final length"
    - "Writ fallback when builtin returns wrong value: arithmetic constant instead of s.len()"
    - "generate.py null guard: writ_compile_ms/writ_run_ms return None, callers use if/continue"

key-files:
  created:
    - benchmark/cases/string_concat/string_concat.writ
    - benchmark/cases/string_concat/string_concat.lua
    - benchmark/cases/string_concat/string_concat.nut
    - benchmark/cases/string_concat/string_concat.py
    - benchmark/cases/string_concat/string_concat.js
    - benchmark/cases/string_concat/string_concat.rs
  modified:
    - benchmark/generate.py

key-decisions:
  - "Used 100000*5 constant instead of s.len() in Writ — StrLen runtime bug returns wrong value (heap slot number instead of string byte length)"
  - "Null guard pattern in generate.py: helper functions return None for missing writ entries, callers use if guards to skip"

requirements-completed: [BENCH-03]

duration: 36min
completed: 2026-03-20
---

# Phase 73 Plan 01: String Concat Benchmark + generate.py Null Guard Summary

**String concatenation benchmark in 6 languages (all output 500000) and null-safe generate.py Writ entry handling for hash_map suite**

## Performance

- **Duration:** 36 min
- **Started:** 2026-03-20T18:36:09Z
- **Completed:** 2026-03-20T19:12:09Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Created string_concat benchmark suite with all 6 language implementations (Writ, Lua, Squirrel, Python, Node.js, Rust)
- Writ implementation compiles and outputs 500000 (uses 100000*5 constant due to StrLen runtime bug)
- Patched generate.py: writ_total_ms/writ_compile_ms/writ_run_ms return None for null entries; build_chart and generate_results_md skip Writ for suites with no .writ file

## Task Commits

Each task was committed atomically:

1. **Task 1: Create string_concat benchmark files for all 6 languages** - `e30ab65` (feat)
2. **Task 2: Patch generate.py for null Writ entry handling** - `74bfc0f` (feat)

## Files Created/Modified
- `benchmark/cases/string_concat/string_concat.writ` - Writ: while loop + string concat, outputs 500000
- `benchmark/cases/string_concat/string_concat.lua` - Lua: for loop with .. operator
- `benchmark/cases/string_concat/string_concat.nut` - Squirrel: for loop with += operator
- `benchmark/cases/string_concat/string_concat.py` - Python: for range + += operator
- `benchmark/cases/string_concat/string_concat.js` - Node.js: for loop with += operator
- `benchmark/cases/string_concat/string_concat.rs` - Rust: push_str loop, println! len
- `benchmark/generate.py` - Null-safe writ_compile_ms/writ_run_ms/writ_total_ms + chart/table guards

## Decisions Made
- Used `let len: int = 100000 * 5;` in Writ instead of `s.len()` — StrLen instruction has a runtime bug where it returns the heap slot number (e.g., 7) instead of the string byte length. The loop+concatenation logic is correct and exercises the string concat path; the length computation via arithmetic constant produces the correct 500000 output.
- Null guard pattern: helper functions return None (not raise), callers use `if wc is None or wr is None: continue` pattern for charts and `if wc is not None and wr is not None:` for table rows. Writ simply omits from output for that suite, no error.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed build failure: S2i/S2f/S2b instructions missing from Instruction enum**
- **Found during:** Task 1 (compiling string_concat.writ)
- **Issue:** `writ-assembler`, `writ-runtime`, and `writ-compiler` all referenced `Instruction::S2i`, `Instruction::S2f`, `Instruction::S2b` variants that did not exist in `writ-module/src/instruction.rs`. Also `exec_s2i/exec_s2f/exec_s2b` were duplicated in arith.rs, and `StringIntoInt/Float/Bool` intrinsic IDs were unhandled in intrinsics.rs.
- **Fix:** Already fixed in prior commit `da68b0b` (73-02 array_sort agent ran first). No additional fix needed — changes were already present when this agent ran.
- **Files modified:** writ-module/src/instruction.rs, writ-runtime/src/dispatch/arith.rs, writ-runtime/src/dispatch/intrinsics.rs
- **Committed in:** da68b0b (73-02 array_sort commit)

**2. [Rule 3 - Blocking] s.len() produces wrong value at runtime**
- **Found during:** Task 1 (running string_concat.writ)
- **Issue:** `StrLen` instruction returns the heap slot number (HeapRef.0) instead of string byte length. For "hello" at heap slot 7, returns 7 not 5. Root cause not investigated (out of scope).
- **Fix:** Applied plan's documented fallback: `let len: int = 100000 * 5; log::info($"{len}");` — arithmetic constant instead of s.len(). The string concatenation loop still runs (exercises the correct code path), only the measurement uses a constant.
- **Files modified:** benchmark/cases/string_concat/string_concat.writ
- **Committed in:** e30ab65 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both were blocking issues. Fix 1 was already resolved by a prior agent. Fix 2 uses the plan's own documented fallback — no scope creep.

## Issues Encountered
- The `cargo run -p writ-cli` compilation was blocked repeatedly by a background fib(40) process holding writ.exe lock (Windows file locking). Required manual process kill via wmic.
- The StrLen runtime bug needs future investigation — heap slot index returned instead of string length. Deferred to separate quick task.

## Next Phase Readiness
- string_concat suite is complete and ready for Docker benchmark execution
- generate.py null guard ready for hash_map suite (BENCH-05) which has no .writ file
- Phase 73-02 (array_sort + hash_map) was already executed by a prior agent session

## Self-Check: PASSED

- benchmark/cases/string_concat/string_concat.writ: FOUND
- benchmark/cases/string_concat/string_concat.lua: FOUND
- benchmark/cases/string_concat/string_concat.nut: FOUND
- benchmark/cases/string_concat/string_concat.py: FOUND
- benchmark/cases/string_concat/string_concat.js: FOUND
- benchmark/cases/string_concat/string_concat.rs: FOUND
- benchmark/generate.py: FOUND
- .planning/phases/73-remaining-benchmark-categories/73-01-SUMMARY.md: FOUND
- Commit e30ab65: FOUND
- Commit 74bfc0f: FOUND

---
*Phase: 73-remaining-benchmark-categories*
*Completed: 2026-03-20*
