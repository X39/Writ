//! tower-lsp backend for writ-lsp.
//!
//! Connects the AnalysisHost (compiler pipeline) and diagnostic conversion
//! layer to the LSP protocol. Handles document lifecycle notifications and
//! publishes diagnostics back to the editor.
//!
//! ## SPLIT-13 review (Phase 64)
//!
//! Reviewed for split opportunities at 888 lines. Conclusion: no split.
//! The `LanguageServer` trait impl (tower-lsp) requires all 12 async handlers
//! in a single `impl LanguageServer for Backend` block — Rust does not allow
//! splitting a trait impl across files. A delegation pattern (each handler
//! calls a helper in a separate module) would add boilerplate without reducing
//! cognitive load. The private `impl Backend` section (publish_diagnostics_for,
//! publish_grouped_diagnostics, identifier_completion) is 190 lines and already
//! at natural granularity.

use dashmap::DashMap;
use lsp_types::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tower_lsp::{jsonrpc, Client, LanguageServer};
use url::Url;
use writ_diagnostics::FileId;

/// The tower-lsp backend.
///
/// Holds the LSP client handle, an in-memory document store, the workspace root,
/// and a set of URIs that currently have published diagnostics (so stale
/// squiggles can be cleared when errors disappear).
pub struct Backend {
    pub(crate) client: Client,
    /// URI string -> current source text. Updated on did_open / did_change.
    pub(crate) document_map: DashMap<String, String>,
    /// Workspace root path, set during initialize.
    pub(crate) workspace_root: tokio::sync::RwLock<Option<PathBuf>>,
    /// URIs for which we have published at least one non-empty diagnostic set.
    /// Used to clear stale squiggles when errors are fixed.
    pub(crate) published_uris: DashMap<String, ()>,
    /// Per-URI cache of the most recent successful analysis result.
    /// Used by hover, goto-def, completions, and other LSP handlers.
    pub(crate) analysis_cache: DashMap<String, Arc<crate::analysis_host::AnalysisResult>>,
}

