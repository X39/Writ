//! Writ parser: lexing and parsing of Writ source code into a Concrete Syntax Tree.
//!
//! ## Module structure
//!
//! - `cst`          -- Concrete Syntax Tree node types (40+ enums/structs)
//! - `lexer`        -- Logos-based lexer producing token streams
//! - `parser`       -- Chumsky parser combinators with Pratt precedence
//! - `string_utils` -- String escape processing and raw string dedenting

pub mod cst;
pub mod lexer;
pub mod parser;
pub mod string_utils;

// Intentional re-export: cst module is the public API surface of writ-parser —
// all CST types are re-exported for downstream consumers (compiler, LSP, DAP).
pub use cst::*;
pub use lexer::{lex, Token};
pub use parser::parse;
pub use string_utils::{dedent_raw_string, process_escapes, EscapeError};
