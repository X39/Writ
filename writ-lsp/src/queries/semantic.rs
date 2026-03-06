//! Semantic token collection query functions for LSP semantic token handler.
//!
//! Provides semantic token collection for typed AST nodes and dialogue speaker
//! tokens from source text, used by the LSP textDocument/semanticTokens/full handler.

use chumsky::span::SimpleSpan;
use writ_compiler::check::ir::{TypedAst, TypedDecl, TypedExpr, TypedStmt};
use writ_compiler::check::ty::TyInterner;
use writ_parser::cst::{DlgElse, DlgLine, Item};

// =============================================================================
// Semantic token query functions
// =============================================================================

/// A raw semantic token before delta encoding.
/// Positions are absolute (not delta-encoded).
pub struct RawSemanticToken {
    pub line: u32,
    pub start_char: u32, // UTF-16 code units
    pub length: u32,     // UTF-16 code units
    pub token_type: u32,
}

// Token type indices — must match the order in SemanticTokensLegend.token_types
// registered in ServerCapabilities (backend.rs).
#[allow(dead_code)]
const TOKEN_TYPE_KEYWORD: u32 = 0;
const TOKEN_TYPE_TYPE: u32 = 1;
const TOKEN_TYPE_ENTITY: u32 = 2;
const TOKEN_TYPE_COMPONENT: u32 = 3;
const TOKEN_TYPE_DIALOGUE_SPEAKER: u32 = 4;
const TOKEN_TYPE_FUNCTION: u32 = 5;
const TOKEN_TYPE_VARIABLE: u32 = 6;
#[allow(dead_code)]
const TOKEN_TYPE_PARAMETER: u32 = 7;

/// Collect semantic tokens from a `TypedAst` for a specific file.
///
/// Walks declarations and expressions, emitting tokens for:
/// - Entity declaration names (`token_type = entity`)
/// - Struct/Class/Enum/Contract declaration names (`token_type = type`)
/// - Function declaration names (`token_type = function`)
/// - Component/ExternComponent declaration names (`token_type = component`)
/// - Const/Global declaration names (`token_type = variable`)
/// - Entity-typed variable references (`token_type = entity`)
/// - ComponentAccess component names (`token_type = component`)
///
/// Returns tokens sorted by position (line, then start_char).
pub fn collect_semantic_tokens(
    ast: &TypedAst,
    interner: &TyInterner,
    source: &str,
    file_id: writ_diagnostics::FileId,
) -> Vec<RawSemanticToken> {
    let mut tokens = Vec::new();

    for decl in &ast.decls {
        match decl {
            TypedDecl::Entity { def_id } => {
                let entry = ast.def_map.get_entry(*def_id);
                if entry.file_id == file_id {
                    push_token_for_span(&mut tokens, source, &entry.name_span, TOKEN_TYPE_ENTITY);
                }
            }
            TypedDecl::Struct { def_id } | TypedDecl::Class { def_id } | TypedDecl::Enum { def_id } => {
                let entry = ast.def_map.get_entry(*def_id);
                if entry.file_id == file_id {
                    push_token_for_span(&mut tokens, source, &entry.name_span, TOKEN_TYPE_TYPE);
                }
            }
            TypedDecl::Fn { def_id, body, .. } => {
                let entry = ast.def_map.get_entry(*def_id);
                if entry.file_id == file_id {
                    push_token_for_span(&mut tokens, source, &entry.name_span, TOKEN_TYPE_FUNCTION);
                }
                collect_tokens_in_expr(body, interner, source, &mut tokens);
            }
            TypedDecl::Impl { def_id: _, methods } => {
                for (method_def_id, body) in methods {
                    let entry = ast.def_map.get_entry(*method_def_id);
                    if entry.file_id == file_id {
                        push_token_for_span(
                            &mut tokens,
                            source,
                            &entry.name_span,
                            TOKEN_TYPE_FUNCTION,
                        );
                    }
                    collect_tokens_in_expr(body, interner, source, &mut tokens);
                }
            }
            TypedDecl::Component { def_id } | TypedDecl::ExternComponent { def_id } => {
                let entry = ast.def_map.get_entry(*def_id);
                if entry.file_id == file_id {
                    push_token_for_span(&mut tokens, source, &entry.name_span, TOKEN_TYPE_COMPONENT);
                }
            }
            TypedDecl::Contract { def_id } => {
                let entry = ast.def_map.get_entry(*def_id);
                if entry.file_id == file_id {
                    push_token_for_span(&mut tokens, source, &entry.name_span, TOKEN_TYPE_TYPE);
                }
            }
            TypedDecl::Const { def_id, value } | TypedDecl::Global { def_id, value } => {
                let entry = ast.def_map.get_entry(*def_id);
                if entry.file_id == file_id {
                    push_token_for_span(&mut tokens, source, &entry.name_span, TOKEN_TYPE_VARIABLE);
                }
                collect_tokens_in_expr(value, interner, source, &mut tokens);
            }
            // ExternFn, ExternStruct, ExternClass have no body
            _ => {}
        }
    }

    // Phase 58: Emit dialogue speaker tokens by re-parsing the source.
    // The TypedAst has no dialogue-specific nodes (dlg is lowered to fn).
    let speaker_tokens = collect_dialogue_speaker_tokens(source);
    tokens.extend(speaker_tokens);

    // Sort by position (line, then start_char)
    tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.start_char.cmp(&b.start_char)));
    tokens
}

