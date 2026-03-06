# Phase 52: Compiler and Runtime Preparation - Research

**Researched:** 2026-03-13
**Domain:** Rust compiler internals — source position tracking, parser error recovery pipeline tolerance, VM debug hooks, binary format extension
**Confidence:** HIGH (all findings based on direct source code inspection of the Writ workspace)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Error recovery (PREP-02)**
- Parser already has declaration-level and statement-level recovery via chumsky (Expr::Error, Stmt::Error, balanced delimiter tracking)
- The real work is making the downstream pipeline (resolver, typechecker, codegen) tolerate Error nodes gracefully — skip them, don't crash, still produce diagnostics for valid code

**Debug hook design (PREP-03)**
- `RuntimeHost` gets a `debug_enabled() -> bool` method, defaulting to `false`
- VM only calls debug hooks when `debug_enabled()` returns true — zero overhead in production
- Rich `before_instruction` hook:
  ```
  fn before_instruction(&mut self, task_id: TaskId, method_idx: u32, pc: u32, source_line: u32, source_col: u16) -> DebugAction
  ```
- Additional `on_function_enter(task_id, method_idx)` and `on_function_exit(task_id, method_idx)` hooks
- All three debug methods have default no-op implementations
- `NullHost` and `CliHost` return `debug_enabled() = false` and are unaffected

**DebugAction responses (PREP-03/PREP-04)**
- VM-managed stepping:
  ```
  enum DebugAction { Continue, Break, StepOver, StepInto, StepOut, Disconnect }
  ```
- `StepOver`: break when source line changes at same or lower call depth
- `StepInto`: break when source line changes at any depth
- `StepOut`: break when current frame returns
- `Disconnect`: clear all step state, set debug_enabled=false, resume without debug overhead
- Stepping is source-line based — VM tracks "last stopped line" and breaks when line number changes

**SuspendReason (PREP-04)**
- Discriminant with context data:
  ```
  enum SuspendReason {
    HostRequest(RequestId),
    Breakpoint { method_idx: u32, pc: u32, line: u32, col: u16 },
    DebugStep { mode: DebugAction, method_idx: u32, pc: u32, line: u32, col: u16 },
  }
  ```
- Task.pending_request extended or SuspendReason added as separate field on Task
- DAP server can immediately report why execution paused without extra lookups

**Debug locals format (PREP-05)**
- DebugLocal extended with TypeRef blob index for type info:
  ```
  DebugLocal { register: u16, name: u32, type_ref: u32, start_pc: u32, end_pc: u32 }
  ```
- All registers emitted including temporaries (synthetic names like `$tmp_0`)
- Precise variable scoping — each local's start_pc/end_pc reflects actual scope boundaries
- Emitter must track when variables enter/exit scope during codegen

**Disassembler output (PREP-05)**
- `.locals` section at top of each method shows register, name, type, and scope range
- Inline type annotations on instruction operands as comments: `LOAD_I32 r0, 42  ; x: int`

### Claude's Discretion
- How to propagate source text line offsets through the pipeline for SourceSpan fix (PREP-01)
- Exact mechanism for downstream pipeline Error-node tolerance (PREP-02)
- Internal step-tracking state machine design in the VM
- How SuspendReason integrates with existing Task struct fields

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PREP-01 | Compiled .writil debug info contains real line/column numbers (not byte offsets) for all source spans | Source text must flow to `emit_bodies`; `build_source_spans` in `serialize.rs` needs a line-offset table computed from source bytes |
| PREP-02 | Parser produces useful partial ASTs from incomplete or syntactically broken source files (error recovery) | Parser already emits Error nodes; `emit_all_bodies` in `emit/mod.rs` and `emit/body/mod.rs` currently aborts entirely on any error node — need function-granular skip instead |
| PREP-03 | RuntimeHost trait has a `before_instruction` hook that allows debug breakpoint and stepping control | `host.rs` RuntimeHost trait — add three default methods + `debug_enabled()` guard in `execute_one()` |
| PREP-04 | Task distinguishes DAP debug suspension from host-request suspension via a SuspendReason discriminant | `task.rs` Task struct has `pending_request: Option<(RequestId, HostRequest)>` — add `suspend_reason: Option<SuspendReason>` field alongside |
| PREP-05 | Compiled .writil includes debug local variable info (register index → variable name + type) per function | `DebugLocal` in `writ-module/src/module.rs` needs `type_ref: u32` field; binary format serialization in reader/writer needs 4 extra bytes per entry |
</phase_requirements>

---

## Summary

