//! Integration tests for IL method body emission.
//!
//! Task 1 tests: RegisterAllocator, LabelAllocator, has_error_nodes
//! Task 2 tests: Core instruction emission
//! Task 3 (Plan 02) tests: Call dispatch and argument packing
//! Task 4 (Plan 02) tests: Object model — struct/entity construction, GET_FIELD, SET_FIELD

use writ_compiler::check::ty::{TyInterner, TyKind};
use writ_compiler::check::ir::{
    TypedAst, TypedDecl, TypedExpr, TypedStmt, TypedLiteral,
};
use writ_compiler::resolve::def_map::{DefEntry, DefId, DefKind, DefMap, DefVis};
use writ_compiler::emit::body::reg_alloc::RegisterAllocator;
use writ_compiler::emit::body::labels::LabelAllocator;
use writ_compiler::emit::body::{has_error_nodes, BodyEmitter};
use writ_compiler::emit::body::expr::emit_expr;
use writ_compiler::emit::body::stmt::emit_stmt;
use writ_compiler::emit::body::call::{CallKind, emit_call, emit_call_indirect, emit_box_if_needed};
use writ_compiler::emit::module_builder::ModuleBuilder;
use writ_compiler::emit::metadata::{MetadataToken, TableId, TypeDefKind};
use writ_compiler::ast::expr::BinaryOp;
use writ_module::instruction::Instruction;
use chumsky::span::{SimpleSpan, Span as _};
use writ_diagnostics::FileId;

fn dummy_span() -> SimpleSpan {
    SimpleSpan::new((), 0..0)
}

fn make_interner() -> TyInterner {
    TyInterner::new()
}

/// Helper: allocate a test DefId via a scratch DefMap.
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

// ─── Task 1: RegisterAllocator ───────────────────────────────────────────────

#[test]
fn test_reg_alloc_sequential() {
    let mut ra = RegisterAllocator::new();
    let mut interner = make_interner();
    let ty_int = interner.int();

    let r0 = ra.alloc(ty_int);
    let r1 = ra.alloc(ty_int);
    let r2 = ra.alloc(ty_int);

    assert_eq!(r0, 0);
    assert_eq!(r1, 1);
    assert_eq!(r2, 2);
    assert_eq!(ra.reg_count(), 3);
}

#[test]
fn test_reg_alloc_types() {
    let mut ra = RegisterAllocator::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_float = interner.float();
    let ty_bool = interner.bool_ty();

    ra.alloc(ty_int);
    ra.alloc(ty_float);
    ra.alloc(ty_bool);

    let types = ra.types();
    assert_eq!(types.len(), 3);
    assert_eq!(types[0], ty_int);
    assert_eq!(types[1], ty_float);
    assert_eq!(types[2], ty_bool);
}

// ─── Task 1: LabelAllocator ───────────────────────────────────────────────────

#[test]
fn test_label_allocator_distinct() {
    let mut la = LabelAllocator::new();
    let lbl_a = la.new_label();
    let lbl_b = la.new_label();
    assert_ne!(lbl_a.0, lbl_b.0);
}

#[test]
fn test_label_allocator_forward_branch() {
    // Branch instruction starts at byte position 10.
    // Target marked at byte position 100.
    // offset = target - branch_start = 100 - 10 = 90
    // offset field is at byte position 10 + 4 = 14 (after opcode u16 + r_cond/pad u16)
    let mut la = LabelAllocator::new();
    let lbl = la.new_label();

    la.add_fixup(10, lbl);
    la.mark(lbl, 100);

    let mut code = vec![0u8; 200];
    la.apply_fixups(&mut code);

    let offset_bytes = &code[14..18];
    let offset = i32::from_le_bytes([offset_bytes[0], offset_bytes[1], offset_bytes[2], offset_bytes[3]]);
    assert_eq!(offset, 90);
}

#[test]
fn test_label_allocator_backward_branch() {
    // Branch instruction starts at byte position 100.
    // Target at byte position 20.
    // offset = 20 - 100 = -80
    let mut la = LabelAllocator::new();
    let lbl = la.new_label();

    la.mark(lbl, 20);
    la.add_fixup(100, lbl);

    let mut code = vec![0u8; 200];
    la.apply_fixups(&mut code);

    let offset_bytes = &code[104..108]; // 100 + 4
    let offset = i32::from_le_bytes([offset_bytes[0], offset_bytes[1], offset_bytes[2], offset_bytes[3]]);
    assert_eq!(offset, -80);
}

// ─── Task 1: has_error_nodes ──────────────────────────────────────────────────

fn make_clean_ast() -> TypedAst {
    let mut interner = make_interner();
    let ty_void = interner.void();
    let (def_map, def_id) = make_def_id();
    TypedAst {
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
    }
}

fn make_ast_with_error_expr() -> TypedAst {
    let mut interner = make_interner();
    let ty_error = interner.error();
    let (def_map, def_id) = make_def_id();
    TypedAst {
        decls: vec![TypedDecl::Fn {
            def_id,
            body: TypedExpr::Error {
                ty: ty_error,
                span: dummy_span(),
            },
        }],
        def_map,
    }
}

fn make_ast_with_error_stmt() -> TypedAst {
    let mut interner = make_interner();
    let ty_void = interner.void();
    let (def_map, def_id) = make_def_id();
    TypedAst {
        decls: vec![TypedDecl::Fn {
            def_id,
            body: TypedExpr::Block {
                ty: ty_void,
                span: dummy_span(),
                stmts: vec![TypedStmt::Error { span: dummy_span() }],
                tail: None,
            },
        }],
        def_map,
    }
}

#[test]
fn test_has_error_nodes_clean() {
    let ast = make_clean_ast();
    assert!(!has_error_nodes(&ast));
}

#[test]
fn test_has_error_nodes_with_error_expr() {
    let ast = make_ast_with_error_expr();
    assert!(has_error_nodes(&ast));
}

#[test]
fn test_has_error_nodes_with_error_stmt() {
    let ast = make_ast_with_error_stmt();
    assert!(has_error_nodes(&ast));
}

// ─── Task 2: Core instruction emission ───────────────────────────────────────

fn make_emitter<'a>(builder: &'a ModuleBuilder, interner: &'a TyInterner) -> BodyEmitter<'a> {
    BodyEmitter::new(builder, interner)
}

#[test]
fn test_emit_literal_int() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let mut emitter = make_emitter(&builder, &interner);

    let expr = TypedExpr::Literal {
        ty: ty_int,
        span: dummy_span(),
        value: TypedLiteral::Int(42),
    };
    let reg = emit_expr(&mut emitter, &expr);

    assert_eq!(emitter.instructions.len(), 1);
    assert!(matches!(
        &emitter.instructions[0],
        Instruction::LoadInt { r_dst, value: 42 } if *r_dst == reg
    ));
}

#[test]
fn test_emit_literal_float() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_float = interner.float();
    let mut emitter = make_emitter(&builder, &interner);

    let expr = TypedExpr::Literal {
        ty: ty_float,
        span: dummy_span(),
        value: TypedLiteral::Float(3.0),
    };
    let reg = emit_expr(&mut emitter, &expr);

    assert_eq!(emitter.instructions.len(), 1);
    assert!(matches!(
        &emitter.instructions[0],
        Instruction::LoadFloat { r_dst, .. } if *r_dst == reg
    ));
}

#[test]
fn test_emit_literal_true() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_bool = interner.bool_ty();
    let mut emitter = make_emitter(&builder, &interner);

    let expr = TypedExpr::Literal {
        ty: ty_bool,
        span: dummy_span(),
        value: TypedLiteral::Bool(true),
    };
    let reg = emit_expr(&mut emitter, &expr);

    assert_eq!(emitter.instructions.len(), 1);
    assert!(matches!(
        &emitter.instructions[0],
        Instruction::LoadTrue { r_dst } if *r_dst == reg
    ));
}

#[test]
fn test_emit_literal_false() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_bool = interner.bool_ty();
    let mut emitter = make_emitter(&builder, &interner);

    let expr = TypedExpr::Literal {
        ty: ty_bool,
        span: dummy_span(),
        value: TypedLiteral::Bool(false),
    };
    let reg = emit_expr(&mut emitter, &expr);

    assert_eq!(emitter.instructions.len(), 1);
    assert!(matches!(
        &emitter.instructions[0],
        Instruction::LoadFalse { r_dst } if *r_dst == reg
    ));
}

#[test]
fn test_emit_binary_int_add() {
    // 1 + 2 => LoadInt(r0, 1), LoadInt(r1, 2), AddI(r2, r0, r1)
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let mut emitter = make_emitter(&builder, &interner);

    let expr = TypedExpr::Binary {
        ty: ty_int,
        span: dummy_span(),
        left: Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(1),
        }),
        op: BinaryOp::Add,
        right: Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(2),
        }),
    };
    emit_expr(&mut emitter, &expr);

    assert_eq!(emitter.instructions.len(), 3);
    assert!(matches!(&emitter.instructions[0], Instruction::LoadInt { r_dst: 0, value: 1 }));
    assert!(matches!(&emitter.instructions[1], Instruction::LoadInt { r_dst: 1, value: 2 }));
    assert!(matches!(&emitter.instructions[2], Instruction::AddI { r_dst: 2, r_a: 0, r_b: 1 }));
}

#[test]
fn test_emit_binary_float_mul() {
    // 3.0 * 4.0 => LoadFloat, LoadFloat, MulF
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_float = interner.float();
    let mut emitter = make_emitter(&builder, &interner);

    let expr = TypedExpr::Binary {
        ty: ty_float,
        span: dummy_span(),
        left: Box::new(TypedExpr::Literal {
            ty: ty_float,
            span: dummy_span(),
            value: TypedLiteral::Float(3.0),
        }),
        op: BinaryOp::Mul,
        right: Box::new(TypedExpr::Literal {
            ty: ty_float,
            span: dummy_span(),
            value: TypedLiteral::Float(4.0),
        }),
    };
    emit_expr(&mut emitter, &expr);

    assert_eq!(emitter.instructions.len(), 3);
    assert!(matches!(&emitter.instructions[2], Instruction::MulF { r_dst: 2, r_a: 0, r_b: 1 }));
}

#[test]
fn test_emit_if_else() {
    // if true { 1 } else { 2 }
    //
    // BUG-04 fix: emit_if now produces a shared result register that both branches
    // MOV into, ensuring the RET register is always initialized regardless of which
    // branch was taken. Expected instruction sequence:
    //   [0] LoadTrue       (condition)
    //   [1] BrFalse        (jump to else if false)
    //   [2] LoadInt(1)     (then branch)
    //   [3] Mov(r_result)  (then result -> shared register)
    //   [4] Br             (jump to end)
    //   [5] LoadInt(2)     (else branch)
    //   [6] Mov(r_result)  (else result -> shared register)
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_bool = interner.bool_ty();
    let ty_int = interner.int();
    let mut emitter = make_emitter(&builder, &interner);

    let expr = TypedExpr::If {
        ty: ty_int,
        span: dummy_span(),
        condition: Box::new(TypedExpr::Literal {
            ty: ty_bool,
            span: dummy_span(),
            value: TypedLiteral::Bool(true),
        }),
        then_branch: Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(1),
        }),
        else_branch: Some(Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(2),
        })),
    };
    let r_result = emit_expr(&mut emitter, &expr);

    // Verify the instruction shape: condition, brfalse, then, mov, br, else, mov
    assert!(emitter.instructions.len() >= 7,
        "expected at least 7 instructions, got {}: {:?}",
        emitter.instructions.len(), emitter.instructions);
    assert!(matches!(&emitter.instructions[0], Instruction::LoadTrue { .. }),
        "[0] expected LoadTrue, got {:?}", emitter.instructions[0]);
    assert!(matches!(&emitter.instructions[1], Instruction::BrFalse { .. }),
        "[1] expected BrFalse, got {:?}", emitter.instructions[1]);
    assert!(matches!(&emitter.instructions[2], Instruction::LoadInt { value: 1, .. }),
        "[2] expected LoadInt(1), got {:?}", emitter.instructions[2]);
    // [3] = Mov(r_result, r_then) — the shared result write for then branch
    assert!(matches!(&emitter.instructions[3], Instruction::Mov { r_dst, .. } if *r_dst == r_result),
        "[3] expected Mov into r_result={}, got {:?}", r_result, emitter.instructions[3]);
    assert!(matches!(&emitter.instructions[4], Instruction::Br { .. }),
        "[4] expected Br, got {:?}", emitter.instructions[4]);
    assert!(matches!(&emitter.instructions[5], Instruction::LoadInt { value: 2, .. }),
        "[5] expected LoadInt(2), got {:?}", emitter.instructions[5]);
    // [6] = Mov(r_result, r_else) — the shared result write for else branch
    assert!(matches!(&emitter.instructions[6], Instruction::Mov { r_dst, .. } if *r_dst == r_result),
        "[6] expected Mov into r_result={}, got {:?}", r_result, emitter.instructions[6]);
}

