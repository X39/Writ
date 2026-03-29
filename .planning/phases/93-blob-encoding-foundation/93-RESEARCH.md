# Phase 93: Blob Encoding Foundation - Research

**Researched:** 2026-03-27
**Domain:** Binary module format — blob heap encoding/decoding of attribute argument values
**Confidence:** HIGH

## Summary

Phase 93 establishes the wire format for attribute arguments in compiled `.writc` modules. Currently the compiler's `collect_attributes` function passes `0` (the null blob offset) as the `value` field of every `AttributeDefRow`, meaning all attribute argument data is silently discarded at emit time. This phase replaces that stub with a real tagged encoding and adds a matching decoder, both sharing constants from `writ-module` so neither crate duplicates the format definition.

The work is entirely self-contained: no parser changes, no type system changes, no new AST nodes. The three types that carry attribute arguments through the existing pipeline — `AstAttributeArg::Positional(AstExpr::IntLit)`, `AstAttributeArg::Positional(AstExpr::BoolLit)`, `AstAttributeArg::Positional(AstExpr::StringLit)`, and `AstAttributeArg::Named` — are already parsed and reach `collect_attributes` fully intact. The gap is purely in `encoding.rs` where those values get encoded into the blob heap, and in `writ-module` where a decoder must live alongside the tag constants.

