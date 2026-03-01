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
