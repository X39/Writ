/// Integration tests for the writ-cli toolchain.
///
/// These tests exercise the assembler, disassembler, and runtime directly
/// (not via CLI process invocation) to validate end-to-end workflows.
use writ_assembler::{assemble, disassemble};
use writ_module::{
    heap::read_string, instruction::Instruction, module::MethodBody, Module, ModuleBuilder,
};
use writ_runtime::{
    ExecutionLimit, GcStats, HostRequest, HostResponse, LogLevel, RequestId, RuntimeBuilder,
    RuntimeHost, TickResult, Value,
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Encode a slice of instructions to bytes.
fn encode(instrs: &[Instruction]) -> Vec<u8> {
    let mut code = Vec::new();
    for instr in instrs {
        instr.encode(&mut code).unwrap();
    }
    code
}

// ─── Minimal test host that captures ExternCall invocations ──────────────────

struct TestSayHost {
    pub captured: Vec<String>,
    /// Captured display_args from ExternCall requests (FIX-03 verification).
    pub display_captured: Vec<Vec<String>>,
}

impl TestSayHost {
    fn new() -> Self {
        TestSayHost {
            captured: Vec::new(),
            display_captured: Vec::new(),
        }
    }
}

impl RuntimeHost for TestSayHost {
    fn on_request(&mut self, _id: RequestId, req: &HostRequest) -> HostResponse {
        match req {
            HostRequest::ExternCall { extern_idx, args, display_args, .. } => {
                let arg_strs: Vec<String> = args
                    .iter()
                    .map(|v| match v {
                        Value::Int(i) => i.to_string(),
                        Value::Float(f) => f.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Ref(_) => "<ref>".to_string(),
                        Value::Void => "void".to_string(),
                        Value::Entity(_) => "<entity>".to_string(),
                    })
                    .collect();
                self.captured
                    .push(format!("extern_idx={} args=[{}]", extern_idx, arg_strs.join(",")));
                // FIX-03: capture display_args for string content verification
                self.display_captured.push(display_args.clone());
                HostResponse::Value(Value::Void)
            }
            HostRequest::EntitySpawn { .. } => HostResponse::Confirmed,
            HostRequest::DestroyEntity { .. } => HostResponse::Confirmed,
            HostRequest::FieldRead { .. } => HostResponse::Value(Value::Int(0)),
            HostRequest::FieldWrite { .. } => HostResponse::Confirmed,
            HostRequest::GetComponent { .. } => HostResponse::Value(Value::Void),
            HostRequest::InitEntity { .. } => HostResponse::Confirmed,
            HostRequest::GetOrCreate { .. } => HostResponse::Confirmed,
            HostRequest::Join { .. } => HostResponse::Confirmed,
        }
    }

    fn on_log(&mut self, _level: LogLevel, _message: &str) {}
    fn on_gc_complete(&mut self, _stats: &GcStats) {}
}

// ─── Test: assemble and disassemble ──────────────────────────────────────────

#[test]
fn test_assemble_and_disassemble() {
    let src = r#"
.module "test" "0.1.0" {
    .method "greet" () -> void {
        .reg r0 int
        LOAD_INT r0 99
        RET_VOID
    }
}
"#;

    let module = assemble(src).expect("assemble should succeed");
    let text = disassemble(&module);

    assert!(text.contains(".module"), "output should contain .module directive");
    assert!(text.contains(".method"), "output should contain .method directive");
    assert!(text.contains("LOAD_INT"), "output should contain LOAD_INT instruction");
    assert!(text.contains("RET_VOID"), "output should contain RET_VOID instruction");
}

// ─── Test: run simple module (programmatic build with export) ─────────────────

#[test]
fn test_run_simple_module() {
    // Build a module programmatically with an export pointing to "main"
    let mut builder = ModuleBuilder::new("simple").version("0.1.0");

    let body = MethodBody {
        register_types: vec![0],
        code: encode(&[
            Instruction::LoadInt { r_dst: 0, value: 0 },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    let method_tok = builder.add_method("main", &[0x00], 0, 1, body);

    // Add export: item_kind=0 (Method), item=method_tok
    builder.add_export_def("main", 0, method_tok);

    let module = builder.build();

    // Serialize then deserialize to test the full round-trip
    let bytes = module.to_bytes().expect("to_bytes should succeed");
    let loaded = Module::from_bytes(&bytes).expect("from_bytes should succeed");

    // Find "main" export
    let main_export = loaded
        .export_defs
        .iter()
        .find(|e| read_string(&loaded.string_heap, e.name).unwrap_or("") == "main")
        .expect("should find 'main' export");

    assert_eq!(main_export.item_kind, 0, "export should be a method (kind=0)");

    let method_idx = (main_export.item.0 & 0x00FF_FFFF) as usize - 1;

    let mut runtime = RuntimeBuilder::new(loaded).build().expect("runtime should build");

    runtime.spawn_task(method_idx, vec![]).expect("spawn_task should succeed");

    loop {
        match runtime.tick(0.0, ExecutionLimit::None) {
            TickResult::AllCompleted | TickResult::Empty => break,
            TickResult::TasksSuspended(_) => break,
            TickResult::ExecutionLimitReached => break,
        }
    }
    // Test passes if no panic — the main method ran to completion
}

// ─── Test: entry point not found ──────────────────────────────────────────────

#[test]
fn test_entry_point_not_found() {
    // Build a module with no exports
    let mut builder = ModuleBuilder::new("noexport").version("0.1.0");

    let body = MethodBody {
        register_types: vec![0],
        code: encode(&[Instruction::RetVoid]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    builder.add_method("helper", &[0x00], 0, 1, body);

    let module = builder.build();

    // Attempt to find "main" export — should not be present
    let main_export = module
        .export_defs
        .iter()
        .find(|e| read_string(&module.string_heap, e.name).unwrap_or("") == "main");

    assert!(
        main_export.is_none(),
        "module with no exports should not have 'main' export"
    );

    // Simulate the error message logic from cmd_run
    let available: Vec<&str> = module
        .export_defs
        .iter()
        .filter_map(|e| read_string(&module.string_heap, e.name).ok())
        .collect();

    let error_msg = if available.is_empty() {
        "no exported method 'main' found. Available exports: (none)".to_string()
    } else {
        format!(
            "no exported method 'main' found. Available exports: [{}]",
            available.join(", ")
        )
    };

    assert!(
        error_msg.contains("no exported method 'main' found"),
        "error message should mention the missing export"
    );
}

// ─── Test: end-to-end dialogue say ────────────────────────────────────────────

#[test]
fn test_end_to_end_dialogue_say() {
    // Build a module with:
    // - An extern def "say" (gets token table_id=16, row=1 => 0x10000001)
    // - A "main" method that does LOAD_INT r0 42 + CALL_EXTERN r1 <extern_tok> r0 1 + RET_VOID
    // - An export for "main"
    let mut builder = ModuleBuilder::new("dialogue").version("0.1.0");

    // Add extern def — token will be (ExternDef=16 << 24) | 1 = 0x10000001
    let extern_tok = builder.add_extern_def("say", &[0x00], "", 0);

    // Build the method body
    let body = MethodBody {
        register_types: vec![0, 0],
        code: encode(&[
            Instruction::LoadInt { r_dst: 0, value: 42 },
            Instruction::CallExtern {
                r_dst: 1,
                extern_idx: extern_tok.0,
                r_base: 0,
                argc: 1,
            },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    let method_tok = builder.add_method("main", &[0x00], 0, 2, body);

    // Export "main" as method
    builder.add_export_def("main", 0, method_tok);

    let module = builder.build();

    // Step 2: Serialize to bytes
    let bytes = module.to_bytes().expect("to_bytes should succeed");

    // Step 3: Deserialize
    let loaded = Module::from_bytes(&bytes).expect("from_bytes should succeed");

    // Step 4: Find "main" export
    let main_export = loaded
        .export_defs
        .iter()
        .find(|e| read_string(&loaded.string_heap, e.name).unwrap_or("") == "main")
        .expect("should find 'main' export");

    let method_idx = (main_export.item.0 & 0x00FF_FFFF) as usize - 1;

    // Step 5: Build runtime with TestSayHost
    let test_host = TestSayHost::new();
    let mut runtime = RuntimeBuilder::new(loaded)
        .with_host(test_host)
        .build()
        .expect("runtime should build");

    // Step 6: Spawn and run
    runtime.spawn_task(method_idx, vec![]).expect("spawn_task should succeed");

    loop {
        match runtime.tick(0.0, ExecutionLimit::None) {
            TickResult::AllCompleted | TickResult::Empty => break,
            TickResult::TasksSuspended(_) => break,
            TickResult::ExecutionLimitReached => break,
        }
    }

    // Step 7: Assert captured output contains "42"
    let all_output = runtime.host().captured.join("\n");
    assert!(
        all_output.contains("42"),
        "expected captured say() output to contain '42', got: {:?}",
        all_output
    );
}

// ─── FIX-03: String display_args test ────────────────────────────────────────

/// FIX-03: When CALL_EXTERN passes a string argument (Value::Ref), the runtime
/// must pre-resolve it via the GC heap and supply the actual string content in
/// HostRequest::ExternCall::display_args.
///
/// Module layout:
///   ExternDef[0] "say"
///   method[0] "main":
///     LOAD_INT r0 99
///     I2S r1 r0        -- creates Value::Ref to "99" on GC heap
///     CALL_EXTERN r2 say r1 1
///     RET_VOID
///
/// The say() arg is Value::Ref (a GC heap string). display_args must show "99".
#[test]
fn fix03_extern_call_display_args_contains_string_content() {
    let mut builder = ModuleBuilder::new("fix03").version("0.1.0");

    // Add extern def "say"
    let extern_tok = builder.add_extern_def("say", &[0x00], "", 0);

    // Build method body: LOAD_INT r0 99 + I2S r1 r0 + CALL_EXTERN r2 say(r1) + RET_VOID
    // I2S converts int to string, producing a Value::Ref on the GC heap
    let body = MethodBody {
        register_types: vec![0, 0, 0],
        code: encode(&[
            Instruction::LoadInt { r_dst: 0, value: 99 },
            Instruction::I2s { r_dst: 1, r_src: 0 },
            Instruction::CallExtern {
                r_dst: 2,
                extern_idx: extern_tok.0,
                r_base: 1,
                argc: 1,
            },
            Instruction::RetVoid,
        ]),
        debug_locals: vec![],
        source_spans: vec![],
    };
    let method_tok = builder.add_method("main", &[0x00], 0, 3, body);
    builder.add_export_def("main", 0, method_tok);

    let module = builder.build();
    let bytes = module.to_bytes().expect("to_bytes should succeed");
    let loaded = Module::from_bytes(&bytes).expect("from_bytes should succeed");

    let main_export = loaded
        .export_defs
        .iter()
        .find(|e| read_string(&loaded.string_heap, e.name).unwrap_or("") == "main")
        .expect("should find 'main' export");
    let method_idx = (main_export.item.0 & 0x00FF_FFFF) as usize - 1;

    let test_host = TestSayHost::new();
    let mut runtime = RuntimeBuilder::new(loaded)
        .with_host(test_host)
        .build()
        .expect("runtime should build");

    runtime.spawn_task(method_idx, vec![]).expect("spawn should succeed");

    loop {
        match runtime.tick(0.0, ExecutionLimit::None) {
            TickResult::AllCompleted | TickResult::Empty => break,
            TickResult::TasksSuspended(_) => break,
            TickResult::ExecutionLimitReached => break,
        }
    }

    // FIX-03: display_args must contain the actual string "99", not "<ref>" or "<string>"
    let display = &runtime.host().display_captured;
    assert!(
        !display.is_empty(),
        "expected at least one ExternCall to have been recorded"
    );
    let first = &display[0];
    assert_eq!(
        first.len(), 1,
        "expected exactly one display arg, got {:?}", first
    );
    assert_eq!(
        first[0], "99",
        "display_args[0] must be the actual string content '99', got {:?}", first[0]
    );
}
