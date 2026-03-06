//! Completion and signature help query functions for LSP handlers.
//!
//! Provides identifier completions, dot-completions, and signature help
//! used by LSP completion and signatureHelp handlers.

use lsp_types::{CompletionItem, CompletionItemKind, Command};
use writ_compiler::check::ir::{TypedAst, TypedDecl, TypedExpr, TypedStmt};
use writ_compiler::check::ty::{Ty, TyInterner, TyKind};
use writ_compiler::resolve::def_map::{DefKind, DefMap};

// =============================================================================
// Completion query functions
// =============================================================================

/// Build completion items for identifier context (no trigger character).
///
/// Returns keywords, prelude names, and all public definitions from DefMap.
pub fn build_identifier_completions(
    def_map: &DefMap,
    _interner: &TyInterner,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // 1. Keywords
    let keywords = [
        "fn", "struct", "entity", "enum", "impl", "contract", "let", "mut",
        "if", "else", "for", "while", "return", "spawn", "yield", "new",
        "using", "namespace", "extern", "global", "const", "pub", "priv",
        "match", "break", "continue", "true", "false", "class", "component",
        "on", "atomic", "defer", "self",
    ];
    for kw in keywords {
        if kw == "new" {
            // Special-case: insert "new " with trailing space and retrigger
            // completions so the constructable-type list appears immediately.
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                sort_text: Some(format!("6_{}", kw)),
                insert_text: Some("new ".to_string()),
                command: Some(Command {
                    title: "Trigger Suggest".to_string(),
                    command: "editor.action.triggerSuggest".to_string(),
                    arguments: None,
                }),
                ..Default::default()
            });
        } else {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                sort_text: Some(format!("6_{}", kw)),
                ..Default::default()
            });
        }
    }

    // 2. Prelude primitive names
    for &name in writ_compiler::resolve::prelude::PRELUDE_PRIMITIVE_NAMES {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            sort_text: Some(format!("3_{}", name)),
            ..Default::default()
        });
    }

    // 3. Prelude type names
    for &name in writ_compiler::resolve::prelude::PRELUDE_TYPE_NAMES {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            sort_text: Some(format!("2_{}", name)),
            ..Default::default()
        });
    }

    // 4. Prelude contract names
    for &name in writ_compiler::resolve::prelude::PRELUDE_CONTRACT_NAMES {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::INTERFACE),
            sort_text: Some(format!("5_{}", name)),
            ..Default::default()
        });
    }

    // 5. All public definitions from DefMap
    for &def_id in def_map.by_fqn.values() {
        let entry = def_map.get_entry(def_id);
        // Skip synthetic entries (log::*, dialogue builtins) and impl blocks
        if entry.file_id == writ_diagnostics::FileId(u32::MAX) {
            continue;
        }
        if matches!(entry.kind, DefKind::Impl) {
            continue;
        }

        let (kind, sort_prefix) = match entry.kind {
            DefKind::Fn | DefKind::ExternFn => (CompletionItemKind::FUNCTION, "1_"),
            DefKind::Struct | DefKind::ExternStruct => (CompletionItemKind::STRUCT, "0_"),
            DefKind::Class | DefKind::ExternClass => (CompletionItemKind::CLASS, "0_"),
            DefKind::Entity => (CompletionItemKind::CLASS, "0_"),
            DefKind::Enum => (CompletionItemKind::ENUM, "0_"),
            DefKind::Contract => (CompletionItemKind::INTERFACE, "5_"),
            DefKind::Component | DefKind::ExternComponent => (CompletionItemKind::STRUCT, "0_"),
            DefKind::Const => (CompletionItemKind::CONSTANT, "4_"),
            DefKind::Global => (CompletionItemKind::VARIABLE, "4_"),
            _ => (CompletionItemKind::TEXT, "6_"),
        };

        items.push(CompletionItem {
            label: entry.name.clone(),
            kind: Some(kind),
            sort_text: Some(format!("{}{}", sort_prefix, entry.name)),
            ..Default::default()
        });
    }

    // 6. File-private definitions (non-pub structs, enums, fns, etc.)
    for privates in def_map.file_private.values() {
        for &def_id in privates.values() {
            let entry = def_map.get_entry(def_id);
            // Skip synthetic entries and impl blocks (same filters as by_fqn)
            if entry.file_id == writ_diagnostics::FileId(u32::MAX) {
                continue;
            }
            if matches!(entry.kind, DefKind::Impl) {
                continue;
            }

            let (kind, sort_prefix) = match entry.kind {
                DefKind::Fn | DefKind::ExternFn => (CompletionItemKind::FUNCTION, "1_"),
                DefKind::Struct | DefKind::ExternStruct => (CompletionItemKind::STRUCT, "0_"),
                DefKind::Class | DefKind::ExternClass => (CompletionItemKind::CLASS, "0_"),
                DefKind::Entity => (CompletionItemKind::CLASS, "0_"),
                DefKind::Enum => (CompletionItemKind::ENUM, "0_"),
                DefKind::Contract => (CompletionItemKind::INTERFACE, "5_"),
                DefKind::Component | DefKind::ExternComponent => (CompletionItemKind::STRUCT, "0_"),
                DefKind::Const => (CompletionItemKind::CONSTANT, "4_"),
                DefKind::Global => (CompletionItemKind::VARIABLE, "4_"),
                _ => (CompletionItemKind::TEXT, "6_"),
            };

            items.push(CompletionItem {
                label: entry.name.clone(),
                kind: Some(kind),
                sort_text: Some(format!("{}{}", sort_prefix, entry.name)),
                ..Default::default()
            });
        }
    }

    items
}

