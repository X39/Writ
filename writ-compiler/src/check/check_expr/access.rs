//! Member access and bracket (index/component) access type checking.

use chumsky::span::SimpleSpan;

use crate::ast::expr::AstExpr;
use crate::resolve::def_map::DefKind;
use super::CheckCtx;
use super::check_expr;
use super::super::error::TypeError;
use super::super::ir::TypedExpr;
use super::super::ty::TyKind;

pub(super) fn check_member_access(
    ctx: &mut CheckCtx,
    object: &AstExpr,
    field: &str,
    field_span: SimpleSpan,
    span: SimpleSpan,
) -> TypedExpr {
    let typed_obj = check_expr(ctx, object);
    let obj_ty = typed_obj.ty();

    // Poison propagation
    if ctx.is_error(obj_ty) {
        return TypedExpr::Field {
            ty: ctx.interner.error(),
            span,
            receiver: Box::new(typed_obj),
            field: field.to_string(),
        };
    }

    let kind = ctx.interner.kind(obj_ty).clone();
    match kind {
        TyKind::Struct(def_id) | TyKind::Class(def_id) | TyKind::Entity(def_id) => {
            // Look up in struct_fields or entity_fields
            let fields = if matches!(kind, TyKind::Struct(_) | TyKind::Class(_)) {
                ctx.type_env.struct_fields.get(&def_id)
            } else {
                ctx.type_env.entity_fields.get(&def_id)
            };

            if let Some(field_list) = fields {
                for (fname, fty, _fspan) in field_list {
                    if fname == field {
                        return TypedExpr::Field {
                            ty: *fty,
                            span,
                            receiver: Box::new(typed_obj),
                            field: field.to_string(),
                        };
                    }
                }
            }

            // Check impl_index for methods
            if let Some(impls) = ctx.type_env.impl_index.get(&def_id) {
                for impl_entry in impls {
                    for (method_name, method_sig) in &impl_entry.methods {
                        if method_name == field {
                            // Method access: build a Func type for the method
                            let param_tys: Vec<_> = method_sig.params.iter().map(|(_, t)| *t).collect();
                            let fn_ty = ctx.interner.func(param_tys, method_sig.ret);
                            return TypedExpr::Field {
                                ty: fn_ty,
                                span,
                                receiver: Box::new(typed_obj),
                                field: field.to_string(),
                            };
                        }
                    }
                }
            }

            // Not found
            let ty_name = ctx.display_ty(obj_ty);
            let err_ty = ctx.emit_error(TypeError::UnknownField {
                ty_name,
                field_name: field.to_string(),
                span: field_span,
                file: ctx.current_file,
            });
            TypedExpr::Field {
                ty: err_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::Contract(contract_def_id) => {
            // Contract types have no fields — only methods from the contract definition.
            if let Some(methods) = ctx.type_env.contract_methods.get(&contract_def_id) {
                for method_sig in methods {
                    if method_sig.name == field {
                        // Build Func type from the method signature (excluding self param).
                        let param_tys: Vec<_> = method_sig.params.iter().map(|(_, t)| *t).collect();
                        let fn_ty = ctx.interner.func(param_tys, method_sig.ret);
                        return TypedExpr::Field {
                            ty: fn_ty,
                            span,
                            receiver: Box::new(typed_obj),
                            field: field.to_string(),
                        };
                    }
                }
            }

            // Method not found on contract
            let contract_name = ctx.def_map.get_entry(contract_def_id).name.clone();
            let err_ty = ctx.emit_error(TypeError::UnknownField {
                ty_name: contract_name,
                field_name: field.to_string(),
                span: field_span,
                file: ctx.current_file,
            });
            TypedExpr::Field {
                ty: err_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::Option(inner_ty) => {
            let bool_ty = ctx.interner.bool_ty();
            let fn_ty = match field {
                "is_some" | "is_none" => ctx.interner.func(vec![], bool_ty),
                "unwrap" => ctx.interner.func(vec![], inner_ty),
                _ => {
                    let ty_name = ctx.display_ty(obj_ty);
                    let err_ty = ctx.emit_error(TypeError::UnknownField {
                        ty_name,
                        field_name: field.to_string(),
                        span: field_span,
                        file: ctx.current_file,
                    });
                    return TypedExpr::Field {
                        ty: err_ty,
                        span,
                        receiver: Box::new(typed_obj),
                        field: field.to_string(),
                    };
                }
            };
            TypedExpr::Field {
                ty: fn_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::Result(ok_ty, err_ty_inner) => {
            let bool_ty = ctx.interner.bool_ty();
            let fn_ty = match field {
                "is_ok" | "is_err" => ctx.interner.func(vec![], bool_ty),
                "unwrap" => ctx.interner.func(vec![], ok_ty),
                "unwrap_err" => ctx.interner.func(vec![], err_ty_inner),
                _ => {
                    let ty_name = ctx.display_ty(obj_ty);
                    let err_ty = ctx.emit_error(TypeError::UnknownField {
                        ty_name,
                        field_name: field.to_string(),
                        span: field_span,
                        file: ctx.current_file,
                    });
                    return TypedExpr::Field {
                        ty: err_ty,
                        span,
                        receiver: Box::new(typed_obj),
                        field: field.to_string(),
                    };
                }
            };
            TypedExpr::Field {
                ty: fn_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::Array(elem_ty) => {
            let void_ty = ctx.interner.intern(TyKind::Void);
            let int_ty = ctx.interner.int();
            let fn_ty = match field {
                "len" => ctx.interner.func(vec![], int_ty),
                "slice" => {
                    let arr_ty = ctx.interner.intern(TyKind::Array(elem_ty));
                    ctx.interner.func(vec![int_ty, int_ty], arr_ty)
                }
                "resize" => ctx.interner.func(vec![int_ty], void_ty),
                "copy_from" => {
                    // copy_from(src: T[], src_idx: int, dst_idx: int, len: int) -> void
                    let arr_ty = ctx.interner.intern(TyKind::Array(elem_ty));
                    ctx.interner.func(vec![arr_ty, int_ty, int_ty, int_ty], void_ty)
                }
                _ => {
                    let ty_name = ctx.display_ty(obj_ty);
                    let err_ty = ctx.emit_error(TypeError::UnknownField {
                        ty_name,
                        field_name: field.to_string(),
                        span: field_span,
                        file: ctx.current_file,
                    });
                    return TypedExpr::Field {
                        ty: err_ty,
                        span,
                        receiver: Box::new(typed_obj),
                        field: field.to_string(),
                    };
                }
            };
            TypedExpr::Field {
                ty: fn_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::AnyEntity => {
            // Entity namespace static methods: getOrCreate, destroy, isAlive, findAll
            let entity_ty = ctx.interner.any_entity();
            let void_ty = ctx.interner.void();
            let bool_ty = ctx.interner.bool_ty();
            let fn_ty = match field {
                "getOrCreate" => {
                    // Generic: fn<T>() -> T — the return type is resolved at the call site
                    // via check_generic_call. For member access, return fn() -> Entity.
                    ctx.interner.func(vec![], entity_ty)
                }
                "destroy" => ctx.interner.func(vec![entity_ty], void_ty),
                "isAlive" => ctx.interner.func(vec![entity_ty], bool_ty),
                "findAll" => {
                    // Generic: fn<T>() -> EntityList<T> — simplified as fn() -> Entity[]
                    let arr_ty = ctx.interner.array(entity_ty);
                    ctx.interner.func(vec![], arr_ty)
                }
                _ => {
                    let ty_name = ctx.display_ty(obj_ty);
                    let err_ty = ctx.emit_error(TypeError::UnknownField {
                        ty_name,
                        field_name: field.to_string(),
                        span: field_span,
                        file: ctx.current_file,
                    });
                    return TypedExpr::Field {
                        ty: err_ty,
                        span,
                        receiver: Box::new(typed_obj),
                        field: field.to_string(),
                    };
                }
            };
            TypedExpr::Field {
                ty: fn_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::Enum(def_id) => {
            // Check for associated methods via impl_index
            if let Some(impls) = ctx.type_env.impl_index.get(&def_id) {
                for impl_entry in impls {
                    for (method_name, method_sig) in &impl_entry.methods {
                        if method_name == field {
                            let param_tys: Vec<_> = method_sig.params.iter().map(|(_, t)| *t).collect();
                            let fn_ty = ctx.interner.func(param_tys, method_sig.ret);
                            return TypedExpr::Field {
                                ty: fn_ty,
                                span,
                                receiver: Box::new(typed_obj),
                                field: field.to_string(),
                            };
                        }
                    }
                }
            }

            let ty_name = ctx.display_ty(obj_ty);
            let err_ty = ctx.emit_error(TypeError::UnknownField {
                ty_name,
                field_name: field.to_string(),
                span: field_span,
                file: ctx.current_file,
            });
            TypedExpr::Field {
                ty: err_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::String => {
            let int_ty = ctx.interner.int();
            let string_ty = ctx.interner.string_ty();
            let fn_ty = match field {
                "len" => ctx.interner.func(vec![], int_ty),
                "into_string" => ctx.interner.func(vec![], string_ty),
                "into_int" => ctx.interner.func(vec![], int_ty),
                "into_float" => {
                    let float_ty = ctx.interner.intern(TyKind::Float);
                    ctx.interner.func(vec![], float_ty)
                }
                "into_bool" => {
                    let bool_ty = ctx.interner.bool_ty();
                    ctx.interner.func(vec![], bool_ty)
                }
                "trim" => ctx.interner.func(vec![], string_ty),
                "to_upper" => ctx.interner.func(vec![], string_ty),
                "to_lower" => ctx.interner.func(vec![], string_ty),
                "starts_with" => {
                    let bool_ty = ctx.interner.bool_ty();
                    ctx.interner.func(vec![string_ty], bool_ty)
                }
                "ends_with" => {
                    let bool_ty = ctx.interner.bool_ty();
                    ctx.interner.func(vec![string_ty], bool_ty)
                }
                "contains" => {
                    let bool_ty = ctx.interner.bool_ty();
                    ctx.interner.func(vec![string_ty], bool_ty)
                }
                "replace" => ctx.interner.func(vec![string_ty, string_ty], string_ty),
                "split" => {
                    let arr_string_ty = ctx.interner.intern(TyKind::Array(string_ty));
                    ctx.interner.func(vec![string_ty], arr_string_ty)
                }
                _ => {
                    let ty_name = ctx.display_ty(obj_ty);
                    let err_ty = ctx.emit_error(TypeError::UnknownField {
                        ty_name,
                        field_name: field.to_string(),
                        span: field_span,
                        file: ctx.current_file,
                    });
                    return TypedExpr::Field {
                        ty: err_ty,
                        span,
                        receiver: Box::new(typed_obj),
                        field: field.to_string(),
                    };
                }
            };
            TypedExpr::Field {
                ty: fn_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::Int => {
            let string_ty = ctx.interner.string_ty();
            let float_ty = ctx.interner.intern(TyKind::Float);
            let fn_ty = match field {
                "into_string" => ctx.interner.func(vec![], string_ty),
                "into_float" => ctx.interner.func(vec![], float_ty),
                _ => {
                    let ty_name = ctx.display_ty(obj_ty);
                    let err_ty = ctx.emit_error(TypeError::UnknownField {
                        ty_name,
                        field_name: field.to_string(),
                        span: field_span,
                        file: ctx.current_file,
                    });
                    return TypedExpr::Field {
                        ty: err_ty,
                        span,
                        receiver: Box::new(typed_obj),
                        field: field.to_string(),
                    };
                }
            };
            TypedExpr::Field {
                ty: fn_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::Float => {
            let string_ty = ctx.interner.string_ty();
            let int_ty = ctx.interner.int();
            let fn_ty = match field {
                "into_string" => ctx.interner.func(vec![], string_ty),
                "into_int" => ctx.interner.func(vec![], int_ty),
                _ => {
                    let ty_name = ctx.display_ty(obj_ty);
                    let err_ty = ctx.emit_error(TypeError::UnknownField {
                        ty_name,
                        field_name: field.to_string(),
                        span: field_span,
                        file: ctx.current_file,
                    });
                    return TypedExpr::Field {
                        ty: err_ty,
                        span,
                        receiver: Box::new(typed_obj),
                        field: field.to_string(),
                    };
                }
            };
            TypedExpr::Field {
                ty: fn_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        TyKind::Bool => {
            let string_ty = ctx.interner.string_ty();
            let fn_ty = match field {
                "into_string" => ctx.interner.func(vec![], string_ty),
                _ => {
                    let ty_name = ctx.display_ty(obj_ty);
                    let err_ty = ctx.emit_error(TypeError::UnknownField {
                        ty_name,
                        field_name: field.to_string(),
                        span: field_span,
                        file: ctx.current_file,
                    });
                    return TypedExpr::Field {
                        ty: err_ty,
                        span,
                        receiver: Box::new(typed_obj),
                        field: field.to_string(),
                    };
                }
            };
            TypedExpr::Field {
                ty: fn_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
        _ => {
            let ty_name = ctx.display_ty(obj_ty);
            let err_ty = ctx.emit_error(TypeError::UnknownField {
                ty_name,
                field_name: field.to_string(),
                span: field_span,
                file: ctx.current_file,
            });
            TypedExpr::Field {
                ty: err_ty,
                span,
                receiver: Box::new(typed_obj),
                field: field.to_string(),
            }
        }
    }
}

pub(super) fn check_bracket_access(
    ctx: &mut CheckCtx,
    object: &AstExpr,
    index: &AstExpr,
    span: SimpleSpan,
) -> TypedExpr {
    let typed_obj = check_expr(ctx, object);
    let obj_ty = typed_obj.ty();

    // Poison propagation
    if ctx.is_error(obj_ty) {
        let typed_index = check_expr(ctx, index);
        return TypedExpr::Index {
            ty: ctx.interner.error(),
            span,
            receiver: Box::new(typed_obj),
            index: Box::new(typed_index),
        };
    }

    let kind = ctx.interner.kind(obj_ty).clone();
    match kind {
        TyKind::Array(elem_ty) => {
            // Array indexing: index must be int
            let typed_index = check_expr(ctx, index);
            let index_ty = typed_index.ty();
            let int_ty = ctx.interner.int();

            if !ctx.is_error(index_ty) && index_ty != int_ty {
                ctx.emit_error(TypeError::TypeMismatch {
                    expected: "int".to_string(),
                    found: ctx.display_ty(index_ty),
                    expected_span: span,
                    found_span: typed_index.span(),
                    file: ctx.current_file,
                    help: Some("array index must be int".to_string()),
                });
            }

            TypedExpr::Index {
                ty: elem_ty,
                span,
                receiver: Box::new(typed_obj),
                index: Box::new(typed_index),
            }
        }
        TyKind::Entity(def_id) => {
            // Component access: entity[ComponentName]
            // The index should be an identifier naming the component
            let component_name = match index {
                AstExpr::Ident { name, .. } => Some(name.as_str()),
                AstExpr::Path { segments, .. } if segments.len() == 1 => Some(segments[0].as_str()),
                _ => None,
            };

            if let Some(comp_name) = component_name {
                // Check if the entity has this component declared (guaranteed access)
                let has_component = ctx.type_env.entity_components
                    .get(&def_id)
                    .map(|comps| comps.iter().any(|c| c == comp_name))
                    .unwrap_or(false);

                // Look up the component's type by searching for its DefId
                let comp_def_id = ctx.def_map.get(comp_name);
                let comp_ty = comp_def_id.and_then(|did| {
                    let entry = ctx.def_map.get_entry(did);
                    if matches!(entry.kind, DefKind::Component | DefKind::ExternComponent) {
                        // Components are struct-like data containers; use TyKind::Struct for now
                        Some(ctx.interner.intern(TyKind::Struct(did)))
                    } else {
                        None
                    }
                });

                if let Some(component_ty) = comp_ty {
                    let _typed_index = check_expr(ctx, index);
                    if has_component {
                        // Guaranteed access: return the component type directly
                        TypedExpr::ComponentAccess {
                            ty: component_ty,
                            span,
                            receiver: Box::new(typed_obj),
                            component: comp_name.to_string(),
                        }
                    } else {
                        // Optional access: wrap in Option
                        let opt_ty = ctx.interner.option(component_ty);
                        TypedExpr::ComponentAccess {
                            ty: opt_ty,
                            span,
                            receiver: Box::new(typed_obj),
                            component: comp_name.to_string(),
                        }
                    }
                } else {
                    let _typed_index = check_expr(ctx, index);
                    TypedExpr::ComponentAccess {
                        ty: ctx.interner.error(),
                        span,
                        receiver: Box::new(typed_obj),
                        component: comp_name.to_string(),
                    }
                }
            } else {
                // Not a component access identifier
                let typed_index = check_expr(ctx, index);
                TypedExpr::Index {
                    ty: ctx.interner.error(),
                    span,
                    receiver: Box::new(typed_obj),
                    index: Box::new(typed_index),
                }
            }
        }
        _ => {
            // Not indexable
            let typed_index = check_expr(ctx, index);
            let ty_name = ctx.display_ty(obj_ty);
            let err_ty = ctx.emit_error(TypeError::TypeMismatch {
                expected: "array or entity".to_string(),
                found: ty_name,
                expected_span: span,
                found_span: typed_obj.span(),
                file: ctx.current_file,
                help: Some("bracket access requires an array or entity type".to_string()),
            });
            TypedExpr::Index {
                ty: err_ty,
                span,
                receiver: Box::new(typed_obj),
                index: Box::new(typed_index),
            }
        }
    }
}
