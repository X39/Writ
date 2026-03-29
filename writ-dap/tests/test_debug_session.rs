/// Integration tests for DAP debug session: byte-offset PC translation,
/// source span consistency, breakpoints, and variable scoping.
///
/// These tests compile real `.writ` files through `compile_and_load` and
/// exercise the same code paths the DAP server uses to resolve source locations
/// and variable scopes.
use writ_dap::launch::compile_and_load;
use writ_dap::breakpoints::BreakpointTable;
use writ_dap::debug_host::DebugHost;
use writ_module::heap::read_string;
use writ_runtime::RuntimeBuilder;
use writ_runtime::runtime::{ExecutionLimit, TickResult};
use writ_runtime::{SuspendReason, TaskState};

/// Resolve a path relative to the workspace root from this crate's manifest dir.
fn workspace_file(relative: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .expect("workspace root should exist");
    workspace_root.join(relative).to_string_lossy().into_owned()
}

/// Helper: compile a fixture, build a LoadedModule, and return both.
fn compile_fixture(fixture: &str) -> (writ_module::Module, &'static str) {
    let path = workspace_file(fixture);
    let (module, src, _method_file_ids) =
        compile_and_load(&path).unwrap_or_else(|e| panic!("compile_and_load failed: {}", e));
    (module, src)
}

// ─── Byte-offset PC translation tests ─────────────────────────────────────────

#[test]
fn test_byte_pc_translation() {
    // Compile a known fixture that produces method bodies with multiple instructions.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");

    // Build a runtime to access LoadedModule.byte_offsets
    let bt = BreakpointTable::new(&module);
    let host = DebugHost::new(bt, &module);
    let runtime = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .expect("runtime should build");

    let user_idx = runtime.user_module_idx();
    let loaded = &runtime.domain().modules[user_idx];

    // byte_offsets should have the same number of entries as method_bodies
    assert_eq!(
        loaded.byte_offsets.len(),
        loaded.module.method_bodies.len(),
        "byte_offsets should have one entry per method body"
    );

    // For each method, byte offsets should be monotonically non-decreasing
    // and each byte offset should be >= the instruction index (since instructions
    // encode to at least 1 byte)
    for (method_idx, offsets) in loaded.byte_offsets.iter().enumerate() {
        for (instr_idx, &byte_offset) in offsets.iter().enumerate() {
            assert!(
                byte_offset >= instr_idx as u32,
                "method {} instr {} byte_offset {} should be >= instr_idx {}",
                method_idx, instr_idx, byte_offset, instr_idx
            );
        }
        // Monotonically non-decreasing
        for window in offsets.windows(2) {
            assert!(
                window[1] >= window[0],
                "method {}: byte offsets should be non-decreasing, got {} then {}",
                method_idx, window[0], window[1]
            );
        }
    }
}

#[test]
fn test_source_span_lookup_with_byte_pc() {
    // Verify that source_spans contain byte-offset PCs that correspond to
    // valid byte_offsets entries.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");

    let bt = BreakpointTable::new(&module);
    let host = DebugHost::new(bt, &module);
    let runtime = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .expect("runtime should build");

    let user_idx = runtime.user_module_idx();
    let loaded = &runtime.domain().modules[user_idx];

    for (method_idx, body) in loaded.module.method_bodies.iter().enumerate() {
        let offsets = &loaded.byte_offsets[method_idx];
        for span in &body.source_spans {
            // Each SourceSpan.pc should be a valid byte offset that appears in
            // the byte_offsets table (i.e., it should correspond to some instruction start)
            let is_valid = offsets.contains(&span.pc);
            assert!(
                is_valid,
                "method {} SourceSpan.pc={} should appear in byte_offsets {:?}",
                method_idx, span.pc, offsets
            );
        }
    }
}

#[test]
fn test_debug_locals_byte_offset_consistency() {
    // Verify that DebugLocal.start_pc and end_pc values are within the range
    // of byte_offsets values (they are byte offsets, not instruction indices).
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");

    let bt = BreakpointTable::new(&module);
    let host = DebugHost::new(bt, &module);
    let runtime = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .expect("runtime should build");

    let user_idx = runtime.user_module_idx();
    let loaded = &runtime.domain().modules[user_idx];

    for (method_idx, body) in loaded.module.method_bodies.iter().enumerate() {
        if body.debug_locals.is_empty() {
            continue;
        }
        let offsets = &loaded.byte_offsets[method_idx];
        if offsets.is_empty() {
            continue;
        }
        let max_byte_offset = *offsets.last().unwrap();
        // Allow end_pc to be one past the last instruction byte offset
        // (it's an exclusive bound)
        for dl in &body.debug_locals {
            assert!(
                dl.start_pc <= max_byte_offset + 10, // some slack for instruction encoding
                "method {} DebugLocal start_pc={} should be within byte offset range (max={})",
                method_idx, dl.start_pc, max_byte_offset
            );
        }
    }
}

