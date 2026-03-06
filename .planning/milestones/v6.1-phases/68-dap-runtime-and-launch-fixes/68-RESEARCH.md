# Phase 68: DAP Runtime and Launch Fixes - Research

**Researched:** 2026-03-18
**Domain:** Rust compiler IL emission, DAP server multi-file launch
**Confidence:** HIGH

## Summary

This phase fixes two independent defects in the DAP pipeline. Both fixes are surgical and well-understood — the root causes are fully traced through source code inspection, and all the building blocks needed are already present in the codebase.

**DAP-01 (SWITCH byte-offset bug):** `emit_enum_match` in `patterns.rs` patches SWITCH offsets as instruction-index distances (e.g., offset = target_instr_idx - switch_instr_idx). The runtime's `decode_and_reindex` in `loader.rs` expects those offsets to be byte-position-relative (like all other branch instructions). The fix is to add a SWITCH post-processing pass inside `encode_instructions()` in `serialize.rs`, converting instruction-index-relative offsets to byte-position-relative offsets using the already-computed `instr_byte_starts[]` table. No changes to the runtime needed.

**DAP-02 (multi-file launch):** `compile_and_load()` in `launch.rs` hard-codes single-file mode. Extending it to detect whether `program` points to a `.writ` file, a directory, or a `writ.toml` file and dispatch accordingly is straightforward. The `writ-compiler::config` APIs (`load_config`, `discover_source_files`) and the `run_pipeline` function in `launch.rs` already accept `Vec<(FileId, String, &'static str)>` — only the `compile_and_load` entry point and `DapServer.source_path` field need extending.

**Primary recommendation:** Fix both issues in `writ-dap` and `writ-compiler/src/emit` using the existing helper functions; add test coverage for the SWITCH encoding round-trip and for the multi-file launch path.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Decode error fix (DAP-01)
- Fix at compile time in the emitter, not in the runtime loader
- Root cause: `emit_enum_match` in `writ-compiler/src/emit/body/patterns.rs` patches SWITCH instruction offsets as instruction-index distances (target_pos - switch_idx), but the binary format expects byte-position-relative offsets
- Br/BrTrue/BrFalse go through the `add_fixup()` → `apply_fixups()` pipeline in `serialize.rs` which converts instruction indices to byte offsets — SWITCH bypasses this because it uses inline Vec patching
- Fix: extend `encode_instructions()` in `serialize.rs` to post-process SWITCH instructions, converting their instruction-index offsets to byte-position offsets using `instr_byte_starts[]`
- Alternatively, SWITCH could use the fixup system, but since it has variable-length offsets (one per variant), the current per-instruction fixup model doesn't fit — post-process is simpler

#### Multi-file launch (DAP-02)
- Auto-detect mode from the `program` launch argument: if path ends in `.writ` → single-file mode (existing behavior); if path is a directory or `writ.toml` → project mode
- Project mode reuses `writ_compiler::config::load_config()` and `discover_source_files()` from the `writ build` pipeline
- DAP's `compile_and_load` gets a second entry point (or refactored to accept both modes) that mirrors `cmd_build` in `writ-cli/src/commands/build.rs`
- All discovered source files get unique FileId values, just like `writ build`

#### Multi-file source tracking
- Extend `DapServer.source_path: Option<String>` to `source_paths: Vec<(FileId, String)>` to track all source files
- Stack frame source references use FileId from SourceSpan to look up the correct file path
- Breakpoints are resolved per-file using the source path to FileId mapping

### Claude's Discretion
- Whether to refactor `compile_and_load` into one function with an enum parameter or two separate functions
- Exact error message wording for project-mode failures (missing writ.toml, no source files found)
- Whether to share the `run_pipeline` function between writ-cli and writ-dap (currently duplicated)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DAP-01 | User can run quest_system.writ through DAP without decode errors ("Switch target byte offset not found in offset map") | Root cause fully identified: SWITCH offsets emitted as instruction-index distances, must be byte-position-relative. Fix location: `encode_instructions()` in `serialize.rs`. |
| DAP-02 | User can launch writ.toml multi-file projects through DAP, not just single files | `compile_and_load()` in `launch.rs` needs mode detection + project-mode branch. `load_config`/`discover_source_files` APIs ready. `DapServer.source_path` field needs extension to `source_paths: Vec<(FileId, String)>`. |
</phase_requirements>

