# Phase 21: Disassembler and Runner CLI - Research

**Researched:** 2026-03-02
**Domain:** CLI tooling, binary disassembly, runtime execution loop
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Disassembly Output Format**
- Round-trippable: disasm produces valid `.writil` text format that can be fed back to `writ assemble`
- Covers all 21 metadata tables for full round-trip fidelity
- Default output is clean (no extra annotations)
- `--verbose` flag adds hex offsets and opcode byte comments for debugging

**Runner say() and Host Output**
- Always annotated: all host interactions prefixed with `[say]`, `[choice]`, `[entity:spawn]`, `[entity:destroy]`, `[extern]`
- `choice()` prompts the user interactively when `--interactive` flag is passed
- Default: auto-pick choice 0 (non-interactive, safe for CI/piped usage)
- `--verbose` flag adds execution stats at the end (instructions executed, tasks spawned, etc.)

**Entry Point Convention**
- Export-based: runner looks for an exported method named `"main"` (lowercase)
- CLI override: `--entry <name>` to run a different exported method
- If no exported `"main"` found and no `--entry` override: error with helpful message listing available exports
- Entry method signature flexibility: if method accepts a parameter, pass `Array<String>` of CLI args (from `-- arg1 arg2`); if zero-parameter, call with no args

**CLI Binary Structure**
- Binary lives in existing `writ-cli` crate, binary name is `writ`
- Long-term vision: unified CLI (`writ build`, `writ run`, `writ disasm`, `writ assemble`, etc.)
- For now: `run`, `assemble`, `disasm` subcommands
- CLI argument parsing: clap with derive macros
- `assemble` accepts file path or stdin (`-` for stdin)
- Binary output extension: `.writc` (text = `.writil`, compiled binary = `.writc`)

**Code Organization**
- Disassembler logic in `writ-assembler` crate (text<->binary bridge, symmetric with assemble)
- `CliHost` (RuntimeHost impl for say/choice/entity logging) in `writ-cli` (CLI-specific behavior)
- `NullHost` in `writ-runtime` stays untouched (testing-only host)

### Claude's Discretion
- Whether disassembler resolves string heap offsets to inline literals or keeps raw indices
- Dialogue output to stdout vs entity/debug output to stderr split
- Exact execution stats format and content
- Interactive choice prompt UX details (prompt formatting, timeout behavior)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TOOL-01 | Disassembler converts binary modules to human-readable text IL | Disassembler as inverse of assembler; `writ-module::heap::read_string` and `read_blob` for heap resolution; `Instruction::decode` for body disassembly |
| TOOL-02 | Standalone runner CLI loads and executes IL modules | `RuntimeBuilder::new(module).with_host(CliHost).build()` pattern; `spawn_task` + tick loop; `ExportDefRow` for "main" discovery |
| TOOL-03 | NullHost outputs say() to stdout, choice() returns 0, externs return defaults | Note: per CONTEXT.md, `NullHost` stays untouched — `CliHost` is the new annotating host; `NullHost` already returns Void for externs |
| TOOL-04 | Runner CLI provides assemble/disasm/run subcommands | clap 4.5 derive macros; `writ-assembler::assemble()` for assemble subcommand; existing `Module::from_bytes`/`to_bytes` for binary I/O |
</phase_requirements>

## Summary

Phase 21 adds the `writ` binary to the existing `writ-cli` stub crate, wiring together all previously built components into a developer-facing CLI. The three subcommands form a clean triangle: `assemble` (text -> binary) already exists in `writ-assembler`; `disasm` (binary -> text) is the new symmetric inverse; `run` (binary -> execution) drives `writ-runtime` with a new `CliHost`.

The largest new piece of logic is the disassembler. It must reconstruct valid `.writil` text from a `Module` struct by: resolving string heap offsets to quoted string literals, decoding blob heap type signatures back to type reference syntax, and disassembling instruction byte sequences using `Instruction::decode`. The output must be parseable by the existing assembler — this is the round-trip constraint.

The runner loop is straightforward: discover the entry export, convert to a method index, spawn one task, then tick until `AllCompleted` or an error. The `CliHost` intercepts `HostRequest::ExternCall` to detect `say()` by extern index, printing `[say] <text>` to stdout. Entity events log to stdout with prefixes. The `--interactive` flag switches `choice()` from auto-0 to stdin prompting.

