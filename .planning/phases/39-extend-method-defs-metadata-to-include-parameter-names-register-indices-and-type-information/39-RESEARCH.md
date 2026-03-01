# Phase 39: Extend method_defs metadata — Research

**Researched:** 2026-03-04
**Domain:** IL metadata structure, method parameter tracking, register allocation conventions
**Confidence:** HIGH

## Summary

Phase 39 extends MethodDef metadata to emit parameter names, register indices, and type information that is currently reconstructed at method entry. The work builds on Phases 30-38 which established correct IL generation, method token resolution, and parameter register pre-allocation (fn_param_map).

Currently, parameter information is split across three systems:
1. **ParamDef table** (9) — exists but is not populated (empty in current compiler output)
2. **fn_param_map** — compiler-side only, contains (name, Ty) list used during body emission to pre-allocate r0..r(n-1)
3. **DebugLocal** — optional (debug build only), maps registers to source names but doesn't store sequence/type info required for reflection or IDE support

Phase 39 should populate **ParamDef** table entries (one per parameter, in declaration order) with names and type signatures, and extend method metadata to track param_count and register index ranges so tooling and the runtime can reconstruct calling conventions without parsing the method body.

**Primary recommendation:** Add param_count field to MethodDef metadata (separate from ParamDef list), emit ParamDef rows during collection with name/type/sequence, and thread parameter information from fn_param_map into serialization. This enables reflection, IDE parameter hints, and debugging tools.

## Standard Stack

### Core

| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|--------------|
| IL Spec §2.16.5, §2.16.6 | Latest | Method parameter metadata format | Authoritative source; defines ParamDef table structure and register layout |
| writ-compiler collect.rs | Current | Parameter collection from AST | Single source of truth for source-level parameter names and types |
| writ-compiler module_builder.rs | Current | Metadata accumulation and finalization | Handles table row ordering via list-ownership pattern |
| writ-assembler disassembler.rs | Current | Metadata display for human inspection | Shows what's currently emitted; baseline for validation |

### Supporting

| Component | Purpose | When to Use |
|-----------|---------|-------------|
| fn_param_map (ModuleBuilder field) | Compiler-side parameter tracking | During emit_all_bodies to pre-allocate parameter registers |
| DebugLocal entries | Variable-to-register mapping for debuggers | Already working; param info can be added alongside |
| ParamDef table (MetadataRow) | Formal parameter metadata in binary module | Currently defined but never populated |

## Architecture Patterns

### Current Register Layout Convention (from IL Spec §2.16.6)

```
r0 .. r(param_count - 1)    → Parameters
r(param_count) .. r(reg_count - 1) → Locals and temporaries
```

The **param_count** value is implicitly derived from the method body's first `param_count` register type declarations. Phase 31.2 established that parameters are pre-allocated at r0..r(param_count-1) during collection, stored in fn_param_map.

### Pattern 1: Metadata Completeness — Two-Pass Approach

**What:** Split collection into:
1. **Collection pass** (collect.rs): walk TypedAst, populate ModuleBuilder with row definitions
2. **Finalization pass** (module_builder.finalize): assign contiguous indices respecting list-ownership, transform handles to tokens

**When to use:** ParamDef rows must be added during collection, with sequence numbers (0-based ordinal) matching their source declaration order.

**Example:** (Phase 24 established this pattern)
```rust
// In collect_fn (collect.rs):
for (seq, param) in params.iter().enumerate() {
    let name_offset = builder.string_heap.intern(&param.name);
    let type_sig_offset = encode_type_sig(...);
    builder.add_param_def(method_handle, name_offset, type_sig_offset, seq as u16);
}

// In finalize (module_builder.rs):
// Params grouped under their parent method via list-ownership
let param_start = params[method_idx].start;
let param_end = params[method_idx + 1].start;
// Output rows param_start..param_end
```

### Pattern 2: Register Index Metadata — Annotating Method Signatures

**What:** Extend MethodDefRow to include param_count (u16) so tooling can read register layout without parsing bytecode.

**When to use:** Debuggers, IDE parameter hints, and reflection APIs need to know which registers hold parameters vs locals.

**Example:**
```rust
// Current MethodDefRow (bytes 1-26):
pub struct MethodDefRow {
    pub name: u32,       // string heap offset
    pub signature: u32,  // blob heap offset
    pub flags: u16,
    pub body_offset: u32,
    pub body_size: u32,
    pub reg_count: u16,  // ← already present
}

// Extended (add after reg_count):
pub struct MethodDefRow {
    // ... existing fields ...
    pub reg_count: u16,
    pub param_count: u16, // ← NEW: number of parameters (reg_count - num_locals)
}
```

### Anti-Patterns to Avoid

