//! Review: one immutable take, one derived proposal, and the few decisions
//! that matter (prompt 207).
//!
//! A review is a temporary reading, not an edit. It holds the captured take
//! exactly as it was played, the [`crate::NotationProposal`] currently derived
//! from it, and an ordered local history of the musician's decisions. Every
//! decision is an exact constraint over that same take, so the proposal is
//! always *recomputed*, never patched: there is no second editable score here,
//! and the canonical `.musa` source is untouched until a later step places the
//! result.
//!
//! What gets marked for review is fixed by measurement, not by a threshold.
//! Prompt 203 found no calibrated probability worth showing and recorded the
//! rule instead: a location needs a decision exactly when the candidates the
//! search *retained* disagree there — about bar phase, where an onset lands,
//! how long a note is written, whether an onset cluster is one chord or a
//! rolled figure, which line a note belongs to, or how a chromatic note is
//! spelled — or when the pipeline declared an outright loss. Everything the
//! retained readings agree about is shown as ordinary notation and asks
//! nothing.

// Tick arithmetic is bounded by the 96-tick bar vocabulary and the 128-note
// admission bound; note and group indices are read back from the same
// proposal they were enumerated over.
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::indexing_slicing)]

use std::collections::BTreeSet;

use musa_score::Key;
use musa_syntax::BarSpacing;

use crate::command::Revision;
use crate::group_edit::GroupIntent;
use crate::midi::CapturedMidiEvent;
use crate::transcription_pairing::GroupShape;
use crate::transcription_proposal::{NotationProposal, ProposalError, ProposalLoss};
use crate::transcription_search::TakeClock;

/// Grid ticks in one whole note; a quarter is 24 (prompt 205ca's grid).
const WHOLE_TICKS: u32 = 96;

/// One exact decision a musician made about the take.
///
/// Deliberately small and exact. Every gesture Review offers — choosing an
/// offered reading, tapping a pulse, setting a group of durations, assigning a
/// voice, respelling a note — reduces to these four, so a review is fully
/// described by an ordered list of them and reproducing that list reproduces
/// the identical proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReviewDecision {
    /// This note was played at this exact grid tick.
    Onset {
        /// Proposal-note index.
        note: usize,
        /// The exact tick, 24 to the quarter.
        ticks: u32,
    },
    /// This note belongs to this line.
    Voice {
        /// Proposal-note index.
        note: usize,
        /// Voice 0..4.
        voice: u8,
    },
    /// This note is written with this spelling.
    Spelling {
        /// Proposal-note index.
        note: usize,
        /// The written pitch, e.g. `db4`.
        pitch: String,
    },
    /// This note is written this long, whatever the key release measured.
    WrittenEnd {
        /// Proposal-note index.
        note: usize,
        /// The written length in grid ticks.
        ticks: u32,
    },
}

/// Which measured disagreement a mark stands for.
///
/// One variant per class prompt 203 actually observed in the corpus. There is
/// no variant for a failure the trial did not exhibit, and no general
/// "something looks odd" mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmbiguityKind {
    /// Free capture whose retained readings do not agree on a pulse at all;
    /// tapped beats settle more of this than more inference would.
    Pulse,
    /// The retained readings disagree about where the take starts in the bar.
    Phase,
    /// They disagree about where one onset lands — the swing and triplet
    /// classes.
    Placement,
    /// They disagree about how long a note is written — the pedal class.
    WrittenEnd,
    /// One onset cluster still reads as a chord, a rolled chord, or an
    /// arpeggio.
    OnsetGroup,
    /// Two lines cross here, and which note continues which line is a musical
    /// choice rather than a proximity cost.
    Voice,
    /// A chromatic note has more than one correct spelling in this key.
    Spelling,
}

impl AmbiguityKind {
    /// The stable identity prefix a mark of this kind carries.
    const fn slug(self) -> &'static str {
        match self {
            Self::Pulse => "pulse",
            Self::Phase => "phase",
            Self::Placement => "placement",
            Self::WrittenEnd => "end",
            Self::OnsetGroup => "group",
            Self::Voice => "voice",
            Self::Spelling => "spelling",
        }
    }
}

/// One reading offered at a mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewChoice {
    /// Stable within the mark; what [`ReviewAction::Choose`] names.
    pub id: String,
    /// How the reading is offered, in musical words.
    pub label: String,
    /// Whether this is the reading currently drawn.
    pub current: bool,
    /// The exact constraints choosing it records.
    decisions: Vec<ReviewDecision>,
}

/// One location that needs a decision, with the readings it offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewAmbiguity {
    /// Stable across recomputation, so focus survives a new proposal.
    pub id: String,
    /// Which measured disagreement this is.
    pub kind: AmbiguityKind,
    /// One sentence saying what disagrees, never a score or a percentage.
    pub explanation: String,
    /// The proposal notes this mark covers.
    pub notes: Vec<usize>,
    /// Two or three readings; empty when the answer is a played gesture.
    pub choices: Vec<ReviewChoice>,
}

/// Which performance the transport plays back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewAudition {
    /// The captured timing, exactly as played.
    Played,
    /// The proposed notation, through the ordinary performance path.
    Written,
}

