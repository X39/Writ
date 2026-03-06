# Phase 55: DAP Server Core - Research

**Researched:** 2026-03-14
**Domain:** Debug Adapter Protocol (DAP) server in Rust, stdio transport, VM debug hook integration
**Confidence:** HIGH (codebase read directly; DAP protocol from official spec; dap crate from docs.rs)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
None — all areas delegated to Claude's discretion.

### Claude's Discretion
- DAP protocol crate: use `dap` 0.4.1-alpha1 or hand-roll with serde_json. Validate compilation as first task; serde_json fallback if it fails.
- Launch flow: whether DAP server compiles .writ on launch or expects pre-compiled input.
- Breakpoint model: verified vs pending, snapping to nearest valid line.
- Call stack frame naming format and extern frame visibility.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DAP-01 | User can launch a Writ program in the VS Code debugger via F5 with a launch.json configuration | Launch flow, `initialize`/`launch`/`configurationDone` request handling, DebugHost compilation-on-launch |
| DAP-02 | User can set source-level breakpoints on .writ file lines and execution pauses there | `setBreakpoints` request, SourceSpan table lookup, `DebugAction::Break` from `before_instruction`, verified/unverified model |
| DAP-03 | User can step over, step into, and step out of function calls at source level | `next`/`stepIn`/`stepOut` requests, stepping state machine in DebugHost, `DebugAction::StepOver/StepInto/StepOut` |
| DAP-05 | User can see the full call stack with source locations when execution is paused | `stackTrace`/`threads` requests, `Task.call_stack` + `SourceSpan` lookup per frame, method name resolution from `MethodDef` table |
</phase_requirements>

---

## Summary

Phase 55 builds `writ-dap`, a new workspace crate implementing a Debug Adapter Protocol server over stdio. It is structurally analogous to `writ-lsp` — a binary that VS Code launches as a child process and communicates with over stdin/stdout. Unlike the LSP, DAP is synchronous (no async/await); the `dap` 0.4.1-alpha1 crate wraps stdin/stdout in a blocking `Server` with a `poll_request()` loop. All runtime hooks (`before_instruction`, `on_function_enter`, `on_function_exit`) are already in place from Phase 52. The key new work is: (1) a `DebugHost` struct that implements `RuntimeHost` and translates VM suspension events into DAP protocol messages, (2) a stepping state machine tracking call depth and source lines, (3) source-span-based breakpoint lookup and verification, and (4) call stack rendering using the module's `MethodDef` table and `SourceSpan` data.

The `dap` crate (v0.4.1-alpha1, released September 2023) provides type-safe DAP message parsing/serialization including all 42 request `Command` variants, 40 `ResponseBody` variants, and 17 `Event` variants. The crate is pre-1.0 and explicitly warns of breaking changes, but it compiles and provides significant boilerplate reduction over raw serde_json. Validating it compiles against the workspace toolchain (Rust edition 2024) is the mandatory first task.

The runtime is already structured for DAP integration: `SuspendReason::Breakpoint` and `SuspendReason::DebugStep` carry `(method_idx, pc, line, col)`, `ExecutionResult::DebugSuspend` stops the scheduler loop, and `Runtime::resume_debug(task_id)` re-enqueues the task. The DAP server only needs to drive this: run `runtime.tick()`, check for `DebugSuspend` outcomes via task state inspection, emit `Stopped` events to VS Code, then act on user commands.

