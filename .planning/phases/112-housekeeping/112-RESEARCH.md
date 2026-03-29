# Phase 112: Housekeeping - Research

**Researched:** 2026-03-29
**Domain:** Spec documentation, golden test registration, LSP code hygiene
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all implementation choices are at Claude's discretion.

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SPEC-01 | §26.4 appears in spec table of contents | Already present in `language-spec/spec/01_table_of_contents.md` — verified in place; only a confirmation pass is needed |
| SPEC-02 | `using log::*;` behavior documented in spec | Not yet documented; must add a note to §1.24.4.4 in `language-spec/spec/25_24_modules_namespaces.md` |
| TEST-01 | `test_fn_optional` registered and running in golden_tests.rs | Already registered with `#[test]`, passes, and `fn_optional.writil` is blessed; only a confirmation pass is needed |
| LSP-02 | Orphaned `collect_dialogue_speaker_tokens` re-export removed from `queries/mod.rs` | Line 41 of `writ-lsp/src/queries/mod.rs`; safe to delete — no external callers confirmed |
</phase_requirements>

## Summary

Phase 112 is a four-item housekeeping pass. Two of the four items (SPEC-01, TEST-01) are already satisfied in the
current codebase and only need a confirmation pass before marking done. The remaining two (SPEC-02, LSP-02) are genuine
one-file edits: a single sentence added to the modules spec, and a single `pub use` line removed from the LSP query
module.

The total code surface is minimal: one line deleted in Rust, and a short prose addition to a Markdown file. No new
tests are needed, no new files are created, and no functional behaviour changes.

**Primary recommendation:** Write a single plan with one wave. SPEC-01 and TEST-01 are verification tasks; SPEC-02 and
LSP-02 are the two edits. Verify workspace compiles after removing the re-export.

## Standard Stack

No external libraries are introduced. This phase only modifies existing files.

### Relevant Files

| File | Purpose | Modification |
|------|---------|-------------|
| `language-spec/spec/01_table_of_contents.md` | Spec TOC | Read-only verification |
| `language-spec/spec/25_24_modules_namespaces.md` | Modules spec §1.24 | Add `using log::*;` limitation note |
| `writ-golden/tests/golden_tests.rs` | Golden test harness | Read-only verification |
| `writ-golden/tests/golden/fn_optional.writil` | Blessed output snapshot | Read-only verification |
| `writ-lsp/src/queries/mod.rs` | LSP query re-exports | Remove line 41 (`pub use semantic::collect_dialogue_speaker_tokens;`) |

## Architecture Patterns

### Golden Test Structure

The golden test system in `writ-golden/tests/golden_tests.rs` follows a consistent pattern:

```rust
/// Doc comment describing what the test locks.
#[test]
fn test_NAME() {
    run_golden_test("NAME");
}
```

`run_golden_test` reads `tests/golden/{name}.writ`, compiles and disassembles it, then either blesses
(`BLESS=1` env var) or diffs against `tests/golden/{name}.writil`. Both files must exist for the test
to pass without blessing.

**Current state of SPEC-01 / TEST-01:** These items were introduced as tech debt at milestone v3.2
but were addressed in subsequent phases before the v12.0 roadmap was created:

- `test_fn_optional` was added in commit `0ce1cbf` (phase 43) — well before this phase.
- `fn_optional.writil` was blessed in the same commit.
- `§1.26.4 Compiler Tooling` was added to `01_table_of_contents.md` in commit `094c2e3` (phase 47-04).

The PROJECT.md and REQUIREMENTS.md carry stale descriptions of these as open items. The plan must verify
current state and mark them done rather than performing redundant work.

### LSP Query Module Structure

`writ-lsp/src/queries/mod.rs` acts as the public surface of the `queries` submodule. It re-exports
symbols for use by `backend.rs` and any other callers. The convention is: re-export only what
external callers need.

`collect_dialogue_speaker_tokens` is defined in `semantic.rs` and called only within `semantic.rs`
(inside `collect_semantic_tokens`). The re-export on line 41 was added when the function was public
but has never been used from `backend.rs` or any other file outside `semantic.rs`.

