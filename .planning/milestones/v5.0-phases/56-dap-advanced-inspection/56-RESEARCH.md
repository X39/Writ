# Phase 56: DAP Advanced Inspection - Research

**Researched:** 2026-03-14
**Domain:** DAP variables/scopes/evaluate protocol extension, Writ VM register inspection, cooperative task enumeration
**Confidence:** HIGH (codebase read directly; DAP protocol from official spec; dap crate types from docs.rs)

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DAP-04 | User can inspect local variable names and values when execution is paused | DebugLocal table lookup, register-per-frame read, Scopes+Variables DAP requests |
| DAP-06 | User can evaluate watch expressions against the current stack frame | Evaluate DAP request — limited to name lookup in DebugLocal table for current frame |
| DAP-07 | DAP shows all Writ cooperative tasks as separate debugger threads | Scheduler.tasks enumeration, per-task Thread entries, per-task StackTrace/Scopes |
</phase_requirements>

---

## Summary

Phase 56 extends the `writ-dap` crate built in Phase 55 with three new capabilities: variable inspection, watch-expression evaluation, and multi-task thread display.

**Variable inspection (DAP-04)** is driven by the DAP `Scopes` and `Variables` request pair. When the user pauses, VS Code sends `Scopes(frameId)` to discover what containers of variables exist for that frame. The adapter responds with one `Scope` named "Locals" carrying a `variablesReference` integer. VS Code then sends `Variables(variablesReference)` and the adapter returns a `Variable` list with name, value, and type for each. The data source is the module's `DebugLocal` table (already populated in Phase 52 via PREP-05): `MethodBody.debug_locals` maps register index + PC range to variable name and type. The runtime exposes register values through the `CallFrame.registers` vector, accessible via `Runtime::call_stack_frames` and direct scheduler task access. A key gap to address: `Runtime::register_value` reads only the top frame. Phase 56 needs a new accessor `register_value_at_frame(task_id, frame_index, reg)` to support inspecting non-top frames.

**Watch evaluation (DAP-06)** is driven by the DAP `Evaluate` request carrying an expression string and an optional `frameId`. A full expression evaluator is out of scope. The practical implementation is name lookup: if the expression is a simple identifier, look it up in the `DebugLocal` table for the specified frame at its current PC, and return the current register value. Complex expressions (arithmetic, field access) respond with an error message. This is explicitly sufficient for the phase's success criterion ("evaluates against the current stack frame and shows the result while paused") when the expression is a local variable name.

**Multi-task threads (DAP-07)** replaces the hardcoded `threads: [Thread { id: 1, name: "main" }]` response in Phase 55 with real enumeration from `Runtime.scheduler.tasks`. Each active (non-terminal) Writ task becomes a DAP thread. When VS Code switches to a different thread and requests its `StackTrace`, `Scopes`, or `Variables`, the adapter uses that thread's `task_id` to answer.

**Primary recommendation:** All three features are additions to `server.rs`. No new source files are required. The key new runtime accessor needed is `frame_registers(task_id, frame_index) -> Option<Vec<Value>>` for per-frame register reads, plus `all_task_ids() -> Vec<TaskId>` for thread enumeration.

---

## Standard Stack

### Core (unchanged from Phase 55)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `dap` | 0.4.1-alpha1 | `Scopes`/`Variables`/`Evaluate`/`Threads` types + Server | Already in workspace; has all needed response types |
| `writ-runtime` | path | Register reads, task enumeration via scheduler | Direct VM state access |
| `writ-module` | path | `DebugLocal` table, string heap for var names, type blob for var types | All debug metadata lives here |

### No New Dependencies
Phase 56 requires no new crate dependencies. All needed building blocks exist in the Phase 55 workspace.

---

## Architecture Patterns

### Pattern 1: variablesReference Encoding

**What:** DAP uses opaque integers called `variablesReference` to identify containers of variables. The adapter assigns these integers and must map them back to actual data when `Variables` is requested.

**Contract:** `variablesReference` values are only valid while execution remains suspended. They are invalidated when the program resumes.

**Design for Writ Phase 56:**