**Primary recommendation:** Build the disassembler first (it enables round-trip testing), then the CliHost, then wire the CLI. Keep each piece small and testable in isolation.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.5.60 | CLI argument parsing with derive macros | Industry standard for Rust CLIs; derive API eliminates boilerplate |
| writ-assembler | workspace | Disassembler lives here; assemble subcommand uses `assemble()` | Symmetric text<->binary bridge per user decision |
| writ-runtime | workspace | `RuntimeBuilder`, `Runtime`, `RuntimeHost` trait | All execution infrastructure already built |
| writ-module | workspace | `Module::from_bytes`, `to_bytes`, heap access, `Instruction::decode` | Pure-data layer shared across all crates |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| thiserror | 2.0 | Error types for disassembler errors | Already in workspace; consistent with existing crates |
| std::io | std | stdin reading for `assemble -` and interactive choice | No external dep needed |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| clap derive | clap builder API | Builder is more verbose; derive is idiomatic for simple CLIs |
| String format disassembly | Write to `fmt::Write` trait | `fmt::Write` is slightly more composable but String output is simpler and matches `assemble()` signature |

**Installation (Cargo.toml changes):**
```toml
# writ-cli/Cargo.toml
[dependencies]
writ-module = { path = "../writ-module" }
writ-runtime = { path = "../writ-runtime" }
writ-assembler = { path = "../writ-assembler" }
clap = { version = "4.5", features = ["derive"] }

[[bin]]
name = "writ"
path = "src/main.rs"
```

```toml
# writ-assembler/Cargo.toml — add thiserror already present
# No new deps needed for disassembler
```

## Architecture Patterns

### Recommended Project Structure
```
writ-cli/src/
├── main.rs          # clap CLI struct, subcommand dispatch
├── cli_host.rs      # CliHost: RuntimeHost impl with say/choice/entity logging
└── commands/
    ├── run.rs       # run subcommand: load + entry discovery + exec loop
    ├── assemble.rs  # assemble subcommand: wraps writ_assembler::assemble()
    └── disasm.rs    # disasm subcommand: calls writ_assembler::disassemble()

writ-assembler/src/
├── lib.rs           # add: pub mod disassembler; pub fn disassemble(module) -> String
└── disassembler.rs  # new: Module -> .writil text
```

Flat `commands/` module (or inline in `main.rs`) is fine for three commands; subdirectory structure shown above for clarity.

### Pattern 1: clap Derive CLI Structure
**What:** Top-level `Cli` struct with `Commands` enum; each subcommand is a struct.
**When to use:** Always — this is the locked decision.
**Example:**
```rust
// writ-cli/src/main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "writ", about = "Writ IL toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Assemble .writil text to binary .writc
    Assemble {
        /// Input file (or '-' for stdin)
        input: String,
        /// Output file (default: input with .writc extension)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Disassemble binary .writc to .writil text
    Disasm {
        /// Input binary module file
        input: String,
        /// Add hex offsets and opcode byte comments
        #[arg(long)]
        verbose: bool,
    },
    /// Run a binary .writc module
    Run {
        /// Input binary module file
        input: String,
        /// Entry method name (default: "main")
        #[arg(long, default_value = "main")]
        entry: String,
        /// Interactive choice prompts
        #[arg(long)]
        interactive: bool,
        /// Print execution stats after run
        #[arg(long)]
        verbose: bool,
        /// Arguments to pass to entry method (after --)
        #[arg(last = true)]
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Assemble { input, output } => cmd_assemble(input, output),
        Commands::Disasm { input, verbose } => cmd_disasm(input, verbose),
        Commands::Run { input, entry, interactive, verbose, args } => {
            cmd_run(input, entry, interactive, verbose, args)
        }
    };
    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
```

### Pattern 2: Disassembler as Module Walker
**What:** Walk `Module` struct fields in the same order the assembler writes them, emitting `.writil` directives. String heap offsets resolved via `writ_module::heap::read_string`.
**When to use:** Always — this produces round-trippable text.

The disassembler reconstruction order mirrors the assembler's Phase 1 declaration order:
1. `.module "name" "version" {`
2. `.extern "ModuleRef::name" "min_version"` for each ModuleRef
3. `.type "Name" kind { .field ... }` for each TypeDef + its FieldDefs
4. `.contract "Name" { .method ... }` for each ContractDef + ContractMethods + GenericParams
5. `.impl TypeName : ContractName { .method ... }` for each ImplDef + its methods
6. `.global "name" type_ref flags` for each GlobalDef
7. `.extern fn "name" (sig) -> ret "import_name"` for each ExternDef
8. Top-level `.method "name" (params) -> ret { ... }` for each unowned MethodDef
9. `}` closing the module

For method bodies: decode each `MethodBody.code` using `Instruction::decode` in a cursor loop, then map each `Instruction` variant back to its mnemonic string and operand text.