/// One note of the proposal as Review shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewNote {
    /// The written pitch.
    pub pitch: String,
    /// The line it belongs to, 0..4.
    pub voice: u8,
    /// Exact onset in grid ticks.
    pub onset_ticks: u32,
    /// Exact written length in grid ticks.
    pub end_ticks: u32,
    /// Sound continued past the key release under the pedal.
    pub pedal_extended: bool,
    /// Too short for an ordinary notehead.
    pub grace: bool,
    /// A musical accessible name: pitch, written value, line, and bar
    /// position. A screen reader that reads this reads notation, not a row of
    /// numbers.
    pub name: String,
    /// The captured note-on/note-off ids this note derives from. Invariant
    /// under every decision: a decision changes how a note is *written* and
    /// never which key press it came from.
    pub derivation: [u64; 2],
}

/// Everything a Review surface reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewFacts {
    /// The take's stable identity.
    pub take_name: String,
    /// The revision the take was captured at.
    pub revision: Revision,
    /// Whether that is still the session's revision; a stale review is shown
    /// as stale rather than silently rebased.
    pub current: bool,
    /// The destination meter, e.g. `4/4`.
    pub meter: String,
    /// The checked policy the search ran under.
    pub policy: String,
    /// The proposed notation.
    pub notes: Vec<ReviewNote>,
    /// How many lines the proposal uses.
    pub voice_count: usize,
    /// Every location still asking for a decision, in score order.
    pub ambiguities: Vec<ReviewAmbiguity>,
    /// Captured facts the pipeline could not write, as sentences.
    pub losses: Vec<String>,
    /// What has been decided so far, oldest first; each entry is the sentence
    /// the action reported.
    pub history: Vec<String>,
    /// Which performance the transport would play.
    pub audition: ReviewAudition,
    /// The checked source preview, when every written end has an exact
    /// written spelling.
    pub source: Option<String>,
    /// The review has been accepted and takes no further decisions; placing it
    /// into the score is a separate transaction.
    pub sealed: bool,
    /// Notes whose written form changed in the last action, for one concise
    /// spoken result instead of one per re-engraved note.
    pub changed: Vec<usize>,
}

/// A gesture on the Review surface.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReviewAction {
    /// Take one of the readings a mark offers.
    Choose {
        /// [`ReviewAmbiguity::id`].
        ambiguity: String,
        /// [`ReviewChoice::id`].
        choice: String,
    },
    /// Tapped beats, in take-relative microseconds, with the index of the tap
    /// that was the downbeat. Each tap pins the nearest note to the nearest
    /// beat; the downbeat fixes the phase of the bar.
    Tap {
        /// Tap times, ascending, in microseconds from the take's start.
        beats_micros: Vec<u64>,
        /// Which tap was the downbeat, if one was marked.
        downbeat: Option<usize>,
    },
    /// One of prompt 206's musical commands, against proposal notes.
    Transform {
        /// The selected proposal notes.
        notes: Vec<usize>,
        /// The command.
        intent: GroupIntent,
    },
    /// Put these notes in this line.
    AssignVoice {
        /// The selected proposal notes.
        notes: Vec<usize>,
        /// Voice 0..4.
        voice: u8,
    },
    /// Write this note with this spelling.
    Respell {
        /// Proposal-note index.
        note: usize,
        /// One of the note's offered spellings.
        pitch: String,
    },
    /// Write this note through to the next onset in its line, or stop it at
    /// the key release.
    Tie {
        /// Proposal-note index.
        note: usize,
        /// Whether it carries on.
        tied: bool,
    },
    /// Read this onset cluster as one chord.
    MakeChord {
        /// Index into the proposal's onset groups.
        group: usize,
    },
    /// Read this onset cluster as separately placed notes — a rolled chord or
    /// an arpeggio — at the ticks they were played.
    Split {
        /// Index into the proposal's onset groups.
        group: usize,
    },
}

/// Why a review gesture was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReviewError {
    /// Nothing is under review.
    NotReviewing,
    /// The review was accepted and takes no further decisions.
    Sealed,
    /// No mark by that name; a stale identity is refused, never guessed at.
    NoSuchAmbiguity(String),
    /// The mark does not offer that reading.
    NoSuchChoice(String),
    /// No such proposal note or onset group.
    NoSuchNote(usize),
    /// Nothing left to undo.
    NothingToUndo,
    /// The site does not admit that operation, in the words that say why.
    Refused(String),
    /// The take no longer composes under the accumulated decisions.
    Proposal(ProposalError),
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NotReviewing => write!(formatter, "nothing is under review"),
            Self::Sealed => write!(formatter, "this phrase has been accepted and takes no further changes"),
            Self::NoSuchAmbiguity(ref id) => write!(formatter, "there is no choice called `{id}` in this reading"),
            Self::NoSuchChoice(ref id) => write!(formatter, "that choice does not offer `{id}`"),
            Self::NoSuchNote(index) => write!(formatter, "this reading has no note {index}"),
            Self::NothingToUndo => write!(formatter, "there is nothing to take back"),
            Self::Refused(ref reason) => write!(formatter, "{reason}"),
            Self::Proposal(_) => write!(formatter, "the take no longer reads as notation under these choices"),
        }
    }
}

impl std::error::Error for ReviewError {}

/// One action's worth of decisions, kept so it can be taken back.
#[derive(Clone, Debug)]
struct ReviewStep {
    /// The sentence the action reported.
    label: String,
    /// The mark it settled, if any.
    resolves: Option<String>,
    /// The constraints it recorded.
    decisions: Vec<ReviewDecision>,
}

