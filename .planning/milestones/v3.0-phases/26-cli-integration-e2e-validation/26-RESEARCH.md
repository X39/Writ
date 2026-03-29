# Phase 26: CLI Integration and End-to-End Validation - Research

**Researched:** 2026-03-03
**Domain:** Rust CLI toolchain integration, runtime lifecycle hooks, generic dispatch
**Confidence:** HIGH

## Summary

Phase 26 has six requirements split into two distinct tracks: (A) CLI integration (CLI-01, CLI-02, CLI-03) wires `writ-compiler`'s existing `emit_bodies()` public API into the `writ` binary as a new `compile` subcommand; and (B) runtime gap closure (FIX-01, FIX-02, FIX-03) fixes three known bugs in `writ-runtime` and `writ-cli` that block e2e validation.

The three CLI requirements are straightforward plumbing: `emit_bodies()` already exists at `writ-compiler::emit_bodies`, the parser and pipeline are already operational, and `ariadne` (already a `writ-diagnostics` dep) renders errors. The primary integration work is (1) adding `writ-compiler` as a dep of `writ-cli`, (2) implementing `cmd_compile()` that chains `writ_parser::parse` -> `writ_compiler::lower` -> `writ_compiler::resolve::resolve` -> `writ_compiler::check::typecheck` -> `writ_compiler::emit_bodies`, (3) rendering any diagnostics via `writ_diagnostics::render_diagnostics`, and (4) writing the binary to a `.writil` file.

The three runtime fixes require targeted surgical changes. FIX-01 (lifecycle hooks) requires `writ-runtime` to scan MethodDef names ("on_create", "on_destroy", "on_interact", "on_finalize") for entity types and call them at the appropriate instruction handler sites - the infrastructure (two-phase destroy protocol, EntityRegistry, call-stack frame pushing) already exists. FIX-02 (generic dispatch collision) requires including the generic type argument blob in the DispatchKey so `Int:Into<Float>` and `Int:Into<String>` do not overwrite each other. FIX-03 (CliHost Ref display) requires passing the runtime's `GcHeap` reference into `CliHost::on_request` via a new `format_value_with_heap` helper or an `ExternCall` variant that pre-resolves strings.

**Primary recommendation:** Implement in two waves: Wave 1 = CLI plumbing (CLI-01, CLI-02, CLI-03); Wave 2 = runtime fixes (FIX-01, FIX-02, FIX-03). Each wave can be tested independently before the combined e2e integration tests are run.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLI-01 | `writ compile` subcommand accepts .writ source file(s) and outputs .writil binary module | Clap `Subcommand` derive pattern already in use in `writ-cli/src/main.rs`; `emit_bodies()` public API exists in `writ-compiler::emit`; output path convention matches `.writil` -> `.writc` pattern already present |
| CLI-02 | Compiler pipeline runs end-to-end: parse -> lower -> resolve -> typecheck -> codegen -> write module | All five pipeline stages have public entry points: `writ_parser::parse`, `writ_compiler::lower`, `writ_compiler::resolve::resolve`, `writ_compiler::check::typecheck`, `writ_compiler::emit_bodies`; stages are already sequenced in tests |
| CLI-03 | Compilation errors display with source spans, multi-span context (ariadne), and actionable messages | `writ_diagnostics::render_diagnostics` is already implemented; `ariadne = "0.6"` is a `writ-diagnostics` dep; the `Diagnostic` builder pattern supports primary + secondary labels, help text, and notes |
| FIX-01 | Runtime dispatches lifecycle hooks (on_create, on_destroy, on_interact, on_finalize) via method name lookup | Infrastructure exists: EntityRegistry has `begin_destroy`/`complete_destroy`, DESTROY_ENTITY sets Destroying state and decrements PC (two-phase protocol). Hook lookup is the missing piece: scan MethodDefs for entity TypeDef by name. The comment at `dispatch.rs:793` marks the exact insertion point |
| FIX-02 | Runtime resolves generic contract specialization without collision | `domain.rs:377-379` has a comment documenting the known bug. Fix: extend DispatchKey with a `type_args: u32` field (hash of the generic arg blob) so `(Int, Into, Float)` and `(Int, Into, String)` get different table entries |
| FIX-03 | CliHost dereferences GC heap Ref values for display (strings, objects) | `cli_host.rs:66-76` documents the limitation. Runtime already exposes `heap.as_ref()` internally; the fix requires threading a heap reference into `CliHost` (either via a callback closure or by changing `ExternCall` to pre-resolve Ref args in the runtime before issuing the request) |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.5 (already dep) | CLI argument parsing with `Subcommand` derive | Already used for `assemble`/`disasm`/`run` commands |
| ariadne | 0.6 (already dep via writ-diagnostics) | Source span rendering with colors and multi-span labels | Already integrated in `writ_diagnostics::render_diagnostics` |
| writ-compiler | workspace | Parse -> lower -> resolve -> typecheck -> emit_bodies pipeline | All stages implemented; `emit_bodies()` is the Phase 25 public API |
| writ-diagnostics | workspace | `Diagnostic`, `render_diagnostics`, `FileId` | Shared crate; already used by writ-compiler and writ-cli tests |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| writ-parser | workspace | CST parse (entry point: `writ_parser::parse`) | First pipeline stage in `cmd_compile` |
| writ-runtime | workspace (already dep) | Entity registry, dispatch, GC heap for FIX-01/02/03 | Runtime gap closure in Wave 2 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Pre-resolving strings in runtime before ExternCall (FIX-03) | Passing GcHeap reference to CliHost | Pre-resolving keeps the host interface clean; passing heap ref requires a trait extension or unsafe |
| Hash of type_args blob for DispatchKey (FIX-02) | Full Vec<u8> key | Hash is O(1) lookup; Vec key requires HashMap with custom Eq; hash is good enough for CLI e2e validation |

