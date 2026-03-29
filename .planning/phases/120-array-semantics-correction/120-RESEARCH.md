# Phase 120: Array Semantics Correction - Research

**Researched:** 2026-03-29
**Domain:** Writ compiler/runtime/module format — array opcode overhaul
**Confidence:** HIGH (all findings from direct source inspection)

## Summary

Phase 120 replaces the "growable array" model (inherited from v13.0) with an
"allocation-explicit" model. The four growth opcodes (`ArrayAdd`, `ArrayRemove`,
`ArrayInsert`, `ArrayContains`) are eliminated. Two new allocation primitives
(`ArrayResize`, `ArrayCopy`) and two new creation opcodes (`NewArraySized`,
`NewArrayFilled`) are added. The module format_version bumps from 4 to 5. The
language spec §1.6 is rewritten from "growable" to "allocation-explicit."

The change spans exactly seven files (module, compiler, runtime x2, assembler
x2, spec x4) plus three test-update surfaces (golden snapshots, vm_tests,
golden .writ fixtures). Every file has a well-established pattern to follow;
no novel architecture is needed.

**Primary recommendation:** Follow the eight-step opcode addition checklist
documented in CONTEXT.md (enum → opcode() → encode/decode → assembler →
disassembler → runtime → compiler), applied once for each new opcode, and
once in reverse for each removed opcode.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Clean break — old opcodes (ArrayAdd, ArrayRemove, ArrayInsert,
  ArrayContains) are treated as if they never existed. No tombstoning, no
  deprecation errors, no backward-compat shims.
- **D-02:** format_version bumps from 4 to 5. Reader rejects anything below 5.
- **D-03:** New opcode assignments in a compact contiguous block:
  - 0x0900 NEW_ARRAY (unchanged)
  - 0x0901 ARRAY_INIT (unchanged)
  - 0x0902 ARRAY_LOAD (unchanged)
  - 0x0903 ARRAY_STORE (unchanged)
  - 0x0904 ARRAY_LEN (unchanged)
  - 0x0905 ARRAY_RESIZE (new — Shape RR: r_arr, r_new_len)
  - 0x0906 ARRAY_COPY (new — Shape var: r_dst_arr, r_dst_idx, r_src_arr, r_src_idx, r_len)
  - 0x0907 ARRAY_SLICE (moved from 0x0908 — compacted)
  - 0x0908 NEW_ARRAY_SIZED (new — Shape var: r_dst, elem_type:u32, r_len)
  - 0x0909 NEW_ARRAY_FILLED (new — Shape var: r_dst, elem_type:u32, r_len, r_fill)
- **D-04:** resize(n) where n > current len: new slots filled with type default
  values (int→0, string→"", bool→false, float→0.0, reference types→null).
- **D-05:** resize(n) where n < current len: silent truncation.
- **D-06:** resize(0) = valid empty array. Negative values crash at runtime.
- **D-07:** Direction semantics must be unambiguous in the copy method name.
  Claude's discretion on exact name.
- **D-08:** Out-of-bounds on either source or destination range crashes at runtime.
  No clamping.
- **D-09:** Overlapping regions within the same array handled correctly
  (memmove semantics).
- **D-10:** Spec wording: "ordered, homogeneous collections with explicit
  allocation. Size changes require reallocation via resize(n)."
- **D-11:** NEW_ARRAY keeps creating zero-length arrays.
- **D-12:** NEW_ARRAY_SIZED(r_dst, elem_type, r_len) creates r_len elements
  filled with type defaults.
- **D-13:** NEW_ARRAY_FILLED(r_dst, elem_type, r_len, r_fill) creates r_len
  elements filled with a specific value.

### Claude's Discretion