---

## Standard Stack

### Core (all internal — no new dependencies)

| Component | File | Purpose | Status |
|-----------|------|---------|--------|
| SWITCH offset emitter | `writ-compiler/src/emit/body/patterns.rs` | Emits GET_TAG + SWITCH for enum match | BUG: emits instr-index offsets |
| Instruction encoder | `writ-compiler/src/emit/serialize.rs` | `encode_instructions()` — encodes + fixup pass | FIX TARGET: add SWITCH post-process |
| Branch fixup model | `writ-compiler/src/emit/body/labels.rs` | `LabelAllocator` / `apply_fixups()` | WORKING: handles Br/BrTrue/BrFalse |
| Runtime loader | `writ-runtime/src/loader.rs` | `decode_and_reindex()` — expects byte-relative offsets | READ-ONLY (verification) |
| DAP launch entry | `writ-dap/src/launch.rs` | `compile_and_load()` + `run_pipeline()` | EXTEND for multi-file |
| DAP server struct | `writ-dap/src/server/mod.rs` | `DapServer` — holds `source_path: Option<String>` | EXTEND to `source_paths` |
| DAP handlers | `writ-dap/src/server/handlers.rs` | `handle_launch()` | EXTEND for project mode detection |
| DAP inspection | `writ-dap/src/server/inspection.rs` | `build_stack_frames()` — uses `source_path` | UPDATE for multi-file lookup |
| Compiler config | `writ-compiler/src/config.rs` | `load_config()` + `discover_source_files()` | READY — reuse as-is |
| CLI build pattern | `writ-cli/src/commands/build.rs` | `cmd_build()` — multi-file reference | READ-ONLY (reference) |

**Installation:** No new Cargo dependencies required.

---

## Architecture Patterns

### Pattern 1: SWITCH Byte-Offset Post-Processing

**What:** After encoding all instructions to bytes in Pass 2 of `encode_instructions()`, add a Pass 4 (or extend Pass 3) that iterates the code buffer looking for SWITCH instructions and patches their offsets from instruction-index-relative to byte-position-relative.

**When to use:** Already decided. The key insight is that `instr_byte_starts[]` is computed in Pass 1 and is available for the entire duration of `encode_instructions()`.

**How it works:** The SWITCH instruction encodes as:
```
u16(opcode=0x0603) | u16(r_tag) | u16(n) | i32[n](offsets)
```
Total size: `6 + 4*n` bytes. Each offset in the encoded binary is relative to the start byte of the SWITCH instruction itself. The emitter currently stores offsets as `target_instr_idx - switch_instr_idx`. The fix converts each to `instr_byte_starts[target_instr_idx] - instr_byte_starts[switch_instr_idx]`.

**Why not use the fixup model:** `LabelAllocator.add_fixup()` records `(branch_instr_byte_pos, label)` and patches one `i32` at `branch_start + 4`. SWITCH has N offsets at `switch_start + 6`, `switch_start + 10`, `switch_start + 14`, etc. The fixup model is designed for single-offset instructions. A post-processing pass on the encoded bytes is simpler.

**Example — what the post-processing pass does:**
```rust
// Source: writ-compiler/src/emit/serialize.rs encode_instructions()
// (pseudocode for the new Pass 4 logic)

// Walk encoded bytes, find SWITCH instructions, patch offsets
let mut byte_pos = 0usize;
for (instr_idx, instr) in instructions.iter().enumerate() {
    if let Instruction::Switch { offsets, .. } = instr {
        let switch_byte_start = instr_byte_starts[instr_idx];
        // SWITCH layout: opcode(2) + r_tag(2) + count(2) + offsets(4*n)
        let offsets_start = switch_byte_start + 6;
        for (slot, &instr_idx_offset) in offsets.iter().enumerate() {
            // instr_idx_offset is currently (target_instr_idx - switch_instr_idx)
            // convert to byte offset: target_byte_start - switch_byte_start
            let target_instr = (instr_idx as i64 + instr_idx_offset as i64) as usize;
            let target_byte = instr_byte_starts[target_instr];
            let byte_offset = (target_byte as i64 - switch_byte_start as i64) as i32;
            let patch_pos = offsets_start + slot * 4;
            code[patch_pos..patch_pos + 4].copy_from_slice(&byte_offset.to_le_bytes());
        }
    }
}
```

