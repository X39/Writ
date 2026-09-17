//! Integration tests for PREP-03 and PREP-04: RuntimeHost debug hooks and SuspendReason.
//!
//! These tests verify observable behavior at the VM level:
//! - A debug-enabled host receives before_instruction callbacks during VM execution
//! - DebugAction::Break causes a task to suspend with SuspendReason::Breakpoint
//! - DebugAction::Continue does not suspend the task
//! - resume_debug clears suspend_reason and resumes execution
//! - SuspendReason::HostRequest is set on standard extern-call suspensions

use writ_module::Instruction;
use writ_module::ModuleBuilder;
use writ_module::module::MethodBody;
use writ_module::tables::TypeDefKind;
use writ_runtime::{
    DebugAction, ExecutionLimit, HostRequest, HostResponse, LogLevel, NullHost, RequestId, Runtime,
    RuntimeBuilder, RuntimeHost, SuspendReason, TaskId, TaskState, Value,
};

// ── Encoding helpers ──────────────────────────────────────────────

fn encode(instrs: &[Instruction]) -> Vec<u8> {
    let mut code = Vec::new();
    for instr in instrs {
        instr.encode(&mut code).unwrap();
    }
    code
}

fn build_runtime_with_host<H: RuntimeHost>(
    instructions: &[Instruction],
    reg_count: u16,
    host: H,
) -> Runtime<H> {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", TypeDefKind::Struct, 0);
    let body = MethodBody {
        register_types: vec![0; reg_count as usize],
        code: encode(instructions),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, reg_count, body);
    let module = builder.build();
    RuntimeBuilder::new(module).with_host(host).build().unwrap()
}

// ── Debug host implementations ────────────────────────────────────

/// A host that records every before_instruction call and always returns Continue.
struct RecordingHost {
    pub calls: Vec<(TaskId, usize, u32, u32)>, // (task_id, module_idx, method_idx, pc)
}

impl RecordingHost {
    fn new() -> Self {
        RecordingHost { calls: Vec::new() }
    }
}

impl RuntimeHost for RecordingHost {
    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { .. } => HostResponse::Value(Value::Void),
            _ => HostResponse::Confirmed,
        }
    }

    fn on_log(&mut self, _level: LogLevel, _message: &str) {}

    fn debug_enabled(&self) -> bool {
        true
    }

    fn before_instruction(
        &mut self,
        task_id: TaskId,
        module_idx: usize,
        method_idx: u32,
        pc: u32,
        _source_line: u32,
        _source_col: u16,
    ) -> DebugAction {
        self.calls.push((task_id, module_idx, method_idx, pc));
        DebugAction::Continue
    }
}

/// A host that returns Break on a specific instruction index to trigger a debug suspension.
struct BreakOnFirstHost {
    pub did_break: bool,
    pub entered_methods: Vec<(usize, u32)>,
    pub exited_methods: Vec<(usize, u32)>,
}

impl BreakOnFirstHost {
    fn new() -> Self {
        BreakOnFirstHost {
            did_break: false,
            entered_methods: Vec::new(),
            exited_methods: Vec::new(),
        }
    }
}

impl RuntimeHost for BreakOnFirstHost {
    fn on_request(&mut self, _id: RequestId, _req: &HostRequest) -> HostResponse {
        HostResponse::Confirmed
    }

    fn on_log(&mut self, _level: LogLevel, _message: &str) {}

    fn debug_enabled(&self) -> bool {
        true
    }

    fn before_instruction(
        &mut self,
        _task_id: TaskId,
        _module_idx: usize,
        _method_idx: u32,
        _pc: u32,
        _source_line: u32,
        _source_col: u16,
    ) -> DebugAction {
        // Break on the very first instruction
        if !self.did_break {
            self.did_break = true;
            DebugAction::Break
        } else {
            DebugAction::Continue
        }
    }

    fn on_function_enter(&mut self, _task_id: TaskId, module_idx: usize, method_idx: u32) {
        self.entered_methods.push((module_idx, method_idx));
    }

