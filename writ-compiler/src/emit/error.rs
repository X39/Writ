//! Emit-phase error types.

use writ_diagnostics::Diagnostic;

/// Errors that can occur during IL metadata emission.
#[derive(Debug, Clone)]
pub enum EmitError {
    /// A DefId has no corresponding AST declaration.
    MissingAstDecl {
        name: String,
    },
    /// A DefId could not be resolved to a MetadataToken.
    UnresolvedDef {
        name: String,
    },
}

impl From<EmitError> for Diagnostic {
    fn from(err: EmitError) -> Diagnostic {
        match err {
            EmitError::MissingAstDecl { name } => {
                Diagnostic::error(
                    "E2001",
                    format!("cannot find AST declaration for '{}'", name),
                )
                .build()
            }
            EmitError::UnresolvedDef { name } => {
                Diagnostic::error(
                    "E2002",
                    format!("cannot resolve definition '{}' to metadata token", name),
                )
                .build()
            }
        }
    }
}
