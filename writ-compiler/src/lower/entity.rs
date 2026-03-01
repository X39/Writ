use chumsky::span::SimpleSpan;
use writ_parser::cst::{
    EntityDecl, EntityMember, FnDecl, Param, Spanned, Stmt, UseField, Visibility,
};
use crate::ast::AstDecl;
use crate::ast::decl::{
    AstComponentSlot, AstEntityDecl, AstEntityHook, AstFnDecl, AstFnParam,
    AstImplDecl, AstImplMember, AstStructField,
};
use crate::ast::expr::AstExpr;
use crate::ast::stmt::AstStmt;
use crate::ast::types::AstType;
use crate::lower::context::LoweringContext;
use crate::lower::error::LoweringError;
use crate::lower::expr::lower_expr;
use crate::lower::optional::lower_type;
use crate::lower::stmt::lower_stmt;
use super::{lower_fn, lower_param, lower_vis, lower_attrs};

// =========================================================
// Intermediate types for partition_entity_members
// =========================================================

struct EntityProperty<'src> {
    vis: Option<Visibility>,
    name: Spanned<&'src str>,
    ty: Spanned<writ_parser::cst::TypeExpr<'src>>,
    default: Option<Spanned<writ_parser::cst::Expr<'src>>>,
    span: SimpleSpan,
}

struct EntityUseClause<'src> {
    component: Spanned<&'src str>,
    fields: Vec<Spanned<UseField<'src>>>,
    span: SimpleSpan,
}

struct EntityHook<'src> {
    event: Spanned<&'src str>,
    params: Option<Vec<Spanned<Param<'src>>>>,
    body: Vec<Spanned<Stmt<'src>>>,
    span: SimpleSpan,
}

struct PartitionedMembers<'src> {
    properties: Vec<EntityProperty<'src>>,
    use_clauses: Vec<EntityUseClause<'src>>,
    methods: Vec<(FnDecl<'src>, SimpleSpan)>,
    hooks: Vec<EntityHook<'src>>,
}

// =========================================================
// partition_entity_members
// =========================================================

/// Scans entity members into four typed buckets and validates for errors.
///
/// Validation rules (all accumulated, none halt processing):
/// - Duplicate property names → `LoweringError::DuplicateProperty`
/// - Duplicate use clauses (same component used twice) → `LoweringError::DuplicateUseClause`
/// - Property-component name collisions (in either direction) → `LoweringError::PropertyComponentCollision`
/// - Unknown lifecycle events → `LoweringError::UnknownLifecycleEvent`; hook is skipped
fn partition_entity_members<'src>(
    members: Vec<Spanned<EntityMember<'src>>>,
    entity_name: &str,
    ctx: &mut LoweringContext,
) -> PartitionedMembers<'src> {
    let mut properties: Vec<EntityProperty<'src>> = Vec::new();
    let mut use_clauses: Vec<EntityUseClause<'src>> = Vec::new();
    let mut methods: Vec<(FnDecl<'src>, SimpleSpan)> = Vec::new();
    let mut hooks: Vec<EntityHook<'src>> = Vec::new();

    // Track names for duplicate/collision detection
    let mut seen_props: Vec<String> = Vec::new();
    let mut seen_components: Vec<String> = Vec::new();

    for (member, member_span) in members {
        match member {
            EntityMember::Property { vis, name, ty, default } => {
                let prop_name = name.0.to_string();

                // Check duplicate property
                if seen_props.contains(&prop_name) {
                    ctx.emit_error(LoweringError::DuplicateProperty {
                        property: prop_name.clone(),
                        entity: entity_name.to_string(),
                        span: name.1,
                    });
                    continue;
                }

                // Check property-component collision (component was declared before this property)
                if seen_components.contains(&prop_name) {
                    ctx.emit_error(LoweringError::PropertyComponentCollision {
                        name: prop_name.clone(),
                        entity: entity_name.to_string(),
                        span: name.1,
                    });
                    continue;
                }

                seen_props.push(prop_name);
                properties.push(EntityProperty { vis, name, ty, default, span: member_span });
            }

            EntityMember::Use { component, fields } => {
                let comp_name = component.0.to_string();

                // Check duplicate use clause
                if seen_components.contains(&comp_name) {
                    ctx.emit_error(LoweringError::DuplicateUseClause {
                        component: comp_name.clone(),
                        entity: entity_name.to_string(),
                        span: component.1,
                    });
                    continue;
                }

                // Check property-component collision (property was declared before this use clause)
                if seen_props.contains(&comp_name) {
                    ctx.emit_error(LoweringError::PropertyComponentCollision {
                        name: comp_name.clone(),
                        entity: entity_name.to_string(),
                        span: component.1,
                    });
                    continue;
                }

                seen_components.push(comp_name);
                use_clauses.push(EntityUseClause { component, fields, span: member_span });
            }

            EntityMember::Fn((fn_decl, fn_span)) => {
                methods.push((fn_decl, fn_span));
            }

            EntityMember::On { event, params, body } => {
                let event_name = event.0;
                match event_name {
                    "create" | "interact" | "destroy" | "finalize" | "serialize" | "deserialize" => {
                        hooks.push(EntityHook { event, params, body, span: member_span });
                    }
                    _ => {
                        ctx.emit_error(LoweringError::UnknownLifecycleEvent {
                            event: event_name.to_string(),
                            span: event.1,
                        });
                        // Skip this hook — do not add to hooks vec
                    }
                }
            }
        }
    }

    PartitionedMembers { properties, use_clauses, methods, hooks }
}