Encode `(task_id_index, frame_index)` into a single i64. The variablesReference for a Locals scope is:
```rust
// Pack (task_id.index, frame_index) into i64
fn make_variables_ref(task_idx: u32, frame_idx: u32) -> i64 {
    ((task_idx as i64) << 32) | (frame_idx as i64)
}

fn unpack_variables_ref(r: i64) -> (u32, u32) {  // (task_idx, frame_idx)
    ((r >> 32) as u32, (r & 0xFFFFFFFF) as u32)
}
```

This avoids a separate HashMap and survives multiple Variables requests in one suspension.

**Stack frame IDs** are already built in `build_stack_frames()` as `frame_index as i64` (0 = innermost). The `Scopes` request sends `frame_id` matching that integer, so the adapter can decode which frame to inspect.

### Pattern 2: Scopes Request Handler

**What:** VS Code sends `Scopes { frame_id: i64 }`. The adapter replies with a list of `Scope` objects. For Writ, one Scope named "Locals" is sufficient.

**Implementation:**
```rust
Command::Scopes(args) => {
    let frame_id = args.frame_id;
    // frame_id = frame_index as returned by build_stack_frames() (0=innermost)

    // Decode which task this frame belongs to.
    // Phase 55: single task. Phase 56: need to map frame_id to task.
    // Since frame_ids are indices within a single task's stack, and all
    // frames are from the primary paused task, use task_id directly.
    // For multi-task: frame_id must encode task too — but DAP StackTrace
    // response assigns frame IDs globally. Use: frame_id = task_idx * MAX_FRAMES + frame_idx.

    let vars_ref = make_variables_ref(task_idx, frame_id as u32);

    let scope = types::Scope {
        name: "Locals".to_string(),
        variables_reference: vars_ref,
        expensive: false,
        presentation_hint: Some("locals".to_string()),
        named_variables: Some(count_locals_in_frame(...)),
        ..Default::default()
    };

    let rsp = req.success(ResponseBody::Scopes(responses::ScopesResponse {
        scopes: vec![scope],
    }));
    let _ = self.server.respond(rsp);
}
```

**Note:** `Scope.expensive = false` tells VS Code it can eagerly expand the scope. `presentation_hint: Some("locals")` makes VS Code label it "Locals" in the Variables panel.

### Pattern 3: Variables Request Handler

**What:** VS Code sends `Variables { variables_reference: i64 }`. The adapter unpacks `(task_idx, frame_idx)`, reads the frame's registers, cross-references with `DebugLocal` entries active at the frame's current PC, and returns a `Variable` for each named local.

**Key data sources:**
- `module.method_bodies[method_idx].debug_locals` — list of `DebugLocal { register, name, type_ref, start_pc, end_pc }`
- `CallFrame.registers` — current register values (needs per-frame read accessor)
- Current `pc` from `call_stack_frames` (returned as `(method_idx, pc)` tuples)

**Algorithm:**
```rust
Command::Variables(args) => {
    let (task_idx, frame_idx) = unpack_variables_ref(args.variables_reference);
    // task_idx: use to find task_id (e.g., index into sorted task list)
    // frame_idx: index into call_stack (0 = bottom/oldest frame per call_stack_frames ordering)
    // Note: build_stack_frames() reverses for display, but call_stack_frames() is bottom-to-top.
    // frame_id 0 from StackTrace = innermost (top) = call_stack.last()
    // So: actual_stack_index = call_stack.len() - 1 - frame_idx

    let frames = runtime.call_stack_frames(task_id)?;
    // frames[i] = (method_idx, pc), ordered bottom-to-top (oldest=0)
    // frame_idx 0 from Scopes = innermost = frames.last()
    let actual_idx = frames.len() - 1 - frame_idx as usize;
    let (method_idx, pc) = frames[actual_idx];

    // Find active DebugLocals for this method at this pc
    let debug_locals = &module.method_bodies[method_idx].debug_locals;
    let active: Vec<_> = debug_locals.iter()
        .filter(|dl| dl.start_pc <= pc as u32 && pc as u32 < dl.end_pc)
        .collect();

    // Read register values for this frame
    let regs = runtime.frame_registers(task_id, actual_idx)?;

    let variables: Vec<types::Variable> = active.iter().map(|dl| {
        let name = read_string(&module.string_heap, dl.name)
            .unwrap_or("?").to_string();
        let reg_val = regs.get(dl.register as usize)
            .cloned()
            .unwrap_or(Value::Void);
        let value_str = format_value(&reg_val, &module, runtime.heap());
        let type_str = decode_type_blob(&module.blob_heap, dl.type_ref);

        types::Variable {
            name,
            value: value_str,
            type_field: Some(type_str),
            variables_reference: 0,  // no children for primitives
            ..Default::default()
        }
    }).collect();

    let rsp = req.success(ResponseBody::Variables(responses::VariablesResponse { variables }));
    let _ = self.server.respond(rsp);
}
```