**Primary recommendation:** Implement the DAP server as a synchronous loop on the main thread — `poll_request()` to read VS Code commands, run the runtime in a tight tick loop until a debug suspension, send `Stopped` event, wait for the next client command. No async, no threads. The `dap` crate's `Server` handles framing; `DebugHost` owns breakpoint state and stepping state. The compilation-on-launch approach (compile `.writ` → bytes → load module) keeps the server self-contained.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `dap` | 0.4.1-alpha1 | DAP message types, Server, poll/respond/event API | Only Rust DAP server crate with complete Command/ResponseBody coverage; reduces ~400 lines of serde_json hand-rolling |
| `serde_json` | 1 | JSON serialization (already in workspace via writ-lsp) | Fallback if `dap` fails to compile; already available |
| `tokio` | 1 (io-std) | Stdio I/O for LSP pattern — BUT: dap crate uses sync BufRead/BufWrite | Already in workspace; NOT needed for dap — dap uses std::io |
| `writ-compiler` | path | Compile .writ source to bytes on launch | `run_pipeline()` reuse for compile-on-launch |
| `writ-runtime` | path | VM execution, RuntimeHost trait, DebugHost integration | All debug hooks already implemented |
| `writ-module` | path | Module decoding, SourceSpan table, MethodDef name lookup | Breakpoint mapping and call stack naming |
| `writ-diagnostics` | path | FileId for multi-file compilation | Already used in writ-cli run_pipeline pattern |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde_json` | 1 | Direct fallback if dap crate doesn't compile | Wave 0 validation task determines need |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `dap` 0.4.1-alpha1 | Hand-rolled serde_json types | ~400 lines of boilerplate with no compile-time safety; use only if dap fails compilation check |
| `dap` 0.4.1-alpha1 | `dap-types` (lapce crate) | Type definitions only, no Server transport; still need framing manually |

**Installation:**
```bash
# In writ-dap/Cargo.toml
dap = "0.4.1-alpha1"
writ-compiler = { path = "../writ-compiler" }
writ-runtime = { path = "../writ-runtime" }
writ-module = { path = "../writ-module" }
writ-diagnostics = { path = "../writ-diagnostics" }
```

---

## Architecture Patterns

### Recommended Project Structure
```
writ-dap/
├── Cargo.toml              # dap = "0.4.1-alpha1", writ-* deps
├── src/
│   ├── main.rs             # Entry point: create Server(stdin, stdout), run dap_server loop
│   ├── lib.rs              # pub mod declarations
│   ├── server.rs           # DapServer: owns Server<stdin,stdout>, DebugHost, Runtime
│   ├── debug_host.rs       # DebugHost: implements RuntimeHost, owns step state + breakpoints
│   ├── breakpoints.rs      # BreakpointTable: file→[(line→(bp_id, method_idx, pc))] mapping
│   ├── call_stack.rs       # build_stack_frames(): Task.call_stack → Vec<StackFrame>
│   └── launch.rs           # compile_and_load(): .writ → Module via run_pipeline
```

### Pattern 1: Synchronous DAP Server Loop

**What:** Single-threaded polling loop alternating between reading DAP client commands and running the VM.

**When to use:** Always — the `dap` crate's `Server` is synchronous (BufRead/BufWrite), and the Writ runtime is synchronous. No async needed or beneficial here.

**Example:**
```rust
// Source: dap 0.4.1-alpha1 docs, lib.rs/crates/dap pattern
use dap::prelude::*;
use std::io::{BufReader, BufWriter};

fn main() {
    let input = BufReader::new(std::io::stdin());
    let output = BufWriter::new(std::io::stdout());
    let mut server = Server::new(input, output);

    let mut dap_server = DapServer::new(server);
    dap_server.run(); // blocks until Disconnect
}
```

### Pattern 2: DebugHost State Machine

**What:** `DebugHost` implements `RuntimeHost` and is the bridge between the VM's instruction-level debug hooks and the DAP step/breakpoint model.

**When to use:** The VM calls `before_instruction()` on every instruction when `debug_enabled()` is true. `DebugHost` inspects current state and returns the appropriate `DebugAction`.

**Key state in DebugHost:**
```rust
pub struct DebugHost {
    // Breakpoint state
    breakpoints: BreakpointTable,  // file+line -> method_idx+pc (pre-resolved)
    next_bp_id: u32,

    // Stepping state
    step_mode: StepMode,
    // For StepOver: the call depth at which we started stepping
    step_origin_depth: usize,
    // For StepOver/Into: the source line we started on (don't stop on same line)
    step_origin_line: u32,
    step_origin_method: u32,

    // Current execution control
    current_action: DebugAction,
    debug_active: bool,
}