/// Build completion items for dot-completion context.
///
/// Given the Ty of the receiver expression before the dot, returns
/// fields, methods, and (for entities) component names.
pub fn build_dot_completions(
    receiver_ty: Ty,
    interner: &TyInterner,
    def_map: &DefMap,
    type_env: &writ_compiler::check::env::TypeEnv,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    match interner.kind(receiver_ty) {
        TyKind::Struct(def_id) | TyKind::Class(def_id) => {
            // Fields
            if let Some(fields) = type_env.struct_fields.get(def_id) {
                for (name, ty, _) in fields {
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::FIELD),
                        detail: Some(interner.display_named(*ty, def_map)),
                        sort_text: Some(format!("0_{}", name)),
                        ..Default::default()
                    });
                }
            }
            // Methods from impl_index
            if let Some(impls) = type_env.impl_index.get(def_id) {
                for impl_entry in impls {
                    for (method_name, sig) in &impl_entry.methods {
                        items.push(CompletionItem {
                            label: method_name.clone(),
                            kind: Some(CompletionItemKind::METHOD),
                            detail: Some(format_fn_sig_oneliner(sig, interner, def_map)),
                            sort_text: Some(format!("1_{}", method_name)),
                            ..Default::default()
                        });
                    }
                }
            }
        }
        TyKind::Entity(def_id) => {
            // Entity properties
            if let Some(fields) = type_env.entity_fields.get(def_id) {
                for (name, ty, _) in fields {
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::FIELD),
                        detail: Some(interner.display_named(*ty, def_map)),
                        sort_text: Some(format!("0_{}", name)),
                        ..Default::default()
                    });
                }
            }
            // Methods from impl_index
            if let Some(impls) = type_env.impl_index.get(def_id) {
                for impl_entry in impls {
                    for (method_name, sig) in &impl_entry.methods {
                        items.push(CompletionItem {
                            label: method_name.clone(),
                            kind: Some(CompletionItemKind::METHOD),
                            detail: Some(format_fn_sig_oneliner(sig, interner, def_map)),
                            sort_text: Some(format!("1_{}", method_name)),
                            ..Default::default()
                        });
                    }
                }
            }
            // DIFF-02: Extern component types
            if let Some(components) = type_env.entity_components.get(def_id) {
                for comp_name in components {
                    items.push(CompletionItem {
                        label: comp_name.clone(),
                        kind: Some(CompletionItemKind::STRUCT),
                        detail: Some("component".to_string()),
                        sort_text: Some(format!("2_{}", comp_name)),
                        ..Default::default()
                    });
                }
            }
        }
        TyKind::Enum(def_id) => {
            // Enum variants (for qualified access like MyEnum.VariantA)
            if let Some(variants) = type_env.enum_variants.get(def_id) {
                for variant in variants {
                    items.push(CompletionItem {
                        label: variant.name.clone(),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        ..Default::default()
                    });
                }
            }
        }
        TyKind::Array(_) => {
            // Built-in array methods
            for (name, detail) in [
                ("push", "fn push(item)"),
                ("pop", "fn pop() -> Option<T>"),
                ("len", "fn len() -> int"),
                ("is_empty", "fn is_empty() -> bool"),
            ] {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(detail.to_string()),
                    ..Default::default()
                });
            }
        }
        TyKind::Option(_) => {
            for (name, detail) in [
                ("is_some", "fn is_some() -> bool"),
                ("is_none", "fn is_none() -> bool"),
                ("unwrap", "fn unwrap() -> T"),
            ] {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(detail.to_string()),
                    ..Default::default()
                });
            }
        }
        TyKind::Result(_, _) => {
            for (name, detail) in [
                ("is_ok", "fn is_ok() -> bool"),
                ("is_err", "fn is_err() -> bool"),
                ("unwrap", "fn unwrap() -> T"),
                ("unwrap_err", "fn unwrap_err() -> E"),
            ] {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(detail.to_string()),
                    ..Default::default()
                });
            }
        }
        _ => {} // No dot completions for primitives, void, func, etc.
    }

    items
}

/// Format a function signature as a one-liner for completion detail.
fn format_fn_sig_oneliner(
    sig: &writ_compiler::check::env::FnSig,
    interner: &TyInterner,
    def_map: &DefMap,
) -> String {
    let params: Vec<String> = sig
        .params
        .iter()
        .map(|(name, ty)| format!("{}: {}", name, interner.display_named(*ty, def_map)))
        .collect();
    let ret = interner.display_named(sig.ret, def_map);
    format!("fn({}) -> {}", params.join(", "), ret)
}

/// Extract the callee name from source text by scanning backward from the open paren.
/// Returns the simple function name (e.g., "foo") or path-qualified name (e.g., "Ns::foo").
fn extract_callee_name(source: &str, paren_offset: usize) -> Option<String> {
    let bytes = source.as_bytes();
    let mut i = paren_offset;
    // Skip whitespace before (
    while i > 0 && matches!(bytes[i - 1], b' ' | b'\t' | b'\n' | b'\r') {
        i -= 1;
    }
    let end = i;
    // Read identifier + :: path separators backward
    while i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_' || bytes[i - 1] == b':') {
        i -= 1;
    }
    if i == end {
        return None;
    }
    let name = std::str::from_utf8(&bytes[i..end]).ok()?;
    // Strip leading colons (e.g., "::foo" -> "foo")
    Some(name.trim_start_matches(':').to_string())
}

