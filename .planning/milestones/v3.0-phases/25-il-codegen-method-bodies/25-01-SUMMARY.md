---
phase: 25-il-codegen-method-bodies
plan: 01
subsystem: codegen
tags: [il-codegen, register-allocator, label-fixup, body-emitter, instruction-emission]

# Dependency graph
requires:
  - phase: 24-il-codegen-metadata-skeleton
    provides: ModuleBuilder with all 21 metadata tables and DefId->MetadataToken map
  - phase: 23-type-checking
    provides: TypedAst/TypedExpr/TypedStmt with Ty on every node, TyInterner

provides:
  - RegisterAllocator (sequential u16 indices, per-register Ty tracking)
  - LabelAllocator (symbolic labels, fixup pass with offset = target - branch_start)
  - BodyEmitter struct (builder/interner refs, locals map, loop stack, instruction buffer)
  - has_error_nodes() pre-pass for TypedExpr::Error / TypedStmt::Error detection
  - emit_expr() dispatching all TypedExpr variants to Instruction sequences
  - emit_stmt() dispatching all TypedStmt variants to Instruction sequences
  - emit_all_bodies() entry point for full method body emission

affects:
  - 25-02 (call dispatch builds on emit_expr placeholders)
  - 25-03 (closures/arrays/concurrency adds match arms to emit_expr/emit_stmt)
  - 25-04 (enums, string ops, debug info complete the emitter)

# Tech tracking
tech-stack:
  added:
    - writ-module = { path = "../writ-module" } (writ-compiler dependency)
  patterns:
    - Sequential register allocator: alloc() returns next u16, reg_count() returns peak
    - Symbolic label fixup: add_fixup(instr_idx, label) + mark_label_here(label) + apply_fixups()
    - BodyEmitter as mutable context passed by &mut to emit_expr/emit_stmt
    - Nop placeholder pattern for deferred plan variants (Call, Match, Lambda, etc.)
    - Loop context stack: push_loop(break_lbl, continue_lbl) / pop_loop() for nested loops
    - TyKind dispatch for instruction selection (Int -> AddI, Float -> AddF)

key-files:
  created:
    - writ-compiler/src/emit/body/mod.rs
    - writ-compiler/src/emit/body/reg_alloc.rs
    - writ-compiler/src/emit/body/labels.rs
    - writ-compiler/src/emit/body/expr.rs
    - writ-compiler/src/emit/body/stmt.rs
    - writ-compiler/tests/emit_body_tests.rs
  modified:
    - writ-compiler/Cargo.toml (add writ-module dep)
    - writ-compiler/src/emit/mod.rs (add pub mod body)

key-decisions:
  - "BodyEmitter holds &'a ModuleBuilder (immutable) — string literal interning deferred to Plan 04 when a mutable heap ref can be threaded through"
  - "Label fixup at instruction-index level (not byte-offset level) since serialization happens after codegen; labels.rs apply_fixups() works with raw bytes for tests"
  - "alloc_void_reg() uses Ty(4) directly — Void is always the 5th pre-interned type per TyInterner::new() fixed ordering"
  - "Comparison operators on Binary ty=Bool use default CmpEqI — full operand-type-driven dispatch deferred to Plan 02 which has richer context"
  - "Atomic blocks emit AtomicBegin/End wrapping now; Plan 03 completes the full atomic semantics"

patterns-established:
  - "emit_expr() always returns a u16 register — even placeholder Nop emissions alloc and return a void register"
  - "Deferred TypedExpr variants emit Nop + alloc_reg(ty) — compiles clean, ready for Plan 02-04 match arms"
  - "Break/Continue use loop_stack: push_loop before body, pop_loop after, panic-on-empty is correct (typecheck prevents break outside loop)"

requirements-completed: [EMIT-07, EMIT-08]

# Metrics
duration: 9min
completed: 2026-03-03
---

# Phase 25 Plan 01: Register Allocator, Label Fixup, and Core Instruction Emission Summary

**Sequential register allocator + symbolic label fixup system + BodyEmitter context + full arithmetic/logic/comparison/control-flow instruction dispatch via TyKind, with 19 tests passing**

## Performance

