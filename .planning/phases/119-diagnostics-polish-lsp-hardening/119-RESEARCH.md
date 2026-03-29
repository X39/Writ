# Phase 119: Diagnostics Polish & LSP Hardening - Research

**Researched:** 2026-03-29
**Domain:** Writ compiler diagnostics, CLI flag handling, LSP partial-parse resilience
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — discuss phase was skipped per `workflow.skip_discuss`. All implementation choices are at Claude's discretion.

### Claude's Discretion
All implementation choices are at Claude's discretion. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Deferred Ideas (OUT OF SCOPE)
None — discuss phase skipped.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DIAG-01 | Constraint violation errors show both call site and constraint declaration spans | Infrastructure already exists; need to verify the cross-file sources slice construction does not panic ariadne |
| DIAG-02 | Errors include fix suggestions with concrete code snippets | `UnsatisfiedBound` already has `with_help`; need to audit ALL other error sites for completeness, and wire fix suggestions into the LSP code actions for E0103 |
| DIAG-03 | `--deny-warnings` CLI flag treats warnings as errors | Clap flag needed on `build` and `compile` subcommands; pipeline must re-check severity after all warnings are collected |
| DIAG-04 | LSP provides completions and hover on files with syntax errors (partial parse recovery) | chumsky already returns partial CSTs; the LSP analysis_host must pass the partial AST forward through resolve/typecheck with catch_unwind even when parse errors are present |
</phase_requirements>

---

## Summary

Phase 119 is a quality-polish phase touching four distinct subsystems: the compiler diagnostic data layer (DIAG-01/02), the CLI exit-code behavior (DIAG-03), and the LSP robustness under edit conditions (DIAG-04). None of these require new language features; all require targeted changes to existing infrastructure.

**DIAG-01 and DIAG-02** are largely already implemented. `TypeError::UnsatisfiedBound` emits a secondary label pointing to the generic param declaration (`bound_decl_span`) and a `with_help` suggestion ("consider adding `impl ContractName for TypeName { ... }`"). Two unit tests (`generic_bound_error_has_secondary_label`, `generic_bound_error_has_help_suggestion`) pass today. The gap is the `render_diagnostics` sources slice: when `bound_decl_file` differs from the primary file, ariadne panics if that FileId is absent from the sources slice. This is the critical pitfall from STATE.md that must be audited and guarded.

**DIAG-03** is a pure CLI addition. The `--deny-warnings` flag must be added to the `build` and `compile` subcommands via clap, threaded into `run_pipeline`, and cause the pipeline to return `Err` (exit code 1) when any `Severity::Warning` diagnostic is present.

**DIAG-04** is the most nuanced requirement. The chumsky parser already performs item-level error recovery and returns a partial CST alongside errors. The issue is that `analyze_standalone` returns early (`return AnalysisResult { typed_ast: None, ... }`) when `cst_opt` is `None`, meaning a total parse failure yields no typed AST and therefore no hover/completion. The partial-CST path (where `cst_opt` is `Some` despite `parse_errs` being non-empty) already continues — this is the path to verify and harden. The LSP handlers for hover and completion already guard against `typed_ast: None` by returning `Ok(None)`, which is the correct graceful behavior. The 200ms responsiveness requirement is met by the existing `spawn_blocking` dispatch — no additional work needed there.

**Primary recommendation:** Implement in three independent tasks: (1) audit + guard ariadne sources slice for cross-file secondary labels, (2) add `--deny-warnings` to CLI, (3) write and pass LSP tests that exercise hover/completion on syntactically incomplete source.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ariadne | in Cargo.lock | Terminal diagnostic rendering with colored spans | Already the project's renderer; `render_diagnostics` wraps it |
| chumsky | in Cargo.lock | Parser combinators with error recovery | Parser is already chumsky; recovery strategy already in place |
| clap | in Cargo.lock | CLI argument parsing | Already used for all subcommands |
| tower-lsp | in Cargo.lock | LSP protocol server | Already the LSP framework |

No new dependencies required for this phase.

---

## Architecture Patterns

### Recommended Project Structure

No structural changes. Changes are in-place modifications to:

