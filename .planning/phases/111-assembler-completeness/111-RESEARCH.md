# Phase 111: Assembler Completeness - Research

**Researched:** 2026-03-29
**Domain:** writ-assembler crate — text IL assembler/disassembler parity
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — infrastructure phase.

### Claude's Discretion
All implementation choices are at Claude's discretion — pure infrastructure phase. Use ROADMAP phase goal, success criteria, and codebase conventions to guide decisions.

### Deferred Ideas (OUT OF SCOPE)
None — infrastructure phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ASM-01 | Assembler supports `.export`, `.extern_fn`, `.component`, `.locale`, `.attribute` directives (round-trip with disassembler) | Disassembler currently emits these as comments (sections 6, 8–11 in `disassemble_inner`). All five builder methods already exist in `ModuleBuilder`. Both lexer and parser need extensions. |
| ASM-02 | Register type blob offsets are real values, not 0 placeholders | `assemble_method_body` line 275: `let register_types = vec![0u32; method.registers.len()]`. The fix requires interning each `.reg` declaration's type blob into the module blob heap after `builder.build()`, then patching `method_bodies[i].register_types`. |
</phase_requirements>

## Summary

Phase 111 is a targeted gap-fill inside `writ-assembler`. The disassembler (`disassembler.rs`) can decode all table entries a compiled module contains, but five of them are deliberately emitted as comments (with a note that "parser doesn't support" them). The assembler (`assembler.rs`) and parser (`parser.rs`) never handle those directives, so the assemble-disassemble-reassemble round-trip destroys information for `.export`, `.extern_fn`, `.component`, `.locale`, and `.attribute`. Additionally, the assembler always writes `0` for every register type blob offset rather than interning the actual type signature, which means a binary assembled from text never carries the per-register type metadata that the compiler emits.

The fix is self-contained within `writ-assembler/src/`. All five `ModuleBuilder` API methods exist (`add_export_def`, `add_extern_def`, `add_component_slot`, `add_locale_def`, `add_attribute_def`). The required table structs (`ExportDefRow`, `ExternDefRow`, `ComponentSlotRow`, `LocaleDefRow`, `AttributeDefRow`) are already in `writ-module/src/tables.rs`. The only work is: (1) extend the lexer's known-directives list, (2) add five `parse_*` methods and wire them into `parse_module`, (3) add five new AST node types, (4) call the five builder methods from `assemble_module`, and (5) patch register type blob offsets after the module is built.

The register-type fix (ASM-02) is slightly subtle: `ModuleBuilder::build()` takes ownership of `self` and interns all blobs at that point. The assembler calls `builder.build()` to get the module, then replaces method bodies. The fix must replicate the blob-interning logic (using `writ_module::heap::write_blob`) on the already-built `module.blob_heap`, matching exactly what `ModuleBuilder::build()` does.

**Primary recommendation:** Implement all changes in a single plan — lexer extension, AST additions, parser additions, assembler additions, register-type blob fix — with one round-trip integration test that exercises all five directives.

## Standard Stack

No new dependencies required. All changes are within `writ-assembler/src/`.

| Crate | Purpose | Location |
|-------|---------|----------|
| `writ-assembler` | Text IL assembler/disassembler | `writ-assembler/src/` |
| `writ-module` | Binary module format, builder, heap | `writ-module/src/` |

### Key APIs already present in writ-module

| API | Location | Notes |
|-----|----------|-------|
| `ModuleBuilder::add_export_def(name, item_kind, item)` | `builder.rs:339` | Returns `MetadataToken` |
| `ModuleBuilder::add_extern_def(name, sig, import_name, flags)` | `builder.rs:299` | Returns `MetadataToken` |
| `ModuleBuilder::add_component_slot(owner, component_type)` | `builder.rs:321` | Returns `MetadataToken` |
| `ModuleBuilder::add_locale_def(dlg_method, locale, loc_method)` | `builder.rs:328` | Returns `MetadataToken` |
| `ModuleBuilder::add_attribute_def(owner, owner_kind, name, value)` | `builder.rs:350` | Returns `MetadataToken` |
| `heap::write_blob(heap, data)` | `heap.rs:53` | Returns `u32` blob offset — used for ASM-02 |

## Architecture Patterns

### How the current assembler pipeline works

```
assemble(src: &str) -> Result<Module, Vec<AssembleError>>
  ├── lexer::tokenize(src)       → Vec<Token>
  ├── parser::parse(&tokens)     → AsmModule  (AST)
  └── assembler::assemble_module(ast) → Module
        ├── Phase 1: declare all entities via ModuleBuilder calls
        └── Phase 2: assemble method bodies
              └── builder.build() → Module  (then patch method_bodies[i])
```

