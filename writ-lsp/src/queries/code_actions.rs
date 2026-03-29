//! Code action providers for the Writ LSP.
//!
//! Currently supports:
//! - **E0123** (`IncompleteContractImpl`): generates method stubs for missing contract methods.

use lsp_types::{
    CodeAction, CodeActionKind, NumberOrString, TextEdit, Url, WorkspaceEdit,
};
use std::collections::HashMap;

use writ_compiler::check::env::TypeEnv;
use writ_compiler::check::ty::TyInterner;
use writ_compiler::resolve::def_map::{DefKind, DefMap};

/// Build code actions for diagnostics at the given position.
///
/// `diags` are the LSP diagnostics sent by the client in the code action request.
/// Returns a list of quick-fix code actions (e.g., "Implement missing methods").
pub fn build_code_actions(
    diags: &[lsp_types::Diagnostic],
    uri: &Url,
    source: &str,
    type_env: &TypeEnv,
    interner: &TyInterner,
    def_map: &DefMap,
) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    for diag in diags {
        if let Some(NumberOrString::String(code)) = &diag.code {
            if code == "E0123" {
                if let Some(action) =
                    build_implement_missing_methods(diag, uri, source, type_env, interner, def_map)
                {
                    actions.push(action);
                }
            }
        }
    }

    actions
}

/// Build a quick-fix code action for E0123 (incomplete contract impl).
///
/// Parses the contract name and missing method names from the diagnostic,
/// looks up the full signatures in the TypeEnv, and generates stub code.
fn build_implement_missing_methods(
    diag: &lsp_types::Diagnostic,
    uri: &Url,
    source: &str,
    type_env: &TypeEnv,
    interner: &TyInterner,
    def_map: &DefMap,
) -> Option<CodeAction> {
    // Parse contract name from message: "incomplete implementation of contract `Foo` for `Bar`"
    let contract_name = extract_between(&diag.message, "contract `", "`")?;

    // Parse missing method names from the diagnostic message itself.
    // The help text is not included in the LSP diagnostic message field, so
    // we look for it in related_information or fall back to the message.
    // Actually, the help is NOT forwarded to the LSP diagnostic by writ_diag_to_lsp.
    // We'll extract missing methods from the TypeEnv instead by comparing against
    // what the impl already provides.

    // Find the contract DefId by name.
    let contract_def_id = def_map
        .arena
        .iter()
        .find(|(_, entry)| entry.name == contract_name && entry.kind == DefKind::Contract)?
        .0;

    // Get all required method signatures from the contract.
    let required_sigs = type_env.contract_methods.get(&contract_def_id)?;

    // Find what methods the impl block already provides.
    // Parse type name from the diagnostic message.
    let ty_name = extract_between(&diag.message, "for `", "`")?;

    // Find the target type's DefId.
    let target_def_id = def_map
        .arena
        .iter()
        .find(|(_, entry)| {
            entry.name == ty_name
                && matches!(
                    entry.kind,
                    DefKind::Struct | DefKind::Class | DefKind::Entity | DefKind::Enum
                )
        })?
        .0;

    // Get the impl entries for this type and find the one for our contract.
    let impl_entries = type_env.impl_index.get(&target_def_id)?;
    let impl_entry = impl_entries
        .iter()
        .find(|e| e.contract_def_id == Some(contract_def_id))?;

    let provided: std::collections::HashSet<&str> = impl_entry
        .methods
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();

    // Filter to only missing methods.
    let missing_sigs: Vec<_> = required_sigs
        .iter()
        .filter(|sig| !provided.contains(sig.name.as_str()))
        .collect();

    if missing_sigs.is_empty() {
        return None;
    }

    // Find insertion point: just before the closing `}` of the impl block.
    // The diagnostic range covers the impl block. We search backward from the
    // end of the range to find the `}`.
    let insert_offset = find_closing_brace_offset(source, &diag.range)?;

    // Detect indentation of the impl block body.
    let base_indent = detect_impl_indent(source, &diag.range);

    // Generate stub code for each missing method.
    let mut stubs = String::new();
    for sig in &missing_sigs {
        stubs.push('\n');
        stubs.push_str(&format_method_stub(sig, interner, def_map, &base_indent));
    }

    let insert_pos = crate::convert::offset_to_position(source, insert_offset);

    let edit = TextEdit {
        range: lsp_types::Range {
            start: insert_pos,
            end: insert_pos,
        },
        new_text: stubs,
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);

    let title = if missing_sigs.len() == 1 {
        format!("Implement `{}`", missing_sigs[0].name)
    } else {
        format!("Implement {} missing methods", missing_sigs.len())
    };

    Some(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        is_preferred: Some(true),
        ..Default::default()
    })
}

