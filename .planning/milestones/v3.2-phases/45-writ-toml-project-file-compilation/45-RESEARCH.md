# Phase 45: writ.toml Project File Compilation - Research

**Researched:** 2026-03-06
**Domain:** Rust CLI (clap 4.5), multi-file compiler pipeline, TOML profile system
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- `writ build` is the dedicated project-mode subcommand — `writ compile .` does NOT auto-detect directories
- `writ compile foo.writ` continues to work as before — single-file mode is unaffected
- `writ build` accepts an optional path argument: `writ build [path]` — defaults to `.` (cwd), can point at any directory with a writ.toml
- When no writ.toml is found: error with hint — "writ.toml not found. Run `writ new <name>` to create a project."
- The `writ new` scaffold "Next steps" message updated to say `writ build` instead of `writ compile sources/main.writ`
- Full `[profile.debug]` and `[profile.release]` sections in writ.toml — spec amendment needed
- Debug profile is the default when neither --release nor --debug is passed
- Release strips debug info (DebugLocal entries); debug includes them — this is the only concrete difference for now
- Profile sections exist in toml for future extensibility (optimization, strip, etc.) but only `debug_info` has effect in this phase
- The `debug` condition flag is NOT automatically set by profile — conditions and profiles are separate systems
- Output path: `{output_base}/{profile}/{name}.writc` — e.g., `build/debug/my-game.writc`
- Default `output_base` is `build/`; `compiler.output` in writ.toml overrides the base path
- Output directories auto-created silently if they don't exist
- Scaffold .gitignore updated to include `/build/` (in addition to existing `*.writc` glob)
- `--name` flag on `writ build` overrides the module name
- Fallback chain: `--name` flag → `project.name` from writ.toml
- All discovered `.writ` files are compiled into ONE module — all top-level declarations share the same namespace
- Output verbosity: list each discovered file path, then the output path (not just a summary count)

### Claude's Discretion

- How to merge multiple ASTs through the pipeline (concatenation strategy)
- FileId assignment for multi-file compilation (how to map errors back to source files)
- Profile toml section field names and defaults
- Exact error message formatting and ariadne integration for multi-file diagnostics

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TOOL-01 | User can compile a Writ project by running `writ compile .` or `writ build` in a directory containing `writ.toml` — all `.writ` files from the configured sources directory are compiled into one module | Research confirms the pipeline already accepts multi-file slices; `writ build` subcommand + `discover_source_files()` is the integration path |
| TOOL-02 | `--release` and `--debug` flags are supported on `writ compile`/`writ build` — `[profile.release]` and `[profile.debug]` in `writ.toml` are respected | Research identifies the `DebugLocal` emission gate in `serialize.rs`; new `ProfileConfig` struct slots into existing `WritConfig` |
</phase_requirements>

---

## Summary

Phase 45 adds a `writ build` subcommand that compiles a multi-file Writ project using `writ.toml` for configuration. The existing codebase is exceptionally well-prepared for this change: `config.rs` already has `load_config()` and `discover_source_files()`, and the entire pipeline from `resolve()` through `emit_bodies()` already accepts `&[(FileId, &Ast)]` slices — meaning multi-file support is already wired into stages 3, 4, and 5. The only stage that processes one file at a time is stage 2 (`lower()`), which takes a single file's CST items. The strategy is to call `lower()` per file and concatenate the resulting `Ast` items before feeding the merged slice to the downstream stages.

The profile system is additive: add a `ProfileConfig` struct with a `debug_info: bool` field (defaulting to `true` for debug, `false` for release), extend `WritConfig` with `profile` subsections, and pass a boolean flag into `emit_bodies` (or a thin wrapper) to control whether `DebugLocal` entries are emitted in the serializer. The only behaviorally observable difference between profiles in this phase is presence/absence of `DebugLocal` rows in method bodies.

