//! Diagnostic rendering via ariadne.
//!
//! Converts `Diagnostic` structs into human-readable output with colored spans,
//! error codes, and help notes using the ariadne crate.

use crate::diagnostic::{Diagnostic, FileId, Severity};

/// Render a list of diagnostics to a string using ariadne.
///
/// Each source is provided as `(FileId, filename, source_text)`.
/// The output includes colored spans, error codes, and help text.
pub fn render_diagnostics(diagnostics: &[Diagnostic], sources: &[(FileId, &str, &str)]) -> String {
    use ariadne::{Color, Label, Report, ReportKind};
    use std::fmt::Write as _;

    let mut output = String::new();

    for diag in diagnostics {
        let kind = match diag.severity {
            Severity::Error => ReportKind::Error,
            Severity::Warning => ReportKind::Warning,
            Severity::Note => ReportKind::Advice,
        };

        let primary_offset = diag.primary_span.start;
        let mut builder = Report::build(kind, (diag.primary_file, primary_offset..primary_offset))
            .with_code(&diag.code)
            .with_message(&diag.message);

        // Primary label
        let primary_range = diag.primary_span.start..diag.primary_span.end;
        builder = builder.with_label(
            Label::new((diag.primary_file, primary_range))
                .with_message(&diag.primary_label)
                .with_color(match diag.severity {
                    Severity::Error => Color::Red,
                    Severity::Warning => Color::Yellow,
                    Severity::Note => Color::Blue,
                }),
        );

        // Secondary labels
        for sec in &diag.secondary_labels {
            let sec_range = sec.span.start..sec.span.end;
            builder = builder.with_label(
                Label::new((sec.file_id, sec_range))
                    .with_message(&sec.message)
                    .with_color(Color::Blue),
            );
        }

        // Help text
        if !diag.help.is_empty() {
            builder = builder.with_help(&diag.help);
        }

        // Notes
        for note in &diag.notes {
            builder = builder.with_note(note);
        }

        let report = builder.finish();

        // Build ariadne cache from raw source strings
        let cache = ariadne::sources(
            sources.iter().map(|(id, _name, text)| (*id, *text)),
        );

        let mut buf = Vec::new();
        let _ = report.write_for_stdout(cache, &mut buf);
        let rendered = String::from_utf8_lossy(&buf);
        let _ = write!(output, "{rendered}");
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Diagnostic, FileId};
    use chumsky::span::SimpleSpan;
    use chumsky::span::Span as _;

    #[test]
    fn render_error_diagnostic() {
        let diag = Diagnostic::error("E0001", "duplicate definition of `Foo`")
            .with_primary(FileId(0), SimpleSpan::new((), 10..13), "redefined here")
            .with_secondary(FileId(0), SimpleSpan::new((), 0..3), "first defined here")
            .with_help("consider renaming one of the definitions")
            .build();

        let sources = vec![(FileId(0), "test.writ", "fn Foo() {}\nfn Foo() {}")];
        let output = render_diagnostics(&[diag], &sources);
        assert!(output.contains("E0001"), "output should contain error code");
        assert!(output.contains("duplicate definition"), "output should contain message");
    }

    #[test]
    fn render_warning_diagnostic() {
        let diag = Diagnostic::warning("W0004", "namespace does not match file path")
            .with_primary(FileId(0), SimpleSpan::new((), 0..18), "declared here")
            .build();

        let sources = vec![(FileId(0), "test.writ", "namespace survival;")];
        let output = render_diagnostics(&[diag], &sources);
        assert!(output.contains("W0004"), "output should contain warning code");
    }
}
