# Phase 99: LSP Integration, Golden Test Sweep, and Spec Update - Research

**Researched:** 2026-03-28
**Domain:** LSP E2E testing, golden file maintenance, language spec authoring
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Use the existing `test_protocol.rs` LspClient infrastructure (in-memory duplex streams, JSON-RPC framing)
- Deprecated test: open a `.writ` source with `[Deprecated("msg")]` usage, assert publishDiagnostics contains Warning severity with the message string
- Speaker validation test: open a `.writ` source with `@speaker` targeting a non-Singleton entity, assert publishDiagnostics contains E0007
- Test fixtures can be inline strings (no separate fixture files needed)
- Run `cargo insta test --review` to identify any pending snapshot changes from phases 93-98
- Bless all correct snapshots and commit
- Verify `cargo test` passes clean with no pending review items
- Add a new spec section covering: attribute argument blob encoding format, `attribute Name(params);` declaration syntax, builtin attribute semantic effects, and the three runtime query method signatures
- Place in the existing `language-spec/spec/` splatted files, numbered appropriately after existing sections

### Claude's Discretion
All implementation details at Claude's discretion — infrastructure/testing phase with clear success criteria from ROADMAP.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TOOL-01 | LSP E2E tests cover attribute diagnostics (deprecated warnings, speaker validation errors) | See Critical Constraint section — deprecated warning requires temp-dir project mode; speaker validation works with inline source |
| TOOL-02 | Golden files are updated/reblessed for attribute pipeline changes | `cargo test -p writ-golden` already passes 48/48; sweep confirms no pending failures |
| TOOL-03 | Language spec is updated to document attribute argument encoding, user-defined attributes, and runtime query API | New file at `language-spec/spec/19_17a_attributes_extended.md` or numbered inline after `18_17_attributes.md` |
</phase_requirements>

---

## Summary

Phase 99 is a pure integration and documentation phase. All compiler features for attributes (phases 93-98) are already implemented. This phase adds regression-safe E2E LSP tests, ensures all golden snapshots are accurate, and updates the language spec with the new attribute system documentation.

The most important finding is a **critical constraint** on the deprecated warning LSP test. The `analyze_standalone` path (used when no `writ.toml` exists) assigns all declarations and references the same `FileId(0)`. W0006 is explicitly suppressed for same-file references by design. A single-inline-source LSP test will therefore NOT produce a W0006 publishDiagnostics warning. The test must use the `analyze_project` path, which requires a temporary directory with a `writ.toml` and two `.writ` files on disk, or it must test hover deprecation instead of publishDiagnostics.

The golden file sweep is straightforward: `cargo test -p writ-golden` currently passes all 48 tests with no failures. The new `.writ` files from phases 93-98 that have `.writil` counterparts are already passing. The sweep task is to confirm no regressions were introduced and run `BLESS=1` only if any are found.

**Primary recommendation:** For the deprecated W0006 LSP test, use a temp-dir with `writ.toml` and two `.writ` files so the backend enters project mode and assigns distinct `FileId`s, triggering the cross-file W0006 guard.

---

## Critical Constraint: W0006 Cross-File Guard

This is the most important finding for TOOL-01 planning.

### How W0006 is gated

W0006 (deprecated item warning) is emitted by the type-checker only when:
```
entry.file_id != ctx.current_file
```

This guard exists in three places:
- `writ-compiler/src/check/check_expr/call.rs` — function calls
- `writ-compiler/src/check/check_expr/ident.rs` — function-as-value references
- `writ-compiler/src/check/check_expr/construction.rs` — `new T { }` construction

### Why single-file LSP tests cannot trigger W0006

`AnalysisHost::analyze_standalone` (used when no `writ.toml` is in the workspace root):
- Assigns `FileId(0)` to both the definition and the reference
- `entry.file_id == ctx.current_file` (both are `FileId(0)`)
- W0006 is suppressed — no publishDiagnostics warning emitted

### Existing precedent in test_protocol.rs

The file already contains this documented comment at line 746-750:
```
// Note: W0006 diagnostic squiggles are integration-tested at the compiler level in
// writ-compiler/tests/deprecated_tests.rs. The LSP diagnostic pipeline routes
// W0006 through Severity::Warning → DiagnosticSeverity::WARNING automatically
// (see writ-lsp/src/convert.rs). Single-file LSP tests do not trigger W0006
// because same-file deprecation references are suppressed by design.
```

The existing LSP tests for deprecated (`test_deprecated_hover_on_declaration`, `test_deprecated_hover_on_call_site`) test hover tooltips, not publishDiagnostics.

