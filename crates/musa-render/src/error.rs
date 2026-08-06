//! Notation errors: explicit, never raw backend escapes (roadmap §7.2).

use musa_compiler::EventId;

/// A failure to plan notation for a score.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NotationError {
    /// A duration cannot be spelled with standard note values and ties
    /// inside one measure; tuplets arrive in prompt 22.
    #[error("event {event:?}: duration {duration} cannot be notated without tuplets")]
    UnspellableDuration {
        /// The score event carrying the duration.
        event: EventId,
        /// The duration's source spelling.
        duration: String,
    },
}

/// A failure of the `render_notation` facade: planning or backend writing.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    /// The score could not be planned.
    #[error(transparent)]
    Notation(#[from] NotationError),
    /// The backend writer failed.
    #[error("XML writer failed: {0}")]
    Xml(String),
    /// A construct the backend cannot express (§7.2: explicit, never a raw
    /// escape hatch).
    #[error("event {event:?}: {what}")]
    Unsupported {
        /// The score event involved.
        event: EventId,
        /// What cannot be expressed.
        what: String,
    },
}

impl RenderError {
    pub(crate) fn xml(error: &std::io::Error) -> Self {
        Self::Xml(error.to_string())
    }
}