Removal is safe: only two files contain the symbol name, and both are in `writ-lsp/src/queries/`.

### Spec Documentation Pattern

The modules spec (`25_24_modules_namespaces.md`) documents `using Enum::*;` glob imports in
§1.24.4.4. The section lists four rules. The `using log::*;` limitation (errors with E0003
UnresolvedName because `log` is not an enum type — it is a namespace/module alias for inbuilt
functions) should appear as an additional clarifying note in §1.24.4.4.

The correct location is the **Rules** block or a **Note** immediately after it — consistent with
the existing note about `using Option::*;` in the same section.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Verifying compilation | Ad-hoc grep check | `cargo build --workspace` | Catches all crate-level breakage including transitive |
| Blessing golden output | Manual file write | `BLESS=1 cargo test -p writ-golden test_fn_optional` | Uses the harness's own bless path for consistency |

## Common Pitfalls

### Pitfall 1: Removing the re-export breaks compilation
**What goes wrong:** If any file outside `semantic.rs` imports `collect_dialogue_speaker_tokens`
via the re-export, removing it causes a compile error.
**Why it happens:** The re-export was public and any file that `use`s it directly would break.
**How to avoid:** Confirmed by `grep` — only `mod.rs` (re-export line) and `semantic.rs` (definition
and call site) contain the symbol. No external callers exist.
**Warning signs:** `cargo build --workspace` after the deletion returns an `unresolved import` error.

### Pitfall 2: Over-engineering the `using log::*;` note
**What goes wrong:** Adding a full section or example instead of a brief clarifying sentence.
**Why it happens:** The existing §1.24.4.4 is already the right location; the fix is additive prose.
**How to avoid:** Add one to two sentences explaining that `using log::*;` produces E0003 because
`log` is a namespace alias for inbuilt calls, not an enum — and reference the inbuilt calls section.

### Pitfall 3: Re-blessing fn_optional when it is already correct
**What goes wrong:** Running `BLESS=1` when the snapshot is current causes a no-op diff but can
accidentally overwrite if the compiler state has changed.
**How to avoid:** Run the test without `BLESS=1` first. If it passes, the snapshot is correct.

### Pitfall 4: Treating SPEC-01 / TEST-01 as unresolved
**What goes wrong:** Writing implementation tasks to add TOC entries or register the test when
both are already in place.
**How to avoid:** Read the current files before writing tasks. The plan must verify then close.

## Code Examples

### Removing the orphaned re-export (LSP-02)

Current state of `writ-lsp/src/queries/mod.rs` lines 40-42:

```rust
pub use semantic::collect_semantic_tokens;
pub use semantic::collect_dialogue_speaker_tokens;  // <- DELETE this line
pub use semantic::RawSemanticToken;
```

After deletion:

```rust
pub use semantic::collect_semantic_tokens;
pub use semantic::RawSemanticToken;
```

Verification command:

```bash
cargo build --workspace
```

Expected: `Finished` with no errors.

### Adding the `using log::*;` limitation note (SPEC-02)

Target: `language-spec/spec/25_24_modules_namespaces.md`, section `### 1.24.4.4 Glob Enum Imports and Sub-Prelude Builtins`.

The existing **Rules** block ends with rule 4 at line ~187. After rule 4 (or as a new rule 5), add:

```markdown
5. `using log::*;` is a compile error — `log` is a namespace alias for inbuilt functions (see
   §1.27.4), not an enum type. Only enum types support the `::*` glob form. Attempting to glob-import
   a non-enum name produces **E0003 UnresolvedName** with a clear diagnostic.
```

Alternatively, add it as a standalone note block after the rules:

```markdown
> **Note:** `using log::*;` is invalid — `log` is not an enum but a namespace alias for inbuilt
> functions. Using `::*` on a non-enum produces **E0003 UnresolvedName**. Use `::log` or
> `log::info(...)` directly instead.
```