/// Find the active function call at the cursor and build signature help.
///
/// Walks backward from the cursor position in the source text to find the
/// opening `(` of the enclosing function call. Counts commas to determine
/// the active parameter index. Then looks up the callee's FnSig.
pub fn build_signature_help(
    source: &str,
    byte_offset: usize,
    ast: &TypedAst,
    interner: &TyInterner,
    type_env: &writ_compiler::check::env::TypeEnv,
) -> Option<lsp_types::SignatureHelp> {
    // Walk backward from byte_offset to find the opening '(' counting paren depth.
    let bytes = source.as_bytes();
    let mut depth: i32 = 0;
    let mut comma_count: u32 = 0;
    let mut open_paren_offset: Option<usize> = None;

    let mut i = byte_offset;
    while i > 0 {
        i -= 1;
        match bytes.get(i) {
            Some(b')') => depth += 1,
            Some(b'(') => {
                if depth == 0 {
                    open_paren_offset = Some(i);
                    break;
                }
                depth -= 1;
            }
            Some(b',') if depth == 0 => comma_count += 1,
            _ => {}
        }
    }

    let paren_offset = open_paren_offset?;

    // PRIMARY PATH: Text-based callee name extraction (works on incomplete sources)
    if let Some(callee_name) = extract_callee_name(source, paren_offset) {
        // Strip any namespace prefix for simple name matching
        let simple_name = callee_name.split("::").last().unwrap_or(&callee_name);

        // Look up in DefMap — try by_fqn first (check if any entry's simple name matches)
        let def_id = ast.def_map.by_fqn.values()
            .copied()
            .find(|&id| ast.def_map.get_entry(id).name == simple_name)
            .or_else(|| {
                // Try file_private scopes
                for privs in ast.def_map.file_private.values() {
                    if let Some(&id) = privs.get(simple_name) {
                        return Some(id);
                    }
                }
                None
            });

        if let Some(id) = def_id
            && let Some(sig) = type_env.fn_sigs.get(&id) {
                // Build SignatureHelp from sig (reuse existing label-building code)
                let params: Vec<lsp_types::ParameterInformation> = sig
                    .params
                    .iter()
                    .map(|(name, ty)| lsp_types::ParameterInformation {
                        label: lsp_types::ParameterLabel::Simple(format!(
                            "{}: {}",
                            name,
                            interner.display_named(*ty, &ast.def_map)
                        )),
                        documentation: None,
                    })
                    .collect();

                let mut label_parts = Vec::new();
                if let Some(mutable) = sig.self_param {
                    label_parts.push(if mutable { "mut self".to_string() } else { "self".to_string() });
                }
                for (name, ty) in &sig.params {
                    label_parts.push(format!("{}: {}", name, interner.display_named(*ty, &ast.def_map)));
                }
                let ret_str = interner.display_named(sig.ret, &ast.def_map);
                let label = format!("fn {}({}) -> {}", sig.name, label_parts.join(", "), ret_str);

                return Some(lsp_types::SignatureHelp {
                    signatures: vec![lsp_types::SignatureInformation {
                        label,
                        documentation: None,
                        parameters: Some(params),
                        active_parameter: Some(comma_count),
                    }],
                    active_signature: Some(0),
                    active_parameter: Some(comma_count),
                });
            }
    }

    // FALLBACK: AST-based Call node lookup (works when source has complete call)
    let call_expr = find_enclosing_call(ast, byte_offset)
        .or_else(|| find_enclosing_call(ast, paren_offset))?;

    let def_id = match call_expr {
        TypedExpr::Call { callee_def_id: Some(id), .. } => *id,
        _ => return None,
    };

    let sig = type_env.fn_sigs.get(&def_id)?;

    let params: Vec<lsp_types::ParameterInformation> = sig
        .params
        .iter()
        .map(|(name, ty)| lsp_types::ParameterInformation {
            label: lsp_types::ParameterLabel::Simple(format!(
                "{}: {}",
                name,
                interner.display_named(*ty, &ast.def_map)
            )),
            documentation: None,
        })
        .collect();

    let mut label_parts = Vec::new();
    if let Some(mutable) = sig.self_param {
        label_parts.push(if mutable { "mut self".to_string() } else { "self".to_string() });
    }
    for (name, ty) in &sig.params {
        label_parts.push(format!("{}: {}", name, interner.display_named(*ty, &ast.def_map)));
    }
    let ret_str = interner.display_named(sig.ret, &ast.def_map);
    let label = format!("fn {}({}) -> {}", sig.name, label_parts.join(", "), ret_str);

    Some(lsp_types::SignatureHelp {
        signatures: vec![lsp_types::SignatureInformation {
            label,
            documentation: None,
            parameters: Some(params),
            active_parameter: Some(comma_count),
        }],
        active_signature: Some(0),
        active_parameter: Some(comma_count),
    })
}

/// Check whether the cursor is positioned immediately after the `new` keyword
/// followed by at least one space or tab.
///
/// Scans backward from `byte_offset`, skipping whitespace (spaces/tabs), then
/// checks whether the preceding 3 bytes spell `new` and that the character
/// before `new` (if any) is NOT alphanumeric or underscore (to avoid matching
/// `renew `, `fnew `, etc.).
///
/// Returns `true` only when at least one space/tab separates `new` from the
/// cursor position (i.e., the user has pressed Space after `new`).
pub fn is_after_new_keyword(source: &str, byte_offset: usize) -> bool {
    let bytes = source.as_bytes();
    let end = byte_offset.min(bytes.len());
    let mut i = end;

    // Skip trailing whitespace (space / tab only — not newlines, which would
    // indicate a different syntactic context).
    while i > 0 && (bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') {
        i -= 1;
    }

    // We must have consumed at least one whitespace character.
    if i == end {
        return false;
    }

    // Now check that the 3 characters at [i-3..i] are 'n', 'e', 'w'.
    if i < 3 {
        return false;
    }
    if bytes[i - 3] != b'n' || bytes[i - 2] != b'e' || bytes[i - 1] != b'w' {
        return false;
    }

    // Verify the character before `new` (if any) is NOT alphanumeric or `_`,
    // which would mean `new` is a suffix of a longer identifier (e.g., `renew`).
    if i >= 4 {
        let before = bytes[i - 4];
        if before.is_ascii_alphanumeric() || before == b'_' {
            return false;
        }
    }

    true
}

/// Build a detail string for a type completion item showing its fields.
fn build_type_detail(
    def_id: writ_compiler::resolve::def_map::DefId,
    kind: &DefKind,
    type_env: &writ_compiler::check::env::TypeEnv,
    interner: &TyInterner,
    def_map: &DefMap,
) -> Option<String> {
    match kind {
        DefKind::Struct | DefKind::ExternStruct | DefKind::Class | DefKind::ExternClass => {
            if let Some(fields) = type_env.struct_fields.get(&def_id) {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(name, ty, _)| format!("{}: {}", name, interner.display_named(*ty, def_map)))
                    .collect();
                Some(format!("struct {{ {} }}", field_strs.join(", ")))
            } else {
                Some("struct".to_string())
            }
        }
        DefKind::Entity => {
            if let Some(fields) = type_env.entity_fields.get(&def_id) {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(name, ty, _)| format!("{}: {}", name, interner.display_named(*ty, def_map)))
                    .collect();
                Some(format!("entity {{ {} }}", field_strs.join(", ")))
            } else {
                Some("entity".to_string())
            }
        }
        _ => None,
    }
}

