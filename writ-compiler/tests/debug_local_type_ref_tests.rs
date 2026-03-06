//! Tests for PREP-05: DebugLocal type_ref back-fill in serialize::translate().
//!
//! The requirement: after translate(), DebugLocal.type_ref carries the blob heap
//! offset for the register's encoded type — it must NOT be 0 for registers with
//! known concrete types (int, float, bool, string).
//!
//! This tests the back-fill logic in translate() at lines ~352-358 of serialize.rs:
//!   for dl in &mut debug_locals {
//!       if (dl.register as usize) < register_types.len() {
//!           dl.type_ref = register_types[dl.register as usize];
//!       }
//!   }

use rustc_hash::FxHashMap;
use std::sync::LazyLock;

use writ_compiler::check::ty::{Ty, TyInterner};
use writ_compiler::resolve::def_map::{DefEntry, DefId, DefKind, DefMap, DefVis};
use writ_compiler::emit::body::EmittedBody;
use writ_compiler::emit::body::labels::LabelAllocator;
use writ_compiler::emit::module_builder::ModuleBuilder;
use writ_compiler::emit::serialize;
use writ_module::instruction::Instruction;
use writ_module::heap::read_blob;
use chumsky::span::{SimpleSpan, Span as _};
use writ_diagnostics::FileId;

#[allow(dead_code)]
static EMPTY_STRUCT_FIELD_TYPES: LazyLock<FxHashMap<DefId, Vec<(String, Ty)>>> =
    LazyLock::new(FxHashMap::default);

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

/// Build a minimal EmittedBody with one named register of the given type,
/// including a debug_local entry for that register.
fn make_body_with_named_local(
    def_id: DefId,
    ty: Ty,
    reg_name: &str,
) -> EmittedBody {
    EmittedBody {
        method_def_id: Some(def_id),
        instructions: vec![
            Instruction::LoadInt { r_dst: 0, value: 0 },
            Instruction::RetVoid,
        ],
        reg_count: 1,
        reg_types: vec![ty],
        source_spans: vec![],
        // debug_locals: (register, name, start_pc, end_pc)
        debug_locals: vec![(0u16, reg_name.to_string(), 0u32, u32::MAX)],
        pending_strings: vec![],
        label_allocator: LabelAllocator::new(),
    }
}

/// After translate(), a DebugLocal for a register with a concrete int type
/// must have type_ref != 0 (i.e., the back-fill wrote the blob heap offset).
#[test]
fn type_ref_backfill_sets_nonzero_offset_for_int_register() {
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, def_id) = make_def_id();

    let mut builder = ModuleBuilder::new();
    builder.set_module_def("test", "0.1.0", 0);
    let _handle = builder.add_methoddef(None, "test_fn", 0, 0, Some(def_id), 0);
    builder.finalize();

    let body = make_body_with_named_local(def_id, ty_int, "x");

    // Use translate() (not serialize()) so we can inspect the resulting Module directly
    let module = serialize::translate(&mut builder, &[body], &interner, true, &[]);

    assert!(!module.method_bodies.is_empty(), "module should have at least one method body");
    let mb = &module.method_bodies[0];

    assert!(!mb.debug_locals.is_empty(), "method body should have debug locals");
    let dl = &mb.debug_locals[0];

    assert_eq!(dl.register, 0, "DebugLocal should be for register 0");
    assert_ne!(
        dl.type_ref, 0,
        "DebugLocal.type_ref must be non-zero after back-fill for an int register; \
         got 0 which means back-fill did not run or blob encoding produced empty offset"
    );
}