Phase 52 is a targeted infrastructure phase: five focused changes to the compiler and runtime that have no visible user features of their own but are required foundations for every LSP and DAP feature in phases 53-57. All five requirements address known confirmed deficiencies — not speculative gaps. The codebase is well-structured for these changes; each requirement maps to one or two specific files.

The most architecturally interesting requirement is PREP-01 (source spans). The root cause is well-understood: `build_source_spans` in `writ-compiler/src/emit/serialize.rs` (line 474-492) correctly converts instruction indices to byte offsets but hardcodes `line: 0, column: 0`. The missing ingredient is a line-offset table computed from the source text, which is available in the CLI's `run_pipeline` but is not currently threaded through to `emit_bodies`. The fix is a small, non-breaking signature extension.

PREP-02 is also a real gap: `emit_all_bodies` currently aborts the entire compilation if *any* error node exists anywhere (checked by `has_error_nodes`). The LSP needs per-function resilience — a function with a syntax error in its body should not prevent other functions from being compiled. The change is to move error detection from file-level to function-level in `emit_all_bodies`.

PREP-03, PREP-04, and PREP-05 are additive changes (new trait methods with defaults, a new enum, a new struct field). They require careful binary format versioning for PREP-05 since `DebugLocal` grows by 4 bytes.

**Primary recommendation:** Work bottom-up: PREP-05 (module format) first since it affects binary layout, then PREP-01 (source spans use the same debug info path), then PREP-02 (pipeline tolerance), then PREP-03 and PREP-04 together (they share the DebugAction/SuspendReason types).

---

## Standard Stack

### Core

| Component | Location | Purpose | Notes |
|-----------|----------|---------|-------|
| chumsky SimpleSpan | `chumsky::span::SimpleSpan` | Byte-offset span type carried on all CST/AST/TypedAST nodes | `.start` and `.end` are byte offsets into the source string |
| writ-module DebugLocal | `writ-module/src/module.rs:35-40` | Binary format struct for per-register debug info | Currently 14 bytes: u16 + u32 + u32 + u32 — needs `type_ref: u32` to become 18 bytes |
| writ-module SourceSpan | `writ-module/src/module.rs:43-48` | Binary format struct for PC-to-source-location mapping | `line` currently stores byte offset (known bug); fix: store 1-based line number |
| writ-runtime RuntimeHost | `writ-runtime/src/host.rs:93-102` | Trait for all host implementations | `on_gc_complete` already uses default no-op pattern — use same for debug methods |
| writ-runtime Task | `writ-runtime/src/task.rs:20-33` | Per-task execution state | Has `pending_request: Option<(RequestId, HostRequest)>` — SuspendReason field goes alongside |
| execute_one | `writ-runtime/src/dispatch/mod.rs:186` | Single-instruction dispatch function | Location for debug hook insertion |

### Supporting

| Component | Location | Purpose | Notes |
|-----------|----------|---------|-------|
| build_source_spans | `writ-compiler/src/emit/serialize.rs:474` | Converts (instr_idx, SimpleSpan) to SourceSpan | Currently hardcodes line=0; fix: accept a line-offset table |
| build_debug_locals | `writ-compiler/src/emit/serialize.rs:432` | Builds DebugLocal entries with string heap interning | Needs `type_ref` resolver plumbed in |
| emit_all_bodies | `writ-compiler/src/emit/body/mod.rs:370` | Iterates all decls and emits bodies | Current error pre-pass is file-level; needs function-level skip |
| has_error_nodes | `writ-compiler/src/emit/body/mod.rs:184` | Scans TypedAst for any Error nodes | Use same logic per-function, not globally |
| disassemble_body | `writ-assembler/src/disassembler.rs:496` | Renders a method body to .writil text | Needs `.locals` section and inline type annotations |

---

## Architecture Patterns

### Recommended Execution Order

The five requirements have dependencies:

```
PREP-05 (DebugLocal format)
  └─► PREP-01 (SourceSpan line numbers — same debug info emission path)
        └─► disassembler update (shows .locals and line:col comments)

PREP-02 (pipeline error tolerance)
  [independent of above]

PREP-03 (RuntimeHost debug hooks) + PREP-04 (SuspendReason)
  [define shared types DebugAction and SuspendReason together]
```

Recommend: PREP-05 → PREP-01 → disassembler → PREP-02 → PREP-03+PREP-04.

### Pattern 1: Source Text Line-Offset Table (PREP-01)

**What:** Convert byte offsets (from SimpleSpan) to 1-based line/column numbers using a pre-computed table.