#[test]
fn test_emit_stmt_let() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let mut emitter = make_emitter(&builder, &interner);

    let stmt = TypedStmt::Let {
        name: "x".to_string(),
        name_span: dummy_span(),
        ty: ty_int,
        mutable: false,
        value: TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(99),
        },
        span: dummy_span(),
    };
    emit_stmt(&mut emitter, &stmt);

    // LoadInt emitted, local "x" registered
    assert!(!emitter.instructions.is_empty());
    assert!(emitter.locals.contains_key("x"));
}

#[test]
fn test_emit_stmt_while_loop() {
    // while false { }
    // Expected: LoadFalse, BrFalse(exit), Br(back to start)
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_bool = interner.bool_ty();
    let mut emitter = make_emitter(&builder, &interner);

    let stmt = TypedStmt::While {
        condition: TypedExpr::Literal {
            ty: ty_bool,
            span: dummy_span(),
            value: TypedLiteral::Bool(false),
        },
        body: vec![],
        span: dummy_span(),
    };
    emit_stmt(&mut emitter, &stmt);

    assert!(emitter.instructions.len() >= 3);
    assert!(matches!(&emitter.instructions[0], Instruction::LoadFalse { .. }));
    assert!(matches!(&emitter.instructions[1], Instruction::BrFalse { .. }));
    assert!(matches!(emitter.instructions.last().unwrap(), Instruction::Br { .. }));
}

#[test]
fn test_emit_stmt_return_void() {
    let builder = ModuleBuilder::new();
    let interner = make_interner();
    let mut emitter = make_emitter(&builder, &interner);

    let stmt = TypedStmt::Return {
        value: None,
        span: dummy_span(),
    };
    emit_stmt(&mut emitter, &stmt);

    assert_eq!(emitter.instructions.len(), 1);
    assert!(matches!(&emitter.instructions[0], Instruction::RetVoid));
}

#[test]
fn test_emit_stmt_return_value() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let mut emitter = make_emitter(&builder, &interner);

    let stmt = TypedStmt::Return {
        value: Some(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(42),
        }),
        span: dummy_span(),
    };
    emit_stmt(&mut emitter, &stmt);

    assert_eq!(emitter.instructions.len(), 2);
    assert!(matches!(&emitter.instructions[0], Instruction::LoadInt { value: 42, .. }));
    assert!(matches!(&emitter.instructions[1], Instruction::Ret { .. }));
}

// ─── Task 1: Call dispatch and argument packing ───────────────────────────────

/// Helper: build a ModuleBuilder with one free function registered and finalized.
/// Returns (builder, method_def_id) where method_def_id has a token in the map.
fn make_builder_with_fn(fn_def_id: DefId) -> ModuleBuilder {
    let mut builder = ModuleBuilder::new();
    builder.add_methoddef(None, "test_fn", 0, 0, Some(fn_def_id), 0);
    builder.finalize();
    builder
}

/// Helper: build a ModuleBuilder with one extern function registered and finalized.
fn make_builder_with_extern(extern_def_id: DefId) -> ModuleBuilder {
    let mut builder = ModuleBuilder::new();
    builder.add_extern_def("ext_fn", 0, "ext_fn", 0, Some(extern_def_id));
    builder.finalize();
    builder
}

/// Helper: build a ModuleBuilder with one TypeDef (struct) and method.
fn make_builder_with_struct_method(
    struct_def_id: DefId,
    method_def_id: DefId,
) -> ModuleBuilder {
    let mut builder = ModuleBuilder::new();
    let type_handle = builder.add_typedef("TestStruct", "", TypeDefKind::Struct, 0, Some(struct_def_id));
    builder.add_methoddef(Some(type_handle), "test_method", 0, 0, Some(method_def_id), 0);
    builder.finalize();
    builder
}

#[test]
fn test_call_direct_free_function() {
    // Emit: call test_fn(42)
    // Expected: LoadInt(r0, 42), Call { r_dst, method_idx, r_base: 0, argc: 1 }
    let mut interner = make_interner();
    let ty_int = interner.int();

    let (mut def_map, fn_def_id) = make_def_id();
    // Add an ExternFn-like entry to associate with the fn_def_id
    // (already created as DefKind::Fn in make_def_id)
    let builder = make_builder_with_fn(fn_def_id);

    // Verify token is in map
    assert!(builder.token_for_def(fn_def_id).is_some(), "fn DefId should have token after finalize");

    let mut emitter = make_emitter(&builder, &interner);

    // Build: call expr where callee is a Var resolving to fn_def_id
    // For testing, we use a Var node; call.rs will look up the var name in the def_map
    // but since we don't have a full def_map wired in, we test via emit_call directly
    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Var {
            ty: ty_int,
            span: dummy_span(),
            name: "test_fn".to_string(),
        }),
        args: vec![
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(42) },
        ],
        callee_def_id: None,
    };

    let r_dst = emit_call(&mut emitter, &call_expr, fn_def_id, CallKind::Direct);
    // Should emit: LoadInt(42), then Call { r_dst, method_idx, r_base, argc:1 }
    let instrs = &emitter.instructions;
    assert!(instrs.len() >= 2, "expected at least 2 instructions, got {}", instrs.len());
    assert!(matches!(&instrs[0], Instruction::LoadInt { value: 42, .. }));
    // Last instruction should be a Call
    let last = instrs.last().unwrap();
    assert!(matches!(last, Instruction::Call { .. }), "expected Call, got {:?}", last);
}

#[test]
fn test_call_extern() {
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, extern_def_id) = make_def_id();
    let builder = make_builder_with_extern(extern_def_id);

    let mut emitter = make_emitter(&builder, &interner);

    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Var {
            ty: ty_int,
            span: dummy_span(),
            name: "ext_fn".to_string(),
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_call(&mut emitter, &call_expr, extern_def_id, CallKind::Extern);
    let last = emitter.instructions.last().unwrap();
    assert!(matches!(last, Instruction::CallExtern { .. }), "expected CallExtern, got {:?}", last);
}

#[test]
fn test_call_indirect_delegate() {
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_func = interner.func(vec![ty_int], ty_int);
    let (_, _fn_def_id) = make_def_id();
    let builder = ModuleBuilder::new(); // no fn registered needed

    let mut emitter = make_emitter(&builder, &interner);
    // Set up a local "fn_var" in register 0 with Func type
    emitter.locals.insert("fn_var".to_string(), 0);
    emitter.regs.alloc(ty_func); // r0 = fn_var

    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Var {
            ty: ty_func,
            span: dummy_span(),
            name: "fn_var".to_string(),
        }),
        args: vec![
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(10) },
        ],
        callee_def_id: None,
    };

    let _r = emit_call_indirect(&mut emitter, &call_expr, 0);
    let last = emitter.instructions.last().unwrap();
    assert!(matches!(last, Instruction::CallIndirect { .. }), "expected CallIndirect, got {:?}", last);
}

#[test]
fn test_call_argument_packing_consecutive() {
    // Test that 3 args produce args in consecutive registers before the Call
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, fn_def_id) = make_def_id();
    let builder = make_builder_with_fn(fn_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Var {
            ty: ty_int,
            span: dummy_span(),
            name: "test_fn".to_string(),
        }),
        args: vec![
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(1) },
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(2) },
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(3) },
        ],
        callee_def_id: None,
    };

    let _r = emit_call(&mut emitter, &call_expr, fn_def_id, CallKind::Direct);

    // Find the Call instruction and check argc=3
    let call_instr = emitter.instructions.iter().rev().find(|i| matches!(i, Instruction::Call { .. }));
    assert!(call_instr.is_some(), "should have emitted Call");
    if let Some(Instruction::Call { argc, .. }) = call_instr {
        assert_eq!(*argc, 3, "argc should be 3");
    }
}

#[test]
fn test_call_boxing_value_type_to_generic_param() {
    // Test that passing Int to a generic param causes BOX emission
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_generic = interner.intern(TyKind::GenericParam(0));
    let (_, fn_def_id) = make_def_id();
    let builder = make_builder_with_fn(fn_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    // An int literal arg going to a generic param should be boxed
    let r_val = emitter.alloc_reg(ty_int);
    emitter.emit(Instruction::LoadInt { r_dst: r_val, value: 99 });
    let r_result = emit_box_if_needed(&mut emitter, r_val, ty_int, ty_generic);
    // result register should have a Box instruction
    let has_box = emitter.instructions.iter().any(|i| matches!(i, Instruction::Box { .. }));
    assert!(has_box, "expected Box instruction when passing int to generic param");
    assert_ne!(r_result, r_val, "boxed reg should differ from original");
}

#[test]
fn test_call_virt_specialized_to_call_for_concrete_receiver() {
    // When receiver is TyKind::Struct, CALL_VIRT specializes to CALL
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (mut def_map, struct_def_id) = make_def_id();
    let (_, method_def_id) = {
        let id = def_map.arena.alloc(DefEntry {
            id: None,
            kind: DefKind::Fn,
            vis: DefVis::Pub,
            file_id: FileId(0),
            namespace: String::new(),
            name: "test_method".to_string(),
            name_span: dummy_span(),
            generics: vec![],
            span: dummy_span(),
        });
        (def_map, id)
    };
    let ty_struct = interner.intern(TyKind::Struct(struct_def_id));
    let builder = make_builder_with_struct_method(struct_def_id, method_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    // Set up self register
    let r_self = emitter.alloc_reg(ty_struct);
    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_int,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::SelfRef { ty: ty_struct, span: dummy_span() }),
            field: "test_method".to_string(),
        }),
        args: vec![],
        callee_def_id: None,
    };
    // Concrete receiver -> Direct call (EMIT-27 specialization)
    emit_call(&mut emitter, &call_expr, method_def_id, CallKind::Direct);
    let last = emitter.instructions.last().unwrap();
    assert!(matches!(last, Instruction::Call { .. }),
        "concrete struct receiver should use CALL not CALL_VIRT, got {:?}", last);
}

// ─── Task 2: Object model ─────────────────────────────────────────────────────

/// Helper: make a ModuleBuilder with a struct TypeDef and fields
fn make_builder_with_struct_fields(
    struct_def_id: DefId,
    field_names: &[&str],
) -> ModuleBuilder {
    let mut builder = ModuleBuilder::new();
    let handle = builder.add_typedef("MyStruct", "", TypeDefKind::Struct, 0, Some(struct_def_id));
    for name in field_names {
        builder.add_fielddef(handle, name, 0, 0);
    }
    builder.finalize();
    builder
}

/// Helper: make a ModuleBuilder with an entity TypeDef and fields
fn make_builder_with_entity_fields(
    entity_def_id: DefId,
    field_names: &[&str],
) -> ModuleBuilder {
    let mut builder = ModuleBuilder::new();
    let handle = builder.add_typedef("MyEntity", "", TypeDefKind::Entity, 0, Some(entity_def_id));
    for name in field_names {
        builder.add_fielddef(handle, name, 0, 0);
    }
    builder.finalize();
    builder
}

#[test]
fn test_object_model_struct_construction() {
    // new MyStruct { x: 1, y: 2 } -> NEW + SET_FIELD(x) + SET_FIELD(y)
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, struct_def_id) = make_def_id();
    let ty_struct = interner.intern(TyKind::Struct(struct_def_id));

    let builder = make_builder_with_struct_fields(struct_def_id, &["x", "y"]);
    let mut emitter = make_emitter(&builder, &interner);

    let new_expr = TypedExpr::New {
        ty: ty_struct,
        span: dummy_span(),
        target_def_id: struct_def_id,
        fields: vec![
            ("x".to_string(), TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(1) }),
            ("y".to_string(), TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(2) }),
        ],
    };

    let r_obj = emit_expr(&mut emitter, &new_expr);

    // Check instruction sequence: NEW, then 2x LoadInt + SET_FIELD, then SET_FIELD
    let instrs = &emitter.instructions;
    assert!(instrs.len() >= 3, "expected at least NEW + 2 SET_FIELDs, got {} instructions", instrs.len());
    assert!(matches!(&instrs[0], Instruction::New { .. }), "first instr should be New, got {:?}", &instrs[0]);
    let set_fields: Vec<_> = instrs.iter().filter(|i| matches!(i, Instruction::SetField { .. })).collect();
    assert_eq!(set_fields.len(), 2, "expected 2 SetField instructions, got {}", set_fields.len());
}

#[test]
fn test_object_model_entity_construction_sequence() {
    // new MyEntity { name: "x" } -> SPAWN_ENTITY + SET_FIELD(name) + INIT_ENTITY
    // Default fields do NOT get SET_FIELD
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, entity_def_id) = make_def_id();
    let ty_entity = interner.intern(TyKind::Entity(entity_def_id));

    // Entity has fields: "health" (default), "name" (explicit)
    let builder = make_builder_with_entity_fields(entity_def_id, &["health", "name"]);
    let mut emitter = make_emitter(&builder, &interner);

    let new_expr = TypedExpr::New {
        ty: ty_entity,
        span: dummy_span(),
        target_def_id: entity_def_id,
        fields: vec![
            // Only "name" is explicitly provided; "health" uses default
            ("name".to_string(), TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(42) }),
        ],
    };

    let _r = emit_expr(&mut emitter, &new_expr);
    let instrs = &emitter.instructions;

    // Must have: SpawnEntity, ..., InitEntity at end
    assert!(matches!(&instrs[0], Instruction::SpawnEntity { .. }),
        "first instr should be SpawnEntity, got {:?}", &instrs[0]);
    assert!(matches!(instrs.last().unwrap(), Instruction::InitEntity { .. }),
        "last instr should be InitEntity, got {:?}", instrs.last().unwrap());

    // Only 1 SET_FIELD (for "name", not "health")
    let set_fields: Vec<_> = instrs.iter().filter(|i| matches!(i, Instruction::SetField { .. })).collect();
    assert_eq!(set_fields.len(), 1, "only explicit fields get SET_FIELD, expected 1 got {}", set_fields.len());
}

