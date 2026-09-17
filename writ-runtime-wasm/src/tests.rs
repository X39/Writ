use super::*;
use writ_module::module::MethodBody;
use writ_module::signature::{TypeSignature, encode_method_signature};
use writ_module::{Instruction, ModuleBuilder};

fn body(instructions: &[Instruction], registers: u16) -> MethodBody {
    let mut code = vec![];
    for instruction in instructions {
        instruction.encode(&mut code).unwrap();
    }
    MethodBody {
        register_types: vec![0; registers as usize],
        code,
        debug_locals: vec![],
        source_spans: vec![],
    }
}

fn module_bytes(instructions: &[Instruction], registers: u16, with_extern: bool) -> Vec<u8> {
    let mut builder = ModuleBuilder::new("wasm-test");
    if with_extern {
        let signature =
            encode_method_signature(&[TypeSignature::Int], &TypeSignature::Int).unwrap();
        builder.add_extern_def("host_add_one", &signature, "host_add_one", 0);
    }
    let signature = encode_method_signature(&[], &TypeSignature::Int).unwrap();
    builder.add_method(
        "main",
        &signature,
        0,
        registers,
        body(instructions, registers),
    );
    builder.build().to_bytes().unwrap()
}

fn handle(index: u32, generation: u32) -> WireHandle {
    WireHandle { index, generation }
}

#[test]
fn loads_module_spawns_ticks_and_returns_value() {
    let bytes = module_bytes(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 42,
            },
            Instruction::Ret { r_src: 0 },
        ],
        1,
        false,
    );
    let mut vm = Engine::new();
    vm.load_module(&bytes).unwrap();
    let task = vm.spawn("main", vec![]).unwrap();
    assert_eq!(vm.state(&task).unwrap(), "ready");
    assert_eq!(vm.tick(0.016, 1).unwrap(), "budgetExhausted");
    assert_eq!(vm.tick(0.016, 1).unwrap(), "allCompleted");
    assert_eq!(vm.state(&task).unwrap(), "completed");
    assert_eq!(
        vm.return_value(&task).unwrap(),
        Some(WireValue::Int { value: "42".into() })
    );
}

#[test]
fn reports_invalid_modules_and_unknown_entries() {
    let mut vm = Engine::new();
    assert!(
        vm.load_module(b"not a module")
            .unwrap_err()
            .contains("invalid .writc module")
    );
    let bytes = module_bytes(&[Instruction::RetVoid], 1, false);
    vm.load_module(&bytes).unwrap();
    assert!(
        vm.spawn("missing", vec![])
            .unwrap_err()
            .contains("was not found")
    );
}

#[test]
fn accepts_precompiled_libraries_before_the_user_module() {
    let library = ModuleBuilder::new("browser-library")
        .build()
        .to_bytes()
        .unwrap();
    let bytes = module_bytes(&[Instruction::RetVoid], 1, false);
    let mut vm = Engine::new();
    vm.add_library(&library).unwrap();
    vm.load_module(&bytes).unwrap();
    assert!(
        vm.add_library(&library)
            .unwrap_err()
            .contains("before loadModule")
    );
}

#[test]
fn exposes_runtime_crash_with_context() {
    let bytes = module_bytes(&[Instruction::Crash { r_msg: 0 }], 1, false);
    let mut vm = Engine::new();
    vm.load_module(&bytes).unwrap();
    let task = vm.spawn("main", vec![]).unwrap();
    assert_eq!(vm.tick(0.0, 10).unwrap(), "allCompleted");
    assert_eq!(vm.state(&task).unwrap(), "cancelled");
    let error = vm.task_error(&task).unwrap().unwrap();
    assert!(error.contains("Runtime crash"));
    assert!(error.contains("main"));
}

#[test]
fn host_request_can_be_inspected_resolved_and_resumed() {
    let bytes = module_bytes(
        &[
            Instruction::LoadInt { r_dst: 0, value: 6 },
            Instruction::CallExtern {
                r_dst: 1,
                extern_idx: 0x1000_0001,
                r_base: 0,
                argc: 1,
            },
            Instruction::Ret { r_src: 1 },
        ],
        2,
        true,
    );
    let mut vm = Engine::new();
    vm.load_module(&bytes).unwrap();
    let task = vm.spawn("main", vec![]).unwrap();
    assert_eq!(vm.tick(0.0, 20).unwrap(), "suspended");
    let requests = vm.pending_wire().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].id, 1);
    assert!(
        matches!(&requests[0].operation, WireOperation::ExternCall { name, args, .. }
        if name == "host_add_one" && args == &vec![WireValue::Int { value: "6".into() }])
    );
    vm.resolve(
        1,
        WireResponse::Value {
            value: WireValue::Int { value: "7".into() },
        },
    )
    .unwrap();
    assert_eq!(vm.tick(0.0, 20).unwrap(), "allCompleted");
    assert_eq!(
        vm.return_value(&task).unwrap(),
        Some(WireValue::Int { value: "7".into() })
    );
}

#[test]
fn deferred_marker_is_not_a_final_resolution() {
    let bytes = module_bytes(
        &[
            Instruction::CallExtern {
                r_dst: 0,
                extern_idx: 0x1000_0001,
                r_base: 0,
                argc: 0,
            },
            Instruction::RetVoid,
        ],
        1,
        true,
    );
    let mut vm = Engine::new();
    vm.load_module(&bytes).unwrap();
    vm.spawn("main", vec![]).unwrap();
    vm.tick(0.0, 20).unwrap();
    assert!(
        vm.resolve(1, WireResponse::Deferred)
            .unwrap_err()
            .contains("final response")
    );
    assert_eq!(vm.pending_wire().unwrap().len(), 1);
}

#[test]
fn rejects_invalid_request_ids_and_stale_task_handles() {
    let bytes = module_bytes(&[Instruction::RetVoid], 1, false);
    let mut vm = Engine::new();
    vm.load_module(&bytes).unwrap();
    assert!(
        vm.resolve(999, WireResponse::Confirmed)
            .unwrap_err()
            .contains("no suspended task")
    );
    assert!(vm.state(&handle(999, 0)).unwrap_err().contains("stale"));
}

#[test]
fn strings_are_allocated_and_never_expose_heap_references() {
    let bytes = module_bytes(&[Instruction::Ret { r_src: 0 }], 1, false);
    let mut vm = Engine::new();
    vm.load_module(&bytes).unwrap();
    let task = vm
        .spawn(
            "main",
            vec![WireValue::String {
                value: "browser safe".into(),
            }],
        )
        .unwrap();
    vm.tick(0.0, 10).unwrap();
    assert_eq!(
        vm.return_value(&task).unwrap(),
        Some(WireValue::String {
            value: "browser safe".into()
        })
    );
}

#[test]
fn log_messages_are_buffered_for_boundary_delivery() {
    let mut host = WasmHost {
        logs: vec![],
        extern_names: vec!["log::info".into()],
    };
    let response = host.on_request(
        RequestId(1),
        &HostRequest::ExternCall {
            task_id: TaskId::new(0, 0),
            extern_idx: 0x1000_0001,
            args: vec![],
            display_args: vec!["hello browser".into()],
        },
    );
    assert!(matches!(response, HostResponse::Confirmed));
    assert_eq!(host.logs.len(), 1);
    assert_eq!(host.logs[0].1, "hello browser");
}