**Alternative approach:** Instead of post-processing the byte buffer, the emitter could store absolute target instruction indices in SWITCH (not relative), and `encode_instructions()` could convert them directly to byte offsets during Pass 2 encoding. This would be cleaner but requires changing how `emit_enum_match` computes and stores offsets. The post-processing approach requires no changes to `patterns.rs`.

### Pattern 2: Mode Detection in `compile_and_load`

**What:** Inspect the `program` path string to determine launch mode before entering the compilation pipeline.

**When to use:** Always — the DAP launch handler calls `compile_and_load` (or the new wrapper) with the raw `program` path.

**Detection logic:**
```rust
// Source: to be added in writ-dap/src/launch.rs
enum LaunchTarget {
    SingleFile(PathBuf),   // path ends with .writ
    Project(PathBuf),      // path is a directory or ends with writ.toml
}

fn detect_launch_target(program_path: &str) -> Result<LaunchTarget, String> {
    let path = PathBuf::from(program_path);
    if path.is_dir() || program_path.ends_with("writ.toml") {
        let project_root = if program_path.ends_with("writ.toml") {
            path.parent().unwrap_or(&path).to_path_buf()
        } else {
            path
        };
        Ok(LaunchTarget::Project(project_root))
    } else if program_path.ends_with(".writ") {
        Ok(LaunchTarget::SingleFile(path))
    } else {
        Err(format!("'{}' is not a .writ file or writ.toml directory", program_path))
    }
}
```

**Project mode pipeline (mirrors cmd_build):**
```rust
// Source: based on writ-cli/src/commands/build.rs
fn compile_and_load_project(project_root: &Path) -> Result<(Module, Vec<(FileId, String)>), String> {
    let config = writ_compiler::config::load_config(project_root)
        .map_err(|e| format!("failed to load writ.toml: {}", e))?;

    let discovered = writ_compiler::config::discover_source_files(project_root, &config)
        .map_err(|e| format!("failed to discover source files: {}", e))?;

    if discovered.is_empty() {
        return Err("no .writ source files found".to_string());
    }

    let mut file_sources: Vec<(FileId, String, &'static str)> = Vec::new();
    let mut file_id_paths: Vec<(FileId, String)> = Vec::new();

    for (n, file_path) in discovered.iter().enumerate() {
        let file_id = FileId(n as u32);
        let bytes = std::fs::read(file_path)
            .map_err(|e| format!("failed to read '{}': {}", file_path.display(), e))?;
        let src_owned = decode_utf8_strip_bom(&bytes)
            .map_err(|e| format!("failed to decode '{}': {}", file_path.display(), e))?;
        let src: &'static str = Box::leak(src_owned.into_boxed_str());
        let display_path = file_path.display().to_string();
        file_sources.push((file_id, display_path.clone(), src));
        file_id_paths.push((file_id, display_path));
    }

    let compiled_bytes = run_pipeline(file_sources, true)?;
    let module = Module::from_bytes(&compiled_bytes)
        .map_err(|e| format!("failed to decode compiled module: {:?}", e))?;

    Ok((module, file_id_paths))
}
```

### Pattern 3: Multi-File Source Tracking in DapServer

**What:** Replace `source_path: Option<String>` with `source_paths: Vec<(FileId, String)>` in `DapServer`. For single-file launches, `source_paths` holds one entry. For project mode, it holds all discovered files.

**Impact on `build_stack_frames` in `inspection.rs`:** Currently uses `self.source_path.as_deref().unwrap_or("")` for all frames. After the change, it needs to look up the FileId from the SourceSpan to get the per-frame source path.

