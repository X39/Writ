---
phase: quick-260319-tpt
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - writ-runtime/src/task.rs
  - writ-runtime/src/scheduler.rs
  - writ-dap/src/server/inspection.rs
  - writ-dap/tests/test_protocol.rs
autonomous: true
requirements: [BREAK-BEFORE-UNWIND]

must_haves:
  truths:
    - "In debug mode, a crash suspends the task with live stack frames instead of immediately unwinding"
    - "In non-debug mode, crash behavior is unchanged (immediate unwind into CrashInfo)"
    - "VSCode shows live stack frames and variables at the crash point when debugging"
    - "Clicking Continue after crash inspection triggers deferred unwind and terminates the session"
  artifacts:
    - path: "writ-runtime/src/task.rs"
      provides: "CrashPending suspend reason variant"
      contains: "CrashPending"
    - path: "writ-runtime/src/scheduler.rs"
      provides: "Debug-gated crash handling"
      contains: "CrashPending"
    - path: "writ-dap/src/server/inspection.rs"
      provides: "CrashPending detection in run_until_stop and resume handling"
      contains: "CrashPending"
  key_links:
    - from: "writ-runtime/src/scheduler.rs"
      to: "writ-runtime/src/task.rs"
      via: "SuspendReason::CrashPending"
      pattern: "SuspendReason::CrashPending"
    - from: "writ-dap/src/server/inspection.rs"
      to: "writ-runtime/src/task.rs"
      via: "Matching on CrashPending to emit stopped(exception) and handle resume->unwind"
      pattern: "CrashPending"
---

<objective>
Implement break-before-unwind for DAP crash debugging: when a runtime crash occurs in debug mode, suspend the task with its live call stack intact instead of immediately unwinding into CrashInfo. This lets VSCode inspect real stack frames, real registers, and real local variables at the crash point. On resume, perform the deferred unwind and terminate.

Purpose: Currently crashes immediately unwind the stack and clone registers into CrashInfo snapshots. This works but the cloned snapshots are a second-class path. Suspending before unwind means the existing primary inspection path (live call_stack + frame_registers) works directly for crash debugging, with zero fallback logic needed.

Output: Modified scheduler, task, DAP inspection code; updated integration test.
</objective>

<execution_context>
@C:/Users/msili/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/msili/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md

Key interfaces from the codebase (executor should use directly, no exploration needed):

From writ-runtime/src/task.rs:
```rust
#[derive(Debug, Clone)]
pub enum SuspendReason {
    HostRequest(crate::host::RequestId),
    Breakpoint { method_idx: u32, pc: u32, line: u32, col: u16 },
    DebugStep { mode: crate::host::DebugAction, method_idx: u32, pc: u32, line: u32, col: u16 },
}
```

From writ-runtime/src/scheduler.rs (lines 139-168, the Crash branch):
```rust
ExecutionResult::Crash(msg) => {
    // Currently: immediately calls execute_crash() which clones registers into
    // CrashInfo, unwinds defers, sets task to Cancelled.
    // The `host` parameter is available here (dyn RuntimeHost).
    // host.debug_enabled() returns true when DAP is attached.
}
```

From writ-runtime/src/dispatch/mod.rs:
```rust
pub(crate) fn execute_crash(
    task: &mut Task, msg: String, modules: &[LoadedModule],
    current_module_idx: usize, dispatch_table: &DispatchTable,
    heap: &mut dyn GcHeap, host: &mut dyn RuntimeHost,
    globals: &mut Vec<Value>, next_request_id: &mut u32,
    entity_registry: &mut EntityRegistry,
);
```

From writ-runtime/src/runtime.rs:
```rust
pub fn resume_debug(&mut self, task_id: TaskId) -> Result<(), RuntimeError> {
    // Clears suspend_reason, sets Ready, pushes to ready_queue
}
pub fn all_task_ids(&self) -> Vec<TaskId> {
    // Returns non-terminal tasks (Ready, Running, Suspended) -- excludes Completed/Cancelled
}
pub fn suspend_reason(&self, task_id: TaskId) -> Option<&SuspendReason> { ... }
```