**How it works:** Given source text, scan once for `\n` positions to build a sorted `Vec<u32>` of newline byte offsets. Then for any byte offset `b`, binary-search to find the preceding newline count (= 0-based line index), and subtract that newline's position from `b` to get the column.

**Where to add it:** `writ-compiler/src/emit/serialize.rs` or a new `writ-compiler/src/emit/line_map.rs`. The function must be called from `build_source_spans`.

**Where to thread the source text:** `emit_bodies` in `writ-compiler/src/emit/mod.rs` currently receives `asts: &[(FileId, &Ast)]` but not source text. Add `sources: &[(FileId, &str)]` parameter (parallel to asts). The CLI's `run_pipeline` already has `file_sources` containing `(FileId, display_path, &'static str)` — use the `&str` component.

**Example pattern:**
```rust
// Source: direct inspection of writ-compiler/src/emit/serialize.rs + standard Rust pattern
/// Build a line-offset table from source text.
/// Returns sorted Vec of byte offsets where each '\n' ends a line.
/// line_starts[0] = 0 (start of line 1), line_starts[n] = offset after nth newline.
fn build_line_starts(src: &str) -> Vec<u32> {
    let mut starts = vec![0u32];
    for (i, ch) in src.char_indices() {
        if ch == '\n' {
            starts.push(i as u32 + 1);
        }
    }
    starts
}

/// Convert a byte offset to (1-based line, 1-based column).
fn byte_offset_to_line_col(offset: u32, line_starts: &[u32]) -> (u32, u16) {
    let line_idx = line_starts.partition_point(|&s| s <= offset).saturating_sub(1);
    let col = (offset - line_starts[line_idx]) + 1; // 1-based
    ((line_idx as u32) + 1, col as u16)             // 1-based line
}
```

**Note:** `chumsky::span::SimpleSpan` stores byte offsets, not char offsets — this is consistent with how ariadne renders diagnostics (it uses the same byte-offset approach with `ariadne::Source`).

### Pattern 2: Function-Granular Error-Node Skip (PREP-02)

**What:** Instead of aborting codegen when any error node exists, skip only the specific function body that contains error nodes. Other functions compile normally and produce output.

**Current code (aborts all):** `emit_all_bodies` in `emit/body/mod.rs:370`:
```rust
// Source: writ-compiler/src/emit/body/mod.rs:377-387
if has_error_nodes(typed_ast) {
    diags.push(Diagnostic::error("E9000", "Codegen aborted...").build());
    return (Vec::new(), diags);
}
```

**New pattern (skip per-function):**
```rust
// Source: direct code review of emit/body/mod.rs
// Remove the file-level pre-pass. Instead, at each function/method emission site:
TypedDecl::Fn { def_id, body } => {
    // Skip bodies with error nodes — emit diagnostic but continue to next function
    if expr_has_error(body) {
        diags.push(Diagnostic::error("E9001", "Skipping function body due to errors").build());
        continue; // not 'return' — keep going
    }
    // ... normal emission ...
}
```

**Resolver and typechecker already handle error nodes gracefully** — `TypedExpr::Error` and `TypedStmt::Error` are already produced by `check_expr.rs` and `check_stmt.rs` without aborting. The codegen is the only layer that currently aborts on them.

**Impact on `emit_bodies` in `emit/mod.rs`:** The `has_error_nodes` pre-pass in `emit_bodies` (line 77) must also be removed or made function-granular. `emit_all_bodies` becoming tolerant is sufficient — the caller in `emit/mod.rs` can pass on whatever bodies it produced.

### Pattern 3: RuntimeHost Default Method Pattern (PREP-03)

**What:** Add new methods with default no-op implementations — matching the existing `on_gc_complete` pattern.

**Current pattern (from host.rs:101):**
```rust
// Source: writ-runtime/src/host.rs:101
fn on_gc_complete(&mut self, _stats: &GcStats) {}
```

**New additions:**
```rust
// Source: decision from 52-CONTEXT.md + host.rs pattern
fn debug_enabled(&self) -> bool { false }

fn before_instruction(
    &mut self,
    _task_id: TaskId,
    _method_idx: u32,
    _pc: u32,
    _source_line: u32,
    _source_col: u16,
) -> DebugAction { DebugAction::Continue }