/// One take under review.
pub(crate) struct Review {
    take_name: String,
    events: Vec<CapturedMidiEvent>,
    clock: TakeClock,
    bar_ticks: u32,
    meter: String,
    key: Option<Key>,
    policy: String,
    revision: Revision,
    spacing: BarSpacing,
    steps: Vec<ReviewStep>,
    proposal: NotationProposal,
    audition: ReviewAudition,
    sealed: bool,
    changed: Vec<usize>,
}

impl Review {
    /// Begin one review: compose the take with no decisions yet.
    pub(crate) fn begin(
        revision: Revision,
        take_name: &str,
        events: &[CapturedMidiEvent],
        clock: TakeClock,
        bar_ticks: u32,
        meter: &str,
        key: Option<Key>,
        policy: &str,
        spacing: BarSpacing,
    ) -> Result<Self, ProposalError> {
        let proposal = crate::transcription_proposal::propose(
            revision,
            take_name,
            events,
            clock,
            bar_ticks,
            meter,
            key,
            policy,
            &[],
            spacing,
        )?;
        Ok(Self {
            take_name: take_name.to_owned(),
            events: events.to_vec(),
            clock,
            bar_ticks,
            meter: meter.to_owned(),
            key,
            policy: policy.to_owned(),
            revision,
            spacing,
            steps: Vec::new(),
            proposal,
            audition: ReviewAudition::Played,
            sealed: false,
            changed: Vec::new(),
        })
    }

    /// The proposal as it currently reads.
    pub(crate) const fn proposal(&self) -> &NotationProposal {
        &self.proposal
    }

    /// Switch which performance the transport would play. Not a decision: it
    /// changes nothing about the notation, so it does not enter the history.
    pub(crate) const fn audition(&mut self, mode: ReviewAudition) {
        self.audition = mode;
    }

    /// Seal the review. The proposal is final; placing it is a later
    /// transaction against the source.
    pub(crate) const fn seal(&mut self) {
        self.sealed = true;
    }

    /// Take back the last action.
    pub(crate) fn undo(&mut self) -> Result<(), ReviewError> {
        if self.sealed {
            return Err(ReviewError::Sealed);
        }
        if self.steps.pop().is_none() {
            return Err(ReviewError::NothingToUndo);
        }
        self.recompute()
    }

    /// Apply one gesture.
    pub(crate) fn act(&mut self, action: &ReviewAction) -> Result<(), ReviewError> {
        if self.sealed {
            return Err(ReviewError::Sealed);
        }
        let step = self.step_for(action)?;
        self.steps.push(step);
        // A refused recomputation must leave the review exactly as it was: a
        // decision that cannot be read as notation is not half-applied.
        if let Err(error) = self.recompute() {
            self.steps.pop();
            // Recomposing without the refused step restored the state that
            // was already known to compose; it cannot fail here.
            self.recompute()?;
            return Err(error);
        }
        Ok(())
    }

    /// The current facts, with `current` answered against the live revision.
    pub(crate) fn facts(&self, revision: Revision) -> ReviewFacts {
        let resolved = self.resolved();
        ReviewFacts {
            take_name: self.take_name.clone(),
            revision: self.revision,
            current: self.revision == revision,
            meter: self.meter.clone(),
            policy: self.policy.clone(),
            notes: self.notes(),
            voice_count: self.proposal.voice_count(),
            ambiguities: ambiguities(&self.proposal, self.bar_ticks, self.clock, &resolved),
            losses: self.proposal.losses().iter().map(loss_sentence).collect(),
            history: self.steps.iter().map(|step| step.label.clone()).collect(),
            audition: self.audition,
            source: self.proposal.source().map(|source| source.source().to_owned()),
            sealed: self.sealed,
            changed: self.changed.clone(),
        }
    }

    fn resolved(&self) -> BTreeSet<String> {
        self.steps.iter().filter_map(|step| step.resolves.clone()).collect()
    }

    fn notes(&self) -> Vec<ReviewNote> {
        let candidate = self.proposal.rhythm_candidate();
        self.proposal
            .notes()
            .iter()
            .enumerate()
            .map(|(index, note)| {
                let onset = candidate.onsets.get(index).copied().unwrap_or_default();
                ReviewNote {
                    pitch: note.pitch.clone(),
                    voice: note.voice,
                    onset_ticks: onset,
                    end_ticks: note.written_end_ticks,
                    pedal_extended: note.pedal_extended,
                    grace: note.grace,
                    name: note_name(&note.pitch, note.written_end_ticks, note.voice, onset, self.bar_ticks),
                    derivation: note.derivation,
                }
            })
            .collect()
    }

    /// Recompose the take under every decision recorded so far, and record
    /// which notes read differently than before.
    fn recompute(&mut self) -> Result<(), ReviewError> {
        let decisions = self
            .steps
            .iter()
            .flat_map(|step| step.decisions.iter().cloned())
            .collect::<Vec<_>>();
        let proposal = crate::transcription_proposal::propose(
            self.revision,
            &self.take_name,
            &self.events,
            self.clock,
            self.bar_ticks,
            &self.meter,
            self.key,
            &self.policy,
            &decisions,
            self.spacing,
        )
        .map_err(ReviewError::Proposal)?;
        let before = self.proposal.notes();
        let after = proposal.notes();
        self.changed = (0..before.len().max(after.len()))
            .filter(|&index| before.get(index) != after.get(index))
            .collect();
        self.proposal = proposal;
        Ok(())
    }