enum StepMode {
    None,
    StepOver { origin_depth: usize, origin_line: u32 },
    StepInto { origin_line: u32, origin_method: u32 },
    StepOut  { origin_depth: usize },
}
```

**`before_instruction` logic:**
```rust
fn before_instruction(&mut self, task_id: TaskId, method_idx: u32, pc: u32,
                       source_line: u32, source_col: u16) -> DebugAction {
    // 1. Check breakpoints (always, regardless of step mode)
    if let Some(_bp_id) = self.breakpoints.lookup(method_idx, pc) {
        self.pending_stop = Some(StopReason::Breakpoint(_bp_id));
        return DebugAction::Break;
    }

    // 2. Check stepping
    match self.step_mode {
        StepMode::StepOver { origin_depth, origin_line } => {
            let depth = current_call_depth; // passed from scheduler context
            if depth <= origin_depth && source_line != origin_line {
                self.pending_stop = Some(StopReason::Step);
                return DebugAction::Break;
            }
        }
        StepMode::StepInto { origin_line, origin_method } => {
            if source_line != origin_line || method_idx != origin_method {
                self.pending_stop = Some(StopReason::Step);
                return DebugAction::Break;
            }
        }
        StepMode::StepOut { origin_depth } => {
            if current_call_depth < origin_depth {
                self.pending_stop = Some(StopReason::Step);
                return DebugAction::Break;
            }
        }
        StepMode::None => {}
    }

    DebugAction::Continue
}
```

**Design note on call depth:** The `before_instruction` hook receives `(task_id, method_idx, pc, line, col)` but NOT call depth. Call depth must be tracked via `on_function_enter`/`on_function_exit` hooks, which fire for every function call. `DebugHost` maintains a `call_depth: HashMap<TaskId, usize>` updated by these hooks.

### Pattern 3: Launch Flow

**What:** On receiving a DAP `Launch` request, compile the .writ file to bytes in-process, load the module, create the runtime, and spawn the entry task.

**When to use:** Single-file launch. If `writ.toml` detected, use `discover_source_files` + multi-file compilation.

```rust
// Source: writ-cli/src/main.rs run_pipeline() pattern
fn compile_and_load(program_path: &str) -> Result<Module, String> {
    let src: &'static str = Box::leak(std::fs::read_to_string(program_path)
        .map_err(|e| e.to_string())?.into_boxed_str());
    let file_id = writ_diagnostics::FileId(0);
    let bytes = run_pipeline(
        vec![(file_id, program_path.to_string(), src)],
        None,
        true, // always emit debug info
    ).map_err(|e| e)?;
    Module::from_bytes(&bytes).map_err(|e| format!("{:?}", e))
}
```

**Note:** `run_pipeline` is not pub in writ-cli. The DAP server must replicate the 5-stage pipeline (parse → lower → resolve → typecheck → emit) using the public APIs from writ-compiler, or `run_pipeline` must be extracted to a shared crate. **Best approach**: copy the pipeline into `writ-dap/src/launch.rs` (same as writ-cli does it). The function is ~80 lines and both callers need it independently.

### Pattern 4: Breakpoint Mapping

**What:** Translate DAP's `(source_path, line_number)` into `(method_idx, pc)` pairs using the module's `SourceSpan` table.

**When to use:** On `setBreakpoints` request. Build a reverse-lookup index from the loaded module at launch time.

```rust
// Source: writ-module/src/module.rs SourceSpan structure
// Each MethodBody has: source_spans: Vec<SourceSpan>
// SourceSpan { pc: u32, line: u32, column: u16 }

// Build index: line -> (method_idx, pc)
fn build_breakpoint_index(module: &Module) -> HashMap<u32, Vec<(usize, u32)>> {
    let mut index: HashMap<u32, Vec<(usize, u32)>> = HashMap::new();
    for (method_idx, body) in module.method_bodies.iter().enumerate() {
        for span in &body.source_spans {
            index.entry(span.line).or_default().push((method_idx, span.pc));
        }
    }
    index
}