fn on_function_enter(&mut self, _task_id: TaskId, _method_idx: u32) {}
fn on_function_exit(&mut self, _task_id: TaskId, _method_idx: u32) {}
```

**Guard in execute_one (dispatch/mod.rs):**
```rust
// Source: direct inspection of execute_one in dispatch/mod.rs:186
// Insert before the instruction fetch:
if host.debug_enabled() {
    let (line, col) = lookup_source_location(module, method_idx, frame.pc);
    match host.before_instruction(task.id, method_idx as u32, frame.pc as u32, line, col) {
        DebugAction::Continue => {},
        DebugAction::Break | DebugAction::StepOver | ... => {
            // suspend task with SuspendReason::Breakpoint or DebugStep
            task.suspend_reason = Some(SuspendReason::Breakpoint { ... });
            task.state = TaskState::Suspended;
            return ExecutionResult::Suspended(RequestId(0)); // synthetic
        }
        DebugAction::Disconnect => {
            // clear debug state and proceed
        }
    }
}
```

**Source location lookup helper:** Given a module, method_idx, and pc (byte offset), scan `module.method_bodies[method_idx].source_spans` for the largest `span.pc <= pc`. O(n) scan is fine for debug mode — can use binary search after sorting if needed.

### Pattern 4: SuspendReason Field on Task (PREP-04)

**What:** Add `suspend_reason: Option<SuspendReason>` to Task, set it whenever the task transitions to Suspended, clear it on resume.

**Current Task struct (task.rs:20-33):**
```rust
// Source: writ-runtime/src/task.rs:20-33
pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
    pub call_stack: Vec<CallFrame>,
    pub parent_id: Option<TaskId>,
    pub scoped_children: Vec<TaskId>,
    pub pending_request: Option<(RequestId, HostRequest)>,  // existing
    pub return_value: Option<Value>,
    pub crash_info: Option<CrashInfo>,
    pub atomic_depth: u32,
    pub instructions_executed: u64,
    pub suspend_count: u32,
    pub atomic_locks: Vec<u32>,
}
```

**Addition:**
```rust
pub suspend_reason: Option<SuspendReason>, // new field, set alongside state = Suspended
```

**Placement note:** `pending_request` is already `Option<(RequestId, HostRequest)>`. Keep it. The `SuspendReason::HostRequest(RequestId)` variant provides the DAP-visible discriminant — it wraps the same RequestId that's in pending_request. When setting `state = Suspended` for a host request, also set `suspend_reason = Some(SuspendReason::HostRequest(req_id))`. On resume (`confirm`), clear both.

### Pattern 5: Binary Format Extension for DebugLocal (PREP-05)

**What:** Add `type_ref: u32` field to `DebugLocal` struct. Update reader, writer, and all construction sites.

**Current binary layout (writer.rs:209-215):**
```
register: u16    (2 bytes)
name:     u32    (4 bytes)
start_pc: u32    (4 bytes)
end_pc:   u32    (4 bytes)
Total: 14 bytes per DebugLocal
```

**New binary layout:**
```
register:  u16   (2 bytes)
name:      u32   (4 bytes)
type_ref:  u32   (4 bytes)   ← new
start_pc:  u32   (4 bytes)
end_pc:    u32   (4 bytes)
Total: 18 bytes per DebugLocal
```

**Files to update:**
1. `writ-module/src/module.rs` — Add `type_ref: u32` to `DebugLocal` struct
2. `writ-module/src/writer.rs` — Write the new field; update `compute_body_size` (14 → 18 bytes)
3. `writ-module/src/reader.rs` — Read the new field
4. `writ-compiler/src/emit/serialize.rs` — `build_debug_locals` must now populate `type_ref` from register type blobs (the blob heap offsets from `register_types`)
5. `writ-assembler/src/disassembler.rs` — `disassemble_body` must emit `.locals` section and inline type comments

**Version note:** The module format version is currently 3 (set in `module.rs:87` and `serialize.rs:349`). Since DebugLocal layout changes are only present when `flags & 1` (debug info flag is set), and since the module is not a stable ABI for external consumers yet, incrementing to format_version 4 is the correct approach. All existing golden test `.writc` files must be rebless after this change.

### Anti-Patterns to Avoid

- **Do NOT pass source text as `'static`** inside the compile pipeline just to thread it to emit_bodies. The source is already available in `run_pipeline`'s `file_sources` — pass it through the normal way.
- **Do NOT implement source location lookup by scanning all source spans on every instruction in the hot path.** The lookup only runs when `debug_enabled()` is true. In production (NullHost/CliHost), there is zero overhead.
- **Do NOT add `type_ref` to the DebugLocal binary format without bumping `format_version`.** Golden tests will silently pass incorrect bytes otherwise.
- **Do NOT remove `pending_request` from Task.** It is used by `runtime.rs:confirm()` to deliver host responses. `SuspendReason` is a separate field that tells the DAP why the task suspended — it does not replace the request mechanism.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Byte offset to line/col conversion | Custom tokenizer walk | Pre-computed `Vec<u32>` line-starts + `partition_point` binary search | O(n) pre-computation, O(log n) per lookup; fits in <10 lines |
| Source span table lookup at debug time | Custom interval tree | Linear scan of `source_spans` (already sorted by pc) or `partition_point` on sorted vec | At most ~100 spans per method; overhead only matters in debug mode |
| Error node detection per function | New AST visitor | Reuse existing `expr_has_error` / `stmt_has_error` from `emit/body/mod.rs:211` | Already correct and complete; just call it per-function instead of globally |
| Type text in disassembler | New type decoder | Reuse existing `decode_type_sig` in `disassembler.rs` | Already handles all TypeRef blob formats |

