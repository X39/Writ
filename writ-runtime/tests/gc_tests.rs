//! GC integration tests for writ-runtime.
//!
//! These tests verify Phase 18 Plan 03 requirements:
//! - GC-01: Precise mark-and-sweep traces from register roots
//! - GC-03: Finalization queue with two-cycle collection
//! - GC-04: GC runs at safe points when host triggers it (Manual mode)
//! - GC-05: GcHeap trait — BumpHeap and MarkSweepHeap both work

use std::sync::{Arc, Mutex};
use writ_module::module::MethodBody;
use writ_module::tables::TypeDefKind;
use writ_module::Instruction;
use writ_module::ModuleBuilder;
use writ_runtime::{
    ExecutionLimit, GcStats, HostRequest, HostResponse, LogLevel, NullHost,
    RequestId, Runtime, RuntimeBuilder, RuntimeHost, TaskState, Value,
};

// ── Encoding helper ──────────────────────────────────────────────

fn encode(instrs: &[Instruction]) -> Vec<u8> {
    let mut code = Vec::new();
    for instr in instrs {
        instr.encode(&mut code).unwrap();
    }
    code
}

// ── Recording Host ───────────────────────────────────────────────

/// A host that records on_gc_complete calls.
struct RecordingHost {
    gc_stats: Arc<Mutex<Vec<GcStats>>>,
}

impl RecordingHost {
    fn new() -> (Self, Arc<Mutex<Vec<GcStats>>>) {
        let stats = Arc::new(Mutex::new(Vec::new()));
        (RecordingHost { gc_stats: stats.clone() }, stats)
    }
}

impl RuntimeHost for RecordingHost {
    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { .. } => HostResponse::Value(Value::Void),
            HostRequest::FieldRead { .. } => HostResponse::Value(Value::Int(0)),
            HostRequest::EntitySpawn { .. } => HostResponse::Confirmed,
            HostRequest::FieldWrite { .. } => HostResponse::Confirmed,
            HostRequest::GetComponent { .. } => HostResponse::Value(Value::Void),
            HostRequest::InitEntity { .. } => HostResponse::Confirmed,
            HostRequest::DestroyEntity { .. } => HostResponse::Confirmed,
            HostRequest::GetOrCreate { .. } => HostResponse::Confirmed,
            HostRequest::Join { .. } => HostResponse::Confirmed,
        }
    }

    fn on_log(&mut self, _level: LogLevel, _message: &str) {}

    fn on_gc_complete(&mut self, stats: &GcStats) {
        self.gc_stats.lock().unwrap().push(stats.clone());
    }
}

// ── Test helpers ─────────────────────────────────────────────────

fn build_gc_runtime(instructions: &[Instruction], reg_count: u16) -> Runtime<NullHost> {
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
    RuntimeBuilder::new(module).with_gc().build().unwrap()
}

fn build_gc_runtime_with_host(
    instructions: &[Instruction],
    reg_count: u16,
    host: RecordingHost,
) -> Runtime<RecordingHost> {
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
    RuntimeBuilder::new(module)
        .with_host(host)
        .with_gc()
        .build()
        .unwrap()
}

// ── Tests ────────────────────────────────────────────────────────

#[test]
fn gc_collects_unreachable_string() {
    // Allocate a string, then overwrite the register. The string is unreachable.
    let mut runtime = build_gc_runtime(
        &[
            // r0 = "hello"
            Instruction::LoadString { r_dst: 0, string_idx: 0 },
            // Overwrite r0 with null — string now unreachable
            Instruction::LoadNull { r_dst: 0 },
            Instruction::RetVoid,
        ],
        1,
    );
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // Before GC, heap should have 1 object (the string)
    assert_eq!(runtime.heap().heap_size(), 1);

    // Collect — the string is unreachable (task completed, no frames/registers)
    let stats = runtime.collect_garbage();
    assert_eq!(stats.objects_freed, 1);
    assert_eq!(stats.heap_after, 0);
}

