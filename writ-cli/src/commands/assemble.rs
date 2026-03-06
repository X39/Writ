//! `writ assemble` subcommand — convert .writil text to binary .writc.

use std::io::Read;

use crate::bom_utils::strip_bom_and_decode;

pub fn cmd_assemble(input: String, output: Option<String>) -> Result<(), String> {
    // Read source text
    let src = if input == "-" {
        // Read from stdin
        let mut bytes = Vec::new();
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        strip_bom_and_decode(&bytes)
            .map_err(|e| format!("failed to decode stdin: {e}"))?
    } else {
        let bytes = std::fs::read(&input)
            .map_err(|e| format!("failed to read '{}': {}", input, e))?;
        strip_bom_and_decode(&bytes)
            .map_err(|e| format!("failed to decode '{}': {}", input, e))?
    };

    // Assemble
    let module = writ_assembler::assemble(&src).map_err(|errs| {
        errs.into_iter()
            .map(|e| format!("{}:{}: {}", e.line, e.col, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    // Determine output path
    let out_path = output.unwrap_or_else(|| {
        if input.ends_with(".writil") {
            input[..input.len() - 7].to_string() + ".writc"
        } else if input == "-" {
            "output.writc".to_string()
        } else {
            input.clone() + ".writc"
        }
    });

    // Serialize
    let bytes = module.to_bytes().map_err(|e| format!("serialization error: {e:?}"))?;

    // Write output
    std::fs::write(&out_path, &bytes)
        .map_err(|e| format!("failed to write '{}': {}", out_path, e))?;

    eprintln!("Assembled: {out_path}");
    Ok(())
}