// Snap to nearest valid line (find smallest line >= requested_line)
fn snap_to_nearest(index: &HashMap<u32, Vec<(usize, u32)>>, requested: u32)
    -> Option<(u32, Vec<(usize, u32)>)> {
    let mut lines: Vec<u32> = index.keys().copied()
        .filter(|&l| l >= requested)
        .collect();
    lines.sort();
    lines.first().map(|&l| (l, index[&l].clone()))
}
```

**Multi-file note:** The current `SourceSpan` struct has no `file_id` field — it only stores `(pc, line, col)`. For multi-file projects, all source files are compiled into a single module and line numbers are file-relative. The breakpoint mapping must use the `source.path` from the DAP request to find the correct source file, but the `SourceSpan` table doesn't track file origins. For Phase 55 (single-file focus), this is fine. For multi-file support, a file_id→method range mapping would be needed. **Decision**: support single-file launch for Phase 55; multi-file is follow-on.

### Pattern 5: Call Stack Rendering

**What:** Convert `Task.call_stack` (Vec<CallFrame>) into DAP `StackFrame` objects with names and source locations.

**When to use:** On `stackTrace` request.

```rust
// Source: writ-module/src/module.rs MethodDef, SourceSpan
fn build_stack_frames(task: &Task, module: &Module,
                      source_path: &str) -> Vec<StackFrame> {
    task.call_stack.iter().rev().enumerate().map(|(frame_id, frame)| {
        // Resolve method name from MethodDef table
        let name = module.method_defs.get(frame.method_idx)
            .and_then(|m| read_string(&module.string_heap, m.name).ok())
            .unwrap_or("?")
            .to_string();

        // Look up source location at current PC
        let (line, col) = lookup_source_location_for_frame(module, frame);

        StackFrame {
            id: frame_id as i64,
            name,
            source: Some(Source {
                path: Some(source_path.to_string()),
                name: Some(source_path.split('/').last()
                    .unwrap_or(source_path).to_string()),
                ..Default::default()
            }),
            line: line as i64,
            column: col as i64,
            ..Default::default()
        }
    }).collect()
}
```

**Frame naming format decision:** Use just the method name (e.g., "greet"), not fully-qualified (e.g., "module::greet"). VS Code shows source file + line alongside the name, making the module prefix redundant. For extern frames (method_idx in extern range), show `[extern]`.

### Anti-Patterns to Avoid

- **Spawning a thread for VM execution:** The VM is synchronous, the dap crate is synchronous. No threads needed. Threads add complexity with no benefit here.
- **Using `tokio::main` like writ-lsp:** The `dap` crate uses `std::io::BufReader/BufWriter`, not tokio async streams. Do NOT use `#[tokio::main]`.
- **Caching breakpoints by DAP source path strings:** Paths from VS Code can be URIs or absolute paths; normalize to canonical path before storing.
- **Stopping on the same source line during StepOver/Into:** Must record the origin line and skip stops where `source_line == origin_line` and `method_idx == origin_method`.
- **Not sending `initialized` before accepting `setBreakpoints`:** DAP requires `initialized` event after `initialize` response, before configuration. Missing this blocks VS Code from sending breakpoints.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| DAP message framing (Content-Length headers) | Custom stdin parser | `dap::Server::poll_request()` | Content-Length framing is fiddly; easy to misparse multi-byte UTF-8 |
| DAP message sequence numbers | Manual counter | `dap::Server` auto-manages | Protocol requires monotonic seq; crate handles it |
| DAP request/response JSON schema | Custom serde structs | `dap::prelude::*` types | 42 request variants; getting the field names wrong breaks VS Code |
| Breakpoint ID management | Global counter | `BreakpointTable` in DebugHost | DAP requires stable numeric IDs per session; wrap into the table |

**Key insight:** The `dap` crate eliminates ~400 lines of protocol boilerplate. Even as alpha, it's far less risky than hand-rolled JSON parsing for a protocol with 42 request types.

---

## Common Pitfalls

### Pitfall 1: Stepping Stops on Same Source Line
**What goes wrong:** Step-over issues a `Stopped` event on the exact same line the user stepped from, causing the debugger to appear frozen.
**Why it happens:** Multiple instructions map to the same source line (e.g., function call setup instructions). `before_instruction` fires on all of them.
**How to avoid:** Record `(origin_line, origin_method)` when step begins. In `before_instruction`, only stop when `source_line != origin_line || method_idx != origin_method`.
**Warning signs:** Debugger stops but cursor doesn't move in the editor.