// =========================================================
// lower_entity
// =========================================================

/// Lowers an `entity` declaration to `Vec<AstDecl>`.
///
/// Emits a single `AstDecl::Entity(AstEntityDecl)` containing:
/// - Properties (regular typed fields)
/// - Component slots (host-managed descriptors, not inline struct fields)
/// - Lifecycle hooks (all six: create, destroy, interact, finalize, serialize, deserialize)
///   with implicit `mut self` injected as first parameter
/// - Optional inherent impl for methods
pub fn lower_entity(
    entity: EntityDecl<'_>,
    entity_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> Vec<AstDecl> {
    let entity_name = entity.name.0.to_string();
    let entity_name_span = entity.name.1;

    // Step 1: Lower attrs and vis
    let attrs = lower_attrs(entity.attrs, ctx);
    let vis = lower_vis(entity.vis);

    // Step 2: Partition members with validation
    let partitioned = partition_entity_members(entity.members, &entity_name, ctx);

    // =========================================================
    // Properties: regular typed fields
    // =========================================================
    let properties: Vec<AstStructField> = partitioned.properties.iter()
        .map(|prop| {
            let lowered_ty = lower_type(prop.ty.clone());
            let lowered_default = prop.default.clone().map(|d| lower_expr(d, ctx));
            AstStructField {
                vis: lower_vis(prop.vis.clone()),
                name: prop.name.0.to_string(),
                name_span: prop.name.1,
                ty: lowered_ty,
                default: lowered_default,
                span: prop.span,
            }
        })
        .collect();

    // =========================================================
    // Component slots: host-managed descriptors (ENT-02)
    // =========================================================
    let component_slots: Vec<AstComponentSlot> = partitioned.use_clauses.iter()
        .map(|use_clause| {
            let comp_name = use_clause.component.0.to_string();
            let comp_span = use_clause.component.1;

            let overrides: Vec<(String, SimpleSpan, AstExpr)> = use_clause.fields.iter()
                .map(|(uf, _uf_span)| {
                    let field_name = uf.name.0.to_string();
                    let field_name_span = uf.name.1;
                    let value = lower_expr(uf.value.clone(), ctx);
                    (field_name, field_name_span, value)
                })
                .collect();

            AstComponentSlot {
                component: comp_name,
                component_span: comp_span,
                overrides,
                span: use_clause.span,
            }
        })
        .collect();

    // =========================================================
    // Hooks: lifecycle event registrations (ENT-01, ENT-03)
    // All hooks get implicit `mut self` as first parameter
    // =========================================================
    let hooks: Vec<AstEntityHook> = partitioned.hooks.into_iter()
        .map(|hook| {
            let event_name = hook.event.0;
            let event_span = hook.event.1;

            let (contract_name, method_name) = match event_name {
                "create"      => ("OnCreate",      "on_create"),
                "interact"    => ("OnInteract",    "on_interact"),
                "destroy"     => ("OnDestroy",     "on_destroy"),
                "finalize"    => ("OnFinalize",    "on_finalize"),
                "serialize"   => ("OnSerialize",   "on_serialize"),
                "deserialize" => ("OnDeserialize", "on_deserialize"),
                // Unreachable: partition_entity_members already filtered unknown events
                _ => unreachable!("unknown lifecycle event passed validation: {}", event_name),
            };

            // Build params with implicit mut self injection (ENT-03, spec §14.6)
            let mut params: Vec<AstFnParam> = Vec::new();
            // Inject implicit mut self as first parameter
            params.push(AstFnParam::SelfParam { mutable: true, span: event_span });
            // Then add any explicit params (e.g., `who: Entity` for interact)
            if let Some(explicit_params) = hook.params {
                for (param, param_span) in explicit_params {
                    params.push(AstFnParam::Regular(lower_param(param, param_span)));
                }
            }

            // Lower hook body
            let body: Vec<AstStmt> = hook.body
                .into_iter()
                .map(|s| lower_stmt(s, ctx))
                .collect();

            let method = AstFnDecl {
                attrs: vec![],
                vis: None,
                name: method_name.to_string(),
                name_span: event_span,
                generics: vec![],
                params,
                return_type: None,
                body,
                span: hook.span,
            };

            AstEntityHook {
                contract: contract_name.to_string(),
                contract_span: event_span,
                method,
                span: hook.span,
            }
        })
        .collect();

    // =========================================================
    // Inherent impl: methods (if any)
    // =========================================================
    let inherent_impl = if !partitioned.methods.is_empty() {
        let members: Vec<AstImplMember> = partitioned.methods
            .into_iter()
            .map(|(fn_decl, fn_span)| AstImplMember::Fn(lower_fn(fn_decl, fn_span, ctx)))
            .collect();

        Some(AstImplDecl {
            generics: vec![],
            contract: None,
            target: AstType::Named { name: entity_name.clone(), span: entity_name_span },
            members,
            span: entity_span,
        })
    } else {
        None
    };

    // =========================================================
    // Emit single AstDecl::Entity (ENT-04)
    // =========================================================
    vec![AstDecl::Entity(AstEntityDecl {
        attrs,
        vis,
        name: entity_name,
        name_span: entity_name_span,
        properties,
        component_slots,
        hooks,
        inherent_impl,
        span: entity_span,
    })]
}