---

## Common Pitfalls

### Pitfall 1: Format Version Not Bumped After DebugLocal Extension
**What goes wrong:** Existing `.writc` files are decoded with the new reader that expects 18 bytes per DebugLocal but gets 14 — deserialization produces garbage or panics.
**Why it happens:** `format_version` in the module header is checked on read, but the binary layout of method bodies is not versioned separately — the flag byte (`flags & 1`) is the only discriminant for debug info presence.
**How to avoid:** Bump `format_version` from 3 to 4 in `Module::new()` (module.rs) and `serialize.rs`. Add a check in `reader.rs` that reads DebugLocal in 14-byte format for v3 and 18-byte format for v4+. Re-bless all golden `.writc` files.
**Warning signs:** `adv_atomic.writil` golden diff shows registers with wrong names or wrong type annotations.

### Pitfall 2: source_spans Vec is Empty (No Spans Are Actually Pushed)
**What goes wrong:** PREP-01 fix writes correct line/column conversion code but `source_spans` is empty in all methods, so no SourceSpan entries appear in disasm output.
**Why it happens:** `BodyEmitter.source_spans` is initialized as `Vec::new()` (mod.rs:105) and nothing in `expr.rs` or `stmt.rs` currently pushes to it. The `build_source_spans` function (serialize.rs:474) maps over whatever is in that vec.
**How to avoid:** As part of PREP-01, add source span push calls in `emit_expr` (at minimum at statement boundaries in `emit_stmt`, and for each expression that represents a distinct source location). Start with one span per statement.
**Warning signs:** `writ disasm` shows method bodies with no source location comments at all.

### Pitfall 3: SuspendReason::HostRequest Confusion with DAP
**What goes wrong:** The DAP server queries `suspend_reason` for a HostRequest suspension and gets `None` because the existing `confirm()` path in `runtime.rs` sets `state = Suspended` but never sets `suspend_reason`.
**Why it happens:** All existing suspension sites in `dispatch/mod.rs` that emit `ExecutionResult::Suspended(req_id)` predate `SuspendReason`. The scheduler in `scheduler.rs:130-134` calls `task.state = Suspended` without knowing about the new field.
**How to avoid:** When `scheduler.run_one_task` handles `ExecutionResult::Suspended(req_id)`, set `task.suspend_reason = Some(SuspendReason::HostRequest(req_id))` at the same site. When `runtime.confirm()` resumes the task, clear `task.suspend_reason = None`.
**Warning signs:** `task.suspend_reason` is always None for all suspension types.

### Pitfall 4: Breaking NullHost/CliHost Compilation
**What goes wrong:** Adding `debug_enabled() -> bool` and `before_instruction(...)` to the `RuntimeHost` trait without default implementations forces `NullHost` and `CliHost` to implement them or fail to compile.
**Why it happens:** Rust traits require all non-default methods to be implemented.
**How to avoid:** ALL three new debug methods (`debug_enabled`, `before_instruction`, `on_function_enter`, `on_function_exit`) must have default implementations in the trait definition. The locked decision explicitly states this — verify all four have defaults before compiling.
**Warning signs:** Compiler error `E0277: NullHost doesn't implement RuntimeHost` after adding trait methods.

### Pitfall 5: Column Calculation Off-By-One
**What goes wrong:** Disassembled source locations show column 0 or column 1 less than the actual source position.
**Why it happens:** `SimpleSpan.start` is a byte offset from start-of-file; converting to column requires subtracting the byte offset of the current line's start. If `line_starts` includes position 0 correctly and column is computed as `offset - line_start[line_idx] + 1`, results are 1-based correctly. Forgetting the `+1` makes it 0-based.
**How to avoid:** Use 1-based output for both line and column (matching LSP/DAP expectations). Test with a known source: `fn main() {` — `main` starts at col 4 (1-based), byte offset 3. Verify `byte_offset_to_line_col(3, &line_starts)` returns `(1, 4)`.
**Warning signs:** Golden test `.writil` shows `// line:1 col:0` for first-token-on-line cases.