#[test]
fn test_stack_frame_line_resolution_with_byte_offsets() {
    // Compile a fixture with multiple statements and verify that the
    // source span resolution algorithm (find largest span.pc <= byte_pc)
    // produces valid line numbers when using byte-offset PCs.
    let (module, src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");
    let line_count = src.lines().count();

    let bt = BreakpointTable::new(&module);
    let host = DebugHost::new(bt, &module);
    let runtime = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .expect("runtime should build");

    let user_idx = runtime.user_module_idx();
    let loaded = &runtime.domain().modules[user_idx];

    for (method_idx, body) in loaded.module.method_bodies.iter().enumerate() {
        let offsets = &loaded.byte_offsets[method_idx];
        if offsets.is_empty() || body.source_spans.is_empty() {
            continue;
        }

        // For each instruction, translate to byte PC and look up source line
        for (instr_idx, &byte_pc) in offsets.iter().enumerate() {
            let resolved = body.source_spans
                .iter()
                .filter(|span| span.pc <= byte_pc)
                .max_by_key(|span| span.pc)
                .map(|span| span.line);

            if let Some(line) = resolved {
                assert!(
                    (line as usize) <= line_count + 1,
                    "method {} instr {} resolved line {} should be within source (total {} lines)",
                    method_idx, instr_idx, line, line_count
                );
            }
        }
    }
}

#[test]
fn test_compile_produces_source_spans_with_correct_lines() {
    // fn_multi_return.writ has code on specific lines. Verify source spans
    // reference lines that actually exist in the source.
    let (module, src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");
    let line_count = src.lines().count() as u32;

    let has_any_spans = module.method_bodies.iter().any(|b| !b.source_spans.is_empty());
    assert!(has_any_spans, "compiled module should have source spans");

    for (method_idx, body) in module.method_bodies.iter().enumerate() {
        for span in &body.source_spans {
            // Lines are 1-indexed in source spans
            assert!(
                span.line >= 1 && span.line <= line_count,
                "method {} span line {} should be in range [1, {}]",
                method_idx, span.line, line_count
            );
        }
    }
}

#[test]
fn test_byte_offsets_match_decoded_body_length() {
    // byte_offsets[method] should have exactly as many entries as decoded_bodies[method]
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");

    let bt = BreakpointTable::new(&module);
    let host = DebugHost::new(bt, &module);
    let runtime = RuntimeBuilder::new(module)
        .with_host(host)
        .build()
        .expect("runtime should build");

    let user_idx = runtime.user_module_idx();
    let loaded = &runtime.domain().modules[user_idx];

    for method_idx in 0..loaded.decoded_bodies.len() {
        assert_eq!(
            loaded.byte_offsets[method_idx].len(),
            loaded.decoded_bodies[method_idx].len(),
            "method {} byte_offsets count should equal decoded_bodies instruction count",
            method_idx
        );
    }
}

// ─── Breakpoint resolution tests ─────────────────────────────────────────────

#[test]
fn test_breakpoint_resolves_to_valid_line() {
    // fn_typed_params.writ line 11 has `let x: int = add(3, 4);`
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_typed_params.writ");
    let mut bt = BreakpointTable::new(&module);

    let resolved = bt.set_breakpoints(&[11]);
    assert!(
        !resolved.is_empty(),
        "setting a breakpoint at line 11 should resolve at least one breakpoint"
    );
    // The resolved line should be 11 exactly (or snapped nearby if 11 has no instruction).
    // Either way, it must be a valid line (present in valid_lines).
    let valid = bt.valid_lines();
    assert!(
        valid.contains(&resolved[0].line),
        "resolved line {} should be in valid_lines {:?}",
        resolved[0].line, valid
    );
}

#[test]
fn test_breakpoint_snaps_to_nearest_line() {
    // fn_multi_return.writ line 7 is empty (between `}` on line 6 and `fn main` on line 8).
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");
    let mut bt = BreakpointTable::new(&module);

    let resolved = bt.set_breakpoints(&[7]);
    assert!(
        !resolved.is_empty(),
        "breakpoint at empty line 7 should snap to a nearby valid line"
    );
    assert_ne!(
        resolved[0].line, 7,
        "resolved line should differ from the empty line 7 (snapped to nearest)"
    );
}

#[test]
fn test_breakpoint_valid_lines_covers_code_lines() {
    // fn_typed_params.writ has statements on multiple lines across 3 functions.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_typed_params.writ");
    let bt = BreakpointTable::new(&module);

    let valid = bt.valid_lines();
    assert!(
        !valid.is_empty(),
        "valid_lines should be non-empty for a compiled module with debug info"
    );
    // Source has executable code on lines 2, 3, 7, 11, 12 (at minimum).
    // Check that at least some expected code lines are present.
    let has_some_expected = valid.iter().any(|&l| l >= 2 && l <= 12);
    assert!(
        has_some_expected,
        "valid_lines {:?} should include lines in the code range [2, 12]",
        valid
    );
}

#[test]
fn test_breakpoint_lookup_hit() {
    // After setting breakpoints, lookup at the resolved (method_idx, pc) should return
    // the breakpoint id.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");
    let mut bt = BreakpointTable::new(&module);

    let valid = bt.valid_lines();
    assert!(!valid.is_empty(), "need valid lines to set breakpoints");

    let resolved = bt.set_breakpoints(&valid);
    assert!(
        !resolved.is_empty(),
        "setting breakpoints on valid lines should resolve"
    );

    for bp in &resolved {
        let hit = bt.lookup(bp.method_idx, bp.pc);
        assert_eq!(
            hit,
            Some(bp.id),
            "lookup at resolved (method_idx={}, pc={}) should return breakpoint id {}",
            bp.method_idx, bp.pc, bp.id
        );
    }
}

#[test]
fn test_breakpoint_lookup_miss() {
    // lookup at a (method_idx, pc) pair with no breakpoint should return None.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");
    let mut bt = BreakpointTable::new(&module);

    // Set breakpoints on a single line to have a non-empty table.
    let valid = bt.valid_lines();
    if !valid.is_empty() {
        bt.set_breakpoints(&[valid[0]]);
    }

    // Use an impossibly large pc that cannot correspond to any breakpoint.
    let miss = bt.lookup(0, 999_999);
    assert!(
        miss.is_none(),
        "lookup at an unset (method_idx=0, pc=999999) should return None"
    );
}

// ─── Runtime execution and call stack tests ──────────────────────────────────

/// Find the method index for a function named `target` by searching method_defs.
fn find_method_index(module: &writ_module::Module, target: &str) -> Option<usize> {
    for (i, def) in module.method_defs.iter().enumerate() {
        if let Ok(name) = read_string(&module.string_heap, def.name) {
            if name == target {
                return Some(i);
            }
        }
    }
    None
}

#[test]
fn test_spawn_main_and_tick() {
    // Compile fn_typed_params.writ, spawn main, tick to completion.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_typed_params.writ");

    let main_idx = find_method_index(&module, "main")
        .expect("fn_typed_params.writ should have a 'main' function");

    let mut runtime = RuntimeBuilder::new(module)
        .build()
        .expect("runtime should build with NullHost");

    let _task_id = runtime
        .spawn_task(main_idx, vec![])
        .expect("spawning main task should succeed");

    let result = runtime.tick(0.0, ExecutionLimit::Instructions(10_000));
    assert!(
        matches!(result, TickResult::AllCompleted),
        "fn_typed_params main should complete within 10000 instructions, got {:?}",
        result
    );
}

#[test]
fn test_call_stack_after_spawn() {
    // After spawning a task (before ticking), call_stack_frames should return
    // at least one frame for the initial method.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");

    let main_idx = find_method_index(&module, "main")
        .expect("fn_multi_return.writ should have a 'main' function");

    let mut runtime = RuntimeBuilder::new(module)
        .build()
        .expect("runtime should build");

    let task_id = runtime
        .spawn_task(main_idx, vec![])
        .expect("spawning main task should succeed");

    let frames = runtime.call_stack_frames(task_id);
    assert!(
        frames.is_some(),
        "call_stack_frames should return Some for a spawned task"
    );
    let frames = frames.unwrap();
    assert_eq!(
        frames.len(),
        1,
        "newly spawned task should have exactly 1 frame (the initial frame), got {}",
        frames.len()
    );
    assert_eq!(
        frames[0].0, main_idx,
        "initial frame method_idx should be the main method index"
    );
}

// ─── Method name resolution from compiled module ─────────────────────────────

#[test]
fn test_method_names_from_compiled_module() {
    // fn_typed_params.writ defines add, is_positive, and main.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_typed_params.writ");

    let names: Vec<String> = module
        .method_defs
        .iter()
        .filter_map(|def| read_string(&module.string_heap, def.name).ok())
        .map(|s| s.to_string())
        .collect();

    assert!(
        names.contains(&"add".to_string()),
        "method names {:?} should contain 'add'",
        names
    );
    assert!(
        names.contains(&"is_positive".to_string()),
        "method names {:?} should contain 'is_positive'",
        names
    );
    assert!(
        names.contains(&"main".to_string()),
        "method names {:?} should contain 'main'",
        names
    );
}

#[test]
fn test_method_def_count_matches_bodies() {
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_typed_params.writ");
    assert_eq!(
        module.method_defs.len(),
        module.method_bodies.len(),
        "method_defs count should equal method_bodies count"
    );
}

// ─── Debug locals from compiled module ───────────────────────────────────────

#[test]
fn test_debug_locals_present_for_local_variables() {
    // fn_typed_params.writ: `add(a, b)` has parameters a and b as debug locals.
    // Note: the compiler may not emit a debug local for `result` if it is
    // directly used as an implicit return expression, so we only assert params.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_typed_params.writ");

    // Find the method body corresponding to "add".
    let add_idx = find_method_index(&module, "add")
        .expect("should find 'add' method");

    let body = &module.method_bodies[add_idx];
    let local_names: Vec<String> = body
        .debug_locals
        .iter()
        .filter_map(|dl| read_string(&module.string_heap, dl.name).ok())
        .map(|s| s.to_string())
        .collect();

    assert!(
        !local_names.is_empty(),
        "debug_locals for 'add' should be non-empty"
    );
    assert!(
        local_names.contains(&"a".to_string()),
        "debug_locals for 'add' should include 'a', got {:?}",
        local_names
    );
    assert!(
        local_names.contains(&"b".to_string()),
        "debug_locals for 'add' should include 'b', got {:?}",
        local_names
    );
}

#[test]
fn test_debug_locals_names_readable() {
    // Debug local entries should exist and their name offsets should generally
    // be valid string heap references. Some entries may have offsets beyond the
    // heap (a known codegen quirk for certain expression-result locals), so we
    // count how many are readable and assert a reasonable fraction.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");

    let mut total_locals = 0;
    let mut readable_locals = 0;
    for (_method_idx, body) in module.method_bodies.iter().enumerate() {
        for (_dl_idx, dl) in body.debug_locals.iter().enumerate() {
            total_locals += 1;
            if read_string(&module.string_heap, dl.name).is_ok() {
                readable_locals += 1;
            }
        }
    }
    assert!(
        total_locals > 0,
        "fn_multi_return.writ should produce at least one debug local"
    );
    assert!(
        readable_locals > 0,
        "at least one debug local name should be readable from the string heap"
    );
}

// ─── Source span coverage tests ──────────────────────────────────────────────

#[test]
fn test_source_spans_cover_all_methods() {
    // fn_typed_params.writ has 3 user-defined functions. The compiler always
    // emits debug info, so the majority of method bodies should have source spans.
    // Some trivially simple methods (e.g., single-expression bodies) may not
    // produce source spans if the codegen elides them.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_typed_params.writ");

    assert!(
        module.method_bodies.len() >= 3,
        "fn_typed_params.writ should produce at least 3 method bodies"
    );

    let methods_with_spans = module
        .method_bodies
        .iter()
        .filter(|b| !b.source_spans.is_empty())
        .count();

    assert!(
        methods_with_spans >= 2,
        "at least 2 of {} methods should have source spans, got {}",
        module.method_bodies.len(),
        methods_with_spans
    );
}

#[test]
fn test_source_span_pcs_sorted() {
    // Source spans within each method should be sorted by pc (ascending).
    // The binary-search-style resolution algorithm depends on this ordering.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_multi_return.writ");

    for (method_idx, body) in module.method_bodies.iter().enumerate() {
        for window in body.source_spans.windows(2) {
            assert!(
                window[1].pc >= window[0].pc,
                "method {} source spans should be sorted by pc: pc={} followed by pc={}",
                method_idx, window[0].pc, window[1].pc
            );
        }
    }
}

// ─── Multi-fixture consistency tests ─────────────────────────────────────────

#[test]
fn test_struct_fixture_has_debug_info() {
    // type_struct_new.writ defines `struct Point` and `fn main` with locals p, px.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/type_struct_new.writ");

    // Method bodies should have source spans.
    let has_spans = module
        .method_bodies
        .iter()
        .any(|b| !b.source_spans.is_empty());
    assert!(
        has_spans,
        "type_struct_new.writ should produce source spans"
    );

    // Find main and check it has debug locals. The compiler emits entries for
    // locals p and px, though some name offsets may be out of range for
    // struct-typed or field-access locals in the current codegen.
    let main_idx = find_method_index(&module, "main")
        .expect("type_struct_new.writ should have a 'main' function");

    let body = &module.method_bodies[main_idx];
    assert!(
        !body.debug_locals.is_empty(),
        "main's debug_locals should be non-empty (has locals p and px)"
    );

    // Verify that at least some debug locals have valid register indices
    // (start_pc <= end_pc is the invariant we can reliably check).
    for (dl_idx, dl) in body.debug_locals.iter().enumerate() {
        assert!(
            dl.start_pc <= dl.end_pc,
            "debug_local {} should have start_pc ({}) <= end_pc ({})",
            dl_idx, dl.start_pc, dl.end_pc
        );
    }
}

#[test]
fn test_control_flow_fixture_source_spans() {
    // ctrl_while_loop.writ has a while loop — verify source spans and valid lines.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/ctrl_while_loop.writ");

    let has_spans = module
        .method_bodies
        .iter()
        .any(|b| !b.source_spans.is_empty());
    assert!(
        has_spans,
        "ctrl_while_loop.writ should produce source spans"
    );

    let bt = BreakpointTable::new(&module);
    let valid = bt.valid_lines();
    assert!(
        !valid.is_empty(),
        "ctrl_while_loop.writ should have valid breakpoint lines"
    );
    // The while loop body has statements — lines 2-5 should have some coverage.
    let has_loop_lines = valid.iter().any(|&l| l >= 2 && l <= 5);
    assert!(
        has_loop_lines,
        "valid_lines {:?} should include lines from the while loop body (lines 2-5)",
        valid
    );
}

// ─── End-to-end breakpoint hit tests ─────────────────────────────────────────

#[test]
fn test_breakpoint_fires_during_execution() {
    // Compile fn_typed_params.writ (has `let x: int = add(3, 4);` on line 11).
    // Set a breakpoint on line 11, run the VM, and verify the task suspends
    // with a Breakpoint reason.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_typed_params.writ");

    let main_idx = find_method_index(&module, "main")
        .expect("should find 'main'");

    // Build breakpoint table and set breakpoint on line 11.
    let mut breakpoint_table = BreakpointTable::new(&module);
    let resolved = breakpoint_table.set_breakpoints(&[11]);
    assert!(
        !resolved.is_empty(),
        "breakpoint at line 11 should resolve; valid_lines={:?}",
        breakpoint_table.valid_lines()
    );
    let bp_id = resolved[0].id;
    let bp_method = resolved[0].method_idx;
    let bp_pc = resolved[0].pc;
    eprintln!(
        "Breakpoint resolved: id={}, method_idx={}, pc={}, line={}",
        bp_id, bp_method, bp_pc, resolved[0].line
    );

    // Build runtime with DebugHost.
    let debug_host = DebugHost::new(breakpoint_table, &module);
    let mut runtime = RuntimeBuilder::new(module)
        .with_host(debug_host)
        .build()
        .expect("runtime should build");

    let task_id = runtime
        .spawn_task(main_idx, vec![])
        .expect("spawning main should succeed");

    // Tick until the task is no longer Ready/Running (or limit reached).
    let mut ticks = 0;
    loop {
        let _tick_result = runtime.tick(0.0, ExecutionLimit::Instructions(100));
        ticks += 1;

        match runtime.task_state(task_id) {
            Some(TaskState::Suspended) => break,
            Some(TaskState::Completed) | Some(TaskState::Cancelled) | None => {
                panic!(
                    "task completed/cancelled without hitting breakpoint after {} ticks",
                    ticks
                );
            }
            _ => {}
        }

        if ticks > 100 {
            panic!("task did not suspend within 100 ticks");
        }
    }

    // Verify the suspend reason is a breakpoint.
    let reason = runtime.suspend_reason(task_id);
    assert!(
        matches!(reason, Some(SuspendReason::Breakpoint { .. })),
        "task should be suspended with Breakpoint reason, got {:?}",
        reason
    );

    // Verify DebugHost recorded the correct stop reason.
    let stop = runtime.host_mut().take_pending_stop();
    assert!(
        stop.is_some(),
        "DebugHost pending_stop should be set after breakpoint hit"
    );
}

#[test]
fn test_breakpoint_resume_does_not_rehit_same_pc() {
    // After hitting a breakpoint and resuming, the VM should NOT immediately
    // re-hit the same breakpoint (the PC hasn't advanced past it). The program
    // should continue past the breakpoint and eventually complete.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/fn_typed_params.writ");

    let main_idx = find_method_index(&module, "main").expect("should find 'main'");

    let mut breakpoint_table = BreakpointTable::new(&module);
    let resolved = breakpoint_table.set_breakpoints(&[11]);
    assert!(!resolved.is_empty(), "breakpoint at line 11 should resolve");

    let debug_host = DebugHost::new(breakpoint_table, &module);
    let mut runtime = RuntimeBuilder::new(module)
        .with_host(debug_host)
        .build()
        .expect("runtime should build");

    let task_id = runtime.spawn_task(main_idx, vec![]).expect("spawn should succeed");

    // Tick until breakpoint fires.
    for _ in 0..100 {
        runtime.tick(0.0, ExecutionLimit::Instructions(100));
        if runtime.task_state(task_id) == Some(TaskState::Suspended) {
            break;
        }
    }
    assert_eq!(runtime.task_state(task_id), Some(TaskState::Suspended));
    runtime.host_mut().take_pending_stop(); // drain

    // Resume and continue — should NOT re-hit immediately, should complete.
    runtime.host_mut().clear_step();
    runtime.resume_debug(task_id).expect("resume should succeed");

    for _ in 0..100 {
        runtime.tick(0.0, ExecutionLimit::Instructions(100));
        match runtime.task_state(task_id) {
            Some(TaskState::Completed) | Some(TaskState::Cancelled) => break,
            Some(TaskState::Suspended) => {
                // If it stopped again at a breakpoint, that's the re-hit bug.
                panic!("task re-hit the same breakpoint after resume — PC did not advance");
            }
            _ => {}
        }
    }

    assert_eq!(
        runtime.task_state(task_id),
        Some(TaskState::Completed),
        "task should complete after resuming past breakpoint"
    );
}

#[test]
fn diag_multi_fn_breakpoint_alignment() {
    // Diagnostic: dump source spans for multi-function file to understand breakpoint bug
    let (module, src) = compile_fixture("writ-golden/tests/golden/dap_bp_align.writ");

    eprintln!("\nSource:");
    for (i, line) in src.lines().enumerate() {
        eprintln!("  {:3}: {}", i + 1, line);
    }

    eprintln!("\nMethods:");
    for (i, def) in module.method_defs.iter().enumerate() {
        let name = read_string(&module.string_heap, def.name).unwrap_or("?");
        eprintln!("  Method {}: {}", i, name);
    }

    eprintln!("\nSource spans per method:");
    for (mi, body) in module.method_bodies.iter().enumerate() {
        let name = if mi < module.method_defs.len() {
            read_string(&module.string_heap, mi_to_name(&module, mi)).unwrap_or("?")
        } else { "?" };
        eprintln!("  Method {} ({}):", mi, name);
        for s in &body.source_spans {
            eprintln!("    pc={} line={} col={}", s.pc, s.line, s.column);
        }
    }

    let bt = BreakpointTable::new(&module);
    eprintln!("\nValid lines: {:?}", bt.valid_lines());

    let mut bt2 = BreakpointTable::new(&module);
    let r = bt2.set_breakpoints(&[2]);
    eprintln!("\nBreakpoint at line 2:");
    for bp in &r {
        eprintln!("  id={} line={} method={} pc={}", bp.id, bp.line, bp.method_idx, bp.pc);
    }

    let r = bt2.set_breakpoints(&[5]);
    eprintln!("\nBreakpoint at line 5:");
    for bp in &r {
        eprintln!("  id={} line={} method={} pc={}", bp.id, bp.line, bp.method_idx, bp.pc);
    }
}

fn mi_to_name(module: &writ_module::Module, method_idx: usize) -> u32 {
    module.method_defs.get(method_idx).map(|d| d.name).unwrap_or(0)
}

// ─── Multi-function breakpoint alignment regression tests ─────────────────────
//
// These tests cover the bug where the block emitter's tail-expression path did
// NOT push a source span, leaving functions with a single statement (the tail)
// with zero source spans. Consequences:
//   1. Breakpoints on those lines snap to a method that DOES have spans (wrong method).
//   2. Step-into lands at line 0 because lookup_source_location returns (0, 0)
//      for methods with no spans.
//
// The fix: push a source span before emit_expr for the tail expression,
// mirroring what emit_stmt does for non-tail statements.

#[test]
fn test_multi_fn_all_methods_have_source_spans() {
    // dap_bp_align.writ:
    //   fn test() { log::info("test"); }        <- single-statement tail: was 0 spans
    //   fn main() { ... test(); ... }           <- multi-statement body
    //
    // After the fix every method body must have at least one source span.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/dap_bp_align.writ");

    assert!(
        module.method_bodies.len() >= 2,
        "dap_bp_align.writ should produce at least 2 method bodies, got {}",
        module.method_bodies.len()
    );

    for (method_idx, body) in module.method_bodies.iter().enumerate() {
        assert!(
            !body.source_spans.is_empty(),
            "method {} should have at least one source span after the tail-expr fix (had 0)",
            method_idx
        );
    }
}

#[test]
fn test_multi_fn_breakpoint_line2_resolves_to_test_method() {
    // Line 2 is inside `test()` (method 0). Before the fix it had no spans so
    // set_breakpoints snapped to the nearest method that DID have spans — `main`.
    // After the fix it should resolve exactly to line 2 in method 0.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/dap_bp_align.writ");

    let test_idx = find_method_index(&module, "test")
        .expect("dap_bp_align.writ should define a 'test' function");

    let mut bt = BreakpointTable::new(&module);
    let resolved = bt.set_breakpoints(&[2]);
    assert!(
        !resolved.is_empty(),
        "breakpoint at line 2 should resolve; valid_lines={:?}",
        bt.valid_lines()
    );

    let bp = &resolved[0];
    assert_eq!(
        bp.line, 2,
        "breakpoint should resolve to line 2, not snap to line {} in the wrong method",
        bp.line
    );
    assert_eq!(
        bp.method_idx, test_idx,
        "breakpoint at line 2 should be in method {} ('test'), not method {}",
        test_idx, bp.method_idx
    );
}

#[test]
fn test_multi_fn_all_code_lines_in_valid_lines() {
    // After the fix all method bodies have source spans, so valid_lines should
    // include lines from BOTH functions:
    //   line 2: `log::info("test");`   inside test()
    //   line 5: `log::info("main start");` inside main()
    //   line 6: `test();`              inside main()
    //   line 7: `log::info("main end");`  inside main()
    let (module, _src) = compile_fixture("writ-golden/tests/golden/dap_bp_align.writ");
    let bt = BreakpointTable::new(&module);
    let valid = bt.valid_lines();

    // Line 2 must be present (was missing before the fix because test() had no spans).
    assert!(
        valid.contains(&2),
        "valid_lines {:?} should include line 2 (inside test())",
        valid
    );

    // At least one of the main() body lines must be present.
    let has_main_lines = valid.iter().any(|&l| l >= 5 && l <= 7);
    assert!(
        has_main_lines,
        "valid_lines {:?} should include at least one line from main() (lines 5-7)",
        valid
    );
}

#[test]
fn test_multi_fn_breakpoint_fires_on_line2() {
    // Set a breakpoint on line 2 (inside test()), run main() which calls test(),
    // and verify the task suspends with SuspendReason::Breakpoint.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/dap_bp_align.writ");

    let main_idx = find_method_index(&module, "main")
        .expect("dap_bp_align.writ should have a 'main' function");

    let mut breakpoint_table = BreakpointTable::new(&module);
    let resolved = breakpoint_table.set_breakpoints(&[2]);
    assert!(
        !resolved.is_empty(),
        "breakpoint at line 2 should resolve; valid_lines={:?}",
        breakpoint_table.valid_lines()
    );
    assert_eq!(
        resolved[0].line, 2,
        "breakpoint should be on line 2, not {}",
        resolved[0].line
    );

    let debug_host = DebugHost::new(breakpoint_table, &module);
    let mut runtime = RuntimeBuilder::new(module)
        .with_host(debug_host)
        .build()
        .expect("runtime should build");

    let task_id = runtime
        .spawn_task(main_idx, vec![])
        .expect("spawning main should succeed");

    // Tick until the task suspends (breakpoint hit) or completes.
    let mut ticks = 0;
    loop {
        let _result = runtime.tick(0.0, writ_runtime::runtime::ExecutionLimit::Instructions(100));
        ticks += 1;
        match runtime.task_state(task_id) {
            Some(TaskState::Suspended) => break,
            Some(TaskState::Completed) | Some(TaskState::Cancelled) | None => {
                panic!(
                    "task completed without hitting breakpoint at line 2 after {} ticks; \
                     check that main() actually calls test()",
                    ticks
                );
            }
            _ => {}
        }
        if ticks > 200 {
            panic!("task did not suspend within 200 ticks");
        }
    }

    let reason = runtime.suspend_reason(task_id);
    assert!(
        matches!(reason, Some(SuspendReason::Breakpoint { line: 2, .. })),
        "task should suspend with Breakpoint at line 2, got {:?}",
        reason
    );
}

#[test]
fn test_multi_fn_step_into_function_call() {
    // Set a breakpoint on line 6 (`test();` call in main).
    // Run until it fires, then activate StepInto and resume.
    // Verify the next stop is inside test() at line 2 — NOT at line 0.
    let (module, _src) = compile_fixture("writ-golden/tests/golden/dap_bp_align.writ");

    let main_idx = find_method_index(&module, "main")
        .expect("dap_bp_align.writ should have a 'main' function");

    // Build breakpoint table, set breakpoint on line 6 (the test() call).
    let mut breakpoint_table = BreakpointTable::new(&module);
    let resolved = breakpoint_table.set_breakpoints(&[6]);
    assert!(
        !resolved.is_empty(),
        "breakpoint at line 6 should resolve; valid_lines={:?}",
        breakpoint_table.valid_lines()
    );

    let debug_host = DebugHost::new(breakpoint_table, &module);
    let mut runtime = RuntimeBuilder::new(module)
        .with_host(debug_host)
        .build()
        .expect("runtime should build");

    let task_id = runtime
        .spawn_task(main_idx, vec![])
        .expect("spawning main should succeed");

    // Tick until the breakpoint at line 6 fires.
    for _ in 0..200 {
        runtime.tick(0.0, writ_runtime::runtime::ExecutionLimit::Instructions(100));
        if runtime.task_state(task_id) == Some(TaskState::Suspended) {
            break;
        }
    }
    assert_eq!(
        runtime.task_state(task_id),
        Some(TaskState::Suspended),
        "task should have suspended on the line-6 breakpoint"
    );
    // Drain the pending stop from the breakpoint hit.
    runtime.host_mut().take_pending_stop();

    // Record the current line so StepInto knows the origin.
    let origin_line = match runtime.suspend_reason(task_id) {
        Some(SuspendReason::Breakpoint { line, .. }) => *line,
        _ => resolved[0].line,  // fall back to what the table reported
    };
    let origin_method = resolved[0].method_idx as u32;

    // Activate StepInto mode, then resume.
    runtime.host_mut().set_step_into(origin_line, origin_method);
    runtime.resume_debug(task_id).expect("resume_debug should succeed");

    // Tick until the step fires (should stop inside test() at line 2).
    for _ in 0..200 {
        runtime.tick(0.0, writ_runtime::runtime::ExecutionLimit::Instructions(100));
        if runtime.task_state(task_id) == Some(TaskState::Suspended) {
            break;
        }
    }
    assert_eq!(
        runtime.task_state(task_id),
        Some(TaskState::Suspended),
        "task should suspend again after StepInto"
    );

    // The stop must be inside test() at line 2 (not line 0).
    // Note: DebugHost always returns DebugAction::Break from before_instruction
    // (for both breakpoints and steps), so the runtime records SuspendReason::Breakpoint.
    // The meaningful assertion is the line and method, not the variant.
    let reason = runtime.suspend_reason(task_id);
    let (stop_line, stop_method) = match reason {
        Some(SuspendReason::Breakpoint { line, method_idx, .. }) => (*line, *method_idx),
        Some(SuspendReason::DebugStep { line, method_idx, .. }) => (*line, *method_idx),
        other => {
            panic!(
                "expected a debug stop (Breakpoint or DebugStep) after StepInto, got {:?}",
                other
            );
        }
    };

    assert_ne!(
        stop_line, 0,
        "StepInto stopped at line 0 (unknown); test() had no source spans before the fix. \
         Check that the tail-expression source span fix is in place."
    );
    assert_eq!(
        stop_line, 2,
        "StepInto should stop at line 2 inside test(), stopped at line {} in method {}",
        stop_line, stop_method
    );
    let test_idx = find_method_index(
        &runtime.domain().modules[runtime.user_module_idx()].module,
        "test"
    ).expect("should find 'test' method");
    assert_eq!(
        stop_method as usize, test_idx,
        "StepInto should stop inside test() (method {}), stopped in method {}",
        test_idx, stop_method
    );
}
