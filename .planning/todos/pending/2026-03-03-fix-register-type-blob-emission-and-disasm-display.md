---
created: "2026-03-03T16:05:39.469Z"
title: Fix register type blob emission and disasm display
area: compiler
files:
  - writ-compiler/src/emit/body/mod.rs
  - writ-assembler/src/disassembler.rs:462-473
---

## Problem

The disassembler shows all registers as `int` regardless of their actual type. Root cause is two-layered:

1. **Compiler** (`emit/body/mod.rs`): emits `rt_offset=0` for all `register_types` entries — it never interns each register's `Ty` into the blob heap as a TypeRef blob. The `BodyEmitter` holds `&ModuleBuilder` (immutable), so it cannot intern into the blob heap during body emission.

2. **Disassembler** (`disassembler.rs` line 465): when `rt_offset == 0`, silently defaults to `"int"` instead of showing `?` or `<unknown>`. This masks the compiler bug — every register appears as `int` with no indication that the type is actually unknown.

Observed with this Writ source:
```writ
pub fn test_bol() -> bool {
    let result = false;
    result
}
```
Disasm output shows `.reg r0 int` and `.reg r1 int` — both should show `bool` (r0 for the variable, r1 for the return value register).

## Solution

1. **Compiler fix**: After `emit_all_bodies()` returns (when `&mut builder` is available again), iterate each `EmittedBody`'s register types and intern the `Ty` values into the blob heap as TypeRef blobs, patching the `register_types` Vec with real offsets. Similar pattern to the existing `pending_strings` deferred interning.

2. **Disassembler fix**: Change the `rt_offset == 0` fallback from `"int"` to `"?"` so unknown register types are visible rather than silently wrong.
