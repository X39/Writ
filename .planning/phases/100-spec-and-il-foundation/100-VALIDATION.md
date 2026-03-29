---
phase: 100
slug: spec-and-il-foundation
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-28
---

# Phase 100 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Manual review (pure documentation phase — no code changes) |
| **Config file** | none |
| **Quick run command** | `grep -c "## " language-spec/spec/29_28_reflection.md` |
| **Full suite command** | `grep -l "TypeOf\|Reflectable\|FieldInfo" language-spec/spec/*.md` |
| **Estimated runtime** | ~1 second |

---

## Sampling Rate

- **After every task commit:** Verify target spec file contains expected section headers
- **After every plan wave:** Verify all 8 SPEC requirements have corresponding spec content
- **Before `/gsd:verify-work`:** All spec files must exist with expected sections
- **Max feedback latency:** 1 second

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 100-01-01 | 01 | 1 | SPEC-01 | grep | `grep "Type\|FieldInfo\|MethodInfo" language-spec/spec/29_28_reflection.md` | N/A | ⬜ pending |
| 100-01-02 | 01 | 1 | SPEC-02 | grep | `grep "typeof\|get_type" language-spec/spec/29_28_reflection.md` | N/A | ⬜ pending |
| 100-01-03 | 01 | 1 | SPEC-03 | grep | `grep "Reflectable" language-spec/spec/29_28_reflection.md` | N/A | ⬜ pending |
| 100-01-04 | 01 | 1 | SPEC-04 | grep | `grep "invoke\|FieldInfo.set" language-spec/spec/29_28_reflection.md` | N/A | ⬜ pending |
| 100-02-01 | 02 | 1 | SPEC-05 | grep | `grep "TypeOf\|TYPEOF" language-spec/spec/67_4_2_opcode_assignment_table.md` | N/A | ⬜ pending |
| 100-02-02 | 02 | 1 | SPEC-06 | grep | `grep "format_version.*4\|version.*4" language-spec/spec/33_2_4_binary_format.md` | N/A | ⬜ pending |
| 100-02-03 | 02 | 1 | SPEC-07 | grep | `grep "BOX\|UNBOX\|coercion" language-spec/spec/29_28_reflection.md` | N/A | ⬜ pending |
| 100-02-04 | 02 | 1 | SPEC-08 | grep | `grep "type_args\|generic.*reflect" language-spec/spec/29_28_reflection.md` | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. This is a pure documentation phase — no test framework needed.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Spec readability | SPEC-01 | Prose quality check | Read §1.28 Reflection and verify it follows existing spec section style |
| Cross-reference integrity | ALL | Inter-section links | Verify §1.28 references match §2.18, §4.2 updates |

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 1s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