- Exact method name for the copy operation (must be directionally clear)
- Internal implementation details for memmove-style overlap handling
- Exact spec section restructuring
- Whether NEW_ARRAY_SIZED and NEW_ARRAY_FILLED get language-level syntax sugar
  or are only accessible through compiler emission

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ARR-01 | `T[]` does NOT support `add`, `remove_at`, or `insert` — compiler rejects these as unknown methods | Remove `"add"`, `"remove_at"`, `"insert"` arms from `builtins.rs` TyKind::Array match; compiler falls through to unknown-method error |
| ARR-02 | `T[]` supports `resize(new_len: int)` to reallocate the array to a new fixed size | Add `"resize"` arm in compiler, `ArrayResize` instruction, `exec_array_resize` in runtime |
| ARR-03 | `T[]` supports `copy(dst_idx, src, src_idx, len)` for bulk element transfer | Add `"copy_to"` or equivalent arm in compiler, `ArrayCopy` instruction, `exec_array_copy` in runtime |
| ARR-04 | `T[]` retains `len()`, `slice(start, end)`, and indexed access | These paths are already present; preserve them (ARRAY_SLICE moves from 0x0908 to 0x0907 — update opcode number everywhere) |
| ARR-05 | `contains` is removed from `T[]` | Remove `"contains"` arm from TyKind::Array match in `builtins.rs` |
| ARR-06 | Language spec describes arrays as "fixed-size" with resize/copy as explicit operations; growth method docs removed | Rewrite §1.6.1-1.6.3 in `07_6_primitive_types.md`; rewrite `57_3_9_arrays.md` opcode table; update `67_4_2_opcode_assignment_table.md`; update `65_4_0_instruction_count_by_category.md` |
</phase_requirements>

---

## Standard Stack

This is an internal compiler change. No external libraries are added or removed.
All work is in the existing Rust workspace.

### Affected Crates

| Crate | Role | Change Type |
|-------|------|-------------|
| `writ-module` | Instruction enum, encode/decode, format_version | Major edit |
| `writ-compiler` | Dot-call dispatch for array methods | Minor edit |
| `writ-runtime` | Opcode execution handlers, dispatch routing | Moderate edit |
| `writ-assembler` | Text assembler mnemonic parsing + disassembly | Minor edit |
| `writ-golden` | Golden snapshot fixtures (.writ files + .writil files) | Update |
| `language-spec` | Spec documents | Rewrite sections |

**Build and test command:**
```bash
cargo test --workspace 2>&1
```

**Bless golden snapshots after compiler changes:**
```bash
BLESS=1 cargo test -p writ-golden 2>&1
```

---

## Architecture Patterns

### Established Opcode Addition Checklist (from CONTEXT.md §Established Patterns)

Every opcode change follows this exact sequence. Do it once per new opcode,
once in reverse per removed opcode:

1. `writ-module/src/instruction.rs` — add/remove enum variant
2. `writ-module/src/instruction.rs` — add/remove `opcode()` match arm
3. `writ-module/src/instruction.rs` — add/remove `encode()` match arm
4. `writ-module/src/instruction.rs` — add/remove `decode()` match arm
5. `writ-assembler/src/assembler.rs` — add/remove mnemonic arm
6. `writ-assembler/src/disassembler.rs` — add/remove disassembly arm
7. `writ-runtime/src/dispatch/objects.rs` — add/remove `exec_array_*` function
8. `writ-runtime/src/dispatch/mod.rs` — add/remove dispatch routing arm
9. `writ-compiler/src/emit/body/expr/builtins.rs` — add/remove dot-call arm

### Opcode Renumbering: ArraySlice 0x0908 → 0x0907

The current code assigns:
- 0x0905 ArrayAdd
- 0x0906 ArrayRemove
- 0x0907 ArrayInsert
- 0x0908 ArraySlice
- 0x0909 ArrayContains

After the change (D-03):
- 0x0905 ArrayResize (new)
- 0x0906 ArrayCopy (new)
- 0x0907 ArraySlice (moved — was 0x0908)
- 0x0908 NewArraySized (new)
- 0x0909 NewArrayFilled (new)

This means the existing `ArraySlice` variant needs its opcode number changed
from 0x0908 to 0x0907 in both `opcode()` and `decode()`. The variant name and
struct fields stay identical.

### format_version Bump: 4 → 5

Three locations:
- `writ-module/src/builder.rs` line 598: change `format_version: 4` to `format_version: 5`
- `writ-module/src/module.rs` line 94: change `format_version: 4` to `format_version: 5`
- `writ-module/src/reader.rs` lines 59-60: change `!= 4` to `!= 5`

### New Instruction Shapes

**ARRAY_RESIZE** (Shape RR, 6 bytes):
```
u16(opcode=0x0905) u16(r_arr) u16(r_new_len)
```
Reads from `r_arr` and `r_new_len` registers. Mutates the heap array in-place.