impl Backend {
    /// Create a new Backend with the given LSP client handle.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            document_map: DashMap::new(),
            workspace_root: tokio::sync::RwLock::new(None),
            published_uris: DashMap::new(),
            analysis_cache: DashMap::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> jsonrpc::Result<InitializeResult> {
        // Determine workspace root from workspace_folders (preferred) or root_uri (fallback).
        let root: Option<PathBuf> = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .and_then(|f| f.uri.to_file_path().ok())
            .or_else(|| {
                #[allow(deprecated)]
                params.root_uri.as_ref().and_then(|u| u.to_file_path().ok())
            });

        *self.workspace_root.write().await = root;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    ..Default::default()
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::TYPE,
                                    SemanticTokenType::new("entity"),
                                    SemanticTokenType::new("component"),
                                    SemanticTokenType::new("dialogueSpeaker"),
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::PARAMETER,
                                ],
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        self.document_map
            .insert(uri.to_string(), params.text_document.text);
        self.publish_diagnostics_for(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // FULL sync — the last content_change is the complete new document text.
        if let Some(change) = params.content_changes.into_iter().last() {
            self.document_map.insert(uri.to_string(), change.text);
        }
        // Publish diagnostics on every change for live feedback.
        // No debounce needed: spawn_blocking queues naturally.
        self.publish_diagnostics_for(uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        // If the save notification includes the text, update the document map.
        if let Some(text) = params.text {
            self.document_map.insert(uri.to_string(), text);
        }
        self.publish_diagnostics_for(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        let uri_str = uri.to_string();
        self.document_map.remove(&uri_str);
        // Clear squiggles for the closed file.
        self.published_uris.remove(&uri_str);
        // Remove cached analysis so stale data is not returned.
        self.analysis_cache.remove(&uri_str);
        self.client
            .publish_diagnostics(uri, vec![], None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let uri_str = params.text_document_position_params.text_document.uri.to_string();
        let pos = params.text_document_position_params.position;

        let source = match self.document_map.get(&uri_str) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        let cache_entry = match self.analysis_cache.get(&uri_str) {
            Some(e) => e.clone(),
            None => return Ok(None),
        };
        let (typed_ast, interner, type_env) = match (
            &cache_entry.typed_ast,
            &cache_entry.ty_interner,
            &cache_entry.type_env,
        ) {
            (Some(t), Some(i), Some(e)) => (t, i, e),
            _ => return Ok(None),
        };

        let byte_offset = match crate::queries::position_to_byte_offset(&source, pos) {
            Some(o) => o,
            None => return Ok(None),
        };

        let trigger_uri = &params.text_document_position_params.text_document.uri;
        let trigger_file_id = resolve_trigger_file_id(
            &cache_entry.file_sources,
            &uri_str,
            trigger_uri,
        );

        // Priority 1: Binding name (let, for, fn param) — checked BEFORE expr
        // because expr_at_offset always returns the enclosing Block for any position
        // inside a function body, masking binding names.
        if let Some(binding) = crate::queries::binding_at_offset(typed_ast, byte_offset, type_env, trigger_file_id) {
            let ty_str = interner.display_named(binding.ty, &typed_ast.def_map);
            let hover_text = format!("```writ\n{}: {}\n```", binding.name, ty_str);
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_text,
                }),
                range: Some(crate::convert::span_to_range(&source, &binding.name_span)),
            }));
        }

        // Priority 2: Declaration name (fn/enum/struct/const declaration site)
        if let Some(def_id) = crate::queries::def_at_offset(&typed_ast.def_map, byte_offset, trigger_file_id) {
            let hover_text = crate::queries::hover_text_for_def(
                def_id,
                &typed_ast.def_map,
                interner,
                type_env,
                &source,
                typed_ast,
            );
            if !hover_text.is_empty() {
                let entry = typed_ast.def_map.get_entry(def_id);
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_text,
                    }),
                    range: Some(crate::convert::span_to_range(&source, &entry.name_span)),
                }));
            }
        }

        // Priority 3: Expression at offset (call, field access, variable use, etc.)
        let expr_hover = crate::queries::expr_at_offset(typed_ast, byte_offset, trigger_file_id)
            .map(|expr| {
                let hover_text = crate::queries::hover_text_for_expr(
                    expr, &typed_ast.def_map, interner, type_env, &source, typed_ast,
                );
                (expr, hover_text)
            })
            .filter(|(_, hover_text)| !hover_text.is_empty());

        if let Some((expr, hover_text)) = expr_hover {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_text,
                }),
                range: Some(crate::convert::span_to_range(&source, &expr.span())),
            }));
        }

        // Priority 4: Match arm pattern (e.g., QuestStatus::Completed in match arm)
        if let Some(pattern_info) = crate::queries::pattern_at_offset(typed_ast, byte_offset, trigger_file_id) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: pattern_info.text,
                }),
                range: Some(crate::convert::span_to_range(&source, &pattern_info.span)),
            }));
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let pos = params.text_document_position_params.position;

        let source = match self.document_map.get(&uri_str) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        let cache_entry = match self.analysis_cache.get(&uri_str) {
            Some(e) => e.clone(),
            None => return Ok(None),
        };
        let typed_ast = match &cache_entry.typed_ast {
            Some(t) => t,
            None => return Ok(None),
        };

        let byte_offset = match crate::queries::position_to_byte_offset(&source, pos) {
            Some(o) => o,
            None => return Ok(None),
        };

        let trigger_uri = &params.text_document_position_params.text_document.uri;
        let trigger_file_id = resolve_trigger_file_id(
            &cache_entry.file_sources,
            &uri_str,
            trigger_uri,
        );

        // 1. Try expr-based def lookup (existing path)
        let def_id = crate::queries::expr_at_offset(typed_ast, byte_offset, trigger_file_id)
            .and_then(|expr| crate::queries::find_def_id_at_offset(expr, &typed_ast.def_map));

        // 2. Fallback: type annotation def (cursor on type name in let binding)
        let def_id = def_id.or_else(|| {
            crate::queries::type_ann_def_id_at_offset(typed_ast, byte_offset, trigger_file_id)
        });

        // 3. Fallback: declaration name
        let def_id = def_id.or_else(|| {
            crate::queries::def_at_offset(&typed_ast.def_map, byte_offset, trigger_file_id)
        });

        let def_id = match def_id {
            Some(id) => id,
            None => return Ok(None),
        };

        let entry = typed_ast.def_map.get_entry(def_id);

        // Synthetic builtins (log::*, dialogue builtins) have FileId(u32::MAX) — no location.
        if entry.file_id == FileId(u32::MAX) {
            return Ok(None);
        }

        // Find the source text for the target file.
        let target_source = cache_entry
            .file_sources
            .iter()
            .find(|(fid, _, _)| *fid == entry.file_id)
            .map(|(_, _, src)| src.as_str())
            .unwrap_or("");

        // Build target URI from file_sources display_path.
        let trigger_uri = params.text_document_position_params.text_document.uri;
        let target_uri = cache_entry
            .file_sources
            .iter()
            .find(|(fid, _, _)| *fid == entry.file_id)
            .map(|(_, path, _)| display_path_to_url(path, &trigger_uri))
            .unwrap_or_else(|| trigger_uri.clone());

        let range = crate::convert::span_to_range(target_source, &entry.name_span);

        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: target_uri,
            range,
        })))
    }

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> jsonrpc::Result<Option<Vec<Location>>> {
        let uri_str = params
            .text_document_position
            .text_document
            .uri
            .to_string();
        let pos = params.text_document_position.position;

        let source = match self.document_map.get(&uri_str) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        let cache_entry = match self.analysis_cache.get(&uri_str) {
            Some(e) => e.clone(),
            None => return Ok(None),
        };
        let typed_ast = match &cache_entry.typed_ast {
            Some(t) => t,
            None => return Ok(None),
        };

        let byte_offset = match crate::queries::position_to_byte_offset(&source, pos) {
            Some(o) => o,
            None => return Ok(None),
        };

        let trigger_uri = &params.text_document_position.text_document.uri;
        let trigger_file_id = resolve_trigger_file_id(
            &cache_entry.file_sources,
            &uri_str,
            trigger_uri,
        );

        // 1. Try expr-based def lookup (existing path)
        let def_id = crate::queries::expr_at_offset(typed_ast, byte_offset, trigger_file_id)
            .and_then(|expr| crate::queries::find_def_id_at_offset(expr, &typed_ast.def_map));

        // 2. Fallback: declaration name (cursor on a fn/struct/entity declaration)
        let def_id = def_id.or_else(|| {
            crate::queries::def_at_offset(&typed_ast.def_map, byte_offset, trigger_file_id)
        });

        let def_id = match def_id {
            Some(id) => id,
            None => return Ok(None),
        };

        let ref_spans =
            crate::queries::collect_references(typed_ast, def_id, &typed_ast.def_map);

        let trigger_uri = params.text_document_position.text_document.uri;
        let mut locations = Vec::new();

        // Optionally include the definition site itself.
        if params.context.include_declaration {
            let entry = typed_ast.def_map.get_entry(def_id);
            if entry.file_id != FileId(u32::MAX)
                && let Some((_, path, src)) = cache_entry
                    .file_sources
                    .iter()
                    .find(|(fid, _, _)| *fid == entry.file_id)
                {
                    let def_uri = display_path_to_url(path, &trigger_uri);
                    let range = crate::convert::span_to_range(src, &entry.name_span);
                    locations.push(Location { uri: def_uri, range });
                }
        }

        // Add all reference spans. Try to match spans to file sources by containment.
        for span in &ref_spans {
            let mut matched = false;
            for (_, path, src) in &cache_entry.file_sources {
                if span.start < src.len() && span.end <= src.len() {
                    let ref_uri = display_path_to_url(path, &trigger_uri);
                    let range = crate::convert::span_to_range(src, span);
                    locations.push(Location { uri: ref_uri, range });
                    matched = true;
                    break;
                }
            }
            if !matched {
                // Fallback: use trigger URI with best-effort conversion.
                let range = crate::convert::span_to_range(&source, span);
                locations.push(Location { uri: trigger_uri.clone(), range });
            }
        }

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        let uri_str = params.text_document_position.text_document.uri.to_string();
        let pos = params.text_document_position.position;

        let source = match self.document_map.get(&uri_str) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };

        // Detect trigger character for dot-completion vs identifier-completion
        let trigger_char = params
            .context
            .as_ref()
            .and_then(|ctx| ctx.trigger_character.as_deref());

        if trigger_char == Some(".") {
            // DOT COMPLETION (LSP-03, DIFF-02)
            //
            // The source has a trailing '.' at the cursor which makes it syntactically
            // invalid. Strip the dot, re-analyze, then find the type of the expression
            // immediately before the dot.
            let byte_offset = match crate::queries::position_to_byte_offset(&source, pos) {
                Some(o) => o,
                None => return Ok(None),
            };

            // Strip the '.' character at (byte_offset - 1) if it exists
            let dot_offset =
                if byte_offset > 0 && source.as_bytes().get(byte_offset - 1) == Some(&b'.') {
                    byte_offset - 1
                } else {
                    // No dot found at expected position; fall back to identifier completions
                    return self.identifier_completion(&uri_str, &source, pos).await;
                };

            // Build modified source without the dot
            let modified_source = format!("{}{}", &source[..dot_offset], &source[byte_offset..]);
            let display_path = uri_str.clone();

            // Re-run standalone analysis on the modified source to get receiver type
            let result = tokio::task::spawn_blocking(move || {
                crate::analysis_host::AnalysisHost::analyze_standalone(
                    modified_source,
                    display_path,
                )
            })
            .await;

            let analysis = match result {
                Ok(r) => r,
                Err(_) => return Ok(None),
            };

            let (typed_ast, interner, type_env) =
                match (&analysis.typed_ast, &analysis.ty_interner, &analysis.type_env) {
                    (Some(t), Some(i), Some(e)) => (t, i, e),
                    _ => return Ok(None),
                };

            // Find the receiver expression at (dot_offset - 1) in the modified source.
            // For standalone dot-completion analysis, use FileId(0) since analyze_standalone
            // produces a single-file result with the first available FileId.
            if dot_offset == 0 {
                return Ok(None);
            }
            let receiver_expr =
                match crate::queries::expr_at_offset(typed_ast, dot_offset.saturating_sub(1), FileId(0)) {
                    Some(e) => e,
                    None => return Ok(None),
                };

            let receiver_ty = receiver_expr.ty();
            let items = crate::queries::build_dot_completions(
                receiver_ty,
                interner,
                &typed_ast.def_map,
                type_env,
            );

            if items.is_empty() {
                return Ok(None);
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }

        if trigger_char == Some(":") {
            // NAMESPACE COMPLETION (LSP-02)
            //
            // Trigger arrives once per ':' keypress. On the first ':', the source has
            // "foo:" — extract_namespace_prefix returns None (needs at least "::").
            // On the second ':', the source has "foo::" — extraction succeeds.
            let byte_offset = match crate::queries::position_to_byte_offset(&source, pos) {
                Some(o) => o,
                None => return Ok(None),
            };
            if let Some(namespace) =
                crate::queries::extract_namespace_prefix(&source, byte_offset)
            {
                // Use cached analysis — no re-analysis needed for :: completions
                let cache_entry = match self.analysis_cache.get(&uri_str) {
                    Some(e) => e.clone(),
                    None => return Ok(None),
                };
                let (typed_ast, type_env) =
                    match (&cache_entry.typed_ast, &cache_entry.type_env) {
                        (Some(t), Some(e)) => (t, e),
                        _ => return Ok(None),
                    };
                let items = crate::queries::build_namespace_completions(
                    &namespace,
                    &typed_ast.def_map,
                    type_env,
                );
                if items.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(CompletionResponse::Array(items)));
            }
            // First colon of "::" — no valid namespace yet, return nothing
            return Ok(None);
        }

        // IDENTIFIER COMPLETION (LSP-02)
        self.identifier_completion(&uri_str, &source, pos).await
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> jsonrpc::Result<Option<SignatureHelp>> {
        let uri_str = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let pos = params.text_document_position_params.position;

        let source = match self.document_map.get(&uri_str) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        let cache_entry = match self.analysis_cache.get(&uri_str) {
            Some(e) => e.clone(),
            None => return Ok(None),
        };
        let (typed_ast, interner, type_env) = match (
            &cache_entry.typed_ast,
            &cache_entry.ty_interner,
            &cache_entry.type_env,
        ) {
            (Some(t), Some(i), Some(e)) => (t, i, e),
            _ => return Ok(None),
        };

        let byte_offset = match crate::queries::position_to_byte_offset(&source, pos) {
            Some(o) => o,
            None => return Ok(None),
        };

        let sig_help = crate::queries::build_signature_help(
            &source,
            byte_offset,
            typed_ast,
            interner,
            type_env,
        );

        Ok(sig_help)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        let uri_str = params.text_document.uri.to_string();

        let source = match self.document_map.get(&uri_str) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        let cache_entry = match self.analysis_cache.get(&uri_str) {
            Some(e) => e.clone(),
            None => return Ok(None),
        };
        let (typed_ast, interner) = match (&cache_entry.typed_ast, &cache_entry.ty_interner) {
            (Some(t), Some(i)) => (t, i),
            _ => return Ok(None),
        };

        // Determine the FileId for this URI by matching against file_sources
        let file_id = cache_entry
            .file_sources
            .iter()
            .find(|(_, path, _)| {
                // Match by URI comparison
                let p = std::path::Path::new(path);
                if p.is_absolute()
                    && let Ok(u) = Url::from_file_path(p) {
                        return u.to_string() == uri_str;
                    }
                false
            })
            .map(|(fid, _, _)| *fid)
            .unwrap_or(FileId(0));

        let raw_tokens =
            crate::queries::collect_semantic_tokens(typed_ast, interner, &source, file_id);

        // Delta-encode the tokens
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;
        let mut data = Vec::new();
        for token in &raw_tokens {
            let delta_line = token.line - prev_line;
            let delta_start = if delta_line == 0 {
                token.start_char - prev_start
            } else {
                token.start_char
            };
            data.push(SemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: token.token_type,
                token_modifiers_bitset: 0,
            });
            prev_line = token.line;
            prev_start = token.start_char;
        }

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }
}