### Solution options for the deprecated LSP test

**Option A (Recommended): Temp-dir project mode**
Create a temp directory with:
- `writ.toml` (source dirs configured)
- `src/lib.writ` — `[Deprecated("msg")] pub fn foo() {}`
- `src/main.writ` — `pub fn main() { foo(); }`

Then initialize the LSP with `rootUri` pointing to the temp dir. The backend enters `analyze_project` mode, assigns `FileId(0)` to `lib.writ` and `FileId(1)` to `main.writ`, so the cross-file guard fires and W0006 is published.

**Option B: Test hover W0006 content instead**
The existing hover tests already cover this. The `deprecation_notice()` function shows deprecation regardless of same-file. However, this does not match the CONTEXT.md requirement "assert publishDiagnostics contains Warning severity."

**Option C: Restructure the source so deprecated fn is in a prelude string passed to multi-file API**
Not feasible without changing the backend.

Option A is the only path to meeting the literal CONTEXT.md requirement (publishDiagnostics + Warning severity). The planner should use Option A.

---

## Standard Stack

### Core (all existing, no new dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tokio` | existing | async runtime for LSP tests | already in `writ-lsp` dev-deps |
| `tower-lsp` | existing | LSP server framework | project standard |
| `serde_json` | existing | JSON-RPC message encoding | project standard |
| `writ-lsp` | workspace | LSP backend under test | the crate being tested |

### Test Infrastructure

| Utility | Location | Purpose |
|---------|----------|---------|
| `LspClient` | `writ-lsp/tests/test_protocol.rs` | In-memory LSP wire-protocol client |
| `open_document_and_collect_diagnostics()` | `LspClient` method | Sends didOpen, waits 3s, drains publishDiagnostics |
| `drain_notifications()` | `LspClient` method | Collects all notifications within timeout |
| `encode_lsp()` / `read_lsp()` | helpers | Content-Length framing |
| `run_golden_test()` | `writ-golden/tests/golden_tests.rs` | Compile + disassemble + compare |
| `BLESS=1 cargo test -p writ-golden` | env-var pattern | Rebless golden snapshot files |

---

## Architecture Patterns

### LSP E2E Test Pattern (existing)

All tests use `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`:

```rust
// Source: writ-lsp/tests/test_protocol.rs (existing pattern)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_diagnostics_invalid_source() {
    let mut client = LspClient::start().await;

    let invalid_source = r#"pub fn main() {
    let x: int = "hello";
}"#;
    let error_uri = "file:///test/invalid.writ";

    let diag_notifications = client
        .open_document_and_collect_diagnostics(error_uri, invalid_source)
        .await;

    // filter notifications by URI, then check severity
    let mut all_diagnostics: Vec<Value> = Vec::new();
    for notif in &diag_notifications {
        let params = notif.get("params").cloned().unwrap_or(json!({}));
        let notif_uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
        if notif_uri == error_uri {
            if let Some(diags) = params.get("diagnostics").and_then(|v| v.as_array()) {
                all_diagnostics.extend(diags.iter().cloned());
            }
        }
    }
    // LSP severity: 1=Error, 2=Warning, 3=Information, 4=Hint
    let has_error = all_diagnostics.iter().any(|d| {
        d.get("severity").and_then(|v| v.as_i64()).map(|s| s == 1).unwrap_or(false)
    });
}
```

### LSP Severity Mapping (from convert.rs)

```rust
// Source: writ-lsp/src/convert.rs
Severity::Error   => DiagnosticSeverity::ERROR      // LSP severity = 1
Severity::Warning => DiagnosticSeverity::WARNING    // LSP severity = 2
Severity::Note    => DiagnosticSeverity::INFORMATION // LSP severity = 3
```

### Diagnostic Code Pattern

Diagnostics carry a `code` field (string) set to the error/warning code:
- W0006 for deprecated warnings
- E0007 for invalid speaker
- E0003 for non-existent entity

### Project-Mode LSP Test Pattern (temp-dir approach for W0006)

```rust
// Pattern for triggering analyze_project (project mode)
use std::fs;

let tmp = std::env::temp_dir().join("writ_lsp_test_XXXX");
fs::create_dir_all(&tmp.join("src")).unwrap();
fs::write(&tmp.join("writ.toml"), r#"
[project]
name = "test"

[[sources]]
dir = "src"
"#).unwrap();
fs::write(&tmp.join("src/lib.writ"), "[Deprecated(\"use bar instead\")]\npub fn foo() {}\n").unwrap();
fs::write(&tmp.join("src/main.writ"), "pub fn main() { foo(); }\n").unwrap();

let tmp_uri = Url::from_file_path(&tmp).unwrap().to_string();
// initialize with rootUri pointing to tmp
// then open src/main.writ — backend will use analyze_project
// cleanup tmp dir after test
```

