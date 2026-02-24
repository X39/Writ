pub mod context;
pub mod error;
pub mod optional;
pub mod fmt_string;
pub mod expr;
pub mod stmt;
pub mod operator;
pub mod dialogue;
pub mod entity;

use chumsky::span::SimpleSpan;
use writ_parser::cst::{
    Attribute, AttrArg, ComponentDecl, ComponentMember, ConstDecl, ContractDecl, ContractMember,
    EnumDecl, EnumVariant, ExternDecl, FnDecl, FnSig, GlobalDecl, Item,
    NamespaceDecl, OpSig, OpSymbol, Param, GenericParam, Spanned, StructDecl, StructField,
    UsingDecl, Visibility,
};
use crate::ast::{Ast, AstDecl};
use crate::ast::decl::{
    AstAttribute, AstAttributeArg, AstComponentDecl, AstComponentMember, AstConstDecl,
    AstContractDecl, AstContractMember, AstEnumDecl, AstEnumVariant, AstExternDecl, AstFnDecl,
    AstFnSig, AstGenericParam, AstGlobalDecl, AstNamespaceDecl,
    AstOpSig, AstOpSymbol, AstParam, AstStructDecl, AstStructField, AstUsingDecl,
    AstVisibility,
};
use crate::lower::context::LoweringContext;
use crate::lower::error::LoweringError;
use crate::lower::expr::lower_expr;
use crate::lower::dialogue::lower_dialogue;
use crate::lower::entity::lower_entity;
use crate::lower::operator::lower_operator_impls;
use crate::lower::optional::lower_type;
use crate::lower::stmt::lower_stmt;

/// Lowers a CST item list to a simplified AST.
///
/// # Pass Ordering
///
/// Passes execute in this order (rationale: each pass's output is required
/// by subsequent passes):
///
/// 1. **Expression helpers** (invoked from inside structural passes, not top-level):
///    - `lower_optional` — `T?` → `Option<T>`, `null` → `Option::None` (type positions + null exprs)
///    - `lower_fmt_string` — `$"..."` → string concatenation with `.into<string>()` calls
///    - `lower_compound_assign` — `a += b` → `a = a + b` (mechanical expansion)
///    - `lower_operator` — operator decls → contract impl methods
///    - `lower_concurrency` — spawn/join/cancel/defer/detached pass-through (1:1 mapping)
///
/// 2. **Structural passes** (top-level, process Item variants):
///    - `lower_fn` — Fn items; invokes expression helpers on body
///    - `lower_dialogue` — Dlg items → Fn decls; invokes expression helpers + localization sub-pass
///    - `lower_entity` — Entity items → Struct + Impl + lifecycle registrations
///
/// Expression helpers run BEFORE structural passes because structural passes
/// invoke them when they encounter expression/type positions. The helpers are
/// not standalone top-level passes — they are called per-node during structural
/// lowering.
///
/// # Error Handling
///
/// All errors are accumulated in `LoweringContext::errors`. No pass halts on
/// error — remaining items continue lowering. The returned `Vec<LoweringError>`
/// contains all errors from all passes.
pub fn lower(items: Vec<Spanned<Item<'_>>>) -> (Ast, Vec<LoweringError>) {
    let mut ctx = LoweringContext::new();
    let mut decls = Vec::new();

    for (item, _item_span) in items {
        match item {
            Item::Fn((fn_decl, fn_span)) => {
                decls.push(AstDecl::Fn(lower_fn(fn_decl, fn_span, &mut ctx)));
            }

            // --- Structural pass-throughs ---
            Item::Namespace((ns, ns_span)) => {
                decls.push(lower_namespace(ns, ns_span, &mut ctx));
            }
            Item::Using((u, u_span)) => {
                decls.push(lower_using(u, u_span));
            }
            Item::Struct((s, s_span)) => {
                decls.push(AstDecl::Struct(lower_struct(s, s_span, &mut ctx)));
            }
            Item::Enum((e, e_span)) => {
                decls.push(AstDecl::Enum(lower_enum(e, e_span, &mut ctx)));
            }
            Item::Contract((c, c_span)) => {
                decls.push(AstDecl::Contract(lower_contract(c, c_span, &mut ctx)));
            }
            Item::Impl((i, i_span)) => {
                decls.extend(lower_operator_impls(i, i_span, &mut ctx));
            }
            Item::Component((c, c_span)) => {
                decls.push(AstDecl::Component(lower_component(c, c_span, &mut ctx)));
            }
            Item::Extern((e, e_span)) => {
                decls.push(AstDecl::Extern(lower_extern(e, e_span, &mut ctx)));
            }
            Item::Const((c, c_span)) => {
                decls.push(AstDecl::Const(lower_const(c, c_span, &mut ctx)));
            }
            Item::Global((g, g_span)) => {
                decls.push(AstDecl::Global(lower_global(g, g_span, &mut ctx)));
            }
            Item::Stmt((s, s_span)) => {
                decls.push(AstDecl::Stmt(lower_stmt((s, s_span), &mut ctx)));
            }

            // --- Structural passes deferred to later phases ---
            Item::Dlg((dlg_decl, dlg_span)) => {
                decls.push(AstDecl::Fn(lower_dialogue(dlg_decl, dlg_span, &mut ctx)));
            }
            Item::Entity((e, e_span)) => {
                decls.extend(lower_entity(e, e_span, &mut ctx));
            }
        }
    }

    (Ast { items: decls }, ctx.take_errors())
}

