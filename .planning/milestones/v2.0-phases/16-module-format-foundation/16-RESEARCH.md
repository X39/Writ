# Phase 16: Module Format Foundation - Research

**Researched:** 2026-03-01
**Domain:** Binary format serialization / deserialization in Rust; custom file format reader/writer; metadata table modeling
**Confidence:** HIGH

## Summary

Phase 16 creates the `writ-module` crate — a pure-data, standalone Rust crate that owns the binary representation of a Writ IL module. The spec (§2.16) is fully written and unambiguous, defining a 200-byte header, 21 fixed-row metadata tables, string/blob heaps, method bodies, a MetadataToken u32 newtype with 1-based indexing, and 91 opcodes across 16 categories with 8 distinct instruction shapes (including several var-layout instructions).

The implementation approach is manual binary I/O using Rust's standard library (`std::io::Cursor`, `Vec<u8>`, native `to_le_bytes`/`from_le_bytes`), supplemented by `byteorder 1.5` for ergonomic multi-byte reads/writes over `impl Read`. No derive macro, no codec crate, and no external binary format library is needed — the spec is precise enough that hand-written encode/decode is the safest approach. `thiserror 2.0` is already in the workspace for error types; the workspace's existing `insta` snapshot crate covers round-trip tests.

The `ModuleBuilder` should be a plain Rust struct with mutating methods (not a typestate builder) since the spec allows partially-populated tables and the primary consumer is test authoring where ergonomics matter more than compile-time completeness guarantees.

**Primary recommendation:** Hand-write the reader/writer using `std::io::Cursor` + `byteorder 1.5` for LE reads/writes; model each metadata table as a `Vec<RowStruct>`; implement `MetadataToken` as a `u32` newtype with `table_id()` and `row_index()` decomposition; make `ModuleBuilder` a plain `mut` fluent builder returning `Module`; test with round-trip byte equality assertions and a few golden binary fixtures.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Crate placement**
- New `writ-module` crate in the workspace — standalone with no VM or runtime dependencies
- Both `writ-runtime` and `writ-compiler` (and future `writ-assembler`) will depend on it
- Pure-data crate: defines types, binary format, and builder — no execution logic

**Module API shape**
- Unified `Module` struct that holds the in-memory representation of a complete IL module
- `Module::from_bytes(bytes)` to deserialize from binary format
- `module.to_bytes()` to serialize back to binary format
- Round-trip identity: `write → read → write` produces identical bytes

**ModuleBuilder API**
- Fluent builder pattern for programmatic construction of modules
- Chained method calls: `ModuleBuilder::new("my_mod").add_type(...).add_method(...).build()`
- Builder returns a `Module` — same type as the reader produces
- Primary use case: test authoring and future assembler/compiler backends

### Claude's Discretion

- Error handling model (rich enum vs simple error — pick what best serves downstream VM, assembler, and test consumers)
- Internal data representation choices (how to model 21 metadata tables, heap storage, newtype wrappers)
- Instruction enum structure (flat vs categorized, operand representation per variant)
- Test strategy details (round-trip tests, snapshot approach, golden binaries)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope

</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| MOD-01 | Module reader can parse a spec-compliant binary module (200-byte header, 21 metadata tables, string/blob heaps, method bodies) | §2.16 spec is complete; manual reader with Cursor<&[u8]> + byteorder is the pattern |
| MOD-02 | Module writer can produce a spec-compliant binary module from in-memory representation | Same approach; write to Vec<u8> via byteorder WriteBytesExt |
| MOD-03 | ModuleBuilder API can programmatically construct valid IL modules for test authoring | Plain fluent builder (mut methods, returns Self) building Module; no typestate needed |
| MOD-04 | Instruction enum represents all 91 opcodes with encode/decode round-trip correctness | 8 instruction shapes identified; var-layout instructions need per-opcode match arms; opcode u16 fully specified |
| MOD-05 | MetadataToken newtype enforces 1-based indexing (0 = null token) | u32 newtype with table_id()/row_index() methods; lookup returns Option<&Row> |
| MOD-06 | Module reader/writer round-trip produces identical bytes (write → read → write = identical bytes) | Deterministic serialization; heap deduplication must be order-preserving on round-trip |

