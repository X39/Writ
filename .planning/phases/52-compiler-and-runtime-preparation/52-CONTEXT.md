# Phase 52: Compiler and Runtime Preparation - Context

**Gathered:** 2026-03-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix SourceSpan line numbers in compiled .writil (currently byte offsets, not line:col), add parser error recovery tolerance in downstream pipeline, add VM debug hooks for breakpoint/stepping control, add SuspendReason discriminant for DAP, and emit debug local variable info with types. This is foundational infrastructure — no LSP/DAP server code, just making the compiler and runtime debug-ready.

</domain>

<decisions>
## Implementation Decisions

### Error recovery (PREP-02)
- Parser already has declaration-level and statement-level recovery via chumsky (Expr::Error, Stmt::Error, balanced delimiter tracking)
- The real work is making the downstream pipeline (resolver, typechecker, codegen) tolerate Error nodes gracefully — skip them, don't crash, still produce diagnostics for valid code

### Debug hook design (PREP-03)
- `RuntimeHost` gets a `debug_enabled() -> bool` method, defaulting to `false`
- VM only calls debug hooks when `debug_enabled()` returns true — zero overhead in production
- Rich `before_instruction` hook receives source location data pre-resolved from SourceSpan tables:
  ```
  fn before_instruction(&mut self, task_id: TaskId, method_idx: u32, pc: u32, source_line: u32, source_col: u16) -> DebugAction
  ```
- Additional `on_function_enter(task_id, method_idx)` and `on_function_exit(task_id, method_idx)` hooks for call stack tracking
- All three debug methods have default no-op implementations
- `NullHost` and `CliHost` return `debug_enabled() = false` and are unaffected

### DebugAction responses (PREP-03/PREP-04)
- VM-managed stepping — DebugAction enum includes step variants:
  ```
  enum DebugAction { Continue, Break, StepOver, StepInto, StepOut, Disconnect }
  ```
- `StepOver`: break when source line changes at same or lower call depth
- `StepInto`: break when source line changes at any depth
- `StepOut`: break when current frame returns
- `Disconnect`: clear all step state, set debug_enabled=false, resume without debug overhead
- Stepping is source-line based — VM tracks "last stopped line" and breaks when line number changes (multiple IL instructions on same line are skipped)

### SuspendReason (PREP-04)
- Discriminant with context data — each variant carries location info:
  ```
  enum SuspendReason {
    HostRequest(RequestId),
    Breakpoint { method_idx: u32, pc: u32, line: u32, col: u16 },
    DebugStep { mode: DebugAction, method_idx: u32, pc: u32, line: u32, col: u16 },
  }
  ```
- Task.pending_request extended or SuspendReason added as separate field on Task
- DAP server can immediately report why execution paused without extra lookups

### Debug locals format (PREP-05)
- DebugLocal extended with TypeRef blob index for type info:
  ```
  DebugLocal { register: u16, name: u32, type_ref: u32, start_pc: u32, end_pc: u32 }
  ```
- All registers emitted including temporaries (synthetic names like `$tmp_0`) — DAP can filter for display
- Precise variable scoping — each local's start_pc/end_pc reflects actual scope boundaries, not full method span
- Emitter must track when variables enter/exit scope during codegen

### Disassembler output (PREP-05)
- `.locals` section at top of each method shows register, name, type, and scope range
- Inline type annotations on instruction operands as comments: `LOAD_I32 r0, 42  ; x: int`

### Claude's Discretion
- How to propagate source text line offsets through the pipeline for SourceSpan fix (PREP-01)
- Exact mechanism for downstream pipeline Error-node tolerance (PREP-02)
- Internal step-tracking state machine design in the VM
- How SuspendReason integrates with existing Task struct fields

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `writ-compiler/src/emit/body/debug.rs`: emit_debug_locals and emit_source_spans — these are the exact files to modify for PREP-01 and PREP-05
- `writ-module/src/module.rs`: DebugLocal and SourceSpan structs — need type_ref field added to DebugLocal
- `writ-runtime/src/host.rs`: RuntimeHost trait — add debug_enabled, before_instruction, on_function_enter, on_function_exit
- `writ-runtime/src/task.rs`: Task struct — add SuspendReason field or extend pending_request
- `writ-parser/src/parser.rs`: Already has Expr::Error/Stmt::Error recovery nodes via chumsky

### Established Patterns
- RuntimeHost trait uses default method implementations (on_gc_complete already has a default no-op)
- Module format uses u32 offsets into string heap and TypeRef blob table
- Task state machine uses TaskState enum with Ready/Running/Suspended/Completed/Cancelled

### Integration Points
- `writ-compiler/src/emit/serialize.rs`: Where DebugLocal name offsets are resolved — needs to also resolve type_ref
- `writ-runtime/src/runtime.rs` and `writ-runtime/src/scheduler.rs`: VM execution loop where before_instruction hook must be inserted
- `writ-assembler/src/assembler.rs` and disassembler: Need to handle new DebugLocal format and inline type annotations
- `writ-module/src/reader.rs` and `writ-module/src/writer.rs`: Binary format changes for extended DebugLocal

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 52-compiler-and-runtime-preparation*
*Context gathered: 2026-03-13*
