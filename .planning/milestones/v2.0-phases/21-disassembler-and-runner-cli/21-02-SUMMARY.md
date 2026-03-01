---
phase: 21-disassembler-and-runner-cli
plan: 02
subsystem: tooling
tags: [cli, runner, assembler, disassembler, clap, RuntimeHost, end-to-end]

requires:
  - phase: 20-text-assembler
    provides: writ-assembler crate with assemble() function
  - phase: 21-01
    provides: disassemble() and disassemble_verbose() in writ-assembler

provides:
  - writ binary with run/assemble/disasm subcommands
  - CliHost implementing RuntimeHost with annotated [say]/[choice]/[entity:*]/[extern] output
  - End-to-end pipeline: assemble text IL -> serialize -> load -> execute -> host output
  - Integration tests validating assembler + disassembler + runtime together

affects: [future-game-host, writ-cli-users, developer-tooling]

tech-stack:
  added:
    - clap 4.5 (derive macros for CLI argument parsing)
  patterns:
    - RuntimeHost implementation in separate crate (CliHost in writ-cli, NullHost in writ-runtime untouched)
    - MetadataToken decode pattern: (extern_idx & 0x00FF_FFFF) as usize - 1 for 0-based index
    - ExternDef name resolution at CliHost construction time (not per-request)
    - Integration tests use programmatic ModuleBuilder (not assembler text) for export-bearing modules

key-files:
  created:
    - writ-cli/src/cli_host.rs
    - writ-cli/tests/cli_integration.rs
  modified:
    - writ-cli/Cargo.toml
    - writ-cli/src/main.rs

key-decisions:
  - "CliHost resolves extern names once at construction time by iterating module.extern_defs — no per-request heap lookups"
  - "Integration tests use programmatic ModuleBuilder for export-bearing modules since .export is not a parsed directive"
  - "HeapRef inner field is pub(crate) — CliHost prints <string> placeholder for Ref values (documented known limitation)"
  - "GcStats fields are objects_freed/objects_traced/heap_before/heap_after (plan doc had wrong field names collected/alive_after — auto-fixed)"
  - "Entry method arg passing (Array<String> of CLI args) deferred to future phase as documented in plan"

patterns-established:
  - "CliHost construction: pass &Module, extract extern names to Vec<String> by walking extern_defs + read_string()"
  - "cmd_run entry discovery: find ExportDefRow where name matches AND item_kind==0, decode token to 0-based method_idx"
  - "Tick loop pattern: loop { match runtime.tick(0.0, ExecutionLimit::None) { AllCompleted | Empty => break, ... } }"

requirements-completed: [TOOL-02, TOOL-03, TOOL-04]

duration: 6min
completed: 2026-03-02
---

# Phase 21 Plan 02: Runner CLI Summary

**writ binary with run/assemble/disasm subcommands, CliHost with [say]/[choice]/[entity:*]/[extern] annotated output, and end-to-end pipeline test validating assemble->serialize->load->run->host-output**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-02T17:51:44Z
- **Completed:** 2026-03-02T17:57:09Z
- **Tasks:** 1 (TDD: integration tests pass, then CliHost + main.rs implemented)
- **Files modified:** 4

## Accomplishments

- `writ-cli/Cargo.toml` updated with `[[bin]] name = "writ"`, clap 4.5 derive, writ-module/writ-runtime/writ-assembler dependencies
- `writ-cli/src/cli_host.rs` (175 lines) — CliHost implementing RuntimeHost:
  - Builds extern name table at construction from module.extern_defs + read_string()
  - `say()` prints `[say] {value}` — Int/Float/Bool print directly, Ref prints `<string>` (documented limitation)
  - `choice()` auto-selects 0 non-interactively with `[choice] auto-selecting 0` prefix
  - Unknown externs print `[extern] {name}()` prefix
  - Entity spawn/destroy print `[entity:spawn]`/`[entity:destroy]` with type/entity info
  - `on_log` prints `[{level:?}] {message}` to stderr
  - `on_gc_complete` prints GC stats when verbose
  - 6 unit tests embedded in `#[cfg(test)] mod tests`
- `writ-cli/src/main.rs` (196 lines) — clap CLI with three subcommands:
  - `assemble`: reads .writil (or stdin with `-`), calls `writ_assembler::assemble()`, writes .writc
  - `disasm`: reads .writc, calls `disassemble()` or `disassemble_verbose()`, prints to stdout
  - `run`: finds exported "main" (or --entry override), builds RuntimeBuilder with CliHost, ticks to completion
- `writ-cli/tests/cli_integration.rs` — 4 integration tests:
  - `test_assemble_and_disassemble`: assembles .writil text, disassembles, verifies directives in output
  - `test_run_simple_module`: programmatic Module with export, serialize+deserialize, spawn+tick
  - `test_entry_point_not_found`: verifies error message when no "main" export exists
  - `test_end_to_end_dialogue_say`: full pipeline — ModuleBuilder with extern+method+export, serialize, load, runtime with TestSayHost, assert "42" captured from say() call

## Task Commits

1. **Task 1: Create CliHost, wire CLI with clap, implement all three subcommands** - `84a2eba` (feat)

## Files Created/Modified

- `writ-cli/Cargo.toml` — Added `[[bin]]`, clap 4.5, workspace deps
- `writ-cli/src/cli_host.rs` — CliHost (175 lines, 6 unit tests)
- `writ-cli/src/main.rs` — clap CLI with assemble/disasm/run subcommands (196 lines)
- `writ-cli/tests/cli_integration.rs` — 4 integration tests including end-to-end dialogue test

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] GcStats field names in on_gc_complete**

- **Found during:** Task 1 (compilation)
- **Issue:** Plan documented `stats.collected` and `stats.alive_after` but actual GcStats has `objects_freed`, `objects_traced`, `heap_before`, `heap_after`
- **Fix:** Used correct field names in the verbose GC output
- **Files modified:** `writ-cli/src/cli_host.rs`
- **Commit:** `84a2eba`

**2. [Rule 1 - Bug] Integration tests used invalid assembler syntax**

- **Found during:** Task 1 (RED phase compilation)
- **Issue:** Initial test code used `.fn "name" () -> void { ... }` (not a valid parser directive) and `\n.module "name" "ver"\n` (module must use block syntax `{ ... }`)
- **Fix:** Rewrote tests to use correct `.module "name" "ver" { .method ... }` block syntax and `ModuleBuilder` for modules requiring export defs (since `.export` is not a parser directive)
- **Files modified:** `writ-cli/tests/cli_integration.rs`
- **Commit:** `84a2eba`

## Test Results

- 6 CliHost unit tests: all pass
- 4 integration tests: all pass
- 0 regressions in writ-assembler (65 tests)
- 0 failures across entire workspace

## Next Phase Readiness

- `writ` binary is fully functional as the developer-facing entry point to the IL toolchain
- Phase 21 complete: disassembler (Plan 01) + runner CLI (Plan 02) done
- Ready for v2.0 milestone completion and PROJECT.md update

---
*Phase: 21-disassembler-and-runner-cli*
*Completed: 2026-03-02*