</phase_requirements>

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `byteorder` | 1.5.0 | LE multi-byte reads/writes over `impl Read`/`impl Write` | Ergonomic, zero-copy, well-tested; Rust stdlib `to_le_bytes` works for isolated calls but `byteorder` is cleaner over streaming I/O |
| `thiserror` | 2.0.18 | Derive macro for `ModuleError` enum | Already in workspace (writ-compiler); generates Display + Error + From impls from attributes |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `std::io::Cursor` | stdlib | Read cursor over `&[u8]` or `Vec<u8>` | Reader wraps `Cursor<&[u8]>`; writer builds into `Cursor<Vec<u8>>` |
| `insta` | 1.x | Snapshot testing (RON format) | Already in workspace; use for struct snapshots after deserializing known binary |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `byteorder` + manual IO | `binrw` derive macro | binrw reduces boilerplate but generates opaque code; the spec has enough custom logic (var-layout instructions, heap offset resolution) that manual decoding is more transparent |
| `byteorder` + manual IO | `nom` parser combinators | nom is good for unknown-length formats; our format has fixed-size rows and explicit sizes, making positional reads simpler and more direct |
| `byteorder` + manual IO | Rust stdlib `to_le_bytes`/`from_le_bytes` directly | Valid for writer; for reader, `byteorder::ReadBytesExt` is cleaner than manual 4-byte reads |
| Plain `Vec<RowStruct>` for tables | `indexmap` or hash tables | Tables are positionally addressed (1-based index); Vec is exactly right; no need for key-based lookup at this layer |

**Installation:**
```toml
# In writ-module/Cargo.toml
[dependencies]
byteorder = "1.5"
thiserror = "2.0"

[dev-dependencies]
insta = { version = "1", features = ["ron"] }
```

---

## Architecture Patterns

### Recommended Project Structure

```
writ-module/
├── src/
│   ├── lib.rs              # Public re-exports: Module, ModuleBuilder, MetadataToken, Instruction, error types
│   ├── module.rs           # Module struct, from_bytes / to_bytes
│   ├── builder.rs          # ModuleBuilder struct and fluent API
│   ├── tables.rs           # All 21 row structs (TypeDefRow, MethodDefRow, etc.)
│   ├── token.rs            # MetadataToken newtype
│   ├── instruction.rs      # Instruction enum (all 91 opcodes)
│   ├── reader.rs           # Binary deserialization (private)
│   ├── writer.rs           # Binary serialization (private)
│   └── error.rs            # ModuleError, DecodeError, EncodeError
└── tests/
    ├── round_trip.rs       # MOD-06: write→read→write byte equality
    ├── token_tests.rs      # MOD-05: MetadataToken null/1-based behavior
    ├── instruction_tests.rs# MOD-04: all 91 opcodes encode/decode
    └── cases/              # Golden binary fixtures (optional)
```

### Pattern 1: Module Struct as Owned Tables

**What:** `Module` holds all 21 tables as `Vec<Row>` plus two heap vecs. Reader populates them; writer serializes them. No lazy parsing, no arena — this is a pure in-memory representation.

**When to use:** Always. The module format is meant for complete load-at-once use.

```rust
pub struct Module {
    pub header: ModuleHeader,
    pub string_heap: Vec<u8>,    // raw bytes; strings are looked up by offset
    pub blob_heap: Vec<u8>,      // raw bytes; blobs looked up by offset
    // 21 tables:
    pub module_defs: Vec<ModuleDefRow>,
    pub module_refs: Vec<ModuleRefRow>,
    pub type_defs: Vec<TypeDefRow>,
    pub type_refs: Vec<TypeRefRow>,
    pub type_specs: Vec<TypeSpecRow>,
    pub field_defs: Vec<FieldDefRow>,
    pub field_refs: Vec<FieldRefRow>,
    pub method_defs: Vec<MethodDefRow>,
    pub method_refs: Vec<MethodRefRow>,
    pub param_defs: Vec<ParamDefRow>,
    pub contract_defs: Vec<ContractDefRow>,
    pub contract_methods: Vec<ContractMethodRow>,
    pub impl_defs: Vec<ImplDefRow>,
    pub generic_params: Vec<GenericParamRow>,
    pub generic_constraints: Vec<GenericConstraintRow>,
    pub global_defs: Vec<GlobalDefRow>,
    pub extern_defs: Vec<ExternDefRow>,
    pub component_slots: Vec<ComponentSlotRow>,
    pub locale_defs: Vec<LocaleDefRow>,
    pub export_defs: Vec<ExportDefRow>,
    pub attribute_defs: Vec<AttributeDefRow>,
    pub method_bodies: Vec<MethodBody>,   // indexed by MethodDef row order
}
```

