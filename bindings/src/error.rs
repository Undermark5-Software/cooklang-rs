use cooklang::error::{
    Severity as OriginalSeverity, SourceDiag as OriginalSourceDiag,
    SourceReport as OriginalSourceReport, Stage as OriginalStage,
};
use cooklang::Span as OriginalSpan;

/// Location in the source code (character offsets)
#[derive(uniffi::Record, Debug, Clone)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl From<OriginalSpan> for Span {
    fn from(span: OriginalSpan) -> Self {
        Span {
            start: span.start() as u32,
            end: span.end() as u32,
        }
    }
}

/// Severity of a diagnostic (error or warning)
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

impl From<OriginalSeverity> for Severity {
    fn from(severity: OriginalSeverity) -> Self {
        match severity {
            OriginalSeverity::Error => Severity::Error,
            OriginalSeverity::Warning => Severity::Warning,
        }
    }
}

/// Parsing stage where the diagnostic originated
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Stage {
    Parse,
    Analysis,
}

impl From<OriginalStage> for Stage {
    fn from(stage: OriginalStage) -> Self {
        match stage {
            OriginalStage::Parse => Stage::Parse,
            OriginalStage::Analysis => Stage::Analysis,
        }
    }
}

/// A label pointing to a location in the source code with an optional message
#[derive(uniffi::Record, Debug, Clone)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub message: Option<String>,
}

/// A diagnostic message (error or warning) with location information
#[derive(uniffi::Record, Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub stage: Stage,
    pub message: String,
    pub labels: Vec<DiagnosticLabel>,
    pub hints: Vec<String>,
}

impl From<&OriginalSourceDiag> for Diagnostic {
    fn from(diag: &OriginalSourceDiag) -> Self {
        Diagnostic {
            severity: diag.severity.into(),
            stage: diag.stage.into(),
            message: diag.message.to_string(),
            labels: diag
                .labels
                .iter()
                .map(|(span, msg)| DiagnosticLabel {
                    span: (*span).into(),
                    message: msg.as_ref().map(|m| m.to_string()),
                })
                .collect(),
            hints: diag.hints.iter().map(|h| h.to_string()).collect(),
        }
    }
}

/// A collection of diagnostics (errors and warnings)
#[derive(uniffi::Record, Debug, Clone)]
pub struct DiagnosticReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl From<&OriginalSourceReport> for DiagnosticReport {
    fn from(report: &OriginalSourceReport) -> Self {
        DiagnosticReport {
            diagnostics: report.iter().map(|d| d.into()).collect(),
        }
    }
}

/// Check if the report contains any errors
#[uniffi::export]
pub fn diagnostic_report_has_errors(report: &DiagnosticReport) -> bool {
    report
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
}

/// Check if the report contains any warnings
#[uniffi::export]
pub fn diagnostic_report_has_warnings(report: &DiagnosticReport) -> bool {
    report
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Warning)
}

/// Get only the errors from this report
#[uniffi::export]
pub fn diagnostic_report_errors(report: &DiagnosticReport) -> Vec<Diagnostic> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .cloned()
        .collect()
}

/// Get only the warnings from this report
#[uniffi::export]
pub fn diagnostic_report_warnings(report: &DiagnosticReport) -> Vec<Diagnostic> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .cloned()
        .collect()
}

/// Error type for recipe parsing failures
///
/// Avoid naming any field `message` — UniFFI maps Error enums onto
/// kotlin.Exception which already declares `message: String?`.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CooklangError {
    #[error("Parse failed with {error_count} error(s)")]
    ParseError {
        errors: DiagnosticReport,
        error_count: u32,
    },
}