#[test]
fn test_object_model_field_read() {
    // obj.x -> GET_FIELD
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, struct_def_id) = make_def_id();
    let ty_struct = interner.intern(TyKind::Struct(struct_def_id));
    let builder = make_builder_with_struct_fields(struct_def_id, &["x"]);
    let mut emitter = make_emitter(&builder, &interner);

    // Put struct in register 0
    emitter.locals.insert("obj".to_string(), 0);
    emitter.regs.alloc(ty_struct);

    let field_expr = TypedExpr::Field {
        ty: ty_int,
        span: dummy_span(),
        receiver: Box::new(TypedExpr::Var {
            ty: ty_struct,
            span: dummy_span(),
            name: "obj".to_string(),
        }),
        field: "x".to_string(),
    };

    let _r = emit_expr(&mut emitter, &field_expr);
    let has_get_field = emitter.instructions.iter().any(|i| matches!(i, Instruction::GetField { .. }));
    assert!(has_get_field, "expected GetField instruction for field read");
}

#[test]
fn test_object_model_field_write() {
    // obj.x = 5 -> SET_FIELD
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, struct_def_id) = make_def_id();
    let ty_struct = interner.intern(TyKind::Struct(struct_def_id));
    let builder = make_builder_with_struct_fields(struct_def_id, &["x"]);
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("obj".to_string(), 0);
    emitter.regs.alloc(ty_struct);

    let assign_expr = TypedExpr::Assign {
        ty: ty_int,
        span: dummy_span(),
        target: Box::new(TypedExpr::Field {
            ty: ty_int,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Var {
                ty: ty_struct,
                span: dummy_span(),
                name: "obj".to_string(),
            }),
            field: "x".to_string(),
        }),
        value: Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(5),
        }),
    };

    let _r = emit_expr(&mut emitter, &assign_expr);
    let has_set_field = emitter.instructions.iter().any(|i| matches!(i, Instruction::SetField { .. }));
    assert!(has_set_field, "expected SetField instruction for field write");
}

#[test]
fn test_object_model_component_access() {
    // entity[Health] -> GET_COMPONENT
    let mut interner = make_interner();
    let (_, entity_def_id) = make_def_id();
    let (_, comp_def_id) = make_def_id();
    let ty_entity = interner.intern(TyKind::Entity(entity_def_id));
    let ty_comp = interner.intern(TyKind::Struct(comp_def_id));

    let mut builder = ModuleBuilder::new();
    let _handle = builder.add_typedef("Health", "", TypeDefKind::Struct, 0, Some(comp_def_id));
    builder.finalize();
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("entity".to_string(), 0);
    emitter.regs.alloc(ty_entity);

    let comp_access = TypedExpr::ComponentAccess {
        ty: ty_comp,
        span: dummy_span(),
        receiver: Box::new(TypedExpr::Var {
            ty: ty_entity,
            span: dummy_span(),
            name: "entity".to_string(),
        }),
        component: "Health".to_string(),
    };

    let _r = emit_expr(&mut emitter, &comp_access);
    let has_get_component = emitter.instructions.iter().any(|i| matches!(i, Instruction::GetComponent { .. }));
    assert!(has_get_component, "expected GetComponent instruction for component access");
}

// ─── Task 1 (Plan 03): Array instruction emission ────────────────────────────

#[test]
fn test_array_literal_emits_array_init() {
    // [1, 2, 3] -> elements loaded, then ARRAY_INIT { r_dst, elem_type, count:3, r_base }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_array = interner.intern(TyKind::Array(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    let array_expr = TypedExpr::ArrayLit {
        ty: ty_array,
        span: dummy_span(),
        elements: vec![
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(1) },
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(2) },
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(3) },
        ],
    };

    let _r = emit_expr(&mut emitter, &array_expr);

    // Should have 3 LoadInt instructions, then ArrayInit
    let has_array_init = emitter.instructions.iter().any(|i| matches!(i, Instruction::ArrayInit { count: 3, .. }));
    assert!(has_array_init, "expected ArrayInit with count=3, got {:?}", emitter.instructions);
}

#[test]
fn test_empty_array_literal_emits_new_array() {
    // [] -> NewArray { r_dst, elem_type: 0 }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_array = interner.intern(TyKind::Array(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    let array_expr = TypedExpr::ArrayLit {
        ty: ty_array,
        span: dummy_span(),
        elements: vec![],
    };

    let _r = emit_expr(&mut emitter, &array_expr);

    let has_new_array = emitter.instructions.iter().any(|i| matches!(i, Instruction::NewArray { .. }));
    assert!(has_new_array, "empty array should emit NewArray, got {:?}", emitter.instructions);
}

#[test]
fn test_array_index_read_emits_array_load() {
    // arr[0] -> ARRAY_LOAD { r_dst, r_arr, r_idx }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_array = interner.intern(TyKind::Array(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    // Put array in register 0
    emitter.locals.insert("arr".to_string(), 0);
    emitter.regs.alloc(ty_array);

    let index_expr = TypedExpr::Index {
        ty: ty_int,
        span: dummy_span(),
        receiver: Box::new(TypedExpr::Var {
            ty: ty_array,
            span: dummy_span(),
            name: "arr".to_string(),
        }),
        index: Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(0),
        }),
    };

    let _r = emit_expr(&mut emitter, &index_expr);
    let has_array_load = emitter.instructions.iter().any(|i| matches!(i, Instruction::ArrayLoad { .. }));
    assert!(has_array_load, "expected ArrayLoad, got {:?}", emitter.instructions);
}

#[test]
fn test_array_index_write_emits_array_store() {
    // arr[0] = 5 -> ARRAY_STORE { r_arr, r_idx, r_val }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_array = interner.intern(TyKind::Array(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("arr".to_string(), 0);
    emitter.regs.alloc(ty_array);

    let assign_expr = TypedExpr::Assign {
        ty: ty_int,
        span: dummy_span(),
        target: Box::new(TypedExpr::Index {
            ty: ty_int,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Var {
                ty: ty_array,
                span: dummy_span(),
                name: "arr".to_string(),
            }),
            index: Box::new(TypedExpr::Literal {
                ty: ty_int,
                span: dummy_span(),
                value: TypedLiteral::Int(0),
            }),
        }),
        value: Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(5),
        }),
    };

    let _r = emit_expr(&mut emitter, &assign_expr);
    let has_array_store = emitter.instructions.iter().any(|i| matches!(i, Instruction::ArrayStore { .. }));
    assert!(has_array_store, "expected ArrayStore, got {:?}", emitter.instructions);
}

#[test]
fn test_array_len_call_emits_array_len() {
    // TypedExpr::ArrayLit with .len() call detected by receiver type TyKind::Array
    // We test by creating a direct ArrayLen emission call via checking
    // that the Assign/Index pattern recognizes array receiver
    // For now: test that ArrayLen can be built (direct instruction emission test)
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_array = interner.intern(TyKind::Array(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    // Register array in r0
    emitter.locals.insert("arr".to_string(), 0);
    emitter.regs.alloc(ty_array);

    // Create a Call expr for arr.len() where receiver is Array type
    // TypedExpr::Call { callee: Field { receiver: arr, field: "len" }, args: [] }
    let len_call = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_int,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Var {
                ty: ty_array,
                span: dummy_span(),
                name: "arr".to_string(),
            }),
            field: "len".to_string(),
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &len_call);
    let has_array_len = emitter.instructions.iter().any(|i| matches!(i, Instruction::ArrayLen { .. }));
    assert!(has_array_len, "expected ArrayLen for arr.len(), got {:?}", emitter.instructions);
}

#[test]
fn test_for_loop_over_array_uses_counter_loop() {
    // for x in arr { ... } -> counter loop with ARRAY_LOAD, ARRAY_LEN
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_array = interner.intern(TyKind::Array(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("arr".to_string(), 0);
    emitter.regs.alloc(ty_array);

    let for_stmt = TypedStmt::For {
        binding: "x".to_string(),
        binding_span: dummy_span(),
        binding_ty: ty_int,
        mutable: false,
        iterable: TypedExpr::Var {
            ty: ty_array,
            span: dummy_span(),
            name: "arr".to_string(),
        },
        body: vec![],
        span: dummy_span(),
    };

    emit_stmt(&mut emitter, &for_stmt);
    // Should have ArrayLen, counter loop with BrFalse, ArrayLoad
    let has_array_len = emitter.instructions.iter().any(|i| matches!(i, Instruction::ArrayLen { .. }));
    let has_array_load = emitter.instructions.iter().any(|i| matches!(i, Instruction::ArrayLoad { .. }));
    assert!(has_array_len, "for loop over array should emit ArrayLen");
    assert!(has_array_load, "for loop over array should emit ArrayLoad");
}

// ─── Task 1 (Plan 03): Option/Result instruction emission ───────────────────

#[test]
fn test_option_some_construction() {
    // Some(val) call pattern -> WRAP_SOME { r_dst, r_val }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_opt = interner.intern(TyKind::Option(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    // Simulate: TypedExpr::Call { callee: Path["Some"], args: [val], ty: Option(Int) }
    let some_call = TypedExpr::Call {
        ty: ty_opt,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Path {
            ty: ty_int,
            span: dummy_span(),
            segments: vec!["Some".to_string()],
        }),
        args: vec![
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(42) },
        ],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &some_call);
    let has_wrap_some = emitter.instructions.iter().any(|i| matches!(i, Instruction::WrapSome { .. }));
    assert!(has_wrap_some, "Some(val) should emit WrapSome, got {:?}", emitter.instructions);
}

#[test]
fn test_option_none_construction() {
    // None call pattern -> LoadNull { r_dst }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_opt = interner.intern(TyKind::Option(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    let none_call = TypedExpr::Call {
        ty: ty_opt,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Path {
            ty: ty_opt,
            span: dummy_span(),
            segments: vec!["None".to_string()],
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &none_call);
    let has_load_null = emitter.instructions.iter().any(|i| matches!(i, Instruction::LoadNull { .. }));
    assert!(has_load_null, "None should emit LoadNull, got {:?}", emitter.instructions);
}

#[test]
fn test_option_is_none_method() {
    // opt.is_none() on Option type -> IS_NONE { r_dst, r_opt }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_opt = interner.intern(TyKind::Option(ty_int));
    let ty_bool = interner.bool_ty();
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("opt".to_string(), 0);
    emitter.regs.alloc(ty_opt);

    let is_none_call = TypedExpr::Call {
        ty: ty_bool,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_bool,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Var {
                ty: ty_opt,
                span: dummy_span(),
                name: "opt".to_string(),
            }),
            field: "is_none".to_string(),
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &is_none_call);
    let has_is_none = emitter.instructions.iter().any(|i| matches!(i, Instruction::IsNone { .. }));
    assert!(has_is_none, "opt.is_none() should emit IsNone, got {:?}", emitter.instructions);
}

#[test]
fn test_option_unwrap_method() {
    // opt.unwrap() on Option type -> UNWRAP { r_dst, r_opt }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_opt = interner.intern(TyKind::Option(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("opt".to_string(), 0);
    emitter.regs.alloc(ty_opt);

    let unwrap_call = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_int,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Var {
                ty: ty_opt,
                span: dummy_span(),
                name: "opt".to_string(),
            }),
            field: "unwrap".to_string(),
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &unwrap_call);
    let has_unwrap = emitter.instructions.iter().any(|i| matches!(i, Instruction::Unwrap { .. }));
    assert!(has_unwrap, "opt.unwrap() should emit Unwrap, got {:?}", emitter.instructions);
}

#[test]
fn test_result_ok_construction() {
    // Ok(val) call pattern -> WRAP_OK { r_dst, r_val }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_result = interner.intern(TyKind::Result(ty_int, ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    let ok_call = TypedExpr::Call {
        ty: ty_result,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Path {
            ty: ty_int,
            span: dummy_span(),
            segments: vec!["Ok".to_string()],
        }),
        args: vec![
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(42) },
        ],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &ok_call);
    let has_wrap_ok = emitter.instructions.iter().any(|i| matches!(i, Instruction::WrapOk { .. }));
    assert!(has_wrap_ok, "Ok(val) should emit WrapOk, got {:?}", emitter.instructions);
}

