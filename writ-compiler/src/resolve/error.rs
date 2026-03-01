//! Resolution error types and conversion to diagnostics.

use chumsky::span::SimpleSpan;
use writ_diagnostics::{code, Diagnostic, FileId};

/// Errors produced during name resolution.
#[derive(Debug, Clone)]
pub enum ResolutionError {
    /// Two definitions with the same fully-qualified name.
    DuplicateDefinition {
        name: String,
        first_file: FileId,
        first_span: SimpleSpan,
        second_file: FileId,
        second_span: SimpleSpan,
    },
    /// User-defined type shadows a prelude name.
    PreludeShadow {
        name: String,
        file: FileId,
        span: SimpleSpan,
    },
    /// Name could not be resolved in scope.
    UnresolvedName {
        name: String,
        file: FileId,
        span: SimpleSpan,
        suggestion: Option<String>,
    },
    /// Name resolved to multiple candidates.
    AmbiguousName {
        name: String,
        file: FileId,
        span: SimpleSpan,
        candidates: Vec<(FileId, SimpleSpan, String)>,
    },
    /// Accessing a private definition from outside its file.
    VisibilityViolation {
        name: String,
        file: FileId,
        span: SimpleSpan,
        defined_in: FileId,
        defined_span: SimpleSpan,
    },
    /// File path does not match the declared namespace.
    NamespacePathMismatch {
        declared_ns: String,
        file_path: String,
        file: FileId,
        span: SimpleSpan,
    },
    /// Invalid attribute target.
    InvalidAttributeTarget {
        attr_name: String,
        target_kind: String,
        file: FileId,
        span: SimpleSpan,
    },
    /// Invalid speaker reference.
    InvalidSpeaker {
        name: String,
        file: FileId,
        span: SimpleSpan,
    },
    /// Unresolved namespace in using declaration.
    UnresolvedNamespace {
        name: String,
        file: FileId,
        span: SimpleSpan,
    },
    /// Unused import.
    UnusedImport {
        alias: String,
        file: FileId,
        span: SimpleSpan,
    },
    /// Generic parameter shadows outer name.
    GenericShadow {
        name: String,
        file: FileId,
        span: SimpleSpan,
    },
    /// Component slot target is not a component type.
    NotAComponent {
        name: String,
        file: FileId,
        span: SimpleSpan,
    },
}

impl From<ResolutionError> for Diagnostic {
    fn from(err: ResolutionError) -> Self {
        match err {
            ResolutionError::DuplicateDefinition {
                name,
                first_file,
                first_span,
                second_file,
                second_span,
            } => Diagnostic::error(code::E0001, format!("duplicate definition of `{name}`"))
                .with_primary(second_file, second_span, "redefined here")
                .with_secondary(first_file, first_span, "first defined here")
                .with_help(format!("consider renaming one of the `{name}` definitions"))
                .build(),

            ResolutionError::PreludeShadow { name, file, span } => {
                Diagnostic::error(code::E0002, format!("cannot shadow prelude name `{name}`"))
                    .with_primary(file, span, format!("`{name}` is a built-in name"))
                    .with_help(format!(
                        "`{name}` is part of the Writ prelude and cannot be redefined"
                    ))
                    .build()
            }

            ResolutionError::UnresolvedName {
                name,
                file,
                span,
                suggestion,
            } => {
                let mut builder =
                    Diagnostic::error(code::E0003, format!("cannot find name `{name}` in scope"))
                        .with_primary(file, span, "not found in this scope");
                if let Some(suggestion) = suggestion {
                    builder = builder.with_help(format!("did you mean `{suggestion}`?"));
                }
                builder.build()
            }

            ResolutionError::AmbiguousName {
                name,
                file,
                span,
                candidates,
            } => {
                let mut builder = Diagnostic::error(
                    code::E0004,
                    format!("`{name}` is ambiguous — multiple candidates found"),
                )
                .with_primary(file, span, "ambiguous name");
                for (cand_file, cand_span, cand_desc) in candidates {
                    builder = builder.with_secondary(cand_file, cand_span, cand_desc);
                }
                builder
                    .with_help("use a fully-qualified path to disambiguate")
                    .build()
            }

            ResolutionError::VisibilityViolation {
                name,
                file,
                span,
                defined_in,
                defined_span,
            } => Diagnostic::error(
                code::E0005,
                format!("`{name}` is private and cannot be accessed from this file"),
            )
            .with_primary(file, span, "private name used here")
            .with_secondary(defined_in, defined_span, "defined as private here")
            .with_help(format!("consider making `{name}` pub"))
            .build(),

            ResolutionError::NamespacePathMismatch {
                declared_ns,
                file_path,
                file,
                span,
            } => {
                let ns_as_path = declared_ns.replace("::", "/");
                Diagnostic::warning(
                    code::W0004,
                    format!(
                        "namespace `{declared_ns}` does not match file path `{file_path}`"
                    ),
                )
                .with_primary(file, span, "declared here")
                .with_help(format!(
                    "convention is for file path to mirror the namespace (e.g., `src/{ns_as_path}/`)"
                ))
                .build()
            }

            ResolutionError::InvalidAttributeTarget {
                attr_name,
                target_kind,
                file,
                span,
            } => Diagnostic::error(
                code::E0006,
                format!("attribute `{attr_name}` is not valid on {target_kind}"),
            )
            .with_primary(file, span, "invalid attribute target")
            .build(),

            ResolutionError::InvalidSpeaker { name, file, span } => {
                Diagnostic::error(code::E0007, format!("invalid speaker `{name}`"))
                    .with_primary(file, span, "speaker not found")
                    .with_help("speakers must be declared as entities or structs in scope")
                    .build()
            }

            ResolutionError::UnresolvedNamespace { name, file, span } => {
                Diagnostic::error(code::E0003, format!("cannot find namespace `{name}`"))
                    .with_primary(file, span, "namespace not found")
                    .with_help("check that the namespace is spelled correctly and the containing file is included in compilation")
                    .build()
            }

            ResolutionError::UnusedImport { alias, file, span } => {
                Diagnostic::warning(code::W0001, format!("unused import `{alias}`"))
                    .with_primary(file, span, "this import is never used")
                    .with_help("remove this unused import")
                    .build()
            }

            ResolutionError::GenericShadow { name, file, span } => {
                Diagnostic::warning(code::W0003, format!("generic parameter `{name}` shadows outer type"))
                    .with_primary(file, span, "shadows an outer name")
                    .with_help(format!("consider renaming the generic parameter to avoid shadowing `{name}`"))
                    .build()
            }

            ResolutionError::NotAComponent { name, file, span } => {
                Diagnostic::error(code::E0003, format!("`{name}` is not a component type"))
                    .with_primary(file, span, "expected a component type")
                    .with_help("component slots must reference types declared with `component` or `extern component`")
                    .build()
            }
        }
    }
}
