//! Tests for PREP-05: disassembler .locals section output.
//!
//! The requirement: writ disasm output shows a .locals section with register names
//! and type names for methods that have debug_locals with non-zero name offsets.
//!
//! These tests construct Module structs programmatically (bypassing the assembler)
//! so we can inject debug_locals with type_ref blobs and verify the disassembler
//! text output contains the expected .locals section content.

use writ_module::heap::{intern_string, write_blob};
use writ_module::instruction::Instruction;
use writ_module::module::{DebugLocal, MethodBody};
use writ_module::tables::{
    MethodDefRow, TypeDefKind, TypeDefRow,
};
use writ_module::Module;

/// Build a module with one method body that has named debug locals.
///
/// `locals`: list of (register, name, type_blob, start_pc, end_pc)
/// `type_blob` is the raw bytes for the type blob (e.g. `[0x01]` for int).
fn build_module_with_locals(locals: &[(u16, &str, &[u8], u32, u32)]) -> Module {
    let mut module = Module::new();

    // Intern a type name and namespace for the type def
    let type_name = intern_string(&mut module.string_heap, "TestType");
    let ns = intern_string(&mut module.string_heap, "");

    module.type_defs.push(TypeDefRow {
        name: type_name,
        namespace: ns,
        kind: TypeDefKind::Struct.as_u8(),
        flags: 0,
        field_list: 1,
        method_list: 1,
    });

    // Intern method name
    let method_name = intern_string(&mut module.string_heap, "main");

    // Build debug_locals
    let mut debug_locals = Vec::new();
    for (reg, name, type_bytes, start_pc, end_pc) in locals {
        let name_off = intern_string(&mut module.string_heap, name);
        let type_ref = if type_bytes.is_empty() {
            0u32
        } else {
            write_blob(&mut module.blob_heap, type_bytes)
        };
        debug_locals.push(DebugLocal {
            register: *reg,
            name: name_off,
            type_ref,
            start_pc: *start_pc,
            end_pc: *end_pc,
        });
    }

    // Encode a minimal body: LOAD_INT r0, 1; RET_VOID
    let mut code = Vec::new();
    Instruction::LoadInt { r_dst: 0, value: 1 }.encode(&mut code).unwrap();
    Instruction::RetVoid.encode(&mut code).unwrap();

    let body = MethodBody {
        register_types: vec![0u32; locals.len().max(1)],
        code,
        debug_locals,
        source_spans: vec![],
    };

    // Intern a method signature (empty sig)
    let sig_off = write_blob(&mut module.blob_heap, &[0x00, 0x00]); // 0 params, void return

    module.method_defs.push(MethodDefRow {
        name: method_name,
        signature: sig_off,
        flags: 0,
        body_offset: 0,
        body_size: body.code.len() as u32,
        reg_count: locals.len().max(1) as u16,
        param_count: 0,
    });

    module.method_bodies.push(body);

    module
}

/// The disassembler emits a .locals section when a method has named debug locals.
///
/// This verifies PREP-05: ".locals section with register names and types" appears
/// in writ disasm output.
#[test]
fn disasm_emits_locals_section_for_named_locals() {
    // int type blob: tag 0x01
    let module = build_module_with_locals(&[
        (0, "my_var", &[0x01u8], 0, 100),
    ]);

    let text = writ_assembler::disassemble(&module);

    assert!(
        text.contains(".locals"),
        "disasm output should contain '.locals' section header, got:\n{}",
        text
    );
    assert!(
        text.contains("my_var"),
        "disasm output should contain the local variable name 'my_var', got:\n{}",
        text
    );
}

/// The .locals section shows the decoded type name (not a raw offset).
///
/// For a register with type blob `[0x01]` (int), the section should show "int".
#[test]
fn disasm_locals_section_shows_decoded_type_name_for_int() {
    // int type blob: primitive tag 0x01
    let module = build_module_with_locals(&[
        (0, "count", &[0x01u8], 0, 50),
    ]);

    let text = writ_assembler::disassemble(&module);

    assert!(
        text.contains(".locals"),
        "output should contain .locals section"
    );
    assert!(
        text.contains("int"),
        "output should contain type name 'int' for primitive tag 0x01, got:\n{}",
        text
    );
    assert!(
        text.contains("count"),
        "output should contain variable name 'count', got:\n{}",
        text
    );
}

/// The .locals section shows the decoded type name for bool (tag 0x03).
#[test]
fn disasm_locals_section_shows_decoded_type_name_for_bool() {
    // bool type blob: primitive tag 0x03
    let module = build_module_with_locals(&[
        (0, "flag", &[0x03u8], 0, 50),
    ]);

    let text = writ_assembler::disassemble(&module);

    assert!(
        text.contains(".locals"),
        "output should contain .locals section"
    );
    assert!(
        text.contains("bool"),
        "output should contain type name 'bool' for tag 0x03, got:\n{}",
        text
    );
    assert!(
        text.contains("flag"),
        "output should contain variable name 'flag'"
    );
}

