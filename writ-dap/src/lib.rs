//! Writ DAP server: Debug Adapter Protocol implementation for Writ.
//!
//! ## Module structure
//!
//! - `debug_host`  -- RuntimeHost implementation for debug sessions
//! - `breakpoints` -- Breakpoint management and source mapping
//! - `launch`      -- Program launch and attach configuration
//! - `server`      -- DAP server with request handlers and variable inspection
//! - `variables`   -- Variable presentation for debug UI

pub mod debug_host;
pub mod breakpoints;
pub mod launch;
pub mod server;
pub mod variables;