### E0007 Speaker Validation Test Pattern (inline, no temp-dir needed)

E0007 is a resolve-stage error (not a cross-file type-check warning). It fires in `analyze_standalone` because it checks the attribute table (Singleton), not file boundaries:

```rust
// Inline source works for E0007
let source = r#"entity Npc {}

dlg greet() {
    @Npc say("hello");
}
"#;
// This produces E0007: invalid speaker `Npc` (entity is not [Singleton])
```

### Golden Test Bless Pattern

```bash
# Check current state (should pass)
cargo test -p writ-golden

# Rebless if any snapshot changed (phases 93-98 may have altered IL output)
BLESS=1 cargo test -p writ-golden

# Verify after blessing
cargo test -p writ-golden
```

No `cargo insta` is used — the project uses a custom bless pattern with `BLESS=1` env var and `similar` crate for diffs, NOT `insta`. The CONTEXT.md mentions `cargo insta test --review` but that is incorrect for this project's actual test infrastructure.

### Spec File Naming Pattern

Existing splatted spec files:
- `18_17_attributes.md` — Section 1.17 (already documents syntax, builtin attrs, conditional compilation)
- Next section number in the language spec (1.17 is already taken, next available: `19_17a_` or follow the module pattern)

Looking at the existing file listing, `18_17_attributes.md` is the last "language spec" file before IL spec files. The new spec content (attribute argument encoding, user-defined attributes, runtime query API) logically extends Section 1.17. Options:
1. Append content directly to `18_17_attributes.md` (Section 1.17.5+)
2. Create a new file `19_17a_attributes_extended.md` or renumber

Given the spec is splatted by numerical prefix for ordering, adding a subsection directly to `18_17_attributes.md` is cleanest and avoids renumbering `19_` through `30_`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| LSP wire protocol | Custom parser | Existing `encode_lsp()` / `read_lsp()` in test_protocol.rs | Already handles Content-Length framing |
| Diagnostic collection | New notification listener | `open_document_and_collect_diagnostics()` | Handles timing (3s wait) and filtering |
| Snapshot diffing | Custom diff | `similar` crate via `run_golden_test()` | Already implemented, shows unified diffs |
| Multi-file project analysis | Multiple `didOpen` calls | `analyze_project` via temp-dir with `writ.toml` | Backend switches modes based on workspace root |

---

## Current Golden Test State

**Confirmed (2026-03-28):** `cargo test -p writ-golden` passes 48/48 tests.

The golden sweep for TOOL-02 is therefore a verification-only task:
1. Run `cargo test -p writ-golden` — confirm still 48/48 passing
2. If any fail, run `BLESS=1 cargo test -p writ-golden` to rebless
3. Commit any updated `.writil` files

New golden files added by phases 93-98 (already have `.writ` and `.writil` counterparts confirmed passing):
- `conditional_active.writ` / `.writil`
- `conditional_inactive.writ` / `.writil`
- `entity_get_or_create.writ` / `.writil`
- `expr_string_escapes.writ` / `.writil`
- `fn_overload.writ` / `.writil`

Files that exist without `.writil` (intentional — these are error-case tests):
- `crash_unwrap_none.writ` (runtime crash test — no IL golden needed)
- `dap_bp_align.writ` (DAP test fixture — no IL golden needed)
- `documented_functions.writ` (hover test fixture — no IL golden needed)
- `type_recursive_struct.writ` (compile error test — IL never produced)

---

## Common Pitfalls

### Pitfall 1: Assuming W0006 fires in single-file LSP tests
**What goes wrong:** Test opens a single inline source containing both `[Deprecated]` declaration and usage, expects a publishDiagnostics Warning, gets none.
**Why it happens:** W0006 has an explicit cross-file guard (`entry.file_id != ctx.current_file`). Single-file analysis assigns `FileId(0)` to everything.
**How to avoid:** Use temp-dir project mode for the deprecated test, or test hover content instead.
**Warning signs:** `all_diagnostics` is empty after opening the document; no publishDiagnostics notification arrives.