**Important caveat:** The current `Module` / `SourceSpan` structure does NOT carry a `file_id` field on individual spans — `SourceSpan` has `{ pc: u32, line: u32, column: u16 }`. This means the DAP server cannot currently distinguish which file a span belongs to without additional metadata. For the purposes of DAP-02 (success criterion: "multi-file project can be launched"), the minimum viable change is:
- `source_paths` holds all files.
- Stack frames still use the first/primary file for source references (acceptable for Phase 68 scope — multi-file source attribution is a future improvement).
- Breakpoints are resolved per file using the `source_path` → `FileId` mapping when the client sends `setBreakpoints`.

**SourceSpan FileId gap:** The IL spec supports per-span file tracking (there may be a file_id field in the spec), but the current writ-module `SourceSpan` struct does not encode one. Investigate whether this is needed for Phase 68 or can be deferred.

### Recommended Project Structure (no changes needed)

The existing crate layout is correct. Changes are contained to:
```
writ-compiler/
└── src/emit/serialize.rs         # Pass 4: SWITCH byte-offset patching

writ-dap/
├── src/launch.rs                 # compile_and_load + project mode
├── src/server/mod.rs             # DapServer struct: source_path -> source_paths
├── src/server/handlers.rs        # handle_launch: mode detection + source_paths setup
└── src/server/inspection.rs      # build_stack_frames: use source_paths lookup
```

### Anti-Patterns to Avoid

- **Fixing in the runtime loader:** The CONTEXT.md decision is to fix in the emitter. Do not change `decode_and_reindex` in `writ-runtime/src/loader.rs` — it is correct for byte-relative offsets and should remain so.
- **Duplicating `run_pipeline`:** `writ-dap/src/launch.rs` already has its own `run_pipeline`. Extending it to serve both single-file and multi-file callers is preferable to adding a second copy. Whether to extract the shared implementation to `writ-compiler` or keep it in `writ-dap` is Claude's discretion.
- **Changing the fixup model for SWITCH:** The existing `LabelAllocator` handles single-slot fixups. Do not extend it to handle multi-slot (SWITCH) fixups — post-processing is simpler and more localized.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| writ.toml parsing | Custom TOML parser | `writ_compiler::config::load_config()` | Already implemented + tested |
| .writ file discovery | Custom directory walk | `writ_compiler::config::discover_source_files()` | Handles walkdir, filters .writ, sorts |
| Byte-start computation | Duplicate logic | `compute_instr_byte_starts()` in `serialize.rs` (already exists) | Available in same file; or just use `instr_byte_starts` already computed in Pass 1 |
| BOM stripping | Custom UTF-8 decode | `decode_utf8_strip_bom()` in `launch.rs` (already exists) | Same file, already handles BOM |

---

## Common Pitfalls

### Pitfall 1: SWITCH Offset Sign and Directionality

**What goes wrong:** The SWITCH offsets in the emitter (`target_pos - switch_idx`) are instruction-index distances. When converting to byte distances, both the "numerator" (target byte start) and the "denominator" (switch byte start) must use the same basis — the byte start of the SWITCH instruction, not byte 0.

**Why it happens:** It's easy to accidentally compute `instr_byte_starts[target] - 0` (absolute byte offset) instead of `instr_byte_starts[target] - instr_byte_starts[switch_idx]` (relative).

**How to avoid:** The runtime's `decode_and_reindex` confirms the expected convention:
```rust
// From loader.rs — what the runtime expects:
let target_byte = (current_byte_offset as i64 + *off as i64) as u32;
// So: target_byte = switch_start_byte + off
// Therefore: off = target_byte - switch_start_byte  (NOT absolute)
```

**Warning signs:** If the test produces "Switch target byte offset X not found in offset map" with X being larger than the total method byte size, you have an absolute-vs-relative confusion.

### Pitfall 2: instr_byte_starts Sentinel Index

**What goes wrong:** `compute_instr_byte_starts()` pushes a sentinel entry at `instructions.len()` (the total byte size). When converting a SWITCH offset that points to the instruction after the last one (a forward jump past the end), you'd be indexing `instr_byte_starts[instructions.len()]` which is the sentinel — correct behavior. But if `target_instr_idx` is out of range, it panics.

**How to avoid:** In `emit_enum_match`, `mark_label_here(arm_labels[i])` is called after the SWITCH and arm body emission. Labels point to valid instruction indices. No out-of-range access should occur for well-formed enum matches.