**Installation:**
No new crates needed. `writ-cli/Cargo.toml` needs one additional dep:
```toml
writ-compiler = { path = "../writ-compiler" }
writ-diagnostics = { path = "../writ-diagnostics" }
```

## Architecture Patterns

### Recommended Project Structure
The phase adds to existing files; no new source files are required. Changes:
```
writ-cli/
├── src/
│   ├── main.rs          # Add Commands::Compile variant + cmd_compile() function
│   └── cli_host.rs      # FIX-03: format_value_with_heap(), or runtime pre-resolution
├── Cargo.toml           # Add writ-compiler + writ-diagnostics deps
writ-runtime/
├── src/
│   ├── dispatch.rs      # FIX-01: hook lookup + frame push at INIT_ENTITY/DESTROY_ENTITY
│   └── domain.rs        # FIX-02: extend DispatchKey with type_args discriminator
```

### Pattern 1: Pipeline Chaining in cmd_compile
**What:** All five pipeline stages are chained in sequence, with each stage's error output converted to `Vec<Diagnostic>` and rendered via `render_diagnostics` before exiting.
**When to use:** Every time a source file is compiled.
**Example:**
```rust
fn cmd_compile(input: String, output: Option<String>) -> Result<(), String> {
    let src = std::fs::read_to_string(&input)
        .map_err(|e| format!("failed to read '{}': {}", input, e))?;

    // Stage 1: Parse
    let (cst, parse_errs) = writ_parser::parse(&src);
    if !parse_errs.is_empty() {
        // parse errors are already rendered by parser (check parser API)
        return Err(format!("{} parse error(s)", parse_errs.len()));
    }

    // Stage 2: Lower CST -> AST
    let (ast, lower_errs) = writ_compiler::lower(cst);
    if !lower_errs.is_empty() {
        let diags: Vec<_> = lower_errs.iter().map(|e| e.to_diagnostic(FileId(0))).collect();
        eprint!("{}", render_diagnostics(&diags, &[(FileId(0), &input, &src)]));
        return Err(format!("{} lowering error(s)", diags.len()));
    }

    // Stage 3: Name resolution
    let file_id = FileId(0);
    let (resolved, resolve_diags) = writ_compiler::resolve::resolve(
        &[(file_id, &ast)],
        &[(file_id, &input)],
    );
    if resolve_diags.iter().any(|d| d.severity == Severity::Error) {
        eprint!("{}", render_diagnostics(&resolve_diags, &[(file_id, &input, &src)]));
        return Err(format!("{} error(s)", resolve_diags.len()));
    }

    // Stage 4: Type checking
    let (typed_ast, interner, type_diags) = writ_compiler::check::typecheck(resolved, &[(file_id, &ast)]);
    if type_diags.iter().any(|d| d.severity == Severity::Error) {
        eprint!("{}", render_diagnostics(&type_diags, &[(file_id, &input, &src)]));
        return Err(format!("{} error(s)", type_diags.len()));
    }

    // Stage 5: Codegen
    let bytes = writ_compiler::emit_bodies(&typed_ast, &interner).map_err(|diags| {
        let rendered = render_diagnostics(&diags, &[(file_id, &input, &src)]);
        eprint!("{rendered}");
        format!("{} codegen error(s)", diags.len())
    })?;

    // Write output
    let out_path = output.unwrap_or_else(|| {
        if input.ends_with(".writ") {
            input[..input.len() - 5].to_string() + ".writil"
        } else {
            input.clone() + ".writil"
        }
    });
    std::fs::write(&out_path, &bytes)
        .map_err(|e| format!("failed to write '{}': {}", out_path, e))?;
    eprintln!("Compiled: {out_path}");
    Ok(())
}
```