/// Recursively walk an expression tree and collect semantic tokens.
fn collect_tokens_in_expr(
    expr: &TypedExpr,
    interner: &TyInterner,
    source: &str,
    tokens: &mut Vec<RawSemanticToken>,
) {
    match expr {
        TypedExpr::Var { ty, span, .. } => {
            // Entity-typed variable references get the entity token type
            if matches!(interner.kind(*ty), writ_compiler::check::ty::TyKind::Entity(_)) {
                push_token_for_span(tokens, source, span, TOKEN_TYPE_ENTITY);
            }
        }
        TypedExpr::ComponentAccess { span, component, receiver, .. } => {
            // Recurse into the receiver
            collect_tokens_in_expr(receiver, interner, source, tokens);
            // For the component name portion: compute offset from end of span.
            // The span covers `receiver.component`; the component name is at the end.
            let comp_len = component.len();
            if span.end >= comp_len {
                let comp_start = span.end - comp_len;
                let comp_span = SimpleSpan { start: comp_start, end: span.end, context: () };
                push_token_for_span(tokens, source, &comp_span, TOKEN_TYPE_COMPONENT);
            }
        }
        TypedExpr::Call { callee, args, .. } => {
            collect_tokens_in_expr(callee, interner, source, tokens);
            for arg in args {
                collect_tokens_in_expr(arg, interner, source, tokens);
            }
        }
        TypedExpr::Field { receiver, .. } => {
            collect_tokens_in_expr(receiver, interner, source, tokens);
        }
        TypedExpr::Index { receiver, index, .. } => {
            collect_tokens_in_expr(receiver, interner, source, tokens);
            collect_tokens_in_expr(index, interner, source, tokens);
        }
        TypedExpr::Binary { left, right, .. } => {
            collect_tokens_in_expr(left, interner, source, tokens);
            collect_tokens_in_expr(right, interner, source, tokens);
        }
        TypedExpr::UnaryPrefix { expr: inner, .. } => {
            collect_tokens_in_expr(inner, interner, source, tokens);
        }
        TypedExpr::Match { scrutinee, arms, .. } => {
            collect_tokens_in_expr(scrutinee, interner, source, tokens);
            for arm in arms {
                collect_tokens_in_expr(&arm.body, interner, source, tokens);
            }
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            collect_tokens_in_expr(condition, interner, source, tokens);
            collect_tokens_in_expr(then_branch, interner, source, tokens);
            if let Some(eb) = else_branch {
                collect_tokens_in_expr(eb, interner, source, tokens);
            }
        }
        TypedExpr::Block { stmts, tail, .. } => {
            collect_tokens_in_stmts(stmts, interner, source, tokens);
            if let Some(t) = tail {
                collect_tokens_in_expr(t, interner, source, tokens);
            }
        }
        TypedExpr::Lambda { body, .. } => {
            collect_tokens_in_expr(body, interner, source, tokens);
        }
        TypedExpr::Assign { target, value, .. } => {
            collect_tokens_in_expr(target, interner, source, tokens);
            collect_tokens_in_expr(value, interner, source, tokens);
        }
        TypedExpr::New { fields, .. } => {
            for (_, v) in fields {
                collect_tokens_in_expr(v, interner, source, tokens);
            }
        }
        TypedExpr::ArrayLit { elements, .. } => {
            for elem in elements {
                collect_tokens_in_expr(elem, interner, source, tokens);
            }
        }
        TypedExpr::Range { start, end, .. } => {
            if let Some(s) = start {
                collect_tokens_in_expr(s, interner, source, tokens);
            }
            if let Some(e) = end {
                collect_tokens_in_expr(e, interner, source, tokens);
            }
        }
        TypedExpr::Spawn { expr: inner, .. }
        | TypedExpr::SpawnDetached { expr: inner, .. }
        | TypedExpr::Join { expr: inner, .. }
        | TypedExpr::Cancel { expr: inner, .. }
        | TypedExpr::Defer { expr: inner, .. } => {
            collect_tokens_in_expr(inner, interner, source, tokens);
        }
        TypedExpr::Return { value: Some(v), .. } => {
            collect_tokens_in_expr(v, interner, source, tokens);
        }
        // Leaf nodes: Literal, SelfRef, Path, Error — no sub-expressions to recurse into
        _ => {}
    }
}