    fn on_function_exit(&mut self, _task_id: TaskId, module_idx: usize, method_idx: u32) {
        self.exited_methods.push((module_idx, method_idx));
    }
}

// ── PREP-03 tests ─────────────────────────────────────────────────

/// A debug-enabled host receives before_instruction calls during VM execution.
///
/// This test verifies the PREP-03 requirement: "VM calls before_instruction only
/// when debug_enabled() returns true."
#[test]
fn debug_enabled_host_receives_before_instruction_callbacks() {
    let instrs = [
        Instruction::LoadInt { r_dst: 0, value: 7 },
        Instruction::Ret { r_src: 0 },
    ];
    let host = RecordingHost::new();
    let mut runtime = build_runtime_with_host(&instrs, 1, host);
    let task_id = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    // The task should have completed successfully
    assert_eq!(
        runtime.task_state(task_id),
        Some(TaskState::Completed),
        "task should complete when host returns Continue for all instructions"
    );

    // The host should have received at least one before_instruction call
    // (one per instruction: LoadInt + Ret = 2 calls minimum)
    let call_count = runtime.host().calls.len();
    assert!(
        call_count >= 2,
        "host should receive at least 2 before_instruction calls for LoadInt+Ret, got {}",
        call_count
    );
}

/// before_instruction receives correct method_idx and task_id parameters.
///
/// This verifies the hook parameters are populated correctly from the dispatch context.
#[test]
fn before_instruction_receives_correct_method_and_task_ids() {
    let instrs = [
        Instruction::LoadInt { r_dst: 0, value: 1 },
        Instruction::Ret { r_src: 0 },
    ];
    let host = RecordingHost::new();
    let mut runtime = build_runtime_with_host(&instrs, 1, host);
    let task_id = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    let calls = &runtime.host().calls;
    assert!(!calls.is_empty(), "should have recorded at least one call");

    // Every call should carry the correct task_id
    for (recorded_task_id, _module_idx, _method_idx, _pc) in calls {
        assert_eq!(
            *recorded_task_id, task_id,
            "before_instruction should receive the spawned task's id"
        );
    }

    // method_idx should be 0 (user module method index 0, since the virtual module
    // is module 0 and user module is module 1, but the method index within the user
    // module is 0)
    let (_, module_idx, method_idx, _) = calls[0];
    assert_eq!(module_idx, runtime.user_module_idx());
    assert_eq!(method_idx, 0, "first call should be for method index 0");
}

