use crate::domain::Domain;
use crate::dispatch::DispatchTable;
use crate::entity::EntityRegistry;
use crate::error::{CrashInfo, RuntimeError};
use crate::gc::{GcHeap, GcStats, MarkSweepHeap};
use crate::heap::BumpHeap;
use crate::host::{HostRequest, HostResponse, NullHost, RequestId, RuntimeHost};
use crate::scheduler::Scheduler;
use crate::task::{SuspendReason, TaskState};
use crate::value::{HeapRef, TaskId, Value};

/// Execution budget for a tick.
#[derive(Debug, Clone, Copy)]
pub enum ExecutionLimit {
    /// Maximum number of instructions per task per tick.
    Instructions(u64),
    /// No limit -- run until all tasks complete or suspend.
    None,
}

/// Result of a single tick.
#[derive(Debug)]
pub enum TickResult {
    /// All tasks have completed or been cancelled.
    AllCompleted,
    /// Some tasks are suspended waiting for host responses.
    TasksSuspended(Vec<PendingRequest>),
    /// Execution budget was exhausted with tasks still ready.
    ExecutionLimitReached,
    /// No tasks exist in the scheduler.
    Empty,
}

/// A pending host request from a suspended task.
#[derive(Debug, Clone)]
pub struct PendingRequest {
    pub task_id: TaskId,
    pub request_id: RequestId,
    pub request: HostRequest,
}

/// Builder for constructing a Runtime with configurable host.
pub struct RuntimeBuilder<H: RuntimeHost = NullHost> {
    module: writ_module::Module,
    host: H,
    use_gc: bool,
}

impl RuntimeBuilder<NullHost> {
    /// Create a new builder with the given module and a default NullHost.
    pub fn new(module: writ_module::Module) -> Self {
        RuntimeBuilder {
            module,
            host: NullHost,
            use_gc: false,
        }
    }
}

impl<H: RuntimeHost> RuntimeBuilder<H> {
    /// Replace the host with a different implementation.
    pub fn with_host<H2: RuntimeHost>(self, host: H2) -> RuntimeBuilder<H2> {
        RuntimeBuilder {
            module: self.module,
            host,
            use_gc: self.use_gc,
        }
    }

    /// Use MarkSweepHeap instead of the default BumpHeap.
    pub fn with_gc(mut self) -> Self {
        self.use_gc = true;
        self
    }

    /// Build the Runtime, loading the virtual module and user module into a Domain.
    pub fn build(self) -> Result<Runtime<H>, RuntimeError> {
        let mut domain = Domain::new();

        // Add virtual module at index 0
        domain.add_module(crate::virtual_module::build_writ_runtime_module())?;
        // Add user module at index 1
        let user_idx = domain.add_module(self.module)?;
        // Resolve cross-module references
        domain.resolve_refs()?;
        // Build dispatch table
        let dispatch_table = domain.build_dispatch_table();

        let user_module = &domain.modules[user_idx];
        let global_count = user_module.module.global_defs.len();

        let mut scheduler = Scheduler::new();
        scheduler.globals = vec![Value::Void; global_count];

        let heap: Box<dyn GcHeap> = if self.use_gc {
            Box::new(MarkSweepHeap::new())
        } else {
            Box::new(BumpHeap::new())
        };

        Ok(Runtime {
            domain,
            dispatch_table,
            user_module_idx: user_idx,
            scheduler,
            heap,
            host: self.host,
            next_request_id: 1,
        })
    }
}

/// The main runtime entry point. Manages module execution, task scheduling,
/// and host communication.
pub struct Runtime<H: RuntimeHost = NullHost> {
    pub(crate) domain: Domain,
    pub(crate) dispatch_table: DispatchTable,
    pub(crate) user_module_idx: usize,
    pub(crate) scheduler: Scheduler,
    pub(crate) heap: Box<dyn GcHeap>,
    pub(crate) host: H,
    pub(crate) next_request_id: u32,
}