---

## Code Examples

### Line-Offset Table Construction

```rust
// Source: direct inspection of writ-compiler/src/emit/serialize.rs:474-492 (existing gap)
// Standard Rust pattern — no external crate needed

/// Build sorted table of byte offsets for line starts (line_starts[0] = 0).
pub fn build_line_starts(src: &str) -> Vec<u32> {
    let mut starts = vec![0u32];
    for (i, b) in src.as_bytes().iter().enumerate() {
        if *b == b'\n' {
            starts.push(i as u32 + 1);
        }
    }
    starts
}

/// Convert byte offset to (1-based line, 1-based column).
pub fn byte_offset_to_line_col(offset: u32, line_starts: &[u32]) -> (u32, u16) {
    let line_idx = line_starts.partition_point(|&s| s <= offset).saturating_sub(1);
    let line = line_idx as u32 + 1;
    let col = (offset.saturating_sub(line_starts[line_idx])) + 1;
    (line, col.min(u16::MAX as u32) as u16)
}
```

### Updated build_source_spans

```rust
// Source: based on writ-compiler/src/emit/serialize.rs:474-492 (the function to fix)
fn build_source_spans(
    source_spans: &[(u32, chumsky::span::SimpleSpan)],
    instr_byte_starts: &[usize],
    line_starts: &[u32],  // new parameter
) -> Vec<SourceSpan> {
    source_spans
        .iter()
        .map(|(instr_idx, span)| {
            let byte_offset = instr_byte_starts
                .get(*instr_idx as usize)
                .copied()
                .unwrap_or(0) as u32;
            let (line, col) = byte_offset_to_line_col(span.start as u32, line_starts);
            SourceSpan { pc: byte_offset, line, column: col }
        })
        .collect()
}
```

### Updated DebugLocal Struct

```rust
// Source: based on writ-module/src/module.rs:35-40 (the struct to extend)
#[derive(Debug, Clone, PartialEq)]
pub struct DebugLocal {
    pub register: u16,
    pub name: u32,      // string heap offset
    pub type_ref: u32,  // blob heap offset (NEW — was absent in v3 format)
    pub start_pc: u32,
    pub end_pc: u32,
}
```

### Updated writer.rs DebugLocal serialization

```rust
// Source: based on writ-module/src/writer.rs:209-215 (the write site)
for local in &body.debug_locals {
    out.write_u16::<LittleEndian>(local.register)?;
    out.write_u32::<LittleEndian>(local.name)?;
    out.write_u32::<LittleEndian>(local.type_ref)?;  // new
    out.write_u32::<LittleEndian>(local.start_pc)?;
    out.write_u32::<LittleEndian>(local.end_pc)?;
}
// compute_body_size: 18 bytes per DebugLocal (was 14)
```

### DebugAction and SuspendReason Enum Definitions

```rust
// Source: writ-runtime/src/host.rs (add DebugAction alongside DebugAction)
// Source: writ-runtime/src/task.rs (add SuspendReason)
// Placement: define DebugAction in host.rs (alongside RuntimeHost); SuspendReason in task.rs

// In host.rs:
pub enum DebugAction {
    Continue,
    Break,
    StepOver,
    StepInto,
    StepOut,
    Disconnect,
}

// In task.rs:
pub enum SuspendReason {
    HostRequest(RequestId),
    Breakpoint { method_idx: u32, pc: u32, line: u32, col: u16 },
    DebugStep { mode: DebugAction, method_idx: u32, pc: u32, line: u32, col: u16 },
}
```

### Disassembler .locals Section

