use std::collections::VecDeque;
use rustc_hash::FxHashMap;

use crate::dispatch::{execute_batch, execute_crash, DispatchTable, ExecutionResult};
use crate::entity::EntityRegistry;
use crate::frame::{CallFrame, RegisterPool};
use crate::gc::GcHeap;
use crate::host::RuntimeHost;
use crate::loader::LoadedModule;
use crate::reflection::ReflectionIndex;
use crate::task::{SuspendReason, Task, TaskState};
use crate::value::{pack_task_id, TaskId, Value};

/// Task scheduler managing task lifecycle and execution.
pub struct Scheduler {
    pub(crate) tasks: FxHashMap<TaskId, Task>,
    pub(crate) ready_queue: VecDeque<TaskId>,
    pub(crate) next_task_index: u32,
    pub(crate) globals: Vec<Value>,
    pub(crate) global_locks: FxHashMap<u32, TaskId>,
    /// Tasks waiting to join on another task. Maps target_task_id -> Vec<(waiting_task_id, r_dst)>.
    pub(crate) join_waiters: FxHashMap<TaskId, Vec<(TaskId, u16)>>,
    /// Entity registry for entity lifecycle management.
    pub(crate) entity_registry: EntityRegistry,
    /// Free-list pool for reusing register Vecs across call frames.
    pub(crate) pool: RegisterPool,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: FxHashMap::default(),
            ready_queue: VecDeque::new(),
            next_task_index: 0,
            globals: Vec::new(),
            global_locks: FxHashMap::default(),
            join_waiters: FxHashMap::default(),
            entity_registry: EntityRegistry::new(),
            pool: RegisterPool::new(),
        }
    }

    /// Create a new task with an initial call frame for the given method.
    pub fn create_task(
        &mut self,
        method_idx: usize,
        args: Vec<Value>,
        parent_id: Option<TaskId>,
        module: &LoadedModule,
    ) -> TaskId {
        let id = TaskId::new(self.next_task_index, 0);
        self.next_task_index += 1;

        let reg_count = if method_idx < module.module.method_bodies.len() {
            module.module.method_bodies[method_idx].register_types.len()
        } else {
            args.len().max(1)
        };

        let mut frame = CallFrame::with_pool(&mut self.pool, method_idx, reg_count, 0);
        for (i, arg) in args.into_iter().enumerate() {
            if i < frame.registers.len() {
                frame.registers[i] = arg;
            }
        }

        let mut task = Task::new(id, frame);
        task.parent_id = parent_id;
        self.tasks.insert(id, task);
        self.ready_queue.push_back(id);
        id
    }

    /// Run the next ready task until it completes, suspends, crashes, or hits the limit.
    #[allow(clippy::too_many_arguments)] // scheduler passes full execution context through to dispatch
    pub(crate) fn run_one_task(
        &mut self,
        modules: &[LoadedModule],
        current_module_idx: usize,
        dispatch_table: &DispatchTable,
        heap: &mut dyn GcHeap,
        host: &mut dyn RuntimeHost,
        limit: u64,
        next_request_id: &mut u32,
        reflection: &mut ReflectionIndex,
    ) -> Option<(TaskId, ExecutionResult)> {
        let task_id = self.ready_queue.pop_front()?;

        {
            let task = self.tasks.get_mut(&task_id)?;
            task.state = TaskState::Running;
        }

        loop {
            let result = {
                let task = self.tasks.get_mut(&task_id).unwrap();
                execute_batch(
                    task,
                    modules,
                    current_module_idx,
                    dispatch_table,
                    heap,
                    host,
                    &mut self.globals,
                    next_request_id,
                    &mut self.entity_registry,
                    &mut self.pool,
                    reflection,
                    limit,
                )
            };

            match result {
                ExecutionResult::Continue => continue,
                ExecutionResult::Completed(val) => {
                    let task = self.tasks.get_mut(&task_id).unwrap();
                    task.state = TaskState::Completed;
                    task.return_value = Some(val.clone());
                    // Wake any tasks waiting to JOIN this one
                    self.wake_joiners(task_id, Some(val.clone()));
                    return Some((task_id, ExecutionResult::Completed(val)));
                }
                ExecutionResult::Suspended(req_id) => {
                    let task = self.tasks.get_mut(&task_id).unwrap();
                    task.state = TaskState::Suspended;
                    task.suspend_reason = Some(SuspendReason::HostRequest(req_id));
                    return Some((task_id, ExecutionResult::Suspended(req_id)));
                }
                ExecutionResult::DebugSuspend => {
                    // Task already has state=Suspended and suspend_reason set by dispatch.
                    // Stop executing this task — the DAP server will call resume_debug() to resume it.
                    return Some((task_id, ExecutionResult::DebugSuspend));
                }
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

                    // Non-debug mode: immediate crash unwind (unchanged behavior).
                    // Use the full crash unwinding engine with defer execution
                    {
                        let task = self.tasks.get_mut(&task_id).unwrap();
                        execute_crash(
                            task, msg.clone(), modules, current_module_idx, dispatch_table, heap, host,
                            &mut self.globals, next_request_id,
                            &mut self.entity_registry, &mut self.pool, reflection,
                        );
                    }
                    // Cancel scoped children
                    let children = self.tasks.get(&task_id)
                        .map(|t| t.scoped_children.clone())
                        .unwrap_or_default();
                    for child_id in children {
                        self.cancel_task_tree(child_id, modules, current_module_idx, dispatch_table, heap, host, next_request_id, reflection);
                    }
                    // Release any global locks held by this task
                    let locks: Vec<u32> = self.tasks.get(&task_id)
                        .map(|t| t.atomic_locks.clone())
                        .unwrap_or_default();
                    for global_idx in locks {
                        self.global_locks.remove(&global_idx);
                    }
                    if let Some(t) = self.tasks.get_mut(&task_id) {
                        t.atomic_locks.clear();
                    }
                    // Wake any tasks waiting to JOIN this one
                    self.wake_joiners(task_id, None);
                    return Some((task_id, ExecutionResult::Crash(msg)));
                }
                ExecutionResult::LimitReached => {
                    let task = self.tasks.get_mut(&task_id).unwrap();
                    task.state = TaskState::Ready;
                    self.ready_queue.push_back(task_id);
                    return Some((task_id, ExecutionResult::LimitReached));
                }
                ExecutionResult::DeferComplete => {
                    // Should not happen during normal execution
                    continue;
                }

                // ── Concurrency results handled by scheduler ──
                ExecutionResult::SpawnChild { r_dst, method_idx, args } => {
                    let child_id = self.create_task(
                        method_idx, args, Some(task_id), &modules[current_module_idx],
                    );
                    // Add child to parent's scoped_children and store result
                    if let Some(parent) = self.tasks.get_mut(&task_id) {
                        parent.scoped_children.push(child_id);
                        if let Some(frame) = parent.call_stack.last_mut() {
                            frame.registers[r_dst as usize] = pack_task_id(child_id);
                        }
                    }
                    continue;
                }
                ExecutionResult::SpawnDetachedTask { r_dst, method_idx, args } => {
                    let child_id = self.create_task(
                        method_idx, args, None, &modules[current_module_idx],
                    );
                    if let Some(parent) = self.tasks.get_mut(&task_id)
                        && let Some(frame) = parent.call_stack.last_mut() {
                            frame.registers[r_dst as usize] = pack_task_id(child_id);
                        }
                    continue;
                }
                ExecutionResult::JoinTask { r_dst, target } => {
                    // Check if target is already terminal
                    let target_info = self.tasks.get(&target)
                        .map(|t| (t.state, t.return_value.clone()));
                    match target_info {
                        Some((TaskState::Completed, ret_val)) | Some((TaskState::Cancelled, ret_val)) => {
                            let task = self.tasks.get_mut(&task_id).unwrap();
                            if let Some(frame) = task.call_stack.last_mut() {
                                frame.registers[r_dst as usize] = ret_val.unwrap_or(Value::Void);
                            }
                            continue;
                        }
                        Some(_) => {
                            // Target still running — suspend the joining task
                            let task = self.tasks.get_mut(&task_id).unwrap();
                            task.state = TaskState::Suspended;
                            self.join_waiters
                                .entry(target)
                                .or_default()
                                .push((task_id, r_dst));
                            return Some((task_id, ExecutionResult::Suspended(
                                crate::host::RequestId(0),
                            )));
                        }
                        None => {
                            // Target doesn't exist — just return Void
                            let task = self.tasks.get_mut(&task_id).unwrap();
                            if let Some(frame) = task.call_stack.last_mut() {
                                frame.registers[r_dst as usize] = Value::Void;
                            }
                            continue;
                        }
                    }
                }
                ExecutionResult::CancelTask { target } => {
                    self.cancel_task_tree(target, modules, current_module_idx, dispatch_table, heap, host, next_request_id, reflection);
                    continue;
                }
            }
        }
    }

    /// Cancel a task and all its scoped children recursively.
    /// Executes defer handlers at each frame level during unwinding.
    #[allow(clippy::too_many_arguments)] // cancellation requires full context for defer handler execution
    pub(crate) fn cancel_task_tree(
        &mut self,
        task_id: TaskId,
        modules: &[LoadedModule],
        current_module_idx: usize,
        dispatch_table: &DispatchTable,
        heap: &mut dyn GcHeap,
        host: &mut dyn RuntimeHost,
        next_request_id: &mut u32,
        reflection: &mut crate::reflection::ReflectionIndex,
    ) {
        // Get children first (depth-first)
        let children = self.tasks.get(&task_id)
            .map(|t| t.scoped_children.clone())
            .unwrap_or_default();

        // Cancel children first
        for child_id in children {
            self.cancel_task_tree(child_id, modules, current_module_idx, dispatch_table, heap, host, next_request_id, reflection);
        }

        // Cancel this task
        {
            let task = match self.tasks.get_mut(&task_id) {
                Some(t) => t,
                None => return,
            };

            if matches!(task.state, TaskState::Completed | TaskState::Cancelled) {
                return; // Already terminal
            }

            // Full crash unwind with defers
            execute_crash(
                task,
                "task cancelled".into(),
                modules, current_module_idx, dispatch_table, heap, host, &mut self.globals, next_request_id,
                &mut self.entity_registry, &mut self.pool, reflection,
            );
        }

        // Release global locks
        let locks: Vec<u32> = self.tasks.get(&task_id)
            .map(|t| t.atomic_locks.clone())
            .unwrap_or_default();
        for global_idx in locks {
            self.global_locks.remove(&global_idx);
        }
        if let Some(t) = self.tasks.get_mut(&task_id) {
            t.atomic_locks.clear();
        }

        // Remove from ready queue
        self.ready_queue.retain(|id| *id != task_id);

        // Wake any tasks waiting to JOIN this one
        self.wake_joiners(task_id, None);
    }

    /// Wake all tasks waiting to JOIN the given task.
    pub(crate) fn wake_joiners(&mut self, target_id: TaskId, return_value: Option<Value>) {
        if let Some(waiters) = self.join_waiters.remove(&target_id) {
            for (waiter_id, r_dst) in waiters {
                if let Some(waiter) = self.tasks.get_mut(&waiter_id)
                    && waiter.state == TaskState::Suspended {
                        waiter.state = TaskState::Ready;
                        waiter.pending_request = None;
                        if let Some(frame) = waiter.call_stack.last_mut() {
                            frame.registers[r_dst as usize] = return_value.clone().unwrap_or(Value::Void);
                        }
                        self.ready_queue.push_back(waiter_id);
                    }
            }
        }
    }

    /// Get the state of a task.
    pub fn task_state(&self, task_id: TaskId) -> Option<TaskState> {
        self.tasks.get(&task_id).map(|t| t.state)
    }

    /// Schedule a finalizer task for the given method with a self argument.
    ///
    /// The task is detached (no parent) and runs the on_finalize hook.
    pub fn schedule_finalizer(
        &mut self,
        method_idx: usize,
        self_arg: Value,
        module: &LoadedModule,
    ) -> TaskId {
        self.create_task(method_idx, vec![self_arg], None, module)
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