impl<H: RuntimeHost> Runtime<H> {
    /// Execute one tick of the runtime. Runs ready tasks within the given budget.
    ///
    /// Returns a TickResult describing the outcome:
    /// - AllCompleted: all tasks finished or were cancelled
    /// - TasksSuspended: some tasks are waiting for host responses
    /// - ExecutionLimitReached: budget exhausted with tasks still ready
    /// - Empty: no tasks in the scheduler
    pub fn tick(&mut self, _delta_time: f64, limit: ExecutionLimit) -> TickResult {
        if self.scheduler.tasks.is_empty() {
            return TickResult::Empty;
        }

        let per_task_limit = match limit {
            ExecutionLimit::Instructions(n) => n,
            ExecutionLimit::None => 0, // 0 means no limit in run_one_task
        };

        // Run all ready tasks (one pass through the queue)
        let mut ran_any = false;
        let initial_ready = self.scheduler.ready_queue.len();
        for _ in 0..initial_ready {
            if self.scheduler.ready_queue.is_empty() {
                break;
            }
            let result = self.scheduler.run_one_task(
                &self.domain.modules,
                self.user_module_idx,
                &self.dispatch_table,
                self.heap.as_mut(),
                &mut self.host,
                per_task_limit,
                &mut self.next_request_id,
            );
            if result.is_some() {
                ran_any = true;
            }
        }

        // Determine tick result
        self.classify_tick_result(ran_any)
    }

    /// Classify the current scheduler state into a TickResult.
    fn classify_tick_result(&self, ran_any: bool) -> TickResult {
        let has_ready = !self.scheduler.ready_queue.is_empty();
        let mut pending = Vec::new();
        let mut has_non_terminal = false;

        for task in self.scheduler.tasks.values() {
            match task.state {
                TaskState::Ready | TaskState::Running => {
                    has_non_terminal = true;
                }
                TaskState::Suspended => {
                    has_non_terminal = true;
                    if let Some((req_id, ref req)) = task.pending_request {
                        pending.push(PendingRequest {
                            task_id: task.id,
                            request_id: req_id,
                            request: req.clone(),
                        });
                    }
                }
                TaskState::Completed | TaskState::Cancelled => {}
            }
        }

        if has_ready {
            TickResult::ExecutionLimitReached
        } else if !pending.is_empty() {
            TickResult::TasksSuspended(pending)
        } else if !has_non_terminal {
            TickResult::AllCompleted
        } else if !ran_any {
            TickResult::Empty
        } else {
            TickResult::AllCompleted
        }
    }

    /// Confirm a pending host request, resuming the suspended task.
    pub fn confirm(
        &mut self,
        request_id: RequestId,
        response: HostResponse,
    ) -> Result<(), RuntimeError> {
        // Find the task with this pending request
        let task_id = self
            .scheduler
            .tasks
            .values()
            .find(|t| {
                t.state == TaskState::Suspended
                    && t.pending_request
                        .as_ref()
                        .is_some_and(|(id, _)| *id == request_id)
            })
            .map(|t| t.id);

        let task_id = task_id.ok_or_else(|| {
            RuntimeError::ExecutionError(format!(
                "no suspended task found for request {:?}",
                request_id
            ))
        })?;

        let task = self.scheduler.tasks.get_mut(&task_id).unwrap();

        // Deliver the response value to the task's frame
        match &response {
            HostResponse::Value(val) => {
                if let Some(frame) = task.call_stack.last_mut() {
                    frame.registers[0] = val.clone();
                }
            }
            HostResponse::EntityHandle(eid) => {
                if let Some(frame) = task.call_stack.last_mut() {
                    frame.registers[0] = Value::Entity(*eid);
                }
            }
            HostResponse::Confirmed => {
                // No value to deliver
            }
            HostResponse::Error(e) => {
                // Set task to cancelled on error
                task.state = TaskState::Cancelled;
                task.pending_request = None;
                return Err(RuntimeError::ExecutionError(format!(
                    "host request failed: {:?}",
                    e
                )));
            }
        }

        task.state = TaskState::Ready;
        task.pending_request = None;
        task.suspend_reason = None;
        self.scheduler.ready_queue.push_back(task_id);
        Ok(())
    }