impl Backend {
    /// Run analysis for the given URI and push the resulting diagnostics to the client.
    ///
    /// Spawns the analysis on a blocking thread so the async executor is not stalled.
    pub(crate) async fn publish_diagnostics_for(&self, uri: Url) {
        let uri_str = uri.to_string();

        let source = match self.document_map.get(&uri_str) {
            Some(s) => s.clone(),
            None => return,
        };

        let workspace_root = self.workspace_root.read().await.clone();

        let file_path = uri.to_file_path().ok();
        let display_path = file_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| uri_str.clone());

        let result = tokio::task::spawn_blocking(move || {
            // Project mode if the workspace root contains a writ.toml.
            if let Some(ref root) = workspace_root
                && root.join("writ.toml").exists() {
                    return crate::analysis_host::AnalysisHost::analyze_project(
                        root,
                        Some(&display_path),
                        Some(source),
                    );
                }
            // Fallback: standalone analysis of the single file.
            crate::analysis_host::AnalysisHost::analyze_standalone(source, display_path)
        })
        .await;

        match result {
            Ok(analysis_result) => {
                let arc_result = Arc::new(analysis_result);
                self.analysis_cache.insert(uri_str.clone(), arc_result.clone());
                self.publish_grouped_diagnostics(&uri, &arc_result).await;
            }
            Err(e) => {
                // spawn_blocking panicked — log to stderr but do not crash the server.
                eprintln!("writ-lsp: analysis panicked: {e:?}");
            }
        }
    }

    /// Distribute diagnostics to the correct per-URI channels.
    ///
    /// For standalone analysis all diagnostics go to `trigger_uri`. For project
    /// analysis diagnostics are routed to the file they originated from. Any URI
    /// that previously had diagnostics but now has none receives an empty publish
    /// to clear stale squiggles.
    pub(crate) async fn publish_grouped_diagnostics(
        &self,
        trigger_uri: &Url,
        result: &Arc<crate::analysis_host::AnalysisResult>,
    ) {
        // Build FileId -> Url and FileId -> &'static str maps from file_sources.
        // Box::leak is the established pattern for turning owned Strings into 'static strs
        // (mirrors run_pipeline in writ-cli and AnalysisHost itself).
        let mut file_id_to_url: HashMap<FileId, Url> = HashMap::new();
        let mut file_id_to_source: HashMap<FileId, &'static str> = HashMap::new();

        for (file_id, display_path, source_text) in &result.file_sources {
            // Convert display_path to a file:// URL. Fall back to trigger_uri
            // for paths that cannot be converted (e.g., in-memory buffers).
            let url = display_path_to_url(display_path, trigger_uri);
            file_id_to_url.insert(*file_id, url);

            let leaked: &'static str = Box::leak(source_text.clone().into_boxed_str());
            file_id_to_source.insert(*file_id, leaked);
        }

        // If file_sources is empty (shouldn't happen in normal flow), treat all
        // diagnostics as belonging to the trigger URI with empty source.
        let uri_for_file = |fid: FileId| -> Url {
            file_id_to_url
                .get(&fid)
                .cloned()
                .unwrap_or_else(|| trigger_uri.clone())
        };
        let source_for_file = |fid: FileId| -> &'static str {
            file_id_to_source.get(&fid).copied().unwrap_or("")
        };

        // Group LSP diagnostics by their target URI string.
        let mut by_uri: HashMap<String, Vec<lsp_types::Diagnostic>> = HashMap::new();

        for diag in &result.diagnostics {
            let target_uri = uri_for_file(diag.primary_file);
            let lsp_diag = crate::convert::writ_diag_to_lsp(
                diag,
                &uri_for_file,
                &source_for_file,
            );
            by_uri
                .entry(target_uri.to_string())
                .or_default()
                .push(lsp_diag);
        }

        // Publish diagnostics for every URI that has new results.
        let mut current_uris: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for (uri_str, diags) in &by_uri {
            if let Ok(publish_uri) = Url::parse(uri_str) {
                self.client
                    .publish_diagnostics(publish_uri, diags.clone(), None)
                    .await;
                // Track that this URI now has diagnostics published.
                self.published_uris.insert(uri_str.clone(), ());
                current_uris.insert(uri_str.clone());
            }
        }

        // If no diagnostics were produced for a URI that previously had some,
        // publish an empty vec to clear the stale squiggles.
        let stale: Vec<String> = self
            .published_uris
            .iter()
            .map(|e| e.key().clone())
            .filter(|u| !current_uris.contains(u))
            .collect();

        for uri_str in stale {
            if let Ok(clear_uri) = Url::parse(&uri_str) {
                self.client
                    .publish_diagnostics(clear_uri, vec![], None)
                    .await;
            }
            self.published_uris.remove(&uri_str);
        }

        // If the trigger file itself had zero diagnostics and is not in by_uri,
        // ensure we clear any previously published diagnostics for it.
        let trigger_str = trigger_uri.to_string();
        if !current_uris.contains(&trigger_str)
            && self.published_uris.contains_key(&trigger_str) {
                self.client
                    .publish_diagnostics(trigger_uri.clone(), vec![], None)
                    .await;
                self.published_uris.remove(&trigger_str);
            }
    }

    /// Build identifier completions from the cached analysis for `uri_str`.
    ///
    /// When the cursor is positioned immediately after `new ` (with at least one
    /// space), dispatches to `build_new_keyword_completions` which returns only
    /// constructable types (Struct, Class, Entity). Otherwise falls through to the
    /// full keyword + identifier list.
    ///
    /// Falls back to keyword-only completions if no typed data is cached.
    async fn identifier_completion(
        &self,
        uri_str: &str,
        source: &str,
        pos: lsp_types::Position,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        // Check whether the cursor is right after `new ` and use a filtered
        // constructable-type list when it is.
        if let Some(byte_offset) = crate::queries::position_to_byte_offset(source, pos) {
            if crate::queries::is_after_new_keyword(source, byte_offset) {
                // Use cached analysis to get the DefMap; if cache is missing return empty.
                let cache_entry = match self.analysis_cache.get(uri_str) {
                    Some(e) => e.clone(),
                    None => return Ok(Some(CompletionResponse::Array(vec![]))),
                };
                let (typed_ast, interner, type_env) =
                    match (&cache_entry.typed_ast, &cache_entry.ty_interner, &cache_entry.type_env) {
                        (Some(t), Some(i), Some(e)) => (t, i, e),
                        _ => return Ok(Some(CompletionResponse::Array(vec![]))),
                    };
                let items =
                    crate::queries::build_new_keyword_completions(&typed_ast.def_map, interner, type_env);
                return Ok(Some(CompletionResponse::Array(items)));
            }
        }

        let cache_entry = match self.analysis_cache.get(uri_str) {
            Some(e) => e.clone(),
            None => {
                // No cache yet — return keyword completions at minimum
                let items = crate::queries::build_identifier_completions(
                    &writ_compiler::resolve::def_map::DefMap::new(),
                    &writ_compiler::check::ty::TyInterner::new(),
                );
                return Ok(Some(CompletionResponse::Array(items)));
            }
        };

        let (typed_ast, interner) =
            match (&cache_entry.typed_ast, &cache_entry.ty_interner) {
                (Some(t), Some(i)) => (t, i),
                _ => {
                    // Parse/resolve failed — still return keyword completions
                    let items = crate::queries::build_identifier_completions(
                        &writ_compiler::resolve::def_map::DefMap::new(),
                        &writ_compiler::check::ty::TyInterner::new(),
                    );
                    return Ok(Some(CompletionResponse::Array(items)));
                }
            };

        let items =
            crate::queries::build_identifier_completions(&typed_ast.def_map, interner);
        Ok(Some(CompletionResponse::Array(items)))
    }
}

