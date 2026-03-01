# Roadmap: Writ Compiler

## Milestones

- ✅ **v1.0 CST-to-AST Lowering Pipeline** — Phases 1-7 (shipped 2026-02-27)
- ✅ **v1.1 Spec v0.4 Conformance** — Phases 8-13 (shipped 2026-03-01)
- ✅ **v1.2 Gap Closure** — Phases 14-15 (shipped 2026-03-01)
- ✅ **v2.0 Writ Runtime** — Phases 16-21 (shipped 2026-03-02)
- ✅ **v3.0 Writ Compiler** — Phases 22-29 (shipped 2026-03-03)
- 🚧 **v3.1 Compiler Bug Fixes** — Phases 30-36 (in progress)

## Phases

<details>
<summary>✅ v1.0 CST-to-AST Lowering Pipeline (Phases 1-7) — SHIPPED 2026-02-27</summary>

- [x] Phase 1: AST Foundation (2/2 plans) — completed 2026-02-26
- [x] Phase 2: Foundational Expression Lowering (2/2 plans) — completed 2026-02-26
- [x] Phase 3: Operator and Concurrency Lowering (2/2 plans) — completed 2026-02-26
- [x] Phase 4: Dialogue Lowering and Localization (2/2 plans) — completed 2026-02-26
- [x] Phase 5: Entity Lowering (3/3 plans) — completed 2026-02-27
- [x] Phase 6: Pipeline Integration and Snapshot Testing (1/1 plan) — completed 2026-02-27
- [x] Phase 7: Dialogue Stack Fix and Dead Code Cleanup (1/1 plan) — completed 2026-02-27