    /// Resume a task that was suspended for debug (breakpoint or step).
    ///
    /// The DAP server calls this after the user issues a Continue, Step, or similar command.
    /// For normal debug suspensions, clears `suspend_reason` and puts the task back in the
    /// ready queue. For `CrashPending` suspensions, performs the deferred crash unwind
    /// (executes defers, sets Cancelled, stores CrashInfo) instead of re-queuing.
    pub fn resume_debug(&mut self, task_id: TaskId) -> Result<(), crate::error::RuntimeError> {
        let task = self.scheduler.tasks.get_mut(&task_id).ok_or_else(|| {
            crate::error::RuntimeError::ExecutionError(format!(
                "no task found with id {:?}",
                task_id
            ))
        })?;

        if task.state != TaskState::Suspended {
            return Err(crate::error::RuntimeError::ExecutionError(format!(
                "task {:?} is not suspended (state: {:?})",
                task_id, task.state
            )));
        }

        // Check if this is a deferred crash — if so, execute the crash unwind now
        // instead of resuming normal execution.
        if let Some(SuspendReason::CrashPending { message }) = task.suspend_reason.take() {
            // Perform the full crash unwind (defers, cancellation, CrashInfo).
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

            // Cancel scoped children, mirrors the non-debug crash path in scheduler.rs.
            let children = self.scheduler.tasks.get(&task_id)
                .map(|t| t.scoped_children.clone())
                .unwrap_or_default();
            for child_id in children {
                self.scheduler.cancel_task_tree(
                    child_id,
                    &self.domain.modules,
                    self.user_module_idx,
                    &self.dispatch_table,
                    self.heap.as_mut(),
                    &mut self.host,
                    &mut self.next_request_id,
                );
            }

            // Release any global locks held by this task.
            let locks: Vec<u32> = self.scheduler.tasks.get(&task_id)
                .map(|t| t.atomic_locks.clone())
                .unwrap_or_default();
            for global_idx in locks {
                self.scheduler.global_locks.remove(&global_idx);
            }
            if let Some(t) = self.scheduler.tasks.get_mut(&task_id) {
                t.atomic_locks.clear();
            }

            // Wake any tasks waiting to JOIN this one.
            self.scheduler.wake_joiners(task_id, None);
            return Ok(());
        }

        // Normal resume path (breakpoint/step): put task back in the ready queue.
        task.state = TaskState::Ready;
        task.suspend_reason = None;
        self.scheduler.ready_queue.push_back(task_id);
        Ok(())
    }

    /// Spawn a new task that will begin executing at the given method index.
    pub fn spawn_task(
        &mut self,
        method_idx: usize,
        args: Vec<Value>,
    ) -> Result<TaskId, RuntimeError> {
        let user_module = &self.domain.modules[self.user_module_idx];
        if method_idx >= user_module.decoded_bodies.len() {
            return Err(RuntimeError::ExecutionError(format!(
                "method index {} out of range",
                method_idx
            )));
        }
        let task_id =
            self.scheduler
                .create_task(method_idx, args, None, user_module);
        Ok(task_id)
    }

    /// Get the current state of a task.
    pub fn task_state(&self, task_id: TaskId) -> Option<TaskState> {
        self.scheduler.task_state(task_id)
    }

    /// Read a register value from a task's top call frame.
    pub fn register_value(&self, task_id: TaskId, reg: u16) -> Option<Value> {
        self.scheduler.tasks.get(&task_id).and_then(|t| {
            t.call_stack
                .last()
                .and_then(|f| f.registers.get(reg as usize).cloned())
        })
    }

