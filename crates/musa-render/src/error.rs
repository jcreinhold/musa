//! Notation errors: explicit, never raw backend escapes (roadmap §7.2).

use musa_compiler::EventId;

/// A failure to plan notation for a score.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NotationError {
    /// A duration cannot be spelled with standard note values and ties
    /// inside one measure; tuplets arrive in prompt 17.
    #[error("event {event:?}: duration {duration} cannot be notated without tuplets")]
    UnspellableDuration {
        /// The score event carrying the duration.
        event: EventId,
        /// The duration's source spelling.
        duration: String,
    },
}
