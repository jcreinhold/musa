//! Syntax-level errors: plain data with spans. Rich rendering (`miette`)
//! happens at the CLI boundary, not here.

use text_size::TextRange;

/// A lexical or parse error with a source span.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{message} at {}..{}", usize::from(.range.start()), usize::from(.range.end()))]
pub struct SyntaxError {
    range: TextRange,
    message: String,
}

impl SyntaxError {
    pub(crate) fn new(range: TextRange, message: impl Into<String>) -> Self {
        Self {
            range,
            message: message.into(),
        }
    }

    /// Byte range of the offending source.
    pub fn range(&self) -> TextRange {
        self.range
    }

    /// Human-readable description of what went wrong.
    pub fn message(&self) -> &str {
        &self.message
    }
}