    /// Get the call stack depth of a task.
    pub fn call_depth(&self, task_id: TaskId) -> Option<usize> {
        self.scheduler
            .tasks
            .get(&task_id)
            .map(|t| t.call_stack.len())
    }

    /// Get the return value of a completed task.
    pub fn return_value(&self, task_id: TaskId) -> Option<Value> {
        self.scheduler
            .tasks
            .get(&task_id)
            .and_then(|t| t.return_value.clone())
    }

    /// Run a specific task within the given budget.
    pub fn run_task(&mut self, task_id: TaskId, limit: ExecutionLimit) -> TickResult {
        let per_task_limit = match limit {
            ExecutionLimit::Instructions(n) => n,
            ExecutionLimit::None => 0,
        };

        // Move the task to the front of the ready queue if it's ready
        let task_state = self.scheduler.task_state(task_id);
        if task_state != Some(TaskState::Ready) {
            return TickResult::Empty;
        }

        // Remove from wherever it is in the queue and put at front
        self.scheduler.ready_queue.retain(|id| *id != task_id);
        self.scheduler.ready_queue.push_front(task_id);

        // Run just this task
        self.scheduler.run_one_task(
            &self.domain.modules,
            self.user_module_idx,
            &self.dispatch_table,
            self.heap.as_mut(),
            &mut self.host,
            per_task_limit,
            &mut self.next_request_id,
        );

        self.classify_tick_result(true)
    }

    /// Run a method to completion synchronously, ignoring execution limits.
    /// Returns the return value on success, or CrashInfo on crash.
    pub fn call_sync(
        &mut self,
        method_idx: usize,
        args: Vec<Value>,
    ) -> Result<Value, CrashInfo> {
        let user_module = &self.domain.modules[self.user_module_idx];
        if method_idx >= user_module.decoded_bodies.len() {
            return Err(CrashInfo {
                message: format!("method index {} out of range", method_idx),
                stack_trace: vec![],
            });
        }

        let task_id = self
            .scheduler
            .create_task(method_idx, args, None, user_module);

        // Run until completion (no limit)
        loop {
            // Move task to front of ready queue
            self.scheduler.ready_queue.retain(|id| *id != task_id);
            if self.scheduler.task_state(task_id) == Some(TaskState::Ready) {
                self.scheduler.ready_queue.push_front(task_id);
            }

            let result = self.scheduler.run_one_task(
                &self.domain.modules,
                self.user_module_idx,
                &self.dispatch_table,
                self.heap.as_mut(),
                &mut self.host,
                0, // no limit
                &mut self.next_request_id,
            );

            match self.scheduler.task_state(task_id) {
                Some(TaskState::Completed) => {
                    let ret = self.scheduler.tasks.get(&task_id)
                        .and_then(|t| t.return_value.clone())
                        .unwrap_or(Value::Void);
                    return Ok(ret);
                }
                Some(TaskState::Cancelled) => {
                    let crash = self.scheduler.tasks.get(&task_id)
                        .and_then(|t| t.crash_info.clone())
                        .unwrap_or(CrashInfo {
                            message: "task cancelled".into(),
                            stack_trace: vec![],
                        });
                    return Err(crash);
                }
                _ => {
                    // If run_one_task returned None, the task isn't in the ready queue
                    if result.is_none() {
                        return Err(CrashInfo {
                            message: "task could not be scheduled".into(),
                            stack_trace: vec![],
                        });
                    }
                }
            }
        }
    }

    /// Get a reference to the heap (for testing/inspection).
    pub fn heap(&self) -> &dyn GcHeap {
        self.heap.as_ref()
    }

    /// Get a mutable reference to the heap.
    pub fn heap_mut(&mut self) -> &mut dyn GcHeap {
        self.heap.as_mut()
    }

    /// Get a reference to the entity registry.
    pub fn entity_registry(&self) -> &EntityRegistry {
        &self.scheduler.entity_registry
    }

    /// Get a mutable reference to the entity registry.
    pub fn entity_registry_mut(&mut self) -> &mut EntityRegistry {
        &mut self.scheduler.entity_registry
    }

