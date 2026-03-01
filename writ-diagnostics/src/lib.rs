//! Shared diagnostic types and rendering for the Writ compiler.
//!
//! This crate provides the `Diagnostic` type used throughout the compiler
//! pipeline, plus ariadne-based rendering for Rust-style error output.

pub mod code;
pub mod diagnostic;
pub mod render;

pub use diagnostic::{Diagnostic, DiagnosticBuilder, FileId, SecondaryLabel, Severity};
pub use render::render_diagnostics;
