//! What a caller can ask the session to do, and what it learns in return.

use crate::diagnostic::Span;

/// A replacement of one byte range of the source with new text.
///
/// The session's own edit type, stated in the same [`Span`] its diagnostics
/// use, so a caller can act on a diagnostic without a second vocabulary — and
/// so `text-size` stops here rather than reaching the frontend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// The range to replace.
    pub span: Span,
    /// What to put in its place; empty for a deletion.
    pub replacement: String,
}

impl TextEdit {
    /// Replace `span` with `replacement`.
    pub fn new(span: Span, replacement: impl Into<String>) -> Self {
        Self {
            span,
            replacement: replacement.into(),
        }
    }

    pub(crate) fn to_language(&self) -> musa_language::TextEdit {
        musa_language::TextEdit::new(
            text_size::TextRange::new(self.span.start.into(), self.span.end.into()),
            self.replacement.clone(),
        )
    }
}

/// A monotonic revision number.
///
/// Every successful source-changing command produces a new one; undo and redo
/// move between existing revisions rather than minting new ones, so a
/// revision identifies a *state*, not an edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(pub u64);

impl std::fmt::Display for Revision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A command against the session.
///
/// Source-changing commands are the editor's primitives; structured score
/// edits are one further variant that resolves into them.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ProjectCommand {
    /// A structured score edit, resolved through provenance into text edits
    /// and applied transactionally: if the result does not compile, the
    /// session is unchanged (roadmap §14.6).
    EditScore(crate::edit::EditCommand),
    /// A structured studio edit — a knob turned in the Sound workspace, a
    /// fader moved in the Mix workspace, a part pointed at another patch.
    /// Resolved into text edits against the studio source and applied
    /// transactionally, exactly as a score edit is (roadmap §11, §14.4).
    EditStudio(crate::studio::StudioEdit),
    /// Replace the whole document. The GUI's debounced text editor uses this.
    SetSource(String),
    /// Apply text edits — the mechanism every structured edit resolves to.
    ApplyEdits(Vec<TextEdit>),
    /// Reformat the source canonically. A source-changing command like any
    /// other, so it lands in the history and can be undone.
    Format,
    /// Save the current source to the project path.
    Save,
    /// Take back the work a previous session left in its recovery copy,
    /// as an ordinary edit: it lands in the history and can be undone.
    RestoreRecovery,
    /// Keep what is on disk and delete the recovery copy.
    DiscardRecovery,
    /// Drive the transport. Playback is unaffected by whether the *current*
    /// source compiles: it runs the last valid plan (roadmap §14.7).
    Transport(TransportRequest),
}

/// Transport requests, in the session's own vocabulary. The engine's own
/// command type never leaves `musa-engine`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportRequest {
    /// Start or resume playback, opening the audio device on first use.
    Play,
    /// Stop playback, keeping the position.
    Stop,
    /// Move to an absolute frame.
    Seek {
        /// Target frame.
        frame: u64,
    },
    /// Loop `[start, end)`; ignored if `end` is not after `start`.
    SetLoop {
        /// Loop start frame.
        start: u64,
        /// Loop end frame, exclusive.
        end: u64,
    },
    /// Stop looping.
    ClearLoop,
}

/// What a command changed.
///
/// This exists so a frontend can decide what to redo without diffing
/// snapshots: re-engraving a score is expensive (`docs/interface/06-performance.md`
/// B2), and most commands do not change it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectUpdate {
    /// The revision after the command.
    pub revision: Revision,
    /// The source text changed.
    pub source_changed: bool,
    /// The engraved score changed, so the view must be re-rendered.
    pub score_changed: bool,
    /// The diagnostic set changed.
    pub diagnostics_changed: bool,
    /// Whether the score and playback describe this revision or an earlier one.
    pub validity: Validity,
}

/// Whether what the interface is showing is the revision it is editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Validity {
    /// The current source compiles; score and playback are current.
    Current,
    /// The current source does not compile; the score and playback come from
    /// the last revision that did, and the interface must say so
    /// (roadmap §14.7).
    Stale,
}

impl Validity {
    pub(crate) fn from_compiles(compiles: bool) -> Self {
        if compiles { Self::Current } else { Self::Stale }
    }
}

impl ProjectUpdate {
    /// A command that changed nothing observable.
    pub(crate) fn unchanged(revision: Revision, validity: Validity) -> Self {
        Self {
            revision,
            source_changed: false,
            score_changed: false,
            diagnostics_changed: false,
            validity,
        }
    }
}