```
writ-diagnostics/src/render.rs        # sources slice guard
writ-compiler/src/check/error.rs      # help text audit
writ-cli/src/main.rs                  # --deny-warnings flag on Build + Compile
writ-cli/src/pipeline.rs              # deny_warnings parameter + warning check
writ-lsp/src/analysis_host.rs         # partial-parse continuity audit
writ-lsp/tests/test_protocol.rs       # new LSP robustness tests
writ-compiler/tests/typecheck_tests.rs  # existing GEN-05/06 tests already pass
```

### Pattern 1: Ariadne Sources Slice Guard

**What:** Before calling `report.write_for_stdout(cache, &mut buf)`, all FileIds referenced in the diagnostic (primary + all secondary labels) must appear in the `sources` slice passed to `render_diagnostics`.

**The panic:** ariadne calls `cache.fetch(file_id)` and panics if the FileId is absent. This path is triggered when `UnsatisfiedBound` references `bound_decl_file` (the file where the generic function is declared) which differs from the primary file (where the call site is). In single-file compilation this is the same file and safe. In multi-file project compilation the two may differ.

**Guard approach:** In `render_diagnostics`, before building the ariadne cache, filter secondary labels to only include those whose `file_id` is present in the `sources` slice. Alternatively, the caller (`run_pipeline` and `analyze_standalone`) must ensure the sources slice includes all files referenced by any secondary label. The cleaner fix is defensive filtering inside `render_diagnostics` so callers cannot accidentally trigger a panic.

**Special case: `FileId(u32::MAX)`** is the sentinel for synthetic builtins (prelude functions, builtin contracts). A secondary label pointing to `FileId(u32::MAX)` must always be stripped from the diagnostic or its `file_id` overridden to the call-site file before rendering. The bound declaration spans for builtin functions default to `SimpleSpan::new((), 0..0)` — safe to drop.

```rust
// In render_diagnostics, filter secondary labels to known sources:
let known_file_ids: std::collections::HashSet<FileId> =
    sources.iter().map(|(id, _, _)| *id).collect();

for sec in &diag.secondary_labels {
    if !known_file_ids.contains(&sec.file_id) {
        continue; // skip labels for files not in the sources slice
    }
    // ... add label
}
```

**Source:** Codebase audit of `render.rs:65-66` and STATE.md pitfall note (HIGH confidence).

### Pattern 2: --deny-warnings CLI Flag

**What:** A new `--deny-warnings` flag on `build` and `compile` subcommands causes the pipeline to exit with code 1 when any warning diagnostic is emitted.

**Implementation:**

1. Add `#[arg(long)] deny_warnings: bool` to `Commands::Build` and `Commands::Compile` in `writ-cli/src/main.rs`.
2. Thread `deny_warnings` into `run_pipeline` as a new parameter.
3. After each stage that emits diagnostics (`resolve_diags`, `type_diags`), check for warnings when `deny_warnings` is true:

```rust
if deny_warnings && type_diags.iter().any(|d| d.severity == Severity::Warning) {
    eprint!("{}", render_diagnostics(&type_diags, &sources_for_render));
    return Err("warnings treated as errors (--deny-warnings)".to_string());
}
```

4. The `run_pipeline` signature change: add `deny_warnings: bool` parameter.
5. The `main.rs` dispatch must thread the flag from each `Commands::*` arm into the pipeline call.

**Exit code behavior:** `main.rs` already calls `process::exit(1)` on any `Err` from the command handlers, so no special exit-code handling is needed — returning `Err` is sufficient.

### Pattern 3: LSP Partial-Parse Continuity

**What:** When the parser returns `(Some(cst), parse_errs_non_empty)`, the analysis host must continue through lower, resolve, and typecheck so hover/completion work on the valid parts of the AST.

**Current behavior in `analyze_standalone`:**
- If `cst_opt` is `None` → returns early (correct: no AST to work with).
- If `cst_opt` is `Some(cst)` despite `parse_errs` non-empty → **already continues** to lower/resolve/typecheck.

This means partial-parse continuity is largely in place. The issue to verify is whether `resolve` and `typecheck` panic on `Cst::Expr::Error` nodes that appear in partially-recovered syntax. Both stages are already wrapped in `catch_unwind` in the analysis host, so panics degrade to an internal diagnostic rather than crashing the server.

**The actual risk:** Chumsky's `Expr::Error` sentinel nodes flow into the lowering stage. The lowerer must produce an `AstExpr::Error` (or equivalent) for these. Verify `lower()` does not panic on `cst::Expr::Error`.

