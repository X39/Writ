//! Writ compiler: source-to-IL compilation pipeline.
//!
//! ## Module structure
//!
//! - `ast`     -- Simplified AST produced by lowering (CST -> AST)
//! - `lower`   -- CST-to-AST lowering: desugars and normalises syntax
//! - `resolve` -- Name resolution: builds DefMap, resolves names to DefIds
//! - `check`   -- Type checking: produces TypedAst from resolved AST
//! - `emit`    -- IL emission: TypedAst -> binary .writc module bytes
//! - `config`  -- writ.toml parsing and project configuration

pub mod ast;
pub mod check;
pub mod config;
pub mod emit;
pub mod lower;
pub mod resolve;

// Public API re-exports
pub use ast::Ast;
pub use lower::lower;
pub use lower::error::LoweringError;
pub use lower::context::LoweringContext;

// Emit API
pub use emit::emit_bodies;