/// Format a single method stub from a contract FnSig.
fn format_method_stub(
    sig: &writ_compiler::check::env::FnSig,
    interner: &TyInterner,
    def_map: &DefMap,
    indent: &str,
) -> String {
    let mut s = String::new();
    s.push_str(indent);
    s.push_str("fn ");
    s.push_str(&sig.name);
    s.push('(');

    let mut params = Vec::new();

    // Self parameter
    if let Some(mutable) = sig.self_param {
        if mutable {
            params.push("mut self".to_string());
        } else {
            params.push("self".to_string());
        }
    }

    // Regular parameters
    for (name, ty) in &sig.params {
        let ty_str = interner.display_named(*ty, def_map);
        params.push(format!("{}: {}", name, ty_str));
    }

    s.push_str(&params.join(", "));
    s.push(')');

    // Return type
    let ret_str = interner.display_named(sig.ret, def_map);
    if ret_str != "void" {
        s.push_str(" -> ");
        s.push_str(&ret_str);
    }

    s.push_str(" {\n");
    s.push_str(indent);
    s.push_str("    // TODO: implement\n");
    s.push_str(indent);
    s.push_str("}\n");

    s
}

/// Extract a substring between `prefix` and `suffix` in `s`.
fn extract_between<'a>(s: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let start = s.find(prefix)? + prefix.len();
    let rest = &s[start..];
    let end = rest.find(suffix)?;
    Some(&rest[..end])
}

/// Find the byte offset of the closing `}` of an impl block.
///
/// Searches backward from the end of the diagnostic range in the source text.
fn find_closing_brace_offset(source: &str, range: &lsp_types::Range) -> Option<usize> {
    // Convert the end position of the diagnostic range to a byte offset.
    let end_offset = position_to_byte_offset_simple(source, range.end)?;

    // Search backward from end_offset for '}'.
    let search_start = end_offset.min(source.len());
    for i in (0..search_start).rev() {
        if source.as_bytes()[i] == b'}' {
            return Some(i);
        }
    }
    None
}

/// Detect the indentation used inside the impl block body.
///
/// Looks at the line where the impl block starts and adds one level of indentation.
fn detect_impl_indent(source: &str, range: &lsp_types::Range) -> String {
    let start_offset = position_to_byte_offset_simple(source, range.start).unwrap_or(0);

    // Find the start of the line containing the impl keyword.
    let line_start = source[..start_offset]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);

    // Extract leading whitespace of the impl line.
    let line = &source[line_start..];
    let outer_indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();

    // Impl body is indented one level deeper.
    format!("{}    ", outer_indent)
}

/// Simple position-to-byte-offset conversion for code action use.
fn position_to_byte_offset_simple(source: &str, pos: lsp_types::Position) -> Option<usize> {
    let mut line = 0u32;
    let mut col = 0u32;

    for (idx, ch) in source.char_indices() {
        if line == pos.line && col == pos.character {
            return Some(idx);
        }
        if ch == '\n' {
            // If we're on the target line and at end-of-line, the position
            // might be past the last character — return current index.
            if line == pos.line {
                return Some(idx);
            }
            line += 1;
            col = 0;
        } else {
            col += ch.len_utf16() as u32;
        }
    }

    // Position at end of source.
    if line == pos.line && col == pos.character {
        return Some(source.len());
    }

    // Past end of source: return end.
    if line <= pos.line {
        return Some(source.len());
    }

    None
}