```rust
// writ-assembler/src/disassembler.rs
use std::fmt::Write;
use writ_module::{Instruction, Module};
use writ_module::heap::{read_string, read_blob};

pub fn disassemble(module: &Module) -> String {
    let mut out = String::new();
    // Read module name/version from string heap
    let name = read_string(&module.string_heap, module.header.module_name)
        .unwrap_or("");
    let ver = read_string(&module.string_heap, module.header.module_version)
        .unwrap_or("0.0.0");
    writeln!(out, ".module {:?} {:?} {{", name, ver).unwrap();
    // ... walk all 21 tables ...
    writeln!(out, "}}").unwrap();
    out
}
```

**Key detail:** String heap offsets are always resolved to inline quoted literals — this is the clean choice for round-trippability. Raw index output would require changes to the assembler to accept `@N` syntax, which does not exist.

### Pattern 3: Type Signature Decoding
**What:** Blob bytes from field/method signatures decoded back to type reference text.
**When to use:** Everywhere a field type, method parameter, or return type must appear in text output.

The encoding (from `assembler.rs`) is:
```
0x00 -> void
0x01 -> int
0x02 -> float
0x03 -> bool
0x04 -> string
0x10 u32(token) -> TypeDef or TypeRef by token value (resolved to name via type_defs/type_refs tables)
0x20 + elem_blob -> array<elem>
```

Method signature blobs start with `u16(param_count)`, then return type blob, then param type blobs.

The disassembler needs a `decode_type_ref(blob: &[u8], pos: &mut usize, module: &Module) -> String` function that walks these bytes and produces the text equivalent.

### Pattern 4: Instruction-to-Mnemonic Mapping
**What:** Inverse of `map_instruction` in assembler — each `Instruction` variant produces a mnemonic string and operand list.
**When to use:** In method body disassembly.

Every `Instruction` variant already has its opcode documented in `instruction.rs`. The mnemonic is the Rust variant name in SCREAMING_SNAKE_CASE (e.g., `Instruction::LoadInt` -> `"LOAD_INT"`). A `match` on all 91 variants producing `(mnemonic, operand_strings)` is the cleanest approach.

**Branch offset reconstruction:** The loaded `Module.method_bodies` contains raw byte offsets (not instruction indices). The disassembler operates on raw bytes using `Instruction::decode`, tracking byte position. Branch offsets in text IL are relative byte offsets — the disassembler can either:
- Emit raw integer offsets: `BR 8` (simpler, but not label-named)
- Emit synthetic labels: `BR .L_0010` with labels before their targets (more useful for human reading)

The round-trip constraint is satisfied either way, since the assembler accepts integer offsets directly. **Recommendation: start with raw integer offsets** (simpler to implement correctly) and add label synthesis as a follow-on if desired.

### Pattern 5: Entry Point Discovery
**What:** Scan `module.export_defs` for the row whose name string resolves to the entry name.
**When to use:** `run` subcommand entry point resolution.

```rust
fn find_entry(module: &Module, name: &str) -> Option<usize> {
    for export in &module.export_defs {
        let export_name = read_string(&module.string_heap, export.name).ok()?;
        if export_name == name {
            // item is a MetadataToken into MethodDef table (1-based)
            let method_idx = (export.item.0 as usize).saturating_sub(1);
            return Some(method_idx);
        }
    }
    None
}
```

`ExportDefRow.item_kind` should be checked (method export vs type export), but for the "main" convention it will always be a method.

### Pattern 6: Runner Execution Loop
**What:** Build runtime with `CliHost`, spawn entry task, tick until done.
**When to use:** `run` subcommand.

```rust
fn cmd_run(input: String, entry: String, interactive: bool, verbose: bool, args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let bytes = std::fs::read(&input)?;
    let module = Module::from_bytes(&bytes)
        .map_err(|e| format!("failed to load module: {:?}", e))?;

    // Find entry export
    let method_idx = find_entry(&module, &entry)
        .ok_or_else(|| format!("no exported method '{}' found", entry))?;

    let host = CliHost::new(interactive, verbose);
    let mut runtime = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .map_err(|e| format!("runtime init failed: {:?}", e))?;

    // Spawn entry task (with args if method has parameters — phase can simplify to zero-arg first)
    runtime.spawn_task(method_idx, vec![])?;

    // Tick loop until completion
    loop {
        let result = runtime.tick(0.0, ExecutionLimit::None);
        match result {
            TickResult::AllCompleted => break,
            TickResult::Empty => break,
            TickResult::TasksSuspended(pending) => {
                // CliHost handles requests synchronously; this should not occur
                // unless host returned a non-confirming response
                for req in pending {
                    let response = runtime.host_mut().resolve_pending(&req);
                    runtime.confirm(req.request_id, response)?;
                }
            }
            TickResult::ExecutionLimitReached => {
                // With ExecutionLimit::None this should not occur
                break;
            }
        }
    }

    if verbose {
        // Print stats from host
        runtime.host().print_stats();
    }
    Ok(())
}
```

