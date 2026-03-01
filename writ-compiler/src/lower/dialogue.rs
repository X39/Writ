use std::collections::HashMap;
use chumsky::span::SimpleSpan;
use writ_parser::cst::{
    DlgChoice, DlgDecl, DlgEscape, DlgIf, DlgElse, DlgLine, DlgMatch,
    DlgTextSegment, DlgTransition, Spanned,
};
use crate::ast::decl::{AstAttributeArg, AstFnDecl, AstFnParam};
use crate::ast::expr::{AstArg, AstExpr, AstMatchArm, BinaryOp};
use crate::ast::stmt::AstStmt;
use crate::ast::types::AstType;
use crate::lower::context::{LoweringContext, SpeakerScope};
use crate::lower::error::LoweringError;
use crate::lower::expr::{lower_expr, lower_pattern};
use crate::lower::stmt::lower_stmt;
use super::{lower_attrs, lower_param, lower_vis};

// =========================================================
// Private state for a single dlg lowering session
// =========================================================

struct DlgLowerState {
    dlg_name: String,
    namespace: String,
    param_names: Vec<String>,
    /// Maps (namespace, method, speaker, content) → occurrence count
    occurrence_tracker: HashMap<(String, String, String, String), u32>,
    /// Maps manual #key string → span of first occurrence
    manual_keys: HashMap<String, SimpleSpan>,
}

// =========================================================
// Public entry point
// =========================================================