### Pattern 2: MetadataToken Newtype

**What:** `MetadataToken(u32)` wraps the raw u32. `0x00_000000` is the null token. `table_id()` extracts bits 31–24; `row_index()` extracts bits 23–0.

**When to use:** Everywhere a metadata reference is stored in a row struct.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MetadataToken(pub u32);

impl MetadataToken {
    pub const NULL: MetadataToken = MetadataToken(0);

    pub fn new(table_id: u8, row_index: u32) -> Self {
        assert!(row_index <= 0x00FF_FFFF, "row index exceeds 24 bits");
        MetadataToken(((table_id as u32) << 24) | row_index)
    }

    pub fn table_id(self) -> u8 {
        (self.0 >> 24) as u8
    }

    pub fn row_index(self) -> Option<u32> {
        let idx = self.0 & 0x00FF_FFFF;
        if idx == 0 { None } else { Some(idx) }
    }

    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}
```

### Pattern 3: Instruction Enum — Flat with Operand Variants

**What:** One flat enum with one variant per opcode. Operands are inline fields (not boxed). The opcode u16 is the implicit discriminant in the binary representation.

**When to use:** The instruction decode/encode functions match on this enum directly.

```rust
pub enum Instruction {
    // Shape N (2B)
    Nop,
    RetVoid,
    AtomicBegin,
    AtomicEnd,
    // Shape R (4B)
    Ret { r_src: u16 },
    Crash { r_msg: u16 },
    LoadTrue { r_dst: u16 },
    LoadFalse { r_dst: u16 },
    // Shape RR (6B)
    Mov { r_dst: u16, r_src: u16 },
    StrLen { r_dst: u16, r_str: u16 },
    // Shape RRR (8B)
    AddI { r_dst: u16, r_a: u16, r_b: u16 },
    // ... etc.
    // Shape RI32 (8B)
    LoadString { r_dst: u16, idx: u32 },
    NewArray { r_dst: u16, elem_type: u32 },
    // Shape RI64 (12B)
    LoadInt { r_dst: u16, val: i64 },
    LoadFloat { r_dst: u16, val: f64 },
    // Shape I32 (8B) — BR only
    Br { offset: i32 },
    // Shape CALL (12B)
    Call { r_dst: u16, method_idx: u32, r_base: u16, argc: u16 },
    // var layout — per spec
    Switch { r_tag: u16, offsets: Vec<i32> },
    CallVirt { r_dst: u16, r_obj: u16, contract_idx: u32, slot: u16, r_base: u16, argc: u16 },
    GetField { r_dst: u16, r_obj: u16, field_idx: u32 },
    SetField { r_obj: u16, field_idx: u32, r_val: u16 },
    // ... (all 91)
}
```

**Note on SWITCH:** `offsets: Vec<i32>` makes this instruction heap-allocated. This is acceptable for a pure-data representation. The VM (Phase 17) may prefer a different representation internally.

### Pattern 4: Reader Using Cursor + byteorder

**What:** Wrap `&[u8]` in `std::io::Cursor`, call `byteorder::ReadBytesExt` methods for u16/u32/u64/i32.

```rust
use std::io::{Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt};

fn read_module(bytes: &[u8]) -> Result<Module, DecodeError> {
    let mut cur = Cursor::new(bytes);
    // magic check
    let mut magic = [0u8; 4];
    cur.read_exact(&mut magic)?;
    if &magic != b"WRIT" {
        return Err(DecodeError::BadMagic(magic));
    }
    let format_version = cur.read_u16::<LittleEndian>()?;
    let flags = cur.read_u16::<LittleEndian>()?;
    // ... header fields ...
    Ok(module)
}
```

### Pattern 5: Writer Using Vec<u8>

**What:** Allocate a `Vec<u8>`, use `byteorder::WriteBytesExt` to append LE values.

```rust
use std::io::Write;
use byteorder::{LittleEndian, WriteBytesExt};