The clap integration requires adding one `Build` variant to the `Commands` enum with `path: Option<String>`, `--release`, `--debug`, and `--name` flags. The existing `cmd_compile` thread-spawning pattern (16 MB stack for deep AST recursion) must be replicated for `cmd_build`.

**Primary recommendation:** Refactor the 5-stage pipeline out of `cmd_compile` into a shared `run_pipeline(sources: &[(FileId, &'static str, &'static str)], module_name: &str, emit_debug_info: bool) -> Result<Vec<u8>, String>` function called by both `cmd_compile` and `cmd_build`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.5 (already in writ-cli) | CLI parsing — `Build` subcommand variant | Already the project CLI framework; derive pattern used throughout |
| serde + toml | 1 + 0.9 (already in writ-compiler) | `ProfileConfig` deserialization | Already used for `WritConfig`; just add a new struct |
| walkdir | 2 (already in writ-compiler) | Source file discovery | Already used in `discover_source_files()` |
| std::fs::create_dir_all | std | Auto-create output directories | No extra dependency needed |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| writ_diagnostics::FileId | workspace | Per-file identity for error attribution | Used throughout pipeline already; assign sequential `FileId(n)` per discovered file |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Sequential FileId assignment | Path-hash FileIds | Sequential is simpler and sufficient; hashing adds complexity with no benefit since FileIds are only compared by equality |
| Flag on `emit_bodies` | Separate `emit_bodies_release` function | Flag approach is lower surface area and avoids code duplication |

**Installation:** No new dependencies — all needed libraries are already present.

---

## Architecture Patterns

### Recommended Project Structure

No new files needed. All changes are confined to:

```
writ-cli/src/
└── main.rs           # Add Build variant to Commands, add cmd_build(), refactor pipeline helper

writ-compiler/src/
└── config.rs         # Add ProfileConfig, extend WritConfig with profiles field

writ-compiler/src/emit/
└── mod.rs            # Add debug_info parameter to emit_bodies (or thin wrapper)
└── serialize.rs      # Gate DebugLocal emission on debug_info flag
```

### Pattern 1: Multi-File Pipeline — Lower Per File, Merge ASTs

