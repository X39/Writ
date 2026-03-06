//! `writ disasm` subcommand — convert binary .writc to .writil text.

use writ_module::Module;

use crate::bom_utils::add_utf8_bom;

pub fn cmd_disasm(input: String, verbose: bool) -> Result<(), String> {
    let bytes =
        std::fs::read(&input).map_err(|e| format!("failed to read '{}': {}", input, e))?;

    let module =
        Module::from_bytes(&bytes).map_err(|e| format!("failed to parse module: {e:?}"))?;

    let text = if verbose {
        writ_assembler::disassemble_verbose(&module)
    } else {
        writ_assembler::disassemble(&module)
    };

    // Add UTF-8 BOM to disasm output
    let text_with_bom = add_utf8_bom(&text);
    print!("{text_with_bom}");
    Ok(())
}
