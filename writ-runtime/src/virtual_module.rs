//! Compatibility re-export for the canonical `writ-runtime` virtual module.
//!
//! The module definition lives in `writ-module` so the compiler and runtime
//! construct exactly the same core metadata without introducing a dependency
//! cycle between those crates.

pub use writ_module::virtual_module::build_writ_runtime_module;
