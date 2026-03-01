# Phase 25: IL Codegen — Method Bodies - Context

**Gathered:** 2026-03-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Emit spec-compliant IL instruction sequences for every method body in the TypedAst, produce a complete binary .writil module as a `Vec<u8>` buffer. This phase consumes the populated ModuleBuilder from Phase 24 (all 21 metadata tables, DefId→MetadataToken map) and the TypedAst/TyInterner from Phase 23.

Input: TypedAst + TyInterner + ModuleBuilder (populated metadata skeleton)
Output: `Vec<u8>` — a complete spec-compliant .writil binary module with 200-byte header, serialized metadata tables, heap data, and a contiguous method body stream

</domain>

<decisions>
## Implementation Decisions

### Binary Serialization Scope
- Phase 25 includes full binary serialization — the output is a complete `.writil` module, not just in-memory instruction buffers
- Full spec-compliant 200-byte header with magic bytes, version, all 21 table offsets/counts, heap sizes
- `emit()` returns `Vec<u8>` — file I/O is a CLI concern (writ-cli), not codegen's job
- Method bodies stored in a single contiguous byte stream; `MethodDefRow.body_offset` is relative to stream start

### Instruction Representation
- Typed `Instruction` enum with all 90 opcodes — codegen produces `Vec<Instruction>` per method body
- Instructions are serialized to raw bytes as a final step after all codegen is complete
- Enum provides type safety, testability, and enables validation passes over instruction sequences before serialization

### Claude's Discretion (Instruction Details)
- Branch/jump target representation: symbolic labels vs pre-computed offsets — Claude picks what fits the instruction set best
- Register allocator type tracking: at allocation time vs inferred from usage
- Whether Instruction enum lives in emit module (compiler-only) or a shared crate — Claude picks based on crate dependency graph

### Error Node Handling
- **Abort entire module**: if any `TypedExpr::Error` or `TypedStmt::Error` nodes exist anywhere, produce no `.writil` output
- **Pre-pass check**: before any codegen work, scan the TypedAst for Error nodes; if found, return immediately with diagnostics (no wasted work)
- **Pipeline short-circuit**: the compiler pipeline should not invoke `emit()` at all if the type checker reported any errors; the pre-pass check is a safety net, not the primary guard

### Claude's Discretion (Codegen Warnings)
- Whether codegen emits its own warnings (e.g., constant folding revealing dead branches) is at Claude's discretion — semantic warnings like dead code and unused vars remain the type checker's responsibility

### Debug Info Granularity
- **SourceSpan**: per-statement granularity — one entry per source statement/expression that generates instructions; sub-expression instructions inherit the parent span
- **DebugLocal**: ALL registers get entries, including compiler temporaries — temps get synthetic names; user-declared variables (let bindings, params, for-loop bindings, match arms) get their source names
- **Flag-controlled**: debug info emission is controlled by a compiler flag (future `--debug`), but **default is debug-on** — during Phase 25 all codegen produces debug info unless explicitly disabled
- A future `--release` or `--strip-debug` flag will suppress debug info

</decisions>

<specifics>
## Specific Ideas

- Full .writil binary output enables end-to-end testing: compile a .writ file → produce .writil → verify byte-level correctness
- All-register DebugLocal gives game modders maximum visibility when debugging scripts
- Pre-pass Error node check + pipeline short-circuit means codegen can assume a clean, fully-typed AST — no defensive checks needed inside instruction emission
- Typed Instruction enum enables potential optimization or verification passes between codegen and serialization

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ModuleBuilder` (`writ-compiler/src/emit/module_builder.rs`): Populated metadata skeleton with DefId→MetadataToken mapping, string/blob heaps, all 21 table row types
- `MetadataToken` / `TableId` (`writ-compiler/src/emit/metadata.rs`): Token encoding (table ID + row index), all 21 table IDs, row struct types, flag encoding helpers
- `TypedAst` / `TypedExpr` / `TypedStmt` / `TypedDecl` (`writ-compiler/src/check/ir.rs`): Fully-typed IR with Ty on every node, Capture with ByValue/ByRef modes, TypedPattern for match arms
- `Ty` / `TyKind` / `TyInterner` (`writ-compiler/src/check/ty.rs`): Interned type system — used to determine instruction selection (e.g., INT_ADD vs FLOAT_ADD) and TypeRef blob generation
- `StringHeap` / `BlobHeap` (`writ-compiler/src/emit/heaps.rs`): Already in ModuleBuilder for string/blob interning
- `TypeSigBuilder` (`writ-compiler/src/emit/type_sig.rs`): Existing type signature encoding for metadata — can be reused for register TypeRef table

### Established Patterns
- Two-pass emission pattern (collect then finalize) from Phase 24 — Phase 25 can follow similar pattern: emit instructions, then serialize
- Arena-based ID allocation (DefId) — Register allocation can follow similar LIFO pattern
- `writ-diagnostics` crate for error/warning reporting
- Module structure: `emit/` module already exists with `mod.rs`, `metadata.rs`, `heaps.rs`, `type_sig.rs`, `module_builder.rs`, `slots.rs`, `collect.rs`

### Integration Points
- `emit::emit()` (`writ-compiler/src/emit/mod.rs`): Current entry point returns `(ModuleBuilder, Vec<Diagnostic>)` — needs to be extended or a new entry point added to produce `Vec<u8>`
- `MethodDefRow.body_offset` / `body_size` / `reg_count`: Currently zeroed in Phase 24 — Phase 25 fills these
- Pipeline in `writ-compiler/src/lib.rs`: Must add short-circuit logic — if `check()` reports errors, skip `emit()` entirely
- Plans 25-01 through 25-04 already outlined in ROADMAP: register allocator → call dispatch/object model → arrays/closures/concurrency → enums/boxing/debug info

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 25-il-codegen-method-bodies*
*Context gathered: 2026-03-03*
