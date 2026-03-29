# Phase 120: Array Semantics Correction - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-29
**Phase:** 120-array-semantics-correction
**Areas discussed:** Opcode strategy, Resize semantics, Copy signature, Spec wording

---

## Opcode Strategy

### Q1: How to handle removed opcodes (0x0905-0x0909)?

| Option | Description | Selected |
|--------|-------------|----------|
| Repurpose slots | Reassign freed slots for new opcodes. Bump format_version to 5. | |
| New slots only | Keep old slots as reserved/illegal. Add new opcodes at 0x090A+. | |
| In-place rename | Reuse exact slots with new names. | |

**User's choice:** Breaking change — treat old opcodes as if they never existed. No reserved slots, no deprecation errors.
**Notes:** Clean break philosophy. Old .writc compilations are ignored entirely.

### Q2: Format version bump?

| Option | Description | Selected |
|--------|-------------|----------|
| Bump to 5 | Clean signal of incompatibility. Reader rejects version<5. | ✓ |
| Keep at 4 | Avoid version churn. Risk of stale module confusion. | |

**User's choice:** Bump to 5

### Q3: Old opcode slot handling?

| Option | Description | Selected |
|--------|-------------|----------|
| Reserved (crash) | Standard decode error on unknown opcode. | |
| Named error | Specific 'removed in v5' error message. | |

**User's choice:** (Other) Breaking change — opcodes never existed. No special handling.

### Q4: New opcode placement?

| Option | Description | Selected |
|--------|-------------|----------|
| 0x0905 + 0x0906 | Reuse first freed slots. Compact, no gaps. | ✓ |
| 0x090A + 0x090B | Skip past old slots. Clearer separation. | |

**User's choice:** 0x0905 + 0x0906 (compact)

### Q5: ARRAY_SLICE slot?

| Option | Description | Selected |
|--------|-------------|----------|
| Compact to 0x0907 | Move from 0x0908 for contiguous block. | ✓ |
| Keep at 0x0908 | Avoid touching working code. | |

**User's choice:** Compact to 0x0907

---

## Resize Semantics

### Q1: Fill value for new slots on grow?

| Option | Description | Selected |
|--------|-------------|----------|
| Type default | int→0, string→"", bool→false, float→0.0, refs→null | ✓ |
| Undefined (crash on read) | Uninitialized slots crash when read | |
| User-supplied fill value | Two-arg resize(n, fill) | |

**User's choice:** Type default

### Q2: Shrink behavior?

| Option | Description | Selected |
|--------|-------------|----------|
| Silent truncate | Elements beyond n dropped. GC reclaims. | ✓ |
| Crash on shrink | Runtime error if n < current len. | |

**User's choice:** Silent truncate

### Q3: Edge cases (resize(0), negative)?

| Option | Description | Selected |
|--------|-------------|----------|
| resize(0) = empty, negative crashes | Consistent with 'n is the new length' | ✓ |
| Both crash | More restrictive | |
| Clamp negatives to 0 | Lenient but hides bugs | |

**User's choice:** resize(0) = empty, negative crashes

---

## Copy Signature

### Q1: Receiver — destination or source?

| Option | Description | Selected |
|--------|-------------|----------|
| Destination receives | dst.copy(dst_idx, src, src_idx, len) | |
| Source receives | src.copy_to(dst, dst_idx, src_idx, len) | |
| Static function | Array.copy(dst, dst_idx, src, src_idx, len) | |

**User's choice:** (Other) Claude's discretion, but direction semantics must be unambiguous. User noted `src.copy_to(...)` reads clearly because "copy from src to dst" is obvious.

### Q2: Bounds checking?

| Option | Description | Selected |
|--------|-------------|----------|
| Crash on out-of-bounds | Consistent with ARRAY_LOAD/STORE. | ✓ |
| Clamp to available | Silently copy fewer elements. | |

**User's choice:** Crash on out-of-bounds

### Q3: Overlapping regions (self-copy)?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes, memmove semantics | Correct overlap handling. Enables shift-in-place. | ✓ |
| No, crash on self-copy | Simpler implementation. | |
| Undefined behavior | Allow but don't guarantee correctness. | |

**User's choice:** Yes, memmove semantics

---

## Spec Wording

### Q1: How to describe fixed-size arrays?

| Option | Description | Selected |
|--------|-------------|----------|
| Fixed-size with explicit resize | "fixed-size...can only be changed via resize(n)" | |
| Strictly fixed-size | "length is immutable" — contradicts resize | |
| Allocation-explicit | "explicit allocation. Size changes require reallocation" | ✓ |

**User's choice:** Allocation-explicit

### Q2: NEW_ARRAY behavior?

| Option | Description | Selected |
|--------|-------------|----------|
| Keep zero-length | NEW_ARRAY creates len=0 array. | |

**User's choice:** (Other) Add new opcodes for non-empty array creation. Keep NEW_ARRAY for empty arrays, update name for clarity.

### Q3: Sized array opcode — length only or length + fill?

| Option | Description | Selected |
|--------|-------------|----------|
| Length only + defaults | NEW_ARRAY_SIZED(r_dst, elem_type, r_len) | |
| Length + fill value | NEW_ARRAY_FILLED(r_dst, elem_type, r_len, r_fill) | |

**User's choice:** Both — add both opcodes for maximum flexibility.

---

## Claude's Discretion

- Exact copy method name (must be directionally clear)
- Internal memmove implementation details
- Spec section restructuring
- Whether NEW_ARRAY_SIZED/FILLED get language-level syntax sugar

## Deferred Ideas

None — discussion stayed within phase scope