**Semantic tokens on partial parse:** `collect_semantic_tokens` already takes `typed_ast` which is `None` when typecheck failed — the semantic tokens handler already returns `Ok(None)` in that case, which is the correct behavior (no tokens rather than crash). For partial parse where typecheck succeeds on the valid subset, tokens are returned normally.

**200ms responsiveness:** The `spawn_blocking` dispatch in `backend.rs:publish_diagnostics_for` already ensures the async executor is never blocked. The compiler pipeline on a 500-line file takes well under 200ms in practice. No additional work needed.

### Anti-Patterns to Avoid

- **Dropping all secondary labels globally:** Would lose useful context. Only drop labels pointing to FileIds absent from the sources slice or pointing to `FileId(u32::MAX)`.
- **Running warning-check before diagnostics are rendered:** Users need to see what warnings triggered the failure. Always render before returning `Err`.
- **Returning `Err` from `run_pipeline` on warnings before completing all diagnostic stages:** Collect and render all diagnostics from the current stage before returning. Do not short-circuit mid-stage.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Diagnostic rendering | Custom renderer | ariadne (already present) | ariadne handles span highlighting, color, and multi-span layouts |
| CLI arg parsing | Manual argv | clap (already present) | Consistent `--help` output, conflicts_with, default_value |
| LSP panic recovery | Manual panic catching | `std::panic::catch_unwind` (already present in analysis_host.rs) | Already established pattern in this codebase |

---

## Common Pitfalls

### Pitfall 1: ariadne Panics on Missing FileId (CRITICAL — from STATE.md)

**What goes wrong:** `ariadne::Report::write_for_stdout` calls `cache.fetch(file_id)` for every label. If a secondary label references a FileId not present in the `sources` slice, ariadne panics with an index-out-of-bounds or unwrap failure.

**Why it happens:** `UnsatisfiedBound` always attaches a secondary label to `bound_decl_file` (the file where the generic function is declared). In single-file compilation all FileIds are the same (FileId(0)). In multi-file compilation or when the function is a builtin (FileId(u32::MAX)), the secondary file differs and may not be in the sources slice.

**How to avoid:** In `render_diagnostics`, filter secondary labels to only those whose `file_id` is present in the `sources` slice. Filter before building the ariadne cache, not after.

**Warning signs:** Tests pass for single-file sources but crash when a generic function is defined in one file and called from another.

### Pitfall 2: --deny-warnings Must Render Before Failing

**What goes wrong:** Returning `Err` before rendering diagnostics produces a "warnings treated as errors" message with no indication of which warnings triggered it.

**Why it happens:** It is tempting to check `has_warnings` and return immediately without rendering.

**How to avoid:** Always render the diagnostics first (`eprint!("{}", render_diagnostics(...))`), then check the warning condition and return `Err` with a summary message.

### Pitfall 3: Partial-Parse Path Requires Lower Not to Panic on Cst::Expr::Error

**What goes wrong:** If `writ_compiler::lower()` panics when it encounters `cst::Expr::Error` nodes (which the parser inserts during error recovery), the catch_unwind in the LSP's analysis_host will catch it but emit an internal-stage-panic diagnostic instead of the real parse errors. Hover/completion return nothing.

**Why it happens:** The lowerer may have a `match expr { ... _ => unreachable!() }` pattern that doesn't handle `Expr::Error`.

**How to avoid:** Before writing the DIAG-04 LSP tests, verify `lower()` handles `Expr::Error` by producing a no-op AST node (not panicking). The existing test suite should catch this if there is a test with syntax errors.

**Warning signs:** `analyze_standalone` emits "internal-stage-panic in lower" diagnostics on incomplete source.

### Pitfall 4: Semantic Tokens Must Tolerate None TypedAst

**What goes wrong:** The semantic tokens handler uses `analysis_cache` to look up `typed_ast`. If analysis failed (parse or typecheck errors), `typed_ast` is `None`. The handler returns `Ok(None)` which the client interprets as "no tokens" — this is correct behavior but may look like a regression if the test expects tokens on a clean file that was made syntactically incomplete.

**Why it happens:** Tests that previously worked on clean source now test with broken source.

**How to avoid:** Tests for DIAG-04 should verify that hover/completion return `None` (gracefully) rather than a server crash. They should also verify tokens are returned on the VALID portion of a partially-valid file (where `typed_ast` is Some despite parse errors).

---

## Code Examples

### Fix: render_diagnostics sources slice guard