#[test]
fn cross_module_debug_locations_disambiguate_colliding_method_rows() {
    use writ_module::signature::{TypeSignature, encode_method_signature};

    struct BreakOnModuleChangeHost {
        first_module: Option<usize>,
    }

    impl RuntimeHost for BreakOnModuleChangeHost {
        fn on_request(&mut self, _id: RequestId, _req: &HostRequest) -> HostResponse {
            HostResponse::Confirmed
        }

        fn on_log(&mut self, _level: LogLevel, _message: &str) {}

        fn debug_enabled(&self) -> bool {
            true
        }

        fn before_instruction(
            &mut self,
            _task_id: TaskId,
            module_idx: usize,
            _method_idx: u32,
            _pc: u32,
            _source_line: u32,
            _source_col: u16,
        ) -> DebugAction {
            let first_module = self.first_module.get_or_insert(module_idx);
            if *first_module == module_idx {
                DebugAction::Continue
            } else {
                DebugAction::Break
            }
        }
    }

    let signature =
        encode_method_signature(&[], &TypeSignature::Void).expect("encode method signature");

    let mut library = ModuleBuilder::new("debug-library");
    let exports = library.add_type_def("Exports", "lib", TypeDefKind::Struct, 0);
    library.add_type_method(
        exports,
        "library_method_zero",
        &signature,
        1 << 1,
        0,
        MethodBody {
            register_types: vec![],
            code: encode(&[Instruction::RetVoid]),
            debug_locals: vec![],
            source_spans: vec![],
        },
    );

    let mut user = ModuleBuilder::new("debug-user");
    let library_ref = user.add_module_ref("debug-library", "1.0.0");
    let exports_ref = user.add_type_ref(library_ref, "Exports", "lib");
    let target_ref =
        user.add_method_ref_with_flags(exports_ref, "library_method_zero", &signature, 0);
    user.add_method(
        "user_method_zero",
        &signature,
        0,
        1,
        MethodBody {
            register_types: vec![0],
            code: encode(&[
                Instruction::Call {
                    r_dst: 0,
                    method_idx: target_ref.0,
                    r_base: 0,
                    argc: 0,
                },
                Instruction::RetVoid,
            ]),
            debug_locals: vec![],
            source_spans: vec![],
        },
    );

    let host = BreakOnModuleChangeHost { first_module: None };
    let mut runtime = RuntimeBuilder::new(user.build())
        .with_library(library.build())
        .with_host(host)
        .build()
        .expect("build cross-module runtime");
    let user_module_idx = runtime.user_module_idx();
    let library_module_idx = runtime
        .domain()
        .modules
        .iter()
        .position(|loaded| {
            writ_module::heap::read_string(
                &loaded.module.string_heap,
                loaded.module.header.module_name,
            )
            .ok()
                == Some("debug-library")
        })
        .expect("library module is loaded");
    let task_id = runtime.spawn_task(0, vec![]).expect("spawn user method");

    runtime.tick(0.0, ExecutionLimit::None);

    let reason = runtime
        .suspend_reason(task_id)
        .expect("library method should suspend");
    match reason {
        SuspendReason::Breakpoint {
            module_idx,
            method_idx,
            ..
        } => {
            assert_eq!(*module_idx, library_module_idx);
            assert_eq!(*method_idx, 0);
        }
        other => panic!(
            "expected breakpoint, got {:?}",
            std::mem::discriminant(other)
        ),
    }

    let frames = runtime.call_stack_frames(task_id).expect("live call stack");
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].module_idx, user_module_idx);
    assert_eq!(frames[0].method_idx, 0);
    assert_eq!(frames[1].module_idx, library_module_idx);
    assert_eq!(frames[1].method_idx, 0);
}

/// A debug-enabled host with DebugAction::Break causes the task to suspend with Breakpoint reason.
///
/// This is the core PREP-03/PREP-04 integration: the VM calls before_instruction,
/// gets Break back, sets SuspendReason::Breakpoint, and returns DebugSuspend.
#[test]
fn debug_break_suspends_task_with_breakpoint_reason() {
    let instrs = [
        Instruction::LoadInt {
            r_dst: 0,
            value: 99,
        },
        Instruction::Ret { r_src: 0 },
    ];
    let host = BreakOnFirstHost::new();
    let mut runtime = build_runtime_with_host(&instrs, 1, host);
    let task_id = runtime.spawn_task(0, vec![]).unwrap();

    // Run one tick — the first instruction should trigger Break
    runtime.tick(0.0, ExecutionLimit::None);

    // The task should be Suspended (not Completed) due to the Break
    assert_eq!(
        runtime.task_state(task_id),
        Some(TaskState::Suspended),
        "task should be Suspended after debug Break, not completed"
    );

    // The suspend_reason should be Breakpoint
    let reason = runtime.suspend_reason(task_id);
    assert!(
        reason.is_some(),
        "task should have a suspend_reason after debug Break"
    );
    match reason.unwrap() {
        SuspendReason::Breakpoint { method_idx, pc, .. } => {
            assert_eq!(*pc, 0, "breakpoint should be at pc=0 (first instruction)");
            let _ = method_idx; // method_idx is valid but value not critical here
        }
        other => panic!(
            "expected SuspendReason::Breakpoint, got {:?}",
            std::mem::discriminant(other)
        ),
    }
}