/// Recursively collect semantic tokens from a list of statements.
fn collect_tokens_in_stmts(
    stmts: &[TypedStmt],
    interner: &TyInterner,
    source: &str,
    tokens: &mut Vec<RawSemanticToken>,
) {
    for stmt in stmts {
        collect_tokens_in_stmt(stmt, interner, source, tokens);
    }
}

/// Collect semantic tokens from a single statement.
fn collect_tokens_in_stmt(
    stmt: &TypedStmt,
    interner: &TyInterner,
    source: &str,
    tokens: &mut Vec<RawSemanticToken>,
) {
    match stmt {
        TypedStmt::Let { value, .. } => {
            collect_tokens_in_expr(value, interner, source, tokens);
        }
        TypedStmt::Expr { expr, .. } => {
            collect_tokens_in_expr(expr, interner, source, tokens);
        }
        TypedStmt::For { iterable, body, .. } => {
            collect_tokens_in_expr(iterable, interner, source, tokens);
            collect_tokens_in_stmts(body, interner, source, tokens);
        }
        TypedStmt::While { condition, body, .. } => {
            collect_tokens_in_expr(condition, interner, source, tokens);
            collect_tokens_in_stmts(body, interner, source, tokens);
        }
        TypedStmt::Atomic { body, .. } => {
            collect_tokens_in_stmts(body, interner, source, tokens);
        }
        TypedStmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_tokens_in_expr(v, interner, source, tokens);
            }
        }
        TypedStmt::Break { value, .. } => {
            if let Some(v) = value {
                collect_tokens_in_expr(v, interner, source, tokens);
            }
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => {}
    }
}

/// Convert a `SimpleSpan` (byte-offset range) to a `RawSemanticToken`.
///
/// For single-line tokens, length is the UTF-16 code unit count of the token text.
/// Multi-line tokens (rare for identifiers) use the start portion only.
fn push_token_for_span(
    tokens: &mut Vec<RawSemanticToken>,
    source: &str,
    span: &SimpleSpan,
    token_type: u32,
) {
    let start_pos = crate::convert::offset_to_position(source, span.start);
    let end_pos = crate::convert::offset_to_position(source, span.end);
    // Length in UTF-16 code units (only works accurately for single-line tokens)
    let length = if start_pos.line == end_pos.line {
        end_pos.character - start_pos.character
    } else {
        // Multi-line tokens: use just the first line's portion (rare for identifiers)
        end_pos.character
    };
    if length > 0 {
        tokens.push(RawSemanticToken {
            line: start_pos.line,
            start_char: start_pos.character,
            length,
            token_type,
        });
    }
}

