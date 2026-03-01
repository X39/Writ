# Phase 20: Text Assembler - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

`writ-assembler` crate: human-readable text IL format and two-pass assembler producing spec-valid binary modules. A developer writes text IL, runs the assembler, and gets a binary module that the VM can execute. Forward label references resolve correctly via two-pass assembly. Round-trip test validates text IL -> binary -> disassemble -> compare. Disassembler itself is Phase 21.

</domain>

<decisions>
## Implementation Decisions

### Instruction mnemonics
- Case-insensitive: accept both `ADD_I` and `add_i`, emit as uppercase in canonical form
- Mnemonic names match spec exactly (§4.2 opcode assignment table): `NOP`, `CRASH`, `MOV`, `LOAD_INT`, `ADD_I`, `CALL_VIRT`, etc.

### Register notation
- Numbered with `r` prefix: `r0`, `r1`, `r2`, ...
- Matches spec's abstract register naming convention (r_dst, r_src)

### Label syntax
- Dot prefix with colon for definition: `.loop:` on its own line
- Dot prefix for reference: `BR .loop`
- Disambiguated from directives by trailing colon (`.loop:` = label, `.method` = directive)

### Comment syntax
- `//` line comments, matching Writ source language convention
- No block comments needed for IL

### Directive format
- Dot-prefixed directives, CIL-style: `.module`, `.type`, `.field`, `.method`, `.contract`, `.impl`, `.reg`, `.extern`, `.global`, etc.
- Directives are contextual (e.g., `.field` inside `.type` block, `.reg` inside `.method` block)

### Type references in text
- Human-readable names: `int`, `float`, `bool`, `string`, `void` for primitives
- Named types by qualified name: `MyNamespace.MyType`
- Generic instantiation: `Array<int>`, `Option<MyType>`
- Hex escape hatch for edge cases: `blob(0x10 00 00 00 05)` for raw blob encoding
- Assembler resolves names to TypeRef blob encoding

### Metadata token references
- Qualified name-based: `CALL r0, MyType::my_method, r1, 2`
- Cross-module: `[OtherModule]Namespace.Type::method`
- Numeric fallback: `token(0x07_00000A)` for raw token references
- Assembler resolves names to metadata tokens via lookup

### File extension
- `.writil` for text IL source files
- Binary modules use existing `.writilb` or whatever Phase 16 established

### Error diagnostics
- Line:column + descriptive error message format
- Multiple error collection — report all errors in a single pass, don't fail-fast
- Error format: `error: <message> at line <N>, column <M>`

### Claude's Discretion
- Exact directive syntax details (keyword arguments, block delimiters, etc.)
- Parser implementation strategy (hand-written vs parser combinator)
- String literal escaping rules in text IL
- Exact method body / register declaration syntax within `.method` blocks
- Two-pass architecture details (what's collected in pass 1 vs resolved in pass 2)
- Whether to use `{...}` blocks or indentation for nesting directives

</decisions>

<specifics>
## Specific Ideas

- Follow CIL conventions for directive style (dot-prefixed like `.method`, `.field`, `.type`)
- Spec §2.4 explicitly says "text disassembly is a tooling concern, not part of the spec" — all format design is implementation choice
- All 91 spec mnemonics (§4.2) are the canonical instruction names, case-insensitive matching
- Phase 21 (Disassembler) will need to emit canonical text IL that can round-trip — keep format unambiguous

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-module::ModuleBuilder`: Fluent API for programmatic module construction with all 21 table types — assembler feeds into this
- `writ-module::Instruction` enum: All 91 opcodes with typed operands — assembler maps text mnemonics to these variants
- `writ-module::Module::to_bytes()` / `from_bytes()`: Binary serialization already implemented — assembler produces Module, serializes
- `writ-module::MetadataToken`: Newtype with table ID + row index — assembler resolves names to these
- `writ-module::MethodBody`: Code bytes + register types + debug info — assembler constructs these

### Established Patterns
- Builder pattern: `ModuleBuilder::new("name").version("1.0.0").add_type_def(...)` — assembler calls this chain
- 1-based indexing: MetadataToken(0) = null, tables start at row 1
- Blob heap: TypeRef encodings stored as byte sequences with length prefix
- String heap: Length-prefixed UTF-8 strings, offset 0 = null/empty

### Integration Points
- `writ-module` crate is the sole output dependency — assembler produces a `Module` struct
- The assembled binary module must be loadable by `writ-runtime::loader` (Domain::load_module)
- Phase 21 disassembler will consume the same Module struct and emit text IL — format must be canonical

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 20-text-assembler*
*Context gathered: 2026-03-02*