**Important:** `CliHost` uses a synchronous `on_request` interface. All requests are handled immediately in `on_request`, returning `HostResponse::Confirmed` or `HostResponse::Value`. Tasks never reach `Suspended` state when using `CliHost` (same as `NullHost`). The tick loop's `TasksSuspended` branch is dead code for the CliHost case.

### Pattern 7: CliHost Design
**What:** Implements `RuntimeHost`; intercepts `ExternCall` to detect say/choice by extern index; all other requests confirm immediately.
**When to use:** `run` subcommand.

The key challenge: how does `CliHost` know which extern index corresponds to `say()` vs `choice()`? The host needs to look up ExternDef names at startup or during request handling.

**Approach:** Pass the loaded `Module` (or its extern names) to `CliHost` at construction. During `on_request(ExternCall { extern_idx, args })`, resolve extern_idx -> `ExternDef.name` -> action.

```rust
pub struct CliHost {
    extern_names: Vec<String>, // parallel to module.extern_defs
    interactive: bool,
    instructions_executed: u64,
    tasks_spawned: u64,
}

impl CliHost {
    pub fn new(module: &Module, interactive: bool) -> Self {
        let extern_names = module.extern_defs.iter()
            .map(|e| read_string(&module.string_heap, e.name).unwrap_or("").to_string())
            .collect();
        CliHost { extern_names, interactive, instructions_executed: 0, tasks_spawned: 0 }
    }
}

impl RuntimeHost for CliHost {
    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { extern_idx, args, .. } => {
                // extern_idx is a full MetadataToken: decode row_index from bits 23-0 (1-based)
                let row_idx = (extern_idx & 0x00FF_FFFF) as usize;
                let name = if row_idx > 0 {
                    self.extern_names.get(row_idx - 1).map(|s| s.as_str()).unwrap_or("?")
                } else { "?" };
                match name {
                    "say" => {
                        // Value::Ref(HeapRef) — no string variant in Value enum.
                        // For Phase 21: print heap ref placeholder; full string resolution is future work.
                        let text = match args.first() {
                            Some(Value::Ref(h)) => format!("<string@{}>", h.0),
                            Some(Value::Int(i)) => i.to_string(),
                            _ => "<no text>".to_string(),
                        };
                        println!("[say] {}", text);
                        HostResponse::Value(Value::Void)
                    }
                    "choice" => {
                        if self.interactive {
                            // prompt stdin, return selected index
                            HostResponse::Value(Value::Int(prompt_choice(args)))
                        } else {
                            println!("[choice] auto-selecting 0");
                            HostResponse::Value(Value::Int(0))
                        }
                    }
                    _ => {
                        println!("[extern] {}()", name);
                        HostResponse::Value(Value::Void)
                    }
                }
            }
            HostRequest::EntitySpawn { type_idx, .. } => {
                println!("[entity:spawn] type={}", type_idx);
                HostResponse::Confirmed
            }
            HostRequest::DestroyEntity { entity, .. } => {
                println!("[entity:destroy] entity={:?}", entity);
                HostResponse::Confirmed
            }
            _ => HostResponse::Confirmed,
        }
    }
    fn on_log(&mut self, level: LogLevel, message: &str) {
        eprintln!("[{:?}] {}", level, message);
    }
}
```

**String value printing problem:** `say()` will pass a string argument, but in the VM strings are heap references (`Value::Ref(HeapRef)`). `CliHost.on_request` receives `Vec<Value>` but has no access to the GC heap to dereference string objects. This is a real architectural gap.

**Resolution options:**
1. The runtime could resolve string registers to `Value::String(String)` before making ExternCall requests (simplest for this phase)
2. Pass the heap reference to the host and let host call back (overcomplicates interface)
3. Pre-intern strings as a separate primitive `Value::String` in the value type

Looking at `value.rs` in writ-runtime for what `Value` contains is important here.