### Pattern 2: FIX-01 Hook Lookup by Method Name
**What:** After `INIT_ENTITY` transitions the entity to Alive, scan the entity's TypeDef method range for methods named "on_create" and push a call frame. For `DESTROY_ENTITY`, scan for "on_destroy" and push a frame before decrementing PC.
**When to use:** At `INIT_ENTITY` and `DESTROY_ENTITY` instruction handlers in `dispatch.rs`.
**Example:**
```rust
// In INIT_ENTITY handler, after commit_init():
if let Some(hook_method_idx) = find_hook_by_name(&module.module, type_idx as usize, "on_create") {
    // Push hook frame: self = entity handle in r0
    let reg_count = get_method_reg_count(&module.module, hook_method_idx);
    let mut hook_frame = CallFrame::new(hook_method_idx, reg_count);
    hook_frame.registers[0] = Value::Entity(entity_id);
    task.call_stack.push(hook_frame);
}

// Helper: scan TypeDef's method range by name
fn find_hook_by_name(module: &Module, type_idx: usize, name: &str) -> Option<usize> {
    let td = &module.type_defs[type_idx];
    let method_start = td.method_list.saturating_sub(1) as usize;
    let method_end = if type_idx + 1 < module.type_defs.len() {
        module.type_defs[type_idx + 1].method_list.saturating_sub(1) as usize
    } else {
        module.method_defs.len()
    };
    for idx in method_start..method_end {
        let md_name = read_string(&module.string_heap, module.method_defs[idx].name).unwrap_or("");
        if md_name == name {
            return Some(idx);
        }
    }
    None
}
```