- **Deriving param_count from register type table**: Register types are optional (not emitted in release builds without debug flag). Param_count must be explicitly tracked.
- **Storing param_count only in ParamDef rows**: ParamDef is optional list-owned data; MethodDef must carry it for fast lookup.
- **Inferring parameters from method signature blob**: Signature blob encodes parameter types but not register indices. Register layout is runtime concern (§2.16.6).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Parameter sequence tracking | Custom (name, index) mapping in compiler | Existing fn_param_map + ParamDef sequence field | ParamDef.sequence (u16) is spec-defined; avoids off-by-one bugs across collection/finalization |
| Type encoding for params | Custom type blob format | Existing type_sig_offset infrastructure (Phase 24) | TypeRef encoding is unified (primitive tags, named types, generics); Phase 24 established correct encoding pattern |
| Register counting | Custom register allocation | Register pre-allocation in body emitter (Phase 31.2) | Parameter pre-allocation already working; extend it to metadata rather than reinvent |

**Key insight:** Parameter metadata is already threaded through the compiler (fn_param_map → body emission); Phase 39 just needs to emit the same data into the ParamDef table and extend MethodDef. Reimplementing would duplicate logic and risk divergence.

## Common Pitfalls

### Pitfall 1: ParamDef Sequence Mismatch

**What goes wrong:** ParamDef.sequence doesn't match source declaration order or parameter position in MethodDef.signature blob. Debuggers misalign parameter names to actual registers.

**Why it happens:** Easy to iterate method_body.registers (r0..rN) and assign sequence numbers in that order, but source parameters may have been reordered or inlined into signature blob differently.

**How to avoid:** Use fn_param_map iteration order (established during collect_fn before type-checking reorders them) as canonical. Assign sequence numbers 0, 1, 2, ... in that order, matching source position not register position.

**Warning signs:** Disassembler output shows `param[0] = r1` instead of `param[0] = r0`; IDE shows parameter hints misaligned with source.

### Pitfall 2: param_count ≠ fn_param_map.len()

**What goes wrong:** ParamDef rows emitted for (name, Ty) pairs but param_count stored in MethodDef is off-by-one (includes/excludes `self`, includes type parameters as params, etc.).

**Why it happens:** Self is implicit in methods but explicit in parameter lists; generic type parameters show up in signature but not in register file.

**How to avoid:** Keep param_count as the number of **runtime registers** allocated for parameters, not source-level parameters. For methods: count self + regular params (not type params). For free functions: count regular params only. Verify: `param_count == fn_param_map.len() + (has_self ? 1 : 0)`.

**Warning signs:** Register pre-allocation in body emitter (Phase 31.2) succeeds but runtime register layout diverges from metadata; method calls mis-count arguments.

### Pitfall 3: Type Signature Encoding in ParamDef

**What goes wrong:** ParamDef.type_sig points to blob heap but encoding doesn't match compiler's phase-24 TypeRef format (primitive tags, kind bytes, etc.). Disassembly shows "???" for parameter types.

**Why it happens:** Easy to store raw Ty enum values instead of using encode_type_ref which is already centralized in type_sig.rs.

**How to avoid:** Reuse Phase 24's `encode_type_ref()` for ParamDef.type_sig just as it's used for FieldDef.type_sig. Call it during collect_fn parameter iteration. Verify round-trip: disasm output shows correct parameter types.

**Warning signs:** Disassembler decode_method_sig fails or returns `Unknown` types for parameters.

## Code Examples

Verified patterns from implementation:

### Emitting ParamDef Entries (collect.rs pattern from Phase 24)

```rust
// Source: writ-compiler/src/emit/collect.rs pattern (adapted from collect_field)
fn collect_fn(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    methoddef_handles: &mut FxHashMap<DefId, MethodDefHandle>,
    _diags: &mut Vec<Diagnostic>,
) {
    let entry = def_map.get(def_id).unwrap();
    let decl = entry.decl.as_fn().unwrap();
    let sig = &decl.sig;

    // ... collect method header ...
    let method_handle = builder.add_method_def(...);
    methoddef_handles.insert(def_id, method_handle);

    // NEW: Iterate parameters and emit ParamDef rows
    for (seq, param) in sig.params.iter().enumerate() {
        // Skip 'self' — it's implicit in methods
        if param.name == "self" {
            continue;
        }

        let name_offset = builder.string_heap.intern(&param.name);
        let ty = /* look up Ty from typechecker */;
        let type_sig_offset = encode_type_ref(ty, interner, &mut builder.blob_heap);

        builder.add_param_def(
            method_handle,
            name_offset,
            type_sig_offset,
            seq as u16,  // sequence = source declaration order (excluding self)
        );
    }
}
```

### Extending MethodDefRow (metadata.rs)

```rust
// Source: writ-compiler/src/emit/metadata.rs
/// Table 7: MethodDef — Methods/functions defined here (extended).
#[derive(Debug, Clone)]
pub struct MethodDefRow {
    pub name: u32,       // string heap offset
    pub signature: u32,  // blob heap offset (includes param types + return type)
    pub flags: u16,
    pub body_offset: u32,
    pub body_size: u32,
    pub reg_count: u16,
    pub param_count: u16, // ← NEW: number of parameters occupying r0..r(param_count-1)
}
```