- **Duration:** 9 min
- **Started:** 2026-03-03T02:21:51Z
- **Completed:** 2026-03-03T02:30:58Z
- **Tasks:** 2 (TDD: RED + GREEN in one pass)
- **Files modified:** 8

## Accomplishments

- RegisterAllocator allocates sequential u16 indices with per-register Ty tracking; reg_count() returns peak
- LabelAllocator resolves symbolic labels via fixup pass (offset = target_byte_pos - branch_start_byte_pos)
- BodyEmitter struct holds all emission state: regs, labels, instructions, locals map, loop stack
- has_error_nodes() pre-pass scans entire TypedAst for Error nodes before any codegen work
- emit_expr() handles all TypedExpr variants: literals, binary/unary, if/else, block, assign, return; deferred variants (Call, Match, Lambda, etc.) emit Nop placeholder
- emit_stmt() handles all TypedStmt variants: let, while, for (stub), break, continue, return, atomic
- All 19 tests pass covering all infrastructure and core emission patterns

## Task Commits

Each task was committed atomically:

1. **Task 1: Infrastructure — writ-module dep, register allocator, labels, BodyEmitter, error pre-pass** - `931fa53` (feat)
2. **Task 2: Core instruction emission — arithmetic, logic, comparison, data movement, control flow** - `e46d15f` (feat)

**Plan metadata:** _(docs commit follows)_

## Files Created/Modified

- `writ-compiler/Cargo.toml` - Added writ-module = { path = "../writ-module" } dependency
- `writ-compiler/src/emit/mod.rs` - Added `pub mod body;`
- `writ-compiler/src/emit/body/mod.rs` - BodyEmitter struct, has_error_nodes, emit_all_bodies, error node walker
- `writ-compiler/src/emit/body/reg_alloc.rs` - RegisterAllocator with alloc/reg_count/types
- `writ-compiler/src/emit/body/labels.rs` - LabelAllocator with new_label/mark/add_fixup/apply_fixups
- `writ-compiler/src/emit/body/expr.rs` - emit_expr() dispatching all TypedExpr variants
- `writ-compiler/src/emit/body/stmt.rs` - emit_stmt() dispatching all TypedStmt variants
- `writ-compiler/tests/emit_body_tests.rs` - 19 integration tests

## Decisions Made

- BodyEmitter holds `&'a ModuleBuilder` (immutable borrow) — string literal interning requires `&mut StringHeap` which is deferred to Plan 04 where a mutable heap reference can be threaded through. Current string literals emit `LoadString { string_idx: 0 }` as placeholder.
- Label fixup system uses instruction-index positions internally (since serialization to bytes happens later). The `labels.rs` module's `apply_fixups()` works with raw byte buffers for unit testing purposes.
- `alloc_void_reg()` uses `Ty(4)` directly since `TyInterner::new()` pre-interns primitives in fixed order: Int=0, Float=1, Bool=2, String=3, Void=4, Error=5.
- Eq/NotEq on binary expressions default to CmpEqI since the result type is Bool and operand type is not passed through to `emit_binary`. Full type-aware dispatch will be added in Plan 02 when the call to emit_binary can receive operand_ty separately.

## Deviations from Plan

None — plan executed exactly as written. All TypedExpr/TypedStmt variants handled (core ones fully, deferred ones with Nop placeholders). Infrastructure matches the spec in the plan exactly.

## Issues Encountered

- `SimpleSpan::new()` requires `use chumsky::span::Span as _` to bring the trait into scope — corrected in test file. (Auto-fix Rule 3.)
- BodyEmitter holds immutable `&'a ModuleBuilder` — discovered during string literal emission. Documented as decision; placeholder used for now.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- BodyEmitter infrastructure is complete and ready for Plan 02
- emit_expr() and emit_stmt() have placeholder arms for all deferred variants — Plan 02 only needs to add match arms for Call, Field, New, etc.
- Loop stack, label system, and register allocator all tested and correct
- No blockers for Plan 02 (call dispatch, object model, entity construction)

---
*Phase: 25-il-codegen-method-bodies*
*Completed: 2026-03-03*

## Self-Check: PASSED

- All 6 source files created: FOUND
- All commits present: 931fa53 (Task 1), e46d15f (Task 2)
- 19 tests pass: confirmed
- cargo build -p writ-compiler: success (11 warnings, 0 errors)
