---
phase: 46
slug: structs-as-value-types-design-discussion
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 46 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | N/A — documentation-only phase |
| **Config file** | none |
| **Quick run command** | `cargo test --workspace` (confirm no Rust tests broken) |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~60 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace` (only to verify no accidental Rust changes)
- **After every plan wave:** Human review of written documents
- **Before `/gsd:verify-work`:** Full suite must be green + human review of spec sections
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 46-01-01 | 01 | 1 | DES-01 | manual | human review of spec amendment section | ❌ W0 | ⬜ pending |
| 46-01-02 | 01 | 1 | DES-01 | manual | human review of GC/IL/VM coverage | ❌ W0 | ⬜ pending |
| 46-01-03 | 01 | 1 | DES-01 | manual | human review of MILESTONES.md | ❌ W0 | ⬜ pending |
| 46-01-04 | 01 | 1 | DES-01 | automated | `git diff --name-only HEAD \| grep '\.rs$'` — must be empty | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements. No test framework changes needed — this phase produces documentation only.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Spec amendment section exists with YES decision | DES-01 | Content quality review | Check §8.4 exists in spec with decision: YES |
| Decision record covers GC tracing | DES-01 | Content completeness | Verify GC section addresses ref fields in value structs |
| Decision record covers IL encoding changes | DES-01 | Content completeness | Verify TypeDef.kind, NEW, MOV, BOX/UNBOX changes listed |
| Decision record covers VM representation | DES-01 | Content completeness | Verify register model changes documented |
| Decision record covers v4.0 scope | DES-01 | Content completeness | Verify estimated scope with phase categories |
| v4.0 milestone entry in MILESTONES.md | DES-01 | File creation review | Check MILESTONES.md has v4.0 entry with scope boundary |
| No Rust source modified | DES-01 | Automated check | `git diff --name-only HEAD \| grep '\.rs$'` must be empty |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