/// Lowers a CST `DlgDecl` to an `AstFnDecl`.
///
/// The transformation:
///   dlg name(params) { body }
///   → fn name(params) { hoisted_entity_lets + lowered_body }
///
/// Speaker resolution uses three tiers:
/// 1. Params: passed directly as identifiers
/// 2. Singletons: hoisted as `let _name = Entity.getOrCreate<Name>()` at function top
/// 3. TextLine with no active speaker: emits `UnknownSpeaker` error
///
/// Localization keys are 8-char hex FNV-1a 32-bit hashes. Manual `#key` overrides
/// replace auto-generated keys; duplicates within a dlg block emit `DuplicateLocKey`.
pub fn lower_dialogue(
    dlg: DlgDecl<'_>,
    dlg_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstFnDecl {
    let dlg_name = dlg.name.0.to_string();

    // Lower attributes from CST to AST.
    let attrs = lower_attrs(dlg.attrs, ctx);

    // Detect [Locale("tag")] and suffix the function name to avoid resolver
    // duplicate-name collisions. The $ character is not valid in user-written
    // Writ identifiers, so greet$ja cannot collide with user-defined names.
    let locale_tag = attrs.iter().find_map(|a| {
        if a.name != "Locale" {
            return None;
        }
        a.args.iter().find_map(|arg| {
            if let AstAttributeArg::Positional(AstExpr::StringLit { value, .. }) = arg {
                Some(value.clone())
            } else {
                None
            }
        })
    });
    let dlg_name_final = if let Some(ref tag) = locale_tag {
        format!("{}${}", dlg_name, tag)
    } else {
        dlg_name.clone()
    };

    // Lower params (dialogue params are always regular, not self)
    let params = dlg
        .params
        .unwrap_or_default()
        .into_iter()
        .map(|(p, ps)| {
            let lowered = lower_param(p, ps);
            AstFnParam::Regular(lowered)
        })
        .collect::<Vec<_>>();

    // Collect param names for Tier 1 speaker lookup
    let param_names: Vec<String> = params.iter().filter_map(|p| match p {
        AstFnParam::Regular(param) => Some(param.name.clone()),
        AstFnParam::SelfParam { .. } => None,
    }).collect();

    // Pre-scan for singleton speakers (Tier 2 hoisting)
    let singleton_speakers = collect_singleton_speakers(&dlg.body, &param_names);

    // Generate hoisted let-bindings for singleton entities
    let mut hoisted_stmts: Vec<AstStmt> = singleton_speakers
        .iter()
        .map(|(speaker_name, speaker_span)| {
            let span = *speaker_span;
            AstStmt::Let {
                mutable: false,
                name: format!("_{}", speaker_name.to_lowercase()),
                name_span: span,
                ty: None,
                value: AstExpr::GenericCall {
                    callee: Box::new(AstExpr::MemberAccess {
                        object: Box::new(AstExpr::Ident {
                            name: "Entity".to_string(),
                            span,
                        }),
                        field: "getOrCreate".to_string(),
                        field_span: span,
                        span,
                    }),
                    type_args: vec![AstType::Named {
                        name: speaker_name.clone(),
                        span,
                    }],
                    args: vec![],
                    span,
                },
                span,
            }
        })
        .collect();

    // Initialize lowering state
    let mut state = DlgLowerState {
        dlg_name: dlg_name.clone(),
        namespace: ctx.current_namespace(),
        param_names,
        occurrence_tracker: HashMap::new(),
        manual_keys: HashMap::new(),
    };

    // Save speaker stack depth before lowering body
    let speaker_depth = ctx.speaker_stack_depth();

    // Lower the dialogue body
    let body_stmts = lower_dlg_lines(&dlg.body, &mut state, ctx);

    // Drain any speaker scopes pushed during body lowering —
    // prevents SpeakerTag leaks across sequential dlg items in the same lower() call
    while ctx.speaker_stack_depth() > speaker_depth {
        ctx.pop_speaker();
    }

    // Combine hoisted + body
    hoisted_stmts.extend(body_stmts);

    AstFnDecl {
        attrs,
        vis: lower_vis(dlg.vis),
        name: dlg_name_final,
        name_span: dlg.name.1,
        generics: vec![],
        params,
        return_type: None,
        body: hoisted_stmts,
        span: dlg_span,
    }
}

// =========================================================
// Private: collect_singleton_speakers
// =========================================================

/// Recursively scans a dialogue body for speaker names that are NOT in `param_names`.
/// Returns a deduplicated list in discovery order (first occurrence's span wins).
fn collect_singleton_speakers(
    lines: &[Spanned<DlgLine<'_>>],
    param_names: &[String],
) -> Vec<(String, SimpleSpan)> {
    let mut seen: Vec<String> = Vec::new();
    let mut result: Vec<(String, SimpleSpan)> = Vec::new();
    collect_singleton_speakers_inner(lines, param_names, &mut seen, &mut result);
    result
}

fn collect_singleton_speakers_inner(
    lines: &[Spanned<DlgLine<'_>>],
    param_names: &[String],
    seen: &mut Vec<String>,
    result: &mut Vec<(String, SimpleSpan)>,
) {
    for (line, _line_span) in lines {
        match line {
            DlgLine::SpeakerLine { speaker: (name, span), .. } => {
                let name_str = name.to_string();
                if !param_names.contains(&name_str) && !seen.contains(&name_str) {
                    seen.push(name_str.clone());
                    result.push((name_str, *span));
                }
            }
            DlgLine::SpeakerTag((name, span)) => {
                let name_str = name.to_string();
                if !param_names.contains(&name_str) && !seen.contains(&name_str) {
                    seen.push(name_str.clone());
                    result.push((name_str, *span));
                }
            }
            DlgLine::TextLine { .. } => {}
            DlgLine::CodeEscape(_) => {}
            DlgLine::Choice((choice, _choice_span)) => {
                for (arm, _arm_span) in &choice.arms {
                    collect_singleton_speakers_inner(&arm.body, param_names, seen, result);
                }
            }
            DlgLine::If((dlg_if, _if_span)) => {
                collect_singleton_speakers_inner(&dlg_if.then_block, param_names, seen, result);
                collect_dlg_if_else(&dlg_if.else_block, param_names, seen, result);
            }
            DlgLine::Match((dlg_match, _match_span)) => {
                for (arm, _arm_span) in &dlg_match.arms {
                    collect_singleton_speakers_inner(&arm.body, param_names, seen, result);
                }
            }
            DlgLine::Transition(_) => {}
        }
    }
}

fn collect_dlg_if_else<'src>(
    else_block: &Option<Box<Spanned<DlgElse<'src>>>>,
    param_names: &[String],
    seen: &mut Vec<String>,
    result: &mut Vec<(String, SimpleSpan)>,
) {
    if let Some(boxed) = else_block {
        let (dlg_else, _else_span) = boxed.as_ref();
        match dlg_else {
            DlgElse::ElseIf(elif) => {
                collect_singleton_speakers_inner(&elif.then_block, param_names, seen, result);
                collect_dlg_if_else(&elif.else_block, param_names, seen, result);
            }
            DlgElse::Else(lines) => {
                collect_singleton_speakers_inner(lines, param_names, seen, result);
            }
        }
    }
}

// =========================================================
// Private: lower_dlg_lines
// =========================================================

fn lower_dlg_lines(
    lines: &[Spanned<DlgLine<'_>>],
    state: &mut DlgLowerState,
    ctx: &mut LoweringContext,
) -> Vec<AstStmt> {
    let len = lines.len();
    let mut stmts = Vec::new();

    for (i, (line, line_span)) in lines.iter().enumerate() {
        let line_span = *line_span;
        match line {
            DlgLine::SpeakerLine { speaker: (speaker_name, speaker_span), text, loc_key } => {
                let speaker_ref = resolve_speaker(speaker_name, *speaker_span, state);
                let raw = raw_text_content(text);
                let fallback = lower_dlg_text(text.clone(), line_span, ctx);

                let expr = if loc_key.is_some() {
                    // Manual #key present -> say_localized
                    let key = compute_or_use_loc_key(
                        *loc_key,
                        speaker_name,
                        &raw,
                        line_span,
                        state,
                        ctx,
                    );
                    make_say_localized(speaker_ref, key, fallback, line_span)
                } else {
                    // No #key -> say() (auto FNV key is for CSV tooling only)
                    // Still compute key for occurrence tracking
                    let _csv_key = compute_or_use_loc_key(
                        None,
                        speaker_name,
                        &raw,
                        line_span,
                        state,
                        ctx,
                    );
                    make_say(speaker_ref, fallback, line_span)
                };

                stmts.push(AstStmt::Expr {
                    expr,
                    span: line_span,
                });
            }

            DlgLine::SpeakerTag((speaker_name, speaker_span)) => {
                ctx.push_speaker(SpeakerScope {
                    name: speaker_name.to_string(),
                    span: *speaker_span,
                });
                // No statement emitted — side-effect only
            }

            DlgLine::TextLine { text, loc_key } => {
                let (speaker_ref, speaker_name_str) =
                    if let Some(scope) = ctx.current_speaker() {
                        let name = scope.name.clone();
                        let span = scope.span;
                        let sp_ref = resolve_speaker(&name, span, state);
                        (sp_ref, name)
                    } else {
                        ctx.emit_error(LoweringError::UnknownSpeaker {
                            name: String::new(),
                            span: line_span,
                        });
                        (AstExpr::Error { span: line_span }, String::new())
                    };
                let raw = raw_text_content(text);
                let fallback = lower_dlg_text(text.clone(), line_span, ctx);

                let expr = if loc_key.is_some() {
                    // Manual #key present -> say_localized
                    let key = compute_or_use_loc_key(
                        *loc_key,
                        &speaker_name_str,
                        &raw,
                        line_span,
                        state,
                        ctx,
                    );
                    make_say_localized(speaker_ref, key, fallback, line_span)
                } else {
                    // No #key -> say() (auto FNV key is for CSV tooling only)
                    let _csv_key = compute_or_use_loc_key(
                        None,
                        &speaker_name_str,
                        &raw,
                        line_span,
                        state,
                        ctx,
                    );
                    make_say(speaker_ref, fallback, line_span)
                };

                stmts.push(AstStmt::Expr {
                    expr,
                    span: line_span,
                });
            }

            DlgLine::CodeEscape((escape, _escape_span)) => {
                match escape {
                    DlgEscape::Statement(stmt) => {
                        stmts.push(lower_stmt(*stmt.clone(), ctx));
                    }
                    DlgEscape::Block(stmts_cst) => {
                        for s in stmts_cst {
                            stmts.push(lower_stmt(s.clone(), ctx));
                        }
                    }
                }
            }

            DlgLine::Choice((choice, choice_span)) => {
                stmts.push(lower_choice(choice.clone(), *choice_span, state, ctx));
            }

            DlgLine::If((dlg_if, if_span)) => {
                stmts.push(lower_dlg_if(dlg_if.clone(), *if_span, state, ctx));
            }

            DlgLine::Match((dlg_match, match_span)) => {
                stmts.push(lower_dlg_match(dlg_match.clone(), *match_span, state, ctx));
            }

            DlgLine::Transition((trans, trans_span)) => {
                // Non-terminal check: transition must be last in its block
                if i < len - 1 {
                    ctx.emit_error(LoweringError::NonTerminalTransition { span: *trans_span });
                }
                stmts.push(lower_transition(trans.clone(), *trans_span, ctx));
            }
        }
    }

    stmts
}

// =========================================================
// Private: resolve_speaker
// =========================================================

fn resolve_speaker(
    speaker_name: &str,
    speaker_span: SimpleSpan,
    state: &DlgLowerState,
) -> AstExpr {
    if state.param_names.contains(&speaker_name.to_string()) {
        // Tier 1: direct param reference
        AstExpr::Ident {
            name: speaker_name.to_string(),
            span: speaker_span,
        }
    } else {
        // Tier 2: singleton entity — reference hoisted let-binding
        AstExpr::Ident {
            name: format!("_{}", speaker_name.to_lowercase()),
            span: speaker_span,
        }
    }
}

// =========================================================
// Private: compute_or_use_loc_key
// =========================================================

fn compute_or_use_loc_key(
    manual_key: Option<Spanned<&str>>,
    speaker_name: &str,
    raw_content: &str,
    _line_span: SimpleSpan,
    state: &mut DlgLowerState,
    ctx: &mut LoweringContext,
) -> String {
    if let Some((key_str, key_span)) = manual_key {
        // Manual key override
        if let Some(&existing_span) = state.manual_keys.get(key_str) {
            ctx.emit_error(LoweringError::DuplicateLocKey {
                key: key_str.to_string(),
                first_span: existing_span,
                second_span: key_span,
            });
        } else {
            state.manual_keys.insert(key_str.to_string(), key_span);
        }
        key_str.to_string()
    } else {
        // Auto-generate FNV-1a key
        let occurrence_key = (
            state.namespace.clone(),
            state.dlg_name.clone(),
            speaker_name.to_string(),
            raw_content.to_string(),
        );
        let occurrence_index = state.occurrence_tracker.entry(occurrence_key).or_insert(0);
        let idx = *occurrence_index;
        *occurrence_index += 1;

        let input = format!(
            "{}\0{}\0{}\0{}\0{}",
            state.namespace, state.dlg_name, speaker_name, raw_content, idx
        );
        fnv1a_32(&input)
    }
}

// =========================================================
// Private: fnv1a_32
// =========================================================

/// Computes FNV-1a 32-bit hash, returns 8-char lowercase hex string.
/// Per spec section 25.2.2 — exact algorithm mandated.
fn fnv1a_32(input: &str) -> String {
    const OFFSET_BASIS: u32 = 0x811c9dc5;
    const PRIME: u32 = 0x01000193;
    let mut hash: u32 = OFFSET_BASIS;
    for &byte in input.as_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{:08x}", hash)
}

// =========================================================
// Private: raw_text_content
// =========================================================

/// Concatenates text segments with interpolation slot identities preserved.
/// E.g., `{name}` stays `{name}`, `{player.name}` stays `{player.name}`.
/// Used for FNV-1a key computation and localization content strings.
fn raw_text_content(segments: &[Spanned<DlgTextSegment<'_>>]) -> String {
    let mut out = String::new();
    for (seg, _) in segments {
        match seg {
            DlgTextSegment::Text(s) => out.push_str(s),
            DlgTextSegment::Expr(inner) => {
                out.push('{');
                out.push_str(&expr_to_slot_text(inner));
                out.push('}');
            }
        }
    }
    out
}

/// Reconstruct a textual representation of a CST expression for localization slot content.
/// Preserves interpolation slot identities: `{name}` -> "name", `{player.name}` -> "player.name".
fn expr_to_slot_text(expr: &Spanned<writ_parser::cst::Expr<'_>>) -> String {
    use writ_parser::cst::Expr;
    match &expr.0 {
        Expr::Ident(name) => name.to_string(),
        Expr::MemberAccess(object, (field, _field_span)) => {
            format!("{}.{}", expr_to_slot_text(object), field)
        }
        Expr::Call(callee, _args) => {
            format!("{}(..)", expr_to_slot_text(callee))
        }
        // For other expression types, use a span-based representation for uniqueness
        _ => format!("expr@{}..{}", expr.1.start, expr.1.end),
    }
}

// =========================================================
// Private: lower_dlg_text
// =========================================================

/// Lowers dialogue text segments to a left-associative Add chain (mirrors lower_fmt_string).
fn lower_dlg_text(
    segments: Vec<Spanned<DlgTextSegment<'_>>>,
    outer_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstExpr {
    if segments.is_empty() {
        return AstExpr::StringLit {
            value: String::new(),
            span: outer_span,
        };
    }

    let parts: Vec<AstExpr> = segments
        .into_iter()
        .map(|(seg, seg_span)| match seg {
            DlgTextSegment::Text(s) => AstExpr::StringLit {
                value: s.to_string(),
                span: seg_span,
            },
            DlgTextSegment::Expr(inner) => {
                let lowered = lower_expr(*inner, ctx);
                AstExpr::GenericCall {
                    callee: Box::new(AstExpr::MemberAccess {
                        object: Box::new(lowered),
                        field: "into".to_string(),
                        field_span: seg_span,
                        span: seg_span,
                    }),
                    type_args: vec![AstType::Named {
                        name: "string".to_string(),
                        span: seg_span,
                    }],
                    args: vec![],
                    span: seg_span,
                }
            }
        })
        .collect();

    // Left-associative fold: (((a + b) + c) + d) + ...
    let mut iter = parts.into_iter();
    let first = iter.next().expect("segments non-empty: checked above");
    iter.fold(first, |acc, next| AstExpr::Binary {
        left: Box::new(acc),
        op: BinaryOp::Add,
        right: Box::new(next),
        span: outer_span,
    })
}

// =========================================================
// Private: make_say / make_say_localized
// =========================================================

/// Emits `say(speaker, text)` — used when no manual #key is present.
fn make_say(
    speaker_ref: AstExpr,
    text: AstExpr,
    span: SimpleSpan,
) -> AstExpr {
    AstExpr::Call {
        callee: Box::new(AstExpr::Ident {
            name: "say".to_string(),
            span,
        }),
        args: vec![
            AstArg { name: None, value: speaker_ref, span },
            AstArg { name: None, value: text, span },
        ],
        span,
    }
}

/// Emits `say_localized(speaker, key, fallback)` — used when manual #key is present.
fn make_say_localized(
    speaker_ref: AstExpr,
    loc_key: String,
    fallback: AstExpr,
    span: SimpleSpan,
) -> AstExpr {
    AstExpr::Call {
        callee: Box::new(AstExpr::Ident {
            name: "say_localized".to_string(),
            span,
        }),
        args: vec![
            AstArg { name: None, value: speaker_ref, span },
            AstArg {
                name: None,
                value: AstExpr::StringLit { value: loc_key, span },
                span,
            },
            AstArg { name: None, value: fallback, span },
        ],
        span,
    }
}

// =========================================================
// Private: lower_choice
// =========================================================

fn lower_choice(
    choice: DlgChoice<'_>,
    choice_span: SimpleSpan,
    state: &mut DlgLowerState,
    ctx: &mut LoweringContext,
) -> AstStmt {
    let arm_exprs: Vec<AstExpr> = choice
        .arms
        .into_iter()
        .map(|(arm, arm_span)| {
            // Save speaker scope depth
            let depth = ctx.speaker_stack_depth();

            // Compute loc key for choice label (empty speaker for choice labels)
            let label_text = arm.label.0.to_string();
            let label_span = arm.label.1;
            let key = compute_or_use_loc_key(
                arm.loc_key,
                "",
                &label_text,
                arm_span,
                state,
                ctx,
            );
            // Lower arm body
            let body = lower_dlg_lines(&arm.body, state, ctx);

            // Restore speaker scope
            while ctx.speaker_stack_depth() > depth {
                ctx.pop_speaker();
            }

            // Build: Option(label_text, loc_key, fn() { body })
            AstExpr::Call {
                callee: Box::new(AstExpr::Ident {
                    name: "Option".to_string(),
                    span: arm_span,
                }),
                args: vec![
                    AstArg {
                        name: None,
                        value: AstExpr::StringLit {
                            value: label_text,
                            span: label_span,
                        },
                        span: arm_span,
                    },
                    AstArg {
                        name: None,
                        value: AstExpr::StringLit {
                            value: key,
                            span: arm_span,
                        },
                        span: arm_span,
                    },
                    AstArg {
                        name: None,
                        value: AstExpr::Lambda {
                            params: vec![],
                            return_type: None,
                            body,
                            span: arm_span,
                        },
                        span: arm_span,
                    },
                ],
                span: arm_span,
            }
        })
        .collect();

    AstStmt::Expr {
        expr: AstExpr::Call {
            callee: Box::new(AstExpr::Ident {
                name: "choice".to_string(),
                span: choice_span,
            }),
            args: vec![AstArg {
                name: None,
                value: AstExpr::ArrayLit {
                    elements: arm_exprs,
                    span: choice_span,
                },
                span: choice_span,
            }],
            span: choice_span,
        },
        span: choice_span,
    }
}

// =========================================================
// Private: lower_dlg_if
// =========================================================

fn lower_dlg_if(
    dlg_if: DlgIf<'_>,
    if_span: SimpleSpan,
    state: &mut DlgLowerState,
    ctx: &mut LoweringContext,
) -> AstStmt {
    let condition = lower_expr(*dlg_if.condition, ctx);

    // Save speaker scope before then-block (DLG-05)
    let depth = ctx.speaker_stack_depth();
    let then_block = lower_dlg_lines(&dlg_if.then_block, state, ctx);
    // Restore speaker scope after then-block
    while ctx.speaker_stack_depth() > depth {
        ctx.pop_speaker();
    }

    let else_block = lower_dlg_else(dlg_if.else_block, state, ctx);

    AstStmt::Expr {
        expr: AstExpr::If {
            condition: Box::new(condition),
            then_block,
            else_block,
            span: if_span,
        },
        span: if_span,
    }
}

fn lower_dlg_else<'src>(
    else_block: Option<Box<Spanned<DlgElse<'src>>>>,
    state: &mut DlgLowerState,
    ctx: &mut LoweringContext,
) -> Option<Box<AstExpr>> {
    match else_block {
        None => None,
        Some(boxed) => {
            let (dlg_else, else_span) = *boxed;
            match dlg_else {
                DlgElse::ElseIf(elif) => {
                    // ElseIf recurses into lower_dlg_if which handles its own scope isolation
                    let elif_stmt = lower_dlg_if(elif, else_span, state, ctx);
                    Some(Box::new(AstExpr::Block {
                        stmts: vec![elif_stmt],
                        span: else_span,
                    }))
                }
                DlgElse::Else(lines) => {
                    // Save/restore speaker scope for else block (DLG-05)
                    let depth = ctx.speaker_stack_depth();
                    let stmts = lower_dlg_lines(&lines, state, ctx);
                    while ctx.speaker_stack_depth() > depth {
                        ctx.pop_speaker();
                    }
                    Some(Box::new(AstExpr::Block {
                        stmts,
                        span: else_span,
                    }))
                }
            }
        }
    }
}

// =========================================================
// Private: lower_dlg_match
// =========================================================

fn lower_dlg_match(
    dlg_match: DlgMatch<'_>,
    match_span: SimpleSpan,
    state: &mut DlgLowerState,
    ctx: &mut LoweringContext,
) -> AstStmt {
    let scrutinee = lower_expr(*dlg_match.scrutinee, ctx);
    let arms = dlg_match
        .arms
        .into_iter()
        .map(|(arm, arm_span)| {
            // Save speaker scope before each arm (DLG-05)
            let depth = ctx.speaker_stack_depth();
            let body = lower_dlg_lines(&arm.body, state, ctx);
            // Restore speaker scope after each arm
            while ctx.speaker_stack_depth() > depth {
                ctx.pop_speaker();
            }
            AstMatchArm {
                pattern: lower_pattern(arm.pattern, ctx),
                body,
                span: arm_span,
            }
        })
        .collect();

    AstStmt::Expr {
        expr: AstExpr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span: match_span,
        },
        span: match_span,
    }
}

// =========================================================
// Private: lower_transition
// =========================================================

fn lower_transition(
    trans: DlgTransition<'_>,
    trans_span: SimpleSpan,
    ctx: &mut LoweringContext,
) -> AstStmt {
    let args = trans
        .args
        .unwrap_or_default()
        .into_iter()
        .map(|e| AstArg {
            name: None,
            value: lower_expr(e, ctx),
            span: trans_span,
        })
        .collect();

    AstStmt::Return {
        value: Some(AstExpr::Call {
            callee: Box::new(AstExpr::Ident {
                name: trans.target.0.to_string(),
                span: trans.target.1,
            }),
            args,
            span: trans_span,
        }),
        span: trans_span,
    }
}