### Adding param_count During Finalization (module_builder.rs pattern)

```rust
// Source: writ-compiler/src/emit/module_builder.rs
// In finalize(), when converting MethodDefEntry → MethodDefRow:

for (handle, entry) in method_defs.iter().enumerate() {
    let param_count = fn_param_map
        .get(&entry.def_id.unwrap())
        .map(|params| params.len() as u16)
        .unwrap_or(0);

    let final_row = MethodDefRow {
        name: entry.row.name,
        signature: entry.row.signature,
        flags: entry.row.flags,
        body_offset: entry.row.body_offset,
        body_size: entry.row.body_size,
        reg_count: entry.row.reg_count,
        param_count,  // ← populated here
    };

    method_defs_final.push(final_row);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual param tracking in disassembler | ParamDef table emission + MethodDef.param_count | Phase 39 | Debuggers/IDEs gain reliable parameter metadata without bytecode parsing |
| Inferring param_count from first N register types | Explicit param_count in MethodDef | Phase 39 | Eliminates ambiguity; works with release builds (no debug flag) |
| No ParamDef table population | Full ParamDef emission during collection | Phase 39 | Formal parameter metadata now queryable at runtime |

**Deprecated/outdated:**
- Manual parameter list reconstruction from method body — no longer needed with ParamDef + param_count

## Open Questions

1. **Should self be counted in param_count?**
   - What we know: ParamDef.sequence excludes self (only for regular params)
   - What's unclear: Whether param_count in MethodDef includes self or not
   - Recommendation: Clarify via IL spec amendment. Proposal: param_count = number of registers r0..r(param_count-1) = len(params_including_self_if_method). This matches register layout convention exactly.

2. **Are generic type parameters separate from param_count?**
   - What we know: Generic type parameters exist (GenericParam rows) but don't occupy registers
   - What's unclear: Should they be reflected in ParamDef table?
   - Recommendation: No. param_count counts runtime registers only. Generic parameters belong in GenericParam table. ParamDef is for runtime method parameters (what occupies registers).

3. **Should param_count be serialized as u16 or stored inline in MethodDefRow?**
   - What we know: MethodDefRow is fixed-size; param_count is small (rarely >100)
   - What's unclear: Breaking change to binary module format if added to MethodDefRow
   - Recommendation: Check module format version. If this is format_version=1, will need version bump to 2. Coordinate with loader/disassembler updates.

## Validation Architecture

**Note:** workflow.nyquist_validation is false in .planning/config.json; skipping formal test mapping.

However, golden file tests (Phase 31+) provide implicit validation:
- Disassembler must correctly display emitted ParamDef rows (test: `writ-assembler` disasm_basic, disasm_round_trip)
- Parameter register pre-allocation must match metadata (test: all fn_*.writ golden files with parameters)
- IDE/debugger integrations can be validated post-phase by confirming parameter hints work end-to-end

## Sources

### Primary (HIGH confidence)

- **IL Spec §2.16.5, §2.16.6** — Method parameter and register layout requirements
  - MethodDef table structure (name, signature, flags, body info, reg_count)
  - ParamDef table (name, type_sig, sequence)
  - Register convention: r0..r(param_count-1) = parameters, rest = locals/temps
  - Source: `language-spec/spec/45_2_16_il_module_format.md` (latest)

- **writ-compiler Phase 31.2** — Parameter register pre-allocation established
  - fn_param_map populated during collection with source parameter names and types
  - Parameter registers (r0..rN) pre-allocated before body emission
  - Source: `.planning/phases/31.2-fix-register-convention-debug-info-and-parser-bugs/31.2-01-PLAN.md` (complete)

- **writ-compiler Phase 24** — Type signature encoding for metadata
  - encode_type_ref pattern for FieldDef.type_sig (blob heap offset)
  - Apply same pattern to ParamDef.type_sig
  - Source: `writ-compiler/src/emit/type_sig.rs` (verified)

- **writ-compiler Module Format** — Metadata table structures
  - ParamDefRow already defined but not populated
  - MethodDefRow currently lacks param_count
  - Source: `writ-compiler/src/emit/metadata.rs` (verified)

### Secondary (MEDIUM confidence)

- **writ-assembler disassembler.rs** — ParamDef reading and display
  - decode_method_sig reads signature blob but doesn't currently use ParamDef rows
  - Will need updates to emit parameter names alongside types
  - Source: `writ-assembler/src/disassembler.rs` lines 104-110 (verified)

## Metadata

**Confidence breakdown:**
- **Standard stack:** HIGH — IL spec defines exact requirements; compiler infrastructure already supports ParamDef emission
- **Architecture:** HIGH — Phase 31.2 established patterns; Phase 24 proved type encoding works
- **Pitfalls:** MEDIUM — Parameter matching pitfalls inferred from register layout conventions; self handling requires spec clarification

**Research date:** 2026-03-04
**Valid until:** 2026-04-04 (IL spec stable, no major changes expected in 1 month)