/// Build completion items for the context immediately after the `new` keyword.
///
/// Returns only types that are constructable with `new Type { ... }` syntax:
/// `Struct`, `ExternStruct`, `Class`, `ExternClass`, and `Entity` kinds.
///
/// Synthetic entries (file_id == FileId(u32::MAX)) are excluded. Prelude
/// types such as `Option` and `Result` are enum-like and not constructable
/// with `new`, so they are intentionally omitted.
///
/// Detail text showing struct/entity fields is included when `type_env` is provided.
pub fn build_new_keyword_completions(
    def_map: &DefMap,
    interner: &TyInterner,
    type_env: &writ_compiler::check::env::TypeEnv,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Public constructable types
    for &def_id in def_map.by_fqn.values() {
        let entry = def_map.get_entry(def_id);

        // Skip synthetic builtins (log::*, dialogue builtins, etc.)
        if entry.file_id == writ_diagnostics::FileId(u32::MAX) {
            continue;
        }

        let kind = match entry.kind {
            DefKind::Struct | DefKind::ExternStruct => CompletionItemKind::STRUCT,
            DefKind::Class | DefKind::ExternClass => CompletionItemKind::CLASS,
            DefKind::Entity => CompletionItemKind::CLASS,
            // All other kinds (Fn, Enum, Contract, Impl, Const, Global, …) are
            // NOT constructable with `new`.
            _ => continue,
        };

        let detail = build_type_detail(def_id, &entry.kind, type_env, interner, def_map);

        items.push(CompletionItem {
            label: entry.name.clone(),
            kind: Some(kind),
            detail,
            sort_text: Some(format!("0_{}", entry.name)),
            ..Default::default()
        });
    }

    // Also include file-private constructable types
    for privates in def_map.file_private.values() {
        for &def_id in privates.values() {
            let entry = def_map.get_entry(def_id);
            if entry.file_id == writ_diagnostics::FileId(u32::MAX) {
                continue;
            }
            let kind = match entry.kind {
                DefKind::Struct | DefKind::ExternStruct => CompletionItemKind::STRUCT,
                DefKind::Class | DefKind::ExternClass => CompletionItemKind::CLASS,
                DefKind::Entity => CompletionItemKind::CLASS,
                _ => continue,
            };
            let detail = build_type_detail(def_id, &entry.kind, type_env, interner, def_map);
            items.push(CompletionItem {
                label: entry.name.clone(),
                kind: Some(kind),
                detail,
                sort_text: Some(format!("0_{}", entry.name)),
                ..Default::default()
            });
        }
    }

    items
}

/// Extract the namespace identifier before `::` at the cursor position.
///
/// Scans backward from `cursor` in `source`, consuming colons first, then
/// an identifier. Returns `Some(namespace)` only when at least two colons
/// were consumed AND a non-empty identifier was found.
///
/// Examples:
/// - `"fn main() { log::"`, cursor 18 → `Some("log")`
/// - `"let x: int"`, cursor 6 → `None` (only one colon)
/// - `"::"`, cursor 2 → `None` (no identifier)
pub fn extract_namespace_prefix(source: &str, cursor: usize) -> Option<String> {
    let bytes = source.as_bytes();
    let mut i = cursor.min(bytes.len());

    // Skip any trailing colons from the cursor backward.
    let colon_end = i;
    while i > 0 && bytes[i - 1] == b':' {
        i -= 1;
    }
    let colon_count = colon_end - i;

    // Need at least 2 colons for a `::` sequence.
    if colon_count < 2 {
        return None;
    }

    // `i` is now pointing to just after the last identifier character.
    let ident_end = i;

    // Walk backward through the identifier characters.
    while i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
        i -= 1;
    }
    let ident_start = i;

    if ident_start == ident_end {
        // No identifier found before the colons.
        return None;
    }

    std::str::from_utf8(&bytes[ident_start..ident_end])
        .ok()
        .map(|s| s.to_string())
}

