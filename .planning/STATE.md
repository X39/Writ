---
gsd_state_version: 1.0
milestone: v3.1
milestone_name: Compiler Bug Fixes
status: unknown
last_updated: "2026-03-04T21:51:21.043Z"
progress:
  total_phases: 15
  completed_phases: 14
  total_plans: 41
  completed_plans: 40
---

---
gsd_state_version: 1.0
milestone: v3.1
milestone_name: Compiler Bug Fixes
status: active
last_updated: "2026-03-04T21:23:00Z"
progress:
  total_phases: 13
  completed_phases: 13
  total_plans: 38
  completed_plans: 38
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-03)

**Core value:** Correct, spec-compliant implementation at every layer — lowering matches Section 28 exactly, runtime matches the IL spec exactly — structured so each layer can be extended independently.
**Current focus:** v3.1 Compiler Bug Fixes — Phase 38 COMPLETE (1 plan done)

## Current Position

Phase: 38 COMPLETE (Fix CALL Method Token Resolution in Runtime)
Plan: 1 of 1 in phase 38 — ALL PLANS COMPLETE
Status: Phase 38 complete — BUG-17 fixed; runtime CALL/TailCall/NewDelegate/SpawnTask/SpawnDetached handlers now decode MethodDef metadata tokens; all 78 vm_tests pass.
Last activity: 2026-03-04 — Plan 38-01 complete: decode_method_token() added to dispatch.rs, all five method-dispatch handlers updated, vm_tests updated with proper tokens + new regression test

```
Progress: [====================] 100% (1/1 plans complete in phase 38)
```

## Performance Metrics

**All Milestones:**

| Milestone | Phases | Plans | Commits | LOC (src) | Duration |
|-----------|--------|-------|---------|-----------|----------|
| v1.0 | 7 | 13 | 67 | 3,493 | 2 days |
| v1.1 | 6 | 12 | 27 | 8,826 | 1 day |
| v1.2 | 2 | 3 | 13 | 8,826 | ~30 min |
| v2.0 | 6 | 16 | 60 | 13,937 | 2 days |
| v3.0 | 8 | 24 | 41 | 57,146 | 2 days |
| Phase 30-critical-bug-fixes P01 | 10 | 2 tasks | 2 files |
| Phase 30 P02 | 25 | 3 tasks | 10 files |
| Phase 31-test-harness-and-functions-golden-test P01 | 8 | 1 tasks | 4 files |
| Phase 31.1-fix-function-il-emission-bugs P01 | ~10 min | 2 tasks | 4 files |
| Phase 31.1-fix-function-il-emission-bugs P02 | ~3 min | 2 tasks | 9 files |
| Phase 31.1-fix-function-il-emission-bugs P03 | 3 | 2 tasks | 0 files |
| Phase 31.2-fix-register-convention-debug-info-and-parser-bugs P01 | 10min | 2 tasks | 4 files |
| Phase 37 P01 | 8min | 1 tasks | 5 files |
| Phase 37 P02 | 1min | 1 tasks | 0 files |
| Phase 38 P01 | 15min | 2 tasks | 2 files |

## Accumulated Context

### Decisions

Decisions logged in PROJECT.md Key Decisions table.

