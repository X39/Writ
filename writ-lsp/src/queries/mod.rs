//! Query functions for LSP handlers.
//!
//! Provides position-to-node walking over the TypedAst and helper functions
//! used by hover, goto-def, find-refs, completions, and signature help.

pub mod code_actions;
pub mod completion;
pub mod hover;
pub mod references;
pub mod semantic;
pub mod walk;

// Re-exports — preserve the full public surface so all crate::queries::* call
// sites in backend.rs remain unchanged.

pub use walk::position_to_byte_offset;
pub use walk::expr_at_offset;
pub use walk::find_def_id_at_offset;

pub use hover::hover_text_for_expr;
pub use hover::hover_text_for_def;
pub use hover::extract_doc_comment;
pub use hover::pattern_at_offset;
pub use hover::PatternHoverInfo;

pub use references::collect_references;
pub use references::binding_at_offset;
pub use references::def_at_offset;
pub use references::type_ann_def_id_at_offset;
pub use references::BindingInfo;

pub use completion::build_identifier_completions;
pub use completion::build_dot_completions;
pub use completion::build_signature_help;
pub use completion::build_namespace_completions;
pub use completion::extract_namespace_prefix;
pub use completion::build_new_keyword_completions;
pub use completion::is_after_new_keyword;

pub use semantic::collect_semantic_tokens;
pub use semantic::RawSemanticToken;

pub use code_actions::build_code_actions;