### Disassembler comment sections to convert into real directives

The following sections in `disassemble_inner` currently emit `// .directive ...` comments. Each must become a real parseable directive:

| Disassembler section | Comment form | Real directive form |
|----------------------|--------------|---------------------|
| §6 Extern functions | `// .extern_fn "name" (params) -> ret "import_name"` | `.extern_fn "name" (params) -> ret "import_name"` |
| §8 Export defs | `// .export "name" method 42` | `.export "name" method 42` |
| §9 Component slots | `// .component_slot 123 456` | `.component_slot EntityToken ComponentToken` |
| §10 Locale defs | `// .locale 123 "en-US" 456` | `.locale MethodToken "locale" MethodToken` |
| §11 Attribute defs | `// .attribute 123 "name"` | `.attribute OwnerToken "name"` |

### Token encoding for directives that reference other items by MetadataToken

The disassembler currently writes raw token integers. For the assembler to assemble them back, the simplest round-trip approach is:

- Disassembler emits tokens as decimal integers (already does this)
- Assembler parses them as `AsmOperand::IntLit` / `u32` literal
- No name-resolution needed for these directives — they use raw token values

This is acceptable for the round-trip requirement. The disassembler already emits resolved names for types/contracts/impls, but for these secondary tables it emits token integers.

### ASM-02: Register type blob offset fix

Current code (`assembler.rs`, lines 273–275):
```rust
// Register types: store 0 as placeholder blob heap offsets.
// ModuleBuilder doesn't expose blob heap for external interning of register types.
let register_types = vec![0u32; method.registers.len()];
```

The fix requires encoding and interning each register's type signature into the built module's blob heap. After `builder.build()` returns the `Module`, `module.blob_heap` is a `Vec<u8>` that is fully owned and mutable. The fix:

```rust
// Encode and intern each register type into the built module's blob heap
let register_types: Vec<u32> = method.registers.iter().map(|reg| {
    let encoded = encode_type_ref(&reg.type_ref, ctx);
    writ_module::heap::write_blob(&mut module.blob_heap, &encoded)
}).collect();
```

This must happen after `builder.build()` (so the heap is live) but the register_types vec must be placed into the correct `MethodBody`. The current flow assembles all bodies first into `assembled_bodies`, then patches them into `module.method_bodies` after `builder.build()`. The fix is to move register type encoding into the post-build phase, or to defer it to the patching step.

The cleanest approach: compute `register_types` per method during the post-build patching loop (when `module.blob_heap` is already available), not in `assemble_method_body`. Pass the register declarations alongside each assembled body.

### Recommended struct to carry register info from method assembly

```rust
struct AssembledMethod {
    body: MethodBody,           // code, debug_locals, source_spans (register_types stays empty)
    register_decls: Vec<AsmRegDecl>,  // raw register declarations for post-build interning
}
```

After `builder.build()`:
```rust
for (i, am) in assembled_methods.iter().enumerate() {
    let reg_types: Vec<u32> = am.register_decls.iter()
        .map(|r| {
            let encoded = encode_type_ref(&r.type_ref, &ctx);
            writ_module::heap::write_blob(&mut module.blob_heap, &encoded)
        })
        .collect();
    module.method_bodies[i] = MethodBody {
        register_types: reg_types,
        ..am.body.clone()
    };
}
```

### New AST nodes needed

```rust
/// Export def: `.export "name" method|type|global TokenInt`
pub struct AsmExport {
    pub name: String,
    pub item_kind: u8,   // 0=method, 1=type, 2=global
    pub item_token: u32, // raw MetadataToken integer
}

/// Extern function: `.extern_fn "name" (params) -> ret "import_name"`
// Already exists as AsmExternFn in ast.rs

/// Component slot: `.component_slot EntityToken ComponentToken`
pub struct AsmComponentSlot {
    pub owner_entity: u32,    // raw token
    pub component_type: u32,  // raw token
}

/// Locale def: `.locale DlgMethodToken "locale" LocMethodToken`
pub struct AsmLocaleDef {
    pub dlg_method: u32,  // raw token
    pub locale: String,
    pub loc_method: u32,  // raw token
}

/// Attribute def: `.attribute OwnerToken "name"`
pub struct AsmAttributeDef {
    pub owner: u32,    // raw token
    pub name: String,
}
```

Note: `AsmExternFn` already exists in `ast.rs` (line 167). `AsmModule` already has `extern_fns: Vec<AsmExternFn>` (line 11). The `assembler.rs` already calls `add_extern_def` (line 136). The only missing piece for `.extern_fn` is: (a) lexer recognizes `extern_fn` as a directive, and (b) parser dispatches to `parse_extern_fn`.

