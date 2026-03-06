//! Conversion utilities: byte-offset spans to LSP Positions, severity mapping,
//! and writ_diagnostics::Diagnostic to lsp_types::Diagnostic.

use chumsky::span::SimpleSpan;
use lsp_types::{
    DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Position, Range,
};
use url::Url;
use writ_diagnostics::{Diagnostic, FileId, Severity};

/// Convert a byte offset into an LSP Position (line/character) within `source`.
///
/// LSP uses 0-based line numbers and UTF-16 code-unit counts for column.
/// Out-of-bounds offsets return `Position { line: 0, character: 0 }`.
pub fn offset_to_position(source: &str, byte_offset: usize) -> Position {
    if byte_offset > source.len() {
        return Position { line: 0, character: 0 };
    }

    let mut line: u32 = 0;
    let mut character: u32 = 0;

    for (idx, ch) in source.char_indices() {
        if idx == byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            // Sum UTF-16 code units for each character
            character += ch.len_utf16() as u32;
        }
    }

    Position { line, character }
}

/// Convert a `SimpleSpan` (byte-offset range) to an LSP `Range`.
pub fn span_to_range(source: &str, span: &SimpleSpan) -> Range {
    let start = offset_to_position(source, span.start);
    let end = offset_to_position(source, span.end);
    Range { start, end }
}

/// Map a `writ_diagnostics::Severity` to an LSP `DiagnosticSeverity`.
pub fn severity_to_lsp(s: Severity) -> DiagnosticSeverity {
    match s {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Note => DiagnosticSeverity::INFORMATION,
    }
}

/// Convert a `writ_diagnostics::Diagnostic` to an `lsp_types::Diagnostic`.
///
/// - `uri_for_file`: maps a `FileId` to its document URI (needed for related information)
/// - `source_for_file`: maps a `FileId` to the source text (needed for span conversion)
pub fn writ_diag_to_lsp(
    diag: &Diagnostic,
    uri_for_file: &dyn Fn(FileId) -> Url,
    source_for_file: &dyn Fn(FileId) -> &'static str,
) -> lsp_types::Diagnostic {
    let primary_source = source_for_file(diag.primary_file);
    let range = span_to_range(primary_source, &diag.primary_span);

    let code = if diag.code.is_empty() {
        None
    } else {
        Some(NumberOrString::String(diag.code.clone()))
    };

    let related_information = if diag.secondary_labels.is_empty() {
        None
    } else {
        let infos: Vec<DiagnosticRelatedInformation> = diag
            .secondary_labels
            .iter()
            .map(|label| {
                let sec_source = source_for_file(label.file_id);
                let sec_range = span_to_range(sec_source, &label.span);
                DiagnosticRelatedInformation {
                    location: Location {
                        uri: uri_for_file(label.file_id),
                        range: sec_range,
                    },
                    message: label.message.clone(),
                }
            })
            .collect();
        Some(infos)
    };

    lsp_types::Diagnostic {
        range,
        severity: Some(severity_to_lsp(diag.severity)),
        code,
        source: Some("writ".to_string()),
        message: diag.message.clone(),
        related_information,
        ..Default::default()
    }
}

