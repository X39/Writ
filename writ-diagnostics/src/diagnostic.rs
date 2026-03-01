//! Core diagnostic types for the Writ compiler.

use chumsky::span::SimpleSpan;
use chumsky::span::Span as _;
use std::fmt;

/// Unique file identifier for multi-file diagnostic reporting.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Ord, PartialOrd)]
pub struct FileId(pub u32);

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Hard error that prevents compilation.
    Error,
    /// Warning about potential issues.
    Warning,
    /// Informational note.
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Note => write!(f, "note"),
        }
    }
}

/// A secondary label attached to a diagnostic, pointing to related source locations.
#[derive(Debug, Clone)]
pub struct SecondaryLabel {
    /// The file and span this label points to.
    pub file_id: FileId,
    /// The span within the file.
    pub span: SimpleSpan,
    /// A message describing what this location means in context.
    pub message: String,
}

/// A compiler diagnostic with severity, code, message, labels, and help text.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Error/warning severity level.
    pub severity: Severity,
    /// Diagnostic code (e.g., "E0001", "W0004").
    pub code: String,
    /// Primary diagnostic message.
    pub message: String,
    /// The file containing the primary span.
    pub primary_file: FileId,
    /// The primary source span.
    pub primary_span: SimpleSpan,
    /// Label text for the primary span.
    pub primary_label: String,
    /// Additional labeled spans in the same or other files.
    pub secondary_labels: Vec<SecondaryLabel>,
    /// Help text suggesting how to fix the issue.
    pub help: String,
    /// Additional notes providing context.
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Create an error diagnostic.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            primary_file: FileId(0),
            primary_span: SimpleSpan::new((), 0..0),
            primary_label: String::new(),
            secondary_labels: Vec::new(),
            help: String::new(),
            notes: Vec::new(),
        }
    }

    /// Create a warning diagnostic.
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder {
            severity: Severity::Warning,
            code: code.into(),
            message: message.into(),
            primary_file: FileId(0),
            primary_span: SimpleSpan::new((), 0..0),
            primary_label: String::new(),
            secondary_labels: Vec::new(),
            help: String::new(),
            notes: Vec::new(),
        }
    }
}

/// Builder for constructing diagnostics fluently.
pub struct DiagnosticBuilder {
    severity: Severity,
    code: String,
    message: String,
    primary_file: FileId,
    primary_span: SimpleSpan,
    primary_label: String,
    secondary_labels: Vec<SecondaryLabel>,
    help: String,
    notes: Vec<String>,
}

impl DiagnosticBuilder {
    /// Set the primary span location.
    pub fn with_primary(mut self, file_id: FileId, span: SimpleSpan, label: impl Into<String>) -> Self {
        self.primary_file = file_id;
        self.primary_span = span;
        self.primary_label = label.into();
        self
    }

    /// Add a secondary label.
    pub fn with_secondary(mut self, file_id: FileId, span: SimpleSpan, message: impl Into<String>) -> Self {
        self.secondary_labels.push(SecondaryLabel {
            file_id,
            span,
            message: message.into(),
        });
        self
    }

    /// Set the help text.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = help.into();
        self
    }

    /// Add a note.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Build the final diagnostic.
    pub fn build(self) -> Diagnostic {
        Diagnostic {
            severity: self.severity,
            code: self.code,
            message: self.message,
            primary_file: self.primary_file,
            primary_span: self.primary_span,
            primary_label: self.primary_label,
            secondary_labels: self.secondary_labels,
            help: self.help,
            notes: self.notes,
        }
    }
}
