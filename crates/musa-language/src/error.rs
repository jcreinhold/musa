//! Syntax-level errors: plain data with spans. Rich rendering (`miette`)
//! happens at the CLI boundary, not here.
//!
//! A syntax error carries the same four parts a semantic one does (prompt 56)
//! — the claim, what is wrong at the place, what to do, and the edit that does
//! it — because the reader cannot tell which pass produced their problem and
//! should not have to.

use text_size::TextRange;

/// A lexical or parse error with a source span.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct SyntaxError {
    range: TextRange,
    message: String,
    label: String,
    help: Option<String>,
    fix: Option<(String, String)>,
}

impl SyntaxError {
    pub(crate) fn new(range: TextRange, message: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            range,
            message: message.into(),
            label: label.into(),
            help: None,
            fix: None,
        }
    }

    #[must_use]
    pub(crate) fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Offer an insertion at this error's own span.
    ///
    /// Only ever a literal the grammar requires — a `;`, a `}`, an `=`. There
    /// is no guesswork in adding the character the language demands at the one
    /// place it can go, which is what makes this a fix and "did you mean" a
    /// help line.
    #[must_use]
    pub(crate) fn with_fix(mut self, title: impl Into<String>, replacement: impl Into<String>) -> Self {
        self.fix = Some((title.into(), replacement.into()));
        self
    }

    /// Byte range of the offending source.
    pub fn range(&self) -> TextRange {
        self.range
    }

    /// What is wrong, as one line.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// What is wrong *at the span*, in a few words.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// What to do about it, when saying so adds something.
    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    /// The edit that resolves it: a title and the text to insert at
    /// [`Self::range`].
    pub fn fix(&self) -> Option<(&str, &str)> {
        self.fix
            .as_ref()
            .map(|(title, replacement)| (title.as_str(), replacement.as_str()))
    }
}