// =========================================================
// lower_fn and helpers
// =========================================================

pub(crate) fn lower_fn(f: FnDecl<'_>, fn_span: SimpleSpan, ctx: &mut LoweringContext) -> AstFnDecl {
    AstFnDecl {
        attrs: lower_attrs(f.attrs, ctx),
        vis: lower_vis(f.vis),
        name: f.name.0.to_string(),
        name_span: f.name.1,
        generics: f
            .generics
            .unwrap_or_default()
            .into_iter()
            .map(|(gp, gp_span)| lower_generic_param(gp, gp_span))
            .collect(),
        params: f
            .params
            .into_iter()
            .map(|(param, param_span)| lower_param(param, param_span))
            .collect(),
        return_type: f.return_type.map(lower_type),
        body: f.body.into_iter().map(|s| lower_stmt(s, ctx)).collect(),
        span: fn_span,
    }
}

fn lower_fn_sig(f: FnSig<'_>, sig_span: SimpleSpan, ctx: &mut LoweringContext) -> AstFnSig {
    AstFnSig {
        attrs: lower_attrs(f.attrs, ctx),
        vis: lower_vis(f.vis),
        name: f.name.0.to_string(),
        name_span: f.name.1,
        generics: f
            .generics
            .unwrap_or_default()
            .into_iter()
            .map(|(gp, gp_span)| lower_generic_param(gp, gp_span))
            .collect(),
        params: f
            .params
            .into_iter()
            .map(|(param, param_span)| lower_param(param, param_span))
            .collect(),
        return_type: f.return_type.map(lower_type),
        span: sig_span,
    }
}

fn lower_op_sig(op: OpSig<'_>, op_span: SimpleSpan) -> AstOpSig {
    AstOpSig {
        vis: lower_vis(op.vis),
        symbol: lower_op_symbol(op.symbol.0),
        symbol_span: op.symbol.1,
        params: op
            .params
            .into_iter()
            .map(|(param, param_span)| lower_param(param, param_span))
            .collect(),
        return_type: op.return_type.map(lower_type),
        span: op_span,
    }
}

fn lower_op_symbol(sym: OpSymbol) -> AstOpSymbol {
    match sym {
        OpSymbol::Add => AstOpSymbol::Add,
        OpSymbol::Sub => AstOpSymbol::Sub,
        OpSymbol::Mul => AstOpSymbol::Mul,
        OpSymbol::Div => AstOpSymbol::Div,
        OpSymbol::Mod => AstOpSymbol::Mod,
        OpSymbol::Eq => AstOpSymbol::Eq,
        OpSymbol::Lt => AstOpSymbol::Lt,
        OpSymbol::Not => AstOpSymbol::Not,
        OpSymbol::Index => AstOpSymbol::Index,
        OpSymbol::IndexSet => AstOpSymbol::IndexSet,
    }
}

// =========================================================
// Shared helpers
// =========================================================

pub(crate) fn lower_vis(vis: Option<Visibility>) -> Option<AstVisibility> {
    match vis {
        Some(Visibility::Pub) => Some(AstVisibility::Pub),
        Some(Visibility::Priv) => Some(AstVisibility::Priv),
        None => None,
    }
}

pub(crate) fn lower_param(param: Param<'_>, param_span: SimpleSpan) -> AstParam {
    AstParam {
        name: param.name.0.to_string(),
        name_span: param.name.1,
        ty: lower_type(param.ty),
        span: param_span,
    }
}

fn lower_generic_param(gp: GenericParam<'_>, gp_span: SimpleSpan) -> AstGenericParam {
    AstGenericParam {
        name: gp.name.0.to_string(),
        name_span: gp.name.1,
        bounds: gp.bounds.into_iter().map(lower_type).collect(),
        span: gp_span,
    }
}