Either form is acceptable; the note block is preferred for readability.

### Verifying §1.26.4 in TOC (SPEC-01)

The entry already exists at lines 197-198 of `language-spec/spec/01_table_of_contents.md`:

```markdown
    * [1.26.4 Compiler Tooling](#1264-compiler-tooling)
      * [1.26.4.1 Incremental Export](#12641-incremental-export)
```

No edit required. The plan task is a `Read` + verify step only.

### Verifying test_fn_optional (TEST-01)

The test already exists and passes:

```
test test_fn_optional ... ok
```

Both `fn_optional.writ` and `fn_optional.writil` exist under `writ-golden/tests/golden/`. No edit
required. The plan task is `cargo test -p writ-golden test_fn_optional` and confirm `ok`.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`) |
| Config file | none — standard Cargo test integration |
| Quick run command | `cargo test -p writ-golden test_fn_optional` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SPEC-01 | §1.26.4 linked entry present in TOC | manual verification | read `01_table_of_contents.md` and grep for `1264` | N/A |
| SPEC-02 | `using log::*;` limitation documented in §1.24.4.4 | manual verification | grep `using log` in modules spec | N/A |
| TEST-01 | `test_fn_optional` runs and passes | unit | `cargo test -p writ-golden test_fn_optional` | Yes |
| LSP-02 | Re-export absent; workspace compiles | build | `cargo build --workspace` | N/A |

### Sampling Rate

- **Per task commit:** `cargo build --workspace`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

None — existing test infrastructure covers all phase requirements.

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — Rust/Cargo toolchain already verified functional, all changes
are in-tree files only).

## Open Questions

1. **Are SPEC-01 and TEST-01 truly already done?**
   - What we know: Files exist, tests pass, TOC entry is present.
   - What's unclear: Whether the REQUIREMENTS.md tracking was updated or still shows them as pending.
   - Recommendation: The plan should include a fast verification step to confirm and then immediately
     mark them done; don't write implementation tasks for work that's complete.

2. **Where exactly in §1.24.4.4 should the `using log::*;` note go?**
   - What we know: The section has 4 rules and ends with a `None`/`Some` sub-prelude note.
   - What's unclear: Whether to extend the rules list (rule 5) or add a separate note block.
   - Recommendation: Note block after the rules is more readable and consistent with the existing
     `None`/`Some` explanation format.

## Sources

### Primary (HIGH confidence)

- Direct file reads of `language-spec/spec/01_table_of_contents.md` — §1.26.4 entry confirmed
- Direct file reads of `writ-golden/tests/golden_tests.rs` — `test_fn_optional` confirmed with `#[test]`
- Direct file read of `writ-golden/tests/golden/fn_optional.writil` — blessed snapshot confirmed present
- Direct file read of `writ-lsp/src/queries/mod.rs` — orphaned re-export at line 41 confirmed
- `cargo test -p writ-golden test_fn_optional` — test passes (1 passed, 0 failed)
- `cargo build --workspace` — workspace compiles cleanly
- `grep -r collect_dialogue_speaker_tokens writ-lsp/src/` — only 2 files, no external callers
- Git log — `test_fn_optional` added in `0ce1cbf`, §1.26.4 TOC added in `094c2e3`

### Secondary (MEDIUM confidence)

- `.planning/milestones/v3.2-MILESTONE-AUDIT.md` — original identification of the 4 tech debt items
- `.planning/PROJECT.md` lines 260-270 — stale issue list (pre-dates Phase 43/47 fixes)

## Metadata

**Confidence breakdown:**
- Current state assessment: HIGH — verified by direct file reads and test execution
- SPEC-02 edit location: HIGH — §1.24.4.4 is the correct and only section describing `::*` glob imports
- LSP-02 safety: HIGH — grep confirms zero external callers
- SPEC-01 / TEST-01 as already-done: HIGH — git log + test execution confirms

**Research date:** 2026-03-29
**Valid until:** 2026-04-28 (stable — no fast-moving dependencies)