    /// Trigger garbage collection. Host-controlled, Manual mode (GC-04).
    ///
    /// Collects roots from all task registers, globals, and entity data_refs.
    /// Calls `heap.collect(roots)`, reports stats via `host.on_gc_complete()`,
    /// and drains the finalization queue.
    pub fn collect_garbage(&mut self) -> GcStats {
        let roots = self.collect_roots();
        let stats = self.heap.collect(&roots);

        // Report to host
        self.host.on_gc_complete(&stats);

        // Drain the finalization queue
        let _finalization_queue = self.heap.drain_finalization_queue();

        // TODO: For each HeapRef in finalization_queue, look up
        // the on_finalize hook method for the object's type and schedule
        // a finalizer task via scheduler.schedule_finalizer().

        stats
    }

    /// Collect all heap references that are roots for GC.
    fn collect_roots(&self) -> Vec<HeapRef> {
        use crate::gc::collect_value_refs;
        let mut roots = Vec::new();

        // Task registers (all frames in all tasks)
        for task in self.scheduler.tasks.values() {
            for frame in &task.call_stack {
                for reg in &frame.registers {
                    collect_value_refs(reg, &mut roots);
                }
            }
            // Also check return_value for completed tasks
            if let Some(ref rv) = task.return_value {
                collect_value_refs(rv, &mut roots);
            }
        }

        // Globals
        for global in &self.scheduler.globals {
            collect_value_refs(global, &mut roots);
        }

        // Entity data refs for alive entities
        for (_entity_id, slot) in self.scheduler.entity_registry.alive_entities() {
            if let Some(href) = slot.data_ref {
                roots.push(href);
            }
        }

        roots
    }

    /// Get the crash info for a crashed/cancelled task.
    pub fn crash_info(&self, task_id: TaskId) -> Option<&CrashInfo> {
        self.scheduler.tasks.get(&task_id)
            .and_then(|t| t.crash_info.as_ref())
    }

    /// Get the number of tasks in the scheduler.
    pub fn task_count(&self) -> usize {
        self.scheduler.tasks.len()
    }

    /// Get a reference to the host (for testing/inspection).
    pub fn host(&self) -> &H {
        &self.host
    }