fn write_module(module: &Module) -> Result<Vec<u8>, EncodeError> {
    let mut out: Vec<u8> = Vec::new();
    out.write_all(b"WRIT")?;
    out.write_u16::<LittleEndian>(module.header.format_version)?;
    out.write_u16::<LittleEndian>(module.header.flags)?;
    // ...
    Ok(out)
}
```

### Pattern 6: ModuleBuilder — Plain Fluent Builder

**What:** Accumulates state in mutable fields; `build()` produces a fully-formed `Module` with consistent heap offsets.

```rust
pub struct ModuleBuilder {
    name: String,
    version: String,
    type_defs: Vec<TypeDefRow>,
    method_defs: Vec<MethodDefRow>,
    method_bodies: Vec<MethodBody>,
    // ... other tables
}

impl ModuleBuilder {
    pub fn new(name: &str) -> Self { ... }
    pub fn version(mut self, v: &str) -> Self { self.version = v.to_string(); self }
    pub fn add_type(mut self, row: TypeDefRow) -> Self { ... }
    pub fn add_method(mut self, row: MethodDefRow, body: MethodBody) -> Self { ... }
    pub fn build(self) -> Module { /* intern strings into heap, compute offsets */ }
}
```

`build()` is where the string and blob heap interning happens: the builder accumulates raw strings/blobs; `build()` assigns them offsets and writes them into the heap byte vec.

### Anti-Patterns to Avoid

- **Using table row indices directly as usize array indices:** The spec is 1-based; `rows[token.row_index() - 1]` is the correct indexing, but you should encapsulate this in a `Module::lookup_type_def(token)` style helper — do not scatter the `-1` arithmetic.
- **Storing String in row structs instead of heap offsets:** Row structs should store `u32` heap offset fields matching the binary format. String-typed fields (for builder ergonomics) belong in the builder, not in `Row` types.
- **Mutable heap during reading:** The heaps are read once, then treated as immutable byte slices. Don't mutate heap bytes after deserialization.
- **Recomputing heap offsets on every `to_bytes()` call:** The Module stores the pre-serialized heap bytes; `to_bytes()` writes them verbatim. This guarantees round-trip identity. If the builder changes heap content, a new Module is produced.
- **SWITCH offsets stored as `SmallVec` or fixed array:** The spec says up to 256 variants (SWITCH uses u16 for count). Use `Vec<i32>` — simplicity beats premature optimization.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| LE byte read/write | Custom `read_u32_le(buf, pos)` functions | `byteorder::ReadBytesExt` / `WriteBytesExt` | byteorder handles all widths, cursor advancement, and EOF errors; hand-rolling is tedious and error-prone |
| Error types | Generic `String` errors or `anyhow::Error` | `thiserror` derive | `thiserror` generates correct `Display`/`Error`/`From` impls; anyhow is opaque for downstream callers (VM/assembler need to match on decode errors) |
| String interning in builder | Custom intern table with HashMap | Simple `Vec<u8>` with offset tracking | Deduplication is an optimization; for Phase 16, linear append is correct and simpler; if heap size becomes a concern that's Phase 16+ |

**Key insight:** This is a pure I/O + data modeling problem. The spec is the single source of truth. Any library that obscures the byte-level mapping (e.g., binrw, serde_bytes) adds complexity without benefit because the format has non-trivial structure (var-length instructions, heap offset cross-references).

---

## Common Pitfalls

### Pitfall 1: Table Ordering in the Header Directory

**What goes wrong:** The table directory has a fixed order of 21 entries (§2.16.1). Writing tables in the wrong order produces a valid-looking header with wrong offsets.
**Why it happens:** It's easy to write tables in the order they're populated by the builder, not in the spec order.
**How to avoid:** Define a `TableId` enum with explicit u8 discriminants 0–20 matching the spec table. Use that enum to index both the directory and the serialization order.
**Warning signs:** Round-trip test passes but a different spec-compliant reader fails to find rows.

### Pitfall 2: String Heap Offset 0 Reserved

**What goes wrong:** Offset 0 is reserved as the empty/null string per §2.16.1. If the builder emits strings starting at offset 0 for the first real string, null references and the first real string become indistinguishable.
**Why it happens:** Builder initializes heap as empty Vec, then appends the first string at position 0.
**How to avoid:** Initialize string heap with a `0x00 0x00 0x00 0x00` (4-byte length-prefix for zero-length string). First real string begins at offset 4.
**Warning signs:** `module_name` reads back as the first user-defined type name.

### Pitfall 3: Method Body Alignment

**What goes wrong:** §2.16.5 states table rows are aligned to 4-byte boundaries. Method body offsets must also be valid. Writer may produce mis-aligned bodies.
**Why it happens:** Variable-length instruction streams don't naturally end on 4-byte boundaries.
**How to avoid:** Pad method body streams to 4-byte alignment after `code_size` bytes. Write the pad bytes before computing `body_offset` for the next method.
**Warning signs:** Round-trip passes for single-method modules but fails for multi-method modules.

### Pitfall 4: SWITCH Instruction Variable Length

**What goes wrong:** Decoding a SWITCH instruction reads `n` offsets; if `n` is incorrect, the cursor position is off and every subsequent instruction in the method decodes wrong.
**Why it happens:** Off-by-one when reading `n`; forgetting that `n` is u16 in the encoding.
**How to avoid:** Encode SWITCH as: `u16(op) u16(r_tag) u16(n) i32[n]`. Write a dedicated test that encodes SWITCH with n=0, n=1, and n=5 and verifies round-trip.
**Warning signs:** Instructions after a SWITCH decode to garbage opcodes.

### Pitfall 5: Blob Heap TypeRef Recursive Encoding

**What goes wrong:** TypeRef encoding (§2.15.3) is variable-length and recursive (`Array<Option<int>>` nests three levels). A reader that only handles one-level TypeRefs silently truncates signatures.
**Why it happens:** The simple case (primitive tag byte) works immediately; recursive case is added later and missed.
**How to avoid:** Write a standalone `decode_type_ref(cursor) -> TypeRef` function that recursively calls itself. Test it on a three-level nested type before integrating into the module reader.
**Warning signs:** Generics-heavy method signatures fail to round-trip; MethodDef signature blobs produce incorrect types.

### Pitfall 6: Round-Trip Identity vs. Semantic Equivalence

**What goes wrong:** The spec says `write → read → write` produces identical bytes. If the builder normalizes or reorders anything (e.g., alphabetizing strings in the heap), the second write produces different bytes from the first.
**Why it happens:** Convenience optimizations in the builder affect the canonical representation.
**How to avoid:** The `Module` struct stores the heap bytes verbatim from deserialization. `to_bytes()` writes them back without transformation. The builder produces a Module with its own heap ordering — which is then stable across subsequent `to_bytes()` calls.
**Warning signs:** First `to_bytes()` call equals the builder output, but after `from_bytes()` + `to_bytes()` the bytes differ.

---

## Code Examples

Verified patterns from spec:

### Header Deserialization (Pattern from §2.16.1)

```rust
// Magic: "WRIT" + u16 format_version + u16 flags
// Module header: u32 module_name, u32 module_version, u32 string_heap_offset,
//                u32 string_heap_size, u32 blob_heap_offset, u32 blob_heap_size
// Table directory: [u32 offset, u32 row_count] × 21
// Total: 8 + 8 + 16 + (8 × 21) = 200 bytes

