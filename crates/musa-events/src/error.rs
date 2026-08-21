//! Static violations of the event track rules (docs/rules/events/02). The event track has no
//! warnings: a construct is well-formed or rejected.

/// An event track construction error, naming the violated rule.
///
/// The offending times are carried as their reduced rational text rather than
/// as `Position<C>`: an error is not in a coordinate, and threading `C` through
/// it would make every caller of a fallible constructor name one just to talk
/// about a failure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EventsError {
    /// A span's start exceeds its end, or either is negative (K1).
    #[error("invalid span [{start}, {end}): spans satisfy 0 <= start <= end")]
    InvalidSpan {
        /// The offending start.
        start: String,
        /// The offending end.
        end: String,
    },
    /// A negative amount of time offered where a duration is required.
    ///
    /// Durations are the ordered monoid `(ℚ≥0, +, 0)`; this is the one place
    /// the `≥ 0` is checked, so every `Duration<C>` that exists is nonnegative.
    #[error("a duration is never negative, got {amount}")]
    NegativeDuration {
        /// The offending amount, in reduced form.
        amount: String,
    },
    /// An occurrence lies outside its track's duration (K1).
    #[error("occurrence span {span} outside the track's duration [0, {duration})")]
    OccurrenceOutOfBounds {
        /// The offending span.
        span: String,
        /// The track's duration.
        duration: String,
    },
    /// A `follow` or `together` term with no arguments (K7).
    #[error("`{form}` needs at least one argument; the empty case is written as a literal track")]
    EmptyComposition {
        /// The offending form, `"follow"` or `"together"`.
        form: &'static str,
    },
    /// A term names something no enclosing `let` binds (K7).
    #[error("`{name}` is not bound by any enclosing `let`")]
    FreeName {
        /// The offending name.
        name: String,
    },
    /// A `let` rebinds a name already in scope (K7).
    #[error("`{name}` is already bound; shadowing is rejected so substitution stays textual")]
    ShadowedName {
        /// The offending name.
        name: String,
    },
    /// Events text that is not a term (docs/rules/events/01).
    #[error("events text at byte {offset}: {message}")]
    Parse {
        /// The byte offset the reader stopped at.
        offset: usize,
        /// What was expected there.
        message: String,
    },
    /// Time scaling by a non-positive factor (K2/D5).
    #[error("scale factor must be a positive rational, got {factor}")]
    NonPositiveScale {
        /// The offending factor, in reduced form.
        factor: String,
    },
}