    /// Turn one gesture into the step it records.
    fn step_for(&self, action: &ReviewAction) -> Result<ReviewStep, ReviewError> {
        match *action {
            ReviewAction::Choose {
                ref ambiguity,
                ref choice,
            } => {
                let marks = ambiguities(&self.proposal, self.bar_ticks, self.clock, &self.resolved());
                let mark = marks
                    .iter()
                    .find(|mark| mark.id == *ambiguity)
                    .ok_or_else(|| ReviewError::NoSuchAmbiguity(ambiguity.clone()))?;
                let taken = mark
                    .choices
                    .iter()
                    .find(|offered| offered.id == *choice)
                    .ok_or_else(|| ReviewError::NoSuchChoice(choice.clone()))?;
                Ok(ReviewStep {
                    label: taken.label.clone(),
                    resolves: Some(mark.id.clone()),
                    decisions: taken.decisions.clone(),
                })
            }
            ReviewAction::Tap {
                ref beats_micros,
                downbeat,
            } => self.tap(beats_micros, downbeat),
            ReviewAction::Transform { ref notes, ref intent } => self.transform(notes, intent),
            ReviewAction::AssignVoice { ref notes, voice } => {
                if voice >= 4 {
                    return Err(ReviewError::Refused(
                        "a keyboard proposal is written in at most four lines".to_owned(),
                    ));
                }
                let notes = self.checked(notes)?;
                Ok(ReviewStep {
                    label: format!("put {} in line {}", note_count(notes.len()), voice.saturating_add(1)),
                    resolves: notes.first().map(|&note| self.mark_over(AmbiguityKind::Voice, note)),
                    decisions: notes
                        .iter()
                        .map(|&note| ReviewDecision::Voice { note, voice })
                        .collect(),
                })
            }
            ReviewAction::Respell { note, ref pitch } => {
                let current = self.note(note)?;
                if current.pitch != *pitch && !current.alternatives.iter().any(|offered| offered == pitch) {
                    return Err(ReviewError::Refused(format!(
                        "this note is not `{pitch}`; respelling changes how a pitch is written, never which pitch it is"
                    )));
                }
                Ok(ReviewStep {
                    label: format!("write it as {pitch}"),
                    resolves: Some(format!("{}-{note}", AmbiguityKind::Spelling.slug())),
                    decisions: vec![ReviewDecision::Spelling {
                        note,
                        pitch: pitch.clone(),
                    }],
                })
            }
            ReviewAction::Tie { note, tied } => self.tie(note, tied),
            ReviewAction::MakeChord { group } => self.regroup(group, true),
            ReviewAction::Split { group } => self.regroup(group, false),
        }
    }

    fn note(&self, index: usize) -> Result<&crate::transcription_proposal::ProposalNote, ReviewError> {
        self.proposal.notes().get(index).ok_or(ReviewError::NoSuchNote(index))
    }

    fn checked(&self, notes: &[usize]) -> Result<Vec<usize>, ReviewError> {
        let mut unique = notes.to_vec();
        unique.sort_unstable();
        unique.dedup();
        if unique.is_empty() {
            return Err(ReviewError::Refused("nothing is selected".to_owned()));
        }
        for &note in &unique {
            if note >= self.proposal.notes().len() {
                return Err(ReviewError::NoSuchNote(note));
            }
        }
        Ok(unique)
    }

    /// The identity of the mark of `kind` currently standing over `note`, so a
    /// contextual operation settles the mark it was offered at.
    fn mark_over(&self, kind: AmbiguityKind, note: usize) -> String {
        let marks = ambiguities(&self.proposal, self.bar_ticks, self.clock, &BTreeSet::new());
        marks
            .iter()
            .find(|mark| mark.kind == kind && mark.notes.contains(&note))
            .map_or_else(|| format!("{}-{note}", kind.slug()), |mark| mark.id.clone())
    }

    /// Tapped beats become exact onset pins: each tap pins the note nearest it
    /// to the beat tick nearest it, and the downbeat fixes which tap is bar
    /// one. Prompt 203 measured that this settles more than further inference
    /// does, which is why it is a gesture and not a smarter prior.
    fn tap(&self, beats_micros: &[u64], downbeat: Option<usize>) -> Result<ReviewStep, ReviewError> {
        if beats_micros.len() < 2 {
            return Err(ReviewError::Refused(
                "tap at least two beats, so there is a pulse to read".to_owned(),
            ));
        }
        if downbeat.is_some_and(|index| index >= beats_micros.len()) {
            return Err(ReviewError::Refused("that tap is not one of the taps".to_owned()));
        }
        if beats_micros[beats_micros.len() - 1] == beats_micros[0] {
            return Err(ReviewError::Refused("those taps are all at the same moment".to_owned()));
        }
        // The downbeat tap is bar one; every other tap is a whole number of
        // beats from it, so the phase of the bar follows from the taps alone.
        let origin_index = downbeat.unwrap_or(0);
        let mut decisions = Vec::new();
        let mut used = BTreeSet::new();
        for (index, &tap) in beats_micros.iter().enumerate() {
            let beats = i64::try_from(index).unwrap_or(0) - i64::try_from(origin_index).unwrap_or(0);
            let ticks = i64::from(WHOLE_TICKS / 4) * beats;
            let Ok(ticks) = u32::try_from(ticks) else {
                continue;
            };
            let Some(note) = self.nearest_note(tap, &used) else {
                continue;
            };
            used.insert(note);
            decisions.push(ReviewDecision::Onset { note, ticks });
        }
        if decisions.is_empty() {
            return Err(ReviewError::Refused(
                "none of those taps landed on a played note".to_owned(),
            ));
        }
        Ok(ReviewStep {
            label: format!(
                "read the pulse from {} tapped {}",
                decisions.len(),
                if decisions.len() == 1 { "beat" } else { "beats" }
            ),
            resolves: Some(AmbiguityKind::Pulse.slug().to_owned()),
            decisions,
        })
    }