```rust
// writ-diagnostics/src/render.rs
// Source: codebase audit of render.rs + ariadne API
pub fn render_diagnostics(diagnostics: &[Diagnostic], sources: &[(FileId, &str, &str)]) -> String {
    use ariadne::{Color, Label, Report, ReportKind};
    use std::fmt::Write as _;

    let known_file_ids: std::collections::HashSet<FileId> =
        sources.iter().map(|(id, _, _)| *id).collect();

    let mut output = String::new();

    for diag in diagnostics {
        // Skip diagnostics whose primary file is not in sources
        if !known_file_ids.contains(&diag.primary_file) {
            continue;
        }
        // ... build report as before ...

        // Secondary labels: skip any pointing to absent FileIds
        for sec in &diag.secondary_labels {
            if !known_file_ids.contains(&sec.file_id) {
                continue; // do not add label for unknown file
            }
            // ... add label ...
        }
    }

    output
}
```

### Add --deny-warnings to run_pipeline

```rust
// writ-cli/src/pipeline.rs
pub fn run_pipeline(
    file_sources: Vec<(writ_diagnostics::FileId, String, &'static str)>,
    _module_name: Option<&str>,
    emit_debug_info: bool,
    active_conditions: &std::collections::HashSet<String>,
    deny_warnings: bool,   // NEW
) -> Result<Vec<u8>, String> {
    // ...
    // After rendering type_diags:
    let has_type_errors = type_diags.iter().any(|d| d.severity == Severity::Error);
    let has_warnings = type_diags.iter().any(|d| d.severity == Severity::Warning);
    if !type_diags.is_empty() {
        eprint!("{}", render_diagnostics(&type_diags, &sources_for_render));
    }
    if has_type_errors {
        return Err("type checking failed".to_string());
    }
    if deny_warnings && has_warnings {
        return Err("compilation failed: warnings treated as errors (--deny-warnings)".to_string());
    }
    // ...
}
```

### LSP partial-parse test pattern (DIAG-04)

```rust
// writ-lsp/tests/test_protocol.rs
// Pattern for testing LSP features on syntactically incomplete source.
// The parser returns a partial CST; hover/completion should return Ok(None)
// gracefully — not crash or hang.
async fn test_lsp_hover_on_incomplete_source() {
    // Source with unterminated string — parser recovers partially
    let incomplete_source = r#"
        pub fn main() {
            let x: int = "unterminated
        }
    "#;
    // Send did_open with incomplete source, then send hover request
    // Expect: no crash, Ok response (possibly None hover, not server error)
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Panic on missing ariadne FileId | Guard sources slice before rendering | Phase 119 | Prevents crash on cross-file secondary labels |
| No --deny-warnings flag | `--deny-warnings` clap flag on build/compile | Phase 119 | CI pipelines can enforce zero-warning policy |
| LSP returns nothing on parse errors | LSP continues analysis on partial CST | Phase 119 (verify existing) | Hover/completion work even mid-edit |

---

## Open Questions

1. **Does the lowerer handle `Cst::Expr::Error` without panicking?**
   - What we know: the lowerer produces an AST from the CST; error recovery nodes exist in the CST (`cst::Expr::Error`).
   - What's unclear: whether the lowerer has a match arm for `Expr::Error` or relies on `unreachable!()`.
   - Recommendation: write a unit test in `lowering_tests.rs` that parses a file with a syntax error, runs `lower()`, and asserts no panic. This should be the first step in the DIAG-04 plan.

2. **Should `--deny-warnings` apply only to warnings from the type-check stage, or also from parse/lower/resolve?**
   - What we know: warnings currently only come from the type-check stage (W000x codes are all emitted in check_expr or resolve).
   - What's unclear: future warnings from lower or resolve would need coverage.
   - Recommendation: apply `deny_warnings` check after every stage that produces warnings, for forward-compatibility.

3. **Does the `build` command need `--deny-warnings` or is `compile` sufficient?**
   - What we know: DIAG-03 says "CLI flag" without specifying subcommand.
   - Recommendation: add to both `build` and `compile` for completeness. The test should exercise `compile` (simpler setup).

---

## Environment Availability

Step 2.6: SKIPPED — Phase 119 is code/config only. No external tools, databases, or services beyond the existing Rust/Cargo toolchain are required.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`#[test]`) |
| Config file | None — standard `cargo test` |
| Quick run command | `cargo test --package writ-compiler` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| DIAG-01 | `UnsatisfiedBound` emits secondary label pointing to bound decl | unit | `cargo test --package writ-compiler generic_bound_error_has_secondary_label` | YES (passing) |
| DIAG-01 | render_diagnostics does NOT panic when secondary label FileId absent from sources | unit | `cargo test --package writ-diagnostics render_diagnostics_cross_file_guard` | NO — Wave 0 |
| DIAG-02 | `UnsatisfiedBound` help text includes "consider adding `impl ContractName for TypeName`" | unit | `cargo test --package writ-compiler generic_bound_error_has_help_suggestion` | YES (passing) |
| DIAG-02 | All other error variants that have actionable fixes include non-empty help | unit | `cargo test --package writ-compiler all_fixable_errors_have_help` | NO — Wave 0 |
| DIAG-03 | `--deny-warnings` causes exit code 1 when warnings present | integration | `cargo test --package writ-compiler deny_warnings_exits_on_warning` | NO — Wave 0 |
| DIAG-03 | `--deny-warnings` exit code 0 when no warnings | integration | `cargo test --package writ-compiler deny_warnings_clean_source` | NO — Wave 0 |
| DIAG-04 | LSP hover returns Ok(None) gracefully on incomplete expression | integration | test in `writ-lsp/tests/test_protocol.rs` | NO — Wave 0 |
| DIAG-04 | LSP completion returns results on syntactically valid context despite earlier parse error | integration | test in `writ-lsp/tests/test_protocol.rs` | NO — Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --package writ-compiler` (fast, < 5s)
- **Per wave merge:** `cargo test` (full suite)
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-diagnostics/src/render.rs` test: `render_diagnostics_cross_file_guard` — covers DIAG-01 ariadne safety
- [ ] `writ-compiler/tests/typecheck_tests.rs` test: `all_fixable_errors_have_help` — covers DIAG-02 completeness
- [ ] `writ-compiler/tests/typecheck_tests.rs` or separate integration test: `deny_warnings_exits_on_warning`, `deny_warnings_clean_source` — covers DIAG-03
- [ ] `writ-lsp/tests/test_protocol.rs`: two new async tests for DIAG-04 partial-parse LSP resilience

