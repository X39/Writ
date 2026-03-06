# Phase 48: IL Format - Context

**Gathered:** 2026-03-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Add `Class = 4` to the TypeDef kind enum, bump `format_version` to 3, and ensure round-trip fidelity for kind=4 TypeDefs across the binary module reader/writer, text assembler, and disassembler. Unify the `TypeDefKind` enum across all workspace crates. No VM semantics changes — pure format-layer work.

</domain>

<decisions>
## Implementation Decisions

### Backward Compatibility
- Reject `format_version < 3` at header parse time — reader refuses v2 modules immediately (fail-fast)
- New `DecodeError::UnsupportedVersion(u16)` error variant — distinct from data corruption errors, callers can match specifically
- `Module::new()` hardcodes `format_version = 3`, no API to override
- No mixed-version domain support — all modules must be v3

### Validation Strictness
- Reader errors on unknown TypeDef kind values: new `DecodeError::InvalidTypeDefKind(u8)` variant
- `TypeDefRow::kind` stays as `u8` (matches binary format 1:1, keeps tables crate as raw data layer)
- `ModuleBuilder::add_type_def()` changes signature to accept `TypeDefKind` enum instead of `u8` — compile-time prevention of invalid kinds
- Add `Display` trait to `TypeDefKind` — "struct", "enum", "entity", "component", "class" — for error messages and disassembler

### Assembler/Disassembler
- Disassembler updated: emit `"class"` for `TypeDefKind::Class` in the kind match
- `None` arm in disassembler's TypeDefKind match becomes `unreachable!()` — reader validates kinds, so unknown kinds here are logic bugs
- Text assembler: add `.class` directive parsing (mirrors `.struct`, `.enum`, `.entity`, `.component`)
- Text assembler round-trip test for `.class` types — write `.writil` with `.class`, assemble, disassemble, verify output

### TypeDefKind Unification
- Add `Class = 4` to `writ-module::tables::TypeDefKind` (the canonical source)
- Re-export `TypeDefKind` from `writ_module` crate root (consistent with `Module`, `ModuleBuilder`, `Instruction` re-exports)
- Delete `writ-compiler::emit::metadata::TypeDefKind` — import from `writ_module` instead
- Update all compiler imports (`collect.rs`, `module_builder.rs`, `body/closure.rs`, `emit_tests.rs`, `emit_body_tests.rs`) to use `writ_module::TypeDefKind`
- Update `writ-runtime::virtual_module.rs` to use `TypeDefKind::Enum`, `TypeDefKind::Struct`, `TypeDefKind::Entity` instead of raw integer literals
- Full unification across all 3 consumer crates: writ-module, writ-compiler, writ-runtime

### Claude's Discretion
- Exact error message wording for UnsupportedVersion and InvalidTypeDefKind
- Whether to add helper constants like `CURRENT_FORMAT_VERSION = 3`
- Test organization within existing test files vs new test files

</decisions>

<specifics>
## Specific Ideas

- The user wants strict, fail-fast validation — reject bad data at the earliest possible point rather than letting it propagate
- Type-safe API boundaries preferred over runtime validation — if the type system can prevent a bug, use it
- `TypeDefRow` stays as raw data (u8 kind) because it mirrors the binary format 1:1, but all public-facing APIs should use the enum

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TypeDefKind` enum in `writ-module/src/tables.rs` — add `Class = 4` variant, add `Display` impl
- `ModuleBuilder` in `writ-module/src/builder.rs` — change `add_type_def` kind param from `u8` to `TypeDefKind`
- `DecodeError` in `writ-module/src/error.rs` — add `UnsupportedVersion(u16)` and `InvalidTypeDefKind(u8)` variants
- Existing round-trip tests in `writ-module/tests/round_trip.rs` and `builder_tests.rs` — extend with kind=4 test cases

### Established Patterns
- Reader/writer pass raw bytes — `TypeDefRow::kind` is `u8`, validation at boundaries
- Builder API uses `ModuleBuilder::add_type_def(name, namespace, kind, flags)` — change `kind: u8` to `kind: TypeDefKind`
- Disassembler matches `TypeDefKind::from_u8(td.kind)` with `Some(variant)` arms
- Assembler handles directives like `.struct`, `.enum`, `.entity`, `.component` in its parser

### Integration Points
- `writ-module/src/lib.rs` — add `pub use tables::TypeDefKind` re-export
- `writ-compiler/src/emit/metadata.rs` — delete local `TypeDefKind`, add `use writ_module::TypeDefKind`
- `writ-compiler/src/emit/collect.rs` — update 7 call sites from local to writ_module enum
- `writ-compiler/src/emit/module_builder.rs` — update `add_type_def` signature
- `writ-compiler/src/emit/body/closure.rs` — update import
- `writ-runtime/src/virtual_module.rs` — replace raw integer kind values (0, 1, 2) with TypeDefKind variants
- `writ-assembler/src/disassembler.rs` — add `Class` match arm, change `None` to `unreachable!()`

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 48-il-format*
*Context gathered: 2026-03-12*