fn read_header(cur: &mut Cursor<&[u8]>) -> Result<ModuleHeader, DecodeError> {
    let mut magic = [0u8; 4];
    cur.read_exact(&mut magic).map_err(DecodeError::Io)?;
    if &magic != b"WRIT" { return Err(DecodeError::BadMagic); }
    let format_version = cur.read_u16::<LittleEndian>().map_err(DecodeError::Io)?;
    let flags = cur.read_u16::<LittleEndian>().map_err(DecodeError::Io)?;
    let module_name_offset = cur.read_u32::<LittleEndian>().map_err(DecodeError::Io)?;
    let module_version_offset = cur.read_u32::<LittleEndian>().map_err(DecodeError::Io)?;
    let string_heap_offset = cur.read_u32::<LittleEndian>().map_err(DecodeError::Io)?;
    let string_heap_size = cur.read_u32::<LittleEndian>().map_err(DecodeError::Io)?;
    let blob_heap_offset = cur.read_u32::<LittleEndian>().map_err(DecodeError::Io)?;
    let blob_heap_size = cur.read_u32::<LittleEndian>().map_err(DecodeError::Io)?;
    let mut table_dir = [(0u32, 0u32); 21];
    for i in 0..21 {
        let offset = cur.read_u32::<LittleEndian>().map_err(DecodeError::Io)?;
        let row_count = cur.read_u32::<LittleEndian>().map_err(DecodeError::Io)?;
        table_dir[i] = (offset, row_count);
    }
    Ok(ModuleHeader { format_version, flags, module_name_offset, module_version_offset,
                       string_heap_offset, string_heap_size, blob_heap_offset, blob_heap_size,
                       table_dir })
}
```

### Instruction Round-Trip (Pattern from §2.5, §4.1, §3.6, §3.7, §3.8, §3.9, §3.14)

```rust
// Variable-layout instructions that need special handling:
// SWITCH:       u16(op) u16(r_tag) u16(n) i32[n]        6 + 4n bytes
// CALL_VIRT:    u16(op) u16(r_dst) u16(r_obj) u32(contract_idx) u16(slot) u16(r_base) u16(argc) = 14B
// NEW_DELEGATE: u16(op) u16(r_dst) u32(method_idx) u16(r_target) = 10B
// CALL_INDIRECT:u16(op) u16(r_dst) u16(r_delegate) u16(r_base) u16(argc) = 10B
// TAIL_CALL:    u16(op) u32(method_idx) u16(r_base) u16(argc) = 10B
// GET_FIELD:    u16(op) u16(r_dst) u16(r_obj) u32(field_idx) = 10B
// SET_FIELD:    u16(op) u16(r_obj) u32(field_idx) u16(r_val) = 10B
// GET_COMPONENT:u16(op) u16(r_dst) u16(r_entity) u32(comp_type_idx) = 10B  (same as GET_FIELD shape)
// ARRAY_INIT:   u16(op) u16(r_dst) u32(elem_type) u16(count) u16(r_base) = 12B
// ARRAY_SLICE:  u16(op) u16(r_dst) u16(r_arr) u16(r_start) u16(r_end) = 10B
// STR_BUILD:    u16(op) u16(r_dst) u16(count) u16(r_base) = 8B

