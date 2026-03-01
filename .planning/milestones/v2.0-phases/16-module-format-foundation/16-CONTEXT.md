# Phase 16: Module Format Foundation - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

The `writ-module` crate exists as a standalone pure-data crate; any IL module can be written to bytes, read back from bytes, and produced programmatically without touching VM logic. Covers: 200-byte header, 21 metadata tables, string/blob heaps, method bodies, MetadataToken newtype (1-based indexing, 0 = null), all 91 opcodes with encode/decode round-trip, and ModuleBuilder API.

</domain>

<decisions>
## Implementation Decisions

### Crate placement
- New `writ-module` crate in the workspace — standalone with no VM or runtime dependencies
- Both `writ-runtime` and `writ-compiler` (and future `writ-assembler`) will depend on it
- Pure-data crate: defines types, binary format, and builder — no execution logic

### Module API shape
- Unified `Module` struct that holds the in-memory representation of a complete IL module
- `Module::from_bytes(bytes)` to deserialize from binary format
- `module.to_bytes()` to serialize back to binary format
- Round-trip identity: `write → read → write` produces identical bytes

### ModuleBuilder API
- Fluent builder pattern for programmatic construction of modules
- Chained method calls: `ModuleBuilder::new("my_mod").add_type(...).add_method(...).build()`
- Builder returns a `Module` — same type as the reader produces
- Primary use case: test authoring and future assembler/compiler backends

### Claude's Discretion
- Error handling model (rich enum vs simple error — pick what best serves downstream VM, assembler, and test consumers)
- Internal data representation choices (how to model 21 metadata tables, heap storage, newtype wrappers)
- Instruction enum structure (flat vs categorized, operand representation per variant)
- Test strategy details (round-trip tests, snapshot approach, golden binaries)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The spec (§2.16) defines the binary format precisely; implementation should follow it faithfully.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- No existing module/binary format code — this is greenfield
- Workspace already uses Rust 2024 edition with `resolver = "3"`
- `serde` is available transitively; could be used for debug serialization if needed

### Established Patterns
- Crate naming: `writ-{name}` kebab-case (existing: writ-parser, writ-compiler, writ-cli, writ-runtime)
- Module file naming: `lowercase_with_underscores.rs`
- Test file pattern: `{module}_tests.rs` in `tests/` directory
- Test cases use numbered files in `tests/cases/`
- PascalCase for types, snake_case for functions
- `insta` snapshot testing with RON format available (used in writ-parser)

### Integration Points
- Workspace Cargo.toml needs new `writ-module` member
- Future phases depend on this crate: Phase 17 (VM) reads modules, Phase 20 (assembler) writes modules via builder
- `writ-runtime` crate will eventually depend on `writ-module` for the `writ-runtime` virtual module definition

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 16-module-format-foundation*
*Context gathered: 2026-03-01*