```rust
// Source: based on writ-assembler/src/disassembler.rs:496-546 (disassemble_body)
// Insert before the register declarations loop:

// Emit .locals section (per-variable debug info, if any named locals present)
let named_locals: Vec<_> = body.debug_locals.iter()
    .filter(|dl| dl.name != 0)  // skip unnamed/tmp registers
    .collect();
if !named_locals.is_empty() {
    writeln!(out, "{}.locals {{", indent).unwrap();
    for dl in &named_locals {
        let name = read_string(&module.string_heap, dl.name).unwrap_or("?");
        let type_text = if dl.type_ref != 0 {
            decode_type_sig(&module.blob_heap, dl.type_ref, module)
        } else {
            "?".to_string()
        };
        writeln!(out, "{}    r{}: {} \"{}\" [{}, {})",
            indent, dl.register, type_text, name, dl.start_pc, dl.end_pc).unwrap();
    }
    writeln!(out, "{}}}", indent).unwrap();
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `span.start` stored as `line` field | Will store real 1-based line number | Phase 52 (PREP-01) | Enables LSP hover/go-to-def and DAP breakpoint resolution |
| Whole-file codegen abort on any error | Per-function skip of error bodies | Phase 52 (PREP-02) | Enables LSP to show diagnostics for error function while keeping completions/types for others |
| No debug hooks in VM | `before_instruction` + step state machine | Phase 52 (PREP-03) | Enables DAP breakpoint/stepping without polling |
| All suspensions look identical | `SuspendReason` discriminant on Task | Phase 52 (PREP-04) | DAP can immediately distinguish breakpoints from dialogue say() waits |
| DebugLocal has name only | DebugLocal has name + type_ref | Phase 52 (PREP-05) | DAP variables panel can show `x: int = 42` instead of `r0 = 42` |

**Deprecated/outdated:**
- `emit_source_spans` in `debug.rs` (line 56-66): This function was the original Phase 25 placeholder that directly stored `span.start` as line. It is no longer called — `build_source_spans` in `serialize.rs` replaced it. The function in `debug.rs` can be removed in Phase 52 to avoid confusion.
- `has_error_nodes` global pre-pass in `emit_all_bodies`: Useful as a function-level utility but wrong as a file-level abort — the Phase 52 PREP-02 change makes this a per-function check.

---

## Open Questions

1. **Should source spans be emitted for every instruction or only statement boundaries?**
   - What we know: `build_source_spans` maps whatever is in `BodyEmitter.source_spans`. Currently nothing pushes to it.
   - What's unclear: Whether PREP-01 requires spans for every instruction (useful for instruction-level stepping) or only statement starts (simpler, sufficient for line-level stepping).
   - Recommendation: Emit one span per statement (per `emit_stmt` call) for Phase 52. Per-expression spans are a Phase 56 concern (DAP variable values).

2. **Does the step-tracking state machine need to live on Task or on RuntimeHost?**
   - What we know: Decisions say "VM-managed stepping" with DebugAction variants.
   - What's unclear: Whether the "last stopped line" and call depth tracking lives in a field on Task or in a field added to a debug-aware RuntimeHost subtype.
   - Recommendation: Add `debug_step_state: Option<StepState>` to Task where `StepState { mode: DebugAction, stopped_line: u32, stopped_depth: usize }`. This keeps the scheduler/dispatcher logic self-contained without needing the host to track per-task state.

3. **Do golden .writil files need to show source span info?**
   - What we know: Currently disasm output has no source location comments. PREP-05 decisions describe `.locals` section and inline type annotations but are silent on source spans.
   - What's unclear: Whether disasm should show `// line:5 col:3` comments per instruction.
   - Recommendation: Emit source location as an end-of-line comment (`; line:5 col:3`) on instructions that have a SourceSpan entry. Use the verbose disasm path. The golden tests use the non-verbose path — so this does not affect golden test re-blessing.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness + cargo test |
