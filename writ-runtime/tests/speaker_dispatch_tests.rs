//! Tests for Speaker contract dispatch.
//!
//! Verifies that:
//! 1. The Speaker contract exists in the virtual module
//! 2. When an entity type implements Speaker, the dispatch table includes the entry
//! 3. When say() is called, entity display names use Speaker override when available
//! 4. Entities without Speaker fall back to type name

use std::sync::{Arc, Mutex};
use writ_module::module::MethodBody;
use writ_module::tables::TypeDefKind;
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

/// Find the string heap offset for a given string in a built module.
/// Panics if the string is not found.
fn find_string_offset(heap: &[u8], needle: &str) -> u32 {
    let needle_bytes = needle.as_bytes();
    let needle_len = needle_bytes.len() as u32;
    let mut offset = 0u32;
    while (offset as usize) + 4 <= heap.len() {
        let len = u32::from_le_bytes([
            heap[offset as usize],
            heap[offset as usize + 1],
            heap[offset as usize + 2],
            heap[offset as usize + 3],
        ]);
        let str_start = offset as usize + 4;
        let str_end = str_start + len as usize;
        if len == needle_len && str_end <= heap.len() && &heap[str_start..str_end] == needle_bytes {
            return offset;
        }
        offset = str_end as u32;
    }
    panic!("string '{}' not found in string heap", needle);
}

/// A test host that records display_args from ExternCall requests.
struct DisplayArgsHost {
    /// Shared record of display_args from each ExternCall (in order).
    display_args_log: Arc<Mutex<Vec<Vec<String>>>>,
    /// ExternDef name table for decoding extern_idx tokens.
    #[allow(dead_code)]
    extern_names: Vec<String>,
}

impl DisplayArgsHost {
    fn new(extern_names: Vec<String>, log: Arc<Mutex<Vec<Vec<String>>>>) -> Self {
        DisplayArgsHost {
            display_args_log: log,
            extern_names,
        }
    }

    fn _resolve_extern_name(&self, extern_idx: u32) -> String {
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

impl RuntimeHost for DisplayArgsHost {
    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { display_args, .. } => {
                self.display_args_log
                    .lock()
                    .unwrap()
                    .push(display_args.clone());
                HostResponse::Value(Value::Void)
            }
            HostRequest::EntitySpawn { type_idx, .. } => {
                HostResponse::EntityHandle(writ_runtime::EntityId::new(*type_idx, 0))
            }
            HostRequest::InitEntity { .. } => HostResponse::Confirmed,
            HostRequest::DestroyEntity { .. } => HostResponse::Confirmed,
            HostRequest::FieldRead { .. } => HostResponse::Value(Value::Int(0)),
            HostRequest::FieldWrite { .. } => HostResponse::Confirmed,
            HostRequest::GetComponent { .. } => HostResponse::Value(Value::Void),
            HostRequest::GetOrCreate { type_idx, .. } => {
                HostResponse::EntityHandle(writ_runtime::EntityId::new(*type_idx, 0))
            }
            HostRequest::Join { .. } => HostResponse::Confirmed,
        }
    }

    fn on_log(&mut self, _level: LogLevel, _message: &str) {}
    fn on_gc_complete(&mut self, _stats: &GcStats) {}
}

// ── Tests ────────────────────────────────────────────────────────────────

/// The Speaker contract should exist in the virtual module with a single method.
#[test]
fn speaker_contract_exists_in_virtual_module() {
    let mut builder = ModuleBuilder::new("test");
    builder.add_type_def("TestType", "", TypeDefKind::Struct, 0);
    let body = make_body(&[Instruction::RetVoid], 1);
    builder.add_method("main", &[0], 0, 1, body);
    let module = builder.build();
    let runtime = RuntimeBuilder::new(module).build().unwrap();

    // The virtual module (module 0) should have Speaker in its contract defs
    let domain = runtime.domain();
    let vmod = &domain.modules[0].module;
    let contract_names: Vec<&str> = vmod
        .contract_defs
        .iter()
        .filter_map(|cd| writ_module::heap::read_string(&vmod.string_heap, cd.name).ok())
        .collect();
    assert!(
        contract_names.contains(&"Speaker"),
        "virtual module should contain Speaker contract; got: {:?}",
        contract_names
    );
}