From writ-dap/src/server/inspection.rs:
```rust
// run_until_stop(): tick loop checks task state after each tick
//   - Suspended + Breakpoint/DebugStep -> stopped event
//   - Cancelled + crash_info -> stopped(exception) event  [THIS IS THE PATH WE'RE REPLACING]
//   - Completed -> terminated + exited

// run_until_stop() also has a pre-check at top:
//   If task is Cancelled with crash_info -> emit terminated + exited (for "Continue after crash")
//   If Suspended -> resume_debug() first

// build_stack_frames(): Falls back to CrashInfo when call_stack is empty
// get_variables(), count_active_locals(), do_evaluate(): All have primary (call_stack) + crash fallback paths
```
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add CrashPending variant and gate crash handling on debug mode</name>
  <files>writ-runtime/src/task.rs, writ-runtime/src/scheduler.rs</files>
  <action>
1. In `writ-runtime/src/task.rs`, add a new variant to `SuspendReason`:
   ```rust
   /// Suspended before crash unwind so the debugger can inspect the live stack.
   /// On resume, the runtime will execute the full crash unwind (defers, cancellation).
   CrashPending { message: String },
   ```
   Also add a unit test `suspend_reason_crash_pending_can_be_constructed` following the pattern of the existing variant tests.

2. In `writ-runtime/src/scheduler.rs`, modify the `ExecutionResult::Crash(msg)` match arm (lines 139-168). Replace the unconditional `execute_crash()` call with a debug gate:

   ```rust
   ExecutionResult::Crash(msg) => {
       // In debug mode, suspend BEFORE unwinding so the debugger can
       // inspect the live call stack and registers at the crash point.
       if host.debug_enabled() {
           let task = self.tasks.get_mut(&task_id).unwrap();
           task.state = TaskState::Suspended;
           task.suspend_reason = Some(SuspendReason::CrashPending {
               message: msg.clone(),
           });
           return Some((task_id, ExecutionResult::DebugSuspend));
       }

       // Non-debug mode: immediate crash unwind (unchanged behavior)
       {
           let task = self.tasks.get_mut(&task_id).unwrap();
           execute_crash(
               task, msg.clone(), modules, current_module_idx, dispatch_table,
               heap, host, &mut self.globals, next_request_id,
               &mut self.entity_registry,
           );
       }
       // ... rest of non-debug crash handling unchanged (cancel children, release locks, wake joiners)
   ```

   The key insight: returning `DebugSuspend` makes the scheduler stop executing this task. The task remains `Suspended` with `SuspendReason::CrashPending`. Its `call_stack` is intact with real registers. `all_task_ids()` will return it (it's Suspended, not Cancelled). All existing DAP inspection methods (stack frames, variables, evaluate) will use the primary live-stack path, not the CrashInfo fallback.

3. In `writ-runtime/src/runtime.rs`, modify `resume_debug()` to detect `CrashPending` and perform deferred unwind instead of normal resume. After clearing suspend_reason and before pushing to ready_queue, check:

   ```rust
   pub fn resume_debug(&mut self, task_id: TaskId) -> Result<(), crate::error::RuntimeError> {
       let task = self.scheduler.tasks.get_mut(&task_id).ok_or_else(|| { ... })?;
       if task.state != TaskState::Suspended {
           return Err(...);
       }

       // Check if this is a deferred crash — if so, execute the crash unwind now
       // instead of resuming normal execution.
       if let Some(SuspendReason::CrashPending { message }) = task.suspend_reason.take() {
           // Perform the full crash unwind (defers, cancellation, CrashInfo)
           crate::dispatch::execute_crash(
               task,
               message,
               &self.domain.modules,
               self.user_module_idx,
               &self.dispatch_table,
               self.heap.as_mut(),
               &mut self.host,
               &mut self.scheduler.globals,
               &mut self.next_request_id,
               &mut self.scheduler.entity_registry,
           );
           // Also cancel scoped children, release locks, wake joiners
           // (mirrors the non-debug crash path in scheduler.rs)
           let children = self.scheduler.tasks.get(&task_id)
               .map(|t| t.scoped_children.clone())
               .unwrap_or_default();
           for child_id in children {
               self.scheduler.cancel_task_tree(
                   child_id, &self.domain.modules, self.user_module_idx,
                   &self.dispatch_table, self.heap.as_mut(), &mut self.host,
                   &mut self.next_request_id,
               );
           }
           let locks: Vec<u32> = self.scheduler.tasks.get(&task_id)
               .map(|t| t.atomic_locks.clone())
               .unwrap_or_default();
           for global_idx in locks {
               self.scheduler.global_locks.remove(&global_idx);
           }
           if let Some(t) = self.scheduler.tasks.get_mut(&task_id) {
               t.atomic_locks.clear();
           }
           self.scheduler.wake_joiners(task_id, None);
           return Ok(());
       }

       // Normal resume path (breakpoint/step)
       task.state = TaskState::Ready;
       task.suspend_reason = None;
       self.scheduler.ready_queue.push_back(task_id);
       Ok(())
   }
   ```

   Note: `cancel_task_tree` is `pub(crate)` on Scheduler — if it's not accessible from Runtime, check visibility. The Runtime already holds `&mut self.scheduler` so it should be callable. Also check if `wake_joiners` is accessible (it's `pub(crate)` on Scheduler, and Runtime owns the scheduler).
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test -p writ-runtime --lib -- task::tests 2>&1 | tail -20</automated>
  </verify>
  <done>SuspendReason::CrashPending variant exists. Scheduler gates crash handling on debug_enabled(). resume_debug() performs deferred crash unwind for CrashPending. Non-debug crash path is unchanged. Unit tests pass.</done>