| Config file | none (workspace Cargo.toml) |
| Quick run command | `cargo test -p writ-module -p writ-compiler -p writ-runtime -p writ-assembler 2>&1` |
| Full suite command | `cargo test --workspace 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PREP-01 | `writ disasm` shows 1-based line/col for each instruction's SourceSpan | unit | `cargo test -p writ-compiler source_span` | ❌ Wave 0 |
| PREP-01 | Line-offset table converts byte offsets correctly | unit | `cargo test -p writ-compiler line_map` | ❌ Wave 0 |
| PREP-02 | File with syntax error in function A still emits bodies for function B | unit | `cargo test -p writ-compiler error_recovery` | ❌ Wave 0 |
| PREP-03 | `RuntimeHost::debug_enabled()` defaults to false; NullHost/CliHost compile unmodified | unit | `cargo test -p writ-runtime debug_hook` | ❌ Wave 0 |
| PREP-03 | `before_instruction` hook receives correct task_id, method_idx, pc, line, col | integration | `cargo test -p writ-runtime before_instruction` | ❌ Wave 0 |
| PREP-04 | Task has `suspend_reason: None` after construction | unit | `cargo test -p writ-runtime suspend_reason` | ❌ Wave 0 |
| PREP-04 | Host-request suspension sets `SuspendReason::HostRequest` | integration | `cargo test -p writ-runtime suspend_host_request` | ❌ Wave 0 |
| PREP-04 | Breakpoint suspension sets `SuspendReason::Breakpoint` | integration | `cargo test -p writ-runtime suspend_breakpoint` | ❌ Wave 0 |
| PREP-05 | DebugLocal round-trips through writer/reader with type_ref field | unit | `cargo test -p writ-module debug_local_roundtrip` | ❌ Wave 0 |
| PREP-05 | `writ disasm` output includes `.locals` section with register names and types | golden | `cargo test -p writ-golden` (rebless required) | ✅ (needs rebless) |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-module -p writ-compiler -p writ-runtime --lib 2>&1`
- **Per wave merge:** `cargo test --workspace 2>&1`
- **Phase gate:** Full suite green + golden tests reblessed before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `writ-compiler/src/emit/line_map.rs` — `build_line_starts` + `byte_offset_to_line_col` functions with unit tests covering edge cases (empty file, single line, CRLF, multi-byte UTF-8 characters)
- [ ] `writ-compiler/src/emit/tests/source_span_tests.rs` (or inline `#[cfg(test)]`) — tests that `build_source_spans` maps to correct 1-based line/col after PREP-01 fix
- [ ] `writ-compiler/src/emit/tests/error_recovery_tests.rs` — tests that compiling a two-function file with one broken function produces output for the working function
- [ ] `writ-runtime/src/tests/debug_hook_tests.rs` — tests for `debug_enabled`, `before_instruction` default behavior, and that NullHost still compiles
- [ ] `writ-runtime/src/tests/suspend_reason_tests.rs` — tests for SuspendReason field on Task
- [ ] `writ-module/src/tests/debug_local_v4_roundtrip.rs` — writer/reader round-trip test for `DebugLocal` with `type_ref` field in format v4
- [ ] Golden `.writc` rebless: all existing `.writc` files in `writ-golden/tests/golden/` must be regenerated after the format version bump

---

## Sources

### Primary (HIGH confidence)

All findings are from direct source code inspection of the Writ workspace. No external sources required — this is an internal implementation phase.

- `writ-compiler/src/emit/serialize.rs` — `build_source_spans` (confirmed: hardcodes line=0, column=0)
- `writ-compiler/src/emit/body/mod.rs` — `has_error_nodes` and `emit_all_bodies` (confirmed: global abort on any error node)
- `writ-runtime/src/host.rs` — `RuntimeHost` trait (confirmed: `on_gc_complete` default pattern; no debug methods)
- `writ-runtime/src/task.rs` — `Task` struct (confirmed: `pending_request` exists; no `suspend_reason`)
- `writ-module/src/module.rs` — `DebugLocal` struct (confirmed: 4 fields, no `type_ref`)
- `writ-module/src/writer.rs` — body serialization (confirmed: 14 bytes per DebugLocal, no type_ref)
- `writ-module/src/reader.rs` — body deserialization (confirmed: reads 14 bytes per DebugLocal)
- `writ-runtime/src/dispatch/mod.rs` — `execute_one` (confirmed: no debug hook calls)
- `writ-runtime/src/scheduler.rs` — `run_one_task` (confirmed: `task.state = Suspended` at line 132)
- `writ-assembler/src/disassembler.rs` — `disassemble_body` (confirmed: no `.locals` section, no source location comments)
- `writ-cli/src/main.rs` — `run_pipeline` (confirmed: source text is available but not passed to `emit_bodies`)

### Secondary (MEDIUM confidence)

- Chumsky `SimpleSpan` byte-offset semantics: inferred from `ariadne` usage in `writ-diagnostics/src/render.rs` which uses byte offsets consistently. HIGH confidence after cross-referencing with `writ-parser/src/parser.rs` which uses `SimpleSpan = chumsky::span::SimpleSpan`.

---

## Metadata

**Confidence breakdown:**
- PREP-01 fix approach: HIGH — root cause confirmed, pattern is standard
- PREP-02 fix approach: HIGH — exact code location confirmed, change is minimal
- PREP-03 fix approach: HIGH — exact trait pattern confirmed from `on_gc_complete`
- PREP-04 fix approach: HIGH — Task struct confirmed, integration point confirmed
- PREP-05 fix approach: HIGH — binary layout confirmed from reader/writer; format version strategy is standard

**Research date:** 2026-03-13
**Valid until:** Indefinite (internal codebase — no external version drift risk)