**ARRAY_COPY** (Shape var, 12 bytes):
```
u16(opcode=0x0906) u16(r_dst_arr) u16(r_dst_idx) u16(r_src_arr) u16(r_src_idx) u16(r_len)
```
Five register operands. Must handle overlap (memmove semantics) when src and dst
are the same heap object.

**NEW_ARRAY_SIZED** (Shape var, 10 bytes):
```
u16(opcode=0x0908) u16(r_dst) u32(elem_type) u16(r_len)
```
Creates a new heap array of r_len elements, all set to the type default.

**NEW_ARRAY_FILLED** (Shape var, 12 bytes):
```
u16(opcode=0x0909) u16(r_dst) u32(elem_type) u16(r_len) u16(r_fill)
```
Creates a new heap array of r_len elements, all set to the value in r_fill.

### Recommended Project Structure (unchanged)

The existing workspace layout is unchanged. Files are edited in-place.

### Anti-Patterns to Avoid

- **Do not tombstone old opcodes:** D-01 is a clean break. Do not add
  `0x0905 => Err("ArrayAdd removed")` in the decoder. Simply remove the arms.
- **Do not add backward-compat version checks:** The reader already rejects
  format_version != 5 (after the bump). No dual-version path needed.
- **Do not update stdlib in this phase:** `writ-std/src/collections.writ`
  calls `add`, `remove_at`, `insert`, and `contains` on arrays. This WILL
  produce compiler errors during Phase 120. That is expected and correct.
  The stdlib rewrite is Phase 121. The golden tests that exercise collection
  types (`coll_list_basic`, `coll_map_basic`, etc.) will fail and must be
  either deleted or skipped in this phase. Do not attempt to fix them here.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| memmove-style overlap | Custom loop with forward/backward decision | Rust `Vec::copy_within` — handles overlap correctly on a single `Vec<Value>` |
| Default values for resize | Type-switch in runtime | Pattern-match on `elem_type` against the existing type-encoding constants, produce `Value::Int(0)`, `Value::Float(0.0)`, etc. |
| Encode/decode boilerplate | One-off ad hoc writes | Use the existing `read_rr`, `read_rrr`, `write_u16` helpers already in `instruction.rs` |

**Key insight:** The heap's `HeapObject::Array { elements: Vec<Value>, elem_type: u32 }`
already stores `elem_type`. Both resize (for default fill) and copy (for type
validation, if any) can read it directly. No additional runtime type tracking needed.

---

## Common Pitfalls

### Pitfall 1: Missing ArraySlice Renumber in All Three Places
**What goes wrong:** ArraySlice encode/decode/opcode() each have the number
0x0908. If only two of three are updated to 0x0907, the module round-trips
incorrectly and golden tests produce wrong output.
**Why it happens:** The opcode number appears independently in `opcode()`,
the `encode()` match arm (via `self.opcode()`), and the `decode()` match arm.
**How to avoid:** Search for `0x0908` across `instruction.rs` and update
every occurrence associated with `ArraySlice`.
**Warning signs:** Golden test for `array_primitives` shows `ARRAY_SLICE`
at wrong byte offset, or `BLESS=1` run produces different output on re-run.

### Pitfall 2: Golden Tests That Use Removed Opcodes
**What goes wrong:** After removing `ArrayAdd`, the golden tests
`array_primitives` and `type_array_ops` reference `arr.add(...)` in their
`.writ` source files — they will no longer compile, causing the test to
panic before even reaching the snapshot comparison.
**Why it happens:** Both `.writ` fixtures were written when add/remove_at/
insert/contains were legal array methods.
**How to avoid:** Rewrite both `.writ` files to use `resize` + indexed
assignment. Then re-bless the `.writil` snapshots.
**Warning signs:** `test_type_array_ops` or `test_array_primitives` panics
with "unknown method `add` on `int[]`" during compilation stage.

### Pitfall 3: Collection Golden Tests Break During Phase 120
**What goes wrong:** The coll_* golden tests (`coll_list_basic`, etc.) load
`writ-std/src/collections.writ` which calls `add`, `remove_at`, and `contains`
on arrays. These become compiler errors in Phase 120.
**Why it happens:** The stdlib is not updated until Phase 121.
**How to avoid:** The planner must decide: either (a) delete these golden tests
temporarily and restore them after Phase 121, or (b) mark them `#[ignore]`
with a note. Option (b) is safer — the tests remain as documentation of
what must pass in Phase 121.
**Warning signs:** Any golden test that pre-loads `collections.writ` panics
with "unknown method on array".