**What:** Call `lower(cst_items)` once per `.writ` file, collect all resulting `Ast` structs, then concatenate their `items` vectors into a single merged `Ast`. Pass the merged AST (with all files' FileIds) to `resolve()`, `typecheck()`, and `emit_bodies()`.

**When to use:** Always for multi-file project builds.

**The lower() function signature (confirmed from source):**
```rust
// writ-compiler/src/lower/mod.rs
pub fn lower(items: Vec<Spanned<Item<'_>>>) -> (Ast, Vec<LoweringError>)
```

`lower()` takes the CST item list from a single file and returns an `Ast { items: Vec<AstDecl> }`. The merge is straightforward concatenation:

```rust
// Pseudocode for cmd_build multi-file lowering
let mut merged_decls: Vec<AstDecl> = Vec::new();
let mut all_file_asts: Vec<(FileId, Ast)> = Vec::new();

for (file_id, path, src) in &file_sources {
    let (cst, parse_errs) = writ_parser::parse(src);
    // ... handle errors
    let cst = cst.unwrap();
    let (ast, lower_errs) = writ_compiler::lower(cst);
    // ... handle errors
    merged_decls.extend(ast.items.clone()); // merge into global decl list
    all_file_asts.push((*file_id, ast));    // keep individual Asts for pipeline
}
```

**Critical insight:** The downstream APIs already accept `&[(FileId, &Ast)]`. The individual `Ast` objects (one per file) should be kept for the pipeline, NOT a single merged `Ast`. Looking at `resolve()`, `typecheck()`, and `emit_bodies()` signatures:

```rust
// All three stages already accept the multi-file slice form:
pub fn resolve(asts: &[(FileId, &Ast)], file_paths: &[(FileId, &str)]) -> ...
pub fn typecheck(resolved: NameResolvedAst, asts: &[(FileId, &Ast)]) -> ...
pub fn emit_bodies(typed_ast: &TypedAst, interner: &TyInterner, asts: &[(FileId, &Ast)]) -> ...
```

So the strategy is: parse and lower each file independently, accumulate `Vec<(FileId, Ast)>`, build a reference slice `&[(FileId, &Ast)]` from it, and pass that directly to resolve/typecheck/emit. No manual AST merging is needed for stages 3-5. For stages 3-5, the `NameResolvedAst` collects declarations from all files in a single `DefMap` (the collector already walks all `asts`).

**FileId assignment for multi-file:** Assign sequential `FileId(n)` to each discovered file in sorted order. This ensures deterministic builds. The diagnostic rendering system uses `FileId` to look up source text — the `sources` slice for `render_diagnostics` must include all `(FileId, &str path, &str src)` tuples.

```rust
// FileId assignment pattern
for (n, path) in discovered_files.iter().enumerate() {
    let file_id = writ_diagnostics::FileId(n as u32);
    // ...
}
```

### Pattern 2: Profile System in WritConfig

**What:** Add `ProfileConfig` with `debug_info: bool`, add `[profile.debug]` and `[profile.release]` optional sections to `WritConfig`.

**TOML shape (decided: Claude's discretion on field names):**

```toml
[profile.debug]
debug_info = true

[profile.release]
debug_info = false
```

**Rust struct:**
```rust
// writ-compiler/src/config.rs — addition
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileConfig {
    /// Whether to emit DebugLocal entries. Default: true for debug, false for release.
    #[serde(default = "default_debug_info")]
    pub debug_info: bool,
}

fn default_debug_info() -> bool { true }

#[derive(Debug, Clone, Deserialize)]
pub struct ProfilesConfig {
    /// [profile.debug]
    #[serde(default = "default_debug_profile")]
    pub debug: ProfileConfig,
    /// [profile.release]
    #[serde(default = "default_release_profile")]
    pub release: ProfileConfig,
}

fn default_debug_profile() -> ProfileConfig { ProfileConfig { debug_info: true } }
fn default_release_profile() -> ProfileConfig { ProfileConfig { debug_info: false } }
```

Add to `WritConfig`:
```rust
pub struct WritConfig {
    pub project: ProjectConfig,
    pub locale: Option<LocaleConfig>,
    #[serde(default)]
    pub compiler: CompilerConfig,
    #[serde(default)]
    pub conditions: HashMap<String, bool>,
    // NEW:
    #[serde(default)]
    pub profile: ProfilesConfig,  // [profile.debug] + [profile.release]
}
```

**Key insight:** TOML nested table `[profile.debug]` maps to `WritConfig.profile.debug` when `ProfilesConfig` is the type of the `profile` field. Serde handles this automatically.

### Pattern 3: DebugLocal Emission Gate

**What:** The `emit_bodies()` function currently always emits `DebugLocal` entries. For release builds, they must be omitted.

**Where DebugLocal is produced:** In `writ-compiler/src/emit/body/mod.rs`, `EmittedBody` carries `debug_locals: Vec<(u16, String, u32, u32)>`. The serializer in `serialize.rs` writes these into the binary.

**Implementation options:**

Option A — Pass `emit_debug_info: bool` through to `emit_bodies`:
```rust
pub fn emit_bodies(
    typed_ast: &TypedAst,
    interner: &TyInterner,
    asts: &[(FileId, &Ast)],
    emit_debug_info: bool,  // NEW
) -> Result<Vec<u8>, Vec<Diagnostic>>
```
Then in the serializer, gate the `DebugLocal` rows:
```rust
// In serialize.rs, when building MethodBody
let debug_locals = if emit_debug_info {
    emit_debug_locals(&emitter, total_code_size)
} else {
    Vec::new()
};
```

Option B — Clear `debug_locals` in each `EmittedBody` after emission, before serialization.

**Recommendation:** Option A — pass the flag through. It's explicit and doesn't mutate data structures post-emission. The `emit_bodies()` API change is a minor additive change; existing callers (tests, `cmd_compile`) pass `true`.

**Where `DebugLocal` is written in serialize.rs:** The `translate()` function in `serialize.rs` reads `EmittedBody.debug_locals` when constructing `writ_module::module::MethodBody`. The gate belongs there.

### Pattern 4: clap Build Subcommand

**What:** Add `Build` variant to `Commands` enum following the existing clap derive pattern.

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing variants ...

    /// Compile all .writ sources in a Writ project directory
    Build {
        /// Project directory containing writ.toml (default: current directory)
        #[arg(default_value = ".")]
        path: String,

        /// Compile with release profile (strips debug info)
        #[arg(long, conflicts_with = "debug")]
        release: bool,

        /// Compile with debug profile (default; includes debug info)
        #[arg(long, conflicts_with = "release")]
        debug: bool,

        /// Override the output module name (default: project.name from writ.toml)
        #[arg(long)]
        name: Option<String>,
    },
}
```

**`conflicts_with`:** clap 4.x supports mutual exclusion via `conflicts_with` — this ensures `--release` and `--debug` cannot both be specified. [HIGH confidence — verified from clap 4.x derive docs]

### Pattern 5: Output Directory Auto-Creation

**What:** Output path `{output_base}/{profile}/{name}.writc` — create directories silently if they don't exist.

```rust
let out_dir = std::path::Path::new(&output_base).join(&profile_name);
std::fs::create_dir_all(&out_dir)
    .map_err(|e| format!("failed to create output directory '{}': {}", out_dir.display(), e))?;
let out_path = out_dir.join(format!("{}.writc", module_name));
```

### Pattern 6: cmd_compile Refactoring

**What:** Extract the 5-stage pipeline from `cmd_compile` into a shared function that both `cmd_compile` and `cmd_build` call.

The refactored shared pipeline helper:
```rust
fn run_pipeline(
    file_sources: Vec<(writ_diagnostics::FileId, String, &'static str)>,
    // (file_id, display_path, leaked_src)
    module_name: &str,
    emit_debug_info: bool,
) -> Result<Vec<u8>, String>
```

Both `cmd_compile` (single file, FileId(0)) and `cmd_build` (N files, FileId(0..N-1)) call this. The thread spawning pattern (16 MB stack) must wrap the entire call — the deep AST recursion risk applies equally to multi-file builds.

### Anti-Patterns to Avoid

- **Merging all file items into one mega-Ast before passing to the pipeline:** The pipeline APIs already accept slices of per-file Asts. One merged Ast loses per-file FileId attribution for error diagnostics.
- **Running `lower()` on an empty item list:** `discover_source_files()` returns an empty Vec if the sources directory doesn't exist (already handles this gracefully — just warns or errors before reaching lower).
- **Using `cmd_compile` to detect directories:** The CONTEXT.md locks this — `writ compile .` does NOT detect directories. Only `writ build` does project mode.
- **Hardcoding "module" as the module name in `emit_bodies`:** Currently `emit_bodies` calls `builder.set_module_def("module", "0.1.0", 0)` — for project builds, this must use `project.name` from writ.toml (or `--name` override).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Recursive `.writ` file discovery | Custom directory walker | `discover_source_files()` in `config.rs` | Already implemented with `walkdir`, sorted output, link-following |
| TOML profile deserialization | Manual string parsing | serde + toml (already in `writ-compiler/Cargo.toml`) | Handles nested tables, defaults, missing sections cleanly |
| Mutual-exclusive CLI flags | Manual flag validation in `cmd_build` | `conflicts_with` in clap derive | clap generates the error message automatically |
| Output directory creation | Manual `mkdir` chain | `std::fs::create_dir_all` | Handles nested paths, idempotent, cross-platform |

**Key insight:** The existing codebase has invested heavily in making multi-file compilation a first-class use case — `resolve()`, `typecheck()`, and `emit_bodies()` all already accept `&[(FileId, &Ast)]`. The planner should recognize this as a "glue phase" not an "infrastructure phase."

---

## Common Pitfalls

### Pitfall 1: 'static Lifetime for Source Strings

**What goes wrong:** `writ_parser::parse()` requires the source string to have `'static` lifetime. In `cmd_compile`, this is handled by `Box::leak(src_owned.into_boxed_str())`. With multiple files, each file's source string must be leaked separately.

**Why it happens:** The parser produces `Rich<'static, Token<'src>, Span>` error types that embed the source string. The `'static` constraint propagates upward.

**How to avoid:** `Box::leak()` per file, stored in the `file_sources` Vec as `&'static str`. Since the binary process exits after compilation, the leak is acceptable (no long-running server).

**Warning signs:** Lifetime compile errors about `'src` not outliving the pipeline stages.

### Pitfall 2: module_name Hardcoded in emit_bodies

**What goes wrong:** `emit_bodies()` currently calls `builder.set_module_def("module", "0.1.0", 0)` with a hardcoded name. For project builds, the module name must come from `project.name` in writ.toml.

**Why it happens:** The single-file `cmd_compile` had no config context.

**How to avoid:** The `emit_bodies()` function signature change (adding `emit_debug_info`) is an opportunity to also pass the module name, OR emit_bodies can keep its current behavior and the builder's `set_module_def` can be called externally before `emit_bodies` (which currently calls it again, overwriting). The cleanest fix: pass `module_name: &str` into the pipeline helper function, which sets it on the builder before `collect_defs` runs. Alternatively, `collect_defs` in `collect.rs` already calls `set_module_def` using `find_module_name()` which derives the name from the DefMap namespace — but for projects with no namespaces, it falls back to `"main"`. Projects should use `project.name`, not the namespace heuristic.

**Recommendation:** Add a `module_name: Option<&str>` parameter to `emit_bodies` or its internal `collect_defs` call. If `Some`, use it; if `None`, fall through to the existing namespace heuristic.

### Pitfall 3: current_file in typecheck CheckCtx

**What goes wrong:** `typecheck()` in `check/mod.rs` initializes `CheckCtx.current_file` with the first file's FileId (`asts.first().map(...).unwrap_or(FileId(0))`). For multi-file builds, diagnostics from later files may be attributed to the first file.

**Why it happens:** `current_file` is set once for the entire typecheck pass but should update per-declaration.

**How to avoid:** Look at `check_decl.rs` — `check_fn_decl` already reads `entry.file_id` from the DefMap entry, so individual diagnostics have the right FileId. The `current_file` field in `CheckCtx` is likely overridden per-declaration during check. Verify `check_decl.rs` sets `ctx.current_file = file_id` before checking each declaration — if it does, multi-file is safe. If not, that assignment needs to be added.

**Warning signs:** Error messages pointing to the wrong file in multi-file projects.

### Pitfall 4: discover_source_files Default Mismatch

**What goes wrong:** `CompilerConfig::default()` uses `vec!["src/".to_string()]`, but the `writ new` scaffold creates a `sources/` directory (not `src/`). The scaffold's `writ.toml` explicitly sets `sources = ["sources/"]`, so freshly scaffolded projects work. But if a user omits the `[compiler]` section from their `writ.toml`, `writ build` will look in `src/` and find nothing.

**Why it happens:** The default in `config.rs` and the scaffold default differ. This was a known issue addressed in Phase 40 (SPEC-03 alignment), but the `default_sources()` function still returns `["src/"]`.

**How to avoid:** This is an existing design tension. For Phase 45, document that users who omit `[compiler]` get the `src/` default. The planner may want to evaluate whether to change the default to `sources/` — but changing it would be a breaking change for any users who have `src/` directories. This is a discretion item.

### Pitfall 5: Empty Project (No .writ Files Found)

**What goes wrong:** `discover_source_files()` returns an empty Vec if the sources directory is empty or doesn't exist. Running the pipeline with zero files produces a module with no methods and no exports — this is valid but potentially confusing.

**Why it happens:** The function silently skips non-existent directories.

**How to avoid:** `cmd_build` should check if the discovered file list is empty and emit a warning or error: "no .writ source files found in [sources dirs]". This is a UX concern, not a correctness issue.

### Pitfall 6: .gitignore Already Has /build/

**What goes wrong:** The existing `cmd_new()` scaffold already generates `.gitignore` with `/build/` and `/dist/` entries (verified from source). The CONTEXT.md says to update `.gitignore` to include `/build/`, but it's already there.

**Why it happens:** The gitignore was written with future build system anticipation.

**How to avoid:** No change needed for the `.gitignore` content. The planner should not include a gitignore update task — just verify it's correct as-is.

---

## Code Examples

Verified patterns from direct source inspection:

### Existing Pipeline — Single File (reference for refactoring)

```rust
// writ-cli/src/main.rs — cmd_compile (current)
let handle = std::thread::Builder::new()
    .stack_size(16 * 1024 * 1024)
    .spawn(move || -> Result<(), String> {
        let bytes = std::fs::read(&input)
            .map_err(|e| format!("failed to read '{}': {}", input, e))?;
        let src_owned = strip_bom_and_decode(&bytes)
            .map_err(|e| format!("failed to decode '{}': {}", input, e))?;
        let src: &'static str = Box::leak(src_owned.into_boxed_str());
        let file_id = writ_diagnostics::FileId(0);
        let sources = [(file_id, input.as_str(), src)];
        // Stage 1: Parse
        let (cst_opt, parse_errs) = writ_parser::parse(src);
        // ... stages 2-5
    })
    .map_err(|e| format!("failed to spawn compile thread: {e}"))?;
handle.join().unwrap_or_else(|_| Err("compilation panicked".to_string()))
```

### Multi-File Lower + Merge

```rust
// Pattern for cmd_build: lower per file, accumulate per-file Asts for pipeline
let mut per_file_asts: Vec<(FileId, Ast)> = Vec::new();
let mut all_source_refs: Vec<(FileId, String, &'static str)> = Vec::new();

for (n, path) in discovered_files.iter().enumerate() {
    let file_id = FileId(n as u32);
    let bytes = std::fs::read(path).map_err(|e| format!(...))?;
    let src_owned = strip_bom_and_decode(&bytes).map_err(|e| format!(...))?;
    let src: &'static str = Box::leak(src_owned.into_boxed_str());
    let display_path = path.display().to_string();
    eprintln!("  {}", display_path);  // print each file as discovered

    let (cst_opt, parse_errs) = writ_parser::parse(src);
    // handle errors
    let cst = cst_opt.ok_or("parse failed")?;
    let (ast, lower_errs) = writ_compiler::lower(cst);
    // handle errors

    per_file_asts.push((file_id, ast));
    all_source_refs.push((file_id, display_path, src));
}

// Build slice refs for pipeline
let asts_refs: Vec<(FileId, &Ast)> = per_file_asts.iter()
    .map(|(fid, ast)| (*fid, ast))
    .collect();
let path_refs: Vec<(FileId, &str)> = all_source_refs.iter()
    .map(|(fid, path, _)| (*fid, path.as_str()))
    .collect();
let sources_for_render: Vec<(FileId, &str, &str)> = all_source_refs.iter()
    .map(|(fid, path, src)| (*fid, path.as_str(), *src))
    .collect();

// Stage 3 onward — same as single-file but with multi-entry slices
let (resolved, resolve_diags) = writ_compiler::resolve::resolve(&asts_refs, &path_refs);
```

### Profile TOML Deserialization

```rust
// writ-compiler/src/config.rs — new additions

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileConfig {
    #[serde(default = "default_debug_info")]
    pub debug_info: bool,
}
fn default_debug_info() -> bool { true }

#[derive(Debug, Clone, Deserialize)]
pub struct ProfilesConfig {
    #[serde(default = "default_debug_profile")]
    pub debug: ProfileConfig,
    #[serde(default = "default_release_profile")]
    pub release: ProfileConfig,
}
impl Default for ProfilesConfig {
    fn default() -> Self {
        Self {
            debug: ProfileConfig { debug_info: true },
            release: ProfileConfig { debug_info: false },
        }
    }
}

// WritConfig gains:
// #[serde(default)]
// pub profile: ProfilesConfig,
```

### Profile Selection in cmd_build

```rust
// cmd_build — determine active profile
let use_release = release; // from clap arg
let active_profile = if use_release { "release" } else { "debug" };
let profile_cfg = if use_release {
    &config.profile.release
} else {
    &config.profile.debug
};
let emit_debug_info = profile_cfg.debug_info;

// Output path construction
let output_base = config.compiler.output.as_deref().unwrap_or("build");
let out_dir = std::path::Path::new(&project_root).join(output_base).join(active_profile);
std::fs::create_dir_all(&out_dir).map_err(|e| format!(...))?;
let out_path = out_dir.join(format!("{}.writc", effective_module_name));
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single-file CLI only | Multi-file APIs already exist in stages 3-5 | Built progressively across phases 20-44 | Phase 45 is integration work, not new infrastructure |
| Hardcoded module name "module" | `find_module_name()` heuristic in `collect.rs` | Phase 44 | Must be overridden by `project.name` for project builds |
| No profile system | `WritConfig` + `ProfileConfig` (NEW in this phase) | Phase 45 | First profile-aware build |

**Deprecated/outdated:**

- "Next steps" message in `cmd_new()` says `writ compile sources/main.writ` — must change to `writ build` in this phase
- `writ-compiler/src/emit/mod.rs` `emit_bodies()` hardcodes `"module"` as module name at line 88 — the path through `collect_defs` takes precedence (it calls `set_module_def` again via `find_module_name`), but this stale call should be removed or updated

---

## Open Questions

1. **Should `writ compile .` error with a helpful message, or silently invoke `writ build`?**
   - What we know: CONTEXT.md locks the decision — `writ compile .` does NOT auto-detect directories; it tries to compile `.` as a file path and fails with a file-not-found error.
   - What's unclear: Is the current error message ("failed to read '.'") helpful enough, or should `cmd_compile` detect that the input is a directory and emit the hint?
   - Recommendation: As a Claude's Discretion item, add a directory-detection check in `cmd_compile`: if the input path is a directory, emit "error: '.' is a directory. Use `writ build` to compile a project." This is a 2-line addition with high UX value.

2. **Does `check_decl.rs` update `ctx.current_file` per declaration?**
   - What we know: `check_fn_decl` reads `entry.file_id` from DefMap entry (line 39 in check_decl.rs). The `CheckCtx.current_file` field exists.
   - What's unclear: Whether `ctx.current_file` is updated per-declaration before diagnostic emission, ensuring multi-file error attribution is correct.
   - Recommendation: The planner should include a task to verify and add `ctx.current_file = entry.file_id;` at the start of each `check_*_decl` function if not already present.

3. **Should the default `sources` in `CompilerConfig` change from `["src/"]` to `["sources/"]`?**
   - What we know: The scaffold creates `sources/` but the default fallback in code is `src/`. The scaffold explicitly sets `sources = ["sources/"]` so scaffolded projects are fine.
   - What's unclear: Whether changing the default would be considered breaking.
   - Recommendation: Leave it as-is for Phase 45. The spec already aligns with the scaffold via the explicit TOML key.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (built-in) |
| Config file | none — workspace uses `cargo test` directly |
| Quick run command | `cargo test -p writ-cli -- cmd_build` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TOOL-01 | `writ build` in a dir with writ.toml discovers and compiles all .writ files | integration | `cargo test -p writ-cli -- build_compiles_project` | ❌ Wave 0 |
| TOOL-01 | `writ build` without writ.toml errors with hint | integration | `cargo test -p writ-cli -- build_missing_toml_error` | ❌ Wave 0 |
| TOOL-01 | single-file `writ compile foo.writ` still works | regression | `cargo test -p writ-cli -- compile_single_file` | ❌ Wave 0 (existing e2e tests cover compilation, not CLI invocation pattern) |
| TOOL-01 | `writ new` + `writ build` works without editing | integration | `cargo test -p writ-cli -- new_then_build` | ❌ Wave 0 |
| TOOL-02 | `--release` produces module without DebugLocal rows | integration | `cargo test -p writ-cli -- build_release_no_debug_locals` | ❌ Wave 0 |
| TOOL-02 | `--debug` (or no flag) produces module with DebugLocal rows | integration | `cargo test -p writ-cli -- build_debug_has_debug_locals` | ❌ Wave 0 |
| TOOL-02 | `[profile.release]` parsed from writ.toml | unit | `cargo test -p writ-compiler -- profile_config_round_trips` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p writ-cli 2>&1 | tail -5`
- **Per wave merge:** `cargo test --workspace 2>&1 | tail -10`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `writ-cli/tests/build_tests.rs` — covers all TOOL-01 and TOOL-02 integration tests (new file; uses `std::process::Command` or calls `cmd_build` directly via a test harness)
- [ ] `writ-compiler/tests/config_tests.rs` — covers `ProfileConfig` deserialization (extend existing `config.rs` unit tests or add new test file)

*(Note: the existing `writ-cli/tests/e2e_compile_tests.rs` covers the pipeline API but not the CLI `cmd_build` function. New integration tests should use `std::process::Command` to invoke the binary, or expose `cmd_build` in a way testable without full subprocess spawning.)*

---

## Sources

### Primary (HIGH confidence)

- Direct source inspection of `writ-cli/src/main.rs` — `cmd_compile`, `cmd_new`, `Commands` enum, clap derive patterns
- Direct source inspection of `writ-compiler/src/config.rs` — `WritConfig`, `load_config`, `discover_source_files`, `CompilerConfig::default`
- Direct source inspection of `writ-compiler/src/emit/mod.rs` — `emit_bodies` signature, module def naming
- Direct source inspection of `writ-compiler/src/emit/collect.rs` — `find_module_name`, `collect_defs`
- Direct source inspection of `writ-compiler/src/emit/body/debug.rs` — `emit_debug_locals` behavior
- Direct source inspection of `writ-compiler/src/resolve/mod.rs` — `resolve()` multi-file signature
- Direct source inspection of `writ-compiler/src/check/mod.rs` — `typecheck()` multi-file signature
- Direct source inspection of `writ-compiler/src/lower/mod.rs` — `lower()` single-file signature
- Direct source inspection of `writ-cli/Cargo.toml` — clap 4.5 confirmed
- Direct source inspection of `writ-compiler/Cargo.toml` — toml 0.9, serde 1, walkdir 2 confirmed

### Secondary (MEDIUM confidence)

- clap 4.x derive docs (from training knowledge, confirmed by existing code patterns) — `conflicts_with` attribute, `default_value` on positional args

### Tertiary (LOW confidence)

- None

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — all libraries already in the codebase
- Architecture: HIGH — directly inspected all relevant source files
- Pitfalls: HIGH — derived from actual code behavior (hardcoded module name, 'static lifetime, .gitignore)
- Validation: MEDIUM — test file names are new; framework behavior is well-established

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable Rust project; no external API changes expected)