### Anti-Patterns to Avoid
- **Emitting raw heap offsets:** `field "name_offset_42"` instead of `field "player_name"` — makes output unreadable and non-round-trippable
- **Skipping the disassembler for method bodies:** Only emitting type/field metadata without instruction bodies — breaks round-trip
- **Building a new execution loop from scratch:** The `Runtime` API already has `spawn_task` + `tick` + `confirm` — use it
- **Putting CliHost in writ-runtime:** User decision is explicit: CliHost lives in writ-cli

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI arg parsing | Custom argv parser | clap 4.5 with derive | Help text, error messages, shell completion — all free |
| Binary file I/O | Manual byte reads | `std::fs::read` / `std::fs::write` | One-liner for module loading |
| Instruction decoding | Re-implement decode | `Instruction::decode(&mut cursor)` in a loop | Already exists in writ-module with full 91-opcode coverage |
| String heap reads | Manual offset arithmetic | `writ_module::heap::read_string(heap, offset)` | Already implemented with UTF-8 validation |
| Blob heap reads | Manual offset arithmetic | `writ_module::heap::read_blob(heap, offset)` | Already implemented |
| Module loading | Custom binary parser | `Module::from_bytes(bytes)` | Full spec-compliant parser already in writ-module |

**Key insight:** The entire binary format is already handled. The disassembler's job is purely text generation from the already-decoded `Module` struct.

## Common Pitfalls

### Pitfall 1: String Heap Offset 0 Is Not a Bug
**What goes wrong:** Disassembler encounters offset 0 in a string field (e.g., `ModuleDefRow.name = 0`) and either panics or emits garbage.
**Why it happens:** Offset 0 is the null/empty string by spec (`heap::init_string_heap` writes a u32(0) at the start). `read_string(heap, 0)` correctly returns `""`.
**How to avoid:** Always use `read_string` — never assume offset 0 is an error.

### Pitfall 2: Method Body Index Alignment
**What goes wrong:** `module.method_bodies[i]` does not align to `module.method_defs[i]` as expected — crashes with index out of bounds or emits wrong body.
**Why it happens:** `method_bodies` is supposed to be parallel to `method_defs` (both indexed the same way), but the assembler's two-pass approach replaces placeholder bodies in order. If a method has `body_size == 0`, there may still be an entry in `method_bodies` (empty code vec). Verify the parallel structure holds.
**How to avoid:** Assert `method_bodies.len() == method_defs.len()` at disassembler start. Treat `body.code.is_empty()` as "no body" (emit no instruction block or just `{}`).

### Pitfall 3: Branch Offsets Are Byte Offsets, Not Instruction Indices
**What goes wrong:** Disassembler outputs instruction-index offsets (as `LoadedModule` does for the VM), but the assembler expects byte offsets.
**Why it happens:** `writ-runtime::loader` converts branch targets from byte offsets to instruction indices for the VM's use. The raw `Module.method_bodies[i].code` bytes contain the original byte offsets.
**How to avoid:** Disassemble from `module.method_bodies[i].code` (raw bytes), not from `LoadedModule.decoded_bodies` (reindexed). Use `Instruction::decode` in a cursor loop on raw bytes. Branch offsets in the raw form are byte-relative and exactly what the assembler expects.

### Pitfall 4: say() Gets a Heap Reference, Not a String
**What goes wrong:** `CliHost` tries to print `args[0]` but gets `Value::Ref(HeapRef { idx: 5, gen: 0 })` instead of the string text.
**Why it happens:** Writ strings are reference types living in the GC heap. `ExternCall` args are `Vec<Value>`. The host has no access to the GC heap.
**How to avoid:** Before implementing `[say]` output, check what `Value` variant `say()` actually receives. Look at `writ-runtime/src/value.rs` to see if `Value::String(String)` exists as a variant (distinct from heap-allocated strings). If strings passed to externs are pre-resolved to `Value::String(String)`, printing is trivial. If they are `Value::Ref`, the exec loop needs to resolve them before the `ExternCall` is issued, or `CliHost` needs heap access.

### Pitfall 5: Token Values in Instructions Are 1-Based
**What goes wrong:** Disassembler converts `type_idx: 3` in a `New` instruction to the string "type 3", but in a module with only 2 TypeDefs, this is out of range — or worse, outputs the wrong type name.
**Why it happens:** `MetadataToken` is 1-based (token 0 = null, token 1 = first row). `type_defs[0]` corresponds to token value 1.
**How to avoid:** When resolving token operands to names, use `token_val - 1` as the slice index. Guard against 0 (null token) and out-of-range values.

