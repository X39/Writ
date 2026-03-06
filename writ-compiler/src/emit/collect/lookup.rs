//! AST lookup helpers: find_*_decl functions for locating original AST nodes by DefEntry.

use writ_diagnostics::FileId;

use crate::ast::decl::{
    AstDecl, AstExternDecl, AstFnDecl, AstFnSig, AstStructDecl, AstEntityDecl, AstEnumDecl,
    AstContractDecl, AstImplDecl, AstComponentDecl, AstClassDecl,
    AstConstDecl, AstGlobalDecl, AstAttribute,
};
use crate::ast::Ast;
use crate::resolve::def_map::DefEntry;

// =============================================================================
// AST lookup helpers (adapted from check/env.rs)
// =============================================================================

pub(super) fn find_struct_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstStructDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Struct(s) = decl
                && s.name == entry.name && s.name_span == entry.name_span {
                    return Some(s);
                }
        }
    }
    None
}

pub(super) fn find_entity_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstEntityDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Entity(e) = decl
                && e.name == entry.name && e.name_span == entry.name_span {
                    return Some(e);
                }
        }
    }
    None
}

pub(super) fn find_enum_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstEnumDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Enum(e) = decl
                && e.name == entry.name && e.name_span == entry.name_span {
                    return Some(e);
                }
        }
    }
    None
}

pub(super) fn find_fn_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstFnDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Fn(f) = decl
                && f.name == entry.name && f.name_span == entry.name_span {
                    return Some(f);
                }
        }
    }
    None
}

pub(super) fn find_contract_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstContractDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Contract(c) = decl
                && c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
        }
    }
    None
}

pub(super) fn find_impl_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstImplDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Impl(i) = decl
                && i.span == entry.span {
                    return Some(i);
                }
        }
    }
    None
}

pub(super) fn find_component_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstComponentDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            match decl {
                AstDecl::Component(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return Some(c);
                }
                AstDecl::Extern(AstExternDecl::Component(_, c)) if c.name == entry.name && c.name_span == entry.name_span => {
                    return Some(c);
                }
                _ => {}
            }
        }
    }
    None
}

pub(super) fn find_extern_fn_sig<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstFnSig> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Extern(AstExternDecl::Fn(_, sig)) = decl
                && sig.name == entry.name && sig.name_span == entry.name_span {
                    return Some(sig);
                }
        }
    }
    None
}

pub(super) fn find_extern_struct_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstStructDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Extern(AstExternDecl::Struct(_, s)) = decl
                && s.name == entry.name && s.name_span == entry.name_span {
                    return Some(s);
                }
        }
    }
    None
}

pub(super) fn find_class_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstClassDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Class(c) = decl
                && c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
        }
    }
    None
}

pub(super) fn find_extern_class_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstClassDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Extern(AstExternDecl::Class(_, c)) = decl
                && c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
        }
    }
    None
}

pub(super) fn find_const_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstConstDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Const(c) = decl
                && c.name == entry.name && c.name_span == entry.name_span {
                    return Some(c);
                }
        }
    }
    None
}

pub(super) fn find_global_decl<'a>(asts: &'a [(FileId, &Ast)], entry: &DefEntry) -> Option<&'a AstGlobalDecl> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id { continue; }
        for decl in &ast.items {
            if let AstDecl::Global(g) = decl
                && g.name == entry.name && g.name_span == entry.name_span {
                    return Some(g);
                }
        }
    }
    None
}

// =============================================================================
// Attribute lookup helper (used by encoding.rs)
// =============================================================================

pub(super) fn find_attrs_for_entry(asts: &[(FileId, &Ast)], entry: &DefEntry) -> Vec<AstAttribute> {
    for (file_id, ast) in asts {
        if *file_id != entry.file_id {
            continue;
        }
        for decl in &ast.items {
            match decl {
                AstDecl::Struct(s) if s.name == entry.name && s.name_span == entry.name_span => {
                    return s.attrs.clone();
                }
                AstDecl::Entity(e) if e.name == entry.name && e.name_span == entry.name_span => {
                    return e.attrs.clone();
                }
                AstDecl::Enum(e) if e.name == entry.name && e.name_span == entry.name_span => {
                    return e.attrs.clone();
                }
                AstDecl::Contract(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return c.attrs.clone();
                }
                AstDecl::Component(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return c.attrs.clone();
                }
                AstDecl::Fn(f) if f.name == entry.name && f.name_span == entry.name_span => {
                    return f.attrs.clone();
                }
                AstDecl::Const(c) if c.name == entry.name && c.name_span == entry.name_span => {
                    return c.attrs.clone();
                }
                AstDecl::Global(g) if g.name == entry.name && g.name_span == entry.name_span => {
                    return g.attrs.clone();
                }
                _ => {}
            }
        }
    }
    Vec::new()
}