Key constraint for v3.1: Bug fixes (Phase 30) must land before any golden file test phases — you cannot validate IL output against the spec if the compiler crashes or emits structurally wrong IL.
- [Phase 30-critical-bug-fixes]: 16MB stack size for compile thread (standard rustc/swc pattern)
- [Phase 30-critical-bug-fixes]: Snapshot def_token_map clone to avoid split-borrow when mutating blob_heap in serialize.rs
- [Phase 30-critical-bug-fixes]: Error/Infer-typed registers use blob offset 0 silently to prevent debug_assert panic in encode_type
- [Phase 30]: emit_if uses shared r_result register for BUG-04: both branches MOV into it, ensuring RET always references initialized register
- [Phase 30]: pack_args_consecutive() centralized in call.rs for BUG-06: consecutive check avoids phantom MOV for all 7 argument packing sites
- [Phase 30]: ExternDef check added in emit_expr _ arm for BUG-05: checks token table before defaulting to Direct
- [Phase 31-01]: Round-trip isolation: compile_and_disassemble goes through Module::from_bytes, not in-memory compiler state
- [Phase 31-01]: bless_golden() exposed as pub(crate) for testability with temp dirs without env var manipulation
- [Phase 31-01]: 16MB stack thread in compile_and_disassemble matches writ-cli cmd_compile pattern for deep AST recursion
- [Phase 31.1-01]: BUG-07: guard TyKind::Func delegate path with callee_def_id.is_none() so named function calls always use direct/extern/virtual dispatch
- [Phase 31.1-01]: BUG-10: TypedStmt::Expr tail detection in Block emitter — typechecker sets tail=None; emitter unwraps last Expr stmt to propagate block value register
- [Phase 31.1-01]: BUG-09 is downstream of BUG-10 — no mod.rs change needed once Block propagation is fixed
- [Phase 31.1-02]: BUG-08: fix both assembler encoder and disassembler together — they must be inverses of each other and of the compiler's collect.rs encode_fn_sig
- [Phase 31.1-02]: BUG-11: 3-pass encode_instructions — pass 1 maps instr-index to byte-start (dry encode), pass 2 encodes bytes, pass 3 translates label fixups to byte positions and applies them
- [Phase 31.1-fix-function-il-emission-bugs]: BLESS=1 confirmed Plans 01 and 02 already updated .expected files correctly — no re-blessing changes needed
- [Phase 31.1-fix-function-il-emission-bugs]: Auto-approved human-verify checkpoint (auto_advance=true): all five IL emission bugs confirmed fixed by diff evidence and passing tests
- [Phase 31.2-fix-register-convention-debug-info-and-parser-bugs]: BUG-12: fn_param_map populated in collect pass, consumed in body emission to pre-allocate r0..r(n-1) for parameters before body emission
- [Phase 31.2-fix-register-convention-debug-info-and-parser-bugs]: ast_type_to_ty_simple uses fixed primitive Ty indices without mutable interner; non-primitives fall back to Ty(5)=Error (safe: register type only affects debug info)
- [Phase 31.2-fix-register-convention-debug-info-and-parser-bugs]: params.clone() in body emitter avoids split borrow between builder.get_fn_params (&Vec) and emitter.alloc_reg (&mut emitter)
- [Phase 31.2-fix-register-convention-debug-info-and-parser-bugs]: BUG-15 root cause was UTF-8 BOM in fn_empty_main.writ (not a parser logic bug); parser already handles bare fn via attrs.repeated().collect() + visibility.or_not()
- [Phase 31.2-fix-register-convention-debug-info-and-parser-bugs]: fn_empty_main blessed IL: .method main () -> void, r0 void (block tail expression register), RET_VOID
- [Phase 31.2-fix-register-convention-debug-info-and-parser-bugs]: BUG-13: build_debug_locals passes &mut builder.string_heap as separate field borrow (Rust allows while blob_heap also borrowed mutably in same loop)
- [Phase 31.2-fix-register-convention-debug-info-and-parser-bugs]: BUG-14: compute_instr_byte_starts standalone helper avoids exposing pass-1 data from encode_instructions; source_spans vec empty in practice (body emitter doesn't thread spans through emit_expr yet)
- [Phase 31.2-fix-register-convention-debug-info-and-parser-bugs]: u32::MAX sentinel for end_pc in debug_locals tuples; clamped to total_code_size at serialize time (avoids scope-exit detection during body emission)
- [Phase 31.2-fix-register-convention-debug-info-and-parser-bugs]: Auto-approved human-verify checkpoint (auto_advance=true): 7 golden tests pass, IL content verified spec-correct — fn_recursion 10-reg, r0=param n; fn_typed_params r0/r1 pre-allocated; fn_empty_main bare fn, RET_VOID
- [Phase 37]: BUG-16: return 0 (not alloc_void_reg) for void blocks — caller emits RetVoid without using the register, so no .reg declaration is needed
- [Phase 37]: Fix applies to both the empty-block arm and the Let/While/etc-tail arm in emit_expr Block match
- [Phase 37]: Auto-approved human-verify checkpoint (auto_advance=true): fn_empty_main has zero register declarations (only RET_VOID), fn_basic_call greet has zero register declarations, main retains .reg r0 void as legitimate CALL destination — BUG-16 locked
- [Phase 38]: BUG-17: decode_method_token strips table_id byte (bits 31-24) and converts 1-based row_index to 0-based by subtracting 1; returns None for null token (row_index=0)
- [Phase 38]: CallIndirect not updated — delegate heap object stores already-decoded 0-based index (NewDelegate decodes at creation time)
- [Phase 38]: CallExtern not updated — ExternDef tokens use table_id=16 (0x10), handled separately from MethodDef tokens

### Roadmap Evolution

- Phase 37 added: Fix spurious void register in empty function bodies
- Phase 38 added: Fix CALL method token resolution in runtime
- Phase 39 added: Extend method_defs metadata to include parameter names, register indices, and type information

### Pending Todos

None.

### Blockers/Concerns

None.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 1 | Recreate README and add GitHub community files | 2026-03-02 | 248e0ff | [1-recreate-readme-and-add-github-community](./quick/1-recreate-readme-and-add-github-community/) |

## Session Continuity

Last session: 2026-03-04
Stopped at: Completed 38-01-PLAN.md (Fix CALL method token resolution — BUG-17 fixed)
Resume file: None
Next action: Phase 38 fully complete — 1 plan done; continue with Phase 39 (extend method_defs metadata)
