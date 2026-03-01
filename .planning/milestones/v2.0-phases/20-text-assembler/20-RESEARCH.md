# Phase 20: Text Assembler - Research

**Researched:** 2026-03-02
**Domain:** Text IL parsing and two-pass assembly into binary IL modules
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Instruction mnemonics:** Case-insensitive; accept `ADD_I` or `add_i`, emit uppercase in canonical form. Names match spec §4.2 exactly.
- **Register notation:** `r` prefix with number: `r0`, `r1`, `r2`, ...
- **Label syntax:** Dot prefix + colon on definition (`.loop:` on its own line), dot prefix for reference (`BR .loop`). Disambiguated from directives by trailing colon.
- **Comment syntax:** `//` line comments only. No block comments needed for IL.
- **Directive format:** Dot-prefixed, CIL-style: `.module`, `.type`, `.field`, `.method`, `.contract`, `.impl`, `.reg`, `.extern`, `.global`, etc. Contextual (`.field` inside `.type` block, `.reg` inside `.method` block).
- **Type references in text:** Human-readable names `int`, `float`, `bool`, `string`, `void` for primitives; `MyNamespace.MyType` for named types; `Array<int>`, `Option<MyType>` for generics; `blob(0x10 00 00 00 05)` hex escape hatch. Assembler resolves to TypeRef blob encoding.
- **Metadata token references:** Qualified name-based: `CALL r0, MyType::my_method, r1, 2`; cross-module: `[OtherModule]Namespace.Type::method`; numeric fallback: `token(0x07_00000A)`.
- **File extension:** `.writil` for text IL source files.
- **Error diagnostics:** `line:column + descriptive error message`; multiple error collection (don't fail-fast); format: `error: <message> at line <N>, column <M>`.

### Claude's Discretion

- Exact directive syntax details (keyword arguments, block delimiters, etc.)
- Parser implementation strategy (hand-written vs parser combinator)
- String literal escaping rules in text IL
- Exact method body / register declaration syntax within `.method` blocks
- Two-pass architecture details (what's collected in pass 1 vs resolved in pass 2)
- Whether to use `{...}` blocks or indentation for nesting directives

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| ASM-01 | Human-readable text IL format with directives for modules, types, fields, methods | Grammar design, directive set, two-level nesting model; `.writil` files with `{...}` blocks |
| ASM-02 | Two-pass assembler resolves forward label references correctly | Two-pass architecture: pass 1 collects label byte offsets, pass 2 patches branch operands |
| ASM-03 | Assembler produces spec-valid binary modules from text IL input | `ModuleBuilder` integration, TypeRef blob encoding, name-to-token resolution |
| ASM-04 | Round-trip test validates text IL -> binary -> disassemble -> compare | Placeholder disassembler stub required; or: assemble twice and compare binaries from identical source |
</phase_requirements>

---

## Summary

Phase 20 builds a new `writ-assembler` crate: a hand-written parser for `.writil` text files that produces `Module` structs via the existing `writ-module::ModuleBuilder` API, then serializes to binary. The phase is entirely self-contained within its own new crate; no existing crates are modified except to be depended on.

The core technical work divides into three pieces: (1) a lexer and recursive descent parser for the text format, (2) a two-pass assembler that builds the symbol table in pass 1 and resolves forward label references in pass 2, and (3) a name-resolution layer that maps human-readable type names and qualified method/field names to metadata tokens.

The round-trip requirement (ASM-04) is nuanced: a true round-trip requires the Phase 21 disassembler. For Phase 20, this is satisfied by a narrower guarantee: assemble a `.writil` file to binary, read that binary back with `Module::from_bytes()`, and verify no decode error. A canonical-output stub can emit the exact text that was assembled from, allowing a partial round-trip test. The planner should clarify whether Phase 20 must stub the disassembler or whether ASM-04 is satisfied by the binary-roundtrip-through-reader test alone.

**Primary recommendation:** New `writ-assembler` crate, hand-written recursive-descent parser, `{...}` blocks for directive nesting, two-pass label resolution. No external parser crate needed — the grammar is simple enough.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `writ-module` | workspace | `ModuleBuilder`, `Instruction`, `Module::to_bytes()` — the sole output target | Already exists; assembler is a producer into this API |
| Rust stdlib (`std`) | — | `HashMap` for symbol tables, `String`/`Vec` for AST, `std::io::Read` for input | No external deps needed for this complexity level |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `thiserror` | 2.0 | Structured error types with `#[derive(Error)]` | Matches the pattern in `writ-module::error` and `writ-runtime` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-written recursive descent | `nom` / `pest` / `winnow` | Parser combinators add a heavy dependency for a simple line-oriented grammar; hand-written is ~500 lines and fully debuggable |
| `{...}` nesting | Indentation-based | Braces are unambiguous in error recovery and canonical output; indentation parsing adds complexity for no benefit in a developer-only tool |
| `HashMap<String, u32>` symbol table | `IndexMap` | Insertion-order not needed; plain `HashMap` is sufficient |

**Installation:**
```bash
# No new external crates. thiserror is already used in the workspace.
# Cargo.toml for writ-assembler:
[dependencies]
writ-module = { path = "../writ-module" }
thiserror = "2.0"
```

---

## Architecture Patterns

### Recommended Project Structure

```
writ-assembler/
├── Cargo.toml
└── src/
    ├── lib.rs            # pub fn assemble(src: &str) -> Result<Module, Vec<AssembleError>>
    ├── lexer.rs          # Token enum + tokenize() function
    ├── ast.rs            # AST structs: AsmModule, AsmType, AsmMethod, AsmInstruction, etc.
    ├── parser.rs         # Recursive-descent parser: parse() -> AsmModule
    ├── resolver.rs       # Name resolution: type names -> TypeRef blobs, qualified names -> tokens
    ├── assembler.rs      # Two-pass: pass1() builds label map, pass2() emits Instruction stream
    ├── error.rs          # AssembleError with line:col, multi-error collection
    └── tests/
        ├── asm_basic.rs        # Smoke tests: assemble minimal .writil
        ├── asm_labels.rs       # Forward label resolution tests
        ├── asm_round_trip.rs   # Assemble -> Module::from_bytes() tests
        └── asm_errors.rs       # Error recovery tests
```

### Pattern 1: Lexer-Parser-Assembler Pipeline

**What:** Three-phase pipeline: (1) tokenize text to `Vec<Token>`, (2) parse tokens to `AsmModule` AST, (3) assemble AST to `Module`.
**When to use:** Whenever separating concerns improves testability — each phase can be unit-tested independently.

```rust
// lib.rs
pub fn assemble(src: &str) -> Result<writ_module::Module, Vec<AssembleError>> {
    let tokens = lexer::tokenize(src)?;
    let ast = parser::parse(&tokens)?;
    assembler::assemble(ast)
}

// error.rs
#[derive(Debug, thiserror::Error)]
pub enum AssembleError {
    #[error("error: {message} at line {line}, column {col}")]
    Error { message: String, line: u32, col: u32 },
}
```

### Pattern 2: Two-Pass Label Resolution

**What:** Method body assembly proceeds in two passes over the instruction list in the AST.
- **Pass 1:** Iterate instructions, compute byte offset of each instruction (each `Instruction` variant has a fixed or computable size via `encode()` length), record `label_name -> byte_offset` in a `HashMap`.
- **Pass 2:** Iterate instructions again, for each branch instruction substitute the label name with `target_offset - current_offset` as an `i32` relative offset, then encode to the code buffer.

**When to use:** Always for any assembler with forward jumps. This is the canonical two-pass approach used by NASM, MASM, and CIL assembler implementations.

```rust
// assembler.rs (sketch)
fn assemble_method_body(
    instrs: &[AsmInstr],
    ctx: &ResolutionCtx,
) -> Result<(Vec<u8>, u32), Vec<AssembleError>> {
    // Pass 1: compute label byte offsets
    let mut label_map: HashMap<String, u32> = HashMap::new();
    let mut byte_offset: u32 = 0;
    for instr in instrs {
        match instr {
            AsmInstr::Label(name) => { label_map.insert(name.clone(), byte_offset); }
            AsmInstr::Instr(i) => {
                let mut dummy = Vec::new();
                i.encode(&mut dummy).unwrap(); // size probe
                byte_offset += dummy.len() as u32;
            }
        }
    }

    // Pass 2: emit code with resolved offsets
    let mut code = Vec::new();
    let mut current_offset: u32 = 0;
    for instr in instrs {
        if let AsmInstr::Instr(i) = instr {
            // resolve label references in branch operands
            i.encode(&mut code).unwrap();
            current_offset += ...;
        }
    }
    Ok((code, byte_offset))
}
```

**Key insight on offset semantics:** Branch offsets in the spec are relative to the byte immediately *after* the branch instruction. This must be computed as `target_offset - (current_offset + instruction_size)`. Verify this against §3.6 (control flow instructions) in the spec.

### Pattern 3: Name Resolution Context

**What:** A struct that carries lookup tables built from the parsed module declarations, used during instruction encoding to resolve `MyType::my_method` to a `MetadataToken` and `Array<int>` to a blob offset.

```rust
struct ResolutionCtx<'a> {
    builder: &'a mut ModuleBuilder,
    // Maps "TypeName" (or "Namespace.TypeName") -> MetadataToken
    type_map: HashMap<String, MetadataToken>,
    // Maps "TypeName::method_name" -> MetadataToken
    method_map: HashMap<String, MetadataToken>,
    // Maps "TypeName::field_name" -> MetadataToken
    field_map: HashMap<String, MetadataToken>,
    // String heap interning cache
    string_cache: HashMap<String, u32>,
}
```

### Pattern 4: Text Format Grammar

The grammar is simple and line-oriented. Braces (`{...}`) delimit blocks. Based on CIL assembly conventions and the locked decisions:

```
// Module-level skeleton
.module "name" "1.0.0" {
    .extern "OtherModule" ">=1.0.0"

    .type "MyNamespace.MyStruct" struct {
        .field "x" int pub
        .field "y" float pub
    }

    .contract "MyNamespace.IFoo" {
        .method "foo" (int) -> void slot 0
    }

    .impl MyNamespace.MyStruct : MyNamespace.IFoo {
        .method "foo" (int r0) -> void regs 2 {
            .reg r0 int
            .reg r1 void
            NOP
            RET_VOID
        }
    }

    .method "global_fn" () -> int regs 1 {
        .reg r0 int
        LOAD_INT r0, 42
        RET r0
    }
}
```

Labels on their own line, dot-prefixed with colon:
```
    .loop:
    ADD_I r0, r0, r1
    BR_FALSE r2, .done
    BR .loop
    .done:
    RET r0
```

### Anti-Patterns to Avoid

- **Fail-fast on first error:** The spec requires multiple error collection. The parser should collect errors into a `Vec<AssembleError>` and continue parsing after recoverable errors (unknown directives, malformed operands).
- **Resolving forward references in a single pass:** Without pass 1's label map, branch operands for forward labels are unknown at emit time. Single-pass with backpatching is an alternative but adds complexity.
- **Encoding TypeRef blobs on the fly during parsing:** TypeRef encoding requires the blob heap to be available. Defer all blob writes to the assembly phase, not the parse phase. The AST stores `AsmTypeRef` (a structured enum), not raw bytes.
- **Conflating label offsets with instruction indices:** Labels must track *byte offsets* in the code buffer, not instruction sequence numbers, because instructions have variable sizes.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Binary serialization | Custom serializer | `writ_module::Module::to_bytes()` | Already tested, spec-correct; assembler produces a `Module`, calls `to_bytes()` |
| TypeRef blob encoding | Custom encoder | `writ_module::heap::write_blob()` + existing blob patterns | Blob format is specified in §2.15.3; patterns visible in test fixtures (`&[0x00]` for int, etc.) |
| Metadata token construction | Custom bit-packing | `writ_module::MetadataToken::new(table_id, row_index)` | Newtype enforces 1-based indexing; assembler calls `MetadataToken::new(TableId::MethodDef.as_u8(), idx)` |
| Module construction | Direct struct mutation | `writ_module::ModuleBuilder` | Handles `field_list`/`method_list` bookkeeping automatically; builder pattern already established |

**Key insight:** The assembler is a *frontend* to `ModuleBuilder`. It parses text, resolves names, and calls the builder API. The builder handles all heap management and table list bookkeeping.

---

## Common Pitfalls

### Pitfall 1: Branch Offset Semantics

**What goes wrong:** Branch offset is computed from wrong anchor point, causing jumps to land one instruction early or late.
**Why it happens:** The spec defines offsets as relative to the byte *after* the branch instruction, not the byte *of* the branch instruction. Assemblers that use the start-of-instruction as anchor are off by the instruction size.
**How to avoid:** In pass 2, compute `offset = label_byte_offset - (current_byte_offset + instruction_byte_size)`.
**Warning signs:** Forward jump tests land 6-8 bytes off (the size of a branch instruction). Backward loops execute extra iterations.

### Pitfall 2: Variable-Size Instructions in Label Offset Calculation

**What goes wrong:** Pass 1 assigns incorrect byte offsets to labels after a `SWITCH` instruction.
**Why it happens:** `SWITCH` has variable size: `6 + 4 * n` bytes where `n` is the number of cases. If pass 1 probes size by calling `encode()` on a placeholder, it needs the actual case count.
**How to avoid:** The AST must carry the full `SWITCH` case list before pass 1. Do not use a fixed-size placeholder for `SWITCH`.
**Warning signs:** Labels after `SWITCH` instructions resolve to wrong offsets.

### Pitfall 3: TypeRef Encoding for Complex Types

**What goes wrong:** `Array<int>` or `Option<MyType>` produces wrong blob bytes, causing the assembled module to fail `Module::from_bytes()` validation.
**Why it happens:** The encoding is recursive (§2.15.3): `Array<int>` = `[0x20, 0x01]`, `Option<MyType>` = `[0x11, type_spec_idx_as_u32_le]`. Getting the nesting wrong or forgetting the length prefix causes decode errors.
**How to avoid:** Write a dedicated `encode_type_ref(ast_type: &AsmTypeRef, blob_heap: &mut Vec<u8>) -> u32` helper that handles all cases and is tested independently.
**Warning signs:** `Module::from_bytes()` returns `DecodeError::InvalidTypeRefKind` for assembled modules.

### Pitfall 4: `field_list` / `method_list` Bookkeeping Order

**What goes wrong:** A type's `field_list` points to wrong row, causing field lookups to be off by one.
**Why it happens:** `ModuleBuilder::add_type_def()` captures `field_list` as `current_field_count + 1` at the time the type is added. Fields must be added *after* the type. If you declare all types first and then add fields, the `field_list` indices are wrong.
**How to avoid:** Use `ModuleBuilder` as intended: add a type, then immediately add its fields and methods, then add the next type. The AST walk must process types in order.
**Warning signs:** `field_defs[0]` belongs to type 2, not type 1. Field token lookup by name returns wrong token.

### Pitfall 5: String Heap vs. LoadString Index

**What goes wrong:** `LOAD_STRING r0, "hello"` resolves to a blob heap offset instead of a string heap offset.
**Why it happens:** `LoadString.string_idx` is a string heap offset, not a token or blob offset. The assembler must intern the string into the string heap and use that offset as the operand.
**How to avoid:** Maintain a `string_literal_cache: HashMap<String, u32>` that interns literals into the string heap, separate from blob heap writes.
**Warning signs:** `LoadString` at runtime produces garbage or panics.

### Pitfall 6: Multi-Error Collection in Parser

**What goes wrong:** Parser panics or returns only the first error on malformed input.
**Why it happens:** The locked decision requires collecting all errors. A simple recursive-descent parser that returns `Result<T, Error>` on first failure must be restructured to accumulate errors.
**How to avoid:** Use a `parse_errors: Vec<AssembleError>` field on the parser struct. On recoverable errors (unknown directive, wrong token), push to the list and call a `synchronize()` method that skips to the next `}` or newline boundary.
**Warning signs:** `cargo test` failure on multi-error test cases.

---

## Code Examples

### TypeRef Blob Encoding (from spec §2.15.3 + test fixtures)

```rust
// Source: writ-module/tests/round_trip.rs patterns + spec §2.15.3
fn encode_primitive(tag: u8) -> Vec<u8> {
    vec![tag]  // 0x01=int, 0x02=float, 0x03=bool, 0x04=string, 0x00=void
}

fn encode_array_of(elem_bytes: &[u8]) -> Vec<u8> {
    let mut v = vec![0x20u8];  // Array tag
    v.extend_from_slice(elem_bytes);
    v
}
// Array<int> = [0x20, 0x01]
// Array<Array<int>> = [0x20, 0x20, 0x01]
```

### ModuleBuilder Integration (from existing builder_tests.rs pattern)

```rust
// Source: writ-module/tests/builder_tests.rs
let mut builder = ModuleBuilder::new("my_module");
builder.version("1.0.0");

// MUST add type before its fields/methods
let type_tok = builder.add_type_def("MyType", "ns", TypeDefKind::Struct.as_u8(), 0);
let _field = builder.add_field_def("x", &[0x01], 0);  // int field

// Build method body
let mut code = Vec::new();
Instruction::LoadInt { r_dst: 0, value: 42 }.encode(&mut code).unwrap();
Instruction::RetVoid.encode(&mut code).unwrap();

let body = MethodBody {
    register_types: vec![0],  // blob heap offsets for register types
    code,
    debug_locals: Vec::new(),
    source_spans: Vec::new(),
};
builder.add_method("init", &[0x00], 0, 1, body);

let module = builder.build();
let bytes = module.to_bytes().unwrap();
```

### MetadataToken Construction (from token.rs)

```rust
// Source: writ-module/src/token.rs
let method_token = MetadataToken::new(TableId::MethodDef.as_u8(), 3); // row 3
let type_ref_token = MetadataToken::new(TableId::TypeRef.as_u8(), 1);

// For instructions that take u32 token values:
let raw: u32 = method_token.0;  // used as method_idx in Call instruction
```

### Instruction Size Probing (for pass 1 offset calculation)

```rust
// Each instruction's encoded size can be probed by encoding to a temporary Vec
fn instruction_byte_size(instr: &Instruction) -> u32 {
    let mut buf = Vec::new();
    instr.encode(&mut buf).unwrap();
    buf.len() as u32
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `.il` extension (CIL) | `.writil` extension | Phase 20 decision | Writ-specific; avoids confusion with MSIL |
| Text format as part of spec | Spec §2.4 explicitly defers format to tooling | During IL spec design | Assembler has full freedom over syntax |

---

## Open Questions

1. **ASM-04 scope: does Phase 20 require a disassembler stub?**
   - What we know: The success criterion says "text IL -> binary -> disassemble -> compare". Phase 21 is the disassembler.
   - What's unclear: Can ASM-04 be satisfied in Phase 20 by assemble -> `Module::from_bytes()` (proving spec validity) without actual disassembly? Or does the phase need a minimal disassembler stub?
   - Recommendation: Scope ASM-04 as "assemble, read back with `Module::from_bytes()`, verify no decode error". Full round-trip to text is Phase 21's job. The plan should note this explicitly.

2. **Block delimiter style: `{...}` vs keyword-terminated**
   - What we know: This is Claude's discretion. CIL uses `.end method`, `.end class` terminators. Most modern formats use `{...}`.
   - What's unclear: The planner must choose one.
   - Recommendation: Use `{...}` blocks. Cleaner for multi-pass parsing; canonical output is unambiguous; familiar to Rust developers.

3. **`.reg` directive syntax: all registers declared up front vs inline**
   - What we know: The spec has `reg_count` in the MethodDef and a `register_types` array in the body. The `.reg` directive must declare all registers with their types.
   - What's unclear: Does `.reg r0 int` appear inside `.method` before instructions? Or as a sub-block?
   - Recommendation: Declare all registers in a contiguous block at the top of the method body, before any instructions:
     ```
     .method "foo" () -> int {
         .regs {
             r0 int
             r1 float
         }
         LOAD_INT r0, 42
         RET r0
     }
     ```

4. **`.impl` directive: method ownership model**
   - What we know: `ImplDef` has a `method_list` that is the 1-based index of the first MethodDef row for that impl's methods.
   - What's unclear: Are impl methods emitted inside the `.impl` block (and added to the MethodDef table as part of impl processing) or declared separately with a reference?
   - Recommendation: Inline impl methods inside the `.impl` block. The assembler adds them to the MethodDef table in sequence during the `.impl` block processing. This keeps locality and avoids forward references to impl methods.

---

## Validation Architecture

*(config.json does not have `workflow.nyquist_validation: true` — section included as standard practice)*

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (no external framework needed) |
| Config file | none — standard Cargo test runner |
| Quick run command | `cargo test -p writ-assembler` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ASM-01 | Assemble `.writil` with all directives: `.module`, `.type`, `.field`, `.contract`, `.impl`, `.method`, `.reg` | integration | `cargo test -p writ-assembler asm_basic` | No — Wave 0 |
| ASM-02 | Forward label reference in BR resolves to correct byte offset | unit | `cargo test -p writ-assembler asm_labels` | No — Wave 0 |
| ASM-03 | Assembled binary passes `Module::from_bytes()` without error | integration | `cargo test -p writ-assembler asm_round_trip` | No — Wave 0 |
| ASM-04 | Assemble text -> binary -> `Module::from_bytes()` -> no error (narrowed scope) | integration | `cargo test -p writ-assembler asm_round_trip` | No — Wave 0 |

### Wave 0 Gaps

- [ ] `writ-assembler/` crate — does not exist yet; must be created and added to workspace
- [ ] `writ-assembler/src/lib.rs` — covers public API
- [ ] `writ-assembler/tests/asm_basic.rs` — covers ASM-01
- [ ] `writ-assembler/tests/asm_labels.rs` — covers ASM-02
- [ ] `writ-assembler/tests/asm_round_trip.rs` — covers ASM-03 and ASM-04

---

## Sources

### Primary (HIGH confidence)

- `writ-module/src/instruction.rs` — All 91 opcodes with `encode()`/`decode()` implementations; instruction sizes computable by encoding to `Vec<u8>`
- `writ-module/src/builder.rs` — `ModuleBuilder` API, `add_type_def`/`add_field_def`/`add_method` call ordering requirements
- `writ-module/src/tables.rs` — All 21 table row structs, `TableId` enum, `TypeDefKind`
- `writ-module/src/module.rs` — `MethodBody` struct: `register_types: Vec<u32>`, `code: Vec<u8>`
- `language-spec/spec/44_2_15_il_type_system.md` — TypeRef blob encoding: primitive tags 0x00-0x04, kind bytes 0x10/0x11/0x12/0x20/0x30
- `language-spec/spec/45_2_16_il_module_format.md` — Module binary layout, 21 tables, `body_offset`/`body_size` pattern
- `.planning/phases/20-text-assembler/20-CONTEXT.md` — All locked decisions

### Secondary (MEDIUM confidence)

- `writ-module/tests/round_trip.rs` — Shows correct blob heap patterns (`&[0x00]` = int, `&[0x01]` = float-like, `body_size = 0` for no-body methods)
- `writ-module/tests/builder_tests.rs` — Confirms `add_type_def` before `add_field_def` ordering requirement
- CIL (Common Intermediate Language) assembler conventions — confirmed as the basis for directive style in CONTEXT.md

### Tertiary (LOW confidence)

None — all findings verified against source code.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — verified against Cargo.toml and existing crate code
- Architecture: HIGH — builder API is fully understood from source code inspection
- Pitfalls: HIGH — derived from actual code analysis (instruction sizes, builder ordering, TypeRef encoding)

**Research date:** 2026-03-02
**Valid until:** Stable until spec changes (file format is frozen post-Phase 16)
