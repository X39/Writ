---
phase: 106
slug: read-only-introspection-integration-tests-and-lsp
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-28
---

# Phase 106 — Validation Strategy

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust) |
| **Quick run command** | `cargo test -p writ-runtime -p writ-golden --lib 2>&1` |
| **Full suite command** | `cargo test -p writ-runtime -p writ-golden -p writ-lsp 2>&1` |
| **Estimated runtime** | ~30 seconds |

## Validation Sign-Off

- [x] All tasks have automated verify
- [x] `nyquist_compliant: true` set

**Approval:** pending