### Pitfall 3: source_paths FileId Alignment

**What goes wrong:** In project mode, FileId values are assigned sequentially starting at 0 during file discovery. If `compile_and_load_project` assigns FileId(n) to `discovered[n]`, the `source_paths` Vec must align: `source_paths[n] = (FileId(n), path_n)`. Misalignment causes wrong source files to be shown in stack frames.

**How to avoid:** Use the same enumeration loop for both `file_sources` (compilation) and `source_paths` (tracking), assigning `FileId(n as u32)` from the same `enumerate()` call.

### Pitfall 4: BreakpointTable Initialization with Multi-File Module

**What goes wrong:** `BreakpointTable::new(&module)` scans all SourceSpans in all method bodies. In multi-file mode, if spans from different files use the same line numbers (which they will), breakpoints might be ambiguous or resolved to the wrong file.

**How to avoid:** For Phase 68's success criteria (just launch and compile), this is acceptable. The test does not require per-file breakpoint precision. `BreakpointTable` can be initialized as-is — multi-file breakpoint disambiguation is a future enhancement.

### Pitfall 5: Stack Frame Source Path in Multi-File Mode

**What goes wrong:** `build_stack_frames` currently sets the same `source_path` for every frame. In multi-file mode, functions defined in different files will show the wrong source path.

**How to avoid:** Since `SourceSpan` does not carry a `file_id` field in the current module format, true per-frame source attribution requires either (a) adding file_id to SourceSpan (spec change, out of scope) or (b) heuristics. For Phase 68, using `source_paths[0]` as the fallback for all frames is acceptable. The success criterion is "can be launched" not "shows correct per-file source in debugger".

---

## Code Examples

### Current SWITCH Offset Emission (the Bug Site)

```rust
// Source: writ-compiler/src/emit/body/patterns.rs lines 140-148
// This stores instruction-index-relative offsets in the Switch instruction:
{
    let mut patched_offsets: Vec<i32> = Vec::with_capacity(n_variants);
    for label in &arm_labels {
        let target_pos = emitter.labels.resolve(*label).unwrap_or(0);
        // BUG: target_pos and switch_idx are instruction indices, NOT byte positions
        patched_offsets.push((target_pos as i64 - switch_idx as i64) as i32);
    }
    if let Instruction::Switch { offsets, .. } = &mut emitter.instructions[switch_idx] {
        *offsets = patched_offsets;
    }
}
```

### encode_instructions (Fix Target)

```rust
// Source: writ-compiler/src/emit/serialize.rs lines 428-462
// Pass 1 already computes instr_byte_starts.
// Pass 3 applies Br/BrTrue/BrFalse fixups.
// Add Pass 4 after Pass 3 to fix SWITCH offsets:

// Pass 4: convert SWITCH instruction-index offsets to byte-position offsets
for (instr_idx, instr) in instructions.iter().enumerate() {
    if let Instruction::Switch { offsets, .. } = instr {
        let switch_byte_start = instr_byte_starts[instr_idx];
        // SWITCH layout: opcode(2) + r_tag(2) + count(2) + offsets(4 each)
        let offsets_patch_start = switch_byte_start + 6;
        for (slot_idx, &instr_offset) in offsets.iter().enumerate() {
            // instr_offset = target_instr_idx - switch_instr_idx  (instruction distance)
            let target_instr_idx = (instr_idx as i64 + instr_offset as i64) as usize;
            let target_byte_start = instr_byte_starts
                .get(target_instr_idx)
                .copied()
                .unwrap_or(code.len());
            let byte_offset = (target_byte_start as i64 - switch_byte_start as i64) as i32;
            let patch_pos = offsets_patch_start + slot_idx * 4;
            code[patch_pos..patch_pos + 4].copy_from_slice(&byte_offset.to_le_bytes());
        }
    }
}
```

### Runtime Expectation (Confirmation)