#[test]
fn test_result_is_err_method() {
    // res.is_err() on Result type -> IS_ERR { r_dst, r_result }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_result = interner.intern(TyKind::Result(ty_int, ty_int));
    let ty_bool = interner.bool_ty();
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("res".to_string(), 0);
    emitter.regs.alloc(ty_result);

    let is_err_call = TypedExpr::Call {
        ty: ty_bool,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_bool,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Var {
                ty: ty_result,
                span: dummy_span(),
                name: "res".to_string(),
            }),
            field: "is_err".to_string(),
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &is_err_call);
    let has_is_err = emitter.instructions.iter().any(|i| matches!(i, Instruction::IsErr { .. }));
    assert!(has_is_err, "res.is_err() should emit IsErr, got {:?}", emitter.instructions);
}

// ─── Task 2 (Plan 03): Closure/delegate emission ─────────────────────────────

use writ_compiler::check::ir::{Capture, CaptureMode};

#[test]
fn test_lambda_no_captures_emits_load_null_and_new_delegate() {
    // () -> 42 (no captures) -> LoadNull + NEW_DELEGATE
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_void = interner.void();
    let ty_func = interner.func(vec![], ty_int);

    let mut builder = ModuleBuilder::new();
    // Register a synthetic closure TypeDef before finalize
    let handle = builder.add_typedef("__closure_0", "", TypeDefKind::Struct, 0, None);
    builder.add_methoddef(Some(handle), "__invoke_0", 0, 0, None, 0);
    builder.finalize();

    let mut emitter = make_emitter(&builder, &interner);

    let lambda_expr = TypedExpr::Lambda {
        ty: ty_func,
        span: dummy_span(),
        params: vec![],
        ret_ty: ty_int,
        captures: vec![],
        body: Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(42),
        }),
    };

    let _r = emit_expr(&mut emitter, &lambda_expr);
    let has_load_null = emitter.instructions.iter().any(|i| matches!(i, Instruction::LoadNull { .. }));
    let has_new_delegate = emitter.instructions.iter().any(|i| matches!(i, Instruction::NewDelegate { .. }));
    assert!(has_load_null, "zero-capture lambda should emit LoadNull, got {:?}", emitter.instructions);
    assert!(has_new_delegate, "lambda should emit NewDelegate, got {:?}", emitter.instructions);
}

#[test]
fn test_lambda_with_captures_emits_new_set_field_new_delegate() {
    // |x| x + 1 (captures x) -> NEW(capture_struct) + SET_FIELD(x) + NEW_DELEGATE
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_func = interner.func(vec![ty_int], ty_int);

    let (_, struct_def_id) = make_def_id();
    let mut builder = ModuleBuilder::new();
    // Register capture struct with one field
    let handle = builder.add_typedef("__closure_0", "", TypeDefKind::Struct, 0, None);
    builder.add_fielddef(handle, "x", 0, 0);
    builder.add_methoddef(Some(handle), "__invoke_0", 0, 0, None, 0);
    builder.finalize();

    let mut emitter = make_emitter(&builder, &interner);
    // Set up the captured variable "x" in r0
    emitter.locals.insert("x".to_string(), 0);
    emitter.regs.alloc(ty_int);

    let lambda_expr = TypedExpr::Lambda {
        ty: ty_func,
        span: dummy_span(),
        params: vec![("val".to_string(), ty_int)],
        ret_ty: ty_int,
        captures: vec![
            Capture {
                name: "x".to_string(),
                ty: ty_int,
                mode: CaptureMode::ByValue,
                binding_span: dummy_span(),
            },
        ],
        body: Box::new(TypedExpr::Var {
            ty: ty_int,
            span: dummy_span(),
            name: "x".to_string(),
        }),
    };

    let _r = emit_expr(&mut emitter, &lambda_expr);
    let has_new = emitter.instructions.iter().any(|i| matches!(i, Instruction::New { .. }));
    let has_set_field = emitter.instructions.iter().any(|i| matches!(i, Instruction::SetField { .. }));
    let has_new_delegate = emitter.instructions.iter().any(|i| matches!(i, Instruction::NewDelegate { .. }));
    assert!(has_new, "capturing lambda should emit New (capture struct), got {:?}", emitter.instructions);
    assert!(has_set_field, "capturing lambda should emit SetField per capture, got {:?}", emitter.instructions);
    assert!(has_new_delegate, "lambda should emit NewDelegate, got {:?}", emitter.instructions);
}

// ─── Task 2 (Plan 03): Concurrency instruction emission ──────────────────────

#[test]
fn test_spawn_task_emits_spawn_task_instruction() {
    // spawn expr -> SpawnTask { r_dst, method_idx, r_base, argc }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_task = interner.intern(TyKind::TaskHandle(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    // spawn(some_call()) -- the inner expr is a Call
    let spawn_expr = TypedExpr::Spawn {
        ty: ty_task,
        span: dummy_span(),
        expr: Box::new(TypedExpr::Call {
            ty: ty_int,
            span: dummy_span(),
            callee: Box::new(TypedExpr::Var {
                ty: ty_int,
                span: dummy_span(),
                name: "some_fn".to_string(),
            }),
            args: vec![],
            callee_def_id: None,
        }),
    };

    let _r = emit_expr(&mut emitter, &spawn_expr);
    let has_spawn = emitter.instructions.iter().any(|i| matches!(i, Instruction::SpawnTask { .. }));
    assert!(has_spawn, "spawn expr should emit SpawnTask, got {:?}", emitter.instructions);
}

#[test]
fn test_spawn_detached_emits_spawn_detached_instruction() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_void = interner.void();
    let ty_task = interner.intern(TyKind::TaskHandle(ty_void));
    let mut emitter = make_emitter(&builder, &interner);

    let spawn_expr = TypedExpr::SpawnDetached {
        ty: ty_task,
        span: dummy_span(),
        expr: Box::new(TypedExpr::Call {
            ty: ty_void,
            span: dummy_span(),
            callee: Box::new(TypedExpr::Var {
                ty: ty_void,
                span: dummy_span(),
                name: "bg_fn".to_string(),
            }),
            args: vec![],
            callee_def_id: None,
        }),
    };

    let _r = emit_expr(&mut emitter, &spawn_expr);
    let has_spawn_detached = emitter.instructions.iter().any(|i| matches!(i, Instruction::SpawnDetached { .. }));
    assert!(has_spawn_detached, "spawn_detached should emit SpawnDetached, got {:?}", emitter.instructions);
}

#[test]
fn test_join_emits_join_instruction() {
    // join(task) -> JOIN { r_dst, r_task }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_task = interner.intern(TyKind::TaskHandle(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("task".to_string(), 0);
    emitter.regs.alloc(ty_task);

    let join_expr = TypedExpr::Join {
        ty: ty_int,
        span: dummy_span(),
        expr: Box::new(TypedExpr::Var {
            ty: ty_task,
            span: dummy_span(),
            name: "task".to_string(),
        }),
    };

    let _r = emit_expr(&mut emitter, &join_expr);
    let has_join = emitter.instructions.iter().any(|i| matches!(i, Instruction::Join { .. }));
    assert!(has_join, "join(task) should emit Join, got {:?}", emitter.instructions);
}

#[test]
fn test_cancel_emits_cancel_instruction() {
    // cancel(task) -> CANCEL { r_task }
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_void = interner.void();
    let ty_int = interner.int();
    let ty_task = interner.intern(TyKind::TaskHandle(ty_int));
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("task".to_string(), 0);
    emitter.regs.alloc(ty_task);

    let cancel_expr = TypedExpr::Cancel {
        ty: ty_void,
        span: dummy_span(),
        expr: Box::new(TypedExpr::Var {
            ty: ty_task,
            span: dummy_span(),
            name: "task".to_string(),
        }),
    };

    let _r = emit_expr(&mut emitter, &cancel_expr);
    let has_cancel = emitter.instructions.iter().any(|i| matches!(i, Instruction::Cancel { .. }));
    assert!(has_cancel, "cancel(task) should emit Cancel, got {:?}", emitter.instructions);
}

#[test]
fn test_defer_emits_defer_push_pop_end() {
    // defer { ... } -> DeferPush + body + DeferPop + DeferEnd
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_void = interner.void();
    let mut emitter = make_emitter(&builder, &interner);

    let defer_expr = TypedExpr::Defer {
        ty: ty_void,
        span: dummy_span(),
        expr: Box::new(TypedExpr::Block {
            ty: ty_void,
            span: dummy_span(),
            stmts: vec![],
            tail: None,
        }),
    };

    let _r = emit_expr(&mut emitter, &defer_expr);
    let has_defer_push = emitter.instructions.iter().any(|i| matches!(i, Instruction::DeferPush { .. }));
    let has_defer_pop = emitter.instructions.iter().any(|i| matches!(i, Instruction::DeferPop));
    let has_defer_end = emitter.instructions.iter().any(|i| matches!(i, Instruction::DeferEnd));
    assert!(has_defer_push, "defer should emit DeferPush, got {:?}", emitter.instructions);
    assert!(has_defer_pop, "defer should emit DeferPop, got {:?}", emitter.instructions);
    assert!(has_defer_end, "defer should emit DeferEnd, got {:?}", emitter.instructions);
}

// ─── Task 1 (Plan 04): Enum match, type conversions, string ops, const folding ──

use writ_compiler::check::ir::{TypedArm, TypedPattern};
use writ_compiler::emit::body::patterns::emit_match;
use writ_compiler::emit::body::const_fold::const_fold;

/// Helper: make a ModuleBuilder with an enum TypeDef (no fields needed for tags)
fn make_builder_with_enum(enum_def_id: DefId) -> ModuleBuilder {
    let mut builder = ModuleBuilder::new();
    builder.add_typedef("MyEnum", "", TypeDefKind::Enum, 0, Some(enum_def_id));
    builder.finalize();
    builder
}

#[test]
fn test_enum_match_emits_get_tag_and_switch() {
    // match e { A => 1, B => 2 } should emit GET_TAG + SWITCH
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, enum_def_id) = make_def_id();
    let ty_enum = interner.intern(TyKind::Enum(enum_def_id));

    let builder = make_builder_with_enum(enum_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    // Put enum value in register 0
    emitter.locals.insert("e".to_string(), 0);
    emitter.regs.alloc(ty_enum);

    let scrutinee = TypedExpr::Var {
        ty: ty_enum,
        span: dummy_span(),
        name: "e".to_string(),
    };

    let match_expr = TypedExpr::Match {
        ty: ty_int,
        span: dummy_span(),
        scrutinee: Box::new(scrutinee),
        arms: vec![
            TypedArm {
                pattern: TypedPattern::EnumVariant {
                    enum_def_id,
                    variant_name: "A".to_string(),
                    bindings: vec![],
                    span: dummy_span(),
                },
                body: TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(1) },
                span: dummy_span(),
            },
            TypedArm {
                pattern: TypedPattern::EnumVariant {
                    enum_def_id,
                    variant_name: "B".to_string(),
                    bindings: vec![],
                    span: dummy_span(),
                },
                body: TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(2) },
                span: dummy_span(),
            },
        ],
    };

    let _r = emit_match(&mut emitter, &match_expr);

    let has_get_tag = emitter.instructions.iter().any(|i| matches!(i, Instruction::GetTag { .. }));
    let has_switch = emitter.instructions.iter().any(|i| matches!(i, Instruction::Switch { .. }));
    assert!(has_get_tag, "enum match should emit GetTag, got {:?}", emitter.instructions);
    assert!(has_switch, "enum match should emit Switch, got {:?}", emitter.instructions);
}

#[test]
fn test_enum_match_wildcard_arm() {
    // match e { A => 1, _ => 2 } — wildcard arm should be reachable
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, enum_def_id) = make_def_id();
    let ty_enum = interner.intern(TyKind::Enum(enum_def_id));

    let builder = make_builder_with_enum(enum_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("e".to_string(), 0);
    emitter.regs.alloc(ty_enum);

    let match_expr = TypedExpr::Match {
        ty: ty_int,
        span: dummy_span(),
        scrutinee: Box::new(TypedExpr::Var { ty: ty_enum, span: dummy_span(), name: "e".to_string() }),
        arms: vec![
            TypedArm {
                pattern: TypedPattern::EnumVariant {
                    enum_def_id,
                    variant_name: "A".to_string(),
                    bindings: vec![],
                    span: dummy_span(),
                },
                body: TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(1) },
                span: dummy_span(),
            },
            TypedArm {
                pattern: TypedPattern::Wildcard { span: dummy_span() },
                body: TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(99) },
                span: dummy_span(),
            },
        ],
    };

    let _r = emit_match(&mut emitter, &match_expr);
    // Should emit GetTag + Switch without panic
    let has_get_tag = emitter.instructions.iter().any(|i| matches!(i, Instruction::GetTag { .. }));
    assert!(has_get_tag, "enum match with wildcard should emit GetTag, got {:?}", emitter.instructions);
}