pub(crate) fn lower_attrs(
    attrs: Vec<Spanned<Vec<Attribute<'_>>>>,
    ctx: &mut LoweringContext,
) -> Vec<AstAttribute> {
    let mut result = Vec::new();
    for (attr_block, block_span) in attrs {
        for attr in attr_block {
            let args = attr
                .args
                .into_iter()
                .map(|(arg, arg_span)| lower_attr_arg(arg, arg_span, ctx))
                .collect();
            result.push(AstAttribute {
                name: attr.name.0.to_string(),
                name_span: attr.name.1,
                args,
                span: block_span,
            });
        }
    }
    result
}

fn lower_attr_arg(
    arg: AttrArg<'_>,
    _arg_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstAttributeArg {
    match arg {
        AttrArg::Positional(expr) => AstAttributeArg::Positional(lower_expr(expr, ctx)),
        AttrArg::Named((name, name_span), expr) => AstAttributeArg::Named {
            name: name.to_string(),
            name_span,
            value: lower_expr(expr, ctx),
        },
    }
}

pub(crate) fn lower_struct_field(field: StructField<'_>, field_span: SimpleSpan, ctx: &mut LoweringContext) -> AstStructField {
    AstStructField {
        vis: lower_vis(field.vis),
        name: field.name.0.to_string(),
        name_span: field.name.1,
        ty: lower_type(field.ty),
        default: field.default.map(|d| lower_expr(d, ctx)),
        span: field_span,
    }
}

// =========================================================
// Structural pass-through lowering functions
// =========================================================

fn lower_namespace(ns: NamespaceDecl<'_>, ns_span: SimpleSpan, ctx: &mut LoweringContext) -> AstDecl {
    match ns {
        NamespaceDecl::Declarative(path) => {
            AstDecl::Namespace(AstNamespaceDecl::Declarative {
                path: path.into_iter().map(|(s, _)| s.to_string()).collect(),
                span: ns_span,
            })
        }
        NamespaceDecl::Block(path, items) => {
            let mut decls = Vec::new();
            for (item, _item_span) in items {
                match item {
                    Item::Fn((fn_decl, fn_span)) => {
                        decls.push(AstDecl::Fn(lower_fn(fn_decl, fn_span, ctx)));
                    }
                    Item::Namespace((inner_ns, inner_ns_span)) => {
                        decls.push(lower_namespace(inner_ns, inner_ns_span, ctx));
                    }
                    Item::Using((u, u_span)) => {
                        decls.push(lower_using(u, u_span));
                    }
                    Item::Struct((s, s_span)) => {
                        decls.push(AstDecl::Struct(lower_struct(s, s_span, ctx)));
                    }
                    Item::Enum((e, e_span)) => {
                        decls.push(AstDecl::Enum(lower_enum(e, e_span, ctx)));
                    }
                    Item::Contract((c, c_span)) => {
                        decls.push(AstDecl::Contract(lower_contract(c, c_span, ctx)));
                    }
                    Item::Impl((i, i_span)) => {
                        decls.extend(lower_operator_impls(i, i_span, ctx));
                    }
                    Item::Component((c, c_span)) => {
                        decls.push(AstDecl::Component(lower_component(c, c_span, ctx)));
                    }
                    Item::Extern((e, e_span)) => {
                        decls.push(AstDecl::Extern(lower_extern(e, e_span, ctx)));
                    }
                    Item::Const((c, c_span)) => {
                        decls.push(AstDecl::Const(lower_const(c, c_span, ctx)));
                    }
                    Item::Global((g, g_span)) => {
                        decls.push(AstDecl::Global(lower_global(g, g_span, ctx)));
                    }
                    Item::Stmt((s, s_span)) => {
                        decls.push(AstDecl::Stmt(lower_stmt((s, s_span), ctx)));
                    }
                    Item::Dlg((dlg_decl, dlg_span)) => {
                        decls.push(AstDecl::Fn(lower_dialogue(dlg_decl, dlg_span, ctx)));
                    }
                    Item::Entity((e, e_span)) => {
                        decls.extend(lower_entity(e, e_span, ctx));
                    }
                }
            }
            AstDecl::Namespace(AstNamespaceDecl::Block {
                path: path.into_iter().map(|(s, _)| s.to_string()).collect(),
                items: decls,
                span: ns_span,
            })
        }
    }
}

fn lower_using(u: UsingDecl<'_>, u_span: SimpleSpan) -> AstDecl {
    AstDecl::Using(AstUsingDecl {
        alias: u.alias.map(|(s, _)| s.to_string()),
        path: u.path.into_iter().map(|(s, _)| s.to_string()).collect(),
        span: u_span,
    })
}

fn lower_struct(s: StructDecl<'_>, s_span: SimpleSpan, ctx: &mut LoweringContext) -> AstStructDecl {
    AstStructDecl {
        attrs: lower_attrs(s.attrs, ctx),
        vis: lower_vis(s.vis),
        name: s.name.0.to_string(),
        name_span: s.name.1,
        generics: s
            .generics
            .unwrap_or_default()
            .into_iter()
            .map(|(gp, gp_span)| lower_generic_param(gp, gp_span))
            .collect(),
        fields: s
            .fields
            .into_iter()
            .map(|(field, field_span)| lower_struct_field(field, field_span, ctx))
            .collect(),
        span: s_span,
    }
}

fn lower_enum(e: EnumDecl<'_>, e_span: SimpleSpan, ctx: &mut LoweringContext) -> AstEnumDecl {
    AstEnumDecl {
        attrs: lower_attrs(e.attrs, ctx),
        vis: lower_vis(e.vis),
        name: e.name.0.to_string(),
        name_span: e.name.1,
        generics: e
            .generics
            .unwrap_or_default()
            .into_iter()
            .map(|(gp, gp_span)| lower_generic_param(gp, gp_span))
            .collect(),
        variants: e
            .variants
            .into_iter()
            .map(|(variant, variant_span)| lower_enum_variant(variant, variant_span))
            .collect(),
        span: e_span,
    }
}

fn lower_enum_variant(variant: EnumVariant<'_>, variant_span: SimpleSpan) -> AstEnumVariant {
    AstEnumVariant {
        name: variant.name.0.to_string(),
        name_span: variant.name.1,
        fields: variant.fields.map(|fields| {
            fields
                .into_iter()
                .map(|(param, param_span)| lower_param(param, param_span))
                .collect()
        }),
        span: variant_span,
    }
}

fn lower_contract(
    c: ContractDecl<'_>,
    c_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstContractDecl {
    AstContractDecl {
        attrs: lower_attrs(c.attrs, ctx),
        vis: lower_vis(c.vis),
        name: c.name.0.to_string(),
        name_span: c.name.1,
        generics: c
            .generics
            .unwrap_or_default()
            .into_iter()
            .map(|(gp, gp_span)| lower_generic_param(gp, gp_span))
            .collect(),
        members: c
            .members
            .into_iter()
            .map(|(member, member_span)| match member {
                ContractMember::FnSig(sig) => {
                    AstContractMember::FnSig(lower_fn_sig(sig, member_span, ctx))
                }
                ContractMember::OpSig(op) => {
                    AstContractMember::OpSig(lower_op_sig(op, member_span))
                }
            })
            .collect(),
        span: c_span,
    }
}

fn lower_component(
    c: ComponentDecl<'_>,
    c_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstComponentDecl {
    AstComponentDecl {
        attrs: lower_attrs(c.attrs, ctx),
        vis: lower_vis(c.vis),
        name: c.name.0.to_string(),
        name_span: c.name.1,
        members: c
            .members
            .into_iter()
            .map(|(member, _member_span)| match member {
                ComponentMember::Field((field, field_span)) => {
                    AstComponentMember::Field(lower_struct_field(field, field_span, ctx))
                }
                ComponentMember::Fn((fn_decl, fn_span)) => {
                    AstComponentMember::Fn(lower_fn(fn_decl, fn_span, ctx))
                }
            })
            .collect(),
        span: c_span,
    }
}

fn lower_extern(e: ExternDecl<'_>, _e_span: SimpleSpan, ctx: &mut LoweringContext) -> AstExternDecl {
    match e {
        ExternDecl::Fn((sig, sig_span)) => AstExternDecl::Fn(lower_fn_sig(sig, sig_span, ctx)),
        ExternDecl::Struct((s, s_span)) => AstExternDecl::Struct(lower_struct(s, s_span, ctx)),
        ExternDecl::Component((c, c_span)) => {
            AstExternDecl::Component(lower_component(c, c_span, ctx))
        }
    }
}

fn lower_const(c: ConstDecl<'_>, c_span: SimpleSpan, ctx: &mut LoweringContext) -> AstConstDecl {
    AstConstDecl {
        attrs: lower_attrs(c.attrs, ctx),
        vis: lower_vis(c.vis),
        name: c.name.0.to_string(),
        name_span: c.name.1,
        ty: lower_type(c.ty),
        value: lower_expr(c.value, ctx),
        span: c_span,
    }
}

fn lower_global(g: GlobalDecl<'_>, g_span: SimpleSpan, ctx: &mut LoweringContext) -> AstGlobalDecl {
    AstGlobalDecl {
        attrs: lower_attrs(g.attrs, ctx),
        vis: lower_vis(g.vis),
        name: g.name.0.to_string(),
        name_span: g.name.1,
        ty: lower_type(g.ty),
        value: lower_expr(g.value, ctx),
        span: g_span,
    }
}