The blob heap infrastructure (both in `writ-module::heap` and the compiler's `BlobHeap`/`heaps.rs`) is already complete and correct. The test pattern for `writ-module` is well established in `writ-module/tests/round_trip.rs`. A new test file `writ-module/tests/attr_encoding.rs` following that pattern is the right location for the round-trip test the success criteria requires.

**Primary recommendation:** Define `AttrValue` enum and `ATTR_TAG_*` constants in `writ-module/src/attr.rs`, expose them from `writ-module/src/lib.rs`, then update `collect_attributes` in `writ-compiler/src/emit/collect/encoding.rs` to call the new encoder, and add the decoder as a free function in `writ-module/src/attr.rs`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — infrastructure phase, all decisions at Claude's discretion.

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BLOB-01 | Attribute arguments are encoded into the blob heap using a tagged format (string → UTF-8, int → i64 LE, bool → u8, named args → name + value) | Wire format design in Architecture Patterns section; encoder implementation in 93-02 |
| BLOB-02 | Attribute arguments can be decoded from the blob heap back to their original types and names (round-trip fidelity) | Decoder design in Architecture Patterns; round-trip test pattern in Code Examples |
| BLOB-03 | Shared tag constants defined in `writ-module` used by both encoder and decoder | Module placement in Standard Stack; public export pattern in Code Examples |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `writ-module` (internal) | workspace | Shared types between compiler and runtime | Existing pattern — `TypeDefKind`, `MetadataToken`, `TableId` all live here and are imported by both compiler and runtime |
| `byteorder` | 1.5 | Little-endian i64/u8 write/read in blob format | Already a `writ-module` dependency; `heap.rs` uses it today |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `writ-compiler/emit/heaps.rs::BlobHeap` | internal | Deduplicating blob heap used during compilation | Compiler-side encoding path only — already used for all other blob data |
| `writ-module/src/heap.rs::read_blob` | internal | Read raw blob bytes back from a `Module` | Decoder and test code |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `writ-module/src/attr.rs` (new file) | Inline in `writ-module/src/lib.rs` | New file is cleaner; module already has 10 files each with a single focused domain |
| Tag byte prefix for each value | CBOR / MessagePack | Custom format is 5 bytes for int/bool, ~N+5 for string — trivially decodable, no dependency |

**Installation:** No new dependencies required. `byteorder` is already in `writ-module`'s `[dependencies]`.

## Architecture Patterns

### Recommended Project Structure

New file:
```
writ-module/src/attr.rs    -- AttrValue enum, ATTR_TAG_* constants, encode_attr_args(), decode_attr_args()
```

Modified files:
```
writ-module/src/lib.rs          -- pub mod attr; pub use attr::{AttrValue, ...}
writ-compiler/src/emit/collect/encoding.rs  -- collect_attributes: replace value=0 with encode call
```

New test file:
```
writ-module/tests/attr_encoding.rs   -- round-trip tests for all three arg types
```

### Pattern 1: Tagged Blob Format

**What:** A variable-length byte sequence where each attribute argument is prefixed with a 1-byte tag identifying its type, followed by its payload.

**When to use:** Every `AttributeDefRow.value` that has at least one argument.

**Wire format for a single attribute argument:**

```
+--------+---------------------------+
| tag(1) | payload (variable)        |
+--------+---------------------------+

ATTR_TAG_STRING = 0x01
  payload: u32(byte_len) + UTF-8 bytes  (matches string heap format)

ATTR_TAG_INT    = 0x02
  payload: i64 little-endian (8 bytes)

ATTR_TAG_BOOL   = 0x03
  payload: u8 (0x00 = false, 0x01 = true)

ATTR_TAG_NAMED  = 0x04
  payload: u32(name_byte_len) + UTF-8 name bytes + [inner arg encoding]
  (inner arg is the value, encoded as above recursively — named args wrap a positional value)
```

A blob with multiple arguments is a sequential concatenation: `[arg0_encoded][arg1_encoded]...`.

An empty argument list encodes as the empty blob (offset 0 — the null blob).

**Why this format:**
- Matches the requirement spec verbatim (BLOB-01): string → UTF-8, int → i64 LE, bool → u8, named args → name + value
- Completely self-describing — decoder needs no schema
- Compatible with `writ-module::heap::read_blob` / `heap::write_blob` — just wraps the raw bytes
- The compiler-side `BlobHeap::intern` deduplicates identical argument blobs for free

### Pattern 2: AttrValue enum in writ-module

**What:** A Rust enum representing a decoded attribute argument value. Defined in `writ-module` so both compiler and runtime share it without duplication.

```rust
// Source: writ-module/src/attr.rs (new)

pub const ATTR_TAG_STRING: u8 = 0x01;
pub const ATTR_TAG_INT:    u8 = 0x02;
pub const ATTR_TAG_BOOL:   u8 = 0x03;
pub const ATTR_TAG_NAMED:  u8 = 0x04;

/// A decoded attribute argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    String(String),
    Int(i64),
    Bool(bool),
    Named { name: String, value: Box<AttrValue> },
}
```

### Pattern 3: Encoder (compiler side)

**What:** A free function in `writ-module/src/attr.rs` that serializes a slice of `AttrValue` into a `Vec<u8>`. The compiler builds `AttrValue` instances from `AstAttributeArg` nodes and calls this to get the blob bytes.

```rust
// Source: writ-module/src/attr.rs (new)
pub fn encode_attr_args(args: &[AttrValue]) -> Vec<u8> {
    let mut buf = Vec::new();
    for arg in args {
        encode_attr_value(arg, &mut buf);
    }
    buf
}

fn encode_attr_value(val: &AttrValue, buf: &mut Vec<u8>) {
    match val {
        AttrValue::String(s) => {
            buf.push(ATTR_TAG_STRING);
            let bytes = s.as_bytes();
            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(bytes);
        }
        AttrValue::Int(n) => {
            buf.push(ATTR_TAG_INT);
            buf.extend_from_slice(&n.to_le_bytes());
        }
        AttrValue::Bool(b) => {
            buf.push(ATTR_TAG_BOOL);
            buf.push(if *b { 1 } else { 0 });
        }
        AttrValue::Named { name, value } => {
            buf.push(ATTR_TAG_NAMED);
            let name_bytes = name.as_bytes();
            buf.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(name_bytes);
            encode_attr_value(value, buf);
        }
    }
}
```

### Pattern 4: Decoder (writ-module side)

**What:** A free function that reads the raw blob bytes (as returned by `heap::read_blob`) and returns a `Vec<AttrValue>`.

```rust
// Source: writ-module/src/attr.rs (new)
pub fn decode_attr_args(blob: &[u8]) -> Result<Vec<AttrValue>, DecodeError> {
    let mut cursor = 0usize;
    let mut out = Vec::new();
    while cursor < blob.len() {
        let (val, consumed) = decode_one(blob, cursor)?;
        out.push(val);
        cursor += consumed;
    }
    Ok(out)
}
```

### Pattern 5: Compiler-side AstAttributeArg → AttrValue mapping

**What:** In `collect_attributes` (encoding.rs), map each `AstAttributeArg` to `AttrValue`, call `encode_attr_args`, intern the resulting bytes into `builder.blob_heap`, then pass the blob offset to `builder.add_attribute_def`.

The key `AstExpr` variants to handle:
- `AstExpr::StringLit { value, .. }` → `AttrValue::String(value.clone())`
- `AstExpr::IntLit { value, .. }` → `AttrValue::Int(*value as i64)` (check actual field type in `AstExpr`)
- `AstExpr::BoolLit { value, .. }` → `AttrValue::Bool(*value)`
- `AstAttributeArg::Named { name, value, .. }` → `AttrValue::Named { name: name.clone(), value: Box::new(map_expr(value)) }`

**Note:** Unsupported arg expressions (e.g., binary ops, function calls) should be silently skipped or encoded as a placeholder — the phase goal is stdlib attributes (`[Deprecated]`, `[Locale]`, `[Singleton]`, `[Conditional]`) which use only string/bool/int literals.

### Pattern 6: NULL-owner guard in collect_attributes

**What:** The current code calls `builder.token_for_def(def_id).unwrap_or(MetadataToken::NULL)`. If the owner is `NULL`, the `AttributeDefRow` is written with a null token — this is not meaningful and wastes space. The fix is to skip emission entirely when `token_for_def` returns `None`.

```rust
// In collect_attributes, replace:
let owner_token = builder.token_for_def(def_id).unwrap_or(MetadataToken::NULL);
// With:
let owner_token = match builder.token_for_def(def_id) {
    Some(t) => t,
    None => continue,
};
```

This is a correctness fix that should ship in the same plan as the encoding change (93-02) since touching the function twice is wasteful.

### Anti-Patterns to Avoid

- **Encoding AttrValue in writ-compiler instead of writ-module:** The tag constants and enum belong in `writ-module` so the runtime can import them without depending on the compiler. The encoder and decoder must both live in `writ-module/src/attr.rs`.
- **Using `heap::write_blob` (non-deduplicating) in the compiler path:** The compiler uses `BlobHeap::intern` (deduplicating) for all blob data. Use `builder.blob_heap.intern(&bytes)` to get the offset, then pass that `u32` to `builder.add_attribute_def`.
- **Defining separate `encode`/`decode` in writ-compiler and writ-runtime:** That violates BLOB-03 and is the exact mistake the requirement exists to prevent.
- **Encoding into the string heap:** Attribute argument blobs are binary data and belong in the blob heap, not the string heap.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Blob read/write | Custom byte buffer type | `writ-module::heap::read_blob` / `write_blob` | Already handles the u32-length-prefix format; used by all other blob data in the module |
| Blob interning | Hash map deduplication | `BlobHeap::intern` (compiler-side) | Already exists; deduplicates identical attribute blobs for free |
| String encoding | Custom UTF-8 serializer | `s.as_bytes()` + u32 length prefix | The string heap uses this exact format; matching it keeps the decoder consistent |

**Key insight:** The only custom logic in this phase is the tag-dispatch layer. Everything below that (heap I/O, blob interning) is pre-existing infrastructure.

## Common Pitfalls

### Pitfall 1: Passing `value=0` vs encoding an empty arg list

**What goes wrong:** An attribute with no arguments (`[Singleton]`) correctly emits `value=0` (the null blob offset). An attribute with arguments that fail to encode would silently write `value=0` too, making it indistinguishable from a no-arg attribute at decode time.

**Why it happens:** The stub currently hardcodes `0`. After the fix, the encoder for an empty `args` slice must also return offset `0`, not a non-zero offset pointing to a zero-length blob, to maintain the convention.

**How to avoid:** `encode_attr_args(&[])` should return `vec![]` (empty), and the caller should only call `builder.blob_heap.intern(...)` when `!bytes.is_empty()`, using `0` otherwise.

**Warning signs:** A round-trip test for a no-arg attribute returns non-zero `value` in the `AttributeDefRow`.

### Pitfall 2: i64 overflow when mapping from AstExpr::IntLit

**What goes wrong:** `AstExpr::IntLit` stores the parsed integer. If its internal field is `i64` already, the cast is free. If it is stored as `i128` or `u64`, casting to `i64` without bounds check silently wraps.

**Why it happens:** The phase only touches the emit path and does not add type-checking for attribute arg ranges.

**How to avoid:** Check the actual field type in `AstExpr::IntLit` before writing the cast. If it is already `i64`, no issue. If wider, a saturating cast or truncation is acceptable for Phase 93 since the spec says int args are i64 — out-of-range values would be a semantic error for a later phase.

**Warning signs:** Round-trip test for `Int(i64::MAX)` fails after decode.

### Pitfall 3: Named arg wrapping the inner value

**What goes wrong:** A named arg `[Deprecated(msg: "text")]` has `AstAttributeArg::Named { name: "msg", value: AstExpr::StringLit { value: "text" } }`. The encode path must recurse into the `value` expression to produce the inner payload after the name bytes. If the recursion is missing, the inner type tag and payload are absent and the decoder returns a truncated result.

**Why it happens:** The `ATTR_TAG_NAMED` encoding is `tag + u32(name_len) + name_bytes + [inner value encoding]`. Forgetting the final inner encoding is easy.

**How to avoid:** The round-trip test must include at least one named arg, not just positional.

### Pitfall 4: add_attribute_def signature mismatch

**What goes wrong:** The compiler's `ModuleBuilder::add_attribute_def` takes `value: u32` (a blob offset). The `writ-module::builder::ModuleBuilder::add_attribute_def` takes `value: &[u8]` (raw bytes). These are different structs. The compiler uses its own `ModuleBuilder` from `emit/module_builder.rs`, not the one from `writ-module/src/builder.rs`.

**Why it happens:** Both crates have a struct named `ModuleBuilder`. They are not the same type.

**How to avoid:** In the compiler path, build the `Vec<u8>` bytes, call `builder.blob_heap.intern(&bytes)` to get a `u32`, then pass that `u32` to `builder.add_attribute_def(owner, owner_kind, name, offset)`. Do not call `writ-module::builder::ModuleBuilder` from the compiler.

### Pitfall 5: Decoder cursor arithmetic off-by-one

**What goes wrong:** The decoder walks the blob linearly. If one arg's consumed-byte count is wrong, all subsequent args are decoded from the wrong offset, producing garbage or a `DecodeError`.

**Why it happens:** Manual cursor arithmetic. `TAG(1) + u32(4) + N` bytes for a string; `TAG(1) + i64(8)` for int; `TAG(1) + u8(1)` for bool.

**How to avoid:** The round-trip test must assert exact equality of decoded values for all three types in a single multi-arg blob.

## Code Examples

Verified patterns from existing codebase:

### Existing blob intern in compiler
```rust
// Source: writ-compiler/src/emit/collect/encoding.rs, encode_type_from_ast
let mut buf = Vec::new();
encode_ast_type_into(ast_type, generics, &mut buf);
builder.blob_heap.intern(&buf)  // returns u32 offset
```

### Existing blob read in writ-module tests
```rust
// Source: writ-module/tests/round_trip.rs, test_module_with_typedef_round_trip
let type_sig = heap::write_blob(&mut module.blob_heap, &[0x00]);
// after round-trip:
let blob_bytes = heap::read_blob(&module2.blob_heap, type_sig).unwrap();
```

### Existing AttributeDefRow in tables.rs
```rust
// Source: writ-module/src/tables.rs
pub struct AttributeDefRow {
    pub owner: MetadataToken,
    pub owner_kind: u8,
    pub name: u32,   // string heap offset
    pub value: u32,  // blob heap offset — 0 = no args
}
```

### Existing add_attribute_def in compiler ModuleBuilder
```rust
// Source: writ-compiler/src/emit/module_builder.rs (line ~508)
pub fn add_attribute_def(
    &mut self,
    owner: MetadataToken,
    owner_kind: u8,
    name: &str,
    value: u32,   // blob offset, pass 0 for no-arg attributes
) -> usize
```

### Existing collect_attributes stub (the target of 93-02)
```rust
// Source: writ-compiler/src/emit/collect/encoding.rs (line ~100)
for attr in &attrs {
    // Value blob: empty for now (args encoding deferred).
    builder.add_attribute_def(owner_token, owner_kind, &attr.name, 0);
}
```

### Existing AstAttributeArg variants
```rust
// Source: writ-compiler/src/ast/decl.rs
pub enum AstAttributeArg {
    Positional(AstExpr),
    Named { name: String, name_span: SimpleSpan, value: AstExpr },
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| value=0 stub in collect_attributes | Real tagged blob encoding | Phase 93 (this phase) | AttributeDefRow.value will be non-zero for attrs with args |

**Deprecated/outdated:**
- The `value=0` hardcode at line 101 of `encoding.rs`: replaced by `builder.blob_heap.intern(&encoded_bytes)` or `0` for no-arg attrs.

## Open Questions

1. **AstExpr::IntLit internal field type**
   - What we know: `AstExpr::IntLit` exists and is matched in `collect_locale_defs` for `AstExpr::StringLit`
   - What's unclear: Whether the integer field is `i64`, `i128`, or another width — not verified by reading `ast/expr.rs`
   - Recommendation: Implementer should read `writ-compiler/src/ast/expr.rs` during Plan 93-02 to confirm the field name and type before writing the cast. The encoding specifies i64 per BLOB-01.

2. **Float literal support**
   - What we know: BLOB-01 enumerates string, int, bool, named — no float tag
   - What's unclear: Whether `[SomeAttr(3.14)]` would silently drop the float arg or produce an error
   - Recommendation: Silently skip unsupported arg expressions in Phase 93. Float attributes are not used by any builtin attribute in v10.0.

## Environment Availability

Step 2.6: SKIPPED — this phase makes no changes to external tools, services, CLIs, or runtimes. All work is Rust source code changes within the existing Cargo workspace.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) |
| Config file | `Cargo.toml` per crate — no separate config |
| Quick run command | `cargo test -p writ-module attr_encoding` |
| Full suite command | `cargo test -p writ-module && cargo test -p writ-compiler` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BLOB-01 | String arg encodes as TAG_STRING + u32(len) + UTF-8 bytes | unit | `cargo test -p writ-module test_encode_string_arg` | Wave 0 |
| BLOB-01 | Int arg encodes as TAG_INT + i64 LE | unit | `cargo test -p writ-module test_encode_int_arg` | Wave 0 |
| BLOB-01 | Bool arg encodes as TAG_BOOL + u8 | unit | `cargo test -p writ-module test_encode_bool_arg` | Wave 0 |
| BLOB-01 | Named arg encodes as TAG_NAMED + u32(name_len) + name_bytes + inner | unit | `cargo test -p writ-module test_encode_named_arg` | Wave 0 |
| BLOB-02 | Round-trip: encode then decode string/int/bool equals original | unit | `cargo test -p writ-module test_round_trip_attr_args` | Wave 0 |
| BLOB-03 | ATTR_TAG_* constants are pub-exported from writ-module | compile-time | `cargo build -p writ-module` | Wave 0 |
| (success criterion 3) | Compiled module with attrs has non-zero value in AttributeDefRow | integration | `cargo test -p writ-compiler emit_body_tests` | Exists |
| (success criterion 4) | Round-trip test for string/int/bool args passes in writ-module | unit | `cargo test -p writ-module test_round_trip_attr_args` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-module`
- **Per wave merge:** `cargo test -p writ-module && cargo test -p writ-compiler`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-module/tests/attr_encoding.rs` — covers BLOB-01, BLOB-02, BLOB-03 (round-trip tests)
- [ ] `writ-module/src/attr.rs` — the module itself must exist before tests can import it

*(Existing `emit_body_tests.rs` in `writ-compiler` covers success criterion 3 once encoding.rs is patched — no new test file needed there.)*

## Sources

### Primary (HIGH confidence)
- `writ-module/src/heap.rs` — blob heap format: u32(len) + raw bytes, offset 0 = empty blob
- `writ-module/src/tables.rs` — `AttributeDefRow.value: u32` is a blob offset
- `writ-module/src/builder.rs` — `add_attribute_def` takes raw `&[u8]` bytes (writ-module builder, not compiler builder)
- `writ-compiler/src/emit/module_builder.rs` — compiler `add_attribute_def` takes `value: u32` (blob offset, not bytes)
- `writ-compiler/src/emit/heaps.rs` — `BlobHeap::intern(&[u8]) -> u32` is the compiler deduplicating intern
- `writ-compiler/src/emit/collect/encoding.rs` — current stub at line 101: `builder.add_attribute_def(owner_token, owner_kind, &attr.name, 0)`
- `writ-compiler/src/ast/decl.rs` — `AstAttributeArg::Positional(AstExpr)` and `Named { name, value }` variants
- `writ-module/tests/round_trip.rs` — established test pattern for `writ-module` tests
- `.planning/REQUIREMENTS.md` — BLOB-01 through BLOB-03 wire format specification

### Secondary (MEDIUM confidence)
- `writ-module/Cargo.toml` — `byteorder = "1.5"` already present, no new dep needed

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries and their API shapes verified by direct source reading
- Architecture: HIGH — wire format derived directly from REQUIREMENTS.md BLOB-01 spec; encoder/decoder pattern verified against existing heap API
- Pitfalls: HIGH — identified from direct code inspection of the two-builder confusion, NULL-owner guard, and cursor arithmetic; not from speculation

**Research date:** 2026-03-27
**Valid until:** 2026-04-27 (stable internal codebase; no external dependencies)
