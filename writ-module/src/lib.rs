pub mod builder;
pub mod error;
pub mod heap;
pub mod instruction;
pub mod module;
pub(crate) mod reader;
pub mod tables;
pub mod token;
pub(crate) mod writer;

pub use builder::ModuleBuilder;
pub use error::{DecodeError, EncodeError, ModuleError};
pub use instruction::Instruction;
pub use module::Module;
pub use token::MetadataToken;
