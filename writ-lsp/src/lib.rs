//! Writ LSP server: Language Server Protocol implementation for Writ.
//!
//! ## Module structure
//!
//! - `analysis_host` -- Incremental analysis pipeline (parse -> lower -> resolve -> check)
//! - `backend`       -- tower-lsp Backend trait implementation (12 LSP request handlers)
//! - `convert`       -- Span/position conversion between compiler and LSP types
//! - `queries`       -- LSP feature queries (hover, completion, references, definition, etc.)

pub mod analysis_host;
pub mod backend;
pub mod convert;
pub mod queries;