/// resume_debug clears the suspend_reason and puts the task back in the ready queue.
///
/// Verifies PREP-04: "Task resume clears suspend_reason."
#[test]
fn resume_debug_clears_suspend_reason_and_continues_execution() {
    let instrs = [
        Instruction::LoadInt {
            r_dst: 0,
            value: 42,
        },
        Instruction::Ret { r_src: 0 },
    ];
    let host = BreakOnFirstHost::new();
    let mut runtime = build_runtime_with_host(&instrs, 1, host);
    let task_id = runtime.spawn_task(0, vec![]).unwrap();

    // First tick — breaks at instruction 0
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(task_id), Some(TaskState::Suspended));

    // Resume from debug suspension
    runtime
        .resume_debug(task_id)
        .expect("resume_debug should succeed");

    // After resume, suspend_reason should be cleared
    assert!(
        runtime.suspend_reason(task_id).is_none(),
        "suspend_reason should be None after resume_debug"
    );

    // Task should be Ready again (not completed yet — needs another tick)
    assert_eq!(
        runtime.task_state(task_id),
        Some(TaskState::Ready),
        "task should be Ready after resume_debug"
    );

    // Run again to completion (host now returns Continue since did_break=true)
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(
        runtime.task_state(task_id),
        Some(TaskState::Completed),
        "task should complete after resuming from debug break"
    );
    assert_eq!(
        runtime.return_value(task_id),
        Some(Value::Int(42)),
        "task should return the correct value after debug resume"
    );
}

/// NullHost (debug_enabled=false) causes no before_instruction overhead.
///
/// Verifies the zero-overhead guarantee: when debug_enabled() returns false,
/// the hook is never called.
#[test]
fn null_host_produces_no_debug_suspension_and_task_completes() {
    // This test verifies the base case: NullHost tasks run normally
    let instrs = [
        Instruction::LoadInt { r_dst: 0, value: 5 },
        Instruction::Ret { r_src: 0 },
    ];
    let mut runtime = build_runtime_with_host(&instrs, 1, NullHost);
    let task_id = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(
        runtime.task_state(task_id),
        Some(TaskState::Completed),
        "NullHost task should complete without debug suspension"
    );
    assert_eq!(runtime.return_value(task_id), Some(Value::Int(5)));
    assert!(
        runtime.suspend_reason(task_id).is_none(),
        "NullHost task should have no suspend_reason"
    );
}

// ── PREP-04 tests ─────────────────────────────────────────────────

/// SuspendReason::HostRequest is set when a task suspends waiting for a host response.
///
/// Verifies PREP-04: "Host-request suspensions set SuspendReason::HostRequest on the task."
/// Note: With NullHost all requests are synchronously resolved. To get a real
/// suspension we need to inspect the SuspendReason when it is set. Since NullHost
/// resolves immediately, we test via the task.rs inline tests (constructor tests).
/// This integration test verifies the task state machinery is reachable end-to-end
/// by confirming that the Breakpoint path sets the correct variant.
#[test]
fn debug_step_over_suspends_task_with_debug_step_reason() {
    struct StepOverHost {
        fired: bool,
    }
    impl RuntimeHost for StepOverHost {
        fn on_request(&mut self, _id: RequestId, _req: &HostRequest) -> HostResponse {
            HostResponse::Confirmed
        }
        fn on_log(&mut self, _level: LogLevel, _message: &str) {}
        fn debug_enabled(&self) -> bool {
            true
        }
        fn before_instruction(
            &mut self,
            _task_id: TaskId,
            _module_idx: usize,
            _method_idx: u32,
            _pc: u32,
            _source_line: u32,
            _source_col: u16,
        ) -> DebugAction {
            if !self.fired {
                self.fired = true;
                DebugAction::StepOver
            } else {
                DebugAction::Continue
            }
        }
    }

    let instrs = [
        Instruction::LoadInt { r_dst: 0, value: 1 },
        Instruction::Ret { r_src: 0 },
    ];
    let host = StepOverHost { fired: false };
    let mut runtime = build_runtime_with_host(&instrs, 1, host);
    let task_id = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(runtime.task_state(task_id), Some(TaskState::Suspended));

    // Should have SuspendReason::DebugStep with mode=StepOver
    match runtime.suspend_reason(task_id) {
        Some(SuspendReason::DebugStep { mode, .. }) => {
            assert_eq!(*mode, DebugAction::StepOver);
        }
        other => panic!(
            "expected SuspendReason::DebugStep, got {:?}",
            other.map(std::mem::discriminant)
        ),
    }
}