```rust
// Source: writ-runtime/src/loader.rs lines 136-149
// The runtime confirms: offsets are relative to current_byte_offset (the SWITCH instruction's byte start)
Instruction::Switch { offsets, .. } => {
    for off in offsets.iter_mut() {
        let target_byte = (current_byte_offset as i64 + *off as i64) as u32;
        let target_idx = offset_map.get(&target_byte).ok_or_else(|| {
            // This is the error from the bug report:
            // "Switch target byte offset X not found in offset map"
        })?;
        *off = *target_idx as i32;
    }
}
```

### SWITCH Binary Layout (for byte-patching arithmetic)

```
// Source: writ-module/src/instruction.rs lines 608-614
Instruction::Switch { r_tag, offsets } => {
    w.write_u16::<LittleEndian>(*r_tag)?;       // bytes 2-3
    w.write_u16::<LittleEndian>(offsets.len())?; // bytes 4-5 (count)
    for off in offsets {
        w.write_i32::<LittleEndian>(*off)?;      // bytes 6, 10, 14, ... (each 4 bytes)
    }
}
// Full layout: opcode(2) + r_tag(2) + count(2) + offsets(4 * n)
// Patch position for slot i: switch_byte_start + 6 + i * 4
```

### compile_and_load Existing Pattern (Single-File)

```rust
// Source: writ-dap/src/launch.rs lines 18-39
pub fn compile_and_load(program_path: &str) -> Result<(Module, &'static str), String> {
    let bytes = std::fs::read(program_path)
        .map_err(|e| format!("failed to read '{}': {}", program_path, e))?;
    let src_owned = decode_utf8_strip_bom(&bytes)
        .map_err(|e| format!("failed to decode '{}': {}", program_path, e))?;
    let src: &'static str = Box::leak(src_owned.into_boxed_str());
    let file_id = writ_diagnostics::FileId(0);
    let display_path = program_path.to_string();
    let compiled_bytes = run_pipeline(vec![(file_id, display_path, src)], true)?;
    let module = Module::from_bytes(&compiled_bytes)
        .map_err(|e| format!("failed to decode compiled module: {:?}", e))?;
    Ok((module, src))
}
```

### DapServer Struct (Before / After)

```rust
// BEFORE (writ-dap/src/server/mod.rs):
pub struct DapServer<I: Read, O: Write> {
    pub(super) source_path: Option<String>,
    // ...
}

// AFTER:
pub struct DapServer<I: Read, O: Write> {
    /// Maps FileId -> source file path for all files in the launched program.
    /// Single-file: one entry. Project mode: one entry per discovered .writ file.
    pub(super) source_paths: Vec<(writ_diagnostics::FileId, String)>,
    // ...
}
```

### handle_launch Source Path Setup (Before / After)