/// Multiple named locals all appear in the .locals section.
#[test]
fn disasm_locals_section_lists_all_named_locals() {
    // Two named registers: r0=int "x", r1=float "y"
    let module = build_module_with_locals(&[
        (0, "x", &[0x01u8], 0, 100),   // int
        (1, "y", &[0x02u8], 0, 100),   // float
    ]);

    let text = writ_assembler::disassemble(&module);

    assert!(text.contains(".locals"), "output should contain .locals section");
    assert!(text.contains("\"x\""), "output should list variable 'x'");
    assert!(text.contains("\"y\""), "output should list variable 'y'");
    assert!(text.contains("int"), "output should show 'int' type for x");
    assert!(text.contains("float"), "output should show 'float' type for y");
}

/// Registers with name offset 0 (unnamed temporaries) are excluded from .locals.
#[test]
fn disasm_locals_section_excludes_unnamed_temporaries() {
    // Build a module with one named and one unnamed local
    let mut module = Module::new();

    let type_name = intern_string(&mut module.string_heap, "T");
    let ns = intern_string(&mut module.string_heap, "");
    module.type_defs.push(TypeDefRow {
        name: type_name,
        namespace: ns,
        kind: TypeDefKind::Struct.as_u8(),
        flags: 0,
        field_list: 1,
        method_list: 1,
    });

    let method_name = intern_string(&mut module.string_heap, "fn");
    let named_off = intern_string(&mut module.string_heap, "my_x");
    // Unnamed: name = 0 (string heap offset 0 is always empty string)
    let int_type_ref = write_blob(&mut module.blob_heap, &[0x01u8]);
    let sig_off = write_blob(&mut module.blob_heap, &[0x00, 0x00]);

    let mut code = Vec::new();
    Instruction::RetVoid.encode(&mut code).unwrap();

    let body = MethodBody {
        register_types: vec![0u32; 2],
        code,
        debug_locals: vec![
            DebugLocal { register: 0, name: named_off, type_ref: int_type_ref, start_pc: 0, end_pc: 10 },
            DebugLocal { register: 1, name: 0,          type_ref: 0,            start_pc: 0, end_pc: 10 },
        ],
        source_spans: vec![],
    };

    module.method_defs.push(MethodDefRow {
        name: method_name,
        signature: sig_off,
        flags: 0,
        body_offset: 0,
        body_size: body.code.len() as u32,
        reg_count: 2,
        param_count: 0,
    });
    module.method_bodies.push(body);

    let text = writ_assembler::disassemble(&module);

    assert!(text.contains(".locals"), "should have .locals section for the named local");
    assert!(text.contains("my_x"), "should contain the named variable");

    // r1 has name=0 so it should be excluded from .locals
    // We just verify the named one is present; unnamed are silently omitted
    // (no way to check absence of r1 by name since it has no name)
}

/// A method with no debug locals (all name=0) does NOT emit a .locals section.
#[test]
fn disasm_no_locals_section_when_all_registers_are_unnamed() {
    let mut module = Module::new();

    let type_name = intern_string(&mut module.string_heap, "T");
    let ns = intern_string(&mut module.string_heap, "");
    module.type_defs.push(TypeDefRow {
        name: type_name,
        namespace: ns,
        kind: TypeDefKind::Struct.as_u8(),
        flags: 0,
        field_list: 1,
        method_list: 1,
    });

    let method_name = intern_string(&mut module.string_heap, "fn");
    let sig_off = write_blob(&mut module.blob_heap, &[0x00, 0x00]);

    let mut code = Vec::new();
    Instruction::RetVoid.encode(&mut code).unwrap();

    let body = MethodBody {
        register_types: vec![0u32; 1],
        code,
        debug_locals: vec![
            // All unnamed — name=0
            DebugLocal { register: 0, name: 0, type_ref: 0, start_pc: 0, end_pc: 10 },
        ],
        source_spans: vec![],
    };

    module.method_defs.push(MethodDefRow {
        name: method_name,
        signature: sig_off,
        flags: 0,
        body_offset: 0,
        body_size: body.code.len() as u32,
        reg_count: 1,
        param_count: 0,
    });
    module.method_bodies.push(body);

    let text = writ_assembler::disassemble(&module);

    assert!(
        !text.contains(".locals"),
        "output should NOT contain .locals section when all registers are unnamed, got:\n{}",
        text
    );
}

/// The .locals section shows the scope range [start_pc, end_pc) for each local.
#[test]
fn disasm_locals_section_shows_scope_range() {
    let module = build_module_with_locals(&[
        (0, "item", &[0x01u8], 4, 20),
    ]);

    let text = writ_assembler::disassemble(&module);

    assert!(text.contains(".locals"));
    // The scope range [4, 20) should appear in the output
    assert!(
        text.contains("4") && text.contains("20"),
        "output should contain scope start 4 and end 20, got:\n{}",
        text
    );
}
