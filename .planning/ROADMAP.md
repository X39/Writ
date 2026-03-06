# Roadmap: Writ Compiler

## Milestones

- ✅ **v1.0 CST-to-AST Lowering Pipeline** — Phases 1-7 (shipped 2026-02-27)
- ✅ **v1.1 Spec v0.4 Conformance** — Phases 8-13 (shipped 2026-03-01)
- ✅ **v1.2 Gap Closure** — Phases 14-15 (shipped 2026-03-01)
- ✅ **v2.0 Writ Runtime** — Phases 16-21 (shipped 2026-03-02)
- ✅ **v3.0 Writ Compiler** — Phases 22-29 (shipped 2026-03-03)
- ✅ **v3.1 Compiler Bug Fixes** — Phases 30-39 (shipped 2026-03-04)
- ✅ **v3.2 Spec Corrections and Tooling** — Phases 40-46 (shipped 2026-03-07)
- ✅ **v4.0 Structs as Value Types** — Phases 47-51 (shipped 2026-03-13)
- ✅ **v5.0 LSP and DAP** — Phases 52-61 (shipped 2026-03-17)
- ✅ **v6.0 Code Cleanup** — Phases 62-66 (shipped 2026-03-18)
- ✅ **v6.1 LSP/DAP Fixes** — Phases 67-69 (shipped 2026-03-18)
- ✅ **v7.0 Benchmark Suite** — Phases 70-74 (shipped 2026-03-20)

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

<details>
<summary>✅ v3.1 Compiler Bug Fixes (Phases 30-39) — SHIPPED 2026-03-04</summary>

- [x] Phase 30: Critical Bug Fixes (2/2 plans) — completed 2026-03-04
- [x] Phase 31: Test Harness and Functions Golden Test (2/2 plans) — completed 2026-03-04
- [x] Phase 31.1: Fix Function IL Emission Bugs (3/3 plans) — completed 2026-03-04
- [x] Phase 31.2: Fix Register Convention, Debug Info, and Parser Bugs (4/4 plans) — completed 2026-03-04
- [x] Phase 32: Core Golden Tests Batch 1 — completed 2026-03-04
- [x] Phase 33: Core Golden Tests Batch 2 — completed 2026-03-04
- [x] Phase 34: Advanced Golden Tests Batch 1 — completed 2026-03-04
- [x] Phase 35: Advanced Golden Tests Batch 2 — completed 2026-03-04
- [x] Phase 36: Remaining Golden Tests and Tech Debt — completed 2026-03-04
- [x] Phase 37: Fix Spurious Void Register in Empty Function Bodies (2/2 plans) — completed 2026-03-04
- [x] Phase 38: Fix CALL Method Token Resolution in Runtime (1/1 plan) — completed 2026-03-04
- [x] Phase 39: Extend method_defs Metadata (3/3 plans) — completed 2026-03-04

Full details: `milestones/v3.1-ROADMAP.md`

</details>

<details>
<summary>✅ v3.2 Spec Corrections and Tooling (Phases 40-46) — SHIPPED 2026-03-07</summary>

- [x] Phase 40: Spec Cleanup (3/3 plans) — completed 2026-03-06
- [x] Phase 41: Fix fn_log_say_choice (3/3 plans) — completed 2026-03-06
- [x] Phase 42: ChoiceOption Rename (1/1 plan) — completed 2026-03-06
- [x] Phase 43: Unqualified None/Some (3/3 plans) — completed 2026-03-06
- [x] Phase 44: Extended Log with Levels (2/2 plans) — completed 2026-03-06
- [x] Phase 45: writ.toml Project File Compilation (2/2 plans) — completed 2026-03-06
- [x] Phase 46: Structs-as-Value-Types Design Discussion (1/1 plan) — completed 2026-03-06

Full details: `milestones/v3.2-ROADMAP.md`

</details>

<details>
<summary>✅ v4.0 Structs as Value Types (Phases 47-51) — SHIPPED 2026-03-13</summary>

- [x] Phase 47: Spec Amendments (4/4 plans) — completed 2026-03-12
- [x] Phase 48: IL Format (2/2 plans) — completed 2026-03-12
- [x] Phase 49: VM Runtime (2/2 plans) — completed 2026-03-12
- [x] Phase 50: Compiler Frontend (2/2 plans) — completed 2026-03-13
- [x] Phase 51: Compiler Backend and Tests (2/2 plans) — completed 2026-03-13