/// After translate(), a DebugLocal for a float register has a different type_ref
/// than one for an int register — confirming type identity is preserved.
#[test]
fn type_ref_backfill_produces_distinct_offsets_for_different_types() {
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_float = interner.float();

    // Build two separate modules — one with int, one with float
    let (_, def_id_int) = make_def_id();
    let mut builder_int = ModuleBuilder::new();
    builder_int.set_module_def("test_int", "0.1.0", 0);
    let _h = builder_int.add_methoddef(None, "fn_int", 0, 0, Some(def_id_int), 0);
    builder_int.finalize();
    let body_int = make_body_with_named_local(def_id_int, ty_int, "a");
    let module_int = serialize::translate(&mut builder_int, &[body_int], &interner, true, &[]);

    let (_, def_id_float) = make_def_id();
    let mut builder_float = ModuleBuilder::new();
    builder_float.set_module_def("test_float", "0.1.0", 0);
    let _h2 = builder_float.add_methoddef(None, "fn_float", 0, 0, Some(def_id_float), 0);
    builder_float.finalize();
    let body_float = make_body_with_named_local(def_id_float, ty_float, "b");
    let module_float = serialize::translate(&mut builder_float, &[body_float], &interner, true, &[]);

    let type_ref_int = module_int.method_bodies[0].debug_locals[0].type_ref;
    let type_ref_float = module_float.method_bodies[0].debug_locals[0].type_ref;

    assert_ne!(type_ref_int, 0, "int type_ref should be non-zero");
    assert_ne!(type_ref_float, 0, "float type_ref should be non-zero");

    // The encoded type blobs for int and float are different (0x01 vs 0x02),
    // so they will be at different offsets in the blob heap.
    // Verify the blobs themselves differ in content.
    let blob_int = read_blob(&module_int.blob_heap, type_ref_int).unwrap_or(&[]);
    let blob_float = read_blob(&module_float.blob_heap, type_ref_float).unwrap_or(&[]);

    assert!(!blob_int.is_empty(), "int type blob should not be empty");
    assert!(!blob_float.is_empty(), "float type blob should not be empty");
    assert_ne!(
        blob_int, blob_float,
        "int and float type blobs should differ"
    );
}

/// When no debug info is requested (emit_debug_info=false), debug_locals is empty
/// and there is nothing to back-fill — the body should have no debug_locals.
#[test]
fn type_ref_backfill_skipped_when_debug_info_disabled() {
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, def_id) = make_def_id();

    let mut builder = ModuleBuilder::new();
    builder.set_module_def("test", "0.1.0", 0);
    let _h = builder.add_methoddef(None, "fn", 0, 0, Some(def_id), 0);
    builder.finalize();

    let body = make_body_with_named_local(def_id, ty_int, "x");

    // emit_debug_info = false
    let module = serialize::translate(&mut builder, &[body], &interner, false, &[]);

    assert!(!module.method_bodies.is_empty());
    let mb = &module.method_bodies[0];

    // With debug disabled, debug_locals should be empty
    assert!(
        mb.debug_locals.is_empty(),
        "debug_locals should be empty when emit_debug_info=false"
    );
}

/// A multi-register body: all registers with concrete types get their type_ref back-filled.
#[test]
fn type_ref_backfill_fills_all_registers_with_concrete_types() {
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_bool = interner.bool_ty();
    let (_, def_id) = make_def_id();

    let mut builder = ModuleBuilder::new();
    builder.set_module_def("test", "0.1.0", 0);
    let _h = builder.add_methoddef(None, "fn", 0, 0, Some(def_id), 0);
    builder.finalize();

    let body = EmittedBody {
        method_def_id: Some(def_id),
        instructions: vec![
            Instruction::LoadInt { r_dst: 0, value: 1 },
            Instruction::LoadTrue { r_dst: 1 },
            Instruction::RetVoid,
        ],
        reg_count: 2,
        reg_types: vec![ty_int, ty_bool],
        source_spans: vec![],
        debug_locals: vec![
            (0u16, "x".to_string(), 0u32, u32::MAX),
            (1u16, "flag".to_string(), 0u32, u32::MAX),
        ],
        pending_strings: vec![],
        label_allocator: LabelAllocator::new(),
    };

    let module = serialize::translate(&mut builder, &[body], &interner, true, &[]);
    let mb = &module.method_bodies[0];

    assert_eq!(mb.debug_locals.len(), 2);

    let dl_r0 = mb.debug_locals.iter().find(|d| d.register == 0).expect("r0 should be present");
    let dl_r1 = mb.debug_locals.iter().find(|d| d.register == 1).expect("r1 should be present");

    assert_ne!(dl_r0.type_ref, 0, "r0 (int) should have non-zero type_ref");
    assert_ne!(dl_r1.type_ref, 0, "r1 (bool) should have non-zero type_ref");

    // int and bool blobs should be different
    let blob_r0 = read_blob(&module.blob_heap, dl_r0.type_ref).unwrap_or(&[]);
    let blob_r1 = read_blob(&module.blob_heap, dl_r1.type_ref).unwrap_or(&[]);
    assert_ne!(blob_r0, blob_r1, "int and bool type blobs should differ");
}
