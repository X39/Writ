# Phase 41: Fix fn_log_say_choice - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Diagnose why the `fn_log_say_choice` golden test produces completely empty method bodies, fix the root cause in the compiler pipeline, add BOM-stripping to the golden test harness, and re-bless the snapshot in UTF-8 encoding.

Out of scope: renaming `Option` → `ChoiceOption` (Phase 42), `.writc` artifact update (milestone completion).

</domain>

<decisions>
## Implementation Decisions

### Root cause investigation approach
- Start at `emit_bodies` entry: add eprintln/debug assertion to confirm the TypedExpr tree for `main` contains the call expressions before any instruction is emitted
- If calls ARE in the tree but not emitting: focus on the ExternDef token lookup path — check whether `log`, `say`, `choice` ExternDef entries are registered in the module builder (codegen dispatch checks `callee_def_id` → ExternDef token; if absent, `method_idx=0` is used but the call should still emit)
- Also check for silent skip guards that bypass body emission entirely without producing resolver errors — if found, also investigate WHY they trigger while the pre-stages (resolve, typecheck) pass without errors
- Document root cause in `.planning/phases/41-fix-fn-log-say-choice/41-NOTES.md` before committing the fix

### Fix scope — root-qualified path forms
- `::log`, `::say`, `::choice` are valid Writ per §23.9 (leading `::` means "from root namespace") — the test source is correct and must not be changed
- The fix must make both `::log` (root-qualified) and `log` (unqualified) resolve and emit IL correctly from a regular `fn` context — both forms are spec-valid and should produce identical codegen
- Check whether a separate phase already covers root-qualified path resolution; if not, fix both forms in Phase 41

### Spec clarification
- Add an explicit note near the inbuilt function definitions (wherever `log`, `say`, `choice` are documented) stating that `::log`, `::say`, `::choice` (root-qualified forms) are valid and equivalent to the unqualified names — this is NOT covered by Phase 40's SPEC-02 (which clarified only that no `Runtime::` qualifier is needed)
- §23.9 already covers the general case; the inbuilt-specific note closes the gap for implementers

### BOM-stripping in golden test harness
- Strip UTF-16 BOM when READING the expected `.writil` file before comparison — never on write (BLESS=1 writes clean UTF-8 from the Rust disassembler; no BOM introduced)
- Reuse the existing BOM-strip utility from the compiler codebase
- Constraint: golden files must NEVER be auto-modified on a test run — BOM-strip is read-only (comparison normalization only)
- This handles the case where a user hand-edits a `.writil` file on a system that saves UTF-16 LE with BOM

### Blessing workflow
- Fix codegen, then run `BLESS=1 cargo test -p writ-golden` to re-bless `fn_log_say_choice.writil`
- The blessed file will be UTF-8 (no BOM) — Rust `String` → `std::fs::write` always produces UTF-8
- `.writc` artifact update is deferred to milestone completion (`writ compile` + `writ disasm` manually)

### Validation
- The round-trip in `compile_and_disassemble` (compile → serialize → `Module::from_bytes` → disassemble) satisfies success criteria 4 — no separate `writ disasm` CLI invocation needed in tests
- Re-bless `.writil` only; `.writc` is for manual inspection and not used by the test suite

### Claude's Discretion
- Exact location of BOM-strip logic (read path vs. helper function)
- Whether to add a UTF-8 assertion in the golden test or rely on the natural disassembler output
- Implementation detail of silent skip guard investigation (eprintln vs. unit test vs. breakpoint)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `bless_golden` (`writ-golden/tests/golden_tests.rs:103`) — writes actual string to `.writil` using `std::fs::write` (UTF-8); BOM-strip should be added to `run_golden_test` read path, not here
- `compile_and_disassemble` (`writ-golden/tests/golden_tests.rs:24`) — full pipeline with 16MB stack thread; includes `Module::from_bytes` round-trip before disassembly
- `writ-compiler/src/resolve/prelude.rs` — prelude definitions; `log`/`say`/`choice` are NOT listed here — they're inbuilt functions handled elsewhere in resolution

### Established Patterns
- Codegen call dispatch (`writ-compiler/src/emit/body/expr.rs:209`) — checks `callee_def_id` → ExternDef token via `emitter.builder.token_for_def(id)`; if token absent, `method_idx=0` is used but instruction is still emitted; this path would NOT produce an empty body — so the empty body has an earlier cause
- `try_emit_builtin_method` (`writ-compiler/src/emit/body/expr.rs:210`) — shortcut for Option/Result/Array methods; `log`/`say`/`choice` are NOT handled here
- Resolution pipeline: `resolve_diags` with `Severity::Error` check aborts before codegen; if codegen runs at all, resolution "succeeded" (but may have produced Warnings)

### Integration Points
- `fn_log_say_choice.writ` — uses `::log`, `::say`, `::choice`, `::Option` (root-qualified inbuilt calls from a regular `fn`, not a `dlg`)
- `fn_log_say_choice.writil` — currently UTF-16 LE with BOM; two closure structs, two empty method bodies (`__invoke_1` and `main`)
- `fn_log_say_choice.writc` — binary artifact for manual inspection; not used by tests

</code_context>

<specifics>
## Specific Ideas

- The completely empty method body (zero instructions) — not wrong instructions, zero — points to either: (a) the `TypedExpr` tree for `main` is empty/missing after typechecking, or (b) a guard in `emit_bodies` aborts body emission before the first instruction. If the typed AST IS present but instructions aren't emitted, the ExternDef registration path is the next place to look.
- `::log` from a regular `fn` is different from `$ log` in a dialogue context — the lowering for dialogue (`lower/dialogue.rs`) handles `say`/`choice` as special AST nodes; from a regular `fn`, they must resolve as extern function calls via the normal path.
- Investigate whether the `.writ` source's leading BOM (UTF-8 BOM visible as `﻿` at file start) affects parsing — the file shows a BOM on the first line.

</specifics>

<deferred>
## Deferred Ideas

- `.writc` artifact update — defer to milestone completion (`writ compile fn_log_say_choice.writ` + `writ disasm fn_log_say_choice.writc > fn_log_say_choice.writil`)
- None/Some unqualified access — Phase 43
- ChoiceOption rename — Phase 42

</deferred>

---

*Phase: 41-fix-fn-log-say-choice*
*Context gathered: 2026-03-06*