</task>

<task type="auto">
  <name>Task 2: Update DAP inspection to handle CrashPending suspend reason</name>
  <files>writ-dap/src/server/inspection.rs, writ-dap/tests/test_protocol.rs</files>
  <action>
1. In `writ-dap/src/server/inspection.rs`, update `run_until_stop()`:

   a. **In the tick loop's `Some(TaskState::Suspended)` arm** (line 84-132): Add `CrashPending` to the `is_debug_suspend` match so it's recognized as a debug stop:
      ```rust
      let is_debug_suspend = matches!(
          runtime.suspend_reason(task_id),
          Some(SuspendReason::Breakpoint { .. })
          | Some(SuspendReason::DebugStep { .. })
          | Some(SuspendReason::CrashPending { .. })
      );
      ```

   b. **In the stop reason handling** inside the same block: When `suspend_reason` is `CrashPending`, emit `stopped(exception)` with the crash message as description/text, regardless of what `DebugHost.take_pending_stop()` returns. Add a check BEFORE the existing stop_reason match:
      ```rust
      if is_debug_suspend {
          // Check for CrashPending first -- this doesn't go through DebugHost's
          // stop reason, it's set directly by the scheduler.
          if let Some(SuspendReason::CrashPending { ref message }) = runtime.suspend_reason(task_id) {
              // Emit crash message as stderr output
              let _ = self.server.send_event(Event::Output(events::OutputEventBody {
                  output: format!("Runtime crash: {}\n", message),
                  category: Some(types::OutputEventCategory::Stderr),
                  ..Default::default()
              }));
              let thread_id = self.task_id.map(|t| t.index as i64).unwrap_or(0);
              let _ = self.server.send_event(Event::Stopped(events::StoppedEventBody {
                  reason: types::StoppedEventReason::Exception,
                  description: Some(message.clone()),
                  thread_id: Some(thread_id),
                  preserve_focus_hint: None,
                  text: Some(message.clone()),
                  all_threads_stopped: Some(true),
                  hit_breakpoint_ids: None,
              }));
              return;
          }
          // ... existing stop_reason handling for Breakpoint/DebugStep
      ```

   c. **In the pre-check at the top of run_until_stop()** (lines 29-56): The Cancelled+crash_info early-exit remains as-is (it handles the case after resume_debug has unwound the crash). But update the Suspended resume logic: when `suspend_reason` is `CrashPending` and we call `resume_debug()`, it performs the deferred unwind (task becomes Cancelled). After resume_debug returns, check if the task is now Cancelled and emit terminated+exited:
      ```rust
      // If the task is suspended, resume it.
      if runtime.task_state(task_id) == Some(TaskState::Suspended) {
          // Check if this is a CrashPending resume (user clicked Continue after crash)
          let is_crash_resume = matches!(
              runtime.suspend_reason(task_id),
              Some(SuspendReason::CrashPending { .. })
          );
          if let Err(e) = runtime.resume_debug(task_id) {
              eprintln!("[writ-dap] resume_debug error: {:?}", e);
              return;
          }
          // If we just resumed a CrashPending, the task is now Cancelled (unwound).
          // Emit terminated + exited with code 1 and return.
          if is_crash_resume {
              let _ = self.server.send_event(Event::Terminated(Some(
                  events::TerminatedEventBody { restart: None },
              )));
              let _ = self.server.send_event(Event::Exited(events::ExitedEventBody {
                  exit_code: 1,
              }));
              return;
          }
      }
      ```