### AsmModule additions

Add four new vec fields to `AsmModule`:
```rust
pub exports: Vec<AsmExport>,
pub component_slots: Vec<AsmComponentSlot>,
pub locale_defs: Vec<AsmLocaleDef>,
pub attribute_defs: Vec<AsmAttributeDef>,
```

(`extern_fns` already exists.)

### Disassembler updates

For ASM-01, the disassembler must also be updated to emit real directives instead of comments. This ensures the disassembler output is actually parseable, completing the round-trip.

Change each comment-emit block to a real directive emit:

```rust
// Before (section 6):
writeln!(out, "    // .extern_fn {:?} ({}) -> {} {:?}", ...);

// After:
writeln!(out, "    .extern_fn {:?} ({}) -> {} {:?}", ...);
```

Similarly for sections 8–11.

For ASM-02, the disassembler already handles non-zero register types correctly (lines 524–533). When `rt_offset == 0` it falls back to `"int"` with a comment. After the fix, non-zero offsets will decode the actual type, so the disassembler output will carry real types rather than placeholder `int` for all registers.

### Lexer additions

The `known_directives` list (lexer.rs, line 182) must include the new directive names:

```rust
let known_directives = [
    "module", "type", "field", "method", "contract", "impl",
    "reg", "extern", "global", "regs",
    // New:
    "extern_fn", "export", "component_slot", "locale", "attribute",
];
```

Note: `extern_fn` contains an underscore. The lexer already collects alphanumeric-plus-underscore characters for directive names (line 162: `chars[pos].is_alphanumeric() || chars[pos] == '_'`), so this will tokenize correctly.

### Anti-Patterns to Avoid

- **Don't change `parse_extern` for `.extern_fn`:** `.extern` (module refs) and `.extern_fn` (extern function defs) are distinct directives. `parse_extern` parses the module-ref form; add a separate `parse_extern_fn` for the function form.
- **Don't try to intern register types before `builder.build()`:** The blob heap doesn't exist until after `build()`. Attempting to pre-intern will require re-architecting the builder or duplicating heap state.
- **Don't use name resolution for token-referenced items:** `.export`, `.component_slot`, `.locale`, `.attribute` reference items by raw integer token values in the current disassembler output. Parsing them as integer literals avoids needing to resolve names for the round-trip.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Blob heap interning | Custom encode + write loop | `writ_module::heap::write_blob` (already used by builder) |
| Type encoding | Custom byte serialization | `encode_type_ref` already in `assembler.rs:357` |
| Token construction | Manual bit-shifting | `MetadataToken(raw_u32)` or just pass `u32` to builder methods |

## Common Pitfalls

### Pitfall 1: `.extern` vs `.extern_fn` collision
**What goes wrong:** The parser's `parse_extern` currently handles `.extern "name" "version"` (module refs). If `extern_fn` is not added as a distinct directive, the lexer might tokenize `.extern_fn` as directive "extern" followed by identifier "fn", causing a parse error.
**Why it happens:** The lexer stops consuming characters at the first non-`[a-zA-Z0-9_]` character, but underscore IS allowed. So `.extern_fn` tokenizes as `Directive("extern_fn")` correctly — as long as `extern_fn` is in the known-directives list.
**How to avoid:** Add `"extern_fn"` to `known_directives`. The lexer's character collection already handles underscore.
**Warning signs:** Parse test for `.extern_fn` fails with "expected string literal" (meaning it parsed as `Directive("extern")` + identifier "fn").

### Pitfall 2: `assemble_method_body` returns register_types before blob heap exists
**What goes wrong:** `assemble_method_body` currently builds the `MethodBody` and sets `register_types = vec![0u32; ...]`. If you try to call `heap::write_blob` there, the blob heap isn't available (it's inside `ModuleBuilder`, which hasn't called `build()` yet).
**Why it happens:** `ModuleBuilder::build()` calls `intern_blob` closures during `build()`. The blob heap is initialized inside `build()` and returned as part of the `Module`.
**How to avoid:** Move register type interning to the post-build patch loop. Return register decls from `assemble_method_body` alongside the body, then intern types after `builder.build()`.
**Warning signs:** Borrow checker error trying to call builder methods after partial build, or `module.blob_heap` is read-only.

