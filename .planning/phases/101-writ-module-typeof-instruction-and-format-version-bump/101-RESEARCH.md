# Phase 101: writ-module TypeOf Instruction and Format Version Bump - Research

**Researched:** 2026-03-28
**Domain:** Binary IL module format — instruction encoding, format versioning, assembler/disassembler text protocol
**Confidence:** HIGH

## Summary

Phase 101 is a surgical, pattern-following change. The TypeOf instruction (opcode 0x0A30, shape RI32) must be added to five well-defined locations inside two crates: `writ-module` and `writ-assembler`. All architectural decisions are pre-locked in the spec and in CONTEXT.md. The format_version bump and rejection logic are already implemented in the codebase — `module.rs` already creates `format_version: 4`, and `reader.rs` already rejects anything other than 4 with `DecodeError::UnsupportedVersion`. The spec-work from Phase 100 pre-satisfied the requirements at the document level; this phase implements them in code.

The change is mechanical repetition of established RI32 patterns (identical shape to `New`, `SpawnEntity`, `GetOrCreate`, `FindAll`, `NewArray`, `DeferPush`, `LoadGlobal`). No new shapes, no new error variants, no schema changes, no heap changes. The entire change surface is: one enum variant, one opcode match arm, one encode match arm, one decode match arm, one assembler mnemonic arm, one disassembler text arm, and tests.

**Primary recommendation:** Follow the `New`/`GetOrCreate` patterns exactly for all five code locations. Every detail (byte order, field names, u32 cast) is established precedent.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- TypeOf opcode is 0x0A30 with RI32 shape: r_dst, type_idx:u32
- 8-byte encoding: u16(0x0A30) u16(r_dst) u32(type_idx)
- format_version bumps from 3 to 4
- format_version=3 modules rejected with UnsupportedVersion error
- Instruction count bumps from 91 to 92
- Assembler mnemonic: `typeof`

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SPEC-05 | TypeOf opcode assigned in §4.2 opcode table | Opcode 0x0A30 confirmed in CONTEXT.md; 0x0A range already established in instruction.rs (0x0A00-0x0A22 present) |
| SPEC-06 | format_version bumped to 4 in spec | Already implemented in module.rs (`format_version: 4`) and reader.rs (rejects != 4); Phase 101 implements the remaining binary instruction support |
</phase_requirements>

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| byteorder | existing | LittleEndian u16/u32 read/write | Used by every existing instruction encode/decode |
| writ-module | workspace | Instruction enum, reader, writer, module | The crate being extended |
| writ-assembler | workspace | assembler.rs map_instruction, disassembler.rs instr_to_text | The crate being extended |

No new dependencies are required. All tools are already present in Cargo.toml.

## Architecture Patterns

### Established RI32 Pattern

TypeOf uses Shape RI32: `u16(opcode) u16(r_dst) u32(type_idx)` = 8 bytes total.

This shape is already used by seven instructions: `LoadString`, `New`, `SpawnEntity`, `GetOrCreate`, `FindAll`, `NewArray`, `DeferPush`, `LoadGlobal`. The canonical template to follow is `New { r_dst, type_idx }`:

**Enum declaration (`instruction.rs`, around line 117):**
```rust
/// 0x0A30 — Shape RI32 (8B)
TypeOf { r_dst: u16, type_idx: u32 },
```

**`opcode()` match arm (around line 308 in the 0x0A block):**
```rust
Instruction::TypeOf { .. } => 0x0A30,
```

**`encode()` match arm (around line 540 in the RI32 section):**
```rust
Instruction::TypeOf { r_dst, type_idx } => {
    w.write_u16::<LittleEndian>(*r_dst)?;
    w.write_u32::<LittleEndian>(*type_idx)?;
}
```

**`decode()` match arm (around line 924, in the 0x0A Enum section, after 0x0A22):**
```rust
// ── 0x0A Type Operations — Reflection ─────────────────
0x0A30 => {
    let r_dst = r.read_u16::<LittleEndian>()?;
    let type_idx = r.read_u32::<LittleEndian>()?;
    Ok(Instruction::TypeOf { r_dst, type_idx })
}
```

**Assembler `map_instruction` arm (`assembler.rs`, after the Enum section around line 675):**
```rust
// ── 0x0A Type Operations — Reflection ──
"TYPEOF" => Ok(Instruction::TypeOf { r_dst: reg(0)?, type_idx: token_val(1)? }),
```

**Disassembler `instr_to_text` arm (`disassembler.rs`, after the Enum section around line 758):**
```rust
// ── 0x0A Type Operations — Reflection ──
Instruction::TypeOf { r_dst, type_idx } => ("TYPEOF".into(), vec![r(*r_dst), tok(*type_idx)]),
```

### Format Version Status (Already Complete)

The format_version handling is already done in the codebase:

- `module.rs` line 92: `format_version: 4` — `Module::new()` already emits version 4
- `reader.rs` line 59: `if format_version != 4 { return Err(DecodeError::UnsupportedVersion(format_version)); }` — already rejects non-4 versions including version 3
- `error.rs` line 21: `UnsupportedVersion(u16)` variant already exists

