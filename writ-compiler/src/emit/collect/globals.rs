//! Const and global definition collection.

use writ_diagnostics::{Diagnostic, FileId};

use crate::ast::Ast;
use crate::check::ty::TyInterner;
use crate::resolve::def_map::{DefId, DefMap, DefVis};

use crate::emit::module_builder::ModuleBuilder;

use super::encoding::encode_type_from_ast;
use super::lookup::{find_const_decl, find_global_decl};

pub(super) fn collect_const(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    _diags: &mut Vec<Diagnostic>,
) {
    let entry = def_map.get_entry(def_id);
    let is_pub = matches!(entry.vis, DefVis::Pub);

    if let Some(const_decl) = find_const_decl(asts, entry) {
        let type_blob = encode_type_from_ast(&const_decl.ty, interner, &entry.generics, builder);
        // Flags: bit 0 = pub, bit 1 = is_const
        let flags: u16 = (if is_pub { 1 } else { 0 }) | (1 << 1);
        builder.add_global_def(&entry.name, type_blob, flags, 0, Some(def_id));
    }
}

pub(super) fn collect_global(
    def_id: DefId,
    def_map: &DefMap,
    asts: &[(FileId, &Ast)],
    interner: &TyInterner,
    builder: &mut ModuleBuilder,
    _diags: &mut Vec<Diagnostic>,
) {
    let entry = def_map.get_entry(def_id);
    let is_pub = matches!(entry.vis, DefVis::Pub);

    if let Some(global_decl) = find_global_decl(asts, entry) {
        let type_blob = encode_type_from_ast(&global_decl.ty, interner, &entry.generics, builder);
        // Flags: bit 0 = pub, bit 2 = is_mutable
        let flags: u16 = (if is_pub { 1 } else { 0 }) | (1 << 2);
        builder.add_global_def(&entry.name, type_blob, flags, 0, Some(def_id));
    }
}