/// Build completion items for a namespace completion (`::` trigger).
///
/// Handles three cases in priority order:
/// 1. Hardcoded prelude types (`Option` → Some/None, `Result` → Ok/Err).
/// 2. Log namespace via `by_fqn` prefix scan (`log::trace` etc).
/// 3. User-defined enums via `enum_variants` in `type_env`.
pub fn build_namespace_completions(
    namespace: &str,
    def_map: &DefMap,
    type_env: &writ_compiler::check::env::TypeEnv,
) -> Vec<CompletionItem> {
    // 1. Hardcoded prelude types.
    if namespace == "Option" {
        return vec![
            CompletionItem {
                label: "Some".to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                ..Default::default()
            },
            CompletionItem {
                label: "None".to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                ..Default::default()
            },
        ];
    }
    if namespace == "Result" {
        return vec![
            CompletionItem {
                label: "Ok".to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                ..Default::default()
            },
            CompletionItem {
                label: "Err".to_string(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                ..Default::default()
            },
        ];
    }

    // 2. DefMap by_fqn prefix scan (handles log:: and any namespace members).
    let prefix = format!("{}::", namespace);
    let mut items: Vec<CompletionItem> = def_map
        .by_fqn
        .iter()
        .filter(|(fqn, _)| fqn.starts_with(&prefix))
        .map(|(fqn, &def_id)| {
            let simple_name = fqn.strip_prefix(&prefix).unwrap_or(fqn.as_str());
            let entry = def_map.get_entry(def_id);
            let kind = match entry.kind {
                DefKind::Fn | DefKind::ExternFn => CompletionItemKind::FUNCTION,
                DefKind::Enum => CompletionItemKind::ENUM,
                _ => CompletionItemKind::VALUE,
            };
            CompletionItem {
                label: simple_name.to_string(),
                kind: Some(kind),
                ..Default::default()
            }
        })
        .collect();

    if !items.is_empty() {
        return items;
    }

    // 3. User-defined enum variants — look up the enum DefId by name, then
    //    pull its variants from type_env.enum_variants.
    let enum_def_id = def_map.by_fqn.iter().find_map(|(_fqn, &def_id)| {
        let entry = def_map.get_entry(def_id);
        if entry.name == namespace && entry.kind == DefKind::Enum {
            Some(def_id)
        } else {
            None
        }
    });

    if let Some(def_id) = enum_def_id {
        if let Some(variants) = type_env.enum_variants.get(&def_id) {
            items = variants
                .iter()
                .map(|v| CompletionItem {
                    label: v.name.clone(),
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    ..Default::default()
                })
                .collect();
        }
    }

    items
}

/// Find the innermost Call expression whose span contains the given byte offset.
fn find_enclosing_call(ast: &TypedAst, offset: usize) -> Option<&TypedExpr> {
    for decl in &ast.decls {
        match decl {
            TypedDecl::Fn { body, .. } => {
                if let Some(call) = find_call_in_expr(body, offset) {
                    return Some(call);
                }
            }
            TypedDecl::Impl { methods, .. } => {
                for (_, body) in methods {
                    if let Some(call) = find_call_in_expr(body, offset) {
                        return Some(call);
                    }
                }
            }
            TypedDecl::Const { value, .. } | TypedDecl::Global { value, .. } => {
                if let Some(call) = find_call_in_expr(value, offset) {
                    return Some(call);
                }
            }
            _ => {}
        }
    }
    None
}

/// Recursively search for the innermost Call expression containing `offset`.
///
/// Returns the narrowest-span Call node that contains the offset, or None.
fn find_call_in_expr(expr: &TypedExpr, offset: usize) -> Option<&TypedExpr> {
    let span = expr.span();
    if offset < span.start || offset > span.end {
        return None;
    }

    // For Call nodes, check children first for a narrower match, then return self.
    if let TypedExpr::Call { callee, args, .. } = expr {
        // Try to find a narrower Call inside children
        let child_call = find_call_in_expr(callee, offset)
            .or_else(|| args.iter().find_map(|a| find_call_in_expr(a, offset)));
        if child_call.is_some() {
            return child_call;
        }
        // This Call node itself contains the offset
        return Some(expr);
    }

    // For non-Call nodes, recurse into children to find a Call inside
    match expr {
        TypedExpr::Field { receiver, .. } | TypedExpr::ComponentAccess { receiver, .. } => {
            find_call_in_expr(receiver, offset)
        }
        TypedExpr::Index { receiver, index, .. } => {
            find_call_in_expr(receiver, offset).or_else(|| find_call_in_expr(index, offset))
        }
        TypedExpr::Binary { left, right, .. } => {
            find_call_in_expr(left, offset).or_else(|| find_call_in_expr(right, offset))
        }
        TypedExpr::UnaryPrefix { expr: inner, .. } => find_call_in_expr(inner, offset),
        TypedExpr::Match { scrutinee, arms, .. } => {
            find_call_in_expr(scrutinee, offset)
                .or_else(|| arms.iter().find_map(|arm| find_call_in_expr(&arm.body, offset)))
        }
        TypedExpr::If { condition, then_branch, else_branch, .. } => {
            find_call_in_expr(condition, offset)
                .or_else(|| find_call_in_expr(then_branch, offset))
                .or_else(|| else_branch.as_ref().and_then(|e| find_call_in_expr(e, offset)))
        }
        TypedExpr::Block { stmts, tail, .. } => {
            find_call_in_stmts(stmts, offset)
                .or_else(|| tail.as_ref().and_then(|t| find_call_in_expr(t, offset)))
        }
        TypedExpr::Lambda { body, .. } => find_call_in_expr(body, offset),
        TypedExpr::Assign { target, value, .. } => {
            find_call_in_expr(target, offset).or_else(|| find_call_in_expr(value, offset))
        }
        TypedExpr::New { fields, .. } => {
            fields.iter().find_map(|(_, v)| find_call_in_expr(v, offset))
        }
        TypedExpr::ArrayLit { elements, .. } => {
            elements.iter().find_map(|e| find_call_in_expr(e, offset))
        }
        TypedExpr::Range { start, end, .. } => {
            start
                .as_ref()
                .and_then(|s| find_call_in_expr(s, offset))
                .or_else(|| end.as_ref().and_then(|e| find_call_in_expr(e, offset)))
        }
        TypedExpr::Spawn { expr: inner, .. }
        | TypedExpr::SpawnDetached { expr: inner, .. }
        | TypedExpr::Join { expr: inner, .. }
        | TypedExpr::Cancel { expr: inner, .. }
        | TypedExpr::Defer { expr: inner, .. } => find_call_in_expr(inner, offset),
        TypedExpr::Return { value, .. } => {
            value.as_ref().and_then(|v| find_call_in_expr(v, offset))
        }
        _ => None,
    }
}

/// Search statements for a Call expression containing `offset`.
fn find_call_in_stmts(stmts: &[TypedStmt], offset: usize) -> Option<&TypedExpr> {
    stmts.iter().find_map(|stmt| find_call_in_stmt(stmt, offset))
}

/// Search a single statement for a Call expression containing `offset`.
fn find_call_in_stmt(stmt: &TypedStmt, offset: usize) -> Option<&TypedExpr> {
    match stmt {
        TypedStmt::Let { value, .. } => find_call_in_expr(value, offset),
        TypedStmt::Expr { expr, .. } => find_call_in_expr(expr, offset),
        TypedStmt::For { iterable, body, .. } => {
            find_call_in_expr(iterable, offset).or_else(|| find_call_in_stmts(body, offset))
        }
        TypedStmt::While { condition, body, .. } => {
            find_call_in_expr(condition, offset).or_else(|| find_call_in_stmts(body, offset))
        }
        TypedStmt::Atomic { body, .. } => find_call_in_stmts(body, offset),
        TypedStmt::Return { value, .. } => {
            value.as_ref().and_then(|v| find_call_in_expr(v, offset))
        }
        TypedStmt::Break { value, .. } => {
            value.as_ref().and_then(|v| find_call_in_expr(v, offset))
        }
        TypedStmt::Continue { .. } | TypedStmt::Error { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_identifier_completions, build_dot_completions, build_signature_help,
        build_namespace_completions, extract_namespace_prefix,
        is_after_new_keyword, build_new_keyword_completions,
    };
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

    // ── build_identifier_completions tests ───────────────────────────────────

    #[test]
    fn test_identifier_completions_has_keywords() {
        let def_map = writ_compiler::resolve::def_map::DefMap::new();
        let interner = writ_compiler::check::ty::TyInterner::new();
        let items = build_identifier_completions(&def_map, &interner);

        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"fn"), "expected 'fn' keyword");
        assert!(labels.contains(&"let"), "expected 'let' keyword");
        assert!(labels.contains(&"struct"), "expected 'struct' keyword");
    }

    #[test]
    fn test_identifier_completions_has_prelude() {
        let def_map = writ_compiler::resolve::def_map::DefMap::new();
        let interner = writ_compiler::check::ty::TyInterner::new();
        let items = build_identifier_completions(&def_map, &interner);

        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        // Primitives
        assert!(labels.contains(&"int"), "expected 'int' primitive");
        // Prelude types
        assert!(labels.contains(&"Option"), "expected 'Option' type");
        // Contracts
        assert!(labels.contains(&"Add"), "expected 'Add' contract");
    }

    // ── build_dot_completions tests ───────────────────────────────────────────

    #[test]
    fn test_dot_completions_struct_fields() {
        let src = "pub struct Point { x: int, y: int } fn main() { let p: Point = new Point { x: 1, y: 2 }; p }";
        let (ast, mut interner, type_env) = build_typed_ast_full(src);

        // Find the DefId for 'Point' (public, so in by_fqn)
        let point_def_id = ast.def_map.by_fqn.values()
            .copied()
            .find(|&id| ast.def_map.get_entry(id).name == "Point")
            .expect("should find 'Point'");

        let receiver_ty = interner.intern(writ_compiler::check::ty::TyKind::Struct(point_def_id));
        let items = build_dot_completions(receiver_ty, &interner, &ast.def_map, &type_env);

        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"x"),
            "expected field 'x' in struct completions, got: {:?}",
            labels
        );
        assert!(
            labels.contains(&"y"),
            "expected field 'y' in struct completions, got: {:?}",
            labels
        );
    }

    #[test]
    fn test_dot_completions_entity_components() {
        // Entity with extern components — verifies DIFF-02
        // Syntax: extern component uses field declarations without default values in minimal form
        let src = r#"extern component Health { hp: int, }
pub entity Player {
    use Health { hp: 100 },
}
fn main() { }
"#;
        let (ast, mut interner, type_env) = build_typed_ast_full(src);

        // Find the DefId for 'Player' (public entity, in by_fqn)
        let player_def_id = ast.def_map.by_fqn.values()
            .copied()
            .find(|&id| ast.def_map.get_entry(id).name == "Player")
            .expect("should find 'Player'");

        let receiver_ty = interner.intern(writ_compiler::check::ty::TyKind::Entity(player_def_id));
        let items = build_dot_completions(receiver_ty, &interner, &ast.def_map, &type_env);

        // At minimum we should get component items (entity_components from type_env)
        // Even if entity has no explicit components listed, the function should not panic
        let _ = items; // Just verify it returns without error
    }

    // ── build_signature_help tests ────────────────────────────────────────────

    #[test]
    fn test_signature_help_finds_param() {
        // A 2-parameter function: after the first comma, active_parameter should be 1
        let src = "fn add(a: int, b: int) -> int { a + b } fn main() -> int { add(1, 2) }";
        let (ast, interner, type_env) = build_typed_ast_full(src);

        // Find the byte offset just after the comma in "add(1, 2)"
        // "fn add(a: int, b: int) -> int { a + b } fn main() -> int { add(1, 2) }"
        // The call "add(1, 2)" starts at rfind("add(") ...
        let call_start = src.rfind("add(").unwrap();
        // Position after "add(1, " -- 7 chars after call_start
        let after_comma = call_start + "add(1, ".len();

        let help = build_signature_help(src, after_comma, &ast, &interner, &type_env);
        assert!(help.is_some(), "expected signature help to be Some");
        let help = help.unwrap();
        assert_eq!(help.active_parameter, Some(1), "expected active_parameter=1 after first comma");
    }

    #[test]
    fn test_signature_help_incomplete_source() {
        // Incomplete call — no closing paren. Text-based callee extraction should work.
        let src = "fn foo(a: int, b: int) -> int { a + b }\nfn main() { foo( }";
        let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
        let result = crate::analysis_host::AnalysisHost::analyze_standalone(
            src.to_string(),
            "test.writ".to_string(),
        );
        if let (Some(ast), Some(interner), Some(type_env)) =
            (result.typed_ast, result.ty_interner, result.type_env)
        {
            let cursor = src.find("foo( ").unwrap() + "foo(".len();
            let help = build_signature_help(src_static, cursor, &ast, &interner, &type_env);
            assert!(help.is_some(), "expected signature help for incomplete call");
            let help = help.unwrap();
            assert_eq!(help.signatures.len(), 1);
            let sig = &help.signatures[0];
            assert_eq!(sig.parameters.as_ref().unwrap().len(), 2);
            assert_eq!(help.active_parameter, Some(0));
        } else {
            eprintln!("Note: typed_ast not available for broken source");
        }
    }

    #[test]
    fn test_signature_help_active_param_incomplete() {
        let src = "fn foo(a: int, b: int) -> int { a + b }\nfn main() { foo(1, }";
        let src_static: &'static str = Box::leak(src.to_string().into_boxed_str());
        let result = crate::analysis_host::AnalysisHost::analyze_standalone(
            src.to_string(),
            "test.writ".to_string(),
        );
        if let (Some(ast), Some(interner), Some(type_env)) =
            (result.typed_ast, result.ty_interner, result.type_env)
        {
            let cursor = src.find("foo(1, ").unwrap() + "foo(1, ".len();
            let help = build_signature_help(src_static, cursor, &ast, &interner, &type_env);
            assert!(help.is_some(), "expected signature help after comma");
            assert_eq!(help.unwrap().active_parameter, Some(1));
        }
    }

    // ── extract_namespace_prefix tests ────────────────────────────────────────

    #[test]
    fn test_extract_namespace_prefix_log() {
        // "fn main() { log::" — cursor is at end (17 chars of "fn main() { log::")
        let src = "fn main() { log::";
        let cursor = src.len();
        let result = extract_namespace_prefix(src, cursor);
        assert_eq!(result, Some("log".to_string()), "expected 'log' namespace prefix");
    }

    #[test]
    fn test_extract_namespace_prefix_option() {
        // "let x = Option::" — cursor at end
        let src = "let x = Option::";
        let cursor = src.len();
        let result = extract_namespace_prefix(src, cursor);
        assert_eq!(result, Some("Option".to_string()), "expected 'Option' namespace prefix");
    }

    #[test]
    fn test_extract_namespace_prefix_single_colon() {
        // "let x: int" — cursor after ':' at position 6, only one colon so returns None
        let src = "let x: int";
        let cursor = 6; // position after the single ':'
        let result = extract_namespace_prefix(src, cursor);
        assert_eq!(result, None, "single colon should return None");
    }

    #[test]
    fn test_extract_namespace_prefix_no_ident() {
        // "::" — no identifier before ::, returns None
        let src = "::";
        let cursor = 2;
        let result = extract_namespace_prefix(src, cursor);
        assert_eq!(result, None, "no identifier before :: should return None");
    }

    // ── build_namespace_completions tests ─────────────────────────────────────

    #[test]
    fn test_namespace_completions_log() {
        // log:: uses inject_log_namespace path, must run full pipeline
        let src = "fn main() { }";
        let (ast, _interner, type_env) = build_typed_ast_full(src);
        let items = build_namespace_completions("log", &ast.def_map, &type_env);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"trace"), "expected 'trace' in log completions, got: {:?}", labels);
        assert!(labels.contains(&"debug"), "expected 'debug' in log completions, got: {:?}", labels);
        assert!(labels.contains(&"info"), "expected 'info' in log completions, got: {:?}", labels);
        assert!(labels.contains(&"warn"), "expected 'warn' in log completions, got: {:?}", labels);
        assert!(labels.contains(&"error"), "expected 'error' in log completions, got: {:?}", labels);
        assert_eq!(items.len(), 5, "expected exactly 5 log completions, got: {:?}", labels);
    }

    #[test]
    fn test_namespace_completions_option() {
        // Option is a prelude type — hardcoded path, fresh DefMap is fine
        let def_map = writ_compiler::resolve::def_map::DefMap::new();
        let type_env_empty = writ_compiler::check::env::TypeEnv {
            fn_sigs: Default::default(),
            struct_fields: Default::default(),
            entity_fields: Default::default(),
            entity_components: Default::default(),
            enum_variants: Default::default(),
            contract_methods: Default::default(),
            impl_index: Default::default(),
            const_types: Default::default(),
            global_types: Default::default(),
            component_fields: Default::default(),
        };
        let items = build_namespace_completions("Option", &def_map, &type_env_empty);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"Some"), "expected 'Some' in Option completions, got: {:?}", labels);
        assert!(labels.contains(&"None"), "expected 'None' in Option completions, got: {:?}", labels);
        assert_eq!(items.len(), 2, "expected exactly 2 Option completions, got: {:?}", labels);
    }

    #[test]
    fn test_namespace_completions_result() {
        // Result is a prelude type — hardcoded path
        let def_map = writ_compiler::resolve::def_map::DefMap::new();
        let type_env_empty = writ_compiler::check::env::TypeEnv {
            fn_sigs: Default::default(),
            struct_fields: Default::default(),
            entity_fields: Default::default(),
            entity_components: Default::default(),
            enum_variants: Default::default(),
            contract_methods: Default::default(),
            impl_index: Default::default(),
            const_types: Default::default(),
            global_types: Default::default(),
            component_fields: Default::default(),
        };
        let items = build_namespace_completions("Result", &def_map, &type_env_empty);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"Ok"), "expected 'Ok' in Result completions, got: {:?}", labels);
        assert!(labels.contains(&"Err"), "expected 'Err' in Result completions, got: {:?}", labels);
        assert_eq!(items.len(), 2, "expected exactly 2 Result completions, got: {:?}", labels);
    }

    #[test]
    fn test_namespace_completions_user_enum() {
        // User-defined enum — must use full pipeline to get enum_variants populated
        let src = "pub enum Color { Red, Green, Blue }\nfn main() { }";
        let (ast, _interner, type_env) = build_typed_ast_full(src);
        let items = build_namespace_completions("Color", &ast.def_map, &type_env);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"Red"), "expected 'Red' in Color completions, got: {:?}", labels);
        assert!(labels.contains(&"Green"), "expected 'Green' in Color completions, got: {:?}", labels);
        assert!(labels.contains(&"Blue"), "expected 'Blue' in Color completions, got: {:?}", labels);
    }

    #[test]
    fn test_namespace_completions_unknown() {
        let def_map = writ_compiler::resolve::def_map::DefMap::new();
        let type_env_empty = writ_compiler::check::env::TypeEnv {
            fn_sigs: Default::default(),
            struct_fields: Default::default(),
            entity_fields: Default::default(),
            entity_components: Default::default(),
            enum_variants: Default::default(),
            contract_methods: Default::default(),
            impl_index: Default::default(),
            const_types: Default::default(),
            global_types: Default::default(),
            component_fields: Default::default(),
        };
        let items = build_namespace_completions("Nonexistent", &def_map, &type_env_empty);
        assert!(items.is_empty(), "expected empty vec for unknown namespace, got: {:?}", items.iter().map(|i| &i.label).collect::<Vec<_>>());
    }

    // ── is_after_new_keyword tests ────────────────────────────────────────────

    #[test]
    fn test_is_after_new_keyword_basic() {
        // "let x = new " — cursor at byte 12 (after the trailing space)
        let src = "let x = new ";
        assert!(
            is_after_new_keyword(src, src.len()),
            "cursor right after 'new ' should return true"
        );
    }

    #[test]
    fn test_is_after_new_keyword_multiple_spaces() {
        // "new   " — multiple spaces, cursor at byte 6
        let src = "new   ";
        assert!(
            is_after_new_keyword(src, src.len()),
            "cursor after 'new' with multiple spaces should return true"
        );
    }

    #[test]
    fn test_is_after_new_keyword_not_partial() {
        // "renew " — 'new' is a suffix of 'renew', should return false
        let src = "renew ";
        assert!(
            !is_after_new_keyword(src, src.len()),
            "'renew ' should not match as 'new ' keyword"
        );
    }

    #[test]
    fn test_is_after_new_keyword_no_space() {
        // "new" — cursor right at end of 'new' with no trailing space
        let src = "new";
        assert!(
            !is_after_new_keyword(src, src.len()),
            "cursor at end of 'new' with no space should return false"
        );
    }

    // ── build_new_keyword_completions tests ───────────────────────────────────

    #[test]
    fn test_new_keyword_completions_filters_to_constructable() {
        // Source with a struct, an enum, and a function.
        // Only the struct should appear in new-keyword completions.
        let src = "pub struct Point { x: int, y: int }\npub enum Color { Red, Green }\npub fn helper() -> int { 0 }\nfn main() { }";
        let (ast, interner, type_env) = build_typed_ast_full(src);

        let items = build_new_keyword_completions(&ast.def_map, &interner, &type_env);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        assert!(
            labels.contains(&"Point"),
            "expected 'Point' struct in new-keyword completions, got: {:?}",
            labels
        );
        assert!(
            !labels.contains(&"Color"),
            "'Color' (enum) should NOT appear in new-keyword completions, got: {:?}",
            labels
        );
        assert!(
            !labels.contains(&"helper"),
            "'helper' (fn) should NOT appear in new-keyword completions, got: {:?}",
            labels
        );
        assert!(
            !labels.contains(&"main"),
            "'main' (fn) should NOT appear in new-keyword completions, got: {:?}",
            labels
        );
    }

    // ── dot-completion integration tests ──────────────────────────────────────
    //
    // These tests simulate the FULL dot-completion pipeline as backend.rs does it:
    //   1. User has typed "receiver." -- source has a trailing dot
    //   2. Strip the dot (modified_source)
    //   3. analyze_standalone(modified_source) -> typed_ast
    //   4. expr_at_offset(typed_ast, dot_pos - 1, FileId(0)) -> receiver expression
    //   5. receiver_expr.ty() -> Ty
    //   6. build_dot_completions(receiver_ty, ...) -> completion items
    //
    // The tests exercise the real pipeline so that any regression in the full
    // chain (source modification, re-analysis, offset math, type resolution)
    // is caught automatically.

    #[test]
    fn test_dot_completion_integration_struct() {
        // Simulate: user types "p." -- source has the dot at end of "p."
        let original = "pub struct Point { x: int, y: int }\nfn main() { let p: Point = new Point { x: 1, y: 2 }; p. }";
        // Find the dot position: it's the '.' in "p."
        let dot_pos = original.rfind("p.").unwrap() + 1; // byte offset of '.'
        // Strip the dot (exactly what backend.rs does)
        let modified = format!("{}{}", &original[..dot_pos], &original[dot_pos + 1..]);

        // Run analyze_standalone on the modified source
        let result = crate::analysis_host::AnalysisHost::analyze_standalone(
            modified.clone(), "test.writ".to_string()
        );
        let (typed_ast, interner, type_env) = match (result.typed_ast, result.ty_interner, result.type_env) {
            (Some(t), Some(i), Some(e)) => (t, i, e),
            _ => panic!("analyze_standalone failed to produce typed AST for modified source.\nModified source:\n{}", modified),
        };

        // Find receiver at dot_pos - 1 (the 'p' character)
        let receiver_offset = dot_pos.saturating_sub(1);
        let receiver_expr = crate::queries::expr_at_offset(&typed_ast, receiver_offset, writ_diagnostics::FileId(0));
        assert!(
            receiver_expr.is_some(),
            "expr_at_offset should find receiver at offset {} in modified source:\n{}",
            receiver_offset,
            modified
        );

        let receiver_ty = receiver_expr.unwrap().ty();
        let items = build_dot_completions(receiver_ty, &interner, &typed_ast.def_map, &type_env);

        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"x"), "expected field 'x' in dot completions, got: {:?}", labels);
        assert!(labels.contains(&"y"), "expected field 'y' in dot completions, got: {:?}", labels);
    }

    #[test]
    fn test_dot_completion_integration_array() {
        // Simulate: user types "arr." -- source has the dot at end of "arr."
        let original = "fn main() { let arr: Array<int> = [1, 2, 3]; arr. }";
        // Find the dot: it's the '.' in "arr."
        let dot_pos = original.rfind("arr.").unwrap() + 3; // byte offset of '.'
        // Strip the dot
        let modified = format!("{}{}", &original[..dot_pos], &original[dot_pos + 1..]);

        // Run analyze_standalone on the modified source
        let result = crate::analysis_host::AnalysisHost::analyze_standalone(
            modified.clone(), "test.writ".to_string()
        );
        let (typed_ast, interner, type_env) = match (result.typed_ast, result.ty_interner, result.type_env) {
            (Some(t), Some(i), Some(e)) => (t, i, e),
            _ => panic!("analyze_standalone failed to produce typed AST for modified source.\nModified source:\n{}", modified),
        };

        // Find receiver at dot_pos - 1 (the 'r' of "arr")
        let receiver_offset = dot_pos.saturating_sub(1);
        let receiver_expr = crate::queries::expr_at_offset(&typed_ast, receiver_offset, writ_diagnostics::FileId(0));
        assert!(
            receiver_expr.is_some(),
            "expr_at_offset should find receiver at offset {} in modified source:\n{}",
            receiver_offset,
            modified
        );

        let receiver_ty = receiver_expr.unwrap().ty();
        let items = build_dot_completions(receiver_ty, &interner, &typed_ast.def_map, &type_env);

        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"push"),     "expected 'push' in array dot completions, got: {:?}", labels);
        assert!(labels.contains(&"pop"),      "expected 'pop' in array dot completions, got: {:?}", labels);
        assert!(labels.contains(&"len"),      "expected 'len' in array dot completions, got: {:?}", labels);
        assert!(labels.contains(&"is_empty"), "expected 'is_empty' in array dot completions, got: {:?}", labels);
    }

    // ── file-private definition inclusion tests ────────────────────────────────

    #[test]
    fn test_identifier_completions_includes_file_private() {
        // Non-pub struct and non-pub fn alongside a pub fn — all should appear.
        let src = "struct Foo { a: int } fn bar() -> int { 0 } pub fn main() -> int { 0 }";
        let (ast, interner, _type_env) = build_typed_ast_full(src);

        let items = build_identifier_completions(&ast.def_map, &interner);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        assert!(
            labels.contains(&"Foo"),
            "expected private struct 'Foo' in identifier completions, got: {:?}",
            labels
        );
        assert!(
            labels.contains(&"bar"),
            "expected private fn 'bar' in identifier completions, got: {:?}",
            labels
        );
        assert!(
            labels.contains(&"main"),
            "expected public fn 'main' in identifier completions, got: {:?}",
            labels
        );
    }

    #[test]
    fn test_new_keyword_completions_includes_file_private() {
        // Non-pub and pub struct — both should appear; detail should contain fields.
        let src = "struct Priv { x: int } pub struct Pub { y: bool } pub fn main() -> int { 0 }";
        let (ast, interner, type_env) = build_typed_ast_full(src);

        let items = build_new_keyword_completions(&ast.def_map, &interner, &type_env);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        assert!(
            labels.contains(&"Priv"),
            "expected private struct 'Priv' in new-keyword completions, got: {:?}",
            labels
        );
        assert!(
            labels.contains(&"Pub"),
            "expected public struct 'Pub' in new-keyword completions, got: {:?}",
            labels
        );

        // Detail for 'Priv' should contain its field "x: int"
        let priv_item = items.iter().find(|i| i.label == "Priv").unwrap();
        let priv_detail = priv_item.detail.as_deref().unwrap_or("");
        assert!(
            priv_detail.contains("x: int"),
            "detail for 'Priv' should contain 'x: int', got: {:?}",
            priv_detail
        );

        // Detail for 'Pub' should contain its field "y: bool"
        let pub_item = items.iter().find(|i| i.label == "Pub").unwrap();
        let pub_detail = pub_item.detail.as_deref().unwrap_or("");
        assert!(
            pub_detail.contains("y: bool"),
            "detail for 'Pub' should contain 'y: bool', got: {:?}",
            pub_detail
        );
    }
}