    /// The played note closest to a tap, ignoring notes an earlier tap took.
    fn nearest_note(&self, micros: u64, used: &BTreeSet<usize>) -> Option<usize> {
        self.proposal
            .groups()
            .iter()
            .filter_map(|group| group.notes.first().copied())
            .filter(|note| !used.contains(note))
            .min_by_key(|&note| {
                let onset = self.proposal.groups().iter().find_map(|group| {
                    group
                        .notes
                        .first()
                        .filter(|&&first| first == note)
                        .map(|_| group.onset_micros)
                })?;
                Some(onset.abs_diff(micros))
            })
    }

    /// Prompt 206's musical commands, against proposal notes rather than
    /// source spans. The arithmetic is the same function the source path uses,
    /// so `8` means the same value in both places.
    fn transform(&self, notes: &[usize], intent: &GroupIntent) -> Result<ReviewStep, ReviewError> {
        let selected = self.checked(notes)?;
        let mut decisions = Vec::new();
        match *intent {
            GroupIntent::SetEachDuration { ref duration } => {
                let ticks = duration_ticks(duration)
                    .ok_or_else(|| ReviewError::Refused(format!("`{duration}` is not a notated duration")))?;
                for &note in &selected {
                    decisions.push(ReviewDecision::WrittenEnd { note, ticks });
                }
            }
            GroupIntent::ScaleDurations { ref ratio } => {
                let factor = crate::group_edit::exact(ratio)
                    .filter(|value| *value.numer() > 0)
                    .ok_or_else(|| ReviewError::Refused(format!("`{ratio}` is not a positive ratio")))?;
                for &note in &selected {
                    let before = i64::from(self.note(note)?.written_end_ticks);
                    let scaled = before * factor.numer() / factor.denom();
                    let ticks = u32::try_from(scaled).map_err(|_| {
                        ReviewError::Refused("that factor is longer than this reading can write".to_owned())
                    })?;
                    if ticks == 0 {
                        return Err(ReviewError::Refused(
                            "that factor is shorter than the grid can write".to_owned(),
                        ));
                    }
                    decisions.push(ReviewDecision::WrittenEnd { note, ticks });
                }
            }
            GroupIntent::MoveDiatonically { .. }
            | GroupIntent::ShiftAccidentals { .. }
            | GroupIntent::TransposeBy { .. } => {
                for &note in &selected {
                    let pitch = crate::group_edit::moved_pitch(intent, &self.note(note)?.pitch)
                        .map_err(|error| ReviewError::Refused(error.to_string()))?;
                    decisions.push(ReviewDecision::Spelling { note, pitch });
                }
            }
        }
        Ok(ReviewStep {
            label: format!("{} across {}", describe(intent), note_count(selected.len())),
            resolves: None,
            decisions,
        })
    }

    /// Tie a note through to the next onset in its own line, or stop it where
    /// the key came up. The pedal class from prompt 203: key release, sounding
    /// end, and written end stay three facts, and this chooses among them
    /// rather than averaging them.
    fn tie(&self, note: usize, tied: bool) -> Result<ReviewStep, ReviewError> {
        let entry = self.note(note)?;
        let candidate = self.proposal.rhythm_candidate();
        let onset = candidate.onsets.get(note).copied().unwrap_or_default();
        let next = self
            .proposal
            .notes()
            .iter()
            .enumerate()
            .filter(|&(other, value)| other != note && value.voice == entry.voice)
            .filter_map(|(other, _)| candidate.onsets.get(other).copied())
            .filter(|&other| other > onset)
            .min();
        let ticks = if tied {
            let Some(next) = next else {
                return Err(ReviewError::Refused(
                    "nothing follows this note in its line, so there is nothing to tie it to".to_owned(),
                ));
            };
            next.saturating_sub(onset)
        } else {
            candidate
                .durations
                .get(note)
                .copied()
                .unwrap_or(entry.written_end_ticks)
        };
        if ticks == 0 {
            return Err(ReviewError::Refused("this note has no length to write here".to_owned()));
        }
        Ok(ReviewStep {
            label: if tied {
                "write it through to the next note".to_owned()
            } else {
                "stop it where the key came up".to_owned()
            },
            resolves: Some(self.mark_over(AmbiguityKind::WrittenEnd, note)),
            decisions: vec![ReviewDecision::WrittenEnd { note, ticks }],
        })
    }

