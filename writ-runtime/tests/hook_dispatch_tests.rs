//! TDD tests for FIX-01 (lifecycle hook dispatch) and FIX-02 (generic dispatch key).
//!
//! FIX-01: INIT_ENTITY must push an on_create hook frame for entity types that define one.
//!         DESTROY_ENTITY must push an on_destroy hook frame for entity types that define one.
//! FIX-02: DispatchKey must include type_args_hash so generic specializations
//!         (e.g. Into<Float> vs Into<String>) produce distinct dispatch table entries.

use std::sync::{Arc, Mutex};
use writ_module::module::MethodBody;
use writ_module::Instruction;
use writ_module::ModuleBuilder;
use writ_runtime::{
    ExecutionLimit, GcStats, HostRequest, HostResponse, LogLevel, RequestId,
    RuntimeBuilder, RuntimeHost, TaskState, Value,
};

// ── Helpers ─────────────────────────────────────────────────────────────

fn encode(instrs: &[Instruction]) -> Vec<u8> {
    let mut code = Vec::new();
    for instr in instrs {
        instr.encode(&mut code).unwrap();
    }
    code
}

fn make_body(instrs: &[Instruction], reg_count: usize) -> MethodBody {
    MethodBody {
        register_types: vec![0u32; reg_count],
        code: encode(instrs),
        debug_locals: vec![],
        source_spans: vec![],
    }
}

/// A test host that records which extern functions were called.
/// Uses a shared Vec so the test can inspect after runtime.tick().
struct TrackingHost {
    /// Shared record of extern function names called (in order).
    calls: Arc<Mutex<Vec<String>>>,
    /// ExternDef name table for decoding extern_idx tokens.
    extern_names: Vec<String>,
}

impl TrackingHost {
    fn new(extern_names: Vec<String>, calls: Arc<Mutex<Vec<String>>>) -> Self {
        TrackingHost { calls, extern_names }
    }

    fn resolve_extern_name(&self, extern_idx: u32) -> String {
        let row_1based = (extern_idx & 0x00FF_FFFF) as usize;
        if row_1based == 0 {
            return "?".to_string();
        }
        let idx = row_1based - 1;
        self.extern_names
            .get(idx)
            .cloned()
            .unwrap_or_else(|| "?".to_string())
    }
}

impl RuntimeHost for TrackingHost {
    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { extern_idx, .. } => {
                let name = self.resolve_extern_name(*extern_idx);
                self.calls.lock().unwrap().push(name);
                HostResponse::Value(Value::Void)
            }
            HostRequest::EntitySpawn { .. } => HostResponse::Confirmed,
            HostRequest::InitEntity { .. } => HostResponse::Confirmed,
            HostRequest::DestroyEntity { .. } => HostResponse::Confirmed,
            HostRequest::FieldRead { .. } => HostResponse::Value(Value::Int(0)),
            HostRequest::FieldWrite { .. } => HostResponse::Confirmed,
            HostRequest::GetComponent { .. } => HostResponse::Value(Value::Void),
            HostRequest::GetOrCreate { .. } => HostResponse::Confirmed,
            HostRequest::Join { .. } => HostResponse::Confirmed,
        }
    }

    fn on_log(&mut self, _level: LogLevel, _message: &str) {}
    fn on_gc_complete(&mut self, _stats: &GcStats) {}
}

// ── FIX-01: Lifecycle Hook Tests ────────────────────────────────────────────

