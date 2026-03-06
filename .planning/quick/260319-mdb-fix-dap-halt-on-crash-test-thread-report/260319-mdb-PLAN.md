---
phase: quick-260319-mdb
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - writ-dap/src/server/handlers.rs
  - writ-dap/src/server/inspection.rs
  - writ-dap/tests/test_protocol.rs
  - writ-golden/tests/golden/crash_unwrap_none.writ
autonomous: true
requirements: [MDB-01]
must_haves:
  truths:
    - "After a runtime crash, DAP threads response reports the thread as stopped (not terminated)"
    - "After a runtime crash, DAP stackTrace response returns non-empty stack frames from crash info"
    - "Integration test validates the full crash-halt-inspect flow: stopped event -> threads -> stackTrace"
  artifacts:
    - path: "writ-dap/src/server/handlers.rs"
      provides: "Crash-aware threads handler that returns crashed task with descriptive name"
    - path: "writ-dap/src/server/inspection.rs"
      provides: "Crash-aware stack frame builder using CrashInfo.stack_trace"
    - path: "writ-dap/tests/test_protocol.rs"
      provides: "Integration test for halt-on-crash thread+stackTrace inspection"
    - path: "writ-golden/tests/golden/crash_unwrap_none.writ"
      provides: "Test fixture that triggers a runtime crash (unwrap on None)"
  key_links:
    - from: "writ-dap/src/server/handlers.rs::handle_threads"
      to: "writ-runtime::crash_info"
      via: "Check task_id for crash_info when all_task_ids is empty"
      pattern: "crash_info.*task_id"
    - from: "writ-dap/src/server/inspection.rs::build_stack_frames"
      to: "writ-runtime::CrashInfo::stack_trace"
      via: "Fallback to crash_info stack frames when call_stack is empty"
      pattern: "crash_info.*stack_trace"
---

<objective>
Fix DAP halt-on-crash inspection: after a runtime crash (e.g., unwrap on None), VSCode
receives a `stopped` event with `reason: "exception"` but subsequent `threads` and
`stackTrace` requests return "terminated" thread name and empty stack frames respectively.

Purpose: Enable debugger users to inspect the crash location and call stack after a
runtime error, instead of seeing a dead-end "terminated" state.

Output: Fixed handlers + integration test proving threads/stackTrace work after crash.
</objective>

<execution_context>
@C:/Users/msili/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/msili/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@writ-dap/src/server/handlers.rs
@writ-dap/src/server/inspection.rs
@writ-dap/src/server/helpers.rs
@writ-dap/src/server/mod.rs
@writ-dap/src/debug_host.rs
@writ-dap/tests/test_protocol.rs
@writ-dap/tests/common/mod.rs

<interfaces>
<!-- Key types and contracts the executor needs. -->

From writ-runtime/src/runtime.rs:
```rust
// all_task_ids() FILTERS OUT Cancelled/Completed tasks -- this is the root cause
pub fn all_task_ids(&self) -> Vec<TaskId> {
    self.scheduler.tasks.values()
        .filter(|t| !matches!(t.state, TaskState::Completed | TaskState::Cancelled))
        .map(|t| t.id)
        .collect()
}

// crash_info() accesses crash data for any task (including Cancelled)
pub fn crash_info(&self, task_id: TaskId) -> Option<&CrashInfo> {
    self.scheduler.tasks.get(&task_id)
        .and_then(|t| t.crash_info.as_ref())
}

// call_stack_frames() returns the call stack but it's EMPTY after crash
// because execute_crash() unwinds all frames before setting task to Cancelled
pub fn call_stack_frames(&self, task_id: TaskId) -> Option<Vec<(usize, usize)>> { ... }
```

From writ-runtime/src/error.rs (CrashInfo):
```rust
pub struct CrashInfo {
    pub message: String,
    pub stack_trace: Vec<StackFrame>,  // captured BEFORE unwind
}

pub struct StackFrame {
    pub method_idx: usize,
    pub method_name: String,
    pub pc: usize,
}
```