### Pitfall 2: Call Depth Not Available in `before_instruction`
**What goes wrong:** `StepOver` can't distinguish "still in the called function" from "returned to original frame" because `before_instruction` doesn't receive call depth.
**Why it happens:** The hook signature is `(task_id, method_idx, pc, line, col)` — no depth.
**How to avoid:** Track call depth via `on_function_enter` (+1) and `on_function_exit` (-1) hooks in `DebugHost`, stored in `HashMap<TaskId, usize>`. This is already called from dispatch/mod.rs (the `execute_ret` function fires `on_function_exit`).
**Warning signs:** Step-over descends into called functions instead of jumping over them.

### Pitfall 3: `dap` Crate Compile Failure
**What goes wrong:** `dap` 0.4.1-alpha1 fails to compile against Rust edition 2024 workspace.
**Why it happens:** Pre-release crate; edition 2024 has stricter lifetime and import rules.
**How to avoid:** Make crate validation the first task (Wave 0). If it fails: fall back to 60 hand-rolled structs using serde_json, covering only the 10 commands needed for Phase 55.
**Warning signs:** `cargo check -p writ-dap` fails with edition-related errors.

### Pitfall 4: Missing `initialized` Event
**What goes wrong:** VS Code sends `setBreakpoints` but the DAP server doesn't handle them because VS Code is waiting for `initialized` first.
**Why it happens:** DAP sequence: `initialize` request → adapter responds → adapter sends `initialized` event → client sends `setBreakpoints` → client sends `configurationDone` → client sends `launch`. If `initialized` is skipped, the client hangs.
**How to avoid:** After responding to `initialize`, immediately call `server.send_event(Event::Initialized)` before returning.
**Warning signs:** F5 hangs and VS Code shows no output in Debug Console.

### Pitfall 5: Responding to Step Commands Before VM Resumes
**What goes wrong:** DAP requires the adapter to respond to `next`/`stepIn`/`stepOut` immediately (ACK), then later send a `Stopped` event after stepping completes.
**Why it happens:** DAP is request-response; steps are not synchronous from the client's view.
**How to avoid:** In step command handlers: (1) respond with ACK, (2) set step mode in DebugHost, (3) call `runtime.resume_debug(task_id)`, (4) run tick loop until DebugSuspend, (5) send `Stopped` event.
**Warning signs:** VS Code shows "Debug Adapter timed out" error.

### Pitfall 6: Breakpoints Set Before `launch` (no module yet)
**What goes wrong:** VS Code sends `setBreakpoints` during configuration phase (before `launch`), but the module isn't compiled yet so SourceSpan lookup fails.
**Why it happens:** DAP protocol sends breakpoints before launch so the adapter can set them before execution starts.
**How to avoid:** Store breakpoints as "pending" during configuration phase (just record the file+line pairs). After `launch` compiles and loads the module, resolve all pending breakpoints against the SourceSpan table and send `Breakpoint` event updates to VS Code.
**Warning signs:** Breakpoints show as unverified (grey) and never trigger.

---

## Code Examples

### Minimal DAP Server Loop
```rust
// Source: dap 0.4.1-alpha1 lib.rs/crates/dap documentation
use dap::prelude::*;
use std::io::{BufReader, BufWriter};

fn main_loop(mut server: Server<impl BufRead, impl Write>) {
    loop {
        let req = match server.poll_request() {
            Ok(Some(r)) => r,
            Ok(None) => break, // EOF
            Err(e) => { eprintln!("DAP error: {e}"); break; }
        };

        match &req.command {
            Command::Initialize(_args) => {
                let rsp = req.success(ResponseBody::Initialize(Some(
                    types::Capabilities {
                        supports_configuration_done_request: Some(true),
                        ..Default::default()
                    }
                )));
                server.respond(rsp).unwrap();
                server.send_event(Event::Initialized).unwrap();
            }
            Command::Launch(_args) => {
                let rsp = req.success(ResponseBody::Launch);
                server.respond(rsp).unwrap();
                // ... compile and start VM ...
            }
            Command::Disconnect(_) => {
                let rsp = req.success(ResponseBody::Disconnect);
                server.respond(rsp).unwrap();
                break;
            }
            _ => {
                // Respond with error for unimplemented commands
                let rsp = req.error("not implemented");
                server.respond(rsp).unwrap();
            }
        }
    }
}
```