### Pitfall 3: Round-trip byte equality fails due to heap order
**What goes wrong:** "Assemble → disassemble → re-assemble produces same bytes" can fail if the second assembly interns blobs/strings in a different order than the first.
**Why it happens:** The disassembler emits directives in a fixed order. If the second assembly processes them in the same order (same parser/assembler logic), the heap layout should match. But if any directive emits strings/blobs in a different order than the original compiler output, offsets will differ.
**How to avoid:** The round-trip requirement should be stated as "functionally identical" (same table counts, same decoded content) rather than "byte-identical." The success criterion says "assemble → disassemble → re-assemble produces same bytes" — this is achievable only if the second assembly goes through exactly the same intern sequence. For the assembler-to-assembler case (text → binary → text → binary), heap order IS deterministic because both go through the same code path. Verify with the byte-equality test after implementation.
**Warning signs:** Test comparing `m1.to_bytes()` vs `m2.to_bytes()` fails despite identical table counts.

### Pitfall 4: `item_kind` semantics for `.export`
**What goes wrong:** Assembling `.export "name" method 42` requires knowing the `item_kind` integer for "method" vs "type" vs "global".
**Why it happens:** The disassembler maps `item_kind` `0 → "method"`, `1 → "type"`, `2 → "global"`. The parser must invert this.
**How to avoid:** Parse the string keyword after the name: `method → 0`, `type → 1`, `global → 2`. The disassembler already uses these exact strings (line 238–241 in disassembler.rs).

### Pitfall 5: `attribute_def` owner_kind vs. `value` field
**What goes wrong:** `AttributeDefRow` has `owner_kind: u8` and `value: u32` (blob offset). The disassembler currently only emits `owner` (MetadataToken) and `name` (string), not `owner_kind` or `value`.
**Why it happens:** The current comment form is `// .attribute 123 "name"` — it drops `owner_kind` and `value`. When implementing a real `.attribute` directive, the assembler can default `owner_kind = 0` and `value = []` (empty blob), matching what the disassembler loses.
**How to avoid:** For the round-trip requirement, the disassembler must emit `owner_kind` as well if it's non-zero. Extend the disassembler's attribute emit to include `owner_kind` in the output, and parse it back. Use `ATTR_OWNER_KIND_DECL = 3` (from `tables.rs:275`) as the constant for declaration-type attributes.

## Code Examples

### Verified pattern: how a directive is added end-to-end (using `.global` as the model)

Lexer (`lexer.rs:182–188`): `"global"` is in `known_directives` → emits `Directive("global")`.

Parser (`parser.rs:197–200`): dispatches to `parse_global()`.

Parser method (`parser.rs:911–917`):
```rust
fn parse_global(&mut self) -> Option<AsmGlobal> {
    self.pos += 1; // consume .global
    let name = self.expect_string()?;
    let type_ref = self.parse_type_ref()?;
    let flags = self.parse_flags();
    Some(AsmGlobal { name, type_ref, flags, init_value: None })
}
```

Assembler (`assembler.rs:127–132`):
```rust
for global in &ast.globals {
    let type_sig = encode_type_ref(&global.type_ref, &ctx);
    let init = global.init_value.as_deref().unwrap_or(&[]);
    builder.add_global_def(&global.name, &type_sig, global.flags, init);
}
```

All five new directives follow exactly this same pattern.

### Verified pattern: `heap::write_blob` for register types (ASM-02)

After `let mut module = builder.build();`, and after computing `ctx` (which is already passed to all encode functions):

```rust
// Source: writ-module/src/heap.rs:53
// pub fn write_blob(heap: &mut Vec<u8>, data: &[u8]) -> u32

for (i, (method, _owner)) in all_methods.iter().enumerate() {
    let reg_types: Vec<u32> = method.registers.iter()
        .map(|reg| {
            let encoded = encode_type_ref(&reg.type_ref, &ctx);
            writ_module::heap::write_blob(&mut module.blob_heap, &encoded)
        })
        .collect();

    if i < module.method_bodies.len() {
        module.method_bodies[i].register_types = reg_types;
    }
}
```

This replaces the `module.method_bodies[i] = body` loop at `assembler.rs:185–190`.

### Verified disassembler patterns for the five directive forms

From `disassembler.rs` (current comment forms, to be de-commented):

```
// Section 6 (line 201-207) - to become:
.extern_fn "name" (param_types) -> ret_type "import_name"

// Section 8 (line 235-243) - to become:
.export "name" method|type|global TokenInt

// Section 9 (line 245-247) - to become:
.component_slot EntityTokenInt ComponentTokenInt

// Section 10 (line 249-252) - to become:
.locale DlgMethodTokenInt "locale_string" LocMethodTokenInt

// Section 11 (line 254-258) - to become:
.attribute OwnerTokenInt OwnerKindInt "name"
```