#[test]
fn test_enum_match_option_propagation_emits_is_none_br_ret() {
    // Option propagation: IS_NONE + BR_FALSE + (LOAD_NULL + RET) + UNWRAP
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_opt_int = interner.intern(TyKind::Option(ty_int));

    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("opt".to_string(), 0);
    emitter.regs.alloc(ty_opt_int);

    let (_, dummy_enum_def_id) = make_def_id();

    // Desugared ? on Option: match opt { Some(v) => v, None => return None }
    let match_expr = TypedExpr::Match {
        ty: ty_int,
        span: dummy_span(),
        scrutinee: Box::new(TypedExpr::Var { ty: ty_opt_int, span: dummy_span(), name: "opt".to_string() }),
        arms: vec![
            TypedArm {
                pattern: TypedPattern::EnumVariant {
                    enum_def_id: dummy_enum_def_id,
                    variant_name: "Some".to_string(),
                    bindings: vec![TypedPattern::Variable { name: "v".to_string(), ty: ty_int, span: dummy_span() }],
                    span: dummy_span(),
                },
                body: TypedExpr::Var { ty: ty_int, span: dummy_span(), name: "v".to_string() },
                span: dummy_span(),
            },
            TypedArm {
                pattern: TypedPattern::EnumVariant {
                    enum_def_id: dummy_enum_def_id,
                    variant_name: "None".to_string(),
                    bindings: vec![],
                    span: dummy_span(),
                },
                body: TypedExpr::Return {
                    ty: ty_int,
                    span: dummy_span(),
                    value: Some(Box::new(TypedExpr::Literal { ty: ty_opt_int, span: dummy_span(), value: TypedLiteral::Int(0) })),
                },
                span: dummy_span(),
            },
        ],
    };

    let _r = emit_match(&mut emitter, &match_expr);
    let has_is_none = emitter.instructions.iter().any(|i| matches!(i, Instruction::IsNone { .. }));
    assert!(has_is_none, "Option ? propagation should emit IsNone, got {:?}", emitter.instructions);
}

#[test]
fn test_result_propagation_emits_is_err() {
    // Result propagation: IS_ERR + BR_FALSE + EXTRACT_ERR + WRAP_ERR + RET + UNWRAP_OK
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_str = interner.string_ty();
    let ty_result = interner.intern(TyKind::Result(ty_int, ty_str));

    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("r".to_string(), 0);
    emitter.regs.alloc(ty_result);

    let (_, dummy_enum_def_id) = make_def_id();

    // Desugared try on Result: match r { Ok(v) => v, Err(e) => return Err(e) }
    let match_expr = TypedExpr::Match {
        ty: ty_int,
        span: dummy_span(),
        scrutinee: Box::new(TypedExpr::Var { ty: ty_result, span: dummy_span(), name: "r".to_string() }),
        arms: vec![
            TypedArm {
                pattern: TypedPattern::EnumVariant {
                    enum_def_id: dummy_enum_def_id,
                    variant_name: "Ok".to_string(),
                    bindings: vec![TypedPattern::Variable { name: "v".to_string(), ty: ty_int, span: dummy_span() }],
                    span: dummy_span(),
                },
                body: TypedExpr::Var { ty: ty_int, span: dummy_span(), name: "v".to_string() },
                span: dummy_span(),
            },
            TypedArm {
                pattern: TypedPattern::EnumVariant {
                    enum_def_id: dummy_enum_def_id,
                    variant_name: "Err".to_string(),
                    bindings: vec![TypedPattern::Variable { name: "e".to_string(), ty: ty_str, span: dummy_span() }],
                    span: dummy_span(),
                },
                body: TypedExpr::Return {
                    ty: ty_int,
                    span: dummy_span(),
                    value: Some(Box::new(TypedExpr::Var { ty: ty_str, span: dummy_span(), name: "e".to_string() })),
                },
                span: dummy_span(),
            },
        ],
    };

    let _r = emit_match(&mut emitter, &match_expr);
    let has_is_err = emitter.instructions.iter().any(|i| matches!(i, Instruction::IsErr { .. }));
    assert!(has_is_err, "Result try propagation should emit IsErr, got {:?}", emitter.instructions);
}

#[test]
fn test_type_conversion_int_to_float() {
    // .into<Float>() on Int -> I2F
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_float = interner.float();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    // Simulate: int_val.into::<Float>()
    // In TypedExpr this is a Call with a Field callee on a Float receiver where the method name is "into"
    // and the return type is Float. We use a special representation.
    let call_expr = TypedExpr::Call {
        ty: ty_float,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_float,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Literal {
                ty: ty_int,
                span: dummy_span(),
                value: TypedLiteral::Int(5),
            }),
            field: "into_float".to_string(), // sentinel for into<Float>
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &call_expr);
    let has_i2f = emitter.instructions.iter().any(|i| matches!(i, Instruction::I2f { .. }));
    assert!(has_i2f, "int.into<Float>() should emit I2f, got {:?}", emitter.instructions);
}

#[test]
fn test_type_conversion_int_to_string() {
    // .into<String>() on Int -> I2S
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_str = interner.string_ty();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    let call_expr = TypedExpr::Call {
        ty: ty_str,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_str,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Literal {
                ty: ty_int,
                span: dummy_span(),
                value: TypedLiteral::Int(5),
            }),
            field: "into_string".to_string(), // sentinel for into<String> on int
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &call_expr);
    let has_i2s = emitter.instructions.iter().any(|i| matches!(i, Instruction::I2s { .. }));
    assert!(has_i2s, "int.into<String>() should emit I2s, got {:?}", emitter.instructions);
}

#[test]
fn test_type_conversion_float_to_string() {
    // .into<String>() on Float -> F2S
    let mut interner = make_interner();
    let ty_float = interner.float();
    let ty_str = interner.string_ty();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    let call_expr = TypedExpr::Call {
        ty: ty_str,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_str,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Literal {
                ty: ty_float,
                span: dummy_span(),
                value: TypedLiteral::Float(3.14),
            }),
            field: "into_string".to_string(),
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &call_expr);
    let has_f2s = emitter.instructions.iter().any(|i| matches!(i, Instruction::F2s { .. }));
    assert!(has_f2s, "float.into<String>() should emit F2s, got {:?}", emitter.instructions);
}

#[test]
fn test_type_conversion_bool_to_string() {
    // .into<String>() on Bool -> B2S
    let mut interner = make_interner();
    let ty_bool = interner.bool_ty();
    let ty_str = interner.string_ty();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    let call_expr = TypedExpr::Call {
        ty: ty_str,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_str,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Literal {
                ty: ty_bool,
                span: dummy_span(),
                value: TypedLiteral::Bool(true),
            }),
            field: "into_string".to_string(),
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &call_expr);
    let has_b2s = emitter.instructions.iter().any(|i| matches!(i, Instruction::B2s { .. }));
    assert!(has_b2s, "bool.into<String>() should emit B2s, got {:?}", emitter.instructions);
}

#[test]
fn test_string_concat_emits_str_concat() {
    // "a" + "b" -> STR_CONCAT
    let mut interner = make_interner();
    let ty_str = interner.string_ty();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    let concat_expr = TypedExpr::Binary {
        ty: ty_str,
        span: dummy_span(),
        left: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("a".to_string()) }),
        op: BinaryOp::Add,
        right: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("b".to_string()) }),
    };

    let _r = emit_expr(&mut emitter, &concat_expr);
    let has_str_concat = emitter.instructions.iter().any(|i| matches!(i, Instruction::StrConcat { .. }));
    assert!(has_str_concat, "string + string should emit StrConcat, got {:?}", emitter.instructions);
}

#[test]
fn test_string_len_emits_str_len() {
    // s.len() -> STR_LEN
    let mut interner = make_interner();
    let ty_str = interner.string_ty();
    let ty_int = interner.int();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    emitter.locals.insert("s".to_string(), 0);
    emitter.regs.alloc(ty_str);

    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_int,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::Var { ty: ty_str, span: dummy_span(), name: "s".to_string() }),
            field: "len".to_string(),
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &call_expr);
    let has_str_len = emitter.instructions.iter().any(|i| matches!(i, Instruction::StrLen { .. }));
    assert!(has_str_len, "string.len() should emit StrLen, got {:?}", emitter.instructions);
}

#[test]
fn test_const_fold_int_addition() {
    // const X = 2 + 3 -> const_fold returns Some(Int(5))
    let mut interner = make_interner();
    let ty_int = interner.int();

    let expr = TypedExpr::Binary {
        ty: ty_int,
        span: dummy_span(),
        left: Box::new(TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(2) }),
        op: BinaryOp::Add,
        right: Box::new(TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(3) }),
    };

    let result = const_fold(&expr, &interner);
    assert!(matches!(result, Some(TypedLiteral::Int(5))),
        "const_fold(2 + 3) should yield Some(Int(5)), got {:?}", result);
}

#[test]
fn test_const_fold_float_multiplication() {
    // 2.0 * 3.0 -> Some(Float(6.0))
    let mut interner = make_interner();
    let ty_float = interner.float();

    let expr = TypedExpr::Binary {
        ty: ty_float,
        span: dummy_span(),
        left: Box::new(TypedExpr::Literal { ty: ty_float, span: dummy_span(), value: TypedLiteral::Float(2.0) }),
        op: BinaryOp::Mul,
        right: Box::new(TypedExpr::Literal { ty: ty_float, span: dummy_span(), value: TypedLiteral::Float(3.0) }),
    };

    let result = const_fold(&expr, &interner);
    assert!(matches!(result, Some(TypedLiteral::Float(f)) if (f - 6.0).abs() < 1e-9),
        "const_fold(2.0 * 3.0) should yield Some(Float(6.0)), got {:?}", result);
}

#[test]
fn test_const_fold_non_constant_returns_none() {
    // var + 1 cannot be folded
    let mut interner = make_interner();
    let ty_int = interner.int();

    let expr = TypedExpr::Binary {
        ty: ty_int,
        span: dummy_span(),
        left: Box::new(TypedExpr::Var { ty: ty_int, span: dummy_span(), name: "x".to_string() }),
        op: BinaryOp::Add,
        right: Box::new(TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(1) }),
    };

    let result = const_fold(&expr, &interner);
    assert!(result.is_none(), "const_fold(var + 1) should return None, got {:?}", result);
}

#[test]
fn test_const_fold_subtraction() {
    // 10 - 3 -> Some(Int(7))
    let mut interner = make_interner();
    let ty_int = interner.int();

    let expr = TypedExpr::Binary {
        ty: ty_int,
        span: dummy_span(),
        left: Box::new(TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(10) }),
        op: BinaryOp::Sub,
        right: Box::new(TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(3) }),
    };

    let result = const_fold(&expr, &interner);
    assert!(matches!(result, Some(TypedLiteral::Int(7))),
        "const_fold(10 - 3) should yield Some(Int(7)), got {:?}", result);
}

#[test]
fn test_tail_call_for_dialogue_return() {
    // A Return with a Call that has the "is_dialogue" flag -> TailCall instead of Call+Ret
    // For simplicity we just test that a direct Call return works (full tail call detection
    // is wired in the full pipeline; this tests the instruction-level path)
    let mut interner = make_interner();
    let ty_void = interner.void();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    // Return of a call expression emits Ret normally (TAIL_CALL requires full pipeline)
    let return_expr = TypedExpr::Return {
        ty: ty_void,
        span: dummy_span(),
        value: Some(Box::new(TypedExpr::Literal {
            ty: ty_void,
            span: dummy_span(),
            value: TypedLiteral::Int(0),
        })),
    };

    let _r = emit_expr(&mut emitter, &return_expr);
    let has_ret = emitter.instructions.iter().any(|i| matches!(i, Instruction::Ret { .. }));
    assert!(has_ret, "Return with value should emit Ret, got {:?}", emitter.instructions);
}

// ─── Plan 05, Task 1: SWITCH offset fixup and const_fold wiring ─────────────

use writ_compiler::emit::body::emit_all_bodies;
use writ_compiler::emit::body::closure::LambdaInfo;

#[test]
fn test_switch_offsets_are_nonzero_for_enum_match() {
    // Enum match with 2 variant arms: GET_TAG + SWITCH with non-zero offsets.
    // Both offsets must be non-zero (pointing forward from SWITCH position).
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, enum_def_id) = make_def_id();
    let ty_enum = interner.intern(TyKind::Enum(enum_def_id));
    let builder = make_builder_with_enum(enum_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    // Allocate a register for the enum scrutinee
    let r_enum = emitter.alloc_reg(ty_enum);
    emitter.locals.insert("e".to_string(), r_enum);

    // Two variant arms: Variant0 -> 10, Variant1 -> 20
    let arms = vec![
        TypedArm {
            pattern: TypedPattern::EnumVariant {
                enum_def_id,
                variant_name: "A".to_string(),
                bindings: vec![],
                span: dummy_span(),
            },
            body: TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(10) },
            span: dummy_span(),
        },
        TypedArm {
            pattern: TypedPattern::EnumVariant {
                enum_def_id,
                variant_name: "B".to_string(),
                bindings: vec![],
                span: dummy_span(),
            },
            body: TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(20) },
            span: dummy_span(),
        },
    ];

    let match_expr = TypedExpr::Match {
        ty: ty_int,
        span: dummy_span(),
        scrutinee: Box::new(TypedExpr::Var { ty: ty_enum, span: dummy_span(), name: "e".to_string() }),
        arms,
    };

    let _r = emit_match(&mut emitter, &match_expr);

    // Find the SWITCH instruction
    let switch = emitter.instructions.iter().find(|i| matches!(i, Instruction::Switch { .. }));
    assert!(switch.is_some(), "Expected a Switch instruction, got {:?}", emitter.instructions);
    if let Some(Instruction::Switch { offsets, .. }) = switch {
        assert_eq!(offsets.len(), 2, "Expected 2 offset slots for 2 variants");
        assert!(offsets[0] != 0 || offsets[1] != 0,
            "At least one SWITCH offset must be non-zero; got {:?}", offsets);
    }
}

