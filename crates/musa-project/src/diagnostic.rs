//! Session-level diagnostics.
//!
//! A deliberate restatement of the compiler's diagnostic type rather than a
//! re-export: `musa-project` is the boundary at which compiler types stop
//! (roadmap §15.7), and a frontend that pattern-matched on
//! `musa_compiler::Severity` would be coupled to the compiler forever.

/// How serious a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// The source does not compile.
    Error,
    /// Something was skipped or is suspicious; compilation continued.
    Warning,
}

/// A byte range in the current source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// First byte.
    pub start: u32,
    /// One past the last byte.
    pub end: u32,
}

/// A problem with the source, positioned where the interface can show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// How serious it is.
    pub severity: Severity,
    /// What is wrong, in the interface's voice.
    pub message: String,
    /// Where, when the diagnostic is tied to a place in the source.
    pub span: Option<Span>,
}

impl Diagnostic {
    pub(crate) fn from_compiler(diagnostic: &musa_compiler::Diagnostic) -> Self {
        Self {
            severity: match diagnostic.severity {
                musa_compiler::Severity::Error => Severity::Error,
                musa_compiler::Severity::Warning => Severity::Warning,
            },
            message: diagnostic.message.clone(),
            span: diagnostic.span.map(|span| Span {
                start: span.start,
                end: span.end,
            }),
        }
    }
}
