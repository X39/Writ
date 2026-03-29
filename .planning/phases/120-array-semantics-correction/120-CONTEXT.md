# Phase 120: Array Semantics Correction - Context

**Gathered:** 2026-03-29
**Status:** Ready for planning

<domain>
## Phase Boundary

`T[]` becomes a fixed-size array with explicit allocation semantics. Growth methods (`add`, `remove_at`, `insert`, `contains`) are removed from the compiler, runtime, and spec. `resize(n)` and a directional copy method are added as the explicit reallocation primitives. The IL opcode table is compacted with a format_version bump to 5. Two new array-creation opcodes (`NEW_ARRAY_SIZED`, `NEW_ARRAY_FILLED`) are added. The language spec is updated to describe arrays as allocation-explicit collections.

</domain>

<decisions>
## Implementation Decisions

### Opcode Strategy
- **D-01:** Clean break — old opcodes (ArrayAdd, ArrayRemove, ArrayInsert, ArrayContains) are treated as if they never existed. No tombstoning, no deprecation errors, no backward-compat shims.
- **D-02:** format_version bumps from 4 to 5. Reader rejects anything below 5.
- **D-03:** New opcode assignments in a compact contiguous block:
  - 0x0900 NEW_ARRAY (unchanged — creates empty array, rename for clarity)
  - 0x0901 ARRAY_INIT (unchanged)
  - 0x0902 ARRAY_LOAD (unchanged)
  - 0x0903 ARRAY_STORE (unchanged)
  - 0x0904 ARRAY_LEN (unchanged)
  - 0x0905 ARRAY_RESIZE (new — Shape RR: r_arr, r_new_len)
  - 0x0906 ARRAY_COPY (new — Shape var: r_dst_arr, r_dst_idx, r_src_arr, r_src_idx, r_len)
  - 0x0907 ARRAY_SLICE (moved from 0x0908 — compacted)
  - 0x0908 NEW_ARRAY_SIZED (new — Shape var: r_dst, elem_type:u32, r_len)
  - 0x0909 NEW_ARRAY_FILLED (new — Shape var: r_dst, elem_type:u32, r_len, r_fill)

### Resize Semantics
- **D-04:** resize(n) where n > current len: new slots filled with type default values (int→0, string→"", bool→false, float→0.0, reference types→null). Consistent with existing default-value semantics.
- **D-05:** resize(n) where n < current len: silent truncation. Elements at indices >= n are dropped. GC reclaims reference-type elements.
- **D-06:** resize(0) produces a valid empty array (len=0). Negative values crash at runtime.

### Copy Signature
- **D-07:** Direction semantics must be unambiguous in the method name. Claude's discretion on exact naming (e.g., `copy_to` on source receiver, or similar). The key constraint: reading the call site must make it obvious which array is source and which is destination.
- **D-08:** Out-of-bounds on either source or destination range crashes at runtime. No clamping.
- **D-09:** Overlapping regions within the same array are handled correctly (memmove semantics). Enables shift-in-place patterns needed by List<T> internals.

### Spec Wording
- **D-10:** Arrays described as "ordered, homogeneous collections with explicit allocation. Size changes require reallocation via resize(n)." — emphasizes cost of resizing without implying immutability.
- **D-11:** NEW_ARRAY keeps creating zero-length arrays. Name updated in spec/docs to clarify it creates empty arrays.
- **D-12:** NEW_ARRAY_SIZED(r_dst, elem_type, r_len) — creates array of r_len elements filled with type defaults.
- **D-13:** NEW_ARRAY_FILLED(r_dst, elem_type, r_len, r_fill) — creates array of r_len elements filled with a specific value.

### Claude's Discretion
- Exact method name for the copy operation (must be directionally clear — e.g., `copy_to`, `copy_into`, etc.)
- Internal implementation details for memmove-style overlap handling
- Exact spec section restructuring (which subsections to add/remove/modify)
- Whether `NEW_ARRAY_SIZED` and `NEW_ARRAY_FILLED` get language-level syntax sugar or are only accessible through compiler emission

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### IL Spec
- `language-spec/spec/57_3_9_arrays.md` — Current IL array opcode table (must be rewritten)
- `language-spec/spec/67_4_2_opcode_assignment_table.md` — Master opcode assignment table (must be updated)
- `language-spec/spec/65_4_0_instruction_count_by_category.md` — Instruction count summary (must be updated)

### Language Spec
- `language-spec/spec/07_6_primitive_types.md` §1.6.1-1.6.3 — Array type description, literals, and operations (must be rewritten from "growable" to "allocation-explicit")

### Compiler
- `writ-compiler/src/emit/body/expr/builtins.rs` — Array dot-call method dispatch (remove add/remove_at/insert/contains, add resize/copy)

### Runtime
- `writ-runtime/src/dispatch/objects.rs` — Array opcode execution handlers (remove old, add new)
- `writ-runtime/src/dispatch/mod.rs` — Opcode dispatch routing (update instruction matching)

### Module Format
- `writ-module/src/instruction.rs` — Instruction enum, opcode numbers, serialization/deserialization (major changes)

### Assembler
- `writ-assembler/src/assembler.rs` — Text assembler mnemonic parsing
- `writ-assembler/src/disassembler.rs` — Binary-to-text disassembly

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-module/src/instruction.rs`: Existing Instruction enum with serialization infrastructure — pattern for adding new opcodes is well-established
- `writ-runtime/src/dispatch/objects.rs`: `exec_array_*` function pattern — new handlers follow the same HeapObject::Array match pattern
- `writ-compiler/src/emit/body/expr/builtins.rs`: Array method dispatch via `match field.as_str()` — straightforward to modify

### Established Patterns
- Opcode addition follows: enum variant → opcode() number → serialize() arms → deserialize() match arm → assembler mnemonic → disassembler mnemonic → runtime dispatch → compiler emission
- format_version bump: change constant in writ-module, update reader assertion (reject <5)
- Array methods surfaced as compiler dot-call resolution on TyKind::Array receivers

### Integration Points
- Golden tests in `writ-golden/tests/golden_tests.rs` reference array operations — must be updated
- Collection source files (`writ-std/`) call `add`/`remove_at`/`insert` — Phase 121 handles this, but compiler errors will surface during Phase 120
- VM tests in `writ-runtime/tests/vm_tests.rs` exercise array opcodes — must be updated

</code_context>

<specifics>
## Specific Ideas

- The user wants arrays described as "allocation-explicit" rather than "fixed-size" — emphasizes the cost model, not just the constraint
- Clean break philosophy: old opcodes treated as if they never existed, not deprecated
- Both sized and filled array creation opcodes wanted — maximum flexibility for downstream code generation

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 120-array-semantics-correction*
*Context gathered: 2026-03-29*
