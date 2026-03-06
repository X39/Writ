---
phase: 60
slug: lsp-query-robustness
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 60 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` (cargo test) |
| **Config file** | none — standard Cargo workspace |
| **Quick run command** | `cargo test -p writ-lsp --lib 2>&1 \| tail -20` |
| **Full suite command** | `cargo test -p writ-lsp -p writ-compiler 2>&1 \| tail -30` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p writ-lsp --lib 2>&1 | tail -20`
- **After every plan wave:** Run `cargo test -p writ-lsp -p writ-compiler 2>&1 | tail -30`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 60-01-01 | 01 | 1 | LSP-04 | unit | `cargo test -p writ-lsp binding_at_offset --lib -- --nocapture` | ❌ W0 | ⬜ pending |
| 60-01-02 | 01 | 1 | LSP-04 | unit | `cargo test -p writ-lsp binding_at_offset_fn_param --lib -- --nocapture` | ❌ W0 | ⬜ pending |
| 60-01-03 | 01 | 1 | LSP-05 | unit | `cargo test -p writ-lsp type_ann_def_id_at_offset --lib -- --nocapture` | ❌ W0 | ⬜ pending |
| 60-01-04 | 01 | 1 | LSP-06 | unit | `cargo test -p writ-lsp def_at_offset_declaration --lib -- --nocapture` | ❌ W0 | ⬜ pending |
| 60-01-05 | 01 | 1 | LSP-04 | integration | `cargo test -p writ-lsp hover_let_binding --lib -- --nocapture` | ❌ W0 | ⬜ pending |
| 60-01-06 | 01 | 1 | LSP-05 | integration | `cargo test -p writ-lsp goto_def_type_annotation --lib -- --nocapture` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Tests for `binding_at_offset` — stubs for LSP-04 (let binding hover)
- [ ] Tests for `binding_at_offset` with fn params — stubs for LSP-04 (param hover)
- [ ] Tests for type annotation DefId inspection — stubs for LSP-05
- [ ] Tests for `def_at_offset` — stubs for LSP-06 (find-refs from declaration site)
- [ ] Integration test: full hover pipeline on `let x: int = 42` returns `"x: int"`
- [ ] Integration test: goto-def on `MyStruct` in `let x: MyStruct = ...` jumps to struct def

*Existing infrastructure covers test framework — only new test files needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| VSCode hover tooltip display | LSP-04 | UI rendering | Open `.writ` file, hover over `let x: int = 42`, verify tooltip shows `x: int` |
| VSCode goto-def navigation | LSP-05 | Editor navigation | Right-click type annotation, select Go to Definition, verify cursor jumps |
| VSCode find-refs from decl | LSP-06 | Editor UI | Right-click function declaration name, select Find All References |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