#[test]
fn gc_preserves_reachable_global() {
    // Store a string in a global, then collect — it should survive.
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", TypeDefKind::Struct, 0);
    // Add a global
    builder.add_global_def("g", &[0x01], 0, &[]);
    let body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            Instruction::LoadString { r_dst: 0, string_idx: 0 },
            Instruction::StoreGlobal { global_idx: 0, r_src: 0 },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);
    let module = builder.build();
    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();

    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // String is stored in global — should survive GC
    let stats = runtime.collect_garbage();
    assert_eq!(stats.objects_freed, 0);
    assert_eq!(stats.objects_traced, 1);
}

#[test]
fn gc_preserves_entity_data_ref() {
    // Spawn and init an entity — its data_ref should survive GC.
    let mut runtime = build_gc_runtime(
        &[
            Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::RetVoid,
        ],
        1,
    );
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // Entity data_ref should be a root
    let before = runtime.heap().heap_size();
    assert!(before > 0, "entity should have allocated heap data");

    let stats = runtime.collect_garbage();
    assert_eq!(stats.objects_freed, 0, "entity data should be preserved");
}

#[test]
fn gc_frees_destroyed_entity_data() {
    // Spawn, init, then destroy entity — data should be collectible
    let mut runtime = build_gc_runtime(
        &[
            Instruction::SpawnEntity { r_dst: 0, type_idx: 1 },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::DestroyEntity { r_entity: 0 },
            Instruction::RetVoid,
        ],
        1,
    );
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // Entity is destroyed, data_ref is no longer a root
    let stats = runtime.collect_garbage();
    assert!(stats.objects_freed > 0, "destroyed entity data should be freed");
}

#[test]
fn gc_on_gc_complete_callback_fires() {
    let (host, stats_log) = RecordingHost::new();
    let mut runtime = build_gc_runtime_with_host(
        &[Instruction::RetVoid],
        1,
        host,
    );
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    runtime.collect_garbage();

    let log = stats_log.lock().unwrap();
    assert_eq!(log.len(), 1, "on_gc_complete should have been called once");
}