2. In `writ-dap/tests/test_protocol.rs`, update `test_halt_on_crash_inspect`:

   a. Update the stackTrace comment from "CrashInfo.stack_trace" to "live stack frames" since frames now come from the live call stack, not CrashInfo snapshots.

   b. Update the "Continue after crash" section comment: task is now Suspended (not Cancelled) when inspection happens, and Continue triggers deferred unwind + terminate.

   c. The actual test assertions should remain the same since the observable behavior is identical:
      - stopped(exception) event with crash message -- still emitted (from CrashPending handler)
      - threads returns non-empty list with non-"terminated" name -- still works (task is Suspended, so all_task_ids returns it; build_thread_list gives it a real method name)
      - stackTrace returns non-empty frames -- still works (live call_stack, primary path)
      - scopes/variables return real locals -- still works (live registers, primary path)
      - Continue emits terminated + exited(1) -- still works (deferred unwind path)

   d. The key behavioral improvement to VERIFY: the thread name should now be the actual method name (e.g., "main" or similar) instead of "main (crashed)" since the task appears in all_task_ids() and goes through build_thread_list(). Update the assertion if the old test was checking for "main (crashed)" -- it should now accept a real method name. Check the current assertion: it only asserts `!= "terminated"`, so "main (crashed)" or a real method name both pass. No change needed.

   e. Add a comment at the top of the test noting this now tests live-stack inspection (break-before-unwind) rather than CrashInfo fallback inspection.
  </action>
  <verify>
    <automated>cd D:/dev/git/Writ && cargo test -p writ-dap --test test_protocol -- test_halt_on_crash_inspect 2>&1 | tail -20</automated>
  </verify>
  <done>DAP run_until_stop recognizes CrashPending as a debug stop, emits stopped(exception) with crash message. Continue after crash triggers deferred unwind via resume_debug and emits terminated+exited(1). Integration test passes with live stack frames (not CrashInfo fallback). All other DAP tests still pass.</done>
</task>

</tasks>

<verification>
Run the full test suites to confirm no regressions:

```bash
# Runtime unit tests (CrashPending variant, scheduler debug gate, resume_debug deferred unwind)
cargo test -p writ-runtime

# DAP integration tests (crash inspection + all other protocol tests)
cargo test -p writ-dap

# Golden tests (crash fixture still works in non-debug mode)
cargo test -p writ-golden
```
</verification>

<success_criteria>
- SuspendReason::CrashPending { message } variant exists in task.rs
- Scheduler suspends task instead of unwinding when host.debug_enabled() is true
- resume_debug() detects CrashPending and performs deferred crash unwind
- Non-debug crash path is completely unchanged (CrashInfo still created, defers still run)
- DAP emits stopped(exception) when CrashPending suspension detected
- DAP emits terminated+exited(1) when user continues after crash inspection
- test_halt_on_crash_inspect passes with live stack frames
- All existing DAP, runtime, and golden tests pass without modification
</success_criteria>

<output>
After completion, create `.planning/quick/260319-tpt-dap-break-before-unwind-suspend-on-crash/260319-tpt-SUMMARY.md`
</output>