Full details: `milestones/v4.0-ROADMAP.md`

</details>

<details>
<summary>✅ v5.0 LSP and DAP (Phases 52-61) — SHIPPED 2026-03-17</summary>

- [x] Phase 52: Compiler and Runtime Preparation (3/3 plans) — completed 2026-03-13
- [x] Phase 53: LSP Server Skeleton and Diagnostics (3/3 plans) — completed 2026-03-14
- [x] Phase 54: LSP Navigation and Completions (4/4 plans) — completed 2026-03-14
- [x] Phase 55: DAP Server Core (2/2 plans) — completed 2026-03-14
- [x] Phase 56: DAP Advanced Inspection (2/2 plans) — completed 2026-03-14
- [x] Phase 57: VS Code Extension Integration (2/2 plans) — completed 2026-03-16
- [x] Phase 58: Dialogue Speaker Semantic Tokens (1/1 plan) — completed 2026-03-16
- [x] Phase 59: VSIX Release Build (1/1 plan) — completed 2026-03-16
- [x] Phase 60: LSP Query Robustness (1/1 plan) — completed 2026-03-17
- [x] Phase 61: Signature Help, Diagnostics, and Extension Polish (1/1 plan) — completed 2026-03-17

Full details: `milestones/v5.0-ROADMAP.md`

</details>

<details>
<summary>✅ v6.0 Code Cleanup (Phases 62-66) — SHIPPED 2026-03-18</summary>

- [x] Phase 62: Clippy Warning Elimination (2/2 plans) — completed 2026-03-18
- [x] Phase 63: writ-compiler File Splits (4/4 plans) — completed 2026-03-18
- [x] Phase 64: Cross-Crate File Splits (4/4 plans) — completed 2026-03-18
- [x] Phase 65: Code Duplication and Module Boundaries (3/3 plans) — completed 2026-03-18
- [x] Phase 66: Regression Fixes (1/1 plan) — completed 2026-03-18

Full details: `milestones/v6.0-ROADMAP.md`

</details>

<details>
<summary>✅ v6.1 LSP/DAP Fixes (Phases 67-69) — SHIPPED 2026-03-18</summary>

- [x] Phase 67: LSP Completions (2/2 plans) — completed 2026-03-18
- [x] Phase 68: DAP Runtime and Launch Fixes (2/2 plans) — completed 2026-03-18
- [x] Phase 69: Dialogue/Function Golden Tests (1/1 plan) — completed 2026-03-18

Full details: `milestones/v6.1-ROADMAP.md`

</details>

<details>
<summary>✅ v7.0 Benchmark Suite (Phases 70-74) — SHIPPED 2026-03-20</summary>

- [x] Phase 70: Docker Environment and Measurement Harness (4/4 plans) — completed 2026-03-20
- [x] Phase 71: Compute Benchmarks MVP (2/2 plans) — completed 2026-03-20
- [x] Phase 72: Chart Generation and Results Pipeline (2/2 plans) — completed 2026-03-20
- [x] Phase 73: Remaining Benchmark Categories (3/3 plans) — completed 2026-03-20
- [x] Phase 74: CI Workflow (1/1 plan) — completed 2026-03-20

Full details: `milestones/v7.0-ROADMAP.md`

</details>

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-7 | v1.0 | 13/13 | Complete | 2026-02-27 |
| 8-13 | v1.1 | 12/12 | Complete | 2026-03-01 |
| 14-15 | v1.2 | 3/3 | Complete | 2026-03-01 |
| 16-21 | v2.0 | 16/16 | Complete | 2026-03-02 |
| 22-29 | v3.0 | 24/24 | Complete | 2026-03-03 |
| 30-39 | v3.1 | 22/22 | Complete | 2026-03-04 |
| 40-46 | v3.2 | 15/15 | Complete | 2026-03-07 |
| 47-51 | v4.0 | 12/12 | Complete | 2026-03-13 |
| 52-61 | v5.0 | 20/20 | Complete | 2026-03-17 |
| 62-66 | v6.0 | 14/14 | Complete | 2026-03-18 |
| 67-69 | v6.1 | 5/5 | Complete | 2026-03-18 |
| 70-74 | v7.0 | 12/12 | Complete | 2026-03-20 |
