//! One error type for everything the session can refuse to do.

/// What a [`ProjectSession`](crate::ProjectSession) operation can fail with.
///
/// Compilation *diagnostics* are not errors: a piece that does not compile
/// leaves the session in a valid state with the last good artifacts intact
/// (roadmap §14.7), reported through
/// [`ProjectSnapshot`](crate::ProjectSnapshot). This type covers the cases
/// where the session could not carry out the operation at all.
///
/// The enum is deliberately exhaustive. The workspace forbids wildcard match
/// arms, so consumers — the CLI, the desktop shell — match every variant, and
/// adding one here is a compile error until each of them decides what the new
/// failure means to a user. That is the check we want; `#[non_exhaustive]`
/// would trade it for a source compatibility this workspace has no use for.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    /// The project file could not be read or written.
    #[error("cannot access {path}: {source}")]
    Io {
        /// The path involved.
        path: String,
        /// The underlying I/O failure.
        source: std::io::Error,
    },

    /// A command produced source text that does not parse or compile, and the
    /// command was specified to be transactional (score edits, prompt 25).
    /// The session is unchanged.
    #[error("{intent} would break the source: {reason}")]
    RejectedEdit {
        /// What the caller was trying to do.
        intent: String,
        /// The first diagnostic that made the result unusable.
        reason: String,
    },

    /// There is nothing to undo or redo.
    #[error("nothing to {0}")]
    NothingTo(&'static str),

    /// An export was requested but the piece has never compiled, so there is
    /// no score to export.
    #[error("cannot export: the piece has never compiled successfully")]
    NoValidScore,

    /// Lowering the score for playback or audio export failed.
    #[error("cannot prepare audio: {0}")]
    Performance(String),

    /// A notation backend failed.
    #[error("cannot render notation: {0}")]
    Notation(String),

    /// The audio engine could not be opened or commanded.
    #[error("audio engine: {0}")]
    Engine(String),
}

impl ProjectError {
    /// Attach a path to an I/O failure.
    pub(crate) fn io(path: impl std::fmt::Display, source: std::io::Error) -> Self {
        Self::Io {
            path: path.to_string(),
            source,
        }
    }
}
