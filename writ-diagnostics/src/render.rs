//! Diagnostic rendering via ariadne.
//!
//! Converts `Diagnostic` structs into human-readable output with colored spans,
//! error codes, and help notes using the ariadne crate.

use crate::diagnostic::{Diagnostic, FileId, Severity};

/// Render a list of diagnostics to a string using ariadne.
///
/// Each source is provided as `(FileId, filename, source_text)`.
/// The output includes colored spans, error codes, and help text.
///
/// Secondary labels that reference a `FileId` not present in `sources` are silently
/// skipped. This prevents ariadne from panicking when the compiler attaches labels
/// pointing to synthetic built-in files (e.g., `FileId(u32::MAX)`) or to files that
/// were not included in the current render call (DIAG-01 guard).
pub fn render_diagnostics(diagnostics: &[Diagnostic], sources: &[(FileId, &str, &str)]) -> String {
    use ariadne::{Color, Label, Report, ReportKind};
    use std::fmt::Write as _;

    // Build a set of all known FileIds so we can guard secondary label lookups.
    let known_file_ids: std::collections::HashSet<FileId> =
        sources.iter().map(|(id, _, _)| *id).collect();

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

        // Secondary labels — skip any that reference files absent from sources (DIAG-01 guard).
        for sec in &diag.secondary_labels {
            if !known_file_ids.contains(&sec.file_id) {
                continue; // skip labels for files not in the sources slice
            }
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

    /// DIAG-01: render_diagnostics must not panic when a secondary label references a
    /// FileId that is not present in the sources slice.
    #[test]
    fn render_diagnostics_cross_file_guard() {
        let diag = Diagnostic::error("E0103", "unsatisfied bound")
            .with_primary(FileId(0), SimpleSpan::new((), 0..5), "type does not satisfy bound")
            // FileId(99) is intentionally absent from sources — this used to cause a panic.
            .with_secondary(FileId(99), SimpleSpan::new((), 0..4), "bound declared here")
            .build();

        let sources = vec![(FileId(0), "test.writ", "fn foo() {}")];
        // Must not panic; the secondary label for FileId(99) is silently skipped.
        let output = render_diagnostics(&[diag], &sources);
        assert!(output.contains("E0103"), "output should contain primary error code");
        assert!(!output.contains("bound declared here"), "output must not contain label for absent FileId");
    }

    /// DIAG-01: render_diagnostics must not panic when a secondary label references
    /// the synthetic sentinel FileId(u32::MAX) used for built-in types.
    #[test]
    fn render_diagnostics_sentinel_file_id_guard() {
        let diag = Diagnostic::error("E0103", "unsatisfied bound")
            .with_primary(FileId(0), SimpleSpan::new((), 0..5), "type here")
            // FileId(u32::MAX) is the sentinel used for built-in/synthetic spans.
            .with_secondary(FileId(u32::MAX), SimpleSpan::new((), 0..0), "built-in bound")
            .build();

        let sources = vec![(FileId(0), "test.writ", "fn foo() {}")];
        // Must not panic.
        let output = render_diagnostics(&[diag], &sources);
        assert!(output.contains("E0103"), "output should contain primary error code");
    }
}