// Standard shapes (from §4.1):
// N:    u16(op)                                           = 2B
// R:    u16(op) u16(r)                                   = 4B
// RR:   u16(op) u16(r) u16(r)                            = 6B
// RRR:  u16(op) u16(r) u16(r) u16(r)                    = 8B
// RI32: u16(op) u16(r) u32(imm)                         = 8B
// RI64: u16(op) u16(r) u64(imm)                         = 12B
// I32:  u16(op) u16(pad) i32(imm)                       = 8B  [BR only]
// CALL: u16(op) u16(r_dst) u32(idx) u16(r_base) u16(argc) = 12B
```

### MetadataToken Usage

```rust
// Token encoding: bits 31–24 = table_id, bits 23–0 = row_index (1-based)
// table_id 2 = TypeDef; row 5 → 0x02_000005
let token = MetadataToken::new(2, 5);
assert_eq!(token.0, 0x02_000005);
assert_eq!(token.table_id(), 2);
assert_eq!(token.row_index(), Some(5));

let null = MetadataToken::NULL;
assert_eq!(null.row_index(), None);
assert!(null.is_null());
```

### String Heap Initialization (Null-safe)

```rust
// String heap: offset 0 reserved as empty/null string
// Format: u32(byte_length) followed by string bytes
fn init_string_heap() -> Vec<u8> {
    let mut heap = Vec::new();
    // Write 4-byte zero-length record at offset 0
    heap.write_u32::<LittleEndian>(0).unwrap();
    heap // first real string starts at offset 4
}

fn intern_string(heap: &mut Vec<u8>, s: &str) -> u32 {
    let offset = heap.len() as u32;
    let bytes = s.as_bytes();
    heap.write_u32::<LittleEndian>(bytes.len() as u32).unwrap();
    heap.extend_from_slice(bytes);
    offset
}