/// Convert a chumsky `Rich` parse error to a `writ_diagnostics::Diagnostic`.
///
/// Since `writ_parser::Token` does not implement `Display`, we use the `Debug`
/// representation of the expected tokens to produce a human-readable message.
pub fn parse_error_to_diag(
    err: &chumsky::error::Rich<'_, writ_parser::Token<'_>, SimpleSpan>,
    file_id: FileId,
) -> Diagnostic {
    let raw_span = *err.span();

    // Expand zero-width spans so VS Code renders a visible squiggle.
    // Zero-width spans occur at EOF or during entity/struct recovery.
    let span = if raw_span.start == raw_span.end {
        if raw_span.start > 0 {
            SimpleSpan {
                start: raw_span.start.saturating_sub(1),
                end: raw_span.start,
                context: (),
            }
        } else {
            // At offset 0: expand to 0..1 (VS Code clamps if beyond source)
            SimpleSpan {
                start: 0,
                end: 1,
                context: (),
            }
        }
    } else {
        raw_span
    };

    // Collect expected tokens into a readable message
    let expected: Vec<String> = err
        .expected()
        .map(|e| format!("{:?}", e))
        .collect();
    let found = err.found().map(|t| format!("{:?}", t));

    let message = if expected.is_empty() {
        match found {
            Some(f) => format!("unexpected token: {}", f),
            None => "unexpected end of input".to_string(),
        }
    } else {
        let expected_str = expected.join(", ");
        match found {
            Some(f) => format!("expected {}, found {}", expected_str, f),
            None => format!("expected {}", expected_str),
        }
    };

    Diagnostic::error("E0000", message)
        .with_primary(file_id, span, "parse error here")
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::span::SimpleSpan;
    use writ_diagnostics::{FileId, Severity};

    fn make_file_id() -> FileId {
        FileId(0)
    }

    fn dummy_uri(file_id: FileId) -> Url {
        Url::parse(&format!("file:///test/{}.writ", file_id.0)).unwrap()
    }

    #[test]
    fn test_offset_start_of_string() {
        let pos = offset_to_position("hello\nworld", 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_offset_start_of_second_line() {
        let pos = offset_to_position("hello\nworld", 6);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_offset_end_of_ascii_string() {
        let pos = offset_to_position("abc", 3);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3);
    }

    #[test]
    fn test_offset_multibyte_utf8_two_byte() {
        // "é" is 2 bytes in UTF-8 but 1 UTF-16 code unit (U+00E9)
        // source: "aé b"  bytes: a(0), é(1-2), ' '(3), b(4)
        let source = "a\u{00E9} b";
        let pos = offset_to_position(source, 3); // at the space
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 2); // 'a'=1 + 'é'=1 (UTF-16 unit) = 2
    }

    #[test]
    fn test_offset_multibyte_utf8_four_byte() {
        // "😀" is 4 bytes in UTF-8 but 2 UTF-16 code units (surrogate pair)
        // source: "a😀b"  bytes: a(0), 😀(1-4), b(5)
        let source = "a\u{1F600}b";
        let pos = offset_to_position(source, 5); // at 'b'
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3); // 'a'=1 + '😀'=2 (UTF-16 surrogates) = 3
    }

    #[test]
    fn test_offset_out_of_bounds() {
        let pos = offset_to_position("abc", 100);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_span_to_range() {
        let source = "hello\nworld";
        // span from byte 0 to byte 5 (the '\n')
        let span = SimpleSpan { start: 0, end: 5, context: () };
        let range = span_to_range(source, &span);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 5);
    }

    #[test]
    fn test_span_to_range_cross_line() {
        let source = "hello\nworld";
        // span from byte 0 to byte 6 (start of "world")
        let span = SimpleSpan { start: 0, end: 6, context: () };
        let range = span_to_range(source, &span);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 0);
    }

    #[test]
    fn test_severity_error() {
        assert_eq!(severity_to_lsp(Severity::Error), DiagnosticSeverity::ERROR);
    }

    #[test]
    fn test_severity_warning() {
        assert_eq!(severity_to_lsp(Severity::Warning), DiagnosticSeverity::WARNING);
    }

    #[test]
    fn test_severity_note() {
        assert_eq!(severity_to_lsp(Severity::Note), DiagnosticSeverity::INFORMATION);
    }

    // Leak a string to get a &'static str for tests
    fn leak(s: &str) -> &'static str {
        Box::leak(s.to_string().into_boxed_str())
    }

    #[test]
    fn test_writ_diag_to_lsp_basic() {
        let file_id = make_file_id();
        let span = SimpleSpan { start: 0, end: 3, context: () };
        let diag = Diagnostic::error("E0001", "test error")
            .with_primary(file_id, span, "here")
            .build();

        let source_text = leak("abc def");
        let lsp_diag = writ_diag_to_lsp(
            &diag,
            &|fid| dummy_uri(fid),
            &|_| source_text,
        );

        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(lsp_diag.code, Some(NumberOrString::String("E0001".to_string())));
        assert_eq!(lsp_diag.source, Some("writ".to_string()));
        assert_eq!(lsp_diag.message, "test error");
        assert!(lsp_diag.related_information.is_none());
    }

    #[test]
    fn test_writ_diag_to_lsp_with_secondary_labels() {
        let file_id = make_file_id();
        let span = SimpleSpan { start: 0, end: 3, context: () };
        let sec_span = SimpleSpan { start: 4, end: 7, context: () };
        let diag = Diagnostic::error("E0002", "dual error")
            .with_primary(file_id, span, "primary")
            .with_secondary(file_id, sec_span, "secondary label")
            .build();

        let source_text = leak("abc def");
        let lsp_diag = writ_diag_to_lsp(
            &diag,
            &|fid| dummy_uri(fid),
            &|_| source_text,
        );

        let related = lsp_diag.related_information.expect("should have related_information");
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].message, "secondary label");
    }

    #[test]
    fn test_writ_diag_to_lsp_empty_secondary_is_none() {
        let file_id = make_file_id();
        let span = SimpleSpan { start: 0, end: 3, context: () };
        let diag = Diagnostic::error("E0001", "no secondary")
            .with_primary(file_id, span, "here")
            .build();
        let lsp_diag = writ_diag_to_lsp(&diag, &|fid| dummy_uri(fid), &|_| leak("abc"));
        assert!(lsp_diag.related_information.is_none());
    }

    #[test]
    fn test_writ_diag_to_lsp_empty_code() {
        let file_id = make_file_id();
        let span = SimpleSpan { start: 0, end: 0, context: () };
        let diag = Diagnostic::warning("", "no code")
            .with_primary(file_id, span, "here")
            .build();

        let lsp_diag = writ_diag_to_lsp(&diag, &|fid| dummy_uri(fid), &|_| leak(""));
        assert!(lsp_diag.code.is_none());
    }

    #[test]
    fn test_parse_error_to_diag() {
        // Parse source with a syntax error to get a real chumsky Rich error
        let src = "fn main( {}";
        let (_cst_opt, parse_errs) = writ_parser::parse(src);
        // We should have at least one error
        assert!(!parse_errs.is_empty(), "Expected parse errors for broken syntax");

        let file_id = make_file_id();
        let diag = parse_error_to_diag(&parse_errs[0], file_id);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.code, "E0000");
        assert_eq!(diag.primary_file, file_id);
        assert!(!diag.message.is_empty());
    }

    #[test]
    fn test_zero_width_span_expansion() {
        // Entity with missing closing brace produces EOF error with zero-width span
        let src = "entity Foo {";
        let (_cst_opt, parse_errs) = writ_parser::parse(src);
        assert!(!parse_errs.is_empty(), "Expected parse errors for incomplete entity");

        let file_id = make_file_id();
        for err in &parse_errs {
            let diag = parse_error_to_diag(err, file_id);
            assert!(
                diag.primary_span.start != diag.primary_span.end || src.is_empty(),
                "Expected non-zero-width span for error: {}, got {}..{}",
                diag.message,
                diag.primary_span.start,
                diag.primary_span.end,
            );
        }
    }

    #[test]
    fn test_zero_width_span_at_offset_zero() {
        // Span at offset 0 should expand to 0..1 for non-empty source
        // Test with a source that triggers an error at or near the beginning
        let src = "!"; // invalid token at start
        let (_cst_opt, parse_errs) = writ_parser::parse(src);
        if !parse_errs.is_empty() {
            let file_id = make_file_id();
            let diag = parse_error_to_diag(&parse_errs[0], file_id);
            // Either the span is non-zero-width already (if parser gives a real span),
            // or it was expanded. Either way, the result should have a non-empty message.
            assert!(!diag.message.is_empty());
            // The span must always be non-zero-width after expansion (for non-empty source)
            assert!(
                diag.primary_span.start != diag.primary_span.end,
                "Expected non-zero-width span, got {}..{}",
                diag.primary_span.start,
                diag.primary_span.end,
            );
        }
    }
}
