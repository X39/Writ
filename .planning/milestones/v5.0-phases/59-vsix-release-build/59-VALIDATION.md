---
phase: 59
slug: vsix-release-build
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-16
---

# Phase 59 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | bash smoke-test.sh (custom validation script) |
| **Config file** | writ-vscode/scripts/smoke-test.sh |
| **Quick run command** | `bash writ-vscode/scripts/smoke-test.sh` |
| **Full suite command** | `bash writ-vscode/scripts/smoke-test.sh` |
| **Estimated runtime** | ~5 seconds (after build) |

---

## Sampling Rate

- **After every task commit:** Run `bash writ-vscode/scripts/smoke-test.sh`
- **After every plan wave:** Run `bash writ-vscode/scripts/smoke-test.sh`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 59-01-01 | 01 | 1 | operational | integration | `bash writ-vscode/scripts/smoke-test.sh` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. smoke-test.sh, copy-bins.js, and npm scripts are all in place from Phase 57.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Release binary size reasonable | operational | No automated size threshold defined | Check writ-lsp.exe and writ-dap.exe are < 50MB each |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