Full details: `milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>✅ v1.1 Spec v0.4 Conformance (Phases 8-13) — SHIPPED 2026-03-01</summary>

- [x] Phase 8: Lexer Fixes (2/2 plans) — completed 2026-03-01
- [x] Phase 9: CST Type System Additions (2/2 plans) — completed 2026-03-01
- [x] Phase 10: Parser — Core Syntax (2/2 plans) — completed 2026-03-01
- [x] Phase 11: Parser — Declarations and Expressions (2/2 plans) — completed 2026-03-01
- [x] Phase 12: Lowering — Dialogue and Localization (2/2 plans) — completed 2026-03-01
- [x] Phase 13: Lowering — Entity Model and Misc (2/2 plans) — completed 2026-03-01

Full details: `milestones/v1.1-ROADMAP.md`

</details>

<details>
<summary>✅ v1.2 Gap Closure (Phases 14-15) — SHIPPED 2026-03-01</summary>

- [x] Phase 14: Fix Hex/Binary Literal Lowering (1/1 plan) — completed 2026-03-01
- [x] Phase 15: Tech Debt Cleanup (2/2 plans) — completed 2026-03-01

Full details: `milestones/v1.2-ROADMAP.md`

</details>

<details>
<summary>✅ v2.0 Writ Runtime (Phases 16-21) — SHIPPED 2026-03-02</summary>

- [x] Phase 16: Module Format Foundation (3/3 plans) — completed 2026-03-02
- [x] Phase 17: VM Core and Task Execution (3/3 plans) — completed 2026-03-02
- [x] Phase 18: Entity System and GC (3/3 plans) — completed 2026-03-02
- [x] Phase 19: Contract Dispatch and Virtual Module (3/3 plans) — completed 2026-03-02
- [x] Phase 20: Text Assembler (2/2 plans) — completed 2026-03-02
- [x] Phase 21: Disassembler and Runner CLI (2/2 plans) — completed 2026-03-02

Full details: `milestones/v2.0-ROADMAP.md`

</details>

<details>
<summary>✅ v3.0 Writ Compiler (Phases 22-29) — SHIPPED 2026-03-03</summary>

- [x] Phase 22: Name Resolution (3/3 plans) — completed 2026-03-02
- [x] Phase 23: Type Checking (3/3 plans) — completed 2026-03-03
- [x] Phase 24: IL Codegen — Metadata Skeleton (2/2 plans) — completed 2026-03-03
- [x] Phase 25: IL Codegen — Method Bodies (6/6 plans) — completed 2026-03-03
- [x] Phase 26: CLI Integration and E2E Validation (4/4 plans) — completed 2026-03-03
- [x] Phase 27: Retroactive Verification (3/3 plans) — completed 2026-03-03
- [x] Phase 28: Codegen Bug Fixes (2/2 plans) — completed 2026-03-03
- [x] Phase 29: LocaleDef Emission (1/1 plan) — completed 2026-03-03

Full details: `milestones/v3.0-ROADMAP.md`

</details>

### 🚧 v3.1 Compiler Bug Fixes (In Progress)

**Milestone Goal:** Fix all known IL generation bugs, establish a golden file test harness, and lock hand-validated IL output for every language feature as regression tests.

- [x] **Phase 30: Critical Bug Fixes** — Eliminate stack overflow, fix register types, method tokens, return registers, extern calls, and phantom MOVs (completed 2026-03-04)
- [ ] **Phase 31: Test Harness and Functions Golden Test** — Create the compile-disassemble-compare framework; validate basic function IL
- [x] **Phase 31.1: Fix Function IL Emission Bugs** — Fix CALL vs CALL_INDIRECT, method signature types, RET/MOV register selection, and branch offsets discovered by golden tests (completed 2026-03-04)
- [x] **Phase 31.2: Fix Register Convention, Debug Info, and Parser Bugs** — Fix parameter register pre-allocation (uninitialized registers), debug local names, source span emission, and bare `fn` parse error (completed 2026-03-04)
- [ ] **Phase 32: Core Golden Tests Batch 1** — Structs, enums, and control flow golden files hand-validated and locked
- [ ] **Phase 33: Core Golden Tests Batch 2** — Entities, dialogue, and contracts golden files hand-validated and locked
- [ ] **Phase 34: Advanced Golden Tests Batch 1** — Generics, error handling, and Option type golden files hand-validated and locked
- [ ] **Phase 35: Advanced Golden Tests Batch 2** — Closures, concurrency, and globals golden files hand-validated and locked
- [ ] **Phase 36: Remaining Golden Tests and Tech Debt** — Arrays/strings golden file plus closure captures, GC hooks, and dead code cleanup

## Phase Details

### Phase 30: Critical Bug Fixes
**Goal**: The compiler produces structurally valid IL for all programs — no crashes, correct register types, correct method tokens, correct return wiring, correct extern calls, no phantom moves
**Depends on**: Phase 29 (v3.0 complete)
**Requirements**: BUG-01, BUG-02, BUG-03, BUG-04, BUG-05, BUG-06
**Success Criteria** (what must be TRUE):
  1. `writ compile` on any valid .writ file completes without a stack overflow or panic
  2. Every register in emitted IL carries its actual type blob (not `int` for all types)
  3. Every CALL instruction in emitted IL references a non-zero method metadata token
  4. Every RET instruction references the register that holds the computed return value, not register 0 by default
  5. Calls to extern functions (`::log`, `::print`, etc.) emit CALL_EXTERN instructions with correct extern method tokens
  6. Argument setup for function calls emits no MOV from uninitialized registers (phantom moves eliminated)
**Plans**: 2 plans

Plans:
- [ ] 30-01-PLAN.md — Fix stack overflow (BUG-01) and register type blob encoding (BUG-02)
- [ ] 30-02-PLAN.md — Fix extern call dispatch (BUG-03, BUG-05), return register wiring (BUG-04), and phantom MOV elimination (BUG-06)

### Phase 31: Test Harness and Functions Golden Test
**Goal**: A golden file test infrastructure exists and the basic function IL is hand-validated and locked as the first regression anchor
**Depends on**: Phase 30
**Requirements**: GOLD-01, GOLD-02
**Success Criteria** (what must be TRUE):
  1. Running the test suite compiles .writ fixtures, disassembles the output, and diffs it against a .expected file — failing with a readable diff on mismatch
  2. A functions.writ fixture covering declarations, parameters, return types, locals, and calls between methods exists with a locked .expected IL file
  3. Any future change that alters function IL output causes an explicit test failure with a diff
  4. The harness can be extended by adding a new .writ file plus its corresponding .expected file
**Plans**: 2 plans

Plans:
- [ ] 31-01-PLAN.md — Create writ-golden workspace crate with compile-disassemble-compare harness (BLESS=1 workflow, unified diff on failure)
- [ ] 31-02-PLAN.md — Write fn_basic_call, fn_typed_params, fn_recursion fixture sources; bless and hand-validate .expected IL files

### Phase 31.1: Fix Function IL Emission Bugs
**Goal**: All 5 IL emission bugs discovered by the Phase 31 golden tests are fixed; the three function fixtures are re-blessed with spec-correct IL and all golden tests pass
**Depends on**: Phase 31
**Requirements**: BUG-07, BUG-08, BUG-09, BUG-10, BUG-11
**Success Criteria** (what must be TRUE):
  1. Direct function calls emit `CALL` (not `CALL_INDIRECT`) — `CALL_INDIRECT` only appears for delegate/closure calls
  2. Every method's disassembled signature shows correct parameter types and return type (no swapped types)
  3. Every `RET` in non-void functions references the register holding the computed return value (typed int/bool/etc, not void)
  4. Every `MOV` in if/else branch merging references the register holding the actual branch value (not a void register)
  5. Every `BR`/`BR_FALSE`/`BR_TRUE` emits a non-zero offset that correctly reaches the branch target
  6. All three function golden tests pass (`cargo test -p writ-golden`) with the re-blessed .expected files
**Plans**: 3 plans

Plans:
- [x] 31.1-01-PLAN.md — Fix CALL_INDIRECT for direct calls (BUG-07), void return register (BUG-09), void MOV source in if/else (BUG-10)
- [x] 31.1-02-PLAN.md — Fix method signature type ordering in disassembler (BUG-08), branch offsets always 0 (BUG-11)
- [x] 31.1-03-PLAN.md — Re-bless golden fixtures and human-validate spec-correct IL output

### Phase 31.2: Fix Register Convention, Debug Info, and Parser Bugs
**Goal**: Function parameters occupy the correct registers (r0..r(param_count-1)), debug locals carry their source names, source spans are emitted, and bare `fn` declarations parse without error; fn_recursion and fn_empty_main golden tests pass with spec-correct IL
**Depends on**: Phase 31.1
**Requirements**: BUG-12, BUG-13, BUG-14, BUG-15
**Success Criteria** (what must be TRUE):
  1. Every function parameter is accessible via the same register in all branches — no uninitialized registers appear in emitted IL for functions with parameters
  2. `DebugLocal.name` for each register that corresponds to a source variable contains the correct string heap offset of that variable's name
  3. `SourceSpan` entries exist in every compiled method body, mapping instruction byte offsets to 1-based line/column source coordinates
  4. `fn main() {}` (without `pub`) compiles without a parse error and produces correct IL
  5. All function golden tests pass after re-blessing fn_recursion with spec-correct IL
**Plans**: 4 plans

Plans:
- [ ] 31.2-01-PLAN.md — Fix parameter register pre-allocation in emit_all_bodies (BUG-12); re-bless fn_recursion golden test
- [ ] 31.2-02-PLAN.md — Fix parser to accept bare `fn` declarations (BUG-15); add and bless fn_empty_main golden test
- [ ] 31.2-03-PLAN.md — Fix debug local names and source span emission (BUG-13, BUG-14)
- [ ] 31.2-04-PLAN.md — Human validation of re-blessed golden tests (checkpoint)

### Phase 32: Core Golden Tests Batch 1
**Goal**: Struct, enum, and control flow IL is hand-validated against the spec and locked as regression golden files
**Depends on**: Phase 31
**Requirements**: GOLD-03, GOLD-04, GOLD-15
**Success Criteria** (what must be TRUE):
  1. A structs.writ fixture covers `new` construction, field access, and method calls; its .expected file shows correct GET_FIELD/SET_FIELD instruction sequences
  2. An enums.writ fixture covers unit and payload variants plus pattern matching; its .expected file shows correct GET_TAG/SWITCH sequences and exhaustiveness
  3. A control_flow.writ fixture covers if/else, while, for, match, break, continue, and return; its .expected file shows correct branch and jump instruction sequences
  4. All three golden files pass the harness on the first run after hand-validation; any regression causes a diff failure
**Plans**: TBD

Plans:
- [ ] 32-01: TBD

### Phase 33: Core Golden Tests Batch 2
**Goal**: Entity, dialogue, and contract IL is hand-validated against the spec and locked as regression golden files
**Depends on**: Phase 32
**Requirements**: GOLD-05, GOLD-06, GOLD-07
**Success Criteria** (what must be TRUE):
  1. An entities.writ fixture covers entity definition, SPAWN_ENTITY/INIT_ENTITY/DESTROY_ENTITY sequences, component slots, and lifecycle hooks; its .expected file matches spec Section 28 entity construction
  2. A dialogue.writ fixture covers dlg blocks, say/say_localized emission, choice branches, speaker resolution, and string interpolation; its .expected file shows correct LocaleDef entries and SAY/CHOICE instructions
  3. A contracts.writ fixture covers contract definition, impl blocks, and operator overloading; its .expected file shows correct CALL_VIRT slot assignments and dispatch table entries
  4. All three golden files pass the harness and any regression causes a diff failure
**Plans**: TBD

Plans:
- [ ] 33-01: TBD

### Phase 34: Advanced Golden Tests Batch 1
**Goal**: Generic, error-handling, and Option type IL is hand-validated against the spec and locked as regression golden files
**Depends on**: Phase 33
**Requirements**: GOLD-08, GOLD-10, GOLD-14
**Success Criteria** (what must be TRUE):
  1. A generics.writ fixture covers generic functions, generic structs, type parameters, constraint bounds, and BOX/UNBOX at generic call sites; its .expected file shows correct type parameter specialization
  2. An error_handling.writ fixture covers Result type, `?` operator desugaring (IS_ERR + early return), try blocks, and WRAP_OK/WRAP_ERR; its .expected file matches spec error propagation sequences
  3. An option.writ fixture covers WRAP_SOME/LOAD_NULL, IS_NONE/IS_SOME, `!` unwrap, and `?` propagation; its .expected file matches spec Option instruction sequences
  4. All three golden files pass the harness and any regression causes a diff failure
**Plans**: TBD

Plans:
- [ ] 34-01: TBD

### Phase 35: Advanced Golden Tests Batch 2
**Goal**: Closure, concurrency, and globals IL is hand-validated against the spec and locked as regression golden files
**Depends on**: Phase 34
**Requirements**: GOLD-09, GOLD-11, GOLD-12
**Success Criteria** (what must be TRUE):
  1. A closures.writ fixture covers lambda expressions, capture struct synthesis, NEW_DELEGATE, CALL_INDIRECT, and function value passing; its .expected file shows correct delegate and capture emission
  2. A concurrency.writ fixture covers spawn (SPAWN_TASK), join (JOIN), cancel (CANCEL), defer (DEFER_PUSH/DEFER_POP), and detached spawn; its .expected file matches spec concurrency sequences
  3. A globals.writ fixture covers global constants, global mut, and atomic sections (ATOMIC_BEGIN/ATOMIC_END); its .expected file shows correct static field and atomic wrapper emission
  4. All three golden files pass the harness and any regression causes a diff failure
**Plans**: TBD

Plans:
- [ ] 35-01: TBD

### Phase 36: Remaining Golden Tests and Tech Debt
**Goal**: Arrays/strings IL is validated and locked; closure captures, GC finalization hooks, generic constraint tokens, and dead code are all resolved
**Depends on**: Phase 35
**Requirements**: GOLD-13, DEBT-01, DEBT-02, DEBT-03, DEBT-04
**Success Criteria** (what must be TRUE):
  1. An arrays_strings.writ fixture covers NEW_ARRAY, ARRAY_LOAD/STORE/LEN, STR_CONCAT/STR_BUILD, and I2S/F2S conversions; its .expected file shows correct array and string IL
  2. Closures that capture outer variables compile to delegates with populated capture structs and execute correctly on the VM
  3. GC finalization triggers `on_finalize` method invocation via the scheduler task queue (runtime.rs finalization wired end-to-end)
  4. Generic constraint DefIds resolve to non-zero metadata tokens during module finalization (no zero-token fallbacks for constraints)
  5. Dead code (extract_callee_def_id_opt and any other unreachable code identified during golden testing) is removed and the codebase is clean
**Plans**: TBD

Plans:
- [ ] 36-01: TBD

## Progress

**Execution Order:** 30 → 31 → 31.1 → 32 → 33 → 34 → 35 → 36 → 38

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 30. Critical Bug Fixes | v3.1 | 2/2 | Complete | 2026-03-04 |
| 31. Test Harness and Functions Golden Test | 1/2 | In Progress|  | - |
| 31.1. Fix Function IL Emission Bugs | v3.1 | 3/3 | Complete | 2026-03-04 |
| 31.2. Fix Register Convention, Debug Info, and Parser Bugs | 4/4 | Complete    | 2026-03-04 | - |
| 32. Core Golden Tests Batch 1 | v3.1 | 0/? | Not started | - |
| 33. Core Golden Tests Batch 2 | v3.1 | 0/? | Not started | - |
| 34. Advanced Golden Tests Batch 1 | v3.1 | 0/? | Not started | - |
| 35. Advanced Golden Tests Batch 2 | v3.1 | 0/? | Not started | - |
| 36. Remaining Golden Tests and Tech Debt | v3.1 | 0/? | Not started | - |
| 37. Fix spurious void register in empty function bodies | v3.1 | 2/2 | Complete | 2026-03-04 |
| 38. Fix CALL method token resolution in runtime | v3.1 | Complete    | 2026-03-04 | 2026-03-04 |

### Phase 37: Fix spurious void register in empty function bodies

**Goal:** Empty void function bodies emit zero registers — no spurious `.reg r0 void` declaration for functions like `fn main() {}` that produce no values
**Requirements**: BUG-16
**Depends on:** Phase 31.2
**Plans:** 2/2 plans complete

Plans:
- [x] 37-01-PLAN.md — Fix Block emitter to skip void reg allocation for empty void blocks; re-bless fn_empty_main and fn_basic_call golden files
- [x] 37-02-PLAN.md — Human-validate re-blessed golden IL output (checkpoint)

### Phase 38: Fix CALL method token resolution in runtime

**Goal:** Running any compiled `.writil` file that contains function calls succeeds — the VM resolves the MethodDef metadata token in `CALL` operands to the correct in-module method index instead of treating the raw token value as an array index
**Requirements**: BUG-17
**Depends on:** Phase 37
**Plans:** 1/1 plans complete

Plans:
- [x] 38-01-PLAN.md — Fix CALL/TailCall/NewDelegate/SpawnTask/SpawnDetached to decode MethodDef metadata tokens to 0-based method body indices; update vm_tests to use proper tokens

### Phase 39: Extend method_defs metadata to include parameter names, register indices, and type information

**Goal:** MethodDef metadata is complete — param_count is stored in the binary format (format_version 2), ParamDef rows carry correct names and types, and the disassembler displays parameter names in method signatures for human validation
**Requirements**: META-01
**Depends on:** Phase 38
**Plans:** 3/3 plans complete

Plans:
- [x] 39-01-PLAN.md — Extend MethodDefRow with param_count; update binary format reader/writer (format_version 2); wire param_count through compiler serialize
- [x] 39-02-PLAN.md — Update disassembler to show param names from ParamDef; re-bless all golden .expected files
- [x] 39-03-PLAN.md — Update IL spec with param_count documentation; add META-01 requirement; human validation checkpoint