### Pitfall 4: format_version Mismatch in Tests
**What goes wrong:** Unit tests in `writ-module` or `writ-runtime` that
construct modules directly via `ModuleBuilder` will produce version=5 modules
after the bump, but any test that reads a hardcoded byte literal or calls
`Module::from_bytes` with an old version-4 blob will fail.
**Why it happens:** `ModuleBuilder::build()` writes `format_version: 5`;
`reader.rs` now rejects `!= 5`.
**How to avoid:** Search for hardcoded `format_version: 4` or version `4`
byte in test fixtures. Update those constants. Most tests use `ModuleBuilder`
end-to-end so they will auto-update.
**Warning signs:** `DecodeError::UnsupportedVersion(4)` in test output.

### Pitfall 5: ARRAY_COPY Same-Array Overlap
**What goes wrong:** If dst and src are the same heap array and the ranges
overlap, a naive forward copy corrupts elements before they are read.
**Why it happens:** D-09 explicitly requires memmove semantics.
**How to avoid:** Use `Vec::copy_within(src_range, dst_start)` when dst and
src are the same `HeapRef`. For different arrays, a plain `clone_from_slice`
suffices.
**Warning signs:** A test shifting elements within one array produces
incorrect values at overlapping positions.

---

## Code Examples

All patterns from direct source inspection.

### Pattern: Add a new RR-shaped array instruction (example: ArrayResize)

```rust
// 1. writ-module/src/instruction.rs — enum variant
/// 0x0905 — Shape RR (6B)
ArrayResize { r_arr: u16, r_new_len: u16 },

// 2. opcode() arm
Instruction::ArrayResize { .. } => 0x0905,

// 3. encode() arm (already handled by generic RR path if it exists,
//    otherwise add explicitly like ArrayLen does):
Instruction::ArrayResize { r_arr, r_new_len } => {
    w.write_u16::<LittleEndian>(*r_arr)?;
    w.write_u16::<LittleEndian>(*r_new_len)?;
    Ok(())
}

// 4. decode() arm
0x0905 => read_rr(r).map(|(a, n)| Instruction::ArrayResize { r_arr: a, r_new_len: n }),
```

### Pattern: Add a multi-register var-shape instruction (example: ArrayCopy)

```rust
// Variant (6 register fields × u16 = 12B total including opcode)
ArrayCopy { r_dst_arr: u16, r_dst_idx: u16, r_src_arr: u16, r_src_idx: u16, r_len: u16 },

// encode()
Instruction::ArrayCopy { r_dst_arr, r_dst_idx, r_src_arr, r_src_idx, r_len } => {
    w.write_u16::<LittleEndian>(*r_dst_arr)?;
    w.write_u16::<LittleEndian>(*r_dst_idx)?;
    w.write_u16::<LittleEndian>(*r_src_arr)?;
    w.write_u16::<LittleEndian>(*r_src_idx)?;
    w.write_u16::<LittleEndian>(*r_len)?;
    Ok(())
}

// decode()
0x0906 => {
    let r_dst_arr = r.read_u16::<LittleEndian>()?;
    let r_dst_idx = r.read_u16::<LittleEndian>()?;
    let r_src_arr = r.read_u16::<LittleEndian>()?;
    let r_src_idx = r.read_u16::<LittleEndian>()?;
    let r_len = r.read_u16::<LittleEndian>()?;
    Ok(Instruction::ArrayCopy { r_dst_arr, r_dst_idx, r_src_arr, r_src_idx, r_len })
}
```

### Pattern: Runtime handler for resize

```rust
// writ-runtime/src/dispatch/objects.rs
pub(super) fn exec_array_resize(
    ctx: &mut ExecContext<'_>,
    r_arr: u16,
    r_new_len: u16,
) -> ExecutionResult {
    let frame = ctx.task.call_stack.last().unwrap();
    let arr_ref = helpers::extract_ref(&frame.registers[r_arr as usize]);
    let new_len = helpers::extract_int(&frame.registers[r_new_len as usize]);
    if new_len < 0 {
        return ExecutionResult::Crash("ArrayResize: negative length".into());
    }
    match ctx.heap.get_object_mut(arr_ref) {
        Ok(HeapObject::Array { elements, elem_type }) => {
            let new_len = new_len as usize;
            let et = *elem_type;
            if new_len > elements.len() {
                let default = default_value_for(et);
                elements.resize(new_len, default);
            } else {
                elements.truncate(new_len);
            }
            ExecutionResult::Continue
        }
        _ => ExecutionResult::Crash("ArrayResize: not an array".into()),
    }
}

fn default_value_for(elem_type: u32) -> Value {
    // elem_type encoding: 0=int, 1=float, 2=bool, 3=string, others=ref
    match elem_type {
        0 => Value::Int(0),
        1 => Value::Float(0.0),
        2 => Value::Bool(false),
        _ => Value::Null, // string and reference types
    }
}
```