This means the success criterion "A module with format_version=3 produces an UnsupportedVersion error when loaded" is already satisfied by the existing code. The test needs to verify this is working.

### Doc Comment Update

The enum-level doc comment on line 6 currently reads `/// All 91 IL opcodes`. After adding TypeOf it must read `/// All 92 IL opcodes`. Similarly the assembler's `map_instruction` function doc at line 416 reads `/// Handles all 94 opcodes` — this count appears to be a pre-existing discrepancy (the actual enum has 91 variants). Update the instruction.rs enum doc to 92.

### PartialEq Coverage

`Instruction::PartialEq` is implemented via byte encoding comparison (lines 1018-1039). Because TypeOf follows the existing encode/decode path, `PartialEq` will work correctly without any changes — the byte comparison approach is shape-agnostic.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Byte I/O | Custom slice writing | `byteorder::WriteBytesExt::write_u16/u32` | Already used by every other instruction |
| Instruction size | Manual calculation | `instruction_size()` in assembler calls `encode()` to a Vec and measures length | Already handles all shapes including new ones |
| Round-trip identity | Custom comparison | Existing `round_trip()` helper in `instruction_tests.rs` | Pattern already used for all 91 existing instructions |

## Common Pitfalls

### Pitfall 1: Opcode Conflict in 0x0A Range
**What goes wrong:** 0x0A20-0x0A22 are Enum operations (NewEnum, GetTag, ExtractField). 0x0A30 must not overlap any existing 0x0A range.
**Why it happens:** The 0x0A category is subdivided: 0x0A00-0x0A0F Option, 0x0A10-0x0A1F Result, 0x0A20-0x0A2F Enum, 0x0A30+ Reflection.
**How to avoid:** 0x0A30 is confirmed unoccupied — the existing decode match has no arm for it (falls through to `_ => Err(DecodeError::InvalidOpcode(opcode))`).

### Pitfall 2: Mnemonic Case in Assembler
**What goes wrong:** The assembler converts mnemonics to uppercase before matching (`let upper = mnemonic.to_uppercase()`). The text input `typeof` is case-insensitive, but the match arm must use the uppercase string `"TYPEOF"`.
**Why it happens:** Pattern established from line 425 in assembler.rs.
**How to avoid:** Match on `"TYPEOF"` not `"typeof"`.

### Pitfall 3: Missing PartialEq Arm for LoadFloat
**What goes wrong:** The existing PartialEq impl has a special case for `LoadFloat` (f64 bit comparison). Adding TypeOf does NOT require a new special case — it has no f64 fields.
**Warning signs:** None — no action needed. Confirming non-action is important here.

### Pitfall 4: Assembler Comment Says "94 opcodes"
**What goes wrong:** The `map_instruction` function doc says "Handles all 94 opcodes" (assembler.rs line 416), which already doesn't match the actual 91 enum variants. After adding TypeOf this becomes 92. Update only the instruction.rs enum doc; the assembler doc count discrepancy pre-existed and should also be updated to 92.
**How to avoid:** Update both doc comments to 92.