/// Resolve the `FileId` for the file currently being edited.
///
/// Searches `file_sources` for an entry whose display path converts to a URL
/// matching `uri_str`. Falls back to `FileId(0)` if no match is found (e.g.,
/// standalone single-file analysis where there is only one file).
pub(crate) fn resolve_trigger_file_id(
    file_sources: &[(FileId, String, String)],
    _uri_str: &str,
    trigger_uri: &Url,
) -> FileId {
    // Compare by filesystem path rather than URL string to avoid
    // Windows drive letter case mismatches and percent-encoding differences.
    let trigger_path = trigger_uri.to_file_path().ok();
    file_sources
        .iter()
        .find(|(_, display_path, _)| {
            if let Some(ref tp) = trigger_path {
                let source_path = std::path::Path::new(display_path);
                // Try canonical comparison first (resolves symlinks, normalizes case on Windows)
                if let (Ok(tc), Ok(sc)) = (tp.canonicalize(), source_path.canonicalize()) {
                    return tc == sc;
                }
                // Fall back to case-insensitive string comparison for paths that
                // don't exist on disk (e.g., unsaved buffers)
                return tp.to_string_lossy().eq_ignore_ascii_case(&source_path.to_string_lossy());
            }
            false
        })
        .map(|(fid, _, _)| *fid)
        .unwrap_or(FileId(0))
}

/// Convert a display path string to a `file://` URL.
///
/// If the path looks like an absolute filesystem path, use `Url::from_file_path`.
/// Otherwise fall back to `trigger_uri` (handles in-memory / virtual documents).
fn display_path_to_url(display_path: &str, trigger_uri: &Url) -> Url {
    // Prefer converting via std::path if it looks like a real path.
    let p = std::path::Path::new(display_path);
    if p.is_absolute()
        && let Ok(u) = Url::from_file_path(p) {
            return u;
        }
    // Fall back: try parsing as a URL directly (for virtual URIs).
    if let Ok(u) = Url::parse(display_path) {
        return u;
    }
    trigger_uri.clone()
}