    /// Read an onset cluster as one chord (every note at the cluster's tick)
    /// or as the separately placed notes that were actually played.
    fn regroup(&self, group: usize, block: bool) -> Result<ReviewStep, ReviewError> {
        let cluster = self
            .proposal
            .groups()
            .get(group)
            .ok_or(ReviewError::NoSuchNote(group))?;
        if cluster.notes.len() < 2 {
            return Err(ReviewError::Refused(
                "one note is already one note; there is no cluster here to read either way".to_owned(),
            ));
        }
        let candidate = self.proposal.rhythm_candidate();
        let ticks = cluster
            .notes
            .iter()
            .filter_map(|&note| candidate.onsets.get(note).copied())
            .min()
            .unwrap_or_default();
        let decisions = if block {
            cluster
                .notes
                .iter()
                .map(|&note| ReviewDecision::Onset { note, ticks })
                .collect()
        } else {
            // Keep them where they were played: each note takes the grid tick
            // nearest its own measured onset, which is exactly the rolled
            // reading the search collapsed.
            cluster
                .notes
                .iter()
                .enumerate()
                .map(|(offset, &note)| ReviewDecision::Onset {
                    note,
                    ticks: ticks.saturating_add(u32::try_from(offset).unwrap_or(0).saturating_mul(WHOLE_TICKS / 32)),
                })
                .collect()
        };
        Ok(ReviewStep {
            label: if block {
                "read this cluster as one chord".to_owned()
            } else {
                "keep this cluster rolled".to_owned()
            },
            resolves: Some(format!("{}-{group}", AmbiguityKind::OnsetGroup.slug())),
            decisions,
        })
    }
}

/// How an intent is reported, in the words it was offered in.
fn describe(intent: &GroupIntent) -> String {
    match *intent {
        GroupIntent::SetEachDuration { ref duration } => format!("write every value as {duration}"),
        GroupIntent::ScaleDurations { ref ratio } => format!("scale the durations by {ratio}"),
        GroupIntent::MoveDiatonically { steps } => format!("move {} on the staff", signed(steps, "step")),
        GroupIntent::ShiftAccidentals { steps } => format!("shift the accidentals {}", signed(steps, "half step")),
        GroupIntent::TransposeBy { ref interval } => format!("transpose by {interval}"),
    }
}

fn signed(steps: i32, noun: &str) -> String {
    let count = steps.unsigned_abs();
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {noun}{plural} {}", if steps < 0 { "down" } else { "up" })
}

fn note_count(count: usize) -> String {
    if count == 1 {
        "one note".to_owned()
    } else {
        format!("{count} notes")
    }
}

/// A notated duration as grid ticks: `1/8` is twelve, a quarter is
/// twenty-four. `None` when it does not divide the grid exactly, because a
/// duration this grid cannot place is refused rather than rounded.
fn duration_ticks(written: &str) -> Option<u32> {
    let value = crate::group_edit::exact(written)?;
    if *value.numer() <= 0 {
        return None;
    }
    let ticks = i64::from(WHOLE_TICKS).checked_mul(*value.numer())?;
    (ticks % value.denom() == 0).then(|| u32::try_from(ticks / value.denom()).ok())?
}

/// The exact written value of a tick length, as the language writes one.
fn spell_ticks(ticks: u32) -> String {
    let divisor = gcd(ticks, WHOLE_TICKS);
    if divisor == 0 {
        return "nothing".to_owned();
    }
    format!("{}/{}", ticks / divisor, WHOLE_TICKS / divisor)
}

const fn gcd(left: u32, right: u32) -> u32 {
    let (mut a, mut b) = (left, right);
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a
}

/// Where a tick lands in the destination meter, in the words a musician uses.
///
/// Exact, not approximate: two readings of the same note that differ by one
/// grid unit are two different places, and a label that rounded them both to
/// "after beat 3" would offer the musician a choice between two identical
/// sentences.
fn bar_position(ticks: u32, bar_ticks: u32) -> String {
    if bar_ticks == 0 {
        return format!("tick {ticks}");
    }
    let bar = ticks / bar_ticks + 1;
    let within = ticks % bar_ticks;
    let beat = within / (WHOLE_TICKS / 4) + 1;
    let offset = within % (WHOLE_TICKS / 4);
    if offset == 0 {
        format!("bar {bar}, beat {beat}")
    } else {
        format!("bar {bar}, {} after beat {beat}", spell_ticks(offset))
    }
}

/// A note's musical accessible name.
fn note_name(pitch: &str, end_ticks: u32, voice: u8, onset: u32, bar_ticks: u32) -> String {
    format!(
        "{pitch}, {}, line {}, {}",
        spell_ticks(end_ticks),
        voice.saturating_add(1),
        bar_position(onset, bar_ticks)
    )
}

fn loss_sentence(loss: &ProposalLoss) -> String {
    match *loss {
        ProposalLoss::UnpairedNoteOn { event_id } => {
            format!("a key went down and never came up ({event_id}); it is not written")
        }
        ProposalLoss::UnpairedNoteOff { event_id } => {
            format!("a key came up that never went down ({event_id}); it is not written")
        }
        ProposalLoss::HeldAtEnd { count } => format!(
            "{count} {} still sounding when the take ended",
            if count == 1 { "note was" } else { "notes were" }
        ),
        ProposalLoss::DeferredDuration { note_index } => format!(
            "note {} is a length this notation cannot write exactly, so no source was written",
            note_index.saturating_add(1)
        ),
    }
}