```rust
// BEFORE (writ-dap/src/server/handlers.rs line 192):
self.source_path = Some(program_path);

// AFTER (single-file mode):
self.source_paths = vec![(writ_diagnostics::FileId(0), program_path)];

// AFTER (project mode — compile_and_load_project returns file_id_paths):
self.source_paths = file_id_paths;
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No SWITCH fix | SWITCH offsets wrong (bug) | Pre-Phase 68 | Runtime decode error on enum match |
| Single-file only DAP launch | Single-file only (current) | Pre-Phase 68 | Cannot launch multi-file projects |
| Br/BrTrue/BrFalse via fixup | Br/BrTrue/BrFalse byte-correct | Pre-Phase 68 | Already working; SWITCH is the outlier |

**Not deprecated/outdated:** The fixup model in `labels.rs` is correct and working for all non-SWITCH branch instructions. Do not change it.

---

## Open Questions

1. **SourceSpan lacks FileId**
   - What we know: `writ_module::module::SourceSpan` has `{ pc: u32, line: u32, column: u16 }` — no `file_id` field.
   - What's unclear: Does the IL spec mandate a file_id in SourceSpan? If so, can it be added cheaply?
   - Recommendation: For Phase 68, defer per-frame source-file attribution. Use `source_paths[0]` as fallback for all stack frames. True multi-file source attribution is a future enhancement.

2. **run_pipeline deduplication (Claude's discretion)**
   - What we know: `writ-dap/src/launch.rs` has a private `run_pipeline` that is functionally identical to `writ-cli/src/pipeline.rs::run_pipeline`. They differ only in minor details (error prefix, no module_name param in DAP).
   - What's unclear: Is now the right time to unify them?
   - Recommendation: Keep them separate for Phase 68. Unifying into a `writ-compiler` helper would require a new public API surface and touches more crates than the phase warrants. Mark as future cleanup.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test (`cargo test`) |
| Config file | `Cargo.toml` workspace |
| Quick run command | `cargo test -p writ-dap 2>&1` |
| Full suite command | `cargo test --workspace 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DAP-01 | quest_system.writ compiles and loads without decode error | integration | `cargo test -p writ-dap test_quest_system_compiles -- --nocapture` | YES (`test_quest_system_debug.rs`) |
| DAP-01 | SWITCH instruction encodes byte-relative offsets correctly | unit | `cargo test -p writ-compiler encode_switch` | NO — Wave 0 gap |
| DAP-01 | Full DAP session with quest_system.writ runs to completion | integration | `cargo test -p writ-dap test_quest_system_full_debug_session -- --nocapture` | YES (`test_quest_system_debug.rs`) |
| DAP-02 | Multi-file project can be launched via DAP | integration | `cargo test -p writ-dap test_multi_file_project_launch` | NO — Wave 0 gap |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-dap 2>&1`
- **Per wave merge:** `cargo test --workspace 2>&1`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] Unit test for SWITCH byte-offset encoding in `writ-compiler` (covers DAP-01 at the emitter level)
  - Suggestion: add to `writ-module/tests/instruction_tests.rs` or a new `writ-compiler` test file
  - Test should: emit an enum match with 3 variants, serialize, round-trip through `decode_and_reindex`, verify no decode error
- [ ] Integration test for multi-file DAP launch (covers DAP-02)
  - Suggestion: add to `writ-dap/tests/test_compile_and_load.rs` or new test file
  - Test should: create a temp writ.toml project, call `compile_and_load` with directory path, verify module has all expected methods

---

## Sources

### Primary (HIGH confidence)
- Direct source code inspection — all findings verified against actual code in the repository

### Key Files Examined

| File | Lines Read | Key Finding |
|------|-----------|-------------|
| `writ-compiler/src/emit/body/patterns.rs` | Full | Bug confirmed: line 144 stores `target_pos - switch_idx` (instr-index distance) |
| `writ-compiler/src/emit/serialize.rs` | Full | `encode_instructions()` Pass 1 already computes `instr_byte_starts`; Pass 3 does `apply_fixups`; Pass 4 (SWITCH fix) is missing |
| `writ-compiler/src/emit/body/labels.rs` | Full | `apply_fixups` patches byte position `branch_start + 4`; single-slot only |
| `writ-runtime/src/loader.rs` | Full | `decode_and_reindex` expects byte-relative offsets for SWITCH; confirms the fix direction |
| `writ-dap/src/launch.rs` | Full | `compile_and_load()` single-file only; `run_pipeline()` already accepts `Vec<(FileId, String, &'static str)>` |
| `writ-dap/src/server/mod.rs` | Full | `source_path: Option<String>` — single field to replace |
| `writ-dap/src/server/handlers.rs` | Full | `handle_launch()` sets `self.source_path = Some(program_path)` |
| `writ-dap/src/server/inspection.rs` | Full | `build_stack_frames()` uses `self.source_path.as_deref()` — needs multi-file lookup |
| `writ-compiler/src/config.rs` | Full | `load_config()` + `discover_source_files()` — ready to reuse |
| `writ-cli/src/commands/build.rs` | Full | `cmd_build()` — reference pattern for multi-file compilation |
| `writ-module/src/instruction.rs` | Grep | SWITCH binary layout: `opcode(2) + r_tag(2) + count(2) + offsets(4*n)` |
| `writ-golden/tests/golden/quest_system.writ` | Full | Has 6+ enum match expressions (QuestStatus/QuestType) — exercises SWITCH emission |

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all components are in-repo, code read directly
- Architecture: HIGH — root cause fully traced, fix approach confirmed against runtime expectations
- Pitfalls: HIGH — derived from reading both the emitter and the runtime decode logic

**Research date:** 2026-03-18
**Valid until:** Stable (internal codebase — no external dependency staleness)