### Pattern 3: FIX-02 Generic Dispatch Key Extension
**What:** Add a `type_args_hash: u32` field to `DispatchKey`. Compute it by hashing the ImplDef's type argument blob (or the contract token's generic arg suffix). This makes `(Int, Into, Float)` and `(Int, Into, String)` unique keys.
**When to use:** In `domain.rs::build_dispatch_table` when computing the dispatch key for each ImplDef.
**Example:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DispatchKey {
    pub type_key: u32,
    pub contract_key: u32,
    pub slot: u16,
    pub type_args_hash: u32,  // NEW: 0 for non-generic impls
}
```
The `type_args_hash` is computed from the generic argument tokens in the ImplDef. For non-generic impls it is 0 (no collision possible). The hash can be a simple FNV-1a over the contract token's generic arg blob bytes.

### Pattern 4: FIX-03 String Dereferencing in CliHost
**What:** The runtime pre-resolves `Value::Ref` arguments to strings before issuing `ExternCall` to the host. This keeps the `RuntimeHost` trait clean (no heap exposure) and is the architecturally simpler approach.
**When to use:** In `dispatch.rs` at the `CallExtern` handler, immediately before issuing `HostRequest::ExternCall`.
**Example:**
```rust
// In CALL_EXTERN handler, before constructing HostRequest::ExternCall:
let resolved_args: Vec<Value> = args.iter().map(|v| {
    if let Value::Ref(href) = v {
        if let Ok(s) = heap.read_string(*href) {
            // Re-encode as a synthetic value the host can display
            // Option A: add Value::Str(String) variant (requires value.rs change)
            // Option B: pass Ref through and have CliHost call heap.read_string
        }
    }
    *v
}).collect();
```
**Simpler alternative for FIX-03:** Give `CliHost` a `heap: Arc<Mutex<dyn GcHeap>>` snapshot, or add a `read_heap_string(href: HeapRef) -> Option<String>` method to `Runtime` and pass it as a callback. The cleanest approach that avoids trait changes: add a `Value::Str(String)` variant to `Value` and have the runtime resolve Ref->Str before ExternCall args. However, that changes the value enum across the codebase.

**Recommended approach for FIX-03:** The runtime passes a `heap_snapshot: &dyn GcHeap` reference to the host along with the ExternCall, or simply resolves `Value::Ref` to `Value::Str(String)` specifically for `ExternCall` args only (a local decision in the CALL_EXTERN dispatch handler). The `cli_host.rs` comment at line 15-23 already enumerates the two options. Use approach (a): pre-resolve in the runtime dispatcher before issuing `HostRequest::ExternCall` by replacing `Value::Ref(href)` with a new `Value::Str(String)` variant, or the simpler approach of just extracting the string and formatting it inline before creating the request. Given that `ExternCall` args already copy values into a `Vec<Value>`, changing those copies to resolve strings is safe.

### Anti-Patterns to Avoid
- **Skipping the error-stop discipline:** Each pipeline stage MUST check for errors before calling the next. Phase spec says "errors in resolve prevent typecheck from running." Don't call `typecheck` if `resolve_diags` has errors.
- **Using `emit()` instead of `emit_bodies()`:** `emit()` is the Phase 24 metadata-only function. `emit_bodies()` is the Phase 25 entry point that produces serializable binary. Use `emit_bodies()` in `cmd_compile`.
- **Hook collision via double fire:** The two-phase destroy protocol already decrements PC so `DESTROY_ENTITY` re-executes after the hook frame returns. Don't push the hook frame unconditionally; check `entity_registry.get_state()` first to distinguish first-entry (Alive->Destroying) from second-entry (Destroying->complete).
- **Key collision for zero generic impls:** When computing `type_args_hash` for FIX-02, non-generic impls must produce `0`, not a hash of an empty blob. This ensures non-generic impls always collapse to the same key as before, preserving the existing dispatch behavior.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Error rendering | Custom span formatter | `writ_diagnostics::render_diagnostics` | Already handles multi-span, colors, ariadne integration, error codes |
| Method name lookup in TypeDef | Custom iteration | `Domain::find_method_in_type` (private, but the pattern is already in `domain.rs:320-335`) | Copy the pattern directly; it handles the method_list boundary correctly |
| Pipeline sequencing | Custom orchestrator | Linear function calls in `cmd_compile` | The pipeline is already linear; no dependency injection or registry needed |

**Key insight:** This phase is almost entirely wiring. The computable work (codegen, dispatch, error rendering) already exists. The risk is in getting the connection points right, not in building new algorithms.

## Common Pitfalls

### Pitfall 1: LoweringError to Diagnostic Conversion
**What goes wrong:** `LoweringError` (in `writ_compiler::lower::error`) may not implement `to_diagnostic()`. If the conversion is missing, `cmd_compile` cannot render lowering errors with source spans.
**Why it happens:** Lower errors were implemented in Phase 22; the diagnostic conversion may have been left as a stub.
**How to avoid:** Check `writ-compiler/src/lower/error.rs` for a `to_diagnostic` or `into_diagnostic` method. If absent, add it during Wave 1 implementation.
**Warning signs:** `lower_errs` produces opaque strings without file/span info.

### Pitfall 2: emit_bodies Signature Gap
**What goes wrong:** `emit_bodies(typed_ast, interner)` in `emit/mod.rs` does NOT take the original `asts: &[(FileId, &Ast)]` slice (unlike `emit()`). It therefore calls `builder.set_module_def("module", "0.1.0", 0)` with a hardcoded module name.
**Why it happens:** Phase 25 simplified the signature for testing. The module name/version must be read from the source file (or a manifest) in the real pipeline.
**How to avoid:** Either extend `emit_bodies()` to accept an optional module name+version parameter, or read them from the file. For Phase 26 minimum viable approach: the module name can be derived from the source filename.
**Warning signs:** All compiled modules appear with name "module" in the binary header.

### Pitfall 3: FIX-01 Hook Frame Register Count
**What goes wrong:** Pushing a hook frame with wrong register count causes out-of-bounds access when the hook body accesses `r0` (self).
**Why it happens:** `get_method_reg_count` must read from `MethodDef.reg_count`, not hardcode a value.
**How to avoid:** Read `module.method_defs[hook_method_idx].reg_count` to size the register array.
**Warning signs:** Runtime crash with "register index out of range" in hook methods.

### Pitfall 4: FIX-02 Type-Args Discriminator Source
**What goes wrong:** The generic type argument is encoded in the ImplDef's `contract` token as a TypeRef that includes the generic arg blob, not as a separate field.
**Why it happens:** The IL spec stores generic specialization in the TypeRef blob heap entry for the contract, not in the ImplDef row itself.
**How to avoid:** Read the contract TypeRef's signature blob from the blob heap and hash it. If the blob heap offset is 0 (no generics), `type_args_hash = 0`.
**Warning signs:** Two specializations still collide if the hash is computed from the wrong field.

### Pitfall 5: FIX-03 Value Enum Ripple
**What goes wrong:** Adding `Value::Str(String)` to the `Value` enum causes exhaustive match failures across all match arms in `dispatch.rs`, `frame.rs`, `value.rs`, etc.
**Why it happens:** `Value` is matched exhaustively in many places.
**How to avoid:** Use the less invasive approach: in the CALL_EXTERN handler, collect a `Vec<String>` for display separately from the `Vec<Value>` passed as args. Alternatively, resolve `Value::Ref` to the string content just before creating the `HostRequest` and store the resolved strings in a parallel `Vec<Option<String>>` alongside args.

## Code Examples

Verified patterns from existing source:

### emit_bodies Public API (writ-compiler/src/emit/mod.rs:64-126)
```rust
// Entry point for cmd_compile - already exists
pub fn emit_bodies(
    typed_ast: &TypedAst,
    interner: &TyInterner,
) -> Result<Vec<u8>, Vec<Diagnostic>> { ... }
```

### cmd_assemble Pattern (writ-cli/src/main.rs:93-136)
```rust
// Follow this pattern for cmd_compile - same structure
fn cmd_assemble(input: String, output: Option<String>) -> Result<(), String> {
    let src = std::fs::read_to_string(&input)...;
    let module = writ_assembler::assemble(&src).map_err(...)? ;
    let bytes = module.to_bytes().map_err(...)?;
    std::fs::write(&out_path, &bytes).map_err(...)?;
    eprintln!("Assembled: {out_path}");
    Ok(())
}
```

### Clap Subcommand Addition (writ-cli/src/main.rs:26-69)
```rust
// Add to Commands enum:
Compile {
    /// Input .writ source file
    input: String,
    /// Output .writil binary module (default: replaces .writ with .writil)
    #[arg(short, long)]
    output: Option<String>,
},
```

### Pipeline Entry Points (verified in source)
```rust
// Stage 1: writ_parser::parse(src: &str) -> (Vec<Spanned<Item>>, Vec<ParseError>)
// Stage 2: writ_compiler::lower(items: Vec<Spanned<Item>>) -> (Ast, Vec<LoweringError>)
// Stage 3: writ_compiler::resolve::resolve(asts, file_paths) -> (NameResolvedAst, Vec<Diagnostic>)
// Stage 4: writ_compiler::check::typecheck(resolved, asts) -> (TypedAst, TyInterner, Vec<Diagnostic>)
// Stage 5: writ_compiler::emit_bodies(typed_ast, interner) -> Result<Vec<u8>, Vec<Diagnostic>>
```

### DispatchKey (writ-runtime/src/dispatch.rs:21-27)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DispatchKey {
    pub type_key: u32,
    pub contract_key: u32,
    pub slot: u16,
    // Add for FIX-02:
    // pub type_args_hash: u32,
}
```