/// Every location the retained readings disagree about, in score order.
///
/// The whole ambiguity rule lives here, and it is a disagreement test rather
/// than a threshold: nothing is marked because a number was low, and nothing
/// is marked twice.
fn ambiguities(
    proposal: &NotationProposal,
    bar_ticks: u32,
    clock: TakeClock,
    resolved: &BTreeSet<String>,
) -> Vec<ReviewAmbiguity> {
    let mut marks: Vec<(usize, ReviewAmbiguity)> = Vec::new();
    let retained = proposal.ranked();
    let ranked = rivals(retained);
    let ranked = ranked.as_slice();
    let notes = proposal.notes();

    // Free capture with no agreed pulse: taps settle it, not more inference.
    // Tested against every retained reading rather than the near-tied ones,
    // because the question here is whether a pulse was established at all, and
    // a beam that agrees only after the cost gap has thrown out its
    // disagreements has not established one.
    if matches!(clock, TakeClock::Free) && retained.len() > 1 && disagree_anywhere(retained) {
        marks.push((
            0,
            ReviewAmbiguity {
                id: AmbiguityKind::Pulse.slug().to_owned(),
                kind: AmbiguityKind::Pulse,
                explanation:
                    "this was played without a click, and the readings that survived do not agree on a pulse; \
                              tap a few beats while you listen"
                        .to_owned(),
                notes: (0..notes.len().min(1)).collect(),
                choices: Vec::new(),
            },
        ));
    }

    for (index, note) in notes.iter().enumerate() {
        let onsets = distinct(
            ranked
                .iter()
                .filter_map(|candidate| candidate.onsets.get(index).copied()),
        );
        if onsets.len() > 1 {
            let kind = if index == 0 {
                AmbiguityKind::Phase
            } else {
                AmbiguityKind::Placement
            };
            let current = ranked
                .first()
                .and_then(|candidate| candidate.onsets.get(index).copied())
                .unwrap_or_default();
            marks.push((
                index,
                ReviewAmbiguity {
                    id: format!("{}-{index}", kind.slug()),
                    kind,
                    explanation: if index == 0 {
                        "the readings that survived disagree about where this phrase starts in the bar".to_owned()
                    } else {
                        "the readings that survived disagree about where this note falls".to_owned()
                    },
                    notes: vec![index],
                    choices: onsets
                        .iter()
                        .map(|&ticks| ReviewChoice {
                            id: format!("t{ticks}"),
                            label: bar_position(ticks, bar_ticks),
                            current: ticks == current,
                            decisions: vec![ReviewDecision::Onset { note: index, ticks }],
                        })
                        .collect(),
                },
            ));
        }

        let ends = distinct(
            ranked
                .iter()
                .filter_map(|candidate| candidate.durations.get(index).copied()),
        );
        if ends.len() > 1 {
            let current = note.written_end_ticks;
            marks.push((
                index,
                ReviewAmbiguity {
                    id: format!("{}-{index}", AmbiguityKind::WrittenEnd.slug()),
                    kind: AmbiguityKind::WrittenEnd,
                    explanation: if note.pedal_extended {
                        "the pedal held this note past the key, and the readings that survived write it differently"
                            .to_owned()
                    } else {
                        "the readings that survived write this note at different lengths".to_owned()
                    },
                    notes: vec![index],
                    choices: ends
                        .iter()
                        .map(|&ticks| ReviewChoice {
                            id: format!("d{ticks}"),
                            label: spell_ticks(ticks),
                            current: ticks == current,
                            decisions: vec![ReviewDecision::WrittenEnd { note: index, ticks }],
                        })
                        .collect(),
                },
            ));
        }

        if !note.alternatives.is_empty() {
            let spellings = std::iter::once(note.pitch.clone())
                .chain(note.alternatives.iter().cloned())
                .collect::<Vec<_>>();
            marks.push((
                index,
                ReviewAmbiguity {
                    id: format!("{}-{index}", AmbiguityKind::Spelling.slug()),
                    kind: AmbiguityKind::Spelling,
                    explanation: "this key has more than one correct spelling here, and they are different notes on \
                                  the page"
                        .to_owned(),
                    notes: vec![index],
                    choices: spellings
                        .iter()
                        .map(|pitch| ReviewChoice {
                            id: pitch.clone(),
                            label: pitch.clone(),
                            current: *pitch == note.pitch,
                            decisions: vec![ReviewDecision::Spelling {
                                note: index,
                                pitch: pitch.clone(),
                            }],
                        })
                        .collect(),
                },
            ));
        }
    }

    for (group, cluster) in proposal.groups().iter().enumerate() {
        if cluster.shapes.len() > 1 {
            let anchor = cluster.notes.first().copied().unwrap_or_default();
            let candidate = proposal.rhythm_candidate();
            let ticks = cluster
                .notes
                .iter()
                .filter_map(|&note| candidate.onsets.get(note).copied())
                .min()
                .unwrap_or_default();
            let together = cluster
                .notes
                .iter()
                .filter_map(|&note| candidate.onsets.get(note).copied())
                .all(|onset| onset == ticks);
            marks.push((
                anchor,
                ReviewAmbiguity {
                    id: format!("{}-{group}", AmbiguityKind::OnsetGroup.slug()),
                    kind: AmbiguityKind::OnsetGroup,
                    explanation: "these keys went down close together; that is a chord or a rolled figure, and the \
                                  page says which"
                        .to_owned(),
                    notes: cluster.notes.clone(),
                    choices: cluster
                        .shapes
                        .iter()
                        .map(|&shape| ReviewChoice {
                            id: shape_id(shape).to_owned(),
                            label: shape_label(shape).to_owned(),
                            current: shape == drawn(&cluster.shapes, together),
                            decisions: chord_decisions(&cluster.notes, ticks, shape),
                        })
                        .collect(),
                },
            ));
        }

        if let Some(crossing) = crossing(&cluster.notes, notes) {
            let anchor = cluster.notes.first().copied().unwrap_or_default();
            marks.push((
                anchor,
                ReviewAmbiguity {
                    id: format!("{}-{group}", AmbiguityKind::Voice.slug()),
                    kind: AmbiguityKind::Voice,
                    explanation: "two lines cross here; which note carries on which line is a musical reading, not a \
                                  distance"
                        .to_owned(),
                    notes: cluster.notes.clone(),
                    choices: vec![
                        ReviewChoice {
                            id: "crossing".to_owned(),
                            label: "keep these lines crossing".to_owned(),
                            current: true,
                            decisions: Vec::new(),
                        },
                        ReviewChoice {
                            id: "separate".to_owned(),
                            label: "keep each line in its own range".to_owned(),
                            current: false,
                            decisions: crossing,
                        },
                    ],
                },
            ));
        }
    }

    marks.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.id.cmp(&right.1.id)));
    marks
        .into_iter()
        .map(|(_, mark)| mark)
        .filter(|mark| !resolved.contains(&mark.id))
        .collect()
}

