use chumsky::span::SimpleSpan;
use writ_parser::cst::{ImplDecl, ImplMember, OpDecl, OpSymbol};
use crate::ast::AstDecl;
use crate::ast::decl::{
    AstFnDecl, AstImplDecl, AstImplMember, AstParam,
};
use crate::ast::expr::{AstExpr, BinaryOp, PrefixOp};
use crate::ast::stmt::AstStmt;
use crate::ast::types::AstType;
use crate::lower::context::LoweringContext;
use crate::lower::optional::lower_type;
use crate::lower::stmt::lower_stmt;
use super::{lower_fn, lower_param, lower_vis};

/// Lowers an `impl` block's members by:
/// 1. Keeping regular `Fn` members in the base impl (if any exist, or if the impl has a contract).
/// 2. Extracting each `Op` member and re-emitting it as a standalone contract impl.
/// 3. Generating derived operators from `Eq` and `Ord` (Lt) implementations.
///
/// This ensures that `AstImplMember::Op` does not appear in the lowered output —
/// downstream phases only see contract impls, never raw operator declarations.
pub fn lower_operator_impls(
    i: ImplDecl<'_>,
    i_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> Vec<AstDecl> {
    let target_type = lower_type(i.target);
    let contract_type = i.contract.map(lower_type);

    let mut fn_members: Vec<AstImplMember> = Vec::new();
    let mut operator_decls: Vec<AstDecl> = Vec::new();
    let mut has_eq = false;
    let mut has_ord = false;
    let mut eq_param_type: Option<AstType> = None;
    let mut ord_param_type: Option<AstType> = None;

    for (member, _member_span) in i.members {
        match member {
            ImplMember::Fn((fn_decl, fn_span)) => {
                fn_members.push(AstImplMember::Fn(lower_fn(fn_decl, fn_span, ctx)));
            }
            ImplMember::Op((op_decl, op_span)) => {
                // Track Eq and Ord for derived operator generation
                let symbol = &op_decl.symbol.0;
                match symbol {
                    OpSymbol::Eq => {
                        has_eq = true;
                        // Capture the param type for derived Ne impl
                        if let Some((param, param_span)) = op_decl.params.first() {
                            eq_param_type = Some(lower_type(param.ty.clone()).into());
                            let _ = param_span; // suppress unused warning
                        }
                    }
                    OpSymbol::Lt => {
                        has_ord = true;
                        // Capture the param type for derived Gt/LtEq/GtEq impls
                        if let Some((param, param_span)) = op_decl.params.first() {
                            ord_param_type = Some(lower_type(param.ty.clone()).into());
                            let _ = param_span; // suppress unused warning
                        }
                    }
                    _ => {}
                }

                let contract_impl = op_to_contract_impl(&op_decl, op_span, &target_type, ctx);
                operator_decls.push(contract_impl);
            }
        }
    }

    let mut result: Vec<AstDecl> = Vec::new();

    // Emit base impl only if there are fn members OR the impl has an explicit contract.
    // Do NOT emit an empty base impl for operator-only impls without a contract.
    if !fn_members.is_empty() || contract_type.is_some() {
        result.push(AstDecl::Impl(AstImplDecl {
            contract: contract_type,
            target: target_type.clone(),
            members: fn_members,
            span: i_span,
        }));
    }

    result.extend(operator_decls);

    // Generate derived operators from Eq and Ord
    let derived = generate_derived_operators(
        has_eq,
        has_ord,
        eq_param_type.as_ref(),
        ord_param_type.as_ref(),
        &target_type,
        i_span,
    );
    result.extend(derived);

    result
}

/// Maps a single `OpDecl` to an `AstDecl::Impl` with the appropriate contract.
///
/// The operator body is lifted into an `AstFnDecl` whose name comes from the
/// operator-to-method mapping table. The contract type is constructed from the
/// operator symbol and parameter types.
fn op_to_contract_impl(
    op_decl: &OpDecl<'_>,
    op_span: SimpleSpan,
    target_type: &AstType,
    ctx: &mut LoweringContext,
) -> AstDecl {
    let (contract_name, method_name, type_args) =
        op_symbol_to_contract(&op_decl.symbol.0, op_decl, op_span);

    let contract_type = AstType::Generic {
        name: contract_name,
        args: type_args,
        span: op_span,
    };

    // Lower parameters
    let params: Vec<AstParam> = op_decl
        .params
        .iter()
        .map(|(param, param_span)| lower_param(param.clone(), *param_span))
        .collect();

    // Lower return type
    let return_type = op_decl.return_type.clone().map(lower_type);

    // Lower body statements
    let body: Vec<AstStmt> = op_decl
        .body
        .iter()
        .map(|s| lower_stmt(s.clone(), ctx))
        .collect();

    let op_fn = AstFnDecl {
        attrs: vec![],
        vis: lower_vis(op_decl.vis.clone()),
        name: method_name,
        name_span: op_decl.symbol.1,
        generics: vec![],
        params,
        return_type,
        body,
        span: op_span,
    };

    AstDecl::Impl(AstImplDecl {
        contract: Some(contract_type),
        target: target_type.clone(),
        members: vec![AstImplMember::Fn(op_fn)],
        span: op_span,
    })
}

/// Maps an `OpSymbol` to `(contract_name, method_name, type_args)`.
///
/// The type args are constructed by calling `lower_type` on the relevant
/// parameter types from `op_decl.params`.
///
/// IMPORTANT: `Sub` is disambiguated by param count:
/// - 0 params → unary `Neg`
/// - 1 param → binary `Sub`
///
/// No `_ =>` wildcard — all variants are handled explicitly.
fn op_symbol_to_contract(
    symbol: &OpSymbol,
    op_decl: &OpDecl<'_>,
    op_span: SimpleSpan,
) -> (String, String, Vec<AstType>) {
    // Helper to get the lowered type of param at index i
    let param_type = |i: usize| -> AstType {
        lower_type(op_decl.params[i].0.ty.clone())
    };

    // Helper to get the lowered return type
    let return_type = || -> AstType {
        op_decl
            .return_type
            .clone()
            .map(lower_type)
            .unwrap_or_else(|| AstType::Void { span: op_span })
    };

    match symbol {
        OpSymbol::Add => (
            "Add".to_string(),
            "add".to_string(),
            vec![param_type(0), return_type()],
        ),
        OpSymbol::Sub if op_decl.params.is_empty() => (
            // Unary Sub → Neg
            "Neg".to_string(),
            "neg".to_string(),
            vec![return_type()],
        ),
        OpSymbol::Sub => (
            // Binary Sub
            "Sub".to_string(),
            "sub".to_string(),
            vec![param_type(0), return_type()],
        ),
        OpSymbol::Mul => (
            "Mul".to_string(),
            "mul".to_string(),
            vec![param_type(0), return_type()],
        ),
        OpSymbol::Div => (
            "Div".to_string(),
            "div".to_string(),
            vec![param_type(0), return_type()],
        ),
        OpSymbol::Mod => (
            "Mod".to_string(),
            "mod_op".to_string(),
            vec![param_type(0), return_type()],
        ),
        OpSymbol::Eq => (
            // Return type bool is implied by the contract
            "Eq".to_string(),
            "eq".to_string(),
            vec![param_type(0)],
        ),
        OpSymbol::Lt => (
            // Return type bool is implied by the contract
            "Ord".to_string(),
            "lt".to_string(),
            vec![param_type(0)],
        ),
        OpSymbol::Not => (
            "Not".to_string(),
            "not".to_string(),
            vec![return_type()],
        ),
        OpSymbol::Index => (
            "Index".to_string(),
            "index".to_string(),
            vec![param_type(0), return_type()],
        ),
        OpSymbol::IndexSet => (
            // Two type args: [index_param_type, value_param_type]
            "IndexMut".to_string(),
            "index_set".to_string(),
            vec![param_type(0), param_type(1)],
        ),
    }
}

/// Generates derived operator impls from `Eq` and `Ord` (Lt) implementations.
///
/// Rules:
/// - `has_eq` → emit `impl Ne for target { fn ne(other: T) -> bool { !(self == other) } }`
/// - `has_ord` → emit `impl Gt for target { fn gt(other: T) -> bool { other < self } }`
/// - `has_eq && has_ord` → emit `LtEq` and `GtEq` impls as well
///
/// All synthetic spans use `impl_span` — never `SimpleSpan::new(0, 0)`.
fn generate_derived_operators(
    has_eq: bool,
    has_ord: bool,
    eq_param_type: Option<&AstType>,
    ord_param_type: Option<&AstType>,
    target_type: &AstType,
    impl_span: SimpleSpan,
) -> Vec<AstDecl> {
    let mut derived: Vec<AstDecl> = Vec::new();

    let bool_type = || AstType::Named {
        name: "bool".to_string(),
        span: impl_span,
    };

    let self_expr = || AstExpr::SelfLit { span: impl_span };

    let other_expr = || AstExpr::Ident {
        name: "other".to_string(),
        span: impl_span,
    };

    let make_param = |ty: &AstType| AstParam {
        name: "other".to_string(),
        name_span: impl_span,
        ty: ty.clone(),
        span: impl_span,
    };

    let make_fn = |name: &str, param: AstParam, body: Vec<AstStmt>| AstFnDecl {
        attrs: vec![],
        vis: None,
        name: name.to_string(),
        name_span: impl_span,
        generics: vec![],
        params: vec![param],
        return_type: Some(bool_type()),
        body,
        span: impl_span,
    };

    let make_impl = |contract_name: &str, contract_arg: AstType, fn_decl: AstFnDecl| {
        AstDecl::Impl(AstImplDecl {
            contract: Some(AstType::Generic {
                name: contract_name.to_string(),
                args: vec![contract_arg],
                span: impl_span,
            }),
            target: target_type.clone(),
            members: vec![AstImplMember::Fn(fn_decl)],
            span: impl_span,
        })
    };

    // Ne: !(self == other)
    if has_eq {
        if let Some(param_ty) = eq_param_type {
            let body = vec![AstStmt::Return {
                value: Some(AstExpr::UnaryPrefix {
                    op: PrefixOp::Not,
                    expr: Box::new(AstExpr::Binary {
                        left: Box::new(self_expr()),
                        op: BinaryOp::Eq,
                        right: Box::new(other_expr()),
                        span: impl_span,
                    }),
                    span: impl_span,
                }),
                span: impl_span,
            }];
            let fn_decl = make_fn("ne", make_param(param_ty), body);
            derived.push(make_impl("Ne", param_ty.clone(), fn_decl));
        }
    }

    // Gt: other < self
    if has_ord {
        if let Some(param_ty) = ord_param_type {
            let body = vec![AstStmt::Return {
                value: Some(AstExpr::Binary {
                    left: Box::new(other_expr()),
                    op: BinaryOp::Lt,
                    right: Box::new(self_expr()),
                    span: impl_span,
                }),
                span: impl_span,
            }];
            let fn_decl = make_fn("gt", make_param(param_ty), body);
            derived.push(make_impl("Gt", param_ty.clone(), fn_decl));
        }
    }

    // LtEq and GtEq: only when both Eq and Ord are present
    if has_eq && has_ord {
        // Use eq_param_type for LtEq/GtEq (they have the same T for both operations)
        let param_ty = eq_param_type.or(ord_param_type);
        if let Some(param_ty) = param_ty {
            // LtEq: self < other || self == other
            let lt_eq_body = vec![AstStmt::Return {
                value: Some(AstExpr::Binary {
                    left: Box::new(AstExpr::Binary {
                        left: Box::new(self_expr()),
                        op: BinaryOp::Lt,
                        right: Box::new(other_expr()),
                        span: impl_span,
                    }),
                    op: BinaryOp::Or,
                    right: Box::new(AstExpr::Binary {
                        left: Box::new(self_expr()),
                        op: BinaryOp::Eq,
                        right: Box::new(other_expr()),
                        span: impl_span,
                    }),
                    span: impl_span,
                }),
                span: impl_span,
            }];
            let lt_eq_fn = make_fn("lt_eq", make_param(param_ty), lt_eq_body);
            derived.push(make_impl("LtEq", param_ty.clone(), lt_eq_fn));

            // GtEq: !(self < other)
            let gt_eq_body = vec![AstStmt::Return {
                value: Some(AstExpr::UnaryPrefix {
                    op: PrefixOp::Not,
                    expr: Box::new(AstExpr::Binary {
                        left: Box::new(self_expr()),
                        op: BinaryOp::Lt,
                        right: Box::new(other_expr()),
                        span: impl_span,
                    }),
                    span: impl_span,
                }),
                span: impl_span,
            }];
            let gt_eq_fn = make_fn("gt_eq", make_param(param_ty), gt_eq_body);
            derived.push(make_impl("GtEq", param_ty.clone(), gt_eq_fn));
        }
    }

    derived
}