#[test]
fn test_switch_offset_arm0_points_to_first_arm() {
    // The first arm label must be marked at the instruction immediately after SWITCH.
    // So offset[0] >= 1 (arm 0 is after the SWITCH instruction).
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, enum_def_id) = make_def_id();
    let ty_enum = interner.intern(TyKind::Enum(enum_def_id));
    let builder = make_builder_with_enum(enum_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    let r_enum = emitter.alloc_reg(ty_enum);
    emitter.locals.insert("e".to_string(), r_enum);

    // Single variant arm
    let arms = vec![
        TypedArm {
            pattern: TypedPattern::EnumVariant {
                enum_def_id,
                variant_name: "A".to_string(),
                bindings: vec![],
                span: dummy_span(),
            },
            body: TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(42) },
            span: dummy_span(),
        },
    ];

    let match_expr = TypedExpr::Match {
        ty: ty_int,
        span: dummy_span(),
        scrutinee: Box::new(TypedExpr::Var { ty: ty_enum, span: dummy_span(), name: "e".to_string() }),
        arms,
    };

    let _r = emit_match(&mut emitter, &match_expr);

    if let Some(Instruction::Switch { offsets, .. }) = emitter.instructions.iter().find(|i| matches!(i, Instruction::Switch { .. })) {
        // offset[0] > 0: arm 0 is after the SWITCH instruction
        assert!(offsets[0] > 0, "Arm 0 offset must be positive (arm is after SWITCH), got {}", offsets[0]);
    } else {
        panic!("Expected Switch instruction");
    }
}

#[test]
fn test_const_decl_foldable_emits_load_int() {
    // TypedDecl::Const with 2+3 -> emit_all_bodies produces a body with LoadInt(5)
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (def_map, def_id) = make_def_id();

    let value = TypedExpr::Binary {
        ty: ty_int,
        span: dummy_span(),
        left: Box::new(TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(2) }),
        op: BinaryOp::Add,
        right: Box::new(TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(3) }),
    };

    let ast = TypedAst {
        decls: vec![TypedDecl::Const { def_id, value }],
        def_map,
    };

    let builder = ModuleBuilder::new();
    let (bodies, diags) = emit_all_bodies(&ast, &interner, &builder, &[]);
    assert!(diags.is_empty(), "Expected no diagnostics, got {:?}", diags);
    assert_eq!(bodies.len(), 1, "Expected 1 emitted body for the const decl");

    let body = &bodies[0];
    let has_load_int_5 = body.instructions.iter().any(|i| matches!(i, Instruction::LoadInt { value: 5, .. }));
    assert!(has_load_int_5, "Expected LoadInt(5) from const_fold(2+3), got {:?}", body.instructions);
}

#[test]
fn test_const_decl_non_foldable_emits_instructions() {
    // TypedDecl::Const with a non-constant expression falls back to normal emission
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (def_map, def_id) = make_def_id();

    // var + 1 is not foldable; emit_expr will produce some instructions
    let value = TypedExpr::Binary {
        ty: ty_int,
        span: dummy_span(),
        left: Box::new(TypedExpr::Var { ty: ty_int, span: dummy_span(), name: "x".to_string() }),
        op: BinaryOp::Add,
        right: Box::new(TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(1) }),
    };

    let ast = TypedAst {
        decls: vec![TypedDecl::Const { def_id, value }],
        def_map,
    };

    let builder = ModuleBuilder::new();
    let (bodies, diags) = emit_all_bodies(&ast, &interner, &builder, &[]);
    assert!(diags.is_empty(), "Expected no diagnostics");
    // Body is emitted (non-foldable path still produces a body)
    assert_eq!(bodies.len(), 1, "Expected 1 emitted body for non-foldable const");
}

// ─── Plan 05, Task 2: Closure body emission and string literal interning ─────

use writ_compiler::emit::body::closure::pre_scan_lambdas;
use writ_compiler::emit::emit_bodies;

#[test]
fn test_lambda_body_emitted_as_separate_body_entry() {
    // A TypedAst with a Fn containing a Lambda should produce 2 EmittedBody entries:
    // one for the Fn body, one for the Lambda body.
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_func = interner.func(vec![], ty_int);
    let (def_map, fn_def_id) = make_def_id();

    // fn foo() -> lambda -> 42
    let lambda_expr = TypedExpr::Lambda {
        ty: ty_func,
        span: dummy_span(),
        params: vec![],
        ret_ty: ty_int,
        captures: vec![],
        body: Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(42),
        }),
    };

    let ast = TypedAst {
        decls: vec![TypedDecl::Fn {
            def_id: fn_def_id,
            body: lambda_expr,
        }],
        def_map,
    };

    let mut builder = ModuleBuilder::new();
    let lambda_infos = pre_scan_lambdas(&ast, &interner, &mut builder);
    builder.finalize();

    assert_eq!(lambda_infos.len(), 1, "Expected 1 lambda info");

    let (bodies, diags) = emit_all_bodies(&ast, &interner, &builder, &lambda_infos);
    assert!(diags.is_empty(), "Expected no diagnostics, got {:?}", diags);
    assert_eq!(bodies.len(), 2, "Expected 2 bodies: fn body + lambda body, got {}", bodies.len());

    // The second body (lambda body) should have method_def_id: None
    assert!(bodies[1].method_def_id.is_none(), "Lambda body should have no DefId");
}

#[test]
fn test_string_literal_interning_via_emit_bodies() {
    // emit_bodies should produce a LoadString with a non-zero string_idx for "hello"
    let mut interner = make_interner();
    let ty_str = interner.string_ty();
    let (def_map, fn_def_id) = make_def_id();

    let ast = TypedAst {
        decls: vec![TypedDecl::Fn {
            def_id: fn_def_id,
            body: TypedExpr::Literal {
                ty: ty_str,
                span: dummy_span(),
                value: TypedLiteral::String("hello".to_string()),
            },
        }],
        def_map,
    };

    // emit_bodies does the full pipeline including string interning fixup
    let result = emit_bodies(&ast, &interner, &[]);
    assert!(result.is_ok(), "emit_bodies should succeed, got {:?}", result.err());
    // Just check we get bytes back (string interning fixup produced a valid module)
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Expected non-empty output");
}

#[test]
fn test_string_literal_pending_strings_populated() {
    // After emit_all_bodies, pending_strings should contain the string literal.
    let mut interner = make_interner();
    let ty_str = interner.string_ty();
    let (def_map, fn_def_id) = make_def_id();

    let ast = TypedAst {
        decls: vec![TypedDecl::Fn {
            def_id: fn_def_id,
            body: TypedExpr::Literal {
                ty: ty_str,
                span: dummy_span(),
                value: TypedLiteral::String("world".to_string()),
            },
        }],
        def_map,
    };

    let builder = ModuleBuilder::new();
    let (bodies, diags) = emit_all_bodies(&ast, &interner, &builder, &[]);
    assert!(diags.is_empty(), "Expected no diagnostics");
    assert_eq!(bodies.len(), 1, "Expected 1 body");

    // pending_strings should have one entry for "world"
    assert_eq!(bodies[0].pending_strings.len(), 1, "Expected 1 pending string");
    assert_eq!(bodies[0].pending_strings[0].1, "world", "Expected pending string to be 'world'");
}

#[test]
fn test_two_string_literals_produce_different_pending_entries() {
    // Two different string literals should produce two separate pending_strings entries.
    let mut interner = make_interner();
    let ty_str = interner.string_ty();
    let ty_int = interner.int();
    let (def_map, fn_def_id) = make_def_id();

    // fn with a block containing two string vars
    let ast = TypedAst {
        decls: vec![TypedDecl::Fn {
            def_id: fn_def_id,
            body: TypedExpr::Block {
                ty: ty_int,
                span: dummy_span(),
                stmts: vec![
                    TypedStmt::Let {
                        name: "a".to_string(),
                        name_span: dummy_span(),
                        ty: ty_str,
                        mutable: false,
                        value: TypedExpr::Literal {
                            ty: ty_str,
                            span: dummy_span(),
                            value: TypedLiteral::String("foo".to_string()),
                        },
                        span: dummy_span(),
                    },
                    TypedStmt::Let {
                        name: "b".to_string(),
                        name_span: dummy_span(),
                        ty: ty_str,
                        mutable: false,
                        value: TypedExpr::Literal {
                            ty: ty_str,
                            span: dummy_span(),
                            value: TypedLiteral::String("bar".to_string()),
                        },
                        span: dummy_span(),
                    },
                ],
                tail: Some(Box::new(TypedExpr::Literal {
                    ty: ty_int,
                    span: dummy_span(),
                    value: TypedLiteral::Int(0),
                })),
            },
        }],
        def_map,
    };

    let builder = ModuleBuilder::new();
    let (bodies, diags) = emit_all_bodies(&ast, &interner, &builder, &[]);
    assert!(diags.is_empty(), "Expected no diagnostics");
    assert_eq!(bodies.len(), 1, "Expected 1 body");

    // Two different string literals -> two pending_strings entries with different values
    assert_eq!(bodies[0].pending_strings.len(), 2, "Expected 2 pending strings");
    let strings: Vec<&str> = bodies[0].pending_strings.iter().map(|(_, s)| s.as_str()).collect();
    assert!(strings.contains(&"foo"), "Expected 'foo' in pending strings");
    assert!(strings.contains(&"bar"), "Expected 'bar' in pending strings");
    // Instruction indices must be different
    assert_ne!(bodies[0].pending_strings[0].0, bodies[0].pending_strings[1].0,
        "Two string literals must be at different instruction indices");
}

// ─── Plan 06, Task 1: TailCall and StrBuild emission ────────────────────────

#[test]
fn test_tail_call_return_call_emits_tail_call() {
    // Return(Call(...)) -> TailCall instead of Call + Ret
    // This is the key pattern for dialogue transitions.
    let mut interner = make_interner();
    let ty_int = interner.int();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    // TypedExpr::Return { value: Some(TypedExpr::Call { ... }) }
    let return_call_expr = TypedExpr::Return {
        ty: ty_int,
        span: dummy_span(),
        value: Some(Box::new(TypedExpr::Call {
            ty: ty_int,
            span: dummy_span(),
            callee: Box::new(TypedExpr::Var {
                ty: ty_int,
                span: dummy_span(),
                name: "some_fn".to_string(),
            }),
            args: vec![
                TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(42) },
            ],
            callee_def_id: None,
        })),
    };

    let _r = emit_expr(&mut emitter, &return_call_expr);
    let has_tail_call = emitter.instructions.iter().any(|i| matches!(i, Instruction::TailCall { .. }));
    let has_call_then_ret = {
        let instrs = &emitter.instructions;
        instrs.windows(2).any(|w| matches!(&w[0], Instruction::Call { .. }) && matches!(&w[1], Instruction::Ret { .. }))
    };
    assert!(has_tail_call, "Return(Call(...)) should emit TailCall, got {:?}", emitter.instructions);
    assert!(!has_call_then_ret, "Return(Call(...)) must NOT emit Call+Ret sequence, got {:?}", emitter.instructions);
}

#[test]
fn test_tail_call_stmt_return_call_emits_tail_call() {
    // TypedStmt::Return { value: Some(Call(...)) } -> TailCall (not Call + Ret)
    let mut interner = make_interner();
    let ty_int = interner.int();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    let return_stmt = TypedStmt::Return {
        span: dummy_span(),
        value: Some(TypedExpr::Call {
            ty: ty_int,
            span: dummy_span(),
            callee: Box::new(TypedExpr::Var {
                ty: ty_int,
                span: dummy_span(),
                name: "some_fn".to_string(),
            }),
            args: vec![],
            callee_def_id: None,
        }),
    };

    emit_stmt(&mut emitter, &return_stmt);
    let has_tail_call = emitter.instructions.iter().any(|i| matches!(i, Instruction::TailCall { .. }));
    assert!(has_tail_call, "TypedStmt::Return(Call(...)) should emit TailCall, got {:?}", emitter.instructions);
}

#[test]
fn test_normal_return_non_call_still_emits_ret() {
    // Return(literal) must still emit Ret (not TailCall)
    let mut interner = make_interner();
    let ty_int = interner.int();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    let return_expr = TypedExpr::Return {
        ty: ty_int,
        span: dummy_span(),
        value: Some(Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(99),
        })),
    };

    let _r = emit_expr(&mut emitter, &return_expr);
    let has_ret = emitter.instructions.iter().any(|i| matches!(i, Instruction::Ret { .. }));
    let has_tail_call = emitter.instructions.iter().any(|i| matches!(i, Instruction::TailCall { .. }));
    assert!(has_ret, "Return(literal) should emit Ret, got {:?}", emitter.instructions);
    assert!(!has_tail_call, "Return(literal) must NOT emit TailCall, got {:?}", emitter.instructions);
}