fn read_string(heap: &[u8], offset: u32) -> Result<&str, DecodeError> {
    if offset == 0 { return Ok(""); }
    let off = offset as usize;
    let len = u32::from_le_bytes(heap[off..off+4].try_into()?) as usize;
    std::str::from_utf8(&heap[off+4..off+4+len]).map_err(DecodeError::BadUtf8)
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `byteorder` for all LE operations | Rust stdlib `to_le_bytes`/`from_le_bytes` also valid | Rust 1.32 | For isolated primitive conversions, stdlib is sufficient; byteorder still preferred for streaming I/O via `impl Read` |
| `bincode` for binary serialization | Deprecated / development ceased | ~2023 | No longer a viable option; manual serialization is the right call for custom formats anyway |

**Not deprecated:**
- `byteorder 1.5`: Actively maintained, stable API, no breaking changes planned
- `thiserror 2.0`: Major version bump from 1.x to 2.x; workspace already on 2.0.18

---

## Open Questions

1. **Heap deduplication in ModuleBuilder**
   - What we know: The spec does not require deduplication in the heap; identical strings may appear multiple times
   - What's unclear: Should the builder intern (dedup) strings for efficiency, or append linearly?
   - Recommendation: Linear append for Phase 16 (simpler, correct, enables round-trip identity). Dedup is a future optimization. Note: if dedup is added later, `to_bytes()` must still be deterministic — sort order of heap entries must be stable.

2. **`TypeRef` recursive decode depth limit**
   - What we know: TypeRef encoding is recursive (§2.15.3); Array<Option<int>> is 3 levels deep; there is no spec-mandated depth limit
   - What's unclear: How deep can real modules nest? Infinite recursion on malformed input?
   - Recommendation: Add a depth counter to the recursive TypeRef decoder; return `DecodeError::TypeRefTooDeep` at some reasonable limit (e.g., 32 levels). Document this as a conservative implementation choice, not a spec constraint.

3. **Method body byte alignment between methods**
   - What we know: §2.16.5 says "Table rows are aligned to 4-byte boundaries"; §2.16.6 defines method body layout but does not explicitly say method bodies themselves must be 4-byte-aligned
   - What's unclear: Does `body_offset` need to be 4-byte-aligned, or is it byte-accurate?
   - Recommendation: Align method body start offsets to 4-byte boundaries (pad with zeros between bodies). This is the conservative, CLR-compatible interpretation and guarantees safe reads on aligned-access platforms.

---

## Sources

### Primary (HIGH confidence)

- Writ IL Spec §2.16 (`language-spec/spec/45_2_16_il_module_format.md`) — Binary container, header layout, table definitions, method body layout, metadata tokens
- Writ IL Spec §2.15 (`language-spec/spec/44_2_15_il_type_system.md`) — TypeRef encoding, primitive type tags, register model
- Writ IL Spec §2.5 (`language-spec/spec/34_2_5_instruction_encoding.md`) — Opcode categories, instruction shapes
- Writ IL Spec §4.0 (`language-spec/spec/65_4_0_instruction_count_by_category.md`) — 91 opcode count by category
- Writ IL Spec §4.1 (`language-spec/spec/66_4_1_instruction_shape_reference.md`) — Shape byte layouts
- Instruction category specs §3.6–§3.14 — Var-layout instruction encodings
- https://docs.rs/byteorder/latest/byteorder/ — byteorder 1.5.0 API, LE read/write, stdlib alternatives note
- https://docs.rs/thiserror/latest/thiserror/ — thiserror 2.0.18, derive macro for error enums

### Secondary (MEDIUM confidence)

- https://rust-unofficial.github.io/patterns/patterns/creational/builder.html — Builder pattern; fluent mut methods returning Self is idiomatic Rust for this use case
- https://github.com/BurntSushi/byteorder — byteorder crate, actively maintained

### Tertiary (LOW confidence)

- Web search results on binrw/nom alternatives — Not verified against official docs; consensus that manual IO is appropriate for custom formats with complex logic

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — byteorder and thiserror are both verified via official docs; stdlib Cursor is stdlib
- Architecture: HIGH — directly derived from spec §2.16; all table structures and binary layouts are precisely specified
- Pitfalls: HIGH for heap/alignment pitfalls (derived from spec text); MEDIUM for TypeRef recursion depth (not explicitly addressed in spec)

**Research date:** 2026-03-01
**Valid until:** 2026-09-01 (stable ecosystem; byteorder/thiserror are mature)