/// When a user entity type implements Speaker, the dispatch table should include it.
#[test]
fn speaker_impl_populates_dispatch_table() {
    let mut builder = ModuleBuilder::new("test");

    // Entity type "Merchant" (TypeDef row 0, token row 1)
    let merchant_type = builder.add_type_def("Merchant", "", TypeDefKind::Entity, 0);

    // Reference Speaker contract from writ-runtime
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let speaker_ref = builder.add_type_ref(mod_ref, "Speaker", "writ");

    // Implement Speaker for Merchant
    builder.add_impl_def(merchant_type, speaker_ref);

    // speaker_name method (placeholder: returns void since we just check dispatch table)
    let speaker_body = make_body(
        &[Instruction::RetVoid],
        2,
    );
    builder.add_method("speaker_name", &[], 0, 2, speaker_body);

    // Sentinel type to bound Merchant's method list
    builder.add_type_def("_Sentinel", "", TypeDefKind::Struct, 0);

    // Main method
    let main_body = make_body(&[Instruction::RetVoid], 1);
    builder.add_method("main", &[0], 0, 1, main_body);

    let module = builder.build();
    let runtime = RuntimeBuilder::new(module).build().unwrap();

    // Dispatch table should include the Speaker impl beyond the base intrinsic entries
    let dispatch_table = runtime.dispatch_table();
    assert!(
        dispatch_table.len() > 36,
        "dispatch table should include Speaker impl entry beyond 36 base intrinsics, got {}",
        dispatch_table.len()
    );
}