### Pitfall 5: Format Version Test — Version 3 Must Come From Test Code
**What goes wrong:** The existing `from_bytes` function rejects anything other than version 4. A test verifying UnsupportedVersion must construct raw bytes with version 3 manually (or copy a real module's bytes and patch offset 4-5).
**How to avoid:** Use `let mut bytes = module.to_bytes().unwrap(); bytes[4] = 3; bytes[5] = 0;` to simulate a version-3 module, then assert `from_bytes(&bytes)` returns `Err(DecodeError::UnsupportedVersion(3))`.

## Code Examples

### RI32 decode pattern (from existing 0x0800 New)
```rust
// Source: writ-module/src/instruction.rs line 843
0x0800 => {
    let r_dst = r.read_u16::<LittleEndian>()?;
    let type_idx = r.read_u32::<LittleEndian>()?;
    Ok(Instruction::New { r_dst, type_idx })
}
```

TypeOf decode at 0x0A30 is identical in structure.

### RI32 encode pattern (from existing New)
```rust
// Source: writ-module/src/instruction.rs line 540
Instruction::New { r_dst, type_idx } => {
    w.write_u16::<LittleEndian>(*r_dst)?;
    w.write_u32::<LittleEndian>(*type_idx)?;
}
```

### Round-trip test pattern (from instruction_tests.rs)
```rust
// Source: writ-module/tests/instruction_tests.rs
#[test]
fn test_typeof_round_trip() {
    round_trip(&Instruction::TypeOf { r_dst: 3, type_idx: 42 });
}
```

### UnsupportedVersion test pattern
```rust
#[test]
fn test_unsupported_version_3_rejected() {
    let module = Module::new();
    let mut bytes = module.to_bytes().unwrap();
    // Patch format_version field (bytes 4-5 in the 200-byte header)
    bytes[4] = 3;
    bytes[5] = 0;
    match Module::from_bytes(&bytes) {
        Err(DecodeError::UnsupportedVersion(3)) => {}
        other => panic!("expected UnsupportedVersion(3), got {:?}", other),
    }
}
```

### Assembler test pattern (from asm_basic.rs style)
```rust
#[test]
fn test_typeof_assembles() {
    let src = r#"
.module "test" "1.0.0" {
    .method "main" () -> void {
        .reg r0 int
        .reg r1 int
        TYPEOF r0, 1
        RET_VOID
    }
}
"#;
    let module = writ_assembler::assemble(src).unwrap();
    assert_eq!(module.method_bodies.len(), 1);
    // 8 bytes for TYPEOF + 2 bytes for RET_VOID = 10 bytes
    assert_eq!(module.method_bodies[0].code.len(), 10);
}
```

### Disassembler test pattern
```rust
#[test]
fn test_typeof_disassembles() {
    use writ_module::Instruction;
    // Build a module with TypeOf in the body...
    // Then disassemble and check the output contains "TYPEOF r0, 1"
}
```

## Complete Change Surface

The following files require edits. No other files need changes:

| File | Change | Lines |
|------|--------|-------|
| `writ-module/src/instruction.rs` | Add `TypeOf` variant to enum; add opcode arm; add encode arm; add decode arm; update doc comment to 92 | ~line 188, ~253, ~540, ~924 |
| `writ-assembler/src/assembler.rs` | Add `"TYPEOF"` arm to `map_instruction`; update doc comment to 92 | ~line 675, 416 |
| `writ-assembler/src/disassembler.rs` | Add `TypeOf` arm to `instr_to_text` | ~line 758 |
| `writ-module/tests/instruction_tests.rs` | Add `test_typeof_round_trip` test | new test |
| `writ-module/tests/round_trip.rs` | Add `test_unsupported_version_3_rejected` test | new test |
| `writ-assembler/tests/asm_basic.rs` (or new file) | Add assembler TYPEOF test | new test |
| `writ-assembler/tests/disasm_round_trip.rs` (or new file) | Add TYPEOF disasm round-trip test | new test |

**No changes required to:**
- `writ-module/src/module.rs` — format_version is already 4
- `writ-module/src/reader.rs` — version rejection already implemented
- `writ-module/src/error.rs` — UnsupportedVersion already exists
- `writ-module/src/tables.rs` — no table schema changes
- `writ-module/src/builder.rs` — builder does not enumerate instructions
- Any other crate (writ-compiler, writ-runtime, writ-lsp)

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — purely code changes within existing Rust workspace).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (no separate framework) |
| Config file | none (workspace Cargo.toml) |
| Quick run command | `cargo test -p writ-module -p writ-assembler` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SPEC-05 | TypeOf instruction round-trips through encode/decode | unit | `cargo test -p writ-module test_typeof_round_trip` | Wave 0 |
| SPEC-05 | TypeOf round-trips through full module write/read | unit | `cargo test -p writ-module` | Wave 0 |
| SPEC-05 | Assembler accepts `typeof` mnemonic | unit | `cargo test -p writ-assembler test_typeof` | Wave 0 |
| SPEC-05 | Disassembler emits `TYPEOF` mnemonic | unit | `cargo test -p writ-assembler` | Wave 0 |
| SPEC-06 | format_version=3 produces UnsupportedVersion error | unit | `cargo test -p writ-module test_unsupported_version` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-module -p writ-assembler`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-module/tests/instruction_tests.rs` — add `test_typeof_round_trip` test (file exists, append to it)
- [ ] `writ-module/tests/round_trip.rs` — add `test_unsupported_version_3_rejected` test (file exists, append to it)
- [ ] New test for assembler TYPEOF acceptance (append to `writ-assembler/tests/asm_basic.rs` or create `writ-assembler/tests/typeof_tests.rs`)
- [ ] New test for disassembler TYPEOF emit (append to `writ-assembler/tests/disasm_round_trip.rs`)

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| format_version=3 | format_version=4 | Phase 100 spec work + this phase code | Old .writc files are rejected at load time |
| 91 instructions | 92 instructions | This phase | Adds TypeOf to the instruction set |

## Open Questions

None. All decisions are locked. The change surface is fully defined by existing patterns.

## Sources

### Primary (HIGH confidence)
- `writ-module/src/instruction.rs` — full read, all existing RI32 patterns confirmed
- `writ-module/src/reader.rs` — version check and decode confirmed at lines 59 and 924
- `writ-module/src/module.rs` — format_version=4 confirmed at line 92
- `writ-module/src/error.rs` — UnsupportedVersion variant confirmed
- `writ-assembler/src/assembler.rs` — map_instruction uppercase pattern confirmed
- `writ-assembler/src/disassembler.rs` — instr_to_text pattern confirmed
- `.planning/phases/101-writ-module-typeof-instruction-and-format-version-bump/101-CONTEXT.md` — all locked decisions

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — directly read from source code
- Architecture: HIGH — every pattern verified from source, no inference required
- Pitfalls: HIGH — derived from reading the actual code (uppercase matching, f64 special case, opcode gaps)

**Research date:** 2026-03-28
**Valid until:** Until instruction.rs or assembler.rs are structurally refactored (stable)
