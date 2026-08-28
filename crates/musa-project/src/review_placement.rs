//! Keeping a reviewed phrase: the one transaction that turns a take into
//! source (prompt 208).
//!
//! Everything before this point is derived temporary evidence. A take is
//! memory-only MIDI, a proposal is a reading of it, and a review is a set of
//! decisions about that reading — none of it is canonical, and none of it has
//! touched the document. This module owns the single step where that changes,
//! and it owns it in two halves so the step can be looked at before it is
//! taken: a [`PlacementPlan`] states exactly what would be written, and
//! [`crate::ProjectSession::place_review`] writes it as one revision or
//! writes nothing at all.
//!
//! Two rules shape the whole module.
//!
//! **The anchor is a pair of names.** A take is played at one revision and
//! kept at another, and the edits in between move every byte offset the
//! capture could have recorded. A part and a voice are identities that
//! survive them, so those are what is recorded and what the placement is
//! rebased through; a byte range from the capture revision is never applied
//! to a later document.
//!
//! **Acceptance ends the special provenance.** The plan carries the take,
//! proposal, and policy identities so the pending edit can be explained and
//! validated. What it writes is ordinary source: the notes it commits
//! originate at their new spans exactly as typed notes do, and the
//! timestamps, velocities, calibration, and review decisions behind them are
//! released rather than filed away. Project undo stores source revisions, not
//! performances.

use crate::transcription_proposal::ProposalVoice;

/// One line of a placement: which proposal voice it writes, where, and
/// whether the score is gaining a line by taking it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedVoice {
    /// The proposal voice this line comes from, 0..4.
    pub proposal_voice: u8,
    /// The destination voice's name.
    pub name: String,
    /// The part does not have this voice yet, and placing would add it.
    pub added: bool,
    /// The bars it would write, exactly as the accepted preview spells them.
    pub bars: Vec<String>,
}

/// What keeping this phrase would do, before it is done.
///
/// A query: asking for it changes nothing, and asking twice is free. It is
/// computed against the *current* source rather than the capture revision, so
/// a plan that comes back at all is a plan that still fits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacementPlan {
    /// The take being kept.
    pub take_name: String,
    /// The revision the take was played at.
    pub captured_at: crate::Revision,
    /// The revision this plan was computed against, and the only one it may
    /// be applied to.
    pub revision: crate::Revision,
    /// The checked policy the reading was searched under.
    pub policy: String,
    /// The part the phrase joins.
    pub part: String,
    /// The lines it writes, in proposal-voice order.
    pub voices: Vec<PlacedVoice>,
    /// The document as it would read afterwards.
    pub source: String,
    /// One musical sentence: what a composer would say they had just done.
    pub summary: String,
}

impl PlacementPlan {
    /// The voices this would add to the part, in the order it would add them.
    #[must_use]
    pub fn added_voices(&self) -> Vec<&str> {
        self.voices
            .iter()
            .filter(|line| line.added)
            .map(|line| line.name.as_str())
            .collect()
    }
}

/// What keeping a phrase reports once it is source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacementReport {
    /// The revision the phrase was written into.
    pub revision: crate::Revision,
    /// The sentence the plan promised, now in the past tense of having
    /// happened.
    pub summary: String,
    /// The part it joined.
    pub part: String,
    /// The voices it wrote into, in the order it wrote them.
    pub voices: Vec<String>,
    /// The engraved ids of the events it added, in score order — what the
    /// interface selects so the composer is looking at what they just kept.
    pub events: Vec<String>,
}

/// Why a phrase could not be kept.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlacementError {
    /// Nothing is under review.
    NotReviewing,
    /// The reading has not been accepted yet: keeping is the second half of
    /// accepting, and a phrase still taking decisions is not a phrase.
    NotAccepted,
    /// The reading has a written length it cannot spell exactly, so there is
    /// no phrase to write.
    NoPhrase,
    /// The piece does not currently compile, so there is no part to write
    /// into.
    NoValidScore,
    /// The score no longer has the part the take was played into.
    PieceChanged(String),
    /// Extra lines need names, and naming them is the reviewer's decision
    /// rather than a default.
    NeedsVoiceNames {
        /// How many the proposal writes.
        needed: usize,
        /// How many were given.
        given: usize,
    },
    /// A destination name is not a name this language can write.
    NotAName(String),
    /// Two lines of one placement were pointed at the same voice.
    SameVoiceTwice(String),
    /// The source could not express the placement.
    Uneditable(String),
    /// The document the placement would write does not compile.
    Rejected(String),
}

