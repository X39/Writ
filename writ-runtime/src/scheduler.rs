use std::collections::{HashMap, VecDeque};

use crate::dispatch::{execute_crash, execute_one, DispatchTable, ExecutionResult};
use crate::entity::EntityRegistry;
use crate::frame::CallFrame;
use crate::gc::GcHeap;
use crate::host::RuntimeHost;
use crate::loader::LoadedModule;
use crate::task::{Task, TaskState};
use crate::value::{pack_task_id, TaskId, Value};

/// Task scheduler managing task lifecycle and execution.
pub struct Scheduler {
    pub(crate) tasks: HashMap<TaskId, Task>,
    pub(crate) ready_queue: VecDeque<TaskId>,
    pub(crate) next_task_index: u32,
    pub(crate) globals: Vec<Value>,
    pub(crate) global_locks: HashMap<u32, TaskId>,
    /// Tasks waiting to join on another task. Maps target_task_id -> Vec<(waiting_task_id, r_dst)>.
    pub(crate) join_waiters: HashMap<TaskId, Vec<(TaskId, u16)>>,
    /// Entity registry for entity lifecycle management.
    pub(crate) entity_registry: EntityRegistry,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            ready_queue: VecDeque::new(),
            next_task_index: 0,
            globals: Vec::new(),
            global_locks: HashMap::new(),
            join_waiters: HashMap::new(),
            entity_registry: EntityRegistry::new(),
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

        let mut frame = CallFrame::new(method_idx, reg_count, 0);
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
    pub(crate) fn run_one_task(
        &mut self,
        modules: &[LoadedModule],
        current_module_idx: usize,
        dispatch_table: &DispatchTable,
        heap: &mut dyn GcHeap,
        host: &mut dyn RuntimeHost,
        limit: u64,
        next_request_id: &mut u32,
    ) -> Option<(TaskId, ExecutionResult)> {
        let task_id = self.ready_queue.pop_front()?;

        {
            let task = match self.tasks.get_mut(&task_id) {
                Some(t) => t,
                None => return None,
            };
            task.state = TaskState::Running;
        }

        let mut instructions_run: u64 = 0;

        loop {
            // Check execution limit (skip if in atomic section)
            {
                let task = self.tasks.get(&task_id).unwrap();
                if task.atomic_depth == 0 && limit > 0 && instructions_run >= limit {
                    let task = self.tasks.get_mut(&task_id).unwrap();
                    task.state = TaskState::Ready;
                    self.ready_queue.push_back(task_id);
                    return Some((task_id, ExecutionResult::LimitReached));
                }
            }

            let result = {
                let task = self.tasks.get_mut(&task_id).unwrap();
                execute_one(
                    task,
                    modules,
                    current_module_idx,
                    dispatch_table,
                    heap,
                    host,
                    &mut self.globals,
                    next_request_id,
                    &mut self.entity_registry,
                )
            };
            instructions_run += 1;

            match result {
                ExecutionResult::Continue => continue,
                ExecutionResult::Completed(val) => {
                    let task = self.tasks.get_mut(&task_id).unwrap();
                    task.state = TaskState::Completed;
                    task.return_value = Some(val);
                    // Wake any tasks waiting to JOIN this one
                    self.wake_joiners(task_id, Some(val));
                    return Some((task_id, ExecutionResult::Completed(val)));
                }
                ExecutionResult::Suspended(req_id) => {
                    let task = self.tasks.get_mut(&task_id).unwrap();
                    task.state = TaskState::Suspended;
                    return Some((task_id, ExecutionResult::Suspended(req_id)));
                }
                ExecutionResult::Crash(msg) => {
                    // Use the full crash unwinding engine with defer execution
                    {
                        let task = self.tasks.get_mut(&task_id).unwrap();
                        execute_crash(
                            task, msg.clone(), modules, current_module_idx, dispatch_table, heap, host,
                            &mut self.globals, next_request_id,
                            &mut self.entity_registry,
                        );
                    }
                    // Cancel scoped children
                    let children = self.tasks.get(&task_id)
                        .map(|t| t.scoped_children.clone())
                        .unwrap_or_default();
                    for child_id in children {
                        self.cancel_task_tree(child_id, modules, current_module_idx, dispatch_table, heap, host, next_request_id);
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
                    if let Some(parent) = self.tasks.get_mut(&task_id) {
                        if let Some(frame) = parent.call_stack.last_mut() {
                            frame.registers[r_dst as usize] = pack_task_id(child_id);
                        }
                    }
                    continue;
                }
                ExecutionResult::JoinTask { r_dst, target } => {
                    // Check if target is already terminal
                    let target_info = self.tasks.get(&target)
                        .map(|t| (t.state, t.return_value));
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
                    self.cancel_task_tree(target, modules, current_module_idx, dispatch_table, heap, host, next_request_id);
                    continue;
                }
            }
        }
    }

    /// Cancel a task and all its scoped children recursively.
    /// Executes defer handlers at each frame level during unwinding.
    pub(crate) fn cancel_task_tree(
        &mut self,
        task_id: TaskId,
        modules: &[LoadedModule],
        current_module_idx: usize,
        dispatch_table: &DispatchTable,
        heap: &mut dyn GcHeap,
        host: &mut dyn RuntimeHost,
        next_request_id: &mut u32,
    ) {
        // Get children first (depth-first)
        let children = self.tasks.get(&task_id)
            .map(|t| t.scoped_children.clone())
            .unwrap_or_default();

        // Cancel children first
        for child_id in children {
            self.cancel_task_tree(child_id, modules, current_module_idx, dispatch_table, heap, host, next_request_id);
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
                &mut self.entity_registry,
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
    fn wake_joiners(&mut self, target_id: TaskId, return_value: Option<Value>) {
        if let Some(waiters) = self.join_waiters.remove(&target_id) {
            for (waiter_id, r_dst) in waiters {
                if let Some(waiter) = self.tasks.get_mut(&waiter_id) {
                    if waiter.state == TaskState::Suspended {
                        waiter.state = TaskState::Ready;
                        waiter.pending_request = None;
                        if let Some(frame) = waiter.call_stack.last_mut() {
                            frame.registers[r_dst as usize] = return_value.unwrap_or(Value::Void);
                        }
                        self.ready_queue.push_back(waiter_id);
                    }
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