### Pitfall 2: Using `cargo insta` instead of the BLESS pattern
**What goes wrong:** CONTEXT.md mentions `cargo insta test --review` but this project does NOT use the `insta` crate for golden tests. It uses a custom `BLESS=1` environment variable with the `similar` crate.
**Why it happens:** CONTEXT.md is generated without deep code inspection.
**How to avoid:** Use `BLESS=1 cargo test -p writ-golden` exclusively. Check `Cargo.toml` for the `writ-golden` crate — `similar` is the only snapshot crate present.

### Pitfall 3: Spec section numbering collision
**What goes wrong:** Creating a new file numbered `19_` conflicts with the existing `19_18_operators_overloading.md`.
**Why it happens:** The `18_17_attributes.md` file is followed by `19_18_operators_overloading.md` — sequential numbering is the naming scheme.
**How to avoid:** Append new subsections to `18_17_attributes.md` directly (adding 1.17.5, 1.17.6, 1.17.7) rather than creating a new file.

### Pitfall 4: LSP test timing
**What goes wrong:** `drain_notifications(200)` returns before the server has finished analysis, collecting zero diagnostics.
**Why it happens:** Analysis is async; `open_document_and_collect_diagnostics` already waits 3 seconds but new tests might not use this helper.
**How to avoid:** Always use `open_document_and_collect_diagnostics()` not `open_document()` when testing diagnostics.

### Pitfall 5: E0007 source construction
**What goes wrong:** Speaker validation test doesn't actually use an entity with `@speaker` syntax, so E0007 is never produced.
**Why it happens:** The `@speaker` syntax in a `dlg` block is required; standalone `fn` blocks do not have speakers.
**How to avoid:** Use a `dlg` block, not a `fn` block. The `@EntityName say(...)` syntax is what triggers speaker validation.

---

## Code Examples

### W0006 Diagnostic Code Check

```rust
// Source: writ-lsp/src/convert.rs (verified)
// LSP diagnostic code field is set from writ_diagnostics code string
// W0006 appears as: d.get("code") == Some(&json!("W0006"))
let has_w0006 = all_diagnostics.iter().any(|d| {
    d.get("code").and_then(|v| v.as_str()) == Some("W0006")
});
```

### E0007 Source That Triggers Speaker Validation

```writ
// This source produces E0007 because Npc is not [Singleton]
entity Npc {}

dlg greet() {
    @Npc say("hello");
}
```

Note: `say` is a dialogue builtin. The `@Npc` speaker reference on a non-Singleton entity triggers E0007 at the resolve stage. This DOES work in single-file analysis (no cross-file guard for E0007).

### Attribute Argument Blob Encoding (for spec)

From `writ-module/src/attr.rs` (verified source):

```
ATTR_TAG_STRING (0x01): u32(byte_len, LE) + UTF-8 bytes
ATTR_TAG_INT    (0x02): i64 (little-endian, 8 bytes)
ATTR_TAG_BOOL   (0x03): u8 (0x00 = false, any other = true)
ATTR_TAG_NAMED  (0x04): u32(name_byte_len, LE) + name_bytes + [inner arg encoding]
```

Multi-argument blobs are sequential concatenations. Empty argument list = empty vec (null blob at offset 0).

### User-Defined Attribute Declaration Syntax

```writ
// attribute Name(typed-params);
attribute MinLevel(level: int);
attribute Tag(name: string);
attribute Debug();

// Usage
[MinLevel(5)]
fn advancedMove() { ... }
```

### Runtime Query API Signatures (from writ-runtime/src/domain.rs)

```rust
// Query all attributes by name across all loaded modules
pub fn query_attributes(&self, attr_name: &str) -> Vec<DomainAttributeMatch>

// Query all attributes on a specific typedef (by module + typedef index)
pub fn query_attributes_on(&self, module_idx: usize, typedef_idx: usize) -> Vec<DomainAttributeMatch>

// Query decoded args for a named attribute on a specific owner token
pub fn query_attribute_value(
    &self,
    module_idx: usize,
    owner_token: MetadataToken,
    attr_name: &str,
) -> Option<Vec<AttrValue>>

// DomainAttributeMatch fields:
pub struct DomainAttributeMatch {
    pub module_idx: usize,
    pub name: String,
    pub args: Vec<AttrValue>,    // decoded; empty for no-arg attributes
    pub owner: MetadataToken,    // the definition this attribute is on
    pub owner_kind: u8,          // 0=type, 1=method, 2=other
}
```

### Pre-Load Callback (from writ-runtime/src/host.rs)

