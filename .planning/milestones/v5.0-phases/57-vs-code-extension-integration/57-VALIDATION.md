---
phase: 57
slug: vs-code-extension-integration
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-16
---

# Phase 57 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Node.js built-in assert (structural tests) + shell commands |
| **Config file** | writ-vscode/package.json |
| **Quick run command** | `node writ-vscode/tests/structural.test.js` |
| **Full suite command** | `cd writ-vscode && npx tsc -b --noEmit && node tests/structural.test.js` |
| **Estimated runtime** | ~2 seconds |

---

## Sampling Rate

- **After every task commit:** Run `node writ-vscode/tests/structural.test.js`
- **After every plan wave:** Run `cd writ-vscode && npx tsc -b --noEmit && node tests/structural.test.js`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 57-01-01 | 01 | 1 | EXT-03 | structural | `node writ-vscode/tests/structural.test.js` | ✅ | ✅ green |
| 57-01-02 | 01 | 1 | EXT-03 | structural | `node writ-vscode/tests/structural.test.js` | ✅ | ✅ green |
| 57-02-01 | 02 | 1 | EXT-04 | structural | `node writ-vscode/tests/structural.test.js` | ✅ | ✅ green |
| 57-02-02 | 02 | 1 | EXT-03 | structural | `node writ-vscode/tests/structural.test.js` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Nyquist Gap Closure (added by auditor)

| Gap ID | Requirement | Assertions Added | Result |
|--------|-------------|-----------------|--------|
| Gap 1 | EXT-03 (bundled binaries — extension.ts) | 6 | green |
| Gap 2 | EXT-03 (build pipeline — copy-bins.js + smoke-test.sh) | 8 | green |
| Gap 3 | EXT-04 (launch.json snippet — package.json) | 6 | green |

**Test file:** `writ-vscode/tests/structural.test.js`
**Run command:** `node writ-vscode/tests/structural.test.js`
**Result:** 20/20 tests passed

---

## Wave 0 Requirements

All phase requirements now covered by `writ-vscode/tests/structural.test.js`.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| LSP starts from bundled binary | EXT-03 | Requires VS Code runtime | Install VSIX, open .writ file, check Output panel for "Writ Language Server" |
| DAP starts from bundled binary | EXT-03 | Requires VS Code runtime | Set breakpoint, press F5, verify debug session starts |
| launch.json snippet appears | EXT-04 | Requires VS Code UI | Open launch.json, type "writ", verify snippet appears in picker |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** nyquist-auditor 2026-03-16