### Pattern: Remove dot-call arm from compiler (example: removing "add")

In `writ-compiler/src/emit/body/expr/builtins.rs`, inside `TyKind::Array =>` match:

```rust
// REMOVE these arms entirely:
"add" if args.len() == 1 => { ... }
"remove_at" if args.len() == 1 => { ... }
"insert" if args.len() == 2 => { ... }
"contains" if args.len() == 1 => { ... }

// ADD these arms:
"resize" if args.len() == 1 => {
    let r_new_len = emit_expr(emitter, &args[0]);
    emitter.emit(Instruction::ArrayResize { r_arr, r_new_len });
    let r_dst = emitter.alloc_reg(ty);
    return Some(r_dst);
}
"copy_to" if args.len() == 4 => {
    // copy_to(dst_idx, src_arr, src_idx, len)
    // receiver is source; first arg is destination index into receiver?
    // OR: receiver is destination, args are (dst_idx, src, src_idx, len)
    // See Claude's Discretion note on naming.
    ...
}
```

### Pattern: Assembler mnemonic (example: ARRAY_RESIZE)

In `writ-assembler/src/assembler.rs`:
```rust
"ARRAY_RESIZE" => Ok(Instruction::ArrayResize { r_arr: reg(0)?, r_new_len: reg(1)? }),
```

In `writ-assembler/src/disassembler.rs`:
```rust
Instruction::ArrayResize { r_arr, r_new_len } => ("ARRAY_RESIZE".into(), vec![r(*r_arr), r(*r_new_len)]),
```

---

## Copy Method Naming — Recommended Direction

D-07 gives Claude's discretion on the copy method name. The constraint is:
"reading the call site must make it obvious which array is source and
which is destination."

**Recommendation:** `copy_from(src, src_idx, dst_idx, len)` on the destination
receiver. Call site reads:

```writ
dst.copy_from(src, src_idx: 0, dst_idx: 2, len: 5);
```

The receiver IS the destination, `src` is the first argument. This mirrors
Rust's `copy_from_slice` convention and makes the write target unambiguous.

The IL instruction operand order follows D-03: `r_dst_arr, r_dst_idx, r_src_arr, r_src_idx, r_len`.

---

## State of the Art

| Old Approach | Current Approach | Changed | Impact |
|--------------|------------------|---------|--------|
| T[] is growable (v13.0) | T[] is allocation-explicit (v14.0) | Phase 120 | Compiler rejects add/remove_at/insert/contains on arrays |
| ArraySlice at 0x0908 | ArraySlice at 0x0907 | Phase 120 | format_version bump needed |
| format_version 4 | format_version 5 | Phase 120 | Reader rejects old binaries |

---

## Open Questions

1. **`NEW_ARRAY_SIZED` / `NEW_ARRAY_FILLED` — language-level syntax sugar?**
   - What we know: D-13 defines the opcodes; discretion is whether to expose
     them in source syntax.
   - What's unclear: Will the compiler ever need to emit these for user-facing
     code in Phase 120, or are they purely for Phase 121 stdlib use?
   - Recommendation: In Phase 120, add the opcodes to the instruction set but
     do not wire them to any compiler dot-call. Phase 121 can add compiler
     emission if the stdlib rewrite needs them. This minimizes Phase 120 scope.

2. **Collection golden tests during Phase 120**
   - What we know: `coll_list_basic`, `coll_map_basic`, `coll_set_basic`,
     `coll_hashmap_basic` all load `collections.writ` which calls removed methods.
   - What's unclear: Whether to delete or `#[ignore]` these tests.
   - Recommendation: Mark with `#[ignore]` and a `// Phase 121: re-enable`
     comment. Do not delete — they serve as acceptance criteria for Phase 121.

---

## Environment Availability

