//! The frontend's entire view of the session.

use crate::command::Revision;
use crate::diagnostic::Diagnostic;
use crate::facts::ScoreFacts;

/// Everything a caller may observe, borrowed from the session.
///
/// Borrowed rather than owned because the engraved MEI of a large score is
/// substantial and a GUI takes a snapshot after every command; copying it
/// per observation would be waste with no purpose. Nothing here is a
/// compiler, renderer, or engine type — the frontend owns no semantics
/// (roadmap §14.2, §15.7).
#[derive(Clone, Copy, Debug)]
pub struct ProjectSnapshot<'session> {
    pub(crate) source: &'session str,
    pub(crate) name: &'session str,
    pub(crate) revision: Revision,
    pub(crate) diagnostics: &'session [Diagnostic],
    pub(crate) valid: &'session Option<ValidArtifacts>,
    pub(crate) compiles: bool,
    pub(crate) unsaved: bool,
    pub(crate) playback: PlaybackState,
}

/// The artifacts of the most recent *successful* compilation. They survive
/// an invalid edit untouched, which is what lets the score stay on screen
/// and playback keep running while the text is mid-thought (roadmap §14.7).
#[derive(Debug)]
pub(crate) struct ValidArtifacts {
    /// The engraved score, rendered once per successful compile rather than
    /// once per observation.
    pub(crate) mei: String,
    /// The compiled score, kept for exports and playback preparation.
    pub(crate) score: musa_compiler::ScoreSnapshot,
    /// The compiled studio from the same compilation. Kept beside the score
    /// rather than inside it: they are two documents, and pairing them here
    /// is what stops a render from using one piece's sound with another's
    /// notes (§6.5).
    pub(crate) studio: musa_compiler::StudioSpec,
    /// Everything the interface displays about that score.
    pub(crate) facts: ScoreFacts,
    pub(crate) revision: Revision,
}

impl ProjectSnapshot<'_> {
    /// The current source text — the canonical document (roadmap §11).
    pub fn source(&self) -> &str {
        self.source
    }

    /// The document's display name.
    pub fn name(&self) -> &str {
        self.name
    }

    /// The current revision.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Diagnostics for the *current* source, whether or not it compiles.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.diagnostics
    }

    /// Whether the current source compiles.
    ///
    /// When false, [`Self::mei`] and playback describe
    /// [`Self::score_revision`], not [`Self::revision`] — the interface must
    /// say so rather than blanking the score (roadmap §14.7,
    /// `docs/interface/05-states.md` §4).
    pub fn compiles(&self) -> bool {
        self.compiles
    }

    /// The engraved score as MEI, from the last revision that compiled.
    /// `None` only before the piece has ever compiled.
    pub fn mei(&self) -> Option<&str> {
        self.valid.as_ref().map(|valid| valid.mei.as_str())
    }

    /// The musical facts the interface displays: title, tempo, key, parts,
    /// and every event's pitch, position, and provenance. From the same
    /// revision as [`Self::mei`].
    pub fn score(&self) -> Option<&ScoreFacts> {
        self.valid.as_ref().map(|valid| &valid.facts)
    }

    /// The revision the engraved score and playback plan came from.
    pub fn score_revision(&self) -> Option<Revision> {
        self.valid.as_ref().map(|valid| valid.revision)
    }

    /// Whether the source differs from what is on disk.
    pub fn unsaved(&self) -> bool {
        self.unsaved
    }

    /// What the transport is doing.
    pub fn playback(&self) -> PlaybackState {
        self.playback
    }
}

/// The snapshot as the interface receives it — one flat object, with the
/// last-valid artifacts lifted out of their internal container.
///
/// Serialization lives here rather than in the frontend because the shape of
/// this object is part of the facade: a fixture generated from this type
/// cannot drift from it (prompt 20).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotWire<'a> {
    name: &'a str,
    source: &'a str,
    revision: u64,
    compiles: bool,
    unsaved: bool,
    diagnostics: &'a [Diagnostic],
    mei: Option<&'a str>,
    score: Option<&'a ScoreFacts>,
    score_revision: Option<u64>,
    playback: PlaybackState,
}

impl serde::Serialize for ProjectSnapshot<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SnapshotWire {
            name: self.name,
            source: self.source,
            revision: self.revision.0,
            compiles: self.compiles,
            unsaved: self.unsaved,
            diagnostics: self.diagnostics,
            mei: self.mei(),
            score: self.score(),
            score_revision: self.score_revision().map(|revision| revision.0),
            playback: self.playback,
        }
        .serialize(serializer)
    }
}

/// Transport state as the interface needs to display it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackState {
    /// Whether audio is running.
    pub playing: bool,
    /// The transport position in frames.
    pub position_frames: u64,
    /// The installed plan's total length in frames, including its tail.
    pub total_frames: u64,
    /// The sample rate the plan and stream run at.
    pub sample_rate: u32,
    /// The active loop region, if any.
    pub loop_region: Option<(u64, u64)>,
}