### Sending a Stopped Event
```rust
// Source: dap 0.4.1-alpha1 Event::Stopped and StoppedEventBody
use dap::events::StoppedEventBody;

fn emit_stopped(server: &mut Server<impl BufRead, impl Write>,
                reason: &str,
                thread_id: i64,
                bp_id: Option<u32>) {
    server.send_event(Event::Stopped(StoppedEventBody {
        reason: reason.to_string(),        // "breakpoint", "step", "pause"
        thread_id: Some(thread_id),
        all_threads_stopped: Some(true),   // single-task: all stopped
        hit_breakpoint_ids: bp_id.map(|id| vec![id as i64]),
        ..Default::default()
    })).unwrap();
}
```

### Threads Response (single task = single thread)
```rust
// Source: DAP protocol spec + dap crate types
use dap::types::Thread;
use dap::responses::ThreadsResponse;

fn handle_threads(req: &Request) -> Response {
    req.success(ResponseBody::Threads(ThreadsResponse {
        threads: vec![Thread {
            id: 1,    // task_id maps to thread_id 1 for single-task
            name: "main".to_string(),
        }],
    }))
}
```

### SourceSpan Lookup for Call Stack
```rust
// Source: writ-runtime/src/dispatch/mod.rs lookup_source_location() pattern
fn frame_source_location(module: &Module, frame: &CallFrame) -> (u32, u16) {
    if frame.method_idx >= module.method_bodies.len() {
        return (0, 0);
    }
    let spans = &module.method_bodies[frame.method_idx].source_spans;
    let pc = frame.pc as u32;
    let mut best = (0u32, 0u16);
    for span in spans {
        if span.pc <= pc {
            best = (span.line, span.column);
        } else {
            break; // spans sorted by pc
        }
    }
    best
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| DAP over TCP with attach | DAP over stdio with `debugAdapterExecutable` | VS Code 1.40+ | Extension declares binary path; VS Code launches it; no port management |
| Async DAP server (tokio) | Synchronous polling (`poll_request()`) | dap crate design | Simpler; no async needed since VM is also synchronous |
| Breakpoints set post-launch | Breakpoints sent pre-launch in configuration phase | DAP spec | Adapter must handle `setBreakpoints` before `launch` |

**Deprecated/outdated:**
- `debugAdapter.server` TCP transport: the modern pattern is `debugAdapterExecutable` (stdio). VS Code extension declares `"program": "./out/writ-dap"` in `contributes.debuggers`.

---

## Open Questions

1. **`run_pipeline` duplication**
   - What we know: `writ-cli/src/main.rs` has `run_pipeline()` as a private function. Both `writ-cli` and `writ-dap` need it.
   - What's unclear: Extract to `writ-compiler` as a pub fn, or duplicate into `writ-dap/src/launch.rs`?
   - Recommendation: Duplicate for now (~80 lines). Extraction to `writ-compiler` risks changing the crate's API surface. Deferred extraction is easy.

2. **Multi-file SourceSpan file identity**
   - What we know: `SourceSpan` has no file_id field. Multi-file projects compile all files into one module with file-relative line numbers only.
   - What's unclear: How does a breakpoint on `file_b.writ:10` map to a method compiled from that file?
   - Recommendation: Phase 55 supports single-file launch only. Document multi-file as requiring SourceSpan augmentation (adding a file_id field) — out of scope.

3. **`dap` crate Rust edition 2024 compatibility**
   - What we know: The crate is 0.4.1-alpha1 from September 2023. Rust edition 2024 was stabilized after this.
   - What's unclear: Whether the crate uses any patterns that are errors under edition 2024.
   - Recommendation: Wave 0 task: `cargo check -p writ-dap` with dap in Cargo.toml. If it fails, implement the 10 serde_json structs needed for Phase 55 scope (initialize, launch, configurationDone, setBreakpoints, threads, stackTrace, next, stepIn, stepOut, continue, disconnect) in `src/protocol.rs`.

---

## Validation Architecture

> nyquist_validation key is absent from .planning/config.json — treating as enabled.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | `writ-dap/Cargo.toml` (standard workspace member) |
| Quick run command | `cargo test -p writ-dap` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DAP-01 | Launch flow: compile .writ + load module + spawn entry task | unit | `cargo test -p writ-dap test_compile_and_load` | ❌ Wave 0 |
| DAP-01 | `initialize` → `initialized` event → `launch` sequence compiles | unit | `cargo test -p writ-dap test_initialize_sequence` | ❌ Wave 0 |
| DAP-02 | Breakpoint lookup: line 5 → correct method_idx+pc from SourceSpan | unit | `cargo test -p writ-dap test_breakpoint_lookup` | ❌ Wave 0 |
| DAP-02 | Snap to nearest valid line (no instruction at line 3 → snaps to 5) | unit | `cargo test -p writ-dap test_breakpoint_snap` | ❌ Wave 0 |
| DAP-03 | StepOver stops at next line in same function | unit | `cargo test -p writ-dap test_step_over` | ❌ Wave 0 |
| DAP-03 | StepInto descends into callee function | unit | `cargo test -p writ-dap test_step_into` | ❌ Wave 0 |
| DAP-03 | StepOut resumes until current frame returns | unit | `cargo test -p writ-dap test_step_out` | ❌ Wave 0 |
| DAP-05 | Call stack shows real function names + line numbers | unit | `cargo test -p writ-dap test_call_stack_frames` | ❌ Wave 0 |
| DAP-05 | Call depth tracking via on_function_enter/exit | unit | `cargo test -p writ-dap test_call_depth_tracking` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-dap`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-dap/Cargo.toml` — crate definition, deps validation (dap crate compile check)
- [ ] `writ-dap/src/main.rs` — binary entry point
- [ ] `writ-dap/src/lib.rs` — module declarations
- [ ] `writ-dap/tests/` — test directory with fixtures
- [ ] Workspace `Cargo.toml` — add `writ-dap` to members

---

## Sources

### Primary (HIGH confidence)
- `writ-runtime/src/host.rs` — `RuntimeHost` trait, `DebugAction`, `before_instruction` signature (read directly)
- `writ-runtime/src/task.rs` — `SuspendReason::Breakpoint/DebugStep` structure (read directly)
- `writ-runtime/src/dispatch/mod.rs` — `ExecutionResult::DebugSuspend`, `lookup_source_location()` pattern (read directly)
- `writ-runtime/src/scheduler.rs` — `DebugSuspend` handling, `resume_debug()` API (read directly)
- `writ-runtime/src/runtime.rs` — `Runtime::resume_debug()`, `RuntimeBuilder::with_host()` (read directly)
- `writ-module/src/module.rs` — `SourceSpan { pc, line, column }`, `MethodBody.source_spans` (read directly)
- `writ-cli/src/main.rs` — `run_pipeline()` 5-stage pattern (read directly)
- `writ-lsp/src/main.rs` — stdio binary pattern to mirror (read directly)
- [Microsoft DAP Specification](https://microsoft.github.io/debug-adapter-protocol/specification) — request/event/response contracts, stop reasons, launch flow sequence
- [docs.rs/dap 0.4.1-alpha1](https://docs.rs/dap/0.4.1-alpha1/dap/) — `Command` enum (42 variants), `ResponseBody` enum, `Event` enum (17 variants), `Server` API

### Secondary (MEDIUM confidence)
- [lib.rs/crates/dap](https://lib.rs/crates/dap) — version status (0.4.1-alpha1, September 2023), alpha warning
- [VS Code Debugger Extension Guide](https://code.visualstudio.com/api/extension-guides/debugger-extension) — `contributes.debuggers` package.json fields, `debugAdapterExecutable` pattern
- [deepwiki.com DAP Building Guide](https://deepwiki.com/microsoft/debug-adapter-protocol/4-building-a-debug-adapter) — launch flow, breakpoint flow, stepping flow sequences

### Tertiary (LOW confidence)
- STATE.md warning re: dap crate pre-release risk — confirmed independently via lib.rs

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — verified against docs.rs and crate source; runtime hooks read from actual code
- Architecture: HIGH — DebugHost pattern derived directly from existing RuntimeHost trait; DAP flows from official spec
- Pitfalls: HIGH — stepping call-depth issue derived from actual hook signature (before_instruction has no depth param); others from DAP spec and pre-release crate warning

**Research date:** 2026-03-14
**Valid until:** 2026-04-14 (30 days — dap crate is slow-moving; DAP spec is stable)
