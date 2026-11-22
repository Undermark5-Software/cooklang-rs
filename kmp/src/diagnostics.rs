//! Errors and diagnostics, with source positions expressed in the units Kotlin
//! strings use (UTF-16 code units) rather than Rust's UTF-8 byte offsets.

use cooklang::error::{Severity as CoreSeverity, SourceDiag, SourceReport};

/// Error thrown by the cooklang bindings.
///
/// In Kotlin this is `CooklangException`.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CooklangError {
    /// The input could not be parsed.
    ///
    /// `diagnostics` holds every error and warning that was reported, with
    /// source positions, so they can be shown next to the offending text.
    #[error("{reason}")]
    Parse {
        reason: String,
        diagnostics: Vec<Diagnostic>,
    },
}

impl CooklangError {
    pub(crate) fn from_report(report: &SourceReport, source: &str) -> Self {
        let reason = report
            .errors()
            .next()
            .map(|diag| diag.message.to_string())
            .unwrap_or_else(|| "invalid input".to_string());
        CooklangError::Parse {
            reason,
            diagnostics: diagnostics(report, source),
        }
    }

    pub(crate) fn from_message(reason: impl Into<String>) -> Self {
        CooklangError::Parse {
            reason: reason.into(),
            diagnostics: Vec::new(),
        }
    }
}

/// How serious a [`Diagnostic`] is.
#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Parsing failed.
    Error,
    /// Parsing succeeded, but something looks wrong.
    Warning,
}

/// A problem found in the input.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// Locations in the input this diagnostic refers to, most important first.
    pub labels: Vec<DiagnosticLabel>,
    /// Suggestions for fixing the problem, most important first.
    pub hints: Vec<String>,
}

/// A location in the input, with an optional explanation.
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct DiagnosticLabel {
    pub span: SourceSpan,
    pub message: Option<String>,
}

/// A range of the input text.
///
/// `start` and `end` are UTF-16 offsets, so they can be used directly with
/// `String.substring` in Kotlin. `line` and `column` are 1-based and refer to
/// `start`; `column` is counted in UTF-16 code units.
#[derive(uniffi::Record, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: i32,
    pub end: i32,
    pub line: i32,
    pub column: i32,
}

pub(crate) fn diagnostics(report: &SourceReport, source: &str) -> Vec<Diagnostic> {
    report.iter().map(|diag| diagnostic(diag, source)).collect()
}

fn diagnostic(diag: &SourceDiag, source: &str) -> Diagnostic {
    Diagnostic {
        severity: match diag.severity {
            CoreSeverity::Error => Severity::Error,
            CoreSeverity::Warning => Severity::Warning,
        },
        message: diag.message.to_string(),
        labels: diag
            .labels
            .iter()
            .map(|(span, message)| DiagnosticLabel {
                span: source_span(source, span.start(), span.end()),
                message: message.as_ref().map(|m| m.to_string()),
            })
            .collect(),
        hints: diag.hints.iter().map(|h| h.to_string()).collect(),
    }
}

/// Converts a UTF-8 byte range of `source` into a [`SourceSpan`].
pub(crate) fn source_span(source: &str, start: usize, end: usize) -> SourceSpan {
    let (start16, line, column) = position(source, start);
    let (end16, _, _) = position(source, end.max(start));
    SourceSpan {
        start: start16,
        end: end16,
        line,
        column,
    }
}

/// UTF-16 offset, 1-based line and 1-based UTF-16 column of a byte offset.
fn position(source: &str, byte_offset: usize) -> (i32, i32, i32) {
    let mut offset = 0usize;
    let mut line = 1usize;
    let mut column = 1usize;
    for (index, ch) in source.char_indices() {
        if index >= byte_offset {
            break;
        }
        let width = ch.len_utf16();
        offset += width;
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += width;
        }
    }
    (crate::int(offset), crate::int(line), crate::int(column))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_are_utf16_and_one_based() {
        // "é" is 2 bytes in UTF-8 but 1 UTF-16 unit; "🍳" is 4 bytes / 2 units.
        let src = "é🍳\nab";
        let a = src.find('a').unwrap();
        assert_eq!(
            source_span(src, a, a + 1),
            SourceSpan {
                start: 4,
                end: 5,
                line: 2,
                column: 1
            }
        );
        let pan = src.find('🍳').unwrap();
        assert_eq!(
            source_span(src, pan, pan + 4),
            SourceSpan {
                start: 1,
                end: 3,
                line: 1,
                column: 2
            }
        );
    }
}