### Pattern 4: Value Formatting (`format_value`)

**What:** Converts a `Value` to a human-readable string for the Variables panel.

**Implementation:**
```rust
fn format_value(val: &Value, module: &Module, heap: &dyn GcHeap) -> String {
    match val {
        Value::Void    => "(void)".to_string(),
        Value::Int(n)  => n.to_string(),
        Value::Float(f) => format!("{:.6}", f),
        Value::Bool(b) => b.to_string(),
        Value::Ref(href) => {
            match heap.get_object(*href) {
                Ok(HeapObject::String(s)) => format!("{:?}", s),  // quoted string
                Ok(HeapObject::Struct { fields }) => {
                    format!("struct({})", fields.len())  // Phase 56: no field expansion
                }
                Ok(HeapObject::Array { elements, .. }) => {
                    format!("[{} elements]", elements.len())
                }
                Ok(HeapObject::Delegate { method_idx, .. }) => {
                    format!("fn@{}", method_idx)
                }
                Ok(HeapObject::Enum { tag, .. }) => {
                    format!("enum(tag={})", tag)
                }
                Ok(HeapObject::Boxed(v)) => format!("box({})", format_value(v, module, heap)),
                Err(_) => "<invalid ref>".to_string(),
            }
        }
        Value::Entity(eid) => format!("entity#{}", eid.index),
        Value::InlineStruct { type_idx, fields } => {
            format!("struct{}({})", type_idx, fields.len())
        }
    }
}
```

**Phase 56 scope:** Return flat string values. Do NOT implement `variablesReference > 0` for struct children (DAPX-02 is v6+ scope). Return `variables_reference: 0` for all variables.

### Pattern 5: Type Blob Decoding (`decode_type_blob`)

**What:** Converts a TypeRef blob offset in the module's blob heap into a human-readable type name string for the `type_field` in `Variable`.

**Type encoding (from `writ-compiler/src/emit/type_sig.rs`):**
```
0x00 = void
0x01 = int
0x02 = float
0x03 = bool
0x04 = string
0x10 + u32(row) = TypeDef (struct/enum/entity/class) — row is 1-based TypeDef table row
0x11 + u32(row) = TypeSpec (generic instantiation — placeholder)
0x12 + u16(idx) = GenericParam
0x20 + TypeRef = Array<T>
0x30 + u32(blob_offset) = function signature blob
```

**Implementation:**
```rust
fn decode_type_blob(blob_heap: &[u8], offset: u32) -> String {
    if offset == 0 { return "?".to_string(); }
    let Ok(bytes) = read_blob(blob_heap, offset) else { return "?".to_string(); };
    if bytes.is_empty() { return "?".to_string(); }
    match bytes[0] {
        0x00 => "void".to_string(),
        0x01 => "int".to_string(),
        0x02 => "float".to_string(),
        0x03 => "bool".to_string(),
        0x04 => "string".to_string(),
        0x10 if bytes.len() >= 5 => {
            // TypeDef row (1-based) — could look up TypeDef name in module tables
            // For simplicity, return "Type" for now
            "Type".to_string()
        }
        0x20 => "Array<?>".to_string(),
        _ => "?".to_string(),
    }
}
```

**Enhancement:** For `0x10` (TypeDef), can look up `module.type_defs[row-1].name` via `read_string` for the actual type name. This is worth doing for a good user experience.

### Pattern 6: Evaluate Request Handler (DAP-06)

**What:** VS Code sends `Evaluate { expression: String, frame_id: Option<i64>, context: ... }`. For watch expressions, `context` is `"watch"`. The adapter evaluates against the frame identified by `frame_id`.

**Scope of implementation:** Name lookup only. If the expression is a simple identifier that matches an active `DebugLocal` in the specified frame, return its value. Otherwise return an error string.