/// Collect TOKEN_TYPE_DIALOGUE_SPEAKER tokens for all @Speaker names in the source.
///
/// Re-parses `source` via writ_parser::parse and walks Item::Dlg entries to find
/// SpeakerLine and SpeakerTag lines. Each @SpeakerName occurrence gets a
/// RawSemanticToken with token_type = TOKEN_TYPE_DIALOGUE_SPEAKER (4).
///
/// The speaker span covers the identifier name only -- the @ sigil is excluded
/// by the parser (just(Token::At).ignore_then(ident_with_span)).
///
/// Returns tokens in source order (not sorted -- caller merges and sorts).
pub fn collect_dialogue_speaker_tokens(source: &str) -> Vec<RawSemanticToken> {
    let mut tokens = Vec::new();

    // writ_parser::parse requires &'static str due to chumsky stream constraints.
    // We leak a copy here; the memory cost is bounded by the file size and
    // acceptable for the semantic token refresh frequency.
    let src_static: &'static str = Box::leak(source.to_string().into_boxed_str());

    // Re-parse source; gracefully handle errors -- partial CSTs may still yield items.
    let (items_opt, _parse_errs) = writ_parser::parse(src_static);
    let Some(items) = items_opt else { return tokens };

    // Walk top-level items looking for dlg declarations
    for (item, _item_span) in &items {
        if let Item::Dlg((dlg_decl, _dlg_span)) = item {
            collect_speaker_tokens_in_dlg_body(&dlg_decl.body, source, &mut tokens);
        }
    }

    tokens
}

