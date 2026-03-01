//! Integration tests for IL debug info and binary serialization.
//!
//! Task 2 tests: DebugLocal emission, SourceSpan emission, serialize::translate(),
//! emit() returning Vec<u8>, pipeline error short-circuit.

use writ_compiler::check::ty::{TyInterner, TyKind};
use writ_compiler::check::ir::{
    TypedAst, TypedDecl, TypedExpr, TypedLiteral,
};
use writ_compiler::resolve::def_map::{DefEntry, DefId, DefKind, DefMap, DefVis};
use writ_compiler::emit::body::{BodyEmitter, EmittedBody};
use writ_compiler::emit::body::debug::{emit_debug_locals, emit_source_spans};
use writ_compiler::emit::module_builder::ModuleBuilder;
use writ_compiler::emit::serialize;
use writ_module::module::{DebugLocal, SourceSpan};
use writ_module::instruction::Instruction;
use chumsky::span::{SimpleSpan, Span as _};
use writ_diagnostics::FileId;

fn dummy_span() -> SimpleSpan {
    SimpleSpan::new((), 0..0)
}

fn make_interner() -> TyInterner {
    TyInterner::new()
}

fn make_def_id() -> (DefMap, DefId) {
    let mut def_map = DefMap::new();
    let id = def_map.arena.alloc(DefEntry {
        id: None,
        kind: DefKind::Fn,
        vis: DefVis::Pub,
        file_id: FileId(0),
        namespace: String::new(),
        name: "test_fn".to_string(),
        name_span: dummy_span(),
        generics: vec![],
        span: dummy_span(),
    });
    (def_map, id)
}

// ─── Debug info emission tests ────────────────────────────────────────────────

#[test]
fn test_debug_locals_all_registers_have_entries() {
    // Every allocated register should have a DebugLocal entry
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_float = interner.float();
    let mut emitter = BodyEmitter::new(&builder, &interner);

    // Allocate some registers
    emitter.regs.alloc(ty_int);
    emitter.regs.alloc(ty_float);
    emitter.regs.alloc(ty_int);

    let locals = emit_debug_locals(&emitter, 100);
    assert_eq!(locals.len(), 3,
        "should emit one DebugLocal per register, got {}", locals.len());

    // All registers should be covered
    let covered: Vec<u16> = locals.iter().map(|l| l.register).collect();
    assert!(covered.contains(&0), "r0 should have DebugLocal entry");
    assert!(covered.contains(&1), "r1 should have DebugLocal entry");
    assert!(covered.contains(&2), "r2 should have DebugLocal entry");
}

#[test]
fn test_debug_locals_span_whole_body() {
    // DebugLocal entries should span start_pc=0 to end_pc=total_code_size
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let mut emitter = BodyEmitter::new(&builder, &interner);
    emitter.regs.alloc(ty_int);

    let total_size = 42u32;
    let locals = emit_debug_locals(&emitter, total_size);
    assert_eq!(locals.len(), 1);
    assert_eq!(locals[0].start_pc, 0);
    assert_eq!(locals[0].end_pc, total_size);
}

#[test]
fn test_debug_locals_named_registers() {
    // Registers in the locals map should have names (reflected in DebugLocal)
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let mut emitter = BodyEmitter::new(&builder, &interner);

    let r = emitter.regs.alloc(ty_int);
    emitter.locals.insert("my_var".to_string(), r);

    let locals = emit_debug_locals(&emitter, 100);
    // The register should be included
    assert_eq!(locals.len(), 1);
    assert_eq!(locals[0].register, r);
    // In Phase 25, name is 0 (placeholder string heap offset)
    // This test just checks the register is present
}

#[test]
fn test_source_spans_from_emitter() {
    // Source spans should be emitted from emitter.source_spans
    let builder = ModuleBuilder::new();
    let interner = make_interner();
    let mut emitter = BodyEmitter::new(&builder, &interner);

    // Push some source span entries
    emitter.source_spans.push((0, SimpleSpan::new((), 10..20)));
    emitter.source_spans.push((5, SimpleSpan::new((), 30..40)));

    let spans = emit_source_spans(&emitter);
    assert_eq!(spans.len(), 2, "should emit one SourceSpan per recorded span");
    assert_eq!(spans[0].pc, 0);
    assert_eq!(spans[0].line, 10); // span.start stored as line (Phase 25)
    assert_eq!(spans[1].pc, 5);
    assert_eq!(spans[1].line, 30);
}