---

## Project Constraints (from CLAUDE.md)

No `CLAUDE.md` found in the working directory. No additional project-specific constraints apply beyond what is captured in STATE.md and REQUIREMENTS.md.

---

## Sources

### Primary (HIGH confidence)
- Codebase: `writ-diagnostics/src/render.rs` — ariadne rendering, sources slice construction
- Codebase: `writ-diagnostics/src/diagnostic.rs` — Diagnostic, SecondaryLabel, FileId types
- Codebase: `writ-compiler/src/check/error.rs` — TypeError::UnsatisfiedBound with_help and with_secondary
- Codebase: `writ-compiler/src/check/env.rs` — FnSig.fn_file, bound_decl_spans
- Codebase: `writ-compiler/src/check/check_expr/call.rs` — constraint enforcement emitting UnsatisfiedBound
- Codebase: `writ-cli/src/main.rs`, `pipeline.rs`, `commands/build.rs`, `commands/compile.rs` — CLI structure
- Codebase: `writ-lsp/src/analysis_host.rs` — partial-parse continuity, catch_unwind strategy
- Codebase: `writ-lsp/src/backend.rs` — publish_diagnostics_for dispatch, analysis_cache
- Codebase: `writ-lsp/src/queries/code_actions.rs` — existing E0123 code action pattern
- Codebase: `writ-parser/src/parser/program.rs` — chumsky error recovery strategy
- `.planning/STATE.md` — Phase 119 pitfall: ariadne panics on absent FileId
- `.planning/REQUIREMENTS.md` — DIAG-01 through DIAG-04 definitions

### Secondary (MEDIUM confidence)
- `writ-compiler/tests/typecheck_tests.rs` — existing GEN-05/GEN-06 tests passing (verified by test run)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already present, verified in Cargo.toml
- Architecture: HIGH — all patterns derived from direct codebase inspection
- Pitfalls: HIGH — critical pitfall confirmed by STATE.md note + render.rs code audit; verified by reading the ariadne sources slice construction

**Research date:** 2026-03-29
**Valid until:** 2026-06-29 (stable codebase, no external dependencies)