### Pitfall 6: `writ-cli` Cargo.toml Needs `[[bin]]` Section
**What goes wrong:** `cargo build` does not produce a `writ` binary — only `writ-cli` or nothing.
**Why it happens:** With custom binary names, Cargo requires either `[[bin]] name = "writ"` or a `src/main.rs` (which defaults to package name). Since the package is named `writ-cli`, the auto-detected binary name would be `writ-cli`, not `writ`.
**How to avoid:** Add `[[bin]] name = "writ" path = "src/main.rs"` to `writ-cli/Cargo.toml`, or rename the package to `writ`. The locked decision is binary name `writ`, so `[[bin]]` is required.

### Pitfall 7: `extern_idx` in `HostRequest::ExternCall` Is a Full MetadataToken
**What goes wrong:** `CliHost` uses `extern_idx as usize` directly to index into its `extern_names` vec — gets the wrong name or panics with an out-of-bounds index.
**Why it happens:** `extern_idx` is a full `MetadataToken` value (bits 31-24 = table_id, bits 23-0 = 1-based row index). For ExternDef (table_id=0x10), the first extern has `extern_idx = 0x10000001`, not `0` or `1`.
**How to avoid:** Always decode: `let row_idx = (extern_idx & 0x00FF_FFFF) as usize; let slice_idx = row_idx - 1;`. Then index `extern_names[slice_idx]`. Verified: `token.rs` confirms MetadataToken layout, `builder.rs` confirms ExternDef tokens use `len+1` as row_index.

## Code Examples

### Minimal Disassembler Skeleton
```rust
// writ-assembler/src/disassembler.rs
use std::fmt::Write;
use std::io::Cursor;
use writ_module::{Instruction, Module};
use writ_module::heap::{read_string, read_blob};
use writ_module::tables::{TypeDefKind, TableId};

pub fn disassemble(module: &Module) -> String {
    disassemble_inner(module, false)
}

pub fn disassemble_verbose(module: &Module) -> String {
    disassemble_inner(module, true)
}

fn disassemble_inner(module: &Module, verbose: bool) -> String {
    let mut out = String::new();
    let s = |offset: u32| -> &str {
        read_string(&module.string_heap, offset).unwrap_or("")
    };

    let name = s(module.header.module_name);
    let ver = s(module.header.module_version);
    writeln!(out, ".module {:?} {:?} {{", name, ver).unwrap();

    // Module refs
    for mr in &module.module_refs {
        writeln!(out, "    .extern {:?} {:?}", s(mr.name), s(mr.min_version)).unwrap();
    }

    // Types and fields
    for (ti, td) in module.type_defs.iter().enumerate() {
        let kind_str = match TypeDefKind::from_u8(td.kind) {
            Some(TypeDefKind::Struct) => "struct",
            Some(TypeDefKind::Enum) => "enum",
            Some(TypeDefKind::Entity) => "entity",
            Some(TypeDefKind::Component) => "component",
            None => "struct",
        };
        writeln!(out, "    .type {:?} {} {{", s(td.name), kind_str).unwrap();
        // Fields belonging to this type (field_list range)
        let field_start = td.field_list.saturating_sub(1) as usize;
        let field_end = module.type_defs.get(ti + 1)
            .map(|next| next.field_list.saturating_sub(1) as usize)
            .unwrap_or(module.field_defs.len());
        for fd in &module.field_defs[field_start..field_end] {
            let type_text = decode_type_sig(&module.blob_heap, fd.type_sig, module);
            writeln!(out, "        .field {:?} {} {:#06x}", s(fd.name), type_text, fd.flags).unwrap();
        }
        writeln!(out, "    }}").unwrap();
    }

    // Methods (top-level and impl methods)
    // ... (see patterns above for ordering)

    writeln!(out, "}}").unwrap();
    out
}
```

### Type Signature Decoder
```rust
fn decode_type_sig(blob_heap: &[u8], sig_offset: u32, module: &Module) -> String {
    let blob = writ_module::heap::read_blob(blob_heap, sig_offset).unwrap_or(&[]);
    let mut pos = 0;
    decode_type_ref(blob, &mut pos, module)
}

fn decode_type_ref(blob: &[u8], pos: &mut usize, module: &Module) -> String {
    let s = |offset: u32| -> &str {
        writ_module::heap::read_string(&module.string_heap, offset).unwrap_or("?")
    };
    match blob.get(*pos).copied() {
        Some(0x00) => { *pos += 1; "void".to_string() }
        Some(0x01) => { *pos += 1; "int".to_string() }
        Some(0x02) => { *pos += 1; "float".to_string() }
        Some(0x03) => { *pos += 1; "bool".to_string() }
        Some(0x04) => { *pos += 1; "string".to_string() }
        Some(0x10) => {
            *pos += 1;
            if *pos + 4 <= blob.len() {
                let token = u32::from_le_bytes(blob[*pos..*pos+4].try_into().unwrap());
                *pos += 4;
                let idx = (token as usize).saturating_sub(1);
                if let Some(td) = module.type_defs.get(idx) {
                    s(td.name).to_string()
                } else if let Some(tr) = module.type_refs.get(idx) {
                    s(tr.name).to_string()
                } else {
                    format!("type_{}", token)
                }
            } else {
                *pos = blob.len();
                "?".to_string()
            }
        }
        Some(0x20) => {
            *pos += 1;
            let elem = decode_type_ref(blob, pos, module);
            format!("array<{}>", elem)
        }
        _ => { *pos = blob.len(); "?".to_string() }
    }
}
```