```rust
Command::Evaluate(args) => {
    let expr = &args.expression;
    let frame_id = args.frame_id.unwrap_or(0);

    // Find the frame
    let frames = runtime.call_stack_frames(task_id)?;
    let actual_idx = frames.len().saturating_sub(1 + frame_id as usize);
    let (method_idx, pc) = frames[actual_idx];

    // Look up expression as a variable name
    let debug_locals = &module.method_bodies[method_idx].debug_locals;
    let found = debug_locals.iter()
        .filter(|dl| dl.start_pc <= pc as u32 && pc as u32 < dl.end_pc)
        .find(|dl| {
            read_string(&module.string_heap, dl.name)
                .map(|n| n == expr.as_str())
                .unwrap_or(false)
        });

    let rsp = if let Some(dl) = found {
        let regs = runtime.frame_registers(task_id, actual_idx)?;
        let val = regs.get(dl.register as usize).cloned().unwrap_or(Value::Void);
        let value_str = format_value(&val, &module, runtime.heap());
        let type_str = decode_type_blob(&module.blob_heap, dl.type_ref);
        req.success(ResponseBody::Evaluate(responses::EvaluateResponse {
            result: value_str,
            type_field: Some(type_str),
            variables_reference: 0,
            ..Default::default()
        }))
    } else {
        // Not a known local variable — return error string (not a DAP error, just a result)
        req.success(ResponseBody::Evaluate(responses::EvaluateResponse {
            result: format!("'{}' is not a local variable in the current frame", expr),
            type_field: None,
            variables_reference: 0,
            ..Default::default()
        }))
    };

    let _ = self.server.respond(rsp);
}
```

**Note:** Returning a descriptive error string as `result` (not a DAP protocol error) is correct per spec — VS Code displays it inline in the Watch panel.

### Pattern 7: Multi-Task Thread Enumeration (DAP-07)

**What:** Replace the hardcoded `Thread { id: 1, name: "main" }` with real enumeration of active Writ tasks.

**New runtime accessor needed:**
```rust
// In writ-runtime/src/runtime.rs
pub fn all_task_ids(&self) -> Vec<TaskId> {
    self.scheduler.tasks.values()
        .filter(|t| !matches!(t.state, TaskState::Completed | TaskState::Cancelled))
        .map(|t| t.id)
        .collect()
}
```

**Thread ID mapping:** Use `task_id.index` as the DAP thread ID. TaskId is a `GenHandle<TaskTag>` with `index: u32`, `generation: u32`. The `index` is a monotonically incrementing counter starting at 0, making it a stable, unique identifier per task per session. Map `task_id.index` to `thread_id: i64` directly.

**Thread name:** Use `"task-{index}"` for spawned tasks. If the task's method name can be resolved from the call stack's bottom frame (the entry method), use that as the thread name.

```rust
Command::Threads => {
    let task_ids = runtime.all_task_ids();
    let threads: Vec<types::Thread> = task_ids.iter().map(|&tid| {
        let name = runtime.call_stack_frames(tid)
            .and_then(|frames| frames.first().copied())
            .and_then(|(method_idx, _)| {
                module.method_defs.get(method_idx)
                    .and_then(|def| read_string(&module.string_heap, def.name).ok())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| format!("task-{}", tid.index));

        types::Thread {
            id: tid.index as i64,
            name,
        }
    }).collect();

    let rsp = req.success(ResponseBody::Threads(responses::ThreadsResponse { threads }));
    let _ = self.server.respond(rsp);
}
```

**StackTrace/Scopes/Variables per task:** All three must now handle a `thread_id` argument. When VS Code sends `StackTrace { thread_id: 2 }`, the adapter must find the task with `task_id.index == 2` and return its call stack, not the main task's stack.