From writ-dap/src/server/helpers.rs:
```rust
pub(super) fn build_thread_list(
    task_ids: &[TaskId],
    call_stack_fn: impl Fn(TaskId) -> Option<Vec<(usize, usize)>>,
    module: &Module,
) -> Vec<types::Thread>

pub(super) fn instr_to_byte_pc(
    runtime: &Runtime<DebugHost>, method_idx: usize, instr_pc: usize
) -> u32
```

From writ-dap/src/server/mod.rs:
```rust
pub struct DapServer<I, O> {
    pub(super) runtime: Option<Runtime<DebugHost>>,
    pub(super) module: Option<Module>,
    pub(super) task_id: Option<TaskId>,
    // ...
}
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Fix handle_threads and build_stack_frames for crashed tasks</name>
  <files>writ-dap/src/server/handlers.rs, writ-dap/src/server/inspection.rs</files>
  <action>
**Root Cause:** Two bugs interact to produce the "terminated" + empty stackFrames behavior:

1. `handle_threads()` calls `rt.all_task_ids()` which filters out Cancelled tasks. When the
   only task has crashed (Cancelled), this returns an empty vec, causing `build_thread_list`
   to return the hardcoded fallback `[{ id: 0, name: "terminated" }]`.

2. `build_stack_frames()` calls `runtime.call_stack_frames(task_id)` which returns an empty
   vec because `execute_crash()` unwinds all frames before setting the task to Cancelled.
   The crash's stack trace IS preserved in `CrashInfo.stack_trace` but is never used.

**Fix handle_threads in handlers.rs:**

In `handle_threads()` (around line 290), after the existing logic that builds the thread
list from `all_task_ids()`, add a fallback: if the thread list would be the "terminated"
fallback (empty task_ids) AND `self.task_id` is set AND `runtime.crash_info(task_id)` returns
Some, then instead return a thread entry with the task_id's index as id and a descriptive
name like `"main (crashed)"` or derive the name from crash_info. The thread id MUST match
the thread_id sent in the stopped event (which is `self.task_id.map(|t| t.index as i64)`).

Replace the current implementation:
```rust
pub(super) fn handle_threads(&mut self, req: Request) {
    let threads = if let (Some(rt), Some(module)) = (self.runtime.as_ref(), self.module.as_ref()) {
        let task_ids = rt.all_task_ids();
        if task_ids.is_empty() {
            // Check if the main task crashed -- if so, report it as a stopped thread
            // so VSCode can inspect the crash state.
            if let Some(task_id) = self.task_id {
                if rt.crash_info(task_id).is_some() {
                    vec![types::Thread {
                        id: task_id.index as i64,
                        name: "main (crashed)".to_string(),
                    }]
                } else {
                    vec![types::Thread { id: 0, name: "terminated".to_string() }]
                }
            } else {
                vec![types::Thread { id: 0, name: "terminated".to_string() }]
            }
        } else {
            build_thread_list(&task_ids, |tid| rt.call_stack_frames(tid), module)
        }
    } else {
        vec![types::Thread { id: 0, name: "terminated".to_string() }]
    };
    let rsp = req.success(ResponseBody::Threads(responses::ThreadsResponse { threads }));
    let _ = self.server.respond(rsp);
}
```

**Fix build_stack_frames in inspection.rs:**

In `build_stack_frames()` (around line 307), after the existing call to
`runtime.call_stack_frames(task_id)` returns an empty vec (or None), add a fallback
that checks `runtime.crash_info(task_id)`. If crash_info exists, use its `stack_trace`
vec to build DAP StackFrame entries. The CrashInfo::StackFrame has `method_idx` and `pc`
(instruction-index PC), which is the same format as `call_stack_frames` returns.

After the existing line `let frames = match runtime.call_stack_frames(task_id) { ... }`:
```rust
let frames = match runtime.call_stack_frames(task_id) {
    Some(f) if !f.is_empty() => f,
    _ => {
        // Call stack is empty (crashed task has unwound frames).
        // Fall back to CrashInfo.stack_trace if available.
        if let Some(crash) = runtime.crash_info(for_task_id) {
            crash.stack_trace.iter()
                .map(|sf| (sf.method_idx, sf.pc))
                .collect()
        } else {
            return vec![];
        }
    }
};
```

Note: CrashInfo.stack_trace is already in top-to-bottom order (reversed during capture
in execute_crash), so do NOT re-reverse it. The existing code reverses frames because
call_stack_frames returns bottom-to-top. For crash frames, skip the `.rev()` step.

To handle this, restructure the frame-building loop. After determining `frames`, check if
this is from crash_info (already reversed) vs call_stack (needs reversing). Simplest approach:
since crash_info captures frames with `.rev()` already applied, the frames vec from the
crash path is already top-to-bottom. The existing code does `.iter().rev().enumerate()` to
reverse call_stack frames to top-to-bottom. So for crash frames, just use `.iter().enumerate()`
without `.rev()`.

Implementation approach: Create the frames as a local `Vec<(usize, usize)>` and a boolean
`is_crash_frames`, then use the appropriate iteration order.
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test -p writ-dap -- test_threads_multi_task --no-fail-fast 2>&1 | tail -5</automated>
  </verify>
  <done>
    handle_threads returns crashed task with matching thread_id and descriptive name when
    task is Cancelled with crash_info. build_stack_frames returns non-empty frames from
    CrashInfo.stack_trace when call_stack is empty due to crash unwinding. Existing unit
    tests still pass.
  </done>
</task>

<task type="auto">
  <name>Task 2: Add crash fixture and integration test for halt-on-crash inspection</name>
  <files>writ-golden/tests/golden/crash_unwrap_none.writ, writ-dap/tests/test_protocol.rs</files>
  <action>
**Create test fixture** `writ-golden/tests/golden/crash_unwrap_none.writ`:
```writ
pub fn main() {
    let x: int? = None;
    let y = x!;
}
```
This program crashes at line 3 with an unwrap-on-None runtime error.

**Add integration test** `test_halt_on_crash_inspect` to `writ-dap/tests/test_protocol.rs`:

The test must validate the FULL crash inspection flow that VSCode performs:
1. Initialize, configurationDone, launch `crash_unwrap_none.writ`
2. Expect a `stopped` event with `reason: "exception"` (already works)
3. Extract `threadId` from the stopped event body
4. Send `threads` request -- assert the thread list contains a thread with:
   - `id` matching the `threadId` from the stopped event
   - `name` that is NOT "terminated" (should be something like "main (crashed)")
5. Send `stackTrace` request with the `threadId` -- assert:
   - `stackFrames` array is NOT empty
   - At least one frame exists (the crash location)
6. Send `continue` to let the program terminate (the continue handler should handle
   a crashed task gracefully -- it will call run_until_stop which will see Cancelled
   state and emit terminated event)
7. Shutdown cleanly

Also verify an `output` event with the crash message was emitted (category: stderr,
containing "Runtime crash" or the crash message text).

Use the `FIXTURE_CRASH` constant for the crash fixture path:
```rust
const FIXTURE_CRASH: &str = "writ-golden/tests/golden/crash_unwrap_none.writ";
```

Pattern the test after the existing `test_breakpoint_hit_and_inspect` test but adapted
for crash flow. Use `recv_execution_event()` which collects events until stopped/terminated.

Important: after inspecting threads+stackTrace on a crashed task, the test must handle
the continue/disconnect gracefully. When continuing from a crash, `run_until_stop` will
see the task is already Cancelled and emit terminated+exited events. The test should
account for this or just call `client.shutdown()` directly (which sends disconnect).
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test -p writ-dap -- test_halt_on_crash_inspect --no-fail-fast 2>&1 | tail -20</automated>
  </verify>
  <done>
    Integration test `test_halt_on_crash_inspect` passes, proving that after a runtime
    crash: (1) threads response contains a thread with non-terminated name matching the
    stopped event's threadId, (2) stackTrace response contains non-empty stack frames,
    (3) the output event contains the crash message.
  </done>
</task>

<task type="auto">
  <name>Task 3: Handle continue-after-crash gracefully</name>
  <files>writ-dap/src/server/inspection.rs</files>
  <action>
When the user clicks "Continue" in VSCode after inspecting a crash, `handle_continue`
calls `run_until_stop()`. The task is already `Cancelled`, so `run_until_stop` needs to
handle this case.

Check the current `run_until_stop` flow: it calls `runtime.task_state(task_id)` in the
loop. If the task is already Cancelled at the START (before any tick), the current code
tries to resume_debug (which may fail since it's Cancelled, not Suspended), then ticks.
The tick will return AllCompleted/Empty since there are no runnable tasks, emitting
terminated+exited.

Verify this works correctly. If `resume_debug` returns an error for a Cancelled task,
the function returns early without emitting terminated, leaving VSCode hanging.

Fix: In `run_until_stop`, before the resume_debug check, check if the task is already
in a terminal state (Cancelled/Completed). If Cancelled with crash_info, the user has
already inspected it (the stopped event was already sent). Now emit terminated+exited
and return. This prevents trying to resume a non-suspended task.

At the top of `run_until_stop`, after extracting `task_id` and `runtime`, add:
```rust
// If the task is already terminal (e.g., user clicked Continue after crash inspection),
// just emit terminated + exited and return.
if matches!(runtime.task_state(task_id), Some(TaskState::Completed) | Some(TaskState::Cancelled)) {
    let _ = self.server.send_event(Event::Terminated(Some(
        events::TerminatedEventBody { restart: None },
    )));
    let _ = self.server.send_event(Event::Exited(events::ExitedEventBody {
        exit_code: if runtime.crash_info(task_id).is_some() { 1 } else { 0 },
    }));
    return;
}
```

This must come BEFORE the `resume_debug` check (line 33) since a Cancelled task is not
Suspended and resume_debug would fail.

Note: Need to reborrow `runtime` as `self.runtime.as_mut()` for the `self.server` calls
since we need `&mut self`. The cleanest way is to do the terminal check, and if terminal,
drop the runtime borrow, then use `self.server` directly. Structure it as:
```rust
// Check terminal state before attempting resume
if let Some(rt) = self.runtime.as_ref() {
    let state = rt.task_state(task_id);
    if matches!(state, Some(TaskState::Completed) | Some(TaskState::Cancelled)) {
        let exit_code = if rt.crash_info(task_id).is_some() { 1 } else { 0 };
        let _ = self.server.send_event(Event::Terminated(Some(
            events::TerminatedEventBody { restart: None },
        )));
        let _ = self.server.send_event(Event::Exited(events::ExitedEventBody {
            exit_code,
        }));
        return;
    }
}
```
Place this right after `let task_id = match self.task_id { ... }` and before the existing
`let runtime = match self.runtime.as_mut() { ... }`.
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test -p writ-dap --no-fail-fast 2>&1 | tail -20</automated>
  </verify>
  <done>
    Continue-after-crash emits terminated+exited events with exit_code 1 instead of
    hanging or erroring. All existing DAP tests pass. The halt-on-crash integration test
    can cleanly continue and disconnect after crash inspection.
  </done>
</task>

</tasks>

<verification>
Run all DAP tests:
```
cd D:/dev/git/Writ && cargo test -p writ-dap --no-fail-fast
```

All tests pass including new `test_halt_on_crash_inspect`.
</verification>

<success_criteria>
- After a crash, `threads` response contains a thread with matching id and non-"terminated" name
- After a crash, `stackTrace` response contains non-empty stack frames from CrashInfo
- Continue-after-crash emits terminated+exited events cleanly
- New integration test `test_halt_on_crash_inspect` proves the full flow
- All existing DAP tests continue to pass
</success_criteria>

<output>
After completion, create `.planning/quick/260319-mdb-fix-dap-halt-on-crash-test-thread-report/260319-mdb-SUMMARY.md`
</output>