Step 2.6: SKIPPED — this phase is purely code/spec changes with no external
tool dependencies beyond the existing Rust toolchain.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`) |
| Config file | None — workspace Cargo.toml |
| Quick run command | `cargo test -p writ-module -p writ-compiler -p writ-runtime` |
| Full suite command | `cargo test --workspace` |
| Bless golden snapshots | `BLESS=1 cargo test -p writ-golden` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ARR-01 | Compiler rejects `arr.add(x)` on `T[]` | unit (compiler error) | `cargo test -p writ-compiler` | ❌ Wave 0 — new test needed |
| ARR-02 | `arr.resize(n)` compiles and executes | golden + vm_test | `cargo test -p writ-golden -p writ-runtime` | ❌ Wave 0 — new .writ fixture |
| ARR-03 | `arr.copy_from(...)` compiles and executes | golden + vm_test | `cargo test -p writ-golden -p writ-runtime` | ❌ Wave 0 — new .writ fixture |
| ARR-04 | `arr.len()`, `arr.slice()`, `arr[i]` still work | golden (existing) | `cargo test -p writ-golden` | ✅ `ctrl_for_array.writil`, `type_array_ops.writ` (needs rewrite) |
| ARR-05 | Compiler rejects `arr.contains(x)` on `T[]` | unit (compiler error) | `cargo test -p writ-compiler` | ❌ Wave 0 — new test needed |
| ARR-06 | Spec sections updated | manual review | N/A | ✅ spec files exist, need rewrite |

### Sampling Rate

- **Per task commit:** `cargo test -p writ-module`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green (minus `#[ignore]`-d collection tests) before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] New `.writ` golden fixture for `resize` — covers ARR-02
- [ ] New `.writ` golden fixture for `copy_from` — covers ARR-03
- [ ] Compiler error test: `arr.add(x)` on `T[]` → error message — covers ARR-01
- [ ] Compiler error test: `arr.contains(x)` on `T[]` → error message — covers ARR-05
- [ ] Rewrite `array_primitives.writ` and re-bless `array_primitives.writil` — removes old opcodes
- [ ] Rewrite `type_array_ops.writ` (uses `arr.add(99)`) and re-bless `type_array_ops.writil`

---

## Sources

### Primary (HIGH confidence)

All findings are from direct source inspection of the codebase. No external
sources consulted — all information is self-contained in the repository.

- `writ-module/src/instruction.rs` — current opcode enum, serialization, current assignments
- `writ-module/src/reader.rs` — format_version validation (line 59: `!= 4`)
- `writ-module/src/builder.rs` — format_version constant (line 598)
- `writ-module/src/module.rs` — format_version default (line 94)
- `writ-runtime/src/dispatch/objects.rs` — all existing `exec_array_*` functions
- `writ-runtime/src/dispatch/mod.rs` — dispatch routing for array opcodes
- `writ-compiler/src/emit/body/expr/builtins.rs` — dot-call dispatch including all array methods
- `writ-assembler/src/assembler.rs` — mnemonic table
- `writ-assembler/src/disassembler.rs` — disassembly table
- `writ-golden/tests/golden/array_primitives.writ` + `.writil` — existing fixture using removed methods
- `writ-golden/tests/golden/type_array_ops.writ` + `.writil` — existing fixture using removed `add`
- `writ-std/src/collections.writ` — confirms List/Map/Set/HashMap all call removed methods
- `language-spec/spec/57_3_9_arrays.md` — current array IL opcode table (9 opcodes, needs full rewrite)
- `language-spec/spec/67_4_2_opcode_assignment_table.md` — master opcode table (0x09 section needs rewrite)
- `language-spec/spec/65_4_0_instruction_count_by_category.md` — Arrays row: currently "9" (becomes 7 net — 4 removed, 5 added = 10, minus ARRAY_CONTAINS which was at 0x0909 but current enum shows it's present)
- `language-spec/spec/07_6_primitive_types.md` §1.6.1-1.6.3 — current "growable" language spec

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates identified from direct source inspection
- Architecture: HIGH — opcode addition pattern is well-established and followed consistently
- Pitfalls: HIGH — each pitfall derived from reading actual code, not inference
- Copy naming recommendation: MEDIUM — directionally motivated; final name is Claude's discretion

**Research date:** 2026-03-29
**Valid until:** Stable (internal implementation; no external dependencies)