// ─── Serialization tests ──────────────────────────────────────────────────────

#[test]
fn test_serialize_empty_module_produces_bytes() {
    // A module with no bodies should still serialize to valid bytes
    let mut builder = ModuleBuilder::new();
    let interner = make_interner();
    let bodies: Vec<EmittedBody> = Vec::new();

    let result = serialize::serialize(&mut builder, &bodies, &interner);
    assert!(result.is_ok(), "serialize of empty module should succeed, got {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "serialized module should not be empty");
}

#[test]
fn test_serialize_produces_correct_magic_bytes() {
    // First 4 bytes of the header should be the magic number
    // Per spec: magic = "WRIT"
    let mut builder = ModuleBuilder::new();
    let interner = make_interner();
    let bodies: Vec<EmittedBody> = Vec::new();

    let bytes = serialize::serialize(&mut builder, &bodies, &interner).unwrap();
    assert!(bytes.len() >= 4, "module must be at least 4 bytes");
    assert_eq!(&bytes[0..4], b"WRIT",
        "first 4 bytes should be magic WRIT, got {:?}", &bytes[0..4]);
}

#[test]
fn test_serialize_with_module_def() {
    // A module with a ModuleDef row should serialize without panic
    let mut builder = ModuleBuilder::new();
    builder.set_module_def("test_module", "1.0.0", 0);
    let interner = make_interner();
    let bodies: Vec<EmittedBody> = Vec::new();

    let result = serialize::serialize(&mut builder, &bodies, &interner);
    assert!(result.is_ok(), "module with ModuleDef should serialize, got {:?}", result.err());
}

#[test]
fn test_serialize_module_with_body_produces_bytes() {
    // A module with one method body should serialize to non-empty bytes
    let mut builder = ModuleBuilder::new();
    builder.set_module_def("test", "0.1.0", 0);
    let (_, fn_def_id) = make_def_id();
    let handle = builder.add_methoddef(None, "test_fn", 0, 0, Some(fn_def_id), 0);
    builder.finalize();

    let mut interner = make_interner();
    let ty_int = interner.int();

    // Build a simple EmittedBody
    let body = EmittedBody {
        method_def_id: Some(fn_def_id),
        instructions: vec![
            Instruction::LoadInt { r_dst: 0, value: 42 },
            Instruction::Ret { r_src: 0 },
        ],
        reg_count: 1,
        reg_types: vec![ty_int],
        source_spans: vec![],
        debug_locals: vec![],
        pending_strings: vec![],
        label_allocator: writ_compiler::emit::body::labels::LabelAllocator::new(),
    };

    let result = serialize::serialize(&mut builder, &[body], &interner);
    assert!(result.is_ok(), "module with body should serialize, got {:?}", result.err());
    let bytes = result.unwrap();
    assert!(bytes.len() > 200, "module with body should be > 200 bytes (header size)");
}

#[test]
fn test_emit_returns_bytes_for_valid_ast() {
    // emit() should return Ok(Vec<u8>) for a valid, error-free typed AST
    use writ_compiler::emit;

    let mut interner = make_interner();
    let ty_void = interner.void();
    let (def_map, def_id) = make_def_id();

    let typed_ast = TypedAst {
        decls: vec![TypedDecl::Fn {
            def_id,
            body: TypedExpr::Block {
                ty: ty_void,
                span: dummy_span(),
                stmts: vec![],
                tail: None,
            },
        }],
        def_map,
    };

    let result = emit::emit_bodies(&typed_ast, &interner, &[]);
    assert!(result.is_ok(), "emit_bodies should return Ok for valid AST, got {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "emitted bytes should not be empty");
}

#[test]
fn test_emit_returns_err_for_error_nodes() {
    // emit() should return Err when the TypedAst contains Error nodes
    use writ_compiler::emit;

    let mut interner = make_interner();
    let ty_err = interner.error();
    let (def_map, def_id) = make_def_id();

    let typed_ast = TypedAst {
        decls: vec![TypedDecl::Fn {
            def_id,
            body: TypedExpr::Error {
                ty: ty_err,
                span: dummy_span(),
            },
        }],
        def_map,
    };

    let result = emit::emit_bodies(&typed_ast, &interner, &[]);
    assert!(result.is_err(), "emit_bodies should return Err for AST with Error nodes");
}
