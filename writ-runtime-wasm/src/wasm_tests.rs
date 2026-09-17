use crate::{WireResponse, WireValue, WritVm};
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::JsValue;
use wasm_bindgen_test::*;
use writ_module::module::MethodBody;
use writ_module::signature::{TypeSignature, encode_method_signature};
use writ_module::{Instruction, ModuleBuilder};

wasm_bindgen_test_configure!(run_in_browser);

fn module_bytes(instructions: &[Instruction], with_extern: bool) -> Vec<u8> {
    let mut code = vec![];
    for instruction in instructions {
        instruction.encode(&mut code).unwrap();
    }
    let mut builder = ModuleBuilder::new("wasm-browser-test");
    if with_extern {
        let signature =
            encode_method_signature(&[], &TypeSignature::Int).expect("extern signature");
        builder.add_extern_def("answer", &signature, "answer", 0);
    }
    let signature = encode_method_signature(&[], &TypeSignature::Int).expect("main signature");
    builder.add_method(
        "main",
        &signature,
        0,
        1,
        MethodBody {
            register_types: vec![0],
            code,
            debug_locals: vec![],
            source_spans: vec![],
        },
    );
    builder.build().to_bytes().unwrap()
}

#[wasm_bindgen_test]
fn exported_api_loads_ticks_and_returns_a_value() {
    let bytes = module_bytes(
        &[
            Instruction::LoadInt {
                r_dst: 0,
                value: 42,
            },
            Instruction::Ret { r_src: 0 },
        ],
        false,
    );
    let mut vm = WritVm::new();
    vm.load_module(&bytes).unwrap();
    let task = vm
        .spawn(
            "main",
            serde_wasm_bindgen::to_value(&Vec::<WireValue>::new()).unwrap(),
        )
        .unwrap();
    vm.tick(0.0, 10).unwrap();
    assert_eq!(vm.task_state(task.clone()).unwrap(), "completed");
    let value: Option<WireValue> =
        serde_wasm_bindgen::from_value(vm.return_value(task).unwrap()).unwrap();
    assert_eq!(value, Some(WireValue::Int { value: "42".into() }));
}

#[wasm_bindgen_test]
fn callback_can_defer_then_javascript_can_resolve() {
    let bytes = module_bytes(
        &[
            Instruction::CallExtern {
                r_dst: 0,
                extern_idx: 0x1000_0001,
                r_base: 0,
                argc: 0,
            },
            Instruction::Ret { r_src: 0 },
        ],
        true,
    );
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let callback = Closure::wrap(Box::new(move |_request: JsValue| -> JsValue {
        observed.set(observed.get() + 1);
        serde_wasm_bindgen::to_value(&WireResponse::Deferred).unwrap()
    }) as Box<dyn FnMut(JsValue) -> JsValue>);

    let mut vm = WritVm::new();
    vm.set_host_callback(
        callback
            .as_ref()
            .unchecked_ref::<js_sys::Function>()
            .clone(),
    );
    vm.load_module(&bytes).unwrap();
    let task = vm
        .spawn(
            "main",
            serde_wasm_bindgen::to_value(&Vec::<WireValue>::new()).unwrap(),
        )
        .unwrap();
    vm.tick(0.0, 10).unwrap();
    assert_eq!(calls.get(), 1);
    let requests: Vec<crate::WireRequest> =
        serde_wasm_bindgen::from_value(vm.pending_requests().unwrap()).unwrap();
    assert_eq!(requests.len(), 1);
    vm.resolve_request(
        requests[0].id,
        serde_wasm_bindgen::to_value(&WireResponse::Value {
            value: WireValue::Int { value: "7".into() },
        })
        .unwrap(),
    )
    .unwrap();
    vm.tick(0.0, 10).unwrap();
    assert_eq!(vm.task_state(task).unwrap(), "completed");
    callback.forget();
}