/// When INIT_ENTITY executes for an entity whose TypeDef defines "on_create",
/// the runtime must call that hook.
///
/// Module layout:
///   TypeDef[0] "EntityA" -- method_list=1 (owns methods at index 0..)
///   TypeDef[1] "_Sentinel" -- method_list=2 (bounds EntityA's range to [0..1))
///   method[0] "on_create" -- calls extern "hook_fired", then RET_VOID
///   method[1] "main"      -- SPAWN_ENTITY(type_token=EntityA), INIT_ENTITY, RET_VOID
///   ExternDef[0] "hook_fired" (token 0x10_000001)
///
/// SPAWN_ENTITY type_idx: TypeDef table_id=2, row=1 (1-based) -> raw token = (2<<24)|1
/// After INIT_ENTITY, if on_create fired, tracking calls should contain "hook_fired".
#[test]
fn init_entity_dispatches_on_create_hook() {
    let mut builder = ModuleBuilder::new("test");

    // ExternDef[0]: "hook_fired" (table_id=16, row=1 -> raw token = (16<<24)|1)
    let hook_token = builder.add_extern_def("hook_fired", &[], "hook_fired", 0);

    // TypeDef "EntityA" (row 0 = token 0x02000001): method_list=1 means methods start at index 0
    builder.add_type_def("EntityA", "", 1, 0);

    // method[0]: "on_create" -- calls extern hook_fired, then RET_VOID
    let on_create_body = make_body(&[
        Instruction::CallExtern { r_dst: 1, extern_idx: hook_token.0, r_base: 0, argc: 0 },
        Instruction::RetVoid,
    ], 2);
    builder.add_method("on_create", &[0, 0], 0, 2, on_create_body);

    // TypeDef "_Sentinel" (row 1): method_list=2 bounds EntityA's methods to [0..1)
    builder.add_type_def("_Sentinel", "", 2, 0);

    // method[1]: "main" -- SPAWN_ENTITY, INIT_ENTITY, RET_VOID
    // type_idx: 1 = 1-based row index for TypeDef[0] "EntityA"
    let main_body = make_body(&[
        Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
        Instruction::InitEntity { r_entity: 0 },
        Instruction::RetVoid,
    ], 2);
    builder.add_method("main", &[0, 0], 0, 2, main_body);

    let module = builder.build();
    let extern_names: Vec<String> = module
        .extern_defs
        .iter()
        .map(|ed| {
            writ_module::heap::read_string(&module.string_heap, ed.name)
                .unwrap_or("?")
                .to_string()
        })
        .collect();

    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let host = TrackingHost::new(extern_names, Arc::clone(&calls));
    let mut runtime = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .unwrap();

    // main is method index 1 (on_create is index 0)
    let task_id = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    let state = runtime.task_state(task_id);
    assert_eq!(state, Some(TaskState::Completed), "task should complete");

    let recorded_calls = calls.lock().unwrap().clone();
    assert_eq!(
        recorded_calls,
        vec!["hook_fired"],
        "on_create hook must call extern 'hook_fired' exactly once; got {:?}",
        recorded_calls
    );
}

/// When DESTROY_ENTITY executes for an entity that has "on_destroy",
/// the runtime must call that hook before completing destruction.
#[test]
fn destroy_entity_dispatches_on_destroy_hook() {
    let mut builder = ModuleBuilder::new("test");

    // ExternDef[0]: "destroy_hook_fired"
    let hook_token = builder.add_extern_def("destroy_hook_fired", &[], "destroy_hook_fired", 0);

    // TypeDef "EntityB": method_list=1 (methods start at index 0)
    builder.add_type_def("EntityB", "", 1, 0);

    // method[0]: "on_destroy"
    let on_destroy_body = make_body(&[
        Instruction::CallExtern { r_dst: 1, extern_idx: hook_token.0, r_base: 0, argc: 0 },
        Instruction::RetVoid,
    ], 2);
    builder.add_method("on_destroy", &[0, 0], 0, 2, on_destroy_body);

    // Sentinel type: method_list=2 bounds on_destroy to [0..1)
    builder.add_type_def("_Sentinel", "", 2, 0);

    // method[1]: "main" -- SPAWN, INIT, DESTROY, RET_VOID
    // type_idx: 1 = 1-based row index for TypeDef[0] "EntityB"
    let main_body = make_body(&[
        Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
        Instruction::InitEntity { r_entity: 0 },
        Instruction::DestroyEntity { r_entity: 0 },
        Instruction::RetVoid,
    ], 2);
    builder.add_method("main", &[0, 0], 0, 2, main_body);

    let module = builder.build();
    let extern_names: Vec<String> = module
        .extern_defs
        .iter()
        .map(|ed| {
            writ_module::heap::read_string(&module.string_heap, ed.name)
                .unwrap_or("?")
                .to_string()
        })
        .collect();

    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let host = TrackingHost::new(extern_names, Arc::clone(&calls));
    let mut runtime = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .unwrap();

    let task_id = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    let state = runtime.task_state(task_id);
    assert_eq!(state, Some(TaskState::Completed), "task should complete");

    let recorded_calls = calls.lock().unwrap().clone();
    assert_eq!(
        recorded_calls,
        vec!["destroy_hook_fired"],
        "on_destroy hook must fire exactly once; got {:?}",
        recorded_calls
    );
}

/// Entity type without lifecycle hooks should still init/destroy without crash.
#[test]
fn entity_without_hooks_inits_and_destroys_ok() {
    let mut builder = ModuleBuilder::new("test");

    // TypeDef "EntityNoHooks": method_list=1
    builder.add_type_def("EntityNoHooks", "", 1, 0);
    // Sentinel: method_list=1 (same value) -> EntityNoHooks has 0 methods
    builder.add_type_def("_Sentinel", "", 1, 0);

    // method[0]: "main"
    // type_idx: 1 = 1-based row index for TypeDef[0] "EntityNoHooks"
    let main_body = make_body(&[
        Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
        Instruction::InitEntity { r_entity: 0 },
        Instruction::DestroyEntity { r_entity: 0 },
        Instruction::RetVoid,
    ], 2);
    builder.add_method("main", &[0, 0], 0, 2, main_body);

    let module = builder.build();
    let mut runtime = RuntimeBuilder::new(module).build().unwrap();

    let task_id = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    let state = runtime.task_state(task_id);
    assert_eq!(state, Some(TaskState::Completed), "task should complete without crash");
}