```rust
// RuntimeHost trait method
fn on_module_load(&mut self, _view: &ModuleAttributeView<'_>) -> Result<(), String> {
    Ok(()) // Ok = allow, Err = reject module load
}

// ModuleAttributeView provides same query methods as Domain, scoped to one module
```

---

## Spec Structure Plan

The new spec content belongs in `language-spec/spec/18_17_attributes.md` as additional subsections:

**Section 1.17.5: User-Defined Attributes**
- `attribute Name(params);` declaration syntax
- Parameter types (string, int, bool)
- Builtin name reservation (E0008 on collision)
- Pipeline behavior (pass-through to AttributeDef table)

**Section 1.17.6: Attribute Argument Encoding**
- Tagged binary format in the blob heap
- Tag constants and wire layout
- Named arguments

**Section 1.17.7: Runtime Query API**
- Three query method signatures
- `DomainAttributeMatch` structure
- Pre-load callback (`on_module_load`)
- No automatic semantic effects — host must act on query results

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust / cargo | All compilation and test tasks | Yes | stable (in use) | — |
| `cargo test -p writ-golden` | TOOL-02 | Yes | passes 48/48 | — |
| `cargo test -p writ-lsp` | TOOL-01 | Yes | passes 25/25 | — |
| `std::env::temp_dir()` | W0006 LSP test (temp-dir approach) | Yes | OS standard | — |
| Language spec markdown | TOOL-03 | Yes (files exist) | — | — |

No missing dependencies. Step 2.6: No blockers.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `#[tokio::test]` |
| Config file | None (workspace Cargo.toml) |
| Quick run (LSP) | `cargo test -p writ-lsp --test test_protocol` |
| Quick run (golden) | `cargo test -p writ-golden` |
| Full suite | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TOOL-01 | W0006 warning published for deprecated usage | integration | `cargo test -p writ-lsp --test test_protocol test_deprecated_warning_published` | No — Wave 0 |
| TOOL-01 | E0007 published for non-Singleton @speaker | integration | `cargo test -p writ-lsp --test test_protocol test_speaker_validation_e0007` | No — Wave 0 |
| TOOL-02 | All golden tests pass | regression | `cargo test -p writ-golden` | Yes (48/48 pass) |
| TOOL-03 | Spec file contains required sections | manual review | — | No — Wave 0 |

### Wave 0 Gaps

- [ ] `writ-lsp/tests/test_protocol.rs` — add `test_deprecated_warning_published` (new `#[tokio::test]` fn)
- [ ] `writ-lsp/tests/test_protocol.rs` — add `test_speaker_validation_e0007` (new `#[tokio::test]` fn)
- [ ] `language-spec/spec/18_17_attributes.md` — add sections 1.17.5, 1.17.6, 1.17.7

---

## Sources

### Primary (HIGH confidence)

- `writ-lsp/tests/test_protocol.rs` — full LspClient infrastructure, existing test patterns
- `writ-lsp/src/analysis_host.rs` — `analyze_standalone` vs `analyze_project` code paths, `FileId` assignment
- `writ-lsp/src/backend.rs` — `publish_diagnostics_for`, workspace root detection, project-mode trigger
- `writ-lsp/src/convert.rs` — `severity_to_lsp` mapping (Severity::Warning → DiagnosticSeverity::WARNING = 2)
- `writ-compiler/src/check/check_expr/call.rs` — W0006 cross-file guard verified
- `writ-compiler/src/check/check_expr/ident.rs` — W0006 cross-file guard verified
- `writ-compiler/src/check/check_expr/construction.rs` — W0006 cross-file guard verified
- `writ-compiler/src/resolve/error.rs` — E0007 diagnostic construction verified
- `writ-module/src/attr.rs` — blob encoding wire format verified
- `writ-runtime/src/domain.rs` — query API signatures verified
- `writ-runtime/src/host.rs` — ModuleAttributeView and on_module_load verified
- `writ-golden/tests/golden_tests.rs` — bless pattern verified (BLESS=1, not cargo insta)
- `language-spec/spec/18_17_attributes.md` — existing attribute spec content

### Secondary (MEDIUM confidence)

- `cargo test -p writ-golden` output (run 2026-03-28): 48/48 passing — confirms no pending failures

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries are existing project dependencies
- Architecture: HIGH — directly inspected backend code paths and cross-file guard logic
- Pitfalls: HIGH — W0006 guard discovered by direct code inspection, not inference
- Spec structure: HIGH — existing spec files inspected, naming pattern confirmed

**Research date:** 2026-03-28
**Valid until:** 2026-04-28 (stable codebase, no fast-moving dependencies)