### DESTROY_ENTITY Hook Insertion Point (dispatch.rs:789-796)
```rust
// The comment at line 793 marks the exact insertion point for on_destroy hook lookup:
// (on_destroy hook would be pushed here if hook lookup exists)
// For now, without method name scanning, skip directly to completion.
// FIX-01: Replace this comment with:
if let Some(hook_idx) = find_hook_by_name(&module.module, entity_id.type_idx, "on_destroy") {
    push_hook_frame(task, hook_idx, &module.module, Value::Entity(entity_id));
}
```

### INIT_ENTITY Hook Insertion (dispatch.rs:729-756)
```rust
// After commit_init() and field write application:
// FIX-01: Add on_create hook dispatch after INIT_ENTITY completes field writes
let type_idx = entity_registry.get_type_idx(entity_id).unwrap_or(0) as usize;
if let Some(hook_idx) = find_hook_by_name(&module.module, type_idx, "on_create") {
    push_hook_frame(task, hook_idx, &module.module, Value::Entity(entity_id));
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `emit()` metadata-only | `emit_bodies()` full binary | Phase 25 | Phase 26 uses `emit_bodies()`, not `emit()` |
| Hook dispatch deferred | Hook dispatch by method name lookup | Phase 26 | No flags needed; name-based lookup is the spec approach |

**Deprecated/outdated:**
- `writ-compiler/src/main.rs` "Hello, world!": The compiler crate has a `main.rs` stub that does nothing. Phase 26 should NOT use this - the CLI entry point is `writ-cli/src/main.rs`.

## Open Questions

1. **LoweringError::to_diagnostic conversion**
   - What we know: `LoweringError` exists in `writ-compiler/src/lower/error.rs`; it has span information
   - What's unclear: Whether a `to_diagnostic(FileId) -> Diagnostic` method exists or needs to be added
   - Recommendation: Check `lower/error.rs` at plan time; add the conversion if absent (small task)

2. **emit_bodies module name parameter**
   - What we know: `emit_bodies` hardcodes `"module"` and `"0.1.0"` as the module name/version
   - What's unclear: Whether to extend the signature or derive from filename
   - Recommendation: Derive module name from the input filename stem for Phase 26 (e.g., `foo.writ` -> module name `"foo"`); extend signature properly in a future phase

3. **on_interact hook trigger**
   - What we know: on_interact is triggered by a host "Fire event" command, not by an instruction
   - What's unclear: Whether FIX-01 requires on_interact to be triggerable from CliHost (or just the three instruction-bound hooks)
   - Recommendation: FIX-01 success criterion says "hooks fire at correct entity lifecycle events" - on_interact is host-driven and not instruction-bound; implement on_create, on_destroy, on_finalize in Phase 26 and note that on_interact requires a new host command path (may be deferred or a minimal stub)

4. **FIX-02 blob heap access for type_args**
   - What we know: ImplDef.contract is a MetadataToken; the generic type args are encoded in the TypeRef blob
   - What's unclear: Whether the current `writ-module` tables expose the blob heap at the right granularity for hashing
   - Recommendation: Check `writ-module/src/tables.rs` ImplDefRow - if a `type_args_blob` field does not exist, use the full contract MetadataToken value (including generic specialization index) as the discriminator

## Sources

### Primary (HIGH confidence)
- `writ-cli/src/main.rs` - existing CLI pattern (cmd_assemble, clap setup)
- `writ-compiler/src/emit/mod.rs` - `emit_bodies()` API confirmed
- `writ-compiler/src/lib.rs` - public pipeline API re-exports
- `writ-compiler/src/resolve/mod.rs` - `resolve()` entry point
- `writ-compiler/src/check/mod.rs` - `typecheck()` entry point
- `writ-diagnostics/src/render.rs` - `render_diagnostics()` confirmed implemented
- `writ-runtime/src/dispatch.rs:789-796` - hook insertion comment confirmed
- `writ-runtime/src/domain.rs:377-379` - generic dispatch collision comment confirmed
- `writ-cli/src/cli_host.rs:15-23` - FIX-03 limitation documented

### Secondary (MEDIUM confidence)
- Language spec `language-spec/spec/45_2_16_il_module_format.md:143-144` - hook_kind flags defined (but these are IL format flags, not the runtime lookup mechanism; the runtime uses name-based lookup per the FIX-01 requirement description)

### Tertiary (LOW confidence)
- FIX-02 type_args blob structure: inferred from domain.rs code and IL spec; exact blob layout needs verification during implementation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all deps verified in Cargo.toml files; all pipeline entry points verified in source
- Architecture: HIGH - existing patterns (cmd_assemble, hook dispatch protocol) verified in source; FIX patterns match commented TODO locations
- Pitfalls: MEDIUM - LoweringError->Diagnostic conversion and emit_bodies module name are assumptions needing verification at plan time

**Research date:** 2026-03-03
**Valid until:** 2026-04-03 (stable codebase; valid until next phase changes the pipeline)
