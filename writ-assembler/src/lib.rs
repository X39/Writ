pub mod assembler;
pub mod ast;
pub mod disassembler;
pub mod error;
pub mod lexer;
pub mod parser;

pub use disassembler::{disassemble, disassemble_verbose};
pub use error::AssembleError;

/// Assemble a `.writil` text source into a binary Module.
///
/// The pipeline is: tokenize -> parse -> assemble.
/// Returns a Module on success, or a list of errors with line:column diagnostics.
pub fn assemble(src: &str) -> Result<writ_module::Module, Vec<AssembleError>> {
    // Step 1: Tokenize
    let tokens = lexer::tokenize(src)?;

    // Step 2: Parse
    let ast = parser::parse(&tokens)?;

    // Step 3: Assemble
    assembler::assemble_module(ast)
}
