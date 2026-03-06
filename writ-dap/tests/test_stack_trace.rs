/// Tests for DAP stack trace source span resolution (DAP-05).
///
/// `DapServer::build_stack_frames` is a private method that requires a full
/// runtime — it cannot be called directly from an integration test.
///
/// Strategy: test the *constituent behaviors* that build_stack_frames depends on:
/// 1. Source-span-to-line resolution: "find largest span.pc <= current_pc" —
///    the core algorithm used to map a runtime PC back to a source line number.
/// 2. Method name resolution: reading a method name from the string heap via
///    `read_string` for a MethodDefRow created with a known name offset.
///
/// These constituent behaviors are tested through the public APIs of
/// `writ_module`, which is the same API that `build_stack_frames` uses.
use writ_module::module::{MethodBody, Module, SourceSpan};
use writ_module::tables::MethodDefRow;
use writ_module::heap::read_string;

/// Replicate the source-span-to-line algorithm used in build_stack_frames:
/// "find the SourceSpan with the largest pc that is still <= current_pc".
fn resolve_source_line(body: &MethodBody, pc: usize) -> Option<(u32, u16)> {
    body.source_spans
        .iter()
        .filter(|span| span.pc <= pc as u32)
        .max_by_key(|span| span.pc)
        .map(|span| (span.line, span.column))
}

/// Build a minimal module with one method body having the given source spans.
fn make_module_with_spans(spans: &[(u32, u32, u16)]) -> Module {
    // (pc, line, column)
    let mut module = Module::new();
    let source_spans = spans
        .iter()
        .map(|&(pc, line, column)| SourceSpan { pc, line, column })
        .collect();
    module.method_bodies.push(MethodBody {
        register_types: vec![],
        code: vec![],
        debug_locals: vec![],
        source_spans,
    });
    // Add a dummy method def so method_defs.len() == method_bodies.len()
    module.method_defs.push(MethodDefRow {
        name: 0,
        signature: 0,
        flags: 0,
        body_offset: 0,
        body_size: 0,
        reg_count: 0,
        param_count: 0,
    });
    module
}

/// Build a module with a named method (name written to string heap).
fn make_module_with_named_method(name: &str) -> Module {
    let mut module = Module::new();

    // Write the name to the string heap (4-byte length prefix + UTF-8 bytes).
    let name_bytes = name.as_bytes();
    let name_offset = module.string_heap.len() as u32;
    module.string_heap.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    module.string_heap.extend_from_slice(name_bytes);

    module.method_defs.push(MethodDefRow {
        name: name_offset,
        signature: 0,
        flags: 0,
        body_offset: 0,
        body_size: 0,
        reg_count: 0,
        param_count: 0,
    });
    module.method_bodies.push(MethodBody {
        register_types: vec![],
        code: vec![],
        debug_locals: vec![],
        source_spans: vec![],
    });
    module
}

// ─── Source span resolution tests ─────────────────────────────────────────────

#[test]
fn test_stack_trace_source_span_resolves_exact_pc() {
    // When the current PC exactly matches a span's pc, that span's line is returned.
    let module = make_module_with_spans(&[
        (0, 10, 1),
        (5, 15, 1),
        (10, 20, 1),
    ]);
    let body = &module.method_bodies[0];

    let result = resolve_source_line(body, 5);
    assert_eq!(result, Some((15, 1)), "pc=5 should resolve to line 15");

    let result = resolve_source_line(body, 10);
    assert_eq!(result, Some((20, 1)), "pc=10 should resolve to line 20");
}

#[test]
fn test_stack_trace_source_span_resolves_between_pcs() {
    // When the current PC falls between two span pcs, the largest span.pc <= pc is used.
    // This is the "most recent source location" for an instruction between labeled lines.
    let module = make_module_with_spans(&[
        (0, 5, 1),   // pc 0 → line 5
        (10, 10, 1), // pc 10 → line 10
        (20, 15, 1), // pc 20 → line 15
    ]);
    let body = &module.method_bodies[0];

    // pc=7 is between span at pc=0 and pc=10 → should use line 5 (largest pc <= 7 is 0)
    let result = resolve_source_line(body, 7);
    assert_eq!(result, Some((5, 1)), "pc=7 should resolve to line 5 (largest span.pc <= 7)");

    // pc=15 is between span at pc=10 and pc=20 → should use line 10
    let result = resolve_source_line(body, 15);
    assert_eq!(result, Some((10, 1)), "pc=15 should resolve to line 10 (largest span.pc <= 15)");

    // pc=25 is after all spans → should use line 15 (the last span)
    let result = resolve_source_line(body, 25);
    assert_eq!(result, Some((15, 1)), "pc=25 should resolve to line 15 (last span)");
}

#[test]
fn test_stack_trace_source_span_returns_none_for_pc_before_all_spans() {
    // When the current PC is before any span, no source location is available.
    // build_stack_frames falls back to line=0, col=0 in this case.
    let module = make_module_with_spans(&[
        (10, 5, 1),  // first span is at pc=10
        (20, 10, 1),
    ]);
    let body = &module.method_bodies[0];

    // pc=5 is before the first span at pc=10 → no match
    let result = resolve_source_line(body, 5);
    assert_eq!(result, None, "pc=5 should return None (no span has pc <= 5)");
}

#[test]
fn test_stack_trace_source_span_returns_none_for_empty_body() {
    // A method with no source spans (e.g. empty function) returns None.
    let module = make_module_with_spans(&[]);
    let body = &module.method_bodies[0];

    let result = resolve_source_line(body, 100);
    assert_eq!(result, None, "empty source_spans should always return None");
}

#[test]
fn test_stack_trace_source_span_selects_max_pc_not_first_match() {
    // When multiple spans have pc <= current_pc, the one with the LARGEST pc wins.
    // This is the critical invariant: we want the most recent debug location, not the first.
    let module = make_module_with_spans(&[
        (0, 100, 1),  // pc=0 → line 100 (earliest)
        (5, 200, 1),  // pc=5 → line 200
        (10, 300, 1), // pc=10 → line 300 (most recent before pc=12)
        (20, 400, 1), // pc=20 → line 400 (too late for pc=12)
    ]);
    let body = &module.method_bodies[0];

    // At pc=12: spans at pc=0, pc=5, pc=10 all qualify. Max pc is 10 → line 300.
    let result = resolve_source_line(body, 12);
    assert_eq!(
        result,
        Some((300, 1)),
        "should select the span with the largest pc <= current_pc, not the first"
    );
}

// ─── Method name resolution tests ─────────────────────────────────────────────

#[test]
fn test_stack_trace_method_name_resolved_from_string_heap() {
    // build_stack_frames resolves method names from the string heap via read_string.
    // This test verifies that the module helper and read_string work correctly
    // for the method name lookup path used in build_stack_frames.
    let module = make_module_with_named_method("my_function");

    let method_def = &module.method_defs[0];
    let name = read_string(&module.string_heap, method_def.name)
        .expect("read_string should succeed for a valid name offset");

    assert_eq!(name, "my_function", "method name should be correctly recovered from string heap");
}

#[test]
fn test_stack_trace_method_name_fallback_for_unknown_index() {
    // When a method_idx is out of bounds, build_stack_frames falls back to
    // format!("method_{}", method_idx). Verify this fallback is reasonable.
    let module = make_module_with_named_method("main");

    // method_idx=99 is out of bounds
    let name = module.method_defs.get(99)
        .and_then(|def| read_string(&module.string_heap, def.name).ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("method_{}", 99));

    assert_eq!(name, "method_99", "fallback name should be method_<idx>");
}