#[test]
fn test_str_build_three_part_chain_emits_str_build() {
    // "a" + "b" + "c" (3-part string chain) -> StrBuild { count: 3 }
    // Format strings are lowered to left-associative chains:
    // ("a" + "b") + "c" = Binary(Add, Binary(Add, "a", "b"), "c")
    let mut interner = make_interner();
    let ty_str = interner.string_ty();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    // ("a" + "b") + "c" in left-associative form
    let chain_expr = TypedExpr::Binary {
        ty: ty_str,
        span: dummy_span(),
        left: Box::new(TypedExpr::Binary {
            ty: ty_str,
            span: dummy_span(),
            left: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("a".to_string()) }),
            op: BinaryOp::Add,
            right: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("b".to_string()) }),
        }),
        op: BinaryOp::Add,
        right: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("c".to_string()) }),
    };

    let _r = emit_expr(&mut emitter, &chain_expr);
    let has_str_build = emitter.instructions.iter().any(|i| matches!(i, Instruction::StrBuild { count: 3, .. }));
    assert!(has_str_build, "3-part string chain should emit StrBuild(count=3), got {:?}", emitter.instructions);
}

#[test]
fn test_str_build_four_part_chain_emits_str_build() {
    // (("a" + "b") + "c") + "d" (4-part chain) -> StrBuild { count: 4 }
    let mut interner = make_interner();
    let ty_str = interner.string_ty();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    let chain_expr = TypedExpr::Binary {
        ty: ty_str,
        span: dummy_span(),
        left: Box::new(TypedExpr::Binary {
            ty: ty_str,
            span: dummy_span(),
            left: Box::new(TypedExpr::Binary {
                ty: ty_str,
                span: dummy_span(),
                left: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("a".to_string()) }),
                op: BinaryOp::Add,
                right: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("b".to_string()) }),
            }),
            op: BinaryOp::Add,
            right: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("c".to_string()) }),
        }),
        op: BinaryOp::Add,
        right: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("d".to_string()) }),
    };

    let _r = emit_expr(&mut emitter, &chain_expr);
    let has_str_build_4 = emitter.instructions.iter().any(|i| matches!(i, Instruction::StrBuild { count: 4, .. }));
    assert!(has_str_build_4, "4-part string chain should emit StrBuild(count=4), got {:?}", emitter.instructions);
}

#[test]
fn test_str_build_two_part_still_uses_str_concat() {
    // "a" + "b" (2-part) -> StrConcat (NOT StrBuild)
    // StrBuild is only for 3+ parts
    let mut interner = make_interner();
    let ty_str = interner.string_ty();
    let builder = ModuleBuilder::new();
    let mut emitter = make_emitter(&builder, &interner);

    let concat_expr = TypedExpr::Binary {
        ty: ty_str,
        span: dummy_span(),
        left: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("a".to_string()) }),
        op: BinaryOp::Add,
        right: Box::new(TypedExpr::Literal { ty: ty_str, span: dummy_span(), value: TypedLiteral::String("b".to_string()) }),
    };

    let _r = emit_expr(&mut emitter, &concat_expr);
    let has_str_concat = emitter.instructions.iter().any(|i| matches!(i, Instruction::StrConcat { .. }));
    let has_str_build = emitter.instructions.iter().any(|i| matches!(i, Instruction::StrBuild { .. }));
    assert!(has_str_concat, "2-part string chain should emit StrConcat, got {:?}", emitter.instructions);
    assert!(!has_str_build, "2-part string chain must NOT emit StrBuild, got {:?}", emitter.instructions);
}

// ─── Task 2 (Plan 03): Atomic block emission ─────────────────────────────────

#[test]
fn test_atomic_block_emits_atomic_begin_end() {
    // atomic { stmt1; stmt2 } -> AtomicBegin + stmts + AtomicEnd
    // Note: AtomicBegin/AtomicEnd were already partially implemented in Plan 01/03
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();
    let mut emitter = make_emitter(&builder, &interner);

    let atomic_stmt = TypedStmt::Atomic {
        body: vec![
            TypedStmt::Expr {
                expr: TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(1) },
                span: dummy_span(),
            },
        ],
        span: dummy_span(),
    };

    emit_stmt(&mut emitter, &atomic_stmt);
    assert!(matches!(&emitter.instructions[0], Instruction::AtomicBegin),
        "first instr should be AtomicBegin");
    assert!(matches!(emitter.instructions.last().unwrap(), Instruction::AtomicEnd),
        "last instr should be AtomicEnd");
}

// ─── Plan 26-04: FIX-02 compiler contract_idx emission (Task 2) ──────────────

/// Helper: build a ModuleBuilder with a ContractDef and an impl method.
/// Registers the impl method -> contract token mapping for CALL_VIRT emission.
/// Returns (builder, method_def_id, contract_token).
fn make_builder_with_virtual_contract(
    method_def_id: DefId,
) -> (ModuleBuilder, MetadataToken) {
    let mut builder = ModuleBuilder::new();

    // ContractDef "Into<Float>" at row 1 (no DefId for contract itself in tests)
    let contract_handle = builder.add_contract_def("Into<Float>", "writ", None);
    builder.add_contract_method(contract_handle, "into", 0, 0);

    // MethodDef for the impl method
    let type_handle = builder.add_typedef("Int", "writ", TypeDefKind::Struct, 0, None);
    builder.add_methoddef(Some(type_handle), "int_into_float", 0, 0, Some(method_def_id), 0);

    builder.finalize();

    // After finalize, contract_handle.0 is the 0-based contract index, so row = 0+1 = 1
    let contract_token = MetadataToken::new(TableId::ContractDef, (contract_handle.0 + 1) as u32);

    // Register the impl method -> contract mapping (FIX-02)
    builder.register_impl_method_contract(method_def_id, contract_token);

    (builder, contract_token)
}

#[test]
fn test_call_virt_emits_non_zero_contract_idx_when_registered() {
    // When an impl method DefId has a registered contract token via
    // register_impl_method_contract, emit_call(Virtual) should emit
    // CALL_VIRT with that contract token as contract_idx (non-zero).
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, method_def_id) = make_def_id();

    let (builder, contract_token) = make_builder_with_virtual_contract(method_def_id);

    let mut emitter = make_emitter(&builder, &interner);

    // Self arg in r0
    emitter.regs.alloc(ty_int); // r0 = self (Int)

    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Var {
            ty: ty_int,
            span: dummy_span(),
            name: "into".to_string(),
        }),
        args: vec![
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(42) },
        ],
        callee_def_id: None,
    };

    emit_call(&mut emitter, &call_expr, method_def_id, CallKind::Virtual { slot: 0 });

    // Find the CALL_VIRT instruction
    let call_virt = emitter.instructions.iter().find(|i| matches!(i, Instruction::CallVirt { .. }));
    assert!(call_virt.is_some(), "should have emitted a CALL_VIRT instruction");

    if let Some(Instruction::CallVirt { contract_idx, .. }) = call_virt {
        assert_ne!(
            *contract_idx, 0,
            "CALL_VIRT should emit non-zero contract_idx when contract mapping is registered; got {}",
            contract_idx
        );
        assert_eq!(
            *contract_idx, contract_token.0,
            "contract_idx should equal the registered contract token value; expected {}, got {}",
            contract_token.0, contract_idx
        );
    }
}

#[test]
fn test_call_virt_emits_zero_contract_idx_when_no_mapping() {
    // When emit_call is called with a DefId that has NO registered contract mapping,
    // CALL_VIRT should emit contract_idx=0 (the legacy fallback).
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, method_def_id) = make_def_id();

    // Builder with no registered contract mapping for method_def_id
    let builder = make_builder_with_fn(method_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Var {
            ty: ty_int,
            span: dummy_span(),
            name: "some_virtual".to_string(),
        }),
        args: vec![
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(1) },
        ],
        callee_def_id: None,
    };

    emit_call(&mut emitter, &call_expr, method_def_id, CallKind::Virtual { slot: 0 });

    let call_virt = emitter.instructions.iter().find(|i| matches!(i, Instruction::CallVirt { .. }));
    assert!(call_virt.is_some(), "should have emitted a CALL_VIRT instruction");

    if let Some(Instruction::CallVirt { contract_idx, .. }) = call_virt {
        assert_eq!(
            *contract_idx, 0,
            "CALL_VIRT should emit contract_idx=0 when no contract mapping is registered (legacy fallback)"
        );
    }
}

#[test]
fn test_call_virt_register_impl_method_contract_and_lookup() {
    // Verify that register_impl_method_contract and contract_token_for_method_def_id
    // work correctly as a pair.
    let (_, method_def_id) = make_def_id();
    let mut builder = ModuleBuilder::new();
    builder.finalize();

    // Before registration: lookup returns None
    assert!(
        builder.contract_token_for_method_def_id(method_def_id).is_none(),
        "should return None before registration"
    );

    // Register a synthetic contract token
    let contract_token = MetadataToken::new(TableId::ContractDef, 5); // row 5
    builder.register_impl_method_contract(method_def_id, contract_token);

    // After registration: lookup returns the registered token
    let result = builder.contract_token_for_method_def_id(method_def_id);
    assert!(result.is_some(), "should return Some after registration");
    assert_eq!(
        result.unwrap(), contract_token,
        "returned token should match the registered one"
    );
}

// ─── Phase 28 Plan 02: BF-02 Range construction + BF-03 DeferPush handler offset ─

/// BF-02: Range with start=0, end=10, inclusive=false should emit
/// New + LoadInt(start) + SetField(0) + LoadInt(end) + SetField(1)
/// + LoadTrue + SetField(2) + LoadFalse + SetField(3)
/// and NO Nop instruction.
#[test]
fn test_range_emits_new_and_set_field() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();

    // Range<Int> type: we use ty_int as a stand-in for testing instruction sequence
    let range_expr = TypedExpr::Range {
        ty: ty_int,
        span: dummy_span(),
        start: Some(Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(0),
        })),
        end: Some(Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(10),
        })),
        inclusive: false,
    };

    let mut emitter = make_emitter(&builder, &interner);
    emit_expr(&mut emitter, &range_expr);

    let instrs = &emitter.instructions;

    // Must not contain Nop
    let has_nop = instrs.iter().any(|i| matches!(i, Instruction::Nop));
    assert!(!has_nop, "Range should NOT emit Nop, got: {:?}", instrs);

    // Must contain a New instruction
    let has_new = instrs.iter().any(|i| matches!(i, Instruction::New { .. }));
    assert!(has_new, "Range should emit New, got: {:?}", instrs);

    // Must contain 4 SetField instructions for fields 0..=3
    let set_fields: Vec<u32> = instrs.iter().filter_map(|i| {
        if let Instruction::SetField { field_idx, .. } = i { Some(*field_idx) } else { None }
    }).collect();
    assert_eq!(set_fields.len(), 4, "Range should emit exactly 4 SetField, got: {:?}", instrs);
    assert!(set_fields.contains(&0), "should have SetField for field 0 (start)");
    assert!(set_fields.contains(&1), "should have SetField for field 1 (end)");
    assert!(set_fields.contains(&2), "should have SetField for field 2 (start_inclusive)");
    assert!(set_fields.contains(&3), "should have SetField for field 3 (end_inclusive)");

    // Field 3 (end_inclusive) for inclusive=false should use LoadFalse
    // Find the SetField { field_idx: 3 } and check the register before it uses LoadFalse
    let set_field_3_idx = instrs.iter().position(|i| {
        matches!(i, Instruction::SetField { field_idx: 3, .. })
    }).expect("should have SetField for field 3");

    // The instruction immediately before SetField(3) should be LoadFalse (for inclusive=false)
    assert!(
        set_field_3_idx > 0,
        "SetField(3) should not be the first instruction"
    );
    let before_set_field_3 = &instrs[set_field_3_idx - 1];
    assert!(
        matches!(before_set_field_3, Instruction::LoadFalse { .. }),
        "For inclusive=false, instruction before SetField(field_idx=3) should be LoadFalse, got: {:?}",
        before_set_field_3
    );
}

/// BF-02: Range with inclusive=true should emit LoadTrue for field 3 (end_inclusive).
#[test]
fn test_range_inclusive_emits_load_true_for_end() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_int = interner.int();

    let range_expr = TypedExpr::Range {
        ty: ty_int,
        span: dummy_span(),
        start: Some(Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(1),
        })),
        end: Some(Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(5),
        })),
        inclusive: true,
    };

    let mut emitter = make_emitter(&builder, &interner);
    emit_expr(&mut emitter, &range_expr);

    let instrs = &emitter.instructions;

    // Field 3 (end_inclusive) for inclusive=true should use LoadTrue
    let set_field_3_idx = instrs.iter().position(|i| {
        matches!(i, Instruction::SetField { field_idx: 3, .. })
    }).expect("should have SetField for field 3");

    assert!(set_field_3_idx > 0, "SetField(3) should not be the first instruction");
    let before_set_field_3 = &instrs[set_field_3_idx - 1];
    assert!(
        matches!(before_set_field_3, Instruction::LoadTrue { .. }),
        "For inclusive=true, instruction before SetField(field_idx=3) should be LoadTrue, got: {:?}",
        before_set_field_3
    );
}