**Frame ID global namespace:** When multiple tasks are active, frame IDs must be globally unique. Encode `(task_index * MAX_STACK_DEPTH + frame_index)` as the frame ID. A `MAX_STACK_DEPTH` of 1000 is safe (Writ programs won't have 1000-level deep call stacks in practice). Or encode with the variablesReference packing strategy: `task_index as i64 * 10000 + frame_index as i64`.

### Pattern 8: New Runtime Accessors Required

Phase 56 requires two new public methods on `Runtime<H>` in `writ-runtime/src/runtime.rs`:

```rust
/// Read all register values for a specific call frame (by frame index from bottom).
/// frame_index 0 = oldest frame (entry function), N-1 = current (innermost).
pub fn frame_registers(&self, task_id: TaskId, frame_index: usize) -> Option<Vec<Value>> {
    self.scheduler.tasks.get(&task_id)
        .and_then(|t| t.call_stack.get(frame_index))
        .map(|f| f.registers.clone())
}

/// Return all active (non-terminal) task IDs.
pub fn all_task_ids(&self) -> Vec<TaskId> {
    self.scheduler.tasks.values()
        .filter(|t| !matches!(t.state, TaskState::Completed | TaskState::Cancelled))
        .map(|t| t.id)
        .collect()
}
```

These are simple accessors with no behavioral side effects — safe to add without disrupting existing tests.

### Recommended Project Structure (additions to existing writ-dap/src/)
```
writ-dap/src/
├── server.rs         # MODIFIED: Scopes, Variables, Evaluate, multi-task Threads handlers
├── variables.rs      # NEW: format_value(), decode_type_blob(), make/unpack_variables_ref()
├── debug_host.rs     # UNCHANGED
├── breakpoints.rs    # UNCHANGED
├── launch.rs         # UNCHANGED
├── lib.rs            # MODIFIED: pub mod variables
├── main.rs           # UNCHANGED
```

### Anti-Patterns to Avoid

- **Using the same frame_id encoding as stack frame IDs for variablesReference:** Scopes sends a `frame_id` which is the same i64 as in StackTrace response. Do not conflate with `variablesReference`. Scopes receives frame_id, returns scopes with variablesReference. These are two distinct integer namespaces.
- **Returning a DAP protocol error for unknown expressions in Evaluate:** Return a friendly string result like "'x' is not a local variable". VS Code shows this inline in the Watch panel. A protocol error would make the Watch entry show "Error" without context.
- **Assuming `debug_locals` is always populated:** Debug info is only present when compiled with `emit_debug = true`. The `compile_and_load` in `launch.rs` hardcodes `emit_debug: true` — but add a guard anyway.
- **Exposing register indices as variable names when no DebugLocal entry:** Skip registers with no DebugLocal mapping. Only named variables appear in the Variables panel.
- **Returning variablesReference > 0 for heap objects:** Nested variable expansion is DAPX-02 (v6+). Return 0 for all variables in Phase 56. VS Code shows the formatted string value as a leaf node.
- **Using thread_id 1 hardcoded after adding multi-task support:** The StackTrace/Scopes/Variables handlers must now look up the correct task_id from the thread_id argument.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| DAP response serialization | Custom JSON | `dap::prelude::*` types | `Variable`, `Scope`, `EvaluateResponse` already defined |
| Variable tree traversal | Custom recursive walker | Flat Variables response (variablesReference=0) | DAPX-02 is v6+; flat is sufficient for Phase 56 |
| Expression parser for Evaluate | Mini-parser for arithmetic | Simple name lookup in DebugLocal table | Success criterion is "variable name lookup works"; full eval is out of scope |

---

## Common Pitfalls

### Pitfall 1: Frame Index Convention Mismatch

**What goes wrong:** `call_stack_frames()` returns frames ordered bottom-to-top (oldest=0), but `build_stack_frames()` reverses them for display (innermost=0 in the StackTrace response). The `frame_id` sent in `Scopes` matches the StackTrace display index (0=innermost). Must convert: `actual_stack_index = call_stack.len() - 1 - frame_id`.

**Why it happens:** DAP convention is innermost-first for display; internal call stack is push-order (oldest-first).

**How to avoid:** Always use `call_stack.len() - 1 - frame_id` when converting from DAP frame_id to internal stack index.

**Warning signs:** Variables panel shows variables from the wrong function — e.g., shows outer function's locals when paused inside a nested call.

### Pitfall 2: DebugLocal PC Range Boundary

**What goes wrong:** A variable name appears in the Variables panel before its assignment (e.g., it shows `(void)` on the `let x = ...` line itself).

**Why it happens:** `DebugLocal.start_pc` is the first instruction where the variable is live. The filter `start_pc <= pc && pc < end_pc` is correct, but if the breakpoint fires at the exact `start_pc` before the assignment instruction executes, the register holds the pre-assignment value.

**How to avoid:** This is expected behavior — it matches how native debuggers work. Document in comments. Do not filter out variables at their start_pc.

### Pitfall 3: Threads Response When No Tasks Are Active

**What goes wrong:** After program termination, `all_task_ids()` returns an empty list, and VS Code receives an empty threads response, which can confuse the UI.

**Why it happens:** Tasks move to Completed/Cancelled state after the program exits.

**How to avoid:** After `Terminated`/`Exited` events are sent, VS Code should not send further Threads/StackTrace/Variables requests. But if it does, return a single dummy thread `Thread { id: 0, name: "terminated" }` to avoid an empty response.

### Pitfall 4: Heap Access Requires `&dyn GcHeap`

**What goes wrong:** `format_value` needs to call `heap.get_object(href)` to format `Value::Ref(href)`, but `DapServer` only has `&Runtime<DebugHost>`. The `heap()` method returns `&dyn GcHeap`.

**Why it happens:** The heap type is erased via `Box<dyn GcHeap>`.

**How to avoid:** `Runtime::heap()` is already pub and returns `&dyn GcHeap`. Call `runtime.heap().get_object(href)` in `format_value`. This compiles fine since `get_object` is a trait method on `GcHeap`.

**Warning signs:** Compile error "method get_object not found on &dyn GcHeap" — check the GcHeap trait definition in `writ-runtime/src/gc.rs`.

### Pitfall 5: variablesReference Lifetime

**What goes wrong:** VS Code sends `Variables(ref)` after a Continue/Step resumes execution, expecting the adapter to return variables. But the task is now running (not suspended), so register values are undefined.

**Why it happens:** DAP spec: variablesReference values are only valid while execution is suspended.

**How to avoid:** Guard the `Variables` handler: if `runtime.task_state(task_id) != Some(TaskState::Suspended)`, return an empty variables list (or a single variable with value "not paused").

### Pitfall 6: DebugLocal `end_pc` Encoding

**What goes wrong:** A variable that should be visible at the current PC is filtered out.

**Why it happens:** `end_pc` in the compiled module may be encoded as an exclusive upper bound (pc < end_pc) or inclusive (pc <= end_pc). Need to verify which convention the compiler uses.

**How to avoid:** Check `writ-compiler/src/emit/body/debug.rs` for the DebugLocal end_pc encoding. The Phase 52 implementation likely uses exclusive end (matching LLVM/CLR convention). Use `dl.start_pc <= pc as u32 && pc as u32 < dl.end_pc`.

---

## Code Examples

### Scopes Response
```rust
// Source: dap 0.4.1-alpha1 types::Scope, responses::ScopesResponse
use dap::{prelude::*, types, responses};

fn handle_scopes(req: Request, frame_id: i64, task_idx: u32, frame_count: usize,
                  local_count: usize) -> Response {
    let vars_ref = make_variables_ref(task_idx, (frame_count - 1).saturating_sub(frame_id as usize) as u32);
    req.success(ResponseBody::Scopes(responses::ScopesResponse {
        scopes: vec![types::Scope {
            name: "Locals".to_string(),
            presentation_hint: Some("locals".to_string()),
            variables_reference: vars_ref,
            named_variables: Some(local_count as i64),
            expensive: false,
            ..Default::default()
        }],
    }))
}
```

### Variables Response for a Frame
```rust
// Source: dap 0.4.1-alpha1 types::Variable, responses::VariablesResponse
fn make_variable(name: &str, value: &str, type_name: &str) -> types::Variable {
    types::Variable {
        name: name.to_string(),
        value: value.to_string(),
        type_field: Some(type_name.to_string()),
        variables_reference: 0,  // leaf node — no children
        ..Default::default()
    }
}
```

### Evaluate Response for a Known Variable
```rust
// Source: dap 0.4.1-alpha1 responses::EvaluateResponse
fn make_evaluate_result(value: &str, type_name: &str) -> ResponseBody {
    ResponseBody::Evaluate(responses::EvaluateResponse {
        result: value.to_string(),
        type_field: Some(type_name.to_string()),
        variables_reference: 0,
        ..Default::default()
    })
}
```

### Multi-Task Threads Response
```rust
// Source: dap 0.4.1-alpha1 types::Thread, responses::ThreadsResponse
fn build_threads(runtime: &Runtime<DebugHost>, module: &Module) -> responses::ThreadsResponse {
    let task_ids = runtime.all_task_ids();
    let threads = task_ids.iter().map(|&tid| {
        let name = runtime.call_stack_frames(tid)
            .and_then(|frames| frames.first().copied())
            .and_then(|(m, _)| module.method_defs.get(m))
            .and_then(|def| read_string(&module.string_heap, def.name).ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("task-{}", tid.index));
        types::Thread { id: tid.index as i64, name }
    }).collect();
    responses::ThreadsResponse { threads }
}
```

### DebugLocal Active Variable Lookup
```rust
// Source: writ-module/src/module.rs DebugLocal struct (read directly)
// DebugLocal { register: u16, name: u32, type_ref: u32, start_pc: u32, end_pc: u32 }
fn active_locals<'a>(
    module: &'a Module,
    method_idx: usize,
    pc: usize,
) -> impl Iterator<Item = &'a writ_module::module::DebugLocal> {
    module.method_bodies[method_idx].debug_locals.iter()
        .filter(move |dl| dl.start_pc <= pc as u32 && (pc as u32) < dl.end_pc)
}
```

---

## State of the Art

| Old Approach (Phase 55) | New Approach (Phase 56) | Impact |
|------------------------|------------------------|--------|
| `Scopes` returns empty `scopes: vec![]` | Returns one "Locals" scope with variablesReference | Variables panel becomes functional |
| `Variables` returns empty `variables: vec![]` | Returns DebugLocal-mapped register values | Variable inspection works |
| `Evaluate` unimplemented (returns error) | Returns value for local variable name lookups | Watch panel shows values for simple expressions |
| `Threads` hardcoded to `[Thread { id:1, name:"main" }]` | Enumerates all active Writ tasks | Multi-task programs show each task as a thread |

---

## Open Questions

1. **DebugLocal end_pc convention**
   - What we know: `DebugLocal` has `end_pc: u32`. The compiler emits this in `writ-compiler/src/emit/body/debug.rs`.
   - What's unclear: Is `end_pc` exclusive (`pc < end_pc`) or inclusive (`pc <= end_pc`)?
   - Recommendation: Read `writ-compiler/src/emit/body/debug.rs` at plan time to confirm. Use exclusive convention as default (CLR standard), add test.

2. **frame_id global uniqueness for multi-task**
   - What we know: StackTrace frame IDs are used by Scopes. With one task, `frame_id = frame_index`. With multiple tasks, frame IDs must be globally unique.
   - What's unclear: Does the current Phase 55 `build_stack_frames` assign per-task or global frame IDs?
   - Recommendation: Phase 55 assigns `frame_id = frame_index as i64` (0-based per task). For multi-task support, update to `task_idx * 10000 + frame_idx` as the frame ID. The `variablesReference` encoding already handles this via `make_variables_ref(task_idx, frame_idx)`.
   - **Constraint:** Must verify `10000` is large enough (Writ call stacks in practice are < 100 deep).

3. **TypeDef name resolution in decode_type_blob**
   - What we know: TypeRef `0x10 + u32(row)` encodes a 1-based TypeDef table row. `module.type_defs[row-1].name` is a string heap offset.
   - What's unclear: Is the row in the local module's TypeDef table or a cross-module TypeRef table?
   - Recommendation: For `0x10` tags, look up in `module.type_defs` (local). For `0x11` (TypeSpec/generic), return a placeholder like `"Option<T>"`.

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
| DAP-04 | DebugLocal active variable filter at given PC | unit | `cargo test -p writ-dap test_active_locals` | ❌ Wave 0 |
| DAP-04 | format_value renders int/float/bool/string/void correctly | unit | `cargo test -p writ-dap test_format_value` | ❌ Wave 0 |
| DAP-04 | decode_type_blob returns "int"/"bool"/"string" for primitive tags | unit | `cargo test -p writ-dap test_decode_type_blob` | ❌ Wave 0 |
| DAP-04 | make/unpack_variables_ref roundtrips for task=0..5, frame=0..9 | unit | `cargo test -p writ-dap test_variables_ref_encoding` | ❌ Wave 0 |
| DAP-04 | frame_registers returns correct values for each frame | unit | `cargo test -p writ-runtime test_frame_registers` | ❌ Wave 0 |
| DAP-06 | Evaluate finds active local by name | unit | `cargo test -p writ-dap test_evaluate_local_name` | ❌ Wave 0 |
| DAP-06 | Evaluate returns descriptive message for unknown name | unit | `cargo test -p writ-dap test_evaluate_unknown` | ❌ Wave 0 |
| DAP-07 | all_task_ids returns only non-terminal tasks | unit | `cargo test -p writ-runtime test_all_task_ids` | ❌ Wave 0 |
| DAP-07 | Threads response includes one Thread per active task | unit | `cargo test -p writ-dap test_threads_multi_task` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p writ-dap && cargo test -p writ-runtime`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `writ-dap/src/variables.rs` — `format_value`, `decode_type_blob`, `make_variables_ref`, `unpack_variables_ref` with unit tests
- [ ] `writ-runtime/src/runtime.rs` — add `frame_registers` and `all_task_ids` accessors
- [ ] `writ-runtime` tests for new accessors

*(No new crates or framework installs needed — all existing infrastructure)*

---

## Sources

### Primary (HIGH confidence)
- `writ-dap/src/server.rs` — Phase 55 DapServer: existing Scopes(vec![]) and Variables(vec![]) stubs, Thread hardcoding (read directly)
- `writ-dap/src/debug_host.rs` — DebugHost state machine with pending_stop, StopReason (read directly)
- `writ-module/src/module.rs` — `DebugLocal { register, name, type_ref, start_pc, end_pc }`, `MethodBody.debug_locals` (read directly)
- `writ-runtime/src/runtime.rs` — `register_value` (top-frame only), `call_stack_frames`, `all_task_ids` gap confirmed (read directly)
- `writ-runtime/src/frame.rs` — `CallFrame { method_idx, pc, registers }` (read directly)
- `writ-runtime/src/heap.rs` — `HeapObject` variants for format_value (read directly)
- `writ-runtime/src/value.rs` — `Value` enum (Void/Int/Float/Bool/Ref/Entity/InlineStruct) (read directly)
- `writ-compiler/src/emit/type_sig.rs` — TypeRef blob encoding: 0x00=void, 0x01=int, 0x02=float, 0x03=bool, 0x04=string, 0x10+row=TypeDef, 0x20+T=Array (read directly)
- [docs.rs/dap 0.4.1-alpha1 Scope](https://docs.rs/dap/0.4.1-alpha1/dap/types/struct.Scope.html) — fields: name, variables_reference, presentation_hint, named_variables, expensive (verified)
- [docs.rs/dap 0.4.1-alpha1 Variable](https://docs.rs/dap/0.4.1-alpha1/dap/types/struct.Variable.html) — fields: name, value, type_field, variables_reference, evaluate_name (verified)
- [docs.rs/dap 0.4.1-alpha1 EvaluateResponse](https://docs.rs/dap/0.4.1-alpha1/dap/responses/struct.EvaluateResponse.html) — fields: result, type_field, variables_reference (verified)
- [docs.rs/dap 0.4.1-alpha1 ScopesArguments](https://docs.rs/dap/0.4.1-alpha1/dap/requests/struct.ScopesArguments.html) — field: frame_id (verified)
- [docs.rs/dap 0.4.1-alpha1 EvaluateArguments](https://docs.rs/dap/0.4.1-alpha1/dap/requests/struct.EvaluateArguments.html) — fields: expression, frame_id, context (verified)
- [docs.rs/dap 0.4.1-alpha1 VariablesArguments](https://docs.rs/dap/0.4.1-alpha1/dap/requests/struct.VariablesArguments.html) — field: variables_reference (verified)
- [Microsoft DAP Specification](https://microsoft.github.io/debug-adapter-protocol/specification) — Scopes/Variables/Evaluate/Threads request-response contracts, variablesReference lifetime

### Secondary (MEDIUM confidence)
- Phase 55 RESEARCH.md — DAP crate usage patterns, BreakpointTable, DebugHost patterns (prior phase research)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all types verified from docs.rs and codebase
- Architecture: HIGH — variablesReference encoding derived directly from DAP spec + crate types; DebugLocal structure read from actual code; runtime accessors gap confirmed by reading runtime.rs
- Pitfalls: HIGH — frame index convention mismatch derived from actual Phase 55 `build_stack_frames()` implementation (uses rev() + enumerate()); heap access path confirmed from runtime.rs::heap()

**Research date:** 2026-03-14
**Valid until:** 2026-04-14 (30 days — DAP spec is stable; dap crate is slow-moving alpha)