## State of the Art

This is a project-internal implementation gap — no external libraries or patterns apply. All context is from the codebase itself.

| Current State | Target State |
|---------------|--------------|
| 5 directives emitted as comments by disassembler | 5 directives parsed and assembled |
| Register type blob offsets always 0 | Register type blob offsets are real heap positions |
| Round-trip loses export/extern_fn/component/locale/attribute data | Round-trip is lossless |

## Open Questions

1. **Attribute `value` field round-trip**
   - What we know: `AttributeDefRow.value` is a blob heap offset for attribute value data. The current disassembler comment doesn't emit it.
   - What's unclear: Are any attributes in practice emitted with non-empty value blobs by the compiler? If so, dropping them would fail byte-exact round-trip for those modules.
   - Recommendation: Check the compiler's attribute emission. If `value` is always empty (blob `[]`), drop it. If it can be non-empty, extend the disassembler emit and parser to handle hex-encoded blob values. For now, assume empty value is correct (default: `add_attribute_def(owner, owner_kind, name, &[])`).

2. **`.component_slot` and `.locale` token references**
   - What we know: The disassembler emits raw MetadataToken integers (e.g., `cs.owner_entity.0`). The assembler will parse these as `u32` literals.
   - What's unclear: Would name-based references (e.g., entity type name instead of token integer) be more human-friendly?
   - Recommendation: For this phase, use raw token integers to match the current disassembler output format. Human-readable names are a future enhancement.

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — all changes are in-process Rust code within existing crates).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) |
| Config file | None (standard Rust integration tests) |
| Quick run command | `cargo test -p writ-assembler` |
| Full suite command | `cargo test -p writ-assembler -p writ-module` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ASM-01 | `.export` directive parses and assembles | unit/integration | `cargo test -p writ-assembler round_trip_export` | ❌ Wave 0 |
| ASM-01 | `.extern_fn` directive parses and assembles | unit/integration | `cargo test -p writ-assembler round_trip_extern_fn` | ❌ Wave 0 |
| ASM-01 | `.component_slot` directive parses and assembles | unit/integration | `cargo test -p writ-assembler round_trip_component_slot` | ❌ Wave 0 |
| ASM-01 | `.locale` directive parses and assembles | unit/integration | `cargo test -p writ-assembler round_trip_locale` | ❌ Wave 0 |
| ASM-01 | `.attribute` directive parses and assembles | unit/integration | `cargo test -p writ-assembler round_trip_attribute` | ❌ Wave 0 |
| ASM-01 | Full disassembler round-trip (all 5 directives together) | integration | `cargo test -p writ-assembler round_trip_all_new_directives` | ❌ Wave 0 |
| ASM-02 | Register types are real blob offsets, not 0 | unit | `cargo test -p writ-assembler register_types_real_offsets` | ❌ Wave 0 |
| both | Existing round-trip tests still pass | regression | `cargo test -p writ-assembler` | ✅ exists |

### Sampling Rate

- **Per task commit:** `cargo test -p writ-assembler`
- **Per wave merge:** `cargo test -p writ-assembler -p writ-module`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `writ-assembler/tests/asm_round_trip.rs` — add `round_trip_export`, `round_trip_extern_fn`, `round_trip_component_slot`, `round_trip_locale`, `round_trip_attribute`, `round_trip_all_new_directives` tests (extend existing file)
- [ ] `writ-assembler/tests/asm_round_trip.rs` — add `register_types_real_offsets` test

## Sources

### Primary (HIGH confidence)
- Direct source reading: `writ-assembler/src/disassembler.rs` — identified comment sections 6, 8, 9, 10, 11
- Direct source reading: `writ-assembler/src/assembler.rs` — identified register_types = vec![0] on line 275
- Direct source reading: `writ-assembler/src/parser.rs` — identified missing directive dispatch cases
- Direct source reading: `writ-assembler/src/lexer.rs` — confirmed underscore support in directive names, identified `known_directives` list
- Direct source reading: `writ-module/src/builder.rs` — confirmed all 5 builder methods exist
- Direct source reading: `writ-module/src/heap.rs` — confirmed `write_blob` API
- Direct source reading: `writ-module/src/tables.rs` — confirmed all table row structs

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all code read directly from source
- Architecture: HIGH — complete pipeline trace from lexer through builder
- Pitfalls: HIGH — identified from actual code paths, not speculation
- Register type fix: HIGH — exact line identified, fix pattern confirmed from existing code

**Research date:** 2026-03-29
**Valid until:** Stable until codebase changes — no external dependencies
