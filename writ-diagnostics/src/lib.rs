//! Shared diagnostic types and rendering for the Writ compiler.
//!
//! This crate provides the `Diagnostic` type used throughout the compiler
//! pipeline, plus ariadne-based rendering for Rust-style error output.
//!
//! ## Module structure
//!
//! - `code`       -- Diagnostic error/warning code definitions
//! - `diagnostic` -- Diagnostic, DiagnosticBuilder, Severity types
//! - `render`     -- Ariadne-based terminal rendering of diagnostics

pub mod code;
pub mod diagnostic;
pub mod render;

pub use diagnostic::{Diagnostic, DiagnosticBuilder, FileId, SecondaryLabel, Severity};
pub use render::render_diagnostics;