### Instruction Disassembly Loop
```rust
fn disassemble_method_body(code: &[u8], verbose: bool) -> String {
    let mut out = String::new();
    let mut cursor = Cursor::new(code);
    while (cursor.position() as usize) < code.len() {
        let byte_offset = cursor.position() as usize;
        match Instruction::decode(&mut cursor) {
            Ok(instr) => {
                let (mnemonic, operands) = instr_to_text(&instr);
                if verbose {
                    write!(out, "        // +{:#06x}\n", byte_offset).unwrap();
                }
                if operands.is_empty() {
                    writeln!(out, "        {}", mnemonic).unwrap();
                } else {
                    writeln!(out, "        {} {}", mnemonic, operands.join(", ")).unwrap();
                }
            }
            Err(e) => {
                writeln!(out, "        // decode error at +{:#06x}: {:?}", byte_offset, e).unwrap();
                break;
            }
        }
    }
    out
}

fn instr_to_text(instr: &Instruction) -> (String, Vec<String>) {
    match instr {
        Instruction::Nop => ("NOP".into(), vec![]),
        Instruction::Crash { r_msg } => ("CRASH".into(), vec![format!("r{}", r_msg)]),
        Instruction::Mov { r_dst, r_src } => ("MOV".into(), vec![format!("r{}", r_dst), format!("r{}", r_src)]),
        Instruction::LoadInt { r_dst, value } => ("LOAD_INT".into(), vec![format!("r{}", r_dst), format!("{}", value)]),
        Instruction::LoadFloat { r_dst, value } => ("LOAD_FLOAT".into(), vec![format!("r{}", r_dst), format!("{}", value)]),
        Instruction::LoadTrue { r_dst } => ("LOAD_TRUE".into(), vec![format!("r{}", r_dst)]),
        Instruction::LoadFalse { r_dst } => ("LOAD_FALSE".into(), vec![format!("r{}", r_dst)]),
        Instruction::LoadString { r_dst, string_idx } => ("LOAD_STRING".into(), vec![format!("r{}", r_dst), format!("{}", string_idx)]),
        Instruction::LoadNull { r_dst } => ("LOAD_NULL".into(), vec![format!("r{}", r_dst)]),
        // ... all 91 variants ...
        Instruction::Br { offset } => ("BR".into(), vec![format!("{}", offset)]),
        Instruction::BrTrue { r_cond, offset } => ("BR_TRUE".into(), vec![format!("r{}", r_cond), format!("{}", offset)]),
        Instruction::RetVoid => ("RET_VOID".into(), vec![]),
        Instruction::Ret { r_src } => ("RET".into(), vec![format!("r{}", r_src)]),
        // etc.
    }
}
```