/// When say() is called with an entity that implements Speaker,
/// the display_args should use the speaker_name return value.
#[test]
fn speaker_override_in_display_args() {
    // Build module with:
    //   - TypeDef "Merchant" (entity) with Speaker impl
    //   - speaker_name method that loads and returns a specific string
    //   - ExternDef "say"
    //   - main: spawn entity, call say(entity, "hello")
    let mut builder = ModuleBuilder::new("test");

    // ExternDef for say (table_id=16, row=1 -> token = (16<<24)|1)
    let say_token = builder.add_extern_def("say", &[0x02, 0x00, 0x05, 0x04, 0x00], "say", 0);

    // TypeDef "Merchant" (row 0, token = (2<<24)|1)
    let merchant_type = builder.add_type_def("Merchant", "", TypeDefKind::Entity, 0);

    // Reference Speaker contract from writ-runtime
    let mod_ref = builder.add_module_ref("writ-runtime", "1.0.0");
    let speaker_ref = builder.add_type_ref(mod_ref, "Speaker", "writ");

    // Implement Speaker for Merchant
    builder.add_impl_def(merchant_type, speaker_ref);

    // method[0]: speaker_name — loads a string and returns it
    // The string offset will be patched after build; use placeholder 0 for now
    let speaker_body = make_body(
        &[
            Instruction::LoadString {
                r_dst: 1,
                string_idx: 0, // placeholder — patched below
            },
            Instruction::Ret { r_src: 1 },
        ],
        2,
    );
    builder.add_method("speaker_name", &[], 0, 2, speaker_body);

    // We need a string "The Merchant" in the heap. Adding a dummy type with that name.
    builder.add_type_def("The Merchant", "", TypeDefKind::Struct, 0);

    // method[1]: main — spawn entity, load string, call say
    let main_body = make_body(
        &[
            Instruction::SpawnEntity {
                r_dst: 0,
                type_idx: 1, // 1-based row for TypeDef[0] (Merchant)
            },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::LoadString {
                r_dst: 1,
                string_idx: 0, // placeholder — patched below
            },
            Instruction::CallExtern {
                r_dst: 2,
                extern_idx: say_token.0,
                r_base: 0,
                argc: 2,
            },
            Instruction::RetVoid,
        ],
        3,
    );
    builder.add_method("main", &[0], 0, 3, main_body);

    let mut module = builder.build();

    // Find string offsets and patch the method bodies
    let merchant_str_offset = find_string_offset(&module.string_heap, "The Merchant");
    let test_str_offset = find_string_offset(&module.string_heap, "test");

    // Patch speaker_name method (method_bodies[0]): LoadString string_idx -> merchant_str_offset
    let patched_speaker_code = encode(&[
        Instruction::LoadString {
            r_dst: 1,
            string_idx: merchant_str_offset,
        },
        Instruction::Ret { r_src: 1 },
    ]);
    module.method_bodies[0].code = patched_speaker_code;

    // Patch main method (method_bodies[1]): LoadString for "hello" -> use module name "test"
    let patched_main_code = encode(&[
        Instruction::SpawnEntity {
            r_dst: 0,
            type_idx: 1,
        },
        Instruction::InitEntity { r_entity: 0 },
        Instruction::LoadString {
            r_dst: 1,
            string_idx: test_str_offset,
        },
        Instruction::CallExtern {
            r_dst: 2,
            extern_idx: say_token.0,
            r_base: 0,
            argc: 2,
        },
        Instruction::RetVoid,
    ]);
    module.method_bodies[1].code = patched_main_code;

    // Build runtime with display_args tracking host
    let extern_names: Vec<String> = module
        .extern_defs
        .iter()
        .map(|ed| {
            writ_module::heap::read_string(&module.string_heap, ed.name)
                .unwrap_or("?")
                .to_string()
        })
        .collect();

    let log = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let host = DisplayArgsHost::new(extern_names, Arc::clone(&log));
    let mut runtime = RuntimeBuilder::new(module).with_host(host).build().unwrap();

    // main is method index 1 (speaker_name is index 0)
    let task_id = runtime.spawn_task(1, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(
        runtime.task_state(task_id),
        Some(TaskState::Completed),
        "task should complete"
    );

    let recorded = log.lock().unwrap().clone();
    assert_eq!(recorded.len(), 1, "say() should be called once");

    // The first display_arg should be "The Merchant" (from Speaker) not "Merchant" (type name)
    assert_eq!(
        recorded[0][0], "The Merchant",
        "Speaker override should produce 'The Merchant', got '{}'",
        recorded[0][0]
    );
}

/// When say() is called with an entity that does NOT implement Speaker,
/// the display_args should use the entity type name.
#[test]
fn entity_without_speaker_uses_type_name() {
    let mut builder = ModuleBuilder::new("test");

    // ExternDef for say
    let say_token = builder.add_extern_def("say", &[0x02, 0x00, 0x05, 0x04, 0x00], "say", 0);

    // TypeDef "Guard" (entity, no Speaker impl)
    builder.add_type_def("Guard", "", TypeDefKind::Entity, 0);

    // Sentinel type
    builder.add_type_def("_Sentinel", "", TypeDefKind::Struct, 0);

    // method[0]: main — spawn entity, load string, call say
    let main_body = make_body(
        &[
            Instruction::SpawnEntity {
                r_dst: 0,
                type_idx: 1, // TypeDef table=2, row=1 (Guard)
            },
            Instruction::InitEntity { r_entity: 0 },
            Instruction::LoadString {
                r_dst: 1,
                string_idx: 0, // placeholder
            },
            Instruction::CallExtern {
                r_dst: 2,
                extern_idx: say_token.0,
                r_base: 0,
                argc: 2,
            },
            Instruction::RetVoid,
        ],
        3,
    );
    builder.add_method("main", &[0], 0, 3, main_body);

    let mut module = builder.build();

    // Patch LoadString with "test" offset (any valid string for arg 1)
    let test_str_offset = find_string_offset(&module.string_heap, "test");
    let patched_code = encode(&[
        Instruction::SpawnEntity {
            r_dst: 0,
            type_idx: 1,
        },
        Instruction::InitEntity { r_entity: 0 },
        Instruction::LoadString {
            r_dst: 1,
            string_idx: test_str_offset,
        },
        Instruction::CallExtern {
            r_dst: 2,
            extern_idx: say_token.0,
            r_base: 0,
            argc: 2,
        },
        Instruction::RetVoid,
    ]);
    module.method_bodies[0].code = patched_code;

    let extern_names: Vec<String> = module
        .extern_defs
        .iter()
        .map(|ed| {
            writ_module::heap::read_string(&module.string_heap, ed.name)
                .unwrap_or("?")
                .to_string()
        })
        .collect();

    let log = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let host = DisplayArgsHost::new(extern_names, Arc::clone(&log));
    let mut runtime = RuntimeBuilder::new(module).with_host(host).build().unwrap();

    let task_id = runtime.spawn_task(0, vec![]).unwrap();
    runtime.tick(0.0, ExecutionLimit::None);

    assert_eq!(
        runtime.task_state(task_id),
        Some(TaskState::Completed),
        "task should complete"
    );

    let recorded = log.lock().unwrap().clone();
    assert_eq!(recorded.len(), 1, "say() should be called once");

    // Without Speaker, display_arg[0] should be the entity type name "Guard"
    assert_eq!(
        recorded[0][0], "Guard",
        "without Speaker, display name should be type name 'Guard', got '{}'",
        recorded[0][0]
    );
}