    /// Get a mutable reference to the host.
    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }

    /// Get a reference to the dispatch table (for testing/inspection).
    pub fn dispatch_table(&self) -> &DispatchTable {
        &self.dispatch_table
    }

    /// Get a reference to the domain (for testing/inspection).
    pub fn domain(&self) -> &Domain {
        &self.domain
    }

    /// The user module index in the domain.
    pub fn user_module_idx(&self) -> usize {
        self.user_module_idx
    }

    /// Get the suspend reason for a task (used by the DAP server).
    ///
    /// Returns `None` if the task does not exist or has no suspend reason set.
    pub fn suspend_reason(&self, task_id: TaskId) -> Option<&SuspendReason> {
        self.scheduler.tasks.get(&task_id)
            .and_then(|t| t.suspend_reason.as_ref())
    }

    /// Get a snapshot of all call stack frames for a task (used by the DAP server).
    ///
    /// Returns `(method_idx, pc)` pairs ordered from bottom (oldest) to top (newest frame).
    /// Returns `None` if the task does not exist.
    pub fn call_stack_frames(&self, task_id: TaskId) -> Option<Vec<(usize, usize)>> {
        self.scheduler.tasks.get(&task_id)
            .map(|t| t.call_stack.iter().map(|f| (f.method_idx, f.pc)).collect())
    }

    /// Get a clone of all registers for a specific call frame of a task.
    ///
    /// `frame_index` 0 is the bottom (oldest) frame; `frame_index N-1` is the top (innermost).
    /// Returns `None` if the task does not exist or the frame index is out of range.
    pub fn frame_registers(&self, task_id: TaskId, frame_index: usize) -> Option<Vec<Value>> {
        self.scheduler.tasks.get(&task_id)
            .and_then(|t| t.call_stack.get(frame_index))
            .map(|f| f.registers.clone())
    }

    /// Return the IDs of all non-terminal tasks (Ready, Running, or Suspended).
    ///
    /// Excludes tasks in Completed or Cancelled states.
    pub fn all_task_ids(&self) -> Vec<TaskId> {
        self.scheduler.tasks.values()
            .filter(|t| !matches!(t.state, TaskState::Completed | TaskState::Cancelled))
            .map(|t| t.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::CallFrame;
    use crate::host::NullHost;
    use crate::task::Task;

    fn make_runtime() -> Runtime<NullHost> {
        let module = writ_module::Module::new();
        RuntimeBuilder::new(module).build().expect("build runtime")
    }

    /// Insert a task directly into the scheduler, bypassing method index checks.
    /// Returns the TaskId. The frame has `reg_count` registers initialized to Void,
    /// with optional arg placed in register 0.
    fn insert_task(rt: &mut Runtime<NullHost>, reg_count: usize, arg0: Option<Value>) -> TaskId {
        let id = TaskId::new(rt.scheduler.next_task_index, 0);
        rt.scheduler.next_task_index += 1;
        let mut frame = CallFrame::new(0, reg_count, 0);
        if let Some(v) = arg0 {
            frame.registers[0] = v;
        }
        let task = Task::new(id, frame);
        rt.scheduler.tasks.insert(id, task);
        rt.scheduler.ready_queue.push_back(id);
        id
    }

    #[test]
    fn test_frame_registers_returns_registers_for_frame0() {
        let mut rt = make_runtime();
        let task_id = insert_task(&mut rt, 4, Some(Value::Int(7)));
        let regs = rt.frame_registers(task_id, 0).expect("should have frame 0");
        assert_eq!(regs.len(), 4);
        assert_eq!(regs[0], Value::Int(7));
    }

    #[test]
    fn test_frame_registers_returns_none_for_out_of_range_index() {
        let mut rt = make_runtime();
        let task_id = insert_task(&mut rt, 2, None);
        // Frame 1 doesn't exist — call stack has only 1 frame
        let result = rt.frame_registers(task_id, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_frame_registers_returns_none_for_invalid_task_id() {
        let rt = make_runtime();
        let fake_id = TaskId::new(999, 999);
        assert!(rt.frame_registers(fake_id, 0).is_none());
    }

    #[test]
    fn test_all_task_ids_returns_spawned_task() {
        let mut rt = make_runtime();
        let task_id = insert_task(&mut rt, 1, None);
        let ids = rt.all_task_ids();
        assert!(ids.contains(&task_id));
    }

    #[test]
    fn test_all_task_ids_empty_when_no_tasks() {
        let rt = make_runtime();
        assert!(rt.all_task_ids().is_empty());
    }

    #[test]
    fn test_all_task_ids_excludes_completed_tasks() {
        let mut rt = make_runtime();
        let task_id = insert_task(&mut rt, 1, None);
        rt.scheduler.tasks.get_mut(&task_id).unwrap().state = TaskState::Completed;
        let ids = rt.all_task_ids();
        assert!(!ids.contains(&task_id));
    }

    #[test]
    fn test_all_task_ids_excludes_cancelled_tasks() {
        let mut rt = make_runtime();
        let task_id = insert_task(&mut rt, 1, None);
        rt.scheduler.tasks.get_mut(&task_id).unwrap().state = TaskState::Cancelled;
        let ids = rt.all_task_ids();
        assert!(!ids.contains(&task_id));
    }

    #[test]
    fn test_all_task_ids_includes_suspended_tasks() {
        let mut rt = make_runtime();
        let task_id = insert_task(&mut rt, 1, None);
        rt.scheduler.tasks.get_mut(&task_id).unwrap().state = TaskState::Suspended;
        let ids = rt.all_task_ids();
        assert!(ids.contains(&task_id));
    }
}