### Entry Point Discovery
```rust
fn find_entry_method(module: &Module, name: &str) -> Result<usize, String> {
    use writ_module::heap::read_string;
    let mut available = Vec::new();
    for export in &module.export_defs {
        let export_name = read_string(&module.string_heap, export.name)
            .unwrap_or("");
        if export_name == name {
            // token is 1-based; convert to 0-based method index
            let method_idx = (export.item.0 as usize).saturating_sub(1);
            return Ok(method_idx);
        }
        available.push(export_name.to_string());
    }
    Err(format!(
        "no exported method '{}' found. Available exports: [{}]",
        name,
        available.join(", ")
    ))
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Separate binaries (writ-asm, writ-run) | Unified `writ` binary with subcommands | Phase 21 decision | Single install, unified UX |
| NullHost for all execution | CliHost for CLI, NullHost for tests | Phase 21 | NullHost stays pristine for tests |
| No end-to-end test | `writ run` executing real .writc module | Phase 21 | Validates entire stack |

**Deprecated/outdated:**
- `writ-cli/src/main.rs` stub: Replace entirely with clap CLI structure.

## Open Questions

1. **How does say() receive its string argument?**
   - What we know: `Value` enum (verified in `value.rs`) is `Void | Int(i64) | Float(f64) | Bool(bool) | Ref(HeapRef) | Entity(EntityId)`. There is NO `Value::String(String)` variant. Strings in the VM are always heap references (`Value::Ref(HeapRef)`).
   - What's unclear: How does `CliHost` print the actual string content from a `HeapRef`? The host receives `Vec<Value>` in `on_request` but has no access to the GC heap.
   - Recommendation: For Phase 21, `CliHost` should receive the `Module` (or its string heap) and the GC heap at construction or via a callback. The simplest approach: pass the module to `CliHost` so it can at least resolve string literals from the module's string heap by HeapRef index if strings are stored there. However, runtime-allocated strings (built via `LOAD_STRING r0, idx` + heap allocation) may not map directly. A pragmatic Phase 21 approach: if arg is `Value::Ref(h)`, print `[say] <string ref {}>` as a placeholder. Full string printing may require VM-level pre-resolution before the ExternCall is issued, which is a runtime change. This is a known limitation to document.

2. **How does `extern_idx` in `CallExtern` map to `ExternDef` rows?**
   - What we know: `CallExtern { extern_idx: u32 }` carries a full `MetadataToken` value (table_id=0x10 in bits 31-24, 1-based row in bits 23-0). `dispatch.rs` passes it directly to `HostRequest::ExternCall.extern_idx`. The host receives the full packed token.
   - Resolution: `CliHost` must decode it as `row_index = (extern_idx & 0x00FF_FFFF)` then use `(row_index - 1)` as the 0-based slice index into `module.extern_defs`. Verified by reading `token.rs` (`MetadataToken` layout) and `builder.rs` (`add_extern_def` uses `MetadataToken::new(TableId::ExternDef, len+1)`).
   - Status: RESOLVED — no longer an open question.

3. **Method ownership: which MethodDef rows are impl methods vs top-level?**
   - What we know: `TypeDefRow.method_list` points to the first MethodDef row of a type. `ImplDefRow.method_list` points to the first MethodDef row for that impl.
   - What's unclear: The disassembler needs to know whether to emit a method as a top-level `.method` or inside a `.impl` or inside a `.type`. The boundary detection (which methods belong to which type/impl) requires careful range calculation similar to field_list detection.
   - Recommendation: Reconstruct the same range logic as the assembler's first pass. Methods between `method_list[i]` and `method_list[i+1]` belong to type/impl `i`.

## Sources

### Primary (HIGH confidence)
- Direct codebase reading: `writ-module/src/instruction.rs` — all 91 instructions with opcodes verified
- Direct codebase reading: `writ-module/src/tables.rs` — all 21 table row structs verified
- Direct codebase reading: `writ-module/src/heap.rs` — `read_string`/`read_blob`/`intern_string` API verified
- Direct codebase reading: `writ-assembler/src/assembler.rs` — type encoding bytes (0x00-0x04, 0x10, 0x20) verified
- Direct codebase reading: `writ-runtime/src/runtime.rs` — `RuntimeBuilder`, `tick`, `spawn_task`, `confirm` API verified
- Direct codebase reading: `writ-runtime/src/host.rs` — `RuntimeHost` trait, `HostRequest` variants, `NullHost` verified
- Direct codebase reading: `writ-runtime/src/value.rs` — `Value` enum confirmed: no `Value::String` variant; strings are always `Value::Ref(HeapRef)`
- Direct codebase reading: `writ-module/src/token.rs` — `MetadataToken` layout (table_id in bits 31-24, 1-based row in bits 23-0) verified
- Direct codebase reading: `writ-runtime/src/dispatch.rs` lines 555-587 — `CallExtern` dispatch passes raw `extern_idx` token to host unchanged
- `cargo search clap --limit 1` — clap 4.5.60 confirmed as current version

### Secondary (MEDIUM confidence)
- Existing assembler tests (`asm_basic.rs`, `asm_round_trip.rs`) — text format confirmed as ground truth for round-trip output

### Tertiary (LOW confidence)
- None remaining — all previously uncertain items resolved via direct code reading.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — clap version confirmed, all workspace crates verified
- Architecture: HIGH — all existing APIs confirmed by direct code reading
- Pitfalls: HIGH — value.rs and token.rs read and verified; MetadataToken layout and Value enum both confirmed; say() string issue documented as known limitation

**Research date:** 2026-03-02
**Valid until:** 2026-04-01 (stable codebase, no external deps moving fast)
