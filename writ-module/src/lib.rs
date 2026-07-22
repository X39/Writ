//! Writ module format: binary .writc file reading, writing, and construction.
//!
//! ## Module structure
//!
//! - `builder`     -- ModuleBuilder for constructing modules programmatically
//! - `error`       -- DecodeError, EncodeError, ModuleError types
//! - `heap`        -- Heap blob storage for string and byte data
//! - `instruction` -- IL instruction definitions and encoding
//! - `module`      -- Module struct: in-memory representation of a .writc file
//! - `reader`      -- Binary deserialization from bytes to Module
//! - `tables`      -- 23 metadata table row-struct types (TypeDef, MethodDef, etc.)
//! - `token`       -- MetadataToken type for table row references
//! - `virtual_module` -- Canonical `writ-runtime` core module construction
//! - `writer`      -- Binary serialization from Module to bytes

pub mod attr;
pub mod builder;
pub mod error;
pub mod heap;
pub mod instruction;
pub mod module;
pub(crate) mod reader;
pub mod signature;
pub mod tables;
pub mod token;
pub mod virtual_module;
pub(crate) mod writer;

pub use attr::AttrValue;
pub use builder::ModuleBuilder;
pub use error::{DecodeError, EncodeError, ModuleError};
pub use instruction::Instruction;
pub use module::{FORMAT_VERSION, Module};
pub use tables::TypeDefKind;
pub use token::MetadataToken;
pub use virtual_module::build_writ_runtime_module;