fn collect_speaker_tokens_in_dlg_body(
    lines: &[writ_parser::Spanned<DlgLine<'_>>],
    source: &str,
    tokens: &mut Vec<RawSemanticToken>,
) {
    for (line, _line_span) in lines {
        match line {
            DlgLine::SpeakerLine { speaker: (_, span), .. } => {
                push_token_for_span(tokens, source, span, TOKEN_TYPE_DIALOGUE_SPEAKER);
            }
            DlgLine::SpeakerTag((_, span)) => {
                push_token_for_span(tokens, source, span, TOKEN_TYPE_DIALOGUE_SPEAKER);
            }
            DlgLine::Choice((choice, _)) => {
                for (arm, _) in &choice.arms {
                    collect_speaker_tokens_in_dlg_body(&arm.body, source, tokens);
                }
            }
            DlgLine::If((dlg_if, _)) => {
                collect_speaker_tokens_in_dlg_body(&dlg_if.then_block, source, tokens);
                collect_dlg_if_else_speakers(&dlg_if.else_block, source, tokens);
            }
            DlgLine::Match((dlg_match, _)) => {
                for (arm, _) in &dlg_match.arms {
                    collect_speaker_tokens_in_dlg_body(&arm.body, source, tokens);
                }
            }
            DlgLine::TextLine { .. }
            | DlgLine::CodeEscape(_)
            | DlgLine::Transition(_) => {}
        }
    }
}

fn collect_dlg_if_else_speakers(
    else_block: &Option<Box<writ_parser::Spanned<DlgElse<'_>>>>,
    source: &str,
    tokens: &mut Vec<RawSemanticToken>,
) {
    if let Some(boxed) = else_block {
        let (dlg_else, _) = boxed.as_ref();
        match dlg_else {
            DlgElse::ElseIf(elif) => {
                collect_speaker_tokens_in_dlg_body(&elif.then_block, source, tokens);
                collect_dlg_if_else_speakers(&elif.else_block, source, tokens);
            }
            DlgElse::Else(lines) => {
                collect_speaker_tokens_in_dlg_body(lines, source, tokens);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{collect_semantic_tokens, collect_dialogue_speaker_tokens, TOKEN_TYPE_ENTITY, TOKEN_TYPE_TYPE, TOKEN_TYPE_DIALOGUE_SPEAKER};
    use writ_compiler::check::ir::TypedAst;
    use writ_compiler::check::ty::TyInterner;
    use writ_diagnostics::{FileId, Severity};

    fn build_typed_ast_full(
        src: &str,
    ) -> (TypedAst, TyInterner, writ_compiler::check::env::TypeEnv) {
        let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
        let file_id = FileId(0);

        let (cst_opt, parse_errs) = writ_parser::parse(src_static);
        assert!(parse_errs.is_empty(), "parse errors: {:?}", parse_errs);
        let cst = cst_opt.expect("parse returned no output");

        let (ast, lower_errs) = writ_compiler::lower(cst);
        assert!(lower_errs.is_empty(), "lower errors: {:?}", lower_errs);

        let (resolved, resolve_diags) = writ_compiler::resolve::resolve(
            &[(file_id, &ast)],
            &[(file_id, "test.writ")],
        );
        let resolve_errors: Vec<_> = resolve_diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(resolve_errors.is_empty(), "resolve errors: {:?}", resolve_errors);

        let (typed_ast, interner, type_env, type_diags) =
            writ_compiler::check::typecheck(resolved, &[(file_id, &ast)]);
        let type_errors: Vec<_> = type_diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(type_errors.is_empty(), "type errors: {:?}", type_errors);

        (typed_ast, interner, type_env)
    }

    // ── collect_semantic_tokens tests ─────────────────────────────────────────

    #[test]
    fn test_semantic_tokens_entity_decl() {
        // Entity declaration name should get TOKEN_TYPE_ENTITY (2)
        let src = "pub entity Player { hp: int, }";
        let (ast, interner, _type_env) = build_typed_ast_full(src);

        let tokens = collect_semantic_tokens(&ast, &interner, src, FileId(0));

        // Find the token for "Player"
        let player_offset = src.find("Player").unwrap();
        let player_pos = crate::convert::offset_to_position(src, player_offset);

        let entity_token = tokens
            .iter()
            .find(|t| t.line == player_pos.line && t.start_char == player_pos.character);

        assert!(
            entity_token.is_some(),
            "expected a token at 'Player' position, got tokens: {:?}",
            tokens.iter().map(|t| (t.line, t.start_char, t.token_type)).collect::<Vec<_>>()
        );
        assert_eq!(
            entity_token.unwrap().token_type,
            TOKEN_TYPE_ENTITY,
            "expected entity token type for 'Player' declaration"
        );
    }

    #[test]
    fn test_semantic_tokens_struct_decl() {
        // Struct declaration name should get TOKEN_TYPE_TYPE (1)
        let src = "pub struct Point { x: int, y: int }";
        let (ast, interner, _type_env) = build_typed_ast_full(src);

        let tokens = collect_semantic_tokens(&ast, &interner, src, FileId(0));

        let point_offset = src.find("Point").unwrap();
        let point_pos = crate::convert::offset_to_position(src, point_offset);

        let type_token = tokens
            .iter()
            .find(|t| t.line == point_pos.line && t.start_char == point_pos.character);

        assert!(
            type_token.is_some(),
            "expected a token at 'Point' position"
        );
        assert_eq!(
            type_token.unwrap().token_type,
            TOKEN_TYPE_TYPE,
            "expected type token type for 'Point' declaration"
        );
    }

    #[test]
    fn test_semantic_tokens_entity_var_ref() {
        // A variable with entity type should get TOKEN_TYPE_ENTITY (2)
        let src = r#"pub entity Npc { hp: int, }
fn main() { let n: Npc = new Npc { hp: 10 }; n }
"#;
        let (ast, interner, _type_env) = build_typed_ast_full(src);

        let tokens = collect_semantic_tokens(&ast, &interner, src, FileId(0));

        // Find the tail 'n' in "n }" — this is the entity-typed variable reference
        let tail_n_offset = src.rfind("; n }").map(|i| i + 2).unwrap();
        let n_pos = crate::convert::offset_to_position(src, tail_n_offset);

        let entity_var_token = tokens
            .iter()
            .find(|t| t.line == n_pos.line && t.start_char == n_pos.character);

        assert!(
            entity_var_token.is_some(),
            "expected an entity token for variable 'n', tokens: {:?}",
            tokens.iter().map(|t| (t.line, t.start_char, t.token_type)).collect::<Vec<_>>()
        );
        assert_eq!(
            entity_var_token.unwrap().token_type,
            TOKEN_TYPE_ENTITY,
            "expected entity token type for entity-typed variable reference"
        );
    }

    #[test]
    fn test_semantic_tokens_sorted() {
        // Tokens must be sorted by (line, start_char)
        let src = "pub struct Foo {} pub entity Bar {} fn baz() -> int { 1 }";
        let (ast, interner, _type_env) = build_typed_ast_full(src);

        let tokens = collect_semantic_tokens(&ast, &interner, src, FileId(0));

        // Verify sorting
        for pair in tokens.windows(2) {
            let a = &pair[0];
            let b = &pair[1];
            assert!(
                (a.line, a.start_char) <= (b.line, b.start_char),
                "tokens out of order: ({},{}) before ({},{})",
                a.line, a.start_char, b.line, b.start_char
            );
        }

        // Should have at least 3 tokens (Foo, Bar, baz)
        assert!(
            tokens.len() >= 3,
            "expected at least 3 declaration tokens, got {}",
            tokens.len()
        );
    }

    // ── collect_dialogue_speaker_tokens tests ─────────────────────────────────

    #[test]
    fn test_semantic_tokens_dialogue_speaker() {
        let src = "dlg intro {\n    @Alice Hello there.\n    @Bob Greetings!\n}\n";

        let tokens = collect_dialogue_speaker_tokens(src);

        assert_eq!(tokens.len(), 2, "expected 2 speaker tokens, got {:?}",
            tokens.iter().map(|t| (t.line, t.start_char, t.token_type)).collect::<Vec<_>>());

        // Alice is on line 1 (0-indexed), after 4 spaces of indent
        assert_eq!(tokens[0].token_type, TOKEN_TYPE_DIALOGUE_SPEAKER);
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].start_char, 5); // "@Alice" -> @ at col 4, "Alice" at col 5
        assert_eq!(tokens[0].length, 5);     // "Alice" = 5 chars

        // Bob is on line 2
        assert_eq!(tokens[1].token_type, TOKEN_TYPE_DIALOGUE_SPEAKER);
        assert_eq!(tokens[1].line, 2);
        assert_eq!(tokens[1].start_char, 5); // "@Bob" -> @ at col 4, "Bob" at col 5
        assert_eq!(tokens[1].length, 3);     // "Bob" = 3 chars
    }

    #[test]
    fn test_semantic_tokens_includes_dialogue_speaker() {
        // Dialogue lowering generates Entity.getOrCreate<Name>() for singleton speakers,
        // which produces type errors in the unit test environment (Entity is a runtime
        // builtin not available in tests). collect_semantic_tokens works on partial
        // TypedAsts, so we run the pipeline without asserting zero type errors.
        let src = "pub entity Alice {}\ndlg intro {\n    @Alice\n}\n";
        let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
        let file_id = FileId(0);

        let (cst_opt, _parse_errs) = writ_parser::parse(src_static);
        let cst = cst_opt.expect("parse returned no output");
        let (ast, _lower_errs) = writ_compiler::lower(cst);
        let (resolved, _resolve_diags) = writ_compiler::resolve::resolve(
            &[(file_id, &ast)],
            &[(file_id, "test.writ")],
        );
        // Accept type errors (Entity.getOrCreate is a runtime builtin, unavailable in tests)
        let (typed_ast, interner, _type_env, _type_diags) =
            writ_compiler::check::typecheck(resolved, &[(file_id, &ast)]);

        let tokens = collect_semantic_tokens(&typed_ast, &interner, src, file_id);

        let has_entity = tokens.iter().any(|t| t.token_type == TOKEN_TYPE_ENTITY);
        let has_speaker = tokens.iter().any(|t| t.token_type == TOKEN_TYPE_DIALOGUE_SPEAKER);
        assert!(has_entity, "expected entity token for Alice declaration");
        assert!(has_speaker, "expected dialogue speaker token for @Alice in dlg");
    }

    #[test]
    fn test_semantic_tokens_dialogue_speaker_nested() {
        // Speaker inside a choice arm should still be detected
        let src = r#"dlg quest {
    @NPC Welcome.
    $ choice {
        "Option A" {
            @NPC Good choice.
        }
    }
}
"#;
        let tokens = collect_dialogue_speaker_tokens(src);

        // Should find 2 speakers: @NPC at top level, @NPC inside choice arm
        assert_eq!(tokens.len(), 2, "expected 2 speaker tokens (top-level + nested), got {:?}",
            tokens.iter().map(|t| (t.line, t.start_char, t.token_type)).collect::<Vec<_>>());
        assert!(tokens.iter().all(|t| t.token_type == TOKEN_TYPE_DIALOGUE_SPEAKER));
    }
}