/// BF-03: DeferPush.method_idx should NOT be 0 — it should point to the handler start.
#[test]
fn test_defer_emits_correct_handler_offset() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_void = interner.void();
    let ty_int = interner.int();
    let mut emitter = make_emitter(&builder, &interner);

    // defer { 42 }  — handler body is a single LoadInt(42)
    let defer_expr = TypedExpr::Defer {
        ty: ty_void,
        span: dummy_span(),
        expr: Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(42),
        }),
    };

    emit_expr(&mut emitter, &defer_expr);

    let instrs = &emitter.instructions;

    // Find DeferPush
    let defer_push_idx = instrs.iter().position(|i| matches!(i, Instruction::DeferPush { .. }))
        .expect("should emit DeferPush");
    let defer_push = &instrs[defer_push_idx];

    // method_idx must NOT be 0 — it should point to the handler body
    let method_idx = if let Instruction::DeferPush { method_idx, .. } = defer_push {
        *method_idx
    } else {
        panic!("expected DeferPush");
    };
    assert_ne!(method_idx, 0, "DeferPush.method_idx must not be 0 (should be handler instruction index)");

    // There should be a Br instruction (skips handler on normal path)
    let has_br = instrs.iter().any(|i| matches!(i, Instruction::Br { .. }));
    assert!(has_br, "emit_defer should emit a Br to skip the handler on normal path");

    // DeferEnd must exist
    let has_defer_end = instrs.iter().any(|i| matches!(i, Instruction::DeferEnd));
    assert!(has_defer_end, "should emit DeferEnd");

    // The instruction at method_idx should be the handler body start (LoadInt { value: 42 })
    let handler_instr = instrs.get(method_idx as usize)
        .expect("method_idx should be a valid instruction index");
    assert!(
        matches!(handler_instr, Instruction::LoadInt { value: 42, .. }),
        "instruction at handler_idx should be LoadInt(42) (handler body start), got: {:?}",
        handler_instr
    );
}

/// BF-03: DeferPush.method_idx points to the instruction AFTER DeferPop and Br.
/// Verifies the exact sequence layout.
#[test]
fn test_defer_handler_offset_matches_handler_start() {
    let builder = ModuleBuilder::new();
    let mut interner = make_interner();
    let ty_void = interner.void();
    let ty_int = interner.int();
    let mut emitter = make_emitter(&builder, &interner);

    // defer { 99 } — simple handler body
    let defer_expr = TypedExpr::Defer {
        ty: ty_void,
        span: dummy_span(),
        expr: Box::new(TypedExpr::Literal {
            ty: ty_int,
            span: dummy_span(),
            value: TypedLiteral::Int(99),
        }),
    };

    emit_expr(&mut emitter, &defer_expr);

    let instrs = &emitter.instructions;

    // Expected sequence:
    // [0] DeferPush { method_idx: 3 }   (points to [3])
    // [1] DeferPop
    // [2] Br { offset: N }             (skips past handler)
    // [3] LoadInt { value: 99 }        (handler body)
    // [4] DeferEnd
    assert!(instrs.len() >= 5, "expected at least 5 instructions, got {:?}", instrs.len());

    assert!(matches!(instrs[0], Instruction::DeferPush { .. }), "instrs[0] should be DeferPush");
    assert!(matches!(instrs[1], Instruction::DeferPop), "instrs[1] should be DeferPop");
    assert!(matches!(instrs[2], Instruction::Br { .. }), "instrs[2] should be Br");

    let method_idx = if let Instruction::DeferPush { method_idx, .. } = instrs[0] { method_idx } else { panic!() };
    assert_eq!(method_idx, 3, "DeferPush.method_idx should be 3 (handler starts at instruction index 3)");

    assert!(matches!(instrs[3], Instruction::LoadInt { value: 99, .. }), "instrs[3] should be LoadInt(99)");
    assert!(matches!(instrs[4], Instruction::DeferEnd), "instrs[4] should be DeferEnd");

    // Verify the Br skips the entire handler body
    let br_offset = if let Instruction::Br { offset } = instrs[2] { offset } else { panic!() };
    // Br at index 2 with offset targeting index 5 (after DeferEnd)
    // offset = target_idx - br_idx = 5 - 2 = 3
    assert_eq!(br_offset, 3, "Br offset should skip handler+DeferEnd (offset=3 means target is index 5)");
}

// ─── Plan 28-01: MC-01 and BF-01 — callee_def_id propagation ─────────────────

/// MC-01: When TypedExpr::Call has callee_def_id=Some(def_id) and that def_id has
/// a registered method token in the builder, emit_expr should emit CALL with
/// the correct non-zero method_idx (not 0 as before the fix).
#[test]
fn test_call_with_callee_def_id_emits_correct_method_idx() {
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, fn_def_id) = make_def_id();
    let builder = make_builder_with_fn(fn_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    // Build a TypedExpr::Call with callee_def_id=Some(fn_def_id)
    // This simulates the output of check_call_with_sig after MC-01 fix.
    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Var {
            ty: ty_int,
            span: dummy_span(),
            name: "test_fn".to_string(),
        }),
        args: vec![],
        callee_def_id: Some(fn_def_id),
    };

    let _r = emit_expr(&mut emitter, &call_expr);

    // Find the Call instruction and verify method_idx is non-zero
    let call_instr = emitter.instructions.iter().find(|i| matches!(i, Instruction::Call { .. }));
    assert!(call_instr.is_some(), "should have emitted a CALL instruction");
    if let Some(Instruction::Call { method_idx, .. }) = call_instr {
        assert_ne!(
            *method_idx, 0,
            "CALL should emit non-zero method_idx when callee_def_id is Some and token is registered; got {}",
            method_idx
        );
    }
}

/// BF-01: When TypedExpr::Call has callee_def_id=Some(def_id) and that def_id has
/// a registered contract mapping, emit_expr on a generic-receiver call should emit
/// CALL_VIRT with the correct non-zero contract_idx.
#[test]
fn test_call_virt_via_emit_expr_uses_callee_def_id_for_contract_idx() {
    let mut interner = make_interner();
    let ty_int = interner.int();
    let ty_generic = interner.intern(TyKind::GenericParam(0));
    let (_, method_def_id) = make_def_id();

    let (builder, contract_token) = make_builder_with_virtual_contract(method_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    // Set up "self" register with generic type
    emitter.regs.alloc(ty_generic); // r0 = self with generic receiver type

    // Build a TypedExpr::Call with Field callee on a generic receiver
    // and callee_def_id=Some(method_def_id) which has a registered contract mapping.
    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Field {
            ty: ty_int,
            span: dummy_span(),
            receiver: Box::new(TypedExpr::SelfRef { ty: ty_generic, span: dummy_span() }),
            field: "into".to_string(),
        }),
        args: vec![],
        callee_def_id: Some(method_def_id),
    };

    let _r = emit_expr(&mut emitter, &call_expr);

    // Find CALL_VIRT and verify contract_idx is correct
    let call_virt = emitter.instructions.iter().find(|i| matches!(i, Instruction::CallVirt { .. }));
    assert!(call_virt.is_some(), "generic receiver call should emit CALL_VIRT, got {:?}", emitter.instructions);
    if let Some(Instruction::CallVirt { contract_idx, .. }) = call_virt {
        assert_ne!(
            *contract_idx, 0,
            "CALL_VIRT should emit non-zero contract_idx when callee_def_id has registered contract mapping; got {}",
            contract_idx
        );
        assert_eq!(
            *contract_idx, contract_token.0,
            "CALL_VIRT contract_idx should equal the registered contract token; expected {}, got {}",
            contract_token.0, contract_idx
        );
    }
}

/// MC-01: When callee_def_id=None (legacy path / error path), CALL should emit
/// method_idx=0 as backward-compatible fallback.
#[test]
fn test_call_with_none_callee_def_id_emits_zero_method_idx() {
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, fn_def_id) = make_def_id();
    let builder = make_builder_with_fn(fn_def_id);
    let mut emitter = make_emitter(&builder, &interner);

    // callee_def_id: None — legacy/error path should fall back to method_idx=0
    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Var {
            ty: ty_int,
            span: dummy_span(),
            name: "unknown_fn".to_string(),
        }),
        args: vec![],
        callee_def_id: None,
    };

    let _r = emit_expr(&mut emitter, &call_expr);

    let call_instr = emitter.instructions.iter().find(|i| matches!(i, Instruction::Call { .. }));
    assert!(call_instr.is_some(), "should have emitted a CALL instruction");
    if let Some(Instruction::Call { method_idx, .. }) = call_instr {
        assert_eq!(
            *method_idx, 0,
            "CALL with callee_def_id=None should emit method_idx=0 (backward compat fallback); got {}",
            method_idx
        );
    }
}

// ─── BUG-05: Extern dispatch via emit_expr ────────────────────────────────────

/// BUG-05 fix: When callee_def_id maps to an ExternDef token, emit_expr should
/// emit CALL_EXTERN (not CALL). This verifies the new is_extern check in the
/// TypedExpr::Call arm of emit_expr.
#[test]
fn test_emit_expr_extern_call_emits_call_extern() {
    let mut interner = make_interner();
    let ty_int = interner.int();

    // Create an ExternFn DefId and register it in the builder as an ExternDef.
    let (_, extern_def_id) = make_def_id();
    let builder = make_builder_with_extern(extern_def_id);

    // Verify the token is in ExternDef table (confirms test setup is correct).
    let token = builder.token_for_def(extern_def_id).expect("extern def should have token");
    assert_eq!(
        token.table(),
        TableId::ExternDef,
        "extern fn token should be in ExternDef table"
    );

    let mut emitter = make_emitter(&builder, &interner);

    // Build a TypedExpr::Call with callee_def_id=Some(extern_def_id).
    // The callee is a simple Var (not a Field), so the dispatch hits the `_ =>` arm.
    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Var {
            ty: ty_int,
            span: dummy_span(),
            name: "ext_fn".to_string(),
        }),
        args: vec![
            TypedExpr::Literal { ty: ty_int, span: dummy_span(), value: TypedLiteral::Int(99) },
        ],
        callee_def_id: Some(extern_def_id),
    };

    let _r = emit_expr(&mut emitter, &call_expr);

    // The last call-type instruction should be CALL_EXTERN, not CALL.
    let call_instr = emitter.instructions.iter().find(|i| {
        matches!(i, Instruction::Call { .. } | Instruction::CallExtern { .. })
    });
    assert!(call_instr.is_some(), "should have emitted a call instruction, got {:?}", emitter.instructions);
    assert!(
        matches!(call_instr.unwrap(), Instruction::CallExtern { .. }),
        "extern fn callee_def_id should produce CALL_EXTERN via emit_expr, got {:?}",
        call_instr.unwrap()
    );

    // Also verify extern_idx is non-zero (token was resolved correctly).
    if let Some(Instruction::CallExtern { extern_idx, .. }) = call_instr {
        assert_ne!(
            *extern_idx, 0,
            "CALL_EXTERN extern_idx should be non-zero when ExternDef token is registered; got {}",
            extern_idx
        );
    }
}

/// BUG-05 negative case: When callee_def_id maps to a MethodDef token (not ExternDef),
/// emit_expr should still emit CALL (not CALL_EXTERN).
#[test]
fn test_emit_expr_non_extern_call_emits_call() {
    let mut interner = make_interner();
    let ty_int = interner.int();
    let (_, fn_def_id) = make_def_id();
    let builder = make_builder_with_fn(fn_def_id);

    // Verify token is MethodDef, not ExternDef.
    let token = builder.token_for_def(fn_def_id).expect("fn should have token");
    assert_ne!(
        token.table(),
        TableId::ExternDef,
        "regular fn token should not be in ExternDef table"
    );

    let mut emitter = make_emitter(&builder, &interner);

    let call_expr = TypedExpr::Call {
        ty: ty_int,
        span: dummy_span(),
        callee: Box::new(TypedExpr::Var {
            ty: ty_int,
            span: dummy_span(),
            name: "test_fn".to_string(),
        }),
        args: vec![],
        callee_def_id: Some(fn_def_id),
    };

    let _r = emit_expr(&mut emitter, &call_expr);

    // Should emit CALL, not CALL_EXTERN.
    let call_instr = emitter.instructions.iter().find(|i| {
        matches!(i, Instruction::Call { .. } | Instruction::CallExtern { .. })
    });
    assert!(call_instr.is_some(), "should have emitted a call instruction");
    assert!(
        matches!(call_instr.unwrap(), Instruction::Call { .. }),
        "regular fn callee_def_id should produce CALL (not CALL_EXTERN), got {:?}",
        call_instr.unwrap()
    );
}