#[test]
fn gc_stats_accurate_counts() {
    let mut builder2 = ModuleBuilder::new("test2");
    builder2.add_type_def("TestType", "", TypeDefKind::Struct, 0);
    builder2.add_global_def("g", &[0x01], 0, &[]);
    let body = MethodBody {
        register_types: vec![0; 3],
        code: encode(&[
            Instruction::LoadString { r_dst: 0, string_idx: 0 },
            Instruction::StoreGlobal { global_idx: 0, r_src: 0 },
            Instruction::LoadString { r_dst: 1, string_idx: 0 },
            Instruction::LoadString { r_dst: 2, string_idx: 0 },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder2.add_method("main", &[0], 0, 3, body);
    let module = builder2.build();
    let mut runtime2 = RuntimeBuilder::new(module).with_gc().build().unwrap();

    let tid = runtime2.spawn_task(0, vec![]).unwrap();
    runtime2.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime2.task_state(tid), Some(TaskState::Completed));

    let stats = runtime2.collect_garbage();
    assert_eq!(stats.heap_before, 3);
    assert_eq!(stats.objects_traced, 1); // Only the global string
    assert_eq!(stats.objects_freed, 2); // Other two are unreachable
    assert_eq!(stats.heap_after, 1);
}

#[test]
fn gc_with_bump_heap_is_noop() {
    // Build without .with_gc() — uses BumpHeap
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", TypeDefKind::Struct, 0);
    let body = MethodBody {
        register_types: vec![0; 1],
        code: encode(&[
            Instruction::LoadString { r_dst: 0, string_idx: 0 },
            Instruction::LoadNull { r_dst: 0 },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 1, body);
    let module = builder.build();
    let mut runtime = RuntimeBuilder::new(module).build().unwrap();

    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // BumpHeap collect is a no-op
    let stats = runtime.collect_garbage();
    assert_eq!(stats.objects_traced, 0);
    assert_eq!(stats.objects_freed, 0);
    // Object still exists (never freed)
    assert_eq!(runtime.heap().heap_size(), 1);
}

#[test]
fn gc_empty_heap_collection() {
    let mut runtime = build_gc_runtime(
        &[Instruction::RetVoid],
        1,
    );
    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    let stats = runtime.collect_garbage();
    assert_eq!(stats.objects_traced, 0);
    assert_eq!(stats.objects_freed, 0);
    assert_eq!(stats.heap_before, 0);
    assert_eq!(stats.heap_after, 0);
}

#[test]
fn gc_multiple_collections_progressive() {
    // Run multiple GC cycles, allocating and freeing progressively.
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", TypeDefKind::Struct, 0);
    builder.add_global_def("g", &[0x01], 0, &[]);
    let body = MethodBody {
        register_types: vec![0; 2],
        code: encode(&[
            // Allocate a string and store in global
            Instruction::LoadString { r_dst: 0, string_idx: 0 },
            Instruction::StoreGlobal { global_idx: 0, r_src: 0 },
            // Allocate another string (unreachable after return)
            Instruction::LoadString { r_dst: 1, string_idx: 0 },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 2, body);
    let module = builder.build();
    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();

    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // First GC: free the unreachable string
    let stats1 = runtime.collect_garbage();
    assert_eq!(stats1.objects_freed, 1);
    assert_eq!(stats1.heap_after, 1);

    // Second GC: nothing new to free (global still holds its string)
    let stats2 = runtime.collect_garbage();
    assert_eq!(stats2.objects_freed, 0);
    assert_eq!(stats2.heap_after, 1);
}

#[test]
fn gc_class_containing_array_field_survives() {
    // Build a module with a Class type (kind=4) that has one field.
    // The test method:
    //   r0 = new MyClass (heap object stored as Value::Ref)
    //   r1 = new int[] array
    //   r2 = 42 (int literal)
    //   r1.add(r2)
    //   r0.field[0] = r1  (store array ref inside class instance field)
    //   global[0] = r0   (make class a GC root after task completes)
    //   r1 = null        (remove direct ref to array — only reachable via r0.field[0])
    //   RetVoid
    //
    // After task completion: GC with class as global root.
    // The array must be traced transitively through the class field.
    let mut builder = ModuleBuilder::new("test");
    // Add the class type (TypeDef row 1).
    let class_token = builder.add_type_def("MyClass", "", TypeDefKind::Class, 0);
    // Add one field "items" (type blob [0x01] — only field count matters for allocation).
    builder.add_field_def("items", &[0x01], 0);
    // Add a global to hold the class ref so it survives as a GC root after task completion.
    builder.add_global_def("g", &[0x01], 0, &[]);
    let body = MethodBody {
        register_types: vec![0; 4],
        code: encode(&[
            // r0 = new MyClass (Value::Ref on heap, kind=Class)
            Instruction::New { r_dst: 0, type_idx: class_token.0 },
            // r1 = new int[] (empty array, elem_type=0x01 for int)
            Instruction::NewArray { r_dst: 1, elem_type: 0x01 },
            // resize to 1 so we can store an element
            Instruction::LoadInt { r_dst: 2, value: 1 },
            Instruction::ArrayResize { r_arr: 1, r_new_len: 2 },
            // r2 = 42; store at index 0
            Instruction::LoadInt { r_dst: 2, value: 42 },
            Instruction::LoadInt { r_dst: 3, value: 0 },
            Instruction::ArrayStore { r_arr: 1, r_idx: 3, r_val: 2 },
            // r0.field[0] = r1 (store array ref as class field)
            Instruction::SetField { r_obj: 0, field_idx: 0, r_val: 1 },
            // Store class ref in global so it's a GC root after task completes
            Instruction::StoreGlobal { global_idx: 0, r_src: 0 },
            // Null out r1 — array is now ONLY reachable via r0.field[0]
            Instruction::LoadNull { r_dst: 1 },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("main", &[0], 0, 4, body);
    let module = builder.build();
    let mut runtime = RuntimeBuilder::new(module).with_gc().build().unwrap();

    let tid = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);
    assert_eq!(runtime.task_state(tid), Some(TaskState::Completed));

    // Heap should have 2 objects: the class instance (Struct) and the array.
    assert_eq!(runtime.heap().heap_size(), 2, "heap should contain class + array before GC");

    // Collect: class is reachable via global; array is reachable via class.field[0].
    let stats = runtime.collect_garbage();
    assert_eq!(stats.objects_freed, 0, "GC must trace array through class field — nothing freed");
    assert_eq!(stats.heap_after, 2, "both class and array must survive GC");
    assert_eq!(stats.objects_traced, 2, "both objects should be traced");
}