impl core::fmt::Display for PlacementError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NotReviewing => write!(formatter, "there is no take under review"),
            Self::NotAccepted => write!(formatter, "this reading has not been accepted yet"),
            Self::NoPhrase => write!(
                formatter,
                "this reading has a length it cannot write exactly, so there is no phrase to keep"
            ),
            Self::NoValidScore => write!(
                formatter,
                "this piece does not compile, so there is no part to keep the phrase in"
            ),
            Self::PieceChanged(ref part) => write!(
                formatter,
                "Piece changed—review this phrase against the current score: it has no part `{part}` any more"
            ),
            Self::NeedsVoiceNames { needed, given } => write!(
                formatter,
                "this phrase is written in {needed} lines and {given} were named; name each line before keeping it"
            ),
            Self::NotAName(ref name) => write!(formatter, "`{name}` is not a name a voice can have"),
            Self::SameVoiceTwice(ref name) => {
                write!(formatter, "two lines of this phrase were both pointed at `{name}`")
            }
            Self::Uneditable(ref reason) => write!(formatter, "{reason}"),
            Self::Rejected(ref reason) => write!(formatter, "the phrase would not compile there: {reason}"),
        }
    }
}

impl std::error::Error for PlacementError {}

/// Whether `name` is something the language can write as a voice name.
///
/// Checked here rather than discovered by the parser, because a name that
/// does not parse would turn a composer's typo into "the phrase would not
/// compile" — a sentence about the wrong thing.
pub(crate) fn is_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// The name to offer for each of a proposal's lines.
///
/// The first line goes where the caret was, because that is where the
/// composer was looking when they played. Every line after it is a line the
/// score does not have yet, and it is offered a free name rather than an
/// existing voice: appending a second hand onto somebody's inner voice
/// because it happened to be next in the part is exactly the silent
/// replacement this is not allowed to do.
pub(crate) fn suggest(
    voices: &[ProposalVoice],
    destination: &crate::ReviewDestination,
    taken: &[String],
) -> Vec<String> {
    let mut used: Vec<String> = taken.to_vec();
    let mut names = Vec::with_capacity(voices.len());
    for (line, _) in voices.iter().enumerate() {
        let name = if line == 0 {
            destination
                .voice
                .clone()
                .filter(|name| is_name(name))
                .unwrap_or_else(|| free("voice", &used))
        } else {
            free("captured", &used)
        };
        used.push(name.clone());
        names.push(name);
    }
    names
}

/// `stem`, `stem2`, `stem3`… — the first that nothing has taken.
///
/// Bounded by the number of names already in play plus two, which is one more
/// than the pigeonhole needs: a score has at most four lines a part.
fn free(stem: &str, taken: &[String]) -> String {
    if !taken.iter().any(|name| name == stem) {
        return stem.to_owned();
    }
    let ceiling = u32::try_from(taken.len()).unwrap_or(u32::MAX).saturating_add(2);
    (2_u32..=ceiling)
        .map(|suffix| format!("{stem}{suffix}"))
        .find(|candidate| !taken.contains(candidate))
        .unwrap_or_else(|| stem.to_owned())
}

/// One musical sentence for what this placement writes.
pub(crate) fn summarize(notes: usize, lines: &[PlacedVoice], part: &str) -> String {
    let bars: usize = lines.iter().map(|line| line.bars.len()).sum();
    let notes = if notes == 1 {
        "1 note".to_owned()
    } else {
        format!("{notes} notes")
    };
    let bars = if bars == 1 {
        "1 bar".to_owned()
    } else {
        format!("{bars} bars")
    };
    let where_to = match *lines {
        [ref only] => format!("{}’s {}", part, only.name),
        _ => format!(
            "{}’s {}",
            part,
            lines
                .iter()
                .map(|line| line.name.as_str())
                .collect::<Vec<_>>()
                .join(" and ")
        ),
    };
    let added = lines.iter().filter(|line| line.added).count();
    let new_lines = match added {
        0 => String::new(),
        1 => ", adding one line".to_owned(),
        many => format!(", adding {many} lines"),
    };
    format!("{notes} in {bars} into {where_to}{new_lines}")
}