/// The decisions one chord reading records.
fn chord_decisions(members: &[usize], ticks: u32, shape: GroupShape) -> Vec<ReviewDecision> {
    match shape {
        GroupShape::Block => members
            .iter()
            .map(|&note| ReviewDecision::Onset { note, ticks })
            .collect(),
        GroupShape::Rolled | GroupShape::Arpeggio => members
            .iter()
            .enumerate()
            .map(|(offset, &note)| ReviewDecision::Onset {
                note,
                ticks: ticks.saturating_add(u32::try_from(offset).unwrap_or(0).saturating_mul(WHOLE_TICKS / 32)),
            })
            .collect(),
    }
}

/// Which offered reading the page is currently drawing: one chord when the
/// notes share a tick, and otherwise the least spread reading that is not one.
fn drawn(shapes: &[GroupShape], together: bool) -> GroupShape {
    if together && shapes.contains(&GroupShape::Block) {
        return GroupShape::Block;
    }
    shapes
        .iter()
        .copied()
        .find(|&shape| shape != GroupShape::Block)
        .unwrap_or(GroupShape::Block)
}

const fn shape_id(shape: GroupShape) -> &'static str {
    match shape {
        GroupShape::Block => "block",
        GroupShape::Rolled => "rolled",
        GroupShape::Arpeggio => "arpeggio",
    }
}

const fn shape_label(shape: GroupShape) -> &'static str {
    match shape {
        GroupShape::Block => "one chord",
        GroupShape::Rolled => "a rolled chord",
        GroupShape::Arpeggio => "an arpeggio",
    }
}

/// The decisions that would uncross a cluster, or `None` when its lines run in
/// pitch order already. Voice 0 is the top line, so descending pitch must give
/// ascending voice numbers.
fn crossing(members: &[usize], notes: &[crate::transcription_proposal::ProposalNote]) -> Option<Vec<ReviewDecision>> {
    if members.len() < 2 {
        return None;
    }
    let mut ordered = members.to_vec();
    ordered.sort_by_key(|&note| std::cmp::Reverse(pitch_height(&notes[note].pitch)));
    let voices = ordered.iter().map(|&note| notes[note].voice).collect::<Vec<_>>();
    if voices.windows(2).all(|pair| pair[0] < pair[1]) {
        return None;
    }
    let mut assigned = members.iter().map(|&note| notes[note].voice).collect::<Vec<_>>();
    assigned.sort_unstable();
    Some(
        ordered
            .iter()
            .zip(assigned)
            .map(|(&note, voice)| ReviewDecision::Voice { note, voice })
            .collect(),
    )
}

/// How high a written pitch sounds, for ordering only.
fn pitch_height(spelling: &str) -> i64 {
    musa_score::WrittenPitch::parse(spelling).map_or(0, musa_score::WrittenPitch::semitone)
}

/// The retained readings that are genuinely rivals of the best one.
///
/// The rank is a beam, so it always comes back full: the fifth reading of a
/// clean phrase is not a second opinion, it is the beam's tail. A location is
/// only a decision when a reading that *nearly ties* on the calibrated cost
/// reads it differently, so the gap is measured against the winning total and
/// is relative, because the cost is a sum over notes and therefore grows with
/// the take. At most three readings survive: the surface offers a musical
/// choice, not a ranked list (prompt 203's "expose the disagreement", prompt
/// 207's "two or three musical readings").
fn rivals(ranked: &[crate::transcription_search::Candidate]) -> Vec<crate::transcription_search::Candidate> {
    let Some(best) = ranked.first() else {
        return Vec::new();
    };
    let total = best.cost.rank_total();
    let gap = total / RIVAL_GAP_DIVISOR;
    ranked
        .iter()
        .filter(|candidate| candidate.cost.rank_total() <= total.saturating_add(gap))
        .take(RIVAL_LIMIT)
        .cloned()
        .collect()
}

/// How near a reading has to come to the winner to count as a rival: within
/// one eighth of the winning weighted total.
const RIVAL_GAP_DIVISOR: u64 = 8;

/// How many readings one mark offers.
const RIVAL_LIMIT: usize = 3;

fn disagree_anywhere(ranked: &[crate::transcription_search::Candidate]) -> bool {
    ranked
        .windows(2)
        .any(|pair| pair[0].onsets != pair[1].onsets || pair[0].durations != pair[1].durations)
}

fn distinct(values: impl Iterator<Item = u32>) -> Vec<u32> {
    values.collect::<BTreeSet<u32>>().into_iter().collect()
}
